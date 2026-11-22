from pathlib import Path

import pytest
from invoke import Context, Exit, Result

from tasks.shared.paths import CLI_TARGET_DIR
from tasks.test import integration
from tasks.test.helpers import accelerator_env

_TAP = "# pass 38\n# fail 0\n# skipped 0\n"


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


def test_unexported_trees_are_ensured_together_by_the_dev_launcher(
    context, monkeypatch, trees
):
    driver, browser = trees
    monkeypatch.delenv("ACCELERATOR_TREE_DRIVER", raising=False)
    monkeypatch.delenv("ACCELERATOR_TREE_BROWSER", raising=False)
    monkeypatch.setenv("ACCELERATOR_BIN", "/dev/launcher")
    context.run.side_effect = [_ensured(driver, browser), Result(stdout=_TAP)]

    integration.design_automation(context)

    ensure = context.run.call_args_list[0]
    assert ensure.args[0] == "/dev/launcher cache ensure driver browser"
    assert ensure.kwargs["env"].items() >= accelerator_env().items()


def test_the_trees_are_materialised_outside_the_shipped_bin(
    context, monkeypatch, trees
):
    driver, browser = trees
    monkeypatch.delenv("ACCELERATOR_TREE_DRIVER", raising=False)
    monkeypatch.delenv("ACCELERATOR_TREE_BROWSER", raising=False)
    context.run.side_effect = [_ensured(driver, browser), Result(stdout=_TAP)]

    integration.design_automation(context)

    cache = Path(
        context.run.call_args_list[0].kwargs["env"]["ACCELERATOR_CACHE_DIR"]
    )
    assert cache.is_relative_to(CLI_TARGET_DIR)
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
