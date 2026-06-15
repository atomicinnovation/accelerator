from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
VISUALISER = REPO_ROOT / "skills/visualisation/visualise"
BIN_DIR = VISUALISER / "bin"
CHECKSUMS = BIN_DIR / "checksums.json"
SERVER = VISUALISER / "server"
CARGO_TOML = SERVER / "Cargo.toml"
# The workspace root manifest owns the single inherited version
# ([workspace.package].version) — the source of truth for version coherence.
# CARGO_TOML stays the member manifest for --manifest-path fmt/clippy/test.
WORKSPACE_CARGO_TOML = VISUALISER / "Cargo.toml"
A9R_CARGO_TOML = VISUALISER / "a9r" / "Cargo.toml"
A9R_CORE_CARGO_TOML = VISUALISER / "a9r-core" / "Cargo.toml"
FRONTEND = VISUALISER / "frontend"
PLUGIN_JSON = REPO_ROOT / ".claude-plugin/plugin.json"
MARKETPLACE_JSON = REPO_ROOT / ".claude-plugin/marketplace.json"
PRERELEASE_MARKETPLACE_JSON = (
    REPO_ROOT / ".claude-plugin/marketplace-prerelease.json"
)
CHANGELOG = REPO_ROOT / "CHANGELOG.md"


def binary_path(platform: str, bin_dir: Path = BIN_DIR) -> Path:
    return bin_dir / f"accelerator-visualiser-{platform}"


def a9r_binary_path(platform: str, bin_dir: Path = BIN_DIR) -> Path:
    return bin_dir / f"a9r-{platform}"


def debug_archive_path(platform: str, bin_dir: Path = BIN_DIR) -> Path:
    return bin_dir / f"accelerator-visualiser-{platform}.debug.tar.gz"
