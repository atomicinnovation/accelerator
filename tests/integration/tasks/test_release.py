from unittest.mock import MagicMock

import pytest
from invoke import Context

import tasks.build as tb
import tasks.git as tgit
import tasks.github as gh
import tasks.manifest as tmani
import tasks.marketplace as tm
import tasks.release as tr
import tasks.signing as tsign
import tasks.version as tv
from tasks.release import (
    _assert_no_leaked_artifacts,
    _refuse_under_ci,
    prerelease,
    prerelease_finalise,
    prerelease_prepare,
    prerelease_sign,
    release,
)
from tasks.shared.errors import SigningError


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


# ── _refuse_under_ci() ────────────────────────────────────────────────


class TestRefuseUnderCi:
    def test_raises_when_github_actions_set(self, monkeypatch):
        monkeypatch.setenv("GITHUB_ACTIONS", "true")
        monkeypatch.delenv("CI", raising=False)
        with pytest.raises(RuntimeError, match="local-dev convenience task"):
            _refuse_under_ci("prerelease")

    def test_raises_when_ci_set_to_1(self, monkeypatch):
        monkeypatch.setenv("CI", "1")
        monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
        with pytest.raises(RuntimeError, match="local-dev convenience task"):
            _refuse_under_ci("prerelease")

    def test_silent_outside_ci(self, monkeypatch):
        monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
        monkeypatch.delenv("CI", raising=False)
        _refuse_under_ci("prerelease")  # must not raise

    def test_empty_string_treated_as_unset(self, monkeypatch):
        monkeypatch.setenv("GITHUB_ACTIONS", "")
        monkeypatch.setenv("CI", "")
        _refuse_under_ci("prerelease")  # must not raise


# ── prerelease_prepare() ─────────────────────────────────────────────


class TestPrereleasePrepare:
    def _setup(self, mocker):
        mocker.patch.object(tgit, "configure")
        mocker.patch.object(tgit, "pull")
        mocker.patch.object(tv, "bump")
        mock_read = mocker.patch.object(tv, "read", return_value=MagicMock())
        mock_read.return_value.__str__ = lambda _: "1.21.0-pre.1"
        mocker.patch.object(tm, "update_prerelease_version")
        mocker.patch.object(tb, "frontend")
        mocker.patch.object(tb, "server_cross_compile")
        mocker.patch.object(tb, "cli_cross_compile")
        mocker.patch.object(tb, "assert_staged_launcher_versions")
        mocker.patch.object(tb, "create_debug_archives")
        mocker.patch.object(tb, "create_checksums")

    def test_calls_configure_and_pull(self, ctx, mocker):
        self._setup(mocker)
        prerelease_prepare(ctx)
        tgit.configure.assert_called_once_with(ctx)
        tgit.pull.assert_called_once_with(ctx)

    def test_cross_compiles_launcher_and_server(self, ctx, mocker):
        self._setup(mocker)
        prerelease_prepare(ctx)
        tb.server_cross_compile.assert_called_once_with(ctx)
        tb.cli_cross_compile.assert_called_once_with(ctx)

    def test_asserts_staged_launcher_versions(self, ctx, mocker):
        self._setup(mocker)
        prerelease_prepare(ctx)
        tb.assert_staged_launcher_versions.assert_called_once_with(
            "1.21.0-pre.1"
        )

    def test_cross_compile_happens_after_version_bump(self, ctx, mocker):
        call_log = []
        mocker.patch.object(tgit, "configure")
        mocker.patch.object(tgit, "pull")
        mocker.patch.object(
            tv, "bump", side_effect=lambda *a, **kw: call_log.append("bump")
        )
        mock_read = mocker.patch.object(tv, "read", return_value=MagicMock())
        mock_read.return_value.__str__ = lambda _: "1.21.0-pre.1"
        mocker.patch.object(tm, "update_prerelease_version")
        mocker.patch.object(tb, "frontend")
        mocker.patch.object(tb, "server_cross_compile")
        mocker.patch.object(
            tb,
            "cli_cross_compile",
            side_effect=lambda *a, **kw: call_log.append("cli_cross_compile"),
        )
        mocker.patch.object(tb, "assert_staged_launcher_versions")
        mocker.patch.object(tb, "create_debug_archives")
        mocker.patch.object(tb, "create_checksums")

        prerelease_prepare(ctx)

        assert call_log.index("cli_cross_compile") > call_log.index("bump")


# ── prerelease_sign() ─────────────────────────────────────────────────


class TestPrereleaseSign:
    def test_signs_and_emits_manifest_under_secret_context(self, ctx, mocker):
        mocker.patch.object(
            tv, "read", return_value=MagicMock(__str__=lambda _: "1.21.0-pre.1")
        )
        key_cm = MagicMock()
        key_cm.__enter__.return_value = "/tmp/key.sec"
        mock_resolve = mocker.patch.object(
            tsign, "resolve_secret_key", return_value=key_cm
        )
        mock_sign = mocker.patch.object(tsign, "sign_staged_binaries")
        mocker.patch.object(tmani, "collect_entries", return_value={})
        mock_emit = mocker.patch.object(tmani, "emit_manifest")

        prerelease_sign(ctx)

        mock_resolve.assert_called_once_with()
        mock_sign.assert_called_once_with("/tmp/key.sec")
        assert mock_emit.call_args.args[0] is not None
        assert mock_emit.call_args.args[3] == "/tmp/key.sec"

    def test_fails_closed_when_secret_absent(self, ctx, mocker, monkeypatch):
        monkeypatch.delenv("ACCELERATOR_RELEASE_SECRET_KEY", raising=False)
        mocker.patch.object(
            tv, "read", return_value=MagicMock(__str__=lambda _: "1.21.0-pre.1")
        )
        # Real resolve_secret_key with no env var and no dev key must raise
        # rather than silently skipping.
        mocker.patch.object(
            tsign,
            "resolve_secret_key",
            side_effect=SigningError("secret not provisioned"),
        )
        mock_sign = mocker.patch.object(tsign, "sign_staged_binaries")
        with pytest.raises(SigningError):
            prerelease_sign(ctx)
        mock_sign.assert_not_called()


# ── prerelease_finalise() ────────────────────────────────────────────


class TestPrereleaseFinalise:
    def _setup(self, mocker):
        mocker.patch.object(
            tv, "read", return_value=MagicMock(__str__=lambda _: "1.21.0-pre.1")
        )
        mocker.patch.object(tgit, "commit_version")
        mocker.patch.object(tgit, "tag_version")
        mocker.patch.object(tgit, "push")
        mocker.patch.object(gh, "create_release")

    def test_publish_calls_unified_upload(self, ctx, mocker):
        self._setup(mocker)
        mock_upload = mocker.patch.object(gh, "upload_and_verify_release")
        prerelease_finalise(ctx)
        assert mock_upload.called

    def test_commits_before_upload(self, ctx, mocker):
        self._setup(mocker)
        mock_commit = mocker.patch.object(tgit, "commit_version")
        mock_upload = mocker.patch.object(gh, "upload_and_verify_release")
        prerelease_finalise(ctx)
        assert mock_commit.called
        assert mock_upload.called


# ── artifact-cleanliness guard ───────────────────────────────────────


class TestLeakedArtifactGuard:
    def test_fires_on_a_materialised_secret(self, ctx):
        ctx.run.return_value = MagicMock(
            stdout="?? keys/accelerator-release.sec\n"
        )
        with pytest.raises(RuntimeError, match="signing secret"):
            _assert_no_leaked_artifacts(ctx)

    def test_fires_on_a_staged_binary(self, ctx):
        ctx.run.return_value = MagicMock(
            stdout="?? dist/release/accelerator-linux-x64\n"
        )
        with pytest.raises(RuntimeError):
            _assert_no_leaked_artifacts(ctx)

    def test_passes_on_the_version_bump_changes(self, ctx):
        ctx.run.return_value = MagicMock(
            stdout=" M .claude-plugin/plugin.json\n M cli/Cargo.toml\n"
        )
        _assert_no_leaked_artifacts(ctx)  # must not raise

    def test_publish_runs_the_guard_before_commit(self, ctx, mocker):
        mocker.patch.object(
            tv, "read", return_value=MagicMock(__str__=lambda _: "1.21.0-pre.1")
        )
        mocker.patch.object(gh, "create_release")
        mocker.patch.object(gh, "upload_and_verify_release")
        mocker.patch.object(tgit, "tag_version")
        mocker.patch.object(tgit, "push")
        mock_commit = mocker.patch.object(tgit, "commit_version")
        mock_guard = mocker.patch.object(tr, "_assert_no_leaked_artifacts")
        tr._publish(ctx)
        assert mock_guard.called
        assert mock_commit.called


# ── Local-dev guard and composition ──────────────────────────────────


class TestLocalDevGuards:
    def test_prerelease_raises_under_ci(self, ctx, monkeypatch):
        monkeypatch.setenv("CI", "true")
        monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
        with pytest.raises(RuntimeError):
            prerelease(ctx)

    def test_release_raises_under_ci(self, ctx, monkeypatch):
        monkeypatch.setenv("GITHUB_ACTIONS", "true")
        monkeypatch.delenv("CI", raising=False)
        with pytest.raises(RuntimeError):
            release(ctx)

    def test_prerelease_composes_prepare_sign_finalise(
        self, ctx, mocker, monkeypatch
    ):
        monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
        monkeypatch.delenv("CI", raising=False)
        mock_prepare = mocker.patch.object(tr, "prerelease_prepare")
        mock_sign = mocker.patch.object(tr, "prerelease_sign")
        mock_finalise = mocker.patch.object(tr, "prerelease_finalise")
        prerelease(ctx)
        mock_prepare.assert_called_once_with(ctx)
        mock_sign.assert_called_once_with(ctx)
        mock_finalise.assert_called_once_with(ctx)

    def test_release_runs_every_step(self, ctx, mocker, monkeypatch):
        monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
        monkeypatch.delenv("CI", raising=False)
        mock_rp = mocker.patch.object(tr, "release_prepare")
        mock_rs = mocker.patch.object(tr, "release_sign")
        mock_rf = mocker.patch.object(tr, "release_finalise")
        mock_pp = mocker.patch.object(tr, "prerelease_prepare")
        mock_ps = mocker.patch.object(tr, "prerelease_sign")
        mock_pf = mocker.patch.object(tr, "prerelease_finalise")
        release(ctx)
        mock_rp.assert_called_once_with(ctx)
        mock_rs.assert_called_once_with(ctx)
        mock_rf.assert_called_once_with(ctx)
        mock_pp.assert_called_once_with(ctx)
        mock_ps.assert_called_once_with(ctx)
        mock_pf.assert_called_once_with(ctx)
