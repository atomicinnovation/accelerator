"""The advisory feeds the guard reads, and how a failed request is reported.

A failed request is never "no findings": it becomes a ``FeedFailure`` that
skips the check which issued it. Only the feed-domain errors are translated,
so a local abort or a programming error is never mistaken for a broken feed.
"""

from collections import Counter, defaultdict
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from enum import StrEnum
from typing import Protocol, Self

from tasks.shared.clock import Clock
from tasks.shared.vendor.pin_guard.findings import (
    RELEASING_FEEDS_URL,
    IssueDraft,
    code_span,
    issue_body,
)
from tasks.shared.vendor.pin_guard.local_inputs import Owner
from tasks.shared.vendor.pin_guard.markers import IssueMarker, MarkerKind

GUARD_FEED_DEADLINE_SECONDS = 600
REQUEST_TIMEOUT_SECONDS = 20
CONSECUTIVE_FAILURES_BEFORE_SKIP = 3
UNREACHABLE = "unreachable"


class Feed(StrEnum):
    OSV = "osv"
    VULN_CORE = "vuln-core"
    KEV = "kev"
    NPM_REGISTRY = "npm-registry"


class CheckName(StrEnum):
    PLAYWRIGHT_CORE_ADVISORIES = "playwright-core-advisories"
    NODE_ADVISORIES = "node-advisories"
    CHROMIUM_ADVISORIES = "chromium-advisories"


@dataclass(frozen=True, slots=True)
class FeedFailure:
    feed: Feed
    check: CheckName
    request: str
    reason: str
    detail: str = ""


@dataclass(frozen=True, slots=True)
class FeedOutage:
    """Every failure one feed had in a run, and the checks it left dark.

    Its identity is the feed and the checks it skipped, so an outage that
    blinds a further check is not hidden behind a narrower standing issue.
    """

    feed: Feed
    failures: tuple[FeedFailure, ...]

    @classmethod
    def of(cls, failures: Iterable[FeedFailure]) -> list[Self]:
        by_feed: defaultdict[Feed, list[FeedFailure]] = defaultdict(list)
        for failure in failures:
            by_feed[failure.feed].append(failure)
        return [cls(feed, tuple(by_feed[feed])) for feed in sorted(by_feed)]

    @property
    def skipped_checks(self) -> tuple[CheckName, ...]:
        return tuple(sorted({failure.check for failure in self.failures}))

    @property
    def marker(self) -> IssueMarker:
        return IssueMarker(
            MarkerKind.FEED_FAILURE, (self.feed, *self.skipped_checks)
        )

    def is_covered_by(self, marker: IssueMarker) -> bool:
        return (
            marker.kind is MarkerKind.FEED_FAILURE
            and marker.parts[0] == self.feed
            and set(self.skipped_checks) <= set(marker.parts[1:])
        )

    def draft(self, owner: Owner) -> IssueDraft:
        checks = ", ".join(code_span(check) for check in self.skipped_checks)
        requests = "\n".join(
            f"- {code_span(failure.request)}: {code_span(failure.reason)}"
            + (f" ({code_span(failure.detail)})" if failure.detail else "")
            for failure in self.failures
        )
        return IssueDraft(
            self.marker,
            f"Runtime pin guard feed {self.feed} failed",
            issue_body(
                self.marker,
                f"The {code_span(self.feed)} feed failed in this run, so the "
                f"guard skipped these checks: {checks}. Failed requests:",
                requests,
                f"Owner: @{owner.login}. Check the failed requests from "
                "outside GitHub Actions and close this issue once the feed "
                f"is healthy; a later failure opens a fresh one. See "
                f"{RELEASING_FEEDS_URL}.",
            ),
        )


class FeedRequestError(Exception):
    """One feed request failed, skipping the check that issued it."""

    def __init__(self, failure: FeedFailure) -> None:
        super().__init__(f"{failure.feed} {failure.request}: {failure.reason}")
        self.failure = failure


class FeedUnreachableError(Exception):
    """The feed gave no usable response once its retries were spent."""


class FeedDocumentError(ValueError):
    """The feed responded, but not with the document the check expects."""

    def __init__(self, reason: str) -> None:
        super().__init__(reason)
        self.reason = reason


class MissingFieldError(FeedDocumentError):
    def __init__(self, name: str) -> None:
        super().__init__(f"missing field {name}")


class FeedClient(Protocol):
    def get_json(self, url: str) -> object: ...

    def post_json(self, url: str, body: object) -> object: ...

    def get_bytes(self, url: str, max_bytes: int) -> bytes: ...


def field[T](document: object, key: str, kind: type[T]) -> T:
    """Return ``document[key]`` if it is a ``kind``, else name it missing."""
    if not isinstance(document, dict) or key not in document:
        raise MissingFieldError(key)
    value = document[key]
    if not isinstance(value, kind):
        raise MissingFieldError(key)
    return value


class FeedBudget:
    """When a run stops asking a feed: past the deadline, or once it is down."""

    def __init__(self, deadline: float, clock: Clock) -> None:
        self._deadline = deadline
        self._clock = clock
        self._unreachable_streaks: Counter[Feed] = Counter()

    def skip_reason(self, feed: Feed) -> str | None:
        if self._clock.now() >= self._deadline:
            return "skipped: deadline"
        if self._unreachable_streaks[feed] >= CONSECUTIVE_FAILURES_BEFORE_SKIP:
            return f"skipped: {feed} unreachable"
        return None

    def record(self, feed: Feed, *, reachable: bool) -> None:
        if reachable:
            self._unreachable_streaks[feed] = 0
        else:
            self._unreachable_streaks[feed] += 1


class FeedSession:
    """Every feed request of one run, attributed to the check that made it."""

    def __init__(self, client: FeedClient, budget: FeedBudget) -> None:
        self._client = client
        self._budget = budget

    def request[T](
        self,
        feed: Feed,
        check: CheckName,
        label: str,
        call: Callable[[FeedClient], object],
        parse: Callable[[object], T],
    ) -> T:
        """Make one request, raising ``FeedRequestError`` if it fails."""
        skipped = self._budget.skip_reason(feed)
        if skipped is not None:
            raise FeedRequestError(FeedFailure(feed, check, label, skipped))
        try:
            document = call(self._client)
        except FeedUnreachableError as error:
            self._budget.record(feed, reachable=False)
            raise FeedRequestError(
                FeedFailure(feed, check, label, UNREACHABLE, str(error))
            ) from error
        except FeedDocumentError as error:
            self._budget.record(feed, reachable=True)
            raise FeedRequestError(
                FeedFailure(feed, check, label, error.reason)
            ) from error
        self._budget.record(feed, reachable=True)
        try:
            return parse(document)
        except FeedDocumentError as error:
            raise FeedRequestError(
                FeedFailure(feed, check, label, error.reason)
            ) from error
