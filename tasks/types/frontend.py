from invoke import Context, Exit, task

from tasks.shared.paths import FRONTEND


@task
def check(context: Context) -> None:
    """Type-check the frontend with tsc (tsc -b --noEmit)."""
    result = context.run(
        f"npm --prefix {FRONTEND} run typecheck", warn=True, pty=False
    )
    if result.exited != 0:
        raise Exit("tsc -b --noEmit reported type errors", code=1)
