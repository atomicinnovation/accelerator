#!/usr/bin/env bash
# Shared frontmatter emission rules (ADR-0033 / ADR-0034 / ADR-0040).
#
# This file is the SINGLE SOURCE for the cross-cutting emission rules that have
# no per-type column in templates-schema.tsv: the required base-field set, the
# quoted-`id:` rule, `schema_version: 1` as a bare integer, the
# git_commit/branch-absent rule, the typed-linkage source-type vocabulary, the
# linkage cardinality table, the optional-extra carve-out, and the
# `"doc-type:id"` typed-reference value shape.
#
# It is sourced by BOTH:
#   - scripts/test-template-frontmatter.sh   (asserts templates carry the slots)
#   - scripts/validate-corpus-frontmatter.sh (asserts generated artifacts conform)
# so the two surfaces cannot drift. The PER-TYPE tabular facts (type set, extras,
# status_vocab, code_state_anchored, forbidden_own_id_key, typed_linkage_keys)
# stay in templates-schema.tsv; only the cross-cutting rules live here.
#
# Pure data + pure functions only — no top-level side effects, safe to source
# under `set -euo pipefail` on bash 3.2 (no associative arrays).

# ---- Required base fields -------------------------------------------------
# The base fields every conforming artifact MUST carry. `producer` and `status`
# are deliberately NOT here: `producer` is omitted on hand-written legacy plans,
# and `status` is left unset when a source artifact lacked one (defaults rule).
# Consumers that require those (the template-shape test) append them locally.
FM_BASE_FIELDS=(type id title date author tags last_updated last_updated_by schema_version)

# ---- Provenance bundle ----------------------------------------------------
FM_PROVENANCE_FIELDS=(revision repository)
FM_FORBIDDEN_PROVENANCE_FIELDS=(git_commit branch)

# ---- Typed-linkage source-type vocabulary ---------------------------------
# Pipe-joined for interpolation into ERE patterns. `pr` is the external-entity
# prefix (tolerated; an ADR-0034 supplement is pending). Keep aligned with the
# doc-types that appear in template typed-linkage comments.
FM_SOURCE_TYPE_RE='work-item|plan|adr|pr|note|codebase-research|issue-research|pr-description|design-inventory|design-gap|plan-validation|plan-review|work-item-review|pr-review'

# ---- Linkage vocabulary + cardinality -------------------------------------
# Union of all typed-linkage key names. superseded_by is listed as a guard even
# though no template carries it, so the closed-set check rejects any template
# that adds it. Keep aligned with fm_linkage_cardinality().
FM_LINKAGE_VOCABULARY=(parent superseded_by target source supersedes blocks blocked_by derived_from relates_to)

# Cardinality lookup by linkage-key name. case-based (not `declare -A`) so this
# keeps working on bash 3.2 (the macOS default). Echoes `single`, `list`, or
# empty (unknown key).
fm_linkage_cardinality() {
  case "$1" in
    parent | superseded_by | target | source) echo single ;;
    supersedes | blocks | blocked_by | derived_from | relates_to) echo list ;;
    *) echo "" ;;
  esac
}

# ---- Foreign-reference keys ----------------------------------------------
# Keys that point at another artifact's identity (kept, not own-identity). They
# are omit-when-empty: a present-but-empty foreign ref is a violation.
FM_FOREIGN_REF_KEYS=(work_item_id)

# ---- Optional (omit-when-empty) extras ------------------------------------
# Per-type `extras` (from the TSV) that are legitimately omitted when empty, so
# the corpus validator does not REQUIRE them present. Every other extra is
# treated as always-valued and required. Keyed by name only (a `reviewer` on a
# plan is optional; a `reviewer` on a review is always-valued — treating it as
# optional only relaxes the presence requirement, never forbids it).
# `work_item_id` is the foreign-ref alias the work-item-review row still lists
# transitionally (dropped by Phase 5b of story 0070); omit-when-empty, never
# required.
FM_OPTIONAL_EXTRAS="external_id reviewer pr_url merge_commit decision_makers work_item_id"

# ---- Value-shape regexes (ERE) --------------------------------------------
# `id:` value is a quoted YAML string (the ONLY base field whose quoting is
# enforced; title/author/last_updated_by/repository are presence-checked only).
FM_ID_QUOTED_RE='^id:[[:space:]]+"[^"]*"([[:space:]]+#.*)?$'

# `schema_version:` is the bare integer 1.
FM_SCHEMA_VERSION_RE='^schema_version:[[:space:]]+1([[:space:]]+#.*)?$'

# A typed-linkage *value* (inner, unquoted) is `doc-type:id`, never bare `NNNN`
# and never a path (`meta/work/...` — the `/` keeps a path out). The id part is
# `[A-Za-z0-9.-]+`: bare numbers, ADR-NNNN, and full filename stems — which can
# contain dots (e.g. a version-numbered stem `…-changelog-1.21.0-cleanup`).
FM_TYPED_REF_RE="^(${FM_SOURCE_TYPE_RE}):[A-Za-z0-9.-]+$"

# The blocked_by inverse-key guidance comment line (template surface only).
FM_INVERSE_GUIDANCE_LINE='# inverse of blocks — producers SHOULD prefer writing blocks: on the canonical side'

# Returns 0 if $1 names a typed-linkage key (in the vocabulary).
fm_is_linkage_key() {
  local k="$1" v
  for v in "${FM_LINKAGE_VOCABULARY[@]}"; do
    [ "$v" = "$k" ] && return 0
  done
  return 1
}
