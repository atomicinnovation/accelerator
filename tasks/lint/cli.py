from invoke import Context, Exit, task

from tasks.shared.paths import CLI_WORKSPACE_CARGO_TOML

_BASE = (
    f"cargo clippy --manifest-path {CLI_WORKSPACE_CARGO_TOML} "
    "--workspace --all-targets --all-features"
)


@task
def check(context: Context) -> None:
    """Lint the cli/ workspace with clippy (workspace-wide, -D warnings)."""
    result = context.run(f"{_BASE} -- -D warnings", warn=True, pty=False)
    if result.exited != 0:
        raise Exit(
            "clippy reported findings — run `mise run lint:cli:fix`", code=1
        )


@task
def fix(context: Context) -> None:
    """Apply clippy's machine-applicable fixes to the cli/ workspace.

    Only the machine-rewritable subset is applied; lints such as `unwrap_used`
    cannot be auto-fixed, so `mise run lint:cli:check` must still be run for the
    remainder. --allow-dirty so it runs on an uncommitted tree (VCS revert is
    the recovery path).
    """
    context.run(
        f"{_BASE} --fix --allow-dirty --allow-staged", warn=True, pty=False
    )
