"""Where a run builds its shared trees, and how runs keep out of each other's.

Each run holds an ``flock`` on ``<root>/<run-id>.lock`` for its lifetime, so a
later run can tell a killed run's leftovers from a live run's trees: only a
lock nobody holds is swept.
"""

import contextlib
import dataclasses
import enum
import os
import re
import shutil
import stat
import tempfile
import time
import uuid
from collections.abc import Callable, Iterable, Iterator, Mapping
from datetime import UTC, datetime
from pathlib import Path

from tasks.shared.locking import try_exclusive_flock
from tasks.shared.shared_fixtures.errors import SharedFixturesError
from tasks.shared.shared_fixtures.families import (
    FIXTURE_LOG_VARIABLE,
    HOLD_VARIABLE,
    WORKBENCHES_VARIABLE,
    Family,
)

RUN_ID = re.compile(
    r"\d{8}T\d{12}Z-[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}"
    r"-[0-9a-f]{12}"
)
_ENTRY = re.compile(
    rf"(?P<run>{RUN_ID.pattern})(?P<kind>|\.lock|\.lock\.tmp.*)", re.DOTALL
)
_LOCK = ".lock"
_TEMPORARY_LOCK = ".lock.tmp"
_TEMPORARY_LOCK_GRACE_SECONDS = 60
_PRIVATE = 0o700
_WRITABLE_BY_OTHERS = 0o022
_HOLD_TIMEOUT_SECONDS = 30 * 60
_HOLD_POLL_SECONDS = 1.0

type Flock = Callable[[int], bool]
type Link = Callable[[Path, Path], None]
type OpenLock = Callable[[Path], int]
type Lstat = Callable[[Path], os.stat_result]
type Rmtree = Callable[[Path], None]


def shared_fixtures_root() -> Path:
    return (
        Path(tempfile.gettempdir()).resolve()
        / f"accelerator-shared-fixtures-{os.getuid()}"
    )


def _new_run_id() -> str:
    return f"{datetime.now(UTC):%Y%m%dT%H%M%S%f}Z-{uuid.uuid4()}"


@dataclasses.dataclass
class HeldLock:
    path: Path
    descriptor: int

    def release(self) -> None:
        os.close(self.descriptor)


@dataclasses.dataclass
class FixtureRun:
    run_id: str
    directory: Path
    lock: HeldLock
    rmtree: Rmtree = shutil.rmtree

    def tree(self, family: Family) -> Path:
        return self.directory / family.value

    @property
    def workbenches(self) -> Path:
        return self.directory / "workbenches"

    def handover(self, families: Iterable[Family], log: Path) -> dict[str, str]:
        return {
            **{
                family.handover_variable: str(self.tree(family))
                for family in families
            },
            WORKBENCHES_VARIABLE: str(self.workbenches),
            FIXTURE_LOG_VARIABLE: str(log),
        }

    def close(self) -> None:
        removed = remove_leftover(self.directory, self.rmtree)
        self.lock.path.unlink(missing_ok=True)
        self.lock.release()
        if not removed:
            raise SharedFixturesError(
                f"could not remove {self.directory}; "
                "the next run's sweep removes it"
            )


_ANY_WRITE = stat.S_IWUSR | stat.S_IWGRP | stat.S_IWOTH


class _Liveness(enum.Enum):
    LIVE = enum.auto()
    ABANDONED = enum.auto()
    ORPHANED = enum.auto()
    UNKNOWN = enum.auto()


class FixtureRuns:
    def __init__(
        self,
        root: Path,
        *,
        flock: Flock,
        link: Link,
        open_lock: OpenLock,
        new_run_id: Callable[[], str],
        rmtree: Rmtree,
    ) -> None:
        self._root = root
        self._flock = flock
        self._link = link
        self._open_lock = open_lock
        self._new_run_id = new_run_id
        self._rmtree = rmtree

    @classmethod
    def open(
        cls,
        root: Path,
        *,
        lstat: Lstat = os.lstat,
        flock: Flock = try_exclusive_flock,
        link: Link = os.link,
        open_lock: OpenLock = lambda path: os.open(path, os.O_RDWR),
        new_run_id: Callable[[], str] = _new_run_id,
        rmtree: Rmtree = shutil.rmtree,
    ) -> FixtureRuns:
        _refuse_a_root_inside_a_checkout(root)
        _ensure_a_private_root(root, lstat)
        return cls(
            root,
            flock=flock,
            link=link,
            open_lock=open_lock,
            new_run_id=new_run_id,
            rmtree=rmtree,
        )

    def start(self) -> FixtureRun:
        run_id = self._new_run_id()
        lock = HeldLock(self._root / f"{run_id}{_LOCK}", self._locked(run_id))
        directory = self._root / run_id
        try:
            directory.mkdir(exist_ok=False)
            (directory / "workbenches").mkdir()
        except BaseException:
            lock.path.unlink(missing_ok=True)
            lock.release()
            raise
        return FixtureRun(run_id, directory, lock, self._rmtree)

    def sweep_abandoned(self) -> None:
        run_ids: set[str] = set()
        temporary_locks: list[Path] = []
        for name in sorted(entry.name for entry in self._root.iterdir()):
            entry = _ENTRY.fullmatch(name)
            if entry is None:
                _warn(
                    f"{self._root / name} is not a run's; remove it by hand "
                    "if no run is using it"
                )
            elif entry["kind"].startswith(_TEMPORARY_LOCK):
                temporary_locks.append(self._root / name)
            else:
                run_ids.add(entry["run"])
        for run_id in sorted(run_ids):
            self._sweep_run(run_id)
        for temporary_lock in temporary_locks:
            self._sweep_temporary_lock(temporary_lock)

    def live_run_ids(self) -> frozenset[str]:
        lock_names = (
            name
            for name in (entry.name for entry in self._root.iterdir())
            if (entry := _ENTRY.fullmatch(name)) and entry["kind"] == _LOCK
        )
        live: set[str] = set()
        for name in lock_names:
            run_id = name.removesuffix(_LOCK)
            with self._judging(run_id) as (liveness, _):
                if liveness in {_Liveness.LIVE, _Liveness.UNKNOWN}:
                    live.add(run_id)
        return frozenset(live)

    def _locked(self, run_id: str) -> int:
        lock = self._root / f"{run_id}{_LOCK}"
        temporary = self._root / f"{run_id}{_TEMPORARY_LOCK}{uuid.uuid4().hex}"
        descriptor = os.open(
            temporary, os.O_CREAT | os.O_EXCL | os.O_RDWR, 0o600
        )
        try:
            self._take(descriptor, temporary)
            self._link(temporary, lock)
        except SharedFixturesError:
            os.close(descriptor)
            raise
        except FileExistsError:
            os.close(descriptor)
            raise SharedFixturesError(
                f"the run lock {lock} already exists"
            ) from None
        except OSError as error:
            os.close(descriptor)
            raise SharedFixturesError(
                f"cannot lock a run under {self._root}: "
                f"{error}; the temp filesystem must support flock and hard "
                "links; set TMPDIR to one that does"
            ) from error
        finally:
            temporary.unlink(missing_ok=True)
        return descriptor

    def _take(self, descriptor: int, lock: Path) -> None:
        if not self._flock(descriptor):
            raise SharedFixturesError(
                f"could not take the new run lock {lock}, which nothing else "
                "should hold"
            )
        second_holder = os.open(lock, os.O_RDWR)
        try:
            if self._flock(second_holder):
                raise SharedFixturesError(
                    f"flock under {self._root} does not exclude a second "
                    "holder; the temp filesystem must support flock; set "
                    "TMPDIR to one that does"
                )
        finally:
            os.close(second_holder)

    def _sweep_run(self, run_id: str) -> None:
        directory = self._root / run_id
        with self._judging(run_id) as (liveness, reason):
            if liveness is _Liveness.UNKNOWN:
                _warn(f"kept {directory}: {reason}")
            elif liveness is _Liveness.ORPHANED:
                self._remove_or_warn(directory)
            elif liveness is _Liveness.ABANDONED:
                self._remove_or_warn(directory)
                (self._root / f"{run_id}{_LOCK}").unlink(missing_ok=True)

    @contextlib.contextmanager
    def _judging(self, run_id: str) -> Iterator[tuple[_Liveness, str]]:
        lock = self._root / f"{run_id}{_LOCK}"
        try:
            descriptor = self._open_lock(lock)
        except FileNotFoundError:
            yield _Liveness.ORPHANED, ""
            return
        except OSError as error:
            yield (
                _Liveness.UNKNOWN,
                f"its lock {lock} cannot be opened: {error}",
            )
            return
        try:
            try:
                acquired = self._flock(descriptor)
            except OSError as error:
                yield (
                    _Liveness.UNKNOWN,
                    f"its lock {lock} cannot be taken: {error}",
                )
                return
            yield (_Liveness.ABANDONED if acquired else _Liveness.LIVE), ""
        finally:
            os.close(descriptor)

    def _sweep_temporary_lock(self, temporary: Path) -> None:
        try:
            age = time.time() - temporary.lstat().st_mtime
            if age < _TEMPORARY_LOCK_GRACE_SECONDS:
                return
            descriptor = os.open(temporary, os.O_RDWR)
        except OSError:
            return
        try:
            if self._flock(descriptor):
                temporary.unlink(missing_ok=True)
        except OSError:
            pass
        finally:
            os.close(descriptor)

    def _remove_or_warn(self, directory: Path) -> None:
        if not remove_leftover(directory, self._rmtree):
            _warn(
                f"could not remove {directory}; remove it by hand if no run "
                "is using it"
            )


@dataclasses.dataclass(frozen=True)
class HoldPoint:
    directory: Path
    poll: float = _HOLD_POLL_SECONDS
    timeout: float = _HOLD_TIMEOUT_SECONDS

    @classmethod
    def from_environment(cls, environ: Mapping[str, str]) -> HoldPoint | None:
        directory = environ.get(HOLD_VARIABLE)
        if not directory:
            return None
        if environ.get("CI"):
            raise SharedFixturesError(
                f"{HOLD_VARIABLE} is set, but it is never honoured in CI"
            )
        return cls(Path(directory))

    def wait(self, run: Path) -> None:
        self.directory.mkdir(parents=True, exist_ok=True)
        (self.directory / "held").write_text(str(run))
        deadline = time.monotonic() + self.timeout
        while not (self.directory / "release").exists():
            if time.monotonic() >= deadline:
                raise SharedFixturesError(
                    f"nothing released {self.directory} "
                    f"within {self.timeout:g} s"
                )
            time.sleep(self.poll)


def remove_leftover(directory: Path, rmtree: Rmtree = shutil.rmtree) -> bool:
    """Remove ``directory``, making it writable and retrying once if needed.

    Returns whether it is gone.
    """
    with contextlib.suppress(FileNotFoundError):
        try:
            rmtree(directory)
        except OSError:
            _make_writable(directory)
            try:
                rmtree(directory)
            except OSError:
                return not directory.exists()
    return True


def seal(tree: Path) -> None:
    """Take every write permission from ``tree``, following no link.

    A test that writes into a shared tree then fails at the write, rather
    than only when the tree is digested again after every test has run.
    """
    for directory, _, files in os.walk(tree, topdown=False):
        for name in files:
            _without_write(Path(directory, name))
        _without_write(Path(directory))


def _without_write(path: Path) -> None:
    status = path.lstat()
    if stat.S_ISLNK(status.st_mode):
        return
    path.chmod(stat.S_IMODE(status.st_mode) & ~_ANY_WRITE)


def _make_writable(directory: Path) -> None:
    """Let the owner empty every directory under ``directory``.

    Removing an entry needs write on its directory, not on the entry, so only
    directories change; following no link keeps a file shared with something
    outside the tree at its own mode.
    """
    with contextlib.suppress(OSError):
        status = directory.lstat()
        if not stat.S_ISDIR(status.st_mode):
            return
        directory.chmod(stat.S_IMODE(status.st_mode) | stat.S_IRWXU)
        for entry in os.scandir(directory):
            if entry.is_dir(follow_symlinks=False):
                _make_writable(Path(entry.path))


def _refuse_a_root_inside_a_checkout(root: Path) -> None:
    for ancestor in root.absolute().parents:
        if (ancestor / ".git").exists() or (ancestor / ".jj").exists():
            raise SharedFixturesError(
                f"the shared root {root} lies inside the checkout at "
                f"{ancestor}; set TMPDIR to a directory outside any git or jj "
                "checkout"
            )


def _ensure_a_private_root(root: Path, lstat: Lstat) -> None:
    try:
        status = lstat(root)
    except FileNotFoundError:
        with contextlib.suppress(FileExistsError):
            root.mkdir(mode=_PRIVATE)
            root.chmod(_PRIVATE)
        status = lstat(root)
    mode = stat.S_IMODE(status.st_mode)
    problem = _root_problem(status, mode)
    if problem:
        raise SharedFixturesError(
            f"the shared root {root} {problem} (owner uid {status.st_uid}, "
            f"mode {mode:o}); move or remove {root} if you don't recognise "
            f"it; the next run recreates it with mode {_PRIVATE:04o}"
        )


def _root_problem(status: os.stat_result, mode: int) -> str | None:
    if stat.S_ISLNK(status.st_mode):
        return "is a symlink"
    if not stat.S_ISDIR(status.st_mode):
        return "is not a directory"
    if status.st_uid != os.getuid():
        return f"is owned by another user, not uid {os.getuid()}"
    if mode & _WRITABLE_BY_OTHERS:
        return "is writable by group or world"
    return None


def _warn(message: str) -> None:
    print(f"WARNING: shared VCS fixtures: {message}", flush=True)
