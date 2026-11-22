import re
from collections.abc import Generator, Mapping, Sequence
from contextlib import contextmanager
from pathlib import Path

from invoke import Context, Exit, UnexpectedExit

from tasks.shared.selection import Selection

PYTEST_FILTER_HELP = (
    "pytest -k expression selecting the tests to run, "
    "e.g. 'resume and not slow'"
)

NEXTEST_CONTRACT_FILTER_HELP = (
    "nextest filterset selecting the contract tests to run, e.g. "
    "'test(the_conformance_set_passes_against_a_live_team)'"
)

VITEST_FILTER_HELP = (
    "Vitest -t test-name regex selecting the tests to run, e.g. 'copyText'"
)

NODE_TEST_UNIT_FILTER_HELP = (
    "node --test-name-pattern regex selecting the tests to run, "
    "e.g. 'judges as the corpus says'"
)

NODE_TEST_RUNTIME_FILTER_HELP = (
    "node --test-name-pattern regex selecting the tests to run, "
    "e.g. 'ping: exits 0'"
)

PLAYWRIGHT_HOST_FILTER_HELP = (
    "Playwright --grep regex selecting the tests to run, "
    "e.g. 'kanban renders all seven default columns'"
)

PLAYWRIGHT_DOCKER_FILTER_HELP = (
    "Playwright --grep regex selecting the tests to run, e.g. 'chip-showcase'"
)

LIBTEST_UNIT_FILTER_HELP = (
    "libtest substring of the test path selecting the tests to run, "
    "e.g. 'path_normalisation'"
)

LIBTEST_INTEGRATION_FILTER_HELP = (
    "libtest substring of the test path selecting the tests to run, "
    "e.g. 'spa_route'"
)

_PYTEST_NO_TESTS_COLLECTED = 5
_NEXTEST_NO_TESTS_RUN = 4
_PLAYWRIGHT_NO_TESTS_FOUND = "No tests found"

_LIBTEST_RESULT = re.compile(
    r"^test result: \w+\. (?P<passed>\d+) passed; (?P<failed>\d+) failed;",
    re.MULTILINE,
)

_VITEST_TESTS_SUMMARY = re.compile(r"^\s*Tests\s+(?P<counts>.*)$", re.MULTILINE)
_VITEST_TESTS_RUN = re.compile(r"(?P<count>\d+) (?:passed|failed)\b")
_TAP_TOP_LEVEL_POINT = re.compile(
    r"^(?:not )?ok \d+ - (?P<name>.*?)(?P<directive> # (?i:SKIP|TODO)\b.*)?$",
    re.MULTILINE,
)
_ANSI_ESCAPE = re.compile(r"\x1b\[[0-9;]*m")


def selection_from(filter_: str | None) -> Selection:
    try:
        return Selection(filter_)
    except ValueError as error:
        raise Exit(str(error), code=1) from None


def run_pytest_lane(
    context: Context,
    command: str,
    *,
    mise_task: str,
    filter_: str | None,
    env: Mapping[str, str] | None = None,
) -> None:
    selection = selection_from(filter_)
    with nothing_selected_on_exit(
        _PYTEST_NO_TESTS_COLLECTED, selection, mise_task
    ):
        context.run(
            command_line(command, selection.pytest()), **_environment(env)
        )


def run_nextest_lane(
    context: Context,
    command: str,
    *,
    mise_task: str,
    filter_: str | None,
    env: Mapping[str, str],
) -> None:
    selection = selection_from(filter_)
    with nothing_selected_on_exit(_NEXTEST_NO_TESTS_RUN, selection, mise_task):
        context.run(
            command_line(command, selection.nextest()), env=dict(env), pty=True
        )


@contextmanager
def nothing_selected_on_exit(
    exit_code: int, selection: Selection, mise_task: str
) -> Generator[None]:
    try:
        yield
    except UnexpectedExit as failure:
        if selection.is_filtered and failure.result.exited == exit_code:
            raise Exit(selection.nothing_selected(mise_task), code=1) from None
        raise


@contextmanager
def nothing_selected_when_playwright_finds_no_tests(
    selection: Selection, mise_task: str
) -> Generator[None]:
    try:
        yield
    except UnexpectedExit as failure:
        output = f"{failure.result.stdout}{failure.result.stderr}"
        if selection.is_filtered and _PLAYWRIGHT_NO_TESTS_FOUND in output:
            raise Exit(selection.nothing_selected(mise_task), code=1) from None
        raise


def command_line(*parts: str) -> str:
    return " ".join(filter(None, parts))


def npm_script_options(options: str) -> str:
    return options and f"-- {options}"


def _environment(
    env: Mapping[str, str] | None,
) -> dict[str, dict[str, str]]:
    return {} if env is None else {"env": dict(env)}


def run_libtest_lane(
    context: Context,
    commands: Sequence[str],
    *,
    mise_task: str,
    filter_: str | None,
    env: Mapping[str, str] | None = None,
) -> None:
    selection = selection_from(filter_)
    tests_run = 0
    for command in commands:
        result = context.run(
            command_line(command, selection.libtest()), **_environment(env)
        )
        tests_run += libtest_tests_run(result.stdout)
    if selection.is_filtered and tests_run == 0:
        raise Exit(selection.nothing_selected(mise_task), code=1)


def libtest_tests_run(output: str) -> int:
    return sum(
        int(result["passed"]) + int(result["failed"])
        for result in _LIBTEST_RESULT.finditer(output)
    )


def vitest_tests_run(output: str) -> int:
    summary = _VITEST_TESTS_SUMMARY.search(_ANSI_ESCAPE.sub("", output))
    if summary is None:
        raise Exit("Vitest emitted no 'Tests' summary line", code=1)
    return sum(
        int(run["count"])
        for run in _VITEST_TESTS_RUN.finditer(summary["counts"])
    )


def node_tests_run(tap: str, suites: Sequence[Path]) -> int:
    """Count the tests a `node --test` run reported.

    Node reports a suite file in which no test ran as a passing test named
    after the file, so those points are not tests. Nor are skipped or todo
    points, since neither can fail the run.
    """
    suite_names = {str(suite) for suite in suites}
    return sum(
        1
        for point in _TAP_TOP_LEVEL_POINT.finditer(tap)
        if point["name"] not in suite_names and point["directive"] is None
    )
