import datetime as dt
from pathlib import Path

import pytest

from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.cold_trash import FoundTrash, UnremovedTrash
from tasks.shared.build_measurement.errors import (
    HistoryError,
    OrphanedBuildError,
    SessionInProgressError,
    TrashNotRemovedError,
    UnknownTaskError,
    UnsafeDeletionError,
    UnsupportedPlatformError,
)
from tasks.shared.build_measurement.locations import RUN_DIRECTORY_ROOT
from tasks.shared.build_measurement.measured_build import Mode
from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.reports import (
    DevLauncherNeedsRebuild,
    notices_report,
    summary_report,
)
from tasks.shared.build_measurement.session import (
    IDLE_SAMPLE_SECONDS,
    OrphanedBuild,
    measure_build,
)
from tasks.shared.build_measurement.timeline import TaskOutcome
from tasks.shared.build_measurement.tracked_processes import (
    DEFAULT_TRACKED_PROCESSES,
    PS_ARGV,
)
from tests.unit.tasks.shared.build_measurement.doubles import (
    ADVISORY_FAILS,
    BUILD_ALONE,
    CLEAN_BUILD,
    GRAPH_JSON,
    IDE_COMMAND,
    MEASURED_COMMIT,
    TEST_ALONE,
    TEST_FAILS,
    TEST_FAILS_ALONE,
    BuildScript,
    FakeHistory,
    FakeSession,
    failed_alone,
    succeeded,
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

    @pytest.mark.parametrize("state", ["warm-no-op", "incremental", "cold"])
    def test_the_session_opens_in_one_order(self, session, state):
        session.measure(runs=1, cache_state=state)
        events = session.events
        assert events[:5] == [
            "platform",
            "task graph",
            f"verify {state}",
            "lock",
            "orphan check",
        ]
        assert events[5].startswith("progress run directory:")

    def test_a_held_lock_refuses_before_anything_else_runs(self, tmp_path):
        session = FakeSession(
            tmp_path, lock_refusal=SessionInProgressError("busy")
        )
        with pytest.raises(SessionInProgressError):
            session.measure()
        assert session.events == [
            "platform",
            "task graph",
            "verify warm-no-op",
            "lock",
        ]
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
        assert session.events == [
            "platform",
            "task graph",
            "verify warm-no-op",
            "lock",
            "orphan check",
            "unlock",
        ]
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


def _isolated(tmp_path, *scripts, isolate=("build", "test"), runs=1):
    session = FakeSession(tmp_path, scripts=list(scripts))
    return session, session.measure(runs=runs, isolate=isolate)


class TestIsolation:
    def test_isolation_runs_each_named_task_alone_with_skip_deps(
        self, tmp_path
    ):
        session, _ = _isolated(
            tmp_path, BUILD_ALONE, TEST_ALONE, BUILD_ALONE, TEST_ALONE
        )
        alone = ("mise", "run", "--output", "prefix", "--skip-deps")
        assert [argv for argv, _ in session.build.calls] == [
            (*alone, "build"),
            (*alone, "test"),
            (*alone, "build"),
            (*alone, "test"),
        ]

    def test_isolated_intervals_do_not_overlap(self, tmp_path):
        _, outcome = _isolated(
            tmp_path, BUILD_ALONE, TEST_ALONE, BUILD_ALONE, TEST_ALONE
        )
        tasks = outcome.measured.runs[0].tasks
        assert tasks["build"].end_seconds == 2.0
        assert tasks["test"].start_seconds == 3.5
        assert tasks["test"].end_seconds == 6.5

    def test_isolation_primes_once_over_all_named_tasks(self, tmp_path):
        session, _ = _isolated(
            tmp_path, BUILD_ALONE, TEST_ALONE, BUILD_ALONE, TEST_ALONE
        )
        assert [log.name for _, log in session.build.calls] == [
            "priming-build.log",
            "priming-test.log",
            "run-1-build.log",
            "run-1-test.log",
        ]

    def test_an_isolated_task_is_ready_at_its_own_invocations_start(
        self, tmp_path
    ):
        _, outcome = _isolated(
            tmp_path, BUILD_ALONE, TEST_ALONE, BUILD_ALONE, TEST_ALONE
        )
        test = outcome.measured.runs[0].tasks["test"]
        assert test.queueing_delay_seconds == 1.0

    def test_isolation_reports_duration_nextest_phase_and_status_with_spreads(
        self, tmp_path
    ):
        _, outcome = _isolated(
            tmp_path,
            *[BUILD_ALONE, TEST_ALONE] * 3,
            runs=2,
        )
        measured = outcome.measured
        assert [run.tasks["test"].outcome for run in measured.runs] == [
            TaskOutcome.SUCCEEDED,
            TaskOutcome.SUCCEEDED,
        ]
        across = measured.summary.tasks["test"]
        assert across.duration_seconds is not None
        assert across.duration_seconds.median == 3.0
        assert across.nextest_phase_seconds is not None
        assert across.nextest_phase_seconds.median == 2.0

    def test_an_isolated_runs_wall_clock_spans_every_invocation(self, tmp_path):
        _, outcome = _isolated(
            tmp_path, BUILD_ALONE, TEST_ALONE, BUILD_ALONE, TEST_ALONE
        )
        assert outcome.measured.runs[0].wall_clock_seconds == 7.0

    def test_an_isolated_runs_exit_status_is_its_first_failure(self, tmp_path):
        build_fails_alone = BuildScript(
            lines=failed_alone("build", 0.0), exit_status=5, wall_clock=1.0
        )
        _, outcome = _isolated(
            tmp_path,
            TEST_ALONE,
            BUILD_ALONE,
            TEST_FAILS_ALONE,
            build_fails_alone,
            isolate=("test", "build"),
        )
        [run] = outcome.measured.runs
        assert run.exit_status == 3
        assert run.contains_failure

    def test_a_failed_isolated_task_takes_its_invocations_exit_status(
        self, tmp_path
    ):
        _, outcome = _isolated(
            tmp_path, TEST_ALONE, TEST_FAILS_ALONE, isolate=("test",)
        )
        test = outcome.measured.runs[0].tasks["test"]
        assert test.outcome is TaskOutcome.FAILED
        assert test.exit_status == 3

    def test_the_buckets_list_each_isolated_task_while_it_runs(self, tmp_path):
        long_build = BuildScript(
            lines=succeeded("build", 0.0, 40.0), wall_clock=41.0
        )
        long_test = BuildScript(
            lines=succeeded("test", 1.0, 40.0), wall_clock=42.0
        )
        _, outcome = _isolated(
            tmp_path, BUILD_ALONE, TEST_ALONE, long_build, long_test
        )
        buckets = outcome.measured.runs[0].cpu_buckets
        assert [bucket.running for bucket in buckets] == [
            ("build",),
            ("test",),
            ("test",),
        ]

    def test_the_record_marks_its_mode_and_isolated_tasks(self, tmp_path):
        _, outcome = _isolated(
            tmp_path, BUILD_ALONE, TEST_ALONE, BUILD_ALONE, TEST_ALONE
        )
        measured = outcome.measured
        assert measured.conditions.mode is Mode.ISOLATED
        assert measured.conditions.isolated_tasks == ("build", "test")
        assert measured.run_id.endswith("_devtools_isolated")
        assert set(measured.runs[0].tasks) == {"build", "test"}

    def test_an_unknown_isolated_task_is_refused_before_the_lock(
        self, tmp_path
    ):
        settings = FakeSession(tmp_path).settings(isolate=["build", "tset"])
        with pytest.raises(UnknownTaskError) as refusal:
            measure_build(
                settings,
                untouchable_ports(
                    system=lambda: "Darwin", mise_tasks=lambda: GRAPH_JSON
                ),
            )
        assert str(refusal.value) == (
            "no such task to isolate: 'tset' (closest: 'test')"
        )
        assert not (tmp_path / RUN_DIRECTORY_ROOT).exists()

    def test_an_unknown_task_with_no_close_name_says_so(self, tmp_path):
        settings = FakeSession(tmp_path).settings(isolate=["frobnicate"])
        with pytest.raises(
            UnknownTaskError,
            match=r"^no such task to isolate: 'frobnicate' \(no close match\)$",
        ):
            measure_build(
                settings,
                untouchable_ports(
                    system=lambda: "Darwin", mise_tasks=lambda: GRAPH_JSON
                ),
            )

    def test_isolated_priming_writes_one_log_per_task(self, tmp_path):
        session, _ = _isolated(
            tmp_path, BUILD_ALONE, TEST_ALONE, BUILD_ALONE, TEST_ALONE
        )
        names = {p.name for p in session.run_directory.iterdir()}
        assert {"priming-build.log", "priming-test.log"} <= names
        assert "priming.log" not in names

    def test_progress_names_each_isolated_invocations_log(self, tmp_path):
        session, outcome = _isolated(
            tmp_path,
            BUILD_ALONE,
            TEST_ALONE,
            BUILD_ALONE,
            TEST_FAILS_ALONE,
        )
        run_directory = session.run_directory
        alone = "mise run --output prefix --skip-deps"
        for task in ("build", "test"):
            assert (
                f"  {alone} {task}: tail -f {run_directory}/run-1-{task}.log"
                in session.progress
            )
        record = run_directory / "record.json"
        summary = summary_report(outcome, record_path=record, kept_path=None)
        assert f"failed: test (run 1, see {run_directory}/run-1-test.log)" in (
            summary
        )


class TestCacheStates:
    def test_preparation_precedes_each_timed_run_but_not_priming(self, session):
        session.measure(runs=2, cache_state="incremental")
        events = [
            event
            for event in session.events
            if event.startswith(("prepare", "build"))
        ]
        assert events == [
            "build priming.log",
            "prepare cold-trash-1",
            "build run-1.log",
            "prepare cold-trash-2",
            "build run-2.log",
        ]

    def test_preparation_finishes_before_the_runs_epoch_and_samplers(
        self, tmp_path
    ):
        session = FakeSession(tmp_path)
        session.workspace.preparation_seconds = 50.0
        measured = session.measure(runs=1, cache_state="cold").measured
        events = session.events
        run_start = events.index("start cpu run-1-iostat.log")
        assert events.index("prepare cold-trash-1") < run_start
        assert events.index("remove cold-trash-1") < run_start
        [run] = measured.runs
        assert run.wall_clock_seconds == 6.5
        assert run.tasks["build"].start_seconds == 0.0

    def test_each_timed_run_has_its_own_trash_directory(self, session):
        session.measure(runs=2, cache_state="cold")
        assert [trash for _, trash in session.workspace.prepared] == [
            session.run_directory / "cold-trash-1",
            session.run_directory / "cold-trash-2",
        ]

    def test_each_timed_run_is_prepared_for_the_sessions_state(self, session):
        session.measure(runs=2, cache_state="cold")
        assert {state for state, _ in session.workspace.prepared} == {
            BuildCacheState.COLD
        }

    def test_a_cold_session_never_primes(self, session):
        measured = session.measure(runs=2, cache_state="cold").measured
        assert [log.name for _, log in session.build.calls] == [
            "run-1.log",
            "run-2.log",
        ]
        assert measured.priming is None

    def test_progress_says_priming_was_skipped_and_what_is_being_deleted(
        self, session
    ):
        session.measure(runs=1, cache_state="cold")
        root = session.tmp_path
        progress = session.progress
        start = progress.index("timed run 1/1")
        assert "priming skipped (cold)" in progress[:start]
        assert progress[start + 1 : start + 4] == [
            "  deleting before this run:",
            f"    {root / 'cli/target'}",
            f"    {root / 'cli/visualiser/frontend/node_modules'}",
        ]

    def test_a_state_that_deletes_nothing_names_no_deletion(self, session):
        session.measure(runs=1, cache_state="incremental")
        assert not any("deleting" in line for line in session.progress)
        assert not any(event.startswith("remove") for event in session.events)


def _unremoved(session, number):
    return UnremovedTrash(
        path=session.tmp_path / f"cold-trash-{number}",
        size_bytes=48_000_000,
        cause="[Errno 13] Permission denied",
    )


class TestUnremovedTrash:
    def test_unremoved_trash_stops_the_session_before_the_next_timed_run(
        self, session
    ):
        unremoved = _unremoved(session, 1)
        session.workspace.unremoved = {"cold-trash-1": unremoved}
        with pytest.raises(TrashNotRemovedError) as refusal:
            session.measure(runs=3, cache_state="cold")
        assert str(refusal.value) == (
            "stopped before timed run 2: the cold trash from timed run 1 "
            "could not be removed ([Errno 13] Permission denied), and each "
            "further cold run would leave another copy of cli/target and "
            "cli/visualiser/frontend/node_modules; delete the trash listed "
            "above once the cause is fixed"
        )
        assert [log.name for _, log in session.build.calls] == ["run-1.log"]
        assert [trash.name for _, trash in session.workspace.prepared] == [
            "cold-trash-1"
        ]
        assert notices_report([unremoved]) in session.progress
        assert not (session.run_directory / "record.json").exists()

    def test_the_trash_that_stopped_the_session_is_never_safe_to_delete(
        self, session
    ):
        unremoved = _unremoved(session, 1)
        session.workspace.unremoved = {"cold-trash-1": unremoved}
        session.workspace.found_later = (
            FoundTrash(path=unremoved.path, size_bytes=unremoved.size_bytes),
        )
        with pytest.raises(TrashNotRemovedError):
            session.measure(runs=3, cache_state="cold")
        after = _after_preflight(session)
        assert after.count(str(unremoved.path)) == 1
        assert "safe to delete" not in after

    def test_unremoved_trash_from_the_last_run_becomes_a_notice(self, session):
        unremoved = _unremoved(session, 2)
        session.workspace.unremoved = {"cold-trash-2": unremoved}
        outcome = session.measure(runs=2, cache_state="cold")
        assert outcome.notices == (unremoved,)
        assert len(outcome.measured.runs) == 2

    def test_a_quiet_session_has_no_notices(self, session):
        assert session.measure(runs=2).notices == ()


def _preflight_of(session):
    [preflight] = [
        line for line in session.progress if line.startswith("preflight:")
    ]
    return preflight.splitlines()


_EARLIER_TRASH = FoundTrash(
    path=Path(
        "/checkout/.accelerator-measure/build/build_earlier/cold-trash-1"
    ),
    size_bytes=48_000_000,
)
_REBUILD = notices_report([DevLauncherNeedsRebuild()])


class TestPreflightOfCacheStates:
    def test_a_cold_preflight_lists_what_will_be_deleted(self, session):
        session.measure(runs=1, cache_state="cold")
        root = session.tmp_path
        lines = _preflight_of(session)
        assert f"    {root / 'cli/target'}" in lines
        assert f"    {root / 'cli/visualiser/frontend/node_modules'}" in lines
        events = session.events
        assert _index(events, "progress preflight") < events.index(
            "start cpu idle-iostat.log"
        )

    @pytest.mark.parametrize("state", ["warm-no-op", "incremental", "cold"])
    def test_every_preflight_lists_trash_left_by_earlier_sessions(
        self, session, state
    ):
        session.workspace.found = (_EARLIER_TRASH,)
        session.measure(runs=1, cache_state=state)
        assert f"  {notices_report([_EARLIER_TRASH])}" in _preflight_of(session)

    @pytest.mark.parametrize(
        ("state", "marker", "warned"),
        [
            ("cold", True, True),
            ("cold", False, False),
            ("warm-no-op", True, False),
            ("incremental", True, False),
        ],
    )
    def test_a_cold_preflight_warns_when_the_dev_launcher_marker_exists(
        self, session, state, marker, warned
    ):
        session.workspace.marker = marker
        session.measure(runs=1, cache_state=state)
        assert (_REBUILD in _preflight_of(session)) is warned


def _after_preflight(session):
    start = next(
        index
        for index, line in enumerate(session.progress)
        if line.startswith("preflight:")
    )
    return "\n".join(session.progress[start + 1 :])


def _ends_by_interrupt(session):
    session.build.scripts[1] = BuildScript(raises=KeyboardInterrupt)
    return KeyboardInterrupt


def _ends_by_refusal(session):
    session.workspace.unremoved = {"cold-trash-1": _unremoved(session, 1)}
    return TrashNotRemovedError


def _ends_by_refusing_a_later_preparation(session):
    session.workspace.refusals = {
        "cold-trash-2": UnsafeDeletionError("cli/target moved")
    }
    session.workspace.found_later = (_EARLIER_TRASH,)
    return UnsafeDeletionError


def _ends_while_removing_the_first_trash(session):
    session.workspace.unremoved = {"cold-trash-1": KeyboardInterrupt()}
    return KeyboardInterrupt


def _ends_during_idle_sampling(session):
    def interrupted(seconds):
        raise KeyboardInterrupt

    session.clock.sleep = interrupted
    return KeyboardInterrupt


def _ends_by_refusing_the_first_preparation(session):
    session.workspace.refusals = {
        "cold-trash-1": UnsafeDeletionError("cli/target moved")
    }
    return UnsafeDeletionError


class TestDevLauncherWarning:
    def test_a_successful_cold_session_repeats_it_as_a_notice(self, session):
        session.workspace.marker = True
        session.workspace.found_later = (_EARLIER_TRASH,)
        outcome = session.measure(runs=2, cache_state="cold")
        assert outcome.notices == (_EARLIER_TRASH, DevLauncherNeedsRebuild())

    def test_the_last_runs_unremoved_trash_is_not_also_found(self, session):
        unremoved = _unremoved(session, 2)
        session.workspace.unremoved = {"cold-trash-2": unremoved}
        session.workspace.found_later = (
            FoundTrash(path=unremoved.path, size_bytes=unremoved.size_bytes),
        )
        outcome = session.measure(runs=2, cache_state="cold")
        assert outcome.notices == (unremoved,)

    @pytest.mark.parametrize(
        "ending",
        [
            _ends_by_interrupt,
            _ends_by_refusal,
            _ends_by_refusing_a_later_preparation,
            _ends_while_removing_the_first_trash,
        ],
        ids=["interrupt", "refusal", "later-preparation", "trash-removal"],
    )
    def test_the_dev_launcher_warning_is_repeated_once_anything_is_deleted(
        self, session, ending
    ):
        session.workspace.marker = True
        expected = ending(session)
        with pytest.raises(expected):
            session.measure(runs=2, cache_state="cold")
        after = _after_preflight(session)
        assert _REBUILD in after
        if session.workspace.found_later:
            assert notices_report([_EARLIER_TRASH]) in after
        assert str(session.run_directory) in session.progress[-1]

    @pytest.mark.parametrize(
        "ending",
        [_ends_during_idle_sampling, _ends_by_refusing_the_first_preparation],
        ids=["idle-sampling", "first-preparation"],
    )
    def test_a_session_ending_before_anything_is_deleted_warns_once(
        self, session, ending
    ):
        session.workspace.marker = True
        expected = ending(session)
        with pytest.raises(expected):
            session.measure(runs=1, cache_state="cold")
        assert sum(_REBUILD in line for line in session.progress) == 1

    @pytest.mark.parametrize(
        ("unreadable", "reported"),
        [
            ("found_later", "the cold trash could not be listed: unreadable"),
            (
                "marker_later",
                "the dev launcher marker could not be checked: unreadable",
            ),
        ],
        ids=["trash", "marker"],
    )
    def test_a_failing_exit_report_never_hides_why_the_session_ended(
        self, session, unreadable, reported
    ):
        session.workspace.marker = True
        setattr(session.workspace, unreadable, PermissionError("unreadable"))
        _ends_by_interrupt(session)
        with pytest.raises(KeyboardInterrupt):
            session.measure(runs=2, cache_state="cold")
        after = _after_preflight(session)
        assert reported in after
        assert _REBUILD in after


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

    def test_a_failed_ps_probe_leaves_the_tracked_processes_unknown(
        self, session
    ):
        session.probes[PS_ARGV] = FileNotFoundError("ps")
        session.measure(runs=1)
        [preflight] = [
            line for line in session.progress if line.startswith("preflight:")
        ]
        assert (
            f"  tracked processes: {NoReading(argv=PS_ARGV)}"
            in preflight.splitlines()
        )

    def test_settings_track_the_default_processes_without_ide_patterns(
        self, session
    ):
        assert session.settings().tracked == DEFAULT_TRACKED_PROCESSES
