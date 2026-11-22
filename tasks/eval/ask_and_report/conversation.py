"""Hold one multi-turn conversation with research-topic, turn by turn."""

import subprocess
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from pathlib import Path

from tasks.eval.ask_and_report.layout import SLUG
from tasks.eval.ask_and_report.turns import (
    Exited,
    NotRun,
    TimedOut,
    TurnExit,
    TurnHistory,
    TurnRecord,
    errored,
    save_snapshot,
    snapshot,
    turn_file,
)
from tasks.eval.headless.claude import common_flags, run_session
from tasks.eval.toolchain import Toolchain
from tasks.shared.skill_parsing import frontmatter_allowed_tools

SKILL = "skills/research/research-topic/SKILL.md"
UNUSED_GRANT = "Bash(accelerator research fetch *)"
PLUGIN_ROOT = "${CLAUDE_PLUGIN_ROOT}"

type Runner = Callable[[list[str], Path, Path, dict[str, str]], int]


def _declared_grant(skill: Path) -> list[str]:
    try:
        return frontmatter_allowed_tools(skill.read_text())
    except ValueError as error:
        raise ValueError(f"{skill}: {error}") from error


def research_topic_grant(plugin_root: Path) -> tuple[str, ...]:
    """Grant the skill's declared rules less the fetch neither verb runs.

    ``--allowedTools`` expands no variable, so the plugin root is filled in
    as Claude Code fills it into the skill's own grant.
    """
    return tuple(
        rule.replace(PLUGIN_ROOT, str(plugin_root))
        for rule in _declared_grant(plugin_root / SKILL)
        if rule != UNUSED_GRANT
    )


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


def _run_turn(
    command: list[str],
    conversation: Conversation,
    number: int,
    env: dict[str, str],
    run: Runner,
) -> TurnExit:
    log = turn_file(conversation.directory, number, ".jsonl")
    try:
        return Exited(run(command, conversation.root, log, env))
    except subprocess.TimeoutExpired:
        return TimedOut()


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
    history = TurnHistory(directory, initial)
    stopped = False
    for number, prompt in enumerate(prompts, start=1):
        if stopped:
            turn_file(directory, number, ".exit").write_text(
                NotRun().recorded()
            )
            continue
        build = opening_turn if number == 1 else resumed_turn
        command = build(prompt, conversation.session_id, flags)
        turn_exit = _run_turn(command, conversation, number, env, run)
        turn_file(directory, number, ".exit").write_text(turn_exit.recorded())
        save_snapshot(
            snapshot(conversation.root), turn_file(directory, number, ".json")
        )
        record = history.record(
            number,
            reply=None if number == 1 else prompt,
            writes=conversation.writes[number - 1],
            turn_exit=turn_exit,
        )
        stopped = errored(record) is not None or stops_after(record)
    return history.records
