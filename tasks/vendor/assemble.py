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

from __future__ import annotations

import json
import re
import shutil
import stat
import subprocess
import zipfile
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

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

    Per ADR-0059 the Node/Chromium pairing is structural, so this guards the
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
    """Compose ``spec`` into ``dest``, preserving modes and symlinks."""
    dest.mkdir(parents=True, exist_ok=True)
    for placement in spec.placements:
        target = dest / placement.dest_relpath
        target.parent.mkdir(parents=True, exist_ok=True)
        if placement.source.is_dir():
            shutil.copytree(placement.source, target, symlinks=True)
        else:
            shutil.copy2(placement.source, target)
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
