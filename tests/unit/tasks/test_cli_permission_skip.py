"""The cli test task announces a skipped permission suite.

Setting the skip variable turns the root refusal into a silent skip inside
cargo's captured output, so the task says so on its own line where a person
reading the run will see it.
"""

from invoke import Context, MockContext, Result

from tasks.test import cli

SKIP_VARIABLE = "ACCELERATOR_TEST_SKIP_PERMISSION_TESTS"


def _run_task(monkeypatch, capsys) -> str:
    monkeypatch.setenv("ACCELERATOR_COVERAGE", "off")
    context: Context = MockContext(run=Result(exited=0))
    cli.run(context)
    return capsys.readouterr().out


def test_a_set_skip_variable_prints_a_banner(monkeypatch, capsys):
    monkeypatch.setenv(SKIP_VARIABLE, "1")

    out = _run_task(monkeypatch, capsys)

    banner = [line for line in out.splitlines() if SKIP_VARIABLE in line]
    assert len(banner) == 1, out
    assert "permission tests" in banner[0]
    assert "skipped" in banner[0]


def test_no_banner_without_the_skip_variable(monkeypatch, capsys):
    monkeypatch.delenv(SKIP_VARIABLE, raising=False)

    out = _run_task(monkeypatch, capsys)

    assert SKIP_VARIABLE not in out
