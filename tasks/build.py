import shutil
import tarfile

from invoke import Context, task

from tasks.shared.paths import (
    BIN_DIR,
    CHECKSUMS,
    CARGO_TOML,
    FRONTEND,
    SERVER,
    binary_path,
    debug_archive_path,
)
from tasks.shared.releases import compute_sha256, update_checksums_json, validate_version_coherence
from tasks.shared.targets import TARGETS

_MACHO_MAGIC = frozenset([
    b"\xcf\xfa\xed\xfe",
    b"\xce\xfa\xed\xfe",
    b"\xca\xfe\xba\xbe",
])
_ELF_MAGIC = b"\x7fELF"


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
