"""OSV advisories against the npm ``playwright`` and ``playwright-core`` pin."""

import re
from collections.abc import Iterable
from dataclasses import dataclass
from enum import StrEnum
from typing import Self

import semver

from tasks.shared.vendor.pin_guard.checks import CheckOutcome
from tasks.shared.vendor.pin_guard.feeds import (
    CheckName,
    Feed,
    FeedDocumentError,
    FeedFailure,
    FeedRequestError,
    FeedSession,
    MissingFieldError,
    field,
)
from tasks.shared.vendor.pin_guard.findings import AdvisoryFinding
from tasks.shared.vendor.pin_guard.local_inputs import Pin

OSV_API = "https://api.osv.dev/v1"
QUERYBATCH_URL = f"{OSV_API}/querybatch"
NPM_PACKAGES = ("playwright-core", "playwright")
GHSA_ID = re.compile(r"^GHSA(-[23456789cfghjmpqrvwx]{4}){3}$")
_CHECK = CheckName.PLAYWRIGHT_CORE_ADVISORIES
_RANGE_TYPES = frozenset({"SEMVER", "ECOSYSTEM"})


class EventKind(StrEnum):
    INTRODUCED = "introduced"
    FIXED = "fixed"
    LAST_AFFECTED = "last_affected"


@dataclass(frozen=True, slots=True)
class RangeEvent:
    kind: EventKind
    version: semver.Version

    def is_passed_by(self, version: semver.Version) -> bool:
        if self.kind is EventKind.LAST_AFFECTED:
            return version > self.version
        return version >= self.version


type AffectedRange = tuple[RangeEvent, ...]


def record_url(advisory_id: str) -> str:
    return f"{OSV_API}/vulns/{advisory_id}"


def batch_query(version: str) -> dict[str, object]:
    return {
        "queries": [
            {"package": {"name": name, "ecosystem": "npm"}, "version": version}
            for name in NPM_PACKAGES
        ]
    }


def advisory_ids(response: object) -> frozenset[str]:
    """Return the GHSA IDs a batch response matched, refusing truncation."""
    ids: set[str] = set()
    for result in field(response, "results", list):
        if not isinstance(result, dict):
            raise MissingFieldError("results[]")
        if "next_page_token" in result:
            raise FeedDocumentError("truncated at next_page_token")
        if "vulns" not in result:
            continue
        for vuln in field(result, "vulns", list):
            advisory_id = field(vuln, "id", str)
            if not GHSA_ID.match(advisory_id):
                raise FeedDocumentError(f"unexpected advisory id {advisory_id}")
            ids.add(advisory_id)
    return frozenset(ids)


@dataclass(frozen=True, slots=True)
class NpmAdvisory:
    advisory_id: str
    ranges: tuple[AffectedRange, ...]

    @classmethod
    def parse(cls, record: object) -> Self:
        """Keep the record's ranges for the watched npm packages.

        OSV returned the record for one of those packages, so a record that
        says nothing about how it affects them is malformed.
        """
        advisory_id = field(record, "id", str)
        ranges = tuple(
            _events(affected_range)
            for affected in field(record, "affected", list)
            if _is_watched_package(affected)
            for affected_range in _ranges(affected)
        )
        if not ranges:
            raise MissingFieldError("affected[].ranges")
        return cls(advisory_id, ranges)

    def affects(self, version: semver.Version) -> bool:
        return any(_range_affects(events, version) for events in self.ranges)

    def lowest_fix_above(
        self, version: semver.Version
    ) -> semver.Version | None:
        return min(
            (
                event.version
                for events in self.ranges
                if _range_affects(events, version)
                for event in events
                if event.kind is EventKind.FIXED and event.version > version
            ),
            default=None,
        )


def playwright_core_advisories(pin: Pin, session: FeedSession) -> CheckOutcome:
    """Every OSV advisory affecting the pinned version, or why it is unknown."""
    try:
        ids = session.request(
            Feed.OSV,
            _CHECK,
            QUERYBATCH_URL,
            lambda client: client.post_json(
                QUERYBATCH_URL, batch_query(pin.version)
            ),
            advisory_ids,
        )
    except FeedRequestError as error:
        return CheckOutcome.failed([error.failure])
    advisories: list[NpmAdvisory] = []
    failures: list[FeedFailure] = []
    for advisory_id in sorted(ids):
        try:
            advisories.append(_fetch_advisory(advisory_id, session))
        except FeedRequestError as error:
            failures.append(error.failure)
    if failures:
        return CheckOutcome.failed(failures)
    return CheckOutcome.matched(_findings(pin, advisories))


def _fetch_advisory(advisory_id: str, session: FeedSession) -> NpmAdvisory:
    return session.request(
        Feed.OSV,
        _CHECK,
        advisory_id,
        lambda client: client.get_json(record_url(advisory_id)),
        NpmAdvisory.parse,
    )


def _findings(
    pin: Pin, advisories: Iterable[NpmAdvisory]
) -> list[AdvisoryFinding]:
    version = semver.Version.parse(pin.version)
    findings: list[AdvisoryFinding] = []
    for advisory in advisories:
        if advisory.affects(version):
            fix = advisory.lowest_fix_above(version)
            findings.append(
                AdvisoryFinding(
                    advisory.advisory_id,
                    pin,
                    None if fix is None else str(fix),
                )
            )
    return findings


def _is_watched_package(affected: object) -> bool:
    if not isinstance(affected, dict):
        return False
    package = affected.get("package")
    return (
        isinstance(package, dict)
        and package.get("ecosystem") == "npm"
        and package.get("name") in NPM_PACKAGES
    )


def _ranges(affected: dict[str, object]) -> list[object]:
    ranges = affected.get("ranges", [])
    if not isinstance(ranges, list):
        raise MissingFieldError("affected[].ranges")
    return [
        affected_range
        for affected_range in ranges
        if isinstance(affected_range, dict)
        and affected_range.get("type") in _RANGE_TYPES
    ]


def _events(affected_range: object) -> AffectedRange:
    return tuple(
        RangeEvent(kind, _version(event[kind]))
        for event in field(affected_range, "events", list)
        if isinstance(event, dict)
        for kind in EventKind
        if kind in event
    )


def _version(text: object) -> semver.Version:
    if text == "0":
        return semver.Version(0, 0, 0)
    try:
        return semver.Version.parse(str(text))
    except ValueError as error:
        raise FeedDocumentError(f"unparseable version {text}") from error


def _range_affects(events: AffectedRange, version: semver.Version) -> bool:
    affected = False
    for event in sorted(events, key=lambda event: event.version):
        if event.is_passed_by(version):
            affected = event.kind is EventKind.INTRODUCED
    return affected
