#!/usr/bin/env bash
# DESCRIPTION: Predicate-routing fixture — Phase 4 (mixed band ambiguous/resolved).
# INTERACTIVE: yes
# shellcheck disable=SC2154 # CLAUDE_PLUGIN_ROOT/PROJECT_ROOT provided by the interactive-migration harness environment
set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"

# Fixture data file laid down by the test before each run. Format: one
# `key|path|anchor|proposed|band|prose` line per transformation.
FIXTURE_DATA="$PROJECT_ROOT/.fixture/transformations"

migration_emit_transformations() {
  [ -f "$FIXTURE_DATA" ] || return 0
  local key path anchor proposed band prose
  while IFS='|' read -r key path anchor proposed band prose; do
    [ -z "$key" ] && continue
    harness_extras_set band "$band"
    harness_extras_set prose "$prose"
    harness_emit_transformation \
      key="$key" path="$path" anchor="$anchor" \
      proposed="$proposed" predicate_value="$band" \
      display="Proposed value: $proposed
Surrounding prose: $prose"
  done <"$FIXTURE_DATA"
}

migration_evaluate_predicate() {
  local band
  band=$(harness_field band)
  # Route to prompt iff band is ambiguous.
  [ "$band" = "ambiguous" ]
}

migration_validate_edit() {
  local key="$1" path="$2" anchor="$3" proposed="$4" user_value="$5"
  if [ -z "$user_value" ]; then
    harness_reject "empty value not allowed"
    return 1
  fi
  return 0
}

migration_apply_decision() {
  local key="$1" path="$2" anchor="$3" decision="$4" value="$5"
  # Write a per-key sentinel so tests can verify which keys were applied,
  # in what order, with what decision/value.
  local sentinel_dir="$PROJECT_ROOT/.fixture/applied"
  mkdir -p "$sentinel_dir"
  printf '%s\t%s\t%s\t%s\t%s\n' \
    "$key" "$path" "$anchor" "$decision" "$value" \
    >>"$sentinel_dir/log"
  # Mutate the target artifact: paths in the transformations file are
  # always relative to PROJECT_ROOT.
  local abs="$PROJECT_ROOT/$path"
  mkdir -p "$(dirname "$abs")"
  printf '%s=%s\n' "$anchor" "$value" >>"$abs"
}

harness_run
