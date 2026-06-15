from invoke import Context, Exit, task

from tasks.shared.paths import A9R_CARGO_TOML


@task
def check(context: Context) -> None:
    """Lint a9r + a9r-core with clippy (-D warnings).

    A single default-feature pass: the default `a9r` build uses the
    visualiser's `dev-frontend` feature, so no embedded SPA assets and no
    `frontend/dist` stub are required (unlike `lint:server:check`, which has
    a second `--all-features` pass for the embed-dist arms). `-p` scopes the
    lint to the a9r packages.
    """
    result = context.run(
        f"cargo clippy --manifest-path {A9R_CARGO_TOML} "
        f"--package a9r --package a9r-core --all-targets -- -D warnings",
        warn=True,
        pty=False,
    )
    if result.exited != 0:
        raise Exit("clippy reported findings in a9r/a9r-core", code=1)


@task
def fix(context: Context) -> None:
    """Apply clippy's machine-applicable fixes to a9r + a9r-core.

    Machine-applicable only; --allow-dirty so it runs on an uncommitted tree
    (VCS revert is the recovery path).
    """
    context.run(
        f"cargo clippy --fix --allow-dirty --manifest-path {A9R_CARGO_TOML} "
        f"--package a9r --package a9r-core --all-targets",
        warn=True,
        pty=False,
    )
