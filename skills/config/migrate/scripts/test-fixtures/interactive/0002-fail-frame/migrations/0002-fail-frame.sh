#!/usr/bin/env bash
# DESCRIPTION: Emit FAIL after READY — Phase 3 negative test.
# INTERACTIVE: yes
set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"
migration_emit_transformations() { :; }
migration_evaluate_predicate() { return 0; }
migration_validate_edit() { return 0; }
migration_apply_decision() { return 0; }

# Override harness_run to emit FAIL right after handshake.
harness_run_fail() {
  read_frame
  emit_frame READY ".accelerator/state/migrations-0002-fail-frame-session.jsonl"
  emit_frame FAIL "synthetic failure for testing"
  exit 1
}
harness_run_fail
