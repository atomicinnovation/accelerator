"""The Rust MSRV must be one number, declared coherently in three places.

The cli workspace's MSRV floor is hand-duplicated across `mise.toml` (the
toolchain every CI job provisions), `cli/Cargo.toml` (`rust-version`, which
drives resolver-3 MSRV-aware selection), and `cli/clippy.toml` (`msrv`, which
stops clippy suggesting newer-than-MSRV APIs). If they drift — e.g. a mise rust
bump not mirrored into the declared MSRV — CI would silently move off the
floor and a user on the declared MSRV could hit a break. This test catches that
divergence directly, which is why the redundant full `check-cli-msrv` compile
leg was dropped in favour of it.
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
