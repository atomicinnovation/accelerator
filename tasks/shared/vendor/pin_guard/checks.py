"""What one advisory check reports: its findings, or its failed requests."""

from collections.abc import Callable, Iterable
from dataclasses import dataclass
from typing import Self

from tasks.shared.vendor.pin_guard.feeds import FeedFailure, FeedRequestError
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

    @classmethod
    def gathered[R](
        cls,
        requests: Iterable[str],
        fetch: Callable[[str], R],
        to_findings: Callable[[list[R]], Iterable[Finding]],
    ) -> Self:
        records: list[R] = []
        failures: list[FeedFailure] = []
        for request in requests:
            try:
                records.append(fetch(request))
            except FeedRequestError as error:
                failures.append(error.failure)
        if failures:
            return cls.failed(failures)
        return cls.matched(to_findings(records))
