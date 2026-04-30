#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-adf-to-md.sh
# Run: bash skills/integrations/jira/scripts/test-jira-adf-to-md.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

RENDERER="$SCRIPT_DIR/jira-adf-to-md.sh"
FIXTURES="$SCRIPT_DIR/test-fixtures/adf-samples"

# ============================================================
echo "=== Fixture-pair sweep ==="
echo ""

for adf_file in "$FIXTURES"/*.adf.json; do
  name=$(basename "$adf_file" .adf.json)
  md_file="$FIXTURES/$name.md"
  [[ -f "$md_file" ]] || continue  # skip rendering-only fixtures
  actual=$(bash "$RENDERER" < "$adf_file")
  expected=$(cat "$md_file")
  assert_eq "render $name" "$expected" "$actual"
done

echo ""

# ============================================================
echo "=== Placeholder rendering ==="
echo ""

echo "Test: unsupported panel node emits placeholder"
OUT=$(bash "$RENDERER" < "$FIXTURES/unsupported-panel.adf.json")
assert_contains "panel placeholder" "[unsupported ADF node: panel]" "$OUT"

echo "Test: unsupported mention inline emits placeholder"
OUT=$(bash "$RENDERER" < "$FIXTURES/unsupported-mention.adf.json")
assert_contains "mention placeholder" "[unsupported ADF inline: mention]" "$OUT"

echo ""

# ============================================================
echo "=== Error handling ==="
echo ""

echo "Test: non-JSON input exits E_BAD_JSON"
ERR=$(bash "$RENDERER" <<< "not json" 2>&1 >/dev/null || true)
assert_contains "names error code" "E_BAD_JSON" "$ERR"
assert_exit_code "exits 40" 40 bash "$RENDERER" <<< "not json"

echo "Test: valid JSON but not a doc exits E_BAD_JSON"
ERR=$(bash "$RENDERER" <<< '{"type":"paragraph"}' 2>&1 >/dev/null || true)
assert_contains "names error code" "E_BAD_JSON" "$ERR"
assert_exit_code "exits 40" 40 bash "$RENDERER" <<< '{"type":"paragraph"}'

echo ""

# ============================================================
test_summary
