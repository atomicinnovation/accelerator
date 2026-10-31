"""The gates an eval run passes before it starts any session."""

import re
import subprocess
import tempfile
import uuid
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from tasks.eval.ask_and_report.conversation import RULE, opening_turn
from tasks.eval.ask_and_report.projects import build_project
from tasks.eval.ask_and_report.scenarios import Setup
from tasks.eval.headless.claude import prepare_repository, run_session
from tasks.eval.headless.transcript import Event, Result, events

MINIMUM_CLAUDE = (2, 1, 144)
AUTH_FAILURE_MARKERS = ("authentication", "api key", "401", "log in")
PROBE_FILE = "probe.txt"


def claude_version() -> str:
    return subprocess.run(
        ["claude", "--version"], capture_output=True, text=True, check=True
    ).stdout.strip()


def version_refusal(version: str) -> str | None:
    found = re.search(r"(\d+)\.(\d+)\.(\d+)", version)
    floor = ".".join(map(str, MINIMUM_CLAUDE))
    if found and tuple(map(int, found.groups())) >= MINIMUM_CLAUDE:
        return None
    return f"The eval needs Claude Code {floor} or later, not {version!r}."


@dataclass(frozen=True)
class ProbeOutcome:
    command: Sequence[str]
    exit_status: int
    events: tuple[Event, ...]
    stderr: str
    wrote_probe: bool
    rule: Path


def _denied(outcome: ProbeOutcome, tools: set[str]) -> bool:
    return any(
        denial.get("tool_name") in tools
        for event in outcome.events
        if isinstance(event, Result)
        for denial in event.permission_denials
    )


def _result(outcome: ProbeOutcome) -> Result | None:
    return next((e for e in outcome.events if isinstance(e, Result)), None)


def probe_refusal(outcome: ProbeOutcome) -> str | None:
    """Explain why the probe shows no session could run, if it does."""
    result = _result(outcome)
    failed = result is not None and result.is_error
    if (
        outcome.exit_status == 0
        and not failed
        and not _denied(outcome, {"Read", "Write", "Edit"})
        and outcome.wrote_probe
    ):
        return None
    error = (result.text if failed and result else "") or outcome.stderr
    if any(marker in error.casefold() for marker in AUTH_FAILURE_MARKERS):
        return (
            f"The authentication probe failed: {error.strip()}. Sessions run "
            "under --setting-sources project, so authenticate with the config "
            "dir's own login (log in with CLAUDE_CONFIG_DIR set to it) or "
            "with provider variables exported in this shell; authentication "
            "in user settings, such as apiKeyHelper or an env block, does "
            "not load."
        )
    if _denied(outcome, {"Read"}):
        return (
            "The authentication probe was denied its Read of the citing-once "
            f"rule, so the scoped grant Read(/{outcome.rule.parent}/**) never "
            "matches and every session would be denied it too."
        )
    if _denied(outcome, {"Write", "Edit"}) or (
        outcome.exit_status == 0 and not failed and not outcome.wrote_probe
    ):
        return (
            f"The authentication probe could not write {PROBE_FILE}: "
            "--permission-mode acceptEdits did not approve a write in this "
            "untrusted folder, which Claude Code has handled differently "
            "since 2.1.281."
        )
    return (
        f"The authentication probe failed: {' '.join(outcome.command)} "
        f"exited {outcome.exit_status}."
    )


def authentication_refusal(
    plugin_root: Path,
    evals: Path,
    flags: Sequence[str],
    env: dict[str, str],
) -> str | None:
    """Run one session's turn in a copy of the base project, and judge it.

    The turn reads the citing-once rule and writes a file, as the
    sessions do, so a grant or permission mode that cannot work stops the
    run before it costs anything.
    """
    rule = plugin_root / RULE
    with tempfile.TemporaryDirectory(prefix="ask-report-probe-") as scratch:
        directory = Path(scratch)
        root = directory / "project"
        build_project(Setup(variant="base"), root, evals)
        prepare_repository(root)
        prompt = (
            f"Read {rule} with the Read tool, then write the word ok to "
            f"{PROBE_FILE} in the current directory with the Write tool. "
            "Do nothing else."
        )
        command = opening_turn(prompt, str(uuid.uuid4()), flags)
        log = directory / "probe.jsonl"
        exit_status = run_session(command, root, log, env)
        return probe_refusal(
            ProbeOutcome(
                command=command,
                exit_status=exit_status,
                events=tuple(events(log.read_text())),
                stderr=log.with_suffix(".err").read_text(),
                wrote_probe=(root / PROBE_FILE).exists(),
                rule=rule,
            )
        )
