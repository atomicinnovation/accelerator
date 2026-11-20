"""What a session's stream-json log shows it said, called and got back."""

import json
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class Text:
    text: str
    by_subagent: bool


@dataclass(frozen=True)
class ToolCall:
    name: str
    input: dict[str, Any]
    by_subagent: bool
    id: str = ""

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


@dataclass(frozen=True)
class ToolResult:
    tool_use_id: str
    content: str
    is_error: bool
    by_subagent: bool


@dataclass(frozen=True)
class Init:
    session_id: str
    model: str
    plugins: tuple[tuple[str, str], ...]


@dataclass(frozen=True)
class Result:
    is_error: bool
    permission_denials: tuple[dict[str, Any], ...]
    text: str


type Event = Text | ToolCall | ToolResult | Init | Result


def _records(log: str) -> Iterator[dict[str, Any]]:
    for line in log.splitlines():
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict):
            yield record


def _flattened(content: object) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(
            str(block.get("text", ""))
            for block in content
            if isinstance(block, dict)
        )
    return ""


def _blocks(record: dict[str, Any]) -> list[dict[str, Any]]:
    message = record.get("message")
    if not isinstance(message, dict):
        return []
    content = message.get("content")
    if not isinstance(content, list):
        return []
    return [block for block in content if isinstance(block, dict)]


def _session_event(record: dict[str, Any]) -> Init | Result | None:
    match record.get("type"), record.get("subtype"):
        case "system", "init":
            return Init(
                session_id=record.get("session_id", ""),
                model=record.get("model", ""),
                plugins=tuple(
                    (plugin.get("name", ""), plugin.get("path", ""))
                    for plugin in record.get("plugins", [])
                ),
            )
        case "result", _:
            return Result(
                is_error=bool(record.get("is_error")),
                permission_denials=tuple(
                    record.get("permission_denials") or ()
                ),
                text=str(record.get("result", "")),
            )
    return None


def events(log: str) -> list[Event]:
    found: list[Event] = []
    for record in _records(log):
        if session_event := _session_event(record):
            found.append(session_event)
            continue
        by_subagent = record.get("parent_tool_use_id") is not None
        for block in _blocks(record):
            match record.get("type"), block.get("type"):
                case "assistant", "text":
                    found.append(Text(block.get("text", ""), by_subagent))
                case "assistant", "tool_use":
                    found.append(
                        ToolCall(
                            name=block["name"],
                            input=block.get("input", {}),
                            by_subagent=by_subagent,
                            id=block.get("id", ""),
                        )
                    )
                case "user", "tool_result":
                    found.append(
                        ToolResult(
                            tool_use_id=block.get("tool_use_id", ""),
                            content=_flattened(block.get("content")),
                            is_error=bool(block.get("is_error")),
                            by_subagent=by_subagent,
                        )
                    )
    return found


def tool_calls(log: str) -> list[ToolCall]:
    return [event for event in events(log) if isinstance(event, ToolCall)]
