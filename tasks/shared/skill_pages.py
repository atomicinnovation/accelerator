"""Generate Starlight reference pages from SKILL.md sources.

Pure functions only — the invoke glue lives in tasks/docs.py. Bodies are
sanitised for plain CommonMark rendering: `!` preprocessor commands become
literal inline code and angle-bracket placeholders outside code fences are
escaped so Astro never treats them as HTML.
"""

import json
import re
import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml

from tasks.shared.files import atomic_write_text
from tasks.shared.paths import DOCS_GENERATED_DIR, REPO_ROOT

DOCS_GENERATED_RELATIVE = DOCS_GENERATED_DIR.relative_to(REPO_ROOT)

_EXCLUDED_PATH_PARTS = frozenset({"node_modules", "test-fixtures"})
_FRONTMATTER = re.compile(r"\A---\n(.*?)\n---\n", re.DOTALL)
_PREPROCESSOR = re.compile(r"!`([^`]+)`")
_CODE_SPAN = re.compile(r"`[^`]*`")


class SkillPageError(Exception):
    """A SKILL.md could not be turned into a docs page."""


@dataclass(frozen=True)
class SkillPage:
    name: str
    category: str
    source: Path
    frontmatter: dict[str, Any]
    body: str

    @property
    def internal(self) -> bool:
        return self.frontmatter.get("user-invocable") is False

    @property
    def description(self) -> str:
        return str(self.frontmatter.get("description", "")).strip()


def discover_skills(repo_root: Path) -> list[SkillPage]:
    plugin = json.loads((repo_root / ".claude-plugin/plugin.json").read_text())
    skills_root = repo_root / "skills"
    pages: list[SkillPage] = []
    for entry in plugin["skills"]:
        for skill_md in sorted((repo_root / entry).rglob("SKILL.md")):
            if _EXCLUDED_PATH_PARTS.intersection(skill_md.parts):
                continue
            pages.append(_load_skill(skill_md, skills_root))
    return pages


def _load_skill(skill_md: Path, skills_root: Path) -> SkillPage:
    text = skill_md.read_text()
    match = _FRONTMATTER.match(text)
    if match is None:
        raise SkillPageError(f"no frontmatter in {skill_md}")
    frontmatter: dict[str, Any] = yaml.safe_load(match.group(1))
    category = skill_md.parent.parent.relative_to(skills_root).as_posix()
    return SkillPage(
        name=str(frontmatter["name"]),
        category=category,
        source=skill_md,
        frontmatter=frontmatter,
        body=text[match.end() :],
    )


def sanitise_body(body: str) -> str:
    out: list[str] = []
    fence: str | None = None
    for line in body.splitlines():
        stripped = line.lstrip()
        if fence is not None:
            out.append(line)
            if stripped.startswith(fence):
                fence = None
        elif stripped.startswith(("```", "~~~")):
            fence = stripped[:3]
            out.append(line)
        else:
            neutralised = _PREPROCESSOR.sub(r"`!\1`", line)
            out.append(_escape_outside_code_spans(neutralised))
    return "\n".join(out).strip() + "\n"


def _escape_outside_code_spans(line: str) -> str:
    parts: list[str] = []
    last = 0
    for match in _CODE_SPAN.finditer(line):
        parts.append(line[last : match.start()].replace("<", "\\<"))
        parts.append(match.group())
        last = match.end()
    parts.append(line[last:].replace("<", "\\<"))
    return "".join(parts)


def output_path(page: SkillPage, generated_dir: Path) -> Path:
    if page.internal:
        return generated_dir / "internal" / f"{page.name}.md"
    return generated_dir / page.category / f"{page.name}.md"


def _invocability(page: SkillPage) -> str:
    if page.internal:
        return "Internal — not user-invocable (agent-preloaded)"
    if page.frontmatter.get("disable-model-invocation") is True:
        return "User-invoked only (model invocation disabled)"
    return "User- and model-invocable"


def _starlight_frontmatter(page: SkillPage) -> str:
    sidebar: dict[str, Any] = {"label": page.name}
    if page.internal:
        sidebar["badge"] = {"text": "Internal", "variant": "caution"}
    frontmatter = {
        "title": page.name,
        "description": page.description,
        "sidebar": sidebar,
    }
    dumped = yaml.safe_dump(frontmatter, sort_keys=False, allow_unicode=True)
    return f"---\n{dumped}---\n"


def _header_block(page: SkillPage) -> str:
    lines = [f"**Invocation:** {_invocability(page)}  "]
    hint = page.frontmatter.get("argument-hint")
    if hint:
        lines.append(f"**Argument hint:** `{hint}`  ")
    tools = page.frontmatter.get("allowed-tools")
    if tools:
        if isinstance(tools, str):
            tools = [tools]
        joined = ", ".join(f"`{tool}`" for tool in tools)
        lines.append(f"**Allowed tools:** {joined}  ")
    lines.append(f"**Source:** `{_source_relative(page)}`")
    return "\n".join(lines) + "\n"


def _source_relative(page: SkillPage) -> str:
    parts = page.source.parts
    return Path(*parts[parts.index("skills") :]).as_posix()


def render_page(page: SkillPage) -> str:
    return (
        _starlight_frontmatter(page)
        + "\n"
        + _header_block(page)
        + "\n"
        + sanitise_body(page.body)
    )


def _first_sentence(text: str) -> str:
    collapsed = " ".join(text.split())
    head, separator, _ = collapsed.partition(". ")
    return head + ("." if separator else "")


def render_index(pages: list[SkillPage]) -> str:
    frontmatter = {
        "title": "All skills",
        "description": "Generated reference for every Accelerator skill.",
        "sidebar": {"label": "All skills"},
    }
    dumped = yaml.safe_dump(frontmatter, sort_keys=False, allow_unicode=True)
    lines = [
        f"---\n{dumped}---",
        "",
        "Generated from each skill's `SKILL.md` at build time.",
    ]
    public = [p for p in pages if not p.internal]
    for category in sorted({p.category for p in public}):
        lines += ["", f"## {category}", ""]
        lines += [
            f"- [{p.name}]({p.category}/{p.name}.md) — "
            f"{_first_sentence(p.description)}"
            for p in sorted(public, key=lambda p: p.name)
            if p.category == category
        ]
    internal = sorted((p for p in pages if p.internal), key=lambda p: p.name)
    if internal:
        lines += ["", "## Internal", ""]
        lines += [
            f"- [{p.name}](internal/{p.name}.md) — "
            f"{_first_sentence(p.description)}"
            for p in internal
        ]
    return "\n".join(lines) + "\n"


def generate_pages(repo_root: Path) -> list[tuple[str, Path]]:
    generated_dir = repo_root / DOCS_GENERATED_RELATIVE
    pages = discover_skills(repo_root)
    outputs: dict[Path, str] = {}
    for page in pages:
        path = output_path(page, generated_dir)
        if path in outputs:
            raise SkillPageError(
                f"output collision at {path}: {outputs[path]} and {page.name}"
            )
        outputs[path] = page.name
    if generated_dir.exists():
        shutil.rmtree(generated_dir)
    for page in pages:
        path = output_path(page, generated_dir)
        path.parent.mkdir(parents=True, exist_ok=True)
        atomic_write_text(path, render_page(page))
    atomic_write_text(generated_dir / "index.md", render_index(pages))
    return [(page.name, output_path(page, generated_dir)) for page in pages]
