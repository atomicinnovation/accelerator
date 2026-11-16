"""The fake server, frontend and child processes the dev suite supervises.

Each fake records ``started`` in the events log once its SIGTERM disposition
is in place, and ``sigterm`` when it receives one, so the tests can tell a
graceful stop from an escalated one. It imports only the standard library and
psutil because the tests also run it directly, without the repo on the path.
"""

import argparse
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

import psutil

EVENTS_LOG = "fakes.log"
ESCALATIONS_LOG = "escalations.log"
KILLED_BY_TEST_LOG = "killed-by-test.log"


class FakeLifecycle:
    def __init__(self, role: str, events_log: Path) -> None:
        self._role = role
        self._events_log = events_log
        self._identity = f"{os.getpid()} {psutil.Process().create_time()}"

    def begin(self, *, ignore_sigterm: bool) -> None:
        handler = signal.SIG_IGN if ignore_sigterm else self._on_sigterm
        signal.signal(signal.SIGTERM, handler)
        self._record("started")

    def spawn_recorded_child(self) -> None:
        child = subprocess.Popen(
            [
                sys.executable,
                __file__,
                "child",
                "--role",
                f"{self._role}-child",
                "--events-log",
                str(self._events_log),
            ]
        )
        while f"started {self._role}-child {child.pid} " not in self._log():
            if child.poll() is not None:
                sys.exit(f"{self._role}-child exited before recording itself")
            time.sleep(0.01)

    def _on_sigterm(self, signum, frame) -> None:
        self._record("sigterm")
        os._exit(0)

    def _record(self, event: str) -> None:
        with self._events_log.open("a") as log:
            log.write(f"{event} {self._role} {self._identity}\n")

    def _log(self) -> str:
        return self._events_log.read_text()


def announce_server(args: argparse.Namespace) -> None:
    state_dir = Path.cwd() / ".accelerator" / "tmp" / "visualiser"
    state_dir.mkdir(parents=True, exist_ok=True)
    with (state_dir / "server.log").open("a") as log:
        log.write(f"{args.marker}\n")
    if args.write_info:
        (state_dir / "server-info.json").write_text(
            json.dumps(
                {
                    "url": f"http://127.0.0.1:{args.port}",
                    "port": args.port,
                    "pid": os.getpid(),
                }
            )
        )


def announce_frontend(args: argparse.Namespace) -> None:
    print(args.marker, flush=True)


def announce_child(args: argparse.Namespace) -> None:
    pass


ANNOUNCEMENTS = {
    "server": announce_server,
    "frontend": announce_frontend,
    "child": announce_child,
}


def parse(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("behaviour", choices=ANNOUNCEMENTS)
    parser.add_argument("--role")
    parser.add_argument("--events-log", type=Path, required=True)
    parser.add_argument("--ignore-sigterm", action="store_true")
    parser.add_argument("--spawn-child", action="store_true")
    parser.add_argument("--marker", default="")
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--write-info", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> None:
    args = parse(argv)
    fake = FakeLifecycle(args.role or args.behaviour, args.events_log)
    fake.begin(ignore_sigterm=args.ignore_sigterm)
    if args.spawn_child:
        fake.spawn_recorded_child()
    ANNOUNCEMENTS[args.behaviour](args)
    while True:
        signal.pause()


if __name__ == "__main__":
    main(sys.argv[1:])
