"""Regression: the pulldown-cmark ban must FAIL the build outside `document`.

Exercises the REAL `cli/deny.toml` (via `--config`) against committed offline
fixtures, so a future edit loosening the `wrappers = ["document"]` ban is
caught automatically. Each fixture depends on a LOCAL path crate named
pulldown-cmark (the ban matches on crate name), with a committed `Cargo.lock`,
so `cargo deny` runs `--frozen` (locked + offline) against a fixed graph. The
banned fixture is a package named `corpus` (a domain crate, not the permitted
wrapper) depending directly on pulldown-cmark — it must exit non-zero and name
pulldown-cmark. The clean fixture is a package named `document` (the permitted
wrapper) depending on the same stub — it must exit zero, so a pass means
"evaluated and allowed", not "evaluated nothing".
"""

import os
import shutil
import subprocess
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[2]
_REAL_DENY = _REPO_ROOT / "cli/deny.toml"
_BANNED = _HERE / "fixtures/pulldown-cmark-banned/Cargo.toml"
_CLEAN = _HERE / "fixtures/pulldown-cmark-clean/Cargo.toml"

_CARGO = shutil.which("cargo")
_CARGO_DENY = shutil.which("cargo-deny")


def _in_ci() -> bool:
    return bool(os.environ.get("CI") or os.environ.get("GITHUB_ACTIONS"))


def _require_tools() -> None:
    missing = [
        name
        for name, path in (("cargo", _CARGO), ("cargo-deny", _CARGO_DENY))
        if path is None
    ]
    if not missing:
        return
    message = f"tools not on PATH: {', '.join(missing)}"
    if _in_ci():
        pytest.fail(f"{message} — provisioning regression in CI")
    pytest.skip(message)


def _run_bans(manifest: Path) -> subprocess.CompletedProcess[str]:
    # --frozen == --locked + --offline: proves the committed fixture Cargo.lock
    # drives a fixed graph with no network. CARGO_NET_OFFLINE belt-and-braces.
    return subprocess.run(
        [
            "cargo",
            "deny",
            "--manifest-path",
            str(manifest),
            "--frozen",
            "check",
            "bans",
            "--config",
            str(_REAL_DENY),
        ],
        capture_output=True,
        text=True,
        check=False,
        env={**os.environ, "CARGO_NET_OFFLINE": "true"},
    )


def test_a_direct_pulldown_cmark_dependency_outside_document_is_banned() -> (
    None
):
    _require_tools()
    result = _run_bans(_BANNED)
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "pulldown-cmark" in output, output


def test_document_may_depend_on_pulldown_cmark() -> None:
    _require_tools()
    result = _run_bans(_CLEAN)
    assert result.returncode == 0, result.stdout + result.stderr
