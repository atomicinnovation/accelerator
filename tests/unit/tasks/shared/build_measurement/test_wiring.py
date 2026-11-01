import shlex
import sys

import pytest

from tasks.shared.build_measurement.errors import (
    MeasurementRefusedError,
    TaskGraphError,
)
from tasks.shared.build_measurement.history import GitHistory, JjHistory
from tasks.shared.build_measurement.wiring import (
    history_of,
    mise_tasks_json,
    real_ports,
)
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


def test_the_host_is_sampled_once_a_second(tmp_path):
    (tmp_path / ".jj").mkdir()
    ports = real_ports(tmp_path)
    assert ports.cpu_sampler.argv == ("iostat", "-w", "1")
    assert ports.process_sampler.interval == 1
    assert ports.process_sampler.tick_timeout == 5.0


def test_the_task_graph_is_read_from_the_commands_output(tmp_path):
    argv = (sys.executable, "-c", "print('{}')")
    assert mise_tasks_json(tmp_path, argv=argv) == "{}\n"


def test_a_failing_task_listing_names_the_command_and_its_stderr(tmp_path):
    argv = (
        sys.executable,
        "-c",
        "import sys; sys.stderr.write('boom'); sys.exit(2)",
    )
    with pytest.raises(TaskGraphError) as refusal:
        mise_tasks_json(tmp_path, argv=argv)
    assert shlex.join(argv) in str(refusal.value)
    assert "boom" in str(refusal.value)
