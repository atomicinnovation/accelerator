import json
from pathlib import Path

import pytest

from tasks.shared.build_measurement.errors import InvalidSettingsError
from tasks.shared.build_measurement.measured_build import HostSettings, Mode
from tasks.shared.build_measurement.settings import (
    BuildSettings,
    FullBuild,
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
            {"name": "test", "depends": [], "run": ["invoke test"]},
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
