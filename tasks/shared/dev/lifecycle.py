"""Lifecycle orchestration for the unified dev task.

Dependency wiring (``DevDeps``), result types, and the up/stop/restart/status
orchestrators. This is the imperative coordinator that composes the dev-specific
modules (state, circus, endpoints, health) with the reusable shared helpers
(processes, locking, ports, polling, clock).
"""

import contextlib
import dataclasses
import datetime
import json
import sys
from collections.abc import Callable
from pathlib import Path

from tasks.shared.clock import Clock
from tasks.shared.dev.circus import (
    ArbiterSpec,
    LaunchHandle,
    Supervisor,
    SupervisorUnreachable,
    render_circus_ini,
)
from tasks.shared.dev.endpoints import ipc_socket_paths
from tasks.shared.dev.health import Health, evaluate_health
from tasks.shared.dev.state import DevState, read_dev_state, write_dev_state
from tasks.shared.locking import workspace_lock
from tasks.shared.polling import wait_for_file
from tasks.shared.ports import free_port
from tasks.shared.processes import ProcessOps


def _maybe_close(sup: Supervisor) -> None:
    close = getattr(sup, "close", None)
    if callable(close):
        with contextlib.suppress(Exception):
            close()


# ─── injected dependencies + result types ────────────────────


@dataclasses.dataclass(frozen=True)
class DevDeps:
    """Injected collaborators + resolved paths for the orchestrators.

    Default-constructed by the ``@task`` adapters to the real circus / subprocess
    / psutil / time wiring; the unit tests build it with fakes. Bundling the
    paths here means ``bring_up`` never reaches into ``tasks/dev.py`` constants,
    so the render-then-launch ordering and directory side-effects live in one
    place.
    """

    client_factory: Callable[..., Supervisor]
    launcher: Callable[..., LaunchHandle]
    killer: ProcessOps
    clock: Clock
    config_renderer: Callable[[], Path]
    workspace_root: Path
    state_path: Path
    lock_path: Path
    dev_dir: Path
    pidfile: Path
    ini_path: Path
    server_info_path: Path
    server_pidfile: Path
    server_bin: Path
    frontend: Path
    diagnostic_log: Path
    env: dict[str, str] = dataclasses.field(default_factory=dict)
    npm_bin: str = "npm"
    node_bin: str = "node"
    free_port: Callable[[], int] = free_port
    probe_timeout: float = 2.0
    pidfile_timeout: float = 10.0
    readiness_timeout: float = 30.0
    frontend_active_timeout: float = 15.0
    grace_quit: float = 5.0  # covers circus's own 2 s graceful_timeout + reaping
    grace_kill: float = 2.0  # our SIGTERM grace before SIGKILL


@dataclasses.dataclass
class UpResult:
    kind: str  # "started" | "reused" | "failed"
    frontend_url: str | None = None
    api_url: str | None = None
    api_port: int | None = None
    dev_dir: str | None = None
    message: str | None = None
    artifact: str | None = None


@dataclasses.dataclass
class StopResult:
    kind: str  # "clean" | "refused" | "survivor"
    pid: int | None = None
    message: str | None = None


@dataclasses.dataclass
class ReuseOutcome:
    action: str  # "reuse" | "proceed" | "abort"
    state: DevState | None = None
    stop_result: StopResult | None = None


@dataclasses.dataclass
class LaunchedArbiter:
    handle: LaunchHandle
    pid: int
    start_time: float
    state: DevState
    frontend_port: int
    frontend_url: str
    endpoint: str


class _UpAbort(Exception):
    def __init__(self, result: UpResult):
        self.result = result


# ─── diagnostics ─────────────────────────────────────────────


def log_diagnostic(deps: DevDeps, message: str) -> None:
    """Append a timestamped ``dev:`` line to dev.log and echo it to stderr."""
    stamp = datetime.datetime.now().isoformat(timespec="seconds")
    line = f"{stamp} dev: {message}"
    with contextlib.suppress(OSError):
        Path(deps.diagnostic_log).parent.mkdir(parents=True, exist_ok=True)
        with open(deps.diagnostic_log, "a") as handle:
            handle.write(line + "\n")
    print(line, file=sys.stderr)


def _truncate_diagnostic_log(deps: DevDeps) -> None:
    with contextlib.suppress(OSError):
        Path(deps.diagnostic_log).parent.mkdir(parents=True, exist_ok=True)
        Path(deps.diagnostic_log).write_text("")


def _read_server_info(path: Path | str) -> dict | None:
    try:
        data = json.loads(Path(path).read_text())
    except (OSError, ValueError):
        return None
    return data if isinstance(data, dict) else None


# ─── reuse / probe ───────────────────────────────────────────


def _probe_status(state: DevState, deps: DevDeps, *, timeout: float) -> dict | None:
    sup = deps.client_factory(state.endpoint, timeout=timeout)
    try:
        return sup.status()
    except SupervisorUnreachable:
        return None
    finally:
        _maybe_close(sup)


def _arbiter_identity_ok(state: DevState, deps: DevDeps) -> bool:
    if state.arbiter_pid is None or state.arbiter_start_time is None:
        return False
    return deps.killer.is_alive(state.arbiter_pid) and deps.killer.identity_matches(
        state.arbiter_pid, state.arbiter_start_time
    )


def reuse_or_teardown(state: DevState | None, deps: DevDeps) -> ReuseOutcome:
    """Lock-gated gate: reuse a healthy session, else tear the stale one down.

    Because the endpoint is a per-workspace ``ipc://`` socket, a reachable probe
    can only be *this* workspace's arbiter; the recorded identity is confirmed as
    belt-and-braces before reuse. A degraded/stale session is torn down; if
    teardown can't confirm death (survivor/refused) the caller fails fast rather
    than launching a competitor.
    """
    if state is None:
        return ReuseOutcome("proceed")
    statuses = _probe_status(state, deps, timeout=deps.probe_timeout)
    if statuses is not None and evaluate_health(statuses) == Health.HEALTHY and (
        _arbiter_identity_ok(state, deps)
    ):
        return ReuseOutcome("reuse", state=state)
    result = teardown(state, deps, lock_held=True)
    if result.kind in ("survivor", "refused"):
        return ReuseOutcome("abort", stop_result=result)
    return ReuseOutcome("proceed")


# ─── bring up ────────────────────────────────────────────────


def bring_up(deps: DevDeps) -> UpResult:
    """Start (or reuse) the supervised dev stack. The orchestrator entry point."""
    with workspace_lock(deps.lock_path) as locked:
        if not locked:
            # Another `dev` is mid-launch in this workspace: re-probe and reuse
            # if it is now healthy, else fail fast (never race into a duplicate).
            state = read_dev_state(deps.state_path)
            if state is not None:
                statuses = _probe_status(state, deps, timeout=deps.probe_timeout)
                if (
                    statuses is not None
                    and evaluate_health(statuses) == Health.HEALTHY
                    and _arbiter_identity_ok(state, deps)
                ):
                    return _reused_result(state, deps)
            return UpResult(
                "failed",
                message=(
                    "another `dev` is starting in this workspace; retry in a "
                    "moment, or run `mise run dev:status` / `mise run dev:stop` "
                    "if this persists"
                ),
            )
        return _bring_up_locked(deps)


def _bring_up_locked(deps: DevDeps) -> UpResult:
    _truncate_diagnostic_log(deps)
    state = read_dev_state(deps.state_path)
    outcome = reuse_or_teardown(state, deps)
    if outcome.action == "reuse":
        return _reused_result(outcome.state, deps)
    if outcome.action == "abort":
        sr = outcome.stop_result
        return UpResult(
            "failed",
            message=sr.message
            or f"existing dev stack could not be torn down ({sr.kind})",
            artifact=str(deps.diagnostic_log),
        )

    # Proceed: clean slate so the readiness gate can only see the new server.
    Path(deps.dev_dir).mkdir(parents=True, exist_ok=True)
    Path(deps.server_info_path).unlink(missing_ok=True)
    Path(deps.server_pidfile).unlink(missing_ok=True)

    try:
        launched = allocate_and_launch(deps)
        _readiness_gate(launched, deps)
        start_frontend(launched, deps)
    except _UpAbort as abort:
        return abort.result
    return _started_result(launched, deps)


def allocate_and_launch(deps: DevDeps) -> LaunchedArbiter:
    """Allocate ports/sockets, write provisional state, launch + confirm circusd."""
    endpoint_sock, pubsub_sock = ipc_socket_paths(deps.workspace_root)
    endpoint_sock.parent.mkdir(parents=True, exist_ok=True)
    fe_port = deps.free_port()
    fe_url = f"http://127.0.0.1:{fe_port}"

    config_path = deps.config_renderer()  # render-then-launch ordering, in one place

    spec = ArbiterSpec(
        endpoint_socket=str(endpoint_sock),
        pubsub_socket=str(pubsub_sock),
        pidfile=str(deps.pidfile),
        dev_dir=str(deps.dev_dir),
        server_bin=str(deps.server_bin),
        config_path=str(config_path),
        npm_bin=deps.npm_bin,
        frontend=str(deps.frontend),
        frontend_port=fe_port,
        server_info_path=str(deps.server_info_path),
    )
    Path(deps.ini_path).write_text(render_circus_ini(spec))

    endpoint = f"ipc://{endpoint_sock}"
    state = DevState(
        endpoint=endpoint,
        pubsub_endpoint=f"ipc://{pubsub_sock}",
        frontend_port=fe_port,
        frontend_url=fe_url,
        pidfile=str(deps.pidfile),
        ini_path=str(deps.ini_path),
        npm_bin=deps.npm_bin,
        node_bin=deps.node_bin,
    )
    write_dev_state(deps.state_path, state)  # provisional: PIDs null, discoverable

    handle = deps.launcher(
        ["circusd", "--pidfile", str(deps.pidfile), str(deps.ini_path)],
        env=deps.env,
        cwd=str(deps.workspace_root),
    )

    pid = _poll_arbiter_pid(deps)
    if pid is None:
        # ipc:// endpoints have no port to collide on, so a non-appearing/empty
        # pidfile is a genuine failure (bad INI, missing binary), not a transient
        # race: reap the launch handle (the real arbiter) and fail without retry.
        _reap_handle(handle, deps)
        log_diagnostic(deps, f"circusd did not write a valid pidfile at {deps.pidfile}")
        raise _UpAbort(
            UpResult(
                "failed",
                message=(
                    f"the circus daemon did not start (no pidfile at "
                    f"{deps.pidfile}); see {deps.dev_dir}/circusd.log"
                ),
                artifact=f"{deps.dev_dir}/circusd.log",
            )
        )

    start = deps.killer.create_time(pid)
    if start is None or not deps.killer.is_alive(pid):
        _reap_handle(handle, deps)
        log_diagnostic(deps, f"circusd pidfile PID {pid} was not live")
        raise _UpAbort(
            UpResult(
                "failed",
                message=f"the circus daemon exited immediately; see {deps.dev_dir}/circusd.log",
                artifact=f"{deps.dev_dir}/circusd.log",
            )
        )

    state.arbiter_pid = pid
    state.arbiter_start_time = start
    write_dev_state(deps.state_path, state)
    return LaunchedArbiter(
        handle=handle,
        pid=pid,
        start_time=start,
        state=state,
        frontend_port=fe_port,
        frontend_url=fe_url,
        endpoint=endpoint,
    )


def _poll_arbiter_pid(deps: DevDeps) -> int | None:
    """Bounded poll for a valid integer pidfile (empty/partial → keep polling)."""
    pidfile = Path(deps.pidfile)
    deadline = deps.clock.now() + deps.pidfile_timeout
    while True:
        if pidfile.exists():
            text = pidfile.read_text().strip()
            if text.isdigit():
                return int(text)
        if deps.clock.now() >= deadline:
            return None
        deps.clock.sleep(min(0.1, deadline - deps.clock.now()))


def _reap_handle(handle: LaunchHandle, deps: DevDeps) -> None:
    descendants = deps.killer.children(handle.pid)
    deps.killer.kill(handle.pid)
    for cpid, _ in descendants:
        deps.killer.kill(cpid)


def _readiness_gate(launched: LaunchedArbiter, deps: DevDeps) -> None:
    if not wait_for_file(
        deps.server_info_path,
        timeout=deps.readiness_timeout,
        sleep=deps.clock.sleep,
        now=deps.clock.now,
    ):
        teardown(launched.state, deps, lock_held=True)
        log_diagnostic(deps, "server-info.json did not appear before the readiness timeout")
        raise _UpAbort(
            UpResult(
                "failed",
                message=(
                    f"server did not write {deps.server_info_path} in time; see "
                    f"{deps.dev_dir}/server.log"
                ),
                artifact=f"{deps.dev_dir}/server.log",
            )
        )
    _record_watcher_pid(launched, deps, "server")


def start_frontend(launched: LaunchedArbiter, deps: DevDeps) -> None:
    sup = deps.client_factory(launched.endpoint, timeout=deps.probe_timeout)
    try:
        try:
            sup.start("frontend")
        except SupervisorUnreachable as exc:
            teardown(launched.state, deps, lock_held=True)
            raise _UpAbort(
                UpResult(
                    "failed",
                    message=f"could not start the frontend watcher: {exc}; see {deps.dev_dir}/frontend.log",
                    artifact=f"{deps.dev_dir}/frontend.log",
                )
            )
        # send "start" only confirms acceptance; with respawn=false a frontend
        # that starts then dies goes active->stopped silently. Require two
        # consecutive "active" polls so a flapping watcher is treated as failed.
        deadline = deps.clock.now() + deps.frontend_active_timeout
        consecutive_active = 0
        while deps.clock.now() < deadline:
            statuses = _safe_status(sup)
            if statuses.get("frontend") == "active":
                consecutive_active += 1
                if consecutive_active >= 2:
                    break
            else:
                consecutive_active = 0
            deps.clock.sleep(min(0.1, deadline - deps.clock.now()))
        if consecutive_active < 2:
            teardown(launched.state, deps, lock_held=True)
            log_diagnostic(deps, "frontend watcher did not stay active")
            raise _UpAbort(
                UpResult(
                    "failed",
                    message=f"the frontend watcher did not become active; see {deps.dev_dir}/frontend.log",
                    artifact=f"{deps.dev_dir}/frontend.log",
                )
            )
    finally:
        _maybe_close(sup)
    _record_watcher_pid(launched, deps, "frontend")


def _safe_status(sup: Supervisor) -> dict[str, str]:
    try:
        return sup.status()
    except SupervisorUnreachable:
        return {}


def _record_watcher_pid(launched: LaunchedArbiter, deps: DevDeps, name: str) -> None:
    """Record a watcher PID + start-time into dev-state the instant it is active.

    Incremental (not batched) so there is no launch-window gap where a watcher is
    live but unrecorded — teardown can then reach reparented orphans by identity
    even if the arbiter dies.
    """
    sup = deps.client_factory(launched.endpoint, timeout=deps.probe_timeout)
    try:
        pids = sup.pids(name)
    except SupervisorUnreachable:
        pids = []
    finally:
        _maybe_close(sup)
    if not pids:
        return
    pid = pids[0]
    start = deps.killer.create_time(pid)
    if start is None:
        return
    if name == "server":
        launched.state.server_pid = pid
        launched.state.server_start_time = start
    else:
        launched.state.frontend_pid = pid
        launched.state.frontend_start_time = start
    write_dev_state(deps.state_path, launched.state)


def _reused_result(state: DevState, deps: DevDeps) -> UpResult:
    info = _read_server_info(deps.server_info_path) or {}
    return UpResult(
        "reused",
        frontend_url=state.frontend_url,
        api_url=info.get("url"),
        api_port=info.get("port"),
        dev_dir=str(deps.dev_dir),
    )


def _started_result(launched: LaunchedArbiter, deps: DevDeps) -> UpResult:
    info = _read_server_info(deps.server_info_path) or {}
    return UpResult(
        "started",
        frontend_url=launched.frontend_url,
        api_url=info.get("url"),
        api_port=info.get("port"),
        dev_dir=str(deps.dev_dir),
    )


# ─── teardown ────────────────────────────────────────────────


@dataclasses.dataclass
class TargetSet:
    refused: bool
    arbiter_alive: bool
    descendants: list[tuple[int, float]]  # pre-quit snapshot (recycled-PID baseline)
    # recorded (pid, start_time) for server/frontend — the orphan-reach baseline,
    # gated against the *recorded* start so a recycled recorded PID is skipped.
    watcher_identities: list[tuple[int, float]]


def snapshot_targets(state: DevState, deps: DevDeps) -> TargetSet:
    """Establish the safe target set *before* anything is signalled."""
    refused = False
    arbiter_alive = False
    descendants: list[tuple[int, float]] = []
    pid = state.arbiter_pid
    if pid is not None and deps.killer.is_alive(pid):
        if state.arbiter_start_time is not None and not deps.killer.identity_matches(
            pid, state.arbiter_start_time
        ):
            refused = True  # recorded PID is now an unrelated process
        else:
            arbiter_alive = True
            descendants = deps.killer.children(pid)
    watcher_identities: list[tuple[int, float]] = []
    if state.server_pid is not None and state.server_start_time is not None:
        watcher_identities.append((state.server_pid, state.server_start_time))
    if state.frontend_pid is not None and state.frontend_start_time is not None:
        watcher_identities.append((state.frontend_pid, state.frontend_start_time))
    return TargetSet(refused, arbiter_alive, descendants, watcher_identities)


def teardown(state: DevState | None, deps: DevDeps, *, lock_held: bool) -> StopResult:
    """Tear the stack down. Returns clean / refused / survivor.

    ``lock_held`` is True when called from ``bring_up`` (which owns the lock) so
    stale-socket removal proceeds; from the lock-free ``do_stop``/``do_status``
    it is False and socket removal opportunistically takes the lock.
    """
    if state is None:
        return StopResult("clean", message="Dev stack not running.")
    targets = snapshot_targets(state, deps)
    if targets.refused:
        result = StopResult(
            "refused",
            pid=state.arbiter_pid,
            message=(
                f"arbiter PID {state.arbiter_pid} did not match the recorded "
                f"identity; not killing — investigate {deps.diagnostic_log} and "
                f"re-run `mise run dev:stop`"
            ),
        )
        log_diagnostic(deps, result.message)
        return result  # keep dev-state + sockets + pidfile

    result = kill_arbiter(state, targets, deps)
    if result.kind == "survivor":
        remove_artifacts(state, deps, result, lock_held=lock_held)  # keeps sockets
        return result
    reap_descendants(state, targets, deps)
    remove_artifacts(state, deps, result, lock_held=lock_held)
    return result


def kill_arbiter(state: DevState, targets: TargetSet, deps: DevDeps) -> StopResult:
    """Quit the arbiter, confirm death, escalate to a gated direct kill if needed."""
    statuses = _probe_status(state, deps, timeout=deps.probe_timeout)
    reachable = statuses is not None
    if reachable:
        sup = deps.client_factory(state.endpoint, timeout=deps.probe_timeout)
        try:
            sup.quit()  # circus: SIGTERM -> 2 s graceful_timeout -> SIGKILL w/ stop_children
        except SupervisorUnreachable as exc:
            log_diagnostic(deps, f"unexpected error quitting arbiter: {exc}")
        finally:
            _maybe_close(sup)

    if state.arbiter_pid is None:
        # Provisional/interrupted launch: endpoint is the liveness authority.
        if not reachable:
            return StopResult("clean")
        if _wait_until_unreachable(state, deps, deps.grace_quit):
            return StopResult("clean")
        msg = "arbiter (pid unknown) endpoint still reachable after quit"
        log_diagnostic(deps, msg)
        return StopResult("survivor", pid=None, message=msg)

    # Confirm death, generous enough to cover circus's own 2 s graceful teardown.
    if _wait_until_dead(state.arbiter_pid, deps, deps.grace_quit):
        return StopResult("clean")
    deps.killer.terminate(state.arbiter_pid)
    if _wait_until_dead(state.arbiter_pid, deps, deps.grace_kill):
        return StopResult("clean")
    deps.killer.kill(state.arbiter_pid)
    if _wait_until_dead(state.arbiter_pid, deps, deps.grace_kill):
        return StopResult("clean")
    msg = (
        f"arbiter {state.arbiter_pid} still alive after SIGKILL; left dev-state + "
        f"sockets in place — see {deps.diagnostic_log}, then re-run `mise run dev:stop`"
    )
    log_diagnostic(deps, msg)
    return StopResult("survivor", pid=state.arbiter_pid, message=msg)


def reap_descendants(state: DevState, targets: TargetSet, deps: DevDeps) -> None:
    """Reap surviving descendants, identity-gated, re-enumerated post-grace.

    Combines the pre-quit snapshot (catches the in-tree descendants) with a fresh
    walk of the recorded ``server_pid``/``frontend_pid`` (catches workers spawned
    lazily during teardown and orphans reparented away from a dead arbiter).
    """
    baseline: dict[int, float] = {pid: ct for pid, ct in targets.descendants}
    for wpid, wstart in targets.watcher_identities:
        # Gate the recorded watcher PID against its *recorded* start-time, so a
        # recorded PID that has since been recycled to an unrelated process is
        # skipped (and its "children" — not ours — are never enumerated).
        if not deps.killer.identity_matches(wpid, wstart):
            continue
        baseline.setdefault(wpid, wstart)
        for cpid, cct in deps.killer.children(wpid):
            baseline.setdefault(cpid, cct)
    # SIGTERM the ones whose identity still matches (skip recycled PIDs).
    for pid, ct in baseline.items():
        if deps.killer.identity_matches(pid, ct):
            deps.killer.terminate(pid)
    _sleep(deps, deps.grace_kill)
    for pid, ct in baseline.items():
        if deps.killer.identity_matches(pid, ct):
            deps.killer.kill(pid)


def remove_artifacts(
    state: DevState, deps: DevDeps, result: StopResult, *, lock_held: bool
) -> None:
    """Remove artifacts only on confirmed death; never sever a live channel.

    On survivor/refused the sockets + pidfile + dev-state are kept so a later
    ``dev:stop`` can still ``quit`` through the endpoint.
    """
    if result.kind != "clean":
        return
    Path(deps.state_path).unlink(missing_ok=True)
    Path(deps.ini_path).unlink(missing_ok=True)
    Path(deps.pidfile).unlink(missing_ok=True)
    _unlink_sockets(state, deps, lock_held=lock_held)


def _unlink_sockets(state: DevState, deps: DevDeps, *, lock_held: bool) -> None:
    def _do_unlink() -> None:
        for endpoint in (state.endpoint, state.pubsub_endpoint):
            path = endpoint[len("ipc://") :] if endpoint.startswith("ipc://") else endpoint
            if path:
                Path(path).unlink(missing_ok=True)
        with contextlib.suppress(OSError):
            ipc_socket_paths(deps.workspace_root)[0].parent.rmdir()

    if lock_held:
        _do_unlink()  # we already own the lock (called from bring_up)
        return
    # Lock-free path: opportunistically take the lock so a stale teardown never
    # removes a socket a just-launched arbiter is binding. Skip if we can't.
    with workspace_lock(deps.lock_path) as locked:
        if locked:
            _do_unlink()


def _wait_until_dead(pid: int, deps: DevDeps, timeout: float) -> bool:
    deadline = deps.clock.now() + timeout
    while True:
        if not deps.killer.is_alive(pid):
            return True
        if deps.clock.now() >= deadline:
            return False
        deps.clock.sleep(min(0.1, deadline - deps.clock.now()))


def _wait_until_unreachable(state: DevState, deps: DevDeps, timeout: float) -> bool:
    deadline = deps.clock.now() + timeout
    while True:
        if _probe_status(state, deps, timeout=deps.probe_timeout) is None:
            return True
        if deps.clock.now() >= deadline:
            return False
        deps.clock.sleep(min(0.1, deadline - deps.clock.now()))


def _sleep(deps: DevDeps, duration: float) -> None:
    deadline = deps.clock.now() + duration
    while deps.clock.now() < deadline:
        deps.clock.sleep(min(0.1, deadline - deps.clock.now()))


# ─── stop orchestrator ───────────────────────────────────────


def do_stop(deps: DevDeps) -> StopResult:
    """Tear down the recorded stack, or recompute discovery if state is lost."""
    state = read_dev_state(deps.state_path)
    if state is None:
        return _stop_without_state(deps)
    return teardown(state, deps, lock_held=False)


def _stop_without_state(deps: DevDeps) -> StopResult:
    # dev-state is a cache, not the sole source of truth: recompute the
    # deterministic ipc:// paths and probe so a lost state file can't orphan.
    endpoint_sock, pubsub_sock = ipc_socket_paths(deps.workspace_root)
    synthetic = DevState(
        endpoint=f"ipc://{endpoint_sock}",
        pubsub_endpoint=f"ipc://{pubsub_sock}",
        frontend_port=0,
        frontend_url="",
        pidfile=str(deps.pidfile),
        ini_path=str(deps.ini_path),
    )
    if _probe_status(synthetic, deps, timeout=deps.probe_timeout) is None:
        remove_artifacts(synthetic, deps, StopResult("clean"), lock_held=False)
        return StopResult("clean", message="Dev stack not running.")
    return teardown(synthetic, deps, lock_held=False)


def do_restart(deps: DevDeps) -> UpResult:
    """Stop then start, with an explicit contract on the seam.

    Only ``clean`` (dev-state confirmed removed) proceeds to a fresh launch. On
    ``survivor``/``refused`` we abort rather than launch a second arbiter to
    compete for ports — a subsequent `mise run dev` reconciles via its lock +
    reuse/stale gate. (``bring_up``'s gate is still authoritative, so even a
    clean-but-not-really slate would be reconciled, not double-launched.)
    """
    result = do_stop(deps)
    if result.kind == "clean":
        return bring_up(deps)
    return UpResult(
        "failed",
        message=(
            result.message
            or f"could not stop the existing stack ({result.kind}); investigate "
            "and re-run `mise run dev:stop`, then `mise run dev`"
        ),
        artifact=str(deps.diagnostic_log),
    )
