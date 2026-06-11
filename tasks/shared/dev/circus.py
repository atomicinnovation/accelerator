"""circus integration for the dev arbiter.

INI generation, the ``Supervisor`` control adapter, and the self-detaching
``circusd`` launcher.
"""

import contextlib
import dataclasses
import subprocess
from pathlib import Path
from typing import Any, Protocol


@dataclasses.dataclass(frozen=True)
class ArbiterSpec:
    """Everything ``render_circus_ini`` needs to emit the arbiter config."""

    endpoint_socket: str
    pubsub_socket: str
    pidfile: str
    dev_dir: str
    server_bin: str
    config_path: str
    npm_bin: str
    frontend: str
    frontend_port: int
    server_info_path: str


def render_circus_ini(spec: ArbiterSpec) -> str:
    """Render the circus INI for the two-watcher dev arbiter.

    The INI (not the wire) carries the ``FileStream`` log config and the
    ``stop_children``/``graceful_timeout`` invariants. ``stop_children`` is
    mandatory on both watchers because circus signals by PID, not by process
    group; ``graceful_timeout = 2`` overrides circus's 30 s default to honour
    the 2 s SIGTERM grace contract. The server ``cmd`` deliberately omits
    ``--log-file`` so circus's captured stdout is the single writer of
    ``server.log``.

    Logging note (deviation from the plan, forced by the server binary): the
    Rust server initialises ``tracing`` to its config ``--log-file`` and then
    redirects its own stdout/stderr to ``/dev/null`` (``main.rs``), so circus
    cannot capture the server's output via stdout. The server therefore writes
    ``dev/server.log`` itself (via ``--log-file``, rendered by the config
    helper) and circus captures only the server watcher's **pre-redirect**
    stderr — the early "failed to load config" / "failed to init logging"
    lines — to a *separate* ``dev/server.bootstrap.log``, so the two never
    contend for one file. The frontend (Vite) does log to stdout, so circus's
    ``FileStream`` captures it to ``dev/frontend.log`` normally. ``copy_env``
    lets the watchers inherit the detached daemon's resolved PATH so npm can
    find node.
    """
    frontend_cmd = (
        f"{spec.npm_bin} --prefix {spec.frontend} run dev "
        f"-- --port {spec.frontend_port} --strictPort --host 127.0.0.1"
    )
    return f"""\
[circus]
endpoint = ipc://{spec.endpoint_socket}
pubsub_endpoint = ipc://{spec.pubsub_socket}
check_delay = 1
pidfile = {spec.pidfile}

[watcher:server]
cmd = {spec.server_bin} --config {spec.config_path}
numprocesses = 1
autostart = true
respawn = false
stop_children = true
graceful_timeout = 2
copy_env = true
stdout_stream.class = FileStream
stdout_stream.filename = {spec.dev_dir}/server.bootstrap.log
stderr_stream.class = FileStream
stderr_stream.filename = {spec.dev_dir}/server.bootstrap.log

[watcher:frontend]
cmd = {frontend_cmd}
numprocesses = 1
autostart = false
respawn = false
stop_children = true
graceful_timeout = 2
copy_env = true
stdout_stream.class = FileStream
stdout_stream.filename = {spec.dev_dir}/frontend.log
stderr_stream.class = FileStream
stderr_stream.filename = {spec.dev_dir}/frontend.log

[env:frontend]
VISUALISER_INFO_PATH = {spec.server_info_path}
"""


# ─── supervisor protocol + circus adapter ────────────────────


class SupervisorUnreachableError(Exception):
    """Raised when the arbiter endpoint cannot be reached or errors.

    A local boundary so the orchestrators never see ``circus.exc.CallError`` or
    raw ZMQ errors.
    """


class Supervisor(Protocol):
    """Minimal control surface orchestrators depend on (not circus verbs)."""

    def status(self) -> dict[str, str]: ...
    def pids(self, name: str) -> list[int]: ...
    def start(self, name: str) -> None: ...
    def quit(self) -> None: ...


class CircusSupervisor:
    """Thin circus adapter implementing the ``Supervisor`` protocol.

    Parses the real circus wire shapes (``{"statuses": {name: state}}`` for all
    watchers, ``{"status": state}`` for one) and translates an unreachable or
    timed-out endpoint into ``SupervisorUnreachableError``. circus is imported
    lazily so the helper tests never pull in pyzmq/tornado.
    """

    def __init__(self, endpoint: str, *, timeout: float) -> None:
        from circus.client import CircusClient

        self._client = CircusClient(endpoint=endpoint, timeout=timeout)

    def _call(self, command: str, **props: object) -> dict[str, Any]:
        from circus.exc import CallError

        try:
            return self._client.send_message(command, **props)
        except CallError as exc:
            raise SupervisorUnreachableError(str(exc)) from exc
        except Exception as exc:  # zmq/transport errors connect lazily
            raise SupervisorUnreachableError(str(exc)) from exc

    def status(self) -> dict[str, str]:
        resp = self._call("status")
        if "statuses" in resp:
            return dict(resp["statuses"])
        state = resp.get("status")
        if state not in (None, "ok", "error"):
            return {"_": state}  # single-watcher shape (defensive)
        return {}

    def pids(self, name: str) -> list[int]:
        # circus `list <name>` returns {"pids": [...]} for a watcher's
        # processes.
        resp = self._call("list", name=name)
        return [int(p) for p in resp.get("pids", [])]

    def start(self, name: str) -> None:
        self._call("start", name=name)

    def quit(self) -> None:
        self._call("quit")

    def close(self) -> None:
        with contextlib.suppress(Exception):
            self._client.stop()


def default_client_factory(endpoint: str, *, timeout: float) -> Supervisor:
    return CircusSupervisor(endpoint, timeout=timeout)


# ─── self-detaching circusd launcher ─────────────────────────


class LaunchHandle(Protocol):
    pid: int

    def poll(self) -> int | None: ...


class PopenHandle:
    def __init__(self, popen: subprocess.Popen[bytes]) -> None:
        self._popen = popen
        self.pid = popen.pid

    def poll(self) -> int | None:
        return self._popen.poll()


def default_launcher(
    argv: list[str], *, env: dict[str, str], cwd: str
) -> LaunchHandle:
    # start_new_session detaches into a new session, so the arbiter survives the
    # invoking shell *and* the Popen PID is the real arbiter (we do NOT use
    # --daemon, whose double-fork would make the handle a useless intermediate).
    log_path = Path(cwd) / ".accelerator/tmp/dev/circusd.log"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log = log_path.open("ab")  # handed to Popen, closed on process exit
    popen = subprocess.Popen(
        argv, env=env, cwd=cwd, stdout=log, stderr=log, start_new_session=True
    )
    return PopenHandle(popen)
