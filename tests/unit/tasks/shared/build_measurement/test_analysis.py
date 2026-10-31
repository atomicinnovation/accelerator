import json
from pathlib import Path

from tasks.shared.build_measurement.analysis import (
    BuildResult,
    CapturedInvocation,
    CapturedRun,
    CapturedSession,
    analyse_session,
)
from tasks.shared.build_measurement.fetch_failures import ExternalInput
from tasks.shared.build_measurement.measured_build import (
    CommitMatch,
    PrimingRun,
)
from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.settings import BuildSettings
from tasks.shared.build_measurement.task_graph import TaskGraph
from tasks.shared.build_measurement.timeline import StampedLine, TaskOutcome
from tests.unit.tasks.shared.build_measurement.doubles import (
    REFERENCE_HOST,
    REFERENCE_MISE,
    STARTED_AT,
)

_GRAPH = TaskGraph.parse(
    json.dumps(
        [
            {"name": "default", "depends": ["report", "test"], "run": []},
            {"name": "build", "depends": [], "run": ["invoke build"]},
            {"name": "test", "depends": ["build"], "run": ["invoke test"]},
            {"name": "report", "depends": ["test"], "run": ["invoke report"]},
        ]
    )
)
_SETTINGS = BuildSettings.of(
    root=Path("/checkout"),
    runs=1,
    keep=False,
    host_settings="devtools",
    ide_patterns=(),
)


def _lines(*stamped: tuple[float, str]) -> tuple[StampedLine, ...]:
    return tuple(StampedLine(at=at, text=text) for at, text in stamped)


_CLEAN = _lines(
    (0.0, "[build] $ invoke build"),
    (2.0, "[build] Finished in 2.0s"),
    (2.5, "[test] $ invoke test"),
    (5.5, "[test] Finished in 3.0s"),
    (5.6, "[report] $ invoke report"),
    (6.0, "[report] Finished in 0.4s"),
)
_TEST_FAILS = _lines(
    (0.0, "[build] $ invoke build"),
    (2.0, "[build] Finished in 2.0s"),
    (2.5, "[test] $ invoke test"),
    (4.0, "[test] boom"),
    (5.6, "[report] $ invoke report"),
    (6.0, "[report] Finished in 0.4s"),
    (6.1, "[test] exited with status 3"),
)


def _run(
    number: int = 1,
    lines: tuple[StampedLine, ...] = _CLEAN,
    *,
    exit_status: int = 0,
    ready_from: float = 0.0,
    process_lines: tuple[StampedLine, ...] = (),
    cpu_lines: tuple[StampedLine, ...] = (),
) -> CapturedRun:
    return CapturedRun(
        number=number,
        invocations=(
            CapturedInvocation(
                result=BuildResult(
                    lines=lines, exit_status=exit_status, wall_clock_seconds=6.0
                ),
                graph=_GRAPH,
                reported=("build", "report", "test"),
                ready_from=ready_from,
            ),
        ),
        cpu_lines=cpu_lines,
        process_lines=process_lines,
        wall_clock_seconds=6.5,
    )


def _session(*runs: CapturedRun) -> CapturedSession:
    return CapturedSession(
        run_id="build_2026-10-30T162000Z_engywook_warm-no-op_devtools",
        run_directory=Path(".accelerator-measure/build/x"),
        started_at=STARTED_AT,
        settings=_SETTINGS,
        commit=CommitMatch.of("c", at_start=True, at_end=True),
        host=REFERENCE_HOST,
        mise=REFERENCE_MISE,
        idle_load_percent=12.0,
        task_set=("build", "report", "test"),
        priming=PrimingRun(wall_clock_seconds=7.0, discarded=True),
        runs=runs or (_run(),),
    )


def test_a_clean_run_has_every_reported_task_succeeding():
    [run] = analyse_session(_session()).runs
    assert {task: f.outcome for task, f in run.tasks.items()} == {
        "build": TaskOutcome.SUCCEEDED,
        "report": TaskOutcome.SUCCEEDED,
        "test": TaskOutcome.SUCCEEDED,
    }
    assert not run.contains_failure
    assert run.wall_clock_seconds == 6.5


def test_a_failure_in_one_run_is_reported_and_its_dependants_flagged():
    measured = analyse_session(_session(_run(lines=_TEST_FAILS, exit_status=3)))
    [run] = measured.runs
    assert run.tasks["test"].outcome is TaskOutcome.FAILED
    assert run.tasks["test"].exit_status == 3
    assert run.tasks["report"].after_failure
    assert run.contains_failure
    assert measured.failed_tasks == ("test",)


def test_a_non_zero_mise_exit_marks_the_run_even_with_no_failed_task():
    [run] = analyse_session(_session(_run(lines=(), exit_status=2))).runs
    assert run.contains_failure


def test_a_task_not_run_marks_the_run_even_when_mise_exits_zero():
    lines = _CLEAN[:2]
    measured = analyse_session(_session(_run(lines=lines)))
    [run] = measured.runs
    assert run.tasks["test"].outcome is TaskOutcome.NOT_RUN
    assert run.contains_failure
    assert "test" in measured.failed_tasks


def test_each_invocation_is_placed_from_its_own_ready_time():
    [run] = analyse_session(_session(_run(ready_from=-1.0))).runs
    assert run.tasks["build"].queueing_delay_seconds == 1.0


def test_a_task_failing_only_in_run_two_keeps_three_values_no_spread():
    measured = analyse_session(
        _session(
            _run(1),
            _run(2, lines=_TEST_FAILS, exit_status=3),
            _run(3),
        )
    )
    assert [run.tasks["test"].outcome for run in measured.runs] == [
        TaskOutcome.SUCCEEDED,
        TaskOutcome.FAILED,
        TaskOutcome.SUCCEEDED,
    ]
    assert [run.contains_failure for run in measured.runs] == [
        False,
        True,
        False,
    ]
    assert measured.summary.tasks["test"].duration_seconds is None
    assert measured.summary.wall_clock_seconds.median == 6.5


def test_tracked_process_cpu_is_attributed_per_run():
    process_lines = _lines(
        (0.0, "sample"),
        (0.0, "  479     1   3:21.94 /usr/libexec/syspolicyd"),
        (1.0, "sample"),
        (1.0, "  479     1   3:24.94 /usr/libexec/syspolicyd"),
    )
    [run] = analyse_session(_session(_run(process_lines=process_lines))).runs
    assert run.process_cpu_seconds["syspolicyd"] == 3.0
    assert run.process_cpu_seconds["ide"] is None


def test_a_ps_gap_leaves_the_runs_process_cpu_unread():
    process_lines = (
        StampedLine(at=0.0, text="sample"),
        StampedLine(at=1.0, text="gap"),
    )
    [run] = analyse_session(_session(_run(process_lines=process_lines))).runs
    assert all(
        isinstance(cpu, NoReading) for cpu in run.process_cpu_seconds.values()
    )


def test_cpu_buckets_cover_the_runs_wall_clock():
    cpu_lines = _lines(
        (0.0, "  us sy id"),
        (0.0, " 1 2 3 4 5 6 7 8 9"),
        (2.0, " 1 2 3 4 50 10 40 1 1 1"),
    )
    [run] = analyse_session(_session(_run(cpu_lines=cpu_lines))).runs
    [bucket] = run.cpu_buckets
    assert bucket.start_seconds == 0.0
    assert bucket.busy_percent == 60.0


def test_a_fetch_failure_names_its_run():
    lines = (*_CLEAN, StampedLine(at=6.0, text="[report] curl: (6) no host"))
    measured = analyse_session(_session(_run(2, lines=lines)))
    [failure] = measured.fetch_failures
    assert (failure.run, failure.task, failure.external_input) == (
        2,
        "report",
        ExternalInput.LAUNCHER_RELEASE,
    )


def test_the_record_carries_the_sessions_provenance_and_settings():
    measured = analyse_session(_session())
    assert measured.conditions.host_settings == "devtools"
    assert measured.conditions.mode == "full"
    assert measured.commit.matched
    assert measured.idle_load_percent == 12.0
    assert measured.task_set == ("build", "report", "test")
    assert measured.priming == PrimingRun(
        wall_clock_seconds=7.0, discarded=True
    )
    assert set(measured.tracked_processes) == {
        "syspolicyd",
        "XprotectService",
        "ide",
        "fsnotifier",
    }
