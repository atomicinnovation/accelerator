#!/usr/bin/env bash
# Path-based doc-type classification, single-sourced by the 0007 migration and
# the corpus validator (previously byte-identical duplicated copies).
#
# Two modes, selected by the DOC_TYPE_TABLE_INJECTED sentinel:
#
#   * INJECTED (allowlist) — the caller populates DOC_TYPE_INJECTED_NAMES[] /
#     DOC_TYPE_INJECTED_DIRS[] (resolved, normalised dirs) via the shared
#     load_doc_type_table helper (doc-type-table.sh) exactly once before first
#     use, then sets DOC_TYPE_TABLE_INJECTED=1. infer_type_from_path matches the
#     path against the injected dirs (most-specific wins); out_of_scope returns
#     true iff no injected dir matches. This is the config-aware allowlist.
#   * FALLBACK (legacy denylist) — when no table is injected, the historical
#     hardcoded suffix table + specs/talks/global/docs/announcements denylist is
#     used, preserving pre-config behaviour. (Removed in a later change once both
#     consumers inject; pinned meanwhile by a golden fixture.)
#
# The injected arrays are named distinctly from the static config-defaults.sh
# DOC_TYPE_NAMES registry so the always-present static array cannot be mistaken
# for the injected snapshot. bash 3.2 safe (no associative arrays, no ${var,,},
# no ${var//} slash replacement); safe to source under `set -euo pipefail`.
#
# NB: the awk rewrite (0007-frontmatter-rewrite.awk:path_to_typed) encodes the
# SAME directory→type fact for a DIFFERENT input — the referenced meta-path
# inside a linkage value, not the current file — so it cannot consume the
# file-level `-v type` channel and must stay a third, in-runtime copy. The two
# encodings MUST be kept aligned; a fixture in test-migrate-0007.sh asserts a
# meta/prs/ path resolves to pr-description in both surfaces.

# Default the injected arrays so the functions are set -u safe and shellcheck
# sees them assigned; load_doc_type_table (doc-type-table.sh) overwrites them
# before first use. Guarded so re-sourcing cannot clobber a populated table.
[ -n "${DOC_TYPE_INJECTED_NAMES+set}" ] || DOC_TYPE_INJECTED_NAMES=()
[ -n "${DOC_TYPE_INJECTED_DIRS+set}" ] || DOC_TYPE_INJECTED_DIRS=()

# ---- Injected-table matching (allowlist mode) ------------------------------
# Most-specific (longest) injected dir wins, by plain integer length comparison
# (no sort, no slash replacement). A dir D matches path P when D appears as an
# interior path segment (absolute/nested root) OR P begins exactly with D —
# both arms quote D so glob metacharacters in a config value match literally and
# the trailing `/` enforces a segment boundary (meta/prs never matches
# meta/prs-archive). On an exact-length tie (two doc-types configured to the
# same dir) the first array entry wins deterministically.
_infer_type_injected() {
  local path="$1" i d len best_len=-1 best_type=""
  for i in "${!DOC_TYPE_INJECTED_DIRS[@]}"; do
    d="${DOC_TYPE_INJECTED_DIRS[$i]}"
    [ -n "$d" ] || continue
    case "$path" in
      */"$d"/* | "$d"/*) ;;
      *) continue ;;
    esac
    len=${#d}
    if [ "$len" -gt "$best_len" ]; then
      best_len="$len"
      best_type="${DOC_TYPE_INJECTED_NAMES[$i]}"
    fi
  done
  printf '%s\n' "$best_type"
}

# ---- Legacy hardcoded matching (fallback mode) -----------------------------
# Location → doc-type (exhaustive; reviews discriminated by subdirectory, which
# MUST precede the generic */work/* and */plans/* and the bare */prs/* arms).
_infer_type_fallback() {
  case "$1" in
    */reviews/plans/*) echo plan-review ;;
    */reviews/work/*) echo work-item-review ;;
    */reviews/prs/*) echo pr-review ;;
    */prs/*) echo pr-description ;; # after reviews/prs so it can't shadow it
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

infer_type_from_path() {
  if [ "${DOC_TYPE_TABLE_INJECTED:-}" = "1" ]; then
    _infer_type_injected "$1"
  else
    _infer_type_fallback "$1"
  fi
}

# Out of scope (skip entirely). In injected mode: out iff the path resolves to
# no configured doc-type dir (the allowlist). In fallback mode: the legacy
# denylist — specs/talks/global (freeform) and meta/docs/ + meta/announcements/
# (freeform docs the plugin does not own; no schema type), anchored to
# */meta/docs/* and */meta/announcements/* so they cannot over-match a nested
# segment elsewhere in the corpus.
out_of_scope() {
  if [ "${DOC_TYPE_TABLE_INJECTED:-}" = "1" ]; then
    [ -z "$(infer_type_from_path "$1")" ] && return 0
    return 1
  fi
  case "$1" in
    */specs/* | */talks/* | */global/* | */meta/docs/* | */meta/announcements/*) return 0 ;;
    *) return 1 ;;
  esac
}
