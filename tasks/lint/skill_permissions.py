"""Guard the SKILL.md `!`-preprocessor / `allowed-tools` invocation contract.

For every ``skills/**/SKILL.md`` this asserts, per ADR-0048 as a Python CI
guardrail rather than a shell script:

1. every ``!``-preprocessor command that invokes a plugin script or the launcher
   is covered by at least one ``Bash(...)`` frontmatter rule, matched as a
   prefix/glob where ``*`` spans ``/`` (the empirically-verified matcher);
2. no ``Bash`` rule authorises the launcher without naming a subcommand (an
   ancestor glob would silently pre-authorise every future sub-binary);
3. every ``bin/accelerator config`` command in a ``!`` block carries
   ``--fail-safe`` (without it a read failure discards the whole prompt);
4. no ``!`` command contains a shell metacharacter (the matcher is a literal
   prefix, so a chained command could smuggle an unmatched call past a rule).

It also carries the SKILL.md injection census (formerly in the deleted
``test-config.sh``):

5. every ``config context --skill <name>`` / ``config instructions <name>``
   names the SKILL.md's own frontmatter ``name``;
6. where a skill injects instructions, that is the last ``!`` preprocessor
   command in the body;
7. context and instructions injection are present in exactly the expected
   number of skills, and move together — ``configure`` injects neither and is
   excluded by construction.
"""

import re
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.skill_parsing import (
    BARE_LAUNCHER,
    covered_by,
    frontmatter_bash_rules,
    frontmatter_name,
    has_bare_bash,
    has_metacharacter,
    is_plugin_invocation,
    preprocessor_commands,
)
from tasks.shared.sources import repo_root

# Injection is expected in exactly this many skills (42 at the migration's final
# state). Bump deliberately when a skill's context/instructions injection is
# genuinely added or removed — the equality is what catches an accidental loss.
#
# Unmoved by `browser-executor`'s retirement: that skill injected an executor
# path through its own script, never `accelerator config context|instructions`,
# so it was never one of the counted skills.
EXPECTED_INJECTION_SKILLS = 42

_CONFIG_MARKER = "/bin/accelerator config "
_CONTEXT_SKILL = "/bin/accelerator config context --skill "
_CONTEXT_ANY = "/bin/accelerator config context"
_INSTRUCTIONS = "/bin/accelerator config instructions "
_NAME_TOKEN = re.compile(r"([a-z0-9][a-z0-9-]*)")


def _name_after(command: str, marker: str) -> str:
    """Return the identifier token immediately following ``marker``."""
    tail = command.split(marker, 1)[1]
    match = _NAME_TOKEN.match(tail)
    return match.group(1) if match else ""


def _command_violations(
    command: str, name: str, rel: str, rules: list[str], *, bare: bool
) -> tuple[list[str], bool, bool]:
    """Return one command's violations plus its context/instructions signals."""
    if has_metacharacter(command):
        return (
            [
                f"{rel}: '!`{command}`' contains a shell metacharacter — the "
                "matcher is a literal prefix and cannot see past it"
            ],
            False,
            False,
        )

    found: list[str] = []
    if _CONFIG_MARKER in command and " --fail-safe" not in command:
        found.append(
            f"{rel}: '!`{command}`' is missing --fail-safe — a read failure "
            "would exit non-zero and discard the prompt"
        )

    is_ctx = _CONTEXT_ANY in command
    if _CONTEXT_SKILL in command:
        argument = _name_after(command, _CONTEXT_SKILL)
        if argument != name:
            found.append(
                f"{rel}: 'config context --skill {argument}' does not name "
                f"this skill's frontmatter name '{name}'"
            )

    is_instr = _INSTRUCTIONS in command
    if is_instr:
        argument = _name_after(command, _INSTRUCTIONS)
        if argument != name:
            found.append(
                f"{rel}: 'config instructions {argument}' does not name this "
                f"skill's frontmatter name '{name}'"
            )

    if not bare and not any(covered_by(command, rule) for rule in rules):
        found.append(
            f"{rel}: '!`{command}`' is not covered by any Bash(...) rule — it "
            "will prompt at load"
        )
    return found, is_ctx, is_instr


def _check_skill(path: Path, rel: str) -> tuple[list[str], bool, bool]:
    """Per-skill violations plus whether it injects context / instructions."""
    text = path.read_text()
    rules = frontmatter_bash_rules(text)
    name = frontmatter_name(text)
    bare = has_bare_bash(text)
    commands = preprocessor_commands(text)

    found: list[str] = [
        f"{rel}: rule 'Bash({rule})' authorises the launcher without a "
        "subcommand — name 'config' (or the specific subcommand)"
        for rule in rules
        if covered_by(BARE_LAUNCHER, rule)
    ]
    has_ctx = False
    has_instr = False
    for command in commands:
        if not is_plugin_invocation(command):
            continue
        command_found, is_ctx, is_instr = _command_violations(
            command, name, rel, rules, bare=bare
        )
        found.extend(command_found)
        has_ctx = has_ctx or is_ctx
        has_instr = has_instr or is_instr

    if has_instr:
        plugin_commands = [c for c in commands if is_plugin_invocation(c)]
        last = plugin_commands[-1] if plugin_commands else ""
        if _INSTRUCTIONS not in last:
            found.append(
                f"{rel}: 'config instructions' is not the last `!` "
                "preprocessor command"
            )

    return found, has_ctx, has_instr


def violations(root: Path) -> list[str]:
    """Every contract or census violation across ``skills/**/SKILL.md``."""
    found: list[str] = []
    context_skills = 0
    instructions_skills = 0
    for path in sorted((root / "skills").rglob("SKILL.md")):
        rel = path.relative_to(root).as_posix()
        skill_found, has_ctx, has_instr = _check_skill(path, rel)
        found.extend(skill_found)
        context_skills += int(has_ctx)
        instructions_skills += int(has_instr)

    if context_skills != EXPECTED_INJECTION_SKILLS:
        found.append(
            f"context injection present in {context_skills} skill(s), expected "
            f"{EXPECTED_INJECTION_SKILLS} — bump EXPECTED_INJECTION_SKILLS if "
            "this was intended"
        )
    if instructions_skills != EXPECTED_INJECTION_SKILLS:
        found.append(
            f"instructions injection present in {instructions_skills} "
            f"skill(s), expected {EXPECTED_INJECTION_SKILLS}"
        )
    return found


@task
def check(context: Context) -> None:
    """Fail if any SKILL.md breaks the invocation contract or the census."""
    offenders = violations(repo_root())
    if offenders:
        raise Exit(
            "check-skill-permissions found violation(s):\n  "
            + "\n  ".join(offenders),
            code=1,
        )
