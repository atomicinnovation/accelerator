from invoke import Context, Exit, task

from tasks.shared.paths import CARGO_TOML


@task
def check(context: Context) -> None:
    """Check Rust formatting with rustfmt (cargo fmt --check)."""
    result = context.run(
        f"cargo fmt --manifest-path {CARGO_TOML} --all -- --check",
        warn=True,
        pty=False,
    )
    if result.exited != 0:
        raise Exit(
            "cargo fmt: drift detected — run `mise run format:server:fix`",
            code=1,
        )


@task
def fix(context: Context) -> None:
    """Format Rust in place (cargo fmt)."""
    context.run(
        f"cargo fmt --manifest-path {CARGO_TOML} --all", warn=True, pty=False
    )
