import os
import re
import shlex
import shutil
import tempfile
from collections.abc import Callable, Collection, Mapping
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.coverage import (
    CoverageFloor,
    CoverageReportError,
    CoverageTotals,
    Metric,
    parse_totals,
)
from tasks.shared.instrumented_build import (
    Sweep,
    UnitListing,
    UnmatchedUpliftError,
    live_units,
    parse_build_environment,
    superseded_entries,
    superseded_incremental,
)
from tasks.shared.nextest_listing import NextestListing
from tasks.shared.paths import (
    CLI_INSTRUMENTED_BUILD_DIR,
    CLI_TARGET_DIR,
    CLI_WORKSPACE_CARGO_TOML,
)
from tasks.shared.rust import coverage_enabled
from tasks.shared.selection import Selection
from tasks.shared.shared_fixtures.consumers import families_consumed
from tasks.shared.shared_fixtures.errors import SharedFixturesError
from tasks.shared.shared_fixtures.families import (
    FIXTURE_LOG_VARIABLE,
    SHARED_FAMILIES,
    WORKBENCHES_VARIABLE,
    Family,
)
from tasks.shared.shared_fixtures.runs import shared_fixtures_root
from tasks.shared.shared_fixtures.session import (
    SharedBuildError,
    SharedFixtureSession,
)
from tasks.shared.signals import UNWIND_SIGNALS, Interruption, unwind_then_defer
from tasks.test.filters import selection_from

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
_UNIT_LISTING = f"cargo test {_MANIFEST} --no-run --message-format=json"
_BUILDER = "shared-fixtures"
_shared_root = shared_fixtures_root
_FIXTURE_LOGS = CLI_TARGET_DIR / "vcs-fixture-logs"
_HANDOVER_VARIABLES = frozenset(
    {
        *(family.handover_variable for family in Family),
        WORKBENCHES_VARIABLE,
        FIXTURE_LOG_VARIABLE,
    }
)
_COMPILING = re.compile(r"^\s*Compiling ", re.MULTILINE)
_MISE_TASK = "test:unit:cli"
_NEXTEST_FILTER_SYNTAX = (
    "test() matches test names; binary_id() matches test binaries"
)
_FILTER_HELP = (
    "nextest filterset selecting the tests to run, "
    "e.g. 'binary_id(vcs-adapters::classify)'; a filtered run skips "
    "the coverage report, sweep and floor"
)
_RUN_DIRECTLY = (
    "no cli tests ran; to run them while diagnosing, use "
    f"cargo nextest run {_MANIFEST} (each test builds its own fixtures)"
)

COVERAGE_FLOOR = CoverageFloor(
    {Metric.LINES: 92, Metric.FUNCTIONS: 92, Metric.REGIONS: 91}
)


@task(help={"filter_": _FILTER_HELP})
def run(context: Context, filter_: str | None = None) -> None:
    """Run cli/ workspace tests (instrumented with coverage unless disabled).

    The instrumented run skips cargo-llvm-cov's full clean, which rebuilds
    every crate, and clears only the previous run's profile data: profiles
    left over from an earlier run would be merged into this run's totals.
    After the report it deletes instrumented artefacts the current build no
    longer uses, which a dependency or toolchain change would otherwise leave
    behind.
    The totals must meet COVERAGE_FLOOR. Raise a floor as coverage improves;
    never lower one to get a change through, except to reset the floors after
    a Rust toolchain, llvm-tools or cargo-llvm-cov bump, by the rule in
    tasks/README.md. If instrumented binaries are suspected stale, run
    `cargo llvm-cov clean --workspace` and rerun.
    ACCELERATOR_COVERAGE=off drops to plain nextest for a faster loop.
    A run given --filter lists the selected tests first, builds only the
    shared fixtures they use, and skips the report, sweep and floor.

    The VCS fixture matrix is built once per run and handed to the tests;
    `cargo nextest run` run directly builds it in each test that needs it.

    `--all-features` turns on the `test-loopback` and `test-support` features
    the black-box suites need. The suites that build real repositories run
    unconditionally and need the mise-pinned `jj` and a `git` on `PATH`; an
    absent tool hard-fails rather than skipping.

    The permission tests refuse to run as root;
    ACCELERATOR_TEST_SKIP_PERMISSION_TESTS turns that refusal into a skip,
    announced here because cargo captures the tests' own output.
    """
    selection = selection_from(filter_)
    _announce_skipped_permission_tests()
    with unwind_then_defer(UNWIND_SIGNALS) as interruption:
        try:
            if coverage_enabled():
                _run_instrumented(context, selection, interruption)
            else:
                _run_plain(context, selection, interruption)
        except SharedBuildError as error:
            raise Exit(f"{error}\n{_RUN_DIRECTLY}", code=1) from error
        except SharedFixturesError as error:
            raise Exit(str(error), code=1) from error


def _announce_skipped_permission_tests() -> None:
    if os.environ.get(_PERMISSION_SKIP):
        print(
            f"WARNING: {_PERMISSION_SKIP} is set; permission tests are skipped "
            "when run as root"
        )


def _run_plain(
    context: Context, selection: Selection, interruption: Interruption
) -> None:
    families = _families_to_share(context, selection, build_environment=None)
    builder = _locate_builder(context, build_environment=None)
    with _session(interruption).open(
        families,
        _building_with(context, builder, {}),
        filtered=selection.is_filtered,
    ) as handover:
        _run_tests(
            context,
            _command_line("cargo nextest run", _MANIFEST, selection.nextest()),
            handover,
        )


def _run_instrumented(
    context: Context, selection: Selection, interruption: Interruption
) -> None:
    _clear_previous_profile_data(context)
    build_environment = _instrumented_environment(context)
    families = _families_to_share(context, selection, build_environment)
    builder = _locate_builder(context, build_environment)
    with _session(interruption).open(
        families,
        _building_with(
            context, builder, _profiling_beside_the_tests(build_environment)
        ),
        filtered=selection.is_filtered,
    ) as handover:
        _run_tests(
            context,
            _command_line(
                "cargo llvm-cov nextest",
                _MANIFEST,
                "--no-report",
                selection.nextest(),
            ),
            handover,
        )
    if selection.is_filtered:
        print("coverage: report, sweep and floor skipped for a filtered run")
        return
    totals = _report_totals(context)
    print(totals.summary_line())
    _sweep_superseded_artefacts(context, build_environment)
    _enforce_floor(totals)


def _families_to_share(
    context: Context,
    selection: Selection,
    build_environment: Mapping[str, str] | None,
) -> Collection[Family]:
    if not selection.is_filtered:
        return SHARED_FAMILIES
    listing = _list_selected_tests(context, selection, build_environment)
    if listing.selects_nothing:
        raise Exit(
            f"{selection.nothing_selected(_MISE_TASK)}; "
            f"{_NEXTEST_FILTER_SYNTAX}",
            code=1,
        )
    return families_consumed(listing)


def _list_selected_tests(
    context: Context,
    selection: Selection,
    build_environment: Mapping[str, str] | None,
) -> NextestListing:
    """List what the filter selects, compiling the tests on the way.

    Listing runs every test binary, so under coverage each one writes profile
    data; it goes to a directory of its own and is thrown away, because only
    the tests' run counts towards the totals.
    """
    with tempfile.TemporaryDirectory() as discarded_profiles:
        result = context.run(
            _command_line(
                "cargo nextest list",
                _MANIFEST,
                selection.nextest(),
                "--message-format json",
            ),
            warn=True,
            hide="out",
            pty=False,
            **_running_in(
                _profiling_into(Path(discarded_profiles), build_environment)
            ),
        )
    if result.exited != 0:
        raise Exit(
            f"nextest: listing the tests for {selection.flag} failed (a "
            "malformed filterset or a build error); see nextest's output "
            "above",
            code=1,
        )
    return NextestListing.parse(result.stdout)


def _profiling_into(
    directory: Path, build_environment: Mapping[str, str] | None
) -> Mapping[str, str] | None:
    if build_environment is None:
        return None
    return {
        **build_environment,
        "LLVM_PROFILE_FILE": str(directory / "list-%p.profraw"),
    }


def _running_in(
    environment: Mapping[str, str] | None,
) -> dict[str, Mapping[str, str]]:
    return {} if environment is None else {"env": environment}


def _command_line(*parts: str) -> str:
    return " ".join(part for part in parts if part)


def _profiling_beside_the_tests(
    build_environment: Mapping[str, str],
) -> Mapping[str, str]:
    """Write the builder's profile data where the coverage report reads.

    `show-env` names a profile file in the plain target directory, but
    `cargo llvm-cov nextest` writes and reports from the instrumented one, so
    data written where `show-env` says would never reach the totals.
    """
    profile_file = Path(build_environment["LLVM_PROFILE_FILE"]).name
    return {
        **build_environment,
        "LLVM_PROFILE_FILE": str(_INSTRUMENTED_BUILD.parent / profile_file),
    }


def _session(interruption: Interruption) -> SharedFixtureSession:
    return SharedFixtureSession(
        _shared_root(),
        _FIXTURE_LOGS,
        os.environ,
        interrupted=lambda: interruption.arrived,
    )


def _locate_builder(
    context: Context, build_environment: Mapping[str, str] | None
) -> Path:
    listing = context.run(
        _UNIT_LISTING,
        warn=True,
        hide="out",
        pty=False,
        **_running_in(build_environment),
    )
    if listing.exited != 0:
        raise Exit("cargo: compiling the cli tests failed", code=1)
    try:
        return UnitListing.parse(listing.stdout).executable(_BUILDER)
    except LookupError as error:
        raise Exit(f"cargo: {error}", code=1) from error


def _building_with(
    context: Context, builder: Path, build_environment: Mapping[str, str]
) -> Callable[[Mapping[Family, Path], Path], None]:
    def build(trees: Mapping[Family, Path], log: Path) -> None:
        flags = [
            word
            for family, tree in trees.items()
            for word in (f"--{family.value}", str(tree))
        ]
        result = context.run(
            shlex.join([str(builder), *flags]),
            warn=True,
            hide=True,
            pty=False,
            env=_without_inherited_handover(
                {**build_environment, FIXTURE_LOG_VARIABLE: str(log)}
            ),
            replace_env=True,
        )
        if result.exited != 0:
            raise SharedFixturesError(result.stderr.strip())

    return build


def _without_inherited_handover(
    variables: Mapping[str, str],
) -> dict[str, str]:
    return {
        **{
            name: value
            for name, value in os.environ.items()
            if name not in _HANDOVER_VARIABLES
        },
        **variables,
    }


def _enforce_floor(totals: CoverageTotals) -> None:
    shortfalls = COVERAGE_FLOOR.shortfalls(totals)
    if shortfalls:
        raise Exit(
            "coverage below COVERAGE_FLOOR in tasks/test/cli.py: "
            + ", ".join(map(str, shortfalls))
            + ". If nothing that affects coverage changed, run "
            "`cargo llvm-cov clean --workspace` and rerun.",
            code=1,
        )


def _clear_previous_profile_data(context: Context) -> None:
    result = context.run(
        f"cargo llvm-cov clean --profraw-only {_WORKSPACE}",
        warn=True,
        pty=False,
    )
    if result.exited != 0:
        raise Exit("cargo llvm-cov: clearing profile data failed", code=1)


def _run_tests(
    context: Context, command: str, handover: Mapping[str, str]
) -> None:
    result = context.run(
        command,
        warn=True,
        pty=False,
        env=_without_inherited_handover(handover),
        replace_env=True,
    )
    failures = [
        *(["nextest: cli tests failed"] if result.exited != 0 else []),
        *(
            [
                "nextest recompiled after the build step, so the builder "
                "and the tests were not the same build"
            ]
            if _COMPILING.search(result.stderr or "")
            else []
        ),
    ]
    if failures:
        raise Exit("\n".join(failures), code=1)


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


def _sweep_superseded_artefacts(
    context: Context, build_environment: Mapping[str, str]
) -> None:
    listing = _list_units(context, build_environment)
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
    swept = superseded_entries(
        _INSTRUMENTED_BUILD, live
    ) + superseded_incremental(_INSTRUMENTED_BUILD, listing.crates)
    freed = sum(map(_remove, swept))
    print(Sweep(entries=len(swept), freed_bytes=freed).summary_line())


def _instrumented_environment(context: Context) -> Mapping[str, str]:
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
    return {
        **parse_build_environment(environment.stdout),
        "CARGO_TARGET_DIR": str(_INSTRUMENTED_BUILD.parent),
    }


def _list_units(
    context: Context, build_environment: Mapping[str, str]
) -> UnitListing:
    listing = context.run(
        _UNIT_LISTING,
        warn=True,
        hide="out",
        pty=False,
        env=build_environment,
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
