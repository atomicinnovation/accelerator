"""Non-blocking per-path advisory lock.

``fcntl.flock`` is the primary mechanism (auto-released if the holder dies); a
``mkdir``-with-recorded-owner fallback covers the rare filesystem where
``flock`` is unavailable, and reclaims a stale lock left by a SIGKILLed holder.
"""

import contextlib
import json
import os
import shutil
import uuid
from pathlib import Path

import psutil

from tasks.shared.processes import START_TIME_TOLERANCE, pid_identity_matches


def _try_import_fcntl():
    try:
        import fcntl
    except ImportError:
        return None
    return fcntl


@contextlib.contextmanager
def workspace_lock(path: Path | str):
    """Non-blocking lock over ``path``; yields whether it was acquired.

    Primary path is ``fcntl.flock(LOCK_EX | LOCK_NB)`` on a held fd (available
    on macOS and Linux, auto-released if the holder dies). A ``mkdir``-with-
    recorded-owner fallback fires only when ``flock`` is unavailable on the
    filesystem (raises ``OSError``) or the module is absent.
    """
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fcntl = _try_import_fcntl()
    if fcntl is None:
        with _mkdir_lock(path) as acquired:
            yield acquired
        return

    fd = os.open(str(path), os.O_CREAT | os.O_RDWR, 0o600)
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            yield False
            return
        except OSError:
            # flock unsupported on this filesystem — fall back to mkdir.
            os.close(fd)
            fd = None
            with _mkdir_lock(path) as acquired:
                yield acquired
            return
        try:
            yield True
        finally:
            with contextlib.suppress(OSError):
                fcntl.flock(fd, fcntl.LOCK_UN)
    finally:
        if fd is not None:
            os.close(fd)


@contextlib.contextmanager
def _mkdir_lock(path: Path):
    lock_dir = path.with_name(path.name + ".d")
    acquired = _acquire_mkdir(lock_dir)
    try:
        yield acquired
    finally:
        if acquired:
            shutil.rmtree(lock_dir, ignore_errors=True)


def _acquire_mkdir(lock_dir: Path) -> bool:
    # Create the lock dir already-populated and rename it into place atomically,
    # so there is never an owner-less window a contender could mis-read as stale.
    staged = lock_dir.with_name(f"{lock_dir.name}.stage.{uuid.uuid4().hex}")
    staged.mkdir(parents=True)
    _write_owner(staged / "owner.json")
    try:
        os.rename(staged, lock_dir)  # atomic; fails if lock_dir already exists
        return True
    except OSError:
        shutil.rmtree(staged, ignore_errors=True)
        return _try_reclaim(lock_dir)


def _try_reclaim(lock_dir: Path) -> bool:
    owner = _read_owner(lock_dir / "owner.json")
    if owner is not None and _owner_alive(owner):
        return False  # genuinely held
    # Provably stale (dead/unreadable owner). Arbitrate by renaming the stale
    # dir away — os.rename is atomic, so exactly one contender wins the move.
    graveyard = lock_dir.with_name(f"{lock_dir.name}.stale.{uuid.uuid4().hex}")
    try:
        os.rename(lock_dir, graveyard)
    except OSError:
        return False  # lost the race or it vanished; treat as held
    # Re-stat after the rename: back off if we moved a lock another contender
    # reclaimed (made live) in the window between our read and our rename.
    moved = _read_owner(graveyard / "owner.json")
    if moved is not None and _owner_alive(moved):
        try:
            os.rename(graveyard, lock_dir)
        except OSError:
            shutil.rmtree(graveyard, ignore_errors=True)
        return False
    shutil.rmtree(graveyard, ignore_errors=True)
    return _acquire_mkdir(lock_dir)


def _write_owner(owner_file: Path) -> None:
    pid = os.getpid()
    try:
        start: float | None = psutil.Process(pid).create_time()
    except psutil.NoSuchProcess:
        start = None
    owner_file.write_text(json.dumps({"pid": pid, "start_time": start}))


def _read_owner(owner_file: Path) -> tuple[int, float | None] | None:
    try:
        data = json.loads(owner_file.read_text())
        start = data["start_time"]
        return (int(data["pid"]), float(start) if start is not None else None)
    except (OSError, ValueError, KeyError, TypeError):
        return None


def _owner_alive(owner: tuple[int, float | None]) -> bool:
    pid, start = owner
    if start is None:
        return psutil.pid_exists(pid)
    return pid_identity_matches(pid, start, tolerance=START_TIME_TOLERANCE)
