from pathlib import Path

import pytest

from tasks.shared.files import _atomic_write_text


class TestAtomicWriteText:
    def test_successful_write(self, tmp_path: Path):
        target = tmp_path / "out.txt"
        _atomic_write_text(target, "hello")
        assert target.read_text() == "hello"
        assert not (tmp_path / "out.txt.tmp").exists()

    def test_overwrites_pre_existing_target(self, tmp_path: Path):
        target = tmp_path / "out.txt"
        target.write_text("original")
        _atomic_write_text(target, "updated")
        assert target.read_text() == "updated"
        assert not (tmp_path / "out.txt.tmp").exists()

    def test_mid_write_oserror_preserves_original(self, tmp_path: Path, mocker):
        target = tmp_path / "out.txt"
        target.write_text("original")
        mocker.patch.object(Path, "write_text", side_effect=OSError("disk full"))
        with pytest.raises(OSError, match="disk full"):
            _atomic_write_text(target, "new content")
        assert target.read_text() == "original"
        assert not (tmp_path / "out.txt.tmp").exists()

    def test_keyboard_interrupt_cleans_up_and_propagates(self, tmp_path: Path, mocker):
        target = tmp_path / "out.txt"
        target.write_text("original")
        mocker.patch.object(Path, "write_text", side_effect=KeyboardInterrupt)
        with pytest.raises(KeyboardInterrupt):
            _atomic_write_text(target, "new content")
        assert target.read_text() == "original"
        assert not (tmp_path / "out.txt.tmp").exists()
