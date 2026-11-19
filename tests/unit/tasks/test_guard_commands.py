import pytest

from tests.support.guard_commands import (
    GuardedCommand,
    permitted_command,
    rule_covers,
)

FINDING = "accelerator config template topic-research --kind finding"
LEVEL_NOTE = "accelerator config template topic-research --kind level-note"
NODE_QUERY = "accelerator research topic node s --run r --node 03-a-web:2-1"


def test_permitted_command_reads_each_form() -> None:
    assert permitted_command("FETCH") == GuardedCommand(
        "Prefixed", "accelerator research fetch "
    )
    assert permitted_command("FINDING_TEMPLATE") == GuardedCommand(
        "Exactly", FINDING
    )
    assert permitted_command("LEVEL_NOTE_TEMPLATE") == GuardedCommand(
        "Exactly", LEVEL_NOTE
    )


def test_permitted_command_reads_a_wrapped_constant() -> None:
    source = """
pub const WRAPPED: PermittedCommand =
    PermittedCommand::Exactly(
        "accelerator config template x",
    );
"""
    assert permitted_command("WRAPPED", source) == GuardedCommand(
        "Exactly", "accelerator config template x"
    )


def test_permitted_command_fails_for_an_absent_constant() -> None:
    with pytest.raises(AssertionError, match="ABSENT"):
        permitted_command("ABSENT")


@pytest.mark.parametrize(
    ("rule", "command", "covered"),
    [
        ("Bash(accelerator config *)", FINDING, True),
        ("Bash(accelerator config *)", LEVEL_NOTE, True),
        ("Bash(accelerator conf *)", FINDING, False),
        ("Bash(accelerator research topic *)", NODE_QUERY, True),
        (f"Bash({FINDING})", FINDING, True),
        (f"Bash({FINDING})", LEVEL_NOTE, False),
        (f"Bash({FINDING})", f"{FINDING} --fail-safe", False),
        ("Bash(accelerator config *)", "accelerator config", True),
        ("Bash(accelerator config *)", "accelerator configx a", False),
        ("Read(accelerator config *)", FINDING, False),
    ],
)
def test_rule_covers(rule: str, command: str, covered: bool) -> None:
    assert rule_covers(rule, command) is covered
