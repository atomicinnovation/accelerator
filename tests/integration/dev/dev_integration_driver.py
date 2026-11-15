#!/usr/bin/env python3
"""Subprocess driver for the dev-task integration suite.

Runs the *real* dev orchestrators (bring_up / do_stop / do_status / do_restart)
against a *real* circusd, using lightweight Python fake processes for the
"server" and "frontend" so the lifecycle is exercised without building Rust or
booting Vite. Its actions are run as a subprocess so the launching
process exits after ``up`` returns — reparenting the detached arbiter to init,
which is what makes the post-hoc ``psutil`` liveness checks in the tests
accurate (an un-reaped child would read as a zombie "alive" forever).

Usage:
    python dev_integration_driver.py --workspace <dir> --action up \
        [--opts <json-file>]
"""

import argparse
import dataclasses
import json
import os
import sys
import time
from pathlib import Path

# Make ``tasks`` importable when run as a bare script (cwd is the repo root).
sys.path.insert(0, str(Path.cwd()))

from tasks.shared.clock import Clock
from tasks.shared.dev.circus import (
    PopenHandle,
    default_client_factory,
    default_launcher,
)
from tasks.shared.dev.lifecycle import (
    DevDeps,
    UpResult,
    bring_up,
    do_restart,
    do_status,
    do_stop,
)
from tasks.shared.ports import free_port
from tasks.shared.processes import (
    START_TIME_TOLERANCE,
    ProcessOps,
    PsutilProcessOps,
)
from tests.integration.dev.fake_process import ESCALATIONS_LOG, EVENTS_LOG

REPO_ROOT = Path(__file__).resolve().parents[3]
FAKES_RECORDED_TIMEOUT = 20.0


@dataclasses.dataclass(frozen=True)
class RecordedFake:
    role: str
    pid: int
    start_time: float
    events: tuple[str, ...]

    def is_running(self) -> bool:
        processes = PsutilProcessOps()
        return processes.identity_matches(
            self.pid, self.start_time
        ) and processes.is_alive(self.pid)

    def is_one_of(self, identities: list[tuple[int, float]]) -> bool:
        return any(
            pid == self.pid
            and abs(start_time - self.start_time) <= START_TIME_TOLERANCE
            for pid, start_time in identities
        )


def recorded_fakes(workspace: Path) -> list[RecordedFake]:
    events_log = workspace / EVENTS_LOG
    if not events_log.exists():
        return []
    events: dict[tuple[str, str, str], list[str]] = {}
    for line in events_log.read_text().splitlines():
        fields = line.split()
        if len(fields) == 4:
            event, role, pid, start_time = fields
            events.setdefault((role, pid, start_time), []).append(event)
    return [
        RecordedFake(role, int(pid), float(start_time), tuple(logged))
        for (role, pid, start_time), logged in events.items()
    ]


# Absolute-interpreter shebang so the fakes run regardless of PATH (the
# stripped-PATH test must exercise npm resolution, not break the shebang).
# Circus's own arguments to the server and npm are deliberately ignored.
_LAUNCHER = """\
#!{interp}
import sys
sys.path.insert(0, {repo_root!r})
from tests.integration.dev.fake_process import main
main({argv!r})
"""


def _server_argv(workspace: Path, opts: dict) -> list[str]:
    argv = [
        "server",
        "--events-log",
        str(workspace / EVENTS_LOG),
        "--marker",
        opts.get("server_marker", "SERVER_UP"),
        "--port",
        str(opts.get("api_port", 7777)),
    ]
    if opts.get("server_write_info", True):
        argv.append("--write-info")
    if opts.get("server_ignore_sigterm"):
        argv.append("--ignore-sigterm")
    if opts.get("server_spawn_child", True):
        argv.append("--spawn-child")
    return argv


def _frontend_argv(workspace: Path, opts: dict) -> list[str]:
    argv = [
        "frontend",
        "--events-log",
        str(workspace / EVENTS_LOG),
        "--marker",
        opts.get("fe_marker", "FRONTEND_UP"),
    ]
    if opts.get("fe_ignore_sigterm"):
        argv.append("--ignore-sigterm")
    if opts.get("fe_spawn_child", True):
        argv.append("--spawn-child")
    return argv


def _write_launcher(path: Path, argv: list[str]) -> Path:
    path.write_text(
        _LAUNCHER.format(
            interp=sys.executable, repo_root=str(REPO_ROOT), argv=argv
        )
    )
    path.chmod(0o755)
    return path


def _write_fakes(workspace: Path, opts: dict) -> tuple[Path, Path]:
    bin_dir = workspace / "fakebin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    return (
        _write_launcher(bin_dir / "fake-server", _server_argv(workspace, opts)),
        _write_launcher(bin_dir / "fake-npm", _frontend_argv(workspace, opts)),
    )


def expected_roles(opts: dict) -> set[str]:
    roles = {"server", "frontend"}
    if opts.get("server_spawn_child", True):
        roles.add("server-child")
    if opts.get("fe_spawn_child", True):
        roles.add("frontend-child")
    return roles


def unrecorded_roles(workspace: Path, opts: dict) -> set[str]:
    """Wait for a running recorded fake in every role; return those missing.

    Circus reports a watcher active before its fake has installed its SIGTERM
    disposition, so a stop issued as soon as ``up`` returns could race it.
    """
    deadline = time.monotonic() + FAKES_RECORDED_TIMEOUT
    while True:
        running = {
            fake.role for fake in recorded_fakes(workspace) if fake.is_running()
        }
        missing = expected_roles(opts) - running
        if not missing or time.monotonic() >= deadline:
            return missing
        time.sleep(0.05)


class EscalationRecorder:
    """Records every signal our code sends to a process that is still alive.

    Liveness is checked first because ``identity_matches`` accepts zombies,
    so teardown routinely signals fakes that have exited but not been reaped.
    """

    def __init__(self, inner: ProcessOps, record: Path) -> None:
        self._inner = inner
        self._record = record
        record.touch()

    def is_alive(self, pid: int) -> bool:
        return self._inner.is_alive(pid)

    def create_time(self, pid: int) -> float | None:
        return self._inner.create_time(pid)

    def identity_matches(self, pid: int, start_time: float) -> bool:
        return self._inner.identity_matches(pid, start_time)

    def children(self, pid: int) -> list[tuple[int, float]]:
        return self._inner.children(pid)

    def terminate(self, pid: int) -> None:
        self._note_if_alive("terminate", pid)
        self._inner.terminate(pid)

    def kill(self, pid: int) -> None:
        self._note_if_alive("kill", pid)
        self._inner.kill(pid)

    def _note_if_alive(self, signal_name: str, pid: int) -> None:
        if self._inner.is_alive(pid):
            with self._record.open("a") as record:
                record.write(f"{signal_name} {pid}\n")


def _failing_launcher(argv, *, env, cwd):
    """A circusd stand-in that detaches but exits without writing a pidfile.

    Proves the daemon-startup-failure path: the Popen PID is the real (detached)
    process — reapable by handle — exactly as for a self-detached circusd.
    """
    import subprocess

    popen = subprocess.Popen(
        [sys.executable, "-c", "import time; time.sleep(2)"],
        env=env,
        cwd=cwd,
        start_new_session=True,
    )
    return PopenHandle(popen)


def _build_deps(workspace: Path, opts: dict) -> DevDeps:
    dev_dir = workspace / ".accelerator/tmp/dev"
    state_dir = workspace / ".accelerator/tmp/visualiser"
    dev_dir.mkdir(parents=True, exist_ok=True)
    state_dir.mkdir(parents=True, exist_ok=True)
    server_bin, npm_bin = _write_fakes(workspace, opts)
    server_info_path = state_dir / "server-info.json"

    env = os.environ.copy()
    env["ACCELERATOR_PLUGIN_ROOT"] = str(workspace)
    if opts.get("strip_path"):
        # Only the venv bin (for circusd) — NOT the fake npm's dir. The frontend
        # watcher must still resolve npm via the absolute path rendered into the
        # cmd, proving the PATH-drift mitigation.
        env["PATH"] = str(Path(sys.executable).parent)

    launcher = (
        _failing_launcher if opts.get("fake_circusd_fail") else default_launcher
    )

    return DevDeps(
        client_factory=default_client_factory,
        launcher=launcher,
        killer=EscalationRecorder(
            PsutilProcessOps(), workspace / ESCALATIONS_LOG
        ),
        clock=Clock(),
        project_root=workspace,
        workspace_root=workspace,
        state_path=dev_dir / "dev.json",
        lock_path=dev_dir / "dev.lock",
        dev_dir=dev_dir,
        pidfile=dev_dir / "circusd.pid",
        ini_path=dev_dir / "circus.ini",
        server_info_path=server_info_path,
        server_pidfile=state_dir / "server.pid",
        server_bin=server_bin,
        frontend=workspace / "frontend",
        diagnostic_log=dev_dir / "dev.log",
        env=env,
        npm_bin=str(npm_bin),
        node_bin="node",
        free_port=(lambda: opts["free_port"])
        if "free_port" in opts
        else free_port,
        # Happy-path defaults are deliberately generous: these are real
        # processes (circusd + Python fakes) started on shared CI runners that
        # may have only a few cores under heavy parallel load. The suite
        # asserts direction (it happened), not precise deadlines — the exact
        # poll-count/timeout maths is pinned by the unit tests in test_dev.py.
        # Negative-path tests override these with small values to exercise the
        # timeout branches.
        #
        # These are NOT what the long-running "frontend watcher did not become
        # active" flake needed: circus was refusing the start outright while
        # the arbiter finished start_watchers, so no budget could have helped.
        # See _request_frontend_start.
        probe_timeout=opts.get("probe_timeout", 5.0),
        pidfile_timeout=opts.get("pidfile_timeout", 20.0),
        readiness_timeout=opts.get("readiness_timeout", 20.0),
        frontend_active_timeout=opts.get("frontend_active_timeout", 20.0),
        grace_quit=opts.get("grace_quit", 6.0),
        grace_kill=opts.get("grace_kill", 3.0),
    )


def _report_up(result: UpResult, workspace: Path, opts: dict) -> int:
    kind, message = result.kind, result.message
    if kind in ("started", "reused"):
        missing = unrecorded_roles(workspace, opts)
        if missing:
            kind = "failed"
            message = f"no running recorded fake for: {sorted(missing)}"
    print(json.dumps({"kind": kind, "message": message}))
    return 0 if kind in ("started", "reused") else 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", required=True)
    parser.add_argument(
        "--action", required=True, choices=["up", "stop", "status", "restart"]
    )
    parser.add_argument("--opts")
    args = parser.parse_args()

    workspace = Path(args.workspace)
    opts = json.loads(Path(args.opts).read_text()) if args.opts else {}
    deps = _build_deps(workspace, opts)

    if args.action in ("up", "restart"):
        bring = bring_up if args.action == "up" else do_restart
        return _report_up(bring(deps), workspace, opts)
    if args.action == "stop":
        result = do_stop(deps)
        print(json.dumps({"kind": result.kind, "pid": result.pid}))
        return 0 if result.kind == "clean" else 1
    if args.action == "status":
        result = do_status(deps)
        print(
            json.dumps({"exit_code": result.exit_code, "lines": result.lines})
        )
        return result.exit_code
    return 2


if __name__ == "__main__":
    sys.exit(main())
