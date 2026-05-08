import json

from invoke import Context, task

from tasks.shared.paths import FRONTEND, PLUGIN_JSON, REPO_ROOT, SERVER

_TMP_DIR = REPO_ROOT / ".accelerator/tmp/dev-server"
_CONFIG_PATH = _TMP_DIR / "config.json"
_SERVER_INFO_PATH = _TMP_DIR / "server-info.json"
_SERVER_BIN = SERVER / "target/debug/accelerator-visualiser"


@task
def server(context: Context):
    """Start the visualiser API server in dev mode.

    Writes a minimal server config then starts the debug binary (built by
    build:server:dev). The server binds a random port on 127.0.0.1 and writes
    .accelerator/tmp/dev-server/server-info.json so the Vite dev server can
    discover the port.

    Run in one terminal; run `mise run dev:frontend` in a second terminal once
    the server is up and the info file has been written.
    """
    _TMP_DIR.mkdir(parents=True, exist_ok=True)
    version = json.loads(PLUGIN_JSON.read_text())["version"]
    config = {
        "plugin_root": str(REPO_ROOT),
        "plugin_version": version,
        "project_root": str(REPO_ROOT),
        "tmp_path": str(_TMP_DIR),
        "host": "127.0.0.1",
        "owner_pid": 0,
        "log_path": str(_TMP_DIR / "server.log"),
        "doc_paths": {},
        "templates": {},
    }
    _CONFIG_PATH.write_text(json.dumps(config, indent=2))
    context.run(f"{_SERVER_BIN} --config {_CONFIG_PATH}", pty=True)


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
