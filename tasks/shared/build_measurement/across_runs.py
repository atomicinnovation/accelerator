from collections.abc import Sequence
from enum import StrEnum, unique
from typing import Annotated, Literal, Self

from pydantic import Field, model_validator

from tasks.shared.build_measurement.record_model import NoReading, RecordModel
from tasks.shared.build_measurement.timeline import TaskFigures, TaskOutcome
from tasks.shared.measurement import median


class Spread(RecordModel):
    minimum: float
    median: float
    maximum: float


def spread(values: Sequence[float]) -> Spread:
    return Spread(
        minimum=min(values), median=median(values), maximum=max(values)
    )


@unique
class Caveat(StrEnum):
    FAILED = "failed"
    NOT_RUN = "not-run"
    AFTER_FAILURE = "after-failure"
    SKIPPED_IN_SOME_RUNS = "skipped-in-some-runs"
    SILENT_IN_SOME_RUNS = "silent-in-some-runs"
    ABSENT_IN_SOME_RUNS = "absent-in-some-runs"
    UNREADABLE = "unreadable"


@unique
class Absence(StrEnum):
    SKIPPED = "skipped"
    SILENT = "silent"
    NOT_SEEN = "not-seen"


class Measured(RecordModel):
    standing: Literal["measured"] = "measured"


class Flagged(RecordModel):
    standing: Literal["flagged"] = "flagged"
    caveats: tuple[Caveat, ...] = Field(min_length=1)


class Absent(RecordModel):
    standing: Literal["absent"] = "absent"
    absence: Absence


type Presence = Annotated[
    Measured | Flagged | Absent, Field(discriminator="standing")
]


def _require_spread_exactly_when_measured(
    presence: Presence, figure: Spread | None
) -> None:
    if isinstance(presence, Measured) != (figure is not None):
        raise ValueError(
            "a spread is recorded exactly when the presence is measured"
        )


class TaskAcrossRuns(RecordModel):
    presence: Presence
    duration_seconds: Spread | None
    nextest_phase_seconds: Spread | None

    @model_validator(mode="after")
    def _spread_matches_presence(self) -> Self:
        _require_spread_exactly_when_measured(
            self.presence, self.duration_seconds
        )
        return self


class ProcessAcrossRuns(RecordModel):
    presence: Presence
    cpu_seconds: Spread | None

    @model_validator(mode="after")
    def _spread_matches_presence(self) -> Self:
        _require_spread_exactly_when_measured(self.presence, self.cpu_seconds)
        return self


def _flagged_or_measured(caveats: Sequence[Caveat]) -> Flagged | Measured:
    if caveats:
        return Flagged(caveats=tuple(caveats))
    return Measured()


def _in_some_runs(
    outcomes: Sequence[TaskOutcome], outcome: TaskOutcome
) -> bool:
    return outcome in outcomes and any(each != outcome for each in outcomes)


def _task_presence(per_run: Sequence[TaskFigures]) -> Presence:
    outcomes = [figures.outcome for figures in per_run]
    if all(outcome is TaskOutcome.SKIPPED for outcome in outcomes):
        return Absent(absence=Absence.SKIPPED)
    if all(outcome is TaskOutcome.SILENT for outcome in outcomes):
        return Absent(absence=Absence.SILENT)
    observed = {
        Caveat.FAILED: TaskOutcome.FAILED in outcomes,
        Caveat.NOT_RUN: TaskOutcome.NOT_RUN in outcomes,
        Caveat.AFTER_FAILURE: any(f.after_failure for f in per_run),
        Caveat.SKIPPED_IN_SOME_RUNS: _in_some_runs(
            outcomes, TaskOutcome.SKIPPED
        ),
        Caveat.SILENT_IN_SOME_RUNS: _in_some_runs(outcomes, TaskOutcome.SILENT),
    }
    return _flagged_or_measured(
        [caveat for caveat, holds in observed.items() if holds]
    )


def _spread_of_every(values: Sequence[float | None]) -> Spread | None:
    present = [value for value in values if value is not None]
    if not present or len(present) != len(values):
        return None
    return spread(present)


def task_across_runs(per_run: Sequence[TaskFigures]) -> TaskAcrossRuns:
    presence = _task_presence(per_run)
    if not isinstance(presence, Measured):
        return TaskAcrossRuns(
            presence=presence,
            duration_seconds=None,
            nextest_phase_seconds=None,
        )
    return TaskAcrossRuns(
        presence=presence,
        duration_seconds=_spread_of_every(
            [figures.duration_seconds for figures in per_run]
        ),
        nextest_phase_seconds=_spread_of_every(
            [figures.nextest_phase_seconds for figures in per_run]
        ),
    )


def _process_presence(per_run: Sequence[float | NoReading | None]) -> Presence:
    if all(cpu is None for cpu in per_run):
        return Absent(absence=Absence.NOT_SEEN)
    observed = {
        Caveat.ABSENT_IN_SOME_RUNS: any(cpu is None for cpu in per_run),
        Caveat.UNREADABLE: any(isinstance(cpu, NoReading) for cpu in per_run),
    }
    return _flagged_or_measured(
        [caveat for caveat, holds in observed.items() if holds]
    )


def process_across_runs(
    per_run: Sequence[float | NoReading | None],
) -> ProcessAcrossRuns:
    presence = _process_presence(per_run)
    figures = [cpu for cpu in per_run if isinstance(cpu, float)]
    return ProcessAcrossRuns(
        presence=presence,
        cpu_seconds=spread(figures) if isinstance(presence, Measured) else None,
    )
