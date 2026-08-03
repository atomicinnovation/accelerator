"""Bind every dispatched sub-binary token to a consuming skill, both ways.

A shipped token with no skill invoking it resolves to an asset nobody fetches;
a skill invoking a token the producer never ships resolves to AssetNotFound at
run time. Both directions are checked here, against the same SKILL.md parsing
the permissions lint rule uses, so the two cannot drift to different notions of
an authorised invocation.
"""

import re
from collections.abc import Iterable
from pathlib import Path

from tasks.shared.errors import DispatchCoherenceError
from tasks.shared.paths import (
    DISPATCHED_SUBBINARIES,
    REPO_ROOT,
    SKILL_EXEMPT_SUBBINARIES,
)
from tasks.shared.skill_parsing import (
    BARE_LAUNCHER,
    LAUNCHER,
    covered_by,
    frontmatter_bash_rules,
    has_bare_bash,
    has_metacharacter,
    is_plugin_invocation,
    launcher_token,
    preprocessor_commands,
)

# Must equal the launcher's built-in set; a test pins it against the clap
# `Command` enum, which is not compile-enforced from this side.
BUILTIN_SUBCOMMANDS = frozenset({"version", "config", "help"})
# Staged-but-never-dispatched binaries whose asset name a token would collide
# with, plus `launcher`. Derived by test from _CLI_RELEASE_BINARIES minus the
# dispatched set: a third staged binary cannot silently become registrable, and
# a token whose own binary is staged there stays legal.
RESERVED_TOKENS = frozenset({"verify", "launcher"})
_TOKEN = re.compile(r"^[a-z][a-z0-9-]*$")


def _authorises(rules: list[str], *, token: str, command: str) -> bool:
    return any(
        launcher_token(rule) == token and covered_by(command, rule)
        for rule in rules
    )


def _is_over_broad(text: str, rules: list[str]) -> bool:
    return (
        has_bare_bash(text)
        or any(covered_by(BARE_LAUNCHER, rule) for rule in rules)
        or any(
            segment and not _TOKEN.match(segment)
            for segment in (launcher_token(rule) for rule in rules)
        )
    )


def _launcher_occurrences(command: str) -> Iterable[int]:
    index = command.find(LAUNCHER)
    while index != -1:
        yield index
        index = command.find(LAUNCHER, index + 1)


def _every_token(command: str) -> set[str]:
    tokens: set[str] = set()
    for index in _launcher_occurrences(command):
        token = launcher_token(command[index:])
        if token:
            tokens.add(token)
    return tokens


def _bindings(root: Path) -> tuple[set[str], dict[str, str]]:
    bound: set[str] = set()
    invoked: dict[str, str] = {}
    for path in sorted((root / "skills").rglob("SKILL.md")):
        text = path.read_text()
        rules = frontmatter_bash_rules(text)
        over_broad = _is_over_broad(text, rules)
        rel = path.relative_to(root).as_posix()
        for command in preprocessor_commands(text):
            for token in _every_token(command):
                invoked.setdefault(token, rel)
            if not is_plugin_invocation(command):
                continue
            # `skill_permissions` refuses to coverage-check a chained command,
            # so binding on one would let the two guards disagree.
            if has_metacharacter(command):
                continue
            token = launcher_token(command)
            if (
                token
                and not over_broad
                and _authorises(rules, token=token, command=command)
            ):
                bound.add(token)
    return bound, invoked


def _registry_problems(
    tokens: tuple[str, ...], exempt: tuple[str, ...]
) -> list[str]:
    if not tokens:
        return [
            "no dispatched sub-binaries resolved — DISPATCHED_SUBBINARIES was "
            "lost rather than deliberately emptied"
        ]
    problems = [
        f"{token}: not a valid token — must match {_TOKEN.pattern}, because it "
        "derives ACCELERATOR_<TOKEN>_BIN, which the launcher refuses to build "
        "from a name outside that set"
        for token in tokens
        if not _TOKEN.match(token)
    ]
    problems.extend(
        f"{token}: reserved — its staged asset name or default crate path "
        "collides with the launcher's or the verify shim's"
        for token in sorted(set(tokens) & RESERVED_TOKENS)
    )
    problems.extend(
        f"{token}: shadows a launcher built-in, so it would be signed and "
        "listed in the manifest but never dispatched"
        for token in sorted(set(tokens) & BUILTIN_SUBCOMMANDS)
    )
    problems.extend(
        f"{token}: exempt but not dispatched — either the token was dropped "
        "from DISPATCHED_SUBBINARIES or the exemption is stale"
        for token in sorted(set(exempt) - set(tokens))
    )
    if set(tokens) <= set(exempt):
        problems.append(
            "every dispatched sub-binary is exempt — this guard would check "
            "nothing; an exemption is for a token consumed only by a hook or "
            "another binary, not a way to silence a failure"
        )
    return problems


def violations(
    root: Path,
    *,
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
    exempt: Iterable[str] = SKILL_EXEMPT_SUBBINARIES,
) -> list[str]:
    """Every dispatch-coherence problem, in both directions.

    Registry problems short-circuit: a malformed constant makes the skills scan
    meaningless, so they are reported alone.

    A token is bound when at least one SKILL.md invokes `accelerator <token>`
    through the `!` preprocessor and carries a `Bash(...)` rule whose subcommand
    segment is exactly that token and which covers the invocation, in a skill
    that declares no bare `Bash` tool, no rule authorising the bare launcher and
    no rule with a wildcarded token segment.

    An exemption declares that no SKILL.md invokes the token; one that is
    invoked, or that names an undispatched token, or that covers every token, is
    itself a problem. So an exemption requires at least one non-exempt token.
    """
    names, exemptions = tuple(tokens), tuple(exempt)
    problems = _registry_problems(names, exemptions)
    if problems:
        return problems

    bound, invoked = _bindings(root)
    for token in names:
        # The exemption check precedes the `bound` short-circuit: an exemption
        # asserts that no SKILL.md invokes the token, so one that gained a real
        # binding must surface as stale rather than pass.
        if token in exemptions:
            if token in invoked:
                problems.append(
                    f"{token}: exempt but {invoked[token]} invokes "
                    f"`accelerator {token}` — an exemption is for a token no "
                    "SKILL.md invokes; drop the exemption"
                )
            continue
        if token in bound:
            continue
        if token not in invoked:
            problems.append(
                f"{token}: no skill invokes `accelerator {token}` through the "
                "`!` preprocessor — add a consuming skill, or an entry in "
                "SKILL_EXEMPT_SUBBINARIES if its only consumer is a hook"
            )
        else:
            problems.append(
                f"{token}: {invoked[token]} invokes `accelerator {token}` but "
                "declares no Bash(...) rule naming that subcommand — a bare "
                "`Bash` tool, a rule authorising the bare launcher, or a rule "
                "with a wildcarded token segment disqualifies the skill"
            )
    problems.extend(
        f"{token}: {invoked[token]} invokes `accelerator {token}`, which is "
        "neither dispatched nor a launcher built-in — rename the subcommand if "
        "it is reserved or invalid, otherwise add it to DISPATCHED_SUBBINARIES"
        for token in sorted(invoked)
        if token not in names and token not in BUILTIN_SUBCOMMANDS
    )
    return problems


def validate_dispatch_coherence(
    repo_root: Path | None = None,
    *,
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
    exempt: Iterable[str] = SKILL_EXEMPT_SUBBINARIES,
) -> None:
    """Raise if any dispatch-coherence problem exists. See violations()."""
    problems = violations(repo_root or REPO_ROOT, tokens=tokens, exempt=exempt)
    if problems:
        raise DispatchCoherenceError(
            "dispatch coherence found problem(s):\n  " + "\n  ".join(problems)
        )
