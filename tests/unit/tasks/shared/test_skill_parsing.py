"""Contract tests for the shared SKILL.md parsing primitives.

These functions back two independent guards — the permissions lint rule and the
release-gating dispatch guard — so their behaviour is pinned directly rather
than only through whichever guard happens to exercise it.
"""

import re

import pytest

from tasks.shared.skill_parsing import (
    _METACHARACTERS,
    BARE_LAUNCHER,
    LAUNCHER,
    PLUGIN_PREFIX,
    covered_by,
    frontmatter_bash_rules,
    frontmatter_name,
    has_bare_bash,
    has_metacharacter,
    is_plugin_invocation,
    launcher_token,
    preprocessor_commands,
)
from tasks.shared.sources import repo_root

_VISUALISE = f"{LAUNCHER} visualiser start --port 4173"
_MODULE = repo_root() / "tasks/shared/skill_parsing.py"


@pytest.mark.parametrize(
    ("command", "rule", "expected"),
    [
        (_VISUALISE, f"{LAUNCHER} visualiser *", True),
        (_VISUALISE, f"{LAUNCHER} visualiser start --port 4173", True),
        # A rule without a trailing `*` is silently widened, so it still
        # matches the command plus trailing arguments.
        (_VISUALISE, f"{LAUNCHER} visualiser", True),
        (_VISUALISE, f"{LAUNCHER} config *", False),
        # `*` spans `/`, unlike a shell glob.
        (_VISUALISE, f"{PLUGIN_PREFIX}*", True),
        (_VISUALISE, f"{PLUGIN_PREFIX}bin/*", True),
        # fnmatch's `?`, `[seq]` and `[!seq]` classes are honoured, which is
        # why the dispatch guard cannot rely on `*` alone to spot a wildcard.
        (_VISUALISE, f"{LAUNCHER} ?isualiser *", True),
        (_VISUALISE, f"{LAUNCHER} [a-y]*", True),
        (_VISUALISE, f"{LAUNCHER} [!a-y]*", False),
        # Path-alias and quoting forms evade the matcher entirely: a rule
        # written this way covers nothing, and a command written this way is
        # covered by nothing. Recorded, not closed.
        (_VISUALISE, f"{PLUGIN_PREFIX}bin/../bin/accelerator *", False),
        (
            f"{PLUGIN_PREFIX}bin/../bin/accelerator visualiser",
            f"{LAUNCHER} *",
            False,
        ),
        (_VISUALISE, f"{PLUGIN_PREFIX}./bin/accelerator *", False),
        (_VISUALISE, f"{PLUGIN_PREFIX}/bin/accelerator *", False),
        (_VISUALISE, f'"{LAUNCHER}" *', False),
    ],
)
def test_covered_by_matcher_contract(
    command: str, rule: str, expected: bool
) -> None:
    assert covered_by(command, rule) is expected


def test_bare_launcher_is_covered_by_an_ancestor_glob() -> None:
    for rule in (f"{PLUGIN_PREFIX}*", f"{PLUGIN_PREFIX}bin/*", f"{LAUNCHER} *"):
        assert covered_by(BARE_LAUNCHER, rule)


def test_bare_launcher_is_not_covered_by_a_scoped_rule() -> None:
    assert not covered_by(BARE_LAUNCHER, f"{LAUNCHER} visualiser *")
    # The sentinel starts with `zz`, so a wildcarded token segment below `z`
    # evades it — which is why the dispatch guard needs a charset check too.
    assert not covered_by(BARE_LAUNCHER, f"{LAUNCHER} [a-y]*")


@pytest.mark.parametrize(
    ("text", "expected"),
    [
        (_VISUALISE, "visualiser"),
        (f"{LAUNCHER} visualiser", "visualiser"),
        (f"{LAUNCHER} visualiser *", "visualiser"),
        (f"{LAUNCHER} v*", "v*"),
        (f"{LAUNCHER} [a-y]*", "[a-y]*"),
        (f"{LAUNCHER} --version", ""),
        (f"{LAUNCHER}", ""),
        (f"{LAUNCHER} ", ""),
        # A sibling binary continuing `accelerator` without a separator must
        # not yield a token spliced out of the middle of its filename.
        (f"{PLUGIN_PREFIX}bin/accelerator-verify-darwin-arm64 x", ""),
        (f"{PLUGIN_PREFIX}bin/accelerator-verify *", ""),
        ("echo hello", ""),
    ],
)
def test_launcher_token(text: str, expected: str) -> None:
    assert launcher_token(text) == expected


def test_frontmatter_bash_rules_finds_every_rule() -> None:
    text = (
        "---\n"
        "name: demo\n"
        "allowed-tools: Bash(one), Bash(two)\n"
        "  - Bash(three)\n"
        "---\n"
        "Bash(not-in-frontmatter)\n"
    )
    assert frontmatter_bash_rules(text) == ["one", "two", "three"]


def test_frontmatter_bash_rules_needs_an_opening_fence() -> None:
    assert frontmatter_bash_rules("no frontmatter here\nBash(one)\n") == []


def test_an_unterminated_fence_consumes_the_rest_of_the_file() -> None:
    text = "---\nname: demo\nallowed-tools: Bash(one)\n\nBash(two)\n"
    assert frontmatter_bash_rules(text) == ["one", "two"]


@pytest.mark.parametrize(
    ("line", "expected"),
    [
        ("  - Bash", True),
        ("- Bash", True),
        ("Bash", True),
        ("  - Bash ", True),
        ("  - Bash(x)", False),
        ("  - BashTool", False),
    ],
)
def test_has_bare_bash(line: str, expected: bool) -> None:
    assert has_bare_bash(f"---\nname: demo\n{line}\n---\nbody\n") is expected


@pytest.mark.parametrize(
    ("line", "expected"),
    [
        ("name: demo", "demo"),
        ('name: "demo"', "demo"),
        ("name:    demo   ", "demo"),
        ("title: demo", ""),
    ],
)
def test_frontmatter_name(line: str, expected: str) -> None:
    assert frontmatter_name(f"---\n{line}\n---\nbody\n") == expected


def test_preprocessor_commands_are_returned_in_document_order() -> None:
    text = "!`first one`\n\nprose\n\n!`second one`\n"
    assert preprocessor_commands(text) == ["first one", "second one"]


def test_is_plugin_invocation() -> None:
    assert is_plugin_invocation(_VISUALISE)
    assert is_plugin_invocation(f"{PLUGIN_PREFIX}scripts/thing.sh")
    assert not is_plugin_invocation(f"cd . && {_VISUALISE}")


@pytest.mark.parametrize("metacharacter", _METACHARACTERS)
def test_has_metacharacter(metacharacter: str) -> None:
    assert has_metacharacter(f"{_VISUALISE} {metacharacter} rm")
    assert not has_metacharacter(_VISUALISE)


def test_the_parsing_leaf_imports_nothing_but_re_and_fnmatch() -> None:
    # The whole point of the module: a guard in `tasks/shared/` can consume it
    # without dragging in `invoke`, `tasks.lint` or thence `tasks.build`.
    imports = set(
        re.findall(r"^(?:from|import) (\S+)", _MODULE.read_text(), re.MULTILINE)
    )
    assert imports == {"fnmatch", "re"}
