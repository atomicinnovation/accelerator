"""Run a block of work against the visualiser server started on the *host*.

The Docker visual-regression task containerises only Chromium; the Rust server
runs on the host (exactly as CI does), reached over ``host.docker.internal``.
This helper owns that server's lifecycle: spawn ``node e2e/start-server.mjs``,
wait for it to publish its port to ``.e2e-port`` (short-circuiting if it dies
first), run the caller's ``on_ready`` against that port, and reap the
node→Rust process tree afterwards.

The risk-bearing spawn/poll/teardown is built on the same injected seams the
unified dev lifecycle uses (``Clock``, ``ProcessOps``, and a launcher returning
a ``LaunchHandle``), so all paths — ready, server-died-early,
port-never-published, terminate-times-out-then-kill — are unit-testable with
fakes and no live node process.
"""

import os
import subprocess
from collections.abc import Callable
from pathlib import Path

from tasks.shared.clock import Clock
from tasks.shared.dev.circus import LaunchHandle, PopenHandle
from tasks.shared.paths import FRONTEND
from tasks.shared.playwright import E2E_LANG
from tasks.shared.processes import ProcessOps, PsutilProcessOps


class HostServerError(RuntimeError):
    """The host server never became ready (exited early or timed out)."""


def default_host_server_launcher(
    argv: list[str], *, env: dict[str, str], cwd: str
) -> LaunchHandle:
    # stdout/stderr are inherited (not redirected) so the server's own streamed
    # output reaches the console — start-server.mjs spawns the binary with
    # stdio:"inherit", and the error messages here point the reader at it.
    popen = subprocess.Popen(argv, env=env, cwd=cwd)
    return PopenHandle(popen)


def run_against_host_server(
    *,
    server_bin: Path,
    on_ready: Callable[[str], None],
    frontend: Path = FRONTEND,
    lang: str = E2E_LANG,
    launcher: Callable[..., LaunchHandle] = default_host_server_launcher,
    killer: ProcessOps | None = None,
    clock: Clock | None = None,
    node_bin: str = "node",
    env: dict[str, str] | None = None,
    readiness_timeout: float = 60.0,
    poll_interval: float = 0.1,
    grace_kill: float = 2.0,
) -> None:
    """Spawn the host server, run ``on_ready(port)``, then reap the tree.

    Binds the server on ``0.0.0.0`` (via ``E2E_SERVER_HOST``) so the container
    can reach it over the bridge gateway; the binding is transient (one
    compare/rebaseline run) and serves only non-sensitive committed fixtures.
    """
    procs = killer if killer is not None else PsutilProcessOps()
    clock = clock if clock is not None else Clock()
    port_file = frontend / ".e2e-port"
    port_file.unlink(missing_ok=True)

    spawn_env = dict(env if env is not None else os.environ)
    spawn_env.update(
        {
            "ACCELERATOR_VISUALISER_BIN": str(server_bin),
            # The container reaches the host over the bridge gateway, so the
            # server must bind all interfaces, not just loopback. Transient
            # (one run) and serves only non-sensitive committed fixtures.
            "E2E_SERVER_HOST": "0.0.0.0",  # noqa: S104
            # Opt into the dev-frontend server's non-loopback bind + relaxed
            # Host-header guard for the duration of this run. The bypass exists
            # only in the dev-frontend (test) binary and only when this env var
            # is set — release builds and normal `mise run dev` stay
            # loopback-only.
            "ACCELERATOR_VISUALISER_E2E_INSECURE": "1",
            # The host Rust server's locale comes from the same single source as
            # the container's (tasks/shared/playwright.py:E2E_LANG).
            "LANG": lang,
            "LC_ALL": lang,
        }
    )

    handle = launcher(
        [node_bin, "e2e/start-server.mjs"],
        env=spawn_env,
        cwd=str(frontend),
    )
    try:
        port = _await_port(
            handle, port_file, clock, readiness_timeout, poll_interval
        )
        on_ready(port)
    finally:
        _reap(handle, procs, clock, grace_kill)
        port_file.unlink(missing_ok=True)


def _await_port(
    handle: LaunchHandle,
    port_file: Path,
    clock: Clock,
    timeout: float,
    interval: float,
) -> str:
    """Poll for ``.e2e-port`` while interleaving ``handle.poll()``.

    A server that dies before publishing the port short-circuits immediately;
    the raised error distinguishes *exited (code N)* — the code from
    ``handle.poll()`` — from *timed out*, and points at the streamed output.
    """
    deadline = clock.now() + timeout
    while True:
        exit_code = handle.poll()
        if exit_code is not None:
            raise HostServerError(
                f"host server exited (code {exit_code}) before publishing its "
                "port — see the server output streamed above."
            )
        if port_file.exists():
            text = port_file.read_text().strip()
            if text:
                return text
        if clock.now() >= deadline:
            raise HostServerError(
                f"host server did not publish {port_file} within "
                f"{timeout:.0f}s — see the server output streamed above."
            )
        clock.sleep(min(interval, deadline - clock.now()))


def _reap(
    handle: LaunchHandle, procs: ProcessOps, clock: Clock, grace_kill: float
) -> None:
    """Reap the node leader and its children: terminate → wait → kill.

    Mirrors ``lifecycle._reap_handle`` but escalates SIGTERM→SIGKILL through the
    injected ``ProcessOps`` (so teardown is observable on the fake, and there is
    no ``os.killpg``/``os.getpgid`` to raise on the already-exited path). The
    Rust child is a plain child of the node launcher, so ``children()`` finds it
    while the leader is alive; an already-exited leader makes this a no-op.
    """
    targets = [handle.pid, *(cpid for cpid, _ in procs.children(handle.pid))]
    for pid in targets:
        procs.terminate(pid)
    deadline = clock.now() + grace_kill
    while clock.now() < deadline:
        if not any(procs.is_alive(pid) for pid in targets):
            return
        clock.sleep(min(0.1, deadline - clock.now()))
    for pid in targets:
        if procs.is_alive(pid):
            procs.kill(pid)
