from pathlib import Path

import pytest

from tasks.github import is_prerelease_version
from tasks.shared.releases import (
    InvalidVersionError,
    _atomic_write_text,
    compute_sha256,
)

_FIXTURES = Path(__file__).resolve().parent.parent / "fixtures"


# ── _atomic_write_text ────────────────────────────────────────────


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


# ── compute_sha256 ────────────────────────────────────────────────


class TestComputeSha256:
    def test_empty_file(self, tmp_path: Path):
        f = tmp_path / "empty"
        f.write_bytes(b"")
        assert compute_sha256(f) == (
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        )

    def test_hello_newline(self, tmp_path: Path):
        f = tmp_path / "hello"
        f.write_bytes(b"hello\n")
        assert compute_sha256(f) == (
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
        )

    def test_missing_path_raises(self, tmp_path: Path):
        with pytest.raises(FileNotFoundError):
            compute_sha256(tmp_path / "nonexistent")

    def test_output_is_lowercase(self):
        result = compute_sha256(_FIXTURES / "tiny_binary.bin")
        assert result == result.lower()

    def test_idempotent(self):
        fixture = _FIXTURES / "tiny_binary.bin"
        assert compute_sha256(fixture) == compute_sha256(fixture)


# ── is_prerelease_version ─────────────────────────────────────────


class TestIsPreReleaseVersion:
    def test_stable_returns_false(self):
        assert is_prerelease_version("1.20.0") is False

    def test_pre_suffix_returns_true(self):
        assert is_prerelease_version("1.20.0-pre.1") is True

    def test_rc_suffix_returns_true(self):
        assert is_prerelease_version("1.20.0-rc.1") is True

    def test_pre_with_build_metadata_returns_true(self):
        assert is_prerelease_version("1.20.0-pre.2+build.42") is True

    def test_incomplete_version_raises(self):
        with pytest.raises(InvalidVersionError):
            is_prerelease_version("1.20")

    def test_empty_string_raises(self):
        with pytest.raises(InvalidVersionError):
            is_prerelease_version("")

    def test_none_raises(self):
        with pytest.raises(InvalidVersionError):
            is_prerelease_version(None)
