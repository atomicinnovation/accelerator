#!/usr/bin/env bash
set -euo pipefail

# Regression guard: assert no Jira script contains the legacy hardcoded
# integrations state path. The default is .accelerator/state/integrations/jira/
# (read via config-read-path.sh at runtime).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

echo "=== Jira path hygiene: no hardcoded legacy integrations path ==="
echo ""

LEGACY_PATTERN="meta/integrations/jira"
JIRA_SCRIPTS_DIR="$SCRIPT_DIR"

VIOLATIONS=()
while IFS= read -r -d '' f; do
  # Skip this test file itself
  [ "$(basename "$f")" = "test-jira-paths.sh" ] && continue
  if grep -qF "$LEGACY_PATTERN" "$f" 2>/dev/null; then
    VIOLATIONS+=("$(basename "$f")")
  fi
done < <(find "$JIRA_SCRIPTS_DIR" -name "*.sh" -print0)

if [ ${#VIOLATIONS[@]} -eq 0 ]; then
  echo "  PASS: no .sh file contains hardcoded legacy integrations path"
  PASS=$((PASS + 1))
else
  echo "  FAIL: these files still contain the legacy path ($LEGACY_PATTERN):"
  for v in "${VIOLATIONS[@]}"; do
    echo "    $v"
  done
  FAIL=$((FAIL + ${#VIOLATIONS[@]}))
fi

echo ""

# JIRA_INNER_GITIGNORE_RULES in jira-common.sh must match the
# JIRA_INNER_GITIGNORE_RULES constant in accelerator-migrate's Rust port of
# migration 0003 — the bash original was retired in the migration engine's
# Rust cutover.
echo "=== Jira gitignore rules: jira-common.sh matches migration 0003 ==="
echo ""

JIRA_COMMON="$JIRA_SCRIPTS_DIR/jira-common.sh"
MIGRATION="$PLUGIN_ROOT/cli/migrate/src/migrations/m0003.rs"

# Extract JIRA_INNER_GITIGNORE_RULES values from a bash array declaration:
# lines between the opening '(' and ')' only.
extract_bash_rules() {
  local file="$1"
  awk '/JIRA_INNER_GITIGNORE_RULES=\(/{found=1; next} found && /^\)/{exit} found{gsub(/[[:space:]'"'"']/, ""); print}' "$file" | sort
}

# Extract JIRA_INNER_GITIGNORE_RULES values from the Rust `[&str; N] = [...]`
# array literal: every double-quoted string between the constant's name and
# the literal's closing `];` (NOT the constant's own `[&str; N]` type
# annotation, whose `;` precedes its `]` rather than following it).
extract_rust_rules() {
  local file="$1"
  awk '/const JIRA_INNER_GITIGNORE_RULES/{found=1} found{print; if (/\];/) exit}' "$file" |
    grep -o '"[^"]*"' |
    tr -d '"' |
    sort
}

COMMON_RULES=$(extract_bash_rules "$JIRA_COMMON")
MIGRATION_RULES=$(extract_rust_rules "$MIGRATION")

if [ "$COMMON_RULES" = "$MIGRATION_RULES" ]; then
  echo "  PASS: JIRA_INNER_GITIGNORE_RULES matches between jira-common.sh and migration 0003"
  PASS=$((PASS + 1))
else
  echo "  FAIL: JIRA_INNER_GITIGNORE_RULES mismatch"
  echo "    jira-common.sh: $(echo "$COMMON_RULES" | tr '\n' ' ')"
  echo "    migration 0003: $(echo "$MIGRATION_RULES" | tr '\n' ' ')"
  FAIL=$((FAIL + 1))
fi

echo ""

test_summary
