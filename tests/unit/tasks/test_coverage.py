import json
from pathlib import Path

import pytest

from tasks.shared.coverage import (
    CoverageFloor,
    CoverageReportError,
    CoverageTotals,
    Metric,
    Shortfall,
    parse_totals,
)

FIXTURE = Path(__file__).parent / "fixtures/coverage/llvm-cov-summary.json"


def _export(**percents: object) -> str:
    totals = {
        metric: {"percent": percent} for metric, percent in percents.items()
    }
    return json.dumps({"data": [{"files": [], "totals": totals}]})


def _fixture_totals() -> dict:
    return json.loads(FIXTURE.read_text())["data"][0]["totals"]


class TestParseTotals:
    def test_reads_the_totals_of_a_real_summary_export(self):
        expected = _fixture_totals()

        totals = parse_totals(FIXTURE.read_text())

        assert totals.percents == {
            metric: round(expected[metric]["percent"], 2) for metric in Metric
        }

    @pytest.mark.parametrize(
        ("percent", "rounded"),
        [(77.98742138364779, 77.99), (76.996, 77.0)],
    )
    def test_rounds_to_two_decimals(self, percent: float, rounded: float):
        export = _export(lines=percent, functions=80, regions=70)

        assert parse_totals(export).percents[Metric.LINES] == rounded

    def test_rejects_text_that_is_not_json(self):
        with pytest.raises(CoverageReportError, match="not JSON"):
            parse_totals("error: no profile data")

    @pytest.mark.parametrize(
        "document",
        [
            {},
            {"data": []},
            {"data": [{"files": []}]},
            {"data": [{"totals": {"lines": {"percent": 1}}}]},
            {
                "data": [
                    {
                        "totals": {
                            "lines": {"percent": "lots"},
                            "functions": {"percent": 1},
                            "regions": {"percent": 1},
                        }
                    }
                ]
            },
        ],
        ids=[
            "no data",
            "empty data",
            "no totals",
            "missing metric",
            "non-numeric percent",
        ],
    )
    def test_rejects_an_export_without_totals(self, document: dict):
        with pytest.raises(CoverageReportError, match="carries no totals"):
            parse_totals(json.dumps(document))


class TestCoverageTotals:
    def test_summary_line_shows_each_metric_to_two_decimals(self):
        totals = CoverageTotals(
            {Metric.LINES: 77.99, Metric.FUNCTIONS: 81.2, Metric.REGIONS: 74.05}
        )

        assert totals.summary_line() == (
            "coverage: lines 77.99%, functions 81.20%, regions 74.05%"
        )

    def test_summary_line_of_a_real_export_names_no_files(self):
        files = json.loads(FIXTURE.read_text())["data"][0]["files"]

        line = parse_totals(FIXTURE.read_text()).summary_line()

        assert "{" not in line
        for file in files:
            assert file["filename"] not in line

    def test_requires_every_metric(self):
        with pytest.raises(ValueError, match="regions"):
            CoverageTotals({Metric.LINES: 1.0, Metric.FUNCTIONS: 1.0})


FLOOR = CoverageFloor(
    {Metric.LINES: 70, Metric.FUNCTIONS: 80, Metric.REGIONS: 60}
)


def _totals(**overrides: float) -> CoverageTotals:
    percents = {
        Metric.LINES: 75.0,
        Metric.FUNCTIONS: 85.0,
        Metric.REGIONS: 65.0,
    }
    percents.update({Metric(name): value for name, value in overrides.items()})
    return CoverageTotals(percents)


class TestCoverageFloor:
    def test_totals_above_every_floor_fall_short_of_none(self):
        assert FLOOR.shortfalls(_totals()) == []

    def test_totals_exactly_at_every_floor_fall_short_of_none(self):
        assert (
            FLOOR.shortfalls(_totals(lines=70, functions=80, regions=60)) == []
        )

    @pytest.mark.parametrize("metric", list(Metric))
    def test_one_metric_below_its_own_floor_falls_short_once(
        self, metric: Metric
    ):
        below = FLOOR.percents[metric] - 0.5

        shortfalls = FLOOR.shortfalls(_totals(**{metric: below}))

        assert shortfalls == [Shortfall(metric, below, FLOOR.percents[metric])]

    def test_every_metric_below_falls_short_in_metric_order(self):
        shortfalls = FLOOR.shortfalls(_totals(lines=1, functions=2, regions=3))

        assert [shortfall.metric for shortfall in shortfalls] == list(Metric)

    def test_a_shortfall_reads_as_its_total_against_its_floor(self):
        assert str(Shortfall(Metric.LINES, 69.5, 70)) == (
            "lines 69.50% (floor 70%)"
        )

    def test_requires_every_metric(self):
        with pytest.raises(ValueError, match="functions"):
            CoverageFloor({Metric.LINES: 70, Metric.REGIONS: 60})

    @pytest.mark.parametrize("impossible", [-1, 100.5])
    def test_rejects_a_floor_outside_zero_to_a_hundred(self, impossible: float):
        with pytest.raises(ValueError, match="regions"):
            CoverageFloor(
                {
                    Metric.LINES: 70,
                    Metric.FUNCTIONS: 80,
                    Metric.REGIONS: impossible,
                }
            )
