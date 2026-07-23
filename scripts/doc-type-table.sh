#!/usr/bin/env bash
# Shared loader that turns the `config paths --doc-types --format tsv` TSV into
# the injected arrays consumed by doc-type-inference.sh. Owning the parse, the
# resolve-once invariant, and the DOC_TYPE_TABLE_INJECTED sentinel in ONE place
# means the two consumers (the corpus validator and the 0007 migration) cannot
# drift on TSV shape.
#
# Sourced alongside doc-type-inference.sh. Intentionally omits set -euo pipefail
# (inherits the caller's shell options, matching the config-common.sh
# convention). bash-3.2 safe (no associative arrays, no ${var,,}).

DOC_TYPE_TABLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# The resolver is a full command, not a bare path, so it word-splits at the
# call site; a test may override it with a single stub path (the validator's
# DOC_TYPE_INFERENCE / FM_EMISSION_RULES seams do the same). Not a production
# knob.
DOC_TYPE_PATHS_RESOLVER="${DOC_TYPE_PATHS_RESOLVER:-${ACCELERATOR_BIN:-${DOC_TYPE_TABLE_DIR%/scripts}/bin/accelerator} config paths --doc-types --format tsv}"

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
  local -a resolver_cmd
  # shellcheck disable=SC2206 # the resolver is a command string, split on purpose
  resolver_cmd=($DOC_TYPE_PATHS_RESOLVER)
  # Migrations run against a still-legacy layout; the shell's migration-mode
  # sentinel becomes the explicit read-side flag the launcher honours (the
  # launcher ignores the env var itself). This is the one shared helper the
  # confinement check allowlists to carry --allow-legacy-layout.
  if [ "${ACCELERATOR_MIGRATION_MODE:-}" = "1" ]; then
    resolver_cmd+=(--allow-legacy-layout)
  fi
  if [ -n "${DOC_TYPE_TABLE_TSV:-}" ]; then
    # An already-resolved table, handed down by a caller that spawned the
    # resolver itself. The 0007 migration invokes the linkage parser once per
    # file; without this seam each invocation would re-resolve, turning the
    # resolve-once invariant into one resolver spawn per corpus file.
    tsv="$DOC_TYPE_TABLE_TSV"
  elif [ -n "$root" ]; then
    tsv="$("${resolver_cmd[@]}" "$root")" || rc=$?
  else
    tsv="$("${resolver_cmd[@]}")" || rc=$?
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
