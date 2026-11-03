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
