"""Process identity and control backed by psutil + ``os.kill``."""

import contextlib
import os
import signal
from typing import Protocol

import psutil

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
