"""The commands the research guard permits, read from their Rust definitions.

The Rust constants are the single source: a test naming a guarded command
reads it from here rather than copying the literal.
"""

import re
from pathlib import Path
from typing import Literal, NamedTuple

CONFINEMENT = (
    Path(__file__).resolve().parents[2] / "cli/research/src/confinement.rs"
)


class GuardedCommand(NamedTuple):
    form: Literal["Exactly", "Prefixed"]
    text: str


def permitted_command(name: str, source: str | None = None) -> GuardedCommand:
    """The `PermittedCommand` constant `name`, however rustfmt wraps it."""
    text = CONFINEMENT.read_text(encoding="utf-8") if source is None else source
    match = re.search(
        rf"const {name}: PermittedCommand\s*=\s*"
        r'PermittedCommand::(Exactly|Prefixed)\(\s*"([^"]+)",?\s*\);',
        text,
    )
    assert match, f"confinement.rs no longer defines {name}"
    form: Literal["Exactly", "Prefixed"] = (
        "Exactly" if match.group(1) == "Exactly" else "Prefixed"
    )
    return GuardedCommand(form, match.group(2))


def rule_covers(rule: str, command: str) -> bool:
    """Whether a `Bash(…)` allow rule covers `command`, as Claude Code matches
    it: exactly, or by a trailing ` *` whose prefix ends at a word boundary."""
    matched = re.fullmatch(r"Bash\((.+)\)", rule)
    if not matched:
        return False
    pattern = matched.group(1)
    if not pattern.endswith(" *"):
        return command == pattern
    prefix = pattern.removesuffix(" *")
    return command == prefix or command.startswith(f"{prefix} ")
