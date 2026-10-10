"""Expiry of the Node release keys the vendored runtime is verified against."""

import datetime as dt
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from pathlib import Path

from tasks.shared.vendor.gpg import KeyListingError, ListedKey
from tasks.shared.vendor.pin_guard.findings import KeyExpiryFinding
from tasks.shared.vendor.pin_guard.local_inputs import (
    NODE_KEYRING_NAME,
    LocalInputError,
)

KEY_EXPIRY_WARNING_DAYS = 60

type KeyLister = Callable[[Path], tuple[ListedKey, ...]]


class KeyringError(LocalInputError):
    """The Node keyring could not be listed."""


@dataclass(frozen=True, slots=True)
class CheckedKey:
    fingerprint: str
    expires_at: dt.datetime | None

    @property
    def expires_on(self) -> dt.date | None:
        if self.expires_at is None:
            return None
        return self.expires_at.astimezone(dt.UTC).date()

    def days_to_expiry(self, today: dt.date) -> int | None:
        if self.expires_on is None:
            return None
        return (self.expires_on - today).days


def checked_keys(keys: Iterable[ListedKey]) -> tuple[CheckedKey, ...]:
    """Every primary key, and every subkey able to sign a release."""
    return tuple(
        CheckedKey(key.fingerprint, key.expires_at)
        for key in keys
        if key.is_primary or "s" in key.capabilities
    )


def key_expiry_findings(
    keys: Iterable[CheckedKey], today: dt.date
) -> list[KeyExpiryFinding]:
    return [
        KeyExpiryFinding(key.fingerprint, key.expires_on, days)
        for key in keys
        if key.expires_on is not None
        and (days := key.days_to_expiry(today)) is not None
        and days <= KEY_EXPIRY_WARNING_DAYS
    ]


def node_keyring_expiry_findings(
    keyring: Path, list_keys: KeyLister, today: dt.date
) -> list[KeyExpiryFinding]:
    try:
        listed = list_keys(keyring)
    except KeyListingError as error:
        raise KeyringError(f"{NODE_KEYRING_NAME}: {error}") from error
    return key_expiry_findings(checked_keys(listed), today)
