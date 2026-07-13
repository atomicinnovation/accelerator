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

# The config-driven doc-type matcher, shared with the corpus validator and the
# 0007 migration. Sourced (not reimplemented) so the directory→type fact has one
# encoding; see lp_resolve_path_target.
DOC_TYPE_INFERENCE="${DOC_TYPE_INFERENCE:-$LP_SCRIPT_DIR/doc-type-inference.sh}"
# shellcheck source=doc-type-inference.sh
source "$DOC_TYPE_INFERENCE"
DOC_TYPE_TABLE="${DOC_TYPE_TABLE:-$LP_SCRIPT_DIR/doc-type-table.sh}"
# shellcheck source=doc-type-table.sh
source "$DOC_TYPE_TABLE"

# Resolve the table once, before any inference. LP_PROJECT_ROOT lets a caller
# whose CWD is not the corpus root — the 0007 migration invokes this as a
# subprocess — point config resolution at the right tree.
#
# Fail closed: with no table, infer_type_from_path returns empty for everything,
# which would silently degrade every typed reference to a raw path in the
# ambiguous band rather than reporting a problem.
if ! load_doc_type_table "${LP_PROJECT_ROOT:-}"; then
  echo "linkage-parser.sh: could not resolve the doc-type table" >&2
  if [ "${BASH_SOURCE[0]}" = "${0}" ]; then exit 1; else return 1; fi
fi

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
# Location → doc-type is NOT encoded here. It comes from the shared, config-driven
# infer_type_from_path (doc-type-inference.sh), the same matcher the corpus
# validator and the 0007 migration use.
#
# This file used to carry its own hardcoded `case` over directory globs, ordered
# by hand so the review arms would not be shadowed by the generic ones. It was the
# only one of the three surfaces that was not config-driven — so linkage silently
# stopped resolving in a re-pathed corpus while validation and migration carried
# on working — and it had no meta/prs arm at all, so a PR description was
# unresolvable. The shared matcher is longest-configured-dir-wins, which makes the
# review/generic ordering fall out rather than needing to be maintained.

# For a resolved meta path, echo "type<TAB>id". The id derivation MUST agree with
# the 0007 rewrite awk's path_to_typed and with corpus::linkage — a reference
# resolved to a different id than the target document derives for itself points at
# nothing. Echoes nothing for a path outside every configured doc-type directory.
lp_resolve_path_target() {
  local path="$1" type stem id
  type="$(infer_type_from_path "$path")"
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
    pr-description)
      # Identified by its PR number (templates/pr-description.md: id:
      # "{pr_number}"): a genuine pr-/PR- segment, else a leading number on a
      # stem that is not date-prefixed.
      id="$(printf '%s' "$stem" | grep -oE '(^|-)[Pp][Rr]-?[0-9]+' | grep -oE '[0-9]+' | head -1 || true)"
      if [ -z "$id" ]; then
        case "$stem" in
          [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]*) : ;;
          *) id="$(printf '%s' "$stem" | grep -oE '^[0-9]+' | head -1 || true)" ;;
        esac
      fi
      [ -n "$id" ] || id="$stem"
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
# The scan roots: the distinct leading segments of the configured doc-type
# directories, ERE-escaped and alternated. Scanning by ROOT rather than by full
# directory keeps an out-of-scope subtree (meta/docs/...) a candidate, exactly as
# the old literal `meta/` scan did — it is extracted, fails to infer a type, and
# is carried through as a raw path ref.
#
# Built once, after the table is injected: a hardcoded `meta/` here would make the
# parser blind to a re-pathed corpus even though its type inference is config-aware.
LP_PATH_RE=""
lp_build_path_re() {
  local dir root seen="" alt=""
  for dir in "${DOC_TYPE_INJECTED_DIRS[@]}"; do
    [ -n "$dir" ] || continue
    root="${dir%%/*}"
    [ -n "$root" ] || continue
    case "|$seen|" in *"|$root|"*) continue ;; esac
    seen="${seen:+$seen|}$root"
    # A config value reaches a regex here, so escape ERE metacharacters. `$` sits
    # last in the bracket expression so the literal set carries no `$(` sequence.
    root="$(printf '%s' "$root" | sed 's/[][\\.*^(){}?+|$]/\\&/g')"
    alt="${alt:+$alt|}$root"
  done
  [ -n "$alt" ] || return 1
  LP_PATH_RE="($alt)/[A-Za-z0-9/_.-]+\.md"
}
lp_build_path_re || {
  echo "linkage-parser.sh: no configured doc-type directories to scan" >&2
  if [ "${BASH_SOURCE[0]}" = "${0}" ]; then exit 1; else return 1; fi
}

# Echoes one candidate "raw_token" per line, in priority order. Tokens are:
#   - paths under a configured doc-type root (from backticks, markdown links, bare)
#   - ADR-<digits> ids
#   - pr:<n> references
#   - inside ## Dependencies only: bare 4-digit ids after a recognised label
lp_extract_tokens() {
  local section="$1" line="$2"
  printf '%s\n' "$line" | grep -oE "$LP_PATH_RE" || true
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
  [ -n "$source_type" ] || source_type="$(infer_type_from_path "$file")"
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
        # The extractor only yields path tokens ending `.md` — ADR ids, pr: refs
        # and bare ids never do — so the suffix identifies a path without pinning
        # it to a hardcoded root.
        *.md)
          local resolved
          resolved="$(lp_resolve_path_target "$token")"
          if [ -n "$resolved" ]; then
            ttype="$(printf '%s' "$resolved" | cut -f1)"
            tid="$(printf '%s' "$resolved" | cut -f2)"
          else
            # A path under a configured root but outside every doc-type directory
            # (specs/talks/global/typo) — leave it as a raw path ref.
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
