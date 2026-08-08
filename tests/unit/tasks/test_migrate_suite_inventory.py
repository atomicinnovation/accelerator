"""Tests for the 0172 Phase 9 assertion extractor in
``tasks/lint/migrate_suite_inventory.py``.

Synthetic ``tmp_path`` trees exercise the extractor's own detection logic —
every recognised assertion form the six real suites actually use, plus
deliberately-tricky non-matches (a commented-out assertion, a call embedded
as a continuation of a preceding multi-line call) — before it is trusted
against the real suites (`test_the_real_suites_have_no_duplicate_or_gap`
below). `tmp_path`-based synthetic trees match this repo's own established
convention for this kind of walk-and-classify gate
(`tests/unit/tasks/test_call_site_migration.py`), not a committed fixture
directory — the plan's own text describing a
`tasks/lint/tests/fixtures/migrate_suite_inventory/` corpus predates this
session verifying which convention the landed codebase actually uses.
"""

from pathlib import Path

from tasks.lint import migrate_suite_inventory as inv

REPO_ROOT = Path(__file__).resolve().parents[3]


def _write(root: Path, rel: str, body: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)


def test_a_plain_call_is_repointable(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "suite.sh",
        'assert_contains "label" "$OUT" "needle"\n',
    )
    sites = inv.extract_sites(tmp_path, "suite.sh")
    assert len(sites) == 1
    assert sites[0] == inv.AssertionSite(
        file="suite.sh",
        line=1,
        call="assert_contains",
        classification="repointable",
        reason="asserts CLI-observable stdout/stderr/exit-code/file-state",
    )


def test_a_commented_out_assertion_is_not_matched(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "suite.sh",
        '# assert_eq "label" "a" "b"\nassert_eq "real" "a" "a"\n',
    )
    sites = inv.extract_sites(tmp_path, "suite.sh")
    assert len(sites) == 1
    assert sites[0].line == 2


def test_a_multi_line_call_is_one_site_at_its_start_line(
    tmp_path: Path,
) -> None:
    _write(
        tmp_path,
        "suite.sh",
        'assert_contains "label" "$OUT" \\\n  "own partial migration"\n'
        'assert_eq "next" "a" "a"\n',
    )
    sites = inv.extract_sites(tmp_path, "suite.sh")
    assert len(sites) == 2
    assert sites[0].line == 1
    assert sites[1].line == 3


def test_a_local_definition_is_not_a_call_but_a_later_call_to_it_is(
    tmp_path: Path,
) -> None:
    _write(
        tmp_path,
        "suite.sh",
        "assert_bridge_unmutated() {\n"
        '  assert_files_identical "$1" "$2"\n'
        "}\n"
        'assert_bridge_unmutated "$before" "$after"\n',
    )
    sites = inv.extract_sites(tmp_path, "suite.sh")
    assert [(s.line, s.call) for s in sites] == [
        (2, "assert_files_identical"),
        (4, "assert_bridge_unmutated"),
    ]


def test_a_wire_protocol_marker_is_not_repointable(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "suite.sh",
        'assert_eq "frame type" "PROMPT" "$FRAME_TYPE"\n',
    )
    sites = inv.extract_sites(tmp_path, "suite.sh")
    assert sites[0].classification == "not-repointable"
    assert "$FRAME_TYPE" in sites[0].reason


def test_test_interactive_protocol_sh_is_always_not_repointable(
    tmp_path: Path,
) -> None:
    _write(
        tmp_path,
        "scripts/test-interactive-protocol.sh",
        'assert_eq "cli-shaped label" "$OUT" "$OUT"\n',
    )
    sites = inv.extract_sites(tmp_path, "scripts/test-interactive-protocol.sh")
    assert sites[0].classification == "not-repointable"
    assert "no CLI surface" in sites[0].reason


def test_duplicates_and_gaps_are_empty_for_well_formed_input(
    tmp_path: Path,
) -> None:
    _write(
        tmp_path,
        "suite.sh",
        'assert_contains "a" "$X" "y" \\\n  "continued"\n'
        'assert_eq "b" "1" "1"\n',
    )
    sites = inv.inventory(tmp_path, ("suite.sh",))
    assert inv.duplicates(sites) == []
    assert inv.gaps(tmp_path, ("suite.sh",)) == []


def test_the_real_suites_have_no_duplicate_or_gap() -> None:
    sites = inv.inventory(REPO_ROOT)
    assert inv.duplicates(sites) == []
    assert inv.gaps(REPO_ROOT) == []
    # Confirms the threshold decision recorded in
    # meta/inventories/0172-suite-audit.md: comfortably over 400, so the
    # exhaustive per-assertion mapping narrows to the three named suites.
    assert len(sites) > 400
