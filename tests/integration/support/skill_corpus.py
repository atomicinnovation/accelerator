"""The `!`-site command corpus and the fixture project it is run against.

Extraction reuses `tasks/lint/skill_permissions`'s own public machinery rather
than a second regex, so the population here cannot drift from the population the
permissions guard checks.

The fixture project and the expectations are generated from one data structure,
so the oracle stays single-sourced as skills are renamed. Configuring an
override for *every* skill in the corpus is the point: with only a token few
configured, the ~84 remaining per-skill expectations reduce to ``stdout == ""``,
which is byte-identical to a read failure degrading under the ``--fail-safe``
that every one of these commands carries. Two fifths of the suite would be
unable to tell success from silent degradation.
"""

import re
import shlex
from dataclasses import dataclass
from pathlib import Path

from tasks.lint.skill_permissions import (
    PLUGIN_PREFIX,
    frontmatter_name,
    is_plugin_invocation,
    preprocessor_commands,
)

CONFIG_MARKER = "/bin/accelerator config "
INSTRUCTIONS_MARKER = "/bin/accelerator config instructions "
CONTEXT_MARKER = "/bin/accelerator config context --skill "

# Shell metacharacters. The corpus is entirely static literals today, so a
# command carrying one of these means the corpus changed shape — it is declined
# rather than executed, because `shell=True` over markdown-derived strings would
# make any SKILL.md an execution vector inside the test process.
METACHARACTERS = ("&&", "||", ";", "|", "$(", "`", "<(", ">(", ">", "<")

# A `!` site whose command names the config surface, on one line.
_INLINE_SITE = re.compile(r"!`[^`]*/bin/accelerator config ")

INSTRUCTIONS_BODY = "FIXTURE-INSTRUCTIONS-{skill}"
CONTEXT_BODY = "FIXTURE-CONTEXT-{skill}"

# Values the non-per-skill families read. Only `agents` and `work` need seeding;
# the `paths`/`path`/`review`/`template` families all have plugin or built-in
# defaults that render without any project configuration.
FIXTURE_CONFIG = """\
---
agents:
  reviewer: fixture-reviewer
work:
  integration: linear
  default_project_code: FIX
  id_pattern: 'FIX-[0-9]+'
---

Fixture team context.
"""


@dataclass(frozen=True)
class Command:
    """One `!`-site invocation, with the SKILL.md it was extracted from."""

    source: str
    raw: str

    @property
    def tail(self) -> str:
        """The command text after the `config` marker, e.g. `template adr`."""
        return self.raw.split(CONFIG_MARKER, 1)[1]

    @property
    def family(self) -> str:
        return self.tail.split()[0]

    @property
    def declined(self) -> bool:
        return any(token in self.raw for token in METACHARACTERS)

    def argv(self, plugin_root: Path) -> list[str]:
        """Claude Code's own textual substitution, then a shell-free split."""
        substituted = self.raw.replace(PLUGIN_PREFIX, f"{plugin_root}/", 1)
        return shlex.split(substituted)


def extract(skills_dir: Path) -> list[Command]:
    """Every `bin/accelerator config` `!` command under `skills/`.

    `is_plugin_invocation` alone matches every plugin *script* invocation too,
    some of which write into `meta/` or call remote trackers, so the marker
    filter is not optional.
    """
    found: list[Command] = []
    for path in sorted(skills_dir.rglob("SKILL.md")):
        source = path.as_posix()
        found.extend(
            Command(source=source, raw=command)
            for command in preprocessor_commands(path.read_text())
            if is_plugin_invocation(command) and CONFIG_MARKER in command
        )
    return found


def source_files(commands: list[Command]) -> set[str]:
    return {command.source for command in commands}


def files_with_config_sites(skills_dir: Path) -> set[str]:
    """SKILL.md paths carrying a `config` command inside a `!` site.

    A line-level scan, independent of `extract`'s walk-and-filter, so a file
    that extraction drops shows up as a mismatch. Matching the marker alone
    would over-count: `allowed-tools` rules and documentation code blocks name
    the command without invoking it.
    """
    found: set[str] = set()
    for path in skills_dir.rglob("SKILL.md"):
        for line in path.read_text().splitlines():
            if _INLINE_SITE.search(line):
                found.add(path.as_posix())
                break
    return found


def declared_skill_names(skills_dir: Path) -> set[str]:
    """Every `name:` declared in a `skills/**/SKILL.md` frontmatter.

    The fixture derives its overrides from the corpus, so a `!` site naming a
    skill that does not exist would otherwise have an override helpfully
    created for it and pass. Neither the permissions census nor this suite's
    count checks would catch that — they count skills carrying an injection,
    not whether the name resolves.
    """
    return {
        name
        for path in skills_dir.rglob("SKILL.md")
        if (name := frontmatter_name(path.read_text()))
    }


def _named_skill(command: Command, marker: str) -> str | None:
    if marker not in command.raw:
        return None
    return command.raw.split(marker, 1)[1].split()[0]


def instructions_skills(commands: list[Command]) -> set[str]:
    return {
        name
        for command in commands
        if (name := _named_skill(command, INSTRUCTIONS_MARKER))
    }


def context_skills(commands: list[Command]) -> set[str]:
    return {
        name
        for command in commands
        if (name := _named_skill(command, CONTEXT_MARKER))
    }


def expected_stdout(command: Command) -> str | None:
    """The exact stdout a per-skill command must produce, else `None`.

    `None` means the family is asserted non-empty rather than by content.
    """
    if name := _named_skill(command, INSTRUCTIONS_MARKER):
        return INSTRUCTIONS_BODY.format(skill=name)
    if name := _named_skill(command, CONTEXT_MARKER):
        return CONTEXT_BODY.format(skill=name)
    return None


def write_project(root: Path, commands: list[Command]) -> Path:
    """Materialise the fixture project the corpus is run against."""
    accelerator = root / ".accelerator"
    accelerator.mkdir(parents=True)
    # A boundary marker, so project-root discovery stops inside the fixture
    # rather than escaping into the real working tree.
    (root / ".git").mkdir()
    (accelerator / "config.md").write_text(FIXTURE_CONFIG)
    for skill in sorted(
        instructions_skills(commands) | context_skills(commands)
    ):
        skill_dir = accelerator / "skills" / skill
        skill_dir.mkdir(parents=True)
        (skill_dir / "instructions.md").write_text(
            INSTRUCTIONS_BODY.format(skill=skill) + "\n"
        )
        (skill_dir / "context.md").write_text(
            CONTEXT_BODY.format(skill=skill) + "\n"
        )
    return root
