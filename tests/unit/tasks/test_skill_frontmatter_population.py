"""SKILL-prose frontmatter-population guard, ported from the shell script
``scripts/test-skill-frontmatter-population.sh``.

For each row in ``skills-schema.tsv`` (carried at
``tests/unit/tasks/data/skills-schema.tsv``) this asserts that the consuming
SKILL.md instructs the model to populate every mandatory field, and carries
fill/omit guidance for every omit-when-empty field.

A field counts as populated when its name appears in one of four instruction
contexts inside the SKILL.md:

1. Fenced-block context — the field name is a YAML key (``^<field>:``) inside a
   triple-backtick fenced code block (template-inclusion ``!`` directives are
   stripped first).
2. Imperative-instruction context — inside a persistence-related section, both
   an imperative verb (Substitute|Populate|Set|Write|Emit) and a colon-anchored
   field reference appear (not necessarily on the same line).
3. CLI-delegation context — the field's CLI flag appears in a fenced
   ``accelerator work create``/``update`` invocation.
4. CLI-managed-no-flag context — for fields the binary sets unconditionally
   (``schema_version``/``last_updated``/``last_updated_by``), any such
   invocation satisfies the field by construction.

An omit-when-empty field is satisfied when its own bullet in a
Populate-frontmatter section carries whole-word fill/omit guidance, or when it
is delegated to a bracketed ``[--flag ...]`` CLI argument whose convention is
explained generically nearby.

The synthetic tests exercise each branch and negative case directly; one
live-tree test runs the whole scan over the shipped ``skills/`` and asserts
zero violations.
"""

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SCHEMA_TSV = REPO_ROOT / "tests/unit/tasks/data/skills-schema.tsv"

# Fields the compiled binary sets unconditionally with no CLI flag; there is no
# lever for prose to instruct, so any work-create/update invocation satisfies
# them.
CLI_MANAGED_NO_FLAG: frozenset[str] = frozenset(
    {"schema_version", "last_updated", "last_updated_by"}
)

# Locates a Populate-frontmatter-ish section (matched against the lowered
# heading line). It does NOT enforce the literal "Populate frontmatter" heading
# — that is a separate reviewer assertion below.
_HEADING_RE = re.compile(
    r"persistence|metadata|frontmatter|populate|capture metadata|step [0-9]"
)

_TEMPLATE_DIRECTIVE = re.compile(r"^!`.*accelerator config template")
_VERBS_RE = re.compile(r"[Ss]ubstitute|[Pp]opulate|[Ss]et|[Ww]rite|[Ee]mit")
_WORK_CLI_RE = re.compile(r"accelerator work (create|update)")
_FENCE_INDENTED = re.compile(r"^[ \t]*```")
_BULLET = re.compile(r"^[ \t]*[-*]")
_FILL_OMIT_RE = re.compile(r"(^|[^a-zA-Z])(fill|omit)([^a-zA-Z]|$)")
_OMIT_PHRASE = re.compile(
    r"only when|omits the corresponding|no flag|never writes"
)
_LITERAL_HEADING = re.compile(
    r"^#+[ \t]+Populate frontmatter[ \t]*$", re.MULTILINE
)

# Together these allowlists must cover every SKILL.md surfaced by the discovery
# pass below.
IN_SCOPE_PRODUCERS: tuple[str, ...] = (
    "skills/work/create-work-item/SKILL.md",
    "skills/work/extract-work-items/SKILL.md",
    "skills/work/refine-work-item/SKILL.md",
    "skills/planning/create-plan/SKILL.md",
    "skills/github/describe-pr/SKILL.md",
    "skills/decisions/create-adr/SKILL.md",
    "skills/decisions/extract-adrs/SKILL.md",
    "skills/research/research-codebase/SKILL.md",
    "skills/research/research-issue/SKILL.md",
    "skills/design/inventory-design/SKILL.md",
    "skills/design/analyse-design-gaps/SKILL.md",
    "skills/planning/review-plan/SKILL.md",
    "skills/work/review-work-item/SKILL.md",
    "skills/github/review-pr/SKILL.md",
    "skills/planning/validate-plan/SKILL.md",
    "skills/notes/create-note/SKILL.md",
)
NON_EMITTER_TEMPLATE_CONSUMERS: tuple[str, ...] = (
    "skills/work/update-work-item/SKILL.md",
    "skills/work/list-work-items/SKILL.md",
)

_DISCOVERY_PATTERNS: tuple[re.Pattern[str], ...] = (
    re.compile(r"accelerator config template "),
    re.compile(r"^[ \t]*producer:"),
    re.compile(r"^[ \t]*schema_version:"),
    re.compile(r"^[ \t]*verdict:"),
    re.compile(r"^[ \t]*review_pass:"),
    re.compile(r"^[ \t]*review_target:"),
    re.compile(r"^[ \t]*target:"),
    re.compile(r"^[ \t]*result:"),
    re.compile(r"^[ \t]*pr_number:"),
)

EXPECTED_REVIEWER_HEADINGS = 4


def strip_template_directives(text: str) -> list[str]:
    """Return the body lines with ``!`accelerator config template`` directives
    removed (those are template-inclusion lines, not prose)."""
    return [
        line
        for line in text.splitlines()
        if not _TEMPLATE_DIRECTIVE.search(line)
    ]


def cli_flag_for(field: str) -> str:
    """The CLI flag spelling for a frontmatter field name."""
    if field == "blocks":
        return "block"
    return field.replace("_", "-")


def in_fenced_block(lines: list[str], field: str) -> bool:
    """The field appears as a YAML key inside a triple-backtick fenced block."""
    pattern = re.compile(r"^" + re.escape(field) + r":")
    in_block = False
    for line in lines:
        if line.startswith("```"):
            in_block = not in_block
            continue
        if in_block and pattern.search(line):
            return True
    return False


def in_imperative_section(lines: list[str], field: str) -> bool:
    """Inside a persistence-related section, both an imperative verb and a
    colon-anchored field reference appear (not necessarily on one line)."""
    field_pattern = re.compile(r"(^|[ \t]|`|\*)" + re.escape(field) + r":")
    state = {"in_section": False, "has_verb": False, "has_field": False}
    found = False

    def flush() -> bool:
        satisfied = (
            state["in_section"] and state["has_verb"] and state["has_field"]
        )
        state["has_verb"] = False
        state["has_field"] = False
        return satisfied

    for line in lines:
        if line.startswith("#"):
            found = flush() or found
            state["in_section"] = bool(_HEADING_RE.search(line.lower()))
            continue
        if state["in_section"]:
            if field_pattern.search(line):
                state["has_field"] = True
            if _VERBS_RE.search(line):
                state["has_verb"] = True
    return flush() or found


def has_cli_delegated_invocation(lines: list[str]) -> bool:
    """Any ``accelerator work create``/``update`` invocation is present."""
    return any(_WORK_CLI_RE.search(line) for line in lines)


def in_cli_delegated_block(lines: list[str], field: str) -> bool:
    """The field's CLI flag appears in a fenced work-create/update block."""
    flag = cli_flag_for(field)
    flag_pattern = re.compile(r"--" + re.escape(flag) + r"([^-]|$)")
    in_block = False
    has_cli = False
    has_flag = False
    for line in lines:
        if _FENCE_INDENTED.match(line):
            in_block = not in_block
            if in_block:
                has_cli = False
                has_flag = False
            continue
        if in_block:
            if _WORK_CLI_RE.search(line):
                has_cli = True
            if flag_pattern.search(line):
                has_flag = True
            if has_cli and has_flag:
                return True
    return False


def _omit_guidance_line(
    line: str, bracketed: bool, field_pattern: re.Pattern[str]
) -> bool:
    """A post-block lookahead line carries the omit explanation: the generic
    bracket convention (when a bracketed flag was seen) or a field-named
    note."""
    if not _OMIT_PHRASE.search(line):
        return False
    return bracketed or bool(field_pattern.search(line))


def in_cli_delegated_omit_guidance(lines: list[str], field: str) -> bool:
    """An omit-when-empty field delegated to a bracketed ``[--flag ...]`` CLI
    argument whose bracket convention is explained generically within a
    15-line lookahead after the block, or whose flag-less nature is stated."""
    flag = cli_flag_for(field)
    bracket_pattern = re.compile(r"\[--" + re.escape(flag) + r"([^-]|$)")
    field_pattern = re.compile(
        r"(^|[^a-zA-Z0-9_])" + re.escape(field) + r"([^a-zA-Z0-9_]|$)"
    )
    in_block = False
    has_cli = False
    bracketed = False
    after = 0
    for line in lines:
        if _FENCE_INDENTED.match(line):
            if in_block:
                in_block = False
                if has_cli:
                    after = 15
            else:
                in_block = True
                has_cli = False
                bracketed = False
            continue
        if in_block:
            if _WORK_CLI_RE.search(line):
                has_cli = True
            if bracket_pattern.search(line):
                bracketed = True
            continue
        if after > 0:
            if _omit_guidance_line(line, bracketed, field_pattern):
                return True
            after -= 1
    return False


def in_populate_section_with_guidance(lines: list[str], field: str) -> bool:
    """The field's OWN bullet, inside a Populate-frontmatter-ish section,
    carries a whole-word fill/omit guidance keyword. The keyword is bound to
    the bullet window naming the field, not the section as a whole."""
    field_pattern = re.compile(r"(^|[ \t]|`|\*)" + re.escape(field) + r":")
    state = {"in_section": False, "tracking": False, "saw": False}
    found = False

    def flush() -> bool:
        satisfied = state["in_section"] and state["tracking"] and state["saw"]
        state["tracking"] = False
        state["saw"] = False
        return satisfied

    for line in lines:
        if line.startswith("#"):
            found = flush() or found
            state["in_section"] = bool(_HEADING_RE.search(line.lower()))
            continue
        if state["in_section"]:
            if _BULLET.match(line):
                found = flush() or found
            if not state["tracking"] and field_pattern.search(line):
                state["tracking"] = True
                state["saw"] = False
            if state["tracking"] and _FILL_OMIT_RE.search(line):
                state["saw"] = True
    return flush() or found


def field_populated(lines: list[str], field: str) -> bool:
    """Whether any of the four population-instruction contexts covers a
    mandatory field."""
    return (
        in_fenced_block(lines, field)
        or in_imperative_section(lines, field)
        or in_cli_delegated_block(lines, field)
        or (
            field in CLI_MANAGED_NO_FLAG and has_cli_delegated_invocation(lines)
        )
    )


def omit_guidance_present(lines: list[str], field: str) -> bool:
    """Whether an omit-when-empty field carries fill/omit guidance in either
    supported context."""
    return in_populate_section_with_guidance(
        lines, field
    ) or in_cli_delegated_omit_guidance(lines, field)


def _schema_rows(schema_path: Path) -> list[list[str]]:
    return [line.split("\t") for line in schema_path.read_text().splitlines()]


def _field_count_violations(rows: list[list[str]]) -> list[str]:
    return [
        f"skills-schema.tsv:{number} has {len(row)} fields, expected 4"
        for number, row in enumerate(rows, start=1)
        if len(row) != 4
    ]


def _discovered_skills(root: Path) -> set[str]:
    discovered: set[str] = set()
    for path in (root / "skills").rglob("SKILL.md"):
        lines = path.read_text().splitlines()
        if any(
            pattern.search(line)
            for pattern in _DISCOVERY_PATTERNS
            for line in lines
        ):
            discovered.add(path.relative_to(root).as_posix())
    return discovered


def _discovery_violations(root: Path) -> list[str]:
    allowlist = set(IN_SCOPE_PRODUCERS) | set(NON_EMITTER_TEMPLATE_CONSUMERS)
    unexpected = _discovered_skills(root) - allowlist
    return [
        f"{skill}: SKILL.md surfaced by discovery pass but not allowlisted"
        for skill in sorted(unexpected)
    ]


def _skill_row_violations(
    root: Path, skill_path: str, fields: str, omit_when_empty: str
) -> tuple[list[str], bool]:
    """Population, omit-guidance and reviewer-heading violations for one row,
    plus whether it is a reviewer row that carries the literal heading."""
    full = root / skill_path
    if not full.is_file():
        return [f"{skill_path} — SKILL.md not found"], False

    text = full.read_text()
    stripped = strip_template_directives(text)

    found: list[str] = [
        f"{skill_path}: no instruction to populate '{field}'"
        for field in fields.split()
        if not field_populated(stripped, field)
    ]
    found.extend(
        f"{skill_path}: omit-when-empty field '{field}' missing or lacks "
        "fill/omit guidance in Populate frontmatter section"
        for field in omit_when_empty.split()
        if field != "-" and not omit_guidance_present(stripped, field)
    )

    reviewer_heading = False
    if " target " in f" {fields} ":
        if _LITERAL_HEADING.search(text):
            reviewer_heading = True
        else:
            found.append(
                f"{skill_path}: reviewer producer lacks a literal "
                "'Populate frontmatter' heading"
            )
    return found, reviewer_heading


def violations(root: Path, schema_path: Path) -> list[str]:
    """Every population, omit-guidance, reviewer-heading, field-count and
    discovery violation across the skills named in ``schema_path``."""
    rows = _schema_rows(schema_path)
    found = _field_count_violations(rows)

    reviewer_headings = 0
    for row in rows[1:]:
        if len(row) != 4:
            continue
        skill_path, _producer, fields, omit_when_empty = row
        row_found, reviewer_heading = _skill_row_violations(
            root, skill_path, fields, omit_when_empty
        )
        found.extend(row_found)
        reviewer_headings += int(reviewer_heading)

    if reviewer_headings != EXPECTED_REVIEWER_HEADINGS:
        found.append(
            f"reviewer literal-heading count is {reviewer_headings}, "
            f"expected {EXPECTED_REVIEWER_HEADINGS}"
        )

    found.extend(_discovery_violations(root))
    return found


# --------------------------------------------------------------------------
# Synthetic branch and negative tests.
# --------------------------------------------------------------------------


def test_fenced_block_populates_a_field() -> None:
    body = ["```yaml", "producer: create-work-item", "```"]
    assert in_fenced_block(body, "producer")
    assert field_populated(body, "producer")


def test_a_field_outside_any_fence_is_not_fenced() -> None:
    assert not in_fenced_block(["producer: create-work-item"], "producer")


def test_imperative_section_populates_a_field() -> None:
    body = [
        "## Populate frontmatter",
        "Substitute every field below:",
        "- `producer:` the producer name",
    ]
    assert in_imperative_section(body, "producer")
    assert field_populated(body, "producer")


def test_imperative_field_outside_a_persistence_section_is_not_populated() -> (
    None
):
    body = [
        "## Overview",
        "Substitute every field below:",
        "- `producer:` the producer name",
    ]
    assert not in_imperative_section(body, "producer")


def test_imperative_section_without_a_verb_is_not_populated() -> None:
    body = ["## Populate frontmatter", "- `producer:` the producer name"]
    assert not in_imperative_section(body, "producer")


def test_cli_delegated_block_populates_a_field() -> None:
    body = [
        "```bash",
        "accelerator work create <title> \\",
        '  [--parent "work-item:NNNN"]',
        "```",
    ]
    assert in_cli_delegated_block(body, "parent")
    assert field_populated(body, "parent")


def test_cli_flag_for_maps_blocks_and_underscores() -> None:
    assert cli_flag_for("blocks") == "block"
    assert cli_flag_for("blocked_by") == "blocked-by"
    assert cli_flag_for("parent") == "parent"


def test_cli_flag_prefix_does_not_match_a_longer_flag() -> None:
    body = [
        "```bash",
        "accelerator work create <title> [--block-list x]",
        "```",
    ]
    assert not in_cli_delegated_block(body, "blocks")


def test_cli_managed_no_flag_field_is_populated_by_any_invocation() -> None:
    body = ["```bash", "accelerator work create <title>", "```"]
    assert field_populated(body, "schema_version")
    assert not in_fenced_block(body, "schema_version")


def test_cli_managed_no_flag_needs_an_invocation() -> None:
    assert not field_populated(["no cli here"], "schema_version")


def test_a_missing_population_instruction_is_not_populated() -> None:
    assert not field_populated(["## Overview", "nothing here"], "producer")


def test_populate_section_with_a_fill_omit_bullet_is_accepted() -> None:
    body = [
        "### Populate frontmatter",
        "",
        "- `parent:` the parent ref. Fill when named; otherwise omit.",
    ]
    assert in_populate_section_with_guidance(body, "parent")
    assert omit_guidance_present(body, "parent")


def test_populate_section_without_a_fill_omit_note_is_rejected() -> None:
    body = [
        "### Populate frontmatter",
        "",
        "- `parent:` the parent ref. Set it to the parent work item id.",
    ]
    assert not in_populate_section_with_guidance(body, "parent")


def test_fill_omit_note_bound_to_a_different_field_is_rejected() -> None:
    body = [
        "### Populate frontmatter",
        "",
        "- `parent:` the parent ref. Set it to the parent work item id.",
        "- `source:` the source ref. Fill when explicit; otherwise omit.",
    ]
    assert not in_populate_section_with_guidance(body, "parent")


def test_a_buried_fill_omit_substring_is_rejected() -> None:
    body = [
        "### Populate frontmatter",
        "",
        "- `parent:` the parent ref. We backfill this during reconciliation.",
    ]
    assert not in_populate_section_with_guidance(body, "parent")


def test_guidance_under_a_bold_lead_in_is_rejected() -> None:
    body = [
        "**Populate frontmatter**:",
        "",
        "- `parent:` the parent ref. Fill when named; otherwise omit.",
    ]
    assert not in_populate_section_with_guidance(body, "parent")


def test_cli_delegated_omit_guidance_accepts_a_bracketed_flag() -> None:
    body = [
        "```bash",
        "accelerator work create <title> \\",
        '  [--parent "work-item:NNNN"]',
        "```",
        "Include each bracketed flag only when that field has a value.",
    ]
    assert in_cli_delegated_omit_guidance(body, "parent")
    assert omit_guidance_present(body, "parent")


def test_cli_delegated_omit_guidance_accepts_a_flagless_field() -> None:
    body = [
        "```bash",
        "accelerator work create <title>",
        "```",
        "`external_id` has no flag — work create never writes it.",
    ]
    assert in_cli_delegated_omit_guidance(body, "external_id")


def test_cli_delegated_omit_guidance_needs_the_explanatory_phrase() -> None:
    body = [
        "```bash",
        "accelerator work create <title> \\",
        '  [--parent "work-item:NNNN"]',
        "```",
        "Set the parent to the parent work item id.",
    ]
    assert not in_cli_delegated_omit_guidance(body, "parent")


def test_omit_guidance_beyond_the_lookahead_window_is_rejected() -> None:
    body = (
        [
            "```bash",
            "accelerator work create <title> \\",
            '  [--parent "work-item:NNNN"]',
            "```",
        ]
        + ["filler line"] * 16
        + ["Include each bracketed flag only when that field has a value."]
    )
    assert not in_cli_delegated_omit_guidance(body, "parent")


# --------------------------------------------------------------------------
# Integration tests over synthetic trees + schema TSVs.
# --------------------------------------------------------------------------


def _write_skill(root: Path, rel: str, body: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)


def _write_schema(root: Path, rows: list[str]) -> Path:
    schema = root / "schema.tsv"
    header = "skill_path\tproducer_name\tfields_to_assert\tomit_when_empty"
    schema.write_text("\n".join([header, *rows]) + "\n")
    return schema


def test_a_conforming_skill_yields_no_population_violation(
    tmp_path: Path,
) -> None:
    _write_skill(
        tmp_path,
        "skills/notes/create-note/SKILL.md",
        "```yaml\nproducer: create-note\n```\n",
    )
    schema = _write_schema(
        tmp_path,
        ["skills/notes/create-note/SKILL.md\tcreate-note\tproducer\t-"],
    )
    offenders = violations(tmp_path, schema)
    assert not any("populate 'producer'" in v for v in offenders)


def test_a_missing_populate_instruction_is_flagged(tmp_path: Path) -> None:
    _write_skill(
        tmp_path,
        "skills/notes/create-note/SKILL.md",
        "## Overview\n\nNo frontmatter guidance here.\n",
    )
    schema = _write_schema(
        tmp_path,
        ["skills/notes/create-note/SKILL.md\tcreate-note\tproducer\t-"],
    )
    assert any(
        "no instruction to populate 'producer'" in v
        for v in violations(tmp_path, schema)
    )


def test_a_missing_omit_guidance_is_flagged(tmp_path: Path) -> None:
    _write_skill(
        tmp_path,
        "skills/notes/create-note/SKILL.md",
        "```yaml\nproducer: create-note\n```\n"
        "### Populate frontmatter\n\n- `parent:` the parent ref, set it.\n",
    )
    schema = _write_schema(
        tmp_path,
        ["skills/notes/create-note/SKILL.md\tcreate-note\tproducer\tparent"],
    )
    assert any(
        "omit-when-empty field 'parent'" in v
        for v in violations(tmp_path, schema)
    )


def test_a_missing_skill_is_flagged(tmp_path: Path) -> None:
    schema = _write_schema(
        tmp_path,
        ["skills/notes/create-note/SKILL.md\tcreate-note\tproducer\t-"],
    )
    assert any("SKILL.md not found" in v for v in violations(tmp_path, schema))


def test_a_malformed_schema_row_is_flagged(tmp_path: Path) -> None:
    schema = tmp_path / "schema.tsv"
    schema.write_text(
        "skill_path\tproducer_name\tfields_to_assert\tomit_when_empty\n"
        "only\ttwo\n"
    )
    assert any("expected 4" in v for v in violations(tmp_path, schema))


def test_a_reviewer_row_without_the_literal_heading_is_flagged(
    tmp_path: Path,
) -> None:
    _write_skill(
        tmp_path,
        "skills/planning/review-plan/SKILL.md",
        "```yaml\ntarget: x\n```\n",
    )
    schema = _write_schema(
        tmp_path,
        ["skills/planning/review-plan/SKILL.md\treview-plan\ttarget\t-"],
    )
    assert any(
        "literal 'Populate frontmatter' heading" in v
        for v in violations(tmp_path, schema)
    )


def test_a_discovered_skill_outside_the_allowlist_is_flagged(
    tmp_path: Path,
) -> None:
    _write_skill(
        tmp_path,
        "skills/rogue/unknown/SKILL.md",
        "```yaml\nproducer: unknown\n```\n",
    )
    schema = _write_schema(tmp_path, [])
    assert any("not allowlisted" in v for v in violations(tmp_path, schema))


# --------------------------------------------------------------------------
# Live tree.
# --------------------------------------------------------------------------


def test_the_real_skills_tree_passes() -> None:
    assert violations(REPO_ROOT, SCHEMA_TSV) == []
