import os
import shutil

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
from tasks.shared.paths import CLI_TARGET_DIR, FRONTEND, REPO_ROOT
from tasks.shared.ports import free_port
from tasks.shared.processes import PsutilProcessOps

# Directory-ownership boundary (keep this comment — it survives refactors):
#   .accelerator/tmp/visualiser/ is SERVER-owned: the Model-1 server discovers
#     the project root from its cwd and writes server-info.json, server.pid, and
#     its own server.log there (the composed tmp path).
#   .accelerator/tmp/dev/ is ORCHESTRATION-owned: the lock, dev-state, circus
#     INI, circusd pidfile, captured bootstrap log, and the ipc:// sockets live
#     here.
#   server-info.json is the sole cross-directory contract between them.
_STATE_DIR = REPO_ROOT / ".accelerator/tmp/visualiser"
_SERVER_INFO_PATH = _STATE_DIR / "server-info.json"
_SERVER_PIDFILE = _STATE_DIR / "server.pid"
_SERVER_LOG = _STATE_DIR / "server.log"
_SERVER_BIN = CLI_TARGET_DIR / "debug/accelerator-visualiser"

_DEV_DIR = REPO_ROOT / ".accelerator/tmp/dev"
_DEV_STATE = _DEV_DIR / "dev.json"
_LOCK = _DEV_DIR / "dev.lock"
_PIDFILE = _DEV_DIR / "circusd.pid"
_INI = _DEV_DIR / "circus.ini"
_DIAGNOSTIC_LOG = _DEV_DIR / "dev.log"


def _server_env() -> dict[str, str]:
    """Env for the arbiter and the detached daemon.

    The resolved PATH lets the daemon find node; ACCELERATOR_PLUGIN_ROOT lets
    the Model-1 server resolve plugin templates.
    """
    return {**os.environ, "ACCELERATOR_PLUGIN_ROOT": str(REPO_ROOT)}


def _dev_deps(context: Context) -> DevDeps:
    """Wire DevDeps to the real circus/subprocess/psutil/time collaborators."""
    return DevDeps(
        client_factory=default_client_factory,
        launcher=default_launcher,
        killer=PsutilProcessOps(),
        clock=Clock(),
        project_root=REPO_ROOT,
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
        env=_server_env(),
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
    print(f"  Logs:     {_SERVER_LOG}")
    print(f"            {result.dev_dir}/frontend.log")


@task(default=True)
def up(context: Context) -> None:
    """Start both processes detached in the background under a circus arbiter.

    Returns once ready. The arbiter keeps supervising after this command
    exits — use `dev:stop` to tear it down, or `dev:server`/`dev:frontend`
    for the manual two-terminal flow. Re-running while a healthy session is
    up reuses it.
    """
    result = bring_up(_dev_deps(context))
    if result.kind == "failed":
        raise Exit(result.message, code=1)
    if result.kind == "reused":
        _print_stack_block(
            result,
            heading=(
                "Dev stack already running (reused) — code changes since it "
                "started are NOT live; run `mise run dev:restart` to apply "
                "them."
            ),
        )
        return
    _print_stack_block(result, heading="Visualiser dev stack ready.")


@task
def stop(context: Context) -> None:
    """Stop the supervised dev server + frontend and the circus arbiter."""
    result = do_stop(_dev_deps(context))
    if result.kind == "clean":
        print(result.message or "Dev stack stopped.")
        return
    # refused / survivor: dev-state + sockets kept; point at recovery.
    raise Exit(result.message, code=1)


@task
def restart(context: Context) -> None:
    """Restart the supervised dev stack (stop then start)."""
    result = do_restart(_dev_deps(context))
    if result.kind == "failed":
        raise Exit(result.message, code=1)
    if result.kind == "reused":
        _print_stack_block(
            result,
            heading=(
                "Dev stack already running (reused) — code changes since it "
                "started are NOT live; run `mise run dev:restart` to apply "
                "them."
            ),
        )
        return
    _print_stack_block(result, heading="Visualiser dev stack ready.")


@task
def status(context: Context) -> None:
    """Report dev server + frontend state, frontend URL, and resolved API port.

    Exit code conveys overall state: 0 = both running, 3 = one running,
    4 = neither — identical on macOS and Linux.
    """
    result = do_status(_dev_deps(context))
    for line in result.lines:
        print(line)
    raise Exit(code=result.exit_code)


@task
def server(context: Context) -> None:
    """Start the visualiser API server in dev mode.

    Runs the debug binary (built by build:server:dev) as `serve`, reading
    .accelerator/*.md config directly from the repo root. The server binds a
    random port on 127.0.0.1 and writes
    .accelerator/tmp/visualiser/server-info.json so the Vite dev server can
    discover the port.

    Run in one terminal; run `mise run dev:frontend` in a second terminal once
    the server is up and the info file has been written.
    """
    context.run(
        f"{_SERVER_BIN} serve --owner-pid 0", env=_server_env(), pty=True
    )


@task
def frontend(context: Context) -> None:
    """Start the Vite dev server, proxying /api to the running dev API server.

    Reads the server port from .accelerator/tmp/visualiser/server-info.json,
    which the server writes on startup. Start `mise run dev:server` in a
    separate terminal first.
    """
    context.run(
        f"npm --prefix {FRONTEND} run dev",
        env={"VISUALISER_INFO_PATH": str(_SERVER_INFO_PATH)},
        pty=True,
    )
