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


class TestWorkSuiteGuard:
    def test_guard_fires_when_below_baseline(self, mocker):
        mocker.patch.object(
            integration,
            "run_shell_suites",
            return_value=["skills/work/test-a.sh"],
        )
        with pytest.raises(Exit):
            integration.work(Context())

    def test_guard_passes_at_baseline(self, mocker):
        suites = [
            f"skills/work/test-{i}.sh"
            for i in range(integration._EXPECTED_WORK_SUITES)
        ]
        mocker.patch.object(
            integration, "run_shell_suites", return_value=suites
        )
        integration.work(Context())  # must not raise


class TestIntegrationsSuiteGuard:
    def test_guard_fires_when_below_baseline(self, mocker):
        mocker.patch.object(
            integration,
            "run_shell_suites",
            return_value=["skills/integrations/test-a.sh"],
        )
        with pytest.raises(Exit):
            integration.integrations(Context())

    def test_guard_passes_at_baseline(self, mocker):
        suites = [
            f"skills/integrations/test-{i}.sh"
            for i in range(integration._EXPECTED_INTEGRATIONS_SUITES)
        ]
        mocker.patch.object(
            integration, "run_shell_suites", return_value=suites
        )
        integration.integrations(Context())  # must not raise
