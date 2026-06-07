import json
import subprocess
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from invoke import Context

import tasks.github as gh
from tasks.github import (
    AssetVerificationError,
    create_release,
    download_and_verify,
    download_release_asset,
    is_prerelease_version,
    upload_and_verify,
    upload_release_asset,
    verify_release_asset,
)
from tasks.shared.errors import InvalidVersionError
from tasks.shared.targets import TARGETS

_PLATFORMS = tuple(platform for _, platform in TARGETS)


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


def _setup_upload_and_verify(
    mocker,
    tmp_path: Path,
    *,
    create_binaries: bool = True,
    create_archives: bool = True,
) -> None:
    checksums_file = tmp_path / "checksums.json"
    checksums_file.write_text(json.dumps({
        "version": "1.20.0",
        "binaries": {p: f"sha256:{'a' * 64}" for p in _PLATFORMS},
    }))
    mocker.patch.object(gh, "CHECKSUMS", checksums_file)
    mocker.patch.object(
        gh, "binary_path",
        side_effect=lambda p: tmp_path / f"accelerator-visualiser-{p}",
    )
    mocker.patch.object(
        gh, "debug_archive_path",
        side_effect=lambda p: tmp_path / f"accelerator-visualiser-{p}.debug.tar.gz",
    )
    if create_binaries:
        for platform in _PLATFORMS:
            (tmp_path / f"accelerator-visualiser-{platform}").write_bytes(b"\x00" * 4)
    if create_archives:
        for platform in _PLATFORMS:
            (tmp_path / f"accelerator-visualiser-{platform}.debug.tar.gz").write_bytes(
                b"\x00" * 8
            )


# ── create_release() ─────────────────────────────────────────────────


class TestCreateRelease:
    def test_stable_version_no_prerelease_flag(self, ctx):
        create_release(ctx, target_version="1.20.0")
        cmd = ctx.run.call_args.args[0]
        assert "gh release create v1.20.0" in cmd
        assert "--draft" in cmd
        assert "--prerelease" not in cmd

    def test_prerelease_version_adds_prerelease_flag(self, ctx):
        create_release(ctx, target_version="1.20.0-pre.5")
        cmd = ctx.run.call_args.args[0]
        assert "v1.20.0-pre.5" in cmd
        assert "--prerelease" in cmd

    def test_uses_v_prefixed_draft_tag(self, ctx):
        create_release(ctx, target_version="1.20.0")
        cmd = ctx.run.call_args.args[0]
        assert "v1.20.0" in cmd
        assert "--draft" in cmd

    def test_malformed_version_raises_before_run(self, ctx):
        with pytest.raises(InvalidVersionError):
            create_release(ctx, target_version="not-a-version")
        ctx.run.assert_not_called()


# ── upload_release_asset() ────────────────────────────────────────────


class TestUploadReleaseAsset:
    def test_runs_gh_release_upload(self, ctx, tmp_path):
        path = tmp_path / "binary"
        path.write_bytes(b"\x00")
        upload_release_asset(ctx, "v1.20.0", path)
        ctx.run.assert_called_once()
        assert "gh release upload" in ctx.run.call_args.args[0]
        assert "v1.20.0" in ctx.run.call_args.args[0]


# ── download_release_asset() ──────────────────────────────────────────


class TestDownloadReleaseAsset:
    def test_success_writes_file(self, ctx, tmp_path):
        out = tmp_path / "out"
        captured = {}

        def fake_run(cmd, **kwargs):
            captured["cmd"] = cmd
            Path(cmd[cmd.index("--output") + 1]).write_bytes(b"data")
            return MagicMock(returncode=0, stderr="")

        with pytest.MonkeyPatch().context() as mp:
            mp.setattr(subprocess, "run", fake_run)
            download_release_asset(ctx, "v1.20.0", "binary", out)

        assert out.read_bytes() == b"data"
        cmd = captured["cmd"]
        assert "--pattern" in cmd
        assert cmd[cmd.index("--pattern") + 1] == "binary"

    def test_non_zero_exit_raises(self, ctx, tmp_path):
        out = tmp_path / "out"
        with pytest.MonkeyPatch().context() as mp:
            mp.setattr(
                subprocess,
                "run",
                lambda *a, **kw: MagicMock(returncode=1, stderr="not found"),
            )
            with pytest.raises(AssetVerificationError, match="gh release download failed"):
                download_release_asset(ctx, "v1.20.0", "binary", out)


# ── verify_release_asset() ────────────────────────────────────────────


class TestVerifyReleaseAsset:
    def test_matching_hash_passes(self, ctx, tmp_path):
        path = tmp_path / "f"
        path.write_bytes(b"hello\n")
        expected = "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
        verify_release_asset(ctx, path, expected)

    def test_hash_mismatch_raises(self, ctx, tmp_path):
        path = tmp_path / "f"
        path.write_bytes(b"hello\n")
        with pytest.raises(AssetVerificationError, match="expected sha256"):
            verify_release_asset(ctx, path, "b" * 64)


# ── download_and_verify() ─────────────────────────────────────────────


class TestDownloadAndVerify:
    def _fake_download(self, content: bytes):
        def fake(ctx, tag, asset_name, output_path):
            output_path.write_bytes(content)
        return fake

    def test_matching_hash_passes(self, ctx, mocker):
        expected = "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
        mocker.patch.object(gh, "download_release_asset", side_effect=self._fake_download(b"hello\n"))
        download_and_verify(ctx, "v1.20.0", "binary", expected)

    def test_hash_mismatch_raises(self, ctx, mocker):
        mocker.patch.object(gh, "download_release_asset", side_effect=self._fake_download(b"hello\n"))
        with pytest.raises(AssetVerificationError, match="expected sha256"):
            download_and_verify(ctx, "v1.20.0", "binary", "b" * 64)

    def test_timeout_raises(self, ctx, mocker):
        mocker.patch.object(
            gh,
            "download_release_asset",
            side_effect=subprocess.TimeoutExpired(["gh"], 120),
        )
        with pytest.raises(AssetVerificationError, match="timed out"):
            download_and_verify(ctx, "v1.20.0", "binary", "a" * 64)

    def test_temp_file_cleaned_up_on_success(self, ctx, mocker, tmp_path):
        expected = "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
        created = []

        original_download = self._fake_download(b"hello\n")

        def tracking_download(ctx, tag, asset_name, output_path):
            created.append(output_path)
            original_download(ctx, tag, asset_name, output_path)

        mocker.patch.object(gh, "download_release_asset", side_effect=tracking_download)
        download_and_verify(ctx, "v1.20.0", "binary", expected)
        assert len(created) == 1
        assert not created[0].exists()

    def test_temp_file_cleaned_up_on_failure(self, ctx, mocker):
        created = []

        def tracking_download(ctx, tag, asset_name, output_path):
            created.append(output_path)
            output_path.write_bytes(b"hello\n")

        mocker.patch.object(gh, "download_release_asset", side_effect=tracking_download)
        with pytest.raises(AssetVerificationError):
            download_and_verify(ctx, "v1.20.0", "binary", "b" * 64)
        assert len(created) == 1
        assert not created[0].exists()


# ── upload_and_verify() ───────────────────────────────────────────────


class TestUploadAndVerify:
    def test_missing_binary_raises_before_upload(self, ctx, mocker, tmp_path):
        _setup_upload_and_verify(mocker, tmp_path, create_binaries=False, create_archives=False)
        with pytest.raises(FileNotFoundError):
            upload_and_verify(ctx, "1.20.0")
        ctx.run.assert_not_called()

    def test_missing_archive_raises_before_upload(self, ctx, mocker, tmp_path):
        _setup_upload_and_verify(mocker, tmp_path, create_binaries=True, create_archives=False)
        with pytest.raises(FileNotFoundError):
            upload_and_verify(ctx, "1.20.0")
        ctx.run.assert_not_called()

    def test_all_assets_uploaded(self, ctx, mocker, tmp_path):
        _setup_upload_and_verify(mocker, tmp_path)
        mocker.patch.object(gh, "download_and_verify")
        upload_and_verify(ctx, "1.20.0")
        upload_calls = [c for c in ctx.run.call_args_list if "gh release upload" in str(c)]
        assert len(upload_calls) == 8

    def test_publish_runs_after_verify(self, ctx, mocker, tmp_path):
        _setup_upload_and_verify(mocker, tmp_path)
        mocker.patch.object(gh, "download_and_verify")
        upload_and_verify(ctx, "1.20.0")
        all_cmds = [str(c.args[0]) for c in ctx.run.call_args_list]
        assert any("--draft=false" in s for s in all_cmds)

    def test_verify_called_once_per_binary(self, ctx, mocker, tmp_path):
        _setup_upload_and_verify(mocker, tmp_path)
        mock_verify = mocker.patch.object(gh, "download_and_verify")
        upload_and_verify(ctx, "1.20.0")
        assert mock_verify.call_count == 4

    def test_verify_skips_debug_archives(self, ctx, mocker, tmp_path):
        _setup_upload_and_verify(mocker, tmp_path)
        mock_verify = mocker.patch.object(gh, "download_and_verify")
        upload_and_verify(ctx, "1.20.0")
        for call in mock_verify.call_args_list:
            asset_name = call.args[2] if len(call.args) > 2 else call.kwargs.get("asset_name")
            assert not asset_name.endswith(".debug.tar.gz")

    def test_asset_verification_error_preserves_draft(
        self, ctx, mocker, tmp_path, capsys
    ):
        _setup_upload_and_verify(mocker, tmp_path)
        mocker.patch.object(
            gh, "download_and_verify", side_effect=AssetVerificationError("mismatch")
        )
        with pytest.raises(AssetVerificationError):
            upload_and_verify(ctx, "1.20.0")
        all_cmds = "".join(str(c.args[0]) for c in ctx.run.call_args_list)
        assert "gh release delete" not in all_cmds
        assert "--draft=false" not in all_cmds

    def test_asset_verification_error_emits_alert(
        self, ctx, mocker, tmp_path, capsys
    ):
        _setup_upload_and_verify(mocker, tmp_path)
        mocker.patch.object(
            gh, "download_and_verify", side_effect=AssetVerificationError("mismatch")
        )
        with pytest.raises(AssetVerificationError):
            upload_and_verify(ctx, "1.20.0")
        out = capsys.readouterr().out
        assert "AssetVerificationError" in out
        assert "PRESERVED" in out

    def test_generic_exception_deletes_draft(self, ctx, mocker, tmp_path):
        _setup_upload_and_verify(mocker, tmp_path)
        mocker.patch.object(
            gh, "download_and_verify", side_effect=RuntimeError("transient")
        )
        with pytest.raises(RuntimeError):
            upload_and_verify(ctx, "1.20.0")
        delete_calls = [c for c in ctx.run.call_args_list if "gh release delete" in str(c)]
        assert len(delete_calls) == 1

    def test_cleanup_invocation_uses_warn_and_timeout(self, ctx, mocker, tmp_path):
        _setup_upload_and_verify(mocker, tmp_path)
        mocker.patch.object(
            gh, "download_and_verify", side_effect=RuntimeError("transient")
        )
        with pytest.raises(RuntimeError):
            upload_and_verify(ctx, "1.20.0")
        delete_call = next(
            c for c in ctx.run.call_args_list if "gh release delete" in str(c)
        )
        assert delete_call.kwargs.get("warn") is True
        assert delete_call.kwargs.get("timeout") == 120

    def test_verify_short_circuits_on_first_mismatch(self, ctx, mocker, tmp_path):
        _setup_upload_and_verify(mocker, tmp_path)
        call_count = 0

        def fail_first(*args, **kwargs):
            nonlocal call_count
            call_count += 1
            raise AssetVerificationError("mismatch")

        mocker.patch.object(gh, "download_and_verify", side_effect=fail_first)
        with pytest.raises(AssetVerificationError):
            upload_and_verify(ctx, "1.20.0")
        assert call_count == 1

    def test_keyboard_interrupt_skips_cleanup(self, ctx, mocker, tmp_path):
        _setup_upload_and_verify(mocker, tmp_path)
        mocker.patch.object(
            gh, "download_and_verify", side_effect=KeyboardInterrupt
        )
        with pytest.raises(KeyboardInterrupt):
            upload_and_verify(ctx, "1.20.0")
        delete_calls = [c for c in ctx.run.call_args_list if "gh release delete" in str(c)]
        assert len(delete_calls) == 0

    def test_cleanup_failure_does_not_mask_original_error(
        self, ctx, mocker, tmp_path
    ):
        _setup_upload_and_verify(mocker, tmp_path)
        mocker.patch.object(
            gh,
            "download_and_verify",
            side_effect=subprocess.CalledProcessError(1, "gh upload"),
        )
        with pytest.raises(subprocess.CalledProcessError, match="gh upload"):
            upload_and_verify(ctx, "1.20.0")


# ── is_prerelease_version() ───────────────────────────────────────────


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
