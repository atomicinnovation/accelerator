import datetime as dt
import json

import pytest

from tasks.shared.vendor.pin_guard.feeds import (
    CheckName,
    Feed,
    FeedUnreachableError,
)
from tasks.shared.vendor.pin_guard.local_inputs import Pin, PinName
from tasks.shared.vendor.pin_guard.osv import (
    QUERYBATCH_URL,
    playwright_core_advisories,
)
from tests.unit.tasks.shared.doubles import (
    FakeFeeds,
    fake_session,
    osv_batch,
    osv_fixture,
    osv_record,
)

GHSA = "GHSA-7mvr-c777-76hp"
TWO_RANGES = "GHSA-2222-3333-4444"
LAST_AFFECTED = "GHSA-cfgh-jmpq-rvwx"


def _pin(version):
    return Pin(PinName.PLAYWRIGHT_CORE, version, dt.date(2026, 5, 18))


def _batch(*advisory_ids):
    vulns = [{"id": advisory_id} for advisory_id in advisory_ids]
    return json.dumps({"results": [{"vulns": vulns}, {}]})


def _check(version, batch, records=None):
    feeds = FakeFeeds({osv_batch(version): batch})
    for advisory_id, record in (records or {}).items():
        feeds.answer(osv_record(advisory_id), record)
    return playwright_core_advisories(_pin(version), fake_session(feeds)), feeds


def _fixes(outcome):
    assert outcome.failures == ()
    return [(f.advisory_id, f.fix) for f in outcome.findings]


class TestMatching:
    def test_an_advisory_against_playwright_only_is_found(self):
        outcome, _ = _check(
            "1.55.0",
            osv_fixture("querybatch-playwright-only.json"),
            {GHSA: osv_fixture(f"{GHSA}.json")},
        )
        assert _fixes(outcome) == [(GHSA, "1.55.1")]
        assert outcome.findings[0].pin == _pin("1.55.0")

    def test_one_advisory_under_both_packages_is_one_finding(self):
        outcome, feeds = _check(
            "1.55.0",
            osv_fixture("querybatch-both.json"),
            {GHSA: osv_fixture(f"{GHSA}.json")},
        )
        assert _fixes(outcome) == [(GHSA, "1.55.1")]
        assert feeds.calls.count(("GET", osv_record(GHSA))) == 1

    def test_the_pin_at_the_fix_is_not_affected(self):
        outcome, _ = _check(
            "1.55.1",
            _batch(GHSA),
            {GHSA: osv_fixture(f"{GHSA}.json")},
        )
        assert _fixes(outcome) == []

    def test_the_empty_batch_finds_nothing(self):
        outcome, _ = _check("1.55.1", osv_fixture("querybatch-empty.json"))
        assert _fixes(outcome) == []

    @pytest.mark.parametrize(
        ("version", "fix"),
        [("1.55.0", "1.56.2"), ("1.39.0", "1.40.0")],
    )
    def test_the_fix_of_the_range_holding_the_pin_is_named(self, version, fix):
        outcome, _ = _check(
            version,
            _batch(TWO_RANGES),
            {TWO_RANGES: osv_fixture("two-ranges.json")},
        )
        assert _fixes(outcome) == [(TWO_RANGES, fix)]

    def test_a_pin_between_the_ranges_is_not_affected(self):
        outcome, _ = _check(
            "1.45.0",
            _batch(TWO_RANGES),
            {TWO_RANGES: osv_fixture("two-ranges.json")},
        )
        assert _fixes(outcome) == []

    @pytest.mark.parametrize(
        ("version", "affected"),
        [("1.55.1", True), ("1.55.2", False), ("1.49.9", False)],
    )
    def test_last_affected_bounds_the_range(self, version, affected):
        outcome, _ = _check(
            version,
            _batch(LAST_AFFECTED),
            {LAST_AFFECTED: osv_fixture("last-affected.json")},
        )
        expected = [(LAST_AFFECTED, None)] if affected else []
        assert _fixes(outcome) == expected

    def test_an_open_ended_range_is_affected_with_no_fix(self):
        record = {
            "id": GHSA,
            "affected": [
                {
                    "package": {"name": "playwright", "ecosystem": "npm"},
                    "ranges": [
                        {"type": "SEMVER", "events": [{"introduced": "1.0.0"}]}
                    ],
                }
            ],
        }
        outcome, _ = _check("1.55.1", _batch(GHSA), {GHSA: record})
        assert _fixes(outcome) == [(GHSA, None)]


class TestFeedFailures:
    def _reasons(self, outcome):
        assert outcome.findings == ()
        return [(f.request, f.reason) for f in outcome.failures]

    def test_a_next_page_token_is_a_failure(self):
        outcome, _ = _check("1.55.0", osv_fixture("querybatch-next-page.json"))
        assert self._reasons(outcome) == [
            (QUERYBATCH_URL, "truncated at next_page_token")
        ]

    def test_a_non_ghsa_id_is_a_failure(self):
        outcome, _ = _check("1.55.0", osv_fixture("querybatch-non-ghsa.json"))
        assert self._reasons(outcome) == [
            (QUERYBATCH_URL, "unexpected advisory id MAL-2026-1234")
        ]

    def test_a_batch_vuln_without_an_id_is_a_missing_field(self):
        outcome, _ = _check(
            "1.55.0", osv_fixture("querybatch-missing-field.json")
        )
        assert self._reasons(outcome) == [(QUERYBATCH_URL, "missing field id")]

    def test_an_unparseable_batch_is_a_failure(self):
        outcome, _ = _check("1.55.0", osv_fixture("not-json.txt"))
        assert self._reasons(outcome) == [(QUERYBATCH_URL, "unparseable")]

    def test_a_record_with_no_npm_range_is_a_missing_field(self):
        outcome, _ = _check(
            "1.55.0",
            _batch("GHSA-5555-6666-7777"),
            {"GHSA-5555-6666-7777": osv_fixture("no-npm-range.json")},
        )
        assert self._reasons(outcome) == [
            ("GHSA-5555-6666-7777", "missing field affected[].ranges")
        ]

    def test_an_unparseable_event_version_is_a_failure(self):
        record = json.loads(osv_fixture(f"{GHSA}.json"))
        record["affected"][0]["ranges"][0]["events"][1]["fixed"] = "1.55"
        outcome, _ = _check("1.55.0", _batch(GHSA), {GHSA: record})
        assert self._reasons(outcome) == [(GHSA, "unparseable version 1.55")]

    def test_every_failed_record_is_reported(self):
        outcome, _ = _check(
            "1.55.0",
            _batch(GHSA, TWO_RANGES, LAST_AFFECTED),
            {
                GHSA: FeedUnreachableError("HTTP 404"),
                TWO_RANGES: osv_fixture("two-ranges.json"),
                LAST_AFFECTED: FeedUnreachableError("HTTP 404"),
            },
        )
        assert self._reasons(outcome) == [
            (GHSA, "unreachable"),
            (LAST_AFFECTED, "unreachable"),
        ]
        assert {f.feed for f in outcome.failures} == {Feed.OSV}
        assert {f.check for f in outcome.failures} == {
            CheckName.PLAYWRIGHT_CORE_ADVISORIES
        }

    def test_an_unreachable_batch_is_a_failure(self):
        outcome, _ = _check("1.55.0", FeedUnreachableError("HTTP 503"))
        assert self._reasons(outcome) == [(QUERYBATCH_URL, "unreachable")]
