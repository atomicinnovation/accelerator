"""The conditions the guard reports, each drafting the issue it opens."""

import datetime as dt
from dataclasses import dataclass
from typing import Protocol

from tasks.shared.vendor.pin_guard.local_inputs import Owner, Pin
from tasks.shared.vendor.pin_guard.markers import IssueMarker, MarkerKind

# Absolute, since GitHub resolves a relative link in an issue body against the
# issue's own URL.
RELEASING_GUARD_URL = (
    "https://github.com/atomicinnovation/accelerator/blob/main/"
    "RELEASING.md#vendored-runtime-pin-guard"
)
RELEASING_FEEDS_URL = (
    "https://github.com/atomicinnovation/accelerator/blob/main/"
    "RELEASING.md#feeds"
)


@dataclass(frozen=True, slots=True)
class IssueDraft:
    marker: IssueMarker
    title: str
    body: str


class Finding(Protocol):
    @property
    def marker(self) -> IssueMarker: ...

    def draft(self, owner: Owner) -> IssueDraft: ...


def code_span(text: str) -> str:
    """Render ``text`` as one inert code span, whatever it contains."""
    inert = text.translate(str.maketrans("", "", "`<>"))
    return f"`{inert}`"


def issue_body(marker: IssueMarker, *paragraphs: str) -> str:
    """Join ``paragraphs`` and close the body with ``marker``'s last line."""
    return "\n\n".join((*paragraphs, marker.render()))


@dataclass(frozen=True, slots=True)
class PinAgeFinding:
    pin: Pin
    age_days: int
    maximum_age_days: int

    @property
    def marker(self) -> IssueMarker:
        return IssueMarker(
            MarkerKind.FINDING, ("pin-age", self.pin.name, self.pin.version)
        )

    def draft(self, owner: Owner) -> IssueDraft:
        return IssueDraft(
            self.marker,
            f"Vendored {self.pin.name} pin is {self.age_days} days old",
            issue_body(
                self.marker,
                f"The vendored `{self.pin.name}` pin, version "
                f"{code_span(self.pin.version)}, was bumped on "
                f"{self.pin.bumped.isoformat()} and is {self.age_days} days "
                f"old, past its maximum age of {self.maximum_age_days} days.",
                f"Owner: @{owner.login}. Bump the pin under the refresh "
                f"procedure; see {RELEASING_GUARD_URL}.",
            ),
        )


@dataclass(frozen=True, slots=True)
class KeyringAgeFinding:
    bumped: dt.date
    age_days: int
    maximum_age_days: int

    @property
    def marker(self) -> IssueMarker:
        return IssueMarker(
            MarkerKind.FINDING, ("keyring-age", self.bumped.isoformat())
        )

    def draft(self, owner: Owner) -> IssueDraft:
        return IssueDraft(
            self.marker,
            f"Vendored-runtime keyring is {self.age_days} days old",
            issue_body(
                self.marker,
                "The vendored-runtime trust-anchor keyring "
                "(`keys/nodejs-release.asc`, `keys/npm-registry.pem`) was "
                f"last refreshed on {self.bumped.isoformat()} and is "
                f"{self.age_days} days old, past its maximum age of "
                f"{self.maximum_age_days} days.",
                f"Owner: @{owner.login}. Re-verify or replace the keys out of "
                f"band and update `keyring.bumped`; see {RELEASING_GUARD_URL}.",
            ),
        )


@dataclass(frozen=True, slots=True)
class KeyExpiryFinding:
    fingerprint: str
    expires_on: dt.date
    days_to_expiry: int

    @property
    def marker(self) -> IssueMarker:
        return IssueMarker(
            MarkerKind.FINDING,
            ("key-expiry", self.fingerprint, self.expires_on.isoformat()),
        )

    def draft(self, owner: Owner) -> IssueDraft:
        return IssueDraft(
            self.marker,
            f"Node release key {self.fingerprint[-16:]} {self._lapse}",
            issue_body(
                self.marker,
                f"The Node release key {code_span(self.fingerprint)} in "
                f"`keys/nodejs-release.asc` {self._lapse}, "
                f"{self._distance}.",
                f"Owner: @{owner.login}. Fetch the extended key from "
                "`nodejs/release-keys`, verify it out of band, replace "
                "`keys/nodejs-release.asc` and update `keyring.bumped`; see "
                f"{RELEASING_GUARD_URL}.",
            ),
        )

    @property
    def _lapse(self) -> str:
        verb = "expired" if self.days_to_expiry < 0 else "expires"
        return f"{verb} on {self.expires_on.isoformat()}"

    @property
    def _distance(self) -> str:
        if self.days_to_expiry < 0:
            return f"{-self.days_to_expiry} days ago"
        if self.days_to_expiry == 0:
            return "today"
        return f"in {self.days_to_expiry} days"


@dataclass(frozen=True, slots=True)
class AdvisoryFinding:
    advisory_id: str
    pin: Pin
    fix: str | None
    cves: tuple[str, ...] = ()
    browser_version: str | None = None

    @property
    def marker(self) -> IssueMarker:
        return IssueMarker(
            MarkerKind.FINDING, ("advisory", self.advisory_id, self.pin.name)
        )

    def draft(self, owner: Owner) -> IssueDraft:
        return IssueDraft(
            self.marker,
            f"{self.advisory_id} affects the vendored {self.pin.name} pin",
            issue_body(
                self.marker,
                f"Advisory {code_span(self.advisory_id)} affects the vendored "
                f"`{self.pin.name}` pin, version {code_span(self.pin.version)}"
                f"{self._browser}.",
                *self._cves,
                self._remedy,
                f"Owner: @{owner.login}. Bump the pin past the fix and close "
                "this issue once the bump merges; closing it earlier accepts "
                "the risk for every later version of the pin. See "
                f"{RELEASING_FEEDS_URL}.",
            ),
        )

    @property
    def _browser(self) -> str:
        if self.browser_version is None:
            return ""
        return f" (browser version {code_span(self.browser_version)})"

    @property
    def _cves(self) -> tuple[str, ...]:
        if not self.cves:
            return ()
        return ("CVEs: " + ", ".join(code_span(cve) for cve in self.cves),)

    @property
    def _remedy(self) -> str:
        if self.fix is None:
            return "No fixed version is available."
        return f"Fixed in {code_span(self.fix)}."
