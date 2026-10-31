"""The tool calls a session's stream-json log shows its carrier made."""

from pathlib import Path

from tasks.eval.headless.transcript import ToolCall

SPAWNS = frozenset({"Agent", "Task"})
COMPOSER = "accelerator:composer"


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
