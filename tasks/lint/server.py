from invoke import Context, Exit, task

from tasks.shared.paths import CARGO_TOML


@task
def check(context: Context) -> None:
    """Lint Rust with clippy across both feature configs (-D warnings).

    Runs both passes before raising (collect-all-failures) so a developer sees
    the full diagnostic set in one run. `--all-features` enables dev-frontend;
    the default-feature pass covers the cfg(not(dev-frontend))/embed-dist arms
    in src/assets.rs that --all-features compiles out.
    """
    base = f"cargo clippy --manifest-path {CARGO_TOML} --all-targets"
    passes = {
        "all-features": f"{base} --all-features -- -D warnings",
        "default": f"{base} -- -D warnings",
    }
    failed = [
        name
        for name, cmd in passes.items()
        if context.run(cmd, warn=True, pty=False).exited != 0
    ]
    if failed:
        raise Exit(
            f"clippy reported findings under feature config(s): "
            f"{', '.join(failed)}",
            code=1,
        )


@task
def fix(context: Context) -> None:
    """Apply clippy's machine-applicable fixes (default features only).

    Machine-applicable only (no --unsafe); --allow-dirty so it runs on an
    uncommitted tree (VCS revert is the recovery path). The all-features /
    dev-frontend arms surface via `server:check` and may need manual fixes.
    """
    context.run(
        f"cargo clippy --fix --allow-dirty --manifest-path {CARGO_TOML} "
        f"--all-targets",
        warn=True,
        pty=False,
    )
