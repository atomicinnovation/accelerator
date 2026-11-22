from pathlib import Path

import pytest
from invoke import Exit

from tasks.shared.selection import Selection
from tasks.test.filters import node_tests_run, selection_from


def test_no_filter_is_an_unfiltered_selection():
    assert selection_from(None) == Selection(None)


def test_a_filter_is_a_selection_of_it():
    assert selection_from("x") == Selection("x")


@pytest.mark.parametrize("filter_", ["", "  "])
def test_an_empty_filter_is_one_exit_line(filter_: str):
    with pytest.raises(Exit) as exc_info:
        selection_from(filter_)

    assert str(exc_info.value) == "--filter needs an expression"
    assert exc_info.value.__cause__ is None
    assert exc_info.value.__suppress_context__


_SUITE = Path("lib/daemon.test.js")


def test_a_node_run_counts_top_level_tests_not_their_subtests():
    tap = (
        "TAP version 13\n"
        "# Subtest: a describe block\n"
        "    ok 1 - a nested test\n"
        "    ok 2 - another nested test\n"
        "ok 1 - a describe block\n"
        "ok 2 - a top-level test\n"
    )

    assert node_tests_run(tap, [_SUITE]) == 2


def test_a_suite_file_that_failed_to_load_is_not_a_test_run():
    tap = f"TAP version 13\nnot ok 1 - {_SUITE}\n"

    assert node_tests_run(tap, [_SUITE]) == 0


def test_a_failing_top_level_test_counts_as_run():
    tap = "TAP version 13\nnot ok 1 - a failing test\n"

    assert node_tests_run(tap, [_SUITE]) == 1
