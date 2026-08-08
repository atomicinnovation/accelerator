import tomllib
from collections.abc import Mapping
from pathlib import Path
from types import MappingProxyType
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
# The skill keeps its bin/ (debug archives) here; the server + frontend have
# moved into the cli/ workspace.
VISUALISER = REPO_ROOT / "skills/visualisation/visualise"
BIN_DIR = VISUALISER / "bin"
DOCS_SITE = REPO_ROOT / "docs-site"
DOCS_GENERATED_DIR = DOCS_SITE / "src/content/docs/reference/skills"
CLI_DIR = REPO_ROOT / "cli"
# The server is the 12th cli/ workspace member, so it builds into the shared
# workspace target dir, not a crate-local one.
CLI_TARGET_DIR = CLI_DIR / "target"
SERVER = CLI_DIR / "visualiser/server"
CARGO_TOML = SERVER / "Cargo.toml"
FRONTEND = CLI_DIR / "visualiser/frontend"
CLI_WORKSPACE_CARGO_TOML = CLI_DIR / "Cargo.toml"
RELEASE_STAGING = REPO_ROOT / "dist" / "release"
RELEASE_MANIFEST = RELEASE_STAGING / "manifest.json"
RELEASE_MANIFEST_SIG = RELEASE_STAGING / "manifest.minisig"
VENDORED_SHIM_DIR = REPO_ROOT / "bin"
VENDOR_SHIM_MARKER = VENDORED_SHIM_DIR / "accelerator-verify.vendored.sha256"
# The crates whose binaries the signed manifest lists and the launcher fetches
# by bare token. The visualiser is the first dispatched sub-binary.
DISPATCHED_SUBBINARIES: tuple[str, ...] = (
    "visualiser",
    "vcs",
    "work",
    "corpus",
    "collaboration",
    "migrate",
)
# Tokens whose only consumer is a hook or another binary, never a SKILL.md.
SKILL_EXEMPT_SUBBINARIES: tuple[str, ...] = ()
# Sub-binaries shipping a symbolication archive, and the committed tree each is
# staged into so the provenance glob covers it. Every value must be a `bin/`
# directory — `.gitignore`'s archive rule is `**/bin/*.debug.tar.gz`.
DEBUG_ARCHIVE_DIRS: Mapping[str, Path] = MappingProxyType(
    {"visualiser": BIN_DIR}
)
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


def cli_binary_path(
    name: str, platform: str, staging_dir: Path = RELEASE_STAGING
) -> Path:
    return staging_dir / f"{name}-{platform}"


def subbinary_asset_path(
    token: str, platform: str, staging_dir: Path = RELEASE_STAGING
) -> Path:
    """Resolve the published release asset for a dispatched sub-binary.

    Every sub-binary asset carries the `accelerator-` prefix even though the
    manifest keys on the bare `token`, so the launcher fetches
    `accelerator-<token>-<platform>` — for the visualiser,
    `accelerator-visualiser-<platform>`.
    """
    return staging_dir / f"accelerator-{token}-{platform}"


def vendored_shim_path(
    platform: str, shim_dir: Path = VENDORED_SHIM_DIR
) -> Path:
    return shim_dir / f"accelerator-verify-{platform}"


def debug_archive_path(token: str, platform: str, bin_dir: Path) -> Path:
    """Resolve one sub-binary's symbolication archive.

    `bin_dir` is required: a `BIN_DIR` default is only correct for the
    visualiser, so any other token would silently file under the visualiser's
    skill tree and still satisfy the `bin/` constraint.
    """
    return bin_dir / f"accelerator-{token}-{platform}.debug.tar.gz"
