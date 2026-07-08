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
RELEASE_STAGING = REPO_ROOT / "dist" / "release"
RELEASE_MANIFEST = RELEASE_STAGING / "manifest.json"
RELEASE_MANIFEST_SIG = RELEASE_STAGING / "manifest.minisig"
VENDORED_SHIM_DIR = REPO_ROOT / "bin"
VENDOR_SHIM_MARKER = VENDORED_SHIM_DIR / "accelerator-verify.vendored.sha256"
# The crates whose binaries the manifest lists (empty at HEAD; 0168 appends the
# visualiser).
DISPATCHED_SUBBINARIES: tuple[str, ...] = ()
KEYS_DIR = REPO_ROOT / "keys"
RELEASE_PUBLIC_KEY = KEYS_DIR / "accelerator-release.pub"
RELEASE_SECRET_KEY = KEYS_DIR / "accelerator-release.sec"
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


def cli_binary_path(
    name: str, platform: str, staging_dir: Path = RELEASE_STAGING
) -> Path:
    return staging_dir / f"{name}-{platform}"


def vendored_shim_path(
    platform: str, shim_dir: Path = VENDORED_SHIM_DIR
) -> Path:
    return shim_dir / f"accelerator-verify-{platform}"


def debug_archive_path(platform: str, bin_dir: Path = BIN_DIR) -> Path:
    return bin_dir / f"accelerator-visualiser-{platform}.debug.tar.gz"
