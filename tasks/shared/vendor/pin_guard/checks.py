"""What one advisory check reports: its findings, or its failed requests."""

from collections.abc import Iterable
from dataclasses import dataclass
from typing import Self

from tasks.shared.vendor.pin_guard.feeds import FeedFailure
from tasks.shared.vendor.pin_guard.findings import Finding


@dataclass(frozen=True, slots=True)
class CheckOutcome:
    findings: tuple[Finding, ...] = ()
    failures: tuple[FeedFailure, ...] = ()

    def __post_init__(self) -> None:
        if self.findings and self.failures:
            raise ValueError(
                "a check whose feed request failed reports no findings"
            )

    @classmethod
    def matched(cls, findings: Iterable[Finding]) -> Self:
        return cls(findings=tuple(findings))

    @classmethod
    def failed(cls, failures: Iterable[FeedFailure]) -> Self:
        return cls(failures=tuple(failures))
