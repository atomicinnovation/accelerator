#!/usr/bin/env bash
set -euo pipefail

# Path / gitignore / exit-code hygiene guards for the Linear integration.
# Run: bash skills/integrations/linear/scripts/test-linear-paths.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

LINEAR_COMMON="$SCRIPT_DIR/linear-common.sh"
EXIT_CODES_MD="$SCRIPT_DIR/EXIT_CODES.md"

# ---------------------------------------------------------------------------
echo "=== Linear path hygiene: no hardcoded legacy integrations path ==="
echo ""

LEGACY_PATTERN="meta/integrations/linear"
VIOLATIONS=()
while IFS= read -r -d '' f; do
  [ "$(basename "$f")" = "test-linear-paths.sh" ] && continue
  if grep -qF "$LEGACY_PATTERN" "$f" 2>/dev/null; then
    VIOLATIONS+=("$(basename "$f")")
  fi
done < <(find "$SCRIPT_DIR" -name "*.sh" -print0)

if [ ${#VIOLATIONS[@]} -eq 0 ]; then
  echo "  PASS: no .sh file contains hardcoded legacy integrations path"
  PASS=$((PASS + 1))
else
  echo "  FAIL: these files still contain the legacy path ($LEGACY_PATTERN):"
  for v in "${VIOLATIONS[@]}"; do echo "    $v"; done
  FAIL=$((FAIL + ${#VIOLATIONS[@]}))
fi
echo ""

# ---------------------------------------------------------------------------
# Unlike Jira, the Linear gitignore rules are not pinned to a migration-script
# copy (the linear state path is net-new — no migration writes it). Assert the
# rules in linear-common.sh directly.
echo "=== Linear gitignore rules: LINEAR_INNER_GITIGNORE_RULES is exactly the expected set ==="
echo ""

extract_rules() {
  awk '/LINEAR_INNER_GITIGNORE_RULES=\(/{found=1; next} found && /^\)/{exit} found{gsub(/[[:space:]'"'"']/, ""); print}' "$1" | sort
}
COMMON_RULES=$(extract_rules "$LINEAR_COMMON")
EXPECTED_RULES=$(printf '%s\n' ".lock/" ".refresh-meta.json" "viewer.json" | sort)

if [ "$COMMON_RULES" = "$EXPECTED_RULES" ]; then
  echo "  PASS: LINEAR_INNER_GITIGNORE_RULES = {viewer.json, .refresh-meta.json, .lock/}"
  PASS=$((PASS + 1))
else
  echo "  FAIL: LINEAR_INNER_GITIGNORE_RULES mismatch"
  echo "    got:      $(echo "$COMMON_RULES" | tr '\n' ' ')"
  echo "    expected: $(echo "$EXPECTED_RULES" | tr '\n' ' ')"
  FAIL=$((FAIL + 1))
fi

echo "Test: catalogue.json is NOT in the gitignore rules (it is committed)"
if printf '%s\n' "$COMMON_RULES" | grep -qx "catalogue.json"; then
  echo "  FAIL: catalogue.json must not be gitignored (it is team-shared)"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: catalogue.json is not gitignored"
  PASS=$((PASS + 1))
fi
echo ""

# ---------------------------------------------------------------------------
# Derived-doc honesty: every `readonly E_*=NN` declared in a flow script must
# appear with the same value in EXIT_CODES.md. The constants are the source of
# truth; the table is derived.
echo "=== Exit-code constants match EXIT_CODES.md ==="
echo ""

MISMATCH=0
CHECKED=0
while IFS= read -r -d '' flow; do
  while IFS= read -r decl; do
    # decl looks like: readonly E_FOO=123
    name=$(printf '%s' "$decl" | sed -E 's/^readonly[[:space:]]+([A-Z_][A-Z0-9_]*)=.*/\1/')
    value=$(printf '%s' "$decl" | sed -E 's/^readonly[[:space:]]+[A-Z_][A-Z0-9_]*=([0-9]+).*/\1/')
    [ -z "$name" ] && continue
    [ -z "$value" ] && continue
    CHECKED=$((CHECKED + 1))
    # Look for a table row carrying both the code and the name.
    if grep -E "^\|[[:space:]]*${value}[[:space:]]*\|" "$EXIT_CODES_MD" | grep -qF "\`${name}\`"; then
      :
    else
      echo "  FAIL: $(basename "$flow"): $name=$value not documented with that value in EXIT_CODES.md"
      MISMATCH=$((MISMATCH + 1))
    fi
  done < <(grep -hoE '^[[:space:]]*readonly[[:space:]]+E_[A-Z0-9_]+=[0-9]+' "$flow" | sed -E 's/^[[:space:]]*//')
done < <(find "$SCRIPT_DIR" -name "linear-*-flow.sh" -print0)

if [ "$MISMATCH" -eq 0 ]; then
  echo "  PASS: all $CHECKED flow exit-code constants match EXIT_CODES.md"
  PASS=$((PASS + 1))
else
  FAIL=$((FAIL + MISMATCH))
fi
echo ""

test_summary
