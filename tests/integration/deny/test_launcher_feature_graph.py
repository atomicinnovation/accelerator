"""Regression: the launcher's resolved feature graph is rustls/ring-only.

deny.toml bans native-tls/openssl by name; this asserts the *feature* graph via
`cargo tree`: ring present / aws-lc-rs absent, hickory-resolver present by crate
name (so a feature rename back to getaddrinfo is caught), and no
native-tls/openssl or host-cert-store crate. Selection only — the four-triple
build is the authority for static linking and musl DNS.
"""

import re
import shutil
import subprocess
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[2]
_CLI = _REPO_ROOT / "cli"

_CARGO = shutil.which("cargo")

_PRESENT = ("ring", "hickory-resolver", "rustls", "reqwest", "webpki-roots")
_ABSENT = (
    "aws-lc-rs",
    "native-tls",
    "openssl",
    "openssl-sys",
    "rustls-native-certs",
    "security-framework",
)


def _feature_tree() -> str:
    if _CARGO is None:
        pytest.skip("cargo not on PATH")
    result = subprocess.run(
        ["cargo", "tree", "-e", "features", "-p", "accelerator"],
        cwd=_CLI,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    return result.stdout


def _node_present(tree: str, crate: str) -> bool:
    # A node renders as "<crate> v<version>"; the lookbehind stops `openssl`
    # matching `openssl-probe`, etc.
    return re.search(rf"(?<![\w-]){re.escape(crate)} v\d", tree) is not None


@pytest.mark.parametrize("crate", _PRESENT)
def test_required_crate_is_selected(crate: str) -> None:
    assert _node_present(_feature_tree(), crate), (
        f"{crate} missing from the launcher feature graph"
    )


@pytest.mark.parametrize("crate", _ABSENT)
def test_banned_or_native_crate_is_absent(crate: str) -> None:
    assert not _node_present(_feature_tree(), crate), (
        f"{crate} unexpectedly present in the launcher feature graph"
    )
