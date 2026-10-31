"""Process identity and control backed by psutil + ``os.kill``."""

import contextlib
import os
import signal
from collections.abc import Callable, Iterable
from typing import Protocol

import psutil

from tasks.shared.clock import Clock

# A start time recorded by one psutil ``create_time()`` call and compared
# against another from the same source is expected to match exactly; this
# tolerance only absorbs float round-tripping (e.g. through JSON).
START_TIME_TOLERANCE = 0.5


def pid_identity_matches(
    pid: int, expected_start: float, *, tolerance: float
) -> bool:
    """Return whether the live process at ``pid`` has the expected start time.

    Guards against a recycled PID belonging to an unrelated process. Returns
    ``False`` if no process holds ``pid``. ``create_time()`` is captured and
    compared from the same psutil source on both platforms, so ``tolerance`` is
    an explicit argument (no baked-in ±1 s) — callers pass the value validated
    for the platform, defaulting to a sub-second match.
    """
    try:
        actual = psutil.Process(pid).create_time()
    except psutil.NoSuchProcess:
        return False
    return abs(actual - expected_start) <= tolerance


class ProcessOps(Protocol):
    """Process inspection + signalling, injected so callers stay testable."""

    def is_alive(self, pid: int) -> bool: ...
    def create_time(self, pid: int) -> float | None: ...
    def identity_matches(self, pid: int, start_time: float) -> bool: ...
    def children(self, pid: int) -> list[tuple[int, float]]: ...
    def terminate(self, pid: int) -> None: ...
    def kill(self, pid: int) -> None: ...
    def signal_group(self, pgid: int, number: int) -> None: ...
    def group_of(self, pid: int) -> int | None: ...
    def own_group(self) -> int: ...
    def group_alive(self, pgid: int) -> bool: ...
    def group_members(self, pgid: int) -> tuple[tuple[int, str], ...]: ...


class PsutilProcessOps:
    """Real ``ProcessOps`` backed by psutil + ``os.kill``."""

    def is_alive(self, pid: int) -> bool:
        # A killed-but-not-yet-reaped child reads as a zombie; treat it as dead
        # so teardown confirms death even when the killer is the parent process.
        try:
            return psutil.Process(pid).status() != psutil.STATUS_ZOMBIE
        except psutil.NoSuchProcess:
            return False

    def create_time(self, pid: int) -> float | None:
        try:
            return psutil.Process(pid).create_time()
        except psutil.NoSuchProcess:
            return None

    def identity_matches(self, pid: int, start_time: float) -> bool:
        return pid_identity_matches(
            pid, start_time, tolerance=START_TIME_TOLERANCE
        )

    def children(self, pid: int) -> list[tuple[int, float]]:
        try:
            proc = psutil.Process(pid)
            out = []
            for child in proc.children(recursive=True):
                try:
                    out.append((child.pid, child.create_time()))
                except psutil.NoSuchProcess:
                    continue
        except psutil.NoSuchProcess:
            return []
        else:
            return out

    def terminate(self, pid: int) -> None:
        with contextlib.suppress(ProcessLookupError, OSError):
            os.kill(pid, signal.SIGTERM)

    def kill(self, pid: int) -> None:
        with contextlib.suppress(ProcessLookupError, OSError):
            os.kill(pid, signal.SIGKILL)

    def signal_group(self, pgid: int, number: int) -> None:
        with contextlib.suppress(ProcessLookupError, PermissionError):
            os.killpg(pgid, number)

    def group_of(self, pid: int) -> int | None:
        try:
            return os.getpgid(pid)
        except ProcessLookupError:
            return None

    def own_group(self) -> int:
        return os.getpgrp()

    def group_alive(self, pgid: int) -> bool:
        # A group with no member this user may signal cannot be this user's.
        try:
            os.killpg(pgid, 0)
        except ProcessLookupError, PermissionError:
            return False
        return True

    def group_members(self, pgid: int) -> tuple[tuple[int, str], ...]:
        members: list[tuple[int, str]] = []
        for process in psutil.process_iter():
            try:
                if os.getpgid(process.pid) == pgid:
                    members.append((process.pid, _command_of(process)))
            except (
                ProcessLookupError,
                PermissionError,
                psutil.NoSuchProcess,
                psutil.AccessDenied,
            ):
                continue
        return tuple(members)


def _command_of(process: psutil.Process) -> str:
    return " ".join(process.cmdline()) or process.name()


_POLL_INTERVAL = 0.1


def escalate(
    targets: Iterable[int],
    procs: ProcessOps,
    clock: Clock,
    grace: float,
    *,
    relist: Callable[[], Iterable[int]],
) -> None:
    """Terminate ``targets``, then kill whatever outlives ``grace``.

    Each poll also terminates any process ``relist`` names that is not yet a
    target, so processes spawned during the grace period are not missed.
    """
    signalled = list(dict.fromkeys(targets))
    for pid in signalled:
        procs.terminate(pid)
    deadline = clock.now() + grace
    while (remaining := deadline - clock.now()) > 0:
        for pid in relist():
            if pid not in signalled:
                signalled.append(pid)
                procs.terminate(pid)
        if not any(procs.is_alive(pid) for pid in signalled):
            return
        clock.sleep(min(_POLL_INTERVAL, remaining))
    for pid in signalled:
        if procs.is_alive(pid):
            procs.kill(pid)


def reap_tree(pid: int, procs: ProcessOps, clock: Clock, grace: float) -> None:
    targets = [pid, *(child for child, _ in procs.children(pid))]
    escalate(targets, procs, clock, grace, relist=tuple)


def reap_session(
    leader: int, procs: ProcessOps, clock: Clock, grace: float
) -> None:
    """Reap ``leader``'s process group and every descendant it spawned.

    Descendants are snapshotted before any signal, since a dead leader has no
    children to list; they and later arrivals are escalated individually to
    reach those that left the group with ``setsid``. The closing SIGKILL to
    the group reaches members reparented after the leader died. A leader that
    does not lead its own group, or leads the caller's, is reaped as a tree so
    the caller never signals itself.
    """
    group = procs.group_of(leader)
    if group != leader or group == procs.own_group():
        reap_tree(leader, procs, clock, grace)
        return

    def descendants() -> list[int]:
        return [pid for pid, _ in procs.children(leader)]

    snapshot = descendants()
    procs.signal_group(leader, signal.SIGTERM)
    escalate([leader, *snapshot], procs, clock, grace, relist=descendants)
    procs.signal_group(leader, signal.SIGKILL)
