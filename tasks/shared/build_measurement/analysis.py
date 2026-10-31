"""Turning a captured session's lines into its record, after capture ends."""

import datetime as dt
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from tasks.shared.build_measurement.cpu_load import cpu_buckets, cpu_readings
from tasks.shared.build_measurement.fetch_failures import fetch_failures
from tasks.shared.build_measurement.host import (
    HostDescription,
    MiseDescription,
)
from tasks.shared.build_measurement.measured_build import (
    CommitMatch,
    MeasuredBuild,
    PrimingRun,
    RunFigures,
)
from tasks.shared.build_measurement.settings import BuildSettings
from tasks.shared.build_measurement.task_graph import TaskGraph
from tasks.shared.build_measurement.timeline import (
    StampedLine,
    Timeline,
    analyse_timeline,
)
from tasks.shared.build_measurement.tracked_processes import (
    TrackedProcess,
    cpu_used,
    process_snapshots,
)


@dataclass(frozen=True, slots=True)
class BuildResult:
    lines: tuple[StampedLine, ...]
    exit_status: int
    wall_clock_seconds: float


@dataclass(frozen=True, slots=True)
class CapturedInvocation:
    result: BuildResult
    graph: TaskGraph
    reported: tuple[str, ...]
    ready_from: float

    def timeline(self) -> Timeline:
        return analyse_timeline(
            self.result.lines,
            self.graph,
            self.reported,
            ready_from=self.ready_from,
        )


@dataclass(frozen=True, slots=True)
class CapturedRun:
    number: int
    invocations: tuple[CapturedInvocation, ...]
    cpu_lines: tuple[StampedLine, ...]
    process_lines: tuple[StampedLine, ...]
    wall_clock_seconds: float

    @property
    def exit_status(self) -> int:
        return next(
            (
                each.result.exit_status
                for each in self.invocations
                if each.result.exit_status != 0
            ),
            0,
        )

    @property
    def build_lines(self) -> tuple[StampedLine, ...]:
        return tuple(
            line for each in self.invocations for line in each.result.lines
        )


@dataclass(frozen=True, slots=True)
class CapturedSession:
    run_id: str
    run_directory: Path
    started_at: dt.datetime
    settings: BuildSettings
    commit: CommitMatch
    host: HostDescription
    mise: MiseDescription
    idle_load_percent: float | None
    task_set: tuple[str, ...]
    priming: PrimingRun | None
    runs: tuple[CapturedRun, ...]


def analyse_run(
    run: CapturedRun, tracked: Sequence[TrackedProcess]
) -> RunFigures:
    timeline = Timeline.merged(each.timeline() for each in run.invocations)
    sampled = process_snapshots(run.process_lines)
    return RunFigures.derived(
        index=run.number,
        wall_clock_seconds=run.wall_clock_seconds,
        exit_status=run.exit_status,
        tasks=timeline.tasks,
        cpu_buckets=cpu_buckets(
            cpu_readings(run.cpu_lines),
            timeline,
            until=run.wall_clock_seconds,
        ),
        process_cpu_seconds={
            each.name: cpu_used(sampled, each) for each in tracked
        },
        fetch_failures=fetch_failures(run.build_lines, run=run.number),
    )


def analyse_session(captured: CapturedSession) -> MeasuredBuild:
    settings = captured.settings
    return MeasuredBuild.derived(
        run_id=captured.run_id,
        run_directory=captured.run_directory,
        started_at=captured.started_at,
        cache_state=settings.cache_state,
        mode=settings.scope.mode,
        isolated_tasks=settings.scope.isolated_tasks,
        host_settings=settings.host_settings,
        commit=captured.commit,
        host=captured.host,
        mise=captured.mise,
        idle_load_percent=captured.idle_load_percent,
        task_set=captured.task_set,
        tracked=settings.tracked,
        priming=captured.priming,
        runs=tuple(analyse_run(run, settings.tracked) for run in captured.runs),
    )
