import json
import shutil
import tarfile
import tomllib
from collections.abc import Mapping
from pathlib import Path

from invoke import Context, task

from tasks.shared.errors import InvalidVersionError
from tasks.shared.files import atomic_write_text
from tasks.shared.hashing import compute_sha256
from tasks.shared.paths import (
    A9R_CARGO_TOML,
    BIN_DIR,
    CARGO_TOML,
    CHECKSUMS,
    FRONTEND,
    PLUGIN_JSON,
    REPO_ROOT,
    WORKSPACE_CARGO_TOML,
    a9r_binary_path,
    binary_path,
    debug_archive_path,
)
from tasks.shared.targets import TARGETS


class VersionCoherenceError(Exception): ...


_CARGO_TOML_RELATIVE = WORKSPACE_CARGO_TOML.relative_to(REPO_ROOT)
_PLUGIN_JSON_RELATIVE = PLUGIN_JSON.relative_to(REPO_ROOT)
_CHECKSUMS_RELATIVE = CHECKSUMS.relative_to(REPO_ROOT)

_MACHO_MAGIC = frozenset(
    [
        b"\xcf\xfa\xed\xfe",
        b"\xce\xfa\xed\xfe",
        b"\xca\xfe\xba\xbe",
    ]
)
_ELF_MAGIC = b"\x7fELF"


def _read_plugin_json_version(root: Path) -> str:
    data = json.loads((root / _PLUGIN_JSON_RELATIVE).read_text())
    return data["version"]


def _read_cargo_toml_version(root: Path) -> str:
    with (root / _CARGO_TOML_RELATIVE).open("rb") as f:
        data = tomllib.load(f)
    try:
        return data["workspace"]["package"]["version"]
    except KeyError as exc:
        # Fail closed: a missing [workspace.package].version must abort the
        # coherence guard loudly, never silently pass. server/Cargo.toml
        # inherits this key (version.workspace = true) and carries no literal.
        raise VersionCoherenceError(
            f"{_CARGO_TOML_RELATIVE}: missing [workspace.package].version"
        ) from exc


def _read_checksums_json_version(root: Path) -> str:
    data = json.loads((root / _CHECKSUMS_RELATIVE).read_text())
    return data["version"]


def _assert_magic_bytes(path: Path, triple: str) -> None:
    magic = path.read_bytes()[:4]
    if "darwin" in triple:
        if magic not in _MACHO_MAGIC:
            raise RuntimeError(
                f"unexpected magic bytes for darwin binary "
                f"{path.name}: {magic!r}"
            )
    elif magic != _ELF_MAGIC:
        raise RuntimeError(
            f"unexpected magic bytes for linux binary {path.name}: {magic!r}"
        )


def update_checksums_json(
    manifest_path: Path,
    version: str,
    asset_hashes: Mapping[str, Mapping[str, str]] | None = None,
) -> None:
    """Bump the manifest version and, optionally, asset checksums.

    `asset_hashes` is nested by platform then asset name —
    `{platform: {asset_name: hex}}` — matching the `binaries[platform][asset]`
    schema that lets one manifest carry both the accelerator-visualiser and the
    a9r asset through the rename transition. Passing only `version` (no hashes)
    preserves `binaries` verbatim, which is the version writer's path.
    """
    data = json.loads(manifest_path.read_text())
    data["version"] = version
    if asset_hashes:
        binaries = data.setdefault("binaries", {})
        for platform, assets in asset_hashes.items():
            existing = binaries.get(platform)
            # A legacy flat entry (string) is migrated to the nested shape.
            slot: dict[str, str] = (
                existing if isinstance(existing, dict) else {}
            )
            binaries[platform] = slot
            for asset_name, hex_digest in assets.items():
                slot[asset_name] = f"sha256:{hex_digest}"
    atomic_write_text(manifest_path, json.dumps(data, indent=2) + "\n")


def validate_version_coherence(
    expected_version: str,
    repo_root: Path | None = None,
) -> None:
    if not expected_version:
        raise InvalidVersionError("expected_version must not be empty")
    root = repo_root or REPO_ROOT
    found = {
        "plugin.json": _read_plugin_json_version(root),
        "Cargo.toml": _read_cargo_toml_version(root),
        "checksums.json": _read_checksums_json_version(root),
    }
    mismatches = {k: v for k, v in found.items() if v != expected_version}
    if mismatches:
        raise VersionCoherenceError(
            f"expected {expected_version!r}, found mismatches: {mismatches}"
        )


@task
def frontend(context: Context) -> None:
    """Build the visualiser frontend (Vite production build into dist/)."""
    context.run(f"npm --prefix {FRONTEND} run build")


@task
def frontend_stub(context: Context) -> None:
    """Write a placeholder frontend/dist/index.html if absent (lint-only stub).

    Satisfies the embed-dist build.rs existence check for lint-only compiles
    (cargo clippy) without a full Vite build. Never clobbers a real build; a
    real `build.frontend` overwrites it. A zero-byte file is treated as absent
    so a torn prior write self-heals rather than being embedded.
    """
    index = FRONTEND / "dist" / "index.html"
    if index.is_file() and index.stat().st_size > 0:
        return
    index.parent.mkdir(parents=True, exist_ok=True)
    atomic_write_text(
        index, "<!-- accelerator lint stub — not a real build -->\n"
    )


@task
def server_dev(context: Context) -> None:
    """Build the visualiser server binary with the dev-frontend feature.

    Serves the frontend from the filesystem at runtime; used for local
    development and E2E tests. Not for release.
    """
    context.run(
        f"cargo build --manifest-path {CARGO_TOML} "
        f"--no-default-features --features dev-frontend"
    )


@task
def server_release(context: Context) -> None:
    """Build the visualiser server binary for release.

    Uses the default embed-dist feature, which bakes the frontend assets
    into the binary at compile time for a self-contained release artifact.
    """
    context.run(f"cargo build --manifest-path {CARGO_TOML} --release")


@task
def server_cross_compile(context: Context) -> None:
    """Cross-compile the visualiser server and a9r for all four release targets.

    Produces stripped binaries staged to bin/ alongside debug-symbol archives.
    Both assets are built so a single release can carry the
    accelerator-visualiser binary (still launched by older plugins) and the new
    a9r binary side by side during the rename transition. a9r is built with the
    `visualise` feature so the released artifact embeds the SPA — the
    default-feature-off (no embed-dist) only applies to the dev/lint build.
    """
    # Workspace members share the single target/ at the workspace root, not a
    # per-crate server/target/.
    target_root = WORKSPACE_CARGO_TOML.parent / "target"
    for triple, platform in TARGETS:
        context.run(
            f"cargo zigbuild --release --target {triple} "
            f"--manifest-path {CARGO_TOML}",
            pty=True,
        )
        src = target_root / triple / "release" / "accelerator-visualiser"
        _assert_magic_bytes(src, triple)
        shutil.copy2(src, binary_path(platform))

        context.run(
            f"cargo zigbuild --release --target {triple} "
            f"--manifest-path {A9R_CARGO_TOML} "
            f"--no-default-features --features visualise",
            pty=True,
        )
        a9r_src = target_root / triple / "release" / "a9r"
        _assert_magic_bytes(a9r_src, triple)
        shutil.copy2(a9r_src, a9r_binary_path(platform))


@task
def create_debug_archives(context: Context) -> None:
    """Create .debug.tar.gz archives for all cross-compiled release binaries."""
    for _, platform in TARGETS:
        binary = binary_path(platform)
        archive_path = debug_archive_path(platform)
        with tarfile.open(archive_path, "w:gz") as tar:
            tar.add(binary, arcname=binary.name)


@task
def create_checksums(context: Context, version: str) -> None:
    """Compute SHA-256 checksums for release binaries; write checksums.json.

    Both release assets (accelerator-visualiser and a9r) are checksummed per
    platform so the a9r artifact participates in the same checksum/coherence
    pipeline as the visualiser — nested under binaries[platform][asset-name].
    """
    validate_version_coherence(version)
    asset_hashes: dict[str, dict[str, str]] = {}
    for _, platform in TARGETS:
        vis = binary_path(platform, BIN_DIR)
        a9r = a9r_binary_path(platform, BIN_DIR)
        asset_hashes[platform] = {
            vis.name: compute_sha256(vis),
            a9r.name: compute_sha256(a9r),
        }
    update_checksums_json(CHECKSUMS, version, asset_hashes)
    validate_version_coherence(version)
