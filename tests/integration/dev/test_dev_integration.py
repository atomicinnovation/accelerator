"""End-to-end integration tests for the unified dev task.

Drives the real ``invoke``-level orchestrators (via a subprocess driver) against
a real ``circusd``, using lightweight Python fake processes for the server and
frontend so the full lifecycle is exercised without building Rust or booting
Vite — identically on macOS and Linux. Registered as ``test:integration:dev``.

Timing-sensitive cases assert direction (it happened) with generous margins —
the precise poll-count/deadline maths is pinned by the deterministic unit tests
in ``test_dev.py``; the readiness timeout is parametrised small.
"""

import json
import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

import psutil
import pytest

from tasks.shared.dev.endpoints import ipc_socket_paths

REPO_ROOT = Path(__file__).resolve().parents[3]
DRIVER = Path(__file__).parent / "dev_integration_driver.py"


def run_driver(workspace: Path, action: str, opts: dict | None = None, timeout: float = 120):
    cmd = [sys.executable, str(DRIVER), "--workspace", str(workspace), "--action", action]
    if opts is not None:
        opts_path = workspace / f"opts-{action}.json"
        opts_path.write_text(json.dumps(opts))
        cmd += ["--opts", str(opts_path)]
    env = os.environ.copy()
    env["PYTHONPATH"] = str(REPO_ROOT)
    return subprocess.run(
        cmd, cwd=str(REPO_ROOT), env=env, capture_output=True, text=True, timeout=timeout
    )


def driver_payload(proc: subprocess.CompletedProcess) -> dict:
    line = proc.stdout.strip().splitlines()[-1]
    return json.loads(line)


def read_state(workspace: Path) -> dict | None:
    path = workspace / ".accelerator/tmp/dev/dev.json"
    return json.loads(path.read_text()) if path.exists() else None


def alive(pid: int | None) -> bool:
    if not pid:
        return False
    try:
        return psutil.Process(pid).status() != psutil.STATUS_ZOMBIE
    except psutil.NoSuchProcess:
        return False


def wait_for_text(path: Path, text: str, timeout: float = 5.0) -> bool:
    """Wait for ``text`` to appear in ``path`` (circus's FileStream capture lags
    the watcher's write, so the log can be momentarily empty after up returns)."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        if path.exists() and text in path.read_text():
            return True
        time.sleep(0.1)
    return path.exists() and text in path.read_text()


def descendants(pid: int | None) -> list[int]:
    if not pid:
        return []
    try:
        return [c.pid for c in psutil.Process(pid).children(recursive=True)]
    except psutil.NoSuchProcess:
        return []


def _hard_cleanup(workspace: Path) -> None:
    """Best-effort: stop the stack, kill any survivors, remove ipc sockets."""
    try:
        run_driver(workspace, "stop", timeout=30)
    except Exception:
        pass
    state = read_state(workspace)
    if state:
        for key in ("server_pid", "frontend_pid", "arbiter_pid"):
            pid = state.get(key)
            for child in descendants(pid):
                with _suppress():
                    psutil.Process(child).kill()
            if pid:
                with _suppress():
                    psutil.Process(pid).kill()
    with _suppress():
        shutil.rmtree(ipc_socket_paths(workspace)[0].parent, ignore_errors=True)


class _suppress:
    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return True


@pytest.fixture
def workspace(tmp_path):
    ws = tmp_path / "ws"
    ws.mkdir()
    yield ws
    _hard_cleanup(ws)


# ─── scenarios ───────────────────────────────────────────────


def test_detach_readiness_and_log_routing(workspace):
    proc = run_driver(workspace, "up")
    assert proc.returncode == 0, proc.stderr
    state = read_state(workspace)
    assert state["arbiter_pid"] and state["server_pid"] and state["frontend_pid"]
    assert state["frontend_port"]
    assert state["endpoint"].startswith("ipc://")
    # arbiter survives the (now-exited) launching process
    assert alive(state["arbiter_pid"])
    dev = workspace / ".accelerator/tmp/dev"
    server_log_path = dev / "server.log"
    frontend_log_path = dev / "frontend.log"
    # each marker reaches its own log (AC3) — wait for the async capture to flush
    assert wait_for_text(server_log_path, "SERVER_UP")
    assert wait_for_text(frontend_log_path, "FRONTEND_UP")
    # and crucially each marker appears ONLY in its own log (no cross-wiring)
    assert "FRONTEND_UP" not in server_log_path.read_text()
    assert "SERVER_UP" not in frontend_log_path.read_text()


def test_reuse_starts_no_duplicate(workspace):
    assert run_driver(workspace, "up").returncode == 0
    first = read_state(workspace)["arbiter_pid"]
    proc = run_driver(workspace, "up")
    assert proc.returncode == 0
    assert driver_payload(proc)["kind"] == "reused"
    assert read_state(workspace)["arbiter_pid"] == first


def test_stale_info_does_not_satisfy_gate(workspace):
    server_dir = workspace / ".accelerator/tmp/dev-server"
    server_dir.mkdir(parents=True)
    (server_dir / "server-info.json").write_text(
        json.dumps({"url": "http://127.0.0.1:1111", "port": 1111})
    )
    assert run_driver(workspace, "up", {"api_port": 8888}).returncode == 0
    info = json.loads((server_dir / "server-info.json").read_text())
    assert info["port"] == 8888  # the fresh server's file, not the stale 1111


def test_clean_teardown_leaves_no_orphans(workspace):
    assert run_driver(workspace, "up").returncode == 0
    state = read_state(workspace)
    tracked = (
        [state["server_pid"], state["frontend_pid"], state["arbiter_pid"]]
        + descendants(state["server_pid"])
        + descendants(state["frontend_pid"])
    )
    assert run_driver(workspace, "stop").returncode == 0
    for pid in tracked:
        assert not alive(pid)
    assert read_state(workspace) is None


def test_orphan_reach_when_arbiter_already_dead(workspace):
    assert run_driver(workspace, "up").returncode == 0
    state = read_state(workspace)
    server_pid, frontend_pid = state["server_pid"], state["frontend_pid"]
    os.kill(state["arbiter_pid"], signal.SIGKILL)  # children reparent to init
    time.sleep(0.5)
    assert alive(server_pid)  # still alive, no longer a child of the dead arbiter
    assert run_driver(workspace, "stop").returncode == 0
    assert not alive(server_pid)  # reaped via recorded server_pid
    assert not alive(frontend_pid)  # reaped via recorded frontend_pid


def test_orphan_reach_with_only_server_recorded(workspace):
    # The window after the readiness gate but before start_frontend: only
    # server_pid is recorded (frontend_pid still null). A crafted dev-state with
    # a live server subtree, a dead arbiter, and an unreachable endpoint must
    # still reap the server subtree via the incrementally-recorded server_pid.
    server = subprocess.Popen(
        [
            sys.executable,
            "-c",
            "import subprocess, sys, time; "
            "subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)']); "
            "time.sleep(60)",
        ]
    )
    try:
        time.sleep(0.5)
        child_pids = descendants(server.pid)
        assert child_pids  # the server spawned a worker
        dev = workspace / ".accelerator/tmp/dev"
        dev.mkdir(parents=True)
        endpoint, pubsub = ipc_socket_paths(workspace)
        state = {
            "endpoint": f"ipc://{endpoint}",  # no listener -> unreachable
            "pubsub_endpoint": f"ipc://{pubsub}",
            "frontend_port": 54321,
            "frontend_url": "http://127.0.0.1:54321",
            "pidfile": str(dev / "circusd.pid"),
            "ini_path": str(dev / "circus.ini"),
            "arbiter_pid": 2**31 - 1,  # a PID that cannot exist -> arbiter dead
            "arbiter_start_time": 1.0,
            "server_pid": server.pid,
            "server_start_time": psutil.Process(server.pid).create_time(),
            "frontend_pid": None,  # never reached start_frontend
            "frontend_start_time": None,
        }
        (dev / "dev.json").write_text(json.dumps(state))
        assert run_driver(workspace, "stop").returncode == 0
        assert not alive(server.pid)  # server subtree reaped via recorded server_pid
        for pid in child_pids:
            assert not alive(pid)
    finally:
        with _suppress():
            server.kill()


def test_restart_round_trip(workspace):
    assert run_driver(workspace, "up").returncode == 0
    first = read_state(workspace)["arbiter_pid"]
    proc = run_driver(workspace, "restart")
    assert proc.returncode == 0, proc.stderr
    state = read_state(workspace)
    assert state["arbiter_pid"] != first  # a genuinely new arbiter
    assert alive(state["arbiter_pid"])
    assert state["frontend_port"]


def test_readiness_timeout_fails_and_tears_down(workspace):
    proc = run_driver(
        workspace, "up", {"server_write_info": False, "readiness_timeout": 2.0}
    )
    assert proc.returncode == 1
    assert "server.log" in (driver_payload(proc)["message"] or "")
    assert read_state(workspace) is None  # torn down


def test_daemon_startup_failure_reaps_handle_no_retry(workspace):
    proc = run_driver(
        workspace, "up", {"fake_circusd_fail": True, "pidfile_timeout": 2.0}
    )
    assert proc.returncode == 1
    assert "circusd.log" in (driver_payload(proc)["message"] or "")


def test_discovery_survives_lost_state_file(workspace):
    assert run_driver(workspace, "up").returncode == 0
    arbiter = read_state(workspace)["arbiter_pid"]
    (workspace / ".accelerator/tmp/dev/dev.json").unlink()  # lose the cache
    assert run_driver(workspace, "stop").returncode == 0
    assert not alive(arbiter)  # found via recomputed deterministic ipc paths


def test_stale_cleanup_after_out_of_band_kill(workspace):
    assert run_driver(workspace, "up").returncode == 0
    state = read_state(workspace)
    for key in ("arbiter_pid", "server_pid", "frontend_pid"):
        with _suppress():
            os.kill(state[key], signal.SIGKILL)
    time.sleep(0.5)
    assert run_driver(workspace, "status").returncode == 4  # treated as not-running
    assert run_driver(workspace, "stop").returncode == 0
    endpoint_sock = ipc_socket_paths(workspace)[0]
    assert not endpoint_sock.exists()  # stale socket cleaned up


def test_recycled_pid_is_refused_real_psutil(workspace):
    victim = subprocess.Popen(["sleep", "30"])
    try:
        dev = workspace / ".accelerator/tmp/dev"
        dev.mkdir(parents=True)
        endpoint, pubsub = ipc_socket_paths(workspace)
        state = {
            "endpoint": f"ipc://{endpoint}",
            "pubsub_endpoint": f"ipc://{pubsub}",
            "frontend_port": 54321,
            "frontend_url": "http://127.0.0.1:54321",
            "pidfile": str(dev / "circusd.pid"),
            "ini_path": str(dev / "circus.ini"),
            "arbiter_pid": victim.pid,
            "arbiter_start_time": 1.0,  # bogus -> identity mismatch vs the live victim
            "server_pid": None,
            "server_start_time": None,
            "frontend_pid": None,
            "frontend_start_time": None,
        }
        (dev / "dev.json").write_text(json.dumps(state))
        proc = run_driver(workspace, "stop")
        assert proc.returncode == 1
        assert driver_payload(proc)["kind"] == "refused"
        assert alive(victim.pid)  # the unrelated process was never signalled
        assert (dev / "dev.json").exists()  # state kept
    finally:
        victim.kill()


def test_frontend_resolves_npm_under_stripped_path(workspace):
    proc = run_driver(workspace, "up", {"strip_path": True})
    assert proc.returncode == 0, proc.stderr
    # frontend started despite a stripped PATH -> the absolute npm render worked
    assert read_state(workspace)["frontend_pid"]


def test_frontend_port_and_info_wiring(workspace):
    assert run_driver(workspace, "up", {"free_port": 45678}).returncode == 0
    ini = (workspace / ".accelerator/tmp/dev/circus.ini").read_text()
    assert "--port 45678 --strictPort" in ini
    assert "VISUALISER_INFO_PATH" in ini
    assert read_state(workspace)["frontend_port"] == 45678


def test_status_exit_codes(workspace):
    assert run_driver(workspace, "status").returncode == 4  # neither
    assert run_driver(workspace, "up").returncode == 0
    assert run_driver(workspace, "status").returncode == 0  # both
    state = read_state(workspace)
    os.kill(state["frontend_pid"], signal.SIGKILL)  # respawn=false -> stays stopped
    time.sleep(0.7)
    assert run_driver(workspace, "status").returncode == 3  # one


def test_sigterm_ignoring_frontend_is_sigkilled(workspace):
    assert run_driver(workspace, "up", {"fe_ignore_sigterm": True}).returncode == 0
    frontend_pid = read_state(workspace)["frontend_pid"]
    assert alive(frontend_pid)
    assert run_driver(workspace, "stop", {"grace_quit": 8.0}).returncode == 0
    # circus escalated to SIGKILL past its 2 s graceful_timeout
    assert not alive(frontend_pid)


def test_cross_workspace_concurrency(tmp_path):
    ws_a, ws_b = tmp_path / "a", tmp_path / "b"
    ws_a.mkdir()
    ws_b.mkdir()
    try:
        assert run_driver(ws_a, "up").returncode == 0
        assert run_driver(ws_b, "up").returncode == 0
        a, b = read_state(ws_a), read_state(ws_b)
        assert a["frontend_port"] != b["frontend_port"]
        assert a["endpoint"] != b["endpoint"]  # distinct per-workspace ipc sockets
        assert alive(a["arbiter_pid"]) and alive(b["arbiter_pid"])
        # stopping A leaves B untouched (structural isolation)
        assert run_driver(ws_a, "stop").returncode == 0
        assert not alive(a["arbiter_pid"])
        assert alive(b["arbiter_pid"])
    finally:
        _hard_cleanup(ws_a)
        _hard_cleanup(ws_b)
