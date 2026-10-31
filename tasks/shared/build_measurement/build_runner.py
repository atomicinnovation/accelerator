import contextlib
import io
import subprocess
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import IO

from tasks.shared.build_measurement.analysis import BuildResult
from tasks.shared.build_measurement.errors import BuildLeftRunningError
from tasks.shared.build_measurement.log_files import open_log
from tasks.shared.build_measurement.session import BuildGroup
from tasks.shared.build_measurement.timeline import StampedLine, stamped_text
from tasks.shared.clock import Clock
from tasks.shared.processes import ProcessOps, reap_session
from tasks.shared.signals import (
    UNWIND_SIGNALS,
    Interruption,
    deferred_signals,
    unwind_then_defer,
)


def _decoded(stdout: IO[bytes]) -> io.TextIOWrapper:
    # newline="\n": a lone \r (a cargo progress bar redrawn under a pty)
    # must not end a line, or the next segment loses its [task] prefix.
    return io.TextIOWrapper(
        stdout, encoding="utf-8", errors="replace", newline="\n"
    )


@dataclass(frozen=True, slots=True)
class SubprocessBuildRunner:
    root: Path
    clock: Clock
    procs: ProcessOps
    build_group: BuildGroup
    grace: float = 5.0

    def run(
        self, argv: Sequence[str], *, epoch: float, log: Path
    ) -> BuildResult:
        started = self.clock.now()
        with unwind_then_defer(UNWIND_SIGNALS) as interruption:
            child = subprocess.Popen(
                argv,
                cwd=self.root,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            try:
                self._remember(child.pid)
                lines = self._stream(child, epoch=epoch, log=log)
                exit_status = child.wait()
            finally:
                self._end(child, interruption)
        return BuildResult(
            lines=lines,
            exit_status=exit_status,
            wall_clock_seconds=self.clock.now() - started,
        )

    def _remember(self, pid: int) -> None:
        create_time = self.procs.create_time(pid)
        if create_time is not None:
            self.build_group.remember(pid, create_time)

    def _stream(
        self, child: subprocess.Popen[bytes], *, epoch: float, log: Path
    ) -> tuple[StampedLine, ...]:
        lines: list[StampedLine] = []
        if child.stdout is None:
            return ()
        with open_log(log) as file:
            for raw in _decoded(child.stdout):
                line = StampedLine(self.clock.now() - epoch, raw.rstrip("\n"))
                file.write(stamped_text(line) + "\n")
                lines.append(line)
        return tuple(lines)

    def _end(
        self, child: subprocess.Popen[bytes], interruption: Interruption
    ) -> None:
        if interruption.arrived:
            self._reap(child)
        else:
            with deferred_signals(UNWIND_SIGNALS):
                self._reap(child)
        if self.procs.group_alive(child.pid):
            raise BuildLeftRunningError(child.pid)

    def _reap(self, child: subprocess.Popen[bytes]) -> None:
        if child.poll() is None:
            reap_session(child.pid, self.procs, self.clock, self.grace)
            with contextlib.suppress(subprocess.TimeoutExpired):
                child.wait(timeout=self.grace)
        if child.stdout is not None:
            child.stdout.close()
        self.build_group.forget()
