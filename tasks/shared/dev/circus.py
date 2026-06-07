"""circus integration for the dev arbiter: INI generation."""

import dataclasses


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
    ``server.log`` (no dual-writer with the legacy ``dev-server`` path).
    ``copy_env`` lets the watchers inherit the detached daemon's resolved PATH
    so npm can find node.
    """
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
stdout_stream.filename = {spec.dev_dir}/server.log
stderr_stream.class = FileStream
stderr_stream.filename = {spec.dev_dir}/server.log

[watcher:frontend]
cmd = {spec.npm_bin} --prefix {spec.frontend} run dev -- --port {spec.frontend_port} --strictPort --host 127.0.0.1
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
