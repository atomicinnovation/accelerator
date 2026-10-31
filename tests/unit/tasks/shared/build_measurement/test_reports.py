from dataclasses import replace
from pathlib import Path

import pytest

from tasks.shared.build_measurement.fetch_failures import (
    ExternalInput,
    FetchFailure,
)
from tasks.shared.build_measurement.measured_build import (
    CommitMatch,
    HostSettings,
)
from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.reports import (
    Preflight,
    SessionOutcome,
    preflight_report,
    run_end_line,
    summary_report,
)
from tasks.shared.build_measurement.timeline import TaskOutcome
from tasks.shared.build_measurement.tracked_processes import (
    DEFAULT_TRACKED_PROCESSES,
)
from tests.unit.tasks.shared.build_measurement.doubles import (
    measured_build,
    run_figures,
    task_figures,
)

_ROOT = Path("/checkout")
_RUN_DIRECTORY = (
    _ROOT
    / ".accelerator-measure/build"
    / "build_2026-10-30T162000Z_engywook_warm-no-op_devtools"
)
_RECORD = _RUN_DIRECTORY / "record.json"
_KEPT = (
    _ROOT
    / "meta/measurements"
    / "build_2026-10-30T162000Z_engywook_warm-no-op_devtools.json"
)
_ADVISORY = FetchFailure(
    run=2,
    task="deny:check",
    external_input=ExternalInput.ADVISORY_DATABASE,
    line="[ERROR] failed to get 'FETCH_HEAD' metadata",
)


def _summary(measured=None, *, keep=False, kept=None):
    return summary_report(
        SessionOutcome(measured=measured or measured_build(), keep=keep),
        record_path=_RECORD,
        kept_path=kept,
    )


def _three_runs(*, failing_in_two=None, process_cpu=None, **run_two):
    def tasks(index):
        if index == 2 and failing_in_two:
            return {
                failing_in_two: task_figures(TaskOutcome.FAILED, exit_status=3),
                "report": task_figures(after_failure=True),
            }
        return {
            failing_in_two or "build:cli": task_figures(),
            "report": task_figures(),
        }

    return [
        run_figures(
            index,
            tasks=tasks(index),
            exit_status=3 if index == 2 and failing_in_two else 0,
            wall_clock=wall_clock,
            process_cpu=process_cpu(index) if process_cpu else None,
            **(run_two if index == 2 else {}),
        )
        for index, wall_clock in ((1, 309.4), (2, 305.1), (3, 322.0))
    ]


class TestSummaryReport:
    def test_the_summary_opens_with_the_run_id_and_conditions(self):
        assert _summary().splitlines()[0] == (
            "measure:build build_2026-10-30T162000Z_engywook_warm-no-op_"
            "devtools (warm-no-op, devtools, 1 run)"
        )

    def test_the_summary_gives_the_idle_load(self):
        assert "idle load: 26.8%" in _summary().splitlines()
        assert (
            "idle load: not sampled"
            in _summary(measured_build(idle_load_percent=None)).splitlines()
        )

    def test_the_summary_gives_the_wall_clock_spread_and_record_paths(self):
        lines = _summary(
            measured_build(runs=_three_runs()), keep=True, kept=_KEPT
        ).splitlines()
        assert "wall-clock: median 309.4 s (min 305.1, max 322.0)" in lines
        assert f"record: {_RECORD}" in lines
        assert f"kept: {_KEPT}" in lines

    @pytest.mark.parametrize(
        ("at_start", "at_end", "expected"),
        [
            (True, True, "working copy matched at start and end"),
            (False, True, "working copy did not match at start"),
            (True, False, "working copy did not match at end"),
            (True, None, "working copy match at end unknown"),
        ],
    )
    def test_the_summary_says_whether_the_working_copy_matched(
        self, at_start, at_end, expected
    ):
        commit = CommitMatch.of(
            "d55cad6d85aa", at_start=at_start, at_end=at_end
        )
        [line] = [
            line
            for line in _summary(measured_build(commit=commit)).splitlines()
            if line.startswith("commit:")
        ]
        assert line.startswith("commit: d55cad6d85aa, ")
        assert expected in line

    def test_the_summary_gives_each_tracked_processes_cpu(self):
        runs = _three_runs(
            process_cpu=lambda index: {
                "syspolicyd": 51.3,
                "XprotectService": None,
                "ide": 301.0,
                "fsnotifier": 12.4,
            }
        )
        assert (
            "processes: syspolicyd 51.3 s, XprotectService absent, "
            "ide 301.0 s, fsnotifier 12.4 s"
        ) in _summary(measured_build(runs=runs)).splitlines()

    @pytest.mark.parametrize("name", ["syspolicyd", "ide", "fsnotifier"])
    def test_the_summary_hints_a_retake_when_a_required_process_was_absent(
        self, name
    ):
        process_cpu = {each.name: 1.0 for each in DEFAULT_TRACKED_PROCESSES}
        runs = [run_figures(process_cpu=process_cpu | {name: None})]
        retakes = [
            line
            for line in _summary(measured_build(runs=runs)).splitlines()
            if line.startswith("retake:")
        ]
        assert retakes == [
            f"retake: {name} was absent; a baseline or condition record "
            "needs it measured in every run"
        ]

    def test_an_absent_xprotect_service_needs_no_retake(self):
        process_cpu = {each.name: 1.0 for each in DEFAULT_TRACKED_PROCESSES}
        runs = [
            run_figures(process_cpu=process_cpu | {"XprotectService": None})
        ]
        assert "retake:" not in _summary(measured_build(runs=runs))

    def test_an_unreadable_required_process_needs_a_retake(self):
        process_cpu = {each.name: 1.0 for each in DEFAULT_TRACKED_PROCESSES}
        runs = _three_runs(
            process_cpu=lambda index: (
                process_cpu
                | ({"ide": NoReading(argv=("ps",))} if index == 2 else {})
            )
        )
        assert (
            "retake: ide was flagged unreadable; a baseline or condition "
            "record needs it measured in every run"
        ) in _summary(measured_build(runs=runs)).splitlines()

    def test_the_summary_names_each_failed_task(self):
        lines = _summary(
            measured_build(runs=_three_runs(failing_in_two="deny:check"))
        ).splitlines()
        assert (
            f"failed: deny:check (run 2, see {_RUN_DIRECTORY}/run-2.log) — "
            "downstream flagged: 1 task"
        ) in lines

    def test_a_clean_session_names_no_failure(self):
        assert "failed:" not in _summary()

    def test_the_summary_warns_of_output_contract_drift(self):
        runs = [
            run_figures(
                tasks={
                    "build:cli": task_figures(),
                    "lint:check": task_figures(TaskOutcome.NOT_RUN),
                }
            )
        ]
        assert (
            "warning: mise exited 0 in run 1, yet lint:check has no verdict; "
            "mise 2026.10.8 macos-arm64 (2026-10-11) may have changed its "
            "output"
        ) in _summary(measured_build(runs=runs)).splitlines()

    def test_the_summary_says_a_record_with_fetch_failures_was_not_kept(self):
        lines = _summary(
            measured_build(runs=_three_runs(fetch_failures=[_ADVISORY])),
            keep=True,
        ).splitlines()
        assert lines[-1] == (
            "not kept: 1 fetch failure (advisory-database in run 2); retake"
        )

    def test_the_summary_names_fetch_failures_without_keep_too(self):
        lines = _summary(
            measured_build(runs=_three_runs(fetch_failures=[_ADVISORY]))
        ).splitlines()
        assert "fetch failures: advisory-database (deny:check, run 2)" in lines
        assert not any(line.startswith("not kept:") for line in lines)


def _preflight(**overrides) -> Preflight:
    return replace(
        Preflight(
            commit="d55cad6d85",
            differing_paths=(),
            host_settings=HostSettings.DEVTOOLS,
            power_source="AC Power",
            mise_jobs=8,
            tracked=DEFAULT_TRACKED_PROCESSES,
            running=("syspolicyd", "ide", "fsnotifier"),
            ide_commands=(
                "/Applications/IntelliJ IDEA.app/Contents/MacOS/idea",
            ),
            keep=True,
        ),
        **overrides,
    )


def _warnings(preflight: Preflight) -> list[str]:
    return [
        line
        for line in preflight_report(preflight).splitlines()
        if line.startswith("warning:")
    ]


class TestPreflightReport:
    def test_the_preflight_report_shows_the_conditions(self):
        assert preflight_report(_preflight()).splitlines() == [
            "preflight:",
            "  commit: d55cad6d85, working copy matches",
            "  host settings: devtools",
            "  power: AC Power",
            "  mise jobs: 8",
            "  tracked processes running: syspolicyd, ide, fsnotifier",
            "  tracked processes not running: XprotectService",
            "  IDE command lines matched:",
            "    /Applications/IntelliJ IDEA.app/Contents/MacOS/idea",
        ]

    def test_a_mismatch_names_up_to_five_differing_paths(self):
        paths = tuple(f"file-{index}" for index in range(7))
        [line] = [
            line
            for line in preflight_report(
                _preflight(differing_paths=paths, keep=False)
            ).splitlines()
            if "commit:" in line
        ]
        assert line == (
            "  commit: d55cad6d85, working copy differs in 7 paths: "
            "file-0, file-1, file-2, file-3, file-4, …"
        )

    def test_a_clean_preflight_with_keep_warns_of_nothing(self):
        assert _warnings(_preflight()) == []

    @pytest.mark.parametrize(
        ("overrides", "warning"),
        [
            (
                {"host_settings": HostSettings.UNSPECIFIED},
                "warning: with --keep, the host settings are unspecified; "
                "pass --host-settings",
            ),
            (
                {"differing_paths": ("tasks/measure.py",)},
                "warning: with --keep, the working copy does not match the "
                "measured commit",
            ),
            (
                {"power_source": "Battery Power"},
                "warning: with --keep, the host is on Battery Power, not "
                "AC Power",
            ),
            (
                {"running": ("ide", "fsnotifier")},
                "warning: with --keep, syspolicyd is not running",
            ),
            (
                {"running": ("syspolicyd", "fsnotifier"), "ide_commands": ()},
                "warning: with --keep, ide is not running",
            ),
            (
                {"running": ("syspolicyd", "ide")},
                "warning: with --keep, fsnotifier is not running",
            ),
        ],
        ids=["unspecified", "mismatch", "battery", "syspolicyd", "ide", "fs"],
    )
    def test_the_preflight_warns_of_each_condition_a_kept_record_fails(
        self, overrides, warning
    ):
        assert _warnings(_preflight(**overrides)) == [warning]
        assert _warnings(_preflight(**overrides, keep=False)) == []


class TestRunEndLine:
    def test_a_clean_run_gives_its_wall_clock(self):
        assert (
            run_end_line(1, 3, 312.44, failed=(), fetch_failures=(), keep=True)
            == "timed run 1/3: 312.4 s, no task failed"
        )

    def test_failed_tasks_are_named(self):
        assert run_end_line(
            1, 3, 312.4, failed=("deny:check",), fetch_failures=(), keep=False
        ) == ("timed run 1/3: 312.4 s, 1 failed (deny:check)")

    def test_a_fetch_failure_is_reported_at_the_end_of_its_run(self):
        assert run_end_line(
            1,
            3,
            312.4,
            failed=("deny:check",),
            fetch_failures=(replace_run(_ADVISORY, 1),),
            keep=True,
        ) == (
            "timed run 1/3: 312.4 s, 1 failed (deny:check); fetch failure "
            "(advisory-database): with --keep this record will not be kept; "
            "press Ctrl-C to stop, then rerun"
        )

    @pytest.mark.parametrize(("number", "total"), [(3, 3), (1, 1)])
    def test_the_last_runs_fetch_failure_gives_no_ctrl_c_hint(
        self, number, total
    ):
        line = run_end_line(
            number,
            total,
            312.4,
            failed=(),
            fetch_failures=(replace_run(_ADVISORY, number),),
            keep=True,
        )
        assert line.endswith(
            "fetch failure (advisory-database): with --keep this record will "
            "not be kept"
        )

    def test_without_keep_a_fetch_failure_is_only_named(self):
        assert run_end_line(
            1,
            3,
            312.4,
            failed=(),
            fetch_failures=(replace_run(_ADVISORY, 1),),
            keep=False,
        ).endswith("no task failed; fetch failure (advisory-database)")


def replace_run(failure: FetchFailure, run: int) -> FetchFailure:
    return failure.model_copy(update={"run": run})
