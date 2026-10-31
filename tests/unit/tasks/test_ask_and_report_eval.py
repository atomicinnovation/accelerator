import json

import pytest

from tasks.eval.ask_and_report import formats
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
        ToolCall(
            "Read", {"file_path": "/p/brief.md"}, by_subagent=False, id="t1"
        ),
        ToolResult("t1", "brief", is_error=False, by_subagent=False),
        ToolResult("t2", "x", is_error=False, by_subagent=True),
    ]


@pytest.mark.parametrize(
    ("written", "plain"),
    [
        ("**Coverage:** partial", "Coverage: partial"),
        ("`Coverage: partial`", "Coverage: partial"),
        ("> Coverage: *partial*", "Coverage: partial"),
        ("> > Coverage: __partial__", "Coverage: partial"),
        ("   Coverage: partial  ", "Coverage: partial"),
        ("* [ ] Item", "- [ ] Item"),
        ("rounds_covered stays", "rounds_covered stays"),
    ],
)
def test_a_line_is_read_without_its_markdown(written: str, plain: str) -> None:
    assert formats.normalised(written) == plain


def test_fence_lines_are_dropped_from_a_turn_s_lines() -> None:
    text = "Answer.\n\n```text\nCoverage: answered\n```\n"

    assert formats.lines(text) == ["Answer.", "Coverage: answered"]


def test_a_template_matches_its_filled_line_and_captures_unknowns() -> None:
    found = formats.matches(formats.COVERAGE, "Coverage: partial")

    assert found is not None
    assert found["answered_partial_unanswered"] == "partial"
    assert formats.matches(formats.COVERAGE, "Coverage: mostly") is None


def test_a_known_placeholder_must_match_its_value() -> None:
    line = "Run conduct llm-agent-planning, then synthesise llm-agent-planning."

    assert formats.matches(
        formats.CONDUCT_THEN_SYNTHESISE, line, slug="llm-agent-planning"
    )
    assert not formats.matches(
        formats.CONDUCT_THEN_SYNTHESISE, line, slug="other"
    )


def test_a_repeated_placeholder_matches_a_list_of_one_or_more() -> None:
    one = formats.matches(
        formats.PROPOSED_ITEM, "- [ ] Which GPUs? — profiles: web"
    )
    two = formats.matches(
        formats.PROPOSED_ITEM, "- [ ] Which GPUs? — profiles: web, openalex"
    )

    assert one is not None
    assert one["profile"] == "web"
    assert two is not None
    assert two["profile"] == "web, openalex"


def test_a_template_with_inline_code_matches_its_plain_line() -> None:
    line = (
        "Add the profile to the brief's source_profiles, or fix its name in "
        "the outline."
    )

    assert formats.matches(formats.HELD_BACK_RECOVERY, line)


def test_every_reply_prompt_is_one_line() -> None:
    assert all("\n" not in prompt for prompt in formats.PROMPTS.values())
