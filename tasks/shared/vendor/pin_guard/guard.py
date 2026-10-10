"""One guard run: evaluate every check, then reconcile the issues.

Every check is evaluated before the tracker is touched, so an abort raised
during evaluation still precedes any write.
"""

import datetime as dt
from dataclasses import dataclass

from tasks.shared.vendor.pin_guard.ages import (
    keyring_age_findings,
    pin_age_findings,
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
from tasks.shared.vendor.pin_guard.report import GuardReport


@dataclass(frozen=True, slots=True)
class GuardPorts:
    tracker: IssueTracker
    key_lister: KeyLister


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
        return tripped + failed


def evaluate(
    inputs: LocalInputs, today: dt.date, ports: GuardPorts
) -> GuardReport:
    return GuardReport(
        (
            *pin_age_findings(inputs, today),
            *keyring_age_findings(inputs, today),
            *node_keyring_expiry_findings(
                inputs.node_keyring, ports.key_lister, today
            ),
        )
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
