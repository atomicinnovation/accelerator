"""Background samplers that stamp what they read against a run's epoch."""

import io
import subprocess
import threading
from collections.abc import Callable, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import IO

from tasks.shared.build_measurement.timeline import StampedLine, stamped_text
from tasks.shared.build_measurement.tracked_processes import (
    TrackedProcess,
    gap_marker,
    sample_marker,
    tracked_line_filter,
)
from tasks.shared.clock import Clock

_JOIN_TIMEOUT_SECONDS = 5.0


def open_log(path: Path) -> IO[str]:
    return path.open("w", buffering=1, encoding="utf-8", newline="\n")


@dataclass(slots=True)
class _StampedLog:
    file: IO[str]
    lines: list[StampedLine] = field(default_factory=list)
    lock: threading.Lock = field(default_factory=threading.Lock)

    def append(self, line: StampedLine) -> None:
        with self.lock:
            self.lines.append(line)
            self.file.write(stamped_text(line) + "\n")

    def close(self) -> tuple[StampedLine, ...]:
        with self.lock:
            self.file.close()
            return tuple(self.lines)


class _RunningStream:
    def __init__(
        self, argv: Sequence[str], clock: Clock, epoch: float, log: Path
    ) -> None:
        self._stamped = _StampedLog(open_log(log))
        self._process = subprocess.Popen(
            argv, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL
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
        if stdout is None:
            return
        text = io.TextIOWrapper(
            stdout, encoding="utf-8", errors="replace", newline="\n"
        )
        for raw in text:
            line = StampedLine(clock.now() - epoch, raw.removesuffix("\n"))
            self._stamped.append(line)

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
        self._stamped = _StampedLog(open_log(log))
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
