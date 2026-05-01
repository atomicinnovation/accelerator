import json
from pathlib import Path

import pytest

from tasks.shared.releases import (
    InvalidVersionError,
    VersionCoherenceError,
    _atomic_write_text,
    compute_sha256,
    is_prerelease_version,
    update_checksums_json,
    validate_version_coherence,
)

_FIXTURES = Path(__file__).resolve().parent / "fixtures"


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


# ── update_checksums_json ─────────────────────────────────────────


class TestUpdateChecksumsJson:
    def _load(self, path: Path) -> dict:
        return json.loads(path.read_text())

    def test_all_four_platforms_updated(self, tmp_path: Path):
        manifest = tmp_path / "checksums.json"
        import shutil
        shutil.copy(_FIXTURES / "checksums.with_sentinels.json", manifest)
        hashes = {
            "darwin-arm64": "a" * 64,
            "darwin-x64":   "b" * 64,
            "linux-arm64":  "c" * 64,
            "linux-x64":    "d" * 64,
        }
        update_checksums_json(manifest, "1.20.0", hashes)
        expected = json.loads(
            (_FIXTURES / "checksums.example.json").read_text()
        )
        assert self._load(manifest) == expected

    def test_single_platform_preserves_others(self, tmp_path: Path):
        manifest = tmp_path / "checksums.json"
        import shutil
        shutil.copy(_FIXTURES / "checksums.example.json", manifest)
        update_checksums_json(manifest, "1.20.0", {"darwin-arm64": "e" * 64})
        data = self._load(manifest)
        assert data["binaries"]["darwin-arm64"] == f"sha256:{'e' * 64}"
        assert data["binaries"]["darwin-x64"] == f"sha256:{'b' * 64}"
        assert data["binaries"]["linux-arm64"] == f"sha256:{'c' * 64}"
        assert data["binaries"]["linux-x64"] == f"sha256:{'d' * 64}"

    def test_none_platform_hashes_only_updates_version(self, tmp_path: Path):
        manifest = tmp_path / "checksums.json"
        import shutil
        shutil.copy(_FIXTURES / "checksums.example.json", manifest)
        original_binaries = self._load(manifest)["binaries"]
        update_checksums_json(manifest, "1.21.0")
        data = self._load(manifest)
        assert data["version"] == "1.21.0"
        assert data["binaries"] == original_binaries

    def test_missing_manifest_raises(self, tmp_path: Path):
        with pytest.raises(FileNotFoundError):
            update_checksums_json(tmp_path / "nonexistent.json", "1.20.0")

    def test_atomic_write_failure_preserves_original(self, tmp_path: Path, mocker):
        manifest = tmp_path / "checksums.json"
        import shutil
        shutil.copy(_FIXTURES / "checksums.example.json", manifest)
        original = manifest.read_bytes()
        mocker.patch.object(Path, "write_text", side_effect=OSError("disk full"))
        with pytest.raises(OSError):
            update_checksums_json(manifest, "1.20.0", {"darwin-arm64": "e" * 64})
        assert manifest.read_bytes() == original
        assert not (tmp_path / "checksums.json.tmp").exists()


# ── validate_version_coherence ────────────────────────────────────


class TestValidateVersionCoherence:
    def test_all_match_returns_none(self, fake_repo_tree: Path):
        result = validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert result is None

    def test_cargo_toml_mismatch_raises(self, fake_repo_tree: Path):
        cargo = (
            fake_repo_tree
            / "skills/visualisation/visualise/server/Cargo.toml"
        )
        cargo.write_text('[package]\nname = "x"\nversion = "0.9.0"\n')
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "Cargo.toml" in str(exc_info.value)
        assert "0.9.0" in str(exc_info.value)

    def test_plugin_json_mismatch_raises(self, fake_repo_tree: Path):
        plugin = fake_repo_tree / ".claude-plugin/plugin.json"
        plugin.write_text('{"name":"accelerator","version":"0.9.0"}')
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "plugin.json" in str(exc_info.value)

    def test_checksums_json_mismatch_raises(self, fake_repo_tree: Path):
        checksums = (
            fake_repo_tree / "skills/visualisation/visualise/bin/checksums.json"
        )
        data = json.loads(checksums.read_text())
        data["version"] = "0.9.0"
        checksums.write_text(json.dumps(data))
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "checksums.json" in str(exc_info.value)

    def test_missing_file_raises_file_not_found(self, fake_repo_tree: Path):
        (fake_repo_tree / ".claude-plugin/plugin.json").unlink()
        with pytest.raises(FileNotFoundError):
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)

    def test_empty_expected_version_raises_invalid_version(self, fake_repo_tree: Path):
        with pytest.raises(InvalidVersionError):
            validate_version_coherence("", repo_root=fake_repo_tree)


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
