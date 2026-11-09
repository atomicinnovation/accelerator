"""Every advisory suppression in cli/deny.toml carries a live review-by date.

cargo-deny enforces no expiry of its own: the ignore list is flat, and an entry
added under release pressure outlives its justification silently. That matters
more now the gix/jj-lib closure is in advisories scope under
unmaintained = "all" with yanked = "deny" — one upstream advisory in ~60 crates
no repo code calls turns check-supply-chain red for every unrelated PR, and that
job is in prerelease.needs, so a documented five-minute suppression is exactly
what gets applied reflexively.

A lapsed date is a prompt to re-check upstream and re-date or remove, not to
bump the date blindly. The break-glass procedure in tasks/README.md is scoped to
the unmaintained, yanked and notice classes only.
"""

import datetime as dt
import re
import tomllib
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[3]
_DENY = _REPO_ROOT / "cli/deny.toml"

_REVIEW_BY = re.compile(r"review-by:\s*(\d{4}-\d{2}-\d{2})")


def _ignores() -> list[dict | str]:
    return tomllib.loads(_DENY.read_text())["advisories"]["ignore"]


def test_every_ignore_entry_is_a_table_with_a_reason() -> None:
    # A bare "RUSTSEC-…" string carries nowhere to put the date.
    bare = [entry for entry in _ignores() if not isinstance(entry, dict)]
    assert not bare, (
        f"advisory ignores must be tables with a reason, found bare ids: {bare}"
    )
    missing = [
        entry["id"]
        for entry in _ignores()
        if not entry.get("reason", "").strip()
    ]
    assert not missing, f"advisory ignores with no reason: {missing}"


def _undated(entries: list[dict]) -> list[str]:
    return [
        entry["id"]
        for entry in entries
        if not _REVIEW_BY.search(entry.get("reason", ""))
    ]


def _lapsed(entries: list[dict], today: dt.date) -> list[tuple[str, str]]:
    lapsed = []
    for entry in entries:
        match = _REVIEW_BY.search(entry.get("reason", ""))
        if match is None:
            continue
        if dt.date.fromisoformat(match.group(1)) < today:
            lapsed.append((entry["id"], match.group(1)))
    return lapsed


def test_every_ignore_entry_carries_a_review_by_date() -> None:
    undated = _undated(_ignores())
    assert not undated, (
        "advisory ignores with no machine-readable `review-by: YYYY-MM-DD` in "
        f"their reason: {undated}"
    )


def test_no_ignore_entry_is_past_its_review_by_date() -> None:
    lapsed = _lapsed(_ignores(), dt.datetime.now(tz=dt.UTC).date())
    assert not lapsed, (
        "advisory suppressions past their review-by date — re-check whether "
        "upstream now offers a fix, then remove the entry or re-date it with "
        f"the current reason: {lapsed}"
    )


def test_the_review_by_assertion_is_not_vacuous() -> None:
    # An empty ignore list is the goal, and passes the checks above while
    # proving nothing, so the checks are proved against entries of their own.
    lapsed = {"id": "RUSTSEC-0000-0001", "reason": "x. review-by: 2026-01-01"}
    undated = {"id": "RUSTSEC-0000-0002", "reason": "no date"}

    assert _lapsed([lapsed, undated], dt.date(2026, 1, 2)) == [
        ("RUSTSEC-0000-0001", "2026-01-01")
    ]
    assert _lapsed([lapsed], dt.date(2026, 1, 1)) == []
    assert _undated([lapsed, undated]) == ["RUSTSEC-0000-0002"]


def test_the_ignore_list_is_where_the_checks_read_it() -> None:
    assert isinstance(_ignores(), list), "has [advisories].ignore moved?"
