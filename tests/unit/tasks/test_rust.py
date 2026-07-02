from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks import pup
from tasks.shared import rust
from tasks.shared.rust import PUP_NIGHTLY


@pytest.fixture
def ctx() -> MagicMock:
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(exited=0, stdout="")
    return m


# ── pup_mode() ────────────────────────────────────────────────────────


class TestPupMode:
    def test_defaults_to_deny_when_env_absent(
        self, monkeypatch: pytest.MonkeyPatch
    ):
        monkeypatch.delenv("ACCELERATOR_PUP_MODE", raising=False)
        assert rust.pup_mode() == "deny"

    @pytest.mark.parametrize("value", ["warn", "Warn", " warn ", "WARN"])
    def test_normalises_warn(self, monkeypatch: pytest.MonkeyPatch, value: str):
        monkeypatch.setenv("ACCELERATOR_PUP_MODE", value)
        assert rust.pup_mode() == "warn"

    @pytest.mark.parametrize("value", ["off", "lenient", "0", "true"])
    def test_unrecognised_value_fails_closed_with_warning(
        self,
        monkeypatch: pytest.MonkeyPatch,
        capsys: pytest.CaptureFixture[str],
        value: str,
    ):
        monkeypatch.setenv("ACCELERATOR_PUP_MODE", value)
        assert rust.pup_mode() == "deny"
        assert "WARNING" in capsys.readouterr().out


# ── coverage_enabled() ────────────────────────────────────────────────


class TestCoverageEnabled:
    def test_defaults_on_when_env_absent(self, monkeypatch: pytest.MonkeyPatch):
        monkeypatch.delenv("ACCELERATOR_COVERAGE", raising=False)
        assert rust.coverage_enabled() is True

    @pytest.mark.parametrize(
        "value", ["off", "false", "0", "no", "OFF", " no "]
    )
    def test_falsey_values_disable(
        self, monkeypatch: pytest.MonkeyPatch, value: str
    ):
        monkeypatch.setenv("ACCELERATOR_COVERAGE", value)
        assert rust.coverage_enabled() is False

    @pytest.mark.parametrize("value", ["on", "yes", "1", "anything"])
    def test_non_falsey_values_enable(
        self, monkeypatch: pytest.MonkeyPatch, value: str
    ):
        monkeypatch.setenv("ACCELERATOR_COVERAGE", value)
        assert rust.coverage_enabled() is True


# ── pup.check() leaf branches ─────────────────────────────────────────


class TestPupCheck:
    def test_runs_pup_on_the_pinned_nightly(self, ctx: MagicMock):
        pup.check(ctx)
        assert ctx.run.call_args.args[0] == f"cargo +{PUP_NIGHTLY} pup"

    def test_deny_mode_raises_on_findings(
        self, ctx: MagicMock, monkeypatch: pytest.MonkeyPatch
    ):
        monkeypatch.delenv("ACCELERATOR_PUP_MODE", raising=False)
        ctx.run.return_value = MagicMock(exited=1)
        with pytest.raises(Exit):
            pup.check(ctx)

    def test_warn_mode_logs_and_returns_cleanly(
        self,
        ctx: MagicMock,
        monkeypatch: pytest.MonkeyPatch,
        capsys: pytest.CaptureFixture[str],
    ):
        monkeypatch.setenv("ACCELERATOR_PUP_MODE", "warn")
        ctx.run.return_value = MagicMock(exited=1)
        pup.check(ctx)
        assert "WARNING" in capsys.readouterr().out
