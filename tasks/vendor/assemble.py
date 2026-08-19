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
import stat
import zipfile
from pathlib import Path

_DEFAULT_FILE_MODE = 0o644
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
