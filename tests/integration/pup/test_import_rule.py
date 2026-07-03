"""Regression coverage for the cargo-pup architecture lane.

Needs the nightly lane, so it lives in its own directory and runs only in
check-architecture (never the test roll-up). Proves the inward-dependency
rule's discriminating power against a probe workspace laid out like the real
cli/ (pup.ron at the workspace root, a version::core domain module plus an
adapters module and a shared kernel crate in member crates), mirroring the
shape of the shipped cli/pup.ron rule that matches `launcher::version::core`.

Contract confirmed against cargo-pup 0.1.8 on the pinned nightly: a
`severity: Error` RestrictImports violation compiles-errors and exits 101,
printing `... is not allowed ...` and naming the rule that applied it. The
bare CLI gates by exit code, so no assert_lints wrapper is needed.
"""

import os
import re
import shutil
import subprocess
from pathlib import Path

import pytest

from tasks.shared.rust import PUP_NIGHTLY

REPO_ROOT = Path(__file__).resolve().parents[3]
CLI_DIR = REPO_ROOT / "cli"

# cargo-pup colours its output even when piped; strip SGR escapes before
# asserting on the text.
_ANSI = re.compile(r"\x1b\[[0-9;]*m")

_CARGO = shutil.which("cargo")
_CARGO_PUP = shutil.which("cargo-pup")

_WORKSPACE_MANIFEST = """\
[workspace]
resolver = "2"
members = ["probe", "kernel"]
"""

_PROBE_MANIFEST = """\
[package]
name = "pup-probe"
version = "0.0.0"
edition = "2021"
license = "MIT"

[lib]
path = "src/lib.rs"

[dependencies]
kernel = { path = "../kernel" }
"""

_KERNEL_MANIFEST = """\
[package]
name = "kernel"
version = "0.0.0"
edition = "2021"
license = "MIT"

[lib]
path = "src/lib.rs"
"""

# A two-module kernel: the shared error taxonomy plus one infrastructure module,
# so the probe can exercise both sides of the narrowed allowance.
_KERNEL_LIB = "pub mod logging;\n\npub struct Error;\n"
_KERNEL_LOGGING = "pub fn noop() -> u8 {\n    0\n}\n"

_PROBE_LIB = "pub mod adapters;\npub mod version;\n"
_PROBE_ADAPTERS = "pub struct Client;\n"
_VERSION_MOD = "pub mod core;\n"

# core importing an adapter — the inward-dependency violation.
_CORE_ADAPTER_VIOLATION = (
    "use crate::adapters::Client;\n\npub fn make() -> Client {\n    Client\n}\n"
)
# core importing only its own subtree — compliant (positive control).
_CORE_COMPLIANT = "pub fn make() -> u8 {\n    0\n}\n"
# core importing the shared kernel error taxonomy — permitted by the narrowed
# allowance.
_CORE_KERNEL_ERROR = (
    "use kernel::Error;\n\npub fn make() -> Option<Error> {\n    None\n}\n"
)
# core importing a kernel infrastructure module — rejected: the allowance is
# narrowed to kernel::Error, not the whole kernel crate.
_CORE_KERNEL_INFRA = (
    "use kernel::logging;\n\npub fn make() -> u8 {\n    logging::noop()\n}\n"
)

# Same rule SHAPE as the shipped cli/pup.ron, retargeted at the probe's module,
# including the narrowed kernel::Error allowance.
_PROBE_PUP_RON = """\
(
    lints: [
        Module((
            name: "version_core_imports_only_permitted",
            matches: Module("^pup_probe::version::core($|::)"),
            rules: [
                RestrictImports(
                    allowed_only: Some([
                        "^(std|core|alloc)(::|$)",
                        "^kernel::Error(::|$)",
                        "^crate::version::core(::|$)",
                    ]),
                    denied: None,
                    severity: Error,
                ),
            ],
        )),
    ],
)
"""


def _in_ci() -> bool:
    return bool(os.environ.get("CI") or os.environ.get("GITHUB_ACTIONS"))


def _require_tools() -> None:
    missing = [
        name
        for name, path in (("cargo", _CARGO), ("cargo-pup", _CARGO_PUP))
        if path is None
    ]
    if not missing:
        return
    message = f"tools not on PATH: {', '.join(missing)}"
    if _in_ci():
        pytest.fail(f"{message} — pup provisioning regression in CI")
    pytest.skip(message)


def _pup(*args: str, cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["cargo", f"+{PUP_NIGHTLY}", "pup", *args],
        cwd=cwd,
        capture_output=True,
        text=True,
        check=False,
    )


def _write_probe(root: Path, core_body: str) -> None:
    (root / "Cargo.toml").write_text(_WORKSPACE_MANIFEST)
    (root / "pup.ron").write_text(_PROBE_PUP_RON)

    probe_src = root / "probe/src"
    (probe_src / "version").mkdir(parents=True, exist_ok=True)
    (root / "probe/Cargo.toml").write_text(_PROBE_MANIFEST)
    (probe_src / "lib.rs").write_text(_PROBE_LIB)
    (probe_src / "adapters.rs").write_text(_PROBE_ADAPTERS)
    (probe_src / "version/mod.rs").write_text(_VERSION_MOD)
    (probe_src / "version/core.rs").write_text(core_body)

    kernel_src = root / "kernel/src"
    kernel_src.mkdir(parents=True, exist_ok=True)
    (root / "kernel/Cargo.toml").write_text(_KERNEL_MANIFEST)
    (kernel_src / "lib.rs").write_text(_KERNEL_LIB)
    (kernel_src / "logging.rs").write_text(_KERNEL_LOGGING)


def test_core_importing_adapter_is_rejected(tmp_path: Path) -> None:
    _require_tools()
    _write_probe(tmp_path, _CORE_ADAPTER_VIOLATION)
    result = _pup(cwd=tmp_path)
    output = _ANSI.sub("", result.stdout + result.stderr)
    # The confirmed contract: non-zero exit AND a message naming the rule, so a
    # tool that logged-but-exited-zero would fail this test rather than pass it.
    assert result.returncode != 0, output
    assert "is not allowed" in output, output
    assert "version_core_imports_only_permitted" in output, output


def test_compliant_core_passes(tmp_path: Path) -> None:
    # Positive control: a permitted layout evaluates and passes, so a green run
    # means "evaluated and allowed", not "evaluated nothing" (a rule whose
    # module scope silently matched nothing).
    _require_tools()
    _write_probe(tmp_path, _CORE_COMPLIANT)
    result = _pup(cwd=tmp_path)
    assert result.returncode == 0, _ANSI.sub("", result.stdout + result.stderr)


def test_core_importing_kernel_error_passes(tmp_path: Path) -> None:
    # The narrowed allowance permits the shared error taxonomy.
    _require_tools()
    _write_probe(tmp_path, _CORE_KERNEL_ERROR)
    result = _pup(cwd=tmp_path)
    assert result.returncode == 0, _ANSI.sub("", result.stdout + result.stderr)


def test_core_importing_kernel_infra_is_rejected(tmp_path: Path) -> None:
    # The narrowing bites: kernel::logging is infrastructure, not the taxonomy.
    # A whole-kernel allowance would pass this; the scoped one rejects it.
    _require_tools()
    _write_probe(tmp_path, _CORE_KERNEL_INFRA)
    result = _pup(cwd=tmp_path)
    output = _ANSI.sub("", result.stdout + result.stderr)
    assert result.returncode != 0, output
    assert "is not allowed" in output, output
    assert "version_core_imports_only_permitted" in output, output


def test_real_cli_pup_ron_loads() -> None:
    # Guards the shipped config: print-modules parses cli/pup.ron and exits 0,
    # so a malformed RON edit fails here.
    _require_tools()
    result = _pup("print-modules", cwd=CLI_DIR)
    assert result.returncode == 0, _ANSI.sub("", result.stdout + result.stderr)
