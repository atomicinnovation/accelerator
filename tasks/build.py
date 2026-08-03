import hashlib
import json
import shutil
import subprocess
import tarfile
import tomllib
from collections.abc import Iterable, Mapping
from pathlib import Path

from invoke import Context, task

from tasks.shared.errors import InvalidVersionError
from tasks.shared.files import atomic_write_text
from tasks.shared.paths import (
    CARGO_TOML,
    CLI_DIR,
    CLI_TARGET_DIR,
    CLI_WORKSPACE_CARGO_TOML,
    DEBUG_ARCHIVE_DIRS,
    DISPATCHED_SUBBINARIES,
    FRONTEND,
    PLUGIN_JSON,
    RELEASE_STAGING,
    REPO_ROOT,
    VENDOR_SHIM_MARKER,
    cli_binary_path,
    cli_member_manifests,
    debug_archive_path,
    load_toml,
    subbinary_asset_path,
    vendored_shim_path,
)
from tasks.shared.targets import TARGETS

_CLI_RELEASE_BINARIES = ("accelerator", "accelerator-verify")


class VersionCoherenceError(Exception): ...


_CLI_WORKSPACE_CARGO_TOML_RELATIVE = CLI_WORKSPACE_CARGO_TOML.relative_to(
    REPO_ROOT
)
_PLUGIN_JSON_RELATIVE = PLUGIN_JSON.relative_to(REPO_ROOT)

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


def _is_statically_linked(file_output: str) -> bool:
    """Whether `file`'s output describes a fully-static ELF.

    Accepts the plain-static, static-PIE, and no-interpreter phrasings that
    `file` emits for a musl static binary; a dynamically-linked ELF names its
    interpreter and is rejected.
    """
    return (
        "statically linked" in file_output
        or "static-pie linked" in file_output
        or "not a dynamic executable" in file_output
    )


def _assert_static_elf(path: Path) -> None:
    """Fail closed unless `path` is a fully-static ELF (no interpreter/NEEDED).

    Uses `file`, which reads foreign-arch ELF headers without executing them, so
    all four cross-compiled musl binaries can be checked on the single host
    runner. Raises rather than skipping if `file` is unavailable, so a broken
    cross-compile can never slip through by silently disabling the check.
    """
    reader = shutil.which("file")
    if reader is None:
        raise RuntimeError(
            f"cannot assert static linking for {path.name}: `file` not on PATH"
        )
    result = subprocess.run(
        [reader, "-b", str(path)],
        check=False,
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"`file` failed for {path.name}: {result.stderr.strip()}"
        )
    if not _is_statically_linked(result.stdout):
        raise RuntimeError(
            f"{path.name} is not statically linked: {result.stdout.strip()}"
        )


def validate_version_coherence(
    expected_version: str,
    repo_root: Path | None = None,
    *,
    manifest_path: Path | None = None,
) -> None:
    if not expected_version:
        raise InvalidVersionError("expected_version must not be empty")
    root = repo_root or REPO_ROOT
    found = {
        "plugin.json": _read_plugin_json_version(root),
        _CLI_WORKSPACE_CARGO_TOML_RELATIVE.as_posix(): _read_workspace_version(
            root
        ),
        **_pinned_member_versions(root),
    }
    if manifest_path is not None:
        found["manifest.json"] = json.loads(manifest_path.read_text())[
            "version"
        ]
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
def cli_dev(context: Context) -> None:
    """Build the debug cli launcher (cli/target/debug/accelerator).

    The local launcher the repointed shell suites and cargo integration tests
    invoke through ACCELERATOR_BIN. Declared as a mise build dependency of the
    test tasks so build ordering lives in the task graph, not in ad-hoc cargo
    calls inside the suites.
    """
    context.run(
        f"cargo build --manifest-path {CLI_WORKSPACE_CARGO_TOML} "
        f"--bin accelerator"
    )


@task
def server_release(context: Context) -> None:
    """Build the visualiser server binary for release.

    Uses the default embed-dist feature, which bakes the frontend assets
    into the binary at compile time for a self-contained release artifact.
    """
    context.run(f"cargo build --manifest-path {CARGO_TOML} --release")


def _assert_no_e2e_insecure(src: Path) -> None:
    """Fail staging if a release artifact carries the insecure-bypass symbol.

    ``ACCELERATOR_VISUALISER_E2E_INSECURE`` gates the non-loopback bind + the
    relaxed Host guard and is compiled only into the ``dev-frontend`` (test)
    binary, never a default (``embed-dist``) release. Match the full symbol, not
    the ``ACCELERATOR_VISUALISER_`` prefix — the release binary legitimately
    embeds other ``ACCELERATOR_VISUALISER_*`` literals (idle_timeout/editor).
    """
    marker = b"ACCELERATOR_VISUALISER_E2E_INSECURE"
    if marker in src.read_bytes():
        raise RuntimeError(
            f"{src}: release artifact contains {marker.decode()} — it must be "
            "built with default (embed-dist) features only, never dev-frontend"
        )


@task
def server_cross_compile(context: Context) -> None:
    """Cross-compile the visualiser server for all four release targets.

    Produces stripped default (embed-dist) binaries staged to
    dist/release/accelerator-visualiser-{platform} (the single published binary
    the manifest flow signs and the launcher fetches), after asserting the
    artifact carries no dev-frontend insecure-bypass symbol.
    """
    RELEASE_STAGING.mkdir(parents=True, exist_ok=True)
    for triple, platform in TARGETS:
        context.run(
            f"cargo zigbuild --release --target {triple} "
            f"--manifest-path {CARGO_TOML}",
            pty=True,
        )
        src = CLI_TARGET_DIR / triple / "release" / "accelerator-visualiser"
        _assert_magic_bytes(src, triple)
        _assert_no_e2e_insecure(src)
        shutil.copy2(src, subbinary_asset_path("visualiser", platform))


@task
def cli_cross_compile(context: Context) -> None:
    """Cross-compile the cli launcher + verify shim for all four targets.

    Stages each `accelerator-{platform}` and `accelerator-verify-{platform}`
    into dist/release/ after magic-byte and (musl) static-linking assertions.
    """
    RELEASE_STAGING.mkdir(parents=True, exist_ok=True)
    for triple, platform in TARGETS:
        context.run(
            f"cargo zigbuild --release --target {triple} "
            f"--manifest-path {CLI_WORKSPACE_CARGO_TOML}",
            pty=True,
        )
        for name in _CLI_RELEASE_BINARIES:
            src = CLI_DIR / "target" / triple / "release" / name
            _assert_magic_bytes(src, triple)
            if "musl" in triple:
                _assert_static_elf(src)
            shutil.copy2(src, cli_binary_path(name, platform))


def _minisign_verify_pin(cargo_toml_text: str) -> str:
    for line in cargo_toml_text.splitlines():
        if line.strip().startswith("minisign-verify"):
            return line.strip()
    raise RuntimeError("minisign-verify pin not found in cli/Cargo.toml")


def _cargo_lock_package_block(
    lock_text: str, name: str, *, drop_version: bool = False
) -> str:
    for block in lock_text.split("[[package]]"):
        if f'name = "{name}"' in block:
            lines = block.strip().splitlines()
            if drop_version:
                lines = [
                    line
                    for line in lines
                    if not line.strip().startswith("version = ")
                ]
            return "\n".join(lines)
    raise RuntimeError(f"package {name!r} not found in cli/Cargo.lock")


def _dev_only_dependency_names(manifest_path: Path) -> set[str]:
    """Names declared under `[dev-dependencies]` but not `[dependencies]`.

    Dev-dependencies compile only into the crate's own tests, never into the
    release shim binary, so they are not shim build inputs. A name that is
    also a normal dependency is a real build input and stays counted.
    """
    manifest = tomllib.loads(manifest_path.read_text())
    dev = set(manifest.get("dev-dependencies", {}))
    normal = set(manifest.get("dependencies", {}))
    return dev - normal


def _strip_dev_dependencies_table(text: str) -> str:
    """Return `text` with any `[dev-dependencies]` table removed.

    Mirrors the tests exclusion: dev-dependencies are not shim build inputs,
    so a change to them must not perturb the drift marker. The single blank
    separator preceding the table is dropped too, so removing a table appended
    with a separator reproduces the pre-table bytes exactly.
    """
    lines = text.splitlines(keepends=True)
    kept: list[str] = []
    index = 0
    while index < len(lines):
        header = lines[index].lstrip()
        if header.startswith(("[dev-dependencies]", "[dev-dependencies.")):
            if kept and kept[-1].strip() == "":
                kept.pop()
            index += 1
            while index < len(lines) and not lines[index].lstrip().startswith(
                "["
            ):
                index += 1
            continue
        kept.append(lines[index])
        index += 1
    return "".join(kept)


def _strip_lock_dependencies(block: str, names: set[str]) -> str:
    """Drop `dependencies = [...]` entries in `block` naming `names`.

    A Cargo.lock package block merges every dependency kind into one list, so
    a dev-only dependency surfaces here as a plain entry; removing it keeps the
    hashed block stable across dev-dependency changes.
    """
    kept: list[str] = []
    for line in block.splitlines():
        entry = line.strip()
        if entry.startswith('"') and entry.rstrip(",").endswith('"'):
            dep = entry.rstrip(",").strip('"').split(" ", 1)[0]
            if dep in names:
                continue
        kept.append(line)
    return "\n".join(kept)


def vendor_shim_marker_digest(root: Path = REPO_ROOT) -> str:
    """SHA-256 over the verify shim's build inputs.

    Covers `cli/verify` source (excluding the crate's own tests and its
    `[dev-dependencies]`, neither of which builds into the shim) plus the
    `minisign-verify` pin and its resolved lockfile closure, so a dependency
    bump that never touches `cli/verify/**` still trips the drift guard. The
    `accelerator-verify` lock block's own version line and its dev-only
    dependency entries are dropped so a routine release version bump or a
    test-only dev-dependency change is not drift.
    """
    hasher = hashlib.sha256()
    verify_dir = root / "cli" / "verify"
    tests_dir = verify_dir / "tests"
    manifest_path = verify_dir / "Cargo.toml"
    dev_only = _dev_only_dependency_names(manifest_path)
    for path in sorted(p for p in verify_dir.rglob("*") if p.is_file()):
        if tests_dir in path.parents:
            continue
        hasher.update(path.relative_to(verify_dir).as_posix().encode())
        hasher.update(b"\0")
        if path == manifest_path:
            hasher.update(
                _strip_dev_dependencies_table(path.read_text()).encode()
            )
        else:
            hasher.update(path.read_bytes())
        hasher.update(b"\0")
    cargo_toml = (root / "cli" / "Cargo.toml").read_text()
    hasher.update(_minisign_verify_pin(cargo_toml).encode())
    hasher.update(b"\0")
    lock_text = (root / "cli" / "Cargo.lock").read_text()
    hasher.update(
        _strip_lock_dependencies(
            _cargo_lock_package_block(
                lock_text, "accelerator-verify", drop_version=True
            ),
            dev_only,
        ).encode()
    )
    hasher.update(b"\0")
    hasher.update(
        _cargo_lock_package_block(lock_text, "minisign-verify").encode()
    )
    return hasher.hexdigest()


def assert_staged_launcher_versions(version: str) -> None:
    """Confirm each staged launcher embeds the release version.

    The launcher bakes `CARGO_PKG_VERSION` in as the manifest anti-rollback
    check, so a stale cross-compile would later reject its own manifest. The
    three foreign-arch launchers cannot be executed on the host runner, so the
    version string is grepped out of the binary bytes rather than read via
    `--version`.
    """
    needle = version.encode()
    for _triple, platform in TARGETS:
        binary = cli_binary_path("accelerator", platform)
        if needle not in binary.read_bytes():
            raise RuntimeError(
                f"staged launcher {binary.name} does not embed the release "
                f"version {version}"
            )


@task
def vendor_verify_shims(context: Context) -> None:
    """Copy the cross-compiled verify shims into the committed bin/ tree.

    The shims are the bootstrap's root of trust; they ship inside the plugin
    package (a different lifecycle from the uploaded, gitignored binaries) and
    are refreshed on demand, never in the release hot path. Records a marker
    over the shims' build inputs so the drift guard can flag a stale vendor.
    """
    for _triple, platform in TARGETS:
        source = cli_binary_path("accelerator-verify", platform)
        destination = vendored_shim_path(platform)
        shutil.copy2(source, destination)
        destination.chmod(0o755)
    atomic_write_text(VENDOR_SHIM_MARKER, vendor_shim_marker_digest() + "\n")


def _debug_archive_targets(
    dirs: Mapping[str, Path] = DEBUG_ARCHIVE_DIRS,
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
    staging_dir: Path = RELEASE_STAGING,
) -> list[tuple[Path, Path]]:
    """Each (staged binary, archive path) pair the debug archives cover."""
    unknown = sorted(set(dirs) - set(tokens))
    if unknown:
        raise RuntimeError(
            f"debug-archive registry names undispatched token(s): {unknown} — "
            "nothing cross-compiles them, so the archive source would be absent"
        )
    stray = sorted(str(d) for d in dirs.values() if d.name != "bin")
    if stray:
        raise RuntimeError(
            f"debug-archive directories must be `bin/` trees: {stray} — "
            "`.gitignore`'s archive rule is `**/bin/*.debug.tar.gz`, so an "
            "archive written elsewhere would be committed by `git add .`"
        )
    return [
        (
            subbinary_asset_path(token, platform, staging_dir),
            debug_archive_path(token, platform, directory),
        )
        for token, directory in dirs.items()
        for _triple, platform in TARGETS
    ]


def _write_debug_archives(targets: list[tuple[Path, Path]]) -> None:
    for binary, archive in targets:
        archive.parent.mkdir(parents=True, exist_ok=True)
        with tarfile.open(archive, "w:gz") as tar:
            tar.add(binary, arcname=binary.name)


@task
def create_debug_archives(context: Context) -> None:
    """Archive each cross-compiled sub-binary that ships symbolication data.

    Archives the shared dist/release binary into the sub-binary's committed
    tree, where the provenance glob covers it.
    """
    _write_debug_archives(_debug_archive_targets())
