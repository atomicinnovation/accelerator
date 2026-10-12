import datetime as dt

import pytest

from tasks.shared.vendor.pin_guard.checks import CheckOutcome
from tasks.shared.vendor.pin_guard.feeds import (
    CheckName,
    Feed,
    FeedFailure,
    FeedRequestError,
)
from tasks.shared.vendor.pin_guard.findings import AdvisoryFinding
from tasks.shared.vendor.pin_guard.local_inputs import Pin, PinName

FINDING = AdvisoryFinding(
    "GHSA-7mvr-c777-76hp",
    Pin(PinName.PLAYWRIGHT_CORE, "1.55.0", dt.date(2026, 5, 18)),
    "1.55.1",
)
FAILURE = FeedFailure(
    Feed.OSV, CheckName.PLAYWRIGHT_CORE_ADVISORIES, "GHSA-x", "unreachable"
)


def test_a_check_cannot_report_findings_alongside_failures():
    with pytest.raises(ValueError, match="no findings"):
        CheckOutcome((FINDING,), (FAILURE,))


def test_a_matched_check_carries_its_findings():
    assert CheckOutcome.matched([FINDING]).findings == (FINDING,)


def test_a_failed_check_carries_its_failures():
    assert CheckOutcome.failed([FAILURE]).failures == (FAILURE,)


def _fetching(failing=()):
    attempted = []

    def fetch(request):
        attempted.append(request)
        if request in failing:
            raise FeedRequestError(
                FeedFailure(
                    Feed.OSV,
                    CheckName.PLAYWRIGHT_CORE_ADVISORIES,
                    request,
                    "unreachable",
                )
            )
        return f"record {request}"

    return fetch, attempted


def test_a_gathered_check_matches_findings_from_every_record():
    fetch, _ = _fetching()
    outcome = CheckOutcome.gathered(
        ["a", "b"],
        fetch,
        lambda records: (
            [FINDING] if records == ["record a", "record b"] else []
        ),
    )
    assert outcome.findings == (FINDING,)


def test_a_gathered_check_with_a_failed_request_fails_after_trying_all():
    fetch, attempted = _fetching(failing={"b"})

    def to_findings(_records):
        raise AssertionError("a failed check maps no records")

    outcome = CheckOutcome.gathered(["a", "b", "c"], fetch, to_findings)
    assert [failure.request for failure in outcome.failures] == ["b"]
    assert attempted == ["a", "b", "c"]
