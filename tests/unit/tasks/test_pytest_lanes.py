import shlex

import pytest
from invoke import Context, Exit, Result, UnexpectedExit

from tasks.shared.selection import Selection
from tasks.test import integration, unit

_LANES = {
    "test:integration:dev": (
        integration.dev,
        "uv run pytest tests/integration/dev -n auto -v",
    ),
    "test:integration:entrypoint": (
        integration.entrypoint,
        "uv run pytest tests/integration/entrypoint -v",
    ),
    "test:integration:evals": (
        integration.evals,
        "uv run pytest tests/integration/evals -v",
    ),
    "test:integration:pup": (
        integration.pup,
        "uv run pytest tests/integration/pup -v",
    ),
    "test:integration:hooks": (
        integration.hooks,
        "uv run pytest tests/integration/hooks -v",
    ),
    "test:integration:research": (
        integration.research,
        "uv run pytest tests/integration/research -v",
    ),
    "test:integration:research-exhaustive": (
        integration.research_exhaustive,
        "uv run pytest tests/integration/research -v",
    ),
    "test:integration:tasks": (
        integration.build_system,
        "uv run pytest tests/integration/tasks -v",
    ),
    "test:unit:tasks": (
        unit.build_system,
        "uv run pytest tests/unit/tasks -v",
    ),
    "test:unit:frontend-licenses": (
        unit.frontend_licenses,
        "uv run pytest tests/unit/frontend -v",
    ),
    "test:unit:vcs": (
        unit.vcs,
        "uv run pytest tests/unit/vcs -v",
    ),
}

_MISE_TASKS = sorted(_LANES)


@pytest.fixture
def context(mocker):
    return mocker.MagicMock(spec=Context)


def _lane(mise_task):
    return _LANES[mise_task][0]


def _command_run(context):
    return context.run.call_args.args[0]


def _failing_with(exit_code):
    return UnexpectedExit(Result(exited=exit_code))


@pytest.mark.parametrize("mise_task", _MISE_TASKS)
def test_an_unfiltered_lane_runs_its_command_unchanged(context, mise_task):
    lane, command = _LANES[mise_task]

    lane(context)

    assert _command_run(context) == command


@pytest.mark.parametrize("mise_task", _MISE_TASKS)
def test_a_filtered_lane_ends_its_command_with_the_k_expression(
    context, mise_task
):
    _lane(mise_task)(context, filter_="x and y")

    assert _command_run(context).endswith(" -k 'x and y'")
    assert shlex.split(_command_run(context))[-2:] == ["-k", "x and y"]


@pytest.mark.parametrize("mise_task", _MISE_TASKS)
def test_a_filter_that_selects_nothing_names_the_lane(context, mise_task):
    context.run.side_effect = _failing_with(5)

    with pytest.raises(Exit) as exc_info:
        _lane(mise_task)(context, filter_="x and y")

    assert str(exc_info.value) == Selection("x and y").nothing_selected(
        mise_task
    )
    assert exc_info.value.code == 1


@pytest.mark.parametrize("mise_task", _MISE_TASKS)
def test_a_filtered_lane_reports_any_other_failure_as_it_is(context, mise_task):
    failure = _failing_with(1)
    context.run.side_effect = failure

    with pytest.raises(UnexpectedExit) as exc_info:
        _lane(mise_task)(context, filter_="x and y")

    assert exc_info.value is failure


@pytest.mark.parametrize("mise_task", _MISE_TASKS)
def test_an_unfiltered_lane_reports_no_tests_collected_as_it_is(
    context, mise_task
):
    failure = _failing_with(5)
    context.run.side_effect = failure

    with pytest.raises(UnexpectedExit) as exc_info:
        _lane(mise_task)(context)

    assert exc_info.value is failure


@pytest.mark.parametrize("mise_task", _MISE_TASKS)
def test_an_empty_filter_runs_nothing(context, mise_task):
    with pytest.raises(Exit, match=r"^--filter needs an expression$"):
        _lane(mise_task)(context, filter_="")

    context.run.assert_not_called()


@pytest.mark.parametrize("mise_task", _MISE_TASKS)
def test_the_lane_s_help_names_pytest_s_k_expression(mise_task):
    assert "pytest -k expression" in _lane(mise_task).help["filter_"]


def test_the_exhaustive_research_lane_keeps_its_environment(context):
    integration.research_exhaustive(context, filter_="x")

    assert context.run.call_args.kwargs["env"] == {
        "RESEARCH_DIFFERENTIAL_EXHAUSTIVE": "1"
    }
