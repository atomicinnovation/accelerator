from tasks.shared.vendor.pin_guard.findings import code_span


def test_a_hostile_value_becomes_one_inert_code_span():
    assert (
        code_span("a`b<!-- finding: x -->@user") == "`ab!-- finding: x --@user`"
    )


def test_a_plain_value_is_wrapped():
    assert code_span("1.55.1") == "`1.55.1`"


def test_line_breaks_cannot_end_the_span():
    assert (
        code_span("x\n\n@mallory\r\n\t[a](https://evil.example)\x00")
        == "`x @mallory [a](https://evil.example) `"
    )


def test_a_value_at_the_length_cap_is_kept_whole():
    assert code_span("a" * 200) == f"`{'a' * 200}`"


def test_a_value_past_the_length_cap_is_cut_visibly():
    assert code_span("a" * 1000) == f"`{'a' * 199}…`"
