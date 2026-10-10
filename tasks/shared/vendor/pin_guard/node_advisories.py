"""Node core advisories from the Node security working group's ``vuln/core``."""

import re
from dataclasses import dataclass
from typing import Self

import semver

from tasks.shared.vendor.pin_guard.checks import CheckOutcome
from tasks.shared.vendor.pin_guard.feeds import (
    CheckName,
    Feed,
    FeedDocumentError,
    FeedRequestError,
    FeedSession,
    MissingFieldError,
    field,
)
from tasks.shared.vendor.pin_guard.findings import AdvisoryFinding
from tasks.shared.vendor.pin_guard.local_inputs import Pin
from tasks.shared.vendor.pin_guard.npm_ranges import (
    RangeSyntaxError,
    VersionRange,
)

VULN_CORE_URL = (
    "https://raw.githubusercontent.com/nodejs/security-wg/main/"
    "vuln/core/index.json"
)
SUPPORTED_ENVIRONMENTS = frozenset({"all", "darwin", "linux"})
_ENVIRONMENTS = "affectedEnvironments"
_CVE = re.compile(r"^CVE-\d{4}-\d{4,}$")


@dataclass(frozen=True, slots=True)
class VulnCoreEntry:
    key: str
    cves: tuple[str, ...]
    vulnerable: VersionRange
    patched: VersionRange
    patched_text: str
    environments: frozenset[str]

    @classmethod
    def parse(cls, key: str, entry: object) -> Self:
        if not key.isdigit():
            raise FeedDocumentError(f"unexpected entry key {key}")
        patched_text = field(entry, "patched", str).strip()
        return cls(
            key,
            _cves(entry),
            _range(field(entry, "vulnerable", str)),
            _range(patched_text),
            patched_text,
            _environments(entry),
        )

    @property
    def runs_where_the_runtime_ships(self) -> bool:
        return bool(self.environments & SUPPORTED_ENVIRONMENTS)

    def affects(self, version: semver.Version) -> bool:
        return (
            self.runs_where_the_runtime_ships
            and self.vulnerable.contains(version)
            and not self.patched.contains(version)
        )


def vuln_core_entries(document: object) -> tuple[VulnCoreEntry, ...]:
    """Parse every entry, refusing a feed none of whose entries apply.

    A feed whose environments were all renamed would otherwise match nothing
    and read as clear.
    """
    if not isinstance(document, dict):
        raise FeedDocumentError("unexpected document shape")
    entries = tuple(
        sorted(
            (
                VulnCoreEntry.parse(key, entry)
                for key, entry in document.items()
            ),
            key=lambda entry: int(entry.key),
        )
    )
    if not any(entry.runs_where_the_runtime_ships for entry in entries):
        raise MissingFieldError(_ENVIRONMENTS)
    return entries


def node_advisories(pin: Pin, session: FeedSession) -> CheckOutcome:
    """Every ``vuln/core`` entry affecting the pinned Node version."""
    try:
        entries = session.request(
            Feed.VULN_CORE,
            CheckName.NODE_ADVISORIES,
            VULN_CORE_URL,
            lambda client: client.get_json(VULN_CORE_URL),
            vuln_core_entries,
        )
    except FeedRequestError as error:
        return CheckOutcome.failed([error.failure])
    version = semver.Version.parse(pin.version)
    return CheckOutcome.matched(
        AdvisoryFinding(entry.key, pin, entry.patched_text, entry.cves)
        for entry in entries
        if entry.affects(version)
    )


def _range(text: str) -> VersionRange:
    try:
        return VersionRange.parse(text)
    except RangeSyntaxError as error:
        raise FeedDocumentError(f"unparseable range {text.strip()}") from error


def _cves(entry: object) -> tuple[str, ...]:
    if isinstance(entry, dict) and "cve" not in entry:
        return ()
    cves = field(entry, "cve", list)
    for cve in cves:
        if not isinstance(cve, str) or not _CVE.match(cve):
            raise FeedDocumentError(f"unexpected CVE {cve}")
    return tuple(cves)


def _environments(entry: object) -> frozenset[str]:
    environments = field(entry, _ENVIRONMENTS, list)
    if not all(isinstance(name, str) for name in environments):
        raise MissingFieldError(_ENVIRONMENTS)
    return frozenset(environments)
