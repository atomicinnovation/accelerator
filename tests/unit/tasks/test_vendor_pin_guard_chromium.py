import datetime as dt
import json

import pytest
from invoke import Exit

from tasks.shared.vendor.pin_guard.chromium_advisories import (
    chromium_advisories,
    comparable_fix,
    is_chromium_component,
)
from tasks.shared.vendor.pin_guard.chromium_versions import ChromiumVersion
from tasks.shared.vendor.pin_guard.feeds import (
    CheckName,
    Feed,
    FeedRecordNotFoundError,
    FeedUnreachableError,
)
from tasks.shared.vendor.pin_guard.findings import RELEASING_FEEDS_URL
from tasks.shared.vendor.pin_guard.issues import (
    ExistingIssue,
    IssuePolicy,
    reconcile,
)
from tasks.shared.vendor.pin_guard.local_inputs import Owner, Pin, PinName
from tasks.shared.vendor.pin_guard.report import GuardReport
from tasks.vendor.commands import guard_pins_with
from tests.unit.tasks.shared.doubles import (
    BUMP_SUBJECTS,
    FakeIssueTracker,
    browsers_document,
    chromium_record,
    clear_feeds,
    clear_repository,
    fake_ports,
    fake_session,
    kev_document,
    kev_entry,
    kev_fixture,
    kev_url,
    osv_fixture,
    osv_record,
    playwright_tarball,
    publish_playwright_core,
)

BOT = "github-actions[bot]"
OWNER = Owner("Toby Clemson", "tobyclemson")
BUMPED = dt.date(2026, 5, 18)
PLAYWRIGHT_CORE = Pin(PinName.PLAYWRIGHT_CORE, "1.55.1", BUMPED)
CHROMIUM = Pin(PinName.CHROMIUM, "1193", BUMPED)
CVE = "CVE-2026-0001"


def _versions(*texts):
    return [ChromiumVersion.parse(text) for text in texts]


def _feeds(browser_version, records, kev=None):
    feeds = publish_playwright_core(
        clear_feeds(), playwright_tarball(browsers_document(browser_version))
    )
    feeds.answer(
        kev_url(), kev or kev_document(*(kev_entry(cve) for cve in records))
    )
    for cve, record in records.items():
        feeds.answer(osv_record(cve), record)
    return feeds


def _check(browser_version, records, kev=None):
    feeds = _feeds(browser_version, records, kev)
    return chromium_advisories(PLAYWRIGHT_CORE, CHROMIUM, fake_session(feeds))


def _matched(browser_version, records, kev=None):
    outcome = _check(browser_version, records, kev)
    assert outcome.failures == ()
    return outcome.findings


def _fixes(browser_version, *fixes):
    findings = _matched(browser_version, {CVE: chromium_record(CVE, *fixes)})
    return [finding.fix for finding in findings]


def _open(tracker, findings):
    return reconcile(
        GuardReport(tuple(findings)), tracker, OWNER, IssuePolicy(BOT)
    )


def _filed(marker, *, is_open=True):
    return ExistingIssue(7, is_open, BOT, f"Body.\n\n{marker.render()}")


class TestComparableFix:
    @pytest.mark.parametrize(
        ("browser", "fixes", "expected"),
        [
            (
                "121.0.6167.5",
                ("120.0.6099.300", "121.0.6167.85"),
                "121.0.6167.85",
            ),
            ("122.0.6261.5", ("120.0.6099.300", "121.0.6167.85"), None),
            (
                "119.0.6045.5",
                ("120.0.6099.300", "121.0.6167.85"),
                "120.0.6099.300",
            ),
            (
                "137.0.7151.40",
                ("136.0.7103.113", "138.0.7204.96"),
                "138.0.7204.96",
            ),
            (
                "138.0.7204.94",
                ("138.0.7204.92", "138.0.7204.96"),
                "138.0.7204.96",
            ),
            (
                "143.0.7499.100",
                ("143.0.3650.80", "143.0.7499.110", "143.0.7499.109"),
                "143.0.7499.110",
            ),
        ],
    )
    def test_the_highest_fix_in_the_nearest_major_at_or_above(
        self, browser, fixes, expected
    ):
        fix = comparable_fix(_versions(*fixes), ChromiumVersion.parse(browser))
        assert fix == (
            None if expected is None else ChromiumVersion.parse(expected)
        )


class TestMatching:
    def test_a_higher_fix_opens_an_issue_naming_the_facts(self):
        tracker = FakeIssueTracker()
        _open(
            tracker,
            _matched(
                "140.0.7339.186", {CVE: chromium_record(CVE, "141.0.7390.54")}
            ),
        )
        [(draft, assignee)] = tracker.opened
        assert draft.marker.parts == ("advisory", CVE, "chromium")
        for fact in [
            f"`{CVE}`",
            "`chromium`",
            "`1193`",
            "`140.0.7339.186`",
            "`141.0.7390.54`",
            RELEASING_FEEDS_URL,
        ]:
            assert fact in draft.body
        assert assignee == "tobyclemson"

    def test_a_fix_equal_to_the_browser_version_opens_nothing(self):
        assert _fixes("140.0.7339.186", "140.0.7339.186") == []

    def test_versions_compare_numerically(self):
        assert _fixes("120.0.6099.9", "120.0.6099.10") == ["120.0.6099.10"]

    @pytest.mark.parametrize(
        ("browser", "expected"),
        [
            ("121.0.6167.5", ["121.0.6167.85"]),
            ("122.0.6261.5", []),
            ("119.0.6045.5", ["120.0.6099.300"]),
        ],
    )
    def test_fixes_across_two_majors(self, browser, expected):
        assert _fixes(browser, "120.0.6099.300", "121.0.6167.85") == expected

    @pytest.mark.parametrize(
        ("browser", "expected"),
        [("137.0.7151.40", ["138.0.7204.96"]), ("139.0.7258.5", [])],
    )
    def test_a_browser_between_fixed_majors(self, browser, expected):
        assert _fixes(browser, "136.0.7103.113", "138.0.7204.96") == expected

    @pytest.mark.parametrize(
        ("browser", "expected"),
        [("138.0.7204.94", ["138.0.7204.96"]), ("138.0.7204.96", [])],
    )
    def test_platform_split_fixes_in_one_major(self, browser, expected):
        record = json.loads(osv_fixture("CVE-2025-6554.json"))
        findings = _matched(browser, {"CVE-2025-6554": record})
        assert [finding.fix for finding in findings] == expected

    def test_fixes_for_other_products_are_ignored(self):
        assert _fixes("140.0.7339.186", "26.2", "18.7.3", "141.0.7390.54") == [
            "141.0.7390.54"
        ]

    def test_the_recorded_clear_cve_does_not_affect_the_pinned_browser(self):
        record = json.loads(osv_fixture("CVE-2025-10585.json"))
        assert _matched("140.0.7339.186", {"CVE-2025-10585": record}) == ()

    def test_a_chromium_cve_not_in_kev_opens_nothing(self):
        feeds = _feeds(
            "140.0.7339.186",
            {CVE: chromium_record(CVE, "141.0.7390.54")},
            kev=kev_document(kev_entry("CVE-2026-0002")),
        )
        feeds.answer(
            osv_record("CVE-2026-0002"),
            chromium_record("CVE-2026-0002", "139.0.7258.5"),
        )
        outcome = chromium_advisories(
            PLAYWRIGHT_CORE, CHROMIUM, fake_session(feeds)
        )
        assert (outcome.findings, outcome.failures) == ((), ())
        assert ("GET", osv_record(CVE)) not in feeds.calls


class TestUnassessable:
    def _unassessable(self, record):
        [finding] = _matched("140.0.7339.186", {CVE: record})
        tracker = FakeIssueTracker()
        _open(tracker, [finding])
        [(draft, _)] = tracker.opened
        assert draft.marker.parts == ("advisory", CVE, "chromium")
        assert "`140.0.7339.186`" in draft.body
        assert "assess" in draft.body
        return draft

    def test_a_cve_osv_holds_no_record_for_is_unassessable(self):
        draft = self._unassessable(FeedRecordNotFoundError("HTTP 404"))
        assert "OSV holds no record" in draft.body

    @pytest.mark.parametrize(
        "record",
        [
            chromium_record(CVE, "26.2", "18.7.3"),
            {"id": CVE, "affected": [{"ranges": [{"type": "GIT"}]}]},
            {"id": CVE, "affected": []},
        ],
        ids=["other products only", "git ranges only", "nothing affected"],
    )
    def test_a_record_naming_no_chromium_fix_is_unassessable(self, record):
        draft = self._unassessable(record)
        assert "names no fixed Chromium version" in draft.body

    def test_a_closed_unassessable_issue_suppresses_a_later_assessment(self):
        [unassessable] = _matched(
            "140.0.7339.186", {CVE: FeedRecordNotFoundError("HTTP 404")}
        )
        [assessed] = _matched(
            "140.0.7339.186", {CVE: chromium_record(CVE, "141.0.7390.54")}
        )
        tracker = FakeIssueTracker([_filed(unassessable.marker, is_open=False)])
        _open(tracker, [assessed])
        assert tracker.opened == []


class TestKevFilter:
    @pytest.mark.parametrize(
        ("vendor", "product", "expected"),
        [
            ("Google", "Chromium V8", True),
            ("Google", "Chromium", True),
            ("Google", "Chrome", True),
            ("Google", "Chrome Blink", True),
            ("Google", "Skia", True),
            ("Google", "Dawn", True),
            ("Google", "Pixel", False),
            ("Microsoft", "Chromium V8", False),
        ],
    )
    def test_chromium_components(self, vendor, product, expected):
        assert is_chromium_component(vendor, product) is expected

    def test_every_recorded_google_product_but_pixel_is_requested(self):
        kev = json.loads(kev_fixture("google-products.json"))
        expected = {
            entry["cveID"]
            for entry in kev["vulnerabilities"]
            if entry["vendorProject"] == "Google"
            and entry["product"] != "Pixel"
        }
        records = {cve: chromium_record(cve, "1.0.0.0") for cve in expected}
        feeds = _feeds("140.0.7339.186", records, kev=kev)
        outcome = chromium_advisories(
            PLAYWRIGHT_CORE, CHROMIUM, fake_session(feeds)
        )
        assert outcome.failures == ()
        requested = {
            url.rsplit("/", 1)[1] for _, url in feeds.calls if "osv.dev" in url
        }
        assert requested == expected


class TestFeedFailures:
    def _reasons(self, records, kev=None):
        outcome = _check("140.0.7339.186", records, kev)
        assert outcome.findings == ()
        assert {f.check for f in outcome.failures} == {
            CheckName.CHROMIUM_ADVISORIES
        }
        return [(f.feed, f.request, f.reason) for f in outcome.failures]

    @pytest.mark.parametrize(
        ("kev", "reason"),
        [
            (FeedUnreachableError("HTTP 503"), "unreachable"),
            ("not json", "unparseable"),
            ({"catalogVersion": "x"}, "missing field vulnerabilities"),
            (
                kev_document({"cveID": CVE, "vendorProject": "Google"}),
                "missing field product",
            ),
            (
                kev_fixture("no-chromium-component.json"),
                "missing field vulnerabilities[].product",
            ),
            (
                kev_document(kev_entry("CVE-26-1")),
                "unexpected CVE CVE-26-1",
            ),
        ],
    )
    def test_each_kev_failure_names_its_reason(self, kev, reason):
        assert self._reasons({}, kev) == [(Feed.KEV, kev_url(), reason)]

    def test_an_unreachable_record_fails_the_check(self):
        records = {
            CVE: chromium_record(CVE, "141.0.7390.54"),
            "CVE-2026-0002": FeedUnreachableError("HTTP 503"),
        }
        assert self._reasons(records) == [
            (Feed.OSV, "CVE-2026-0002", "unreachable")
        ]

    def test_a_record_without_affected_is_a_failure(self):
        assert self._reasons({CVE: {"id": CVE}}) == [
            (Feed.OSV, CVE, "missing field affected")
        ]


def test_an_unpinned_revision_aborts_before_any_tracker_call(tmp_path):
    today = dt.date(2026, 10, 10)
    stale = {s: today - dt.timedelta(days=400) for s in BUMP_SUBJECTS}
    feeds = publish_playwright_core(
        clear_feeds(), playwright_tarball(browsers_document(revision="1200"))
    )
    ports = fake_ports(feeds=feeds)
    paths = clear_repository(tmp_path, today, bumped=stale)
    with pytest.raises(Exit, match=r"chromium\.revision 1193") as raised:
        guard_pins_with(paths, today, ports, IssuePolicy(BOT))
    assert raised.value.code == 1
    assert ports.tracker.calls == []
