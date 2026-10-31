"""The build record's schema: what one ``measure:build`` session measured."""

import datetime as dt
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from enum import StrEnum, unique
from pathlib import Path
from typing import Literal, Self

from pydantic import AwareDatetime, field_serializer, model_validator

from tasks.shared.build_measurement.across_runs import (
    ProcessAcrossRuns,
    Spread,
    TaskAcrossRuns,
    process_across_runs,
    spread,
    task_across_runs,
)
from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.cpu_load import CpuBucket
from tasks.shared.build_measurement.fetch_failures import FetchFailure
from tasks.shared.build_measurement.host import (
    HostDescription,
    MiseDescription,
)
from tasks.shared.build_measurement.record_model import NoReading, RecordModel
from tasks.shared.build_measurement.timeline import (
    CargoUnits,
    TaskFigures,
    Timeline,
)
from tasks.shared.build_measurement.tracked_processes import TrackedProcess

SCHEMA_VERSION = 1


@unique
class HostSettings(StrEnum):
    UNSPECIFIED = "unspecified"
    NONE = "none"
    DEVTOOLS = "devtools"
    DEVTOOLS_AND_IDE_EXCLUSION = "devtools+ide-exclusion"


@unique
class Mode(StrEnum):
    FULL = "full"


CONDITIONS_MIRRORING_THE_HOST = frozenset(
    {"hostname", "model", "os_version", "mise_jobs"}
)


@dataclass(frozen=True, slots=True)
class ConditionDifference:
    field: str
    previous: object
    current: object


def _differs(previous: object, current: object) -> bool:
    unread = isinstance(previous, NoReading) or isinstance(current, NoReading)
    return unread or previous != current


class MeasurementConditions(RecordModel):
    hostname: str | NoReading
    model: str | NoReading
    os_version: str | NoReading
    mise_jobs: int | NoReading
    cache_state: BuildCacheState
    mode: Mode
    isolated_tasks: tuple[str, ...]
    host_settings: HostSettings

    @classmethod
    def of(
        cls,
        host: HostDescription,
        mise: MiseDescription,
        *,
        cache_state: BuildCacheState,
        mode: Mode,
        isolated_tasks: tuple[str, ...],
        host_settings: HostSettings,
    ) -> Self:
        return cls(
            hostname=host.hostname,
            model=host.model,
            os_version=host.os_version,
            mise_jobs=mise.jobs,
            cache_state=cache_state,
            mode=mode,
            isolated_tasks=isolated_tasks,
            host_settings=host_settings,
        )

    def differences(self, other: Self) -> tuple[ConditionDifference, ...]:
        return tuple(
            ConditionDifference(name, previous, current)
            for name in type(self).model_fields
            if _differs(
                previous := getattr(self, name),
                current := getattr(other, name),
            )
        )

    def comparable_with(self, other: Self) -> bool:
        return not self.differences(other)


def _both_matched(at_start: bool, at_end: bool | None) -> bool:
    return at_start and at_end is True


class CommitMatch(RecordModel):
    id: str
    matched_at_start: bool
    matched_at_end: bool | None
    matched: bool

    @classmethod
    def of(cls, commit: str, *, at_start: bool, at_end: bool | None) -> Self:
        return cls(
            id=commit,
            matched_at_start=at_start,
            matched_at_end=at_end,
            matched=_both_matched(at_start, at_end),
        )

    @model_validator(mode="after")
    def _matched_needs_both_checks(self) -> Self:
        if self.matched != _both_matched(
            self.matched_at_start, self.matched_at_end
        ):
            raise ValueError("matched disagrees with its two checks")
        return self


class PrimingRun(RecordModel):
    wall_clock_seconds: float
    discarded: bool


def _contains_failure(
    tasks: Mapping[str, TaskFigures], exit_status: int
) -> bool:
    return bool(Timeline(tasks).failed()) or exit_status != 0


class RunFigures(RecordModel):
    index: int
    wall_clock_seconds: float
    exit_status: int
    contains_failure: bool
    tasks: Mapping[str, TaskFigures]
    unit_totals: CargoUnits
    cpu_buckets: tuple[CpuBucket, ...]
    process_cpu_seconds: Mapping[str, float | NoReading | None]
    fetch_failures: tuple[FetchFailure, ...]

    @classmethod
    def derived(
        cls,
        *,
        index: int,
        wall_clock_seconds: float,
        exit_status: int,
        tasks: Mapping[str, TaskFigures],
        cpu_buckets: tuple[CpuBucket, ...],
        process_cpu_seconds: Mapping[str, float | NoReading | None],
        fetch_failures: tuple[FetchFailure, ...],
    ) -> Self:
        return cls(
            index=index,
            wall_clock_seconds=wall_clock_seconds,
            exit_status=exit_status,
            contains_failure=_contains_failure(tasks, exit_status),
            tasks=tasks,
            unit_totals=Timeline(tasks).unit_totals(),
            cpu_buckets=cpu_buckets,
            process_cpu_seconds=process_cpu_seconds,
            fetch_failures=fetch_failures,
        )

    def failed(self) -> tuple[str, ...]:
        return Timeline(self.tasks).failed()

    @model_validator(mode="after")
    def _derived_fields_agree(self) -> Self:
        if self.contains_failure != _contains_failure(
            self.tasks, self.exit_status
        ):
            raise ValueError(
                "contains_failure disagrees with the run's outcomes and exit "
                "status"
            )
        if self.unit_totals != Timeline(self.tasks).unit_totals():
            raise ValueError("unit_totals disagrees with the tasks' units")
        return self


class SessionSummary(RecordModel):
    wall_clock_seconds: Spread
    tasks: Mapping[str, TaskAcrossRuns]
    processes: Mapping[str, ProcessAcrossRuns]

    @classmethod
    def of(cls, runs: Sequence[RunFigures]) -> Self:
        first = runs[0]
        return cls(
            wall_clock_seconds=spread([run.wall_clock_seconds for run in runs]),
            tasks={
                task: task_across_runs([run.tasks[task] for run in runs])
                for task in first.tasks
            },
            processes={
                name: process_across_runs(
                    [run.process_cpu_seconds.get(name) for run in runs]
                )
                for name in first.process_cpu_seconds
            },
        )


def _every_fetch_failure(
    runs: Sequence[RunFigures],
) -> tuple[FetchFailure, ...]:
    return tuple(failure for run in runs for failure in run.fetch_failures)


def _every_failed_task(runs: Sequence[RunFigures]) -> tuple[str, ...]:
    return tuple(dict.fromkeys(task for run in runs for task in run.failed()))


class MeasuredBuild(RecordModel):
    schema_version: Literal[1]
    run_id: str
    run_directory: Path
    started_at: AwareDatetime
    conditions: MeasurementConditions
    commit: CommitMatch
    host: HostDescription
    mise: MiseDescription
    idle_load_percent: float | None
    task_set: tuple[str, ...]
    tracked_processes: Mapping[str, tuple[str, ...]]
    priming: PrimingRun | None
    runs: tuple[RunFigures, ...]
    summary: SessionSummary
    fetch_failures: tuple[FetchFailure, ...]
    failed_tasks: tuple[str, ...]

    @classmethod
    def derived(
        cls,
        *,
        run_id: str,
        run_directory: Path,
        started_at: dt.datetime,
        cache_state: BuildCacheState,
        mode: Mode,
        isolated_tasks: tuple[str, ...],
        host_settings: HostSettings,
        commit: CommitMatch,
        host: HostDescription,
        mise: MiseDescription,
        idle_load_percent: float | None,
        task_set: tuple[str, ...],
        tracked: Sequence[TrackedProcess],
        priming: PrimingRun | None,
        runs: tuple[RunFigures, ...],
    ) -> Self:
        return cls(
            schema_version=SCHEMA_VERSION,
            run_id=run_id,
            run_directory=run_directory,
            started_at=started_at,
            conditions=MeasurementConditions.of(
                host,
                mise,
                cache_state=cache_state,
                mode=mode,
                isolated_tasks=isolated_tasks,
                host_settings=host_settings,
            ),
            commit=commit,
            host=host,
            mise=mise,
            idle_load_percent=idle_load_percent,
            task_set=task_set,
            tracked_processes={each.name: each.patterns for each in tracked},
            priming=priming,
            runs=runs,
            summary=SessionSummary.of(runs),
            fetch_failures=_every_fetch_failure(runs),
            failed_tasks=_every_failed_task(runs),
        )

    @field_serializer("started_at")
    def _with_offset(self, started_at: dt.datetime) -> str:
        return started_at.isoformat()

    @model_validator(mode="after")
    def _copies_agree(self) -> Self:
        mirrored = MeasurementConditions.of(
            self.host,
            self.mise,
            cache_state=self.conditions.cache_state,
            mode=self.conditions.mode,
            isolated_tasks=self.conditions.isolated_tasks,
            host_settings=self.conditions.host_settings,
        )
        if self.conditions != mirrored:
            raise ValueError("conditions disagree with the host and mise")
        if self.fetch_failures != _every_fetch_failure(self.runs):
            raise ValueError("fetch_failures disagrees with the runs' lists")
        if self.failed_tasks != _every_failed_task(self.runs):
            raise ValueError("failed_tasks disagrees with the runs' outcomes")
        return self
