import pytest
from invoke import Exit

from tasks.shared.selection import Selection
from tasks.test.filters import selection_from


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
