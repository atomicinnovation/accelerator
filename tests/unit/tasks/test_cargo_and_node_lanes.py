import shlex

import pytest
from invoke import Context, Exit, Result, UnexpectedExit

from tasks.shared.paths import FRONTEND
from tasks.shared.selection import Selection
from tasks.test import integration, unit
from tasks.test.cli import _MANIFEST

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


def _failing_with(exit_code):
    return UnexpectedExit(Result(exited=exit_code))


def test_the_tracker_contract_lane_runs_its_command_unchanged(context):
    integration.tracker_contract(context)

    assert _commands_run(context) == [
        f"cargo nextest run --profile contract {_MANIFEST}"
    ]


def test_the_tracker_contract_lane_intersects_the_filter_with_its_profile(
    context,
):
    integration.tracker_contract(context, filter_="test(a b)")

    [command] = _commands_run(context)
    assert command.startswith(
        f"cargo nextest run --profile contract {_MANIFEST} "
    )
    assert shlex.split(command)[-2:] == ["-E", "test(a b)"]


def test_a_filtered_tracker_contract_run_with_no_tests_names_the_lane(
    context,
):
    context.run.side_effect = _failing_with(4)

    with pytest.raises(Exit) as exc_info:
        integration.tracker_contract(context, filter_="test(x)")

    assert str(exc_info.value) == Selection("test(x)").nothing_selected(
        "test:integration:tracker-contract"
    )
    assert exc_info.value.code == 1


@pytest.mark.parametrize(
    ("filter_", "exit_code"), [("test(x)", 100), (None, 4)]
)
def test_the_tracker_contract_lane_reports_other_failures_as_they_are(
    context, filter_, exit_code
):
    failure = _failing_with(exit_code)
    context.run.side_effect = failure

    with pytest.raises(UnexpectedExit) as exc_info:
        integration.tracker_contract(context, filter_=filter_)

    assert exc_info.value is failure


def test_the_tracker_contract_lane_keeps_its_environment(context):
    integration.tracker_contract(context, filter_="test(x)")

    assert context.run.call_args.kwargs["env"] == {
        "ACCELERATOR_TRACKER_CONTRACT": "1"
    }


def test_the_tracker_contract_help_names_nextest_s_filterset():
    assert "nextest filterset" in integration.tracker_contract.help["filter_"]


def test_an_empty_filter_runs_no_tracker_contract(context):
    with pytest.raises(Exit, match=r"^--filter needs an expression$"):
        integration.tracker_contract(context, filter_="")

    context.run.assert_not_called()


_VITEST_NOTHING_RAN = (
    " Test Files  126 skipped (126)\n      Tests  2697 skipped (2697)\n"
)
_VITEST_SOME_RAN = (
    " Test Files  1 passed | 125 skipped (126)\n"
    "      Tests  4 passed | 2693 skipped (2697)\n"
)
_VITEST_COLOURED_SOME_RAN = (
    "\x1b[2m      Tests \x1b[22m \x1b[1m\x1b[32m4 passed\x1b[39m\x1b[22m"
    "\x1b[2m | \x1b[22m\x1b[33m2693 skipped\x1b[39m\x1b[90m (2697)\x1b[39m\n"
)
_VITEST_COLOURED_NOTHING_RAN = (
    "\x1b[2m      Tests \x1b[22m \x1b[33m2697 skipped\x1b[39m"
    "\x1b[90m (2697)\x1b[39m\n"
)


def _frontend_command():
    return f"npm --prefix {FRONTEND} run test"


def test_the_frontend_lane_runs_its_command_unchanged(context):
    unit.frontend(context)

    assert _commands_run(context) == [_frontend_command()]


def test_the_frontend_lane_passes_the_filter_to_vitest_after_npm_s_separator(
    context,
):
    context.run.return_value = Result(stdout=_VITEST_SOME_RAN)

    unit.frontend(context, filter_="copy text")

    [command] = _commands_run(context)
    assert shlex.split(command) == [
        *shlex.split(_frontend_command()),
        "--",
        "-t",
        "copy text",
    ]


@pytest.mark.parametrize(
    "output", [_VITEST_NOTHING_RAN, _VITEST_COLOURED_NOTHING_RAN]
)
def test_a_filtered_frontend_run_with_no_test_names_the_lane(context, output):
    context.run.return_value = Result(stdout=output)

    with pytest.raises(Exit) as exc_info:
        unit.frontend(context, filter_="x")

    assert str(exc_info.value) == Selection("x").nothing_selected(
        "test:unit:frontend"
    )
    assert exc_info.value.code == 1


@pytest.mark.parametrize(
    "output", [_VITEST_SOME_RAN, _VITEST_COLOURED_SOME_RAN]
)
def test_a_filtered_frontend_run_with_some_test_passes(context, output):
    context.run.return_value = Result(stdout=output)

    unit.frontend(context, filter_="x")


def test_a_filtered_frontend_run_without_a_summary_fails(context):
    context.run.return_value = Result(stdout="")

    with pytest.raises(Exit, match="no 'Tests' summary line"):
        unit.frontend(context, filter_="x")


def test_an_unfiltered_frontend_run_is_never_checked_for_tests_run(context):
    context.run.return_value = Result(stdout="")

    unit.frontend(context)


def test_the_frontend_help_names_vitest_s_test_name_regex():
    assert "Vitest -t test-name regex" in unit.frontend.help["filter_"]


def test_an_empty_filter_runs_no_frontend_test(context):
    with pytest.raises(Exit, match=r"^--filter needs an expression$"):
        unit.frontend(context, filter_="")

    context.run.assert_not_called()
