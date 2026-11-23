"""Per-test durations from nextest's pass lines, combined across runs.

A figure is combined within each run first, and only then is its median
taken across the runs: the median of sums, never the sum of medians.
"""

import re
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import Self, override

from tasks.shared.build_measurement.errors import MeasurementRefusedError
from tasks.shared.build_measurement.timeline import (
    ANSI_CSI,
    NEXTEST_SUMMARY,
    StampedLine,
)
from tasks.shared.measurement import median
from tasks.shared.shared_fixtures.families import Family

_TASK_OUTPUT = re.compile(r"(?:\[[^\]\s]+\] ?)?(?P<output>.*)", re.DOTALL)
_PASSED = re.compile(
    r"\s*(?:TRY \d+ )?(?:PASS|LEAK) \[\s*(?P<seconds>[\d.]+)s\] "
    r"\(\s*(?P<position>\d+)/\s*\d+\) (?P<binary>\S+) (?P<test>\S.*?)\s*"
)
_FAMILY_BUILT = re.compile(
    r"\s*shared VCS fixtures: (?P<heading>.+) built in (?P<seconds>[\d.]+) s\s*"
)
_FAMILIES_BY_HEADING = MappingProxyType(
    {family.heading: family for family in Family}
)


@dataclass(frozen=True)
class TestId:
    __test__ = False

    binary: str
    test: str

    @classmethod
    def parse(cls, text: str) -> Self:
        binary, _, test = text.strip().partition(" ")
        if not test:
            raise MeasurementRefusedError(
                f"a test is named as nextest prints it, '<binary> <test>': "
                f"{text!r}"
            )
        return cls(binary, test.strip())

    @override
    def __str__(self) -> str:
        return f"{self.binary} {self.test}"


@dataclass(frozen=True)
class PassedTest:
    id: TestId
    seconds: float
    position: int
    finished_at: float | None


@dataclass(frozen=True)
class RunFigure:
    per_run: Sequence[float | None]
    median: float
    missing_from: Sequence[int]

    @classmethod
    def over(cls, per_run: Iterable[float | None]) -> Self:
        values = tuple(per_run)
        return cls(
            per_run=values,
            median=median([each for each in values if each is not None]),
            missing_from=tuple(
                run for run, each in enumerate(values, start=1) if each is None
            ),
        )


@dataclass(frozen=True)
class TimedRun:
    passes: Mapping[TestId, PassedTest]
    phase_seconds: float | None
    phase_end: float | None
    family_builds: Mapping[Family, float]

    @classmethod
    def parse(cls, lines: Iterable[StampedLine]) -> Self:
        return cls._read((line.at, line.text) for line in lines)

    @classmethod
    def parse_unstamped(cls, text: str) -> Self:
        return cls._read((None, line) for line in text.splitlines())

    @classmethod
    def _read(cls, lines: Iterable[tuple[float | None, str]]) -> Self:
        passes: dict[TestId, PassedTest] = {}
        phases: list[float] = []
        phase_end: float | None = None
        family_builds: dict[Family, float] = {}
        for at, text in lines:
            output = _task_output(text)
            if (passed := _PASSED.fullmatch(output)) is not None:
                test = TestId(passed["binary"], passed["test"])
                passes[test] = PassedTest(
                    id=test,
                    seconds=float(passed["seconds"]),
                    position=int(passed["position"]),
                    finished_at=at,
                )
            elif (summary := NEXTEST_SUMMARY.match(output)) is not None:
                phases.append(float(summary["seconds"]))
                phase_end = at
            elif (built := _FAMILY_BUILT.fullmatch(output)) is not None:
                family = _family_headed(built["heading"])
                family_builds[family] = float(built["seconds"])
        return cls(
            passes=MappingProxyType(passes),
            phase_seconds=sum(phases) if phases else None,
            phase_end=phase_end,
            family_builds=MappingProxyType(family_builds),
        )

    @property
    def is_stamped(self) -> bool:
        return any(
            each.finished_at is not None for each in self.passes.values()
        )

    def last_finishers(self, count: int) -> list[PassedTest]:
        by_completion = sorted(
            self.passes.values(), key=lambda each: each.position, reverse=True
        )
        return by_completion[:count]


def _task_output(text: str) -> str:
    unprefixed = _TASK_OUTPUT.fullmatch(ANSI_CSI.sub("", text))
    return "" if unprefixed is None else unprefixed["output"]


def _family_headed(heading: str) -> Family:
    try:
        return _FAMILIES_BY_HEADING[heading]
    except KeyError:
        raise MeasurementRefusedError(
            f"a fixture build line names no known family: {heading!r}"
        ) from None


def _split_at_equals(text: str, shape: str) -> tuple[str, str]:
    left, equals, right = text.partition("=")
    if not equals:
        raise MeasurementRefusedError(f"expected {shape}: {text!r}")
    return left, right


@dataclass(frozen=True)
class Charge:
    """A state-equality test whose duration counts towards its target's."""

    checker: TestId
    target: TestId

    @classmethod
    def parse(cls, text: str) -> Self:
        checker, target = _split_at_equals(text, "<checker>=<target>")
        return cls(TestId.parse(checker), TestId.parse(target))


def parse_served_by(text: str) -> tuple[str, Family]:
    binary, name = _split_at_equals(text, "<binary>=<family>")
    try:
        return binary.strip(), Family(name.strip())
    except ValueError:
        known = ", ".join(family.value for family in Family)
        raise MeasurementRefusedError(
            f"no fixture family is named {name!r}; the families are {known}"
        ) from None


@dataclass(frozen=True)
class DurationFigures:
    runs: Sequence[TimedRun]
    charges: Sequence[Charge] = ()
    served_by: Mapping[str, Family] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not self.runs:
            raise MeasurementRefusedError("no run to read durations from")
        for charge in self.charges:
            for test in (charge.checker, charge.target):
                if not any(test in run.passes for run in self.runs):
                    raise MeasurementRefusedError(f"no pass line for {test}")
        for family in self.served_by.values():
            self.family_build(family)

    def per_test(self, binary: str) -> Mapping[TestId, RunFigure]:
        tests = dict.fromkeys(
            test
            for run in self.runs
            for test in run.passes
            if test.binary == binary
        )
        if not tests:
            raise MeasurementRefusedError(f"no pass line for {binary}")
        return MappingProxyType(
            {
                test: RunFigure.over(
                    self._charged(run, test) for run in self.runs
                )
                for test in tests
            }
        )

    def charges_to(self, test: TestId) -> tuple[TestId, ...]:
        return tuple(
            charge.checker for charge in self.charges if charge.target == test
        )

    def binary_seconds(self, binary: str) -> RunFigure:
        self.per_test(binary)
        return RunFigure.over(
            self._binary_seconds_in(run, binary) for run in self.runs
        )

    def slowest_test(self, binary: str) -> tuple[TestId, RunFigure]:
        return max(
            self.per_test(binary).items(), key=lambda item: item[1].median
        )

    def family_build(self, family: Family) -> RunFigure:
        return _figure_over(
            (run.family_builds.get(family) for run in self.runs),
            missing=f"no build line for {family.noun}",
        )

    def built_families(self) -> tuple[Family, ...]:
        return tuple(
            family
            for family in Family
            if any(family in run.family_builds for run in self.runs)
        )

    def _charged(self, run: TimedRun, test: TestId) -> float | None:
        seconds = [
            run.passes.get(each) for each in (test, *self.charges_to(test))
        ]
        if any(each is None for each in seconds):
            return None
        return sum(each.seconds for each in seconds if each is not None)

    def _binary_seconds_in(self, run: TimedRun, binary: str) -> float | None:
        own = [
            each.seconds
            for test, each in run.passes.items()
            if test.binary == binary
        ]
        if not own:
            return None
        family = self.served_by.get(binary)
        if family is None:
            return sum(own)
        build = run.family_builds.get(family)
        return None if build is None else sum(own) + build


def _figure_over(values: Iterable[float | None], *, missing: str) -> RunFigure:
    per_run = tuple(values)
    if all(each is None for each in per_run):
        raise MeasurementRefusedError(missing)
    return RunFigure.over(per_run)


def _seconds(value: float | None) -> str:
    return "—" if value is None else f"{value:.1f}"


def _figure(figure: RunFigure) -> str:
    runs = " / ".join(_seconds(each) for each in figure.per_run)
    missing = "".join(
        f", missing from run {run}" for run in figure.missing_from
    )
    return f"{runs} s, median {_seconds(figure.median)} s{missing}"


def duration_report(
    figures: DurationFigures, binaries: Sequence[str], *, last: int = 3
) -> str:
    lines: list[str] = []
    for binary in binaries:
        lines.extend(_binary_lines(figures, binary))
    lines.extend(
        f"{family.noun} built in {_figure(figures.family_build(family))}"
        for family in figures.built_families()
    )
    for number, run in enumerate(figures.runs, start=1):
        if run.is_stamped:
            lines.extend(_finisher_lines(number, run, last))
    return "".join(f"{line}\n" for line in lines)


def _binary_lines(figures: DurationFigures, binary: str) -> list[str]:
    per_test = figures.per_test(binary)
    lines = [binary]
    by_median = sorted(per_test.items(), key=lambda item: -item[1].median)
    for test, figure in by_median:
        charged = "".join(
            f" (charged with {checker.test})"
            for checker in figures.charges_to(test)
        )
        lines.append(f"  {test.test}{charged}: {_figure(figure)}")
    served = figures.served_by.get(binary)
    total = "total test-seconds" + (
        "" if served is None else f" with {served.noun}'s build"
    )
    lines.append(f"  {total}: {_figure(figures.binary_seconds(binary))}")
    slowest, figure = figures.slowest_test(binary)
    lines.append(
        f"  slowest test: {slowest.test}, median {_seconds(figure.median)} s"
    )
    return lines


def _finisher_lines(number: int, run: TimedRun, last: int) -> list[str]:
    lines = [f"run {number} phase ends at {_seconds(run.phase_end)} s"]
    lines.extend(
        f"  ({each.position}) {each.id}: {_seconds(each.seconds)} s, "
        f"finished at {_seconds(each.finished_at)} s"
        for each in run.last_finishers(last)
    )
    return lines
