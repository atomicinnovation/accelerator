import shlex

import pytest

from tasks.shared.selection import Selection


def test_no_expression_is_unfiltered_and_renders_nothing():
    selection = Selection(None)

    assert not selection.is_filtered
    assert selection.nextest() == ""


def test_an_expression_is_filtered():
    assert Selection("test(a)").is_filtered


def test_nextest_renders_the_expression_as_one_quoted_option():
    assert (
        Selection("test(a b) & binary(=x)").nextest()
        == "-E 'test(a b) & binary(=x)'"
    )


def test_a_plain_word_is_quoted_too():
    assert Selection("x").nextest() == "-E 'x'"


@pytest.mark.parametrize(
    "expression", ["test(a b) & binary(=x)", "test(it's) | test(=a&b)"]
)
def test_the_rendering_splits_back_into_the_option_and_the_expression(
    expression: str,
):
    assert shlex.split(Selection(expression).nextest()) == ["-E", expression]


@pytest.mark.parametrize("expression", ["", "  "])
def test_an_empty_expression_is_refused(expression: str):
    with pytest.raises(ValueError, match=r"^--filter needs an expression$"):
        Selection(expression)


def test_the_flag_is_the_option_as_the_developer_gave_it():
    assert Selection("x").flag == "--filter 'x'"


def test_the_flag_splits_back_into_the_option_and_the_expression():
    assert shlex.split(Selection("test(it's)").flag) == [
        "--filter",
        "test(it's)",
    ]


def test_nothing_selected_names_the_filter_and_where_its_syntax_is():
    assert Selection("x").nothing_selected("test:unit:tasks") == (
        "--filter 'x' selected no tests; "
        "see mise run test:unit:tasks --help for its syntax"
    )


def test_nothing_selected_quotes_an_expression_holding_a_quote():
    message = Selection("test(it's)").nothing_selected("test:unit:cli")

    assert shlex.split(message)[:2] == ["--filter", "test(it's)"]


def test_pytest_renders_the_expression_as_one_quoted_k_option():
    assert Selection("a and not b").pytest() == "-k 'a and not b'"


def test_pytest_renders_nothing_unfiltered():
    assert Selection(None).pytest() == ""


def test_the_pytest_rendering_splits_back_into_the_option_and_the_expression():
    assert shlex.split(Selection("it's a").pytest()) == ["-k", "it's a"]
