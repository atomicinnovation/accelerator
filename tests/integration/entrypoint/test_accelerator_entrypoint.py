import concurrent.futures
import hashlib
import os
import platform
import shutil
import subprocess
from collections.abc import Callable, Iterator
from pathlib import Path

import pytest

from tests.integration.support.installation import (
    BASH as _BASH,
)
from tests.integration.support.installation import (
    LAUNCHER_STUB_SRC as _LAUNCHER_SRC,
)
from tests.integration.support.installation import (
    REPO_BIN as _REPO_BIN,
)
from tests.integration.support.installation import (
    REPO_BOOTSTRAP as _REPO_BOOTSTRAP,
)
from tests.integration.support.installation import (
    REPO_ROOT as _REPO_ROOT,
)
from tests.integration.support.installation import (
    VERSION as _VERSION,
)
from tests.integration.support.installation import (
    Installation as Harness,
)
from tests.integration.support.installation import (
    build_launcher,
    build_shim,
    copy_bootstrap,
    download_log,
    dumped_env,
    generate_keys,
    make_installation,
    run_bootstrap,
    serve_launcher,
    write_downloader,
)
from tests.integration.support.installation import (
    host_platform as _resolve_host_platform,
)

_HERE = Path(__file__).resolve().parent

_serve_launcher = serve_launcher
_run_bootstrap = run_bootstrap
_dumped_env = dumped_env
_dl_lines = download_log


@pytest.fixture(scope="module")
def host_platform() -> str:
    return _resolve_host_platform()


@pytest.fixture(scope="module")
def shim_bin() -> Path:
    return build_shim()


@pytest.fixture(scope="module")
def launcher_bin() -> Path:
    return build_launcher()


@pytest.fixture(scope="session")
def bootstrap_src(tmp_path_factory: pytest.TempPathFactory) -> Path:
    return copy_bootstrap(tmp_path_factory.mktemp("bootstrap") / "accelerator")


@pytest.fixture(scope="session", autouse=True)
def repo_bin_is_untouched() -> Iterator[None]:
    """Backstop for anything that bypasses the `run_bootstrap` funnel.

    Fires after egress rather than preventing it, which is why the funnel's own
    preconditions exist as well.
    """
    before = {entry.name for entry in _REPO_BIN.iterdir()}
    yield
    added = sorted({entry.name for entry in _REPO_BIN.iterdir()} - before)
    assert not added, f"the suite wrote into the shipped bin/: {added}"


@pytest.fixture(scope="module")
def keys(tmp_path_factory: pytest.TempPathFactory) -> Path:
    return generate_keys(tmp_path_factory.mktemp("keys"))


@pytest.fixture
def downloader(tmp_path: Path) -> Path:
    return write_downloader(tmp_path / "downloader.py")


@pytest.fixture
def make_harness(
    request: pytest.FixtureRequest,
    tmp_path: Path,
    shim_bin: Path,
    keys: Path,
    host_platform: str,
    bootstrap_src: Path,
) -> Callable[..., Harness]:
    """Factory: build a plugin root + release server, return an installation."""
    counter = {"n": 0}

    def _make(
        label: str = "self",
        *,
        secret: str = "release",
        real_launcher: bool = False,
    ) -> Harness:
        counter["n"] += 1
        source = (
            request.getfixturevalue("launcher_bin") if real_launcher else None
        )
        return make_installation(
            tmp_path / f"root{counter['n']}",
            tmp_path / f"server{counter['n']}",
            keys=keys,
            shim=shim_bin,
            bootstrap=bootstrap_src,
            alias=host_platform,
            label=label,
            secret=secret,
            launcher_source=source,
        )

    return _make


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


# ── Self-location ────────────────────────────────────────────────────────────


def test_the_suite_runs_the_bootstrap_on_the_bash_floor() -> None:
    if platform.system() != "Darwin":
        pytest.skip("the 3.2 floor exists because macOS ships 3.2")
    result = subprocess.run(
        [_BASH, "-c", 'printf "%s" "${BASH_VERSINFO[0]}"'],
        capture_output=True,
        text=True,
        check=True,
    )
    assert result.stdout == "3", (
        f"{_BASH} is bash {result.stdout}; the bootstrap is held to 3.2"
    )


def _run_and_capture_env(
    harness: Harness,
    downloader: Path,
    tmp_path: Path,
    **kwargs: object,
) -> tuple[subprocess.CompletedProcess[str], dict[str, str]]:
    """Run the stub launcher and return its run plus its dumped environment.

    Asserting the export directly beats inferring it from rendered output: it
    is cheaper, and it fails at the layer that actually broke.
    """
    env_out = tmp_path / f"{harness.root.name}-env.json"
    extra = dict(kwargs.pop("extra_env", None) or {})  # type: ignore[arg-type]
    extra["LAUNCHER_ENV_OUT"] = str(env_out)
    result = _run_bootstrap(
        harness.root,
        harness.server,
        downloader,
        extra_env=extra,
        **kwargs,  # type: ignore[arg-type]
    )
    assert env_out.exists(), result.stdout + result.stderr
    return result, _dumped_env(env_out)


def _link_chain(directory: Path, target: Path, length: int) -> Path:
    """Build `length` chained symlinks and return the outermost."""
    directory.mkdir(parents=True, exist_ok=True)
    current = target
    for index in range(length):
        link = directory / f"link{index}"
        link.symlink_to(current)
        current = link
    return current


def _bare_root(tmp_path: Path, bootstrap_src: Path) -> Path:
    """An installation-shaped directory that carries no plugin.json."""
    root = tmp_path / "bare"
    (root / "bin").mkdir(parents=True)
    entry = root / "bin/accelerator"
    shutil.copy(bootstrap_src, entry)
    entry.chmod(0o755)
    return root


def test_rootless_template_render_resolves_the_fixture_root(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness(real_launcher=True)
    result = _run_bootstrap(
        harness.root,
        harness.server,
        downloader,
        args=("config", "template", "adr"),
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert harness.sentinel in result.stdout, result.stdout
    assert "accelerator:" not in result.stderr, result.stderr


def test_rootless_instructions_command_degrades_cleanly(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness(real_launcher=True)
    result = _run_bootstrap(
        harness.root,
        harness.server,
        downloader,
        args=("config", "instructions", "commit", "--fail-safe"),
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "accelerator:" not in result.stderr, result.stderr


def test_rootless_templates_list_carries_the_plugin_default_row(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness(real_launcher=True)
    result = _run_bootstrap(
        harness.root,
        harness.server,
        downloader,
        args=("config", "templates", "list"),
    )
    assert result.returncode == 0, result.stdout + result.stderr
    rows = [line for line in result.stdout.splitlines() if "adr" in line]
    assert rows, result.stdout
    assert "plugin default" in rows[0], rows[0]


def test_two_hop_symlink_chain_resolves_to_the_fixture_root(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    harness = make_harness()
    plugin_data = tmp_path / "plugin-data/bin"
    plugin_data.mkdir(parents=True)
    channel = plugin_data / "accelerator"
    channel.symlink_to(harness.root / "bin/accelerator")
    userbin = tmp_path / "userbin"
    userbin.mkdir()
    entry = userbin / "accelerator"
    entry.symlink_to(channel)

    _, dumped = _run_and_capture_env(harness, downloader, tmp_path, entry=entry)
    assert dumped["ACCELERATOR_PLUGIN_ROOT"] == str(harness.root)


def test_a_directory_symlink_in_the_entry_path_resolves_physically(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    # A logical `cd ..` collapses the component textually and yields
    # <tmp>/other; only `cd -P` reaches the fixture root.
    harness = make_harness()
    other = tmp_path / "other"
    other.mkdir()
    (other / "bin").symlink_to(harness.root / "bin", target_is_directory=True)

    _, dumped = _run_and_capture_env(
        harness, downloader, tmp_path, entry=other / "bin/accelerator"
    )
    assert dumped["ACCELERATOR_PLUGIN_ROOT"] == str(harness.root)


@pytest.mark.parametrize("inject", ["old", "new", "both"])
def test_ambient_roots_never_redirect_the_resolved_root(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
    inject: str,
) -> None:
    harness = make_harness("self")
    decoy = make_harness("injected")
    names = {
        "old": ["CLAUDE_PLUGIN_ROOT"],
        "new": ["ACCELERATOR_PLUGIN_ROOT"],
        "both": ["CLAUDE_PLUGIN_ROOT", "ACCELERATOR_PLUGIN_ROOT"],
    }[inject]

    _, dumped = _run_and_capture_env(
        harness,
        downloader,
        tmp_path,
        extra_env={name: str(decoy.root) for name in names},
    )
    assert dumped["ACCELERATOR_PLUGIN_ROOT"] == str(harness.root)


def test_relative_symlink_target_resolves(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    harness = make_harness()
    decoy = tmp_path / "decoy"
    (decoy / ".claude-plugin").mkdir(parents=True)
    (decoy / ".claude-plugin/plugin.json").write_text(
        f'{{"name": "accelerator", "version": "{_VERSION}"}}\n'
    )
    links = tmp_path / "links"
    links.mkdir()
    entry = links / "accelerator"
    entry.symlink_to(Path("..") / harness.root.name / "bin/accelerator")

    _, dumped = _run_and_capture_env(
        harness, downloader, tmp_path, entry=entry, cwd=decoy
    )
    assert dumped["ACCELERATOR_PLUGIN_ROOT"] == str(harness.root)
    assert dumped["ACCELERATOR_PLUGIN_ROOT"] != str(decoy)


def test_an_exported_cdpath_does_not_redirect_the_resolved_root(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    # CDPATH is consulted only for a relative `cd`, so the invocation has to be
    # relative for the hazard to exist at all. On a match `cd` also prints the
    # directory it chose — inside the command substitution, which would make
    # the resolved root two lines.
    harness = make_harness()
    decoy_parent = tmp_path / "cdpath-decoy"
    (decoy_parent / "bin").mkdir(parents=True)

    _, dumped = _run_and_capture_env(
        harness,
        downloader,
        tmp_path,
        entry=Path("bin/accelerator"),
        cwd=harness.root,
        extra_env={"CDPATH": str(decoy_parent)},
    )
    assert dumped["ACCELERATOR_PLUGIN_ROOT"] == str(harness.root)


def test_a_leading_dash_link_target_resolves(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    # readlink output is taken verbatim, so a target whose first component
    # begins with `-` must never reach a command that would read it as an
    # option.
    harness = make_harness()
    links = tmp_path / "dash"
    links.mkdir()
    (links / "-x").symlink_to(harness.root / "bin", target_is_directory=True)
    entry = links / "accelerator"
    entry.symlink_to(Path("-x/accelerator"))

    _, dumped = _run_and_capture_env(harness, downloader, tmp_path, entry=entry)
    assert dumped["ACCELERATOR_PLUGIN_ROOT"] == str(harness.root)


def test_a_sixteen_link_chain_resolves(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    # The at-boundary partner of the seventeen-link case: without it, relaxing
    # the bound's comparison reddens nothing.
    harness = make_harness()
    entry = _link_chain(
        tmp_path / "chain16", harness.root / "bin/accelerator", 16
    )

    _, dumped = _run_and_capture_env(harness, downloader, tmp_path, entry=entry)
    assert dumped["ACCELERATOR_PLUGIN_ROOT"] == str(harness.root)


def test_a_seventeen_link_chain_exceeds_the_hop_bound(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    harness = make_harness()
    entry = _link_chain(
        tmp_path / "chain17", harness.root / "bin/accelerator", 17
    )
    result = _run_bootstrap(
        harness.root, harness.server, downloader, entry=entry
    )
    assert result.returncode != 0, result.stdout + result.stderr
    assert "exceeded 16 hops" in result.stderr, result.stderr


def test_a_symlink_cycle_terminates_rather_than_hanging(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    # Characterises the kernel's ELOOP at open(2), not the in-script counter:
    # with a true cycle bash never reads a byte of the script, so this passes
    # even with the chase removed. The timeout is what makes a hang detectable.
    harness = make_harness()
    cycle = tmp_path / "cycle"
    cycle.mkdir()
    (cycle / "a").symlink_to(cycle / "b")
    (cycle / "b").symlink_to(cycle / "a")
    result = _run_bootstrap(
        harness.root,
        harness.server,
        downloader,
        entry=cycle / "a",
        timeout=30,
    )
    assert result.returncode != 0, result.stdout + result.stderr


def test_a_derived_root_that_is_not_an_installation_aborts_by_name(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
    bootstrap_src: Path,
) -> None:
    # Naming the derived path also pins that self-location ran, rather than
    # something else having failed earlier.
    harness = make_harness()
    bare = _bare_root(tmp_path, bootstrap_src)
    entry = bare / "bin/accelerator"

    result = _run_bootstrap(
        harness.root, harness.server, downloader, entry=entry
    )
    assert result.returncode != 0, result.stdout + result.stderr
    assert "plugin.json not found" in result.stderr, result.stderr
    assert str(bare) in result.stderr, result.stderr

    degraded = _run_bootstrap(
        harness.root,
        harness.server,
        downloader,
        args=("config", "templates", "list", "--fail-safe"),
        entry=entry,
    )
    assert degraded.returncode == 0, degraded.stdout + degraded.stderr
    assert degraded.stdout == "", degraded.stdout
    assert "plugin.json not found" in degraded.stderr, degraded.stderr


@pytest.mark.parametrize(
    "args",
    [
        ("config", "get", "k", "--", "--fail-safe"),
        ("config", "get", "k"),
    ],
)
def test_fail_safe_is_not_honoured_outside_its_scan_window(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
    bootstrap_src: Path,
    args: tuple[str, ...],
) -> None:
    harness = make_harness()
    bare = _bare_root(tmp_path, bootstrap_src)
    result = _run_bootstrap(
        harness.root,
        harness.server,
        downloader,
        args=args,
        entry=bare / "bin/accelerator",
    )
    assert result.returncode != 0, result.stdout + result.stderr


def test_trust_chain_failure_records_durably_under_fail_safe(
    make_harness: Callable[..., Harness],
    downloader: Path,
    host_platform: str,
) -> None:
    # Nothing unverified is ever exec'd either way, so the record buys
    # detectability: without it a trust-chain abort is byte-identical to the
    # many commands that legitimately emit nothing.
    harness = make_harness()
    (harness.root / f"bin/accelerator-verify-{host_platform}").unlink()
    result = _run_bootstrap(
        harness.root,
        harness.server,
        downloader,
        args=("config", "templates", "list", "--fail-safe"),
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert result.stdout == "", result.stdout
    log = harness.root / "bin/.accelerator-unverified.log"
    assert log.exists(), result.stderr
    assert "verify shim missing" in log.read_text().splitlines()[-1]


def test_a_record_is_always_one_line(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
    host_platform: str,
) -> None:
    # A cache dir carrying a newline reaches the staging diagnostic verbatim,
    # so the write-site sanitiser — not any input check — is what keeps the log
    # parseable.
    harness = make_harness()
    cache_dir = tmp_path / "cache\nwith-newline"
    cache_dir.mkdir()
    digest = _source_shim_digest(harness.root, host_platform)
    blocked = cache_dir / f"accelerator-verify-{host_platform}-{digest}"
    blocked.mkdir()
    blocked.chmod(0o555)
    try:
        result = _run_bootstrap(
            harness.root,
            harness.server,
            downloader,
            args=("config", "templates", "list", "--fail-safe"),
            extra_env={"ACCELERATOR_CACHE_DIR": str(cache_dir)},
        )
    finally:
        blocked.chmod(0o755)
    assert result.returncode == 0, result.stdout + result.stderr
    log = cache_dir / ".accelerator-unverified.log"
    assert log.exists(), result.stderr
    assert len(log.read_text().splitlines()) == 1, log.read_text()
