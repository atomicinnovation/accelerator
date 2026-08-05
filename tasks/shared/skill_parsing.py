"""Pure SKILL.md parsing primitives, shared by every guard that reads one.

`covered_by` models Claude Code's own `Bash(...)` rule matcher. Both the
permissions lint rule and the release-gating dispatch guard now depend on that
model agreeing with the real one, so a change here changes what the release
considers an authorised command.

This module depends only on `re` and `fnmatch`. Keeping it a leaf is what lets
`tasks/shared/` consume the parsing without importing `invoke` or `tasks.lint`.
"""

import fnmatch
import re

_BASH_RULE = re.compile(r"Bash\(([^)]*)\)")
_PREPROCESSOR = re.compile(r"!`([^`]*)`")
_BARE_BASH_LINE = re.compile(r"^\s*-?\s*Bash\s*$")
_NAME_LINE = re.compile(r'^name:\s*"?([^"\n]*?)"?\s*$')
_METACHARACTERS = ("&&", "||", ";", "|", "$(", "`", "<(", ">(")
# Public: the conformance suite substitutes this prefix the way Claude Code
# does, and a second literal copy could desynchronise from this guard.
PLUGIN_PREFIX = "${CLAUDE_PLUGIN_ROOT}/"
LAUNCHER = f"{PLUGIN_PREFIX}bin/accelerator"
# A launcher command naming no subcommand — any rule matching it is too broad.
# The sentinel argument is load-bearing: `covered_by` appends `*` to a rule that
# lacks one, so a bare `{LAUNCHER}` probe matches even a correctly scoped rule.
BARE_LAUNCHER = f"{LAUNCHER} zz-external-subcommand-zz"


def _frontmatter_lines(text: str) -> list[str]:
    """Return the frontmatter body lines (between the two ``---`` fences)."""
    lines = text.splitlines()
    if not lines or lines[0] != "---":
        return []
    out: list[str] = []
    for line in lines[1:]:
        if line == "---":
            break
        out.append(line)
    return out


def frontmatter_bash_rules(text: str) -> list[str]:
    """Every ``Bash(...)`` rule inner declared in the frontmatter."""
    rules: list[str] = []
    for line in _frontmatter_lines(text):
        rules.extend(_BASH_RULE.findall(line))
    return rules


def has_bare_bash(text: str) -> bool:
    """Return whether the frontmatter declares a bare ``Bash`` tool."""
    return any(_BARE_BASH_LINE.match(line) for line in _frontmatter_lines(text))


def frontmatter_name(text: str) -> str:
    """Return the frontmatter ``name:`` value (quotes stripped), else empty."""
    for line in _frontmatter_lines(text):
        match = _NAME_LINE.match(line)
        if match:
            return match.group(1)
    return ""


def preprocessor_commands(text: str) -> list[str]:
    """Every ``!``-preprocessor command body, in document order."""
    return _PREPROCESSOR.findall(text)


def is_plugin_invocation(command: str) -> bool:
    """Return whether a command invokes a plugin script or the launcher."""
    return command.startswith(PLUGIN_PREFIX)


def covered_by(command: str, pattern: str) -> bool:
    """Return whether ``command`` matches rule ``pattern`` as a prefix glob.

    A rule not ending in ``*`` still matches the command plus trailing
    arguments; ``*`` spans ``/``, matching the verified matcher semantics.
    """
    glob = pattern if pattern.endswith("*") else pattern + "*"
    return fnmatch.fnmatchcase(command, glob)


def has_metacharacter(command: str) -> bool:
    """Return whether the command holds a metacharacter the matcher misses."""
    return any(token in command for token in _METACHARACTERS)


def launcher_token(text: str) -> str:
    """Return the subcommand token in a launcher command or rule, else empty.

    Applied to both, so a rule's token segment is extracted by exactly the code
    that extracts an invocation's — which is what lets the two be compared for
    equality rather than by glob.
    """
    if not text.startswith(LAUNCHER):
        return ""
    tail = text[len(LAUNCHER) :]
    # The prefix match must be followed by a separator: a sibling binary whose
    # name continues `accelerator` would otherwise yield a token spliced out of
    # the middle of its filename.
    if not tail.startswith(" "):
        return ""
    parts = tail.split()
    if not parts or parts[0].startswith("-"):
        return ""
    return parts[0]
