"""Regression: the serde-saphyr infra-out-of-domain ban must FAIL the build.

Exercises the REAL `cli/deny.toml` (via `--config`) against committed offline
fixtures, so a future edit loosening the `wrappers = ["config-adapters"]` ban is
caught automatically. Each fixture depends on a LOCAL path crate named
serde-saphyr (the ban matches on crate name), with a committed `Cargo.lock`, so
`cargo deny` runs `--frozen` (locked + offline) against a fixed graph. The
banned fixture is a package named `config` (a domain crate, not the permitted
wrapper) depending directly on serde-saphyr — it must exit non-zero and name
serde-saphyr. The clean fixture is a package named `config-adapters` (the
permitted wrapper) depending on the same stub — it must exit zero, so a pass
means "evaluated and allowed", not "evaluated nothing".
"""

import os
import shutil
import subprocess
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[2]
_REAL_DENY = _REPO_ROOT / "cli/deny.toml"
_BANNED = _HERE / "fixtures/serde-saphyr-banned/Cargo.toml"
_CLEAN = _HERE / "fixtures/serde-saphyr-clean/Cargo.toml"

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


def test_direct_serde_saphyr_dependency_fails_the_bans_check() -> None:
    _require_tools()
    result = _run_bans(_BANNED)
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "serde-saphyr" in output, output


def test_config_adapters_wrapper_passes_the_bans_check() -> None:
    _require_tools()
    result = _run_bans(_CLEAN)
    assert result.returncode == 0, result.stdout + result.stderr
