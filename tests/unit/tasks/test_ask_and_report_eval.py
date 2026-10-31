import json

from tasks.eval.headless.transcript import (
    Text,
    ToolCall,
    ToolResult,
    events,
)


def _assistant(*blocks: dict, parent: str | None = None) -> str:
    return json.dumps(
        {
            "type": "assistant",
            "parent_tool_use_id": parent,
            "message": {"content": list(blocks)},
        }
    )


def _user(*blocks: dict, parent: str | None = None) -> str:
    return json.dumps(
        {
            "type": "user",
            "parent_tool_use_id": parent,
            "message": {"content": list(blocks)},
        }
    )


def test_a_transcript_reads_text_calls_and_results_in_order() -> None:
    log = "\n".join(
        [
            _assistant({"type": "text", "text": "Reading."}),
            _assistant(
                {
                    "type": "tool_use",
                    "id": "t1",
                    "name": "Read",
                    "input": {"file_path": "/p/brief.md"},
                }
            ),
            _user(
                {
                    "type": "tool_result",
                    "tool_use_id": "t1",
                    "content": [{"type": "text", "text": "brief"}],
                }
            ),
            _user(
                {"type": "tool_result", "tool_use_id": "t2", "content": "x"},
                parent="spawn",
            ),
            "not json",
        ]
    )

    assert events(log) == [
        Text("Reading.", by_subagent=False),
        ToolCall("Read", {"file_path": "/p/brief.md"}, False, id="t1"),
        ToolResult("t1", "brief", is_error=False, by_subagent=False),
        ToolResult("t2", "x", is_error=False, by_subagent=True),
    ]
