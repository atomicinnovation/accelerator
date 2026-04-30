#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-md-to-adf.sh (Markdown → ADF compiler)
# Run: bash skills/integrations/jira/scripts/test-jira-md-to-adf.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

COMPILER="$SCRIPT_DIR/jira-md-to-adf.sh"
FIXTURES="$SCRIPT_DIR/test-fixtures/adf-samples"

compile() {
  JIRA_ADF_LOCALID_SEED=1 bash "$COMPILER"
}

# ============================================================
echo "=== Fixture-pair sweep ==="
echo ""

for md_file in "$FIXTURES"/*.md; do
  name=$(basename "$md_file" .md)
  adf_file="$FIXTURES/$name.adf.json"
  [[ -f "$adf_file" ]] || continue  # skip compile-only or rejection-only fixtures
  [[ "$name" == reject-* ]] && continue
  actual=$(compile < "$md_file" | jq -S .)
  expected=$(jq -S . "$adf_file")
  assert_eq "compile $name" "$expected" "$actual"
done

echo ""

# ============================================================
echo "=== CRLF normalisation ==="
echo ""

echo "Test: CRLF input compiles same as LF input"
LF_ADF=$(printf 'This is a simple paragraph.\n' | compile | jq -S .)
CRLF_ADF=$(compile < "$FIXTURES/crlf-input.md" | jq -S .)
assert_eq "crlf normalises to lf-equivalent" "$LF_ADF" "$CRLF_ADF"

echo ""

# ============================================================
echo "=== Rejection cases ==="
echo ""

echo "Test: reject-table exits 41 with E_ADF_UNSUPPORTED_TABLE"
ERR=$(bash "$COMPILER" < "$FIXTURES/reject-table.md" 2>&1 >/dev/null || true)
assert_contains "table error code on stderr" "E_ADF_UNSUPPORTED_TABLE" "$ERR"
assert_exit_code "table exits 41" 41 bash "$COMPILER" < "$FIXTURES/reject-table.md"

echo "Test: reject-nested-list exits 41 with E_ADF_UNSUPPORTED_NESTED_LIST"
ERR=$(bash "$COMPILER" < "$FIXTURES/reject-nested-list.md" 2>&1 >/dev/null || true)
assert_contains "nested list error code on stderr" "E_ADF_UNSUPPORTED_NESTED_LIST" "$ERR"
assert_exit_code "nested list exits 41" 41 bash "$COMPILER" < "$FIXTURES/reject-nested-list.md"

echo "Test: reject-blockquote exits 41 with E_ADF_UNSUPPORTED_BLOCKQUOTE"
ERR=$(bash "$COMPILER" < "$FIXTURES/reject-blockquote.md" 2>&1 >/dev/null || true)
assert_contains "blockquote error code on stderr" "E_ADF_UNSUPPORTED_BLOCKQUOTE" "$ERR"
assert_exit_code "blockquote exits 41" 41 bash "$COMPILER" < "$FIXTURES/reject-blockquote.md"

echo "Test: reject-control-chars exits 42 with E_ADF_BAD_INPUT"
ERR=$(bash "$COMPILER" < "$FIXTURES/reject-control-chars.md" 2>&1 >/dev/null || true)
assert_contains "control chars error code on stderr" "E_ADF_BAD_INPUT" "$ERR"
assert_exit_code "control chars exits 42" 42 bash "$COMPILER" < "$FIXTURES/reject-control-chars.md"

echo ""

# ============================================================
echo "=== Underscore warning ==="
echo ""

echo "Test: underscore-warning compiles successfully with notice on stderr"
WARN=$(bash "$COMPILER" < "$FIXTURES/underscore-warning.md" 2>&1 >/dev/null || true)
assert_contains "underscore notice fired" "Notice:" "$WARN"
assert_exit_code "underscore warning exits 0" 0 bash "$COMPILER" < "$FIXTURES/underscore-warning.md"

echo ""

# ============================================================
echo "=== JQ injection resistance ==="
echo ""

echo "Test: jq-injection compiles to single paragraph with literal text"
RESULT=$(bash "$COMPILER" < "$FIXTURES/reject-jq-injection.md")
NODE_TYPE=$(printf '%s' "$RESULT" | jq -r '.content[0].type')
NODE_COUNT=$(printf '%s' "$RESULT" | jq '.content | length')
assert_eq "single paragraph node" "paragraph" "$NODE_TYPE"
assert_eq "only one top-level node" "1" "$NODE_COUNT"

echo ""

# ============================================================
echo "=== Placeholder collision ==="
echo ""

echo "Test: literal placeholder text round-trips without triggering placeholder convention"
RESULT=$(bash "$COMPILER" < "$FIXTURES/placeholder-collision.md")
# Should have paragraph content, not trigger compiler errors
assert_exit_code "placeholder collision exits 0" 0 bash "$COMPILER" < "$FIXTURES/placeholder-collision.md"

echo ""

# ============================================================
test_summary
