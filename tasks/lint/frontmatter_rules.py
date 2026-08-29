"""The cross-cutting frontmatter emission rules.

The single source for the emission rules that have no per-type column in
`templates-schema.tsv`: the required base-field set, the quoted-`id:` rule,
`schema_version: 1` as a bare integer, the git_commit/branch-absent rule, the
typed-linkage source-type vocabulary, the linkage cardinality table, the
optional-extra carve-out, and the `"doc-type:id"` typed-reference value shape.

Imported by the conformance and template ports so the surfaces cannot drift.
The per-type tabular facts (type set, extras, status_vocab,
code_state_anchored, forbidden_own_id_key, typed_linkage_keys) stay in
`templates-schema.tsv`; only the cross-cutting rules live here.
"""

import re

# The base fields every conforming artifact MUST carry. `producer` and `status`
# are deliberately excluded: `producer` is omitted on hand-written legacy plans,
# and `status` is left unset when a source artifact lacked one. Consumers that
# require those (the template-shape test) append them locally.
BASE_FIELDS: tuple[str, ...] = (
    "type",
    "id",
    "title",
    "date",
    "author",
    "tags",
    "last_updated",
    "last_updated_by",
    "schema_version",
)

PROVENANCE_FIELDS: tuple[str, ...] = ("revision", "repository")
FORBIDDEN_PROVENANCE_FIELDS: tuple[str, ...] = ("git_commit", "branch")

# The typed-linkage source-type vocabulary. `pr` is the external-entity prefix.
SOURCE_TYPES: tuple[str, ...] = (
    "work-item",
    "plan",
    "adr",
    "pr",
    "note",
    "codebase-research",
    "issue-research",
    "pr-description",
    "design-inventory",
    "design-gap",
    "plan-validation",
    "plan-review",
    "work-item-review",
    "pr-review",
)

# Pipe-joined for composition into the value-shape patterns below.
SOURCE_TYPE_RE = "|".join(SOURCE_TYPES)

# Union of all typed-linkage key names. `superseded_by` is a guard: no template
# carries it, so the closed-set check rejects any template that adds it.
LINKAGE_VOCABULARY: tuple[str, ...] = (
    "parent",
    "superseded_by",
    "target",
    "source",
    "supersedes",
    "blocks",
    "blocked_by",
    "derived_from",
    "relates_to",
)

_SINGLE_CARDINALITY = frozenset({"parent", "superseded_by", "target", "source"})
_LIST_CARDINALITY = frozenset(
    {"supersedes", "blocks", "blocked_by", "derived_from", "relates_to"}
)


def linkage_cardinality(key: str) -> str:
    """Return `"single"`, `"list"`, or `""` for an unknown key."""
    if key in _SINGLE_CARDINALITY:
        return "single"
    if key in _LIST_CARDINALITY:
        return "list"
    return ""


def is_linkage_key(key: str) -> bool:
    """Whether `key` names a typed-linkage key."""
    return key in LINKAGE_VOCABULARY


# Foreign-reference keys: point at another artifact's identity (kept, not
# own-identity). Omit-when-empty — a present-but-empty foreign ref is a
# violation.
FOREIGN_REF_KEYS: tuple[str, ...] = ("work_item_id",)

# Per-type `extras` that are legitimately omitted when empty, so the validator
# does not REQUIRE them present. Every other extra is treated as always-valued.
OPTIONAL_EXTRAS: frozenset[str] = frozenset(
    {
        "external_id",
        "reviewer",
        "pr_url",
        "merge_commit",
        "decision_makers",
        "work_item_id",
    }
)

# `id:` value is a quoted YAML string — the only base field whose quoting is
# enforced (title/author/last_updated_by/repository are presence-checked only).
ID_QUOTED_RE = re.compile(r'^id:[ \t]+"[^"]*"([ \t]+#.*)?$')

# `schema_version:` is the bare integer 1.
SCHEMA_VERSION_RE = re.compile(r"^schema_version:[ \t]+1([ \t]+#.*)?$")

# A typed-linkage *value* (inner, unquoted) is `doc-type:id`, never bare `NNNN`
# and never a path (the `/` keeps a path out). The id part is `[A-Za-z0-9.-]+`:
# bare numbers, ADR-NNNN, and full filename stems (which can contain dots).
TYPED_REF_RE = re.compile(rf"^({SOURCE_TYPE_RE}):[A-Za-z0-9.-]+$")

# The blocked_by inverse-key guidance comment line (template surface only).
INVERSE_GUIDANCE_LINE = (
    "# inverse of blocks — producers SHOULD prefer writing blocks: on the "
    "canonical side"
)

# The schema column order the positional readers depend on.
SCHEMA_COLUMNS: tuple[str, ...] = (
    "template",
    "type",
    "code_state_anchored",
    "extras",
    "status_vocab",
    "forbidden_own_id_key",
    "typed_linkage_keys",
)


def schema_columns_ok(header: str) -> bool:
    """Whether a schema-TSV header row matches the expected column order.

    Prefix-match: the exact canonical columns, optionally followed by a tab and
    further columns, so a forward-compatible trailing extension is tolerated.
    """
    header = header.rstrip("\r")
    expected = "\t".join(SCHEMA_COLUMNS)
    return header == expected or header.startswith(expected + "\t")
