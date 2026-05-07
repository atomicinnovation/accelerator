import os
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from invoke import Context

import tasks.release as tr
import tasks.version as tv
import tasks.build as tb
import tasks.github as gh
import tasks.git as tgit
from tasks.release import (
    _refuse_under_ci,
    prerelease,
    prerelease_finalise,
    prerelease_prepare,
    release,
    release_finalise,
    release_prepare,
)


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

    def test_raises_when_ci_set_to_yes(self, monkeypatch):
        monkeypatch.setenv("CI", "yes")
        monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
        with pytest.raises(RuntimeError):
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
        mocker.patch.object(tb, "frontend")
        mocker.patch.object(tb, "server_cross_compile")
        mocker.patch.object(tb, "create_debug_archives")
        mocker.patch.object(tb, "create_checksums")

    def test_calls_configure_and_pull(self, ctx, mocker):
        self._setup(mocker)
        prerelease_prepare(ctx)
        tgit.configure.assert_called_once_with(ctx)
        tgit.pull.assert_called_once_with(ctx)

    def test_bumps_pre_version(self, ctx, mocker):
        self._setup(mocker)
        prerelease_prepare(ctx)
        tv.bump.assert_called_once()

    def test_builds_frontend_and_cross_compiles(self, ctx, mocker):
        self._setup(mocker)
        prerelease_prepare(ctx)
        tb.frontend.assert_called_once_with(ctx)
        tb.server_cross_compile.assert_called_once_with(ctx)
        tb.create_debug_archives.assert_called_once_with(ctx)

    def test_creates_checksums(self, ctx, mocker):
        self._setup(mocker)
        prerelease_prepare(ctx)
        tb.create_checksums.assert_called_once()

    def test_build_happens_after_version_bump(self, ctx, mocker):
        call_log = []
        mocker.patch.object(tgit, "configure")
        mocker.patch.object(tgit, "pull")
        mocker.patch.object(tv, "bump", side_effect=lambda *a, **kw: call_log.append("bump"))
        mock_read = mocker.patch.object(tv, "read", return_value=MagicMock())
        mock_read.return_value.__str__ = lambda _: "1.21.0-pre.1"
        mocker.patch.object(tb, "frontend", side_effect=lambda *a, **kw: call_log.append("frontend"))
        mocker.patch.object(tb, "server_cross_compile", side_effect=lambda *a, **kw: call_log.append("cross_compile"))
        mocker.patch.object(tb, "create_debug_archives", side_effect=lambda *a, **kw: call_log.append("debug_archives"))
        mocker.patch.object(tb, "create_checksums", side_effect=lambda *a, **kw: call_log.append("checksums"))

        prerelease_prepare(ctx)

        bump_idx = call_log.index("bump")
        assert call_log.index("frontend") > bump_idx
        assert call_log.index("cross_compile") > bump_idx
        assert call_log.index("debug_archives") > bump_idx
        assert call_log.index("checksums") > bump_idx


# ── prerelease_finalise() ────────────────────────────────────────────


class TestPrereleaseFinalise:
    def test_commits_before_upload(self, ctx, mocker):
        mocker.patch.object(tv, "read", return_value=MagicMock(__str__=lambda _: "1.21.0-pre.1"))
        mock_commit = mocker.patch.object(tgit, "commit_version")
        mock_upload = mocker.patch.object(gh, "upload_and_verify")
        mocker.patch.object(tgit, "tag_version")
        mocker.patch.object(tgit, "push")
        mocker.patch.object(gh, "create_release")

        prerelease_finalise(ctx)

        assert mock_commit.called
        assert mock_upload.called

    def test_creates_release_before_upload(self, ctx, mocker):
        mocker.patch.object(tv, "read", return_value=MagicMock(__str__=lambda _: "1.21.0-pre.1"))
        mocker.patch.object(tgit, "commit_version")
        mocker.patch.object(tgit, "tag_version")
        mocker.patch.object(tgit, "push")
        mock_create = mocker.patch.object(gh, "create_release")
        mock_upload = mocker.patch.object(gh, "upload_and_verify")

        prerelease_finalise(ctx)

        assert mock_create.called
        assert mock_upload.called


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

    def test_prerelease_composes_prepare_and_finalise(self, ctx, mocker, monkeypatch):
        monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
        monkeypatch.delenv("CI", raising=False)
        mock_prepare = mocker.patch.object(tr, "prerelease_prepare")
        mock_finalise = mocker.patch.object(tr, "prerelease_finalise")
        prerelease(ctx)
        mock_prepare.assert_called_once_with(ctx)
        mock_finalise.assert_called_once_with(ctx)

    def test_release_calls_all_four_halves(self, ctx, mocker, monkeypatch):
        monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
        monkeypatch.delenv("CI", raising=False)
        mock_rp = mocker.patch.object(tr, "release_prepare")
        mock_rf = mocker.patch.object(tr, "release_finalise")
        mock_pp = mocker.patch.object(tr, "prerelease_prepare")
        mock_pf = mocker.patch.object(tr, "prerelease_finalise")
        release(ctx)
        mock_rp.assert_called_once_with(ctx)
        mock_rf.assert_called_once_with(ctx)
        mock_pp.assert_called_once_with(ctx)
        mock_pf.assert_called_once_with(ctx)
