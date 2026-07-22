"""Tests for the 0167 inventory reconciliation in ``tasks/lint/inventory.py``.

The jj-backed reads are exercised through injected ``fetch``/``read_source``
fakes, so the pure reconciliation logic is tested without jj history.
"""

from tasks.lint import inventory


def _present_fetch(_rev: str, path: str) -> str | None:
    # Every removal-set path present, each covering suite naming every basename.
    if path in inventory.COVERING_SUITES:
        return " ".join(p for p in inventory.REMOVAL_SET)
    return "content" if path in inventory.REMOVAL_SET else None


def test_removal_set_floor_all_present() -> None:
    assert inventory.removal_set_floor_violations(_present_fetch) == []


def test_removal_set_floor_flags_a_missing_path() -> None:
    def fetch(rev: str, path: str) -> str | None:
        if path == inventory.REMOVAL_SET[0]:
            return None
        return _present_fetch(rev, path)

    flagged = inventory.removal_set_floor_violations(fetch)
    assert any("absent" in v for v in flagged)
    assert any("floor" in v for v in flagged)


def test_divergence_resolves_and_flags() -> None:
    text = "read.rs::my_test parity.rs::other_test"
    sources = {
        "cli/launcher/tests/config_read.rs": "fn my_test() {}",
        "cli/config-adapters/tests/parity.rs": "fn other_test() {}",
    }
    assert inventory.divergence_violations(text, sources.__getitem__) == []

    missing = {
        "cli/launcher/tests/config_read.rs": "fn my_test() {}",
        "cli/config-adapters/tests/parity.rs": "nothing here",
    }
    flagged = inventory.divergence_violations(text, missing.__getitem__)
    assert any("does not resolve" in v for v in flagged)


def test_divergence_empty_is_flagged() -> None:
    assert inventory.divergence_violations("no refs", lambda _p: "") == [
        "divergences record names no tests"
    ]


def test_deletion_ledger_omissions() -> None:
    full = "\n".join(inventory.REMOVAL_SET)
    assert inventory.deletion_ledger_omissions(full) == []
    partial = "\n".join(inventory.REMOVAL_SET[1:])
    assert inventory.deletion_ledger_omissions(partial) == [
        f"deletion ledger omits: {inventory.REMOVAL_SET[0]}"
    ]


def test_member_four_empty_and_uncovered() -> None:
    assert inventory.member_four_violations(_present_fetch) == []

    def bare_fetch(_rev: str, path: str) -> str | None:
        return "" if path in inventory.COVERING_SUITES else "content"

    flagged = inventory.member_four_violations(bare_fetch)
    assert any("member 4 is not empty" in v for v in flagged)
