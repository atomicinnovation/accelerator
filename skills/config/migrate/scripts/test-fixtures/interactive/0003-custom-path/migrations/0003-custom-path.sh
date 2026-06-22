#!/usr/bin/env bash
# DESCRIPTION: declares a non-canonical session-log path — Phase 4 rejection test.
# INTERACTIVE: yes
# shellcheck disable=SC2154 # CLAUDE_PLUGIN_ROOT provided by the interactive-migration harness environment
# shellcheck disable=SC2329 # stub migration_* hooks are required by the harness contract
set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"
migration_emit_transformations() { :; }
migration_evaluate_predicate() { return 0; }
migration_validate_edit() { return 0; }
migration_apply_decision() { return 0; }
migration_session_log_path() { printf 'custom/weird-session.jsonl\n'; }
harness_run
