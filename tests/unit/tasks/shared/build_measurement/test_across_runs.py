import pytest
from pydantic import TypeAdapter, ValidationError

from tasks.shared.build_measurement.across_runs import (
    Absence,
    Absent,
    Caveat,
    Flagged,
    Measured,
    Presence,
    ProcessAcrossRuns,
    Spread,
    TaskAcrossRuns,
    process_across_runs,
    spread,
    task_across_runs,
)
from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.timeline import (
    CargoUnits,
    TaskFigures,
    TaskOutcome,
    failed_tasks,
)

_UNREADABLE = NoReading(argv=("ps",))


def _run(
    outcome: TaskOutcome = TaskOutcome.SUCCEEDED,
    *,
    duration: float | None = 10.0,
    nextest: float | None = None,
    after_failure: bool = False,
) -> TaskFigures:
    printed = outcome in {TaskOutcome.SUCCEEDED, TaskOutcome.FAILED}
    return TaskFigures(
        outcome=outcome,
        exit_status=0 if outcome is TaskOutcome.SUCCEEDED else None,
        start_seconds=0.0 if printed else None,
        duration_seconds=duration if printed else None,
        end_seconds=duration if printed else None,
        units=CargoUnits(),
        lock_wait_seconds=0.0,
        nextest_phase_seconds=nextest,
        queueing_delay_seconds=None,
        after_failure=after_failure,
    )


_SKIPPED = _run(TaskOutcome.SKIPPED)
_SILENT = _run(TaskOutcome.SILENT)


def test_the_spread_of_three_values():
    assert spread((51.3, 44.0, 78.2)) == Spread(
        minimum=44.0, median=51.3, maximum=78.2
    )


class TestTasks:
    def test_a_task_clean_in_every_run_is_measured_with_spreads(self):
        across = task_across_runs(
            (_run(duration=3.0), _run(duration=1.0), _run(duration=2.0))
        )

        assert across.presence == Measured()
        assert across.duration_seconds == Spread(
            minimum=1.0, median=2.0, maximum=3.0
        )

    def test_a_task_failing_in_one_run_keeps_per_run_values_but_no_spread(
        self,
    ):
        per_run = (_run(), _run(TaskOutcome.FAILED, duration=4.0), _run())

        across = task_across_runs(per_run)

        assert across.presence == Flagged(caveats=(Caveat.FAILED,))
        assert across.duration_seconds is None
        assert per_run[1].duration_seconds == 4.0

    def test_a_task_after_a_failure_in_any_run_is_flagged_with_no_spread(
        self,
    ):
        across = task_across_runs((_run(), _run(after_failure=True)))

        assert across.presence == Flagged(caveats=(Caveat.AFTER_FAILURE,))
        assert across.duration_seconds is None

    def test_a_task_skipped_in_every_run_is_absent_as_skipped(self):
        across = task_across_runs((_SKIPPED, _SKIPPED))

        assert across.presence == Absent(absence=Absence.SKIPPED)
        assert across.duration_seconds is None

    def test_a_task_skipped_in_some_runs_is_flagged(self):
        across = task_across_runs((_run(), _SKIPPED))

        assert across.presence == Flagged(
            caveats=(Caveat.SKIPPED_IN_SOME_RUNS,)
        )

    def test_a_task_silent_in_every_run_is_absent_as_silent(self):
        across = task_across_runs((_SILENT, _SILENT))

        assert across.presence == Absent(absence=Absence.SILENT)

    def test_a_task_silent_in_some_runs_is_flagged(self):
        across = task_across_runs((_run(), _SILENT))

        assert across.presence == Flagged(caveats=(Caveat.SILENT_IN_SOME_RUNS,))

    def test_a_task_not_run_in_any_run_is_flagged_with_no_spread(self):
        across = task_across_runs(
            (_run(TaskOutcome.NOT_RUN), _run(TaskOutcome.NOT_RUN))
        )

        assert across.presence == Flagged(caveats=(Caveat.NOT_RUN,))
        assert across.duration_seconds is None

    def test_failing_in_one_run_and_skipped_in_another_carries_both_flags(
        self,
    ):
        across = task_across_runs((_run(), _run(TaskOutcome.FAILED), _SKIPPED))

        assert across.presence == Flagged(
            caveats=(Caveat.FAILED, Caveat.SKIPPED_IN_SOME_RUNS)
        )

    def test_a_nextest_spread_needs_a_phase_in_every_run(self):
        every = task_across_runs((_run(nextest=2.0), _run(nextest=4.0)))
        some = task_across_runs((_run(nextest=2.0), _run(nextest=None)))

        assert every.nextest_phase_seconds == Spread(
            minimum=2.0, median=3.0, maximum=4.0
        )
        assert some.presence == Measured()
        assert some.nextest_phase_seconds is None

    def test_a_run_with_a_failure_is_marked_and_still_counts_for_the_wall_clock(
        self,
    ):
        runs = (
            {"build": _run(), "test": _run()},
            {"build": _run(), "test": _run(TaskOutcome.FAILED)},
        )
        wall_clocks = (305.1, 322.0)

        assert [bool(failed_tasks(run)) for run in runs] == [False, True]
        assert spread(wall_clocks).maximum == 322.0
        assert task_across_runs(
            [run["test"] for run in runs]
        ).presence == Flagged(caveats=(Caveat.FAILED,))


class TestProcesses:
    def test_a_process_present_in_every_run_has_a_spread(self):
        across = process_across_runs((51.3, 44.0, 78.2))

        assert across.presence == Measured()
        assert across.cpu_seconds == Spread(
            minimum=44.0, median=51.3, maximum=78.2
        )

    def test_a_run_using_no_cpu_keeps_the_process_present(self):
        across = process_across_runs((0.0, 2.0, 4.0))

        assert across.presence == Measured()
        assert across.cpu_seconds is not None
        assert across.cpu_seconds.minimum == 0.0

    def test_a_process_absent_from_every_run_is_absent_as_not_seen(self):
        across = process_across_runs((None, None, None))

        assert across.presence == Absent(absence=Absence.NOT_SEEN)
        assert across.cpu_seconds is None

    def test_a_process_absent_from_one_run_of_three_is_flagged_with_no_spread(
        self,
    ):
        across = process_across_runs((51.3, None, 78.2))

        assert across.presence == Flagged(caveats=(Caveat.ABSENT_IN_SOME_RUNS,))
        assert across.cpu_seconds is None

    def test_a_process_unreadable_in_a_run_is_flagged_with_no_spread(self):
        across = process_across_runs((51.3, _UNREADABLE, 78.2))

        assert across.presence == Flagged(caveats=(Caveat.UNREADABLE,))
        assert across.cpu_seconds is None

    def test_a_process_unreadable_in_every_run_is_flagged_not_absent(self):
        across = process_across_runs((_UNREADABLE, _UNREADABLE))

        assert across.presence == Flagged(caveats=(Caveat.UNREADABLE,))

    def test_unreadable_in_one_run_and_unseen_in_another_carries_both_caveats(
        self,
    ):
        across = process_across_runs((51.3, _UNREADABLE, None))

        assert across.presence == Flagged(
            caveats=(Caveat.ABSENT_IN_SOME_RUNS, Caveat.UNREADABLE)
        )


class TestPresence:
    def test_tasks_and_processes_share_one_presence_shape(self):
        task = task_across_runs((_run(), _SKIPPED))
        process = process_across_runs((51.3, None))

        assert isinstance(task.presence, Flagged)
        assert isinstance(process.presence, Flagged)

    def test_a_flagged_presence_needs_a_caveat(self):
        with pytest.raises(ValidationError):
            Flagged(caveats=())

    @pytest.mark.parametrize(
        "presence",
        [
            Measured(),
            Flagged(caveats=(Caveat.FAILED,)),
            Absent(absence=Absence.SKIPPED),
        ],
    )
    @pytest.mark.parametrize("with_spread", [True, False])
    def test_a_spread_is_present_exactly_when_measured(
        self, presence, with_spread
    ):
        one = Spread(minimum=1.0, median=1.0, maximum=1.0)
        given = one if with_spread else None

        def build() -> None:
            TaskAcrossRuns(
                presence=presence,
                duration_seconds=given,
                nextest_phase_seconds=None,
            )
            ProcessAcrossRuns(presence=presence, cpu_seconds=given)

        if with_spread == isinstance(presence, Measured):
            build()
        else:
            with pytest.raises(ValidationError):
                build()

    @pytest.mark.parametrize(
        "presence",
        [
            Measured(),
            Flagged(caveats=(Caveat.FAILED, Caveat.AFTER_FAILURE)),
            Absent(absence=Absence.NOT_SEEN),
        ],
    )
    def test_each_presence_variant_round_trips_through_json(self, presence):
        adapter = TypeAdapter(Presence)

        assert adapter.validate_json(adapter.dump_json(presence)) == presence
