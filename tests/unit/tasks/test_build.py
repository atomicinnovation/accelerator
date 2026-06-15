import json
import shutil
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context

import tasks.build as tb
from tasks.build import (
    VersionCoherenceError,
    create_checksums,
    update_checksums_json,
    validate_version_coherence,
)
from tasks.shared.errors import InvalidVersionError
from tasks.shared.targets import TARGETS

_FIXTURES = Path(__file__).resolve().parent / "fixtures"

_PLATFORMS = tuple(platform for _, platform in TARGETS)


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


@pytest.fixture
def fake_binaries(fake_repo_tree: Path) -> Path:
    bin_dir = fake_repo_tree / "skills/visualisation/visualise/bin"
    for _, platform in TARGETS:
        (bin_dir / f"accelerator-visualiser-{platform}").write_bytes(
            b"\x00" * 4
        )
    return fake_repo_tree


def _patch_paths(mocker, base: Path) -> None:
    bin_dir = base / "skills/visualisation/visualise/bin"
    mocker.patch.object(tb, "BIN_DIR", bin_dir)
    mocker.patch.object(tb, "CHECKSUMS", bin_dir / "checksums.json")


# ── create_checksums() ────────────────────────────────────────────────


class TestCreateChecksums:
    def _common(self, mocker, fake_binaries):
        _patch_paths(mocker, fake_binaries)
        mocker.patch("tasks.build.validate_version_coherence")
        mock_update = mocker.patch("tasks.build.update_checksums_json")
        mocker.patch("tasks.build.compute_sha256", return_value="a" * 64)
        return mock_update

    def test_writes_all_platform_hashes(self, ctx, mocker, fake_binaries):
        mock_update = self._common(mocker, fake_binaries)
        create_checksums(ctx, "1.20.0")
        _, _, hashes_arg = mock_update.call_args.args
        assert set(hashes_arg.keys()) == set(_PLATFORMS)
        # Each platform nests both release assets (visualiser + a9r).
        for platform in _PLATFORMS:
            assert set(hashes_arg[platform].keys()) == {
                f"accelerator-visualiser-{platform}",
                f"a9r-{platform}",
            }

    def test_debug_archives_not_in_checksums_manifest(
        self, ctx, mocker, fake_binaries
    ):
        mock_update = self._common(mocker, fake_binaries)
        create_checksums(ctx, "1.20.0")
        _, _, hashes_arg = mock_update.call_args.args
        assert all(not k.endswith(".debug.tar.gz") for k in hashes_arg)

    def test_version_drift_aborts_before_disk_writes(
        self, ctx, mocker, fake_binaries
    ):
        _patch_paths(mocker, fake_binaries)
        mocker.patch(
            "tasks.build.validate_version_coherence",
            side_effect=VersionCoherenceError("mismatch"),
        )
        mock_update = mocker.patch("tasks.build.update_checksums_json")

        with pytest.raises(VersionCoherenceError):
            create_checksums(ctx, "1.20.0")

        mock_update.assert_not_called()

    def test_no_real_filesystem_writes(self, ctx, mocker, fake_binaries):
        real_checksums = tb.CHECKSUMS
        before = real_checksums.read_bytes()
        self._common(mocker, fake_binaries)

        create_checksums(ctx, "1.20.0")

        assert real_checksums.read_bytes() == before


# ── update_checksums_json() ───────────────────────────────────────────


class TestUpdateChecksumsJson:
    def _load(self, path: Path) -> dict:
        return json.loads(path.read_text())

    def test_all_four_platforms_updated(self, tmp_path: Path):
        manifest = tmp_path / "checksums.json"
        shutil.copy(_FIXTURES / "checksums.with_sentinels.json", manifest)
        hashes = {
            "darwin-arm64": {
                "accelerator-visualiser-darwin-arm64": "a" * 64,
                "a9r-darwin-arm64": "1" * 64,
            },
            "darwin-x64": {
                "accelerator-visualiser-darwin-x64": "b" * 64,
                "a9r-darwin-x64": "2" * 64,
            },
            "linux-arm64": {
                "accelerator-visualiser-linux-arm64": "c" * 64,
                "a9r-linux-arm64": "3" * 64,
            },
            "linux-x64": {
                "accelerator-visualiser-linux-x64": "d" * 64,
                "a9r-linux-x64": "4" * 64,
            },
        }
        update_checksums_json(manifest, "1.20.0", hashes)
        expected = json.loads(
            (_FIXTURES / "checksums.example.json").read_text()
        )
        assert self._load(manifest) == expected

    def test_single_asset_preserves_others(self, tmp_path: Path):
        manifest = tmp_path / "checksums.json"
        shutil.copy(_FIXTURES / "checksums.example.json", manifest)
        update_checksums_json(
            manifest,
            "1.20.0",
            {"darwin-arm64": {"accelerator-visualiser-darwin-arm64": "e" * 64}},
        )
        binaries = self._load(manifest)["binaries"]
        # The updated asset changed…
        assert (
            binaries["darwin-arm64"]["accelerator-visualiser-darwin-arm64"]
            == f"sha256:{'e' * 64}"
        )
        # …its sibling a9r asset on the same platform is preserved…
        assert (
            binaries["darwin-arm64"]["a9r-darwin-arm64"] == f"sha256:{'1' * 64}"
        )
        # …and every other platform is untouched.
        assert (
            binaries["darwin-x64"]["accelerator-visualiser-darwin-x64"]
            == f"sha256:{'b' * 64}"
        )
        assert (
            binaries["linux-arm64"]["accelerator-visualiser-linux-arm64"]
            == f"sha256:{'c' * 64}"
        )
        assert (
            binaries["linux-x64"]["accelerator-visualiser-linux-x64"]
            == f"sha256:{'d' * 64}"
        )

    def test_flat_legacy_entry_migrated_to_nested(self, tmp_path: Path):
        manifest = tmp_path / "checksums.json"
        manifest.write_text(
            json.dumps(
                {
                    "version": "1.20.0",
                    "binaries": {"darwin-arm64": "sha256:" + "0" * 64},
                }
            )
        )
        update_checksums_json(
            manifest,
            "1.20.0",
            {"darwin-arm64": {"a9r-darwin-arm64": "9" * 64}},
        )
        slot = self._load(manifest)["binaries"]["darwin-arm64"]
        assert isinstance(slot, dict)
        assert slot["a9r-darwin-arm64"] == f"sha256:{'9' * 64}"

    def test_none_platform_hashes_only_updates_version(self, tmp_path: Path):
        manifest = tmp_path / "checksums.json"
        shutil.copy(_FIXTURES / "checksums.example.json", manifest)
        original_binaries = self._load(manifest)["binaries"]
        update_checksums_json(manifest, "1.21.0")
        data = self._load(manifest)
        assert data["version"] == "1.21.0"
        assert data["binaries"] == original_binaries

    def test_missing_manifest_raises(self, tmp_path: Path):
        with pytest.raises(FileNotFoundError):
            update_checksums_json(tmp_path / "nonexistent.json", "1.20.0")

    def test_atomic_write_failure_preserves_original(
        self, tmp_path: Path, mocker
    ):
        manifest = tmp_path / "checksums.json"
        shutil.copy(_FIXTURES / "checksums.example.json", manifest)
        original = manifest.read_bytes()
        mocker.patch.object(
            Path, "write_text", side_effect=OSError("disk full")
        )
        with pytest.raises(OSError):
            update_checksums_json(
                manifest,
                "1.20.0",
                {
                    "darwin-arm64": {
                        "accelerator-visualiser-darwin-arm64": "e" * 64
                    }
                },
            )
        assert manifest.read_bytes() == original
        assert not (tmp_path / "checksums.json.tmp").exists()


# ── validate_version_coherence() ─────────────────────────────────────


class TestValidateVersionCoherence:
    def test_all_match_returns_none(self, fake_repo_tree: Path):
        result = validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert result is None

    def test_cargo_toml_mismatch_raises(self, fake_repo_tree: Path):
        cargo = fake_repo_tree / "skills/visualisation/visualise/Cargo.toml"
        cargo.write_text(
            '[workspace]\nmembers = ["server"]\n\n'
            '[workspace.package]\nversion = "0.9.0"\nedition = "2021"\n'
        )
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "Cargo.toml" in str(exc_info.value)
        assert "0.9.0" in str(exc_info.value)

    def test_missing_workspace_version_key_fails_closed(
        self, fake_repo_tree: Path
    ):
        # Fail-closed guard: if the [workspace.package].version key is absent
        # (e.g. a botched inheritance edit), coherence must abort loudly, not
        # silently pass — this is the guard that prevents shipping a binary
        # mismatched to its checksums manifest.
        cargo = fake_repo_tree / "skills/visualisation/visualise/Cargo.toml"
        cargo.write_text('[workspace]\nmembers = ["server"]\n')
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "workspace.package" in str(exc_info.value)

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

    def test_empty_expected_version_raises_invalid_version(
        self, fake_repo_tree: Path
    ):
        with pytest.raises(InvalidVersionError):
            validate_version_coherence("", repo_root=fake_repo_tree)
