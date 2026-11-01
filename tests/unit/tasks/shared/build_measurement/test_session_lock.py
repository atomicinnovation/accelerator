import pytest

from tasks.shared.build_measurement.errors import SessionInProgressError
from tasks.shared.build_measurement.locations import SESSION_LOCK
from tasks.shared.build_measurement.session_lock import WorkspaceSessionLock


def test_a_held_lock_is_refused_saying_another_session_is_running(tmp_path):
    with (
        WorkspaceSessionLock(tmp_path).held(),
        pytest.raises(SessionInProgressError) as refusal,
        WorkspaceSessionLock(tmp_path).held(),
    ):
        pass
    message = str(refusal.value)
    assert "another measure:build session is running" in message
    assert "wait for it to finish" in message
    assert f"do not delete {tmp_path / SESSION_LOCK}" in message


def test_a_released_lock_can_be_taken_again(tmp_path):
    with WorkspaceSessionLock(tmp_path).held():
        pass
    with WorkspaceSessionLock(tmp_path).held():
        pass
