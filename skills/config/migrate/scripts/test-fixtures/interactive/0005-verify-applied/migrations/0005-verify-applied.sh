#!/usr/bin/env bash
# DESCRIPTION: Resume-integrity check fixture — Phase 6.
# INTERACTIVE: yes
# shellcheck disable=SC2154 # CLAUDE_PLUGIN_ROOT/PROJECT_ROOT provided by the interactive-migration harness environment
set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"

migration_emit_transformations() {
  harness_emit_transformation key=k1 path=marker anchor=a proposed=v \
    predicate_value=ambiguous display="x"
}
migration_evaluate_predicate() { return 0; }
migration_validate_edit() { return 0; }

migration_apply_decision() {
  printf 'mutated\n' >"$PROJECT_ROOT/marker"
}

# Verifies the mutation actually landed. Returns non-zero if marker
# file is missing or empty.
migration_verify_applied() {
  [ -s "$PROJECT_ROOT/marker" ]
}

harness_run
