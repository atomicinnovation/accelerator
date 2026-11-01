import json

import pytest

from tasks.shared.build_measurement.errors import TaskGraphError
from tasks.shared.build_measurement.task_graph import TaskGraph


def _mise_tasks(*entries: dict) -> str:
    return json.dumps(
        [{"depends": [], "run": ["invoke x"], **entry} for entry in entries]
    )


_GRAPH = TaskGraph.parse(
    _mise_tasks(
        {"name": "default", "depends": ["check", "test"], "run": []},
        {"name": "check", "depends": ["lint:check"], "run": []},
        {"name": "lint:check", "depends": ["deps:install"]},
        {"name": "test", "depends": ["deps:install"], "run": []},
        {"name": "deps:install"},
    )
)


def test_depends_are_read_from_mise_tasks_json():
    assert _GRAPH.dependencies_of("default") == ("check", "test")


def test_the_closure_of_a_root_excludes_the_root():
    assert "default" not in _GRAPH.closure("default")


def test_the_closure_follows_depends_transitively():
    assert _GRAPH.closure("default") == {
        "check",
        "lint:check",
        "test",
        "deps:install",
    }


def test_a_task_missing_from_the_graph_has_no_dependencies():
    assert _GRAPH.dependencies_of("absent") == ()


def test_unreadable_json_is_refused_naming_mise_tasks():
    with pytest.raises(TaskGraphError, match="mise tasks --json"):
        TaskGraph.parse("not json")


@pytest.mark.parametrize(
    "document",
    [
        "{}",
        '["default"]',
        '[{"depends": [], "run": []}]',
    ],
)
def test_a_document_that_is_not_a_list_of_named_tasks_is_refused(document):
    with pytest.raises(TaskGraphError, match="mise tasks --json"):
        TaskGraph.parse(document)


def test_a_non_list_depends_is_refused():
    with pytest.raises(TaskGraphError, match="mise tasks --json"):
        TaskGraph.parse(_mise_tasks({"name": "a", "depends": "b"}))


def test_a_non_string_dependency_is_refused():
    with pytest.raises(TaskGraphError, match="mise tasks --json"):
        TaskGraph.parse(_mise_tasks({"name": "a", "depends": [1]}))


def test_a_task_with_a_run_is_runnable():
    assert _GRAPH.is_runnable("deps:install")


def test_a_roll_up_with_an_empty_run_is_not_runnable():
    assert not _GRAPH.is_runnable("test")


def test_a_non_list_run_is_refused():
    with pytest.raises(TaskGraphError, match="mise tasks --json"):
        TaskGraph.parse(_mise_tasks({"name": "a", "run": "invoke a"}))
