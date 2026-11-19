import dataclasses
import hashlib
import os
import stat
from collections.abc import Iterator, Mapping, Sequence
from pathlib import Path


def compute_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(64 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


@dataclasses.dataclass(frozen=True)
class TreeChanges:
    added: Sequence[str]
    removed: Sequence[str]
    modified: Sequence[str]


@dataclasses.dataclass(frozen=True)
class TreeDigest:
    """Each entry of a tree by relative path, hashed by content and mode."""

    entries: Mapping[str, str]

    @property
    def value(self) -> str:
        overall = hashlib.sha256()
        for path in sorted(self.entries):
            overall.update(f"{path}\0{self.entries[path]}\n".encode())
        return overall.hexdigest()

    def changes_since(self, earlier: TreeDigest) -> TreeChanges:
        return TreeChanges(
            added=tuple(sorted(self.entries.keys() - earlier.entries.keys())),
            removed=tuple(sorted(earlier.entries.keys() - self.entries.keys())),
            modified=tuple(
                sorted(
                    path
                    for path in self.entries.keys() & earlier.entries.keys()
                    if self.entries[path] != earlier.entries[path]
                )
            ),
        )


def tree_digest(tree: Path) -> TreeDigest:
    """Digest ``tree`` without following a link or opening a non-file.

    Opening a FIFO blocks until a writer appears, so only regular files are
    read.
    """
    return TreeDigest(
        {
            path.relative_to(tree).as_posix(): _entry_hash(path)
            for path in _walk(tree)
        }
    )


def _walk(directory: Path) -> Iterator[Path]:
    for entry in sorted(os.scandir(directory), key=lambda entry: entry.name):
        path = Path(entry.path)
        yield path
        if entry.is_dir(follow_symlinks=False):
            yield from _walk(path)


def _entry_hash(path: Path) -> str:
    status = path.lstat()
    mode = f"{stat.S_IMODE(status.st_mode):o}"
    if stat.S_ISLNK(status.st_mode):
        return f"link:{path.readlink()}"
    if stat.S_ISREG(status.st_mode):
        return f"file:{mode}:{compute_sha256(path)}"
    if stat.S_ISDIR(status.st_mode):
        return f"directory:{mode}"
    return f"type-{stat.S_IFMT(status.st_mode):o}:{mode}"
