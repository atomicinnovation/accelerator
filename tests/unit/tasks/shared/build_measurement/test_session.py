import datetime as dt

import pytest

from tasks.shared.build_measurement.errors import (
    HistoryError,
    SessionInProgressError,
    UnsupportedPlatformError,
)
from tasks.shared.build_measurement.locations import RUN_DIRECTORY_ROOT
from tasks.shared.build_measurement.session import (
    IDLE_SAMPLE_SECONDS,
    OrphanedBuild,
    OrphanedBuildError,
    measure_build,
)
from tasks.shared.build_measurement.timeline import TaskOutcome
from tasks.shared.build_measurement.tracked_processes import (
    DEFAULT_TRACKED_PROCESSES,
    PS_ARGV,
)
from tests.unit.tasks.shared.build_measurement.doubles import (
    ADVISORY_FAILS,
    CLEAN_BUILD,
    IDE_COMMAND,
    MEASURED_COMMIT,
    TEST_FAILS,
    BuildScript,
    FakeHistory,
    FakeSession,
    untouchable_ports,
)


@pytest.fixture
def session(tmp_path):
    return FakeSession(tmp_path)


def _index(events, prefix):
    return next(i for i, event in enumerate(events) if event.startswith(prefix))


class TestOpening:
    def test_a_non_macos_host_is_refused_before_anything_runs(self, tmp_path):
        with pytest.raises(UnsupportedPlatformError, match="macOS"):
            measure_build(
                FakeSession(tmp_path).settings(),
                untouchable_ports(system=lambda: "Linux"),
            )
        assert not (tmp_path / RUN_DIRECTORY_ROOT).exists()

    def test_the_session_opens_in_one_order(self, session):
        session.measure(runs=1)
        events = session.events
        assert events[:3] == ["platform", "lock", "orphan check"]
        assert events[3].startswith("progress run directory:")

    def test_a_held_lock_refuses_before_anything_else_runs(self, tmp_path):
        session = FakeSession(
            tmp_path, lock_refusal=SessionInProgressError("busy")
        )
        with pytest.raises(SessionInProgressError):
            session.measure()
        assert session.events == ["platform", "lock"]
        assert not (tmp_path / RUN_DIRECTORY_ROOT).exists()

    def test_an_orphaned_build_refuses_while_the_lock_is_held(self, tmp_path):
        orphan = OrphanedBuild(
            pgid=4242,
            members=((4243, "cargo nextest run"),),
            path=tmp_path / "build.pgid",
        )
        session = FakeSession(tmp_path, orphan=orphan)
        with pytest.raises(OrphanedBuildError) as refusal:
            session.measure()
        assert session.events == ["platform", "lock", "orphan check", "unlock"]
        assert not (tmp_path / RUN_DIRECTORY_ROOT).exists()
        message = str(refusal.value)
        assert "4243 cargo nextest run" in message
        assert message.index("kill -TERM -4242") < message.index(
            "kill -KILL -4242"
        )
        assert message.endswith(f"if none is, delete {tmp_path / 'build.pgid'}")

    def test_the_session_lock_is_held_from_start_to_finish(self, session):
        session.measure(runs=1)
        events = session.events
        assert events.index("lock") < _index(events, "progress run directory")
        assert events[-1] == "unlock"


class TestRunDirectory:
    def test_the_run_id_carries_the_hostname_and_whole_seconds(self, session):
        measured = session.measure(runs=1).measured
        assert measured.run_id == (
            "build_2026-10-30T162000Z_engywook_warm-no-op_devtools"
        )
        assert measured.started_at == dt.datetime(
            2026, 10, 30, 16, 20, tzinfo=dt.UTC
        )
        assert measured.run_directory == RUN_DIRECTORY_ROOT / measured.run_id

    @pytest.mark.parametrize(
        "scutil", [FileNotFoundError("scutil"), ""], ids=["raises", "empty"]
    )
    def test_the_run_takes_the_kernel_hostname_when_scutil_has_none(
        self, tmp_path, scutil
    ):
        session = FakeSession(tmp_path)
        session.probes[("scutil", "--get", "LocalHostName")] = scutil
        measured = session.measure(runs=1).measured
        assert measured.run_id.split("_")[2] == "kernel-host"

    def test_each_log_is_written_as_its_run_happens(self, session):
        session.measure(runs=2)
        assert sorted(p.name for p in session.run_directory.iterdir()) == [
            "idle-iostat.log",
            "priming.log",
            "run-1-iostat.log",
            "run-1-ps.log",
            "run-1.log",
            "run-2-iostat.log",
            "run-2-ps.log",
            "run-2.log",
        ]
        assert "[build] $ invoke build" in (
            (session.run_directory / "run-2.log").read_text()
        )


class TestBuilds:
    def test_idle_load_is_sampled_for_30_seconds_before_priming(self, session):
        session.measure(runs=1)
        events = session.events
        assert IDLE_SAMPLE_SECONDS == 30.0
        assert session.clock.sleeps[0] == 30.0
        assert (
            events.index("start cpu idle-iostat.log")
            < events.index("stop cpu idle-iostat.log")
            < events.index("build priming.log")
        )

    def test_a_warm_session_primes_then_runs_n_timed_builds(self, session):
        measured = session.measure(runs=3).measured
        assert [log.name for _, log in session.build.calls] == [
            "priming.log",
            "run-1.log",
            "run-2.log",
            "run-3.log",
        ]
        assert measured.priming is not None
        assert measured.priming.wall_clock_seconds == 6.5
        assert measured.priming.discarded
        assert len(measured.runs) == 3

    def test_the_default_build_command(self, session):
        session.measure(runs=2)
        assert {argv for argv, _ in session.build.calls} == {
            (
                "mise",
                "run",
                "--output",
                "prefix",
                "--continue-on-error",
                "default",
            )
        }

    def test_each_invocation_is_ready_from_its_own_start(self, tmp_path):
        session = FakeSession(tmp_path, sampler_start_delay=0.5)
        measured = session.measure(runs=1).measured
        assert measured.runs[0].tasks["build"].queueing_delay_seconds == 0.0

    def test_samplers_bracket_each_timed_run_only(self, session):
        session.measure(runs=2)
        events = [
            event
            for event in session.events
            if event.startswith(("start", "stop", "build"))
        ]
        assert events == [
            "start cpu idle-iostat.log",
            "stop cpu idle-iostat.log",
            "build priming.log",
            "start cpu run-1-iostat.log",
            "start processes run-1-ps.log",
            "build run-1.log",
            "stop cpu run-1-iostat.log",
            "stop processes run-1-ps.log",
            "start cpu run-2-iostat.log",
            "start processes run-2-ps.log",
            "build run-2.log",
            "stop cpu run-2-iostat.log",
            "stop processes run-2-ps.log",
        ]

    def test_the_process_sampler_is_given_the_settings_tracked_set(
        self, session
    ):
        settings = session.settings(runs=2, ide_patterns=["/MacOS/clion"])
        measure_build(settings, session.ports())
        assert {start.tracked for start in session.process_sampler.started} == {
            settings.tracked
        }

    def test_a_failure_in_one_run_is_reported_and_the_session_completes(
        self, tmp_path
    ):
        session = FakeSession(
            tmp_path,
            scripts=[CLEAN_BUILD, CLEAN_BUILD, TEST_FAILS, CLEAN_BUILD],
        )
        measured = session.measure(runs=3).measured
        second = measured.runs[1]
        assert second.tasks["test"].outcome is TaskOutcome.FAILED
        assert second.tasks["report"].after_failure
        assert second.contains_failure
        assert [run.contains_failure for run in measured.runs] == [
            False,
            True,
            False,
        ]
        assert measured.summary.wall_clock_seconds.median == 6.5

    def test_a_non_zero_mise_exit_marks_the_run_even_with_no_failed_task(
        self, tmp_path
    ):
        session = FakeSession(
            tmp_path,
            scripts=[CLEAN_BUILD, BuildScript(exit_status=2, wall_clock=1.0)],
        )
        [run] = session.measure(runs=1).measured.runs
        assert run.contains_failure
        assert {figures.outcome for figures in run.tasks.values()} == {
            TaskOutcome.NOT_RUN
        }

    def test_the_task_set_is_the_default_closure(self, session):
        assert session.measure(runs=1).measured.task_set == (
            "build",
            "report",
            "test",
        )


class TestInterruption:
    def test_an_interrupt_in_run_two_keeps_the_logs_and_writes_no_record(
        self, tmp_path
    ):
        session = FakeSession(
            tmp_path,
            scripts=[
                CLEAN_BUILD,
                CLEAN_BUILD,
                BuildScript(raises=KeyboardInterrupt),
            ],
        )
        with pytest.raises(KeyboardInterrupt):
            session.measure(runs=3)
        names = {p.name for p in session.run_directory.iterdir()}
        assert {"priming.log", "run-1.log"} <= names
        assert "record.json" not in names
        assert str(session.run_directory) in session.progress[-1]
        assert session.run_directory.is_absolute()

    def test_both_samplers_stop_when_the_build_raises(self, tmp_path):
        session = FakeSession(
            tmp_path,
            scripts=[CLEAN_BUILD, BuildScript(raises=KeyboardInterrupt)],
        )
        with pytest.raises(KeyboardInterrupt):
            session.measure(runs=1)
        assert all(
            running.stops >= 1
            for sampler in (session.cpu_sampler, session.process_sampler)
            for running in sampler.running
        )

    def test_the_idle_sampler_stops_when_idle_sampling_is_interrupted(
        self, session
    ):
        def interrupted(seconds):
            raise KeyboardInterrupt

        session.clock.sleep = interrupted
        with pytest.raises(KeyboardInterrupt):
            session.measure(runs=1)
        [idle] = session.cpu_sampler.running
        assert idle.stops == 1


class TestWorkingCopyMatch:
    def test_the_match_is_checked_before_priming_and_after_the_last_run(
        self, session
    ):
        session.measure(runs=2)
        assert session.history.checks == 2

    def test_a_failing_end_check_records_an_unknown_match(self, tmp_path):
        history = FakeHistory(
            later_working_copies=[HistoryError(["jj", "diff"], "locked")]
        )
        session = FakeSession(tmp_path, history=history)
        commit = session.measure(runs=1).measured.commit
        assert commit.matched_at_end is None
        assert not commit.matched

    @pytest.mark.parametrize(
        ("at_start", "at_end"),
        [((), ("tasks/a.py",)), (("tasks/a.py",), ())],
        ids=["mismatch-at-end", "mismatch-at-start"],
    )
    def test_matched_needs_both_checks_to_pass(
        self, tmp_path, at_start, at_end
    ):
        history = FakeHistory(
            working_copy=at_start, later_working_copies=[at_end]
        )
        session = FakeSession(tmp_path, history=history)
        commit = session.measure(runs=1).measured.commit
        assert commit.id == MEASURED_COMMIT
        assert not commit.matched


class TestProgress:
    def test_progress_reports_each_stage_in_order(self, tmp_path):
        session = FakeSession(
            tmp_path, scripts=[CLEAN_BUILD, CLEAN_BUILD, TEST_FAILS]
        )
        session.measure(runs=2)
        run_directory = session.run_directory
        firsts = [line.splitlines()[0] for line in session.progress]
        argv = "mise run --output prefix --continue-on-error default"
        assert firsts == [
            f"run directory: {run_directory}",
            "preflight:",
            "idle sampling: 30 s",
            "idle load: 26.0%",
            "priming build",
            f"  {argv}: tail -f {run_directory / 'priming.log'}",
            "priming: 6.5 s (discarded)",
            "timed run 1/2",
            f"  {argv}: tail -f {run_directory / 'run-1.log'}",
            "timed run 1/2: 6.5 s, no task failed",
            "timed run 2/2",
            f"  {argv}: tail -f {run_directory / 'run-2.log'}",
            "timed run 2/2: 6.5 s, 1 failed (test)",
        ]

    def test_the_preflight_comes_before_idle_sampling(self, session):
        session.measure(runs=1)
        events = session.events
        assert _index(events, "progress preflight") < events.index(
            "start cpu idle-iostat.log"
        )

    def test_idle_load_is_reported_once_sampled(self, session):
        session.measure(runs=1)
        events = session.events
        assert (
            events.index("stop cpu idle-iostat.log")
            < _index(events, "progress idle load: 26.0%")
            < events.index("build priming.log")
        )

    def test_a_fetch_failure_is_reported_at_the_end_of_its_run(self, tmp_path):
        session = FakeSession(
            tmp_path,
            scripts=[CLEAN_BUILD, ADVISORY_FAILS, CLEAN_BUILD, CLEAN_BUILD],
        )
        session.measure(runs=3, keep=True)
        [end] = [
            line
            for line in session.progress
            if line.startswith("timed run 1/3:")
        ]
        assert "fetch failure (launcher-release)" in end
        assert "will not be kept" in end
        assert end.endswith("press Ctrl-C to stop, then rerun")
        assert session.progress.index(end) < session.progress.index(
            "timed run 2/3"
        )

    def test_the_last_runs_fetch_failure_gives_no_ctrl_c_hint(self, tmp_path):
        session = FakeSession(tmp_path, scripts=[CLEAN_BUILD, ADVISORY_FAILS])
        session.measure(runs=1, keep=True)
        [end] = [
            line
            for line in session.progress
            if line.startswith("timed run 1/1:")
        ]
        assert end.endswith("will not be kept")

    def test_the_preflight_names_the_tracked_processes_running_now(
        self, session
    ):
        session.measure(runs=1, ide_patterns=[r"/Contents/MacOS/idea$"])
        [preflight] = [
            line for line in session.progress if line.startswith("preflight:")
        ]
        assert "  tracked processes running: syspolicyd, ide, fsnotifier" in (
            preflight.splitlines()
        )
        assert f"    {IDE_COMMAND}" in preflight.splitlines()
        assert PS_ARGV in session.diagnostics_calls

    def test_settings_track_the_default_processes_without_ide_patterns(
        self, session
    ):
        assert session.settings().tracked == DEFAULT_TRACKED_PROCESSES
