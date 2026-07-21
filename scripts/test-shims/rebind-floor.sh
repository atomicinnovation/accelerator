#!/usr/bin/env bash
set -euo pipefail

# Known-positive floor for the config-suite rebind.
#
# Proves the repointed suites reach the compiled launcher and never the bash
# removal-set scripts: every stubbable removal-set script is temporarily
# replaced by a stub that fails loudly on execution, yet the suites still pass.
# A binding that slipped through un-repointed would invoke a stub and fail.
#
# Run this SERIALLY (`bash scripts/test-shims/rebind-floor.sh`), never inside
# the parallel CI aggregate: it stubs scripts/ in place under a restore trap,
# and the aggregate runs the formatters, linters and other suites against the
# same tree concurrently. It is a standalone completeness proof, recorded at
# the phase-4 commit, not a per-build gate.
#
# The stub keeps the original body appended after its failing exit, so the
# non-repointable source-text greps (the AGENT_KEYS extraction) still find what
# they read. Deleted with the rest of test-shims/ when the removal set retires.
#
# Four removal-set scripts are deliberately NOT stubbed, because retained bash
# still invokes them in-process during these suites — stubbing them would break
# a non-binding test, not surface a missed binding:
#   - config-read-value.sh, config-read-path.sh: called by the retained
#     config-common.sh config_resolve_template (exercised by its own tests).
#   - config-read-work.sh: called by the retained work-common.sh
#     work_resolve_default_project (exercised by its own tests).
#   - config-summary.sh: called by the config-detect.sh hook (retained until
#     the hook cutover), whose output these suites assert.
# Their bindings are proven by the repointed suites passing against the binary;
# the remaining fifteen are proven here.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPTS_DIR/.." && pwd)"

REMOVAL_SET="
config-read-agent-name.sh
config-read-agents.sh
config-read-context.sh
config-read-skill-context.sh
config-read-skill-instructions.sh
config-read-all-paths.sh
config-read-doc-type-paths.sh
config-dump.sh
config-read-review.sh
config-read-template.sh
config-list-template.sh
config-show-template.sh
config-eject-template.sh
config-diff-template.sh
config-reset-template.sh
"

if [ -z "${ACCELERATOR_BIN:-}" ]; then
  cargo build --quiet --manifest-path "$PLUGIN_ROOT/cli/Cargo.toml" \
    --bin accelerator
  ACCELERATOR_BIN="$PLUGIN_ROOT/cli/target/debug/accelerator"
fi
export ACCELERATOR_BIN
export CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT"

STASH="$(mktemp -d)"

restore() {
  local name
  for name in $REMOVAL_SET; do
    if [ -f "$STASH/$name" ]; then
      mv -f "$STASH/$name" "$SCRIPTS_DIR/$name"
    fi
  done
  rm -rf "$STASH"
}
trap restore EXIT INT TERM

for name in $REMOVAL_SET; do
  mv "$SCRIPTS_DIR/$name" "$STASH/$name"
  {
    printf '#!/usr/bin/env bash\n'
    printf 'echo "FLOOR: removal-set script %s was invoked" >&2\n' "$name"
    printf 'exit 42\n'
    cat "$STASH/$name"
  } >"$SCRIPTS_DIR/$name"
  chmod 0755 "$SCRIPTS_DIR/$name"
done

echo "=== rebind floor: removal-set stubbed, re-running suites ==="
FLOOR_RC=0
bash "$SCRIPTS_DIR/test-config.sh" >/dev/null 2>&1 || FLOOR_RC=$?
if [ "$FLOOR_RC" -ne 0 ]; then
  echo "FAIL: test-config.sh did not pass with the removal set stubbed" >&2
  echo "  a binding still reaches a bash removal-set script (rc=$FLOOR_RC)" >&2
  exit 1
fi

FLOOR_RC=0
bash "$SCRIPTS_DIR/test-config-read-doc-type-paths.sh" >/dev/null 2>&1 ||
  FLOOR_RC=$?
if [ "$FLOOR_RC" -ne 0 ]; then
  echo "FAIL: test-config-read-doc-type-paths.sh did not pass stubbed" >&2
  echo "  a binding still reaches config-read-doc-type-paths.sh (rc=$FLOOR_RC)" >&2
  exit 1
fi

echo "PASS: both suites pass with every removal-set script stubbed"
