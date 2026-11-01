"""Known answers over one real warm no-op run on the reference host.

The goldens came from ``mise run measure:build -- --runs 1`` on engywook
(macOS 26.3, mise 2026.10.8 macos-arm64) at commit e49b4bd02684. To
re-capture them, run that command and copy from its run directory into
``tests/unit/tasks/fixtures/build-measurement/``:

- ``run-1.log`` as ``warm-no-op-build.golden.gz``, with ``gzip -9``;
- the lines stamped before 60 s of ``run-1-iostat.log`` as
  ``warm-no-op-iostat.golden``, with ``awk '{ if ($1+0 < 60) print }'``;
- the first 60 ticks of ``run-1-ps.log`` as ``warm-no-op-ps.golden``, with
  ``awk '$2=="sample"{n++} n<=60'``;
- ``mise tasks --json`` at the same commit as
  ``warm-no-op-tasks.golden.json``.

Then re-derive each constant below with the command beside it, run from that
fixtures directory. The goldens are never edited by hand.
"""

import gzip

import pytest

from tasks.shared.build_measurement.cpu_load import cpu_buckets, cpu_readings
from tasks.shared.build_measurement.settings import DEFAULT_TASK
from tasks.shared.build_measurement.task_graph import TaskGraph
from tasks.shared.build_measurement.timeline import (
    TaskOutcome,
    Timeline,
    analyse_timeline,
    read_stamped,
)
from tasks.shared.build_measurement.tracked_processes import (
    DEFAULT_TRACKED_PROCESSES,
    cpu_used,
    process_snapshots,
)
from tasks.shared.sources import repo_root

_FIXTURES = repo_root() / "tests/unit/tasks/fixtures/build-measurement"

# jq over warm-no-op-tasks.golden.json: the default closure's tasks whose
# `run` is [].
ROLL_UPS = frozenset(
    {
        "docs:check",
        "format:fix",
        "lint:check",
        "lint:scripts:check",
        "test",
        "test:e2e",
        "test:integration",
        "test:integration:fixture-size",
        "test:unit",
        "types:check",
    }
)
# zgrep 'sources up-to-date, skipping' warm-no-op-build.golden.gz
UP_TO_DATE = frozenset(
    {
        "deps:install:docs",
        "deps:install:docs-playwright",
        "docs:build",
        "docs:generate",
    }
)
# zgrep -E '^\S+ \[test:unit:cli\] Finished in' warm-no-op-build.golden.gz
TEST_UNIT_CLI_DURATION = 293.85
# zgrep -E '\[test:unit:cli\] +Summary \[' warm-no-op-build.golden.gz
TEST_UNIT_CLI_NEXTEST_PHASE = 149.551
# zgrep -oE '\[test:unit:cli\] .*' warm-no-op-build.golden.gz
#   | grep -oE '(^|\s)Compiling \S+ v' | wc -l
TEST_UNIT_CLI_COMPILING = 45
# test:unit:cli's `$` line at 12.911708208004711, less the end of its one
# dependency, deps:install:rust-components: its `$` line at
# 2.1176625420048367 plus its "Finished in 362.9ms".
TEST_UNIT_CLI_QUEUEING_DELAY = 12.911708208004711 - (
    2.1176625420048367 + 0.3629
)
# awk over warm-no-op-iostat.golden: numeric reports after the first, and
# the mean of us + sy over those stamped before 30 s.
IOSTAT_READINGS = 29
IOSTAT_FIRST_BUCKET_BUSY = 1157 / 14
# awk '/\/usr\/libexec\/syspolicyd$/ {print $4}' warm-no-op-ps.golden:
# 3:26.74 at the first tick, 3:26.83 at the last.
SYSPOLICYD_CPU_SECONDS = 0.09


@pytest.fixture(scope="module")
def graph() -> TaskGraph:
    return TaskGraph.parse(
        (_FIXTURES / "warm-no-op-tasks.golden.json").read_text()
    )


@pytest.fixture(scope="module")
def timeline(graph) -> Timeline:
    with gzip.open(
        _FIXTURES / "warm-no-op-build.golden.gz",
        "rt",
        encoding="utf-8",
        newline="",
    ) as golden:
        lines = read_stamped(golden.read())
    return analyse_timeline(lines, graph, graph.closure(DEFAULT_TASK))


def _with_outcome(timeline: Timeline, outcome: TaskOutcome) -> frozenset[str]:
    return frozenset(
        task
        for task, figures in timeline.tasks.items()
        if figures.outcome is outcome
    )


def test_the_silent_tasks_are_exactly_the_roll_ups(timeline):
    assert _with_outcome(timeline, TaskOutcome.SILENT) == ROLL_UPS


def test_the_skipped_tasks_are_exactly_the_up_to_date_ones(timeline):
    assert _with_outcome(timeline, TaskOutcome.SKIPPED) == UP_TO_DATE


def test_test_unit_cli_duration_is_its_finished_in_figure(timeline):
    assert timeline.tasks["test:unit:cli"].duration_seconds == (
        TEST_UNIT_CLI_DURATION
    )


def test_test_unit_cli_nextest_phase_is_its_summary_figure(timeline):
    assert timeline.tasks["test:unit:cli"].nextest_phase_seconds == (
        TEST_UNIT_CLI_NEXTEST_PHASE
    )


def test_test_unit_cli_compiling_count_matches_the_log(timeline):
    assert timeline.tasks["test:unit:cli"].units.compiling == (
        TEST_UNIT_CLI_COMPILING
    )


def test_one_dependants_queueing_delay(timeline):
    assert timeline.tasks[
        "test:unit:cli"
    ].queueing_delay_seconds == pytest.approx(TEST_UNIT_CLI_QUEUEING_DELAY)


def test_the_iostat_golden_gives_the_known_readings_and_first_bucket():
    readings = cpu_readings(
        read_stamped(
            (_FIXTURES / "warm-no-op-iostat.golden").read_text(newline="")
        )
    )
    assert len(readings) == IOSTAT_READINGS
    first, *_ = cpu_buckets(readings, Timeline({}), until=60.0)
    assert first.busy_percent == pytest.approx(IOSTAT_FIRST_BUCKET_BUSY)


def test_one_tracked_process_cpu_increase():
    [syspolicyd] = [
        each for each in DEFAULT_TRACKED_PROCESSES if each.name == "syspolicyd"
    ]
    sampled = process_snapshots(
        read_stamped((_FIXTURES / "warm-no-op-ps.golden").read_text(newline=""))
    )
    assert cpu_used(sampled, syspolicyd) == pytest.approx(
        SYSPOLICYD_CPU_SECONDS
    )


def test_every_other_closure_task_succeeded(timeline, graph):
    expected = graph.closure(DEFAULT_TASK) - ROLL_UPS - UP_TO_DATE
    assert _with_outcome(timeline, TaskOutcome.SUCCEEDED) == expected
