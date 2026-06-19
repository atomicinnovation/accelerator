#!/usr/bin/env bash
# Shared loader that turns config-read-doc-type-paths.sh's TSV into the injected
# arrays consumed by doc-type-inference.sh. Owning the parse, the resolve-once
# invariant, and the DOC_TYPE_TABLE_INJECTED sentinel in ONE place means the two
# consumers (the corpus validator and the 0007 migration) cannot drift on TSV
# shape.
#
# Sourced alongside doc-type-inference.sh. Intentionally omits set -euo pipefail
# (inherits the caller's shell options, matching the config-common.sh
# convention). bash-3.2 safe (no associative arrays, no ${var,,}).

DOC_TYPE_TABLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Test-only override seam (mirrors the validator's DOC_TYPE_INFERENCE /
# FM_EMISSION_RULES seams); not a production knob.
DOC_TYPE_PATHS_RESOLVER="${DOC_TYPE_PATHS_RESOLVER:-$DOC_TYPE_TABLE_DIR/config-read-doc-type-paths.sh}"

# Populate DOC_TYPE_INJECTED_NAMES[] / DOC_TYPE_INJECTED_DIRS[] from the
# resolver and set the DOC_TYPE_TABLE_INJECTED sentinel. Call exactly once,
# before any infer_type_from_path / out_of_scope use, so the run observes a
# single immutable scope.
#
# Usage: load_doc_type_table [project-root]
#   project-root is forwarded to the resolver so config resolves against the
#   corpus root rather than the caller's CWD (the migration passes its
#   canonicalised PROJECT_ROOT; the validator omits it → CWD).
#
# Returns non-zero (arrays left empty, sentinel unset) if the resolver fails or
# emits zero rows, so callers can fail closed.
load_doc_type_table() {
  local root="${1:-}"
  DOC_TYPE_INJECTED_NAMES=()
  DOC_TYPE_INJECTED_DIRS=()
  local tsv rc=0
  if [ -n "$root" ]; then
    tsv="$(bash "$DOC_TYPE_PATHS_RESOLVER" "$root")" || rc=$?
  else
    tsv="$(bash "$DOC_TYPE_PATHS_RESOLVER")" || rc=$?
  fi
  [ "$rc" -eq 0 ] || return 1
  local name dir
  while IFS=$'\t' read -r name dir; do
    [ -n "$name" ] || continue
    DOC_TYPE_INJECTED_NAMES+=("$name")
    DOC_TYPE_INJECTED_DIRS+=("$dir")
  done <<<"$tsv"
  [ "${#DOC_TYPE_INJECTED_NAMES[@]}" -gt 0 ] || return 1
  # Consumed cross-file by doc-type-inference.sh to select allowlist vs fallback.
  # shellcheck disable=SC2034
  DOC_TYPE_TABLE_INJECTED=1
  return 0
}
