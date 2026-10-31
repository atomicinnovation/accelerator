"""Hold one multi-turn conversation with research-topic, turn by turn."""

import subprocess
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from pathlib import Path

from tasks.eval.ask_and_report.projects import SLUG
from tasks.eval.ask_and_report.turns import (
    TurnRecord,
    errored,
    save_snapshot,
    snapshot,
)
from tasks.eval.headless.claude import common_flags, run_session
from tasks.eval.headless.transcript import Event, events
from tasks.eval.toolchain import Toolchain

SKILL = "skills/research/research-topic/SKILL.md"
RULE = "skills/research/outputters/citing-once/SKILL.md"
UNUSED_GRANT = "Bash(accelerator research fetch *)"
NOT_RUN = "not-run"

type Runner = Callable[[list[str], Path, Path, dict[str, str]], int]


def _declared_grant(skill: str) -> list[str]:
    frontmatter = skill.split("\n---\n", 1)[0]
    declared = frontmatter.split("allowed-tools:\n", 1)[1]
    return [
        line.removeprefix("  - ")
        for line in declared.splitlines()
        if line.startswith("  - ")
    ]


def research_topic_grant(plugin_root: Path) -> tuple[str, ...]:
    """Grant the skill's declared rules less the fetch neither verb runs.

    It adds a ``Read`` of the citing-once rule, the one file outside the
    project either verb reads; the leading ``//`` makes its path absolute.
    """
    bash = [
        rule
        for rule in _declared_grant((plugin_root / SKILL).read_text())
        if rule != UNUSED_GRANT
    ]
    rule_directory = (plugin_root / RULE).parent
    return (*bash, f"Read(/{rule_directory}/**)", "Skill")


def session_flags(model: str | None) -> list[str]:
    """Load only project settings and no MCP server, on ``model`` if named."""
    flags = ["--setting-sources", "project", "--strict-mcp-config"]
    return [*flags, "--model", model] if model else flags


def turn_flags(plugin_root: Path, model: str | None) -> list[str]:
    grant = research_topic_grant(plugin_root)
    return [*common_flags(plugin_root, grant), *session_flags(model)]


def session_env(
    toolchain: Toolchain, config_dir: Path | None
) -> dict[str, str]:
    env = toolchain.session_env()
    return env | {"CLAUDE_CONFIG_DIR": str(config_dir)} if config_dir else env


def opening_prompt(verb: str, question: str) -> str:
    return f"/accelerator:research-topic {verb} {SLUG} {question}"


def opening_turn(prompt: str, session: str, flags: Sequence[str]) -> list[str]:
    return ["claude", "-p", prompt, "--session-id", session, *flags]


def resumed_turn(prompt: str, session: str, flags: Sequence[str]) -> list[str]:
    return ["claude", "-p", prompt, "--resume", session, *flags]


@dataclass(frozen=True)
class Conversation:
    """The turns one session sends, and where it runs and records them.

    ``writes`` holds each turn's declared set-relative write set, the
    opening turn's first, so it is one longer than ``replies``.
    """

    prompt: str
    replies: tuple[str, ...]
    writes: tuple[tuple[str, ...], ...]
    session_id: str
    root: Path
    directory: Path


def turn_file(directory: Path, number: int, suffix: str) -> Path:
    return directory / f"turn-{number}{suffix}"


def _run_turn(
    command: list[str],
    conversation: Conversation,
    number: int,
    env: dict[str, str],
    run: Runner,
) -> str:
    log = turn_file(conversation.directory, number, ".jsonl")
    try:
        return str(run(command, conversation.root, log, env))
    except subprocess.TimeoutExpired:
        return "timeout"


def run_conversation(
    conversation: Conversation,
    flags: Sequence[str],
    env: dict[str, str],
    stops_after: Callable[[TurnRecord], bool],
    run: Runner = run_session,
) -> list[TurnRecord]:
    """Send each turn in order, stopping at an errored or stopping turn.

    Every turn after a stop is recorded as not run, so its absence never
    reads as a missing log.
    """
    directory = conversation.directory
    initial = snapshot(conversation.root)
    save_snapshot(initial, turn_file(directory, 0, ".json"))
    prompts = (conversation.prompt, *conversation.replies)
    records: list[TurnRecord] = []
    earlier: list[tuple[Event, ...]] = []
    before = initial
    stopped = False
    for number, prompt in enumerate(prompts, start=1):
        if stopped:
            turn_file(directory, number, ".exit").write_text(NOT_RUN)
            continue
        build = opening_turn if number == 1 else resumed_turn
        command = build(prompt, conversation.session_id, flags)
        status = _run_turn(command, conversation, number, env, run)
        turn_file(directory, number, ".exit").write_text(status)
        after = snapshot(conversation.root)
        save_snapshot(after, turn_file(directory, number, ".json"))
        log = turn_file(directory, number, ".jsonl")
        record = TurnRecord(
            number=number,
            reply=None if number == 1 else prompt,
            writes=conversation.writes[number - 1],
            events=tuple(events(log.read_text())) if log.exists() else (),
            logged=log.exists(),
            exit_status=status,
            before=before,
            after=after,
            initial=initial,
            earlier=tuple(earlier),
        )
        records.append(record)
        earlier.append(record.events)
        before = after
        stopped = errored(record) is not None or stops_after(record)
    return records
