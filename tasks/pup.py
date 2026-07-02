from invoke import Context, Exit, task

from tasks.shared.paths import CLI_DIR
from tasks.shared.rust import PUP_NIGHTLY, pup_mode


@task
def check(context: Context) -> None:
    """Enforce intra-crate module-import rules with cargo-pup (nightly lane).

    Provisioning is guaranteed by the mise `depends` edge on deps:install:pup,
    so the body just runs the tool. Runs from cli/ because pup resolves pup.ron
    relative to cwd. ACCELERATOR_PUP_MODE=warn downgrades a findings failure to
    advisory (local-only; CI always runs the fail-closed deny default).
    """
    with context.cd(str(CLI_DIR)):
        result = context.run(f"cargo +{PUP_NIGHTLY} pup", warn=True, pty=False)
    if result.exited != 0:
        if pup_mode() == "warn":
            print(
                "WARNING: cargo-pup reported findings (advisory mode, "
                "ACCELERATOR_PUP_MODE=warn — NOT blocking); see output above"
            )
            return
        raise Exit("cargo-pup: module-import rule violation", code=1)
