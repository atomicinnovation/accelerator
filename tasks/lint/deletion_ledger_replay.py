"""Replay the 0167 deletion ledger (ADR-0048 Python guardrail).

For every row of ``meta/inventories/0167-deletion-ledger.md``, assert the named
covering-gate test prefix resolves to at least one real ``fn`` in the
final-state gate file — the gate that SURVIVES the removal-set deletion. This is
stronger than a "a row exists" check: it forces every deleted script's
behaviour to be pinned by a test in a surviving file, so the removed
``test-config.sh`` cannot have been the only thing covering it.

Known-positive floor: the resolved-row count must meet a floor, AND a self-test
proves a mis-named gate row (a prefix resolving to no test) makes the replay
fail — otherwise it could pass vacuously.
"""

import re
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root

# Every ledger row's final-state gate is one of these surviving files.
_READ_TESTS = "cli/launcher/tests/config_read.rs"
_DRIFT_TEST = "cli/config/src/catalogue.rs"

# The removal set has 20 rows; the replay must resolve at least this many.
_ROW_FLOOR = 20

_ROW = re.compile(r"^\| `([^`]*)`")
_PREFIX = re.compile(r"`([a-z_]+)\*`")


def final_state_file(cell: str) -> str | None:
    """Return the surviving gate file a ledger row names, else ``None``."""
    if "config_read.rs" in cell:
        return _READ_TESTS
    if "drift" in cell:
        return _DRIFT_TEST
    return None


def resolves(prefix: str, content: str) -> bool:
    """Whether ``prefix`` names at least one ``fn`` in ``content``."""
    return re.search(rf"fn {re.escape(prefix)}", content) is not None


def ledger_rows(text: str) -> list[tuple[str, str, str]]:
    """Return ``(path, prefix, final-state cell)`` for each ledger row."""
    rows: list[tuple[str, str, str]] = []
    for line in text.splitlines():
        match = _ROW.match(line)
        if not match:
            continue
        path = match.group(1)
        prefix_match = _PREFIX.search(line)
        cells = [c.strip() for c in line.split("|")]
        # A trailing `|` leaves an empty final field, so the gate is the
        # second-to-last populated cell.
        final_cell = cells[-2] if len(cells) >= 2 else ""
        rows.append(
            (path, prefix_match.group(1) if prefix_match else "", final_cell)
        )
    return rows


def _self_test(read_tests: str) -> list[str]:
    """Return the known-positive-floor failures for the resolver.

    A bogus prefix must not resolve, a known-good one must, and an
    unrecognised final-state gate must be rejected.
    """
    found: list[str] = []
    if resolves("zz_absent_gate_", read_tests):
        found.append("self-test: a bogus prefix resolved — replay is vacuous")
    if not resolves("get_", read_tests):
        found.append("self-test: a known-good prefix did not resolve")
    if final_state_file("no-such-gate.rs") is not None:
        found.append("self-test: an unrecognised final-state gate was accepted")
    return found


def violations(root: Path) -> list[str]:
    """Return failing ledger rows plus floor and self-test failures.

    A row fails when its covering gate does not resolve to a surviving test.
    """
    ledger = root / "meta/inventories/0167-deletion-ledger.md"
    text = ledger.read_text()
    found = _self_test((root / _READ_TESTS).read_text())

    resolved = 0
    for path, prefix, final_cell in ledger_rows(text):
        if not prefix:
            found.append(f"row names no covering-gate prefix: {path}")
            continue
        gate = final_state_file(final_cell)
        if gate is None:
            found.append(f"row {path} names an unrecognised final-state gate")
            continue
        gate_path = root / gate
        if not gate_path.is_file():
            found.append(f"final-state gate file is absent: {gate}")
            continue
        if not resolves(prefix, gate_path.read_text()):
            found.append(
                f"row {path}: covering gate '{prefix}*' resolves to no test "
                f"in {gate}"
            )
            continue
        resolved += 1

    if resolved < _ROW_FLOOR:
        found.append(f"resolved {resolved} row(s), expected >= {_ROW_FLOOR}")
    return found


@task
def check(context: Context) -> None:
    """Fail if a deleted script's covering gate is not a surviving test."""
    offenders = violations(repo_root())
    if offenders:
        raise Exit(
            "replay-deletion-ledger found violation(s):\n  "
            + "\n  ".join(offenders),
            code=1,
        )
