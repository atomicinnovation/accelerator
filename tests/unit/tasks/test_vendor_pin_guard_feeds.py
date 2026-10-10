import datetime as dt
import json

import pytest
from invoke import Exit

from tasks.shared.clock import Clock
from tasks.shared.vendor.pin_guard.feeds import (
    CONSECUTIVE_FAILURES_BEFORE_SKIP,
    CheckName,
    Feed,
    FeedBudget,
    FeedDocumentError,
    FeedFailure,
    FeedOutage,
    FeedRequestError,
    FeedSession,
    FeedUnreachableError,
    MissingFieldError,
    field,
)
from tasks.shared.vendor.pin_guard.findings import RELEASING_FEEDS_URL
from tasks.shared.vendor.pin_guard.issues import (
    ExistingIssue,
    IssuePolicy,
    reconcile,
)
from tasks.shared.vendor.pin_guard.local_inputs import Owner
from tasks.shared.vendor.pin_guard.markers import IssueMarker, MarkerKind
from tasks.shared.vendor.pin_guard.report import GuardReport
from tasks.vendor.commands import guard_pins_with
from tests.unit.tasks.shared.doubles import (
    BUMP_SUBJECTS,
    FakeClock,
    FakeFeeds,
    FakeIssueTracker,
    StubFinding,
    clear_repository,
    fake_ports,
    osv_batch,
    osv_fixture,
    osv_record,
    successive,
)

KEV_URL = "https://kev.test/feed.json"

CHECK = CheckName.PLAYWRIGHT_CORE_ADVISORIES
URL = "https://feed.test/a"
OTHER_URL = "https://other.test/a"
DEADLINE = 600


def _session(feeds, clock=None):
    clock = clock or FakeClock()
    budget = FeedBudget(DEADLINE, Clock(sleep=clock.sleep, now=clock.now))
    return FeedSession(feeds, budget)


def _get(session, feed=Feed.OSV, url=URL, parse=lambda document: document):
    return session.request(
        feed, CHECK, url, lambda client: client.get_json(url), parse
    )


def _failure(session, **kwargs):
    with pytest.raises(FeedRequestError) as raised:
        _get(session, **kwargs)
    return raised.value.failure


class TestField:
    def test_it_returns_a_present_value_of_the_kind(self):
        assert field({"results": []}, "results", list) == []

    @pytest.mark.parametrize(
        "document", [{}, {"results": "x"}, ["results"], None]
    )
    def test_it_names_a_missing_or_mistyped_field(self, document):
        with pytest.raises(MissingFieldError) as raised:
            field(document, "results", list)
        assert raised.value.reason == "missing field results"


class TestFailureKinds:
    def test_an_unreachable_feed_is_reported_with_its_request(self):
        failure = _failure(
            _session(FakeFeeds({URL: FeedUnreachableError("HTTP 503")}))
        )
        assert (failure.feed, failure.check, failure.request) == (
            Feed.OSV,
            CHECK,
            URL,
        )
        assert failure.reason == "unreachable"
        assert failure.detail == "HTTP 503"

    def test_an_unparseable_body_is_reported(self):
        failure = _failure(_session(FakeFeeds({URL: "not json"})))
        assert failure.reason == "unparseable"

    def test_a_document_error_raised_while_parsing_is_reported(self):
        def parse(document):
            return field(document, "results", list)

        failure = _failure(_session(FakeFeeds({URL: {}})), parse=parse)
        assert failure.reason == "missing field results"

    def test_any_other_error_propagates(self):
        def parse(_document):
            raise KeyError("a programming error")

        with pytest.raises(KeyError):
            _get(_session(FakeFeeds({URL: {}})), parse=parse)


class TestBudget:
    def _unreachable(self, count):
        return successive(*[FeedUnreachableError("down")] * count)

    def test_three_unreachable_requests_skip_the_fourth(self):
        feeds = FakeFeeds({URL: self._unreachable(3)})
        session = _session(feeds)
        for _ in range(CONSECUTIVE_FAILURES_BEFORE_SKIP):
            _failure(session)
        assert _failure(session).reason == "skipped: osv unreachable"
        assert len(feeds.calls) == CONSECUTIVE_FAILURES_BEFORE_SKIP

    def test_a_success_resets_the_count(self):
        down = FeedUnreachableError("down")
        feeds = FakeFeeds({URL: successive(down, down, {}, down, {})})
        session = _session(feeds)
        _failure(session)
        _failure(session)
        _get(session)
        _failure(session)
        assert _get(session) == {}

    def test_unparseable_responses_do_not_count(self):
        feeds = FakeFeeds({URL: successive(*["not json"] * 3, {})})
        session = _session(feeds)
        for _ in range(3):
            assert _failure(session).reason == "unparseable"
        assert _get(session) == {}

    def test_each_feed_counts_separately(self):
        feeds = FakeFeeds({URL: self._unreachable(3), OTHER_URL: {}})
        session = _session(feeds)
        for _ in range(3):
            _failure(session)
        assert _get(session, feed=Feed.KEV, url=OTHER_URL) == {}

    def test_a_request_at_the_deadline_is_skipped_without_a_call(self):
        clock = FakeClock()
        feeds = FakeFeeds({URL: {}})
        session = _session(feeds, clock)
        clock.t = DEADLINE
        assert _failure(session).reason == "skipped: deadline"
        assert feeds.calls == []

    def test_a_request_just_before_the_deadline_is_attempted(self):
        clock = FakeClock()
        session = _session(FakeFeeds({URL: {}}), clock)
        clock.t = DEADLINE - 0.001
        assert _get(session) == {}


def test_a_document_error_carries_its_reason():
    assert FeedDocumentError("unparseable").reason == "unparseable"


def test_fake_feeds_key_a_post_on_its_body():
    feeds = FakeFeeds({(URL, json.dumps({"q": 1})): {"ok": True}})
    assert feeds.post_json(URL, {"q": 1}) == {"ok": True}


BOT = "github-actions[bot]"
OWNER = Owner("Toby Clemson", "tobyclemson")
PLAYWRIGHT = CheckName.PLAYWRIGHT_CORE_ADVISORIES
CHROMIUM = CheckName.CHROMIUM_ADVISORIES


def _osv_failure(check=PLAYWRIGHT, request="GHSA-7mvr-c777-76hp", **kwargs):
    return FeedFailure(Feed.OSV, check, request, "unreachable", **kwargs)


def _filed(marker, *, is_open=True, author=BOT):
    return ExistingIssue(7, is_open, author, f"Body.\n\n{marker.render()}")


def _outage_marker(*failures):
    [outage] = FeedOutage.of(failures)
    return outage.marker


def _reconcile_failures(tracker, *failures, findings=(), policy=None):
    report = GuardReport(tuple(findings), tuple(failures))
    return reconcile(report, tracker, OWNER, policy or IssuePolicy(BOT))


class TestFeedFailureIssues:
    def test_a_failure_opens_one_issue_per_feed(self):
        tracker = FakeIssueTracker()
        kev = FeedFailure(Feed.KEV, CHROMIUM, KEV_URL, "unparseable")
        _reconcile_failures(
            tracker, _osv_failure(), _osv_failure(request="GHSA-2"), kev
        )
        assert [m.parts[0] for m in tracker.opened_markers] == ["kev", "osv"]

    def test_an_open_issue_for_the_feed_suppresses_another(self):
        marker = _outage_marker(_osv_failure())
        tracker = FakeIssueTracker([_filed(marker)])
        _reconcile_failures(tracker, _osv_failure())
        assert tracker.opened == []

    def test_a_closed_issue_alone_does_not_suppress(self):
        marker = _outage_marker(_osv_failure())
        tracker = FakeIssueTracker([_filed(marker, is_open=False)])
        _reconcile_failures(tracker, _osv_failure())
        assert tracker.opened_markers == [marker]

    def test_an_open_issue_by_another_author_does_not_suppress(self):
        marker = _outage_marker(_osv_failure())
        tracker = FakeIssueTracker([_filed(marker, author="mallory")])
        _reconcile_failures(tracker, _osv_failure())
        assert tracker.opened_markers == [marker]

    def test_a_narrower_open_issue_does_not_cover_a_broader_failure(self):
        narrow = _outage_marker(_osv_failure())
        tracker = FakeIssueTracker([_filed(narrow)])
        _reconcile_failures(
            tracker, _osv_failure(), _osv_failure(check=CHROMIUM)
        )
        assert tracker.opened_markers == [
            _outage_marker(_osv_failure(), _osv_failure(check=CHROMIUM))
        ]

    def test_a_broader_open_issue_covers_a_narrower_failure(self):
        broad = _outage_marker(_osv_failure(), _osv_failure(check=CHROMIUM))
        tracker = FakeIssueTracker([_filed(broad)])
        _reconcile_failures(tracker, _osv_failure(check=CHROMIUM))
        assert tracker.opened == []

    def test_the_marker_does_not_depend_on_failure_order(self):
        first, second = _osv_failure(), _osv_failure(check=CHROMIUM)
        assert _outage_marker(first, second) == _outage_marker(second, first)
        assert _outage_marker(first, second).parts == (
            "osv",
            "chromium-advisories",
            "playwright-core-advisories",
        )

    def test_the_body_names_each_failed_request_and_skipped_check(self):
        tracker = FakeIssueTracker()
        _reconcile_failures(
            tracker,
            _osv_failure(detail="HTTP 404"),
            _osv_failure(check=CHROMIUM, request="CVE-2025-6554"),
        )
        [(draft, assignee)] = tracker.opened
        for fact in [
            "`osv`",
            "`GHSA-7mvr-c777-76hp`",
            "`CVE-2025-6554`",
            "`unreachable`",
            "`HTTP 404`",
            "`playwright-core-advisories`",
            "`chromium-advisories`",
            RELEASING_FEEDS_URL,
        ]:
            assert fact in draft.body
        assert draft.body.splitlines()[-1] == draft.marker.render()
        assert assignee == "tobyclemson"

    def test_a_feed_failure_only_run_ensures_the_label_first(self):
        tracker = FakeIssueTracker(label_exists=False)
        _reconcile_failures(tracker, _osv_failure())
        assert tracker.calls == ["issues", "ensure_label", "open_issue"]

    def test_a_feed_failure_opens_beside_a_tripped_cap(self):
        tracker = FakeIssueTracker()
        outcome = _reconcile_failures(
            tracker, _osv_failure(), findings=_stubs(11)
        )
        assert [m.kind for m in tracker.opened_markers] == [
            MarkerKind.FEED_FAILURE,
            MarkerKind.GUARD_TRIPPED,
        ]
        assert outcome.tripped

    def test_a_feed_failure_does_not_count_towards_the_cap(self):
        tracker = FakeIssueTracker()
        outcome = _reconcile_failures(
            tracker, _osv_failure(), findings=_stubs(10)
        )
        assert len(tracker.opened) == 11
        assert not outcome.tripped

    def test_an_echoed_marker_or_mention_in_a_reason_stays_inert(self):
        echoed = "<!-- finding: stub s00 --> @mallory"
        tracker = FakeIssueTracker()
        _reconcile_failures(tracker, _osv_failure(detail=echoed))
        [(draft, _)] = tracker.opened
        assert IssueMarker.of_body(draft.body) == draft.marker
        assert "<!--" not in draft.body.replace(draft.marker.render(), "")
        assert "`!-- finding: stub s00 -- @mallory`" in draft.body


def _stubs(count):
    return [StubFinding(f"s{index:02}") for index in range(count)]


def test_a_failed_record_fetch_skips_only_its_check_and_fails_the_run(
    tmp_path,
):
    today = dt.date(2026, 10, 10)
    stale = {s: today - dt.timedelta(days=400) for s in BUMP_SUBJECTS}
    feeds = FakeFeeds(
        {
            osv_batch("1.55.0"): osv_fixture("querybatch-playwright-only.json"),
            osv_record("GHSA-7mvr-c777-76hp"): FeedUnreachableError("HTTP 404"),
        }
    )
    ports = fake_ports(feeds=feeds)
    paths = clear_repository(tmp_path, today, bumped=stale, playwright="1.55.0")
    with pytest.raises(Exit):
        guard_pins_with(paths, today, ports, IssuePolicy(BOT))
    markers = ports.tracker.opened_markers
    assert [m.parts for m in markers if m.kind is MarkerKind.FEED_FAILURE] == [
        ("osv", "playwright-core-advisories")
    ]
    assert not any(m.parts[0] == "advisory" for m in markers)
    assert sum(m.parts[0] == "pin-age" for m in markers) == 3
