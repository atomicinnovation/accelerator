import json
from dataclasses import replace as dataclasses_replace
from pathlib import Path

from tasks.shared.clock import Clock
from tasks.shared.dev.circus import SupervisorUnreachableError
from tasks.shared.dev.lifecycle import (
    DevDeps,
    StopResult,
    UpResult,
    bring_up,
    do_restart,
    do_status,
    do_stop,
)
from tasks.shared.dev.state import DevState, read_dev_state, write_dev_state
from tasks.shared.locking import workspace_lock
from tests.unit.tasks.shared.doubles import FakeClock, FakeProcs

# ─── orchestration fakes ─────────────────────────────────────


class FakeWorld:
    def __init__(
        self,
        procs,
        *,
        statuses=None,
        pids=None,
        reachable=True,
        quit_kills=(),
        activate_on_start=True,
        status_hook=None,
    ):
        self.procs = procs
        self.statuses = dict(statuses or {})
        self.pid_map = dict(pids or {})
        self.reachable = reachable
        self.quit_kills = list(quit_kills)
        self.quit_called = 0
        self.started: list[str] = []
        # A real `start` only confirms the arbiter accepted the command; the
        # watcher may reach "active" later, or never. activate_on_start=False
        # models that gap, and status_hook — run on every status() call, before
        # the reachability check so it can toggle that too — scripts what
        # happens next.
        self.activate_on_start = activate_on_start
        self.status_hook = status_hook

    def do_quit(self):
        self.quit_called += 1
        self.reachable = False
        for pid in self.quit_kills:
            for cp, _ in self.procs.children(pid):
                self.procs.kill(cp)
            self.procs.kill(pid)


class FakeSupervisor:
    def __init__(self, world):
        self.world = world

    def status(self):
        if self.world.status_hook:
            self.world.status_hook(self.world)
        if not self.world.reachable:
            raise SupervisorUnreachableError("unreachable")
        return dict(self.world.statuses)

    def pids(self, name):
        if not self.world.reachable:
            raise SupervisorUnreachableError("unreachable")
        return list(self.world.pid_map.get(name, []))

    def start(self, name):
        if not self.world.reachable:
            raise SupervisorUnreachableError("unreachable")
        self.world.started.append(name)
        if self.world.activate_on_start:
            self.world.statuses[name] = "active"

    def quit(self):
        if not self.world.reachable:
            raise SupervisorUnreachableError("unreachable")
        self.world.do_quit()

    def close(self):
        pass


class FakeHandle:
    def __init__(self, pid):
        self.pid = pid

    def poll(self):
        return None


class FakeLauncher:
    def __init__(
        self,
        procs,
        *,
        pidfile,
        server_info_path,
        state_path,
        arbiter_pid=9000,
        arbiter_ct=5000.0,
        server_pid=9001,
        frontend_pid=9002,
        child_ct=6000.0,
        write_pid=True,
        write_info=True,
        register_children=True,
    ):
        self.procs = procs
        self.pidfile = Path(pidfile)
        self.server_info_path = Path(server_info_path)
        self.state_path = Path(state_path)
        self.arbiter_pid = arbiter_pid
        self.arbiter_ct = arbiter_ct
        self.server_pid = server_pid
        self.frontend_pid = frontend_pid
        self.child_ct = child_ct
        self.write_pid = write_pid
        self.write_info = write_info
        self.register_children = register_children
        self.calls = 0
        self.info_existed_at_launch = None
        self.state_at_launch = None

    def __call__(self, argv, *, env, cwd):
        self.calls += 1
        self.info_existed_at_launch = self.server_info_path.exists()
        self.state_at_launch = read_dev_state(self.state_path)
        self.procs.add(self.arbiter_pid, self.arbiter_ct)
        if self.register_children:
            self.procs.add(
                self.server_pid, self.child_ct, parent=self.arbiter_pid
            )
            self.procs.add(
                self.frontend_pid, self.child_ct, parent=self.arbiter_pid
            )
        if self.write_pid:
            self.pidfile.parent.mkdir(parents=True, exist_ok=True)
            self.pidfile.write_text(str(self.arbiter_pid))
        if self.write_info:
            self.server_info_path.parent.mkdir(parents=True, exist_ok=True)
            self.server_info_path.write_text(
                json.dumps({"url": "http://127.0.0.1:7777", "port": 7777})
            )
        return FakeHandle(self.arbiter_pid)


def _orch_deps(
    tmp_path, *, procs, world, launcher=None, clock=None, **overrides
):
    dev_dir = tmp_path / "dev"
    dev_dir.mkdir(parents=True, exist_ok=True)
    server_dir = tmp_path / "dev-server"
    server_dir.mkdir(parents=True, exist_ok=True)
    fc = clock or FakeClock()
    base = {
        "client_factory": lambda endpoint, *, timeout: FakeSupervisor(world),
        "launcher": launcher or (lambda *a, **k: FakeHandle(9000)),
        "killer": procs,
        "clock": Clock(sleep=fc.sleep, now=fc.now),
        "project_root": tmp_path,
        "workspace_root": tmp_path,
        "state_path": dev_dir / "dev.json",
        "lock_path": dev_dir / "dev.lock",
        "dev_dir": dev_dir,
        "pidfile": dev_dir / "circusd.pid",
        "ini_path": dev_dir / "circus.ini",
        "server_info_path": server_dir / "server-info.json",
        "server_pidfile": server_dir / "server.pid",
        "server_bin": tmp_path / "bin/server",
        "frontend": tmp_path / "frontend",
        "diagnostic_log": dev_dir / "dev.log",
        "free_port": lambda: 54321,
        "probe_timeout": 0.1,
        "pidfile_timeout": 1.0,
        "readiness_timeout": 1.0,
        "frontend_active_timeout": 1.0,
        "grace_quit": 0.5,
        "grace_kill": 0.5,
    }
    base.update(overrides)
    return DevDeps(**base)


def _healthy_state(deps, *, arbiter_pid=9000, arbiter_ct=5000.0):
    return DevState(
        endpoint="ipc:///tmp/acc-dev-x/e.sock",
        pubsub_endpoint="ipc:///tmp/acc-dev-x/p.sock",
        frontend_port=54321,
        frontend_url="http://127.0.0.1:54321",
        pidfile=str(deps.pidfile),
        ini_path=str(deps.ini_path),
        arbiter_pid=arbiter_pid,
        arbiter_start_time=arbiter_ct,
        server_pid=9001,
        server_start_time=6000.0,
        frontend_pid=9002,
        frontend_start_time=6000.0,
    )


# ─── bring_up: launch + reuse + lock ─────────────────────────


class TestBringUp:
    def test_happy_path_starts_records_pids_and_returns_started(self, tmp_path):
        procs = FakeProcs()
        world = FakeWorld(
            procs,
            statuses={"server": "active", "frontend": "stopped"},
            pids={"server": [9001], "frontend": [9002]},
        )
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        launcher = FakeLauncher(
            procs,
            pidfile=deps.pidfile,
            server_info_path=deps.server_info_path,
            state_path=deps.state_path,
        )
        deps = dataclasses_replace(deps, launcher=launcher)

        result = bring_up(deps)

        assert result.kind == "started"
        assert result.frontend_url == "http://127.0.0.1:54321"
        assert result.api_port == 7777
        state = read_dev_state(deps.state_path)
        assert state.arbiter_pid == 9000
        assert state.server_pid == 9001
        assert state.frontend_pid == 9002
        assert "frontend" in world.started

    def test_provisional_state_and_clean_info_before_launch(self, tmp_path):
        procs = FakeProcs()
        world = FakeWorld(
            procs,
            statuses={"server": "active", "frontend": "stopped"},
            pids={"server": [9001], "frontend": [9002]},
        )
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        # a stale server-info.json must be deleted before launch
        deps.server_info_path.write_text('{"stale": true}')
        launcher = FakeLauncher(
            procs,
            pidfile=deps.pidfile,
            server_info_path=deps.server_info_path,
            state_path=deps.state_path,
        )
        deps = dataclasses_replace(deps, launcher=launcher)

        bring_up(deps)

        assert (
            launcher.info_existed_at_launch is False
        )  # stale info deleted first
        assert (
            launcher.state_at_launch is not None
        )  # provisional state written first
        assert (
            launcher.state_at_launch.arbiter_pid is None
        )  # PIDs null pre-launch

    def test_reuses_healthy_session_without_launching(self, tmp_path):
        procs = FakeProcs()
        procs.add(9000, 5000.0)
        world = FakeWorld(
            procs, statuses={"server": "active", "frontend": "active"}
        )
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        write_dev_state(deps.state_path, _healthy_state(deps))
        launcher = FakeLauncher(
            procs,
            pidfile=deps.pidfile,
            server_info_path=deps.server_info_path,
            state_path=deps.state_path,
        )
        deps = dataclasses_replace(deps, launcher=launcher)

        result = bring_up(deps)

        assert result.kind == "reused"
        assert launcher.calls == 0  # no duplicate arbiter

    def test_lock_held_reprobe_reuses_when_healthy(self, tmp_path):
        procs = FakeProcs()
        procs.add(9000, 5000.0)
        world = FakeWorld(
            procs, statuses={"server": "active", "frontend": "active"}
        )
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        write_dev_state(deps.state_path, _healthy_state(deps))
        with workspace_lock(deps.lock_path) as held:
            assert held is True
            result = bring_up(deps)  # cannot take the lock -> re-probe
        assert result.kind == "reused"

    def test_lock_held_fail_fast_when_not_healthy(self, tmp_path):
        procs = FakeProcs()
        world = FakeWorld(procs, reachable=False)
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        with workspace_lock(deps.lock_path) as held:
            assert held is True
            result = bring_up(deps)
        assert result.kind == "failed"
        assert "another `dev` is starting" in result.message

    def test_degraded_session_teardown_survivor_aborts(self, tmp_path):
        procs = FakeProcs()
        procs.add(
            9000, 5000.0, ignore_sigterm=True, unkillable=True
        )  # survives
        world = FakeWorld(
            procs, statuses={"server": "active", "frontend": "stopped"}
        )
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        write_dev_state(deps.state_path, _healthy_state(deps))
        launcher = FakeLauncher(
            procs,
            pidfile=deps.pidfile,
            server_info_path=deps.server_info_path,
            state_path=deps.state_path,
        )
        deps = dataclasses_replace(deps, launcher=launcher)

        result = bring_up(deps)

        assert result.kind == "failed"  # do not launch a competitor
        assert launcher.calls == 0

    def test_pidfile_never_appears_reaps_handle_and_fails_no_retry(
        self, tmp_path
    ):
        procs = FakeProcs()
        world = FakeWorld(procs)
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        launcher = FakeLauncher(
            procs,
            pidfile=deps.pidfile,
            server_info_path=deps.server_info_path,
            state_path=deps.state_path,
            write_pid=False,  # circusd never writes its pidfile
        )
        deps = dataclasses_replace(deps, launcher=launcher)

        result = bring_up(deps)

        assert result.kind == "failed"
        assert "circusd.log" in result.message
        assert launcher.calls == 1  # no retry
        assert 9000 in procs.killed  # launch handle (real arbiter) reaped

    def test_empty_pidfile_keeps_polling_then_succeeds(self, tmp_path):
        procs = FakeProcs()
        world = FakeWorld(
            procs,
            statuses={"server": "active", "frontend": "stopped"},
            pids={"server": [9001], "frontend": [9002]},
        )
        fc = FakeClock()
        deps = _orch_deps(tmp_path, procs=procs, world=world, clock=fc)

        def launcher(argv, *, env, cwd):
            procs.add(9000, 5000.0)
            procs.add(9001, 6000.0, parent=9000)
            procs.add(9002, 6000.0, parent=9000)
            deps.pidfile.write_text("")  # created-but-empty
            deps.server_info_path.write_text(
                json.dumps({"url": "http://127.0.0.1:7777", "port": 7777})
            )
            return FakeHandle(9000)

        # the real pid lands after the 2nd poll-sleep
        original_sleep = fc.sleep

        def sleeper(dt):
            original_sleep(dt)
            if len(fc.sleeps) == 2:
                deps.pidfile.write_text("9000")

        deps = dataclasses_replace(
            deps, launcher=launcher, clock=Clock(sleep=sleeper, now=fc.now)
        )

        result = bring_up(deps)
        assert result.kind == "started"

    def test_readiness_timeout_tears_down_and_fails(self, tmp_path):
        procs = FakeProcs()
        world = FakeWorld(
            procs,
            statuses={"server": "active", "frontend": "stopped"},
            quit_kills=[9000],
        )
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        launcher = FakeLauncher(
            procs,
            pidfile=deps.pidfile,
            server_info_path=deps.server_info_path,
            state_path=deps.state_path,
            write_info=False,  # server never writes server-info.json
        )
        deps = dataclasses_replace(deps, launcher=launcher)

        result = bring_up(deps)

        assert result.kind == "failed"
        assert "server.log" in result.message
        assert not deps.state_path.exists()  # torn down


# ─── bring_up: frontend activation ───────────────────────────


def _after_start(action):
    """Run ``action(world, poll_number)`` on each status poll after start.

    Counting only post-start polls keeps a case independent of how many times
    the earlier launch and readiness steps happen to ask for status.
    """
    polls = 0

    def hook(world):
        nonlocal polls
        if "frontend" not in world.started:
            return
        polls += 1
        action(world, polls)

    return hook


class TestFrontendActivation:
    """Separating "is it up?" from "did it stay up?".

    Neither question had a test: `FakeSupervisor.start` flipped the status to
    active, so every case arrived at the poll loop with the answer already
    yes. These are the cases the check has to tell apart on a loaded machine.
    """

    def _world(self, procs, **kwargs):
        return FakeWorld(
            procs,
            statuses={"server": "active", "frontend": "stopped"},
            pids={"server": [9001], "frontend": [9002]},
            **kwargs,
        )

    def _deps(self, tmp_path, procs, world, **overrides):
        deps = _orch_deps(tmp_path, procs=procs, world=world, **overrides)
        return dataclasses_replace(
            deps,
            launcher=FakeLauncher(
                procs,
                pidfile=deps.pidfile,
                server_info_path=deps.server_info_path,
                state_path=deps.state_path,
            ),
        )

    def test_a_watcher_slow_to_report_active_is_accepted(self, tmp_path):
        procs = FakeProcs()

        def activate_on_the_third_poll(world, poll):
            if poll >= 3:
                world.statuses["frontend"] = "active"

        world = self._world(
            procs,
            activate_on_start=False,
            status_hook=_after_start(activate_on_the_third_poll),
        )

        assert bring_up(self._deps(tmp_path, procs, world)).kind == "started"

    def test_a_transiently_unreachable_supervisor_does_not_fail_it(
        self, tmp_path
    ):
        # The CI failure this fixes. On a loaded runner the status endpoint
        # times out from time to time; the watcher is up, the poll just could
        # not read it. An unreadable poll must not count against the watcher —
        # under the old rule it reset the consecutive-active counter, so a
        # healthy frontend could be torn down for the machine being busy.
        procs = FakeProcs()

        def unreachable_for_two_polls(world, poll):
            world.reachable = poll > 2

        world = self._world(
            procs, status_hook=_after_start(unreachable_for_two_polls)
        )

        assert bring_up(self._deps(tmp_path, procs, world)).kind == "started"

    def test_a_watcher_that_never_activates_is_torn_down(self, tmp_path):
        procs = FakeProcs()
        world = self._world(procs, activate_on_start=False, quit_kills=[9000])
        deps = self._deps(tmp_path, procs, world)

        result = bring_up(deps)

        assert result.kind == "failed"
        assert "did not become active" in result.message
        assert "/frontend.log" in result.message
        assert not deps.state_path.exists()

    def test_a_watcher_that_dies_during_the_settle_window_fails(self, tmp_path):
        # Active on the first poll, gone by the confirmation poll: the
        # respawn=false start-then-die the settle window exists to catch.
        procs = FakeProcs()

        def stop_after_the_first_poll(world, poll):
            if poll >= 2:
                world.statuses["frontend"] = "stopped"

        world = self._world(
            procs,
            status_hook=_after_start(stop_after_the_first_poll),
            quit_kills=[9000],
        )
        fc = FakeClock()
        deps = self._deps(tmp_path, procs, world, clock=fc)

        result = bring_up(deps)

        assert result.kind == "failed"
        assert "became active then stopped" in result.message
        # The confirmation waited a fixed duration rather than a second poll.
        assert deps.frontend_settle in fc.sleeps
        assert not deps.state_path.exists()

    def test_an_unreadable_settle_poll_does_not_fail_the_watcher(
        self, tmp_path
    ):
        # "Could not read" is not evidence the watcher died, and failing here
        # would reintroduce the same flake one step later. A watcher that did
        # die is reported by do_status as degraded, which is the safety net.
        procs = FakeProcs()

        def unreachable_from_the_settle_poll(world, poll):
            world.reachable = poll < 2

        world = self._world(
            procs, status_hook=_after_start(unreachable_from_the_settle_poll)
        )

        assert bring_up(self._deps(tmp_path, procs, world)).kind == "started"


# ─── teardown / do_stop ──────────────────────────────────────


class TestTeardown:
    def test_clean_quit_removes_artifacts(self, tmp_path):
        procs = FakeProcs()
        procs.add(9000, 5000.0)
        procs.add(9001, 6000.0, parent=9000)
        procs.add(9002, 6000.0, parent=9000)
        world = FakeWorld(
            procs,
            statuses={"server": "active", "frontend": "active"},
            quit_kills=[9000],
        )
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        deps.ini_path.write_text("ini")
        deps.pidfile.write_text("9000")
        write_dev_state(deps.state_path, _healthy_state(deps))

        result = do_stop(deps)

        assert result.kind == "clean"
        assert world.quit_called == 1
        assert not procs.is_alive(9001) and not procs.is_alive(9002)
        assert not deps.state_path.exists()
        assert not deps.ini_path.exists()
        assert not deps.pidfile.exists()

    def test_refused_when_arbiter_identity_mismatches(self, tmp_path):
        procs = FakeProcs()
        procs.add(9000, 9999.0)  # live but a different process (recycled PID)
        world = FakeWorld(
            procs, statuses={"server": "active", "frontend": "active"}
        )
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        write_dev_state(
            deps.state_path, _healthy_state(deps)
        )  # records ct 5000.0

        result = do_stop(deps)

        assert result.kind == "refused"
        assert result.pid == 9000
        assert procs.is_alive(9000)  # never signalled
        assert deps.state_path.exists()  # state kept

    def test_survivor_keeps_state_and_sockets(self, tmp_path):
        procs = FakeProcs()
        procs.add(9000, 5000.0, ignore_sigterm=True, unkillable=True)
        world = FakeWorld(
            procs, statuses={"server": "active", "frontend": "active"}
        )
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        write_dev_state(deps.state_path, _healthy_state(deps))

        result = do_stop(deps)

        assert result.kind == "survivor"
        assert (
            deps.state_path.exists()
        )  # kept so a later dev:stop can quit again

    def test_orphan_reaped_via_recorded_pids_when_arbiter_dead(self, tmp_path):
        # Arbiter already dead; its children reparented (no longer children()).
        procs = FakeProcs()
        procs.add(9001, 6000.0, parent=1)  # reparented to init
        procs.add(9002, 6000.0, parent=1)
        procs.add(9003, 6000.0, parent=9002)  # a lazily-spawned node worker
        world = FakeWorld(procs, reachable=False)
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        write_dev_state(
            deps.state_path, _healthy_state(deps)
        )  # arbiter_pid 9000 (dead)

        result = do_stop(deps)

        assert result.kind == "clean"
        assert not procs.is_alive(9001)
        assert not procs.is_alive(9002)
        assert not procs.is_alive(9003)  # re-enumerated descendant reaped too

    def test_recycled_child_pid_is_skipped(self, tmp_path):
        procs = FakeProcs()
        procs.add(9001, 6000.0, parent=1)  # recorded server, identity ok
        procs.add(
            9002, 7777.0, parent=1
        )  # frontend PID now a recycled, unrelated proc
        world = FakeWorld(procs, reachable=False)
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        state = _healthy_state(deps)
        state.frontend_start_time = (
            6000.0  # recorded ct != current 7777 -> mismatch
        )
        write_dev_state(deps.state_path, state)

        do_stop(deps)

        assert not procs.is_alive(9001)  # genuine orphan reaped
        assert procs.is_alive(9002)  # recycled PID skipped (identity mismatch)

    def test_null_arbiter_pid_unreachable_is_clean_no_crash(self, tmp_path):
        procs = FakeProcs()
        world = FakeWorld(procs, reachable=False)
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        state = DevState(
            endpoint="ipc:///tmp/acc-dev-x/e.sock",
            pubsub_endpoint="ipc:///tmp/acc-dev-x/p.sock",
            frontend_port=54321,
            frontend_url="http://127.0.0.1:54321",
            pidfile=str(deps.pidfile),
            ini_path=str(deps.ini_path),
        )  # provisional: all PIDs null
        write_dev_state(deps.state_path, state)

        result = do_stop(deps)

        assert result.kind == "clean"

    def test_no_state_file_is_clean(self, tmp_path):
        procs = FakeProcs()
        world = FakeWorld(procs, reachable=False)
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        result = do_stop(deps)
        assert result.kind == "clean"


# ─── do_restart seam ─────────────────────────────────────────


class TestDoRestart:
    def test_clean_stop_proceeds_to_bring_up(self, tmp_path, mocker):
        procs = FakeProcs()
        world = FakeWorld(procs)
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        mocker.patch(
            "tasks.shared.dev.lifecycle.do_stop",
            return_value=StopResult("clean"),
        )
        mocker.patch(
            "tasks.shared.dev.lifecycle.bring_up",
            return_value=UpResult("started"),
        )
        result = do_restart(deps)
        assert result.kind == "started"

    def test_survivor_aborts_without_relaunch(self, tmp_path, mocker):
        procs = FakeProcs()
        world = FakeWorld(procs)
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        mocker.patch(
            "tasks.shared.dev.lifecycle.do_stop",
            return_value=StopResult(
                "survivor", pid=9000, message="still alive"
            ),
        )
        bu = mocker.patch("tasks.shared.dev.lifecycle.bring_up")
        result = do_restart(deps)
        assert result.kind == "failed"
        bu.assert_not_called()

    def test_refused_aborts_without_relaunch(self, tmp_path, mocker):
        procs = FakeProcs()
        world = FakeWorld(procs)
        deps = _orch_deps(tmp_path, procs=procs, world=world)
        mocker.patch(
            "tasks.shared.dev.lifecycle.do_stop",
            return_value=StopResult(
                "refused", pid=9000, message="identity mismatch"
            ),
        )
        bu = mocker.patch("tasks.shared.dev.lifecycle.bring_up")
        result = do_restart(deps)
        assert result.kind == "failed"
        bu.assert_not_called()


# ─── do_status ───────────────────────────────────────────────


def _status_deps(tmp_path, *, statuses, reachable=True, state=None, info=None):
    procs = FakeProcs()
    world = FakeWorld(procs, statuses=statuses, reachable=reachable)
    deps = _orch_deps(tmp_path, procs=procs, world=world)
    if state is not None:
        write_dev_state(deps.state_path, state)
    if info is not None:
        deps.server_info_path.write_text(json.dumps(info))
    return deps


class TestDoStatus:
    def test_both_active_is_healthy_exit_0_with_fields(self, tmp_path):
        deps = _status_deps(
            tmp_path,
            statuses={"server": "active", "frontend": "active"},
            state=_healthy_state(
                _orch_deps(
                    tmp_path, procs=FakeProcs(), world=FakeWorld(FakeProcs())
                )
            ),
            info={"url": "http://127.0.0.1:7777", "port": 7777},
        )
        # rebuild state on the real deps paths
        write_dev_state(deps.state_path, _healthy_state(deps))
        result = do_status(deps)
        assert result.exit_code == 0
        text = "\n".join(result.lines)
        assert "Server:   active" in text
        assert "Frontend: active" in text
        assert "http://127.0.0.1:54321" in text  # frontend URL
        assert "http://127.0.0.1:7777" in text  # API URL
        # log paths printed even when HEALTHY
        assert "/server.log" in text
        assert "/frontend.log" in text

    def test_starting_label_when_frontend_pid_never_recorded(self, tmp_path):
        deps = _status_deps(
            tmp_path, statuses={"server": "active", "frontend": "stopped"}
        )
        state = _healthy_state(deps)
        state.frontend_pid = (
            None  # never recorded => genuinely mid-first-launch
        )
        state.frontend_start_time = None
        write_dev_state(deps.state_path, state)
        result = do_status(deps)
        assert result.exit_code == 3
        assert any("(starting)" in line for line in result.lines)

    def test_settled_dead_frontend_is_degraded_not_starting(self, tmp_path):
        deps = _status_deps(
            tmp_path, statuses={"server": "active", "frontend": "stopped"}
        )
        state = _healthy_state(deps)  # frontend_pid WAS recorded (9002)
        write_dev_state(deps.state_path, state)
        result = do_status(deps)
        assert result.exit_code == 3
        assert not any("(starting)" in line for line in result.lines)

    def test_no_state_is_down_exit_4(self, tmp_path):
        deps = _status_deps(tmp_path, statuses={})
        result = do_status(deps)
        assert result.exit_code == 4

    def test_unreachable_is_down_exit_4(self, tmp_path):
        deps = _status_deps(
            tmp_path,
            statuses={"server": "active", "frontend": "active"},
            reachable=False,
        )
        write_dev_state(deps.state_path, _healthy_state(deps))
        result = do_status(deps)
        assert result.exit_code == 4

    def test_log_paths_printed_on_down_too(self, tmp_path):
        deps = _status_deps(tmp_path, statuses={})
        result = do_status(deps)
        text = "\n".join(result.lines)
        assert "/server.log" in text
        assert "/frontend.log" in text
