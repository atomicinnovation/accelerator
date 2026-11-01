"""The checks every session meets, whatever its scenario expects.

A turn check is judged from one turn as it ends, so the turn loop can stop
a conversation at its first violation. The session check is judged once the
session ends.
"""

import shlex
from collections.abc import Iterator, Sequence
from dataclasses import dataclass
from pathlib import Path

from tasks.eval.ask_and_report import formats
from tasks.eval.ask_and_report.layout import SET
from tasks.eval.ask_and_report.turns import TurnRecord, changed
from tasks.eval.headless.transcript import (
    Event,
    Init,
    Result,
    Text,
    ToolCall,
    ToolResult,
)

RULE = "skills/research/outputters/citing-once/SKILL.md"
RESEARCH_TOPIC = "accelerator:research-topic"
CANARY = "Lanternfish"
SPAWNS = frozenset({"Agent", "Task"})
SEARCHES = frozenset({"Read", "Grep", "Glob"})
WRITES = frozenset({"Write", "Edit"})
GLOB_CHARACTERS = frozenset("*?[{")
ALLOWED_COMMANDS = (
    ("accelerator", "corpus", "resolve"),
    ("accelerator", "corpus", "metadata", "derive"),
    ("accelerator", "corpus", "frontmatter", "validate"),
    ("accelerator", "research", "topic", "consultable"),
    ("accelerator", "research", "topic", "counts"),
    ("accelerator", "research", "topic", "report-path"),
    ("accelerator", "config", "get"),
    ("accelerator", "config", "template"),
)


@dataclass(frozen=True)
class Bounds:
    """Where a session may read and write: its project, and the rule."""

    root: Path
    plugin_root: Path

    @property
    def rule(self) -> Path:
        return (self.plugin_root / RULE).resolve()

    def resolved(self, path: str) -> Path:
        return (self.root / path).resolve()

    def holds(self, path: Path) -> bool:
        return path.is_relative_to(self.root.resolve()) or path == self.rule


def _reach(search_root: str, pattern: str) -> str:
    """Find the highest directory a pattern can reach from its search root.

    A wildcard part is taken to match no directory, so a ``..`` after it
    climbs from the part before it, as far as the pattern can.
    """
    fixed = [
        part for part in Path(pattern).parts if not GLOB_CHARACTERS & set(part)
    ]
    return str(Path(search_root, *fixed))


def _named_paths(call: ToolCall) -> list[str]:
    keys = ("file_path", "path", "pattern", "glob")
    return [str(call.input[key]) for key in keys if key in call.input]


def _searched(call: ToolCall) -> list[str]:
    search_root = str(call.input.get("path", "."))
    match call.name:
        case "Read":
            return [str(call.input.get("file_path", ""))]
        case "Glob":
            pattern = str(call.input.get("pattern", ""))
            return [search_root, _reach(search_root, pattern)]
        case "Grep":
            reached = [search_root]
            if "glob" in call.input:
                reached.append(_reach(search_root, str(call.input["glob"])))
            return reached
    return []


def _search_failures(call: ToolCall, bounds: Bounds) -> Iterator[str]:
    if any(
        part.endswith(".levels")
        for named in _named_paths(call)
        for part in Path(named).parts
    ):
        yield f"reads under .levels: {call.input}"
    for searched in _searched(call):
        if not bounds.holds(bounds.resolved(searched)):
            yield f"reads outside the project: {searched}"


def _listed_level_notes(events: Sequence[Event]) -> Iterator[str]:
    searches = {
        event.id
        for event in events
        if isinstance(event, ToolCall) and event.name in {"Grep", "Glob"}
    }
    for event in events:
        if (
            isinstance(event, ToolResult)
            and event.tool_use_id in searches
            and ".levels/" in event.content
        ):
            yield "a search lists files under .levels"


def _expands_outside_single_quotes(command: str) -> bool:
    quote = None
    escaped = False
    for character in command:
        if escaped:
            escaped = False
        elif quote == "'":
            quote = None if character == "'" else quote
        elif character == "\\":
            escaped = True
        elif character in "$`":
            return True
        elif character in "'\"":
            quote = None if quote == character else quote or character
    return False


def _words(command: str) -> list[str] | None:
    """Split a command into words and operators, or None if it does not lex.

    Bash starts a comment only at the start of a word, so ``shlex`` must
    not read one anywhere else.
    """
    lexer = shlex.shlex(command, posix=True, punctuation_chars=True)
    lexer.commenters = ""
    lexer.whitespace_split = True
    try:
        return list(lexer)
    except ValueError:
        return None


def _command_failure(command: str) -> str | None:
    """Fail anything but one simple command the allow-list names.

    Only single quotes stop the shell expanding ``$`` and backticks, and
    Claude Code approves each part of a compound command on its own, so
    both are scanned before the command's words are read.
    """
    lines = [line for line in command.splitlines() if line.strip()]
    words = _words(command)
    if (
        len(lines) > 1
        or _expands_outside_single_quotes(command)
        or words is None
        or any(word and set(word) <= set("();<>|&") for word in words)
        or not any(
            tuple(words[: len(entry)]) == entry for entry in ALLOWED_COMMANDS
        )
    ):
        return f"runs a command off the allow-list: {command}"
    return None


def _call_failures(
    call: ToolCall, record: TurnRecord, bounds: Bounds
) -> Iterator[str]:
    if call.name in SEARCHES:
        yield from _search_failures(call, bounds)
    if call.name == "Skill":
        named = call.input.get("skill", call.input.get("command"))
        if named != RESEARCH_TOPIC:
            yield f"loads another skill: {named}"
    if call.name in SPAWNS:
        yield f"spawns an agent: {call.input.get('subagent_type')}"
    if call.name in WRITES:
        allowed = {(bounds.root / SET / w).resolve() for w in record.writes}
        target = str(call.input.get("file_path", ""))
        if bounds.resolved(target) not in allowed:
            yield f"writes outside its declared set: {target}"
    if call.name == "Bash" and (
        failure := _command_failure(str(call.input.get("command", "")))
    ):
        yield failure


def _event_failures(
    event: Event, record: TurnRecord, bounds: Bounds
) -> Iterator[str]:
    match event:
        case ToolCall():
            yield from _call_failures(event, record, bounds)
        case Text(text=said) | ToolResult(content=said) if CANARY in said:
            yield "repeats the level-note canary"
        case Init():
            own = [
                Path(path).resolve()
                for name, path in event.plugins
                if name == "accelerator"
            ]
            if own != [bounds.plugin_root.resolve()]:
                yield f"loads accelerator plugins from {own}, not this one"
        case Result() if event.permission_denials:
            names = [d.get("tool_name") for d in event.permission_denials]
            yield f"was denied {names}"


def _states_coverage(event: Event) -> bool:
    return (
        isinstance(event, Text)
        and not event.by_subagent
        and any(
            line.startswith("Coverage:") for line in formats.lines(event.text)
        )
    )


def _rule_reads(events: Sequence[Event], bounds: Bounds) -> set[int]:
    """Find the positions of each whole, successful Read of the rule."""
    failed = {
        event.tool_use_id
        for event in events
        if isinstance(event, ToolResult) and event.is_error
    }
    return {
        position
        for position, event in enumerate(events)
        if isinstance(event, ToolCall)
        and event.name == "Read"
        and event.id not in failed
        and not {"offset", "limit"} & event.input.keys()
        and bounds.resolved(str(event.input.get("file_path", "")))
        == bounds.rule
    }


def _coverage_before_the_rule(record: TurnRecord, bounds: Bounds) -> bool:
    earlier = [event for turn in record.earlier for event in turn]
    events = [*earlier, *record.events]
    first_read = min(_rule_reads(events, bounds), default=len(events))
    return any(
        _states_coverage(event) for event in events[len(earlier) : first_read]
    )


def turn_check_failures(record: TurnRecord, bounds: Bounds) -> list[str]:
    failures = [
        failure
        for event in record.events
        for failure in _event_failures(event, record, bounds)
    ]
    failures += _listed_level_notes(record.events)
    if _coverage_before_the_rule(record, bounds):
        failures.append("states coverage before reading the citing-once rule")
    set_prefix = f"{SET.as_posix()}/"
    failures += [
        f"changes outside the set: {path}"
        for path in changed(record.before, record.after)
        if not path.startswith(set_prefix)
    ]
    return failures


def breaks_a_turn_check(record: TurnRecord, bounds: Bounds) -> bool:
    return bool(turn_check_failures(record, bounds))


def session_check_failures(
    records: Sequence[TurnRecord], bounds: Bounds
) -> list[str]:
    every = [event for record in records for event in record.events]
    if any(map(_states_coverage, every)) and not _rule_reads(every, bounds):
        return ["states coverage but never reads the citing-once rule"]
    return []
