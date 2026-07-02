"""Regression: the native-tls/OpenSSL ban must FAIL the build, not warn.

Exercises the REAL `cli/deny.toml` (via `--config`) against committed offline
fixtures, so a future edit loosening `[bans] deny` is caught automatically. The
fixtures depend on LOCAL path crates named `native-tls`/`openssl` (the ban
matches on crate name), each with a committed `Cargo.lock`, so `cargo deny`
runs `--frozen` (locked + offline) against a fixed graph — no crates.io
resolution on the ubuntu+macos matrix. The banned fixture must exit non-zero
and name both crates; the clean fixture (a permitted `rustls` path dep present)
must exit zero, so a pass means "evaluated and allowed", not "evaluated
nothing".
"""

import os
import shutil
import subprocess
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[2]
_REAL_DENY = _REPO_ROOT / "cli/deny.toml"
_BANNED = _HERE / "fixtures/banned/Cargo.toml"
_CLEAN = _HERE / "fixtures/clean/Cargo.toml"

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


def test_native_tls_and_openssl_dependency_fails_the_bans_check() -> None:
    _require_tools()
    result = _run_bans(_BANNED)
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "native-tls" in output, output
    assert "openssl" in output, output


def test_clean_fixture_passes_the_bans_check() -> None:
    _require_tools()
    result = _run_bans(_CLEAN)
    assert result.returncode == 0, result.stdout + result.stderr
