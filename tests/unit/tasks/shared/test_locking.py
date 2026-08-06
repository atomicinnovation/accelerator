import threading
from pathlib import Path

import pytest

from tasks.shared.locking import workspace_lock


class TestWorkspaceLock:
    def test_acquires_when_free(self, tmp_path: Path):
        lock = tmp_path / "dev.lock"
        with workspace_lock(lock) as acquired:
            assert acquired is True

    def test_fail_fast_when_held(self, tmp_path: Path):
        lock = tmp_path / "dev.lock"
        with workspace_lock(lock) as outer:
            assert outer is True
            with workspace_lock(lock) as inner:
                assert inner is False  # non-blocking: does not wait

    def test_released_on_context_exit(self, tmp_path: Path):
        lock = tmp_path / "dev.lock"
        with workspace_lock(lock) as first:
            assert first is True
        with workspace_lock(lock) as second:
            assert second is True  # re-acquirable after release


class TestWorkspaceLockMkdirFallback:
    """Exercise the mkdir fallback for filesystems without working flock."""

    @pytest.fixture(autouse=True)
    def _force_mkdir_branch(self, monkeypatch):
        from tasks.shared import locking

        monkeypatch.setattr(locking, "_try_import_fcntl", lambda: None)

    def test_acquire_and_release(self, tmp_path: Path):
        lock = tmp_path / "dev.lock"
        with workspace_lock(lock) as acquired:
            assert acquired is True
            assert (tmp_path / "dev.lock.d").is_dir()
        assert not (tmp_path / "dev.lock.d").exists()  # cleaned up on exit

    def test_fail_fast_when_held(self, tmp_path: Path):
        lock = tmp_path / "dev.lock"
        with workspace_lock(lock) as outer:
            assert outer is True
            with workspace_lock(lock) as inner:
                assert inner is False

    def test_reclaims_stale_dead_owner_lock(self, tmp_path: Path):
        import json

        lock = tmp_path / "dev.lock"
        lock_dir = tmp_path / "dev.lock.d"
        lock_dir.mkdir()
        # A dead owner: a PID that cannot exist.
        (lock_dir / "owner.json").write_text(
            json.dumps({"pid": 2**31 - 1, "start_time": 1.0})
        )
        with workspace_lock(lock) as acquired:
            assert acquired is True  # stale dir reclaimed

    def test_two_contenders_dead_owner_resolve_to_single_winner(
        self, tmp_path: Path
    ):
        import json

        lock = tmp_path / "dev.lock"
        lock_dir = tmp_path / "dev.lock.d"
        lock_dir.mkdir()
        (lock_dir / "owner.json").write_text(
            json.dumps({"pid": 2**31 - 1, "start_time": 1.0})
        )

        # The property is mutual exclusion — never two holders at once — not
        # "exactly one of the eight ever succeeds". Acquisition is
        # non-blocking, so a contender descheduled past the winner's hold and
        # release finds the lock free and legitimately takes it; counting
        # winners made that ordinary scheduling look like a lock failure, and
        # it duly failed under parallel CI load.
        import time

        occupancy = threading.Lock()
        held = 0
        overlaps = 0
        wins = 0

        def contend():
            nonlocal held, overlaps, wins
            barrier.wait()
            with workspace_lock(lock) as acquired:
                if not acquired:
                    return
                with occupancy:
                    wins += 1
                    held += 1
                    if held > 1:
                        overlaps += 1
                # hold briefly so any concurrent holder overlaps us here
                time.sleep(0.05)
                with occupancy:
                    held -= 1

        barrier = threading.Barrier(8)
        threads = [threading.Thread(target=contend) for _ in range(8)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        assert overlaps == 0, "two contenders held the lock at the same time"
        assert wins >= 1, "the stale lock was never reclaimed"
