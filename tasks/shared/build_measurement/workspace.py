"""The only code that changes the checkout: touching and deleting."""

import contextlib
import os
import shutil
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.errors import (
    TouchTargetNotAFileError,
    UnsafeDeletionError,
)
from tasks.shared.build_measurement.locations import (
    RUN_DIRECTORY_ROOT,
    TRASH_PREFIX,
)
from tasks.shared.build_measurement.session_notices import (
    FoundTrash,
    UnremovedTrash,
)
from tasks.shared.paths import DEV_LAUNCHER_MARKER_NAME

_REMAINED = "the directory remained after removal"


def delete_from_disk(path: Path) -> BaseException | None:
    failures: list[BaseException] = []

    def collect(_call: object, _path: str, error: BaseException) -> None:
        failures.append(error)

    shutil.rmtree(path, onexc=collect)
    return failures[0] if failures else None


def size_on_disk(path: Path) -> int:
    total = 0
    for directory, directories, files in path.walk():
        for name in (*directories, *files):
            with contextlib.suppress(OSError):
                total += (directory / name).lstat().st_size
    return total


def _deletion_refused(path: Path, why: str) -> UnsafeDeletionError:
    return UnsafeDeletionError(
        f"refusing to delete {path}: {why}; nothing was deleted"
    )


@dataclass(frozen=True, slots=True)
class FilesystemWorkspace:
    root: Path
    remove_tree: Callable[[Path], BaseException | None] = delete_from_disk

    def has_dev_launcher_marker(self) -> bool:
        return (self.root / DEV_LAUNCHER_MARKER_NAME).exists()

    def verify(self, state: BuildCacheState) -> None:
        for path in state.touches:
            self._require_touchable(path)
        for path in state.deletions:
            self._require_deletable(path)

    def prepare_timed_run(self, state: BuildCacheState, *, trash: Path) -> None:
        self.verify(state)
        for path in state.touches:
            os.utime(self.root / path)
        if not state.deletions:
            return
        self._require_in_run_root(trash)
        trash.mkdir(exist_ok=False)
        self._set_aside(state.deletions, trash)

    def remove_trash(self, trash: Path) -> UnremovedTrash | None:
        self._require_in_run_root(trash)
        failure = self.remove_tree(trash)
        if not trash.exists():
            return None
        return UnremovedTrash(
            path=trash.absolute(),
            size_bytes=size_on_disk(trash),
            cause=_REMAINED if failure is None else str(failure),
        )

    def leftover_trash(self) -> tuple[FoundTrash, ...]:
        return tuple(
            FoundTrash(path=found.absolute(), size_bytes=size_on_disk(found))
            for found in sorted(
                (self.root / RUN_DIRECTORY_ROOT).glob(f"*/{TRASH_PREFIX}*")
            )
        )

    def _require_touchable(self, path: Path) -> None:
        target = self.root / path
        if target.is_symlink() or not target.is_file():
            raise TouchTargetNotAFileError(
                f"{path} is missing or not a regular file; --cache-state "
                f"{BuildCacheState.INCREMENTAL} touches it before each timed "
                "run"
            )

    def _require_deletable(self, path: Path) -> None:
        target = self.root / path
        if target.is_symlink():
            raise _deletion_refused(
                path, f"it is a symlink to {target.readlink()}"
            )
        root = self.root.resolve()
        if not target.resolve().is_relative_to(root):
            raise _deletion_refused(path, f"it resolves outside {root}")

    def _require_in_run_root(self, trash: Path) -> None:
        run_root = (self.root / RUN_DIRECTORY_ROOT).resolve()
        if not trash.resolve().is_relative_to(run_root):
            raise UnsafeDeletionError(
                f"refusing to move deletions into {trash}: it is outside "
                f"{run_root}; nothing was deleted"
            )

    def _set_aside(self, deletions: tuple[Path, ...], trash: Path) -> None:
        try:
            for path in deletions:
                self._move_aside(path, trash)
        except BaseException:
            self._put_back(deletions, trash)
            raise

    def _put_back(self, deletions: tuple[Path, ...], trash: Path) -> None:
        for path in deletions:
            aside = trash / path
            if aside.exists():
                aside.rename(self.root / path)
        self.remove_tree(trash)

    def _move_aside(self, path: Path, trash: Path) -> None:
        target = self.root / path
        if not target.exists():
            return
        aside = trash / path
        aside.parent.mkdir(parents=True, exist_ok=True)
        target.rename(aside)
