#!/usr/bin/env bash
# Body-section typed-linkage parser (story 0070, Phase 2).
#
# Reads the five de-facto linkage-bearing body sections of a meta/ artifact
# (## References, ## Dependencies, ## Historical Context, ## Related Research,
# ## Source References) and emits, per candidate reference, a record:
#
#   source_type <TAB> key <TAB> target_ref <TAB> anchor <TAB> band
#
# where target_ref is a typed "doc-type:id" reference (or "pr:<n>", or a path
# for a non-meta target), anchor is a stable per-reference source location
# (body:<section-slug>#<seq>), and band is `resolved` (apply mechanically) or
# `ambiguous` (route to the interactive hook) per ADR-0038's two-band model.
#
# Sourceable (functions, prefix lp_) for unit testing the three spike fixes
# directly, and runnable as a CLI: `linkage-parser.sh <file>`.
#
# Portability: no \b (BSD awk/grep lack PCRE word boundaries); keyword
# boundaries use whitespace/label anchors so hyphenated/underscored compounds
# (code-block, code_block) are NOT treated as the bare keyword — this is the
# spike-mandated fix #2. LC_ALL=C pins byte classes across machines.

set -euo pipefail
export LC_ALL=C

LP_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LP_TYPE_PAIRS="${LP_TYPE_PAIRS:-$LP_SCRIPT_DIR/linkage-type-pairs.tsv}"

# The five qualifying body-section headers (exact H2 text).
LP_SECTIONS='## References|## Dependencies|## Historical Context|## Related Research|## Source References'

# ---- Spike fix #1: template-path blocklist ---------------------------------
# A literal documentation placeholder (ADR-NNNN.md, YYYY-MM-DD-topic.md,
# {number}-description.md, ADR-NNNN-description.md) produces no linkage.
# rc 0 = is a template placeholder (drop it).
lp_is_template_path() {
  case "$1" in
    *NNNN* | *YYYY-MM-DD* | *'{'*'}'*) return 0 ;;
    *) return 1 ;;
  esac
}

# ---- Spike fix #2: tightened "blocks" keyword ------------------------------
# Matches "block"/"blocks" only as a standalone label/word, never as part of a
# hyphenated or underscored compound (so "code-block" / "code_block" prose
# produce NO blocks linkage). rc 0 = keyword present.
lp_has_blocks_keyword() {
  printf '%s' "$1" | grep -qE '(^|[[:space:](["'"'"'])[Bb]locks?([[:space:]:,.)"'"'"']|$)'
}

# "Blocked by" / "blocked-by" label. rc 0 = present.
lp_has_blocked_by_keyword() {
  printf '%s' "$1" | grep -qiE '(^|[[:space:](["'"'"'])blocked[ -]by([[:space:]:,.)"'"'"']|$)'
}

# ---- Spike fix #3: "sibling" keyword → relates_to --------------------------
# rc 0 = standalone "sibling" present (not part of a compound).
lp_has_sibling_keyword() {
  printf '%s' "$1" | grep -qE '(^|[[:space:](["'"'"'])[Ss]ibling([[:space:]:,.)"'"'"']|$)'
}

lp_has_supersedes_keyword() {
  printf '%s' "$1" | grep -qiE '(^|[[:space:](["'"'"'])supersedes?([[:space:]:,.)"'"'"']|$)'
}

# "Source:" label at a list-lead / line-lead position.
lp_has_source_label() {
  printf '%s' "$1" | grep -qE '^[[:space:]]*[-*]?[[:space:]]*[Ss]ource:'
}

# ---- Target-type / id resolution -------------------------------------------
# Location → doc-type (exhaustive; reviews by subdir).
lp_type_from_path() {
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

# For a resolved meta path, echo "type<TAB>id". work-item/adr use the bare
# number / ADR-NNNN; every other type uses the full filename stem (matching how
# those types set their own id:). Echoes nothing for an unmapped directory.
lp_resolve_path_target() {
  local path="$1" type stem id
  type="$(lp_type_from_path "$path")"
  [ -n "$type" ] || return 0
  stem="$(basename "$path")"
  stem="${stem%.md}"
  case "$type" in
    work-item) id="$(printf '%s' "$stem" | grep -oE '^[0-9]+' || true)" ;;
    adr) id="$(printf '%s' "$stem" | grep -oE '^ADR-[0-9]+' || true)" ;;
    design-inventory)
      # Nested manifest: the id is the parent directory name (matching the
      # migration's derive_stem), not the manifest basename `inventory`.
      id="$(basename "$(dirname "$path")")"
      ;;
    *) id="$stem" ;;
  esac
  [ -n "$id" ] || return 0
  printf '%s\t%s\n' "$type" "$id"
}

# ---- ADR-0034 type-pair table queries --------------------------------------
# Canonicalise an inferred key for table lookup (inverse → canonical side).
lp_canonical_key() {
  case "$1" in
    blocked_by) echo blocks ;;
    superseded_by) echo supersedes ;;
    *) echo "$1" ;;
  esac
}

# Echo the candidate target-types for (source_type, key) from the table, one
# per line. Used to resolve a bare-number target whose type is unstated.
lp_pair_target_types() {
  local source="$1" key
  key="$(lp_canonical_key "$2")"
  awk -F'\t' -v s="$source" -v k="$key" \
    'NR > 1 && $1 == s && $2 == k { print $3 }' "$LP_TYPE_PAIRS"
}

# rc 0 = (source, key, target) is a row in the table.
lp_pair_in_table() {
  local source="$1" key target="$3"
  key="$(lp_canonical_key "$2")"
  awk -F'\t' -v s="$source" -v k="$key" -v t="$target" \
    'NR > 1 && $1 == s && $2 == k && $3 == t { found = 1 } END { exit (found ? 0 : 1) }' \
    "$LP_TYPE_PAIRS"
}

# ---- Per-reference key inference -------------------------------------------
# Echoes "key<TAB>explicit" where explicit=1 means a label/keyword hint fired
# (high confidence) and explicit=0 means only a section/pair default applied.
lp_infer_key() {
  local source="$1" section="$2" line="$3" target_type="$4"
  # Explicit prose hints, in priority order. Spike fix #3 (sibling) outranks the
  # derived_from pair default it used to fall through to.
  if lp_has_sibling_keyword "$line"; then
    printf 'relates_to\t1\n'
    return
  fi
  if lp_has_supersedes_keyword "$line"; then
    printf 'supersedes\t1\n'
    return
  fi
  if lp_has_blocked_by_keyword "$line"; then
    printf 'blocked_by\t1\n'
    return
  fi
  if lp_has_blocks_keyword "$line"; then
    printf 'blocks\t1\n'
    return
  fi
  if lp_has_source_label "$line"; then
    # "Source:" disambiguation onto ADR-0034's table (ADR-0038 decision).
    case "$target_type" in
      work-item) printf 'parent\t1\n' ;;
      codebase-research | issue-research) printf 'derived_from\t1\n' ;;
      "") printf 'source\t1\n' ;; # non-meta / unresolved target
      *) printf 'source\t1\n' ;;
    esac
    return
  fi
  # Section-default (low confidence — not explicit).
  case "$section" in
    '## Related Research') printf 'derived_from\t0\n' ;;
    '## Source References') printf 'source\t0\n' ;;
    *) printf 'relates_to\t0\n' ;;
  esac
}

# ---- Band classification ---------------------------------------------------
# Echoes "band<TAB>resolved_target_type". A resolved bare-number target gets its
# type filled in from the (source,key) pair when that pair is single-valued.
lp_band() {
  local source="$1" key="$2" target_type="$3" explicit="$4"
  local cand n
  if [ -z "$target_type" ]; then
    # Bare number / unresolved: type must come from a single-valued pair.
    cand="$(lp_pair_target_types "$source" "$key")"
    n="$(printf '%s' "$cand" | grep -c . || true)"
    if [ "$explicit" = "1" ] && [ "$n" = "1" ]; then
      printf 'resolved\t%s\n' "$cand"
    else
      printf 'ambiguous\t\n'
    fi
    return
  fi
  # Target type known. Resolved requires an explicit hint AND either a
  # table-backed pairing or a universally-valid key (`relates_to` is the flat
  # loose-linkage key; `source` is the deterministic non-meta/external-origin
  # rule) — both are valid for any pairing per ADR-0034 and need no table row.
  if [ "$explicit" = "1" ]; then
    if [ "$key" = "relates_to" ] || [ "$key" = "source" ] ||
      lp_pair_in_table "$source" "$key" "$target_type"; then
      printf 'resolved\t%s\n' "$target_type"
      return
    fi
  fi
  printf 'ambiguous\t%s\n' "$target_type"
}

# ---- Reference extraction from a single line -------------------------------
# Echoes one candidate "raw_token" per line, in priority order. Tokens are:
#   - meta/ paths (from backticks, markdown links, or bare)
#   - ADR-<digits> ids
#   - pr:<n> references
#   - inside ## Dependencies only: bare 4-digit ids after a recognised label
lp_extract_tokens() {
  local section="$1" line="$2"
  printf '%s\n' "$line" | grep -oE 'meta/[A-Za-z0-9/_.-]+\.md' || true
  printf '%s\n' "$line" | grep -oE 'ADR-[0-9]{3,4}' || true
  printf '%s\n' "$line" | grep -oE 'pr:[0-9]+' || true
  if [ "$section" = '## Dependencies' ]; then
    # Bare ids only on a recognised label line (Blocks/Blocked by/Related/
    # Depends on/Sibling/Parent), to avoid matching numbers in free prose.
    if printf '%s' "$line" | grep -qiE '^[[:space:]]*[-*]?[[:space:]]*(blocks?|blocked[ -]by|related|depends? on|sibling|parent):'; then
      printf '%s\n' "$line" | grep -oE '[0-9]{4}' || true
    fi
  fi
}

# ---- Whole-file parse ------------------------------------------------------
# Emits TSV records to stdout. $1 = file; $2 (optional) = source_type override
# (else inferred from path). Diagnostics go to stderr.
lp_parse_file() {
  local file="$1" source_type="${2:-}"
  [ -n "$source_type" ] || source_type="$(lp_type_from_path "$file")"
  [ -n "$source_type" ] || source_type="unknown"

  local section="" seq=0 line token ttype tid target_ref keyinfo key explicit bandinfo band rtype
  while IFS= read -r line || [ -n "$line" ]; do
    # Section state machine: any H2 either enters a qualifying section or exits.
    case "$line" in
      '## '*)
        if printf '%s' "$line" | grep -qxE "$LP_SECTIONS"; then
          section="$line"
        else
          section=""
        fi
        continue
        ;;
    esac
    [ -n "$section" ] || continue

    # Per-line dedup of (key, target_ref): a path token and the ADR-id token it
    # contains resolve to the same reference; the spike preferred path-form, so
    # the first emission (paths are extracted first) wins.
    local seen_this_line=""
    while IFS= read -r token; do
      [ -n "$token" ] || continue
      # Spike fix #1: drop literal template placeholders.
      lp_is_template_path "$token" && continue

      ttype=""
      tid=""
      case "$token" in
        meta/*.md)
          local resolved
          resolved="$(lp_resolve_path_target "$token")"
          if [ -n "$resolved" ]; then
            ttype="$(printf '%s' "$resolved" | cut -f1)"
            tid="$(printf '%s' "$resolved" | cut -f2)"
          else
            # Unmapped meta path (specs/talks/global/typo) — leave as path ref.
            ttype=""
            tid=""
          fi
          ;;
        ADR-*)
          ttype="adr"
          tid="$token"
          ;;
        pr:*)
          ttype="pr"
          tid="${token#pr:}"
          ;;
        [0-9][0-9][0-9][0-9])
          ttype=""
          tid="$token"
          ;;
      esac

      keyinfo="$(lp_infer_key "$source_type" "$section" "$line" "$ttype")"
      key="$(printf '%s' "$keyinfo" | cut -f1)"
      explicit="$(printf '%s' "$keyinfo" | cut -f2)"

      bandinfo="$(lp_band "$source_type" "$key" "$ttype" "$explicit")"
      band="$(printf '%s' "$bandinfo" | cut -f1)"
      rtype="$(printf '%s' "$bandinfo" | cut -f2)"
      [ -n "$rtype" ] && ttype="$rtype"

      # Build the emitted target reference.
      if [ -n "$ttype" ] && [ -n "$tid" ]; then
        target_ref="${ttype}:${tid}"
      elif [ -n "$tid" ]; then
        target_ref="$tid" # bare id with no resolved type (ambiguous)
      else
        target_ref="$token" # non-meta path
      fi

      case "$seen_this_line" in
        *"|${key}=${target_ref}|"*) continue ;;
      esac
      seen_this_line="${seen_this_line}|${key}=${target_ref}|"

      local slug
      # shellcheck disable=SC2018,SC2019 # the preceding sed strips every non-ASCII-alphanumeric, so only ASCII A-Z reaches tr; ASCII-only case folding is intended
      slug="$(printf '%s' "$section" | sed 's/^## //; s/[^A-Za-z0-9]/-/g' | tr 'A-Z' 'a-z')"
      printf '%s\t%s\t%s\tbody:%s#%d\t%s\n' \
        "$source_type" "$key" "$target_ref" "$slug" "$seq" "$band"
      seq=$((seq + 1))
    done < <(lp_extract_tokens "$section" "$line")
  done <"$file"
}

# ---- CLI -------------------------------------------------------------------
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  if [ "$#" -lt 1 ]; then
    echo "usage: $0 <file> [source_type]" >&2
    exit 2
  fi
  lp_parse_file "$@"
fi
