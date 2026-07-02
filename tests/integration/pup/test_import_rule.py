"""Regression coverage for the cargo-pup architecture lane.

Needs the nightly lane, so it lives in its own directory and runs only in
check-architecture (never the test roll-up). Proves the inward-dependency
rule's discriminating power against a probe workspace laid out like the real
cli/ (pup.ron at the workspace root, domain/adapters modules in a member
crate), since the shipped cli/pup.ron rule matches `launcher::domain` — a
module the minimal scaffold does not yet contain.

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
members = ["probe"]
"""

_PROBE_MANIFEST = """\
[package]
name = "pup-probe"
version = "0.0.0"
edition = "2021"
license = "MIT"

[lib]
path = "src/lib.rs"
"""

_PROBE_LIB = "pub mod adapters;\npub mod domain;\n"
_PROBE_ADAPTERS = "pub struct Client;\n"
# domain importing an adapter — the inward-dependency violation.
_DOMAIN_VIOLATION = (
    "use crate::adapters::Client;\n\npub fn make() -> Client {\n    Client\n}\n"
)
# domain importing only its own subtree — compliant (positive control).
_DOMAIN_COMPLIANT = "pub fn make() -> u8 {\n    0\n}\n"

# Same rule SHAPE as the shipped cli/pup.ron, retargeted at the probe's module.
_PROBE_PUP_RON = """\
(
    lints: [
        Module((
            name: "domain_imports_only_permitted",
            matches: Module("^pup_probe::domain($|::)"),
            rules: [
                RestrictImports(
                    allowed_only: Some([
                        "^(std|core|alloc)(::|$)",
                        "^crate::domain(::|$)",
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


def _write_probe(root: Path, domain_body: str) -> None:
    (root / "Cargo.toml").write_text(_WORKSPACE_MANIFEST)
    (root / "pup.ron").write_text(_PROBE_PUP_RON)
    src = root / "probe/src"
    src.mkdir(parents=True, exist_ok=True)
    (root / "probe/Cargo.toml").write_text(_PROBE_MANIFEST)
    (src / "lib.rs").write_text(_PROBE_LIB)
    (src / "adapters.rs").write_text(_PROBE_ADAPTERS)
    (src / "domain.rs").write_text(domain_body)


def test_domain_importing_adapter_is_rejected(tmp_path: Path) -> None:
    _require_tools()
    _write_probe(tmp_path, _DOMAIN_VIOLATION)
    result = _pup(cwd=tmp_path)
    output = _ANSI.sub("", result.stdout + result.stderr)
    # The confirmed contract: non-zero exit AND a message naming the rule, so a
    # tool that logged-but-exited-zero would fail this test rather than pass it.
    assert result.returncode != 0, output
    assert "is not allowed" in output, output
    assert "domain_imports_only_permitted" in output, output


def test_compliant_domain_passes(tmp_path: Path) -> None:
    # Positive control: a permitted layout evaluates and passes, so a green run
    # means "evaluated and allowed", not "evaluated nothing" (a rule whose
    # module scope silently matched nothing).
    _require_tools()
    _write_probe(tmp_path, _DOMAIN_COMPLIANT)
    result = _pup(cwd=tmp_path)
    assert result.returncode == 0, _ANSI.sub("", result.stdout + result.stderr)


def test_real_cli_pup_ron_loads() -> None:
    # Guards the shipped config: print-modules parses cli/pup.ron and exits 0,
    # so a malformed RON edit fails here even though the near-vacuous rule
    # matches no module in the current scaffold.
    _require_tools()
    result = _pup("print-modules", cwd=CLI_DIR)
    assert result.returncode == 0, _ANSI.sub("", result.stdout + result.stderr)
