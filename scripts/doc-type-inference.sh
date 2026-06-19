#!/usr/bin/env bash
# Config-driven, path-based doc-type classification, single-sourced by the 0007
# migration and the corpus validator (previously byte-identical duplicated
# copies, then a hardcoded directory-suffix table paired with a freeform-subtree
# denylist — both now retired in favour of a config-driven allowlist).
#
# Required precondition (NOT a pure function — an explicit injected dependency):
# before the first infer_type_from_path / out_of_scope call, the caller MUST
# populate the parallel arrays DOC_TYPE_INJECTED_NAMES[] / DOC_TYPE_INJECTED_DIRS[]
# (resolved, normalised dirs) and set DOC_TYPE_TABLE_INJECTED=1, exactly once and
# immutable for the run, via the shared load_doc_type_table helper
# (doc-type-table.sh → config-read-doc-type-paths.sh). The functions read those
# caller-populated globals as a snapshot; the injected arrays are named distinctly
# from the static config-defaults.sh DOC_TYPE_NAMES registry so the always-present
# static array cannot be mistaken for the injected snapshot.
#
# Fail-closed: with no table injected, infer_type_from_path returns empty and
# out_of_scope returns true (everything out of scope) — a missing injection can
# never silently fail open and validate/migrate the wrong file set.
#
# bash 3.2 safe (no associative arrays, no ${var,,}, no ${var//} slash
# replacement); safe to source under `set -euo pipefail`.
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

# Path → doc-type by injected-table match. Most-specific (longest) injected dir
# wins, by plain integer length comparison (no sort, no slash replacement). A
# dir D matches path P when D appears as an interior path segment (absolute or
# nested root) OR P begins exactly with D — both arms quote D so glob
# metacharacters in a config value match literally and the trailing `/` enforces
# a segment boundary (meta/prs never matches meta/prs-archive). On an exact-
# length tie (two doc-types configured to the same dir) the first array entry
# wins deterministically. Empty (fail-closed) when no table is injected.
infer_type_from_path() {
  [ "${DOC_TYPE_TABLE_INJECTED:-}" = "1" ] || {
    echo ""
    return 0
  }
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
  echo "$best_type"
}

# Out of scope (skip entirely) iff the path resolves to no configured doc-type
# directory — the config-driven allowlist. Fail-closed (out of scope) when no
# table is injected.
out_of_scope() {
  [ "${DOC_TYPE_TABLE_INJECTED:-}" = "1" ] || return 0
  [ -z "$(infer_type_from_path "$1")" ] && return 0
  return 1
}
