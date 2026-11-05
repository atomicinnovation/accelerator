PLAN = """# Plan

### Acceptance Criterion Map:

| Criterion (work item order) | Test | Phase |
|---|---|---|
| store keeps writes | `a_full_queue_keeps_writes` | 1 |
| retries end | `the_loader_…_gives_up`, `test_cli.py` | 2 |
| docs | `CHANGELOG.md` updated | 3 |

## Next section
"""


def make_repo(root):
    (root / "src").mkdir()
    (root / "src/store.rs").write_text(
        "fn helper() {}\n\n#[test]\nfn a_full_queue_keeps_writes() {}\n"
    )
    (root / "tests").mkdir()
    (root / "tests/test_cli.py").write_text("def test_runs():\n    pass\n")
    (root / "tests/load.rs").write_text(
        "fn the_loader_retrying_three_times_gives_up() {}\n"
    )


def test_each_row_keeps_its_label_and_phase(tmp_path, script):
    make_repo(tmp_path)

    rows = script("criteria_map").criteria(PLAN, tmp_path)

    assert [(r["label"], r["phase"]) for r in rows] == [
        ("store keeps writes", "1"),
        ("retries end", "2"),
        ("docs", "3"),
    ]


def test_a_test_name_resolves_to_its_declaration(tmp_path, script):
    make_repo(tmp_path)

    rows = script("criteria_map").criteria(PLAN, tmp_path)

    hits = rows[0]["tests"][0]["hits"]
    assert hits == [
        {"name": "a_full_queue_keeps_writes", "file": "src/store.rs", "line": 4}
    ]


def test_an_elided_name_matches_by_prefix_and_suffix(tmp_path, script):
    make_repo(tmp_path)

    rows = script("criteria_map").criteria(PLAN, tmp_path)

    hits = rows[1]["tests"][0]["hits"]
    assert [hit["name"] for hit in hits] == [
        "the_loader_retrying_three_times_gives_up"
    ]


def test_a_test_file_name_resolves_to_the_file(tmp_path, script):
    make_repo(tmp_path)

    rows = script("criteria_map").criteria(PLAN, tmp_path)

    assert rows[1]["tests"][1]["hits"] == [
        {"name": "test_cli.py", "file": "tests/test_cli.py", "line": 1}
    ]


def test_text_outside_backticks_becomes_the_rows_note(tmp_path, script):
    make_repo(tmp_path)

    rows = script("criteria_map").criteria(PLAN, tmp_path)

    assert rows[2]["note"] == "updated"


TRACEABILITY_PLAN = """# Plan

## Acceptance-Criteria Traceability

Where a row cites two phases, the first covers every input.

| AC (abridged) | Phase | Test |
|---|---|---|
| store keeps writes | 1 | `a_full_queue_keeps_writes` |
| keeps writes under load | 1, 2 | as above |
| retries end | 2 | `test_cli.py`; as above |

## Performance Considerations
"""


def test_a_traceability_table_is_read_by_its_column_names(tmp_path, script):
    make_repo(tmp_path)

    rows = script("criteria_map").criteria(TRACEABILITY_PLAN, tmp_path)

    assert [(r["label"], r["phase"]) for r in rows] == [
        ("store keeps writes", "1"),
        ("keeps writes under load", "1, 2"),
        ("retries end", "2"),
    ]
    assert rows[0]["tests"][0]["hits"][0]["name"] == "a_full_queue_keeps_writes"


def test_a_row_reading_as_above_takes_the_tests_of_the_row_before(
    tmp_path, script
):
    make_repo(tmp_path)

    rows = script("criteria_map").criteria(TRACEABILITY_PLAN, tmp_path)

    assert [t["ref"] for t in rows[1]["tests"]] == ["a_full_queue_keeps_writes"]
    assert [t["ref"] for t in rows[2]["tests"]] == [
        "test_cli.py",
        "a_full_queue_keeps_writes",
    ]
    assert rows[1]["note"] is None


def test_a_numbered_table_takes_its_label_from_the_criterion_column(
    tmp_path, script
):
    make_repo(tmp_path)
    plan = """### Acceptance-Criteria Traceability:

| # | Criterion (abridged) | Phase | Test(s) |
|---|---|---|---|
| 1 | store keeps writes | 3 | `a_full_queue_keeps_writes` |
"""

    rows = script("criteria_map").criteria(plan, tmp_path)

    assert [(r["label"], r["phase"]) for r in rows] == [
        ("store keeps writes", "3")
    ]
    assert rows[0]["tests"][0]["hits"]


def test_a_table_without_a_phase_column_leaves_the_phase_empty(
    tmp_path, script
):
    make_repo(tmp_path)
    plan = """#### 4. Acceptance-criteria map

| Criterion | Proven by |
|---|---|
| store keeps writes | `a_full_queue_keeps_writes` |
"""

    rows = script("criteria_map").criteria(plan, tmp_path)

    assert [(r["label"], r["phase"]) for r in rows] == [
        ("store keeps writes", "")
    ]
    assert rows[0]["tests"][0]["hits"]
