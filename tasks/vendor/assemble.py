r"""Compose the driver and browser trees from verified upstream inputs.

The driver tree carries the Node binary and ``playwright-core``; the browser
tree carries ``chromium-headless-shell`` only. Assembly extracts an npm tarball
and Chromium's *zip*, composes each tree, writes a ``NOTICES/`` directory, and
packs both deterministically through :mod:`tasks.vendor.archive`.

Two extraction hazards are handled explicitly. Python's ``zipfile`` ignores the
Unix permission bits stored in ``external_attr`` and materialises symlink
entries as regular files, so :func:`extract_zip` reconstructs both — a browser
tree whose ``chrome-headless-shell`` lost its executable bit passes every
downstream check and then fails at ``execve``. And every extracted path is
contained under its destination the way the launcher's own allowlist contains
it, so a hostile input fails at assembly rather than on a user's machine.
"""

import hashlib
import json
import re
import shutil
import stat
import subprocess
import tarfile
import zipfile
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from pathlib import Path

from tasks.shared.paths import (
    CHROMIUM_LICENSE,
    PINS_TOML,
    RELEASE_STAGING,
    tree_artifact_asset_path,
)
from tasks.vendor import pins
from tasks.vendor.archive import ArchiveStats, write_deterministic_archive
from tasks.vendor.attestation import build_attestation

_DEFAULT_FILE_MODE = 0o644
_SMOKE_TIMEOUT = 30
# The vendored playwright version must be exact, not a caret/tilde range: the
# fetched package, the API lib/*.js was written against, and the derived
# Chromium revision are one choice rather than three that can drift.
_EXACT_VERSION = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")


def extract_zip(archive_path: Path, dest: Path) -> None:
    """Extract ``archive_path`` into ``dest``, preserving modes and symlinks.

    Regular files keep the mode recorded in ``external_attr`` (so the
    executable bit survives), symlinks are recreated as symlinks, and any entry
    resolving outside ``dest`` — an absolute path, a ``..`` traversal, or a
    symlink whose target escapes — raises ``ValueError`` rather than being
    written.
    """
    dest.mkdir(parents=True, exist_ok=True)
    root = dest.resolve()
    with zipfile.ZipFile(archive_path) as archive:
        for info in archive.infolist():
            target = _contained_path(root, info.filename)
            mode = info.external_attr >> 16
            if stat.S_ISLNK(mode):
                _write_symlink(root, target, archive.read(info).decode())
            elif info.is_dir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                _write_file(target, archive.read(info), mode)


def _contained_path(root: Path, member: str) -> Path:
    """Resolve ``member`` under ``root``, refusing any escape."""
    candidate = (root / member).resolve()
    if candidate != root and root not in candidate.parents:
        raise ValueError(f"member escapes the extraction root: {member}")
    return candidate


def _write_file(target: Path, data: bytes, mode: int) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    target.chmod((mode & 0o777) or _DEFAULT_FILE_MODE)


def _write_symlink(root: Path, target: Path, link_target: str) -> None:
    resolved = (target.parent / link_target).resolve()
    if resolved != root and root not in resolved.parents:
        raise ValueError(f"symlink target escapes the root: {link_target}")
    target.parent.mkdir(parents=True, exist_ok=True)
    target.symlink_to(link_target)


def read_pinned_playwright_version(package_json: Path) -> str:
    """Return the exact ``playwright`` version declared in ``package_json``.

    Raises if the pin is a range rather than an exact version, so a Playwright
    upgrade cannot leave the driver, its API and the Chromium revision able to
    drift apart.
    """
    dependencies = json.loads(package_json.read_text()).get("dependencies", {})
    version = dependencies.get("playwright", "")
    if not _EXACT_VERSION.match(version):
        raise ValueError(
            f"playwright must be pinned to an exact version, got {version!r}"
        )
    return version


def browser_revision(browsers_json: Path, name: str) -> str:
    """Return the revision the vendored ``browsers.json`` records for ``name``.

    ``name`` is the upstream browser id (``chromium-headless-shell``), matched
    exactly against the ``browsers`` array rather than searched for, so a
    neighbouring entry sharing a revision is never mistaken for it.
    """
    document = json.loads(browsers_json.read_text())
    for entry in document.get("browsers", []):
        if entry.get("name") == name:
            return str(entry["revision"])
    raise ValueError(f"{name} is absent from {browsers_json}")


def assert_version_pairing(
    *,
    fetched_playwright_version: str,
    expected_playwright_version: str,
    fetched_chromium_revision: str,
    expected_chromium_revision: str,
) -> None:
    """Fail the release unless the fetched inputs match their pins.

    The Node/Chromium pairing is structural, so this guards the
    construction rather than testing compatibility after the fact.
    """
    if fetched_playwright_version != expected_playwright_version:
        raise ValueError(
            "fetched playwright "
            f"{fetched_playwright_version} != pinned "
            f"{expected_playwright_version}"
        )
    if fetched_chromium_revision != expected_chromium_revision:
        raise ValueError(
            "fetched Chromium revision "
            f"{fetched_chromium_revision} != pinned "
            f"{expected_chromium_revision}"
        )


@dataclass(frozen=True)
class NoticeSource:
    """One licensed component's redistribution notices."""

    component: str
    licence_files: tuple[Path, ...]


@dataclass(frozen=True)
class TreePlacement:
    """A source file or directory and where it lands within a tree."""

    source: Path
    dest_relpath: str


@dataclass(frozen=True)
class TreeSpec:
    """What one artifact tree is composed from."""

    artifact: str
    placements: tuple[TreePlacement, ...]
    notices: tuple[NoticeSource, ...]
    executables: tuple[str, ...] = ()
    # (source_name, dest_name) renames applied at the tree root after placement,
    # so a placed upstream directory's binary can carry the name the runtime
    # expects (Chromium ships `headless_shell`; the runtime resolves
    # `chrome-headless-shell`).
    renames: tuple[tuple[str, str], ...] = ()


@dataclass(frozen=True)
class ExtractedInputs:
    """The three extracted upstream trees, before composition."""

    playwright_core: Path
    node: Path
    chromium: Path


def write_notices(tree: Path, sources: Iterable[NoticeSource]) -> None:
    """Populate ``tree/NOTICES/<component>/`` from each source's files.

    A component contributing no licence file fails the release: NOTICES are the
    plan's substitute for a legal-review gate, so a silently dropped component
    must not ship.
    """
    for source in sources:
        if not source.licence_files:
            raise ValueError(
                f"component {source.component!r} has no licence files"
            )
        directory = tree / "NOTICES" / source.component
        directory.mkdir(parents=True, exist_ok=True)
        for licence in source.licence_files:
            shutil.copy2(licence, directory / licence.name)


def stage_tree(spec: TreeSpec, dest: Path) -> None:
    """Compose ``spec`` into ``dest``, preserving modes and symlinks.

    A placement with an empty ``dest_relpath`` copies a directory's contents
    into the tree root, so an artifact that *is* an upstream directory (the
    browser's headless-shell tree) needs no wrapper subdirectory.
    """
    dest.mkdir(parents=True, exist_ok=True)
    for placement in spec.placements:
        target = dest / placement.dest_relpath
        target.parent.mkdir(parents=True, exist_ok=True)
        if placement.source.is_dir():
            shutil.copytree(
                placement.source, target, symlinks=True, dirs_exist_ok=True
            )
        else:
            shutil.copy2(placement.source, target)
    for source_name, dest_name in spec.renames:
        (dest / source_name).rename(dest / dest_name)
    write_notices(dest, spec.notices)


def structural_check(
    tree: Path,
    *,
    executables: Iterable[str],
    notice_components: Iterable[str],
) -> None:
    """Fail unless every expected binary is executable and NOTICES populated.

    Cheap enough to run for every platform in the assembling job, so it covers
    the targets the execution smoke matrix cannot reach.
    """
    for name in executables:
        binary = tree / name
        if not binary.is_file():
            raise ValueError(f"expected binary {name} is missing")
        if not binary.stat().st_mode & 0o111:
            raise ValueError(f"expected binary {name} is not executable")
    for component in notice_components:
        directory = tree / "NOTICES" / component
        if not directory.is_dir() or not any(directory.iterdir()):
            raise ValueError(f"NOTICES/{component} is empty or missing")


def smoke_check(tree: Path, *, executables: Iterable[str]) -> None:
    """Fail unless every named binary in ``tree`` runs ``--version``.

    Executing the artifact is a stronger gate than extracting it — a
    correctly-signed, correctly-hashed but structurally-wrong tree passes every
    other check and this one refuses it — which is why it runs in a job holding
    no signing credentials.
    """
    for name in executables:
        binary = tree / name
        try:
            subprocess.run(
                [str(binary), "--version"],
                check=True,
                capture_output=True,
                timeout=_SMOKE_TIMEOUT,
            )
        except (OSError, subprocess.SubprocessError) as exc:
            raise ValueError(f"{name} did not execute: {exc}") from exc


def assemble_specs(
    specs: Iterable[TreeSpec],
    *,
    platform: str,
    staging_dir: Path,
    dist_dir: Path = RELEASE_STAGING,
) -> dict[str, ArchiveStats]:
    """Stage and pack each spec into a flat, deterministically-named archive.

    Staging happens under ``staging_dir`` (kept outside the checkout in CI) and
    the finished ``.tar.gz`` lands flat in ``dist_dir`` under the same asset
    name the launcher fetches, so the provenance attest globs cover it.
    """
    results: dict[str, ArchiveStats] = {}
    for spec in specs:
        tree = staging_dir / f"{spec.artifact}-{platform}"
        stage_tree(spec, tree)
        archive = tree_artifact_asset_path(spec.artifact, platform, dist_dir)
        results[spec.artifact] = write_deterministic_archive(tree, archive)
    return results


def assert_matches_pin(
    archive_path: Path,
    *,
    artifact: str,
    platform: str,
    pins_path: Path = PINS_TOML,
) -> None:
    """Fail unless ``archive_path`` hashes to its reviewed pin.

    Run from the release job's own clean checkout against bytes that arrived as
    an opaque artifact, so the check is a real boundary rather than a
    self-referential one.
    """
    actual = hashlib.sha256(archive_path.read_bytes()).hexdigest()
    expected = pins.expected_digest(artifact, platform, pins_path)
    if actual != expected:
        raise ValueError(
            f"{artifact} {platform}: assembled {actual} != pinned {expected}"
        )


SpecBuilder = Callable[[ExtractedInputs], "tuple[TreeSpec, ...]"]


def extract_tar(archive: Path, dest: Path) -> None:
    """Extract a ``.tar.gz``/``.tar.xz`` into ``dest`` under the data filter.

    The data filter refuses absolute paths, ``..`` traversals and unsafe member
    types, so an upstream tarball cannot write outside the destination.
    """
    dest.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, "r:*") as tar:
        tar.extractall(dest, filter="data")


def assemble_tree_artifacts(
    *,
    playwright_tarball: Path,
    node_tarball: Path,
    chromium_archive: Path,
    platform: str,
    staging_dir: Path,
    dist_dir: Path = RELEASE_STAGING,
    spec_builder: SpecBuilder,
    run_smoke: bool = True,
) -> dict[str, ArchiveStats]:
    """Extract, compose, pack, attest and gate the tree artifacts.

    ``spec_builder`` maps the extracted upstream trees to the composition — the
    version-specific layout, kept out of the orchestration so it is validated
    against the real ``playwright-core`` separately. Every produced tree is
    walked (structural) before it is trusted. ``run_smoke`` executes the
    binaries too; it is left off when assembling other platforms' archives on a
    host that cannot run them, and the per-platform matrix runs the smoke check
    natively instead.
    """
    extracted = ExtractedInputs(
        playwright_core=_extract_into(
            extract_tar, playwright_tarball, staging_dir / "extracted/pw"
        ),
        node=_extract_into(
            extract_tar, node_tarball, staging_dir / "extracted/node"
        ),
        chromium=_extract_into(
            extract_zip, chromium_archive, staging_dir / "extracted/chromium"
        ),
    )
    specs = spec_builder(extracted)
    stats = assemble_specs(
        specs,
        platform=platform,
        staging_dir=staging_dir / "trees",
        dist_dir=dist_dir,
    )
    for spec in specs:
        tree = staging_dir / "trees" / f"{spec.artifact}-{platform}"
        structural_check(
            tree,
            executables=spec.executables,
            notice_components=[source.component for source in spec.notices],
        )
        if run_smoke:
            smoke_check(tree, executables=spec.executables)
        archive = tree_artifact_asset_path(spec.artifact, platform, dist_dir)
        archive.with_name(archive.name + ".sealed").write_bytes(
            build_attestation(spec.artifact, platform, stats[spec.artifact])
        )
    return stats


def _extract_into(
    extractor: Callable[[Path, Path], None], archive: Path, dest: Path
) -> Path:
    extractor(archive, dest)
    return dest


# The binary each artifact tree carries at its root, for the smoke check.
ARTIFACT_EXECUTABLES: dict[str, tuple[str, ...]] = {
    "driver": ("node",),
    "browser": ("chrome-headless-shell",),
}


def _sole(paths: Iterable[Path], description: str) -> Path:
    matches = sorted(paths)
    if not matches:
        raise ValueError(
            f"expected {description} but found none — validate the extracted "
            "layout against the pinned playwright-core, Node and Chromium"
        )
    return matches[0]


def default_spec_builder(extracted: ExtractedInputs) -> tuple[TreeSpec, ...]:
    """Map the extracted upstream trees to the shipped driver/browser layout.

    The glob shapes follow upstream packaging (npm's ``package/`` root, Node's
    ``node-v<version>-<platform>/`` root, Chromium's headless-shell directory)
    and are validated against the real inputs in the release lane; a layout
    change fails loudly here rather than shipping a broken tree.
    """
    node_binary = _sole(
        extracted.node.glob("node-v*/bin/node"), "the Node binary"
    )
    node_licence = _sole(
        extracted.node.glob("node-v*/LICENSE"), "the Node licence"
    )
    package = extracted.playwright_core / "package"
    playwright_licence = _sole(
        [package / "LICENSE"] if (package / "LICENSE").exists() else [],
        "the playwright-core licence",
    )
    # Playwright's Chromium archive ships `chrome-<platform>/headless_shell`
    # (with support files beside it); the runtime resolves
    # `chrome-headless-shell` at the tree root, so the whole directory is placed
    # and the binary renamed.
    shell = _sole(
        extracted.chromium.glob("**/headless_shell"),
        "the chromium headless-shell binary",
    )
    # Playwright's headless-shell archive ships no licence, so the Chromium
    # licence is committed and sourced from the repo rather than the archive.
    if not CHROMIUM_LICENSE.is_file():
        raise ValueError(
            f"the committed Chromium licence is missing at {CHROMIUM_LICENSE}"
        )
    chromium_licence = CHROMIUM_LICENSE
    driver = TreeSpec(
        artifact="driver",
        placements=(
            TreePlacement(node_binary, "node"),
            TreePlacement(package, "node_modules/playwright-core"),
        ),
        notices=(
            NoticeSource("node", (node_licence,)),
            NoticeSource("playwright-core", (playwright_licence,)),
        ),
        executables=("node",),
    )
    browser = TreeSpec(
        artifact="browser",
        placements=(TreePlacement(shell.parent, ""),),
        notices=(NoticeSource("chromium", (chromium_licence,)),),
        executables=("chrome-headless-shell",),
        renames=(("headless_shell", "chrome-headless-shell"),),
    )
    return (driver, browser)


def smoke_downloaded_archives(dist_dir: Path, platform: str) -> None:
    """Extract each downloaded tree archive and execute its binary natively.

    Run per platform on a matching host, since executing the artifact is a
    stronger gate than extracting it — a signed, correctly-hashed but
    structurally-wrong tree passes every other check and this one refuses it.
    """
    for artifact, executables in ARTIFACT_EXECUTABLES.items():
        archive = tree_artifact_asset_path(artifact, platform, dist_dir)
        tree = dist_dir / f".smoke-{artifact}-{platform}"
        extract_tar(archive, tree)
        structural_check(
            tree,
            executables=executables,
            notice_components=(),
        )
        smoke_check(tree, executables=executables)
