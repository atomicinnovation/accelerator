import json
from pathlib import Path

import pytest

from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.errors import (
    IncompatibleSettingsError,
    InvalidSettingsError,
)
from tasks.shared.build_measurement.measured_build import HostSettings, Mode
from tasks.shared.build_measurement.settings import (
    BuildSettings,
    FullBuild,
    IsolatedTasks,
    Priming,
    TimedRun,
    log_name,
)
from tasks.shared.build_measurement.task_graph import TaskGraph
from tasks.shared.build_measurement.tracked_processes import (
    DEFAULT_TRACKED_PROCESSES,
    IDE,
)

_GRAPH = TaskGraph.parse(
    json.dumps(
        [
            {"name": "default", "depends": ["check", "test"], "run": []},
            {"name": "check", "depends": [], "run": ["invoke check"]},
            {"name": "test", "depends": ["check"], "run": ["invoke test"]},
            {"name": "measure:build", "depends": [], "run": ["invoke m"]},
        ]
    )
)


def _settings(**overrides):
    arguments = {
        "root": Path("/checkout"),
        "runs": 3,
        "keep": False,
        "host_settings": "devtools",
        "ide_patterns": (),
        "isolate": (),
        "cache_state": "warm-no-op",
    } | overrides
    return BuildSettings.of(**arguments)


def test_settings_refuse_runs_below_one():
    with pytest.raises(InvalidSettingsError, match="--runs must be at least 1"):
        _settings(runs=0)


def test_settings_refuse_an_unknown_label_listing_the_known_ones():
    with pytest.raises(
        InvalidSettingsError,
        match=(
            r"unknown --host-settings 'x'; known: none, devtools, "
            r"devtools\+ide-exclusion"
        ),
    ):
        _settings(host_settings="x")


def test_settings_accept_the_unspecified_default():
    assert (
        _settings(host_settings="unspecified").host_settings
        is HostSettings.UNSPECIFIED
    )


def test_settings_track_the_default_processes():
    assert _settings().tracked == DEFAULT_TRACKED_PROCESSES


def test_an_ide_pattern_replaces_only_the_ide():
    tracked = _settings(ide_patterns=["/MacOS/clion"]).tracked
    assert [each.patterns for each in tracked if each.name == IDE] == [
        ("/MacOS/clion",)
    ]
    assert [each for each in tracked if each.name != IDE] == [
        each for each in DEFAULT_TRACKED_PROCESSES if each.name != IDE
    ]


def test_settings_refuse_an_ide_pattern_that_is_not_a_regex():
    with pytest.raises(InvalidSettingsError, match=r"'\['.*unterminated"):
        _settings(ide_patterns=["["])


def test_a_full_build_runs_default_once_reporting_its_closure():
    [invocation] = FullBuild().invocations(_GRAPH)
    assert invocation.argv == (
        "mise",
        "run",
        "--output",
        "prefix",
        "--continue-on-error",
        "default",
    )
    assert invocation.log_suffix == ""
    assert invocation.graph is _GRAPH
    assert invocation.reported == ("check", "test")


def test_isolation_runs_each_named_task_alone_with_skip_deps_in_order():
    invocations = IsolatedTasks(("test", "check")).invocations(_GRAPH)
    assert [each.argv for each in invocations] == [
        ("mise", "run", "--output", "prefix", "--skip-deps", "test"),
        ("mise", "run", "--output", "prefix", "--skip-deps", "check"),
    ]
    assert [each.reported for each in invocations] == [("test",), ("check",)]


def test_an_isolated_task_logs_under_its_name_with_colons_as_underscores():
    [invocation] = IsolatedTasks(("test:unit:cli",)).invocations(_GRAPH)
    assert invocation.log_suffix == "-test_unit_cli"
    assert log_name(TimedRun(2), invocation.log_suffix) == (
        "run-2-test_unit_cli.log"
    )
    assert log_name(Priming(), invocation.log_suffix) == (
        "priming-test_unit_cli.log"
    )


def test_an_isolated_task_is_placed_in_a_graph_where_it_has_no_dependencies():
    [invocation] = IsolatedTasks(("test",)).invocations(_GRAPH)
    assert invocation.graph.dependencies_of("test") == ()


def test_isolated_tasks_are_recorded_as_isolated_with_their_names():
    scope = IsolatedTasks(("test", "check"))
    assert scope.mode is Mode.ISOLATED
    assert scope.isolated_tasks == ("test", "check")


def test_settings_isolate_the_named_tasks():
    assert _settings(isolate=["test", "check"]).scope == IsolatedTasks(
        ("test", "check")
    )


def test_settings_without_isolated_tasks_run_a_full_build():
    assert _settings(isolate=[]).scope == FullBuild()


def test_settings_refuse_a_task_isolated_twice():
    with pytest.raises(
        InvalidSettingsError, match="--isolate names 'test' more than once"
    ):
        _settings(isolate=["test", "check", "test"])


def test_settings_measure_a_warm_no_op_build_by_default():
    assert _settings().cache_state is BuildCacheState.WARM_NO_OP


@pytest.mark.parametrize("state", list(BuildCacheState))
def test_settings_take_each_cache_state_by_its_flag_value(state):
    assert _settings(cache_state=str(state)).cache_state is state


def test_settings_refuse_an_unknown_cache_state_listing_the_known_ones():
    with pytest.raises(
        InvalidSettingsError,
        match=(
            r"^unknown --cache-state 'tepid'; known: warm-no-op, "
            r"incremental, cold$"
        ),
    ):
        _settings(cache_state="tepid")


def test_settings_refuse_isolate_with_cold():
    with pytest.raises(
        IncompatibleSettingsError,
        match=r"^--isolate does not support --cache-state cold$",
    ):
        _settings(isolate=["test"], cache_state="cold")


@pytest.mark.parametrize("state", ["warm-no-op", "incremental"])
def test_settings_isolate_under_a_state_that_deletes_nothing(state):
    assert _settings(isolate=["test"], cache_state=state).scope == (
        IsolatedTasks(("test",))
    )


def test_a_full_build_is_recorded_as_full_with_no_isolated_tasks():
    assert FullBuild.mode is Mode.FULL
    assert FullBuild.isolated_tasks == ()


@pytest.mark.parametrize(
    ("run", "suffix", "name"),
    [
        (Priming(), "", "priming.log"),
        (TimedRun(2), "", "run-2.log"),
        (TimedRun(2), "-iostat", "run-2-iostat.log"),
        (TimedRun(2), "-ps", "run-2-ps.log"),
    ],
)
def test_each_log_is_named_for_its_run(run, suffix, name):
    assert log_name(run, suffix) == name
