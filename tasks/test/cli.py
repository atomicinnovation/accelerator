import os

from invoke import Context, Exit, task

from tasks.shared.coverage import (
    CoverageReportError,
    CoverageTotals,
    parse_totals,
)
from tasks.shared.paths import CLI_WORKSPACE_CARGO_TOML
from tasks.shared.rust import coverage_enabled

# The visualiser server is a workspace member but keeps its own dedicated test
# tasks (test:unit:visualiser for the lib across both feature configs,
# test:integration:visualiser for the server harness that needs the launcher on
# PATH and a built frontend). Excluding it here avoids double-running those
# suites under the wrong feature combination and env.
_VISUALISER = "accelerator-visualiser"
_WORKSPACE = f"--manifest-path {CLI_WORKSPACE_CARGO_TOML}"
_MANIFEST = f"{_WORKSPACE} --workspace --exclude {_VISUALISER} --all-features"
_PERMISSION_SKIP = "ACCELERATOR_TEST_SKIP_PERMISSION_TESTS"


@task
def run(context: Context) -> None:
    """Run cli/ workspace tests (instrumented with coverage unless disabled).

    The instrumented run skips cargo-llvm-cov's full clean, which rebuilds
    every crate, and clears only the previous run's profile data: profiles
    left over from an earlier run would be merged into this run's totals.
    The totals are reported, not enforced. If instrumented binaries are
    suspected stale, run `cargo llvm-cov clean --workspace` and rerun.
    ACCELERATOR_COVERAGE=off drops to plain nextest for a faster loop.

    `--all-features` turns on the `test-loopback` and `test-support` features
    the black-box suites need. The suites that build real repositories run
    unconditionally and need the mise-pinned `jj` and a `git` on `PATH`; an
    absent tool hard-fails rather than skipping.

    The permission tests refuse to run as root;
    ACCELERATOR_TEST_SKIP_PERMISSION_TESTS turns that refusal into a skip,
    announced here because cargo captures the tests' own output.
    """
    _announce_skipped_permission_tests()
    if coverage_enabled():
        _run_instrumented(context)
    else:
        _run_tests(context, f"cargo nextest run {_MANIFEST}")


def _announce_skipped_permission_tests() -> None:
    if os.environ.get(_PERMISSION_SKIP):
        print(
            f"WARNING: {_PERMISSION_SKIP} is set; permission tests are skipped "
            "when run as root"
        )


def _run_instrumented(context: Context) -> None:
    _clear_previous_profile_data(context)
    _run_tests(
        context, f"cargo llvm-cov nextest {_MANIFEST} --no-clean --no-report"
    )
    print(_report_totals(context).summary_line())


def _clear_previous_profile_data(context: Context) -> None:
    result = context.run(
        f"cargo llvm-cov clean --profraw-only {_WORKSPACE}",
        warn=True,
        pty=False,
    )
    if result.exited != 0:
        raise Exit("cargo llvm-cov: clearing profile data failed", code=1)


def _run_tests(context: Context, command: str) -> None:
    result = context.run(command, warn=True, pty=False)
    if result.exited != 0:
        raise Exit("nextest: cli tests failed", code=1)


def _report_totals(context: Context) -> CoverageTotals:
    result = context.run(
        f"cargo llvm-cov report {_WORKSPACE} --json --summary-only",
        warn=True,
        hide="out",
        pty=False,
    )
    if result.exited != 0:
        raise Exit("cargo llvm-cov: coverage report failed", code=1)
    try:
        return parse_totals(result.stdout)
    except CoverageReportError as error:
        raise Exit(f"cargo llvm-cov: {error}", code=1) from error
