from collections.abc import Mapping

from invoke import Context, Exit, UnexpectedExit

from tasks.shared.selection import Selection

PYTEST_FILTER_HELP = (
    "pytest -k expression selecting the tests to run, "
    "e.g. 'resume and not slow'"
)

_PYTEST_NO_TESTS_COLLECTED = 5


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
