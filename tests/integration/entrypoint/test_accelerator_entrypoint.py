"""Hermetic tests for the `bin/accelerator` plugin entry point.

Ports the former `scripts/test-accelerator-entrypoint.sh` to Python (ADR-0048:
Python is the test language for the non-Rust surfaces, shell wrappers included).

The bootstrap is exercised end-to-end with its documented test seams: fetches
are stubbed via `ACCELERATOR_BOOTSTRAP_DOWNLOADER` (a script that copies from a
local server dir and logs each requested URL), host detection is forced via the
injected `ACCELERATOR_UNAME_S`/`_M`, and signatures are *real* minisign
signatures verified by the *real* `accelerator-verify` shim built from `cli/`.

Every subprocess runs under an explicit, minimal environment (mirroring the
shell suite's `env -i`) so an ambient variable can't mask a bug. `cargo` and
`minisign` are mise-provisioned, so a missing tool is a CI provisioning
regression (fail) rather than a local convenience skip.
"""

import concurrent.futures
import contextlib
import hashlib
import json
import os
import platform
import shutil
import subprocess
import tempfile
import urllib.parse
from collections.abc import Callable, Iterator
from dataclasses import dataclass
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[2]
_REPO_BOOTSTRAP = _REPO_ROOT / "bin/accelerator"
_REPO_BIN = _REPO_ROOT / "bin"

# The harness pins a synthetic version so cache paths are deterministic and the
# real GitHub release base URL is never contacted (overridden to .invalid).
_VERSION = "9.9.9-test"

# Marks a fixture root's own templates/adr.md, so a resolved installation root
# is provable from the launcher's stdout rather than from a rendered path.
_SENTINEL = "FIXTURE-ADR-SENTINEL-{label}"

# Stand-in for the fetched launcher binary: records its argv (one per line) to
# LAUNCHER_ARGS_OUT, its whole environment as JSON to LAUNCHER_ENV_OUT, and
# exits with LAUNCHER_EXIT. Signed by minisign like a real release asset; its
# content is opaque to verification.
_LAUNCHER_SRC = """\
#!/usr/bin/env python3
import json
import os
import sys

out = os.environ.get("LAUNCHER_ARGS_OUT")
if out:
    with open(out, "w") as handle:
        for arg in sys.argv[1:]:
            handle.write(arg + "\\n")
env_out = os.environ.get("LAUNCHER_ENV_OUT")
if env_out:
    with open(env_out, "w") as handle:
        json.dump(dict(os.environ), handle)
sys.exit(int(os.environ.get("LAUNCHER_EXIT", "0")))
"""

# Injected downloader: copies "${SERVER_DIR}/<basename>" to the destination and
# appends each requested URL to ${DL_LOG}, so a test can assert what was (or was
# not) fetched. Exits 22 (curl's "HTTP error") when the asset is absent.
_DOWNLOADER_SRC = """\
#!/usr/bin/env python3
import os
import shutil
import sys

url, dest = sys.argv[1], sys.argv[2]
with open(os.environ["DL_LOG"], "a") as log:
    log.write(url + "\\n")
src = os.path.join(os.environ["SERVER_DIR"], os.path.basename(url))
if os.path.isfile(src):
    shutil.copy(src, dest)
    sys.exit(0)
sys.exit(22)
"""


def _in_ci() -> bool:
    return bool(os.environ.get("CI") or os.environ.get("GITHUB_ACTIONS"))


def _require(name: str) -> None:
    if shutil.which(name):
        return
    message = f"{name} not on PATH"
    if _in_ci():
        pytest.fail(f"{message} — provisioning regression in CI")
    pytest.skip(message)


def _sig_path(binary: Path) -> Path:
    return binary.with_name(binary.name + ".minisig")


def _sign(secret_key: Path, target: Path) -> None:
    subprocess.run(
        [
            "minisign",
            "-S",
            "-s",
            str(secret_key),
            "-m",
            str(target),
            "-x",
            str(_sig_path(target)),
        ],
        check=True,
        capture_output=True,
        text=True,
    )


def _serve_launcher(
    server: Path, alias: str, secret_key: Path, source: Path | None = None
) -> None:
    """Serve a launcher under the given target alias and sign it.

    With no `source` the stub above is written; with one, that binary is copied
    verbatim. Serving the real compiled launcher is what lets a test assert on
    launcher-rendered stdout while still traversing the genuine fetch → verify
    → cache → exec chain, with no network and no dev override.
    """
    launcher = server / f"accelerator-{alias}"
    if source is None:
        launcher.write_text(_LAUNCHER_SRC)
    else:
        shutil.copy(source, launcher)
    launcher.chmod(0o755)
    _sign(secret_key, launcher)


@pytest.fixture(scope="module")
def host_platform() -> str:
    arch = {
        "arm64": "arm64",
        "aarch64": "arm64",
        "x86_64": "x64",
        "amd64": "x64",
    }.get(platform.machine())
    system = {"Darwin": "darwin", "Linux": "linux"}.get(platform.system())
    if arch is None or system is None:
        pytest.skip(
            f"unsupported host: {platform.system()}/{platform.machine()}"
        )
    return f"{system}-{arch}"


@pytest.fixture(scope="module")
def shim_bin() -> Path:
    """Build and return the real `accelerator-verify` shim from `cli/`."""
    _require("cargo")
    subprocess.run(
        [
            "cargo",
            "build",
            "--quiet",
            "-p",
            "accelerator-verify",
            "--manifest-path",
            str(_REPO_ROOT / "cli/Cargo.toml"),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    shim = _REPO_ROOT / "cli/target/debug/accelerator-verify"
    if not (shim.exists() and os.access(shim, os.X_OK)):
        pytest.fail(f"shim not built: {shim}")
    return shim


@pytest.fixture(scope="module")
def launcher_bin() -> Path:
    """Build and return the real launcher from `cli/`.

    Built in-fixture rather than behind a `mise` build edge, mirroring
    `shim_bin`, so `uv run pytest tests/integration/entrypoint` still works
    standalone. `test:integration:entrypoint` must therefore *not* gain a
    `build:cli:dev` dependency: the two would contend on cargo's target lock.
    """
    _require("cargo")
    subprocess.run(
        [
            "cargo",
            "build",
            "--quiet",
            "--bin",
            "accelerator",
            "--manifest-path",
            str(_REPO_ROOT / "cli/Cargo.toml"),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    launcher = _REPO_ROOT / "cli/target/debug/accelerator"
    if not (launcher.exists() and os.access(launcher, os.X_OK)):
        pytest.fail(f"launcher not built: {launcher}")
    return launcher


@pytest.fixture(scope="session")
def bootstrap_src(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """A copy of `bin/accelerator`, outside the working tree.

    Naming the repo's own bootstrap as a runnable path is the trap this suite
    must not leave reachable: with self-location it resolves the real repo
    root, passes every gate, and fetches the real release over the network into
    the working tree's `bin/`.
    """
    copy = tmp_path_factory.mktemp("bootstrap") / "accelerator"
    shutil.copy(_REPO_BOOTSTRAP, copy)
    copy.chmod(0o755)
    return copy


@pytest.fixture(scope="session", autouse=True)
def repo_bin_is_untouched() -> Iterator[None]:
    """Backstop for anything that bypasses the `_run_bootstrap` funnel.

    Fires after egress rather than preventing it, which is why the funnel's own
    preconditions exist as well.
    """
    before = {entry.name for entry in _REPO_BIN.iterdir()}
    yield
    added = sorted({entry.name for entry in _REPO_BIN.iterdir()} - before)
    assert not added, f"the suite wrote into the shipped bin/: {added}"


@pytest.fixture(scope="module")
def keys(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """A dir holding passwordless release + attacker minisign keypairs."""
    _require("minisign")
    key_dir = tmp_path_factory.mktemp("keys")
    for name in ("release", "attacker"):
        subprocess.run(
            [
                "minisign",
                "-G",
                "-W",
                "-f",
                "-p",
                str(key_dir / f"{name}.pub"),
                "-s",
                str(key_dir / f"{name}.key"),
            ],
            check=True,
            capture_output=True,
            text=True,
        )
    return key_dir


@pytest.fixture
def downloader(tmp_path: Path) -> Path:
    script = tmp_path / "downloader.py"
    script.write_text(_DOWNLOADER_SRC)
    script.chmod(0o755)
    return script


@dataclass(frozen=True)
class Harness:
    root: Path
    server: Path
    sentinel: str


@pytest.fixture
def make_harness(
    request: pytest.FixtureRequest,
    tmp_path: Path,
    shim_bin: Path,
    keys: Path,
    host_platform: str,
    bootstrap_src: Path,
) -> Callable[..., Harness]:
    """Factory: build a plugin root + release server, return a `Harness`.

    The release public key is always the real one; `secret` only chooses the
    key the served launcher is *signed* with, so `secret="attacker"` models an
    asset signed by a non-release key (verification must refuse it).

    `label` names the root in its own `templates/adr.md` sentinel, so a test
    with two roots proves which one resolved from stdout rather than from the
    factory's call order. `real_launcher` serves the compiled launcher instead
    of the stub, for the tests that assert on launcher-rendered output.
    """
    counter = {"n": 0}

    def _make(
        label: str = "self",
        *,
        secret: str = "release",
        real_launcher: bool = False,
    ) -> Harness:
        counter["n"] += 1
        root = tmp_path / f"root{counter['n']}"
        (root / ".claude-plugin").mkdir(parents=True)
        (root / "keys").mkdir()
        (root / "bin").mkdir()
        (root / "templates").mkdir()
        (root / ".claude-plugin/plugin.json").write_text(
            f'{{\n  "name": "accelerator",\n  "version": "{_VERSION}"\n}}\n'
        )
        sentinel = _SENTINEL.format(label=label)
        (root / "templates/adr.md").write_text(
            f"# ADR template\n\n{sentinel}\n"
        )
        shutil.copy(keys / "release.pub", root / "keys/accelerator-release.pub")
        bootstrap = root / "bin/accelerator"
        shutil.copy(bootstrap_src, bootstrap)
        bootstrap.chmod(0o755)
        shim = root / f"bin/accelerator-verify-{host_platform}"
        shutil.copy(shim_bin, shim)
        shim.chmod(0o755)

        server = tmp_path / f"server{counter['n']}"
        server.mkdir()
        source = (
            request.getfixturevalue("launcher_bin") if real_launcher else None
        )
        _serve_launcher(
            server, host_platform, keys / f"{secret}.key", source=source
        )
        return Harness(root=root, server=server, sentinel=sentinel)

    return _make


def _assert_hermetic(env: dict[str, str], entry: Path) -> None:
    """Preconditions enforced at the single funnel every invocation passes.

    An autouse fixture cannot carry these: `_run_bootstrap` composes a complete
    explicit environment, so nothing set on `os.environ` reaches the child.
    """
    assert "ACCELERATOR_BOOTSTRAP_DOWNLOADER" in env, (
        "the bootstrap must never reach a real downloader"
    )
    base_url = env.get("ACCELERATOR_RELEASE_BASE_URL", "")
    host = urllib.parse.urlparse(base_url).hostname or ""
    assert host.endswith(".invalid"), (
        f"the release base URL must be unresolvable, got {base_url!r}"
    )
    assert entry.absolute() != _REPO_BOOTSTRAP, (
        "running the repo's own bootstrap fetches the real release into bin/"
    )
    with contextlib.suppress(OSError):
        assert entry.resolve() != _REPO_BOOTSTRAP, (
            "the entry path resolves to the repo's own bootstrap"
        )


def _run_bootstrap(
    root: Path,
    server: Path,
    downloader: Path,
    *,
    args: tuple[str, ...] = (),
    extra_env: dict[str, str] | None = None,
    path: str | None = None,
    entry: Path | None = None,
    cwd: Path | None = None,
    timeout: float | None = None,
) -> subprocess.CompletedProcess[str]:
    """Run the harness's `bin/accelerator` under a minimal, explicit env.

    Neither root variable is injected: the bootstrap self-locates from `entry`,
    which defaults to the harness root's own copy. `cwd` defaults to an empty
    directory, since the launcher's `config` family reads project config from
    the working directory.
    """
    env = {
        "PATH": path or os.environ["PATH"],
        "HOME": os.environ.get("HOME", "/tmp"),
        "ACCELERATOR_BOOTSTRAP_DOWNLOADER": str(downloader),
        "ACCELERATOR_RELEASE_BASE_URL": f"https://example.invalid/v{_VERSION}",
        "SERVER_DIR": str(server),
        "DL_LOG": str(server / "dl.log"),
    }
    if extra_env:
        env.update(extra_env)
    entry = entry or root / "bin/accelerator"
    _assert_hermetic(env, entry)
    with contextlib.ExitStack() as stack:
        if cwd is None:
            cwd = Path(stack.enter_context(tempfile.TemporaryDirectory()))
        try:
            return subprocess.run(
                ["bash", str(entry), *args],
                capture_output=True,
                text=True,
                env=env,
                cwd=str(cwd),
                timeout=timeout,
                check=False,
            )
        except subprocess.TimeoutExpired:
            pytest.fail(f"the bootstrap did not terminate: {entry}")


def _dumped_env(path: Path) -> dict[str, str]:
    return json.loads(path.read_text())


def _dl_lines(server: Path) -> list[str]:
    log = server / "dl.log"
    return log.read_text().splitlines() if log.exists() else []


@pytest.mark.parametrize(
    ("uname_s", "uname_m", "want"),
    [
        ("Darwin", "arm64", "darwin-arm64"),
        ("Darwin", "aarch64", "darwin-arm64"),
        ("Linux", "x86_64", "linux-x64"),
        ("Linux", "amd64", "linux-x64"),
    ],
)
def test_host_detection_maps_uname_to_target(
    make_harness: Callable[..., Harness],
    downloader: Path,
    shim_bin: Path,
    keys: Path,
    uname_s: str,
    uname_m: str,
    want: str,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    # Serve + verify the launcher under the *expected* alias; a wrong
    # normalisation would request an alias with no served asset (404).
    shim = root / f"bin/accelerator-verify-{want}"
    shutil.copy(shim_bin, shim)
    shim.chmod(0o755)
    _serve_launcher(server, want, keys / "release.key")

    result = _run_bootstrap(
        root,
        server,
        downloader,
        extra_env={
            "ACCELERATOR_UNAME_S": uname_s,
            "ACCELERATOR_UNAME_M": uname_m,
        },
    )
    requested = _dl_lines(server)
    assert any(line.endswith(f"accelerator-{want}") for line in requested), (
        requested,
        result.stdout + result.stderr,
    )


def test_happy_path_forwards_args_and_exit_code(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    args_out = tmp_path / "args.out"
    result = _run_bootstrap(
        root,
        server,
        downloader,
        args=("alpha", "be ta"),
        extra_env={"LAUNCHER_ARGS_OUT": str(args_out), "LAUNCHER_EXIT": "7"},
    )
    assert result.returncode == 7, result.stdout + result.stderr
    assert args_out.read_text().splitlines() == ["alpha", "be ta"]


def test_cache_hit_performs_no_further_fetch(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    _run_bootstrap(root, server, downloader)
    first = len(_dl_lines(server))
    _run_bootstrap(root, server, downloader)
    second = len(_dl_lines(server))
    assert first == second, f"cache refetched: {first} -> {second}"


def test_tampered_cached_launcher_is_refused_and_healed(
    make_harness: Callable[..., Harness],
    downloader: Path,
    host_platform: str,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    _run_bootstrap(root, server, downloader)  # populate the cache
    launcher = root / f"bin/accelerator-launcher-{_VERSION}-{host_platform}"
    launcher.write_text("poisoned")
    result = _run_bootstrap(
        root, server, downloader, extra_env={"LAUNCHER_EXIT": "0"}
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "poisoned" not in launcher.read_text()


def test_non_release_key_signature_is_refused(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness(secret="attacker")
    root, server = harness.root, harness.server
    result = _run_bootstrap(root, server, downloader)
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "verify" in output, output


def test_unrunnable_verify_shim_fails_closed(
    make_harness: Callable[..., Harness],
    downloader: Path,
    host_platform: str,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    shim = root / f"bin/accelerator-verify-{host_platform}"
    shim.write_text("not a binary")
    shim.chmod(0o755)
    result = _run_bootstrap(root, server, downloader)
    assert result.returncode != 0, result.stdout + result.stderr


def test_readonly_root_with_override_runs_from_override(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
    host_platform: str,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    bin_dir = root / "bin"
    bin_dir.chmod(0o555)  # no writes into the default cache dir
    alt = tmp_path / "alt"
    alt.mkdir()
    try:
        result = _run_bootstrap(
            root,
            server,
            downloader,
            extra_env={"ACCELERATOR_CACHE_DIR": str(alt)},
        )
        cached = alt / f"accelerator-launcher-{_VERSION}-{host_platform}"
        assert result.returncode == 0, result.stdout + result.stderr
        assert cached.exists() and os.access(cached, os.X_OK)
    finally:
        bin_dir.chmod(0o755)


def test_readonly_root_without_override_is_a_named_error(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    bin_dir = root / "bin"
    bin_dir.chmod(0o555)
    try:
        result = _run_bootstrap(root, server, downloader)
        output = result.stdout + result.stderr
        assert result.returncode != 0, output
        assert "cache directory" in output, output
    finally:
        bin_dir.chmod(0o755)


def test_stale_lock_is_reclaimed(
    make_harness: Callable[..., Harness],
    downloader: Path,
    host_platform: str,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    lock = root / f"bin/.accelerator-lock-{host_platform}"
    lock.mkdir()
    (lock / "pid").write_text("999999\n")  # a PID that is not running
    result = _run_bootstrap(root, server, downloader)
    assert result.returncode == 0, result.stdout + result.stderr


def test_path_planted_decoy_shim_is_not_used(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
) -> None:
    # Signed by the attacker key so a permissive shim found via PATH would
    # falsely pass; the absolute-path invocation must still refuse.
    harness = make_harness(secret="attacker")
    root, server = harness.root, harness.server
    decoy_dir = tmp_path / "decoy"
    decoy_dir.mkdir()
    decoy = decoy_dir / "accelerator-verify"
    decoy.write_text("#!/bin/sh\nexit 0\n")
    decoy.chmod(0o755)
    result = _run_bootstrap(
        root,
        server,
        downloader,
        path=f"{decoy_dir}:{os.environ['PATH']}",
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "verify" in output, output


# ── Phase 0: local-build override ────────────────────────────────────────────

# A slow injected downloader: sleeps ${DL_SLEEP} before copying, so a cold-cache
# lock holder stays alive long enough that concurrent waiters must extend rather
# than abort.
_SLOW_DOWNLOADER_SRC = """\
#!/usr/bin/env python3
import os
import shutil
import sys
import time

url, dest = sys.argv[1], sys.argv[2]
time.sleep(float(os.environ.get("DL_SLEEP", "1")))
with open(os.environ["DL_LOG"], "a") as log:
    log.write(url + "\\n")
src = os.path.join(os.environ["SERVER_DIR"], os.path.basename(url))
if os.path.isfile(src):
    shutil.copy(src, dest)
    sys.exit(0)
sys.exit(22)
"""


def _local_launcher(
    root: Path, *, rel: str = "cli/target/debug/accelerator"
) -> Path:
    """Write a launcher stub inside the harness root's cli/target/ tree."""
    launcher = root / rel
    launcher.parent.mkdir(parents=True, exist_ok=True)
    launcher.write_text(_LAUNCHER_SRC)
    launcher.chmod(0o755)
    return launcher


def _write_marker(root: Path) -> None:
    (root / ".accelerator-dev-launcher").write_text("")


def _source_shim_digest(root: Path, host_platform: str) -> str:
    shim = root / f"bin/accelerator-verify-{host_platform}"
    return hashlib.sha256(shim.read_bytes()).hexdigest()


@pytest.fixture
def slow_downloader(tmp_path: Path) -> Path:
    script = tmp_path / "slow_downloader.py"
    script.write_text(_SLOW_DOWNLOADER_SRC)
    script.chmod(0o755)
    return script


def test_dev_override_execs_named_binary(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    _write_marker(root)
    launcher = _local_launcher(root)
    args_out = tmp_path / "args.out"
    result = _run_bootstrap(
        root,
        server,
        downloader,
        args=("config", "get"),
        extra_env={
            "ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER": "1",
            "ACCELERATOR_LAUNCHER_BIN": str(launcher),
            "LAUNCHER_ARGS_OUT": str(args_out),
            "LAUNCHER_EXIT": "0",
        },
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert args_out.read_text().splitlines() == ["config", "get"]
    assert "WARNING" in result.stderr, result.stderr
    assert _dl_lines(server) == [], "override must perform no fetch"
    log = root / "bin/.accelerator-unverified.log"
    assert log.exists(), "override must leave a durable record"
    assert str(launcher) in log.read_text()


def test_dev_override_ignored_without_optin(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    _write_marker(root)
    launcher = _local_launcher(root)
    result = _run_bootstrap(
        root,
        server,
        downloader,
        extra_env={"ACCELERATOR_LAUNCHER_BIN": str(launcher)},
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert _dl_lines(server), "no opt-in → the verified path must fetch"
    assert "WARNING" not in result.stderr


def test_dev_override_ignored_without_marker(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    # A pristine tree (no marker) with both env vars set must still take the
    # verified path — the shape a real install ships in.
    harness = make_harness()
    root, server = harness.root, harness.server
    launcher = _local_launcher(root)
    result = _run_bootstrap(
        root,
        server,
        downloader,
        extra_env={
            "ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER": "1",
            "ACCELERATOR_LAUNCHER_BIN": str(launcher),
        },
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert _dl_lines(server), "no marker → the verified path must fetch"
    assert "WARNING" not in result.stderr


def _run_refused_override(
    make_harness: Callable[..., Harness],
    downloader: Path,
    launcher_bin: str,
) -> subprocess.CompletedProcess[str]:
    harness = make_harness()
    root, server = harness.root, harness.server
    _write_marker(root)
    return _run_bootstrap(
        root,
        server,
        downloader,
        extra_env={
            "ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER": "1",
            "ACCELERATOR_LAUNCHER_BIN": launcher_bin,
        },
    )


def test_dev_override_refused_when_symlink(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    _write_marker(root)
    real = _local_launcher(root)
    link = root / "cli/target/debug/accelerator-link"
    link.symlink_to(real)
    result = _run_bootstrap(
        root,
        server,
        downloader,
        extra_env={
            "ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER": "1",
            "ACCELERATOR_LAUNCHER_BIN": str(link),
        },
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "refused" in output, output


def test_dev_override_refused_when_not_executable(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    _write_marker(root)
    binary = root / "cli/target/debug/accelerator"
    binary.parent.mkdir(parents=True, exist_ok=True)
    binary.write_text(_LAUNCHER_SRC)
    binary.chmod(0o644)
    result = _run_bootstrap(
        root,
        server,
        downloader,
        extra_env={
            "ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER": "1",
            "ACCELERATOR_LAUNCHER_BIN": str(binary),
        },
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "refused" in output, output


def test_dev_override_refused_via_symlinked_ancestor(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    _write_marker(root)
    (root / "cli/target").mkdir(parents=True)
    outside = tmp_path / "outside"
    outside.mkdir()
    real = outside / "accelerator"
    real.write_text(_LAUNCHER_SRC)
    real.chmod(0o755)
    link = root / "cli/target/link"
    link.symlink_to(outside)
    result = _run_bootstrap(
        root,
        server,
        downloader,
        extra_env={
            "ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER": "1",
            "ACCELERATOR_LAUNCHER_BIN": str(link / "accelerator"),
        },
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "refused" in output, output


def test_dev_override_refused_outside_target(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    _write_marker(root)
    (root / "cli/target").mkdir(parents=True)
    outside = tmp_path / "outside"
    outside.mkdir()
    binary = outside / "accelerator"
    binary.write_text(_LAUNCHER_SRC)
    binary.chmod(0o755)
    result = _run_bootstrap(
        root,
        server,
        downloader,
        extra_env={
            "ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER": "1",
            "ACCELERATOR_LAUNCHER_BIN": str(binary),
        },
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "refused" in output, output


# ── Phase 0: content-addressed shim staging ──────────────────────────────────


def test_planted_staged_shim_rehashed_then_succeeds(
    make_harness: Callable[..., Harness],
    downloader: Path,
    host_platform: str,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    digest = _source_shim_digest(root, host_platform)
    planted = root / f"bin/accelerator-verify-{host_platform}-{digest}"
    planted.write_text("garbage that is not the shim")
    planted.chmod(0o755)
    result = _run_bootstrap(root, server, downloader)
    assert result.returncode == 0, result.stdout + result.stderr
    source = root / f"bin/accelerator-verify-{host_platform}"
    assert planted.read_bytes() == source.read_bytes(), "stub must be re-staged"


def test_planted_staged_shim_is_not_trusted(
    make_harness: Callable[..., Harness],
    downloader: Path,
    host_platform: str,
) -> None:
    # Launcher signed by a non-release key; a permissive stub pre-written to the
    # content-addressed staging path must be re-staged (bytes mismatch) so the
    # real shim refuses the signature rather than the stub rubber-stamping it.
    harness = make_harness(secret="attacker")
    root, server = harness.root, harness.server
    digest = _source_shim_digest(root, host_platform)
    planted = root / f"bin/accelerator-verify-{host_platform}-{digest}"
    planted.write_text("#!/bin/sh\nexit 0\n")
    planted.chmod(0o755)
    result = _run_bootstrap(root, server, downloader)
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    source = root / f"bin/accelerator-verify-{host_platform}"
    assert planted.read_bytes() == source.read_bytes(), "stub must be re-staged"


def test_planted_staged_shim_via_cache_dir_is_not_trusted(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
    host_platform: str,
) -> None:
    # A caller-chosen cache dir must not let a planted shim be trusted by path,
    # even without the opt-in: the staged bytes are still hash-checked.
    harness = make_harness(secret="attacker")
    root, server = harness.root, harness.server
    alt = tmp_path / "altcache"
    alt.mkdir()
    source = root / f"bin/accelerator-verify-{host_platform}"
    digest = _source_shim_digest(root, host_platform)
    planted = alt / f"accelerator-verify-{host_platform}-{digest}"
    planted.write_text("#!/bin/sh\nexit 0\n")
    planted.chmod(0o755)
    result = _run_bootstrap(
        root, server, downloader, extra_env={"ACCELERATOR_CACHE_DIR": str(alt)}
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert planted.read_bytes() == source.read_bytes(), "stub must be re-staged"


# ── Phase 0: lock ceiling under concurrency ──────────────────────────────────


def test_concurrent_warm_cache_all_succeed(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    _run_bootstrap(root, server, downloader)  # warm the cache
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        results = list(
            pool.map(
                lambda _: _run_bootstrap(root, server, downloader), range(8)
            )
        )
    for result in results:
        assert result.returncode == 0, result.stdout + result.stderr


def test_concurrent_cold_cache_slow_downloader_all_succeed(
    make_harness: Callable[..., Harness], slow_downloader: Path
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server  # cold cache
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        results = list(
            pool.map(
                lambda _: _run_bootstrap(
                    root, server, slow_downloader, extra_env={"DL_SLEEP": "1"}
                ),
                range(6),
            )
        )
    for result in results:
        assert result.returncode == 0, result.stdout + result.stderr
    # The lock serialised the cold fetch to exactly one bin+sig pair; a waiter
    # that aborted mid-fetch would have re-fetched.
    assert len(_dl_lines(server)) == 2, _dl_lines(server)


# ── Phase 0: the dev-launcher marker is gitignored and unshippable ───────────


def test_dev_launcher_marker_is_gitignored_and_unshipped() -> None:
    import pathspec

    gitignore = (_REPO_ROOT / ".gitignore").read_text()
    spec = pathspec.GitIgnoreSpec.from_lines(gitignore.splitlines())
    assert spec.match_file(".accelerator-dev-launcher"), (
        "the dev-launcher marker must be gitignored so no install carries it"
    )
    assert not (_REPO_ROOT / ".accelerator-dev-launcher").exists(), (
        "the marker must never be committed"
    )
    assert ".accelerator-dev-launcher" in _REPO_BOOTSTRAP.read_text(), (
        "the ignore rule must correspond to a real bootstrap gate"
    )
