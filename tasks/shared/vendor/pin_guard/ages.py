"""Pin and keyring age, measured from the bump dates in ``pins.toml``."""

import datetime as dt

from tasks.shared.vendor.pin_guard.findings import (
    KeyringAgeFinding,
    PinAgeFinding,
)
from tasks.shared.vendor.pin_guard.local_inputs import LocalInputs

PIN_MAXIMUM_AGE_DAYS = 90
KEYRING_MAXIMUM_AGE_DAYS = 365


def pin_age_findings(
    inputs: LocalInputs, today: dt.date
) -> list[PinAgeFinding]:
    return [
        PinAgeFinding(pin, age, PIN_MAXIMUM_AGE_DAYS)
        for pin in inputs.pins
        if (age := (today - pin.bumped).days) > PIN_MAXIMUM_AGE_DAYS
    ]


def keyring_age_findings(
    inputs: LocalInputs, today: dt.date
) -> list[KeyringAgeFinding]:
    age = (today - inputs.keyring_bumped).days
    if age <= KEYRING_MAXIMUM_AGE_DAYS:
        return []
    return [
        KeyringAgeFinding(inputs.keyring_bumped, age, KEYRING_MAXIMUM_AGE_DAYS)
    ]
