import json
import shutil
import tarfile
from collections.abc import Mapping
from pathlib import Path

from invoke import Context, task

from tasks.shared.errors import InvalidVersionError
from tasks.shared.files import atomic_write_text
from tasks.shared.hashing import compute_sha256
from tasks.shared.paths import (
    BIN_DIR,
    CARGO_TOML,
    CHECKSUMS,
    CLI_WORKSPACE_CARGO_TOML,
    FRONTEND,
    PLUGIN_JSON,
    REPO_ROOT,
    SERVER,
    binary_path,
    cli_member_manifests,
    debug_archive_path,
    load_toml,
)
from tasks.shared.targets import TARGETS


class VersionCoherenceError(Exception): ...


_CARGO_TOML_RELATIVE = CARGO_TOML.relative_to(REPO_ROOT)
_CLI_WORKSPACE_CARGO_TOML_RELATIVE = CLI_WORKSPACE_CARGO_TOML.relative_to(
    REPO_ROOT
)
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
    return load_toml(root / _CARGO_TOML_RELATIVE)["package"]["version"]


def _read_checksums_json_version(root: Path) -> str:
    data = json.loads((root / _CHECKSUMS_RELATIVE).read_text())
    return data["version"]


def _read_workspace_version(root: Path) -> str:
    data = load_toml(root / _CLI_WORKSPACE_CARGO_TOML_RELATIVE)
    try:
        return data["workspace"]["package"]["version"]
    except KeyError as exc:
        raise VersionCoherenceError(
            f"{_CLI_WORKSPACE_CARGO_TOML_RELATIVE.as_posix()}: "
            "missing [workspace.package].version"
        ) from exc


def _pinned_member_versions(root: Path) -> dict[str, str]:
    """Map each member that pins its own [package].version to that literal.

    A member that inherits (version.workspace = true) parses as a table, not a
    string, so it contributes no entry and can never be a mismatch; only a
    member that opts out of inheritance and hardcodes a version string is named.
    """
    manifest = root / _CLI_WORKSPACE_CARGO_TOML_RELATIVE
    try:
        members = cli_member_manifests(manifest)
    except KeyError as exc:
        raise VersionCoherenceError(
            f"{_CLI_WORKSPACE_CARGO_TOML_RELATIVE.as_posix()}: "
            "missing [workspace].members"
        ) from exc
    pinned = {}
    for member in members:
        try:
            data = load_toml(member)
        except FileNotFoundError as exc:
            raise VersionCoherenceError(
                f"{member.relative_to(root).as_posix()}: listed in "
                "[workspace].members but the manifest is absent"
            ) from exc
        version = data.get("package", {}).get("version")
        if isinstance(version, str):
            pinned[member.relative_to(root).as_posix()] = version
    return pinned


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
    platform_hashes: Mapping[str, str] | None = None,
) -> None:
    data = json.loads(manifest_path.read_text())
    data["version"] = version
    if platform_hashes:
        for platform, hex_digest in platform_hashes.items():
            data.setdefault("binaries", {})[platform] = f"sha256:{hex_digest}"
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
        _CLI_WORKSPACE_CARGO_TOML_RELATIVE.as_posix(): _read_workspace_version(
            root
        ),
        **_pinned_member_versions(root),
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
    """Cross-compile the visualiser server for all four release targets.

    Produces stripped binaries staged to bin/ alongside debug-symbol archives.
    """
    for triple, platform in TARGETS:
        context.run(
            f"cargo zigbuild --release --target {triple} "
            f"--manifest-path {CARGO_TOML}",
            pty=True,
        )
        src = SERVER / "target" / triple / "release" / "accelerator-visualiser"
        _assert_magic_bytes(src, triple)
        shutil.copy2(src, binary_path(platform))


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
    """Compute SHA-256 checksums for release binaries; write checksums.json."""
    validate_version_coherence(version)
    binaries = {
        platform: binary_path(platform, BIN_DIR) for _, platform in TARGETS
    }
    hashes = {
        platform: compute_sha256(path) for platform, path in binaries.items()
    }
    update_checksums_json(CHECKSUMS, version, hashes)
    validate_version_coherence(version)
