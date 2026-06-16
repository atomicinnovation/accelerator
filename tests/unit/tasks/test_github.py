from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context

import tasks.github as tg
from tasks.shared.targets import TARGETS

_PLATFORMS = tuple(platform for _, platform in TARGETS)


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


def _patch_paths(mocker, base: Path) -> None:
    bin_dir = base / "skills/visualisation/visualise/bin"
    mocker.patch.object(tg, "CHECKSUMS", bin_dir / "checksums.json")
    mocker.patch.object(
        tg,
        "binary_path",
        lambda platform: bin_dir / f"accelerator-visualiser-{platform}",
    )
    mocker.patch.object(
        tg, "a9r_binary_path", lambda platform: bin_dir / f"a9r-{platform}"
    )
    mocker.patch.object(
        tg,
        "debug_archive_path",
        lambda platform: (
            bin_dir / f"accelerator-visualiser-{platform}.debug.tar.gz"
        ),
    )


@pytest.fixture
def staged_assets(fake_repo_tree: Path) -> Path:
    """Create all release artefacts the transition release uploads."""
    bin_dir = fake_repo_tree / "skills/visualisation/visualise/bin"
    for platform in _PLATFORMS:
        (bin_dir / f"a9r-{platform}").write_bytes(b"\x00")
        (bin_dir / f"accelerator-visualiser-{platform}").write_bytes(b"\x00")
        (
            bin_dir / f"accelerator-visualiser-{platform}.debug.tar.gz"
        ).write_bytes(b"\x00")
    return fake_repo_tree


class TestUploadAndVerify:
    def _common(self, mocker, staged_assets):
        _patch_paths(mocker, staged_assets)
        uploads: list[str] = []
        verified: list[tuple[str, str]] = []
        mocker.patch.object(
            tg,
            "upload_release_asset",
            side_effect=lambda _ctx, _tag, path: uploads.append(
                Path(path).name
            ),
        )
        mocker.patch.object(
            tg,
            "download_and_verify",
            side_effect=lambda _ctx, _tag, name, hex_: verified.append(
                (name, hex_)
            ),
        )
        return uploads, verified

    def test_uploads_both_asset_names_per_platform(
        self, ctx, mocker, staged_assets
    ):
        uploads, _ = self._common(mocker, staged_assets)
        tg.upload_and_verify(ctx, "1.20.0")
        for platform in _PLATFORMS:
            assert f"a9r-{platform}" in uploads
            assert f"accelerator-visualiser-{platform}" in uploads

    def test_verifies_each_asset_against_its_nested_hash(
        self, ctx, mocker, staged_assets
    ):
        _, verified = self._common(mocker, staged_assets)
        tg.upload_and_verify(ctx, "1.20.0")
        verified_map = dict(verified)
        # Hashes come from checksums.example.json (nested by asset name).
        assert verified_map["a9r-darwin-arm64"] == "1" * 64
        assert verified_map["accelerator-visualiser-darwin-arm64"] == "a" * 64

    def test_missing_a9r_checksum_entry_raises(
        self, ctx, mocker, staged_assets
    ):
        _patch_paths(mocker, staged_assets)
        bin_dir = staged_assets / "skills/visualisation/visualise/bin"
        # Strip the a9r entry from one platform to prove the lookup is keyed by
        # asset name, not a flat per-platform string.
        import json

        manifest = bin_dir / "checksums.json"
        data = json.loads(manifest.read_text())
        del data["binaries"]["darwin-arm64"]["a9r-darwin-arm64"]
        manifest.write_text(json.dumps(data))
        mocker.patch.object(tg, "upload_release_asset")
        mocker.patch.object(tg, "download_and_verify")
        with pytest.raises(FileNotFoundError):
            tg.upload_and_verify(ctx, "1.20.0")
