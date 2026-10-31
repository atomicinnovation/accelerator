import json
import os
import subprocess
from pathlib import Path

import pytest

from tasks.eval.ask_and_report import conversation, formats, turns
from tasks.eval.headless.claude import common_flags
from tasks.eval.headless.transcript import (
    Init,
    Result,
    Text,
    ToolCall,
    ToolResult,
    events,
)
from tasks.eval.toolchain import Toolchain


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


def _init(plugin_path: str = "/repo", session: str = "s-1") -> str:
    return json.dumps(
        {
            "type": "system",
            "subtype": "init",
            "session_id": session,
            "model": "claude-test",
            "plugins": [
                {"name": "accelerator", "path": plugin_path},
                {"name": "cc-plugin-telemetry", "path": "builtin"},
            ],
        }
    )


def _result(*, is_error: bool = False, denials: tuple = (), text="") -> str:
    return json.dumps(
        {
            "type": "result",
            "is_error": is_error,
            "permission_denials": list(denials),
            "result": text,
        }
    )


def test_a_transcript_reads_the_init_and_result_events() -> None:
    denial = {"tool_name": "Bash", "tool_input": {"command": "ls"}}

    assert events(
        "\n".join([_init(), _result(denials=(denial,), text="done")])
    ) == [
        Init(
            session_id="s-1",
            model="claude-test",
            plugins=(
                ("accelerator", "/repo"),
                ("cc-plugin-telemetry", "builtin"),
            ),
        ),
        Result(is_error=False, permission_denials=(denial,), text="done"),
    ]


def test_a_snapshot_covers_every_file_but_git(tmp_path: Path) -> None:
    (tmp_path / ".git").mkdir()
    (tmp_path / ".git/HEAD").write_text("ref")
    (tmp_path / ".claude").mkdir()
    (tmp_path / ".claude/settings.json").write_text("{}")
    (tmp_path / "a.md").write_text("a")

    assert sorted(turns.snapshot(tmp_path)) == [".claude/settings.json", "a.md"]


def test_changed_lists_added_removed_and_modified_paths(tmp_path: Path) -> None:
    (tmp_path / "kept.md").write_text("k")
    (tmp_path / "edited.md").write_text("1")
    (tmp_path / "removed.md").write_text("r")
    before = turns.snapshot(tmp_path)
    (tmp_path / "edited.md").write_text("2")
    (tmp_path / "removed.md").unlink()
    (tmp_path / "added.md").write_text("n")

    assert turns.changed(before, turns.snapshot(tmp_path)) == [
        "added.md",
        "edited.md",
        "removed.md",
    ]


def test_a_snapshot_round_trips_through_its_file(tmp_path: Path) -> None:
    (tmp_path / "p").mkdir()
    (tmp_path / "p/a.md").write_text("text")
    taken = turns.snapshot(tmp_path / "p")

    turns.save_snapshot(taken, tmp_path / "turn-0.json")

    assert turns.load_snapshot(tmp_path / "turn-0.json") == taken
    assert taken["a.md"].text == "text"


def _record(log: str | None, exit_status: str | None = "0") -> turns.TurnRecord:
    return turns.TurnRecord(
        number=1,
        reply=None,
        writes=(),
        events=tuple(events(log)) if log is not None else (),
        logged=log is not None,
        exit_status=exit_status,
        before={},
        after={},
        initial={},
        earlier=(),
    )


@pytest.mark.parametrize(
    ("log", "exit_status", "reason"),
    [
        (_init(), "1", "exit code 1"),
        (_init(), "timeout", "timeout"),
        (None, "0", "no log"),
        (_init(), None, "no exit status"),
        (_result(), "0", "no init event"),
        ("\n".join([_init(), _result(is_error=True)]), "0", "is_error"),
    ],
)
def test_an_infrastructure_failure_errors_the_turn(
    log: str | None, exit_status: str | None, reason: str
) -> None:
    assert turns.errored(_record(log, exit_status)) == reason


def test_a_turn_with_no_text_is_not_errored() -> None:
    assert turns.errored(_record(f"{_init()}\n{_result()}")) is None


def test_the_main_text_leaves_out_subagent_text() -> None:
    record = _record(
        "\n".join(
            [
                _init(),
                _assistant({"type": "text", "text": "Mine."}),
                _assistant({"type": "text", "text": "Theirs."}, parent="x"),
                _assistant({"type": "text", "text": "Also mine."}),
            ]
        )
    )

    assert record.text == "Mine.\nAlso mine."


REPO_ROOT = Path(__file__).resolve().parents[3]
GRANT = (
    "Bash(accelerator config *),"
    "Bash(accelerator corpus resolve *),"
    "Bash(accelerator corpus metadata derive),"
    "Bash(accelerator corpus frontmatter validate *),"
    "Bash(accelerator research topic *),"
    "Read(/<plugin_root>/skills/research/outputters/citing-once/**),"
    "Skill"
)


def _flag(command: list[str], name: str) -> str:
    return command[command.index(name) + 1]


def test_the_grant_is_the_skill_s_less_fetch_plus_the_rule_and_skill() -> None:
    grant = ",".join(conversation.research_topic_grant(REPO_ROOT))

    assert grant == GRANT.replace("<plugin_root>", str(REPO_ROOT))


def test_every_turn_passes_the_common_and_session_flags() -> None:
    flags = conversation.turn_flags(REPO_ROOT, model=None)
    common = common_flags(
        REPO_ROOT, conversation.research_topic_grant(REPO_ROOT)
    )

    assert flags == [
        *common,
        "--setting-sources",
        "project",
        "--strict-mcp-config",
    ]
    assert "--model" not in flags


def test_a_model_reaches_every_turn() -> None:
    flags = conversation.turn_flags(REPO_ROOT, model="claude-test")

    assert _flag(flags, "--model") == "claude-test"


def test_the_opening_turn_names_the_session_and_later_turns_resume_it() -> None:
    flags = conversation.turn_flags(REPO_ROOT, model=None)
    opening = conversation.opening_turn("hello", "s-1", flags)
    resumed = conversation.resumed_turn("accept", "s-1", flags)

    assert opening[:3] == ["claude", "-p", "hello"]
    assert _flag(opening, "--session-id") == "s-1"
    assert "--resume" not in opening
    assert resumed[:3] == ["claude", "-p", "accept"]
    assert _flag(resumed, "--resume") == "s-1"
    assert "--session-id" not in resumed
    for command in (opening, resumed):
        assert "--no-session-persistence" not in command
        assert _flag(command, "--allowedTools") == ",".join(
            conversation.research_topic_grant(REPO_ROOT)
        )


def test_the_opening_prompt_invokes_the_verb_on_the_reference_set() -> None:
    assert conversation.opening_prompt("ask", "Why?") == (
        "/accelerator:research-topic ask llm-agent-planning Why?"
    )


def test_a_config_dir_reaches_the_session_env(tmp_path: Path) -> None:
    toolchain = Toolchain.debug(tmp_path)

    env = conversation.session_env(toolchain, tmp_path / "config")
    inherited = conversation.session_env(toolchain, None)

    assert env["CLAUDE_CONFIG_DIR"] == str(tmp_path / "config")
    assert env["ACCELERATOR_LAUNCHER_BIN"] == str(toolchain.launcher)
    assert inherited.get("CLAUDE_CONFIG_DIR") == os.environ.get(
        "CLAUDE_CONFIG_DIR"
    )


class FakeClaude:
    """Run each turn by writing a distinct file and a canned log."""

    def __init__(self, exits: dict[int, int | str] | None = None) -> None:
        self.commands: list[list[str]] = []
        self.exits = exits or {}

    def __call__(
        self, command: list[str], root: Path, log: Path, env: dict[str, str]
    ) -> int:
        self.commands.append(command)
        turn = len(self.commands)
        (root / f"wrote-{turn}.md").write_text(str(turn))
        log.write_text(_init() + "\n")
        log.with_suffix(".err").write_text("")
        outcome = self.exits.get(turn, 0)
        if outcome == "timeout":
            raise subprocess.TimeoutExpired(command, 1)
        return int(outcome)


def _conversation(
    tmp_path: Path, replies: int = 2
) -> conversation.Conversation:
    root = tmp_path / "project"
    root.mkdir()
    return conversation.Conversation(
        prompt="/accelerator:research-topic ask llm-agent-planning Why?",
        replies=tuple(["reject", "decline"][:replies]),
        writes=((), (), ())[: replies + 1],
        session_id="s-1",
        root=root,
        directory=tmp_path,
    )


def _run(
    tmp_path: Path,
    fake: FakeClaude,
    stops_after=lambda record: False,
    replies: int = 2,
) -> list[turns.TurnRecord]:
    return conversation.run_conversation(
        _conversation(tmp_path, replies),
        flags=["--flag"],
        env={},
        stops_after=stops_after,
        run=fake,
    )


def test_replies_go_out_in_order_on_one_session(tmp_path: Path) -> None:
    fake = FakeClaude()

    records = _run(tmp_path, fake)

    assert [command[2] for command in fake.commands] == [
        "/accelerator:research-topic ask llm-agent-planning Why?",
        "reject",
        "decline",
    ]
    assert _flag(fake.commands[0], "--session-id") == "s-1"
    assert [_flag(c, "--resume") for c in fake.commands[1:]] == ["s-1", "s-1"]
    assert [record.reply for record in records] == [None, "reject", "decline"]


def test_each_turn_s_snapshot_is_taken_after_it(tmp_path: Path) -> None:
    _run(tmp_path, FakeClaude())

    assert "wrote-1.md" not in turns.load_snapshot(tmp_path / "turn-0.json")
    for turn in (1, 2, 3):
        after = turns.load_snapshot(tmp_path / f"turn-{turn}.json")
        assert f"wrote-{turn}.md" in after
        assert f"wrote-{turn + 1}.md" not in after


def test_a_turn_s_exit_status_is_written_as_it_ends(tmp_path: Path) -> None:
    seen: list[str] = []

    def stops_after(record: turns.TurnRecord) -> bool:
        seen.append((tmp_path / f"turn-{record.number}.exit").read_text())
        return False

    _run(tmp_path, FakeClaude(), stops_after)

    assert seen == ["0", "0", "0"]


@pytest.mark.parametrize("outcome", [1, "timeout"])
def test_an_errored_turn_ends_the_conversation(
    tmp_path: Path, outcome: int | str
) -> None:
    fake = FakeClaude(exits={2: outcome})

    records = _run(tmp_path, fake)

    assert len(fake.commands) == 2
    assert [record.exit_status for record in records] == ["0", str(outcome)]
    assert (tmp_path / "turn-2.exit").read_text() == str(outcome)
    assert (tmp_path / "turn-3.exit").read_text() == "not-run"
    assert not (tmp_path / "turn-3.jsonl").exists()


def test_a_turn_that_breaks_a_turn_check_ends_the_conversation(
    tmp_path: Path,
) -> None:
    fake = FakeClaude()

    _run(tmp_path, fake, stops_after=lambda record: record.number == 1)

    assert len(fake.commands) == 1
    for turn in (2, 3):
        assert (tmp_path / f"turn-{turn}.exit").read_text() == "not-run"


def test_a_record_carries_each_earlier_turn_s_events(tmp_path: Path) -> None:
    records = _run(tmp_path, FakeClaude())

    assert [len(record.earlier) for record in records] == [0, 1, 2]
    assert records[2].initial == turns.load_snapshot(tmp_path / "turn-0.json")
