import os
import signal
import sys
import textwrap
import threading
import time

import pytest

from tasks.shared.build_measurement.build_group import BuildGroupFile
from tasks.shared.build_measurement.build_runner import SubprocessBuildRunner
from tasks.shared.build_measurement.errors import BuildLeftRunningError
from tasks.shared.build_measurement.locations import BUILD_PGID
from tasks.shared.build_measurement.timeline import read_stamped
from tasks.shared.clock import Clock
from tasks.shared.processes import PsutilProcessOps
from tasks.shared.signals import unwinding_signals
from tests.unit.tasks.shared.build_measurement.doubles import FakeBuildGroup

_DEADLINE_SECONDS = 10.0
_GRACE_SECONDS = 0.5


def _python(source: str) -> tuple[str, ...]:
    return (sys.executable, "-u", "-c", textwrap.dedent(source))


def _wait_until(predicate) -> None:
    deadline = time.monotonic() + _DEADLINE_SECONDS
    while not predicate():
        assert time.monotonic() < deadline, "timed out waiting"
        time.sleep(0.02)


def _runner(tmp_path, *, procs=None, build_group=None):
    procs = procs or PsutilProcessOps()
    return SubprocessBuildRunner(
        root=tmp_path,
        clock=Clock(),
        procs=procs,
        build_group=build_group or BuildGroupFile(tmp_path, procs),
        grace=_GRACE_SECONDS,
    )


def _interrupt_main_thread_once(log, *, lines):
    def interrupt():
        _wait_until(
            lambda: log.exists() and log.read_text().count("\n") >= lines
        )
        signal.pthread_kill(threading.main_thread().ident, signal.SIGTERM)

    helper = threading.Thread(target=interrupt, daemon=True)
    helper.start()
    return helper


def _pids_in(log) -> list[int]:
    return [
        int(line.text.removeprefix("pid "))
        for line in read_stamped(log.read_text(newline=""))
        if line.text.startswith("pid ")
    ]


def _all_dead(pids) -> bool:
    ops = PsutilProcessOps()
    return not any(ops.is_alive(pid) for pid in pids)


_SPAWNS_A_GRANDCHILD = _python(
    """
    import os, subprocess, sys, time
    grandchild = subprocess.Popen(
        [sys.executable, "-c", "import time; time.sleep(30)"]
    )
    print(f"pid {os.getpid()}")
    print(f"pid {grandchild.pid}")
    time.sleep(30)
    """
)
_IGNORES_SIGTERM = _python(
    """
    import os, signal, time
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    print(f"pid {os.getpid()}")
    time.sleep(30)
    """
)


class SignalOnFirstGroupSignal(PsutilProcessOps):
    def __init__(self):
        self.signalled = 0

    def signal_group(self, pgid, number):
        self.signalled += 1
        if self.signalled == 1:
            os.kill(os.getpid(), signal.SIGTERM)
        super().signal_group(pgid, number)


class SurvivingGroup(PsutilProcessOps):
    def group_alive(self, pgid):
        return True


class TestStreaming:
    def test_the_build_stamps_lines_in_arrival_order(self, tmp_path):
        result = _runner(tmp_path).run(
            _python('import time; print("a"); time.sleep(0.5); print("b")'),
            epoch=Clock().now(),
            log=tmp_path / "run-1.log",
        )
        first, second = result.lines
        assert (first.text, second.text) == ("a", "b")
        assert second.at - first.at > 0.3

    def test_the_build_merges_stderr_and_reports_the_exit_status(
        self, tmp_path
    ):
        result = _runner(tmp_path).run(
            _python(
                'import sys; print("out"); print("err", file=sys.stderr); '
                "sys.exit(3)"
            ),
            epoch=Clock().now(),
            log=tmp_path / "run-1.log",
        )
        assert sorted(line.text for line in result.lines) == ["err", "out"]
        assert result.exit_status == 3

    def test_the_log_holds_every_stamped_line_the_run_returns(self, tmp_path):
        log = tmp_path / "run-1.log"
        result = _runner(tmp_path).run(
            _python('for n in range(5): print(f"line {n}")'),
            epoch=Clock().now(),
            log=log,
        )
        assert read_stamped(log.read_text(newline="")) == result.lines

    def test_a_carriage_return_stays_inside_its_line(self, tmp_path):
        result = _runner(tmp_path).run(
            _python(
                'import sys; sys.stdout.write("[t] Building\\r'
                '   Compiling foo v0.1.0\\n")'
            ),
            epoch=Clock().now(),
            log=tmp_path / "run-1.log",
        )
        assert [line.text for line in result.lines] == [
            "[t] Building\r   Compiling foo v0.1.0"
        ]

    def test_an_invalid_utf8_byte_is_replaced_not_fatal(self, tmp_path):
        result = _runner(tmp_path).run(
            _python('import sys; sys.stdout.buffer.write(b"[t] caf\\xff\\n")'),
            epoch=Clock().now(),
            log=tmp_path / "run-1.log",
        )
        assert [line.text for line in result.lines] == ["[t] caf�"]


class TestEnvironment:
    def test_a_nested_build_reports_task_timings_as_a_direct_run_would(
        self, tmp_path, monkeypatch
    ):
        monkeypatch.setenv("MISE_TASK_TIMINGS", "0")
        monkeypatch.setenv("MISE_TASK_NAME", "measure:build")
        monkeypatch.setenv("MISE_TASK_PGID_MANAGED", "1")
        result = _runner(tmp_path).run(
            _python(
                "import os; "
                'print(os.environ.get("MISE_TASK_TIMINGS")); '
                "print(sorted(k for k in os.environ "
                'if k.startswith("MISE_TASK_")))'
            ),
            epoch=Clock().now(),
            log=tmp_path / "run-1.log",
        )
        assert [line.text for line in result.lines] == [
            "1",
            "['MISE_TASK_TIMINGS']",
        ]


class TestBuildGroupFile:
    def test_the_builds_group_is_remembered_while_it_runs_and_forgotten_after(
        self, tmp_path
    ):
        log = tmp_path / "run-1.log"
        seen = []

        def watch():
            _wait_until(lambda: log.exists() and "pid " in log.read_text())
            seen.append((tmp_path / BUILD_PGID).read_text())

        watcher = threading.Thread(target=watch, daemon=True)
        watcher.start()
        _runner(tmp_path).run(
            _python(
                'import os, time; print(f"pid {os.getpid()}"); time.sleep(1)'
            ),
            epoch=Clock().now(),
            log=log,
        )
        watcher.join(timeout=_DEADLINE_SECONDS)
        [child] = _pids_in(log)
        assert seen[0].split()[0] == str(child)
        assert not (tmp_path / BUILD_PGID).exists()

    def test_the_builds_group_is_forgotten_after_an_interrupt(self, tmp_path):
        log = tmp_path / "run-1.log"
        _interrupt_main_thread_once(log, lines=2)
        with pytest.raises(KeyboardInterrupt), unwinding_signals():
            _runner(tmp_path).run(
                _SPAWNS_A_GRANDCHILD, epoch=Clock().now(), log=log
            )
        assert not (tmp_path / BUILD_PGID).exists()


class TestInterruption:
    def test_an_interrupted_build_leaves_no_child_or_grandchild(self, tmp_path):
        log = tmp_path / "run-1.log"
        _interrupt_main_thread_once(log, lines=2)
        with pytest.raises(KeyboardInterrupt), unwinding_signals():
            _runner(tmp_path).run(
                _SPAWNS_A_GRANDCHILD, epoch=Clock().now(), log=log
            )
        pids = _pids_in(log)
        assert len(pids) == 2
        _wait_until(lambda: _all_dead(pids))

    @pytest.mark.parametrize("entry", ["interrupt", "error"])
    def test_a_signal_during_the_reap_cannot_skip_the_kill(
        self, tmp_path, entry
    ):
        procs = SignalOnFirstGroupSignal()
        build_group = FakeBuildGroup()
        log = tmp_path / "run-1.log"
        if entry == "interrupt":
            _interrupt_main_thread_once(log, lines=1)
        else:
            log.mkdir()
        raised = 0
        try:
            with unwinding_signals():
                _runner(tmp_path, procs=procs, build_group=build_group).run(
                    _IGNORES_SIGTERM, epoch=Clock().now(), log=log
                )
        except KeyboardInterrupt:
            raised += 1
        assert raised == 1
        [(pgid, _)] = build_group.remembered
        assert build_group.forgotten == 1
        _wait_until(lambda: not PsutilProcessOps().group_alive(pgid))

    def test_a_group_that_survives_the_reap_is_named_with_its_kill_command(
        self, tmp_path
    ):
        build_group = FakeBuildGroup()
        log = tmp_path / "run-1.log"
        log.mkdir()
        with pytest.raises(BuildLeftRunningError) as survived:
            _runner(
                tmp_path, procs=SurvivingGroup(), build_group=build_group
            ).run(_IGNORES_SIGTERM, epoch=Clock().now(), log=log)
        [(pgid, _)] = build_group.remembered
        assert str(pgid) in str(survived.value)
        assert f"kill -KILL -{pgid}" in str(survived.value)

    def test_a_quiet_reap_does_not_mention_a_kill_command(self, tmp_path):
        log = tmp_path / "run-1.log"
        log.mkdir()
        with pytest.raises(OSError) as error:
            _runner(tmp_path).run(
                _IGNORES_SIGTERM, epoch=Clock().now(), log=log
            )
        assert "kill" not in str(error.value)
