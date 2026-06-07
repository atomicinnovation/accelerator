import psutil

from tasks.shared.processes import pid_identity_matches


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
