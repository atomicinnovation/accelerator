"""Guards that no test opens a workbench whose hand-over hides its fixture
builds from the run's fixture log.

The compiler already stops tests calling the fixture builders directly; an
explicit ``Workbench::with`` is the one route it cannot see.
"""

from pathlib import Path

from tasks.shared.paths import REPO_ROOT

EXPLICIT_HANDOVER = "Workbench::with("

PERMITTED_FILES = frozenset({Path("cli/vcs-test-support/tests/workbench.rs")})


def violations(root: Path) -> list[str]:
    return [
        f"{relative}:{number}: {EXPLICIT_HANDOVER} can build a shared family"
        " outside the run's fixture log; use Workbench::for_this_test()"
        for path in sorted(root.glob("cli/*/tests/**/*.rs"))
        if (relative := path.relative_to(root)) not in PERMITTED_FILES
        for number, line in enumerate(path.read_text().splitlines(), 1)
        if EXPLICIT_HANDOVER in line
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


def test_no_test_hides_a_fixture_build_from_the_log() -> None:
    assert violations(REPO_ROOT) == []
