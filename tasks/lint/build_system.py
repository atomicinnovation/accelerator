# invoke's auto_dash_names maps the `build_system` module to the `build-system`
# namespace, so `invoke lint.build-system.check` resolves here. build-system =
# the repo-root Python automation toolchain (tasks/ + tests), not build:*.
from invoke import Context, Exit, task

from tasks.shared.paths import REPO_ROOT


@task
def check(context: Context) -> None:
    """Lint Python with ruff (config in pyproject.toml; select = ALL)."""
    with context.cd(str(REPO_ROOT)):
        result = context.run("uv run ruff check", warn=True, pty=False)
    if result.exited != 0:
        raise Exit(
            "ruff reported findings — run `mise run lint:build-system:fix` "
            "for the auto-fixable subset, then fix the rest",
            code=1,
        )


@task
def fix(context: Context) -> None:
    """Apply ruff's safe lint fixes (safe-only; never --unsafe-fixes)."""
    with context.cd(str(REPO_ROOT)):
        context.run("uv run ruff check --fix", warn=True, pty=False)
