"""The tool calls a session's stream-json log shows its carrier made."""

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

SPAWNS = frozenset({"Agent", "Task"})
COMPOSER = "accelerator:composer"


@dataclass(frozen=True)
class ToolCall:
    name: str
    input: dict[str, Any]
    by_subagent: bool

    def reads(self, path: Path) -> bool:
        target = self.input.get("file_path")
        return (
            self.name == "Read"
            and isinstance(target, str)
            and Path(target).resolve() == path.resolve()
        )

    def writes(self, filename: str) -> bool:
        target = self.input.get("file_path")
        return (
            self.name in {"Write", "Edit"}
            and isinstance(target, str)
            and Path(target).name == filename
        )


def tool_calls(log: str) -> list[ToolCall]:
    calls = []
    for line in log.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(event, dict) or event.get("type") != "assistant":
            continue
        calls.extend(
            ToolCall(
                name=block["name"],
                input=block.get("input", {}),
                by_subagent=event.get("parent_tool_use_id") is not None,
            )
            for block in event.get("message", {}).get("content", [])
            if block.get("type") == "tool_use"
        )
    return calls


def composer_log_failures(calls: list[ToolCall], rule: Path) -> list[str]:
    failures = []
    if not any(
        call.name in SPAWNS and call.input.get("subagent_type") == COMPOSER
        for call in calls
    ):
        failures.append("the composer was never spawned")
    if not any(call.by_subagent and call.reads(rule) for call in calls):
        failures.append("the composer never read the rule")
    if any(call.name == "Bash" for call in calls):
        failures.append("a Bash call was made")
    return failures


def synthesise_log_failures(calls: list[ToolCall], rule: Path) -> list[str]:
    failures = []
    rule_file = f"{rule.parent.name}/{rule.name}"
    if any(
        call.name == "Bash" and rule_file in str(call.input.get("command"))
        for call in calls
    ):
        failures.append("the rule was read with Bash")
    written = next(
        (at for at, call in enumerate(calls) if call.writes("synthesis.md")),
        len(calls),
    )
    if not any(call.reads(rule) for call in calls[:written]):
        failures.append(
            "the rule was not read with Read before synthesis.md was written"
        )
    if any(call.name in SPAWNS for call in calls):
        failures.append("an agent was spawned")
    return failures
