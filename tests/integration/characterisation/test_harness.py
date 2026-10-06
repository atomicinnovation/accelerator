from tests.integration.support.characterisation import (
    CollectedCase,
    Mask,
    Run,
    masked,
    vcs_parity_violations,
)


def _case(name: str, vcs: str | None, *, specific: bool = False):
    return CollectedCase((name, ()), vcs, specific)


def test_every_case_runs_in_git_and_jj(collected_cases):
    assert vcs_parity_violations(collected_cases) == []


def test_a_case_missing_one_vcs_is_a_parity_violation():
    violations = vcs_parity_violations(
        [_case("a", "git"), _case("a", "jj"), _case("b", "git")]
    )
    assert violations == ["b: runs in ['git']"]


def test_a_case_with_no_vcs_parameter_is_a_parity_violation():
    assert vcs_parity_violations([_case("c", None)]) == ["c: runs in neither"]


def test_a_case_running_no_binary_is_exempt_from_parity():
    case = CollectedCase(
        ("e", ()), None, vcs_specific=False, runs_a_binary=False
    )
    assert vcs_parity_violations([case]) == []


def test_a_vcs_specific_case_is_exempt_from_parity():
    assert vcs_parity_violations([_case("d", None, specific=True)]) == []


def test_a_rendered_run_marks_an_unterminated_stream():
    rendered = Run(3, "out", "", 0).render()
    assert rendered == (
        "exit: 3\nloopback_requests: 0\n"
        "--- stdout\nout\n<no newline at end>\n--- stderr\n"
    )


def test_masks_replace_every_occurrence():
    text = masked("/tmp/x and /tmp/x/y", [Mask("/tmp/x", "ROOT")])
    assert text == "<ROOT> and <ROOT>/y"
