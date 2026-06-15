"""Tests for the suite-count guards in ``tasks/test/integration.py``.

The ``config`` and ``migrate`` tasks assert an at-least floor on the number of
discovered shell suites, so a dropped exec bit (which makes a fail-closed gate
silently vanish from CI) turns the build red instead of shrinking the
regression net. These tests cover both halves: that ``run_shell_suites``
discovery actually shrinks when an exec bit is dropped, and that the guard
fires when discovery falls below the baseline.
"""

import pytest
from invoke import Context, Exit

from tasks.test import helpers, integration


class _FakeContext:
    """Records run() invocations without executing anything."""

    def __init__(self):
        self.ran = []

    def run(self, cmd, *args, **kwargs):
        self.ran.append(cmd)


class TestRunShellSuitesExecBit:
    def test_dropping_exec_bit_shrinks_discovery(self, tmp_path, monkeypatch):
        monkeypatch.setattr(helpers, "repo_root", lambda: tmp_path)
        sub = tmp_path / "scripts"
        sub.mkdir()
        for name in ("test-a.sh", "test-b.sh"):
            p = sub / name
            p.write_text("#!/usr/bin/env bash\n")
            p.chmod(0o755)

        ctx = _FakeContext()
        discovered = helpers.run_shell_suites(ctx, "scripts")
        assert len(discovered) == 2

        (sub / "test-b.sh").chmod(0o644)  # drop the exec bit
        reduced = helpers.run_shell_suites(ctx, "scripts")
        assert len(reduced) == 1, "exec-bit drop must shrink the discovered set"


class TestConfigSuiteGuard:
    def test_guard_fires_when_below_baseline(self, mocker):
        mocker.patch.object(
            integration, "run_shell_suites", return_value=["scripts/test-x.sh"]
        )
        with pytest.raises(Exit):
            integration.config(Context())

    def test_guard_passes_at_baseline(self, mocker):
        # The required-by-name gates must be present, with generic filler making
        # up the rest of the count floor.
        required = list(integration._REQUIRED_CONFIG_SUITES)
        filler = [
            f"scripts/test-{i}.sh"
            for i in range(integration._EXPECTED_CONFIG_SUITES - len(required))
        ]
        suites = required + filler
        mocker.patch.object(
            integration, "run_shell_suites", return_value=suites
        )
        integration.config(Context())  # must not raise


class TestMigrateSuiteGuard:
    def test_guard_fires_when_below_baseline(self, mocker):
        mocker.patch.object(
            integration, "run_shell_suites", return_value=["x/test-a.sh"]
        )
        with pytest.raises(Exit):
            integration.migrate(Context())


class TestConfigParityTwiceRun:
    """The parity gate is only real if BOTH a bash-mode run (A9R_BIN unset) and
    an a9r-mode run (A9R_BIN exported) are issued. A dropped second run would
    silently test bash twice, so the structural contract is pinned here rather
    than left to hand-wired CI steps."""

    def _floor_suites(self) -> list[str]:
        required = list(integration._REQUIRED_CONFIG_SUITES)
        filler = [
            f"scripts/test-{i}.sh"
            for i in range(integration._EXPECTED_CONFIG_SUITES - len(required))
        ]
        return required + filler

    def test_config_runs_bash_mode_without_a9r_bin(self, mocker):
        captured: list[dict | None] = []

        def fake(context, subtree, env=None):
            captured.append(env)
            return self._floor_suites()

        mocker.patch.object(integration, "run_shell_suites", side_effect=fake)
        integration.config(Context())
        assert captured == [None], "config must run bash mode (no A9R_BIN)"

    def test_config_parity_exports_a9r_bin(self, mocker, tmp_path):
        a9r = tmp_path / "skills/visualisation/visualise/target/debug/a9r"
        a9r.parent.mkdir(parents=True)
        a9r.write_text("#!/bin/sh\n")
        mocker.patch.object(integration, "repo_root", lambda: tmp_path)
        captured: list[dict | None] = []

        def fake(context, subtree, env=None):
            captured.append(env)
            return self._floor_suites()

        mocker.patch.object(integration, "run_shell_suites", side_effect=fake)
        integration.config_parity(Context())
        assert len(captured) == 1
        assert captured[0] == {"A9R_BIN": str(a9r)}

    def test_config_parity_fails_loud_when_binary_absent(
        self, mocker, tmp_path
    ):
        # No binary written: the task must abort rather than degrade to a
        # second bash run.
        mocker.patch.object(integration, "repo_root", lambda: tmp_path)
        mocker.patch.object(
            integration, "run_shell_suites", side_effect=AssertionError
        )
        with pytest.raises(Exit):
            integration.config_parity(Context())
