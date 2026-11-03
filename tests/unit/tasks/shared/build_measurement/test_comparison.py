import pytest

from tasks.shared.build_measurement.across_runs import (
    Absence,
    Absent,
    Caveat,
    Flagged,
    Measured,
)
from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.comparison import (
    ChangeFromIdle,
    ChangeInPresence,
    FigureOnOneSide,
    MeasuredChange,
    Noise,
    RecordSide,
    TaskComparison,
    TaskInNeither,
    TaskMissingFromOne,
    TrackedDifferently,
    UnitCountChange,
    UnitsVaryAcrossRuns,
    compare_records,
)
from tasks.shared.build_measurement.comparison_report import comparison_report
from tasks.shared.build_measurement.measured_build import (
    ConditionDifference,
    HostSettings,
    MeasurementConditions,
    Mode,
)
from tasks.shared.build_measurement.record import parse_record, record_text
from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.timeline import CargoUnits, TaskOutcome
from tasks.shared.build_measurement.tracked_processes import (
    DEFAULT_TRACKED_PROCESSES,
    IDE,
    with_ide_patterns,
)
from tests.unit.tasks.shared.build_measurement.compared_records import (
    DEVTOOLS_RUN_ID,
    OS_VERSION_ARGV,
    cpu_records,
    record,
    with_unit_cli,
    worked_example_current,
    worked_example_previous,
)
from tests.unit.tasks.shared.build_measurement.doubles import (
    REFERENCE_HOST,
    REFERENCE_MISE,
    FakeSession,
    task_figures,
)


def _process(comparison, name):
    [change] = [each for each in comparison.processes if each.name == name]
    return change


def _syspolicyd_change(previous_cpu, current_cpu):
    previous, current = cpu_records(
        [(cpu, 100.0, 5.0) for cpu in previous_cpu],
        [(cpu, 100.0, 5.0) for cpu in current_cpu],
    )
    return _process(compare_records(previous, current), "syspolicyd")


class TestChanges:
    def test_a_change_is_given_in_seconds_and_as_a_ratio(self):
        comparison = compare_records(
            worked_example_previous(), worked_example_current()
        )
        assert comparison.wall_clock.seconds == pytest.approx(-47.4)
        assert comparison.wall_clock.ratio == pytest.approx(262.0 / 309.4)

    def test_a_median_inside_the_previous_range_is_within_noise(self):
        change = _syspolicyd_change((49.0, 51.3, 53.0), (50.0, 52.0, 60.0))
        assert change.noise is Noise.WITHIN

    @pytest.mark.parametrize("median", [49.0, 53.0], ids=["minimum", "maximum"])
    def test_a_median_on_either_end_of_the_previous_range_is_within_noise(
        self, median
    ):
        change = _syspolicyd_change(
            (49.0, 51.3, 53.0), (median - 1.0, median, median + 1.0)
        )
        assert change.noise is Noise.WITHIN

    def test_a_median_outside_the_previous_range_is_not_noise(self):
        change = _syspolicyd_change((49.0, 51.3, 53.0), (11.0, 12.0, 13.0))
        assert change.noise is Noise.BEYOND

    def test_a_previous_record_of_one_run_judges_no_noise(self):
        previous = record(wall_clocks=(309.4,))
        current = record(wall_clocks=(309.4,))
        assert compare_records(previous, current).wall_clock.noise is (
            Noise.UNJUDGED
        )

    def test_a_process_present_in_both_records_has_its_change(self):
        change = _syspolicyd_change((49.0, 51.3, 53.0), (11.0, 12.0, 13.0))
        assert change == MeasuredChange(
            "syspolicyd",
            previous_median=51.3,
            current_median=12.0,
            noise=Noise.BEYOND,
        )

    def test_the_ratio_is_current_over_previous(self):
        change = _syspolicyd_change((49.0, 51.3, 53.0), (11.0, 12.0, 13.0))
        assert round(change.ratio, 3) == 0.234

    def test_a_change_from_an_idle_median_has_seconds_but_no_ratio(self):
        change = _syspolicyd_change((0.0, 0.0, 0.4), (2.0, 3.2, 4.0))
        assert isinstance(change, ChangeFromIdle)
        assert change.seconds == pytest.approx(3.2)
        assert not hasattr(change, "ratio")

    @pytest.mark.parametrize(
        ("current", "noise"),
        [((0.0, 0.2, 0.3), Noise.WITHIN), ((2.0, 3.2, 4.0), Noise.BEYOND)],
        ids=["inside", "outside"],
    )
    def test_a_change_from_an_idle_median_still_has_a_noise_verdict(
        self, current, noise
    ):
        change = _syspolicyd_change((0.0, 0.0, 0.4), current)
        assert change.noise is noise

    def test_a_change_to_an_idle_median_keeps_its_ratio(self):
        change = _syspolicyd_change((49.0, 51.3, 53.0), (0.0, 0.0, 0.0))
        assert isinstance(change, MeasuredChange)
        assert change.ratio == 0.0

    def test_a_wall_clock_from_an_idle_median_has_seconds_but_no_ratio(self):
        previous = record(wall_clocks=(0.0, 0.0, 0.0))
        current = record(run_id=DEVTOOLS_RUN_ID)
        assert compare_records(previous, current).wall_clock == (
            ChangeFromIdle("wall-clock", 309.4, Noise.BEYOND)
        )

    def test_a_written_record_compared_with_itself_shows_no_change(
        self, tmp_path
    ):
        measured = FakeSession(tmp_path).measure(runs=3).measured
        record = parse_record(record_text(measured), source=tmp_path)
        comparison = compare_records(record, record)
        assert comparison.wall_clock.seconds == 0.0
        assert comparison.wall_clock.noise is Noise.WITHIN
        assert comparison.differing_conditions == ()
        assert comparison.tasks_added == comparison.tasks_removed == ()
        measured_processes = [
            each
            for each in comparison.processes
            if isinstance(each, MeasuredChange)
        ]
        assert all(each.seconds == 0.0 for each in measured_processes)


class TestPresenceAndTracking:
    def test_a_process_absent_now_is_named_in_words_without_ratio_or_verdict(
        self,
    ):
        previous, current = cpu_records(
            [(10.0, 100.0, 5.0, 1.0)] * 3, [(10.0, 100.0, 5.0, None)] * 3
        )
        change = _process(compare_records(previous, current), "XprotectService")
        assert change == ChangeInPresence(
            "XprotectService",
            previous=Measured(),
            current=Absent(absence=Absence.NOT_SEEN),
        )
        report = comparison_report(compare_records(previous, current))
        assert (
            "XprotectService: measured in every 'none' run; did not run in "
            "any 'devtools' run"
        ) in report.splitlines()

    def test_a_process_absent_before_is_named_in_words(self):
        previous, current = cpu_records(
            [(10.0, 100.0, 5.0, None)] * 3, [(10.0, 100.0, 5.0, 1.0)] * 3
        )
        change = _process(compare_records(previous, current), "XprotectService")
        assert change == ChangeInPresence(
            "XprotectService",
            previous=Absent(absence=Absence.NOT_SEEN),
            current=Measured(),
        )

    def test_a_process_absent_from_both_records_is_named_as_unchanged(self):
        previous, current = cpu_records(
            [(10.0, 100.0, 5.0)] * 3, [(10.0, 100.0, 5.0)] * 3
        )
        comparison = compare_records(previous, current)
        assert isinstance(
            _process(comparison, "XprotectService"), ChangeInPresence
        )
        assert (
            "XprotectService: did not run in any 'none' or 'devtools' run"
            in comparison_report(comparison).splitlines()
        )

    @pytest.mark.parametrize(
        ("flagged_runs", "caveat"),
        [
            ([1.0, None, 1.0], Caveat.ABSENT_IN_SOME_RUNS),
            ([1.0, NoReading(argv=("ps",)), 1.0], Caveat.UNREADABLE),
        ],
        ids=["absent-in-some-runs", "unreadable"],
    )
    @pytest.mark.parametrize("side", list(RecordSide))
    def test_a_flagged_side_is_named_in_words_with_its_caveats(
        self, flagged_runs, caveat, side
    ):
        flagged = [(10.0, 100.0, 5.0, cpu) for cpu in flagged_runs]
        steady = [(10.0, 100.0, 5.0, 1.0)] * 3
        if side is RecordSide.PREVIOUS:
            previous, current = cpu_records(flagged, steady)
        else:
            previous, current = cpu_records(steady, flagged)
        comparison = compare_records(previous, current)
        change = _process(comparison, "XprotectService")
        assert isinstance(change, ChangeInPresence)
        labelled = {
            RecordSide.PREVIOUS: "'none'",
            RecordSide.CURRENT: "'devtools'",
        }
        assert (
            f"flagged {caveat} in the {labelled[side]} record"
            in comparison_report(comparison)
        )

    def test_a_process_tracked_by_different_patterns_is_not_compared(self):
        clion = with_ide_patterns(DEFAULT_TRACKED_PROCESSES, ["/MacOS/clion"])
        previous, current = cpu_records(
            [(10.0, 100.0, 5.0)] * 3, [(10.0, 100.0, 5.0)] * 3, tracked=clion
        )
        [default_ide] = [
            each.patterns
            for each in DEFAULT_TRACKED_PROCESSES
            if each.name == IDE
        ]
        assert _process(
            compare_records(previous, current), IDE
        ) == TrackedDifferently(IDE, default_ide, ("/MacOS/clion",))

    def test_a_process_tracked_on_one_side_only_is_tracked_differently(self):
        without_fsnotifier = tuple(
            each
            for each in DEFAULT_TRACKED_PROCESSES
            if each.name != "fsnotifier"
        )
        previous = record()
        current = record(
            process_cpu=[{"syspolicyd": 1.0, "XprotectService": None, IDE: 1.0}]
            * 3,
            tracked=without_fsnotifier,
            host_settings=HostSettings.DEVTOOLS,
            run_id=DEVTOOLS_RUN_ID,
        )
        change = _process(compare_records(previous, current), "fsnotifier")
        assert isinstance(change, TrackedDifferently)
        assert change.current_patterns == ()


_DIFFERING = {
    "hostname": {
        "host": REFERENCE_HOST.model_copy(update={"hostname": "kurt"})
    },
    "model": {"host": REFERENCE_HOST.model_copy(update={"model": "Mac15,9"})},
    "os_version": {
        "host": REFERENCE_HOST.model_copy(update={"os_version": "26.4"})
    },
    "mise_jobs": {"mise": REFERENCE_MISE.model_copy(update={"jobs": 4})},
    "cache_state": {"cache_state": BuildCacheState.INCREMENTAL},
    "mode": {"mode": Mode.ISOLATED},
    "isolated_tasks": {"isolated_tasks": ("build:cli",)},
    "host_settings": {"host_settings": HostSettings.DEVTOOLS},
}


class TestConditions:
    @pytest.mark.parametrize("field", list(MeasurementConditions.model_fields))
    def test_each_differing_condition_is_named_with_both_values(self, field):
        previous = record()
        current = record(**_DIFFERING[field])
        comparison = compare_records(previous, current)
        assert comparison.differing_conditions == (
            ConditionDifference(
                field,
                getattr(previous.conditions, field),
                getattr(current.conditions, field),
            ),
        )
        first_line = comparison_report(comparison).splitlines()[0]
        assert first_line.startswith(f"conditions differ: {field} (")

    def test_a_differing_label_is_reported_like_any_condition(self):
        comparison = compare_records(
            worked_example_previous(), worked_example_current()
        )
        assert comparison_report(comparison).splitlines()[0] == (
            "conditions differ: host_settings (none → devtools)"
        )

    def test_identical_conditions_read_as_matching(self):
        comparison = compare_records(record(), record())
        assert comparison_report(comparison).splitlines()[0] == (
            "conditions match"
        )

    @pytest.mark.parametrize("sides", ["both", "one"])
    def test_a_condition_with_no_reading_never_matches(self, sides):
        unread = REFERENCE_HOST.model_copy(
            update={"os_version": NoReading(argv=OS_VERSION_ARGV)}
        )
        previous = record(host=unread)
        current = record(host=unread if sides == "both" else REFERENCE_HOST)
        [difference] = compare_records(previous, current).differing_conditions
        assert difference.field == "os_version"


def _unit_cli_record(figures_per_run, **overrides):
    return record(
        tasks=[
            {"build:cli": task_figures(), "test:unit:cli": figures}
            for figures in figures_per_run
        ],
        **overrides,
    )


def _measured_unit_cli(durations, nextest=(None, None, None)):
    return _unit_cli_record(
        [
            task_figures(duration=duration, nextest=phase)
            for duration, phase in zip(durations, nextest, strict=True)
        ]
    )


_AGREEING_UNITS = (CargoUnits(compiling=61, checking=12),) * 3
_VARYING_UNITS = (
    CargoUnits(compiling=61, checking=12),
    CargoUnits(compiling=61, checking=12),
    CargoUnits(compiling=59, checking=12),
)


def _units_per_run(units_per_run):
    return [
        {"test:unit:cli": task_figures(duration=250.0, units=units)}
        for units in units_per_run
    ]


class TestTasksAndUnits:
    def test_the_task_set_difference_lists_added_and_removed_tasks(self):
        previous = record(
            tasks=[{"build:cli": task_figures(), "lint:cli": task_figures()}]
            * 3
        )
        current = record(
            tasks=[{"build:cli": task_figures(), "test:cli": task_figures()}]
            * 3
        )
        comparison = compare_records(previous, current)
        assert comparison.tasks_added == ("test:cli",)
        assert comparison.tasks_removed == ("lint:cli",)

    def test_a_named_task_gets_its_duration_and_nextest_changes(self):
        comparison = compare_records(
            worked_example_previous(),
            worked_example_current(),
            tasks=["test:unit:cli"],
        )
        assert comparison.named_tasks == (
            TaskComparison(
                "test:unit:cli",
                duration=MeasuredChange(
                    "test:unit:cli",
                    previous_median=297.1,
                    current_median=251.0,
                    noise=Noise.BEYOND,
                ),
                nextest_phase=MeasuredChange(
                    "test:unit:cli",
                    previous_median=149.2,
                    current_median=118.5,
                    noise=Noise.BEYOND,
                ),
            ),
        )

    def test_a_named_task_is_found_in_the_summary(self):
        isolated = {
            "mode": Mode.ISOLATED,
            "isolated_tasks": ("test:unit:cli",),
        }
        previous = record(
            tasks=[{"test:unit:cli": task_figures(duration=300.0)}] * 3,
            **isolated,
        ).model_copy(update={"task_set": ("default", "build:cli")})
        current = record(
            tasks=[{"test:unit:cli": task_figures(duration=250.0)}] * 3,
            **isolated,
        ).model_copy(update={"task_set": ("default", "build:cli")})
        [named] = compare_records(
            previous, current, tasks=["test:unit:cli"]
        ).named_tasks
        assert isinstance(named, TaskComparison)
        assert named.duration == MeasuredChange(
            "test:unit:cli",
            previous_median=300.0,
            current_median=250.0,
            noise=Noise.BEYOND,
        )

    @pytest.mark.parametrize("side", list(RecordSide))
    def test_a_named_task_flagged_on_either_side_is_named_in_words(self, side):
        failing = _unit_cli_record(
            [task_figures(TaskOutcome.FAILED, exit_status=3)] * 3
        )
        passing = _measured_unit_cli((290.0, 297.1, 300.0))
        previous, current = (
            (failing, passing)
            if side is RecordSide.PREVIOUS
            else (passing, failing)
        )
        [named] = compare_records(
            previous, current, tasks=["test:unit:cli"]
        ).named_tasks
        flagged = Flagged(caveats=(Caveat.FAILED,))
        expected = {
            RecordSide.PREVIOUS: (flagged, Measured()),
            RecordSide.CURRENT: (Measured(), flagged),
        }[side]
        assert named == TaskComparison(
            "test:unit:cli",
            duration=ChangeInPresence("test:unit:cli", *expected),
            nextest_phase=None,
        )

    def test_a_nextest_phase_on_one_side_only_is_named_in_words(self):
        previous = _measured_unit_cli(
            (290.0, 297.1, 300.0), (145.0, 149.2, 152.0)
        )
        current = _measured_unit_cli((248.0, 251.0, 255.0))
        [named] = compare_records(
            previous, current, tasks=["test:unit:cli"]
        ).named_tasks
        assert named.nextest_phase == FigureOnOneSide(
            "test:unit:cli", previous_median=149.2, current_median=None
        )

    def test_a_flagged_task_has_no_nextest_line(self):
        previous = _unit_cli_record([task_figures(TaskOutcome.SKIPPED)] * 3)
        current = _measured_unit_cli(
            (248.0, 251.0, 255.0), (115.0, 118.5, 121.0)
        )
        [named] = compare_records(
            previous, current, tasks=["test:unit:cli"]
        ).named_tasks
        assert named.nextest_phase is None
        assert named.duration == ChangeInPresence(
            "test:unit:cli",
            previous=Absent(absence=Absence.SKIPPED),
            current=Measured(),
        )

    @pytest.mark.parametrize("side", list(RecordSide))
    def test_a_named_task_missing_from_a_record_is_said_so(self, side):
        with_task = _measured_unit_cli((290.0, 297.1, 300.0))
        without_task = record()
        previous, current = (
            (without_task, with_task)
            if side is RecordSide.PREVIOUS
            else (with_task, without_task)
        )
        assert compare_records(
            previous, current, tasks=["test:unit:cli"]
        ).named_tasks == (TaskMissingFromOne("test:unit:cli", side),)

    @pytest.mark.parametrize(
        ("name", "closest"),
        [("test:unit:cl", "test:unit:cli"), ("zzzz", None)],
    )
    def test_a_task_in_neither_record_suggests_the_closest_name(
        self, name, closest
    ):
        previous = _measured_unit_cli((290.0, 297.1, 300.0))
        current = _measured_unit_cli((248.0, 251.0, 255.0))
        assert compare_records(previous, current, tasks=[name]).named_tasks == (
            TaskInNeither(name, closest),
        )

    def test_no_task_is_compared_unless_named(self):
        comparison = compare_records(
            worked_example_previous(), worked_example_current()
        )
        assert comparison.named_tasks == ()

    def test_unit_totals_are_compared_as_counts_without_a_noise_verdict(self):
        cold = CargoUnits(compiling=1275, checking=543, documenting=17)
        previous = worked_example_previous()
        current = record(
            tasks=with_unit_cli(
                (248.0, 251.0, 255.0), (115.0, 118.5, 121.0), units=cold
            ),
            host_settings=HostSettings.DEVTOOLS,
            run_id=DEVTOOLS_RUN_ID,
        )
        comparison = compare_records(previous, current)
        assert comparison.unit_totals == (
            UnitCountChange("compiling", 61, 1275),
            UnitCountChange("checking", 12, 543),
            UnitCountChange("documenting", 0, 17),
            UnitCountChange("total", 73, 1835),
        )
        lines = comparison_report(comparison).splitlines()
        assert "units compiling: 61 → 1275, +1214 (×20.902)" in lines
        assert "units documenting: 0 → 17, +17 (none in the 'none' record)" in (
            lines
        )

    @pytest.mark.parametrize(
        ("previous_units", "current_units", "varying"),
        [
            (_VARYING_UNITS, _AGREEING_UNITS, (RecordSide.PREVIOUS,)),
            (_AGREEING_UNITS, _VARYING_UNITS, (RecordSide.CURRENT,)),
            (
                _VARYING_UNITS,
                _VARYING_UNITS,
                (RecordSide.PREVIOUS, RecordSide.CURRENT),
            ),
        ],
        ids=["previous", "current", "both"],
    )
    def test_unit_totals_are_not_compared_when_a_records_runs_disagree(
        self, previous_units, current_units, varying
    ):
        previous = record(tasks=_units_per_run(previous_units))
        current = record(tasks=_units_per_run(current_units))
        assert compare_records(previous, current).unit_totals == (
            UnitsVaryAcrossRuns(varying)
        )
