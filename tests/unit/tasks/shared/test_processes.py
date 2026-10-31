import psutil

from tasks.shared.processes import escalate, pid_identity_matches, reap_tree
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
