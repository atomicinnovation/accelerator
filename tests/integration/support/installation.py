"""Shared fixture-installation apparatus for suites that run the bootstrap.

Building a runnable installation means reproducing the whole trust chain: a
`.claude-plugin/plugin.json`, a verify shim named for the running host, a
generated release keypair, a signed launcher served through a stubbed
downloader, and a writable cache. Encoding that twice would give two
independent copies of the release asset-naming and signing contract, which
would drift the next time `tasks/shared/targets.py` or the shim naming changes.

None of it is committed. The shim is platform-specific and would skip or fail
one CI leg; the keypair is generated per run with `*.key` gitignored, so a
committed public key could never match a signature produced here; and a
committed shim would put a second, unguarded copy of a trust-root binary under
`tests/`.

`run_bootstrap` is the single funnel every invocation passes, and it carries the
network and working-tree preconditions — a caller cannot opt out of them by
constructing its own `subprocess.run`.
"""

import contextlib
import json
import os
import platform
import shutil
import subprocess
import tempfile
import urllib.parse
from dataclasses import dataclass
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
REPO_BOOTSTRAP = REPO_ROOT / "bin/accelerator"
REPO_BIN = REPO_ROOT / "bin"

# The system interpreter, not whatever `bash` resolves to: macOS ships 3.2 and
# the bootstrap is held to that floor, so a contributor with Homebrew bash 5 on
# PATH would otherwise run a suite off-floor and green.
BASH = "/bin/bash" if Path("/bin/bash").exists() else "bash"

# A synthetic version, so cache paths are deterministic and the real GitHub
# release base URL is never contacted (overridden to .invalid).
VERSION = "9.9.9-test"

# Marks a fixture root's own templates/adr.md, so a resolved installation root
# is provable from the launcher's stdout rather than from a rendered path.
SENTINEL = "FIXTURE-ADR-SENTINEL-{label}"

# Stand-in for the fetched launcher binary: records its argv (one per line) to
# LAUNCHER_ARGS_OUT, its whole environment as JSON to LAUNCHER_ENV_OUT, and
# exits with LAUNCHER_EXIT. Signed by minisign like a real release asset; its
# content is opaque to verification.
LAUNCHER_STUB_SRC = """\
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
DOWNLOADER_SRC = """\
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


def in_ci() -> bool:
    return bool(os.environ.get("CI") or os.environ.get("GITHUB_ACTIONS"))


def require(name: str) -> None:
    """Skip locally, fail in CI: a missing tool there is a provisioning bug."""
    if shutil.which(name):
        return
    message = f"{name} not on PATH"
    if in_ci():
        pytest.fail(f"{message} — provisioning regression in CI")
    pytest.skip(message)


def host_platform() -> str:
    """The running host's release-asset alias, e.g. ``darwin-arm64``."""
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


def _cargo_build(package: str, binary: str) -> Path:
    require("cargo")
    subprocess.run(
        [
            "cargo",
            "build",
            "--quiet",
            *package.split(),
            "--manifest-path",
            str(REPO_ROOT / "cli/Cargo.toml"),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    built = REPO_ROOT / "cli/target/debug" / binary
    if not (built.exists() and os.access(built, os.X_OK)):
        pytest.fail(f"not built: {built}")
    return built


def build_shim() -> Path:
    """Build and return the real `accelerator-verify` shim from `cli/`."""
    return _cargo_build("-p accelerator-verify", "accelerator-verify")


def build_launcher() -> Path:
    """Build and return the real launcher from `cli/`.

    Built here rather than behind a `mise` build edge so a suite using it still
    runs standalone under a bare `uv run pytest`. A suite that calls this must
    therefore *not* also gain a `build:cli:dev` dependency: the two would
    contend on cargo's target lock and the asserted edge would be inert.
    """
    return _cargo_build("--bin accelerator", "accelerator")


def generate_keys(key_dir: Path) -> Path:
    """Generate passwordless release + attacker minisign keypairs."""
    require("minisign")
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


def write_downloader(path: Path) -> Path:
    path.write_text(DOWNLOADER_SRC)
    path.chmod(0o755)
    return path


def copy_bootstrap(destination: Path) -> Path:
    """Copy `bin/accelerator` to a runnable path outside the working tree.

    Naming the repo's own bootstrap as a runnable path is the trap these suites
    must not leave reachable: with self-location it resolves the real repo root,
    passes every gate, and fetches the real release over the network into the
    working tree's `bin/`.
    """
    shutil.copy(REPO_BOOTSTRAP, destination)
    destination.chmod(0o755)
    return destination


def sig_path(binary: Path) -> Path:
    return binary.with_name(binary.name + ".minisig")


def sign(secret_key: Path, target: Path) -> None:
    subprocess.run(
        [
            "minisign",
            "-S",
            "-s",
            str(secret_key),
            "-m",
            str(target),
            "-x",
            str(sig_path(target)),
        ],
        check=True,
        capture_output=True,
        text=True,
    )


def serve_launcher(
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
        launcher.write_text(LAUNCHER_STUB_SRC)
    else:
        shutil.copy(source, launcher)
    launcher.chmod(0o755)
    sign(secret_key, launcher)


@dataclass(frozen=True)
class Installation:
    root: Path
    server: Path
    sentinel: str


def make_installation(
    root: Path,
    server: Path,
    *,
    keys: Path,
    shim: Path,
    bootstrap: Path,
    alias: str,
    label: str = "self",
    secret: str = "release",
    launcher_source: Path | None = None,
    templates: Path | None = None,
) -> Installation:
    """Build a plugin root plus its stub release server.

    The release public key placed in the root is always the real one; `secret`
    only chooses the key the served launcher is *signed* with, so
    `secret="attacker"` models an asset signed by a non-release key.

    `label` names the root in its own `templates/adr.md` sentinel, so a caller
    with two roots proves which one resolved from stdout rather than from
    construction order.

    `templates` copies a whole template tree in first, for callers that need
    the plugin-default tier to resolve by name. The sentinel `adr.md` is
    written afterwards either way, so it always wins.
    """
    (root / ".claude-plugin").mkdir(parents=True)
    (root / "keys").mkdir()
    (root / "bin").mkdir()
    if templates is None:
        (root / "templates").mkdir()
    else:
        shutil.copytree(templates, root / "templates")
    (root / ".claude-plugin/plugin.json").write_text(
        f'{{\n  "name": "accelerator",\n  "version": "{VERSION}"\n}}\n'
    )
    sentinel = SENTINEL.format(label=label)
    (root / "templates/adr.md").write_text(f"# ADR template\n\n{sentinel}\n")
    shutil.copy(keys / "release.pub", root / "keys/accelerator-release.pub")
    copy_bootstrap(root / "bin/accelerator")
    installed_shim = root / f"bin/accelerator-verify-{alias}"
    shutil.copy(shim, installed_shim)
    installed_shim.chmod(0o755)

    server.mkdir(exist_ok=True)
    serve_launcher(
        server, alias, keys / f"{secret}.key", source=launcher_source
    )
    return Installation(root=root, server=server, sentinel=sentinel)


def assert_hermetic(env: dict[str, str], entry: Path) -> None:
    """Preconditions enforced at the single funnel every invocation passes.

    An autouse fixture cannot carry these: `run_bootstrap` composes a complete
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
    assert entry.absolute() != REPO_BOOTSTRAP, (
        "running the repo's own bootstrap fetches the real release into bin/"
    )
    with contextlib.suppress(OSError):
        assert entry.resolve() != REPO_BOOTSTRAP, (
            "the entry path resolves to the repo's own bootstrap"
        )


def run_bootstrap(
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
    """Run an installation's `bin/accelerator` under a minimal, explicit env.

    Neither root variable is injected: the bootstrap self-locates from `entry`,
    which defaults to the installation root's own copy. `cwd` defaults to an
    empty directory, since the launcher's `config` family reads project config
    from the working directory.
    """
    env = {
        "PATH": path or os.environ["PATH"],
        "HOME": os.environ.get("HOME", "/tmp"),
        "ACCELERATOR_BOOTSTRAP_DOWNLOADER": str(downloader),
        "ACCELERATOR_RELEASE_BASE_URL": f"https://example.invalid/v{VERSION}",
        "SERVER_DIR": str(server),
        "DL_LOG": str(server / "dl.log"),
    }
    if extra_env:
        env.update(extra_env)
    entry = entry or root / "bin/accelerator"
    with contextlib.ExitStack() as stack:
        if cwd is None:
            cwd = Path(stack.enter_context(tempfile.TemporaryDirectory()))
        assert_hermetic(env, cwd / entry)
        try:
            return subprocess.run(
                [BASH, str(entry), *args],
                capture_output=True,
                text=True,
                env=env,
                cwd=str(cwd),
                timeout=timeout,
                check=False,
            )
        except subprocess.TimeoutExpired:
            pytest.fail(f"the bootstrap did not terminate: {entry}")


def dumped_env(path: Path) -> dict[str, str]:
    return json.loads(path.read_text())


def download_log(server: Path) -> list[str]:
    log = server / "dl.log"
    return log.read_text().splitlines() if log.exists() else []
