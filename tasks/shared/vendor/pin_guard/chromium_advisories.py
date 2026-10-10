"""Exploited Chromium CVEs from KEV, with fixed versions from OSV.

OSV's Chromium ranges sit outside its core schema and mix in other products'
versions (Edge, Apple, WebKitGTK) without saying which is which. Only four-part
versions are Chromium-shaped, and Edge's build numbers sit below Chrome's, so
the highest four-part fix in the nearest major is Chrome's.
"""

import re
from collections.abc import Iterable
from dataclasses import dataclass
from typing import Self

from tasks.shared.vendor.pin_guard.checks import CheckOutcome
from tasks.shared.vendor.pin_guard.chromium_versions import (
    ChromiumVersion,
    ChromiumVersionError,
)
from tasks.shared.vendor.pin_guard.feeds import (
    CheckName,
    Feed,
    FeedClient,
    FeedDocumentError,
    FeedFailure,
    FeedRecordNotFoundError,
    FeedRequestError,
    FeedSession,
    MissingFieldError,
    field,
)
from tasks.shared.vendor.pin_guard.findings import (
    AdvisoryFinding,
    Finding,
    UnassessableAdvisoryFinding,
)
from tasks.shared.vendor.pin_guard.local_inputs import Pin
from tasks.shared.vendor.pin_guard.npm_registry import (
    PinnedBrowser,
    pinned_browser_build,
)
from tasks.shared.vendor.pin_guard.osv import record_url

KEV_URL = (
    "https://www.cisa.gov/sites/default/files/feeds/"
    "known_exploited_vulnerabilities.json"
)
CHROMIUM_VENDOR = "Google"
CHROMIUM_COMPONENT_PREFIXES = ("Chromium", "Chrome")
CHROMIUM_COMPONENTS = frozenset({"Skia", "Dawn"})
CVE_ID = re.compile(r"^CVE-\d{4}-\d{4,}$")
NO_RECORD = "OSV holds no record for it"
NO_FIX = "its OSV record names no fixed Chromium version"
_CHECK = CheckName.CHROMIUM_ADVISORIES


def is_chromium_component(vendor_project: str, product: str) -> bool:
    return vendor_project == CHROMIUM_VENDOR and (
        product.startswith(CHROMIUM_COMPONENT_PREFIXES)
        or product in CHROMIUM_COMPONENTS
    )


def comparable_fix(
    fixes: Iterable[ChromiumVersion], browser: ChromiumVersion
) -> ChromiumVersion | None:
    """Return the fix to hold ``browser`` against, if any is not older.

    That is the highest fix in the lowest major at or above the browser's: a
    browser between two fixed majors is not covered by the older one, and one
    platform's earlier fix does not cover another's later one.
    """
    candidates = [fix for fix in fixes if fix.major >= browser.major]
    if not candidates:
        return None
    nearest = min(fix.major for fix in candidates)
    return max(fix for fix in candidates if fix.major == nearest)


def kev_chromium_cves(document: object) -> tuple[str, ...]:
    """Every KEV CVE in a Chromium component, refusing a feed with none.

    A renamed vendor or product would otherwise empty the check silently.
    """
    cves: set[str] = set()
    for entry in field(document, "vulnerabilities", list):
        if is_chromium_component(
            field(entry, "vendorProject", str), field(entry, "product", str)
        ):
            cve = field(entry, "cveID", str)
            if not CVE_ID.match(cve):
                raise FeedDocumentError(f"unexpected CVE {cve}")
            cves.add(cve)
    if not cves:
        raise MissingFieldError("vulnerabilities[].product")
    return tuple(sorted(cves))


@dataclass(frozen=True, slots=True)
class ChromiumAdvisory:
    cve: str
    fixes: tuple[ChromiumVersion, ...]
    gap: str

    @classmethod
    def of_record(cls, cve: str, record: object | None) -> Self:
        """Read the Chromium fixes from ``record``, ``None`` being no record."""
        if record is None:
            return cls(cve, (), NO_RECORD)
        return cls(
            cve,
            tuple(
                fix
                for affected in field(record, "affected", list)
                for fix in _chromium_fixes(affected)
            ),
            NO_FIX,
        )

    def finding(self, pin: Pin, browser: PinnedBrowser) -> Finding | None:
        version = browser.browser_version
        if not self.fixes:
            return UnassessableAdvisoryFinding(
                self.cve, pin, str(version), self.gap
            )
        fix = comparable_fix(self.fixes, version)
        if fix is None or fix <= version:
            return None
        return AdvisoryFinding(
            self.cve, pin, str(fix), browser_version=str(version)
        )


def chromium_advisories(
    playwright_core: Pin, chromium: Pin, session: FeedSession
) -> CheckOutcome:
    """Every exploited Chromium CVE affecting the pinned browser build."""
    try:
        browser = pinned_browser_build(playwright_core, chromium, session)
        cves = session.request(
            Feed.KEV,
            _CHECK,
            KEV_URL,
            lambda client: client.get_json(KEV_URL),
            kev_chromium_cves,
        )
    except FeedRequestError as error:
        return CheckOutcome.failed([error.failure])
    advisories: list[ChromiumAdvisory] = []
    failures: list[FeedFailure] = []
    for cve in cves:
        try:
            advisories.append(_fetch_advisory(cve, session))
        except FeedRequestError as error:
            failures.append(error.failure)
    if failures:
        return CheckOutcome.failed(failures)
    return CheckOutcome.matched(
        finding
        for advisory in advisories
        if (finding := advisory.finding(chromium, browser)) is not None
    )


def _fetch_advisory(cve: str, session: FeedSession) -> ChromiumAdvisory:
    return session.request(
        Feed.OSV,
        _CHECK,
        cve,
        lambda client: _record_or_none(client, record_url(cve)),
        lambda record: ChromiumAdvisory.of_record(cve, record),
    )


def _record_or_none(client: FeedClient, url: str) -> object | None:
    try:
        return client.get_json(url)
    except FeedRecordNotFoundError:
        return None


def _chromium_fixes(affected: object) -> list[ChromiumVersion]:
    if not isinstance(affected, dict):
        raise MissingFieldError("affected[]")
    specific = _optional(affected, "database_specific", dict, {})
    return [
        fix
        for unresolved in _optional(specific, "unresolved_ranges", list, [])
        for event in field(unresolved, "events", list)
        if isinstance(event, dict) and "fixed" in event
        if (fix := _as_chromium_version(field(event, "fixed", str)))
    ]


def _optional[T](
    document: dict[str, object], key: str, kind: type[T], absent: T
) -> T:
    return field(document, key, kind) if key in document else absent


def _as_chromium_version(text: str) -> ChromiumVersion | None:
    try:
        return ChromiumVersion.parse(text)
    except ChromiumVersionError:
        return None
