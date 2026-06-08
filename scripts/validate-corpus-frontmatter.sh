#!/usr/bin/env bash
# Corpus frontmatter validator (AC-1 gate for story 0070).
#
# Validates one or more *generated* meta/ artifacts against the unified schema
# (ADR-0033 / ADR-0034 / ADR-0040). Per-type tabular facts come from
# templates-schema.tsv; the cross-cutting emission rules come from the shared
# frontmatter-emission-rules.sh helper (single-sourced with
# test-template-frontmatter.sh).
#
# Usage:
#   validate-corpus-frontmatter.sh <dir>          # whole-corpus mode
#   validate-corpus-frontmatter.sh <file> [file…]  # file-list mode
#
# Whole-corpus (single directory argument) mode walks the tree and ALSO runs the
# referential-integrity check (every typed-linkage value resolves to a real
# artifact, `pr:<n>` tolerated). File-list mode runs structural checks only
# (referential integrity is a whole-corpus property). Out-of-scope subtrees
# (specs/, talks/, global/) are skipped in whole-corpus mode.
#
# Exits non-zero with one diagnostic line per violation; exit 0 when clean.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=frontmatter-emission-rules.sh
FM_EMISSION_RULES="${FM_EMISSION_RULES:-$SCRIPT_DIR/frontmatter-emission-rules.sh}"
source "$FM_EMISSION_RULES"
SCHEMA_TSV="${SCHEMA_TSV:-$SCRIPT_DIR/templates-schema.tsv}"

# Byte-stable classes/sorting regardless of host locale (parity with the
# project's LANG=C discipline).
export LC_ALL=C

VIOLATIONS=0
violation() { # $1 = file, $2 = code, $3 = message
  printf '%s: %s — %s\n' "$1" "$2" "$3" >&2
  VIOLATIONS=$((VIOLATIONS + 1))
}

# ---- Schema table (parallel arrays; bash 3.2 has no associative arrays) ----
SCHEMA_TYPES=()
SCHEMA_ANCHORED=()
SCHEMA_EXTRAS=()
SCHEMA_STATUS=()
SCHEMA_FORBIDDEN=()
SCHEMA_LINKKEYS=()
while IFS=$'\t' read -r _tmpl type anchored extras status_vocab forbidden linkkeys; do
  SCHEMA_TYPES+=("$type")
  SCHEMA_ANCHORED+=("$anchored")
  SCHEMA_EXTRAS+=("$extras")
  SCHEMA_STATUS+=("$status_vocab")
  SCHEMA_FORBIDDEN+=("$forbidden")
  SCHEMA_LINKKEYS+=("$linkkeys")
done < <(tail -n +2 "$SCHEMA_TSV")

# Index of a type in the parallel arrays, or empty if not a schema type.
schema_index() {
  local needle="$1" i
  for ((i = 0; i < ${#SCHEMA_TYPES[@]}; i++)); do
    if [ "${SCHEMA_TYPES[$i]}" = "$needle" ]; then
      printf '%s' "$i"
      return 0
    fi
  done
  return 0
}

# ---- Location → doc-type map (for index identity when type: is absent) -----
# Exhaustive per the plan; reviews discriminated by subdirectory.
infer_type_from_path() {
  case "$1" in
    */work/*) echo work-item ;;
    */plans/*) echo plan ;;
    */decisions/*) echo adr ;;
    */research/codebase/*) echo codebase-research ;;
    */research/issues/*) echo issue-research ;;
    */research/design-gaps/*) echo design-gap ;;
    */research/design-inventories/*) echo design-inventory ;;
    */reviews/plans/*) echo plan-review ;;
    */reviews/work/*) echo work-item-review ;;
    */reviews/prs/*) echo pr-review ;;
    */validations/*) echo plan-validation ;;
    */notes/*) echo note ;;
    *) echo "" ;;
  esac
}

# A path is out of scope (skip entirely) if it lives under specs/talks/global.
out_of_scope() {
  case "$1" in
    */specs/* | */talks/* | */global/*) return 0 ;;
    *) return 1 ;;
  esac
}

# ---- Frontmatter extraction + field access --------------------------------
# Loose fence detector (tolerates trailing whitespace on the opening `---`).
# NB: no `exit` in the awk and no upstream-killing pipe — under `set -o
# pipefail` an early awk `exit` SIGPIPEs the upstream `tr`, turning the
# pipeline's status to 141 and (e.g. in has_fence) flipping a valid result.
# The frontmatter is tiny, so reading the whole file is cheap.
extract_frontmatter() {
  awk '
    BEGIN { state = 0 }
    /^---[[:space:]]*$/ {
      if (state == 0) { state = 1; next }
      if (state == 1) { state = 2; next }
    }
    state == 1 { sub(/\r$/, ""); print }
  ' "$1"
}

# Has a leading frontmatter fence (loose: trailing whitespace tolerated). Reads
# only the first line via the builtin `read` — no pipe, so no SIGPIPE.
has_fence() {
  local first
  IFS= read -r first <"$1" 2>/dev/null || return 1
  first="${first%$'\r'}"
  [[ "$first" =~ ^---[[:space:]]*$ ]]
}

# Raw (trimmed) value of a frontmatter key from a block, or empty. Here-string
# input + no awk `exit` (a `done` flag keeps only the first match) so there is
# no upstream writer to receive SIGPIPE under pipefail.
fm_value() {
  awk -v k="$2" '
    !done && index($0, k ":") == 1 {
      line = $0
      sub("^" k ":[ \t]*", "", line)
      sub(/[ \t]+$/, "", line)
      print line
      done = 1
    }' <<<"$1"
}

# Inner (unquoted) value: strip a single layer of surrounding double/single
# quotes; leave a trailing inline comment off only for quoted scalars.
fm_inner() {
  local v="$1"
  case "$v" in
    '"'*'"'*) v="${v#\"}"; v="${v%%\"*}" ;;
    "'"*"'"*) v="${v#\'}"; v="${v%%\'*}" ;;
  esac
  printf '%s' "$v"
}

# Filename stem (basename without .md).
stem_of() {
  local b
  b="$(basename "$1")"
  printf '%s' "${b%.md}"
}

# ---- Identity resolution (matches the migration's rule) --------------------
# id: → legacy own-id key (work_item_id / adr_id) → filename stem.
resolve_id() {
  local block="$1" file="$2" v
  v="$(fm_value "$block" id)"
  if [ -n "$v" ]; then fm_inner "$v"; return 0; fi
  v="$(fm_value "$block" work_item_id)"
  if [ -n "$v" ]; then fm_inner "$v"; return 0; fi
  v="$(fm_value "$block" adr_id)"
  if [ -n "$v" ]; then fm_inner "$v"; return 0; fi
  stem_of "$file"
}

# ---- ISO-8601 timestamp check ---------------------------------------------
ISO_TS_RE='^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(Z|[+-][0-9]{2}:[0-9]{2})$'
is_iso_ts() { grep -qE "$ISO_TS_RE" <<<"$1"; }

# ---- Build the corpus index (referential integrity target set) -------------
# Newline-delimited set of "doc-type:id" identities. Built from all in-scope
# fenced files (plus location-inferred type when type: is absent).
INDEX_KEYS=""
build_index() {
  local root="$1" f block type id
  while IFS= read -r -d '' f; do
    out_of_scope "$f" && continue
    has_fence "$f" || continue
    block="$(extract_frontmatter "$f")"
    type="$(fm_inner "$(fm_value "$block" type)")"
    [ -n "$type" ] || type="$(infer_type_from_path "$f")"
    [ -n "$type" ] || continue
    id="$(resolve_id "$block" "$f")"
    [ -n "$id" ] || continue
    INDEX_KEYS="${INDEX_KEYS}${type}:${id}"$'\n'
  done < <(find "$root" -type f -name '*.md' -print0)
}

index_has() { grep -qxF -- "$1" <<<"$INDEX_KEYS"; }

# ---- Per-file validation ---------------------------------------------------
# $1 = file, $2 = "yes" to run referential integrity (whole-corpus mode).
validate_file() {
  local file="$1" referential="$2" block type idx anchored extras status_vocab forbidden linkkeys

  if ! has_fence "$file"; then
    violation "$file" "NO-FENCE" "no frontmatter fence at file head"
    return 0
  fi
  block="$(extract_frontmatter "$file")"

  type="$(fm_inner "$(fm_value "$block" type)")"
  idx="$(schema_index "$type")"
  if [ -z "$type" ] || [ -z "$idx" ]; then
    violation "$file" "INVALID-TYPE" "type: '${type:-<absent>}' is not a schema type"
    return 0
  fi
  anchored="${SCHEMA_ANCHORED[$idx]}"
  extras="${SCHEMA_EXTRAS[$idx]}"
  status_vocab="${SCHEMA_STATUS[$idx]}"
  forbidden="${SCHEMA_FORBIDDEN[$idx]}"
  linkkeys="${SCHEMA_LINKKEYS[$idx]}"

  # Required base fields present.
  local f
  for f in "${FM_BASE_FIELDS[@]}"; do
    if ! grep -qE "^${f}:[[:space:]]" <<<"$block"; then
      violation "$file" "MISSING-BASE-FIELD" "required base field '$f' absent"
    fi
  done

  # id: is a quoted YAML string.
  if grep -qE '^id:[[:space:]]' <<<"$block" && ! grep -qE "$FM_ID_QUOTED_RE" <<<"$block"; then
    violation "$file" "UNQUOTED-ID" "id: value is not a quoted string"
  fi

  # schema_version: bare integer 1.
  if grep -qE '^schema_version:[[:space:]]' <<<"$block" && ! grep -qE "$FM_SCHEMA_VERSION_RE" <<<"$block"; then
    violation "$file" "BAD-SCHEMA-VERSION" "schema_version: is not the bare integer 1"
  fi

  # date / last_updated are full ISO timestamps (when present).
  for f in date last_updated; do
    local raw inner
    raw="$(fm_value "$block" "$f")"
    [ -n "$raw" ] || continue
    inner="$(fm_inner "$raw")"
    if ! is_iso_ts "$inner"; then
      violation "$file" "BAD-TIMESTAMP" "$f: '$inner' is not a full ISO-8601 timestamp"
    fi
  done

  # status (when present) in the type's vocab.
  local status_raw status_inner
  status_raw="$(fm_value "$block" status)"
  if [ -n "$status_raw" ]; then
    status_inner="$(fm_inner "$status_raw")"
    # status_vocab is a `a | b | c` string; match the inner value as a whole
    # token. The token list is built in a sub-pipeline that runs to completion
    # (no early exit), then matched via a here-string — no upstream SIGPIPE.
    local vocab_tokens
    vocab_tokens="$(printf '%s' "$status_vocab" | tr '|' '\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    if ! grep -qxF -- "$status_inner" <<<"$vocab_tokens"; then
      violation "$file" "BAD-STATUS" "status: '$status_inner' not in vocab ($status_vocab)"
    fi
  fi

  # Provenance bundle iff code_state_anchored=yes; git_commit/branch never.
  if [ "$anchored" = "yes" ]; then
    for f in "${FM_PROVENANCE_FIELDS[@]}"; do
      grep -qE "^${f}:[[:space:]]" <<<"$block" ||
        violation "$file" "MISSING-PROVENANCE" "anchored type missing provenance field '$f'"
    done
  fi
  for f in "${FM_FORBIDDEN_PROVENANCE_FIELDS[@]}"; do
    grep -qE "^${f}:[[:space:]]" <<<"$block" &&
      violation "$file" "FORBIDDEN-PROVENANCE" "legacy provenance field '$f' present"
  done

  # Forbidden own-identity key absent.
  if [ "$forbidden" != "-" ]; then
    for f in $forbidden; do
      grep -qE "^${f}:[[:space:]]" <<<"$block" &&
        violation "$file" "FORBIDDEN-OWN-ID" "forbidden own-id key '$f' present"
    done
  fi

  # Required (always-valued) extras present.
  for f in $extras; do
    case " $FM_OPTIONAL_EXTRAS " in *" $f "*) continue ;; esac
    grep -qE "^${f}:[[:space:]]" <<<"$block" ||
      violation "$file" "MISSING-EXTRA" "required extra '$f' absent"
  done

  # Omit-when-empty: no key (except tags) carries an empty `""` or `[]`.
  local empty_key
  while IFS= read -r empty_key; do
    [ -n "$empty_key" ] || continue
    violation "$file" "EMPTY-PLACEHOLDER" "key '$empty_key' emitted empty (should be omitted)"
  done < <(printf '%s\n' "$block" | awk '
    /^tags:[[:space:]]/ { next }
    /^[A-Za-z_][A-Za-z0-9_]*:[[:space:]]*(""|\[\])[[:space:]]*(#.*)?$/ {
      k = $0; sub(/:.*/, "", k); print k
    }')

  # Typed-linkage values: doc-type:id shape + (corpus mode) referential.
  local key val inner
  for key in $linkkeys; do
    grep -qE "^${key}:[[:space:]]" <<<"$block" || continue
    while IFS= read -r val; do
      [ -n "$val" ] || continue
      inner="$(fm_inner "$val")"
      # Empty already handled by omit-when-empty; skip blanks.
      [ -n "$inner" ] || continue
      if ! printf '%s' "$inner" | grep -qE "$FM_TYPED_REF_RE"; then
        violation "$file" "BAD-LINKAGE-SHAPE" "$key: '$inner' is not a typed \"doc-type:id\" reference"
        continue
      fi
      if [ "$referential" = "yes" ]; then
        case "$inner" in
          pr:*) : ;; # tolerated external-entity prefix
          *)
            index_has "$inner" ||
              violation "$file" "DANGLING-REF" "$key: '$inner' resolves to no artifact in the corpus"
            ;;
        esac
      fi
    done < <(grep -oE '"[^"]*"' <<<"$(grep -E "^${key}:[[:space:]]" <<<"$block")")
  done
}

# ---- Entry point -----------------------------------------------------------
main() {
  if [ "$#" -eq 0 ]; then
    echo "usage: $0 <dir> | <file>..." >&2
    exit 2
  fi

  if [ "$#" -eq 1 ] && [ -d "$1" ]; then
    # Whole-corpus mode.
    build_index "$1"
    local f
    while IFS= read -r -d '' f; do
      out_of_scope "$f" && continue
      validate_file "$f" yes
    done < <(find "$1" -type f -name '*.md' -print0)
  else
    # File-list mode (structural only).
    local f
    for f in "$@"; do
      validate_file "$f" no
    done
  fi

  if [ "$VIOLATIONS" -gt 0 ]; then
    printf 'FAIL: %d frontmatter violation(s)\n' "$VIOLATIONS" >&2
    exit 1
  fi
  exit 0
}

main "$@"
