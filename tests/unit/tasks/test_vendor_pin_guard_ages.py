import datetime as dt

import pytest

from tasks.shared.vendor.pin_guard.ages import (
    keyring_age_findings,
    pin_age_findings,
)
from tasks.shared.vendor.pin_guard.findings import (
    KeyringAgeFinding,
    PinAgeFinding,
)
from tasks.shared.vendor.pin_guard.local_inputs import (
    Owner,
    PinName,
    read_local_inputs,
)
from tasks.shared.vendor.pin_guard.markers import IssueMarker, MarkerKind
from tests.unit.tasks.shared.doubles import clear_repository

TODAY = dt.date(2026, 10, 10)
OWNER = Owner("Toby Clemson", "tobyclemson")
VERSIONS = {
    PinName.PLAYWRIGHT_CORE: "1.55.1",
    PinName.NODE: "22.22.2",
    PinName.CHROMIUM: "1193",
}


def _inputs(tmp_path, **bumped):
    return read_local_inputs(
        clear_repository(tmp_path, TODAY, bumped=bumped), TODAY
    )


def _days_ago(days):
    return TODAY - dt.timedelta(days=days)


@pytest.mark.parametrize("pin", list(PinName))
def test_a_pin_91_days_old_opens_a_finding_naming_it(tmp_path, pin):
    inputs = _inputs(tmp_path, **{pin.value: _days_ago(91)})
    [finding] = pin_age_findings(inputs, TODAY)
    assert isinstance(finding, PinAgeFinding)
    assert finding.pin.name == pin
    assert finding.age_days == 91
    assert finding.marker == IssueMarker(
        MarkerKind.FINDING, ("pin-age", pin.value, VERSIONS[pin])
    )
    body = finding.draft(OWNER).body
    assert pin.value in body
    assert f"`{VERSIONS[pin]}`" in body
    assert "91 days" in body
    assert "@tobyclemson" in body


@pytest.mark.parametrize("pin", list(PinName))
def test_a_pin_exactly_90_days_old_opens_nothing(tmp_path, pin):
    inputs = _inputs(tmp_path, **{pin.value: _days_ago(90)})
    assert pin_age_findings(inputs, TODAY) == []


def test_a_keyring_366_days_old_opens_a_finding_naming_it(tmp_path):
    inputs = _inputs(tmp_path, keyring=_days_ago(366))
    [finding] = keyring_age_findings(inputs, TODAY)
    assert finding == KeyringAgeFinding(_days_ago(366), 366, 365)
    assert finding.marker == IssueMarker(
        MarkerKind.FINDING, ("keyring-age", _days_ago(366).isoformat())
    )
    body = finding.draft(OWNER).body
    assert "keyring" in body
    assert "366 days" in body
    assert "@tobyclemson" in body


def test_a_keyring_exactly_365_days_old_opens_nothing(tmp_path):
    inputs = _inputs(tmp_path, keyring=_days_ago(365))
    assert keyring_age_findings(inputs, TODAY) == []
