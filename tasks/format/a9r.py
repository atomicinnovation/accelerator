from invoke import Context, Exit, task

from tasks.shared.paths import A9R_CARGO_TOML

# Scope to the a9r packages (not --all) so this component's format task does
# not also reformat the server crate — that is format:server's remit.
_PACKAGES = "--package a9r --package a9r-core"


@task
def check(context: Context) -> None:
    """Check a9r Rust formatting with rustfmt (cargo fmt --check)."""
    result = context.run(
        f"cargo fmt --manifest-path {A9R_CARGO_TOML} {_PACKAGES} -- --check",
        warn=True,
        pty=False,
    )
    if result.exited != 0:
        raise Exit(
            "cargo fmt: drift detected — run `mise run format:a9r:fix`",
            code=1,
        )


@task
def fix(context: Context) -> None:
    """Format a9r Rust in place (cargo fmt)."""
    context.run(
        f"cargo fmt --manifest-path {A9R_CARGO_TOML} {_PACKAGES}",
        warn=True,
        pty=False,
    )
