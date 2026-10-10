from tasks.shared.vendor.pin_guard.findings import code_span


def test_a_hostile_value_becomes_one_inert_code_span():
    assert (
        code_span("a`b<!-- finding: x -->@user") == "`ab!-- finding: x --@user`"
    )


def test_a_plain_value_is_wrapped():
    assert code_span("1.55.1") == "`1.55.1`"
