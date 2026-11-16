"""End-to-end integration tests for the unified dev task.

Drives the real ``invoke``-level orchestrators (via a subprocess driver) against
a real ``circusd``, using lightweight Python fake processes for the server and
frontend so the full lifecycle is exercised without building Rust or booting
Vite — identically on macOS and Linux. Registered as ``test:integration:dev``.

Timing-sensitive cases assert direction (it happened) with generous margins —
the precise poll-count/deadline maths is pinned by the deterministic unit tests
in ``test_dev.py``; the readiness timeout is parametrised small.
"""

import ast
import contextlib
import dataclasses
import json
import os
import shutil
import signal
import subprocess
import sys
import time
from collections.abc import Callable
from pathlib import Path

import psutil
import pytest

from tasks.shared.dev.endpoints import ipc_socket_paths
from tasks.shared.processes import PsutilProcessOps
from tests.integration.dev.dev_integration_driver import recorded_fakes
from tests.integration.dev.fake_process import (
    ESCALATIONS_LOG,
    EVENTS_LOG,
    KILLED_BY_TEST_LOG,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
DRIVER = Path(__file__).parent / "dev_integration_driver.py"
FAKE_PROCESS = Path(__file__).parent / "fake_process.py"
LIFECYCLE = REPO_ROOT / "tasks/shared/dev/lifecycle.py"

TIMEOUT_BRANCH_EVIDENCE = (
    "server-info.json did not appear before the readiness timeout",
    "circusd did not write a valid pidfile at",
    "still alive after SIGKILL",
    "endpoint still reachable after quit",
    "did not become active",
)
READINESS_TIMEOUT_EVIDENCE, PIDFILE_TIMEOUT_EVIDENCE = TIMEOUT_BRANCH_EVIDENCE[
    :2
]


def run_driver(
    workspace: Path, action: str, opts: dict | None = None, timeout: float = 120
):
    cmd = [
        sys.executable,
        str(DRIVER),
        "--workspace",
        str(workspace),
        "--action",
        action,
    ]
    if opts is not None:
        opts_path = workspace / f"opts-{action}.json"
        opts_path.write_text(json.dumps(opts))
        cmd += ["--opts", str(opts_path)]
    env = os.environ.copy()
    env["PYTHONPATH"] = str(REPO_ROOT)
    return subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        env=env,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def assert_ok(proc: subprocess.CompletedProcess) -> subprocess.CompletedProcess:
    """Assert the driver succeeded, quoting BOTH streams when it did not.

    The orchestrators report a refusal on stdout (the JSON result) and only
    some paths also log to stderr, so asserting on stderr alone can produce a
    bare `AssertionError:` that says nothing — which is exactly what an
    intermittent failure leaves behind in a CI log.
    """
    assert proc.returncode == 0, (
        f"driver exited {proc.returncode}\n"
        f"stdout: {proc.stdout.strip()}\nstderr: {proc.stderr.strip()}"
    )
    return proc


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


def wait_until(condition: Callable[[], bool], timeout: float = 5.0) -> bool:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if condition():
            return True
        time.sleep(0.05)
    return condition()


def wait_for_text(path: Path, text: str, timeout: float = 5.0) -> bool:
    """Wait for ``text`` to appear in ``path`` (circus's FileStream capture
    lags the watcher's write, so the log can be momentarily empty after up
    returns)."""
    return wait_until(
        lambda: path.exists() and text in path.read_text(), timeout
    )


def descendants(pid: int | None) -> list[int]:
    if not pid:
        return []
    try:
        return [c.pid for c in psutil.Process(pid).children(recursive=True)]
    except psutil.NoSuchProcess:
        return []


@dataclasses.dataclass(frozen=True)
class Escalation:
    signal: str
    pid: int


def escalations(workspace: Path) -> list[Escalation]:
    record = workspace / ESCALATIONS_LOG
    assert record.exists(), f"no escalation record at {record}"
    return [
        Escalation(signal_name, int(pid))
        for signal_name, pid in (
            line.split() for line in record.read_text().splitlines()
        )
    ]


def kill_out_of_band(workspace: Path, pid: int) -> None:
    """SIGKILL ``pid`` and its whole subtree, recording who the test killed.

    The subtree is listed before anything is signalled: once the parent is
    dead its children are reparented and can no longer be found from it.
    """
    try:
        root = psutil.Process(pid)
        victims = [root, *root.children(recursive=True)]
    except psutil.NoSuchProcess:
        return
    identities = []
    for victim in victims:
        with contextlib.suppress(psutil.NoSuchProcess):
            identities.append((victim.pid, victim.create_time()))
    with (workspace / KILLED_BY_TEST_LOG).open("a") as log:
        log.writelines(f"{pid} {start}\n" for pid, start in identities)
    for victim in victims:
        with contextlib.suppress(psutil.NoSuchProcess):
            victim.kill()


def killed_by_test(workspace: Path) -> list[tuple[int, float]]:
    log = workspace / KILLED_BY_TEST_LOG
    if not log.exists():
        return []
    lines = log.read_text().splitlines()
    return [
        (int(pid), float(start))
        for pid, start in (line.split() for line in lines)
    ]


def dev_log(workspace: Path) -> str:
    log = workspace / ".accelerator/tmp/dev/dev.log"
    return log.read_text() if log.exists() else ""


def assert_no_timeout_branch_output(workspace: Path) -> None:
    logged = dev_log(workspace)
    found = [line for line in TIMEOUT_BRANCH_EVIDENCE if line in logged]
    assert not found, f"timeout-branch output in dev.log: {found}"


def stop_cleanly(workspace: Path) -> None:
    assert_ok(run_driver(workspace, "stop"))
    assert_stopped_cleanly(workspace)


def assert_stopped_cleanly(workspace: Path) -> None:
    fakes = recorded_fakes(workspace)
    ours = escalations(workspace)
    sigkilled = [e for e in ours if e.signal == "kill"]
    assert not sigkilled, f"our code SIGKILLed: {sigkilled}"
    fake_pids = {fake.pid for fake in fakes}
    non_fakes = [
        e for e in ours if e.signal == "terminate" and e.pid not in fake_pids
    ]
    assert not non_fakes, f"our code SIGTERMed a non-fake: {non_fakes}"
    survivors = [fake for fake in fakes if fake.is_running()]
    assert not survivors, f"survived the stop: {survivors}"
    killed = killed_by_test(workspace)
    unsignalled = [
        fake
        for fake in fakes
        if not fake.is_one_of(killed) and "sigterm" not in fake.events
    ]
    assert not unsignalled, f"stopped without SIGTERM: {unsignalled}"
    assert_no_timeout_branch_output(workspace)


def _surviving_arbiter(workspace: Path) -> int | None:
    processes = PsutilProcessOps()
    state = read_state(workspace)
    if state and state.get("arbiter_pid") and state.get("arbiter_start_time"):
        pid = state["arbiter_pid"]
        if processes.identity_matches(pid, state["arbiter_start_time"]):
            return pid
    pidfile = workspace / ".accelerator/tmp/dev/circusd.pid"
    text = pidfile.read_text().strip() if pidfile.exists() else ""
    if not text.isdigit():
        return None
    try:
        command = psutil.Process(int(text)).cmdline()
    except psutil.Error:
        return None
    return int(text) if any("circusd" in part for part in command) else None


def _reap_survivors(workspace: Path) -> None:
    """SIGKILL every surviving fake and arbiter, then remove the ipc sockets.

    Never stops through the driver: this runs after the test has made its own
    claims about the stop, and only has to make sure nothing leaks.
    """
    for fake in recorded_fakes(workspace):
        if fake.is_running():
            with contextlib.suppress(psutil.Error):
                psutil.Process(fake.pid).kill()
    arbiter = _surviving_arbiter(workspace)
    if arbiter is not None:
        with contextlib.suppress(psutil.Error):
            psutil.Process(arbiter).kill()
    shutil.rmtree(ipc_socket_paths(workspace)[0].parent, ignore_errors=True)


def _hard_cleanup(workspace: Path) -> None:
    stack_discoverable = (
        read_state(workspace) is not None
        or ipc_socket_paths(workspace)[0].exists()
    )
    if stack_discoverable:
        with contextlib.suppress(Exception):
            run_driver(workspace, "stop", timeout=30)
    _reap_survivors(workspace)


@pytest.fixture
def workspace(tmp_path):
    ws = tmp_path / "ws"
    ws.mkdir()
    try:
        yield ws
        if read_state(ws) is not None:
            stop_cleanly(ws)
    finally:
        _reap_survivors(ws)


@pytest.fixture
def unverified_stop_workspace(tmp_path):
    ws = tmp_path / "ws"
    ws.mkdir()
    yield ws
    _hard_cleanup(ws)


# ─── scenarios ───────────────────────────────────────────────


def test_detach_readiness_and_log_routing(workspace):
    proc = run_driver(workspace, "up")
    assert_ok(proc)
    state = read_state(workspace)
    assert (
        state["arbiter_pid"] and state["server_pid"] and state["frontend_pid"]
    )
    assert state["frontend_port"]
    assert state["endpoint"].startswith("ipc://")
    # arbiter survives the (now-exited) launching process
    assert alive(state["arbiter_pid"])
    dev = workspace / ".accelerator/tmp/dev"
    # The Model-1 server writes its own log under the composed state dir; the
    # frontend's stdout is captured by circus to dev/frontend.log.
    server_log_path = workspace / ".accelerator/tmp/visualiser/server.log"
    frontend_log_path = dev / "frontend.log"
    # each marker reaches its own log (AC3) — wait for the async capture to
    # flush
    assert wait_for_text(server_log_path, "SERVER_UP")
    assert wait_for_text(frontend_log_path, "FRONTEND_UP")
    # and crucially each marker appears ONLY in its own log (no cross-wiring)
    assert "FRONTEND_UP" not in server_log_path.read_text()
    assert "SERVER_UP" not in frontend_log_path.read_text()


def test_reuse_starts_no_duplicate(workspace):
    assert_ok(run_driver(workspace, "up"))
    first = read_state(workspace)["arbiter_pid"]
    proc = run_driver(workspace, "up")
    assert_ok(proc)
    assert driver_payload(proc)["kind"] == "reused"
    assert read_state(workspace)["arbiter_pid"] == first


def test_stale_info_does_not_satisfy_gate(workspace):
    state_dir = workspace / ".accelerator/tmp/visualiser"
    state_dir.mkdir(parents=True)
    (state_dir / "server-info.json").write_text(
        json.dumps({"url": "http://127.0.0.1:1111", "port": 1111})
    )
    assert_ok(run_driver(workspace, "up", {"api_port": 8888}))
    info = json.loads((state_dir / "server-info.json").read_text())
    assert info["port"] == 8888  # the fresh server's file, not the stale 1111


def test_clean_teardown_leaves_no_orphans(workspace):
    assert_ok(run_driver(workspace, "up"))
    state = read_state(workspace)
    tracked = [
        state["server_pid"],
        state["frontend_pid"],
        state["arbiter_pid"],
        *descendants(state["server_pid"]),
        *descendants(state["frontend_pid"]),
    ]
    stop_cleanly(workspace)
    for pid in tracked:
        assert not alive(pid)
    assert read_state(workspace) is None


def test_orphan_reach_when_arbiter_already_dead(workspace):
    assert_ok(run_driver(workspace, "up"))
    state = read_state(workspace)
    server_pid, frontend_pid = state["server_pid"], state["frontend_pid"]
    os.kill(state["arbiter_pid"], signal.SIGKILL)  # children reparent to init
    assert wait_until(lambda: not alive(state["arbiter_pid"]))
    assert alive(
        server_pid
    )  # still alive, no longer a child of the dead arbiter
    stop_cleanly(workspace)
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
            str(FAKE_PROCESS),
            "server",
            "--spawn-child",
            "--events-log",
            str(workspace / EVENTS_LOG),
        ],
        cwd=workspace,
    )
    try:
        assert wait_until(
            lambda: (
                {"server", "server-child"}
                <= {fake.role for fake in recorded_fakes(workspace)}
            )
        )
        child_pids = descendants(server.pid)
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
        stop_cleanly(workspace)
        assert not alive(
            server.pid
        )  # server subtree reaped via recorded server_pid
        for pid in child_pids:
            assert not alive(pid)
    finally:
        with contextlib.suppress(Exception):
            server.kill()
        server.wait()


def test_restart_round_trip(workspace):
    assert_ok(run_driver(workspace, "up"))
    first = read_state(workspace)["arbiter_pid"]
    proc = run_driver(workspace, "restart")
    assert_ok(proc)
    state = read_state(workspace)
    assert state["arbiter_pid"] != first  # a genuinely new arbiter
    assert alive(state["arbiter_pid"])
    assert state["frontend_port"]


def test_readiness_timeout_fails_and_tears_down(unverified_stop_workspace):
    workspace = unverified_stop_workspace
    proc = run_driver(
        workspace, "up", {"server_write_info": False, "readiness_timeout": 2.0}
    )
    assert proc.returncode == 1
    server_info = workspace / ".accelerator/tmp/visualiser/server-info.json"
    assert f"server did not write {server_info} in time" in (
        driver_payload(proc)["message"] or ""
    )
    assert READINESS_TIMEOUT_EVIDENCE in dev_log(workspace)
    assert PIDFILE_TIMEOUT_EVIDENCE not in dev_log(workspace)
    with pytest.raises(AssertionError, match="timeout-branch output"):
        assert_no_timeout_branch_output(workspace)
    assert read_state(workspace) is None  # torn down


def test_daemon_startup_failure_reaps_handle_no_retry(
    unverified_stop_workspace,
):
    workspace = unverified_stop_workspace
    proc = run_driver(
        workspace, "up", {"fake_circusd_fail": True, "pidfile_timeout": 2.0}
    )
    assert proc.returncode == 1
    assert "the circus daemon did not start (no pidfile at" in (
        driver_payload(proc)["message"] or ""
    )
    assert PIDFILE_TIMEOUT_EVIDENCE in dev_log(workspace)
    assert READINESS_TIMEOUT_EVIDENCE not in dev_log(workspace)
    with pytest.raises(AssertionError, match="timeout-branch output"):
        assert_no_timeout_branch_output(workspace)


def test_discovery_survives_lost_state_file(workspace):
    # Without recorded watcher PIDs nothing reaches a watcher's children:
    # circus lists them only after the watcher has already exited.
    childless_watchers = {"server_spawn_child": False, "fe_spawn_child": False}
    assert_ok(run_driver(workspace, "up", childless_watchers))
    arbiter = read_state(workspace)["arbiter_pid"]
    (workspace / ".accelerator/tmp/dev/dev.json").unlink()  # lose the cache
    stop_cleanly(workspace)
    assert not alive(arbiter)  # found via recomputed deterministic ipc paths


def test_stale_cleanup_after_out_of_band_kill(workspace):
    assert_ok(run_driver(workspace, "up"))
    state = read_state(workspace)
    for key in ("arbiter_pid", "server_pid", "frontend_pid"):
        kill_out_of_band(workspace, state[key])
    assert wait_until(
        lambda: not any(fake.is_running() for fake in recorded_fakes(workspace))
    )
    assert (
        run_driver(workspace, "status").returncode == 4
    )  # treated as not-running
    stop_cleanly(workspace)
    endpoint_sock = ipc_socket_paths(workspace)[0]
    assert not endpoint_sock.exists()  # stale socket cleaned up


def test_recycled_pid_is_refused_real_psutil(unverified_stop_workspace):
    workspace = unverified_stop_workspace
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
            # bogus -> identity mismatch vs the live victim
            "arbiter_start_time": 1.0,
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
    assert_ok(proc)
    # frontend started despite a stripped PATH -> the absolute npm render worked
    assert read_state(workspace)["frontend_pid"]


def test_frontend_port_and_info_wiring(workspace):
    assert_ok(run_driver(workspace, "up", {"free_port": 45678}))
    ini = (workspace / ".accelerator/tmp/dev/circus.ini").read_text()
    assert "--port 45678 --strictPort" in ini
    assert "VISUALISER_INFO_PATH" in ini
    assert read_state(workspace)["frontend_port"] == 45678


def test_status_exit_codes(workspace):
    assert run_driver(workspace, "status").returncode == 4  # neither
    assert_ok(run_driver(workspace, "up"))
    assert_ok(run_driver(workspace, "status"))  # both
    state = read_state(workspace)
    kill_out_of_band(
        workspace, state["frontend_pid"]
    )  # respawn=false -> stays stopped
    # Poll rather than a fixed sleep: SIGKILL leaves a zombie until circus (the
    # parent) reaps it, and under parallel-suite load that reap can outlast any
    # fixed wait, so status momentarily still reports both up. respawn=false
    # means once it reads "one" it stays there, so this converges without flap.
    assert wait_until(
        lambda: run_driver(workspace, "status").returncode == 3  # one
    )


def test_sigterm_ignoring_frontend_is_sigkilled(unverified_stop_workspace):
    workspace = unverified_stop_workspace
    assert_ok(run_driver(workspace, "up", {"fe_ignore_sigterm": True}))
    frontend_pid = read_state(workspace)["frontend_pid"]
    assert alive(frontend_pid)
    assert_ok(run_driver(workspace, "stop", {"grace_quit": 8.0}))
    # circus escalated to SIGKILL past its 2 s graceful_timeout
    assert not alive(frontend_pid)
    (frontend,) = [
        fake for fake in recorded_fakes(workspace) if fake.pid == frontend_pid
    ]
    assert frontend.events == ("started",)
    assert frontend_pid not in {e.pid for e in escalations(workspace)}
    with pytest.raises(AssertionError, match="stopped without SIGTERM"):
        assert_stopped_cleanly(workspace)


def test_cross_workspace_concurrency(tmp_path):
    ws_a, ws_b = tmp_path / "a", tmp_path / "b"
    ws_a.mkdir()
    ws_b.mkdir()
    try:
        assert_ok(run_driver(ws_a, "up"))
        assert_ok(run_driver(ws_b, "up"))
        a, b = read_state(ws_a), read_state(ws_b)
        assert a["frontend_port"] != b["frontend_port"]
        assert (
            a["endpoint"] != b["endpoint"]
        )  # distinct per-workspace ipc sockets
        assert alive(a["arbiter_pid"]) and alive(b["arbiter_pid"])
        # stopping A leaves B untouched (structural isolation)
        stop_cleanly(ws_a)
        assert not alive(a["arbiter_pid"])
        assert alive(b["arbiter_pid"])
        stop_cleanly(ws_b)
    finally:
        _reap_survivors(ws_a)
        _reap_survivors(ws_b)


# ─── harness self-tests ──────────────────────────────────────


def test_the_clean_stop_check_sees_our_forced_kill(unverified_stop_workspace):
    workspace = unverified_stop_workspace
    assert_ok(run_driver(workspace, "up", {"server_ignore_sigterm": True}))
    state = read_state(workspace)
    server_pid, arbiter_pid = state["server_pid"], state["arbiter_pid"]
    os.kill(arbiter_pid, signal.SIGKILL)
    assert wait_until(lambda: not alive(arbiter_pid))

    assert_ok(run_driver(workspace, "stop", {"grace_kill": 0.5}))

    ours = escalations(workspace)
    assert Escalation("terminate", server_pid) in ours
    assert Escalation("kill", server_pid) in ours
    with pytest.raises(AssertionError, match="our code SIGKILLed"):
        assert_stopped_cleanly(workspace)


def test_the_clean_stop_check_sees_a_sigterm_of_a_non_fake(tmp_path):
    unrecorded_pid = 4242
    dead_fake = f"server {2**31 - 1} 1.0"
    (tmp_path / EVENTS_LOG).write_text(
        f"started {dead_fake}\nsigterm {dead_fake}\n"
    )
    (tmp_path / ESCALATIONS_LOG).write_text(f"terminate {unrecorded_pid}\n")

    with pytest.raises(AssertionError, match="our code SIGTERMed a non-fake"):
        assert_stopped_cleanly(tmp_path)


def test_the_clean_stop_check_sees_a_survivor(tmp_path):
    (tmp_path / ESCALATIONS_LOG).touch()
    child = subprocess.Popen(
        [
            sys.executable,
            str(FAKE_PROCESS),
            "child",
            "--events-log",
            str(tmp_path / EVENTS_LOG),
        ]
    )
    try:
        assert wait_until(
            lambda: (
                [fake.pid for fake in recorded_fakes(tmp_path)] == [child.pid]
            )
        )
        with pytest.raises(AssertionError, match="survived the stop"):
            assert_stopped_cleanly(tmp_path)
    finally:
        child.kill()
        child.wait()


def _logged_strings(source: str) -> list[str]:
    tree = ast.parse(source)
    docstrings = {
        id(node.body[0].value)
        for node in ast.walk(tree)
        if isinstance(
            node,
            ast.Module | ast.ClassDef | ast.FunctionDef | ast.AsyncFunctionDef,
        )
        and ast.get_docstring(node, clean=False) is not None
    }
    return [
        node.value
        for node in ast.walk(tree)
        if isinstance(node, ast.Constant)
        and isinstance(node.value, str)
        and id(node) not in docstrings
    ]


def test_every_timeout_branch_evidence_line_is_still_logged():
    logged = _logged_strings(LIFECYCLE.read_text())
    missing = [
        evidence
        for evidence in TIMEOUT_BRANCH_EVIDENCE
        if not any(evidence in string for string in logged)
    ]
    assert not missing, f"no longer in {LIFECYCLE.name}: {missing}"
