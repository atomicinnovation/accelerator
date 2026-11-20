import re
from collections.abc import Generator, Mapping, Sequence
from contextlib import contextmanager

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

LIBTEST_FILTER_HELP = (
    "libtest substring of the test path selecting the tests to run, "
    "e.g. 'path_normalisation'"
)

_PYTEST_NO_TESTS_COLLECTED = 5
_NEXTEST_NO_TESTS_RUN = 4

_LIBTEST_RESULT = re.compile(
    r"^test result: \w+\. (?P<passed>\d+) passed; (?P<failed>\d+) failed; "
    r"(?P<ignored>\d+) ignored;",
    re.MULTILINE,
)


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


def command_line(*parts: str) -> str:
    return " ".join(filter(None, parts))


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
        int(result["passed"]) + int(result["failed"]) + int(result["ignored"])
        for result in _LIBTEST_RESULT.finditer(output)
    )
