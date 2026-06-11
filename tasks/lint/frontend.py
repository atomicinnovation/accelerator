from invoke import Context, Exit, task

from tasks.shared.paths import FRONTEND


@task
def check(context: Context) -> None:
    """Lint the frontend with Biome (react+test+project, warnings-as-errors)."""
    result = context.run(
        f"npm --prefix {FRONTEND} run lint", warn=True, pty=False
    )
    if result.exited != 0:
        raise Exit(
            "biome reported findings — run `mise run lint:frontend:fix` for "
            "the safe-fixable subset, then fix the rest",
            code=1,
        )


@task
def fix(context: Context) -> None:
    """Apply Biome's safe lint fixes and import organisation."""
    context.run(f"npm --prefix {FRONTEND} run lint:fix", warn=True, pty=False)
