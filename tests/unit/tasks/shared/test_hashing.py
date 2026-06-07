from pathlib import Path

import pytest

from tasks.shared.hashing import compute_sha256

_FIXTURES = Path(__file__).resolve().parent.parent / "fixtures"


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
