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
FM_EMISSION_RULES="${FM_EMISSION_RULES:-$SCRIPT_DIR/frontmatter-emission-rules.sh}"
# shellcheck source=frontmatter-emission-rules.sh
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
    # Reviews are discriminated by subdirectory and MUST precede the generic
    # */work/* and */plans/* arms: a review path contains both segments (e.g.
    # */reviews/work/*) so the generic arms would otherwise shadow them.
    */reviews/plans/*) echo plan-review ;;
    */reviews/work/*) echo work-item-review ;;
    */reviews/prs/*) echo pr-review ;;
    */work/*) echo work-item ;;
    */plans/*) echo plan ;;
    */decisions/*) echo adr ;;
    */research/codebase/*) echo codebase-research ;;
    */research/issues/*) echo issue-research ;;
    */research/design-gaps/*) echo design-gap ;;
    */research/design-inventories/*) echo design-inventory ;;
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

# Inner (unquoted) value: strip a single layer of surrounding double/single
# quotes; leave a trailing inline comment off only for quoted scalars.
fm_inner() {
  local v="$1"
  case "$v" in
    '"'*'"'*)
      v="${v#\"}"
      v="${v%%\"*}"
      ;;
    "'"*"'"*)
      v="${v#\'}"
      v="${v%%\'*}"
      ;;
  esac
  printf '%s' "$v"
}

# Filename stem (basename without .md).
stem_of() {
  local b
  b="$(basename "$1")"
  printf '%s' "${b%.md}"
}

# ---- Bash-native frontmatter parse (no per-field subprocess) ----------------
# parse_fm <block> fills the parallel arrays BK_KEYS / BK_VALS with one entry
# per `key: value` line (comment and non-key lines skipped), values whitespace-
# trimmed. One read loop, no per-line spawn — this is what keeps the whole-corpus
# validation comfortably inside the migration's post-DONE watchdog.
BK_KEYS=()
BK_VALS=()
parse_fm() {
  BK_KEYS=()
  BK_VALS=()
  local line k v
  while IFS= read -r line; do
    case "$line" in
      [A-Za-z_]*:*) ;;
      *) continue ;;
    esac
    k="${line%%:*}"
    case "$k" in *[!A-Za-z0-9_]*) continue ;; esac
    v="${line#*:}"
    v="${v#"${v%%[![:space:]]*}"}" # strip leading whitespace
    v="${v%"${v##*[![:space:]]}"}" # strip trailing whitespace
    BK_KEYS+=("$k")
    BK_VALS+=("$v")
  done <<<"$1"
}
# Returns 0 if the key is present.
bk_present() {
  local n="$1" i
  for ((i = 0; i < ${#BK_KEYS[@]}; i++)); do
    [ "${BK_KEYS[$i]}" = "$n" ] && return 0
  done
  return 1
}
# Sets BK_VAL to the first value for the key; returns 1 (BK_VAL="") if absent.
bk_value() {
  local n="$1" i
  BK_VAL=""
  for ((i = 0; i < ${#BK_KEYS[@]}; i++)); do
    if [ "${BK_KEYS[$i]}" = "$n" ]; then
      BK_VAL="${BK_VALS[$i]}"
      return 0
    fi
  done
  return 1
}

# ---- ISO-8601 timestamp check ---------------------------------------------
ISO_TS_RE='^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(Z|[+-][0-9]{2}:[0-9]{2})$'

# ---- Build the corpus index (referential integrity target set) -------------
# Newline-delimited set of "doc-type:id" identities. Built from all in-scope
# fenced files (plus location-inferred type when type: is absent).
INDEX_KEYS=""
build_index() {
  local root="$1" f type id
  while IFS= read -r -d '' f; do
    out_of_scope "$f" && continue
    has_fence "$f" || continue
    parse_fm "$(extract_frontmatter "$f")"
    bk_value type && type="$(fm_inner "$BK_VAL")" || type=""
    [ -n "$type" ] || type="$(infer_type_from_path "$f")"
    [ -n "$type" ] || continue
    # id: → legacy own-id key → filename stem.
    id=""
    if bk_value id; then id="$(fm_inner "$BK_VAL")"; fi
    if [ -z "$id" ] && bk_value work_item_id; then id="$(fm_inner "$BK_VAL")"; fi
    if [ -z "$id" ] && bk_value adr_id; then id="$(fm_inner "$BK_VAL")"; fi
    [ -n "$id" ] || id="$(stem_of "$f")"
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
  parse_fm "$block"

  if bk_value type; then type="$(fm_inner "$BK_VAL")"; else type=""; fi
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

  # Value-level regexes (matched against the trimmed value, not the whole line).
  local re_id_val='^"[^"]*"([[:space:]]+#.*)?$'
  local re_sv_val='^1([[:space:]]+#.*)?$'

  # Required base fields present.
  local f
  for f in "${FM_BASE_FIELDS[@]}"; do
    bk_present "$f" || violation "$file" "MISSING-BASE-FIELD" "required base field '$f' absent"
  done

  # id: is a quoted YAML string.
  if bk_value id && [ -n "$BK_VAL" ] && [[ ! "$BK_VAL" =~ $re_id_val ]]; then
    violation "$file" "UNQUOTED-ID" "id: value is not a quoted string"
  fi

  # schema_version: bare integer 1.
  if bk_value schema_version && [[ ! "$BK_VAL" =~ $re_sv_val ]]; then
    violation "$file" "BAD-SCHEMA-VERSION" "schema_version: is not the bare integer 1"
  fi

  # date / last_updated are full ISO timestamps (when present).
  local inner
  for f in date last_updated; do
    bk_value "$f" || continue
    inner="$(fm_inner "$BK_VAL")"
    [ -n "$inner" ] || continue
    [[ "$inner" =~ $ISO_TS_RE ]] ||
      violation "$file" "BAD-TIMESTAMP" "$f: '$inner' is not a full ISO-8601 timestamp"
  done

  # status (when present) in the type's vocab.
  if bk_value status; then
    inner="$(fm_inner "$BK_VAL")"
    if [ -n "$inner" ]; then
      local ok=0 tok oldifs="$IFS"
      IFS='|'
      for tok in $status_vocab; do
        tok="${tok#"${tok%%[![:space:]]*}"}"
        tok="${tok%"${tok##*[![:space:]]}"}"
        [ "$tok" = "$inner" ] && ok=1
      done
      IFS="$oldifs"
      [ "$ok" -eq 1 ] ||
        violation "$file" "BAD-STATUS" "status: '$inner' not in vocab ($status_vocab)"
    fi
  fi

  # Provenance bundle iff code_state_anchored=yes; git_commit/branch never.
  if [ "$anchored" = "yes" ]; then
    for f in "${FM_PROVENANCE_FIELDS[@]}"; do
      bk_present "$f" ||
        violation "$file" "MISSING-PROVENANCE" "anchored type missing provenance field '$f'"
    done
  fi
  for f in "${FM_FORBIDDEN_PROVENANCE_FIELDS[@]}"; do
    bk_present "$f" &&
      violation "$file" "FORBIDDEN-PROVENANCE" "legacy provenance field '$f' present"
  done

  # Forbidden own-identity key absent.
  if [ "$forbidden" != "-" ]; then
    for f in $forbidden; do
      bk_present "$f" &&
        violation "$file" "FORBIDDEN-OWN-ID" "forbidden own-id key '$f' present"
    done
  fi

  # Required (always-valued) extras present.
  for f in $extras; do
    case " $FM_OPTIONAL_EXTRAS " in *" $f "*) continue ;; esac
    bk_present "$f" || violation "$file" "MISSING-EXTRA" "required extra '$f' absent"
  done

  # Omit-when-empty: no key (except tags) carries an empty `""` or `[]`.
  local i ek ev
  for ((i = 0; i < ${#BK_KEYS[@]}; i++)); do
    ek="${BK_KEYS[$i]}"
    [ "$ek" = "tags" ] && continue
    ev="${BK_VALS[$i]}"
    case "$ev" in
      '""' | '[]')
        violation "$file" "EMPTY-PLACEHOLDER" "key '$ek' emitted empty (should be omitted)"
        ;;
    esac
  done

  # Typed-linkage values: doc-type:id shape + (corpus mode) referential.
  local key rest tok
  for key in $linkkeys; do
    bk_value "$key" || continue
    rest="$BK_VAL"
    while [[ "$rest" =~ \"([^\"]*)\" ]]; do
      tok="${BASH_REMATCH[1]}"
      rest="${rest#*\""${tok}"\"}"
      [ -n "$tok" ] || continue
      if [[ ! "$tok" =~ $FM_TYPED_REF_RE ]]; then
        violation "$file" "BAD-LINKAGE-SHAPE" "$key: '$tok' is not a typed \"doc-type:id\" reference"
        continue
      fi
      if [ "$referential" = "yes" ]; then
        case "$tok" in
          pr:*) : ;; # tolerated external-entity prefix
          *)
            index_has "$tok" ||
              violation "$file" "DANGLING-REF" "$key: '$tok' resolves to no artifact in the corpus"
            ;;
        esac
      fi
    done
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
