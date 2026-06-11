from pathlib import Path

from tasks.shared.polling import wait_for_file
from tests.unit.tasks.shared.doubles import FakeClock


class TestWaitForFile:
    def test_returns_true_immediately_when_present(self, tmp_path: Path):
        target = tmp_path / "f"
        target.write_text("x")
        clock = FakeClock()
        assert wait_for_file(target, sleep=clock.sleep, now=clock.now) is True
        assert clock.sleeps == []  # never slept

    def test_returns_true_as_soon_as_it_appears(self, tmp_path: Path):
        target = tmp_path / "f"
        clock = FakeClock()

        def sleep(dt):
            clock.sleep(dt)
            if len(clock.sleeps) == 3:
                target.write_text("x")  # appears on the 3rd poll

        assert (
            wait_for_file(
                target, timeout=10, interval=1, sleep=sleep, now=clock.now
            )
            is True
        )
        assert len(clock.sleeps) == 3

    def test_returns_false_after_timeout_with_exact_sleep_count(
        self, tmp_path: Path
    ):
        target = tmp_path / "never"
        clock = FakeClock()
        # deadline=10, interval=1: sleeps from t=0->1 .. t=9->10 = 10 sleeps.
        result = wait_for_file(
            target, timeout=10, interval=1, sleep=clock.sleep, now=clock.now
        )
        assert result is False
        assert len(clock.sleeps) == 10

    def test_last_sleep_is_clamped_to_the_deadline(self, tmp_path: Path):
        target = tmp_path / "never"
        clock = FakeClock()
        # deadline=5, interval=2: sleeps 2, 2, then 1 (clamped), never past 5.
        wait_for_file(
            target, timeout=5, interval=2, sleep=clock.sleep, now=clock.now
        )
        assert clock.sleeps == [2, 2, 1]
        assert clock.t == 5  # never slept past the deadline
