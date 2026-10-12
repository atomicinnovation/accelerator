"""What one guard run found, before any issue is reconciled against it."""

from dataclasses import dataclass

from tasks.shared.vendor.pin_guard.feeds import FeedFailure
from tasks.shared.vendor.pin_guard.findings import Finding


@dataclass(frozen=True, slots=True)
class GuardReport:
    findings: tuple[Finding, ...]
    feed_failures: tuple[FeedFailure, ...] = ()
