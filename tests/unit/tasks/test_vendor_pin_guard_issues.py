import datetime as dt
import io

import pytest

from tasks.shared.vendor.pin_guard.findings import (
    RELEASING_FEEDS_URL,
    RELEASING_GUARD_URL,
    AdvisoryFinding,
    KeyExpiryFinding,
    KeyringAgeFinding,
    PinAgeFinding,
    UnassessableAdvisoryFinding,
)
from tasks.shared.vendor.pin_guard.issues import (
    LABEL,
    DryRunIssueTracker,
    ExistingIssue,
    IssuePolicy,
    IssueTrackerError,
    reconcile,
)
from tasks.shared.vendor.pin_guard.local_inputs import Owner, Pin, PinName
from tasks.shared.vendor.pin_guard.markers import IssueMarker, MarkerKind
from tasks.shared.vendor.pin_guard.report import GuardReport
from tests.unit.tasks.shared.doubles import FakeIssueTracker, StubFinding

BOT = "github-actions[bot]"
OWNER = Owner("Toby Clemson", "tobyclemson")
POLICY = IssuePolicy(BOT)
BUMPED = dt.date(2026, 5, 18)


def _pin_age(version="1193", name=PinName.CHROMIUM):
    return PinAgeFinding(Pin(name, version, BUMPED), 145, 90)


def _keyring_age(bumped=dt.date(2025, 8, 24)):
    return KeyringAgeFinding(bumped, 412, 365)


def _key_expiry(expires_on=dt.date(2026, 12, 9), days_to_expiry=60):
    return KeyExpiryFinding(
        "86C8D74642E67846F8E120284DAA80D1E737BC9F", expires_on, days_to_expiry
    )


def _issue(finding_or_marker, *, is_open=True, author=BOT, number=1, tail=""):
    marker = getattr(finding_or_marker, "marker", finding_or_marker)
    return ExistingIssue(
        number, is_open, author, f"Body.\n\n{marker.render()}{tail}"
    )


def _reconcile(tracker, *findings, policy=POLICY):
    return reconcile(GuardReport(tuple(findings)), tracker, OWNER, policy)


def _stubs(count):
    return [StubFinding(f"s{index:02}") for index in range(count)]


class TestOpening:
    def test_a_missing_label_is_created_and_the_issue_carries_it(self):
        tracker = FakeIssueTracker(label_exists=False)
        outcome = _reconcile(tracker, _pin_age())
        assert tracker.labels_created == 1
        assert tracker.opened_markers == [_pin_age().marker]
        assert outcome.opened == (1001,)
        assert tracker.calls == ["issues", "ensure_label", "open_issue"]

    def test_every_issue_is_assigned_to_the_owner(self):
        tracker = FakeIssueTracker()
        _reconcile(tracker, _pin_age(), _keyring_age())
        assert {assignee for _, assignee in tracker.opened} == {"tobyclemson"}

    def test_no_findings_writes_nothing(self):
        tracker = FakeIssueTracker(label_exists=False)
        outcome = _reconcile(tracker)
        assert tracker.calls == ["issues"]
        assert outcome.opened == ()
        assert not outcome.tripped

    def test_two_findings_sharing_a_marker_open_one_issue(self):
        tracker = FakeIssueTracker()
        _reconcile(tracker, _pin_age(), _pin_age())
        assert tracker.opened_markers == [_pin_age().marker]

    def test_drafts_open_in_marker_order(self):
        tracker = FakeIssueTracker()
        later, earlier = StubFinding("b"), StubFinding("a")
        _reconcile(tracker, later, earlier)
        assert tracker.opened_markers == [earlier.marker, later.marker]


class TestSuppression:
    @pytest.mark.parametrize("is_open", [True, False])
    def test_a_trusted_issue_of_either_state_suppresses(self, is_open):
        tracker = FakeIssueTracker([_issue(_pin_age(), is_open=is_open)])
        _reconcile(tracker, _pin_age())
        assert tracker.opened == []
        assert "ensure_label" not in tracker.calls

    def test_the_same_marker_by_another_author_does_not_suppress(self):
        tracker = FakeIssueTracker([_issue(_pin_age(), author="mallory")])
        _reconcile(tracker, _pin_age())
        assert tracker.opened_markers == [_pin_age().marker]

    def test_a_marker_not_on_the_last_line_does_not_suppress(self):
        tracker = FakeIssueTracker([_issue(_pin_age(), tail="\nA later note.")])
        _reconcile(tracker, _pin_age())
        assert tracker.opened_markers == [_pin_age().marker]

    def test_a_closed_pin_age_issue_for_an_earlier_version_does_not(self):
        earlier = _pin_age(version="1187")
        tracker = FakeIssueTracker([_issue(earlier, is_open=False)])
        _reconcile(tracker, _pin_age())
        assert tracker.opened_markers == [_pin_age().marker]

    def test_a_closed_keyring_age_issue_for_an_earlier_bump_does_not(self):
        earlier = _keyring_age(dt.date(2024, 8, 1))
        tracker = FakeIssueTracker([_issue(earlier, is_open=False)])
        _reconcile(tracker, _keyring_age())
        assert tracker.opened_markers == [_keyring_age().marker]

    def test_a_closed_key_expiry_issue_does_not_cover_a_re_signed_key(self):
        before = _key_expiry(dt.date(2026, 12, 9))
        after = _key_expiry(dt.date(2028, 12, 9), days_to_expiry=-1)
        tracker = FakeIssueTracker([_issue(before, is_open=False)])
        _reconcile(tracker, after)
        assert tracker.opened_markers == [after.marker]

    def test_a_key_expiry_issue_from_the_window_covers_the_lapsed_key(self):
        nearing = _key_expiry(days_to_expiry=60)
        lapsed = _key_expiry(days_to_expiry=-3)
        tracker = FakeIssueTracker([_issue(nearing, is_open=False)])
        _reconcile(tracker, lapsed)
        assert tracker.opened == []


class TestIssueCap:
    def test_eleven_drafts_open_one_tripped_issue_listing_all_eleven(self):
        tracker = FakeIssueTracker(label_exists=False)
        stubs = _stubs(11)
        outcome = _reconcile(tracker, *stubs)
        [(draft, assignee)] = tracker.opened
        assert outcome.tripped
        assert tracker.labels_created == 1
        assert assignee == "tobyclemson"
        assert draft.marker.kind is MarkerKind.GUARD_TRIPPED
        assert "11" in draft.title
        for stub in stubs:
            assert f"`stub {stub.name}`" in draft.body
            assert stub.marker.render() not in draft.body
        assert IssueMarker.of_body(draft.body) == draft.marker

    def test_ten_drafts_open_normally(self):
        tracker = FakeIssueTracker()
        outcome = _reconcile(tracker, *_stubs(10))
        assert len(tracker.opened) == 10
        assert not outcome.tripped

    def test_a_raised_cap_opens_all_eleven(self):
        tracker = FakeIssueTracker()
        _reconcile(tracker, *_stubs(11), policy=IssuePolicy(BOT, 11))
        assert len(tracker.opened) == 11

    def test_suppressed_findings_do_not_count_towards_the_cap(self):
        stubs = _stubs(11)
        tracker = FakeIssueTracker([_issue(stubs[0], is_open=False)])
        outcome = _reconcile(tracker, *stubs)
        assert len(tracker.opened) == 10
        assert not outcome.tripped

    def _tripped_marker(self, stubs):
        tracker = FakeIssueTracker()
        _reconcile(tracker, *stubs)
        return tracker.opened_markers[0]

    def test_an_open_tripped_issue_for_the_same_set_suppresses_another(self):
        stubs = _stubs(11)
        tracker = FakeIssueTracker([_issue(self._tripped_marker(stubs))])
        outcome = _reconcile(tracker, *stubs)
        assert tracker.opened == []
        assert outcome.tripped

    def test_an_open_tripped_issue_for_a_different_set_does_not(self):
        other = self._tripped_marker(_stubs(12))
        tracker = FakeIssueTracker([_issue(other)])
        _reconcile(tracker, *_stubs(11))
        assert len(tracker.opened) == 1
        assert tracker.opened_markers[0] != other

    def test_a_closed_tripped_issue_does_not_suppress(self):
        stubs = _stubs(11)
        marker = self._tripped_marker(stubs)
        tracker = FakeIssueTracker([_issue(marker, is_open=False)])
        _reconcile(tracker, *stubs)
        assert tracker.opened_markers == [marker]

    def test_an_open_tripped_issue_by_another_author_does_not(self):
        stubs = _stubs(11)
        marker = self._tripped_marker(stubs)
        tracker = FakeIssueTracker([_issue(marker, author="mallory")])
        _reconcile(tracker, *stubs)
        assert tracker.opened_markers == [marker]

    def test_after_a_closed_trip_a_smaller_set_opens_every_finding(self):
        stubs = _stubs(11)
        marker = self._tripped_marker(stubs)
        tracker = FakeIssueTracker([_issue(marker, is_open=False)])
        _reconcile(tracker, *stubs[:10])
        assert tracker.opened_markers == [stub.marker for stub in stubs[:10]]


class TestTrackerFailures:
    def test_a_failed_draft_is_reported_while_the_rest_open(self):
        stubs = _stubs(3)
        tracker = FakeIssueTracker(failing={stubs[1].marker})
        outcome = _reconcile(tracker, *stubs)
        assert tracker.opened_markers == [stubs[0].marker, stubs[2].marker]
        [failed] = outcome.failed
        assert failed.draft.marker == stubs[1].marker
        assert isinstance(failed.error, IssueTrackerError)

    def test_a_failed_tripped_issue_is_reported_the_same_way(self):
        stubs = _stubs(11)
        tracker = FakeIssueTracker()
        _reconcile(tracker, *stubs)
        marker = tracker.opened_markers[0]
        failing = FakeIssueTracker(failing={marker})
        outcome = _reconcile(failing, *stubs)
        assert outcome.tripped
        assert [f.draft.marker for f in outcome.failed] == [marker]

    def test_a_failure_listing_issues_propagates(self):
        class Unreachable(FakeIssueTracker):
            def issues(self):
                raise IssueTrackerError("gh api failed")

        with pytest.raises(IssueTrackerError):
            _reconcile(Unreachable(), _pin_age())


CHROMIUM_PIN = Pin(PinName.CHROMIUM, "1193", BUMPED)


def _advisory(fix="1.55.1"):
    return AdvisoryFinding(
        "GHSA-7mvr-c777-76hp",
        Pin(PinName.PLAYWRIGHT_CORE, "1.55.0", BUMPED),
        fix,
    )


@pytest.mark.parametrize(
    ("finding", "facts", "guide"),
    [
        (
            _pin_age(),
            ["chromium", "`1193`", "145 days", "@tobyclemson"],
            RELEASING_GUARD_URL,
        ),
        (
            _keyring_age(),
            ["keyring", "412 days", "@tobyclemson"],
            RELEASING_GUARD_URL,
        ),
        (
            _key_expiry(),
            [
                "`86C8D74642E67846F8E120284DAA80D1E737BC9F`",
                "2026-12-09",
                "in 60 days",
                "@tobyclemson",
            ],
            RELEASING_GUARD_URL,
        ),
        (
            _key_expiry(dt.date(2026, 7, 8), days_to_expiry=-94),
            ["expired on 2026-07-08", "94 days ago"],
            RELEASING_GUARD_URL,
        ),
        (
            _advisory(),
            [
                "`GHSA-7mvr-c777-76hp`",
                "`playwright-core`",
                "`1.55.0`",
                "Fixed in `1.55.1`",
                "@tobyclemson",
            ],
            RELEASING_FEEDS_URL,
        ),
        (
            _advisory(fix=None),
            ["No fixed version is available"],
            RELEASING_FEEDS_URL,
        ),
        (
            AdvisoryFinding(
                "CVE-2025-13223",
                CHROMIUM_PIN,
                "142.0.7444.175",
                browser_version="140.0.7339.186",
            ),
            [
                "`CVE-2025-13223`",
                "`chromium`",
                "`1193`",
                "browser version `140.0.7339.186`",
                "Fixed in `142.0.7444.175`",
                "@tobyclemson",
            ],
            RELEASING_FEEDS_URL,
        ),
        (
            UnassessableAdvisoryFinding(
                "CVE-2026-87491",
                CHROMIUM_PIN,
                "140.0.7339.186",
                "OSV holds no record for it",
            ),
            [
                "`CVE-2026-87491`",
                "`chromium`",
                "`1193`",
                "browser version `140.0.7339.186`",
                "OSV holds no record for it",
                "Assess the CVE by hand",
                "@tobyclemson",
            ],
            RELEASING_FEEDS_URL,
        ),
    ],
    ids=[
        "pin-age",
        "keyring-age",
        "key-expiry",
        "key-expired",
        "advisory",
        "advisory-unfixed",
        "chromium-advisory",
        "chromium-unassessable",
    ],
)
def test_each_kind_drafts_a_body_naming_what_the_owner_needs(
    finding, facts, guide
):
    tracker = FakeIssueTracker()
    _reconcile(tracker, finding)
    [(draft, assignee)] = tracker.opened
    for fact in facts:
        assert fact in draft.body
    assert guide in draft.body
    assert draft.body.splitlines()[-1] == finding.marker.render()
    assert assignee == OWNER.login


class TestDryRun:
    def test_it_reads_through_and_prints_instead_of_writing(self):
        tracker = FakeIssueTracker([_issue(_keyring_age())], label_exists=False)
        out = io.StringIO()
        _reconcile(DryRunIssueTracker(tracker, out), _pin_age(), _keyring_age())
        printed = out.getvalue()
        assert tracker.calls == ["issues"]
        assert tracker.opened == []
        assert LABEL in printed
        draft = _pin_age().draft(OWNER)
        assert draft.title in printed
        assert draft.body in printed
        assert "tobyclemson" in printed
        assert _keyring_age().draft(OWNER).title not in printed
