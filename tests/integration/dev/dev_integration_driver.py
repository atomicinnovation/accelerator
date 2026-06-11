#!/usr/bin/env python3
"""Subprocess driver for the dev-task integration suite.

Runs the *real* dev orchestrators (bring_up / do_stop / do_status / do_restart)
against a *real* circusd, using lightweight Python fake processes for the
"server" and "frontend" so the lifecycle is exercised without building Rust or
booting Vite. It is run as a subprocess (never imported) so the launching
process exits after ``up`` returns — reparenting the detached arbiter to init,
which is what makes the post-hoc ``psutil`` liveness checks in the tests
accurate (an un-reaped child would read as a zombie "alive" forever).

Usage:
    python dev_integration_driver.py --workspace <dir> --action up \
        [--opts <json-file>]
"""

import argparse
import json
import os
import sys
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
    bring_up,
    do_restart,
    do_status,
    do_stop,
)
from tasks.shared.ports import free_port
from tasks.shared.processes import PsutilProcessOps

_SERVER_TEMPLATE = """\
#!{interp}
import json, signal, subprocess, sys, time
cfg = json.load(open(sys.argv[sys.argv.index("--config") + 1]))
{ignore}
open(cfg["log_path"], "a").write({marker!r} + "\\n")
{spawn}
if {write_info}:
    json.dump({{"url": "http://127.0.0.1:%d" % {port}, "port": {port}}},
              open(cfg["info_path"], "w"))
time.sleep(3600)
"""

_NPM_TEMPLATE = """\
#!{interp}
import signal, subprocess, sys, time
{ignore}
print({marker!r}, flush=True)
{spawn}
time.sleep(3600)
"""

_IGNORE = "signal.signal(signal.SIGTERM, signal.SIG_IGN)"
# Spawn a descendant via the absolute interpreter so it needs no PATH (the
# stripped-PATH test strips everything but the venv bin).
_SPAWN = (
    "subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(3600)'])"
)


def _write_fakes(workspace: Path, opts: dict) -> tuple[Path, Path]:
    bin_dir = workspace / "fakebin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    # Absolute-interpreter shebang so the fakes run regardless of PATH (the
    # stripped-PATH test must exercise npm resolution, not break the shebang).
    server = bin_dir / "fake-server"
    server.write_text(
        _SERVER_TEMPLATE.format(
            interp=sys.executable,
            ignore=_IGNORE if opts.get("server_ignore_sigterm") else "",
            marker=opts.get("server_marker", "SERVER_UP"),
            spawn=_SPAWN if opts.get("server_spawn_child", True) else "",
            write_info=bool(opts.get("server_write_info", True)),
            port=opts.get("api_port", 7777),
        )
    )
    server.chmod(0o755)
    npm = bin_dir / "fake-npm"
    npm.write_text(
        _NPM_TEMPLATE.format(
            interp=sys.executable,
            ignore=_IGNORE if opts.get("fe_ignore_sigterm") else "",
            marker=opts.get("fe_marker", "FRONTEND_UP"),
            spawn=_SPAWN if opts.get("fe_spawn_child", True) else "",
        )
    )
    npm.chmod(0o755)
    return server, npm


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
    server_dir = workspace / ".accelerator/tmp/dev-server"
    dev_dir.mkdir(parents=True, exist_ok=True)
    server_dir.mkdir(parents=True, exist_ok=True)
    server_bin, npm_bin = _write_fakes(workspace, opts)
    server_info_path = server_dir / "server-info.json"

    def config_renderer() -> Path:
        config_path = server_dir / "config.json"
        config_path.write_text(
            json.dumps(
                {
                    "info_path": str(server_info_path),
                    "log_path": str(dev_dir / "server.log"),
                }
            )
        )
        return config_path

    env = os.environ.copy()
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
        killer=PsutilProcessOps(),
        clock=Clock(),
        config_renderer=config_renderer,
        workspace_root=workspace,
        state_path=dev_dir / "dev.json",
        lock_path=dev_dir / "dev.lock",
        dev_dir=dev_dir,
        pidfile=dev_dir / "circusd.pid",
        ini_path=dev_dir / "circus.ini",
        server_info_path=server_info_path,
        server_pidfile=server_dir / "server.pid",
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
        probe_timeout=opts.get("probe_timeout", 5.0),
        pidfile_timeout=opts.get("pidfile_timeout", 20.0),
        readiness_timeout=opts.get("readiness_timeout", 20.0),
        frontend_active_timeout=opts.get("frontend_active_timeout", 20.0),
        grace_quit=opts.get("grace_quit", 6.0),
        grace_kill=opts.get("grace_kill", 3.0),
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", required=True)
    parser.add_argument(
        "--action", required=True, choices=["up", "stop", "status", "restart"]
    )
    parser.add_argument("--opts")
    args = parser.parse_args()

    opts = json.loads(Path(args.opts).read_text()) if args.opts else {}
    deps = _build_deps(Path(args.workspace), opts)

    if args.action == "up":
        result = bring_up(deps)
        print(json.dumps({"kind": result.kind, "message": result.message}))
        return 0 if result.kind in ("started", "reused") else 1
    if args.action == "restart":
        result = do_restart(deps)
        print(json.dumps({"kind": result.kind, "message": result.message}))
        return 0 if result.kind in ("started", "reused") else 1
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
