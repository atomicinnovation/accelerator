"""Two build records compared under the comparability and citation rules."""

from collections.abc import Sequence
from dataclasses import dataclass
from enum import StrEnum, unique

from tasks.shared.build_measurement.across_runs import (
    Measured,
    Presence,
    ProcessAcrossRuns,
    Spread,
    TaskAcrossRuns,
)
from tasks.shared.build_measurement.measured_build import (
    ConditionDifference,
    HostSettings,
    MeasuredBuild,
)
from tasks.shared.build_measurement.task_graph import closest_task
from tasks.shared.build_measurement.timeline import CargoUnits

WALL_CLOCK = "wall-clock"
UNIT_TOTAL = "total"


@unique
class Noise(StrEnum):
    WITHIN = "within"
    BEYOND = "beyond"
    UNJUDGED = "unjudged"


@unique
class RecordSide(StrEnum):
    PREVIOUS = "previous"
    CURRENT = "current"


@dataclass(frozen=True, slots=True)
class MeasuredChange:
    name: str
    previous_median: float
    current_median: float
    noise: Noise

    @property
    def seconds(self) -> float:
        return self.current_median - self.previous_median

    @property
    def ratio(self) -> float:
        return self.current_median / self.previous_median


@dataclass(frozen=True, slots=True)
class ChangeFromIdle:
    name: str
    current_median: float
    noise: Noise

    @property
    def seconds(self) -> float:
        return self.current_median


@dataclass(frozen=True, slots=True)
class ChangeInPresence:
    name: str
    previous: Presence
    current: Presence


@dataclass(frozen=True, slots=True)
class FigureOnOneSide:
    name: str
    previous_median: float | None
    current_median: float | None


@dataclass(frozen=True, slots=True)
class TrackedDifferently:
    name: str
    previous_patterns: tuple[str, ...]
    current_patterns: tuple[str, ...]


type FigureChange = (
    MeasuredChange | ChangeFromIdle | ChangeInPresence | FigureOnOneSide
)
type ProcessChange = FigureChange | TrackedDifferently


@dataclass(frozen=True, slots=True)
class TaskComparison:
    name: str
    duration: FigureChange
    nextest_phase: FigureChange | None


@dataclass(frozen=True, slots=True)
class TaskMissingFromOne:
    name: str
    missing_from: RecordSide


@dataclass(frozen=True, slots=True)
class TaskInNeither:
    name: str
    closest: str | None


type NamedTask = TaskComparison | TaskMissingFromOne | TaskInNeither


@dataclass(frozen=True, slots=True)
class UnitCountChange:
    name: str
    previous: int
    current: int


@dataclass(frozen=True, slots=True)
class UnitsVaryAcrossRuns:
    sides: tuple[RecordSide, ...]


type UnitTotals = tuple[UnitCountChange, ...] | UnitsVaryAcrossRuns


@dataclass(frozen=True, slots=True)
class RecordComparison:
    previous_run_id: str
    current_run_id: str
    previous_runs: int
    current_runs: int
    previous_label: HostSettings
    current_label: HostSettings
    differing_conditions: tuple[ConditionDifference, ...]
    wall_clock: MeasuredChange | ChangeFromIdle
    processes: tuple[ProcessChange, ...]
    tasks_added: tuple[str, ...]
    tasks_removed: tuple[str, ...]
    named_tasks: tuple[NamedTask, ...]
    unit_totals: UnitTotals


@dataclass(frozen=True, slots=True)
class _RecordPair:
    previous: MeasuredBuild
    current: MeasuredBuild

    def noise(self, previous: Spread, current_median: float) -> Noise:
        if len(self.previous.runs) == 1:
            return Noise.UNJUDGED
        if previous.minimum <= current_median <= previous.maximum:
            return Noise.WITHIN
        return Noise.BEYOND

    def spread_change(
        self, name: str, previous: Spread, current: Spread
    ) -> MeasuredChange | ChangeFromIdle:
        noise = self.noise(previous, current.median)
        if previous.median == 0.0:
            return ChangeFromIdle(name, current.median, noise)
        return MeasuredChange(name, previous.median, current.median, noise)

    def figure_change(
        self, name: str, previous: Spread | None, current: Spread | None
    ) -> FigureChange | None:
        match previous, current:
            case Spread(), Spread():
                return self.spread_change(name, previous, current)
            case None, None:
                return None
            case _:
                return FigureOnOneSide(
                    name,
                    previous_median=_median(previous),
                    current_median=_median(current),
                )


def _median(figure: Spread | None) -> float | None:
    return None if figure is None else figure.median


def _measured_spread(figure: Spread | None) -> Spread:
    if figure is None:
        raise ValueError("a measured figure always has a spread")
    return figure


def _both_measured(previous: Presence, current: Presence) -> bool:
    return isinstance(previous, Measured) and isinstance(current, Measured)


def _process_change(
    pair: _RecordPair,
    name: str,
    previous: ProcessAcrossRuns,
    current: ProcessAcrossRuns,
) -> FigureChange:
    if not _both_measured(previous.presence, current.presence):
        return ChangeInPresence(name, previous.presence, current.presence)
    return pair.spread_change(
        name,
        _measured_spread(previous.cpu_seconds),
        _measured_spread(current.cpu_seconds),
    )


def _tracked_process_change(pair: _RecordPair, name: str) -> ProcessChange:
    previous, current = pair.previous, pair.current
    previous_patterns = previous.tracked_processes.get(name, ())
    current_patterns = current.tracked_processes.get(name, ())
    if previous_patterns != current_patterns:
        return TrackedDifferently(name, previous_patterns, current_patterns)
    return _process_change(
        pair,
        name,
        previous.summary.processes[name],
        current.summary.processes[name],
    )


def _process_changes(pair: _RecordPair) -> tuple[ProcessChange, ...]:
    names = dict.fromkeys(
        [*pair.previous.tracked_processes, *pair.current.tracked_processes]
    )
    return tuple(_tracked_process_change(pair, name) for name in names)


def _task_comparison(
    pair: _RecordPair,
    name: str,
    previous: TaskAcrossRuns,
    current: TaskAcrossRuns,
) -> TaskComparison:
    if not _both_measured(previous.presence, current.presence):
        return TaskComparison(
            name,
            duration=ChangeInPresence(
                name, previous.presence, current.presence
            ),
            nextest_phase=None,
        )
    return TaskComparison(
        name,
        duration=pair.spread_change(
            name,
            _measured_spread(previous.duration_seconds),
            _measured_spread(current.duration_seconds),
        ),
        nextest_phase=pair.figure_change(
            name, previous.nextest_phase_seconds, current.nextest_phase_seconds
        ),
    )


def _named_task(pair: _RecordPair, name: str) -> NamedTask:
    previous = pair.previous.summary.tasks.get(name)
    current = pair.current.summary.tasks.get(name)
    match previous, current:
        case TaskAcrossRuns(), TaskAcrossRuns():
            return _task_comparison(pair, name, previous, current)
        case None, TaskAcrossRuns():
            return TaskMissingFromOne(name, RecordSide.PREVIOUS)
        case TaskAcrossRuns(), None:
            return TaskMissingFromOne(name, RecordSide.CURRENT)
        case _:
            known = [*pair.previous.summary.tasks, *pair.current.summary.tasks]
            return TaskInNeither(name, closest_task(name, known))


def _unit_count_changes(
    previous: CargoUnits, current: CargoUnits
) -> tuple[UnitCountChange, ...]:
    return (
        *(
            UnitCountChange(kind, before, after)
            for (kind, before), (_, after) in zip(
                previous.by_kind, current.by_kind, strict=True
            )
        ),
        UnitCountChange(UNIT_TOTAL, previous.total, current.total),
    )


def _units_every_run_agrees_on(build: MeasuredBuild) -> CargoUnits | None:
    first, *rest = (run.unit_totals for run in build.runs)
    return first if all(each == first for each in rest) else None


def _unit_totals(pair: _RecordPair) -> UnitTotals:
    previous = _units_every_run_agrees_on(pair.previous)
    current = _units_every_run_agrees_on(pair.current)
    if previous is None or current is None:
        return UnitsVaryAcrossRuns(
            tuple(
                side
                for side, units in (
                    (RecordSide.PREVIOUS, previous),
                    (RecordSide.CURRENT, current),
                )
                if units is None
            )
        )
    return _unit_count_changes(previous, current)


def compare_records(
    previous: MeasuredBuild,
    current: MeasuredBuild,
    *,
    tasks: Sequence[str] = (),
) -> RecordComparison:
    pair = _RecordPair(previous, current)
    return RecordComparison(
        previous_run_id=previous.run_id,
        current_run_id=current.run_id,
        previous_runs=len(previous.runs),
        current_runs=len(current.runs),
        previous_label=previous.conditions.host_settings,
        current_label=current.conditions.host_settings,
        differing_conditions=previous.conditions.differences(
            current.conditions
        ),
        wall_clock=pair.spread_change(
            WALL_CLOCK,
            previous.summary.wall_clock_seconds,
            current.summary.wall_clock_seconds,
        ),
        processes=_process_changes(pair),
        tasks_added=tuple(
            task for task in current.task_set if task not in previous.task_set
        ),
        tasks_removed=tuple(
            task for task in previous.task_set if task not in current.task_set
        ),
        named_tasks=tuple(_named_task(pair, task) for task in tasks),
        unit_totals=_unit_totals(pair),
    )
