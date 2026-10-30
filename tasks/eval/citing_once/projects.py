"""Lay out a composition as a topic-research set a session can work in."""

import json
import re
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from tasks.eval.citing_once.fixtures import Composition
from tasks.eval.citing_once.sources import Citation

TOPICS = Path("meta/research/topics")
COMPOSED_FOCUS = "cited-sources"
COMPOSED_QUESTION = "What do the cited sources establish?"
FINDINGS_PROSE = "The sources examined agree on an answer to this question."


@dataclass(frozen=True)
class Project:
    """A session's working directory, its inputs in order, and its output."""

    root: Path
    set_root: Path
    slug: str
    inputs: tuple[Path, ...]
    output: Path


def _with_fields(document: str, **fields: object) -> str:
    for key, value in fields.items():
        document, replaced = re.subn(
            rf"^{key}: .*$",
            f"{key}: {json.dumps(value, ensure_ascii=False)}",
            document,
            count=1,
            flags=re.MULTILINE,
        )
        if replaced != 1:
            raise ValueError(f"the seed document has no {key} field")
    return document


def _frontmatter(document: str) -> str:
    return document[: document.index("\n---\n", 4) + len("\n---\n")]


def _research_document(
    seed: str, asked: str, sources: Sequence[Citation], **fields: object
) -> str:
    return (
        _frontmatter(_with_fields(seed, **fields))
        + f"\n# {asked}\n\n## Question\n{asked}\n\n"
        + f"## Findings\n{FINDINGS_PROSE}\n\n## Sources\n"
        + "".join(citation.render() for citation in sources)
    )


def _write(path: Path, text: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def _set_documents(
    set_root: Path, seed: Path, slug: str, questions: Sequence[str]
) -> None:
    title = f"Citing once: {slug}"
    _write(
        set_root / "manifest.md",
        _with_fields(
            (seed / "manifest.md").read_text(),
            id=slug,
            title=f"Research Topic: {title}",
            status="researching",
            slug=slug,
            round_count=1,
            finding_count=len(questions),
        ),
    )
    items = "".join(f"- {question}\n" for question in questions)
    _write(
        set_root / "brief.md",
        _frontmatter(
            _with_fields(
                (seed / "brief.md").read_text(),
                id=f"{slug}-brief",
                title=f"Brief: {title}",
            )
        )
        + f"\n# Brief: {title}\n\n## Subject\n{FINDINGS_PROSE}\n\n"
        + f"## Questions\n{items}",
    )
    checked = "".join(
        f"- [x] {question} — profiles: web\n" for question in questions
    )
    _write(
        set_root / "outline.md",
        _frontmatter(
            _with_fields(
                (seed / "outline.md").read_text(),
                id=f"{slug}-outline",
                title=f"Outline: {title}",
            )
        )
        + f"\n# Outline: {title}\n\n## Focus Areas\n\n## Round 1\n\n{checked}",
    )


def _node_question(node: str) -> str:
    return (
        COMPOSED_QUESTION if node == "1" else f"What does follow-up {node} add?"
    )


def _composer_project(
    composition: Composition,
    order: Sequence[str],
    root: Path,
    seed: Path,
    slug: str,
) -> Project:
    set_root = root / TOPICS / slug
    _set_documents(set_root, seed, slug, [COMPOSED_QUESTION])
    stem = f"01-{COMPOSED_FOCUS}-web"
    levels = set_root / "findings" / f"{stem}.levels"
    seeds = seed / "findings" / "01-first-focus-web.levels"
    children = [node for node in composition.documents if node != "1"]
    for node in composition.documents:
        root_note = node == "1"
        _write(
            levels / f"{node}.md",
            _research_document(
                (seeds / ("1.md" if root_note else "2-1.md")).read_text(),
                _node_question(node),
                composition.citations_in(node),
                id=f"{slug}.{stem}.{node}",
                title=_node_question(node),
                question=_node_question(node),
                level=1 if root_note else 2,
                follow_ups=[_node_question(c) for c in children]
                if root_note
                else [],
            ),
        )
    return Project(
        root=root,
        set_root=set_root,
        slug=slug,
        inputs=tuple(levels / f"{node}.md" for node in order),
        output=set_root / "findings" / f"{stem}.md",
    )


def _synthesise_project(
    composition: Composition,
    order: Sequence[str],
    root: Path,
    seed: Path,
    slug: str,
) -> Project:
    set_root = root / TOPICS / slug
    questions = [f"What does focus {key} establish?" for key in order]
    _set_documents(set_root, seed, slug, questions)
    finding_seed = (seed / "findings" / "01-first-focus-web.md").read_text()
    inputs = []
    for index, (key, question) in enumerate(
        zip(order, questions, strict=True), start=1
    ):
        stem = f"{index:02d}-focus-{key}-web"
        inputs.append(
            _write(
                set_root / "findings" / f"{stem}.md",
                _research_document(
                    finding_seed,
                    question,
                    composition.citations_in(key),
                    id=stem,
                    title=f"Finding: {question}",
                    question=question,
                    depth=1,
                ),
            )
        )
    return Project(
        root=root,
        set_root=set_root,
        slug=slug,
        inputs=tuple(inputs),
        output=set_root / "synthesis.md",
    )


def build_project(
    composition: Composition, order: Sequence[str], root: Path, seed: Path
) -> Project:
    slug = f"citing-once-{composition.name}"
    if composition.carrier == "composer":
        return _composer_project(composition, order, root, seed, slug)
    return _synthesise_project(composition, order, root, seed, slug)
