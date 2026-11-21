import pytest
from invoke import Context, Exit, Result

from tasks.test import integration

_TAP = "# pass 37\n# fail 0\n# skipped 0\n"


@pytest.fixture
def context(mocker):
    return mocker.MagicMock(spec=Context)


@pytest.fixture
def trees(tmp_path):
    driver = tmp_path / "driver"
    browser = tmp_path / "browser"
    driver.mkdir()
    browser.mkdir()
    return driver, browser


def _suite_environment(context):
    [suite_run] = [
        call
        for call in context.run.call_args_list
        if call.args[0].startswith("node --test")
    ]
    return suite_run.kwargs["env"]


def _ensured(driver, browser):
    return Result(
        stdout=(
            f"driver\t{driver}\t{driver}.lease\n"
            f"browser\t{browser}\t{browser}.lease\n"
        ),
        exited=0,
    )


def test_exported_trees_are_handed_to_the_suites(context, monkeypatch, trees):
    driver, browser = trees
    monkeypatch.setenv("ACCELERATOR_TREE_DRIVER", str(driver))
    monkeypatch.setenv("ACCELERATOR_TREE_BROWSER", str(browser))
    context.run.return_value = Result(stdout=_TAP)

    integration.design_automation(context)

    assert _suite_environment(context) == {
        "ACCELERATOR_TREE_DRIVER": str(driver),
        "ACCELERATOR_TREE_BROWSER": str(browser),
    }


def test_unexported_trees_are_ensured_together(context, monkeypatch, trees):
    driver, browser = trees
    monkeypatch.delenv("ACCELERATOR_TREE_DRIVER", raising=False)
    monkeypatch.delenv("ACCELERATOR_TREE_BROWSER", raising=False)
    context.run.side_effect = [_ensured(driver, browser), Result(stdout=_TAP)]

    integration.design_automation(context)

    assert (
        context.run.call_args_list[0]
        .args[0]
        .endswith("cache ensure driver browser")
    )
    assert _suite_environment(context) == {
        "ACCELERATOR_TREE_DRIVER": str(driver),
        "ACCELERATOR_TREE_BROWSER": str(browser),
    }


def test_a_tree_the_ensure_step_does_not_report_refuses_the_lane(
    context, monkeypatch, trees
):
    driver, _ = trees
    monkeypatch.delenv("ACCELERATOR_TREE_DRIVER", raising=False)
    monkeypatch.delenv("ACCELERATOR_TREE_BROWSER", raising=False)
    context.run.return_value = Result(
        stdout=f"driver\t{driver}\t{driver}.lease\n", exited=0
    )

    with pytest.raises(Exit, match="browser tree is not available"):
        integration.design_automation(context)
