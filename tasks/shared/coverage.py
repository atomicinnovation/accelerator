"""Coverage totals from a cargo-llvm-cov summary export."""

import dataclasses
import json
from collections.abc import Mapping
from enum import StrEnum
from typing import override


class CoverageReportError(Exception): ...


class Metric(StrEnum):
    LINES = "lines"
    FUNCTIONS = "functions"
    REGIONS = "regions"


def _require_every_metric(percents: Mapping[Metric, float]) -> None:
    missing = [metric for metric in Metric if metric not in percents]
    if missing:
        raise ValueError(f"no percentage for {', '.join(missing)}")


@dataclasses.dataclass(frozen=True)
class CoverageTotals:
    percents: Mapping[Metric, float]

    def __post_init__(self) -> None:
        _require_every_metric(self.percents)

    def summary_line(self) -> str:
        return "coverage: " + ", ".join(
            f"{metric} {self.percents[metric]:.2f}%" for metric in Metric
        )


@dataclasses.dataclass(frozen=True)
class Shortfall:
    metric: Metric
    total: float
    floor: float

    @override
    def __str__(self) -> str:
        return f"{self.metric} {self.total:.2f}% (floor {self.floor:g}%)"


@dataclasses.dataclass(frozen=True)
class CoverageFloor:
    percents: Mapping[Metric, float]

    def __post_init__(self) -> None:
        _require_every_metric(self.percents)
        impossible = [m for m, p in self.percents.items() if not 0 <= p <= 100]
        if impossible:
            raise ValueError(f"floor outside 0-100 for {', '.join(impossible)}")

    def shortfalls(self, totals: CoverageTotals) -> list[Shortfall]:
        return [
            Shortfall(metric, totals.percents[metric], self.percents[metric])
            for metric in Metric
            if totals.percents[metric] < self.percents[metric]
        ]


def parse_totals(export: str) -> CoverageTotals:
    try:
        document = json.loads(export)
    except json.JSONDecodeError as error:
        raise CoverageReportError(
            f"the coverage report is not JSON ({export[:80]!r})"
        ) from error
    try:
        totals = document["data"][0]["totals"]
        return CoverageTotals(
            {
                metric: round(float(totals[metric]["percent"]), 2)
                for metric in Metric
            }
        )
    except (LookupError, TypeError, ValueError) as error:
        raise CoverageReportError(
            f"the coverage report carries no totals ({error!r})"
        ) from error
