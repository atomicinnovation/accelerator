import re
from collections.abc import Sequence
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.paths import CARGO_TOML, FRONTEND
from tasks.shared.selection import Selection

from .filters import (
    LIBTEST_UNIT_FILTER_HELP,
    NODE_TEST_UNIT_FILTER_HELP,
    PYTEST_FILTER_HELP,
    VITEST_FILTER_HELP,
    command_line,
    node_tests_run,
    npm_script_options,
    run_libtest_lane,
    run_pytest_lane,
    selection_from,
    vitest_tests_run,
)
from .helpers import repo_root


@task(help={"filter_": LIBTEST_UNIT_FILTER_HELP})
def visualiser(context: Context, filter_: str | None = None) -> None:
    """Run the visualiser server unit tests.

    Runs cargo test twice to cover both feature-gated test modules:
      1. `--no-default-features --features dev-frontend` — covers
         `path_normalisation_tests` and `dev_frontend_tests`. Does not
         require the SPA to be built.
      2. default features (embed-dist) — covers `path_normalisation_tests`
         and `embed_tests`. Requires `frontend/dist/index.html` because
         rust-embed reads the folder at compile time; when invoked via
         `mise run test:unit:visualiser`, `build:frontend` runs first.
    """
    run_libtest_lane(
        context,
        [
            f"cargo test --manifest-path {CARGO_TOML} --lib "
            f"--no-default-features --features dev-frontend",
            f"cargo test --manifest-path {CARGO_TOML} --lib",
        ],
        mise_task="test:unit:visualiser",
        filter_=filter_,
    )


@task(help={"filter_": VITEST_FILTER_HELP})
def frontend(context: Context, filter_: str | None = None) -> None:
    """Run the visualiser frontend unit tests (Vitest)."""
    selection = selection_from(filter_)
    result = context.run(
        command_line(
            f"npm --prefix {FRONTEND} run test",
            npm_script_options(selection.vitest()),
        )
    )
    if selection.is_filtered and vitest_tests_run(result.stdout) == 0:
        raise Exit(selection.nothing_selected("test:unit:frontend"), code=1)


@task(name="tasks", help={"filter_": PYTEST_FILTER_HELP})
def build_system(context: Context, filter_: str | None = None) -> None:
    """Run the pytest unit tests for the invoke tasks."""
    run_pytest_lane(
        context,
        "uv run pytest tests/unit/tasks -v",
        mise_task="test:unit:tasks",
        filter_=filter_,
    )


@task(help={"filter_": PYTEST_FILTER_HELP})
def frontend_licenses(context: Context, filter_: str | None = None) -> None:
    """Guard the built frontend bundle's licence closure."""
    run_pytest_lane(
        context,
        "uv run pytest tests/unit/frontend -v",
        mise_task="test:unit:frontend-licenses",
        filter_=filter_,
    )


@task(help={"filter_": PYTEST_FILTER_HELP})
def vcs(context: Context, filter_: str | None = None) -> None:
    """Run the pytest unit tests for the VCS test-support fixtures."""
    run_pytest_lane(
        context,
        "uv run pytest tests/unit/vcs -v",
        mise_task="test:unit:vcs",
        filter_=filter_,
    )


# The runtime-free JavaScript suites under the Playwright executor's `lib/`.
#
# A file-count floor alone would not catch a whole suite evaporating: `node
# --test` reports a skipped test as neither passed nor failed, so a suite that
# gated itself on an absent runtime looked identical to a passing one. Both
# floors are asserted, and the executed count comes from the runner's own TAP
# summary rather than from anything this task counts itself.
_EXPECTED_DESIGN_AUTOMATION_SUITES = 10

# Today's executed total across those files. An at-least floor: a suite may
# legitimately gain cases, but must never quietly lose them.
_EXPECTED_DESIGN_AUTOMATION_CASES = 105

_PLAYWRIGHT_DIR = "skills/design/inventory-design/scripts/playwright"


def _tap_counts(output: str) -> dict[str, int]:
    """Parse the runner's own accounting, rather than proxying for it."""
    counts = {}
    for field in ("pass", "fail", "skipped"):
        found = re.search(rf"^# {field} (\d+)$", output, re.MULTILINE)
        if found is None:
            raise Exit(
                f"node --test emitted no '# {field}' summary line", code=1
            )
        counts[field] = int(found.group(1))
    return counts


def _test_callback_ranges(source: str) -> list[tuple[int, int]]:
    """Character ranges of every `test(...)`/`it(...)` callback body.

    Brace-matched from the callback's opening `{` to its match, so a helper
    defined at module level is outside every range — which is the whole point:
    a whole-file grep cannot tell a helper's legitimate early return from a test
    body silently reporting success.
    """
    ranges = []
    for opener in re.finditer(r"\b(?:test|it)\s*\(", source):
        arrow = source.find("=> {", opener.end())
        if arrow == -1:
            continue
        depth = 0
        for index in range(arrow + 3, len(source)):
            if source[index] == "{":
                depth += 1
            elif source[index] == "}":
                depth -= 1
                if depth == 0:
                    ranges.append((arrow + 3, index))
                    break
    return ranges


# A bare early return inside a test body reports as *passed*, not skipped, so a
# suite that gates itself on an absent runtime stays green while asserting
# nothing.
# Matches a return that yields nothing, wherever it sits on the line — the
# common shape is a guard clause (`if (!installed) return;`), not a statement
# alone on its own line.
_BARE_RETURN = re.compile(r"(?:^|[;{}\s)])return\s*(?:null\s*)?;")


def _bare_returns_in_tests(source: str) -> list[str]:
    offenders = []
    for start, end in _test_callback_ranges(source):
        body = source[start:end]
        offset = source[:start].count("\n")
        for number, line in enumerate(body.split("\n"), start=offset + 1):
            # A test that calls the runner's own skip() is declaring itself
            # skipped, which the zero-skip assertion catches separately.
            if ".skip(" in line or "skip()" in line:
                continue
            if _BARE_RETURN.search(line):
                offenders.append(f"line {number}: {line.strip()}")
    return offenders


def report_filtered_design_automation_run(
    selection: Selection, mise_task: str, tap: str, suites: Sequence[Path]
) -> None:
    if node_tests_run(tap, suites) == 0:
        raise Exit(selection.nothing_selected(mise_task), code=1)
    print(
        "design-automation: pass-count and skip floors skipped for a "
        "filtered run"
    )


@task(help={"filter_": NODE_TEST_UNIT_FILTER_HELP})
def design_automation(context: Context, filter_: str | None = None) -> None:
    """Run the runtime-free Playwright-executor JavaScript suites."""
    selection = selection_from(filter_)
    root = repo_root() / _PLAYWRIGHT_DIR
    suites = sorted((root / "lib").glob("*.test.js"))
    if len(suites) < _EXPECTED_DESIGN_AUTOMATION_SUITES:
        raise Exit(
            f"expected at least {_EXPECTED_DESIGN_AUTOMATION_SUITES} suites "
            f"under {_PLAYWRIGHT_DIR}/lib, found {len(suites)}: a suite has "
            "gone missing rather than been deliberately retired",
            code=1,
        )

    for suite in suites:
        offenders = _bare_returns_in_tests(suite.read_text())
        if offenders:
            listed = "\n  ".join(offenders)
            raise Exit(
                f"{suite.name} returns early inside a test body, which "
                f"`node --test` reports as passed rather than skipped:\n  "
                f"{listed}",
                code=1,
            )

    discovered = " ".join(str(path) for path in suites)
    result = context.run(
        command_line(
            "node --test --test-reporter=tap", selection.node_test(), discovered
        ),
        warn=True,
        pty=False,
    )
    counts = _tap_counts(result.stdout)

    if counts["fail"]:
        raise Exit(f"{counts['fail']} design-automation test(s) failed", code=1)
    if selection.is_filtered:
        report_filtered_design_automation_run(
            selection, "test:unit:design-automation", result.stdout, suites
        )
        return
    if counts["skipped"]:
        raise Exit(
            f"{counts['skipped']} design-automation test(s) skipped; this lane "
            "is runtime-free, so a skip means a suite is gating itself on "
            "something it should not need",
            code=1,
        )
    if counts["pass"] < _EXPECTED_DESIGN_AUTOMATION_CASES:
        raise Exit(
            f"expected at least {_EXPECTED_DESIGN_AUTOMATION_CASES} executed "
            f"cases, got {counts['pass']}",
            code=1,
        )
