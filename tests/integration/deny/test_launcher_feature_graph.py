"""Regression: the launcher's resolved feature graph is rustls/ring-only.

`cli/deny.toml` bans native-tls/openssl by crate name, but that alone does not
prove the launcher selected the *right* TLS + crypto + DNS features. This
asserts the shape of the real `cli/launcher` dependency graph via
`cargo tree -e features`:

- `ring` is present and `aws-lc-rs` is absent — the crypto provider is the
  pure-Rust one the cross-build (0165) needs, not the C/asm one.
- `hickory-resolver` is present *by crate name* (not the reqwest feature label),
  so a silent feature rename that dropped back to `getaddrinfo` is caught
  structurally.
- no native-tls/openssl(-sys) and no host-cert-store crate
  (`rustls-native-certs`/`security-framework`) enters the tree — the static
  binary carries bundled webpki-roots and reads no host state.

This proves the crates are *selected* in the graph, not that they link
statically or resolve DNS on musl — the four-triple build (0165) is the sole
authority for that.
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
    # A crate node renders as "<drawing chars> <crate> v<version>"; the
    # negative lookbehind stops `ring` matching inside another name and
    # `openssl` matching `openssl-probe`, etc.
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
