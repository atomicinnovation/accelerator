from types import MappingProxyType

import pytest

from tasks.shared.build_measurement.task_graph import TaskGraph
from tasks.shared.build_measurement.timeline import (
    CargoUnits,
    OwnFigures,
    StampedLine,
    TaskOutcome,
    TaskTranscript,
    Timeline,
    analyse_timeline,
    own_figures,
    place_in_graph,
    read_stamped,
    stamped_text,
    tasks_mise_reported_failing,
    transcripts,
)
from tasks.shared.sources import repo_root

_FIXTURES = repo_root() / "tests/unit/tasks/fixtures/build-measurement"


def _fixture(name: str) -> tuple[StampedLine, ...]:
    return read_stamped((_FIXTURES / name).read_text(newline=""))


def _stamped(*lines: tuple[float, str]) -> tuple[StampedLine, ...]:
    return tuple(StampedLine(at=at, text=text) for at, text in lines)


def _graph(
    depends: dict[str, tuple[str, ...]], roll_ups: frozenset[str] = frozenset()
) -> TaskGraph:
    tasks = set(depends) | {d for ds in depends.values() for d in ds}
    return TaskGraph(
        depends=MappingProxyType(depends), runnable=frozenset(tasks - roll_ups)
    )


def _analyse(
    lines: tuple[StampedLine, ...],
    depends: dict[str, tuple[str, ...]],
    roll_ups: frozenset[str] = frozenset(),
    *,
    ready_from: float = 0.0,
) -> Timeline:
    graph = _graph(depends, roll_ups)
    tasks = set(depends) | {d for ds in depends.values() for d in ds}
    return analyse_timeline(lines, graph, tasks, ready_from=ready_from)


_FINISHED_IN = {"deps:install": (), "lint:check": ("deps:install",)}
_FAILED = {
    "build": (),
    "lint": (),
    "test": ("build",),
    "report": ("test",),
    "publish": ("report",),
}
_SKIPPED = {"docs:generate": (), "docs:build": ("docs:generate",)}
_SILENT = {"a": (), "b": (), "lint": ("a", "b"), "report": ("lint",)}
_LOCK_WAIT = {"build": (), "doc": ()}
_CARGO_UNITS = {"lint:clippy": (), "build": ()}
_NEXTEST = {"test:unit:cli": ()}


def _own(
    outcome: TaskOutcome = TaskOutcome.SUCCEEDED,
    *,
    start: float | None = 0.0,
    end: float | None = 1.0,
) -> OwnFigures:
    return OwnFigures(
        outcome=outcome,
        exit_status=0 if outcome is TaskOutcome.SUCCEEDED else None,
        start_seconds=start,
        duration_seconds=None if end is None or start is None else end - start,
        end_seconds=end,
        units=CargoUnits(),
        lock_wait_seconds=0.0,
        nextest_phase_seconds=None,
    )


class TestStages:
    def test_transcripts_group_lines_by_prefix_and_drop_unprefixed_ones(self):
        grouped = transcripts(_fixture("finished-in.log"))

        assert set(grouped) == {"deps:install", "lint:check"}
        assert [line.at for line in grouped["deps:install"].counted] == [
            0.1,
            0.4,
        ]

    def test_a_transcript_keeps_the_verdict_status_and_skip_line(self):
        failed = transcripts(_fixture("failed-task.log"))["test"]
        succeeded = transcripts(_fixture("failed-task.log"))["build"]
        skipped = transcripts(_fixture("skipped-task.log"))["docs:generate"]

        assert failed.exit_status == 3
        assert [line.at for line in failed.counted] == [0.52, 0.9]
        assert succeeded.finished_in == 0.5
        assert skipped.skipped_at == 0.2

    @pytest.mark.parametrize(
        ("own", "outcome", "none_fields"),
        [
            (
                OwnFigures.skipped(2.0),
                TaskOutcome.SKIPPED,
                {"duration_seconds", "nextest_phase_seconds", "exit_status"},
            ),
            (
                OwnFigures.silent(),
                TaskOutcome.SILENT,
                {
                    "exit_status",
                    "start_seconds",
                    "duration_seconds",
                    "end_seconds",
                    "nextest_phase_seconds",
                },
            ),
            (
                OwnFigures.not_run(),
                TaskOutcome.NOT_RUN,
                {
                    "exit_status",
                    "start_seconds",
                    "duration_seconds",
                    "end_seconds",
                    "nextest_phase_seconds",
                },
            ),
        ],
    )
    def test_own_figures_follow_the_outcome(self, own, outcome, none_fields):
        assert own.outcome is outcome
        assert {name for name, value in own if value is None} == none_fields

    def test_a_skipped_task_starts_and_ends_at_its_skip_line(self):
        skipped = OwnFigures.skipped(2.0)

        assert (skipped.start_seconds, skipped.end_seconds) == (2.0, 2.0)

    def test_placing_in_the_graph_adds_queueing_and_after_failure(self):
        timeline = place_in_graph(
            {
                "build": _own(start=0.0, end=1.0),
                "test": _own(TaskOutcome.FAILED, start=1.5, end=2.0),
                "report": _own(start=2.25, end=3.0),
            },
            _graph({"build": (), "test": ("build",), "report": ("test",)}),
            ready_from=0.0,
        )

        assert timeline.tasks["test"].queueing_delay_seconds == 0.5
        assert timeline.tasks["report"].queueing_delay_seconds == 0.25
        assert timeline.tasks["report"].after_failure
        assert not timeline.tasks["test"].after_failure

    def test_read_stamped_splits_only_at_newlines(self):
        lines = read_stamped("1.5 [t] a\rb\u2028c\n2.0 [t] d\n")

        assert lines == (
            StampedLine(at=1.5, text="[t] a\rb\u2028c"),
            StampedLine(at=2.0, text="[t] d"),
        )

    def test_a_stamped_line_reads_back_as_written(self):
        line = StampedLine(at=0.123456, text="[t] a\rb")

        assert read_stamped(stamped_text(line) + "\n") == (line,)

    def test_tasks_mise_reported_failing_come_from_the_status_lines(self):
        assert tasks_mise_reported_failing(_fixture("failed-task.log")) == (
            "test",
        )


class TestOutcomesAndDurations:
    def test_duration_is_mises_finished_in_figure(self):
        timeline = _analyse(_fixture("finished-in.log"), _FINISHED_IN)

        assert timeline.tasks["deps:install"].duration_seconds == 1.01
        assert timeline.tasks["lint:check"].duration_seconds == 0.0054

    def test_duration_without_finished_in_spans_first_to_last_line(self):
        timeline = _analyse(_fixture("failed-task.log"), _FAILED)

        # The `$` line at 0.52 to the last output line at 0.9.
        assert timeline.tasks["test"].duration_seconds == pytest.approx(0.38)

    def test_a_failed_tasks_status_comes_from_the_end_of_run_line(self):
        test = _analyse(_fixture("failed-task.log"), _FAILED).tasks["test"]

        assert test.outcome is TaskOutcome.FAILED
        assert test.exit_status == 3
        assert test.end_seconds == 0.9

    def test_a_task_with_no_verdict_counts_as_failed(self):
        timeline = _analyse(
            _stamped((0.0, "[t] $ invoke t"), (0.5, "[t] working")),
            {"t": ()},
        )

        assert timeline.tasks["t"].outcome is TaskOutcome.FAILED
        assert timeline.tasks["t"].exit_status is None
        assert timeline.failed() == ("t",)

    def test_a_skipped_task_has_no_duration_and_ends_at_its_skip_line(self):
        timeline = _analyse(_fixture("skipped-task.log"), _SKIPPED)
        generate = timeline.tasks["docs:generate"]

        assert generate.outcome is TaskOutcome.SKIPPED
        assert generate.duration_seconds is None
        # docs:build's `$` line at 0.5, less the skip line at 0.2.
        assert timeline.tasks[
            "docs:build"
        ].queueing_delay_seconds == pytest.approx(0.3)

    def test_a_roll_up_that_printed_nothing_is_silent(self):
        timeline = _analyse(
            _stamped((0.0, "[a] $ invoke a"), (0.1, "[a] Finished in 0.1s")),
            {"a": (), "lint": ("a",)},
            frozenset({"lint"}),
        )

        assert timeline.tasks["lint"].outcome is TaskOutcome.SILENT
        assert timeline.failed() == ()

    def test_a_runnable_task_that_printed_nothing_was_not_run(self):
        timeline = _analyse(
            _stamped((0.0, "[b] $ invoke b"), (0.1, "[b] Finished in 0.1s")),
            {"a": (), "b": ("a",)},
        )

        assert timeline.tasks["a"].outcome is TaskOutcome.NOT_RUN
        assert timeline.tasks["b"].after_failure
        assert timeline.failed() == ("a",)

    def test_each_task_records_its_end(self):
        failed = _analyse(_fixture("failed-task.log"), _FAILED)
        skipped = _analyse(_fixture("skipped-task.log"), _SKIPPED)

        # build: its `$` line at 0.0 plus `Finished in 0.5s`.
        assert failed.tasks["build"].end_seconds == 0.5
        assert skipped.tasks["docs:generate"].end_seconds == 0.2
        assert failed.tasks["test"].end_seconds == 0.9

    def test_unprefixed_lines_belong_to_no_task(self):
        timeline = _analyse(_fixture("finished-in.log"), _FINISHED_IN)

        assert set(timeline.tasks) == {"deps:install", "lint:check"}
        assert timeline.tasks["lint:check"].duration_seconds == 0.0054


class TestQueueingAndAfterFailure:
    def test_a_failed_dependency_with_no_counted_line_passes_readiness_through(
        self,
    ):
        timeline = _analyse(
            _stamped(
                (0.0, "[build] $ invoke build"),
                (1.0, "[build] Finished in 1.0s"),
                (1.5, "[report] $ invoke report"),
                (1.6, "[report] Finished in 0.1s"),
                (2.0, "[test] exited with status 2"),
                (2.1, "[test] ERROR task failed"),
            ),
            {"build": (), "test": ("build",), "report": ("test",)},
        )

        assert timeline.tasks["test"].end_seconds is None
        # report is ready when build ends at 1.0, and starts at 1.5.
        assert timeline.tasks["report"].queueing_delay_seconds == pytest.approx(
            0.5
        )

    def test_dependants_of_a_failure_are_flagged_from_the_graph(self):
        timeline = _analyse(_fixture("failed-task.log"), _FAILED)

        assert timeline.tasks["report"].after_failure
        assert timeline.tasks["publish"].after_failure
        assert not timeline.tasks["lint"].after_failure
        assert not timeline.tasks["build"].after_failure
        assert not timeline.tasks["test"].after_failure

    def test_a_task_with_no_dependencies_is_ready_at_the_run_start(self):
        timeline = _analyse(_fixture("finished-in.log"), _FINISHED_IN)

        assert timeline.tasks["deps:install"].queueing_delay_seconds == 0.1

    def test_a_task_with_no_dependencies_is_ready_at_the_given_origin(self):
        timeline = _analyse(
            _stamped((40.3, "[t] $ invoke t"), (40.5, "[t] Finished in 0.2s")),
            {"t": ()},
            ready_from=40.0,
        )

        # The `$` line at 40.3, less the origin at 40.0.
        assert timeline.tasks["t"].queueing_delay_seconds == pytest.approx(0.3)

    def test_disjoint_timelines_merge(self):
        first = _analyse(
            _stamped((0.0, "[a] $ invoke a"), (0.1, "[a] Finished in 0.1s")),
            {"a": ()},
        )
        second = _analyse(
            _stamped((1.0, "[b] $ invoke b"), (1.2, "[b] Finished in 0.2s")),
            {"b": ()},
            ready_from=1.0,
        )

        merged = Timeline.merged((first, second))

        assert merged.tasks == {**first.tasks, **second.tasks}

    def test_overlapping_timelines_do_not_merge(self):
        timeline = _analyse(
            _stamped((0.0, "[a] $ invoke a"), (0.1, "[a] Finished in 0.1s")),
            {"a": ()},
        )

        with pytest.raises(ValueError, match="a"):
            Timeline.merged((timeline, timeline))

    def test_a_silent_dependency_ends_when_its_own_dependencies_end(self):
        timeline = _analyse(
            _fixture("silent-roll-up.log"), _SILENT, frozenset({"lint"})
        )

        # b ends last beneath lint: its `$` line at 0.0 plus 1.0 s. report's
        # `$` line is at 1.3.
        assert timeline.tasks["report"].queueing_delay_seconds == pytest.approx(
            0.3
        )

    def test_queueing_delay_is_never_negative(self):
        timeline = _analyse(
            _stamped(
                (0.0, "[a] $ invoke a"),
                (0.4, "[b] $ invoke b"),
                (0.5, "[a] Finished in 0.5s"),
                (0.6, "[b] Finished in 0.2s"),
            ),
            {"a": (), "b": ("a",)},
        )

        assert timeline.tasks["b"].queueing_delay_seconds == 0.0


class TestLockWait:
    def test_lock_wait_runs_to_the_next_line(self):
        build = _analyse(_fixture("lock-wait.log"), _LOCK_WAIT).tasks["build"]

        # 0.5 to the next line at 1.5, plus 2.0 to the next line at 2.25.
        assert build.lock_wait_seconds == pytest.approx(1.25)

    def test_consecutive_lock_waits_each_run_to_the_next_line(self):
        timeline = _analyse(
            _stamped(
                (0.0, "[t] $ cargo build"),
                (
                    1.0,
                    "[t]     Blocking waiting for file lock on package cache",
                ),
                (
                    1.4,
                    "[t]     Blocking waiting for file lock on build directory",
                ),
                (3.0, "[t]    Compiling a v0.1.0"),
                (3.5, "[t] Finished in 3.5s"),
            ),
            {"t": ()},
        )

        # 1.0 to 1.4, then 1.4 to 3.0.
        assert timeline.tasks["t"].lock_wait_seconds == pytest.approx(2.0)

    def test_a_lock_wait_still_open_at_the_last_line_runs_to_the_tasks_end(
        self,
    ):
        doc = _analyse(_fixture("lock-wait.log"), _LOCK_WAIT).tasks["doc"]

        # The wait opens at 3.0; doc ends at its `$` line, 2.6, plus 1.5 s.
        assert doc.lock_wait_seconds == pytest.approx(1.1)

    def test_an_open_wait_in_a_failed_task_runs_to_its_last_counted_line(
        self,
    ):
        timeline = _analyse(
            _stamped(
                (0.0, "[t] $ cargo build"),
                (
                    1.0,
                    "[t]     Blocking waiting for file lock on build directory",
                ),
                (2.5, "[t] error: could not compile `a`"),
                (9.0, "[t] exited with status 101"),
            ),
            {"t": ()},
        )

        assert timeline.tasks["t"].lock_wait_seconds == pytest.approx(1.5)


class TestUnitsAndNextest:
    def test_each_unit_kind_is_counted(self):
        timeline = _analyse(_fixture("cargo-units.log"), _CARGO_UNITS)

        assert timeline.tasks["lint:clippy"].units == CargoUnits(checking=2)
        assert timeline.tasks["build"].units == CargoUnits(
            compiling=2, documenting=1
        )

    def test_units_are_counted_inside_carriage_return_progress_and_colour(
        self,
    ):
        build = _analyse(_fixture("cargo-units.log"), _CARGO_UNITS).tasks[
            "build"
        ]

        # Both `Compiling` segments share one `\r`-joined line, the second
        # wrapped in colour codes.
        assert build.units.compiling == 2

    def test_the_nextest_phase_comes_from_the_summary_line(self):
        timeline = _analyse(_fixture("nextest.log"), _NEXTEST)

        assert timeline.tasks["test:unit:cli"].nextest_phase_seconds == 149.123

    def test_several_summary_lines_sum(self):
        timeline = _analyse(
            _stamped(
                (0.0, "[t] $ cargo nextest run"),
                (1.0, "[t]      Summary [   0.843s] 9 tests run: 9 passed"),
                (2.0, "[t]      Summary [   1.157s] 3 tests run: 3 passed"),
                (2.1, "[t] Finished in 2.1s"),
            ),
            {"t": ()},
        )

        assert timeline.tasks["t"].nextest_phase_seconds == pytest.approx(2.0)

    def test_a_task_without_a_summary_line_has_no_nextest_phase(self):
        timeline = _analyse(_fixture("finished-in.log"), _FINISHED_IN)

        assert timeline.tasks["lint:check"].nextest_phase_seconds is None

    def test_unit_totals_sum_every_task(self):
        timeline = _analyse(_fixture("cargo-units.log"), _CARGO_UNITS)

        assert timeline.unit_totals() == CargoUnits(
            compiling=2, checking=2, documenting=1
        )


def test_a_transcript_with_a_verdict_but_no_lines_has_no_start():
    own = own_figures(
        TaskTranscript(
            counted=(), finished_in=None, exit_status=4, skipped_at=None
        )
    )

    assert own.outcome is TaskOutcome.FAILED
    assert own.start_seconds is None
    assert own.end_seconds is None
