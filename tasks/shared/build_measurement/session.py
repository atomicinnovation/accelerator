"""One ``measure:build`` session: priming, timed runs, then analysis.

Every log is captured to disk before anything is analysed, so whatever ends
a session early leaves the logs taken so far behind.
"""

import datetime as dt
import shlex
from collections.abc import Callable, Sequence
from contextlib import AbstractContextManager, ExitStack
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

from tasks.shared.build_measurement.analysis import (
    BuildResult,
    CapturedInvocation,
    CapturedRun,
    CapturedSession,
    analyse_session,
)
from tasks.shared.build_measurement.cpu_load import cpu_readings, idle_load
from tasks.shared.build_measurement.errors import (
    HistoryError,
    MeasurementRefusedError,
)
from tasks.shared.build_measurement.fetch_failures import fetch_failures
from tasks.shared.build_measurement.host import (
    HostDescription,
    MiseDescription,
    observed_build_host,
    observed_mise,
    require_macos,
)
from tasks.shared.build_measurement.locations import RUN_DIRECTORY_ROOT
from tasks.shared.build_measurement.measured_build import (
    CommitMatch,
    PrimingRun,
)
from tasks.shared.build_measurement.provenance import (
    History,
    measured_commit,
    outside_measurements,
)
from tasks.shared.build_measurement.record import run_id
from tasks.shared.build_measurement.reports import (
    Preflight,
    SessionOutcome,
    preflight_report,
    run_end_line,
)
from tasks.shared.build_measurement.settings import (
    DEFAULT_TASK,
    BuildSettings,
    Priming,
    SessionRun,
    TimedRun,
    log_name,
)
from tasks.shared.build_measurement.task_graph import TaskGraph
from tasks.shared.build_measurement.timeline import (
    StampedLine,
    tasks_mise_reported_failing,
)
from tasks.shared.build_measurement.tracked_processes import (
    IDE,
    PS_ARGV,
    TrackedProcess,
    matching_commands,
    running_tracked,
)
from tasks.shared.clock import Clock
from tasks.shared.diagnostics import DiagnosticRunner

IDLE_SAMPLE_SECONDS = 30.0
IDLE_LOG = "idle-iostat.log"
CPU_LOG_SUFFIX = "-iostat"
PROCESS_LOG_SUFFIX = "-ps"


class BuildRunner(Protocol):
    def run(
        self, argv: Sequence[str], *, epoch: float, log: Path
    ) -> BuildResult: ...


class RunningSampler(Protocol):
    def stop(self) -> tuple[StampedLine, ...]: ...


class Sampler(Protocol):
    def start(self, *, epoch: float, log: Path) -> RunningSampler: ...


class ProcessSampler(Protocol):
    def start(
        self,
        *,
        epoch: float,
        log: Path,
        tracked: Sequence[TrackedProcess],
    ) -> RunningSampler: ...


@dataclass(frozen=True, slots=True)
class OrphanedBuild:
    pgid: int
    members: tuple[tuple[int, str], ...]
    path: Path


class BuildGroup(Protocol):
    def remember(self, pgid: int, create_time: float) -> None: ...
    def forget(self) -> None: ...
    def orphaned(self) -> OrphanedBuild | None: ...


class OrphanedBuildError(MeasurementRefusedError):
    def __init__(self, orphan: OrphanedBuild) -> None:
        members = "\n".join(
            f"  {pid} {command}" for pid, command in orphan.members
        )
        super().__init__(
            f"an earlier measure:build's build may still be running as "
            f"process group {orphan.pgid}:\n{members}\n"
            f"if these are an earlier measure:build's mise, cargo or "
            f"nextest, stop them with `kill -TERM -{orphan.pgid}` (then "
            f"`kill -KILL -{orphan.pgid}` for any that survive) and retry; "
            f"if none is, delete {orphan.path}"
        )


@dataclass(frozen=True, slots=True)
class BuildPorts:
    system: Callable[[], str]
    build: BuildRunner
    cpu_sampler: Sampler
    process_sampler: ProcessSampler
    history: History
    diagnostics: DiagnosticRunner
    kernel_hostname: Callable[[], str]
    mise_tasks: Callable[[], str]
    clock: Clock
    utc_now: Callable[[], dt.datetime]
    progress: Callable[[str], None]
    session_lock: Callable[[], AbstractContextManager[None]]
    build_group: BuildGroup


def measure_build(settings: BuildSettings, ports: BuildPorts) -> SessionOutcome:
    require_macos(ports.system())
    with ports.session_lock():
        orphan = ports.build_group.orphaned()
        if orphan is not None:
            raise OrphanedBuildError(orphan)
        return _Session(settings, ports).run()


def _run_hostname(host: HostDescription, ports: BuildPorts) -> str:
    return (
        host.hostname
        if isinstance(host.hostname, str)
        else (ports.kernel_hostname())
    )


@dataclass(frozen=True, slots=True)
class _Provenance:
    graph: TaskGraph
    commit: str
    start_changes: tuple[str, ...]
    mise: MiseDescription


class _Session:
    def __init__(self, settings: BuildSettings, ports: BuildPorts) -> None:
        self.settings = settings
        self.ports = ports
        self.started_at = ports.utc_now().replace(microsecond=0)
        self.host = observed_build_host(ports.diagnostics)
        self.run_id = run_id(
            self.started_at,
            _run_hostname(self.host, ports),
            cache_state=settings.cache_state,
            host_settings=settings.host_settings,
        )
        self.run_directory = RUN_DIRECTORY_ROOT / self.run_id
        self.logs = settings.root / self.run_directory

    def run(self) -> SessionOutcome:
        self.logs.mkdir(parents=True)
        self._progress(f"run directory: {self.logs}")
        try:
            return self._measure()
        except BaseException:
            self._progress(
                f"stopped; the logs captured so far are in {self.logs}"
            )
            raise

    def _progress(self, line: str) -> None:
        self.ports.progress(line)

    def _measure(self) -> SessionOutcome:
        provenance = self._provenance()
        self._preflight(provenance)
        idle_load_percent = self._idle_load()
        priming = self._priming(provenance.graph)
        runs = tuple(
            self._timed_run(TimedRun(number), provenance.graph)
            for number in range(1, self.settings.runs + 1)
        )
        captured = CapturedSession(
            run_id=self.run_id,
            run_directory=self.run_directory,
            started_at=self.started_at,
            settings=self.settings,
            commit=CommitMatch.of(
                provenance.commit,
                at_start=not provenance.start_changes,
                at_end=self._matches_at_end(provenance.commit),
            ),
            host=self.host,
            mise=provenance.mise,
            idle_load_percent=idle_load_percent,
            task_set=tuple(sorted(provenance.graph.closure(DEFAULT_TASK))),
            priming=priming,
            runs=runs,
        )
        return SessionOutcome(
            measured=analyse_session(captured), keep=self.settings.keep
        )

    def _provenance(self) -> _Provenance:
        history = self.ports.history
        commit = measured_commit(history)
        return _Provenance(
            graph=TaskGraph.parse(self.ports.mise_tasks()),
            commit=commit,
            start_changes=outside_measurements(
                history.working_copy_changes(commit)
            ),
            mise=observed_mise(self.ports.diagnostics),
        )

    def _preflight(self, provenance: _Provenance) -> None:
        try:
            ps_output = self.ports.diagnostics(PS_ARGV)
        except FileNotFoundError, PermissionError:
            ps_output = ""
        tracked = self.settings.tracked
        self._progress(
            preflight_report(
                Preflight(
                    commit=provenance.commit,
                    differing_paths=provenance.start_changes,
                    host_settings=self.settings.host_settings,
                    power_source=self.host.power.source,
                    mise_jobs=provenance.mise.jobs,
                    tracked=tracked,
                    running=running_tracked(ps_output, tracked),
                    ide_commands=tuple(
                        command
                        for each in tracked
                        if each.name == IDE
                        for command in matching_commands(ps_output, each)
                    ),
                    keep=self.settings.keep,
                )
            )
        )

    def _idle_load(self) -> float | None:
        ports = self.ports
        self._progress(f"idle sampling: {IDLE_SAMPLE_SECONDS:.0f} s")
        with ExitStack() as stack:
            sampler = ports.cpu_sampler.start(
                epoch=ports.clock.now(), log=self.logs / IDLE_LOG
            )
            stack.callback(sampler.stop)
            ports.clock.sleep(IDLE_SAMPLE_SECONDS)
            lines = sampler.stop()
        load = idle_load(cpu_readings(lines))
        self._progress(
            "idle load: " + ("not sampled" if load is None else f"{load:.1f}%")
        )
        return load

    def _build(
        self, run: SessionRun, graph: TaskGraph, epoch: float
    ) -> tuple[CapturedInvocation, ...]:
        captured: list[CapturedInvocation] = []
        for invocation in self.settings.scope.invocations(graph):
            ready_from = self.ports.clock.now() - epoch
            log = self.logs / log_name(run, invocation.log_suffix)
            self._progress(f"  {shlex.join(invocation.argv)}: tail -f {log}")
            result = self.ports.build.run(invocation.argv, epoch=epoch, log=log)
            captured.append(
                CapturedInvocation(
                    result=result,
                    graph=invocation.graph,
                    reported=invocation.reported,
                    ready_from=ready_from,
                )
            )
        return tuple(captured)

    def _priming(self, graph: TaskGraph) -> PrimingRun | None:
        if not self.settings.cache_state.primes:
            return None
        self._progress("priming build")
        epoch = self.ports.clock.now()
        self._build(Priming(), graph, epoch)
        wall_clock = self.ports.clock.now() - epoch
        self._progress(f"priming: {wall_clock:.1f} s (discarded)")
        return PrimingRun(wall_clock_seconds=wall_clock, discarded=True)

    def _timed_run(self, run: TimedRun, graph: TaskGraph) -> CapturedRun:
        ports = self.ports
        self._progress(f"timed run {run.number}/{self.settings.runs}")
        epoch = ports.clock.now()
        with ExitStack() as stack:
            cpu = ports.cpu_sampler.start(
                epoch=epoch, log=self.logs / log_name(run, CPU_LOG_SUFFIX)
            )
            stack.callback(cpu.stop)
            processes = ports.process_sampler.start(
                epoch=epoch,
                log=self.logs / log_name(run, PROCESS_LOG_SUFFIX),
                tracked=self.settings.tracked,
            )
            stack.callback(processes.stop)
            invocations = self._build(run, graph, epoch)
            wall_clock = ports.clock.now() - epoch
            captured = CapturedRun(
                number=run.number,
                invocations=invocations,
                cpu_lines=cpu.stop(),
                process_lines=processes.stop(),
                wall_clock_seconds=wall_clock,
            )
        self._progress(
            run_end_line(
                run.number,
                self.settings.runs,
                wall_clock,
                failed=tasks_mise_reported_failing(captured.build_lines),
                fetch_failures=fetch_failures(
                    captured.build_lines, run=run.number
                ),
                keep=self.settings.keep,
            )
        )
        return captured

    def _matches_at_end(self, commit: str) -> bool | None:
        try:
            changes = self.ports.history.working_copy_changes(commit)
        except HistoryError as error:
            self._progress(
                f"warning: the end-of-run match check failed: {error}"
            )
            return None
        return not outside_measurements(changes)
