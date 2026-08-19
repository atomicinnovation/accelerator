r"""Deterministic tree archives and the ``.files`` table they carry.

An archive is a gzipped tar whose first member is the ``.files`` table — one
row per other entry — so the launcher can verify each member against its row as
it extracts, in one pass, and refuse an archive whose first member is not the
table. Determinism is a precondition for pinning: entries are emitted in sorted
order with fixed mtime/uid/gid/owner and masked modes, and the gzip member
carries no embedded timestamp, so the same inputs produce byte-identical bytes
and a release is auditable by anyone who can run the same pins.

The table's row format is the wire contract with the launcher's Rust reader:
``<kind>\\t<octal-mode>\\t<size>\\t<digest-or-dash>\\t<path>[\\t<link-target>]``,
preceded by a ``version N`` line. Both sides hash the same table bytes, so no
agreement about the table's internal shape is needed beyond this reader parsing
what this writer emits.
"""

from __future__ import annotations

import gzip
import hashlib
import io
import stat
import tarfile
from dataclasses import dataclass
from pathlib import Path

TABLE_NAME = ".files"
TABLE_FORMAT_VERSION = 1

# The modes the launcher enforces; assembly masks to the same values so a
# verify computes the expected sealed mode from the recorded one.
_DIR_MODE = 0o755
_EXEC_MODE = 0o755
_FILE_MODE = 0o644

# Fixed ownership and timestamps, so nothing varies between runs or hosts.
_MTIME = 0
_UID = 0
_GID = 0
_UNAME = ""
_GNAME = ""
_CHUNK = 64 * 1024


@dataclass(frozen=True)
class ArchiveStats:
    """What a produced archive measures, for the manifest and attestation."""

    archive_sha256: str
    archive_size: int
    uncompressed_size: int
    entry_count: int
    table_sha256: str


@dataclass(frozen=True)
class _Entry:
    kind: str  # "f", "d", or "l"
    path: str
    mode: int
    size: int
    digest: str | None
    link_target: str | None


def masked_mode(is_dir: bool, is_executable: bool) -> int:
    """Return the mode assembly records for an entry."""
    if is_dir:
        return _DIR_MODE
    return _EXEC_MODE if is_executable else _FILE_MODE


def build_files_table(entries: list[_Entry]) -> bytes:
    """Render the ``.files`` table for ``entries`` in the launcher's format."""
    lines = [f"version {TABLE_FORMAT_VERSION}"]
    for entry in sorted(entries, key=lambda item: item.path):
        digest = entry.digest if entry.kind == "f" else "-"
        row = [
            entry.kind,
            format(entry.mode, "o"),
            str(entry.size),
            digest or "-",
            entry.path,
        ]
        if entry.kind == "l":
            row.append(entry.link_target or "")
        lines.append("\t".join(row))
    return ("\n".join(lines) + "\n").encode("utf-8")


def write_deterministic_archive(tree: Path, dest: Path) -> ArchiveStats:
    """Pack ``tree`` into a deterministic ``.tar.gz`` at ``dest``.

    Regular files, directories and in-tree symlinks are admitted; anything else
    (device, fifo, hardlink) raises, since the launcher would refuse it anyway
    and a silent drop would ship an incomplete tree.
    """
    entries = _scan(tree)
    table = build_files_table(entries)

    tar_bytes = io.BytesIO()
    with tarfile.open(fileobj=tar_bytes, mode="w") as archive:
        _append_bytes(archive, TABLE_NAME, table, _FILE_MODE)
        for entry in sorted(entries, key=lambda item: item.path):
            _append_entry(archive, tree, entry)
    raw = tar_bytes.getvalue()

    # gzip without the embedded mtime (mtime=0), so the container is stable.
    dest.parent.mkdir(parents=True, exist_ok=True)
    with (
        dest.open("wb") as handle,
        gzip.GzipFile(
            filename="",
            fileobj=handle,
            mode="wb",
            mtime=0,
            compresslevel=9,
        ) as gz,
    ):
        gz.write(raw)
    archive_bytes = dest.read_bytes()

    return ArchiveStats(
        archive_sha256=hashlib.sha256(archive_bytes).hexdigest(),
        archive_size=len(archive_bytes),
        uncompressed_size=sum(
            entry.size for entry in entries if entry.kind == "f"
        ),
        entry_count=len(entries),
        table_sha256=hashlib.sha256(table).hexdigest(),
    )


def _scan(tree: Path) -> list[_Entry]:
    entries: list[_Entry] = []
    for path in sorted(tree.rglob("*")):
        relative = path.relative_to(tree).as_posix()
        info = path.lstat()
        if stat.S_ISLNK(info.st_mode):
            target = _read_link_contained(tree, path)
            entries.append(_Entry("l", relative, 0o777, 0, None, target))
        elif path.is_dir():
            entries.append(_Entry("d", relative, _DIR_MODE, 0, None, None))
        elif path.is_file():
            executable = bool(info.st_mode & 0o111)
            mode = masked_mode(is_dir=False, is_executable=executable)
            digest, size = _digest_and_size(path)
            entries.append(_Entry("f", relative, mode, size, digest, None))
        else:
            raise ValueError(
                f"{relative} is neither a file, directory nor symlink"
            )
    return entries


def _read_link_contained(tree: Path, link: Path) -> str:
    """Return the link's target, refused if it escapes the tree.

    The launcher enforces containment too, but a producer that emitted an
    escaping symlink would fail the release at the launcher rather than at
    assembly, which is the worse place to learn of it.
    """
    target = link.readlink().as_posix()
    resolved = (link.parent / target).resolve()
    root = tree.resolve()
    if resolved != root and root not in resolved.parents:
        raise ValueError(f"symlink {link} escapes the tree: {target}")
    return target


def _digest_and_size(path: Path) -> tuple[str, int]:
    hasher = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        while chunk := handle.read(_CHUNK):
            hasher.update(chunk)
            size += len(chunk)
    return hasher.hexdigest(), size


def _append_bytes(
    archive: tarfile.TarFile, name: str, data: bytes, mode: int
) -> None:
    info = _tarinfo(name, tarfile.REGTYPE, mode, len(data))
    archive.addfile(info, io.BytesIO(data))


def _append_entry(archive: tarfile.TarFile, tree: Path, entry: _Entry) -> None:
    if entry.kind == "d":
        info = _tarinfo(entry.path, tarfile.DIRTYPE, entry.mode, 0)
        archive.addfile(info)
    elif entry.kind == "l":
        info = _tarinfo(entry.path, tarfile.SYMTYPE, 0o777, 0)
        info.linkname = entry.link_target or ""
        archive.addfile(info)
    else:
        data = (tree / entry.path).read_bytes()
        info = _tarinfo(entry.path, tarfile.REGTYPE, entry.mode, len(data))
        archive.addfile(info, io.BytesIO(data))


def _tarinfo(
    name: str, typeflag: bytes, mode: int, size: int
) -> tarfile.TarInfo:
    info = tarfile.TarInfo(name)
    info.type = typeflag
    info.mode = mode
    info.size = size
    info.mtime = _MTIME
    info.uid = _UID
    info.gid = _GID
    info.uname = _UNAME
    info.gname = _GNAME
    return info
