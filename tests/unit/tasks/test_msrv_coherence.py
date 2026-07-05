"""The Rust MSRV, hand-duplicated across mise.toml, cli/Cargo.toml, and
cli/clippy.toml, must agree — a bump to one that misses the others would move CI
off the floor a user still builds against.
"""

import tomllib
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[3]
_MISE = _REPO_ROOT / "mise.toml"
_CLI_CARGO = _REPO_ROOT / "cli/Cargo.toml"
_CLIPPY = _REPO_ROOT / "cli/clippy.toml"


def _mise_rust() -> str:
    rust = tomllib.loads(_MISE.read_text())["tools"]["rust"]
    # mise accepts either a bare string or a {version, components} table.
    return rust["version"] if isinstance(rust, dict) else rust


def _cargo_rust_version() -> str:
    data = tomllib.loads(_CLI_CARGO.read_text())
    return data["workspace"]["package"]["rust-version"]


def _clippy_msrv() -> str:
    return tomllib.loads(_CLIPPY.read_text())["msrv"]


def test_msrv_is_coherent_across_mise_cargo_and_clippy() -> None:
    versions = {
        "mise.toml [tools].rust": _mise_rust(),
        "cli/Cargo.toml rust-version": _cargo_rust_version(),
        "cli/clippy.toml msrv": _clippy_msrv(),
    }
    assert len(set(versions.values())) == 1, (
        f"Rust MSRV drift — all three must agree: {versions}"
    )
