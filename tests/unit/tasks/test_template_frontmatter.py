"""Port of the retired template-frontmatter shell guard (template shape).

For each row in ``templates-schema.tsv`` this parses the YAML frontmatter block
at the head of the named template and asserts the unified base fields, the
provenance bundle (when code-state-anchored), the per-type extras, the
status-comment vocabulary, the typed-linkage slot grammar, the closed linkage
set, and the absence of any legacy own-identity key. It also self-checks the
TSV's field count and cross-checks the work-item Schema Reference tables against
the TSV. See ADR-0033 / ADR-0034 / ADR-0040 for the contract.

The cross-cutting emission rules are single-sourced from
``tasks.lint.frontmatter_rules`` so this surface cannot drift from the corpus
validator.
"""

import re
from pathlib import Path

from tasks.lint import frontmatter_rules as fr

REPO_ROOT = Path(__file__).resolve().parents[3]

SCHEMA_TSV_RELPATH = (
    "cli/corpus/src/frontmatter_validation/templates-schema.tsv"
)

# Base fields from the shared emission rules, plus the two the template surface
# additionally pins on every template. The corpus validator does not require
# those two, so they are appended here rather than living in fr.BASE_FIELDS.
BASE_FIELDS: tuple[str, ...] = (*fr.BASE_FIELDS, "producer", "status")

# Each listed work item carries a Schema Reference table; the union of those
# tables must match the TSV exactly.
WORK_ITEM_MDS: tuple[str, ...] = (
    "meta/work/0065-update-artifact-templates-to-unified-schema.md",
    "meta/work/0066-update-review-skills-inline-frontmatter.md",
    "meta/work/0067-create-note-skill.md",
)

_SCHEMA_TAB_FIELDS = 7

_SCHEMA_REF_ROW = re.compile(r"^\|[ \t]+`([a-z0-9-]+\.md)`[ \t]+\|")


def _matches(block: str, pattern: str) -> bool:
    """Whether any line of ``block`` matches ``pattern`` (grep -qE parity)."""
    return re.search(pattern, block, re.MULTILINE) is not None


def extract_frontmatter(text: str) -> str:
    """Lines between the first two ``---`` fences, CRLF-normalised."""
    collected: list[str] = []
    state = 0
    for line in text.replace("\r", "").split("\n"):
        if re.match(r"^---[ \t]*$", line):
            if state == 0:
                state = 1
                continue
            if state == 1:
                break
        if state == 1:
            collected.append(line)
    return "\n".join(collected)


def check_linkage_slot(block: str, key: str) -> int:
    """Return 0 if the slot's shape+comment is valid, 1 if rejected, 2 unknown.

    For ``blocked_by`` the standalone inverse-guidance line must also be
    present. Mirrors the shell ``check_linkage_slot`` return-code contract.
    """
    cardinality = fr.linkage_cardinality(key)
    if cardinality == "single":
        regex = (
            rf'^{key}:[ \t]+""[ \t]+#[ \t]+typed-linkage[ \t]+ref:[ \t]+'
            rf'"({fr.SOURCE_TYPE_RE}):[A-Za-z0-9-]+"[ \t]+or[ \t]+""$'
        )
    elif cardinality == "list":
        regex = (
            rf"^{key}:[ \t]+\[\][ \t]+#[ \t]+typed-linkage[ \t]+list:[ \t]+"
            rf'\["({fr.SOURCE_TYPE_RE}):[A-Za-z0-9-]+",[ \t]+\.\.\.\]'
            rf"[ \t]+or[ \t]+\[\]$"
        )
    else:
        return 2
    if not _matches(block, regex):
        return 1
    if key == "blocked_by" and fr.INVERSE_GUIDANCE_LINE not in block:
        return 1
    return 0


def check_closed_set(block: str, extras: str, keys: str) -> bool:
    """Whether ``block`` carries no spurious linkage-vocabulary key.

    A vocabulary key present in the block is permitted only when it is a
    declared slot (``keys``) or a declared extra (``extras``).
    """
    declared = set(extras.split()) | set(keys.split())
    for vkey in fr.LINKAGE_VOCABULARY:
        if not _matches(block, rf"^{vkey}:[ \t]"):
            continue
        if vkey in declared:
            continue
        return False
    return True


def _presence_violations(
    template_file: str, expected_type: str, block: str
) -> list[str]:
    found = [
        f"{template_file}: base field '{field}' missing"
        for field in BASE_FIELDS
        if not _matches(block, rf"^{field}:[ \t]")
    ]
    if not _matches(block, rf"^type:[ \t]+{expected_type}([ \t]+#.*)?$"):
        found.append(f"{template_file}: type is not '{expected_type}'")
    if not _matches(block, fr.SCHEMA_VERSION_RE.pattern):
        found.append(f"{template_file}: schema_version is not bare integer 1")
    if not _matches(block, fr.ID_QUOTED_RE.pattern):
        found.append(f"{template_file}: id value is not a quoted string")
    return found


def _own_id_violations(
    template_file: str, forbidden_own_id_key: str, block: str
) -> list[str]:
    if forbidden_own_id_key == "-":
        return []
    return [
        f"{template_file}: legacy own-id key '{fkey}' present"
        for fkey in forbidden_own_id_key.split()
        if _matches(block, rf"^{fkey}:[ \t]")
    ]


def _provenance_violations(
    template_file: str, anchored: str, block: str
) -> list[str]:
    found: list[str] = []
    if anchored == "yes":
        found += [
            f"{template_file}: provenance field '{pfield}' missing"
            for pfield in fr.PROVENANCE_FIELDS
            if not _matches(block, rf"^{pfield}:[ \t]")
        ]
    found += [
        f"{template_file}: forbidden provenance field '{pfield}' present"
        for pfield in fr.FORBIDDEN_PROVENANCE_FIELDS
        if _matches(block, rf"^{pfield}:[ \t]")
    ]
    return found


def _extras_violations(
    template_file: str, extras: str, block: str
) -> list[str]:
    return [
        f"{template_file}: extra '{extra}' missing"
        for extra in extras.split()
        if not _matches(block, rf"^{extra}:[ \t]")
    ]


def _linkage_violations(
    template_file: str, extras: str, typed_linkage_keys: str, block: str
) -> list[str]:
    found: list[str] = []
    for lkey in typed_linkage_keys.split():
        rc = check_linkage_slot(block, lkey)
        if rc == 2:
            found.append(f"{template_file}: unknown linkage key '{lkey}'")
        elif rc != 0:
            found.append(
                f"{template_file}: linkage slot '{lkey}' bad shape/comment "
                "(or missing inverse-guidance line)"
            )
    if not check_closed_set(block, extras, typed_linkage_keys):
        found.append(
            f"{template_file}: closed-set violated (a linkage key not in the "
            "TSV row)"
        )
    return found


def _status_violations(
    template_file: str, status_vocab: str, block: str
) -> list[str]:
    status_line = "\n".join(
        line for line in block.split("\n") if re.match(r"^status:[ \t]", line)
    )
    if not status_line:
        return [f"{template_file}: no status line"]
    if status_vocab not in status_line:
        return [
            f"{template_file}: status line missing pinned vocabulary "
            f"'{status_vocab}'"
        ]
    return []


def _check_row(
    template_file: str,
    expected_type: str,
    anchored: str,
    extras: str,
    status_vocab: str,
    forbidden_own_id_key: str,
    typed_linkage_keys: str,
    block: str,
) -> list[str]:
    """Every shape violation for one template's frontmatter block."""
    return [
        *_presence_violations(template_file, expected_type, block),
        *_own_id_violations(template_file, forbidden_own_id_key, block),
        *_provenance_violations(template_file, anchored, block),
        *_extras_violations(template_file, extras, block),
        *_linkage_violations(template_file, extras, typed_linkage_keys, block),
        *_status_violations(template_file, status_vocab, block),
    ]


def _schema_ref_templates(root: Path, existing: list[str]) -> list[str]:
    """Template filenames named in the work items' Schema Reference tables."""
    names: list[str] = []
    for work_item in existing:
        in_section = False
        for line in (root / work_item).read_text().split("\n"):
            if re.match(r"^## Schema Reference", line):
                in_section = True
                continue
            if in_section and re.match(r"^## ", line):
                in_section = False
            if in_section:
                match = _SCHEMA_REF_ROW.match(line)
                if match:
                    names.append(match.group(1))
    return names


def _cross_check(root: Path, tsv_lines: list[str]) -> list[str]:
    """Assert the work-item Schema Reference union matches the TSV exactly."""
    existing = [wi for wi in WORK_ITEM_MDS if (root / wi).is_file()]
    if not existing:
        return []
    wi_templates = sorted(_schema_ref_templates(root, existing))
    tsv_templates = sorted(line.split("\t")[0] for line in tsv_lines[1:])
    if wi_templates != tsv_templates:
        return [
            "work-item Schema Reference templates differ from TSV "
            f"(work-item={wi_templates}, tsv={tsv_templates})"
        ]
    return []


def violations(root: Path) -> list[str]:
    """Every template-shape, self-check, and cross-check violation."""
    tsv_path = root / SCHEMA_TSV_RELPATH
    if not tsv_path.is_file():
        return [f"templates-schema.tsv unreadable at {tsv_path}"]

    lines = tsv_path.read_text().splitlines()
    if not [line for line in lines[1:] if line.strip()]:
        return [f"templates-schema.tsv has no rows at {tsv_path}"]

    field_count_errors = [
        f"templates-schema.tsv:{index} has {len(line.split(chr(9)))} "
        f"fields, expected {_SCHEMA_TAB_FIELDS}"
        for index, line in enumerate(lines, start=1)
        if len(line.split("\t")) != _SCHEMA_TAB_FIELDS
    ]
    if field_count_errors:
        return field_count_errors

    found: list[str] = []
    for line in lines[1:]:
        (
            template_file,
            expected_type,
            anchored,
            extras,
            status_vocab,
            forbidden_own_id_key,
            typed_linkage_keys,
        ) = line.split("\t")
        template_path = root / "templates" / template_file
        if not template_path.is_file():
            found.append(
                f"{template_file}: template file not found at "
                f"templates/{template_file}"
            )
            continue
        block = extract_frontmatter(template_path.read_text())
        if not block:
            found.append(
                f"{template_file}: frontmatter block is empty or missing"
            )
            continue
        found.extend(
            _check_row(
                template_file,
                expected_type,
                anchored,
                extras,
                status_vocab,
                forbidden_own_id_key,
                typed_linkage_keys,
                block,
            )
        )

    found.extend(_cross_check(root, lines))
    return found


# --------------------------------------------------------------------------- #
# Synthetic-tree fixtures.                                                     #
# --------------------------------------------------------------------------- #

_HEADER = "\t".join(fr.SCHEMA_COLUMNS)
_STATUS_VOCAB = "captured | archived"
_ROW = f"demo.md\tdemo-type\tno\t\t{_STATUS_VOCAB}\t-\tparent"

_CONFORMING_FRONTMATTER = (
    "---\n"
    "type: demo-type\n"
    'id: "NNNN"\n'
    'title: "T"\n'
    'date: "2026-01-01T00:00:00+00:00"\n'
    "author: A\n"
    "producer: create-demo\n"
    "status: captured # captured | archived\n"
    "tags: []\n"
    'last_updated: "2026-01-01T00:00:00+00:00"\n'
    "last_updated_by: A\n"
    "schema_version: 1\n"
    'parent: "" # typed-linkage ref: "work-item:NNNN" or ""\n'
    "---\n\n# body\n"
)


def _write_tree(
    root: Path,
    *,
    body: str | None = None,
    header: str = _HEADER,
    row: str = _ROW,
) -> Path:
    tsv = root / SCHEMA_TSV_RELPATH
    tsv.parent.mkdir(parents=True, exist_ok=True)
    tsv.write_text(f"{header}\n{row}\n")
    template = root / "templates" / "demo.md"
    template.parent.mkdir(parents=True, exist_ok=True)
    template.write_text(body if body is not None else _CONFORMING_FRONTMATTER)
    return root


def _write_work_item(root: Path, name: str, template_names: list[str]) -> None:
    rows = "\n".join(f"| `{n}` | `x` | 1 | no | none |" for n in template_names)
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "# WI\n\n## Schema Reference\n\n"
        "| Template file | type | v | prov | extras |\n"
        "|---|---|---|---|---|\n"
        f"{rows}\n\n## Next\n"
    )


# --------------------------------------------------------------------------- #
# Conforming synthetic tree.                                                   #
# --------------------------------------------------------------------------- #


def test_a_conforming_template_yields_no_violations(tmp_path: Path) -> None:
    assert violations(_write_tree(tmp_path)) == []


# --------------------------------------------------------------------------- #
# Per-check negatives against the synthetic tree.                             #
# --------------------------------------------------------------------------- #


def test_missing_base_field_is_flagged(tmp_path: Path) -> None:
    body = _CONFORMING_FRONTMATTER.replace("producer: create-demo\n", "")
    flagged = violations(_write_tree(tmp_path, body=body))
    assert any("base field 'producer' missing" in v for v in flagged)


def test_wrong_type_is_flagged(tmp_path: Path) -> None:
    body = _CONFORMING_FRONTMATTER.replace("type: demo-type", "type: wrong")
    flagged = violations(_write_tree(tmp_path, body=body))
    assert any("type is not 'demo-type'" in v for v in flagged)


def test_non_integer_schema_version_is_flagged(tmp_path: Path) -> None:
    body = _CONFORMING_FRONTMATTER.replace(
        "schema_version: 1", "schema_version: 2"
    )
    flagged = violations(_write_tree(tmp_path, body=body))
    assert any("schema_version is not bare integer 1" in v for v in flagged)


def test_unquoted_id_is_flagged(tmp_path: Path) -> None:
    body = _CONFORMING_FRONTMATTER.replace('id: "NNNN"', "id: NNNN")
    flagged = violations(_write_tree(tmp_path, body=body))
    assert any("id value is not a quoted string" in v for v in flagged)


def test_present_legacy_own_id_key_is_flagged(tmp_path: Path) -> None:
    body = _CONFORMING_FRONTMATTER.replace(
        "schema_version: 1", "schema_version: 1\nold_id: x"
    )
    row = f"demo.md\tdemo-type\tno\t\t{_STATUS_VOCAB}\told_id\tparent"
    flagged = violations(_write_tree(tmp_path, body=body, row=row))
    assert any("legacy own-id key 'old_id' present" in v for v in flagged)


def test_missing_provenance_bundle_is_flagged(tmp_path: Path) -> None:
    row = f"demo.md\tdemo-type\tyes\t\t{_STATUS_VOCAB}\t-\tparent"
    flagged = violations(_write_tree(tmp_path, row=row))
    assert any("provenance field 'revision' missing" in v for v in flagged)


def test_present_forbidden_provenance_field_is_flagged(tmp_path: Path) -> None:
    body = _CONFORMING_FRONTMATTER.replace(
        "schema_version: 1", "schema_version: 1\ngit_commit: abc"
    )
    flagged = violations(_write_tree(tmp_path, body=body))
    assert any(
        "forbidden provenance field 'git_commit' present" in v for v in flagged
    )


def test_missing_extra_is_flagged(tmp_path: Path) -> None:
    row = f"demo.md\tdemo-type\tno\ttopic\t{_STATUS_VOCAB}\t-\tparent"
    flagged = violations(_write_tree(tmp_path, row=row))
    assert any("extra 'topic' missing" in v for v in flagged)


def test_bad_linkage_slot_shape_is_flagged(tmp_path: Path) -> None:
    body = _CONFORMING_FRONTMATTER.replace(
        '# typed-linkage ref: "work-item:NNNN" or ""', "# see ADR-0034"
    )
    flagged = violations(_write_tree(tmp_path, body=body))
    assert any("linkage slot 'parent' bad shape/comment" in v for v in flagged)


def test_unknown_linkage_key_is_flagged(tmp_path: Path) -> None:
    row = f"demo.md\tdemo-type\tno\t\t{_STATUS_VOCAB}\t-\tbogus"
    flagged = violations(_write_tree(tmp_path, row=row))
    assert any("unknown linkage key 'bogus'" in v for v in flagged)


def test_spurious_linkage_key_is_flagged(tmp_path: Path) -> None:
    extra_line = (
        'relates_to: [] # typed-linkage list: ["work-item:NNNN", ...] or []'
    )
    body = _CONFORMING_FRONTMATTER.replace(
        "schema_version: 1", f"schema_version: 1\n{extra_line}"
    )
    flagged = violations(_write_tree(tmp_path, body=body))
    assert any("closed-set violated" in v for v in flagged)


def test_wrong_status_vocabulary_is_flagged(tmp_path: Path) -> None:
    body = _CONFORMING_FRONTMATTER.replace("# captured | archived", "# draft")
    flagged = violations(_write_tree(tmp_path, body=body))
    assert any("status line missing pinned vocabulary" in v for v in flagged)


def test_missing_template_file_is_flagged(tmp_path: Path) -> None:
    row = f"absent.md\tdemo-type\tno\t\t{_STATUS_VOCAB}\t-\tparent"
    tsv = tmp_path / SCHEMA_TSV_RELPATH
    tsv.parent.mkdir(parents=True, exist_ok=True)
    tsv.write_text(f"{_HEADER}\n{row}\n")
    flagged = violations(tmp_path)
    assert any("template file not found" in v for v in flagged)


def test_empty_frontmatter_is_flagged(tmp_path: Path) -> None:
    flagged = violations(_write_tree(tmp_path, body="no frontmatter here\n"))
    assert any("frontmatter block is empty or missing" in v for v in flagged)


def test_field_count_self_check_is_flagged(tmp_path: Path) -> None:
    short_row = f"demo.md\tdemo-type\tno\t\t{_STATUS_VOCAB}\t-"
    flagged = violations(_write_tree(tmp_path, row=short_row))
    assert any("has 6 fields, expected 7" in v for v in flagged)


def test_empty_tsv_is_flagged(tmp_path: Path) -> None:
    tsv = tmp_path / SCHEMA_TSV_RELPATH
    tsv.parent.mkdir(parents=True, exist_ok=True)
    tsv.write_text(f"{_HEADER}\n")
    assert any("has no rows" in v for v in violations(tmp_path))


# --------------------------------------------------------------------------- #
# Cross-check against the work-item Schema Reference tables.                   #
# --------------------------------------------------------------------------- #


def test_matching_schema_reference_passes_cross_check(tmp_path: Path) -> None:
    _write_tree(tmp_path)
    _write_work_item(tmp_path, WORK_ITEM_MDS[0], ["demo.md"])
    assert violations(tmp_path) == []


def test_diverging_schema_reference_is_flagged(tmp_path: Path) -> None:
    _write_tree(tmp_path)
    _write_work_item(tmp_path, WORK_ITEM_MDS[0], ["other.md"])
    flagged = violations(tmp_path)
    assert any(
        "Schema Reference templates differ from TSV" in v for v in flagged
    )


# --------------------------------------------------------------------------- #
# Negative-fixture self-test: each pure check must reject known-bad input.     #
# --------------------------------------------------------------------------- #


def test_list_slot_rejects_a_single_ref_value() -> None:
    fixture = 'blocks: "" # typed-linkage list: ["work-item:NNNN", ...] or []'
    assert check_linkage_slot(fixture, "blocks") != 0


def test_slot_rejects_a_malformed_comment() -> None:
    assert check_linkage_slot('parent: "" # see ADR-0034', "parent") != 0


def test_blocked_by_rejects_a_missing_inverse_guidance_line() -> None:
    fixture = (
        'blocked_by: [] # typed-linkage list: ["work-item:NNNN", ...] or []'
    )
    assert check_linkage_slot(fixture, "blocked_by") != 0


def test_blocked_by_accepts_the_inverse_guidance_line() -> None:
    fixture = (
        'blocked_by: [] # typed-linkage list: ["work-item:NNNN", ...] or []\n'
        f"{fr.INVERSE_GUIDANCE_LINE}"
    )
    assert check_linkage_slot(fixture, "blocked_by") == 0


def test_closed_set_rejects_a_key_absent_from_the_row() -> None:
    fixture = (
        'relates_to: [] # typed-linkage list: ["work-item:NNNN", ...] or []'
    )
    assert check_closed_set(fixture, "", "parent") is False


def test_slot_rejects_an_absent_declared_slot() -> None:
    assert check_linkage_slot('title: "x"', "parent") != 0


def test_slot_rejects_an_out_of_vocabulary_source_type() -> None:
    fixture = 'parent: "" # typed-linkage ref: "ticket:NNNN" or ""'
    assert check_linkage_slot(fixture, "parent") != 0


def test_unknown_key_is_reported_distinctly() -> None:
    assert check_linkage_slot('bogus: ""', "bogus") == 2


# --------------------------------------------------------------------------- #
# Vocabulary-drift guard: every vocabulary key must have a cardinality.        #
# --------------------------------------------------------------------------- #


def test_every_vocabulary_key_has_a_cardinality() -> None:
    for vkey in fr.LINKAGE_VOCABULARY:
        assert fr.linkage_cardinality(vkey) != ""


# --------------------------------------------------------------------------- #
# Live tree: the shipped templates and TSV must be clean.                      #
# --------------------------------------------------------------------------- #


def test_the_real_templates_tree_passes() -> None:
    assert violations(REPO_ROOT) == []
