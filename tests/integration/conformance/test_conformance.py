"""Producer-conformance guard: drives the real corpus validator.

Ports the validator-driving half of the retired shell conformance guard. For
each frontmatter-emitting SKILL.md it extracts the hard-coded literals, derives
the enforced attribute set
from `templates-schema.tsv` and `frontmatter_rules`, and runs `accelerator
corpus frontmatter validate` over a synthesised fixture — asserting acceptance,
plus a per-axis negative self-test proving the guard is wired.

The launcher is resolved from `ACCELERATOR_BIN` (the integration lane sets it
and `ACCELERATOR_CORPUS_BIN` via `accelerator_env(corpus_bin=True)`, and builds
it via `build:cli:dev`). This lane fails rather than skips when the launcher is
absent: a skipped conformance guard is indistinguishable from a passing one.
"""

import os
import re
import subprocess
from pathlib import Path

import pytest

from tasks.lint import frontmatter_rules as fr

REPO_ROOT = Path(__file__).resolve().parents[3]
SCHEMA_TSV = (
    REPO_ROOT / "cli/corpus/src/frontmatter_validation/templates-schema.tsv"
)
TEMPLATES_DIR = REPO_ROOT / "templates"

EMITTERS = (
    "skills/work/create-work-item/SKILL.md",
    "skills/work/extract-work-items/SKILL.md",
    "skills/work/refine-work-item/SKILL.md",
    "skills/work/review-work-item/SKILL.md",
    "skills/planning/create-plan/SKILL.md",
    "skills/planning/review-plan/SKILL.md",
    "skills/planning/validate-plan/SKILL.md",
    "skills/decisions/create-adr/SKILL.md",
    "skills/decisions/extract-adrs/SKILL.md",
    "skills/research/research-codebase/SKILL.md",
    "skills/research/research-issue/SKILL.md",
    "skills/design/inventory-design/SKILL.md",
    "skills/design/analyse-design-gaps/SKILL.md",
    "skills/github/describe-pr/SKILL.md",
    "skills/github/review-pr/SKILL.md",
    "skills/notes/create-note/SKILL.md",
)
# Surfaced by discovery but out of scope: migrate is a corpus transformer with
# no full-block emission.
EXCLUDED = ("skills/config/migrate/SKILL.md",)
# Status-transition mutators: not surfaced by discovery; asserted on the status
# axis only.
STATUS_AXIS = (
    "skills/planning/validate-plan/SKILL.md",
    "skills/decisions/review-adr/SKILL.md",
)

DISCOVERY_RE = re.compile(
    r"schema_version:|Populate frontmatter|Substitute .*frontmatter|"
    r"frontmatter-emission|artifact-derive-metadata\.sh"
)


def _launcher() -> str:
    return os.environ.get(
        "ACCELERATOR_BIN", str(REPO_ROOT / "cli/target/debug/accelerator")
    )


def _schema() -> dict[str, dict[str, str]]:
    rows = SCHEMA_TSV.read_text(encoding="utf-8").splitlines()
    table: dict[str, dict[str, str]] = {}
    for line in rows[1:]:
        if not line.strip():
            continue
        fields = line.split("\t")
        tmpl, type_, anchored, extras, vocab, forbidden, linkkeys = fields[:7]
        table[type_] = {
            "template": tmpl,
            "anchored": anchored,
            "extras": extras,
            "vocab": vocab,
            "forbidden": forbidden,
            "linkkeys": linkkeys,
        }
    return table


SCHEMA = _schema()


def _discovered() -> list[str]:
    found = []
    for path in sorted((REPO_ROOT / "skills").rglob("SKILL.md")):
        text = path.read_text(encoding="utf-8")
        if any(DISCOVERY_RE.search(line) for line in text.splitlines()):
            found.append(path.relative_to(REPO_ROOT).as_posix())
    return found


def _status_in_vocab(status: str, vocab: str) -> bool:
    return status in {token.strip() for token in vocab.split("|")}


def _template_keys(template: str) -> set[str]:
    keys: set[str] = set()
    fences = 0
    for line in (
        (TEMPLATES_DIR / template).read_text(encoding="utf-8").splitlines()
    ):
        if re.match(r"^---[ \t]*$", line):
            fences += 1
            if fences == 2:
                break
            continue
        if fences == 1 and re.match(r"^[A-Za-z_][A-Za-z0-9_]*:", line):
            keys.add(line.split(":", 1)[0])
    return keys


def _extract_cli_literal(text: str, field: str) -> str:
    if not re.search(r"accelerator work (create|update)", text):
        return ""
    if field == "type":
        return "work-item"
    if field == "schema_version":
        return "1"
    if field in ("status", "producer"):
        in_block = False
        for line in text.splitlines():
            if re.match(r"^[ \t]*```", line):
                in_block = not in_block
                continue
            if in_block and re.search(rf"--{field}[ \t]", line):
                value = re.sub(rf".*--{field}[ \t]+", "", line)
                value = re.sub(r'^"', "", value)
                value = re.sub(r"\\$", "", value)
                value = re.sub(r"[ \t].*", "", value)
                return value.replace('"', "")
    return ""


def _extract_literal(skill: str, field: str) -> str:
    text = (REPO_ROOT / skill).read_text(encoding="utf-8")
    token = re.escape(f"`{field}:`")
    for line in text.splitlines():
        if re.match(rf"^[ \t]*-[ \t]*{token}", line):
            match = re.search(rf"{token}[^`]*`([^`]*)`", line)
            return match.group(1) if match else ""
    return _extract_cli_literal(text, field)


def _extract_validate_plan_plan_status() -> str:
    text = (REPO_ROOT / "skills/planning/validate-plan/SKILL.md").read_text(
        encoding="utf-8"
    )
    for line in text.splitlines():
        match = re.search(r"status` field to `([^`]*)`", line)
        if match:
            return match.group(1)
    return ""


def _extract_review_adr_targets() -> list[str]:
    text = (REPO_ROOT / "skills/decisions/review-adr/SKILL.md").read_text(
        encoding="utf-8"
    )
    return sorted(set(re.findall(r"to `status: ([a-z]+)`", text)))


def _emit_valid(
    type_: str,
    anchored: str,
    extras: str,
    vocab: str,
    outfile: Path,
    extra_lines: str = "",
) -> None:
    ids = {"work-item": "0001", "adr": "ADR-0001", "pr-description": "0042"}
    id_ = ids.get(type_, f"fixture-{type_}")
    status = "".join(vocab.split("|", maxsplit=1)[0].split())
    lines = [
        "---",
        f"type: {type_}",
        f'id: "{id_}"',
        f'title: "Fixture {type_}"',
        'date: "2026-01-01T00:00:00+00:00"',
        "author: Fixture Author",
        "producer: fixture",
        f"status: {status}",
        "tags: []",
        'last_updated: "2026-01-01T00:00:00+00:00"',
        "last_updated_by: Fixture Author",
        "schema_version: 1",
    ]
    lines.extend(
        f'{extra}: "x"'
        for extra in extras.split()
        if extra not in fr.OPTIONAL_EXTRAS
    )
    if anchored == "yes":
        lines.append('revision: "abc123"')
        lines.append('repository: "repo"')
    if extra_lines:
        lines.append(extra_lines)
    lines.append("---")
    lines.append("")
    lines.append(f"# Fixture {type_}")
    outfile.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _run_validator(files: list[Path]) -> tuple[int, str]:
    command = [
        _launcher(),
        "corpus",
        "frontmatter",
        "validate",
        "--checks",
        "structure",
    ]
    for path in files:
        command += ["--file", str(path)]
    proc = subprocess.run(
        command, capture_output=True, text=True, cwd=REPO_ROOT, check=False
    )
    return proc.returncode, proc.stderr


def _assert_accepts(files: list[Path]) -> None:
    rc, err = _run_validator(files)
    assert rc == 0, f"validator rejected a valid fixture (rc={rc}):\n{err}"


def _assert_rejects(code: str, files: list[Path]) -> None:
    rc, err = _run_validator(files)
    assert rc != 0 and code in err, (
        f"expected rejection with '{code}' (rc={rc}):\n{err}"
    )


def test_producer_set_reconciliation() -> None:
    discovered = _discovered()
    assert len(discovered) == 17, (
        f"discovery returned {len(discovered)} producing SKILL.md files, "
        f"expected 17: {discovered}"
    )
    assert len(EMITTERS) == 16
    allowlist = set(EMITTERS) | set(EXCLUDED)
    unexpected = sorted(set(discovered) - allowlist)
    assert not unexpected, (
        f"discovered files outside EMITTERS or EXCLUDED: {unexpected}"
    )
    for skill in STATUS_AXIS:
        assert (REPO_ROOT / skill).is_file(), (
            f"status-axis mutator absent: {skill}"
        )


@pytest.mark.parametrize("skill", EMITTERS)
def test_full_block_emitter_conforms(skill: str, tmp_path: Path) -> None:
    type_ = _extract_literal(skill, "type")
    status_lit = _extract_literal(skill, "status")
    producer_lit = _extract_literal(skill, "producer")
    sv_lit = _extract_literal(skill, "schema_version")
    assert type_ and status_lit and producer_lit and sv_lit, (
        f"{skill}: empty literal extraction (type={type_!r} "
        f"status={status_lit!r} producer={producer_lit!r} "
        f"schema_version={sv_lit!r})"
    )
    assert type_ in SCHEMA, (
        f"{skill}: extracted type {type_!r} is not a schema type"
    )
    row = SCHEMA[type_]

    assert sv_lit == "1"
    assert _status_in_vocab(status_lit, row["vocab"]), (
        f"{skill}: status {status_lit!r} not in vocab {row['vocab']!r}"
    )

    covered = _template_keys(row["template"]) | {
        "type",
        "status",
        "producer",
        "schema_version",
    }
    enforced = set(fr.BASE_FIELDS) | {"status"} | set(row["linkkeys"].split())
    enforced |= {
        e for e in row["extras"].split() if e not in fr.OPTIONAL_EXTRAS
    }
    if row["anchored"] == "yes":
        enforced |= set(fr.PROVENANCE_FIELDS)
    missing = sorted(enforced - covered)
    assert not missing, f"{skill} ({type_}): composed emission misses {missing}"

    fixture = tmp_path / f"accept-{type_}.md"
    _emit_valid(type_, row["anchored"], row["extras"], status_lit, fixture)
    _assert_accepts([fixture])


def test_validate_plan_plan_status_axis(tmp_path: Path) -> None:
    vocab = SCHEMA["plan"]["vocab"]
    status = _extract_validate_plan_plan_status()
    assert status, "validate-plan -> plan: status literal not extracted"
    assert _status_in_vocab(status, vocab), (
        f"validate-plan -> plan: status {status!r} not in plan vocab"
    )
    fixture = tmp_path / "vp-plan.md"
    _emit_valid("plan", "yes", "reviewer", status, fixture)
    _assert_accepts([fixture])


def test_review_adr_status_axis(tmp_path: Path) -> None:
    vocab = SCHEMA["adr"]["vocab"]
    targets = _extract_review_adr_targets()
    assert targets, "review-adr -> adr: target statuses not extracted"
    for target in targets:
        assert _status_in_vocab(target, vocab), (
            f"review-adr -> adr: status {target!r} not in adr vocab"
        )
        fixture = tmp_path / f"adr-{target}.md"
        _emit_valid("adr", "no", "decision_makers", target, fixture)
        _assert_accepts([fixture])


def test_conditional_axis_provenance(tmp_path: Path) -> None:
    present = tmp_path / "prov-present.md"
    _emit_valid("plan", "yes", "reviewer", "draft", present)
    _assert_accepts([present])

    absent = tmp_path / "prov-absent.md"
    _emit_valid("work-item", "no", "kind priority external_id", "draft", absent)
    _assert_accepts([absent])

    missing = tmp_path / "prov-missing.md"
    _emit_valid("plan", "yes", "reviewer", "draft", missing)
    stripped = "\n".join(
        line
        for line in missing.read_text(encoding="utf-8").splitlines()
        if not line.startswith(("revision: ", "repository: "))
    )
    missing.write_text(stripped + "\n", encoding="utf-8")
    _assert_rejects("MISSING-PROVENANCE", [missing])

    overemit = tmp_path / "prov-overemit.md"
    _emit_valid(
        "work-item",
        "no",
        "kind priority external_id",
        "draft",
        overemit,
        'revision: "x"\nrepository: "y"',
    )
    _assert_rejects("PROVENANCE-ON-NONANCHORED", [overemit])


def test_conditional_axis_linkage(tmp_path: Path) -> None:
    present = tmp_path / "link-present.md"
    _emit_valid(
        "work-item",
        "no",
        "kind priority external_id",
        "draft",
        present,
        'parent: "work-item:0001"',
    )
    _assert_accepts([present])

    absent = tmp_path / "link-absent.md"
    _emit_valid("work-item", "no", "kind priority external_id", "draft", absent)
    _assert_accepts([absent])

    bare = tmp_path / "link-bare.md"
    _emit_valid(
        "work-item",
        "no",
        "kind priority external_id",
        "draft",
        bare,
        "parent: 0042",
    )
    _assert_rejects("BAD-LINKAGE-SHAPE", [bare])


def test_conditional_axis_omit_when_empty(tmp_path: Path) -> None:
    present = tmp_path / "owe-present.md"
    _emit_valid(
        "work-item",
        "no",
        "kind priority external_id",
        "draft",
        present,
        'external_id: "JIRA-1"',
    )
    _assert_accepts([present])

    absent = tmp_path / "owe-absent.md"
    _emit_valid("work-item", "no", "kind priority external_id", "draft", absent)
    _assert_accepts([absent])

    empty = tmp_path / "owe-empty.md"
    _emit_valid(
        "work-item",
        "no",
        "kind priority external_id",
        "draft",
        empty,
        'external_id: ""',
    )
    _assert_rejects("EMPTY-PLACEHOLDER", [empty])


_MUTATIONS = [
    ("type", "INVALID-TYPE", r"^type: work-item$", "type: nonsense"),
    ("status", "BAD-STATUS", r"^status: .*", "status: bogus"),
    ("extra", "MISSING-EXTRA", r"^kind: .*", None),
    (
        "schema_version",
        "BAD-SCHEMA-VERSION",
        r"^schema_version: 1$",
        'schema_version: "1"',
    ),
]


@pytest.mark.parametrize(("axis", "code", "pattern", "replacement"), _MUTATIONS)
def test_negative_self_test_axis(
    axis: str, code: str, pattern: str, replacement: str | None, tmp_path: Path
) -> None:
    base = tmp_path / "neg-base.md"
    _emit_valid(
        "work-item", "no", "kind priority external_id", "draft | ready", base
    )
    original = base.read_text(encoding="utf-8")
    compiled = re.compile(pattern, re.MULTILINE)
    if replacement is None:
        mutated = compiled.sub("", original)
        mutated = re.sub(r"\n\n+", "\n", mutated)
    else:
        mutated = compiled.sub(replacement, original)
    assert mutated != original, f"axis={axis}: mutation was a no-op"
    out = tmp_path / f"neg-{axis}.md"
    out.write_text(mutated, encoding="utf-8")
    _assert_rejects(code, [out])


def test_contract_is_sourced_not_reencoded() -> None:
    # The enforced set derives from templates-schema.tsv and frontmatter_rules
    # rather than being hard-coded here.
    assert SCHEMA, "templates-schema.tsv yielded no rows"
    assert fr.BASE_FIELDS, "frontmatter_rules.BASE_FIELDS is empty"
