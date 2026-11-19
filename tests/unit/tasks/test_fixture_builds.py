"""Guards the routes to a fixture build the compiler cannot see.

The compiler stops tests calling the shared families' builders directly. An
explicit ``Workbench::with`` can still hide a test's builds from the run's
fixture log, and a direct ``copy_part`` can copy a shared tree without the
log recording it.
"""

from pathlib import Path

from tasks.shared.paths import REPO_ROOT

TESTS = Path("cli/vcs-test-support/tests")

GUARDED_CALLS = {
    "Workbench::with(": (
        "can build a shared family outside the run's fixture log; use "
        "Workbench::for_this_test()",
        frozenset(
            {
                TESTS / "workbench.rs",
                TESTS / "matrix_private_build.rs",
                TESTS / "status_log_parts.rs",
            }
        ),
    ),
    "copy_part(": (
        "copies a shared tree without logging the copy; use "
        "Workbench::writable_status_log_states()",
        frozenset({TESTS / "copies.rs", TESTS / "copy_fidelity.rs"}),
    ),
}


def violations(root: Path) -> list[str]:
    return [
        f"{relative}:{number}: {call} {reason}"
        for path in sorted(root.glob("cli/*/tests/**/*.rs"))
        for relative in (path.relative_to(root),)
        for number, line in enumerate(path.read_text().splitlines(), 1)
        for call, (reason, permitted) in GUARDED_CALLS.items()
        if call in line and relative not in permitted
    ]


def _test_source(root: Path, relative: str, body: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)


def test_an_explicit_handover_in_a_permitted_file_passes(
    tmp_path: Path,
) -> None:
    _test_source(
        tmp_path,
        "cli/vcs-test-support/tests/workbench.rs",
        "let workbench = Workbench::with(handover)?;\n",
    )
    assert violations(tmp_path) == []


def test_an_explicit_handover_elsewhere_is_named_by_file_and_line(
    tmp_path: Path,
) -> None:
    _test_source(
        tmp_path,
        "cli/vcs-adapters/tests/classify.rs",
        "fn a() {\n    let workbench = Workbench::with(handover)?;\n}\n",
    )
    assert violations(tmp_path) == [
        "cli/vcs-adapters/tests/classify.rs:2: Workbench::with( can build a"
        " shared family outside the run's fixture log; use"
        " Workbench::for_this_test()"
    ]


def test_opening_a_workbench_for_this_test_passes(tmp_path: Path) -> None:
    _test_source(
        tmp_path,
        "cli/vcs-adapters/tests/classify.rs",
        "let workbench = Workbench::for_this_test()?;\n",
    )
    assert violations(tmp_path) == []


def test_a_direct_copy_in_a_permitted_file_passes(tmp_path: Path) -> None:
    _test_source(
        tmp_path,
        "cli/vcs-test-support/tests/copy_fidelity.rs",
        "let copy = copy_part(&base, &names, target.path())?;\n",
    )
    assert violations(tmp_path) == []


def test_a_direct_copy_elsewhere_is_named_by_file_and_line(
    tmp_path: Path,
) -> None:
    _test_source(
        tmp_path,
        "cli/vcs-cli/tests/status_log_goldens.rs",
        "let copy = copy_part(&base, &names, work)?;\n",
    )
    assert violations(tmp_path) == [
        "cli/vcs-cli/tests/status_log_goldens.rs:1: copy_part( copies a"
        " shared tree without logging the copy; use"
        " Workbench::writable_status_log_states()"
    ]


def test_no_test_hides_a_fixture_build_from_the_log() -> None:
    assert violations(REPO_ROOT) == []
