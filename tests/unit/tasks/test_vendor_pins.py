import datetime as dt
from pathlib import Path

from tasks.shared.vendor.pins import bump_date


def _pins(tmp_path: Path, text: str) -> Path:
    path = tmp_path / "pins.toml"
    path.write_text(text)
    return path


def test_a_bump_date_is_returned_as_written(tmp_path):
    path = _pins(tmp_path, "[keyring]\nbumped = 2026-08-24\n")
    assert bump_date("keyring", path) == dt.date(2026, 8, 24)


def test_a_missing_table_has_no_bump_date(tmp_path):
    assert bump_date("keyring", _pins(tmp_path, "")) is None


def test_a_table_without_bumped_has_no_bump_date(tmp_path):
    path = _pins(tmp_path, '[node]\nversion = "22.22.2"\n')
    assert bump_date("node", path) is None


def test_a_non_table_subject_has_no_bump_date(tmp_path):
    assert bump_date("node", _pins(tmp_path, 'node = "22"\n')) is None
