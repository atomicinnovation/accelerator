import json
import shutil
import tarfile
import tomllib
from pathlib import Path
from typing import Mapping

from invoke import Context, task

from tasks.shared.paths import (
    BIN_DIR,
    CARGO_TOML,
    CHECKSUMS,
    FRONTEND,
    PLUGIN_JSON,
    REPO_ROOT,
    SERVER,
    binary_path,
    debug_archive_path,
)
from tasks.shared.releases import (
    InvalidVersionError,
    VersionCoherenceError,
    _atomic_write_text,
    compute_sha256,
)
from tasks.shared.targets import TARGETS

_CARGO_TOML_RELATIVE  = CARGO_TOML.relative_to(REPO_ROOT)
_PLUGIN_JSON_RELATIVE = PLUGIN_JSON.relative_to(REPO_ROOT)
_CHECKSUMS_RELATIVE   = CHECKSUMS.relative_to(REPO_ROOT)

_MACHO_MAGIC = frozenset([
    b"\xcf\xfa\xed\xfe",
    b"\xce\xfa\xed\xfe",
    b"\xca\xfe\xba\xbe",
])
_ELF_MAGIC = b"\x7fELF"


def _read_plugin_json_version(root: Path) -> str:
    data = json.loads((root / _PLUGIN_JSON_RELATIVE).read_text())
    return data["version"]


def _read_cargo_toml_version(root: Path) -> str:
    with open(root / _CARGO_TOML_RELATIVE, "rb") as f:
        data = tomllib.load(f)
    return data["package"]["version"]


def _read_checksums_json_version(root: Path) -> str:
    data = json.loads((root / _CHECKSUMS_RELATIVE).read_text())
    return data["version"]


def update_checksums_json(
    manifest_path: Path,
    version: str,
    platform_hashes: Mapping[str, str] | None = None,
) -> None:
    data = json.loads(manifest_path.read_text())
    data["version"] = version
    if platform_hashes:
        for platform, hex_digest in platform_hashes.items():
            data.setdefault("binaries", {})[platform] = f"sha256:{hex_digest}"
    _atomic_write_text(manifest_path, json.dumps(data, indent=2) + "\n")


def validate_version_coherence(
    expected_version: str,
    repo_root: Path | None = None,
) -> None:
    if not expected_version:
        raise InvalidVersionError("expected_version must not be empty")
    root = repo_root or REPO_ROOT
    found = {
        "plugin.json":    _read_plugin_json_version(root),
        "Cargo.toml":     _read_cargo_toml_version(root),
        "checksums.json": _read_checksums_json_version(root),
    }
    mismatches = {k: v for k, v in found.items() if v != expected_version}
    if mismatches:
        raise VersionCoherenceError(
            f"expected {expected_version!r}, found mismatches: {mismatches}"
        )


@task
def frontend(context: Context):
    """Build the visualiser frontend (Vite production build into dist/)."""
    context.run(f"npm --prefix {FRONTEND} run build")


@task
def server_dev(context: Context):
    """Build the visualiser server binary with the dev-frontend feature.

    Serves the frontend from the filesystem at runtime; used for local
    development and E2E tests. Not for release.
    """
    context.run(
        f"cargo build --manifest-path {CARGO_TOML} "
        f"--no-default-features --features dev-frontend"
    )


@task
def server_release(context: Context):
    """Build the visualiser server binary for release.

    Uses the default embed-dist feature, which bakes the frontend assets
    into the binary at compile time for a self-contained release artifact.
    """
    context.run(f"cargo build --manifest-path {CARGO_TOML} --release")


@task
def server_cross_compile(context: Context):
    """Cross-compile the visualiser server for all four release targets.

    Produces stripped binaries staged to bin/ alongside debug-symbol archives.
    """
    for triple, platform in TARGETS:
        context.run(
            f"cargo zigbuild --release --target {triple} --manifest-path {CARGO_TOML}",
            pty=True,
        )
        src = SERVER / "target" / triple / "release" / "accelerator-visualiser"
        _assert_magic_bytes(src, triple)
        shutil.copy2(src, binary_path(platform))


@task
def create_debug_archives(context: Context):
    """Create .debug.tar.gz archives for all cross-compiled release binaries."""
    for _, platform in TARGETS:
        binary = binary_path(platform)
        archive_path = debug_archive_path(platform)
        with tarfile.open(archive_path, "w:gz") as tar:
            tar.add(binary, arcname=binary.name)


@task
def create_checksums(context: Context, version: str) -> None:
    """Compute SHA-256 checksums for all release binaries and write checksums.json."""
    validate_version_coherence(version)
    binaries = {
        platform: binary_path(platform, BIN_DIR)
        for _, platform in TARGETS
    }
    hashes = {platform: compute_sha256(path) for platform, path in binaries.items()}
    update_checksums_json(CHECKSUMS, version, hashes)
    validate_version_coherence(version)


def _assert_magic_bytes(path, triple: str) -> None:
    magic = path.read_bytes()[:4]
    if "darwin" in triple:
        if magic not in _MACHO_MAGIC:
            raise RuntimeError(
                f"unexpected magic bytes for darwin binary {path.name}: {magic!r}"
            )
    else:
        if magic != _ELF_MAGIC:
            raise RuntimeError(
                f"unexpected magic bytes for linux binary {path.name}: {magic!r}"
            )
