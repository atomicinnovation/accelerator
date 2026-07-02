from invoke import Context, Exit, task

from tasks.shared.paths import CLI_DIR


@task
def check(context: Context) -> None:
    """Check the cli/ workspace dependency graph with cargo-deny.

    Runs from cli/ because cargo-deny resolves deny.toml relative to cwd —
    unlike the --manifest-path form the fmt/clippy leaves use.
    """
    with context.cd(str(CLI_DIR)):
        result = context.run(
            "cargo deny check advisories licenses bans sources",
            warn=True,
            pty=False,
        )
    if result.exited != 0:
        raise Exit("cargo-deny reported findings — see output", code=1)
