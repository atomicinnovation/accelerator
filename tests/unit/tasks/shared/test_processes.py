import os
import signal
import subprocess
import sys
import time

import psutil
import pytest

from tasks.shared.processes import (
    PsutilProcessOps,
    escalate,
    pid_identity_matches,
    reap_session,
    reap_tree,
)
from tests.unit.tasks.shared.doubles import FakeClock, FakeProcs


class TestPidIdentityMatches:
    def test_exact_match(self, mocker):
        proc = mocker.Mock()
        proc.create_time.return_value = 1000.0
        mocker.patch.object(psutil, "Process", return_value=proc)
        assert pid_identity_matches(123, 1000.0, tolerance=0.5) is True

    def test_drift_within_tolerance(self, mocker):
        proc = mocker.Mock()
        proc.create_time.return_value = 1000.3
        mocker.patch.object(psutil, "Process", return_value=proc)
        assert pid_identity_matches(123, 1000.0, tolerance=0.5) is True

    def test_mismatch_recycled_pid(self, mocker):
        proc = mocker.Mock()
        proc.create_time.return_value = 2000.0
        mocker.patch.object(psutil, "Process", return_value=proc)
        assert pid_identity_matches(123, 1000.0, tolerance=0.5) is False

    def test_dead_pid_returns_false(self, mocker):
        mocker.patch.object(
            psutil, "Process", side_effect=psutil.NoSuchProcess(123)
        )
        assert pid_identity_matches(123, 1000.0, tolerance=0.5) is False


class TestEscalate:
    def test_targets_are_terminated_first(self):
        procs, clock = FakeProcs(), FakeClock()
        procs.add(10)
        escalate([10], procs, clock, 5.0, relist=tuple)
        assert procs.terminated == [10]
        assert procs.killed == []

    def test_what_outlives_the_grace_is_killed(self):
        procs, clock = FakeProcs(), FakeClock()
        procs.add(10, ignore_sigterm=True)
        escalate([10], procs, clock, 5.0, relist=tuple)
        assert procs.killed == [10]
        assert clock.t >= 5.0

    def test_what_relist_finds_during_the_grace_is_terminated(self):
        procs, clock = FakeProcs(), FakeClock()
        procs.add(10, ignore_sigterm=True)
        appeared = iter([(), (11,)])

        def relist():
            procs.add(11)
            return next(appeared, ())

        escalate([10], procs, clock, 5.0, relist=relist)
        assert 11 in procs.terminated
        assert not procs.is_alive(11)


class AdvancingClock(FakeClock):
    def now(self):
        self.t += 0.07
        return self.t

    def sleep(self, dt):
        if dt < 0:
            raise ValueError("sleep length must be non-negative")
        super().sleep(dt)


class TestEscalateOnAMovingClock:
    def test_the_grace_never_sleeps_a_negative_length(self):
        procs = FakeProcs()
        procs.add(10, ignore_sigterm=True)
        escalate([10], procs, AdvancingClock(), 0.58, relist=tuple)
        assert procs.killed == [10]


class TestReapTree:
    def test_the_leader_and_its_children_are_terminated(self):
        procs, clock = FakeProcs(), FakeClock()
        procs.add(10)
        procs.add(11, parent=10)
        reap_tree(10, procs, clock, 5.0)
        assert procs.terminated == [10, 11]
        assert not procs.is_alive(11)

    def test_a_child_outliving_the_grace_is_killed(self):
        procs, clock = FakeProcs(), FakeClock()
        procs.add(10)
        procs.add(11, parent=10, ignore_sigterm=True)
        reap_tree(10, procs, clock, 5.0)
        assert procs.killed == [11]

    def test_an_exited_leader_is_a_no_op_beyond_its_own_signal(self):
        procs, clock = FakeProcs(), FakeClock()
        reap_tree(10, procs, clock, 5.0)
        assert procs.killed == []


class SpawningClock(FakeClock):
    def __init__(self, on_first_sleep):
        super().__init__()
        self.on_first_sleep = on_first_sleep

    def sleep(self, dt):
        if not self.sleeps:
            self.on_first_sleep()
        super().sleep(dt)


def session_with_leader(**leader):
    procs = FakeProcs()
    procs.caller_group = 1
    procs.add(100, pgid=100, **leader)
    return procs


class TestReapSession:
    def test_reap_session_signals_the_group_then_kills_what_outlives_the_grace(
        self,
    ):
        procs = session_with_leader(ignore_sigterm=True)
        procs.add(101, parent=100, pgid=100, ignore_sigterm=True)
        reap_session(100, procs, FakeClock(), 5.0)
        assert procs.group_signals[0] == (100, signal.SIGTERM)
        assert procs.group_signals[-1] == (100, signal.SIGKILL)
        assert not procs.group_alive(100)

    def test_reap_session_terminates_children_that_appear_during_the_grace(
        self,
    ):
        procs = session_with_leader(ignore_sigterm=True)
        clock = SpawningClock(lambda: procs.add(102, parent=100, pgid=102))
        reap_session(100, procs, clock, 5.0)
        assert 102 in procs.terminated
        assert not procs.is_alive(102)

    def test_an_orphaned_group_member_is_killed_by_the_closing_group_signal(
        self,
    ):
        procs = session_with_leader()
        procs.add(103, pgid=100, ignore_sigterm=True)
        reap_session(100, procs, FakeClock(), 5.0)
        assert 103 not in procs.killed
        assert not procs.is_alive(103)
        assert procs.group_signals[-1] == (100, signal.SIGKILL)

    def test_a_descendant_outside_the_group_is_reaped_from_the_snapshot(self):
        procs = session_with_leader()
        procs.add(104, parent=100, pgid=104)
        reap_session(100, procs, FakeClock(), 5.0)
        assert 104 in procs.terminated
        assert not procs.is_alive(104)

    def test_a_vanished_group_is_not_an_error(self):
        procs = session_with_leader()
        procs.procs[100].alive = False
        reap_session(100, procs, FakeClock(), 5.0)
        assert procs.group_signals == []

    def test_a_leader_in_the_callers_group_gets_no_group_signal(self):
        procs = FakeProcs()
        procs.caller_group = 1
        procs.add(100, pgid=1)
        reap_session(100, procs, FakeClock(), 5.0)
        assert procs.group_signals == []
        assert procs.terminated == [100]

    def test_a_leader_leading_the_callers_group_gets_no_group_signal(self):
        procs = FakeProcs()
        procs.caller_group = 100
        procs.add(100, pgid=100)
        reap_session(100, procs, FakeClock(), 5.0)
        assert procs.group_signals == []
        assert procs.terminated == [100]

    def test_a_leader_already_gone_is_reaped_without_a_group_signal(self):
        procs = FakeProcs()
        procs.caller_group = 1
        reap_session(100, procs, FakeClock(), 5.0)
        assert procs.group_signals == []


def _deadline_wait(predicate, seconds=5.0):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate():
            return True
        time.sleep(0.02)
    return predicate()


@pytest.fixture
def grouped_child():
    child = subprocess.Popen(
        [sys.executable, "-c", "import time; time.sleep(30)"],
        start_new_session=True,
    )
    yield child
    child.kill()
    child.wait(timeout=5)


class TestPsutilProcessGroups:
    def test_signalling_a_vanished_group_raises_nothing(self):
        child = subprocess.Popen(
            [sys.executable, "-c", "pass"], start_new_session=True
        )
        child.wait(timeout=5)
        PsutilProcessOps().signal_group(child.pid, signal.SIGTERM)

    def test_a_vanished_group_is_not_alive(self):
        child = subprocess.Popen(
            [sys.executable, "-c", "pass"], start_new_session=True
        )
        child.wait(timeout=5)
        assert not PsutilProcessOps().group_alive(child.pid)

    @pytest.mark.skipif(os.geteuid() == 0, reason="no permission stops root")
    def test_a_group_this_user_cannot_signal_is_not_alive(self):
        assert not PsutilProcessOps().group_alive(1)

    def test_a_live_childs_group_is_alive(self, grouped_child):
        assert _deadline_wait(
            lambda: PsutilProcessOps().group_alive(grouped_child.pid)
        )

    def test_a_live_childs_group_lists_its_members(self, grouped_child):
        ops = PsutilProcessOps()
        assert _deadline_wait(
            lambda: any(
                "time.sleep(30)" in command
                for _, command in ops.group_members(grouped_child.pid)
            )
        )
        assert [pid for pid, _ in ops.group_members(grouped_child.pid)] == [
            grouped_child.pid
        ]

    def test_a_live_childs_group_is_its_pid(self, grouped_child):
        ops = PsutilProcessOps()
        assert ops.group_of(grouped_child.pid) == grouped_child.pid
        assert ops.own_group() == os.getpgrp()
