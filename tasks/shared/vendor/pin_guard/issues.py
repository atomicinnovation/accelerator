"""Reconciling a guard report against the issues already filed.

Each finding opens at most one issue across that issue's lifecycle, so an
issue of any state suppresses its finding. A feed outage is suppressed only
while an open issue covers it, so a later outage is reported afresh. Only
issues the guard itself wrote count: a labelled issue anyone else wrote could
otherwise silence a finding.
"""

import hashlib
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from typing import Protocol, Self, TextIO

from tasks.shared.vendor.pin_guard.feeds import FeedOutage
from tasks.shared.vendor.pin_guard.findings import (
    RELEASING_GUARD_URL,
    IssueDraft,
    code_span,
    issue_body,
)
from tasks.shared.vendor.pin_guard.local_inputs import Owner
from tasks.shared.vendor.pin_guard.markers import IssueMarker, MarkerKind
from tasks.shared.vendor.pin_guard.report import GuardReport

LABEL = "runtime-pin-guard"
MAXIMUM_NEW_ISSUES = 10
_TRIPPED_DIGEST_LENGTH = 12


class IssueTrackerError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class ExistingIssue:
    number: int
    is_open: bool
    author: str
    body: str


class IssueTracker(Protocol):
    def issues(self) -> list[ExistingIssue]: ...

    def ensure_label(self) -> None: ...

    def open_issue(self, draft: IssueDraft, assignee: str) -> int: ...


@dataclass(frozen=True, slots=True)
class FailedDraft:
    draft: IssueDraft
    error: IssueTrackerError


@dataclass(frozen=True, slots=True)
class IssuePolicy:
    trusted_author: str
    maximum_new_issues: int = MAXIMUM_NEW_ISSUES


@dataclass(frozen=True, slots=True)
class ReconcileOutcome:
    opened: tuple[int, ...]
    held_back: int
    failed: tuple[FailedDraft, ...]

    @property
    def tripped(self) -> bool:
        return self.held_back > 0


@dataclass(frozen=True, slots=True)
class _FiledMarkers:
    any_state: frozenset[IssueMarker]
    still_open: frozenset[IssueMarker]

    @classmethod
    def trusted(
        cls, issues: Iterable[ExistingIssue], trusted_author: str
    ) -> Self:
        filed = [
            (marker, issue.is_open)
            for issue in issues
            if issue.author == trusted_author
            and (marker := IssueMarker.of_body(issue.body)) is not None
        ]
        return cls(
            frozenset(marker for marker, _ in filed),
            frozenset(marker for marker, is_open in filed if is_open),
        )


def reconcile(
    report: GuardReport,
    tracker: IssueTracker,
    owner: Owner,
    policy: IssuePolicy,
) -> ReconcileOutcome:
    """Open an issue for every finding and outage no trusted issue reports.

    Outage issues open first and never count towards the cap, so a tripped
    cap cannot hide a dark feed.
    """
    filed = _FiledMarkers.trusted(tracker.issues(), policy.trusted_author)
    outages = [
        outage.draft(owner)
        for outage in FeedOutage.of(report.feed_failures)
        if not any(outage.is_covered_by(m) for m in filed.still_open)
    ]
    findings = {finding.marker: finding for finding in report.findings}
    drafts = [
        findings[marker].draft(owner)
        for marker in sorted(findings)
        if marker not in filed.any_state
    ]
    if len(drafts) <= policy.maximum_new_issues:
        return _open_all([*outages, *drafts], tracker, owner, held_back=0)
    tripped = _guard_tripped_draft(
        [draft.marker for draft in drafts], policy.maximum_new_issues
    )
    stand_in: list[IssueDraft] = (
        [] if tripped.marker in filed.still_open else [tripped]
    )
    return _open_all(
        [*outages, *stand_in], tracker, owner, held_back=len(drafts)
    )


def _open_all(
    drafts: Sequence[IssueDraft],
    tracker: IssueTracker,
    owner: Owner,
    *,
    held_back: int,
) -> ReconcileOutcome:
    if drafts:
        tracker.ensure_label()
    opened: list[int] = []
    failed: list[FailedDraft] = []
    for draft in drafts:
        try:
            opened.append(tracker.open_issue(draft, owner.login))
        except IssueTrackerError as error:
            failed.append(FailedDraft(draft, error))
    return ReconcileOutcome(tuple(opened), held_back, tuple(failed))


def _guard_tripped_draft(
    held_back: Sequence[IssueMarker], maximum_new_issues: int
) -> IssueDraft:
    rendered = "\n".join(sorted(marker.render() for marker in held_back))
    digest = hashlib.sha256(rendered.encode()).hexdigest()
    marker = IssueMarker(
        MarkerKind.GUARD_TRIPPED, (digest[:_TRIPPED_DIGEST_LENGTH],)
    )
    listing = "\n".join(
        f"- {code_span(' '.join(held.parts))}" for held in sorted(held_back)
    )
    return IssueDraft(
        marker,
        f"Runtime pin guard tripped: {len(held_back)} new findings held back",
        issue_body(
            marker,
            f"This run found {len(held_back)} new findings, more than the "
            f"{maximum_new_issues} one run may open, so it opened none of "
            "them. Look for a matcher defect first; if they are genuine, "
            "dispatch the workflow with `maximum-new-issues` above the count, "
            f"then close this issue. See {RELEASING_GUARD_URL}.",
            listing,
        ),
    )


class DryRunIssueTracker:
    """Reads through to ``tracker`` but prints every write instead."""

    def __init__(self, tracker: IssueTracker, out: TextIO) -> None:
        self._tracker = tracker
        self._out = out

    def issues(self) -> list[ExistingIssue]:
        return self._tracker.issues()

    def ensure_label(self) -> None:
        print(f"would ensure the {LABEL} label exists", file=self._out)

    def open_issue(self, draft: IssueDraft, assignee: str) -> int:
        print(
            f"would open: {draft.title}\n"
            f"assignee: {assignee}\n"
            f"label: {LABEL}\n\n"
            f"{draft.body}\n",
            file=self._out,
        )
        return 0
