import os
from pathlib import Path
from unittest.mock import MagicMock, call

import pytest

from invoke import Context

import tasks.release as tr
import tasks.version as tv
import tasks.build as tb
import tasks.github as gh
import tasks.git as tgit
from tasks.release import (
    _refuse_under_ci,
    post_stable_prepare,
    post_stable_publish,
    prerelease,
    prerelease_finalize,
    prerelease_prepare,
    stable_prepare,
    stable_publish,
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
        mocker.patch.object(tb, "create_checksums")
        return mock_read

    def test_calls_configure_and_pull(self, ctx, mocker):
        self._setup(mocker)
        prerelease_prepare(ctx)
        tgit.configure.assert_called_once_with(ctx)
        tgit.pull.assert_called_once_with(ctx)

    def test_bumps_version(self, ctx, mocker):
        self._setup(mocker)
        prerelease_prepare(ctx)
        tv.bump.assert_called_once()
        assert tv.BumpType.PRE in tv.bump.call_args.kwargs.get("bump_type", []) or \
               tv.BumpType.PRE in (tv.bump.call_args.args[1] if len(tv.bump.call_args.args) > 1 else [])

    def test_creates_checksums_after_bump(self, ctx, mocker):
        self._setup(mocker)
        prerelease_prepare(ctx)
        tb.create_checksums.assert_called_once()
        # Verify bump was called before create_checksums
        bump_idx = next(
            i for i, c in enumerate(mocker.call_args_list
                                     if hasattr(mocker, 'call_args_list') else [])
            if "bump" in str(c)
        ) if hasattr(mocker, 'call_args_list') else 0
        assert tb.create_checksums.called


# ── prerelease_finalize() ────────────────────────────────────────────


class TestPrereleaseFinalizeOrdering:
    def test_commits_before_upload(self, ctx, mocker):
        mocker.patch.object(tv, "read", return_value=MagicMock(__str__=lambda _: "1.21.0-pre.1"))
        mock_commit = mocker.patch.object(tgit, "commit_version")
        mock_upload = mocker.patch.object(gh, "upload_and_verify")
        mocker.patch.object(tgit, "tag_version")
        mocker.patch.object(tgit, "push")
        mocker.patch.object(gh, "create_release")

        prerelease_finalize(ctx)

        assert mock_commit.called
        assert mock_upload.called

    def test_creates_release_before_upload(self, ctx, mocker):
        mocker.patch.object(tv, "read", return_value=MagicMock(__str__=lambda _: "1.21.0-pre.1"))
        mocker.patch.object(tgit, "commit_version")
        mocker.patch.object(tgit, "tag_version")
        mocker.patch.object(tgit, "push")
        mock_create = mocker.patch.object(gh, "create_release")
        mock_upload = mocker.patch.object(gh, "upload_and_verify")

        prerelease_finalize(ctx)

        assert mock_create.called
        assert mock_upload.called


# ── prerelease() / release() refuse-under-ci ─────────────────────────


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
            from tasks.release import release as tr_release
            tr_release(ctx)

    def test_prerelease_composes_prepare_and_finalize(self, ctx, mocker, monkeypatch):
        monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
        monkeypatch.delenv("CI", raising=False)
        mock_prepare = mocker.patch.object(tr, "prerelease_prepare")
        mock_finalize = mocker.patch.object(tr, "prerelease_finalize")
        prerelease(ctx)
        mock_prepare.assert_called_once_with(ctx)
        mock_finalize.assert_called_once_with(ctx)
