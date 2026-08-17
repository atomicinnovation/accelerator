"""Regression: the launcher's resolved feature graph is rustls/ring-only.

deny.toml bans native-tls/openssl by name; this asserts the *feature* graph via
`cargo tree`: ring present / aws-lc-rs absent, hickory-resolver present by crate
name (so a feature rename back to getaddrinfo is caught), and no
native-tls/openssl, host-cert-store or C-backed zlib crate.

The tree is resolved once **per published target triple**, not for the host.
A C-backend edge introduced under a target-specific dependency table would not
appear in the host tree on either a macOS developer machine or the Linux CI
runner, so a host-only guard would pass while the musl target quietly acquired
the C dependency that breaks the fully-static cross-build — deferring detection
to the release lane. Selection only: the four-triple build remains the authority
for static linking and musl DNS.
"""

import re
import shutil
import subprocess
from functools import cache
from pathlib import Path

import pytest

from tasks.shared.targets import TARGETS

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
    # flate2's C backends. Cargo unifies features across the workspace, so any
    # crate enabling one would pull it into the launcher and break the static
    # musl cross-build.
    "libz-sys",
    "zlib-ng-sys",
    "zlib-sys",
)

_TRIPLES = tuple(triple for triple, _alias in TARGETS)


@cache
def _feature_tree(triple: str) -> str:
    if _CARGO is None:
        pytest.skip("cargo not on PATH")
    result = subprocess.run(
        [
            "cargo",
            "tree",
            "-e",
            "features",
            "-p",
            "accelerator",
            "--target",
            triple,
        ],
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


@pytest.mark.parametrize("triple", _TRIPLES)
@pytest.mark.parametrize("crate", _PRESENT)
def test_required_crate_is_selected(crate: str, triple: str) -> None:
    assert _node_present(_feature_tree(triple), crate), (
        f"{crate} missing from the launcher feature graph for {triple}"
    )


@pytest.mark.parametrize("triple", _TRIPLES)
@pytest.mark.parametrize("crate", _ABSENT)
def test_banned_or_native_crate_is_absent(crate: str, triple: str) -> None:
    assert not _node_present(_feature_tree(triple), crate), (
        f"{crate} unexpectedly present in the launcher feature graph "
        f"for {triple}"
    )
