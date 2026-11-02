"""Two build records compared under the comparability and citation rules."""

import difflib
import shlex
from collections.abc import Iterator, Sequence
from dataclasses import dataclass
from enum import StrEnum, unique

from tasks.shared.build_measurement.across_runs import (
    Absence,
    Absent,
    Flagged,
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
from tasks.shared.build_measurement.record_model import NoReading
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
class RecordComparison:
    previous_run_id: str
    current_run_id: str
    previous_runs: int
    current_runs: int
    previous_label: HostSettings
    current_label: HostSettings
    differing_conditions: tuple[ConditionDifference, ...]
    wall_clock: MeasuredChange
    processes: tuple[ProcessChange, ...]
    tasks_added: tuple[str, ...]
    tasks_removed: tuple[str, ...]
    named_tasks: tuple[NamedTask, ...]
    unit_totals: tuple[UnitCountChange, ...]


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
            closest = difflib.get_close_matches(name, known, n=1)
            return TaskInNeither(name, closest[0] if closest else None)


def _unit_totals(pair: _RecordPair) -> tuple[UnitCountChange, ...]:
    previous = pair.previous.runs[0].unit_totals
    current = pair.current.runs[0].unit_totals
    kinds = tuple(CargoUnits.model_fields)
    return (
        *(
            UnitCountChange(
                kind, getattr(previous, kind), getattr(current, kind)
            )
            for kind in kinds
        ),
        UnitCountChange(
            UNIT_TOTAL,
            sum(getattr(previous, kind) for kind in kinds),
            sum(getattr(current, kind) for kind in kinds),
        ),
    )


def compare_records(
    previous: MeasuredBuild,
    current: MeasuredBuild,
    *,
    tasks: Sequence[str] = (),
) -> RecordComparison:
    pair = _RecordPair(previous, current)
    previous_wall_clock = previous.summary.wall_clock_seconds
    current_wall_clock = current.summary.wall_clock_seconds
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
        wall_clock=MeasuredChange(
            WALL_CLOCK,
            previous_wall_clock.median,
            current_wall_clock.median,
            pair.noise(previous_wall_clock, current_wall_clock.median),
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


@dataclass(frozen=True, slots=True)
class _RecordNames:
    previous: str
    current: str

    @classmethod
    def of(cls, comparison: RecordComparison) -> _RecordNames:
        if comparison.previous_label == comparison.current_label:
            return cls(str(RecordSide.PREVIOUS), str(RecordSide.CURRENT))
        return cls(
            f"'{comparison.previous_label}'", f"'{comparison.current_label}'"
        )

    def of_side(self, side: RecordSide) -> str:
        return self.previous if side is RecordSide.PREVIOUS else self.current


def _plural(count: int, noun: str) -> str:
    return f"{count} {noun}" if count == 1 else f"{count} {noun}s"


def _condition_value(value: object) -> str:
    match value:
        case NoReading(argv=argv):
            return f"unreadable ({shlex.join(argv)})"
        case tuple():
            return ", ".join(str(each) for each in value) or "empty"
        case _:
            return str(value)


def _condition(difference: ConditionDifference) -> str:
    if isinstance(difference.previous, NoReading) and isinstance(
        difference.current, NoReading
    ):
        return f"{difference.field} (unreadable in both records)"
    return (
        f"{difference.field} ({_condition_value(difference.previous)} → "
        f"{_condition_value(difference.current)})"
    )


def _verdict(comparison: RecordComparison) -> Iterator[str]:
    if not comparison.differing_conditions:
        yield "conditions match"
        return
    differing = "; ".join(
        _condition(each) for each in comparison.differing_conditions
    )
    yield f"conditions differ: {differing}"


def _records(comparison: RecordComparison) -> Iterator[str]:
    yield (
        f"previous: {comparison.previous_run_id} "
        f"({_plural(comparison.previous_runs, 'run')})"
    )
    yield (
        f"current: {comparison.current_run_id} "
        f"({_plural(comparison.current_runs, 'run')})"
    )


def _noise(noise: Noise) -> str:
    if noise is Noise.UNJUDGED:
        return "no spread to judge noise (1 run)"
    return f"{noise} noise"


def _absence(absence: Absence, *names: str) -> str:
    match absence:
        case Absence.NOT_SEEN:
            return f"did not run in any {' or '.join(names)} run"
        case Absence.SKIPPED:
            return f"skipped in every {' and '.join(names)} run"
        case Absence.SILENT:
            return f"printed nothing in every {' and '.join(names)} run"


def _presence(presence: Presence, name: str) -> str:
    match presence:
        case Measured():
            return f"measured in every {name} run"
        case Flagged(caveats=caveats):
            return f"flagged {', '.join(caveats)} in the {name} record"
        case Absent(absence=absence):
            return _absence(absence, name)


def _presence_change(change: ChangeInPresence, names: _RecordNames) -> str:
    match change.previous, change.current:
        case Absent(absence=before), Absent(absence=after) if before == after:
            return _absence(before, names.previous, names.current)
        case _:
            return (
                f"{_presence(change.previous, names.previous)}; "
                f"{_presence(change.current, names.current)}"
            )


def _one_side(change: FigureOnOneSide, names: _RecordNames) -> str:
    if change.previous_median is not None:
        return (
            f"{change.previous_median:.1f} s in the {names.previous} record, "
            f"not in every {names.current} run"
        )
    return (
        f"{change.current_median:.1f} s in the {names.current} record, "
        f"not in every {names.previous} run"
    )


def _figure(change: FigureChange, names: _RecordNames) -> str:
    match change:
        case MeasuredChange():
            return (
                f"{change.previous_median:.1f} s → "
                f"{change.current_median:.1f} s, {change.seconds:+.1f} s "
                f"(×{change.ratio:.3f}), {_noise(change.noise)}"
            )
        case ChangeFromIdle():
            return (
                f"0.0 s → {change.current_median:.1f} s, "
                f"{change.seconds:+.1f} s (no ratio from zero), "
                f"{_noise(change.noise)}"
            )
        case ChangeInPresence():
            return _presence_change(change, names)
        case FigureOnOneSide():
            return _one_side(change, names)


def _patterns(patterns: tuple[str, ...]) -> str:
    return ", ".join(patterns) or "not tracked"


def _process(change: ProcessChange, names: _RecordNames) -> str:
    if isinstance(change, TrackedDifferently):
        return (
            f"{change.name}: not compared, tracked by different patterns "
            f"({names.previous}: {_patterns(change.previous_patterns)}; "
            f"{names.current}: {_patterns(change.current_patterns)})"
        )
    return f"{change.name}: {_figure(change, names)}"


def _wall_clock_and_processes(
    comparison: RecordComparison, names: _RecordNames
) -> Iterator[str]:
    yield f"{WALL_CLOCK}: {_figure(comparison.wall_clock, names)}"
    for change in comparison.processes:
        yield _process(change, names)


def _task_set(comparison: RecordComparison) -> Iterator[str]:
    differences = [
        f"{verb} {', '.join(tasks)}"
        for verb, tasks in (
            ("added", comparison.tasks_added),
            ("removed", comparison.tasks_removed),
        )
        if tasks
    ]
    yield f"task set: {'; '.join(differences) or 'unchanged'}"


def _named_task_lines(task: NamedTask, names: _RecordNames) -> Iterator[str]:
    match task:
        case TaskComparison(name=name, duration=duration, nextest_phase=phase):
            yield f"{name} duration: {_figure(duration, names)}"
            if phase is not None:
                yield f"{name} nextest phase: {_figure(phase, names)}"
        case TaskMissingFromOne(name=name, missing_from=side):
            yield f"{name}: not in the {names.of_side(side)} record"
        case TaskInNeither(name=name, closest=closest):
            suggestion = (
                "no close match" if closest is None else f"closest: '{closest}'"
            )
            yield f"{name}: in neither record ({suggestion})"


def _named_tasks(
    comparison: RecordComparison, names: _RecordNames
) -> Iterator[str]:
    for task in comparison.named_tasks:
        yield from _named_task_lines(task, names)


def _unit_count(change: UnitCountChange, names: _RecordNames) -> str:
    difference = change.current - change.previous
    if change.previous == change.current == 0:
        return "none in either record"
    counts = f"{change.previous} → {change.current}, {difference:+d}"
    if change.previous == 0:
        return f"{counts} (none in the {names.previous} record)"
    return f"{counts} (×{change.current / change.previous:.3f})"


def _units(comparison: RecordComparison, names: _RecordNames) -> Iterator[str]:
    for change in comparison.unit_totals:
        yield f"units {change.name}: {_unit_count(change, names)}"


def comparison_report(comparison: RecordComparison) -> str:
    names = _RecordNames.of(comparison)
    sections = (
        _verdict(comparison),
        _records(comparison),
        _wall_clock_and_processes(comparison, names),
        _task_set(comparison),
        _named_tasks(comparison, names),
        _units(comparison, names),
    )
    return "".join(f"{line}\n" for section in sections for line in section)
