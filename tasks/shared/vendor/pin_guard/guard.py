"""One guard run: evaluate every check, then reconcile the issues.

Every check is evaluated before the tracker is touched, so an abort raised
during evaluation still precedes any write.
"""

import datetime as dt
from dataclasses import dataclass

from tasks.shared.clock import Clock
from tasks.shared.vendor.pin_guard.ages import (
    keyring_age_findings,
    pin_age_findings,
)
from tasks.shared.vendor.pin_guard.checks import CheckOutcome
from tasks.shared.vendor.pin_guard.chromium_advisories import (
    chromium_advisories,
)
from tasks.shared.vendor.pin_guard.feeds import (
    GUARD_FEED_DEADLINE_SECONDS,
    FeedBudget,
    FeedClient,
    FeedOutage,
    FeedSession,
)
from tasks.shared.vendor.pin_guard.issues import (
    IssuePolicy,
    IssueTracker,
    ReconcileOutcome,
    reconcile,
)
from tasks.shared.vendor.pin_guard.keyring import (
    KeyLister,
    node_keyring_expiry_findings,
)
from tasks.shared.vendor.pin_guard.local_inputs import (
    LocalInputPaths,
    LocalInputs,
    read_local_inputs,
)
from tasks.shared.vendor.pin_guard.node_advisories import node_advisories
from tasks.shared.vendor.pin_guard.osv import playwright_core_advisories
from tasks.shared.vendor.pin_guard.report import GuardReport


@dataclass(frozen=True, slots=True)
class GuardPorts:
    tracker: IssueTracker
    key_lister: KeyLister
    feeds: FeedClient
    clock: Clock


@dataclass(frozen=True, slots=True)
class GuardRun:
    report: GuardReport
    outcome: ReconcileOutcome

    def failure_reasons(self) -> tuple[str, ...]:
        """Every reason the run must fail, so one exit names them all."""
        tripped = (
            (
                f"{self.outcome.held_back} new findings exceeded the per-run "
                "issue cap, so a guard-tripped issue stands in for them",
            )
            if self.outcome.tripped
            else ()
        )
        failed = tuple(
            f"could not open {failed.draft.title!r}: {failed.error}"
            for failed in self.outcome.failed
        )
        dark = tuple(
            f"the {outage.feed} feed failed {len(outage.failures)} "
            f"request(s), skipping {', '.join(outage.skipped_checks)}"
            for outage in FeedOutage.of(self.report.feed_failures)
        )
        return dark + tripped + failed


def evaluate(
    inputs: LocalInputs, today: dt.date, ports: GuardPorts
) -> GuardReport:
    session = FeedSession(
        ports.feeds,
        FeedBudget(
            ports.clock.now() + GUARD_FEED_DEADLINE_SECONDS, ports.clock
        ),
    )
    advisories: tuple[CheckOutcome, ...] = (
        playwright_core_advisories(inputs.playwright_core, session),
        node_advisories(inputs.node, session),
        chromium_advisories(inputs.playwright_core, inputs.chromium, session),
    )
    return GuardReport(
        (
            *pin_age_findings(inputs, today),
            *keyring_age_findings(inputs, today),
            *node_keyring_expiry_findings(
                inputs.node_keyring, ports.key_lister, today
            ),
            *(finding for check in advisories for finding in check.findings),
        ),
        tuple(failure for check in advisories for failure in check.failures),
    )


def run_guard(
    paths: LocalInputPaths,
    today: dt.date,
    ports: GuardPorts,
    policy: IssuePolicy,
) -> GuardRun:
    inputs = read_local_inputs(paths, today)
    report = evaluate(inputs, today, ports)
    return GuardRun(
        report, reconcile(report, ports.tracker, inputs.owner, policy)
    )
