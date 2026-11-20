import shlex

import pytest
from invoke import Context, Exit, Result

from tasks.shared.selection import Selection
from tasks.test import integration, unit

_NO_TESTS = (
    "running 0 tests\n"
    "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; "
    "12 filtered out; finished in 0.00s\n"
)
_SOME_TESTS = (
    "running 0 tests\n"
    "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; "
    "12 filtered out; finished in 0.00s\n"
    "running 3 tests\n"
    "test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; "
    "9 filtered out; finished in 0.01s\n"
)

_LIBTEST_LANES = {
    "test:unit:visualiser": unit.visualiser,
    "test:integration:visualiser": integration.visualiser,
}


@pytest.fixture
def context(mocker):
    context = mocker.MagicMock(spec=Context)
    context.run.return_value = Result(stdout=_SOME_TESTS)
    return context


def _commands_run(context):
    return [call.args[0] for call in context.run.call_args_list]


def test_the_unit_visualiser_lane_runs_both_feature_sets_unchanged(context):
    unit.visualiser(context)

    assert [shlex.split(command)[-1] for command in _commands_run(context)] == [
        "dev-frontend",
        "--lib",
    ]


def test_the_unit_visualiser_lane_ends_both_runs_with_the_filter(context):
    unit.visualiser(context, filter_="path_normalisation")

    commands = _commands_run(context)
    assert len(commands) == 2
    for command in commands:
        assert shlex.split(command)[-1] == "path_normalisation"


def test_the_integration_visualiser_lane_runs_its_command_unchanged(context):
    integration.visualiser(context)

    assert shlex.split(_commands_run(context)[0])[-1] == "dev-frontend"


def test_the_integration_visualiser_filter_follows_its_test_targets(context):
    integration.visualiser(context, filter_="spa serving")

    tokens = shlex.split(_commands_run(context)[0])
    assert tokens[-1] == "spa serving"
    assert tokens.index("--test") < len(tokens) - 1


@pytest.mark.parametrize("mise_task", sorted(_LIBTEST_LANES))
def test_a_filtered_libtest_lane_running_no_test_names_the_lane(
    context, mise_task
):
    context.run.return_value = Result(stdout=_NO_TESTS)

    with pytest.raises(Exit) as exc_info:
        _LIBTEST_LANES[mise_task](context, filter_="x")

    assert str(exc_info.value) == Selection("x").nothing_selected(mise_task)
    assert exc_info.value.code == 1


@pytest.mark.parametrize("mise_task", sorted(_LIBTEST_LANES))
def test_a_filtered_libtest_lane_running_some_test_passes(context, mise_task):
    _LIBTEST_LANES[mise_task](context, filter_="x")


@pytest.mark.parametrize("mise_task", sorted(_LIBTEST_LANES))
def test_an_unfiltered_libtest_lane_is_never_checked_for_tests_run(
    context, mise_task
):
    context.run.return_value = Result(stdout=_NO_TESTS)

    _LIBTEST_LANES[mise_task](context)


def test_the_unit_visualiser_lane_counts_tests_across_both_runs(context):
    context.run.side_effect = [
        Result(stdout=_NO_TESTS),
        Result(stdout=_SOME_TESTS),
    ]

    unit.visualiser(context, filter_="embed")


@pytest.mark.parametrize("mise_task", sorted(_LIBTEST_LANES))
def test_a_libtest_lane_s_help_names_libtest_s_substring(mise_task):
    assert "libtest substring" in _LIBTEST_LANES[mise_task].help["filter_"]


@pytest.mark.parametrize("mise_task", sorted(_LIBTEST_LANES))
def test_an_empty_filter_runs_no_libtest_lane(context, mise_task):
    with pytest.raises(Exit, match=r"^--filter needs an expression$"):
        _LIBTEST_LANES[mise_task](context, filter_="")

    context.run.assert_not_called()


def test_the_integration_visualiser_lane_keeps_its_environment(context):
    integration.visualiser(context, filter_="x")

    assert "env" in context.run.call_args.kwargs
