import os
import shutil
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.coverage import (
    CoverageReportError,
    CoverageTotals,
    parse_totals,
)
from tasks.shared.instrumented_build import (
    Sweep,
    UnitListing,
    UnmatchedUpliftError,
    live_units,
    parse_build_environment,
    superseded_entries,
)
from tasks.shared.paths import (
    CLI_INSTRUMENTED_BUILD_DIR,
    CLI_WORKSPACE_CARGO_TOML,
)
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
_INSTRUMENTED_BUILD = CLI_INSTRUMENTED_BUILD_DIR


@task
def run(context: Context) -> None:
    """Run cli/ workspace tests (instrumented with coverage unless disabled).

    The instrumented run skips cargo-llvm-cov's full clean, which rebuilds
    every crate, and clears only the previous run's profile data: profiles
    left over from an earlier run would be merged into this run's totals.
    After the report it deletes instrumented artefacts the current build no
    longer uses, which a dependency or toolchain change would otherwise leave
    behind.
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
    _run_tests(context, f"cargo llvm-cov nextest {_MANIFEST} --no-report")
    print(_report_totals(context).summary_line())
    _sweep_superseded_artefacts(context)


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


def _sweep_superseded_artefacts(context: Context) -> None:
    listing = _list_units(context)
    if listing.rebuilt:
        raise Exit(
            f"cargo llvm-cov: listing the units rebuilt {listing.rebuilt}; "
            "its environment no longer matches the instrumented build, so "
            "nothing was swept",
            code=1,
        )
    if not listing.live_hashes:
        raise Exit(
            "cargo llvm-cov: the unit listing found no units, so nothing "
            "was swept",
            code=1,
        )
    try:
        live = live_units(listing)
    except UnmatchedUpliftError as error:
        raise Exit(
            f"cargo llvm-cov: {error}, so nothing was swept", code=1
        ) from error
    swept = superseded_entries(_INSTRUMENTED_BUILD, live)
    freed = sum(map(_remove, swept))
    print(Sweep(entries=len(swept), freed_bytes=freed).summary_line())


def _list_units(context: Context) -> UnitListing:
    environment = context.run(
        f"cargo llvm-cov show-env {_WORKSPACE}",
        warn=True,
        hide="out",
        pty=False,
    )
    if environment.exited != 0:
        raise Exit(
            "cargo llvm-cov: reading the build environment failed", code=1
        )
    listing = context.run(
        f"cargo test {_MANIFEST} --no-run --message-format=json",
        warn=True,
        hide="out",
        pty=False,
        env={
            **parse_build_environment(environment.stdout),
            "CARGO_TARGET_DIR": str(_INSTRUMENTED_BUILD.parent),
        },
    )
    if listing.exited != 0:
        raise Exit(
            "cargo llvm-cov: listing the instrumented units failed", code=1
        )
    return UnitListing.parse(listing.stdout)


def _remove(entry: Path) -> int:
    if entry.is_dir() and not entry.is_symlink():
        freed = sum(
            file.lstat().st_size
            for file in entry.rglob("*")
            if not file.is_dir()
        )
        shutil.rmtree(entry)
        return freed
    freed = entry.lstat().st_size
    entry.unlink()
    return freed
