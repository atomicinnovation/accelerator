"""The research guard's lexer, checked against the shells that run its verdicts.

Every command the built guard passes is run under bash and zsh with a stub
`accelerator` first on `PATH`. A pass that writes a file, starts a second
command, or reaches the network is a smuggle the lexer missed. The corpus is
the measured baseline, the lexer-boundary rows, and seeded mutants that
splice shell metacharacters into the boundary rows.

Workers are threads rather than processes: each spends its time waiting on
subprocesses, and a thread owns its loopback listener as a worker process
would.
"""

import contextlib
import json
import os
import random
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
from collections.abc import Iterator
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURES = REPO_ROOT / "cli/research/tests/fixtures"
GUARD = Path(
    os.environ.get(
        "ACCELERATOR_RESEARCH_BIN",
        REPO_ROOT / "cli/target/debug/accelerator-research",
    )
)

PREFIX = "accelerator research fetch "
MEASURED_PREFIX = "./probe"
PORT = "{port}"
ESCAPES = {
    "⏎": "\n",
    "⇥": "\t",
    "␍": "\r",
    "⍽": "\u00a0",
    "␋": "\v",
    "␌": "\f",
}
ALPHABET = (
    " ",
    "\t",
    "\r",
    "\v",
    "\f",
    "\u00a0",
    "\\",
    "\\\n",
    "#",
    "'",
    '"',
    "$",
    "$(",
    "$0[",
    "$=HOME[",
    "$HOME:",
    "<",
    f"</dev/tcp/127.0.0.1/{PORT}",
    ">&",
    ";",
    "&",
    "|",
    "\n",
)
DEFAULT_SEED = 280
MUTANTS_PER_ROW = 150
WORKERS = max(4, os.cpu_count() or 4)
SHELL_TIMEOUT = 10.0

STUB = """#!/bin/sh
printf '%s\\n' "$*" >> "$ACCELERATOR_STUB_LOG"
exit 0
"""


def _unescape(text: str) -> str:
    for symbol, character in ESCAPES.items():
        text = text.replace(symbol, character)
    return text


def _rows(name: str) -> list[list[str]]:
    lines = [
        line
        for line in (FIXTURES / name).read_text(encoding="utf-8").splitlines()
        if not line.startswith("#")
    ]
    return [
        [_unescape(cell) for cell in line.split("\t")] for line in lines[1:]
    ]


def _baseline() -> list[str]:
    return [
        row[0].replace(MEASURED_PREFIX, PREFIX.rstrip(), 1)
        for row in _rows("claude-bash-baseline.tsv")
    ]


def _boundary() -> list[str]:
    return [row[0] for row in _rows("guard-boundary.tsv")]


def _mutants(rows: list[str], seed: int, exhaustive: bool) -> list[str]:
    generator = random.Random(seed)
    mutants: list[str] = []
    for row in rows:
        if not row.startswith(PREFIX):
            continue
        splices = [
            (position, unit)
            for position in range(len(PREFIX), len(row) + 1)
            for unit in ALPHABET
        ]
        if not exhaustive:
            splices = generator.sample(
                splices, min(MUTANTS_PER_ROW, len(splices))
            )
        mutants.extend(row[:at] + unit + row[at:] for at, unit in splices)
    return mutants


class Listener:
    """A loopback TCP listener recording every connection it accepts."""

    def __init__(self) -> None:
        self._socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._socket.bind(("127.0.0.1", 0))
        self._socket.listen(64)
        self.port = self._socket.getsockname()[1]
        self._connections = 0
        self._lock = threading.Lock()
        threading.Thread(target=self._accept, daemon=True).start()

    def _accept(self) -> None:
        while True:
            try:
                connection, _ = self._socket.accept()
            except OSError:
                return
            with self._lock:
                self._connections += 1
            connection.close()

    def clear(self) -> None:
        with self._lock:
            self._connections = 0

    def connected(self) -> bool:
        with self._lock:
            return self._connections > 0


@dataclass
class Harness:
    shells: list[str]
    work: Path
    home: Path
    stub_dir: Path
    stub_log: Path
    local: threading.local = field(default_factory=threading.local)

    def listener(self) -> Listener:
        if not hasattr(self.local, "listener"):
            self.local.listener = Listener()
        return self.local.listener

    def env(self) -> dict[str, str]:
        return {
            "HOME": str(self.home),
            "ZDOTDIR": str(self.home),
            "PATH": f"{self.stub_dir}{os.pathsep}{os.environ['PATH']}",
            "ACCELERATOR_STUB_LOG": str(self.stub_log),
        }

    def verdict(self, command: str) -> int:
        payload = {
            "agent_id": "differential",
            "agent_type": "accelerator:researcher",
            "tool_name": "Bash",
            "cwd": str(self.work),
            "tool_input": {"command": command},
        }
        return subprocess.run(
            [str(GUARD), "guard"],
            input=json.dumps(payload),
            capture_output=True,
            text=True,
            check=False,
        ).returncode

    def smuggled(self, shell: str, command: str, listener: Listener) -> str:
        """What running `command` left behind, or the empty string."""
        listener.clear()
        with tempfile.TemporaryDirectory(dir=self.work) as scratch:
            directory = Path(scratch)
            _run_group(shell, command, directory, self.env())
            left = sorted(path.name for path in directory.iterdir())
        if left:
            return f"left {left}"
        if listener.connected():
            return "reached the network"
        return ""


def _run_group(
    shell: str, command: str, cwd: Path, env: dict[str, str]
) -> None:
    process = subprocess.Popen(
        [shell, "-c", command],
        cwd=cwd,
        env=env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    try:
        process.wait(timeout=SHELL_TIMEOUT)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()
    _await_group(process.pid)


def _await_group(group: int) -> None:
    deadline = time.monotonic() + SHELL_TIMEOUT
    while time.monotonic() < deadline:
        try:
            os.killpg(group, 0)
        except ProcessLookupError, PermissionError:
            return
        time.sleep(0.01)
    with contextlib.suppress(ProcessLookupError):
        os.killpg(group, signal.SIGKILL)


def _shells() -> list[str]:
    shells = []
    for name in ("bash", "zsh"):
        found = shutil.which(name)
        if found is None:
            pytest.fail(
                f"{name} not found: the research guard differential needs it "
                f"(apt-get install {name} / brew install {name})"
            )
        shells.append(found)
    if sys.platform == "darwin":
        shells.append("/bin/bash")
    for shell in shells:
        version = subprocess.run(
            [shell, "--version"], capture_output=True, text=True, check=False
        ).stdout.splitlines()
        print(f"{shell}: {version[0] if version else 'unknown version'}")
    return shells


@pytest.fixture
def harness(tmp_path: Path) -> Iterator[Harness]:
    assert GUARD.is_file(), (
        f"{GUARD} is absent; build it (build:cli:dev) before this lane"
    )
    stub_dir = tmp_path / "stub"
    stub_dir.mkdir()
    stub = stub_dir / "accelerator"
    stub.write_text(STUB)
    stub.chmod(0o755)
    home = tmp_path / "home"
    home.mkdir()
    work = tmp_path / "work"
    work.mkdir()
    yield Harness(
        shells=_shells(),
        work=work,
        home=home,
        stub_dir=stub_dir,
        stub_log=tmp_path / "stub.log",
    )


def _judge(harness: Harness, template: str) -> list[str]:
    listener = harness.listener()
    command = template.replace(PORT, str(listener.port))
    code = harness.verdict(command)
    if code == 2:
        return []
    if code != 0:
        return [f"{command!r}: the guard exited {code}"]
    return [
        f"{command!r} under {shell}: {outcome}"
        for shell in harness.shells
        if (outcome := harness.smuggled(shell, command, listener))
    ]


_CONTROLS = (
    (PREFIX + "x; touch smuggled", "any"),
    (PREFIX + "$(touch smuggled)", "any"),
    (PREFIX + "x > rel", "any"),
    (PREFIX + f"x </dev/tcp/127.0.0.1/{PORT}", "bash"),
)


def _assert_controls_smuggle(harness: Harness) -> None:
    listener = Listener()
    for template, shells in _CONTROLS:
        command = template.replace(PORT, str(listener.port))
        for shell in harness.shells:
            if shells == "bash" and "bash" not in Path(shell).name:
                continue
            outcome = harness.smuggled(shell, command, listener)
            assert outcome, (
                f"control {command!r} smuggled nothing under {shell}: the "
                "harness cannot detect what it is looking for"
            )
    assert harness.stub_log.is_file() and harness.stub_log.read_text(), (
        "the stub accelerator was never invoked: no passed string ran"
    )


def test_no_string_the_guard_passes_smuggles_anything(harness: Harness) -> None:
    seed = int(os.environ.get("RESEARCH_DIFFERENTIAL_SEED", DEFAULT_SEED))
    exhaustive = os.environ.get("RESEARCH_DIFFERENTIAL_EXHAUSTIVE") == "1"
    boundary = _boundary()
    corpus = _baseline() + boundary + _mutants(boundary, seed, exhaustive)
    print(f"seed {seed}: {len(corpus)} strings")

    with ThreadPoolExecutor(max_workers=WORKERS) as pool:
        failures = [
            failure
            for found in pool.map(lambda text: _judge(harness, text), corpus)
            for failure in found
        ]

    assert not failures, (
        f"seed {seed} (RESEARCH_DIFFERENTIAL_SEED): the guard passed "
        f"{len(failures)} smuggling string(s):\n" + "\n".join(failures[:50])
    )
    _assert_controls_smuggle(harness)
