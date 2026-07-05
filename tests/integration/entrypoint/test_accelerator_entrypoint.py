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

import os
import platform
import shutil
import subprocess
from collections.abc import Callable
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[2]
_BOOTSTRAP = _REPO_ROOT / "bin/accelerator"

# The harness pins a synthetic version so cache paths are deterministic and the
# real GitHub release base URL is never contacted (overridden to .invalid).
_VERSION = "9.9.9-test"

# Stand-in for the fetched launcher binary: records its argv (one per line) to
# LAUNCHER_ARGS_OUT and exits with LAUNCHER_EXIT. Signed by minisign like a real
# release asset; its content is opaque to verification.
_LAUNCHER_SRC = """\
#!/usr/bin/env python3
import os
import sys

out = os.environ.get("LAUNCHER_ARGS_OUT")
if out:
    with open(out, "w") as handle:
        for arg in sys.argv[1:]:
            handle.write(arg + "\\n")
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


def _serve_launcher(server: Path, alias: str, secret_key: Path) -> None:
    """Write a launcher stub under the given target alias and sign it."""
    launcher = server / f"accelerator-{alias}"
    launcher.write_text(_LAUNCHER_SRC)
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


@pytest.fixture
def make_harness(
    tmp_path: Path, shim_bin: Path, keys: Path, host_platform: str
) -> Callable[..., tuple[Path, Path]]:
    """Factory: build a plugin root + release server, return (root, server).

    The release public key is always the real one; `secret` only chooses the
    key the served launcher is *signed* with, so `secret="attacker"` models an
    asset signed by a non-release key (verification must refuse it).
    """
    counter = {"n": 0}

    def _make(secret: str = "release") -> tuple[Path, Path]:
        counter["n"] += 1
        root = tmp_path / f"root{counter['n']}"
        (root / ".claude-plugin").mkdir(parents=True)
        (root / "keys").mkdir()
        (root / "bin").mkdir()
        (root / ".claude-plugin/plugin.json").write_text(
            f'{{\n  "name": "accelerator",\n  "version": "{_VERSION}"\n}}\n'
        )
        shutil.copy(keys / "release.pub", root / "keys/accelerator-release.pub")
        bootstrap = root / "bin/accelerator"
        shutil.copy(_BOOTSTRAP, bootstrap)
        bootstrap.chmod(0o755)
        shim = root / f"bin/accelerator-verify-{host_platform}"
        shutil.copy(shim_bin, shim)
        shim.chmod(0o755)

        server = tmp_path / f"server{counter['n']}"
        server.mkdir()
        _serve_launcher(server, host_platform, keys / f"{secret}.key")
        return root, server

    return _make


def _run_bootstrap(
    root: Path,
    server: Path,
    downloader: Path,
    *,
    args: tuple[str, ...] = (),
    extra_env: dict[str, str] | None = None,
    path: str | None = None,
) -> subprocess.CompletedProcess[str]:
    """Run the harness's `bin/accelerator` under a minimal, explicit env."""
    env = {
        "PATH": path or os.environ["PATH"],
        "HOME": os.environ.get("HOME", "/tmp"),
        "CLAUDE_PLUGIN_ROOT": str(root),
        "ACCELERATOR_BOOTSTRAP_DOWNLOADER": str(downloader),
        "ACCELERATOR_RELEASE_BASE_URL": f"https://example.invalid/v{_VERSION}",
        "SERVER_DIR": str(server),
        "DL_LOG": str(server / "dl.log"),
    }
    if extra_env:
        env.update(extra_env)
    return subprocess.run(
        ["bash", str(root / "bin/accelerator"), *args],
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )


def _dl_lines(server: Path) -> list[str]:
    log = server / "dl.log"
    return log.read_text().splitlines() if log.exists() else []


def test_unset_plugin_root_is_a_named_error() -> None:
    result = subprocess.run(
        ["bash", str(_BOOTSTRAP)],
        capture_output=True,
        text=True,
        check=False,
        env={"PATH": os.environ["PATH"]},
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "CLAUDE_PLUGIN_ROOT" in output, output


def test_non_directory_plugin_root_is_a_named_error(tmp_path: Path) -> None:
    not_a_dir = tmp_path / "not-a-dir"
    not_a_dir.write_text("")
    result = subprocess.run(
        ["bash", str(_BOOTSTRAP)],
        capture_output=True,
        text=True,
        check=False,
        env={"PATH": os.environ["PATH"], "CLAUDE_PLUGIN_ROOT": str(not_a_dir)},
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "not a directory" in output, output


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
    make_harness: Callable[..., tuple[Path, Path]],
    downloader: Path,
    shim_bin: Path,
    keys: Path,
    uname_s: str,
    uname_m: str,
    want: str,
) -> None:
    root, server = make_harness()
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
    make_harness: Callable[..., tuple[Path, Path]],
    downloader: Path,
    tmp_path: Path,
) -> None:
    root, server = make_harness()
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
    make_harness: Callable[..., tuple[Path, Path]], downloader: Path
) -> None:
    root, server = make_harness()
    _run_bootstrap(root, server, downloader)
    first = len(_dl_lines(server))
    _run_bootstrap(root, server, downloader)
    second = len(_dl_lines(server))
    assert first == second, f"cache refetched: {first} -> {second}"


def test_tampered_cached_launcher_is_refused_and_healed(
    make_harness: Callable[..., tuple[Path, Path]],
    downloader: Path,
    host_platform: str,
) -> None:
    root, server = make_harness()
    _run_bootstrap(root, server, downloader)  # populate the cache
    launcher = root / f"bin/accelerator-launcher-{_VERSION}-{host_platform}"
    launcher.write_text("poisoned")
    result = _run_bootstrap(
        root, server, downloader, extra_env={"LAUNCHER_EXIT": "0"}
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "poisoned" not in launcher.read_text()


def test_non_release_key_signature_is_refused(
    make_harness: Callable[..., tuple[Path, Path]], downloader: Path
) -> None:
    root, server = make_harness(secret="attacker")
    result = _run_bootstrap(root, server, downloader)
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "verify" in output, output


def test_unrunnable_verify_shim_fails_closed(
    make_harness: Callable[..., tuple[Path, Path]],
    downloader: Path,
    host_platform: str,
) -> None:
    root, server = make_harness()
    shim = root / f"bin/accelerator-verify-{host_platform}"
    shim.write_text("not a binary")
    shim.chmod(0o755)
    result = _run_bootstrap(root, server, downloader)
    assert result.returncode != 0, result.stdout + result.stderr


def test_readonly_root_with_override_runs_from_override(
    make_harness: Callable[..., tuple[Path, Path]],
    downloader: Path,
    tmp_path: Path,
    host_platform: str,
) -> None:
    root, server = make_harness()
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
    make_harness: Callable[..., tuple[Path, Path]], downloader: Path
) -> None:
    root, server = make_harness()
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
    make_harness: Callable[..., tuple[Path, Path]],
    downloader: Path,
    host_platform: str,
) -> None:
    root, server = make_harness()
    lock = root / f"bin/.accelerator-lock-{host_platform}"
    lock.mkdir()
    (lock / "pid").write_text("999999\n")  # a PID that is not running
    result = _run_bootstrap(root, server, downloader)
    assert result.returncode == 0, result.stdout + result.stderr


def test_path_planted_decoy_shim_is_not_used(
    make_harness: Callable[..., tuple[Path, Path]],
    downloader: Path,
    tmp_path: Path,
) -> None:
    # Signed by the attacker key so a permissive shim found via PATH would
    # falsely pass; the absolute-path invocation must still refuse.
    root, server = make_harness(secret="attacker")
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
