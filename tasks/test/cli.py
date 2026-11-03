import os

from invoke import Context, Exit, task

from tasks.shared.paths import CLI_WORKSPACE_CARGO_TOML
from tasks.shared.rust import coverage_enabled

# The visualiser server is a workspace member but keeps its own dedicated test
# tasks (test:unit:visualiser for the lib across both feature configs,
# test:integration:visualiser for the server harness that needs the launcher on
# PATH and a built frontend). Excluding it here avoids double-running those
# suites under the wrong feature combination and env.
_MANIFEST = (
    f"--manifest-path {CLI_WORKSPACE_CARGO_TOML} --workspace "
    "--exclude accelerator-visualiser --all-features"
)
_PERMISSION_SKIP = "ACCELERATOR_TEST_SKIP_PERMISSION_TESTS"


@task
def run(context: Context) -> None:
    """Run cli/ workspace tests (instrumented with coverage unless disabled).

    Coverage is folded into the test run, not a separate task: the default
    instrumented `cargo llvm-cov nextest` pass both runs the tests and reports
    coverage in one go. It is report-only — no `--fail-under`/threshold, by
    design. ACCELERATOR_COVERAGE=off drops to plain nextest for a faster loop.

    `--all-features` turns on the `test-loopback` and `test-support` features
    the black-box suites need. The suites that build real repositories run
    unconditionally and need the mise-pinned `jj` and a `git` on `PATH`; an
    absent tool hard-fails rather than skipping.

    The permission tests refuse to run as root;
    ACCELERATOR_TEST_SKIP_PERMISSION_TESTS turns that refusal into a skip,
    announced here because cargo captures the tests' own output.
    """
    if os.environ.get(_PERMISSION_SKIP):
        print(
            f"WARNING: {_PERMISSION_SKIP} is set; permission tests are skipped "
            "when run as root"
        )
    command = (
        f"cargo llvm-cov nextest {_MANIFEST} --summary-only"
        if coverage_enabled()
        else f"cargo nextest run {_MANIFEST}"
    )
    result = context.run(command, warn=True, pty=False)
    if result.exited != 0:
        raise Exit("nextest: cli tests failed", code=1)
