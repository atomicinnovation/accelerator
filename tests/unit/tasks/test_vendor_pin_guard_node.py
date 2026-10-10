import datetime as dt
import json

import pytest

from tasks.shared.vendor.pin_guard.feeds import (
    CheckName,
    Feed,
    FeedUnreachableError,
)
from tasks.shared.vendor.pin_guard.issues import (
    ExistingIssue,
    IssuePolicy,
    reconcile,
)
from tasks.shared.vendor.pin_guard.local_inputs import Owner, Pin, PinName
from tasks.shared.vendor.pin_guard.node_advisories import (
    VULN_CORE_URL,
    node_advisories,
)
from tasks.shared.vendor.pin_guard.report import GuardReport
from tests.unit.tasks.shared.doubles import (
    FakeFeeds,
    FakeIssueTracker,
    fake_session,
    vuln_core_entry,
    vuln_core_fixture,
)

BOT = "github-actions[bot]"
OWNER = Owner("Toby Clemson", "tobyclemson")
PIN_VERSION = "22.22.2"
UNAFFECTED = vuln_core_entry(vulnerable="20.x", patched="^20.99.0")


def _pin(version=PIN_VERSION):
    return Pin(PinName.NODE, version, dt.date(2026, 8, 24))


def _check(feed, version=PIN_VERSION):
    feeds = FakeFeeds({VULN_CORE_URL: feed})
    return node_advisories(_pin(version), fake_session(feeds))


def _feed(**entries):
    return {"1": UNAFFECTED} | entries


def _matched(feed, version=PIN_VERSION):
    outcome = _check(feed, version)
    assert outcome.failures == ()
    return outcome.findings


def _open(tracker, findings):
    return reconcile(
        GuardReport(tuple(findings)), tracker, OWNER, IssuePolicy(BOT)
    )


class TestMatching:
    @pytest.mark.parametrize("environment", ["all", "darwin", "linux"])
    def test_a_supported_environment_opens_an_issue(self, environment):
        entry = vuln_core_entry(
            environments=(environment,),
            cves=("CVE-2026-0001", "CVE-2026-0002"),
            patched=" ^22.23.0 || ^24.18.1",
        )
        tracker = FakeIssueTracker()
        _open(tracker, _matched(_feed(**{"194": entry})))
        [(draft, assignee)] = tracker.opened
        assert draft.marker.parts == ("advisory", "194", "node")
        for fact in [
            "`194`",
            "`CVE-2026-0001`",
            "`CVE-2026-0002`",
            "`node`",
            f"`{PIN_VERSION}`",
            "`^22.23.0 || ^24.18.1`",
        ]:
            assert fact in draft.body
        assert assignee == "tobyclemson"

    def test_an_unsupported_environment_alone_opens_nothing(self):
        entry = vuln_core_entry(environments=("win32",))
        assert _matched(_feed(**{"2": entry})) == ()

    def test_a_patched_version_inside_the_vulnerable_range_is_clear(self):
        entry = vuln_core_entry(vulnerable="22.x", patched="^22.22.2")
        assert _matched(_feed(**{"2": entry})) == ()

    def test_a_version_outside_the_vulnerable_range_is_clear(self):
        entry = vuln_core_entry(vulnerable="24.x", patched="^24.18.1")
        assert _matched(_feed(**{"2": entry})) == ()

    @pytest.mark.parametrize("cves", [(), ("CVE-2026-0001", "CVE-2026-0002")])
    def test_each_entry_opens_one_issue_across_runs(self, cves):
        feed = _feed(**{"2": vuln_core_entry(cves=cves)})
        tracker = FakeIssueTracker()
        _open(tracker, _matched(feed))
        _open(tracker, _matched(feed))
        assert [m.parts for m in tracker.opened_markers] == [
            ("advisory", "2", "node")
        ]

    def test_an_existing_issue_leaves_only_the_new_match_to_open(self):
        feed = _feed(**{"2": vuln_core_entry(), "3": vuln_core_entry()})
        [first, second] = _matched(feed)
        tracker = FakeIssueTracker([_filed(first.marker)])
        _open(tracker, [first, second])
        assert tracker.opened_markers == [second.marker]

    def test_a_closed_issue_suppresses_its_entry_after_a_bump(self):
        feed = _feed(**{"2": vuln_core_entry(patched="^22.23.0")})
        [before] = _matched(feed)
        [after] = _matched(feed, version="22.22.3")
        tracker = FakeIssueTracker([_filed(before.marker, is_open=False)])
        _open(tracker, [after])
        assert tracker.opened == []

    def test_the_recorded_feed_matches_the_entries_naming_22_x(self):
        feed = json.loads(vuln_core_fixture("index.json"))
        keys = {finding.advisory_id for finding in _matched(feed)}
        assert {"194"} <= keys
        assert "1" not in keys


def _filed(marker, *, is_open=True):
    return ExistingIssue(7, is_open, BOT, f"Body.\n\n{marker.render()}")


class TestFeedFailures:
    def _reasons(self, feed):
        outcome = _check(feed)
        assert outcome.findings == ()
        assert {(f.feed, f.check) for f in outcome.failures} == {
            (Feed.VULN_CORE, CheckName.NODE_ADVISORIES)
        }
        return [(f.request, f.reason) for f in outcome.failures]

    def test_a_non_numeric_key_is_a_failure(self):
        assert self._reasons(_feed(abc=vuln_core_entry())) == [
            (VULN_CORE_URL, "unexpected entry key abc")
        ]

    def test_a_malformed_cve_is_a_failure(self):
        entry = vuln_core_entry(cves=("CVE-26-1",))
        assert self._reasons(_feed(**{"2": entry})) == [
            (VULN_CORE_URL, "unexpected CVE CVE-26-1")
        ]

    def test_renamed_environments_are_a_missing_field(self):
        assert self._reasons(
            vuln_core_fixture("drifted-environments.json")
        ) == [(VULN_CORE_URL, "missing field affectedEnvironments")]

    def test_an_entry_without_a_vulnerable_range_is_a_missing_field(self):
        assert self._reasons(vuln_core_fixture("missing-field.json")) == [
            (VULN_CORE_URL, "missing field vulnerable")
        ]

    def test_an_unparseable_feed_is_a_failure(self):
        assert self._reasons(vuln_core_fixture("not-json.txt")) == [
            (VULN_CORE_URL, "unparseable")
        ]

    def test_an_unreachable_feed_is_a_failure(self):
        assert self._reasons(FeedUnreachableError("HTTP 503")) == [
            (VULN_CORE_URL, "unreachable")
        ]

    def test_a_range_outside_the_supported_syntax_is_a_failure(self):
        entry = vuln_core_entry(vulnerable="~22.1.0")
        assert self._reasons(_feed(**{"2": entry})) == [
            (VULN_CORE_URL, "unparseable range ~22.1.0")
        ]

    def test_a_feed_that_is_not_keyed_by_entry_is_a_failure(self):
        assert self._reasons([vuln_core_entry()]) == [
            (VULN_CORE_URL, "unexpected document shape")
        ]
