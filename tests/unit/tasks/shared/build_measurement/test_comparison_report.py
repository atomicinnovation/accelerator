from dataclasses import replace

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
    RecordComparison,
    RecordSide,
    TaskComparison,
    TaskInNeither,
    TaskMissingFromOne,
    TrackedDifferently,
    UnitsVaryAcrossRuns,
    compare_records,
)
from tasks.shared.build_measurement.comparison_report import comparison_report
from tasks.shared.build_measurement.measured_build import HostSettings
from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.tracked_processes import IDE
from tests.unit.tasks.shared.build_measurement.compared_records import (
    DEVTOOLS_RUN_ID,
    NONE_RUN_ID,
    OS_VERSION_ARGV,
    cpu_records,
    record,
    worked_example_current,
    worked_example_previous,
)
from tests.unit.tasks.shared.build_measurement.doubles import REFERENCE_HOST


def _plain_comparison(**changes) -> RecordComparison:
    base = compare_records(worked_example_previous(), worked_example_current())
    return replace(base, **changes)


def _unit_cli_named(duration, nextest_phase=None):
    return TaskComparison("test:unit:cli", duration, nextest_phase)


_RENDERED = {
    "change-from-idle": (
        {
            "processes": (
                ChangeFromIdle(
                    "syspolicyd", current_median=3.2, noise=Noise.BEYOND
                ),
            )
        },
        "syspolicyd: 0.0 s → 3.2 s, +3.2 s (no ratio from zero), beyond noise",
    ),
    "figure-on-one-side": (
        {
            "named_tasks": (
                _unit_cli_named(
                    MeasuredChange("test:unit:cli", 297.1, 251.0, Noise.BEYOND),
                    FigureOnOneSide("test:unit:cli", 149.2, None),
                ),
            )
        },
        "test:unit:cli nextest phase: 149.2 s in the 'none' record, not in "
        "every 'devtools' run",
    ),
    "tracked-differently": (
        {
            "processes": (
                TrackedDifferently(IDE, ("/MacOS/idea",), ("/MacOS/clion",)),
            )
        },
        "ide: not compared, tracked by different patterns ('none': "
        "/MacOS/idea; 'devtools': /MacOS/clion)",
    ),
    "tracked-on-one-side": (
        {
            "processes": (
                TrackedDifferently("fsnotifier", ("/fsnotifier",), ()),
            )
        },
        "fsnotifier: not compared, tracked by different patterns ('none': "
        "/fsnotifier; 'devtools': not tracked)",
    ),
    "task-missing-from-one": (
        {
            "named_tasks": (
                TaskMissingFromOne("test:unit:cli", RecordSide.CURRENT),
            )
        },
        "test:unit:cli: not in the 'devtools' record",
    ),
    "task-in-neither-with-closest": (
        {"named_tasks": (TaskInNeither("test:unit:cl", "test:unit:cli"),)},
        "test:unit:cl: in neither record (closest: 'test:unit:cli')",
    ),
    "task-in-neither-without-closest": (
        {"named_tasks": (TaskInNeither("zzzz", None),)},
        "zzzz: in neither record (no close match)",
    ),
    "flagged-side": (
        {
            "named_tasks": (
                _unit_cli_named(
                    ChangeInPresence(
                        "test:unit:cli",
                        Measured(),
                        Flagged(caveats=(Caveat.FAILED, Caveat.NOT_RUN)),
                    )
                ),
            )
        },
        "test:unit:cli duration: measured in every 'none' run; flagged "
        "failed, not-run in the 'devtools' record",
    ),
    "unjudged": (
        {
            "wall_clock": MeasuredChange(
                "wall-clock", 309.4, 262.0, Noise.UNJUDGED
            )
        },
        "wall-clock: 309.4 s → 262.0 s, -47.4 s (×0.847), no spread to judge "
        "noise (1 run)",
    ),
    "task-set-difference": (
        {"tasks_added": ("test:cli",), "tasks_removed": ("lint:cli", "x")},
        "task set: added test:cli; removed lint:cli, x",
    ),
    "silent-on-both-sides": (
        {
            "named_tasks": (
                _unit_cli_named(
                    ChangeInPresence(
                        "test:unit:cli",
                        Absent(absence=Absence.SILENT),
                        Absent(absence=Absence.SILENT),
                    )
                ),
            )
        },
        "test:unit:cli duration: printed nothing in every 'none' and "
        "'devtools' run",
    ),
    "absent-differently-on-each-side": (
        {
            "named_tasks": (
                _unit_cli_named(
                    ChangeInPresence(
                        "test:unit:cli",
                        Absent(absence=Absence.SKIPPED),
                        Absent(absence=Absence.NOT_SEEN),
                    )
                ),
            )
        },
        "test:unit:cli duration: skipped in every 'none' run; did not run in "
        "any 'devtools' run",
    ),
    "figure-on-the-current-side-only": (
        {
            "named_tasks": (
                _unit_cli_named(
                    MeasuredChange("test:unit:cli", 297.1, 251.0, Noise.BEYOND),
                    FigureOnOneSide("test:unit:cli", None, 118.5),
                ),
            )
        },
        "test:unit:cli nextest phase: 118.5 s in the 'devtools' record, not "
        "in every 'none' run",
    ),
    "units-vary-on-one-side": (
        {"unit_totals": UnitsVaryAcrossRuns((RecordSide.PREVIOUS,))},
        "units: not compared, they vary across the 'none' record's runs",
    ),
    "units-vary-on-both-sides": (
        {
            "unit_totals": UnitsVaryAcrossRuns(
                (RecordSide.PREVIOUS, RecordSide.CURRENT)
            )
        },
        "units: not compared, they vary across the 'none' and 'devtools' "
        "records' runs",
    ),
    "skipped-on-both-sides": (
        {
            "named_tasks": (
                _unit_cli_named(
                    ChangeInPresence(
                        "test:unit:cli",
                        Absent(absence=Absence.SKIPPED),
                        Absent(absence=Absence.SKIPPED),
                    )
                ),
            )
        },
        "test:unit:cli duration: skipped in every 'none' and 'devtools' run",
    ),
}


_WORKED_EXAMPLE = f"""\
conditions differ: host_settings (none → devtools)
previous ('none'): {NONE_RUN_ID} (3 runs)
current ('devtools'): {DEVTOOLS_RUN_ID} (3 runs)
wall-clock: 309.4 s → 262.0 s, -47.4 s (×0.847), beyond noise
syspolicyd: 51.3 s → 12.0 s, -39.3 s (×0.234), beyond noise
XprotectService: did not run in any 'none' or 'devtools' run
ide: 301.0 s → 296.4 s, -4.6 s (×0.985), within noise
fsnotifier: 12.4 s → 12.1 s, -0.3 s (×0.976), within noise
task set: unchanged
test:unit:cli duration: 297.1 s → 251.0 s, -46.1 s (×0.845), beyond noise
test:unit:cli nextest phase: 149.2 s → 118.5 s, -30.7 s (×0.794), beyond noise
units compiling: 61 → 61, +0 (×1.000)
units checking: 12 → 12, +0 (×1.000)
units documenting: none in either record
units total: 73 → 73, +0 (×1.000)
"""


class TestReport:
    @pytest.mark.parametrize(
        ("changes", "line"), list(_RENDERED.values()), ids=list(_RENDERED)
    )
    def test_the_report_renders_every_change_in_words(self, changes, line):
        report = comparison_report(_plain_comparison(**changes))
        assert line in report.splitlines()

    def test_the_report_names_both_records_by_run_id(self):
        previous = record(wall_clocks=(309.4,))
        current = record(run_id=DEVTOOLS_RUN_ID)
        lines = comparison_report(compare_records(previous, current))
        assert lines.splitlines()[1:3] == [
            f"previous: {NONE_RUN_ID} (1 run)",
            f"current: {DEVTOOLS_RUN_ID} (3 runs)",
        ]

    @pytest.mark.parametrize(
        ("previous_version", "expected"),
        [
            (
                "26.4",
                "os_version (unreadable (sw_vers -productVersion) → 26.4)",
            ),
            (None, "os_version (unreadable in both records)"),
        ],
        ids=["one", "both"],
    )
    def test_a_condition_with_no_reading_renders_its_command(
        self, previous_version, expected
    ):
        unread = NoReading(argv=OS_VERSION_ARGV)
        current_version = previous_version or unread
        previous = record(
            host=REFERENCE_HOST.model_copy(update={"os_version": unread})
        )
        current = record(
            host=REFERENCE_HOST.model_copy(
                update={"os_version": current_version}
            )
        )
        first_line = comparison_report(
            compare_records(previous, current)
        ).splitlines()[0]
        assert first_line == f"conditions differ: {expected}"

    def test_a_tuple_condition_renders_comma_separated(self):
        previous = record()
        current = record(isolated_tasks=("test:unit:cli", "lint:cli"))
        first_line = comparison_report(
            compare_records(previous, current)
        ).splitlines()[0]
        assert first_line == (
            "conditions differ: isolated_tasks (empty → test:unit:cli, "
            "lint:cli)"
        )

    def test_several_differing_conditions_are_separated_by_semicolons(self):
        previous = record()
        current = record(
            cache_state=BuildCacheState.COLD,
            host_settings=HostSettings.DEVTOOLS,
        )
        first_line = comparison_report(
            compare_records(previous, current)
        ).splitlines()[0]
        assert first_line == (
            "conditions differ: cache_state (warm-no-op → cold); "
            "host_settings (none → devtools)"
        )

    def test_records_with_one_label_are_named_previous_and_current(self):
        previous, current = cpu_records(
            [(10.0, 100.0, 5.0)] * 3,
            [(10.0, 100.0, 5.0)] * 3,
            host_settings=HostSettings.NONE,
        )
        assert (
            "XprotectService: did not run in any previous or current run"
            in comparison_report(compare_records(previous, current))
        )

    def test_records_with_different_labels_introduce_each_label(self):
        comparison = compare_records(
            record(wall_clocks=(309.4,)),
            record(host_settings=HostSettings.DEVTOOLS, run_id=DEVTOOLS_RUN_ID),
        )
        assert comparison_report(comparison).splitlines()[1:3] == [
            f"previous ('none'): {NONE_RUN_ID} (1 run)",
            f"current ('devtools'): {DEVTOOLS_RUN_ID} (3 runs)",
        ]

    def test_the_report_matches_the_worked_example(self):
        comparison = compare_records(
            worked_example_previous(),
            worked_example_current(),
            tasks=["test:unit:cli"],
        )
        assert comparison_report(comparison) == _WORKED_EXAMPLE
