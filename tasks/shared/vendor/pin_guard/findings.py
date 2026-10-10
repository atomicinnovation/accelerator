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
