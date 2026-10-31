import pytest

from tasks.shared.build_measurement.errors import MeasurementRefusedError
from tasks.shared.build_measurement.history import GitHistory, JjHistory
from tasks.shared.build_measurement.wiring import history_of, real_ports
from tasks.shared.repository import NotARepositoryError


@pytest.mark.parametrize("with_git", [False, True], ids=["jj", "colocated"])
def test_a_directory_with_jj_gets_jj_history(tmp_path, with_git):
    (tmp_path / ".jj").mkdir()
    if with_git:
        (tmp_path / ".git").mkdir()
    assert isinstance(history_of(tmp_path), JjHistory)


def test_a_git_directory_gets_git_history(tmp_path):
    (tmp_path / ".git").mkdir()
    assert isinstance(history_of(tmp_path), GitHistory)


def test_a_plain_directory_is_refused_naming_it(tmp_path):
    with pytest.raises(MeasurementRefusedError, match=str(tmp_path)) as refusal:
        real_ports(tmp_path)
    assert not isinstance(refusal.value, NotARepositoryError)


def test_the_real_ports_are_rooted_at_the_checkout(tmp_path):
    (tmp_path / ".jj").mkdir()
    ports = real_ports(tmp_path)
    assert ports.history.root == tmp_path
    assert ports.build.root == tmp_path
