from invoke import Context, Exit, task

from tasks.shared.paths import CLI_WORKSPACE_CARGO_TOML

_MANIFEST = f"--manifest-path {CLI_WORKSPACE_CARGO_TOML}"


@task
def check(context: Context) -> None:
    """Check cli/ workspace formatting with rustfmt (fails on drift)."""
    result = context.run(
        f"cargo fmt {_MANIFEST} --all -- --check", warn=True, pty=False
    )
    if result.exited != 0:
        raise Exit(
            "rustfmt: drift detected — run `mise run format:cli:fix`", code=1
        )


@task
def fix(context: Context) -> None:
    """Format the cli/ workspace in place with rustfmt."""
    context.run(f"cargo fmt {_MANIFEST} --all", warn=True, pty=False)
