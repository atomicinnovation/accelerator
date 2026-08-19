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

import stat
import zipfile
from pathlib import Path

_DEFAULT_FILE_MODE = 0o644


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
