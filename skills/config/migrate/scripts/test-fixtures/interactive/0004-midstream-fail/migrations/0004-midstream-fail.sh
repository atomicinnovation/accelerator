#!/usr/bin/env bash
# DESCRIPTION: Fail mid-stream — Phase 5 negative test.
# INTERACTIVE: yes
# shellcheck disable=SC2154 # CLAUDE_PLUGIN_ROOT provided by the interactive-migration harness environment
set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"

migration_emit_transformations() {
  harness_emit_transformation key=k1 path=p1 anchor=a proposed=v1 \
    predicate_value=ambiguous display="x"
  harness_emit_transformation key=k2 path=p2 anchor=a proposed=v2 \
    predicate_value=ambiguous display="x"
  harness_emit_transformation key=k3 path=p3 anchor=a proposed=v3 \
    predicate_value=ambiguous display="x"
}
migration_evaluate_predicate() { return 0; }
migration_validate_edit() { return 0; }

migration_apply_decision() {
  local key="$1"
  if [ "$key" = "k3" ]; then
    harness_reject "synthetic apply failure on k3"
    return 1
  fi
  return 0
}

harness_run
