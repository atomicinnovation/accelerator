"""Background samplers that stamp what they read against a run's epoch."""

import os
import subprocess
import threading
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from pathlib import Path

from tasks.shared.build_measurement.log_files import StampedLog, open_log
from tasks.shared.build_measurement.timeline import StampedLine
from tasks.shared.build_measurement.tracked_processes import (
    TrackedProcess,
    gap_marker,
    sample_marker,
    tracked_line_filter,
)
from tasks.shared.clock import Clock

_JOIN_TIMEOUT_SECONDS = 5.0


def _c_locale() -> dict[str, str]:
    # ps and iostat format decimals by locale; their readings are parsed as C.
    return os.environ | {"LC_ALL": "C"}


class _RunningStream:
    def __init__(
        self, argv: Sequence[str], clock: Clock, epoch: float, log: Path
    ) -> None:
        self._stamped = StampedLog(open_log(log))
        self._process = subprocess.Popen(
            argv,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            env=_c_locale(),
        )
        self._stopped: tuple[StampedLine, ...] | None = None
        self._thread = threading.Thread(
            target=self._read, args=(clock, epoch), daemon=True
        )
        self._thread.start()

    @property
    def pid(self) -> int:
        return self._process.pid

    def _read(self, clock: Clock, epoch: float) -> None:
        stdout = self._process.stdout
        if stdout is not None:
            self._stamped.follow(stdout, clock, epoch)

    def stop(self) -> tuple[StampedLine, ...]:
        if self._stopped is None:
            self._process.terminate()
            try:
                self._process.wait(timeout=_JOIN_TIMEOUT_SECONDS)
            except subprocess.TimeoutExpired:
                self._process.kill()
                self._process.wait(timeout=_JOIN_TIMEOUT_SECONDS)
            self._thread.join(timeout=_JOIN_TIMEOUT_SECONDS)
            self._stopped = self._stamped.close()
        return self._stopped


@dataclass(frozen=True, slots=True)
class StreamSampler:
    argv: tuple[str, ...]
    clock: Clock

    def start(self, *, epoch: float, log: Path) -> _RunningStream:
        return _RunningStream(self.argv, self.clock, epoch, log)


class _RunningTicks:
    def __init__(
        self,
        argv: Sequence[str],
        clock: Clock,
        epoch: float,
        log: Path,
        keep: Callable[[str], bool],
        interval: float,
        tick_timeout: float,
    ) -> None:
        self._argv = tuple(argv)
        self._clock = clock
        self._epoch = epoch
        self._keep = keep
        self._interval = interval
        self._tick_timeout = tick_timeout
        self._stamped = StampedLog(open_log(log))
        self._stopping = threading.Event()
        self._stopped: tuple[StampedLine, ...] | None = None
        self._thread = threading.Thread(target=self._tick, daemon=True)
        self._thread.start()

    def _snapshot(self) -> str | None:
        try:
            completed = subprocess.run(
                self._argv,
                capture_output=True,
                check=False,
                timeout=self._tick_timeout,
                env=_c_locale(),
            )
        except subprocess.TimeoutExpired, OSError:
            return None
        if completed.returncode != 0:
            return None
        return completed.stdout.decode("utf-8", errors="replace")

    def _tick(self) -> None:
        while not self._stopping.is_set():
            at = self._clock.now() - self._epoch
            output = self._snapshot()
            if output is None:
                self._stamped.append(gap_marker(at))
            else:
                self._stamped.append(sample_marker(at))
                for raw in output.splitlines():
                    if self._keep(raw):
                        self._stamped.append(StampedLine(at, raw))
            self._stopping.wait(self._interval)

    def stop(self) -> tuple[StampedLine, ...]:
        if self._stopped is None:
            self._stopping.set()
            self._thread.join(
                timeout=self._tick_timeout + _JOIN_TIMEOUT_SECONDS
            )
            self._stopped = self._stamped.close()
        return self._stopped


@dataclass(frozen=True, slots=True)
class PsProcessSampler:
    argv: tuple[str, ...]
    clock: Clock
    interval: float
    tick_timeout: float

    def start(
        self,
        *,
        epoch: float,
        log: Path,
        tracked: Sequence[TrackedProcess],
    ) -> _RunningTicks:
        return _RunningTicks(
            self.argv,
            self.clock,
            epoch,
            log,
            tracked_line_filter(tracked),
            self.interval,
            self.tick_timeout,
        )
