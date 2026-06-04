from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks.format import scripts as fmt


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(exited=0, stdout="")
    return m


def _command(ctx) -> str:
    return ctx.run.call_args.args[0]


class TestFormatCheck:
    def test_runs_shfmt_diff_with_no_formatting_flags(self, ctx, mocker):
        mocker.patch.object(fmt, "shell_sources", return_value=["a.sh", "b.sh"])
        fmt.check(ctx)
        cmd = _command(ctx)
        assert cmd.startswith("shfmt -d ")
        assert "a.sh" in cmd and "b.sh" in cmd
        # .editorconfig is the single source of truth: no formatting flags.
        for flag in (" -i ", " -ci", " -bn", " -sr", " -kp"):
            assert flag not in cmd

    def test_raises_on_drift(self, ctx, mocker):
        mocker.patch.object(fmt, "shell_sources", return_value=["a.sh"])
        ctx.run.return_value = MagicMock(exited=1)
        with pytest.raises(Exit):
            fmt.check(ctx)

    def test_noop_when_no_sources(self, ctx, mocker):
        mocker.patch.object(fmt, "shell_sources", return_value=[])
        fmt.check(ctx)
        ctx.run.assert_not_called()


class TestFormatFix:
    def test_runs_shfmt_list_write(self, ctx, mocker):
        mocker.patch.object(fmt, "shell_sources", return_value=["a.sh"])
        fmt.fix(ctx)
        assert _command(ctx).startswith("shfmt -l -w ")
