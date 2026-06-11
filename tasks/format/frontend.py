from invoke import Context, Exit, task

from tasks.shared.paths import FRONTEND


@task
def check(context: Context) -> None:
    """Check frontend formatting with Biome (read-only; fails on drift)."""
    result = context.run(
        f"npm --prefix {FRONTEND} run format:check", warn=True, pty=False
    )
    if result.exited != 0:
        raise Exit(
            "biome format: drift — run `mise run format:frontend:fix`", code=1
        )


@task
def fix(context: Context) -> None:
    """Format the frontend in place with Biome."""
    context.run(f"npm --prefix {FRONTEND} run format:fix", warn=True, pty=False)
