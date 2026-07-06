import json
from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import TypedDict

from tasks.build import validate_version_coherence
from tasks.shared.errors import ManifestError
from tasks.shared.files import atomic_write_text
from tasks.shared.hashing import compute_sha256
from tasks.shared.paths import (
    CLI_DIR,
    DISPATCHED_SUBBINARIES,
    RELEASE_STAGING,
    cli_binary_path,
    load_toml,
)
from tasks.shared.targets import TARGETS
from tasks.signing import sign_file

SCHEMA_VERSION = 1


class PlatformAsset(TypedDict):
    sha256: str
    signature: str


class ManifestBinary(TypedDict):
    description: str
    platforms: dict[str, PlatformAsset]


class Manifest(TypedDict):
    schema_version: int
    version: str
    binaries: dict[str, ManifestBinary]


@dataclass(frozen=True)
class BinaryEntry:
    description: str
    platforms: Mapping[str, PlatformAsset]


def _default_subbinary_manifest(name: str) -> Path:
    return CLI_DIR / name / "Cargo.toml"


def _read_description(manifest_path: Path, name: str) -> str:
    description = load_toml(manifest_path).get("package", {}).get("description")
    if not isinstance(description, str) or not description:
        raise ManifestError(
            f"{name}: crate manifest {manifest_path} has no package.description"
        )
    return description


def collect_entries(
    subbinaries: Iterable[str] = DISPATCHED_SUBBINARIES,
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
    for name in subbinaries:
        description = _read_description(manifest_for(name), name)
        platforms: dict[str, PlatformAsset] = {}
        for _triple, platform in TARGETS:
            binary = cli_binary_path(name, platform, staging_dir)
            signature = binary.with_name(binary.name + ".minisig")
            platforms[platform] = {
                "sha256": compute_sha256(binary),
                "signature": signature.read_text(),
            }
        entries[name] = BinaryEntry(
            description=description, platforms=platforms
        )
    return entries


def build_manifest(
    version: str, entries: Mapping[str, BinaryEntry]
) -> Manifest:
    return {
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


def emit_manifest(
    path: Path,
    version: str,
    entries: Mapping[str, BinaryEntry],
    secret_key: Path,
) -> Path:
    """Serialise, version-check, and sign the manifest as a single artifact.

    Writes the manifest once, checks `manifest.version` against every other
    version source, then signs the exact bytes on disk. The signature is written
    to `manifest.minisig` (the name the launcher fetches), never
    `manifest.json.minisig`. No re-serialisation happens between signing and
    upload, so the signature always covers the shipped bytes.
    """
    manifest = build_manifest(version, entries)
    atomic_write_text(path, json.dumps(manifest, indent=2) + "\n")
    validate_version_coherence(version, manifest_path=path)
    signature = path.with_name("manifest.minisig")
    sign_file(secret_key, path, signature)
    return path
