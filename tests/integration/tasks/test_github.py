import json
import os
import shutil
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
    upload_and_verify_release,
    upload_release_asset,
    verify_release_asset,
)
from tasks.shared.errors import InvalidVersionError
from tasks.shared.paths import DISPATCHED_SUBBINARIES
from tasks.shared.targets import TARGETS

_PLATFORMS = tuple(platform for _, platform in TARGETS)
_REPO_ROOT = Path(__file__).resolve().parents[3]


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


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
            with pytest.raises(
                AssetVerificationError, match="gh release download failed"
            ):
                download_release_asset(ctx, "v1.20.0", "binary", out)


# ── verify_release_asset() ────────────────────────────────────────────


class TestVerifyReleaseAsset:
    def test_matching_hash_passes(self, ctx, tmp_path):
        path = tmp_path / "f"
        path.write_bytes(b"hello\n")
        expected = (
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
        )
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
        expected = (
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
        )
        mocker.patch.object(
            gh,
            "download_release_asset",
            side_effect=self._fake_download(b"hello\n"),
        )
        download_and_verify(ctx, "v1.20.0", "binary", expected)

    def test_hash_mismatch_raises(self, ctx, mocker):
        mocker.patch.object(
            gh,
            "download_release_asset",
            side_effect=self._fake_download(b"hello\n"),
        )
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
        expected = (
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
        )
        created = []

        original_download = self._fake_download(b"hello\n")

        def tracking_download(ctx, tag, asset_name, output_path):
            created.append(output_path)
            original_download(ctx, tag, asset_name, output_path)

        mocker.patch.object(
            gh, "download_release_asset", side_effect=tracking_download
        )
        download_and_verify(ctx, "v1.20.0", "binary", expected)
        assert len(created) == 1
        assert not created[0].exists()

    def test_temp_file_cleaned_up_on_failure(self, ctx, mocker):
        created = []

        def tracking_download(ctx, tag, asset_name, output_path):
            created.append(output_path)
            output_path.write_bytes(b"hello\n")

        mocker.patch.object(
            gh, "download_release_asset", side_effect=tracking_download
        )
        with pytest.raises(AssetVerificationError):
            download_and_verify(ctx, "v1.20.0", "binary", "b" * 64)
        assert len(created) == 1
        assert not created[0].exists()


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


# ── upload_and_verify_release() (unified single-gate publish) ─────────


def _in_ci() -> bool:
    return bool(os.environ.get("CI") or os.environ.get("GITHUB_ACTIONS"))


def _require(name: str) -> None:
    if shutil.which(name):
        return
    if _in_ci():
        pytest.fail(f"{name} not on PATH — provisioning regression in CI")
    pytest.skip(f"{name} not on PATH")


def _setup_release(mocker, tmp_path: Path, *, create: bool = True) -> None:
    mocker.patch.object(
        gh,
        "debug_archive_path",
        side_effect=lambda token, p, _dir: (
            tmp_path / f"accelerator-{token}-{p}.debug.tar.gz"
        ),
    )
    mocker.patch.object(
        gh,
        "cli_binary_path",
        side_effect=lambda name, p: tmp_path / f"{name}-{p}",
    )
    mocker.patch.object(
        gh,
        "subbinary_asset_path",
        side_effect=lambda token, p: tmp_path / f"accelerator-{token}-{p}",
    )
    manifest = tmp_path / "manifest.json"
    manifest_sig = tmp_path / "manifest.minisig"
    mocker.patch.object(gh, "RELEASE_MANIFEST", manifest)
    mocker.patch.object(gh, "RELEASE_MANIFEST_SIG", manifest_sig)
    if create:
        for platform in _PLATFORMS:
            (tmp_path / f"accelerator-visualiser-{platform}").write_bytes(
                b"\x00" * 4
            )
            (
                tmp_path / f"accelerator-visualiser-{platform}.debug.tar.gz"
            ).write_bytes(b"\x00" * 8)
            (tmp_path / f"accelerator-{platform}").write_bytes(b"\x00" * 4)
            (tmp_path / f"accelerator-{platform}.minisig").write_text("sig")
            # The shared accelerator-visualiser-{platform} binary (staged above)
            # doubles as the manifest-flow sub-binary asset; sign it once.
            (
                tmp_path / f"accelerator-visualiser-{platform}.minisig"
            ).write_text("sig")
            (tmp_path / f"accelerator-vcs-{platform}").write_bytes(b"\x00" * 4)
            (tmp_path / f"accelerator-vcs-{platform}.minisig").write_text("sig")
            (tmp_path / f"accelerator-work-{platform}").write_bytes(b"\x00" * 4)
            (tmp_path / f"accelerator-work-{platform}.minisig").write_text(
                "sig"
            )
            (tmp_path / f"accelerator-migrate-{platform}").write_bytes(
                b"\x00" * 4
            )
            (tmp_path / f"accelerator-migrate-{platform}.minisig").write_text(
                "sig"
            )
        manifest.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "version": "1.20.0",
                    "binaries": {
                        "visualiser": {
                            "description": (
                                "Launch the interactive meta-directory "
                                "visualiser"
                            ),
                            "platforms": {
                                p: {"sha256": "a" * 64, "signature": "sig"}
                                for p in _PLATFORMS
                            },
                        },
                        "vcs": {
                            "description": (
                                "The vcs detect|status|log|guard sub-binary."
                            ),
                            "platforms": {
                                p: {"sha256": "a" * 64, "signature": "sig"}
                                for p in _PLATFORMS
                            },
                        },
                        "work": {
                            "description": (
                                "The work create|show|resolve|diff|update "
                                "sub-binary."
                            ),
                            "platforms": {
                                p: {"sha256": "a" * 64, "signature": "sig"}
                                for p in _PLATFORMS
                            },
                        },
                        "migrate": {
                            "description": (
                                "Apply pending meta-directory schema "
                                "migrations."
                            ),
                            "platforms": {
                                p: {"sha256": "a" * 64, "signature": "sig"}
                                for p in _PLATFORMS
                            },
                        },
                    },
                }
            )
        )
        manifest_sig.write_text("sig")


class TestUploadAndVerifyRelease:
    def _pass_reverify(self, mocker):
        mocker.patch.object(gh, "download_and_verify")
        mocker.patch.object(gh, "_reverify_via_shim")
        mocker.patch.object(gh, "_reverify_subbinary")

    def test_uploads_every_asset_with_clobber(self, ctx, mocker, tmp_path):
        _setup_release(mocker, tmp_path)
        self._pass_reverify(mocker)
        upload_and_verify_release(ctx, "1.20.0")
        uploads = [
            c for c in ctx.run.call_args_list if "gh release upload" in str(c)
        ]
        debug_and_launcher_uploads = len(_PLATFORMS) * 3
        manifest_and_signature_uploads = 2
        dispatched_subbinary_uploads = (
            len(DISPATCHED_SUBBINARIES) * len(_PLATFORMS) * 2
        )
        assert len(uploads) == (
            debug_and_launcher_uploads
            + manifest_and_signature_uploads
            + dispatched_subbinary_uploads
        )
        assert all("--clobber" in str(c) for c in uploads)

    def test_publishes_once_after_all_reverify(self, ctx, mocker, tmp_path):
        _setup_release(mocker, tmp_path)
        self._pass_reverify(mocker)
        upload_and_verify_release(ctx, "1.20.0")
        flips = [c for c in ctx.run.call_args_list if "--draft=false" in str(c)]
        assert len(flips) == 1

    def test_missing_asset_raises_before_upload(self, ctx, mocker, tmp_path):
        _setup_release(mocker, tmp_path, create=False)
        with pytest.raises(FileNotFoundError):
            upload_and_verify_release(ctx, "1.20.0")
        ctx.run.assert_not_called()

    def test_launcher_reverify_failure_preserves_draft(
        self, ctx, mocker, tmp_path, capsys
    ):
        _setup_release(mocker, tmp_path)
        mocker.patch.object(gh, "download_and_verify")
        mocker.patch.object(
            gh,
            "_reverify_via_shim",
            side_effect=AssetVerificationError("bad"),
        )
        with pytest.raises(AssetVerificationError):
            upload_and_verify_release(ctx, "1.20.0")
        cmds = "".join(str(c.args[0]) for c in ctx.run.call_args_list)
        assert "gh release delete" not in cmds
        assert "--draft=false" not in cmds
        out = capsys.readouterr().out
        assert "Launcher/manifest release" in out
        assert "PRESERVED" in out

    def test_subbinary_reverify_failure_preserves_draft(
        self, ctx, mocker, tmp_path, capsys
    ):
        _setup_release(mocker, tmp_path)
        mocker.patch.object(gh, "download_and_verify")
        mocker.patch.object(gh, "_reverify_via_shim")
        mocker.patch.object(
            gh,
            "_reverify_subbinary",
            side_effect=AssetVerificationError("bad"),
        )
        with pytest.raises(AssetVerificationError):
            upload_and_verify_release(ctx, "1.20.0")
        cmds = "".join(str(c.args[0]) for c in ctx.run.call_args_list)
        assert "gh release delete" not in cmds
        assert "--draft=false" not in cmds
        out = capsys.readouterr().out
        assert "Launcher/manifest release" in out
        assert "PRESERVED" in out

    def test_generic_error_deletes_release(self, ctx, mocker, tmp_path):
        _setup_release(mocker, tmp_path)
        mocker.patch.object(gh, "download_and_verify")
        mocker.patch.object(gh, "_reverify_via_shim")
        mocker.patch.object(
            gh, "_reverify_subbinary", side_effect=RuntimeError("transient")
        )
        with pytest.raises(RuntimeError):
            upload_and_verify_release(ctx, "1.20.0")
        deletes = [
            c for c in ctx.run.call_args_list if "gh release delete" in str(c)
        ]
        assert len(deletes) == 1

    def test_rerun_after_preserved_draft_succeeds(self, ctx, mocker, tmp_path):
        _setup_release(mocker, tmp_path)
        self._pass_reverify(mocker)
        upload_and_verify_release(ctx, "1.20.0")
        upload_and_verify_release(ctx, "1.20.0")
        flips = [c for c in ctx.run.call_args_list if "--draft=false" in str(c)]
        assert len(flips) == 2

    def test_includes_subbinary_assets_when_present(
        self, ctx, mocker, tmp_path
    ):
        _setup_release(mocker, tmp_path)
        for platform in _PLATFORMS:
            (tmp_path / f"accelerator-foo-{platform}").write_bytes(b"\x00" * 4)
            (tmp_path / f"accelerator-foo-{platform}.minisig").write_text("sig")
        (tmp_path / "manifest.json").write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "version": "1.20.0",
                    "binaries": {
                        "foo": {
                            "description": "Foo",
                            "platforms": {
                                p: {"sha256": "a" * 64, "signature": "sig"}
                                for p in _PLATFORMS
                            },
                        }
                    },
                }
            )
        )
        mocker.patch.object(gh, "DISPATCHED_SUBBINARIES", ("foo",))
        self._pass_reverify(mocker)
        mocker.patch.object(gh, "_reverify_subbinary")
        upload_and_verify_release(ctx, "1.20.0")
        uploads = "".join(str(c) for c in ctx.run.call_args_list)
        assert "accelerator-foo-darwin-arm64 --clobber" in uploads
        assert "accelerator-foo-darwin-arm64.minisig --clobber" in uploads


class TestBuilderSeams:
    """Pure list derivation — no signing key, no network."""

    def test_subbinary_uploads_pairs_each_asset_with_its_signature(
        self, mocker, tmp_path
    ):
        _setup_release(mocker, tmp_path, create=False)
        uploads = gh._subbinary_uploads(("alpha", "beta"))

        assert len(uploads) == 2 * len(_PLATFORMS) * 2
        for token in ("alpha", "beta"):
            for platform in _PLATFORMS:
                asset = tmp_path / f"accelerator-{token}-{platform}"
                assert asset in uploads
                assert asset.with_name(asset.name + ".minisig") in uploads

    def test_subbinary_uploads_defaults_to_todays_asset_set(self):
        uploads = gh._subbinary_uploads()

        assert len(uploads) == len(DISPATCHED_SUBBINARIES) * len(_PLATFORMS) * 2
        for token in DISPATCHED_SUBBINARIES:
            assert any(token in path.name for path in uploads)

    def test_subbinary_reverifies_yields_one_item_per_token_per_target(
        self, ctx, mocker, tmp_path
    ):
        _setup_release(mocker, tmp_path, create=False)
        (tmp_path / "manifest.json").write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "version": "1.20.0",
                    "binaries": {
                        token: {
                            "description": token,
                            "platforms": {
                                p: {"sha256": "a" * 64, "signature": "sig"}
                                for p in _PLATFORMS
                            },
                        }
                        for token in ("alpha", "beta")
                    },
                }
            )
        )

        items = gh._subbinary_reverifies(ctx, "v1.20.0", ("alpha", "beta"))

        assert len(items) == 2 * len(_PLATFORMS)

    def test_subbinary_reverifies_defaults_to_todays_asset_set(
        self, ctx, mocker, tmp_path
    ):
        _setup_release(mocker, tmp_path)

        items = gh._subbinary_reverifies(ctx, "v1.20.0")

        assert len(items) == len(DISPATCHED_SUBBINARIES) * len(_PLATFORMS)

    def test_the_dispatched_registry_holds_visualiser_vcs_work_and_migrate(
        self,
    ):
        # A deliberate anti-vacuity anchor, not a count to bump blindly: every
        # default-call assertion above would pass on an emptied registry.
        assert DISPATCHED_SUBBINARIES == (
            "visualiser",
            "vcs",
            "work",
            "migrate",
        )


class TestReverifyViaShim:
    @pytest.fixture(scope="class")
    def shim_bin(self) -> Path:
        _require("cargo")
        subprocess.run(
            [
                "cargo",
                "build",
                "--quiet",
                "-p",
                "accelerator-verify",
                "--manifest-path",
                str(_REPO_ROOT / "cli/Cargo.toml"),
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        shim = _REPO_ROOT / "cli/target/debug/accelerator-verify"
        if not (shim.exists() and os.access(shim, os.X_OK)):
            pytest.fail(f"shim not built: {shim}")
        return shim

    def _keypair(self, tmp_path: Path, name: str) -> tuple[Path, Path]:
        _require("minisign")
        pub = tmp_path / f"{name}.pub"
        sec = tmp_path / f"{name}.sec"
        subprocess.run(
            ["minisign", "-G", "-W", "-f", "-p", str(pub), "-s", str(sec)],
            check=True,
            capture_output=True,
            text=True,
        )
        return pub, sec

    def _sign(self, sec: Path, target: Path) -> Path:
        sig = target.with_name(target.name + ".minisig")
        subprocess.run(
            [
                "minisign",
                "-S",
                "-s",
                str(sec),
                "-x",
                str(sig),
                "-m",
                str(target),
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        return sig

    def _serve(self, mocker, payload: Path, sig: Path) -> None:
        def fake(_ctx, _tag, asset, out):
            src = sig if asset.endswith(".minisig") else payload
            shutil.copy(src, out)

        mocker.patch.object(gh, "download_release_asset", side_effect=fake)

    def test_accepts_a_committed_key_signature(
        self, ctx, mocker, tmp_path, shim_bin
    ):
        pub, sec = self._keypair(tmp_path, "release")
        payload = tmp_path / "accelerator-darwin-arm64"
        payload.write_bytes(b"launcher bytes")
        sig = self._sign(sec, payload)
        self._serve(mocker, payload, sig)
        mocker.patch.object(gh, "vendored_shim_path", return_value=shim_bin)
        mocker.patch.object(gh, "host_platform", return_value="darwin-arm64")
        mocker.patch.object(gh, "RELEASE_PUBLIC_KEY", pub)

        gh._reverify_via_shim(
            ctx, "v1", payload.name, payload.name + ".minisig"
        )

    def test_rejects_a_non_committed_key_signature(
        self, ctx, mocker, tmp_path, shim_bin
    ):
        release_pub, _release_sec = self._keypair(tmp_path, "release")
        _attacker_pub, attacker_sec = self._keypair(tmp_path, "attacker")
        payload = tmp_path / "accelerator-darwin-arm64"
        payload.write_bytes(b"launcher bytes")
        sig = self._sign(attacker_sec, payload)
        self._serve(mocker, payload, sig)
        mocker.patch.object(gh, "vendored_shim_path", return_value=shim_bin)
        mocker.patch.object(gh, "host_platform", return_value="darwin-arm64")
        # Verified against the committed key, not the (attacker) signing key —
        # so the pinning is guarded and cannot pass tautologically.
        mocker.patch.object(gh, "RELEASE_PUBLIC_KEY", release_pub)

        with pytest.raises(AssetVerificationError, match="verification failed"):
            gh._reverify_via_shim(
                ctx, "v1", payload.name, payload.name + ".minisig"
            )
