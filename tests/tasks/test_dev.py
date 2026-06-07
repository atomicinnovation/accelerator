"""Tests for the thin ``@task`` adapters in ``tasks/dev.py``.

The helper/orchestration logic is covered under ``tests/tasks/shared/`` and
``tests/tasks/shared/dev/``; this file covers only the adapter mapping (printed
blocks, exit codes) and the mise-task config shape.
"""

import pytest
from invoke import Context as _Context

from tasks.shared.dev.lifecycle import StopResult, UpResult


# ─── @task adapter output blocks ─────────────────────────────


class TestUpAdapter:
    def test_started_prints_ready_block(self, mocker, capsys):
        from invoke import Exit  # noqa: F401

        from tasks import dev

        mocker.patch.object(dev, "_dev_deps", return_value=object())
        mocker.patch(
            "tasks.dev.bring_up",
            return_value=UpResult(
                "started",
                frontend_url="http://127.0.0.1:54321",
                api_url="http://127.0.0.1:7777",
                api_port=7777,
                dev_dir="/d/dev",
            ),
        )
        dev.up(_Context())
        out = capsys.readouterr().out
        assert "Visualiser dev stack ready." in out
        assert "Frontend: http://127.0.0.1:54321" in out
        assert "API:      http://127.0.0.1:7777" in out
        assert "/d/dev/server.log" in out
        assert "/d/dev/frontend.log" in out

    def test_reused_prints_changes_not_live_heading(self, mocker, capsys):
        from tasks import dev

        mocker.patch.object(dev, "_dev_deps", return_value=object())
        mocker.patch(
            "tasks.dev.bring_up",
            return_value=UpResult(
                "reused",
                frontend_url="http://127.0.0.1:54321",
                api_url="http://127.0.0.1:7777",
                api_port=7777,
                dev_dir="/d/dev",
            ),
        )
        dev.up(_Context())
        out = capsys.readouterr().out
        assert (
            "Dev stack already running (reused) — code changes since it started "
            "are NOT live; run `mise run dev:restart` to apply them." in out
        )

    def test_failed_raises_exit(self, mocker):
        from invoke import Exit

        from tasks import dev

        mocker.patch.object(dev, "_dev_deps", return_value=object())
        mocker.patch(
            "tasks.dev.bring_up",
            return_value=UpResult("failed", message="boom"),
        )
        with pytest.raises(Exit):
            dev.up(_Context())


class TestStopAdapter:
    def test_clean_prints_message(self, mocker, capsys):
        from tasks import dev

        mocker.patch.object(dev, "_dev_deps", return_value=object())
        mocker.patch(
            "tasks.dev.do_stop",
            return_value=StopResult("clean", message="Dev stack not running."),
        )
        dev.stop(_Context())
        assert "Dev stack not running." in capsys.readouterr().out

    def test_survivor_raises_exit(self, mocker):
        from invoke import Exit

        from tasks import dev

        mocker.patch.object(dev, "_dev_deps", return_value=object())
        mocker.patch(
            "tasks.dev.do_stop",
            return_value=StopResult("survivor", pid=9000, message="still alive"),
        )
        with pytest.raises(Exit):
            dev.stop(_Context())
