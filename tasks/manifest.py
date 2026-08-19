import json
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType
from typing import NotRequired, TypedDict

from tasks.build import validate_version_coherence
from tasks.shared.dispatch_coherence import validate_dispatch_coherence
from tasks.shared.errors import ManifestError
from tasks.shared.files import atomic_write_text
from tasks.shared.hashing import compute_sha256
from tasks.shared.paths import (
    CLI_DIR,
    DISPATCHED_SUBBINARIES,
    RELEASE_STAGING,
    TREE_ARTIFACTS,
    load_toml,
    subbinary_asset_path,
    tree_artifact_asset_path,
)
from tasks.shared.targets import TARGETS
from tasks.signing import sign_file

SCHEMA_VERSION = 1

# Tree-artifact descriptions come from the assembly, not a Cargo.toml, so they
# are declared here rather than sourced like the sub-binary descriptions.
TREE_ARTIFACT_DESCRIPTIONS: Mapping[str, str] = MappingProxyType(
    {
        "driver": "Playwright driver bundle (Node runtime + playwright-core)",
        "browser": "Chromium headless shell",
    }
)


class PlatformAsset(TypedDict):
    sha256: str
    signature: str


class ArtifactPlatformAsset(TypedDict):
    sha256: str
    signature: str
    archive_size: int
    uncompressed_size: int
    entry_count: int


class ManifestBinary(TypedDict):
    description: str
    platforms: dict[str, PlatformAsset]


class ManifestArtifact(TypedDict):
    description: str
    platforms: dict[str, ArtifactPlatformAsset]


class Manifest(TypedDict):
    schema_version: int
    version: str
    binaries: dict[str, ManifestBinary]
    artifacts: NotRequired[dict[str, ManifestArtifact]]


@dataclass(frozen=True)
class BinaryEntry:
    description: str
    platforms: Mapping[str, PlatformAsset]


@dataclass(frozen=True)
class ArtifactEntry:
    description: str
    platforms: Mapping[str, ArtifactPlatformAsset]


# Dispatched sub-binaries whose crate manifest is not `cli/<name>/Cargo.toml`
# (the visualiser server lives under `cli/visualiser/server/`; the `vcs`,
# `work`, `corpus`, `collaboration`, and `migrate` tokens' binary crates live
# under `cli/vcs-cli/`/`cli/work-cli/`/`cli/corpus-cli/`/
# `cli/collaboration-cli/`/`cli/migrate-cli/`, not
# `cli/vcs/`/`cli/work/`/`cli/corpus/`/`cli/collaboration/`/`cli/migrate/`,
# which are the domain crates).
_SUBBINARY_MANIFESTS: Mapping[str, Path] = MappingProxyType(
    {
        "visualiser": CLI_DIR / "visualiser/server/Cargo.toml",
        "vcs": CLI_DIR / "vcs-cli/Cargo.toml",
        "work": CLI_DIR / "work-cli/Cargo.toml",
        "corpus": CLI_DIR / "corpus-cli/Cargo.toml",
        "collaboration": CLI_DIR / "collaboration-cli/Cargo.toml",
        "migrate": CLI_DIR / "migrate-cli/Cargo.toml",
        "design": CLI_DIR / "design-cli/Cargo.toml",
    }
)


def _default_subbinary_manifest(name: str) -> Path:
    return _SUBBINARY_MANIFESTS.get(name, CLI_DIR / name / "Cargo.toml")


def _read_description(manifest_path: Path, name: str) -> str:
    description = load_toml(manifest_path).get("package", {}).get("description")
    if not isinstance(description, str) or not description:
        raise ManifestError(
            f"{name}: crate manifest {manifest_path} has no package.description"
        )
    return description


def collect_entries(
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
    *,
    staging_dir: Path = RELEASE_STAGING,
    manifest_for: Callable[[str], Path] = _default_subbinary_manifest,
) -> dict[str, BinaryEntry]:
    """Assemble the typed per-sub-binary manifest entries.

    Sources each sub-binary's description from its crate `Cargo.toml`, computes
    its sha256, and slurps the pre-produced `.minisig` contents as the inline
    signature. The launcher (`accelerator`) is never a manifest entry — the
    bootstrap fetches it via its detached signature — so it is not collected.
    """
    entries: dict[str, BinaryEntry] = {}
    for name in tokens:
        description = _read_description(manifest_for(name), name)
        platforms: dict[str, PlatformAsset] = {}
        for _triple, platform in TARGETS:
            binary = subbinary_asset_path(name, platform, staging_dir)
            signature = binary.with_name(binary.name + ".minisig")
            platforms[platform] = {
                "sha256": compute_sha256(binary),
                "signature": signature.read_text(),
            }
        entries[name] = BinaryEntry(
            description=description, platforms=platforms
        )
    return entries


def collect_artifact_entries(
    tokens: Iterable[str] = TREE_ARTIFACTS,
    *,
    staging_dir: Path = RELEASE_STAGING,
    platforms: Sequence[tuple[str, str]] = TARGETS,
) -> dict[str, ArtifactEntry]:
    """Assemble the typed per-tree-artifact manifest entries.

    Reads each staged archive's sha256, inline `.minisig` and byte size, and the
    extraction bounds from its `.sealed` attestation — so the bounds the
    launcher enforces are the ones the assembly measured, not values restated
    here. Descriptions come from the fixed tree-artifact table, not a crate
    manifest.
    """
    entries: dict[str, ArtifactEntry] = {}
    for name in tokens:
        assets: dict[str, ArtifactPlatformAsset] = {}
        for _triple, platform in platforms:
            archive = tree_artifact_asset_path(name, platform, staging_dir)
            signature = archive.with_name(archive.name + ".minisig")
            sealed = archive.with_name(archive.name + ".sealed")
            document = json.loads(sealed.read_text())
            assets[platform] = {
                "sha256": compute_sha256(archive),
                "signature": signature.read_text(),
                "archive_size": archive.stat().st_size,
                "uncompressed_size": document["uncompressed_size"],
                "entry_count": document["entry_count"],
            }
        entries[name] = ArtifactEntry(
            description=TREE_ARTIFACT_DESCRIPTIONS.get(name, ""),
            platforms=assets,
        )
    return entries


def build_manifest(
    version: str,
    entries: Mapping[str, BinaryEntry],
    *,
    artifacts: Mapping[str, ArtifactEntry] | None = None,
) -> Manifest:
    manifest: Manifest = {
        "schema_version": SCHEMA_VERSION,
        "version": version,
        "binaries": {
            name: {
                "description": entry.description,
                "platforms": {
                    plat: {
                        "sha256": asset["sha256"],
                        "signature": asset["signature"],
                    }
                    for plat, asset in entry.platforms.items()
                },
            }
            for name, entry in entries.items()
        },
    }
    if artifacts:
        manifest["artifacts"] = {
            name: {
                "description": entry.description,
                "platforms": {
                    plat: {
                        "sha256": asset["sha256"],
                        "signature": asset["signature"],
                        "archive_size": asset["archive_size"],
                        "uncompressed_size": asset["uncompressed_size"],
                        "entry_count": asset["entry_count"],
                    }
                    for plat, asset in entry.platforms.items()
                },
            }
            for name, entry in artifacts.items()
        }
    return manifest


def emit_manifest(
    path: Path,
    version: str,
    entries: Mapping[str, BinaryEntry],
    secret_key: Path,
) -> Path:
    """Serialise, version-check, and sign the manifest as a single artifact.

    Dispatch coherence is checked before the write — it reads nothing from the
    manifest, and failing after the write would leave a fresh unsigned
    `manifest.json` beside a stale `manifest.minisig`.

    Writes the manifest once, checks `manifest.version` against every other
    version source, then signs the exact bytes on disk. The signature is written
    to `manifest.minisig` (the name the launcher fetches), never
    `manifest.json.minisig`. No re-serialisation happens between signing and
    upload, so the signature always covers the shipped bytes.
    """
    validate_dispatch_coherence()
    manifest = build_manifest(version, entries)
    atomic_write_text(path, json.dumps(manifest, indent=2) + "\n")
    validate_version_coherence(version, manifest_path=path)
    signature = path.with_name("manifest.minisig")
    sign_file(secret_key, path, signature)
    return path
