from invoke import Context, Exit, task

from tasks.shared.paths import CLI_WORKSPACE_CARGO_TOML
from tasks.shared.rust import coverage_enabled

_MANIFEST = f"--manifest-path {CLI_WORKSPACE_CARGO_TOML} --workspace"


@task
def run(context: Context) -> None:
    """Run cli/ workspace tests (instrumented with coverage unless disabled).

    Coverage is folded into the test run, not a separate task: the default
    instrumented `cargo llvm-cov nextest` pass both runs the tests and reports
    coverage in one go. It is report-only — no `--fail-under`/threshold, by
    design. ACCELERATOR_COVERAGE=off drops to plain nextest for a faster loop.
    """
    command = (
        f"cargo llvm-cov nextest {_MANIFEST} --summary-only"
        if coverage_enabled()
        else f"cargo nextest run {_MANIFEST}"
    )
    result = context.run(command, warn=True, pty=False)
    if result.exited != 0:
        raise Exit("nextest: cli tests failed", code=1)
