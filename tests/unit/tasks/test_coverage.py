import json
from pathlib import Path

import pytest

from tasks.shared.coverage import (
    CoverageReportError,
    CoverageTotals,
    Metric,
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
