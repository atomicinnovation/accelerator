import contextlib
import os
import subprocess
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from tasks.shared.build_measurement.analysis import BuildResult
from tasks.shared.build_measurement.errors import BuildLeftRunningError
from tasks.shared.build_measurement.log_files import StampedLog, open_log
from tasks.shared.build_measurement.session import BuildGroup
from tasks.shared.build_measurement.timeline import StampedLine
from tasks.shared.clock import Clock
from tasks.shared.processes import ProcessGroupOps, reap_session
from tasks.shared.signals import (
    UNWIND_SIGNALS,
    Interruption,
    deferred_signals,
    unwind_then_defer,
)


def _direct_run_environment() -> dict[str, str]:
    # An enclosing `mise run` exports MISE_TASK_* to its children, including
    # MISE_TASK_TIMINGS=0, which silences every per-task "Finished in" line.
    inherited = {
        name: value
        for name, value in os.environ.items()
        if not name.startswith("MISE_TASK_")
    }
    return inherited | {"MISE_TASK_TIMINGS": "1"}


@dataclass(frozen=True, slots=True)
class SubprocessBuildRunner:
    root: Path
    clock: Clock
    procs: ProcessGroupOps
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
                env=_direct_run_environment(),
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
        if child.stdout is None:
            return ()
        stamped = StampedLog(open_log(log))
        try:
            stamped.follow(child.stdout, self.clock, epoch)
        finally:
            lines = stamped.close()
        return lines

    def _end(
        self, child: subprocess.Popen[bytes], interruption: Interruption
    ) -> None:
        if interruption.arrived:
            self._settle(child)
        else:
            with deferred_signals(UNWIND_SIGNALS):
                self._settle(child)

    def _settle(self, child: subprocess.Popen[bytes]) -> None:
        self._reap(child)
        if self.procs.group_alive(child.pid):
            raise BuildLeftRunningError(child.pid)
        self.build_group.forget()

    def _reap(self, child: subprocess.Popen[bytes]) -> None:
        if child.poll() is None or self.procs.group_alive(child.pid):
            reap_session(child.pid, self.procs, self.clock, self.grace)
            with contextlib.suppress(subprocess.TimeoutExpired):
                child.wait(timeout=self.grace)
        if child.stdout is not None:
            child.stdout.close()
