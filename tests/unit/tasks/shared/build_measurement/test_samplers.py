import sys
import textwrap
import threading
import time

from tasks.shared.build_measurement.samplers import (
    PsProcessSampler,
    StreamSampler,
)
from tasks.shared.build_measurement.timeline import read_stamped
from tasks.shared.build_measurement.tracked_processes import (
    DEFAULT_TRACKED_PROCESSES,
    process_snapshots,
)
from tasks.shared.clock import Clock
from tasks.shared.processes import PsutilProcessOps

_DEADLINE_SECONDS = 10.0
_SYSPOLICYD_LINE = "  479     1   3:21.94 /usr/libexec/syspolicyd"
_UNTRACKED_LINE = "  327     1  18:48.01 /usr/libexec/logd"


def _python(source: str) -> tuple[str, ...]:
    return (sys.executable, "-u", "-c", textwrap.dedent(source))


def _wait_until(predicate) -> None:
    deadline = time.monotonic() + _DEADLINE_SECONDS
    while not predicate():
        assert time.monotonic() < deadline, "timed out waiting"
        time.sleep(0.02)


_STREAM = _python(
    """
    import itertools, time
    for n in itertools.count():
        print(f"reading {n}")
        if n >= 50:
            time.sleep(30)
        time.sleep(0.05)
    """
)


def _ps_stand_in(tmp_path, *, fail_on_tick=None, sleep_on_tick=None):
    counter = tmp_path / "ticks"
    return _python(
        f"""
        import pathlib, sys, time
        counter = pathlib.Path({str(counter)!r})
        tick = int(counter.read_text()) + 1 if counter.exists() else 1
        counter.write_text(str(tick))
        if tick == {fail_on_tick!r}:
            sys.exit(1)
        if tick == {sleep_on_tick!r}:
            time.sleep(5)
        print({_SYSPOLICYD_LINE!r})
        print({_UNTRACKED_LINE!r})
        """
    )


def _markers(lines) -> list[str]:
    return [line.text for line in lines if line.text in {"sample", "gap"}]


class TestStreamSampler:
    def test_the_stream_sampler_stamps_each_line_until_stopped(self, tmp_path):
        clock = Clock()
        log = tmp_path / "iostat.log"
        running = StreamSampler(_STREAM, clock).start(
            epoch=clock.now(), log=log
        )
        _wait_until(lambda: log.exists() and log.read_text().count("\n") >= 3)
        lines = running.stop()
        assert [line.text for line in lines[:3]] == [
            "reading 0",
            "reading 1",
            "reading 2",
        ]
        assert lines[0].at < lines[2].at

    def test_each_sampler_writes_its_lines_to_its_log(self, tmp_path):
        clock = Clock()
        log = tmp_path / "iostat.log"
        running = StreamSampler(_STREAM, clock).start(
            epoch=clock.now(), log=log
        )
        _wait_until(lambda: log.exists() and log.read_text().count("\n") >= 2)
        lines = running.stop()
        assert read_stamped(log.read_text(newline="")) == lines

    def test_stopping_twice_returns_the_same_lines(self, tmp_path):
        clock = Clock()
        log = tmp_path / "iostat.log"
        running = StreamSampler(_STREAM, clock).start(
            epoch=clock.now(), log=log
        )
        _wait_until(lambda: log.exists() and log.read_text())
        assert running.stop() == running.stop()

    def test_a_stopped_stream_sampler_leaves_no_process(self, tmp_path):
        clock = Clock()
        log = tmp_path / "iostat.log"
        running = StreamSampler(_STREAM, clock).start(
            epoch=clock.now(), log=log
        )
        _wait_until(lambda: log.exists() and log.read_text())
        running.stop()
        assert not PsutilProcessOps().is_alive(running.pid)

    def test_sampler_threads_are_daemons(self, tmp_path):
        clock = Clock()
        before = set(threading.enumerate())
        running = StreamSampler(_STREAM, clock).start(
            epoch=clock.now(), log=tmp_path / "iostat.log"
        )
        started = set(threading.enumerate()) - before
        running.stop()
        assert started
        assert all(thread.daemon for thread in started)


class TestPsProcessSampler:
    def test_each_tick_writes_a_marker_and_only_tracked_lines(self, tmp_path):
        clock = Clock()
        log = tmp_path / "ps.log"
        running = PsProcessSampler(
            _ps_stand_in(tmp_path), clock, interval=0.05, tick_timeout=5.0
        ).start(epoch=clock.now(), log=log, tracked=DEFAULT_TRACKED_PROCESSES)
        _wait_until(
            lambda: log.exists() and log.read_text().count("sample") >= 2
        )
        lines = running.stop()
        assert {line.text for line in lines} == {"sample", _SYSPOLICYD_LINE}
        assert len(process_snapshots(lines).snapshots) >= 2

    def test_a_failing_tick_writes_a_gap_and_the_sampler_goes_on(
        self, tmp_path
    ):
        clock = Clock()
        log = tmp_path / "ps.log"
        running = PsProcessSampler(
            _ps_stand_in(tmp_path, fail_on_tick=1),
            clock,
            interval=0.05,
            tick_timeout=5.0,
        ).start(epoch=clock.now(), log=log, tracked=DEFAULT_TRACKED_PROCESSES)
        _wait_until(
            lambda: log.exists() and log.read_text().count("sample") >= 2
        )
        markers = _markers(running.stop())
        assert markers[0] == "gap"
        assert markers[1:3] == ["sample", "sample"]

    def test_a_tick_that_times_out_writes_a_gap(self, tmp_path):
        clock = Clock()
        log = tmp_path / "ps.log"
        sampler = PsProcessSampler(
            _ps_stand_in(tmp_path, sleep_on_tick=1),
            clock,
            interval=0.05,
            tick_timeout=0.2,
        )
        running = sampler.start(
            epoch=clock.now(), log=log, tracked=DEFAULT_TRACKED_PROCESSES
        )
        _wait_until(lambda: log.exists() and "sample" in log.read_text())
        markers = _markers(running.stop())
        assert markers[0] == "gap"
        assert "sample" in markers

    def test_stopping_twice_returns_the_same_lines(self, tmp_path):
        clock = Clock()
        log = tmp_path / "ps.log"
        running = PsProcessSampler(
            _ps_stand_in(tmp_path), clock, interval=0.05, tick_timeout=5.0
        ).start(epoch=clock.now(), log=log, tracked=DEFAULT_TRACKED_PROCESSES)
        _wait_until(lambda: log.exists() and "sample" in log.read_text())
        assert running.stop() == running.stop()
        assert read_stamped(log.read_text(newline="")) == running.stop()
