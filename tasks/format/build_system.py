# invoke's auto_dash_names maps the `build_system` module to the `build-system`
# namespace, so `invoke format.build-system.check` resolves here. build-system =
# the repo-root Python automation toolchain (tasks/ + tests), not build:*.
from invoke import Context, Exit, task

from tasks.shared.paths import REPO_ROOT


@task
def check(context: Context) -> None:
    """Check Python formatting with ruff (read-only; fails on drift)."""
    with context.cd(str(REPO_ROOT)):
        result = context.run("ruff format --check", warn=True, pty=False)
    if result.exited != 0:
        raise Exit(
            "ruff format: drift — run `mise run format:build-system:fix`",
            code=1,
        )


@task
def fix(context: Context) -> None:
    """Format Python in place with ruff."""
    with context.cd(str(REPO_ROOT)):
        context.run("ruff format", warn=True, pty=False)
