"""Guard the structural conformance of every review lens ``SKILL.md``.

A Python port of ``scripts/test-lens-structure.sh``. For each ``*-lens``
directory under ``skills/review/lenses`` this asserts:

1. the ``SKILL.md`` exists;
2. its frontmatter declares ``user-invocable: false``;
3. its frontmatter declares ``disable-model-invocation: true``;
4. its frontmatter carries a non-empty ``name``;
5. its frontmatter carries a ``description``;
6. it contains the four required section headings;
7. it has an H1 heading;
8. it has a persona sentence between the H1 and the first ``##``;
9. for the five built-in work-item lenses only, its ``What NOT to Do``
   section names at least three peer work-item lenses;
10. it closes with a ``Remember:`` paragraph.

Synthetic ``tmp_path`` lens trees exercise each branch and its negative case,
plus a live-tree assertion that the shipped lenses pass the guard.
"""

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

WORK_ITEM_LENSES = frozenset(
    {"clarity", "completeness", "dependency", "scope", "testability"}
)

REQUIRED_HEADINGS = (
    "## Core Responsibilities",
    "## Key Evaluation Questions",
    "## Important Guidelines",
    "## What NOT to Do",
)


def _frontmatter(text: str) -> str:
    """Return the lines between the first and second ``---`` delimiters."""
    collected: list[str] = []
    seen = 0
    for line in text.splitlines():
        if line == "---":
            seen += 1
            if seen >= 2:
                break
            continue
        if seen == 1:
            collected.append(line)
    return "\n".join(collected)


def _name_value(frontmatter: str) -> str:
    """Return the value of the first ``name:`` line, leading space stripped."""
    for line in frontmatter.splitlines():
        if line.startswith("name:"):
            return re.sub(r"^name:\s*", "", line)
    return ""


def _has_persona(text: str) -> bool:
    """Whether a non-empty line sits between the H1 and the first ``##``."""
    found_h1 = False
    for line in text.splitlines():
        if not found_h1:
            found_h1 = line.startswith("# ")
            continue
        if line.startswith("## "):
            return False
        if re.search(r"[A-Za-z0-9]", line):
            return True
    return False


def _what_not_to_do_body(text: str) -> str:
    """Return the body between ``## What NOT to Do`` and the next ``## ``."""
    collected: list[str] = []
    found = False
    for line in text.splitlines():
        if not found:
            found = line.startswith("## What NOT to Do")
            continue
        if line.startswith("## "):
            break
        collected.append(line)
    return "\n".join(collected)


def _peer_count(text: str, lens_id: str) -> int:
    """Count peer work-item lenses named in the ``What NOT to Do`` body."""
    body = _what_not_to_do_body(text)
    return sum(
        1
        for peer in WORK_ITEM_LENSES
        if peer != lens_id and re.search(rf"\b{peer}\b", body)
    )


def _frontmatter_violations(lens_name: str, frontmatter: str) -> list[str]:
    """Return the frontmatter-declaration violations for one lens."""
    found: list[str] = []
    if not re.search(r"^user-invocable: false$", frontmatter, re.MULTILINE):
        found.append(
            f"{lens_name}: missing 'user-invocable: false' in frontmatter"
        )
    if not re.search(
        r"^disable-model-invocation: true$", frontmatter, re.MULTILINE
    ):
        found.append(
            f"{lens_name}: missing 'disable-model-invocation: true' in "
            "frontmatter"
        )
    if not _name_value(frontmatter):
        found.append(f"{lens_name}: missing or empty 'name' in frontmatter")
    if not re.search(r"^description:", frontmatter, re.MULTILINE):
        found.append(f"{lens_name}: missing 'description' in frontmatter")
    return found


def _body_violations(lens_name: str, lens_id: str, text: str) -> list[str]:
    """Return the heading, persona, peer and closing violations for one lens."""
    found: list[str] = [
        f"{lens_name}: missing '{heading}'"
        for heading in REQUIRED_HEADINGS
        if heading not in text
    ]
    if not re.search(r"^# ", text, re.MULTILINE):
        found.append(f"{lens_name}: missing H1 heading")
    if not _has_persona(text):
        found.append(
            f"{lens_name}: missing persona sentence between H1 and first ##"
        )
    if lens_id in WORK_ITEM_LENSES:
        peer_count = _peer_count(text, lens_id)
        if peer_count < 3:
            found.append(
                f"{lens_name}: 'What NOT to Do' names only {peer_count} peer "
                "work-item lenses (need >= 3)"
            )
    if not re.search(r"^Remember:", text, re.MULTILINE):
        found.append(f"{lens_name}: missing closing 'Remember:' paragraph")
    return found


def _lens_violations(lens_dir: Path) -> list[str]:
    """Return every structural violation for a single lens directory."""
    lens_name = lens_dir.name
    lens_id = lens_name[: -len("-lens")]
    skill_file = lens_dir / "SKILL.md"

    if not skill_file.is_file():
        return [f"{lens_name}: {skill_file} does not exist"]

    text = skill_file.read_text()
    return _frontmatter_violations(
        lens_name, _frontmatter(text)
    ) + _body_violations(lens_name, lens_id, text)


def violations(root: Path) -> list[str]:
    """Every structural violation across ``skills/review/lenses/*-lens``."""
    base = root / "skills" / "review" / "lenses"
    found: list[str] = []
    for lens_dir in sorted(base.glob("*-lens")):
        if lens_dir.is_dir():
            found.extend(_lens_violations(lens_dir))
    return found


def _write_lens(root: Path, name: str, body: str) -> Path:
    lens_dir = root / "skills" / "review" / "lenses" / name
    lens_dir.mkdir(parents=True, exist_ok=True)
    (lens_dir / "SKILL.md").write_text(body)
    return lens_dir


def _conforming_body(lens_id: str, *, work_item: bool) -> str:
    peers = "clarity, completeness or dependency" if work_item else "other work"
    return (
        "---\n"
        f"name: {lens_id}\n"
        "description: A structural review lens.\n"
        "user-invocable: false\n"
        "disable-model-invocation: true\n"
        "---\n"
        "\n"
        f"# {lens_id.title()} Lens\n"
        "\n"
        "Review as a specialist evaluating this artefact.\n"
        "\n"
        "## Core Responsibilities\n"
        "\n"
        "Assess the artefact.\n"
        "\n"
        "## Key Evaluation Questions\n"
        "\n"
        "What matters here?\n"
        "\n"
        "## Important Guidelines\n"
        "\n"
        "Stay in scope.\n"
        "\n"
        "## What NOT to Do\n"
        "\n"
        f"Don't do {peers} work.\n"
        "\n"
        "## Wrap Up\n"
        "\n"
        "Remember: keep the finding tight.\n"
    )


def test_a_conforming_work_item_lens_has_no_violations(tmp_path: Path) -> None:
    _write_lens(
        tmp_path, "scope-lens", _conforming_body("scope", work_item=True)
    )
    assert violations(tmp_path) == []


def test_a_conforming_code_review_lens_has_no_violations(
    tmp_path: Path,
) -> None:
    _write_lens(
        tmp_path,
        "correctness-lens",
        _conforming_body("correctness", work_item=False),
    )
    assert violations(tmp_path) == []


def test_missing_skill_file_is_flagged(tmp_path: Path) -> None:
    lens_dir = tmp_path / "skills" / "review" / "lenses" / "scope-lens"
    lens_dir.mkdir(parents=True)
    assert any("does not exist" in v for v in violations(tmp_path))


def test_missing_user_invocable_is_flagged(tmp_path: Path) -> None:
    body = _conforming_body("scope", work_item=True).replace(
        "user-invocable: false\n", ""
    )
    _write_lens(tmp_path, "scope-lens", body)
    assert any("user-invocable: false" in v for v in violations(tmp_path))


def test_missing_disable_model_invocation_is_flagged(tmp_path: Path) -> None:
    body = _conforming_body("scope", work_item=True).replace(
        "disable-model-invocation: true\n", ""
    )
    _write_lens(tmp_path, "scope-lens", body)
    assert any(
        "disable-model-invocation: true" in v for v in violations(tmp_path)
    )


def test_empty_name_is_flagged(tmp_path: Path) -> None:
    body = _conforming_body("scope", work_item=True).replace(
        "name: scope\n", "name:\n"
    )
    _write_lens(tmp_path, "scope-lens", body)
    assert any("missing or empty 'name'" in v for v in violations(tmp_path))


def test_missing_description_is_flagged(tmp_path: Path) -> None:
    body = _conforming_body("scope", work_item=True).replace(
        "description: A structural review lens.\n", ""
    )
    _write_lens(tmp_path, "scope-lens", body)
    assert any("missing 'description'" in v for v in violations(tmp_path))


def test_each_missing_section_heading_is_flagged(tmp_path: Path) -> None:
    for heading in REQUIRED_HEADINGS:
        body = _conforming_body("scope", work_item=True).replace(
            f"{heading}\n", ""
        )
        _write_lens(tmp_path, "scope-lens", body)
        assert any(f"missing '{heading}'" in v for v in violations(tmp_path)), (
            heading
        )


def test_missing_h1_is_flagged(tmp_path: Path) -> None:
    body = _conforming_body("scope", work_item=True).replace(
        "# Scope Lens\n", "Scope Lens\n"
    )
    _write_lens(tmp_path, "scope-lens", body)
    assert any("missing H1 heading" in v for v in violations(tmp_path))


def test_missing_persona_is_flagged(tmp_path: Path) -> None:
    body = _conforming_body("scope", work_item=True).replace(
        "Review as a specialist evaluating this artefact.\n", ""
    )
    _write_lens(tmp_path, "scope-lens", body)
    assert any("missing persona sentence" in v for v in violations(tmp_path))


def test_insufficient_peer_references_is_flagged(tmp_path: Path) -> None:
    body = _conforming_body("scope", work_item=True).replace(
        "Don't do clarity, completeness or dependency work.\n",
        "Don't do clarity work.\n",
    )
    _write_lens(tmp_path, "scope-lens", body)
    assert any(
        "names only 1 peer work-item lenses" in v for v in violations(tmp_path)
    )


def test_peer_reference_check_is_skipped_for_code_review_lenses(
    tmp_path: Path,
) -> None:
    body = _conforming_body("correctness", work_item=False).replace(
        "Don't do other work.\n", "Don't do anything unrelated.\n"
    )
    _write_lens(tmp_path, "correctness-lens", body)
    assert not any("peer work-item lenses" in v for v in violations(tmp_path))


def test_missing_remember_paragraph_is_flagged(tmp_path: Path) -> None:
    body = _conforming_body("scope", work_item=True).replace(
        "Remember: keep the finding tight.\n", "Keep the finding tight.\n"
    )
    _write_lens(tmp_path, "scope-lens", body)
    assert any(
        "missing closing 'Remember:' paragraph" in v
        for v in violations(tmp_path)
    )


def test_the_real_lenses_tree_passes() -> None:
    assert violations(REPO_ROOT) == []
