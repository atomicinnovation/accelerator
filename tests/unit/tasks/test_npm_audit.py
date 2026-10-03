import datetime as dt
import json

import pytest

from tasks.shared.npm_audit import (
    AdvisoryIgnore,
    IgnoreListError,
    blocking_advisories,
    lapsed_ignores,
    parse_ignores,
    parse_report,
)

TODAY = dt.date(2026, 10, 3)


def _advisory(ghsa: str, severity: str, package: str = "pkg") -> dict:
    return {
        "source": 1,
        "name": package,
        "dependency": package,
        "title": f"{ghsa} title",
        "url": f"https://github.com/advisories/{ghsa}",
        "severity": severity,
        "range": "<=1.0.0",
    }


def _report(*advisories: dict) -> str:
    vulnerabilities = {}
    for advisory in advisories:
        vulnerabilities[advisory["name"]] = {
            "name": advisory["name"],
            "severity": advisory["severity"],
            "via": [advisory],
        }
    vulnerabilities["dependent"] = {
        "name": "dependent",
        "severity": "high",
        "via": [advisory["name"] for advisory in advisories],
    }
    return json.dumps({"vulnerabilities": vulnerabilities})


def _ignore(ghsa: str, review_by: dt.date = TODAY) -> AdvisoryIgnore:
    return AdvisoryIgnore(ghsa, "no fix upstream", review_by)


class TestParseReport:
    def test_reads_each_advisory_once_with_its_ghsa_id(self):
        report = parse_report(_report(_advisory("GHSA-aaaa", "high", "left")))
        assert [(a.ghsa_id, a.severity, a.package) for a in report] == [
            ("GHSA-aaaa", "high", "left")
        ]

    def test_an_advisory_reached_through_two_packages_appears_once(self):
        shared = _advisory("GHSA-aaaa", "high")
        text = json.dumps(
            {
                "vulnerabilities": {
                    "one": {"via": [shared]},
                    "two": {"via": [shared]},
                }
            }
        )
        assert len(parse_report(text)) == 1

    def test_an_empty_report_has_no_advisories(self):
        assert parse_report(json.dumps({"vulnerabilities": {}})) == []


class TestBlockingAdvisories:
    def test_high_and_critical_advisories_block(self):
        report = parse_report(
            _report(
                _advisory("GHSA-high", "high", "a"),
                _advisory("GHSA-crit", "critical", "b"),
            )
        )
        blocking = blocking_advisories(report, [], TODAY)
        assert {a.ghsa_id for a in blocking} == {"GHSA-high", "GHSA-crit"}

    def test_moderate_and_low_advisories_do_not_block(self):
        report = parse_report(
            _report(
                _advisory("GHSA-mod", "moderate", "a"),
                _advisory("GHSA-low", "low", "b"),
            )
        )
        assert blocking_advisories(report, [], TODAY) == []

    def test_an_ignored_advisory_does_not_block(self):
        report = parse_report(_report(_advisory("GHSA-aaaa", "high")))
        ignores = [_ignore("GHSA-aaaa")]
        assert blocking_advisories(report, ignores, TODAY) == []

    def test_an_ignore_past_its_review_by_date_no_longer_suppresses(self):
        report = parse_report(_report(_advisory("GHSA-aaaa", "high")))
        ignores = [_ignore("GHSA-aaaa", TODAY - dt.timedelta(days=1))]
        assert len(blocking_advisories(report, ignores, TODAY)) == 1

    def test_an_ignore_spares_only_its_own_advisory(self):
        report = parse_report(
            _report(
                _advisory("GHSA-aaaa", "high", "a"),
                _advisory("GHSA-bbbb", "high", "b"),
            )
        )
        blocking = blocking_advisories(report, [_ignore("GHSA-aaaa")], TODAY)
        assert [a.ghsa_id for a in blocking] == ["GHSA-bbbb"]


class TestLapsedIgnores:
    def test_names_ignores_whose_review_by_date_has_passed(self):
        live = _ignore("GHSA-live", TODAY)
        lapsed = _ignore("GHSA-gone", TODAY - dt.timedelta(days=1))
        assert lapsed_ignores([live, lapsed], TODAY) == [lapsed]


class TestParseIgnores:
    def test_reads_id_reason_and_review_by_date(self):
        text = (
            "ignore = [\n"
            '  { id = "GHSA-aaaa", '
            'reason = "no fix. review-by: 2026-11-03" },\n'
            "]\n"
        )
        assert parse_ignores(text) == [
            AdvisoryIgnore(
                "GHSA-aaaa",
                "no fix. review-by: 2026-11-03",
                dt.date(2026, 11, 3),
            )
        ]

    def test_an_ignore_without_a_review_by_date_is_refused(self):
        text = 'ignore = [{ id = "GHSA-aaaa", reason = "no fix" }]\n'
        with pytest.raises(IgnoreListError, match="GHSA-aaaa"):
            parse_ignores(text)

    def test_an_ignore_without_a_reason_is_refused(self):
        text = 'ignore = [{ id = "GHSA-aaaa", reason = " " }]\n'
        with pytest.raises(IgnoreListError, match="GHSA-aaaa"):
            parse_ignores(text)

    def test_no_ignore_table_means_no_ignores(self):
        assert parse_ignores("") == []
