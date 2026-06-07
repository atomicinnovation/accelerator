import json
import os
import shutil
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.clock import Clock
from tasks.shared.dev.circus import default_client_factory, default_launcher
from tasks.shared.dev.lifecycle import (
    DevDeps,
    UpResult,
    bring_up,
    do_restart,
    do_status,
    do_stop,
)
from tasks.shared.paths import FRONTEND, PLUGIN_JSON, REPO_ROOT, SERVER, VISUALISER
from tasks.shared.ports import free_port
from tasks.shared.processes import PsutilProcessOps

# Directory-ownership boundary (keep this comment — it survives refactors):
#   .accelerator/tmp/dev-server/ is SERVER-owned: the server writes config.json,
#     server-info.json, and its own server.pid there.
#   .accelerator/tmp/dev/ is ORCHESTRATION-owned: the lock, dev-state, circus
#     INI, circusd pidfile, captured logs, and the ipc:// sockets (under a short
#     $TMPDIR-rooted hashed base recorded in dev-state) live here.
#   server-info.json is the sole cross-directory contract between them.
# The legacy dev.server runs the binary with --log-file {dev-server}/server.log.
# The unified path points the server's --log-file at {dev}/server.log instead
# (distinct path, no clash) and additionally has circus capture the server
# watcher's pre-/dev/null stderr to {dev}/server.bootstrap.log.
_TMP_DIR = REPO_ROOT / ".accelerator/tmp/dev-server"
_CONFIG_PATH = _TMP_DIR / "config.json"
_SERVER_INFO_PATH = _TMP_DIR / "server-info.json"
_SERVER_PIDFILE = _TMP_DIR / "server.pid"
_SERVER_BIN = SERVER / "target/debug/accelerator-visualiser"
_WRITE_CONFIG = VISUALISER / "scripts/write-visualiser-config.sh"

_DEV_DIR = REPO_ROOT / ".accelerator/tmp/dev"
_DEV_STATE = _DEV_DIR / "dev.json"
_LOCK = _DEV_DIR / "dev.lock"
_PIDFILE = _DEV_DIR / "circusd.pid"
_INI = _DEV_DIR / "circus.ini"
_DIAGNOSTIC_LOG = _DEV_DIR / "dev.log"


def _render_server_config(context: Context, *, log_file: Path) -> Path:
    """Render config.json via write-visualiser-config.sh and return its path.

    Shared by the legacy dev.server (log_file under dev-server/) and the unified
    arbiter path (log_file under dev/). --owner-pid 0 disables owner-based
    auto-shutdown for the dev path.
    """
    _TMP_DIR.mkdir(parents=True, exist_ok=True)
    version = json.loads(PLUGIN_JSON.read_text())["version"]
    result = context.run(
        f"{_WRITE_CONFIG}"
        f" --plugin-version {version}"
        f" --project-root {REPO_ROOT}"
        f" --tmp-dir {_TMP_DIR}"
        f" --log-file {log_file}"
        f" --owner-pid 0",
        hide=True,
    )
    _CONFIG_PATH.write_text(result.stdout)
    return _CONFIG_PATH


def _dev_deps(context: Context) -> DevDeps:
    """Wire DevDeps to the real circus / subprocess / psutil / time collaborators."""
    return DevDeps(
        client_factory=default_client_factory,
        launcher=default_launcher,
        killer=PsutilProcessOps(),
        clock=Clock(),
        config_renderer=lambda: _render_server_config(context, log_file=_DEV_DIR / "server.log"),
        workspace_root=REPO_ROOT,
        state_path=_DEV_STATE,
        lock_path=_LOCK,
        dev_dir=_DEV_DIR,
        pidfile=_PIDFILE,
        ini_path=_INI,
        server_info_path=_SERVER_INFO_PATH,
        server_pidfile=_SERVER_PIDFILE,
        server_bin=_SERVER_BIN,
        frontend=FRONTEND,
        diagnostic_log=_DIAGNOSTIC_LOG,
        env=os.environ.copy(),  # resolved PATH so the detached daemon finds node
        npm_bin=shutil.which("npm") or "npm",
        node_bin=shutil.which("node") or "node",
        free_port=free_port,
    )


def _print_stack_block(result: UpResult, *, heading: str) -> None:
    api_line = (
        f"http://127.0.0.1:{result.api_port}"
        if result.api_url is None and result.api_port is not None
        else (result.api_url or "(not resolved)")
    )
    print(heading)
    print(f"  Frontend: {result.frontend_url}")
    print(f"  API:      {api_line}")
    print(f"  Logs:     {result.dev_dir}/server.log")
    print(f"            {result.dev_dir}/frontend.log")


@task(default=True)
def up(context: Context):
    """Start both processes detached in the background under a circus arbiter.

    Returns once ready. The arbiter keeps supervising after this command exits —
    use `dev:stop` to tear it down, or `dev:server`/`dev:frontend` for the manual
    two-terminal flow. Re-running while a healthy session is up reuses it.
    """
    result = bring_up(_dev_deps(context))
    if result.kind == "failed":
        raise Exit(result.message, code=1)
    if result.kind == "reused":
        _print_stack_block(
            result,
            heading=(
                "Dev stack already running (reused) — code changes since it "
                "started are NOT live; run `mise run dev:restart` to apply them."
            ),
        )
        return
    _print_stack_block(result, heading="Visualiser dev stack ready.")


@task
def stop(context: Context):
    """Stop the supervised dev server + frontend and the circus arbiter."""
    result = do_stop(_dev_deps(context))
    if result.kind == "clean":
        print(result.message or "Dev stack stopped.")
        return
    # refused / survivor: dev-state + sockets kept; point at recovery.
    raise Exit(result.message, code=1)


@task
def restart(context: Context):
    """Restart the supervised dev stack (stop then start)."""
    result = do_restart(_dev_deps(context))
    if result.kind == "failed":
        raise Exit(result.message, code=1)
    if result.kind == "reused":
        _print_stack_block(
            result,
            heading=(
                "Dev stack already running (reused) — code changes since it "
                "started are NOT live; run `mise run dev:restart` to apply them."
            ),
        )
        return
    _print_stack_block(result, heading="Visualiser dev stack ready.")


@task
def status(context: Context):
    """Report dev server + frontend state, frontend URL, and resolved API port.

    Exit code conveys overall state: 0 = both running, 3 = one running,
    4 = neither — identical on macOS and Linux.
    """
    result = do_status(_dev_deps(context))
    for line in result.lines:
        print(line)
    raise Exit(code=result.exit_code)


@task
def server(context: Context):
    """Start the visualiser API server in dev mode.

    Generates a server config via write-visualiser-config.sh (picks up
    .accelerator/config.md overrides) then starts the debug binary (built by
    build:server:dev). The server binds a random port on 127.0.0.1 and writes
    .accelerator/tmp/dev-server/server-info.json so the Vite dev server can
    discover the port.

    Run in one terminal; run `mise run dev:frontend` in a second terminal once
    the server is up and the info file has been written.
    """
    config_path = _render_server_config(context, log_file=_TMP_DIR / "server.log")
    context.run(f"{_SERVER_BIN} --config {config_path}", pty=True)


@task
def frontend(context: Context):
    """Start the Vite dev server, proxying /api to the running dev API server.

    Reads the server port from .accelerator/tmp/dev-server/server-info.json,
    which the server writes on startup. Start `mise run dev:server` in a
    separate terminal first.
    """
    context.run(
        f"npm --prefix {FRONTEND} run dev",
        env={"VISUALISER_INFO_PATH": str(_SERVER_INFO_PATH)},
        pty=True,
    )
