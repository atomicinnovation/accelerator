from pathlib import Path

REPO_ROOT        = Path(__file__).resolve().parent.parent.parent
VISUALISER       = REPO_ROOT / "skills/visualisation/visualise"
BIN_DIR          = VISUALISER / "bin"
CHECKSUMS        = BIN_DIR / "checksums.json"
SERVER           = VISUALISER / "server"
CARGO_TOML       = SERVER / "Cargo.toml"
FRONTEND         = VISUALISER / "frontend"
PLUGIN_JSON                  = REPO_ROOT / ".claude-plugin/plugin.json"
MARKETPLACE_JSON             = REPO_ROOT / ".claude-plugin/marketplace.json"
PRERELEASE_MARKETPLACE_JSON  = REPO_ROOT / ".claude-plugin/marketplace-prerelease.json"
CHANGELOG        = REPO_ROOT / "CHANGELOG.md"


def binary_path(platform: str, bin_dir: Path = BIN_DIR) -> Path:
    return bin_dir / f"accelerator-visualiser-{platform}"


def debug_archive_path(platform: str, bin_dir: Path = BIN_DIR) -> Path:
    return bin_dir / f"accelerator-visualiser-{platform}.debug.tar.gz"
