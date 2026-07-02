import tomllib
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
VISUALISER = REPO_ROOT / "skills/visualisation/visualise"
BIN_DIR = VISUALISER / "bin"
CHECKSUMS = BIN_DIR / "checksums.json"
SERVER = VISUALISER / "server"
CARGO_TOML = SERVER / "Cargo.toml"
FRONTEND = VISUALISER / "frontend"
CLI_DIR = REPO_ROOT / "cli"
CLI_WORKSPACE_CARGO_TOML = CLI_DIR / "Cargo.toml"
PLUGIN_JSON = REPO_ROOT / ".claude-plugin/plugin.json"
MARKETPLACE_JSON = REPO_ROOT / ".claude-plugin/marketplace.json"
PRERELEASE_MARKETPLACE_JSON = (
    REPO_ROOT / ".claude-plugin/marketplace-prerelease.json"
)
CHANGELOG = REPO_ROOT / "CHANGELOG.md"


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as f:
        return tomllib.load(f)


def cli_member_manifests(workspace_manifest: Path) -> list[Path]:
    """Resolve each cli/ workspace member's Cargo.toml.

    The manifest path is required (no default) so a test always exercises the
    injected path and a rootless test can never silently read the real repo
    manifest. Raises KeyError if [workspace].members is absent — callers that
    need a friendly message (version coherence) translate it.
    """
    members = load_toml(workspace_manifest)["workspace"]["members"]
    return [workspace_manifest.parent / m / "Cargo.toml" for m in members]


def binary_path(platform: str, bin_dir: Path = BIN_DIR) -> Path:
    return bin_dir / f"accelerator-visualiser-{platform}"


def debug_archive_path(platform: str, bin_dir: Path = BIN_DIR) -> Path:
    return bin_dir / f"accelerator-visualiser-{platform}.debug.tar.gz"
