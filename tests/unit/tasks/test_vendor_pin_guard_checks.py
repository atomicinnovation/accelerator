import datetime as dt

import pytest

from tasks.shared.vendor.pin_guard.checks import CheckOutcome
from tasks.shared.vendor.pin_guard.feeds import CheckName, Feed, FeedFailure
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
