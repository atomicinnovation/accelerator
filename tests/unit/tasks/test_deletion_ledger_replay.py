"""Tests for the deletion-ledger replay in
``tasks/lint/deletion_ledger_replay.py``.

Unit-level parsing/resolution checks plus a real-tree assertion that every
shipped ledger row resolves to a surviving test.
"""

from pathlib import Path

from tasks.lint import deletion_ledger_replay as replay

REPO_ROOT = Path(__file__).resolve().parents[3]

_ROW = (
    "| `scripts/config-read-value.sh` | `config get` (`get_*` tests) "
    "| abc123 | `config_read.rs` (survives) |"
)


def test_ledger_rows_parses_a_row() -> None:
    rows = replay.ledger_rows(_ROW)
    assert rows == [
        ("scripts/config-read-value.sh", "get_", "`config_read.rs` (survives)")
    ]


def test_final_state_file_mapping() -> None:
    assert replay.final_state_file("`config_read.rs` (survives)").endswith(
        "config_read.rs"
    )
    assert replay.final_state_file("catalogue drift test").endswith(
        "catalogue.rs"
    )
    assert replay.final_state_file("something else") is None


def test_resolves() -> None:
    assert replay.resolves("get_", "fn get_something() {}")
    assert not replay.resolves("nope_", "fn get_something() {}")


def _fixture(tmp_path: Path, rows: str, read_fns: str) -> Path:
    (tmp_path / "meta/inventories").mkdir(parents=True)
    (tmp_path / "meta/inventories/0167-deletion-ledger.md").write_text(rows)
    tests = tmp_path / "cli/launcher/tests"
    tests.mkdir(parents=True)
    (tests / "config_read.rs").write_text(read_fns)
    return tmp_path


def test_a_mis_named_row_fails(tmp_path: Path) -> None:
    # Twenty rows with a bogus prefix; config_read.rs has get_ (for the
    # self-test) but not the bogus prefix.
    row = (
        "| `scripts/config-read-value.sh` | `config get` (`bogus_*` tests) "
        "| abc | `config_read.rs` (survives) |\n"
    )
    _fixture(tmp_path, row * 20, "fn get_x() {}\n")
    flagged = replay.violations(tmp_path)
    assert any("resolves to no test" in v for v in flagged)


def test_self_test_fires_when_known_good_absent(tmp_path: Path) -> None:
    # config_read.rs missing get_ trips the self-test regardless of rows.
    _fixture(tmp_path, _ROW, "fn something_else() {}\n")
    assert any("self-test" in v for v in replay.violations(tmp_path))


def test_the_real_ledger_replays_clean() -> None:
    assert replay.violations(REPO_ROOT) == []
