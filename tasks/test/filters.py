import re
from collections.abc import Mapping, Sequence

from invoke import Context, Exit, UnexpectedExit

from tasks.shared.selection import Selection

PYTEST_FILTER_HELP = (
    "pytest -k expression selecting the tests to run, "
    "e.g. 'resume and not slow'"
)

LIBTEST_FILTER_HELP = (
    "libtest substring of the test path selecting the tests to run, "
    "e.g. 'path_normalisation'"
)

_PYTEST_NO_TESTS_COLLECTED = 5

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
    command_line = " ".join(filter(None, [command, selection.pytest()]))
    environment: dict[str, dict[str, str]] = (
        {} if env is None else {"env": dict(env)}
    )
    try:
        context.run(command_line, **environment)
    except UnexpectedExit as failure:
        if (
            selection.is_filtered
            and failure.result.exited == _PYTEST_NO_TESTS_COLLECTED
        ):
            raise Exit(selection.nothing_selected(mise_task), code=1) from None
        raise


def run_libtest_lane(
    context: Context,
    commands: Sequence[str],
    *,
    mise_task: str,
    filter_: str | None,
    env: Mapping[str, str] | None = None,
) -> None:
    selection = selection_from(filter_)
    environment: dict[str, dict[str, str]] = (
        {} if env is None else {"env": dict(env)}
    )
    tests_run = 0
    for command in commands:
        result = context.run(
            " ".join(filter(None, [command, selection.libtest()])),
            **environment,
        )
        tests_run += libtest_tests_run(result.stdout)
    if selection.is_filtered and tests_run == 0:
        raise Exit(selection.nothing_selected(mise_task), code=1)


def libtest_tests_run(output: str) -> int:
    return sum(
        int(result["passed"]) + int(result["failed"]) + int(result["ignored"])
        for result in _LIBTEST_RESULT.finditer(output)
    )
