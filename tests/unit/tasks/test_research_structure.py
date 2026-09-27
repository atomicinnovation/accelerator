"""Research-agent, source-profile, and fetch-grant structure guard.

A pure content scan (no compiled binary), homed with the other content
scanners. The generic researcher stays source-agnostic with a pinned tool
grant; each academic profile reaches its source only through the fetch; and
every skill that injects an academic profile grants the fetch, because the
researchers it spawns inherit its allow rules.
"""

import re
from pathlib import Path

import pytest

from tasks.shared.skill_parsing import (
    fenced_block_commands,
    frontmatter_bash_rules,
)

REPO_ROOT = Path(__file__).resolve().parents[3]

RESEARCHER = REPO_ROOT / "agents/researcher.md"
RESEARCH_TOPIC = REPO_ROOT / "skills/research/research-topic/SKILL.md"
PROFILES_DIR = Path("skills/research/profiles")

FETCH = "accelerator research fetch "
FETCH_GRANT = f"{FETCH}*"
ACADEMIC_FAMILIES = {"openalex": "OpenAlex", "arxiv": "arXiv"}


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def _split(text: str) -> tuple[str, str]:
    """The frontmatter and the body of a markdown file."""
    _, frontmatter, body = text.split("---\n", 2)
    return frontmatter, body


def _profile_skill(name: str) -> Path:
    return PROFILES_DIR / f"{name}-profile" / "SKILL.md"


def _academic_profiles() -> list[str]:
    """Every installed profile whose skill grants the fetch."""
    return sorted(
        path.parent.name.removesuffix("-profile")
        for path in (REPO_ROOT / PROFILES_DIR).glob("*-profile/SKILL.md")
        if FETCH_GRANT in frontmatter_bash_rules(_read(path))
    )


def test_the_researcher_agent_grants_a_bounded_tool_set() -> None:
    frontmatter, _ = _split(_read(RESEARCHER))
    tools = re.search(r"^tools:(.*)$", frontmatter, re.MULTILINE)
    assert tools, "agents/researcher.md carries no tools: grant"
    granted = {tool.strip() for tool in tools.group(1).split(",")}
    assert granted == {"WebSearch", "WebFetch", "Write", "Read", "Bash"}, (
        "the researcher's tool grant has drifted from the pinned set"
    )


@pytest.mark.parametrize("family", ["openalex", "arxiv", "web-profile"])
def test_the_researcher_body_names_no_source_family(family: str) -> None:
    _, body = _split(_read(RESEARCHER))
    assert family not in body.lower(), (
        f"the researcher's body names {family}; profiles are injected"
    )


@pytest.mark.parametrize(("family", "name"), sorted(ACADEMIC_FAMILIES.items()))
def test_an_academic_profile_grants_the_fetch_and_no_web_tool(
    family: str, name: str
) -> None:
    text = _read(REPO_ROOT / _profile_skill(family))
    frontmatter, _ = _split(text)
    assert FETCH_GRANT in frontmatter_bash_rules(text), (
        f"the {name} profile must grant the fetch"
    )
    assert "Web" not in frontmatter, (
        f"the {name} profile must grant no web tool"
    )


@pytest.mark.parametrize(("family", "name"), sorted(ACADEMIC_FAMILIES.items()))
def test_an_academic_profile_mentions_a_web_tool_only_to_forbid_it(
    family: str, name: str
) -> None:
    _, body = _split(_read(REPO_ROOT / _profile_skill(family)))
    for line in body.splitlines():
        if "Web" in line:
            text = line.lstrip().removeprefix("- ")
            assert text.startswith(("No ", "Never ")), (
                f"the {name} profile mentions a web tool other than to "
                f"forbid it: {line}"
            )


@pytest.mark.parametrize(("family", "name"), sorted(ACADEMIC_FAMILIES.items()))
def test_an_academic_profile_runs_only_its_own_fetch(
    family: str, name: str
) -> None:
    _, body = _split(_read(REPO_ROOT / _profile_skill(family)))
    commands = fenced_block_commands(body)
    assert commands, f"the {name} profile shows no fetch to run"
    for command in commands:
        assert command.startswith(f"{FETCH}{family} "), (
            f"the {name} profile runs something other than its fetch: {command}"
        )


def test_research_topic_grants_the_fetch_its_researchers_inherit() -> None:
    assert FETCH_GRANT in frontmatter_bash_rules(_read(RESEARCH_TOPIC))


def _research_topic_prose() -> str:
    """The skill's text with every run of whitespace collapsed to one space,
    so an assertion never depends on where the prose wraps."""
    return " ".join(_read(RESEARCH_TOPIC).split())


def _section(prose: str, start: str, end: str) -> str:
    begin = prose.find(start)
    assert begin >= 0, f"{RESEARCH_TOPIC.name} lacks {start!r}"
    rest = prose[begin:]
    finish = rest.find(end)
    assert finish >= 0, f"{RESEARCH_TOPIC.name} lacks {end!r}"
    return rest[:finish]


def test_research_topic_batches_through_the_spawn_window() -> None:
    conduct = _section(_research_topic_prose(), "### conduct", "### synthesise")
    for step in (
        "--limit {concurrency} --start",
        "--run {run} --spawned {batch}",
    ):
        assert step in conduct, (
            f"conduct must plan through the spawn window with {step!r}"
        )
    end_run = conduct.find(
        "accelerator research topic end-run SLUG --run {run}"
    )
    assert end_run >= 0, "conduct never ends its run"
    manifest_edit = conduct.find("**Edit `manifest.md`**")
    assert manifest_edit >= 0, "conduct never edits the manifest"
    assert end_run < manifest_edit, (
        "conduct must end its run before the final manifest edit"
    )


def test_research_topic_clamps_concurrency_under_the_knob_rule() -> None:
    knobs = _section(
        _research_topic_prose(),
        "knobs bound the research",
        "## Shared Preamble",
    )
    assert (
        "- concurrency: !`accelerator config get "
        "research.topic.concurrency --fail-safe`"
    ) in knobs, (
        "the knob block must resolve concurrency beside breadth and depth"
    )
    assert (
        "(`--depth` or `--concurrency` on `outline`, `--breadth` on `conduct`)"
    ) in knobs, "the misplaced-flag rule must name --concurrency on outline"


def _injects(skill: str, profile: str) -> bool:
    """Whether `skill` injects `profile`, by name or through the `<profile>`
    placeholder a per-pair injection is written with."""
    return any(
        _profile_skill(name).as_posix() in skill
        for name in (profile, "<profile>")
    )


def test_every_skill_injecting_an_academic_profile_grants_the_fetch() -> None:
    academic = _academic_profiles()
    assert academic, "no profile grants the fetch"
    injecting = [
        path
        for path in sorted((REPO_ROOT / "skills").rglob("SKILL.md"))
        if not path.is_relative_to(REPO_ROOT / PROFILES_DIR)
        and any(_injects(_read(path), profile) for profile in academic)
    ]
    assert injecting, "no skill injects an academic profile"
    for path in injecting:
        assert FETCH_GRANT in frontmatter_bash_rules(_read(path)), (
            f"{path.relative_to(REPO_ROOT)} injects an academic profile "
            f"without granting Bash({FETCH_GRANT})"
        )
