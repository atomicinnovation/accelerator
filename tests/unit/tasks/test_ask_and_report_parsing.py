import pytest

from tasks.eval.ask_and_report.parsing import one_of, only


def test_a_listed_value_is_taken_as_its_option() -> None:
    assert one_of("ask", ("ask", "report"), "verb") == "ask"


def test_an_unlisted_value_is_refused_by_name() -> None:
    with pytest.raises(ValueError, match="'tell' is not a verb"):
        one_of("tell", ("ask", "report"), "verb")


def test_only_the_declared_keys_are_allowed() -> None:
    only({"reply": "x"}, {"reply", "expect"}, "turn")

    with pytest.raises(ValueError, match=r"a turn cannot declare \['extra'\]"):
        only({"reply": "x", "extra": 1}, {"reply", "expect"}, "turn")
