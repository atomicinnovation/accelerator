#!/usr/bin/env bash
# DESCRIPTION: Empty interactive migration (no transformations) — Phase 3 smoke test.
# INTERACTIVE: yes
# shellcheck disable=SC2154 # CLAUDE_PLUGIN_ROOT provided by the interactive-migration harness environment
set -euo pipefail
# shellcheck source=../../../../../../../../scripts/atomic-common.sh
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
# shellcheck source=../../../../../../../../scripts/interactive-harness.sh
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"

migration_emit_transformations() {
  : # no transformations
}

migration_evaluate_predicate() { return 0; }
migration_validate_edit() { return 0; }
migration_apply_decision() { return 0; }

harness_run
