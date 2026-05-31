#!/usr/bin/env bash
# DESCRIPTION: Worked example for the interactive contract.
# INTERACTIVE: yes
set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"

# Three transformations: one ambiguous-band (prompted), one
# resolved-band (mechanical), one ambiguous-band where the validator
# enforces a non-empty user value.
migration_emit_transformations() {
  harness_extras_set band ambiguous
  harness_extras_set prose "the linkage paragraph"
  harness_emit_transformation \
    key=link-A path=meta/work/example-A.md anchor=14 \
    proposed=0034-foo predicate_value=ambiguous \
    display="Proposed value: 0034-foo
Surrounding prose: the linkage paragraph"

  harness_extras_set band resolved
  harness_extras_set prose "the unambiguous citation"
  harness_emit_transformation \
    key=link-B path=meta/work/example-B.md anchor=8 \
    proposed=0042-bar predicate_value=resolved \
    display="Proposed value: 0042-bar
Surrounding prose: the unambiguous citation"

  harness_extras_set band ambiguous
  harness_extras_set prose "the paragraph the author wants to revise"
  harness_emit_transformation \
    key=link-C path=meta/work/example-C.md anchor=21 \
    proposed=0007-baz predicate_value=ambiguous \
    display="Proposed value: 0007-baz
Surrounding prose: the paragraph the author wants to revise"
}

migration_evaluate_predicate() {
  local band
  band=$(harness_field band)
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
  local abs="$PROJECT_ROOT/$path"
  mkdir -p "$(dirname "$abs")"
  printf '%s:%s=%s\n' "$key" "$anchor" "$value" >> "$abs"
}

harness_run
