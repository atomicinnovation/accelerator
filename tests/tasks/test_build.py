from pathlib import Path
from unittest.mock import MagicMock

import pytest

from invoke import Context

import tasks.build as tb
from tasks.build import create_checksums
from tasks.shared.releases import VersionCoherenceError
from tasks.shared.targets import TARGETS

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
        (bin_dir / f"accelerator-visualiser-{platform}").write_bytes(b"\x00" * 4)
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

    def test_debug_archives_not_in_checksums_manifest(self, ctx, mocker, fake_binaries):
        mock_update = self._common(mocker, fake_binaries)
        create_checksums(ctx, "1.20.0")
        _, _, hashes_arg = mock_update.call_args.args
        assert all(not k.endswith(".debug.tar.gz") for k in hashes_arg)

    def test_version_drift_aborts_before_disk_writes(self, ctx, mocker, fake_binaries):
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
