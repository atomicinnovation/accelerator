import errno
import os
import select
import shutil
import signal
import subprocess
import sys
import threading
import time
from collections.abc import Callable, Iterator
from pathlib import Path

import pytest

from tasks.shared.locking import try_exclusive_flock
from tasks.shared.paths import REPO_ROOT
from tasks.shared.shared_fixtures.errors import SharedFixturesError
from tasks.shared.shared_fixtures.families import (
    FIXTURE_LOG_VARIABLE,
    HOLD_VARIABLE,
    WORKBENCHES_VARIABLE,
    Family,
)
from tasks.shared.shared_fixtures.runs import (
    FixtureRuns,
    HoldPoint,
    remove_leftover,
    seal,
    shared_fixtures_root,
)

RUN_ID = "20261119T101500123456Z-0b6f9a0e-6c1d-4f0a-9d7e-2c1b3a4d5e6f"
OTHER_RUN_ID = "20261119T101501123456Z-1c7a8b1f-7d2e-4a1b-8e8f-3d2c4b5e6f70"
HANDSHAKE_TIMEOUT = 5.0

no_permission_stops_root = pytest.mark.skipif(
    hasattr(os, "geteuid") and os.geteuid() == 0,
    reason="no permission stops root",
)

_CHILD = """
import sys
from pathlib import Path
from tasks.shared.shared_fixtures.runs import FixtureRuns

run = FixtureRuns.open(Path(sys.argv[1])).start()
print(run.run_id, flush=True)
sys.stdin.read()
run.close()
"""


class Child:
    def __init__(self, root: Path) -> None:
        self.process = subprocess.Popen(
            [sys.executable, "-c", _CHILD, str(root)],
            cwd=REPO_ROOT,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
        )
        self.run_id = self._handshake()

    def _handshake(self) -> str:
        assert self.process.stdout is not None
        ready, _, _ = select.select(
            [self.process.stdout], [], [], HANDSHAKE_TIMEOUT
        )
        assert ready, "the child never reported its run"
        return self.process.stdout.readline().strip()

    def kill(self) -> None:
        self.process.send_signal(signal.SIGKILL)
        self.process.wait(timeout=HANDSHAKE_TIMEOUT)

    def stop(self) -> None:
        if self.process.poll() is None:
            self.process.kill()
        self.process.communicate(timeout=HANDSHAKE_TIMEOUT)


@pytest.fixture
def children() -> Iterator[Callable[[Path], Child]]:
    started: list[Child] = []

    def start(root: Path) -> Child:
        child = Child(root)
        started.append(child)
        return child

    yield start
    for child in started:
        child.stop()


@pytest.fixture
def root(tmp_path: Path) -> Path:
    return tmp_path / "accelerator-shared-fixtures-test"


def _abandoned_run(root: Path, run_id: str = RUN_ID) -> Path:
    directory = root / run_id
    (directory / "workbenches").mkdir(parents=True)
    (root / f"{run_id}.lock").touch()
    return directory


def _held(lock: Path) -> int:
    fd = os.open(lock, os.O_CREAT | os.O_RDWR)
    assert try_exclusive_flock(fd)
    return fd


def _status_with(path: Path, *, uid: int) -> os.stat_result:
    real = os.lstat(path)
    return os.stat_result(
        (
            real.st_mode,
            real.st_ino,
            real.st_dev,
            real.st_nlink,
            uid,
            real.st_gid,
            real.st_size,
            int(real.st_atime),
            int(real.st_mtime),
            int(real.st_ctime),
        )
    )


def _failing_with(number: int) -> Callable[..., object]:
    def fail(*_: object) -> object:
        raise OSError(number, os.strerror(number))

    return fail


class TestSharedFixturesRoot:
    def test_is_named_for_the_user_under_the_resolved_temp_directory(self):
        root = shared_fixtures_root()

        assert root.name == f"accelerator-shared-fixtures-{os.getuid()}"
        assert root.parent == root.parent.resolve()


class TestOpeningTheRoot:
    def test_a_missing_root_is_created_private_to_its_user(self, root: Path):
        FixtureRuns.open(root)

        assert root.is_dir()
        assert root.stat().st_mode & 0o777 == 0o700

    def test_a_symlinked_root_is_refused_and_its_target_kept(
        self, root: Path, tmp_path: Path
    ):
        target = tmp_path / "elsewhere"
        target.mkdir()
        (target / "precious").write_text("keep")
        root.symlink_to(target)

        with pytest.raises(SharedFixturesError, match="move or remove"):
            FixtureRuns.open(root)

        assert (target / "precious").read_text() == "keep"

    def test_a_root_that_is_not_a_directory_is_refused(self, root: Path):
        root.write_text("not a directory")

        with pytest.raises(SharedFixturesError, match=str(root)):
            FixtureRuns.open(root)

        assert root.read_text() == "not a directory"

    def test_a_root_owned_by_another_user_is_refused(self, root: Path):
        root.mkdir(mode=0o700)

        with pytest.raises(SharedFixturesError, match="owner") as refusal:
            FixtureRuns.open(
                root,
                lstat=lambda path: _status_with(path, uid=os.getuid() + 1),
            )

        assert "the next run recreates it with mode 0700" in str(refusal.value)

    @pytest.mark.parametrize("mode", [0o720, 0o702])
    def test_a_root_others_can_write_is_refused(self, root: Path, mode: int):
        root.mkdir()
        root.chmod(mode)

        with pytest.raises(SharedFixturesError, match=f"mode {mode:o}"):
            FixtureRuns.open(root)

    @pytest.mark.parametrize("marker", [".git", ".jj"])
    def test_a_root_inside_a_checkout_is_refused_untouched(
        self, tmp_path: Path, marker: str
    ):
        (tmp_path / "checkout" / marker).mkdir(parents=True)
        root = tmp_path / "checkout" / "shared"

        with pytest.raises(SharedFixturesError) as refusal:
            FixtureRuns.open(root)

        assert refusal.value.problems == (
            f"the shared root {root} lies inside the checkout at "
            f"{tmp_path / 'checkout'}; set TMPDIR to a directory outside "
            "any git or jj checkout",
        )
        assert not root.exists()

    def test_a_root_another_run_creates_first_is_checked_and_used(
        self, root: Path
    ):
        reported_missing = False

        def created_by_another_run(path: Path) -> os.stat_result:
            nonlocal reported_missing
            if not reported_missing:
                reported_missing = True
                root.mkdir(mode=0o700)
                raise FileNotFoundError(path)
            return os.lstat(path)

        FixtureRuns.open(root, lstat=created_by_another_run).start().close()

        assert reported_missing


class TestStartingARun:
    def test_holds_a_lock_named_for_the_run(self, root: Path):
        run = FixtureRuns.open(root).start()
        try:
            contender = os.open(root / f"{run.run_id}.lock", os.O_RDWR)
            try:
                assert try_exclusive_flock(contender) is False
            finally:
                os.close(contender)
        finally:
            run.close()

    def test_the_run_id_is_a_utc_timestamp_and_a_uuid4(self, root: Path):
        run = FixtureRuns.open(root).start()
        run.close()

        timestamp, _, unique = run.run_id.partition("Z-")
        assert len(timestamp) == len("20261119T101500123456")
        assert unique[14] == "4"

    def test_lays_out_the_run_directory(self, root: Path):
        run = FixtureRuns.open(root).start()
        try:
            assert run.directory == root / run.run_id
            assert run.workbenches == run.directory / "workbenches"
            assert run.workbenches.is_dir()
            assert run.tree(Family.MATRIX) == run.directory / "matrix"
        finally:
            run.close()

    def test_leaves_no_temporary_lock_behind(self, root: Path):
        run = FixtureRuns.open(root).start()
        run.close()

        assert list(root.iterdir()) == []

    def test_never_replaces_an_existing_lock(self, root: Path):
        runs = FixtureRuns.open(root, new_run_id=lambda: RUN_ID)
        (root / f"{RUN_ID}.lock").write_text("someone else's")

        with pytest.raises(SharedFixturesError, match="already exists"):
            runs.start()

        assert (root / f"{RUN_ID}.lock").read_text() == "someone else's"
        assert not (root / RUN_ID).exists()

    def test_a_filesystem_without_flock_is_named(self, root: Path):
        runs = FixtureRuns.open(root, flock=_failing_with(errno.ENOLCK))

        with pytest.raises(
            SharedFixturesError, match="must support flock and hard links"
        ):
            runs.start()

        assert list(root.iterdir()) == []

    def test_a_lock_it_could_not_take_is_never_put_in_place(self, root: Path):
        runs = FixtureRuns.open(root, flock=lambda descriptor: False)

        with pytest.raises(SharedFixturesError, match="could not take"):
            runs.start()

        assert list(root.iterdir()) == []

    def test_a_flock_that_excludes_nothing_is_named(self, root: Path):
        runs = FixtureRuns.open(root, flock=lambda descriptor: True)

        with pytest.raises(
            SharedFixturesError, match="does not exclude a second holder"
        ):
            runs.start()

        assert list(root.iterdir()) == []

    @pytest.mark.parametrize("number", [errno.EPERM, errno.ENOSYS])
    def test_a_filesystem_without_hard_links_is_named(
        self, root: Path, number: int
    ):
        runs = FixtureRuns.open(root, link=_failing_with(number))

        with pytest.raises(SharedFixturesError, match="set TMPDIR"):
            runs.start()

        assert list(root.iterdir()) == []

    def test_hands_over_each_family_the_workbenches_and_the_log(
        self, root: Path
    ):
        run = FixtureRuns.open(root).start()
        try:
            handover = run.handover([Family.MATRIX], Path("/logs/run.jsonl"))
        finally:
            run.close()

        assert handover == {
            Family.MATRIX.handover_variable: str(run.directory / "matrix"),
            WORKBENCHES_VARIABLE: str(run.workbenches),
            FIXTURE_LOG_VARIABLE: "/logs/run.jsonl",
        }


class TestClosingARun:
    def test_removes_the_directory_and_the_lock(self, root: Path):
        run = FixtureRuns.open(root).start()
        (run.tree(Family.MATRIX) / "PG").mkdir(parents=True)

        run.close()

        assert list(root.iterdir()) == []

    @no_permission_stops_root
    def test_removes_a_directory_a_test_left_read_only(self, root: Path):
        run = FixtureRuns.open(root).start()
        stuck = run.workbenches / "vcs-workbench-a"
        stuck.mkdir()
        (stuck / "file").write_text("x")
        stuck.chmod(0o500)

        run.close()

        assert list(root.iterdir()) == []

    def test_releases_the_lock(self, root: Path):
        run = FixtureRuns.open(root).start()
        lock = root / f"{run.run_id}.lock"
        contender = os.open(lock, os.O_RDWR)
        try:
            run.close()

            assert try_exclusive_flock(contender) is True
        finally:
            os.close(contender)


class TestSweepingAbandonedRuns:
    def test_removes_a_run_whose_process_was_killed(self, root: Path, children):
        child = children(root)
        child.kill()

        FixtureRuns.open(root).sweep_abandoned()

        assert list(root.iterdir()) == []

    def test_keeps_a_run_whose_process_is_alive(self, root: Path, children):
        child = children(root)

        FixtureRuns.open(root).sweep_abandoned()

        assert (root / child.run_id).is_dir()
        assert (root / f"{child.run_id}.lock").exists()

    def test_removes_an_orphan_run_directory(self, root: Path):
        _abandoned_run(root)
        (root / f"{RUN_ID}.lock").unlink()

        FixtureRuns.open(root).sweep_abandoned()

        assert list(root.iterdir()) == []

    def test_an_orphan_is_judged_at_the_moment_of_the_decision(
        self, root: Path
    ):
        directory = _abandoned_run(root)
        (root / f"{RUN_ID}.lock").unlink()
        holders: list[int] = []

        def lock_appearing_after_the_listing(path: Path) -> int:
            holders.append(_held(path))
            return os.open(path, os.O_RDWR)

        try:
            FixtureRuns.open(
                root, open_lock=lock_appearing_after_the_listing
            ).sweep_abandoned()
        finally:
            for fd in holders:
                os.close(fd)

        assert directory.is_dir()

    def test_leaves_a_foreign_entry_and_names_it(self, root: Path, capsys):
        runs = FixtureRuns.open(root)
        (root / "someone-elses").mkdir()

        runs.sweep_abandoned()

        assert (root / "someone-elses").is_dir()
        out = capsys.readouterr().out
        assert "someone-elses" in out
        assert "remove it by hand if no run is using it" in out

    def test_removes_an_old_unlocked_temporary_lock(self, root: Path):
        runs = FixtureRuns.open(root)
        temporary = root / f"{RUN_ID}.lock.tmp0123"
        temporary.touch()
        an_hour_ago = time.time() - 3600
        os.utime(temporary, (an_hour_ago, an_hour_ago))

        runs.sweep_abandoned()

        assert not temporary.exists()

    def test_keeps_a_young_temporary_lock(self, root: Path):
        runs = FixtureRuns.open(root)
        temporary = root / f"{RUN_ID}.lock.tmp0123"
        temporary.touch()

        runs.sweep_abandoned()

        assert temporary.exists()

    @no_permission_stops_root
    def test_keeps_a_run_whose_lock_cannot_be_opened(self, root: Path, capsys):
        runs = FixtureRuns.open(root)
        directory = _abandoned_run(root)
        (root / f"{RUN_ID}.lock").chmod(0o000)

        runs.sweep_abandoned()

        assert directory.is_dir()
        assert RUN_ID in capsys.readouterr().out

    def test_keeps_a_run_whose_lock_cannot_be_taken(self, root: Path, capsys):
        directory = _abandoned_run(root)
        runs = FixtureRuns.open(root, flock=_failing_with(errno.ENOLCK))

        runs.sweep_abandoned()

        assert directory.is_dir()
        assert RUN_ID in capsys.readouterr().out

    def test_never_recreates_a_lock_another_sweep_removed(self, root: Path):
        _abandoned_run(root)

        def removed_by_another_sweep(path: Path) -> int:
            path.unlink()
            return os.open(path, os.O_RDWR)

        FixtureRuns.open(
            root, open_lock=removed_by_another_sweep
        ).sweep_abandoned()

        assert list(root.iterdir()) == []

    def test_sweeps_a_run_whose_directory_is_already_gone(self, root: Path):
        directory = _abandoned_run(root)

        def directory_removed_by_another_sweep(path: Path) -> int:
            shutil.rmtree(directory)
            return os.open(path, os.O_RDWR)

        FixtureRuns.open(
            root, open_lock=directory_removed_by_another_sweep
        ).sweep_abandoned()

        assert list(root.iterdir()) == []

    @no_permission_stops_root
    def test_removes_a_read_only_leftover(self, root: Path):
        directory = _abandoned_run(root)
        (directory / "workbenches" / "file").write_text("x")
        (directory / "workbenches").chmod(0o500)

        FixtureRuns.open(root).sweep_abandoned()

        assert list(root.iterdir()) == []

    @no_permission_stops_root
    def test_making_a_leftover_writable_follows_no_link(
        self, root: Path, tmp_path: Path
    ):
        outside = tmp_path / "outside"
        outside.write_text("x")
        outside.chmod(0o400)
        directory = _abandoned_run(root)
        (directory / "workbenches" / "link").symlink_to(outside)
        (directory / "workbenches" / "linked").hardlink_to(outside)
        (directory / "workbenches").chmod(0o500)

        FixtureRuns.open(root).sweep_abandoned()

        assert list(root.iterdir()) == []
        assert outside.stat().st_mode & 0o777 == 0o400

    def test_warns_of_a_leftover_it_cannot_remove_and_carries_on(
        self, root: Path, capsys
    ):
        _abandoned_run(root, RUN_ID)
        _abandoned_run(root, OTHER_RUN_ID)

        def unremovable(path: Path) -> None:
            if path.name == RUN_ID:
                raise PermissionError(errno.EACCES, "denied", str(path))
            shutil.rmtree(path)

        FixtureRuns.open(root, rmtree=unremovable).sweep_abandoned()

        assert not (root / OTHER_RUN_ID).exists()
        out = capsys.readouterr().out
        assert str(root / RUN_ID) in out
        assert "remove it by hand if no run is using it" in out


def _writable(path: Path) -> bool:
    return bool(path.lstat().st_mode & 0o222)


class TestSealingATree:
    @pytest.fixture(autouse=True)
    def removed_afterwards(self, tmp_path: Path) -> Iterator[None]:
        yield
        remove_leftover(tmp_path / "matrix")

    def test_leaves_nothing_in_it_writable(self, tmp_path: Path):
        tree = tmp_path / "matrix"
        (tree / "PG" / ".git").mkdir(parents=True)
        (tree / "PG" / ".git" / "config").write_text("x")

        seal(tree)

        assert [
            path for path in [tree, *tree.rglob("*")] if _writable(path)
        ] == []

    def test_follows_no_link(self, tmp_path: Path):
        outside = tmp_path / "outside"
        outside.write_text("x")
        tree = tmp_path / "matrix"
        tree.mkdir()
        (tree / "link").symlink_to(outside)

        seal(tree)

        assert _writable(outside)

    @no_permission_stops_root
    def test_refuses_a_write_into_it(self, tmp_path: Path):
        tree = tmp_path / "matrix"
        tree.mkdir()

        seal(tree)

        with pytest.raises(PermissionError):
            (tree / "poison.gitconfig").write_text("x")

    def test_leaves_it_removable(self, tmp_path: Path):
        tree = tmp_path / "matrix"
        (tree / "PG").mkdir(parents=True)
        (tree / "PG" / "file").write_text("x")
        seal(tree)

        assert remove_leftover(tree)
        assert not tree.exists()


class TestLiveRunIds:
    def test_names_the_runs_whose_lock_is_held(self, root: Path, children):
        _abandoned_run(root)
        child = children(root)

        assert FixtureRuns.open(root).live_run_ids() == {child.run_id}

    def test_counts_a_run_whose_lock_cannot_be_taken(self, root: Path):
        _abandoned_run(root)
        runs = FixtureRuns.open(root, flock=_failing_with(errno.ENOLCK))

        assert runs.live_run_ids() == {RUN_ID}

    @no_permission_stops_root
    def test_counts_a_run_whose_lock_cannot_be_opened(self, root: Path):
        _abandoned_run(root)
        (root / f"{RUN_ID}.lock").chmod(0o000)

        assert FixtureRuns.open(root).live_run_ids() == {RUN_ID}


class TestHoldPoint:
    def test_waits_for_release_after_saying_where_the_run_is(
        self, tmp_path: Path
    ):
        hold = tmp_path / "hold"
        run = tmp_path / "run"

        def release_shortly() -> None:
            while not (hold / "held").exists():
                time.sleep(0.005)
            (hold / "release").touch()

        releaser = threading.Thread(target=release_shortly)
        releaser.start()
        HoldPoint(hold, poll=0.005, timeout=HANDSHAKE_TIMEOUT).wait(run)
        releaser.join(timeout=HANDSHAKE_TIMEOUT)

        assert (hold / "held").read_text() == str(run)

    def test_a_hold_nobody_releases_times_out(self, tmp_path: Path):
        hold = tmp_path / "hold"

        with pytest.raises(
            SharedFixturesError,
            match=f"shared VCS fixtures: nothing released {hold} within",
        ):
            HoldPoint(hold, poll=0.001, timeout=0.01).wait(tmp_path / "run")

    def test_is_absent_unless_asked_for(self):
        assert HoldPoint.from_environment({}) is None

    def test_reads_its_directory_from_the_environment(self, tmp_path: Path):
        hold = HoldPoint.from_environment({HOLD_VARIABLE: str(tmp_path)})

        assert hold is not None
        assert hold.directory == tmp_path

    def test_is_never_honoured_in_ci(self, tmp_path: Path):
        with pytest.raises(
            SharedFixturesError, match=f"{HOLD_VARIABLE}.*never honoured in CI"
        ):
            HoldPoint.from_environment(
                {HOLD_VARIABLE: str(tmp_path), "CI": "true"}
            )
