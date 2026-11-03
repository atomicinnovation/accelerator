"""Two compared build records, in words."""

import shlex
from collections.abc import Iterator
from dataclasses import dataclass

from tasks.shared.build_measurement.across_runs import (
    Absence,
    Absent,
    Flagged,
    Measured,
    Presence,
)
from tasks.shared.build_measurement.comparison import (
    WALL_CLOCK,
    ChangeFromIdle,
    ChangeInPresence,
    FigureChange,
    FigureOnOneSide,
    MeasuredChange,
    NamedTask,
    Noise,
    ProcessChange,
    RecordComparison,
    RecordSide,
    TaskComparison,
    TaskInNeither,
    TaskMissingFromOne,
    TrackedDifferently,
    UnitCountChange,
    UnitsVaryAcrossRuns,
)
from tasks.shared.build_measurement.measured_build import ConditionDifference
from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.wording import plural


@dataclass(frozen=True, slots=True)
class _RecordNames:
    previous: str
    current: str
    labelled: bool

    @classmethod
    def of(cls, comparison: RecordComparison) -> _RecordNames:
        if comparison.previous_label == comparison.current_label:
            return cls(
                str(RecordSide.PREVIOUS),
                str(RecordSide.CURRENT),
                labelled=False,
            )
        return cls(
            f"'{comparison.previous_label}'",
            f"'{comparison.current_label}'",
            labelled=True,
        )

    def of_side(self, side: RecordSide) -> str:
        return self.previous if side is RecordSide.PREVIOUS else self.current

    def introducing(self, side: RecordSide) -> str:
        return f"{side} ({self.of_side(side)})" if self.labelled else str(side)


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


def _records(
    comparison: RecordComparison, names: _RecordNames
) -> Iterator[str]:
    yield (
        f"{names.introducing(RecordSide.PREVIOUS)}: "
        f"{comparison.previous_run_id} "
        f"({plural(comparison.previous_runs, 'run')})"
    )
    yield (
        f"{names.introducing(RecordSide.CURRENT)}: "
        f"{comparison.current_run_id} "
        f"({plural(comparison.current_runs, 'run')})"
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
    match comparison.unit_totals:
        case UnitsVaryAcrossRuns(sides=sides):
            varying = " and ".join(names.of_side(side) for side in sides)
            records = "record's" if len(sides) == 1 else "records'"
            yield (
                f"units: not compared, they vary across the {varying} "
                f"{records} runs"
            )
        case changes:
            for change in changes:
                yield f"units {change.name}: {_unit_count(change, names)}"


def comparison_report(comparison: RecordComparison) -> str:
    names = _RecordNames.of(comparison)
    sections = (
        _verdict(comparison),
        _records(comparison, names),
        _wall_clock_and_processes(comparison, names),
        _task_set(comparison),
        _named_tasks(comparison, names),
        _units(comparison, names),
    )
    return "".join(f"{line}\n" for section in sections for line in section)
