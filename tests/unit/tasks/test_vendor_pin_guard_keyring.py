import datetime as dt

import pytest

from tasks.shared.vendor.gpg import ListedKey, listed_keys
from tasks.shared.vendor.pin_guard.keyring import (
    KEY_EXPIRY_WARNING_DAYS,
    CheckedKey,
    checked_keys,
    key_expiry_findings,
)
from tests.unit.tasks.shared.doubles import PIN_GUARD_FIXTURES

TODAY = dt.date(2026, 10, 9)
PRIMARY = "890C08DB8579162FEE0DF9DB8BEAB4DFCF555EF4"
SUBKEY = "86C8D74642E67846F8E120284DAA80D1E737BC9F"


def _at(days, hour=12, minute=0):
    day = TODAY + dt.timedelta(days=days)
    return dt.datetime(
        day.year, day.month, day.day, hour, minute, tzinfo=dt.UTC
    )


def _key(days, *, fingerprint=PRIMARY, hour=12, minute=0):
    return CheckedKey(fingerprint, _at(days, hour, minute))


def _listed(fingerprint, *, is_primary, capabilities, days):
    expires_at = None if days is None else _at(days)
    return ListedKey(fingerprint, is_primary, capabilities, expires_at)


class TestWarningWindow:
    def test_a_key_at_the_edge_of_the_window_opens_a_finding(self):
        [finding] = key_expiry_findings([_key(KEY_EXPIRY_WARNING_DAYS)], TODAY)
        assert finding.fingerprint == PRIMARY
        assert finding.expires_on == TODAY + dt.timedelta(days=60)
        assert finding.days_to_expiry == 60

    def test_a_key_one_day_outside_the_window_opens_nothing(self):
        assert key_expiry_findings([_key(61)], TODAY) == []

    def test_the_last_minute_of_day_61_is_still_outside(self):
        assert key_expiry_findings([_key(61, hour=23, minute=59)], TODAY) == []

    @pytest.mark.parametrize("days", [0, -1, -400])
    def test_a_key_expiring_today_or_already_expired_opens(self, days):
        [finding] = key_expiry_findings([_key(days)], TODAY)
        assert finding.days_to_expiry == days

    def test_a_key_with_no_expiry_opens_nothing(self):
        assert key_expiry_findings([CheckedKey(PRIMARY, None)], TODAY) == []


class TestCheckedKeys:
    def test_a_signing_subkey_is_checked_beside_a_distant_primary(self):
        keys = checked_keys(
            [
                _listed(
                    PRIMARY, is_primary=True, capabilities="scSC", days=730
                ),
                _listed(SUBKEY, is_primary=False, capabilities="s", days=30),
            ]
        )
        [finding] = key_expiry_findings(keys, TODAY)
        assert finding.fingerprint == SUBKEY

    def test_an_encryption_only_subkey_is_not_checked(self):
        keys = checked_keys(
            [
                _listed(
                    PRIMARY, is_primary=True, capabilities="scSC", days=730
                ),
                _listed(SUBKEY, is_primary=False, capabilities="e", days=30),
            ]
        )
        assert key_expiry_findings(keys, TODAY) == []

    def test_a_primary_is_checked_whatever_its_capabilities(self):
        keys = checked_keys(
            [_listed(PRIMARY, is_primary=True, capabilities="cC", days=30)]
        )
        assert [key.fingerprint for key in keys] == [PRIMARY]


class TestRecordedNodeKeyring:
    def _findings(self, today):
        listing = (PIN_GUARD_FIXTURES / "nodejs-release.colons").read_text()
        keys = checked_keys(listed_keys(listing.splitlines()))
        return {
            (finding.fingerprint, finding.expires_on)
            for finding in key_expiry_findings(keys, today)
        }

    def test_on_9_october_two_keys_have_expired(self):
        assert self._findings(dt.date(2026, 10, 9)) == {
            (PRIMARY, dt.date(2026, 7, 8)),
            ("A6023530FC53461FEC91F99C04CD3F2FDE079578", dt.date(2025, 6, 1)),
        }

    def test_on_10_october_the_signing_subkey_enters_the_window(self):
        assert self._findings(dt.date(2026, 10, 10)) == {
            (PRIMARY, dt.date(2026, 7, 8)),
            ("A6023530FC53461FEC91F99C04CD3F2FDE079578", dt.date(2025, 6, 1)),
            (SUBKEY, dt.date(2026, 12, 9)),
        }
