# invoke's auto_dash_names maps the `build_system` module to the `build-system`
# namespace, so `invoke types.build-system.check` resolves here. build-system =
# the repo-root Python automation toolchain (tasks/ + tests), not build:*.
from invoke import Context, Exit, task

from tasks.shared.paths import REPO_ROOT


@task
def check(context: Context) -> None:
    """Type-check Python with pyrefly (strict preset)."""
    with context.cd(str(REPO_ROOT)):
        # --output-format github emits inline annotations on the CI runner and
        # is inert (plain text) locally. No autofixer to name on failure.
        result = context.run(
            "pyrefly check --output-format github", warn=True, pty=False
        )
    if result.exited != 0:
        raise Exit("pyrefly reported type errors", code=1)
