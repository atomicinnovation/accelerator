import pytest

from tasks.shared.build_measurement.build_group import BuildGroupFile
from tasks.shared.build_measurement.locations import BUILD_PGID
from tasks.shared.build_measurement.session import OrphanedBuild
from tests.unit.tasks.shared.doubles import FakeProcs

_LEADER = 4242
_STARTED = 1000.0


@pytest.fixture
def procs():
    procs = FakeProcs()
    procs.caller_group = 77
    return procs


def _with_live_build(procs):
    procs.add(_LEADER, ct=_STARTED, pgid=_LEADER, command="mise run default")
    procs.add(4243, pgid=_LEADER, command="cargo nextest run")


def test_a_remembered_group_reads_back(tmp_path, procs):
    group = BuildGroupFile(tmp_path, procs)
    group.remember(_LEADER, _STARTED)
    assert group.read() == (_LEADER, _STARTED)
    assert not (tmp_path / BUILD_PGID).with_suffix(".pgid.tmp").exists()


def test_a_forgotten_group_leaves_no_file(tmp_path, procs):
    group = BuildGroupFile(tmp_path, procs)
    group.remember(_LEADER, _STARTED)
    group.forget()
    assert not (tmp_path / BUILD_PGID).exists()
    group.forget()


def test_no_file_is_no_orphan(tmp_path, procs):
    assert BuildGroupFile(tmp_path, procs).orphaned() is None


def test_the_living_leader_marks_an_orphan(tmp_path, procs):
    _with_live_build(procs)
    group = BuildGroupFile(tmp_path, procs)
    group.remember(_LEADER, _STARTED)
    assert group.orphaned() == OrphanedBuild(
        pgid=_LEADER,
        members=((_LEADER, "mise run default"), (4243, "cargo nextest run")),
        path=tmp_path / BUILD_PGID,
    )


def test_a_live_group_with_no_leader_is_an_orphan(tmp_path, procs):
    procs.add(4243, pgid=_LEADER, command="cargo nextest run")
    group = BuildGroupFile(tmp_path, procs)
    group.remember(_LEADER, _STARTED)
    orphan = group.orphaned()
    assert orphan is not None
    assert orphan.members == ((4243, "cargo nextest run"),)


def test_a_reused_pid_makes_the_file_stale(tmp_path, procs):
    procs.add(_LEADER, ct=_STARTED + 60, pgid=_LEADER, command="vim")
    group = BuildGroupFile(tmp_path, procs)
    group.remember(_LEADER, _STARTED)
    assert group.orphaned() is None
    assert not (tmp_path / BUILD_PGID).exists()


def test_a_vanished_group_is_stale(tmp_path, procs):
    group = BuildGroupFile(tmp_path, procs)
    group.remember(_LEADER, _STARTED)
    assert group.orphaned() is None
    assert not (tmp_path / BUILD_PGID).exists()


def test_a_group_this_user_cannot_signal_is_not_the_orphan(tmp_path, procs):
    _with_live_build(procs)
    procs.foreign_groups.add(_LEADER)
    group = BuildGroupFile(tmp_path, procs)
    group.remember(_LEADER, _STARTED)
    assert group.orphaned() is None
    assert not (tmp_path / BUILD_PGID).exists()


@pytest.mark.parametrize("pgid", [0, 1, 77], ids=["zero", "one", "callers"])
def test_an_implausible_group_is_stale(tmp_path, procs, pgid):
    procs.add(pgid, ct=_STARTED, pgid=pgid)
    group = BuildGroupFile(tmp_path, procs)
    group.remember(pgid, _STARTED)
    assert group.orphaned() is None
    assert not (tmp_path / BUILD_PGID).exists()


@pytest.mark.parametrize(
    "text", ["", "4242\n", "4242 started\n", "42"], ids=str
)
def test_an_unreadable_file_is_stale(tmp_path, procs, text):
    _with_live_build(procs)
    path = tmp_path / BUILD_PGID
    path.parent.mkdir(parents=True)
    path.write_text(text)
    assert BuildGroupFile(tmp_path, procs).orphaned() is None
    assert not path.exists()
