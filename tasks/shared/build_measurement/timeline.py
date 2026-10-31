"""Per-task figures from one timed run's stamped ``mise run --output prefix``.

Each line of a stamped log is ``<seconds since the run began> <raw line>``.
"""

import re
from collections import Counter
from collections.abc import Iterable, Iterator, Mapping, Sequence
from dataclasses import dataclass
from enum import StrEnum, unique
from functools import cache
from types import MappingProxyType
from typing import Self

from tasks.shared.build_measurement.record_model import RecordModel
from tasks.shared.build_measurement.task_graph import TaskGraph

_ANSI_CSI = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
_TASK_PREFIX = re.compile(r"\[(?P<task>[^\]]+)\] ?(?P<rest>.*)", re.DOTALL)
_FINISHED_IN = re.compile(r"Finished in (?P<amount>[\d.]+)(?P<unit>ms|s)")
_EXITED = re.compile(r"exited with status (?P<status>\d+)")
_SKIPPING = "sources up-to-date, skipping"
_TASK_FAILED = "ERROR task failed"
_BLOCKING = "Blocking waiting for file lock"
_UNIT = re.compile(r"(?:^|\s)(?P<kind>Compiling|Checking|Documenting) \S+ v")
_NEXTEST_SUMMARY = re.compile(r"\s*Summary \[\s*(?P<seconds>[\d.]+)s\]")


@dataclass(frozen=True, slots=True)
class StampedLine:
    at: float
    text: str


@unique
class TaskOutcome(StrEnum):
    SUCCEEDED = "succeeded"
    FAILED = "failed"
    SKIPPED = "skipped"
    NOT_RUN = "not-run"
    SILENT = "silent"


_FAILURES = frozenset({TaskOutcome.FAILED, TaskOutcome.NOT_RUN})


class CargoUnits(RecordModel):
    compiling: int = 0
    checking: int = 0
    documenting: int = 0

    def __add__(self, other: CargoUnits) -> CargoUnits:
        return CargoUnits(
            compiling=self.compiling + other.compiling,
            checking=self.checking + other.checking,
            documenting=self.documenting + other.documenting,
        )


class OwnFigures(RecordModel):
    outcome: TaskOutcome
    exit_status: int | None
    start_seconds: float | None
    duration_seconds: float | None
    end_seconds: float | None
    units: CargoUnits
    lock_wait_seconds: float
    nextest_phase_seconds: float | None

    @staticmethod
    def skipped(at: float) -> OwnFigures:
        return OwnFigures(
            outcome=TaskOutcome.SKIPPED,
            exit_status=None,
            start_seconds=at,
            duration_seconds=None,
            end_seconds=at,
            units=CargoUnits(),
            lock_wait_seconds=0.0,
            nextest_phase_seconds=None,
        )

    @staticmethod
    def silent() -> OwnFigures:
        return OwnFigures._unprinted(TaskOutcome.SILENT)

    @staticmethod
    def not_run() -> OwnFigures:
        return OwnFigures._unprinted(TaskOutcome.NOT_RUN)

    @staticmethod
    def _unprinted(outcome: TaskOutcome) -> OwnFigures:
        return OwnFigures(
            outcome=outcome,
            exit_status=None,
            start_seconds=None,
            duration_seconds=None,
            end_seconds=None,
            units=CargoUnits(),
            lock_wait_seconds=0.0,
            nextest_phase_seconds=None,
        )


class TaskFigures(OwnFigures):
    queueing_delay_seconds: float | None
    after_failure: bool

    @classmethod
    def placed(
        cls,
        own: OwnFigures,
        *,
        queueing_delay_seconds: float | None,
        after_failure: bool,
    ) -> Self:
        return cls.model_validate(
            {
                **dict(own),
                "queueing_delay_seconds": queueing_delay_seconds,
                "after_failure": after_failure,
            }
        )


def failed_tasks(figures: Mapping[str, TaskFigures]) -> tuple[str, ...]:
    return tuple(
        task for task, each in figures.items() if each.outcome in _FAILURES
    )


@dataclass(frozen=True, slots=True)
class Timeline:
    tasks: Mapping[str, TaskFigures]

    @classmethod
    def merged(cls, timelines: Iterable[Timeline]) -> Self:
        tasks: dict[str, TaskFigures] = {}
        for timeline in timelines:
            if shared := tasks.keys() & timeline.tasks.keys():
                raise ValueError(
                    f"timelines to merge share tasks: {sorted(shared)}"
                )
            tasks.update(timeline.tasks)
        return cls(MappingProxyType(tasks))

    def failed(self) -> tuple[str, ...]:
        return failed_tasks(self.tasks)

    def unit_totals(self) -> CargoUnits:
        return sum(
            (figures.units for figures in self.tasks.values()), CargoUnits()
        )


@dataclass(frozen=True, slots=True)
class TaskTranscript:
    counted: tuple[StampedLine, ...]
    finished_in: float | None
    exit_status: int | None
    skipped_at: float | None


def read_stamped(text: str) -> tuple[StampedLine, ...]:
    raw_lines = text.split("\n")
    if raw_lines[-1] == "":
        raw_lines.pop()
    return tuple(_stamped(raw) for raw in raw_lines)


def _stamped(raw: str) -> StampedLine:
    at, _, text = raw.partition(" ")
    return StampedLine(at=float(at), text=text)


def attributed_lines(
    lines: Iterable[StampedLine],
) -> Iterator[tuple[str, StampedLine]]:
    for line in lines:
        prefixed = _TASK_PREFIX.fullmatch(_ANSI_CSI.sub("", line.text))
        if prefixed is not None:
            yield (
                prefixed["task"],
                StampedLine(at=line.at, text=prefixed["rest"]),
            )


def _segments(text: str) -> tuple[str, ...]:
    return tuple(segment for segment in text.split("\r") if segment)


def _last_segment(text: str) -> str:
    segments = _segments(text)
    return segments[-1].strip() if segments else ""


def _finished_in(line: StampedLine) -> float | None:
    finished = _FINISHED_IN.fullmatch(_last_segment(line.text))
    if finished is None:
        return None
    amount = float(finished["amount"])
    return amount / 1000 if finished["unit"] == "ms" else amount


def _exit_status(line: StampedLine) -> int | None:
    exited = _EXITED.fullmatch(_last_segment(line.text))
    return None if exited is None else int(exited["status"])


def _is_skip(line: StampedLine) -> bool:
    return _last_segment(line.text) == _SKIPPING


def _is_counted(line: StampedLine) -> bool:
    return not (
        _finished_in(line) is not None
        or _exit_status(line) is not None
        or _is_skip(line)
        or _last_segment(line.text) == _TASK_FAILED
    )


def _first[T](values: Iterable[T | None]) -> T | None:
    return next((value for value in values if value is not None), None)


def _transcript(lines: Sequence[StampedLine]) -> TaskTranscript:
    return TaskTranscript(
        counted=tuple(line for line in lines if _is_counted(line)),
        finished_in=_first(_finished_in(line) for line in lines),
        exit_status=_first(_exit_status(line) for line in lines),
        skipped_at=_first(
            line.at if _is_skip(line) else None for line in lines
        ),
    )


def transcripts(lines: Iterable[StampedLine]) -> Mapping[str, TaskTranscript]:
    grouped: dict[str, list[StampedLine]] = {}
    for task, line in attributed_lines(lines):
        grouped.setdefault(task, []).append(line)
    return MappingProxyType(
        {task: _transcript(own) for task, own in grouped.items()}
    )


def tasks_mise_reported_failing(
    lines: Iterable[StampedLine],
) -> tuple[str, ...]:
    return tuple(
        dict.fromkeys(
            task
            for task, line in attributed_lines(lines)
            if _exit_status(line) is not None
        )
    )


def _lock_wait(counted: Sequence[StampedLine], end: float | None) -> float:
    waited = 0.0
    opened: float | None = None
    for line in counted:
        if opened is not None:
            waited += line.at - opened
            opened = None
        if _BLOCKING in _last_segment(line.text):
            opened = line.at
    if opened is not None and end is not None:
        waited += max(0.0, end - opened)
    return waited


def _units(counted: Iterable[StampedLine]) -> CargoUnits:
    kinds = Counter(
        unit["kind"]
        for line in counted
        for segment in _segments(line.text)
        if (unit := _UNIT.search(segment)) is not None
    )
    return CargoUnits(
        compiling=kinds["Compiling"],
        checking=kinds["Checking"],
        documenting=kinds["Documenting"],
    )


def _nextest_phase(counted: Iterable[StampedLine]) -> float | None:
    phases = [
        float(summary["seconds"])
        for line in counted
        if (summary := _NEXTEST_SUMMARY.match(_last_segment(line.text)))
        is not None
    ]
    return sum(phases) if phases else None


def own_figures(transcript: TaskTranscript) -> OwnFigures:
    if transcript.skipped_at is not None:
        return OwnFigures.skipped(transcript.skipped_at)
    counted = transcript.counted
    start = counted[0].at if counted else None
    if transcript.finished_in is not None:
        outcome = TaskOutcome.SUCCEEDED
        exit_status = 0
        duration = transcript.finished_in
        end = None if start is None else start + duration
    else:
        outcome = TaskOutcome.FAILED
        exit_status = transcript.exit_status
        end = counted[-1].at if counted else None
        duration = None if start is None or end is None else end - start
    return OwnFigures(
        outcome=outcome,
        exit_status=exit_status,
        start_seconds=start,
        duration_seconds=duration,
        end_seconds=end,
        units=_units(counted),
        lock_wait_seconds=_lock_wait(counted, end),
        nextest_phase_seconds=_nextest_phase(counted),
    )


def place_in_graph(
    own: Mapping[str, OwnFigures], graph: TaskGraph, *, ready_from: float
) -> Timeline:
    @cache
    def ready_at(task: str) -> float:
        dependencies = graph.dependencies_of(task)
        if not dependencies:
            return ready_from
        return max(ended_or_ready_at(each) for each in dependencies)

    def ended_or_ready_at(task: str) -> float:
        figures = own.get(task)
        end = None if figures is None else figures.end_seconds
        return ready_at(task) if end is None else end

    def queueing_delay(task: str, figures: OwnFigures) -> float | None:
        if figures.start_seconds is None:
            return None
        return max(0.0, figures.start_seconds - ready_at(task))

    unsuccessful = {
        task for task, figures in own.items() if figures.outcome in _FAILURES
    }
    return Timeline(
        MappingProxyType(
            {
                task: TaskFigures.placed(
                    figures,
                    queueing_delay_seconds=queueing_delay(task, figures),
                    after_failure=bool(graph.closure(task) & unsuccessful),
                )
                for task, figures in own.items()
            }
        )
    )


def _unprinted_figures(task: str, graph: TaskGraph) -> OwnFigures:
    if graph.is_runnable(task):
        return OwnFigures.not_run()
    return OwnFigures.silent()


def analyse_timeline(
    lines: Iterable[StampedLine],
    graph: TaskGraph,
    tasks: Iterable[str],
    *,
    ready_from: float = 0.0,
) -> Timeline:
    printed = transcripts(lines)
    own = {
        task: own_figures(printed[task])
        if task in printed
        else _unprinted_figures(task, graph)
        for task in sorted(tasks)
    }
    return place_in_graph(own, graph, ready_from=ready_from)
