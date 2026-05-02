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
echo "=== Link scheme allowlist ==="
echo ""

# Helper: render an ADF doc containing a single link node and return stdout
render_link() {
  local href="$1" text="${2:-click}"
  jq -cn --arg href "$href" --arg text "$text" '{
    "type":"doc","version":1,
    "content":[{"type":"paragraph","content":[
      {"type":"text","text":$text,"marks":[{"type":"link","attrs":{"href":$href}}]}
    ]}]
  }' | bash "$RENDERER"
}

echo "Test: http scheme passes through"
OUT=$(render_link "http://example.com")
assert_contains "http link preserved" "[click](http://example.com)" "$OUT"

echo "Test: https scheme passes through"
OUT=$(render_link "https://example.com/path?q=1")
assert_contains "https link preserved" "[click](https://example.com/path?q=1)" "$OUT"

echo "Test: mailto scheme passes through"
OUT=$(render_link "mailto:foo@bar.com" "email")
assert_contains "mailto link preserved" "[email](mailto:foo@bar.com)" "$OUT"

echo "Test: javascript: scheme stripped to plain text"
OUT=$(render_link "javascript:alert(1)")
assert_not_contains "javascript href absent" "javascript:" "$OUT"
assert_contains "display text preserved" "click" "$OUT"
assert_not_contains "no link syntax" "](" "$OUT"

echo "Test: data: scheme stripped to plain text"
OUT=$(render_link "data:text/html,<script>x</script>" "image")
assert_not_contains "data href absent" "data:" "$OUT"
assert_contains "display text preserved" "image" "$OUT"

echo "Test: vbscript: scheme stripped to plain text"
OUT=$(render_link "vbscript:msgbox(1)")
assert_not_contains "vbscript href absent" "vbscript:" "$OUT"
assert_contains "display text preserved" "click" "$OUT"

echo "Test: case-insensitive — JavaScript: stripped"
OUT=$(render_link "JavaScript:alert(1)")
assert_not_contains "JavaScript href absent" "JavaScript:" "$OUT"
assert_contains "display text preserved" "click" "$OUT"

echo "Test: case-insensitive — JAVASCRIPT: stripped"
OUT=$(render_link "JAVASCRIPT:alert(1)")
assert_not_contains "JAVASCRIPT href absent" "JAVASCRIPT:" "$OUT"
assert_contains "display text preserved" "click" "$OUT"

echo "Test: schemeless absolute path passes through"
OUT=$(render_link "/path/to/page")
assert_contains "absolute path link preserved" "[click](/path/to/page)" "$OUT"

echo "Test: fragment-only link passes through"
OUT=$(render_link "#section")
assert_contains "fragment link preserved" "[click](#section)" "$OUT"

echo "Test: relative URL passes through"
OUT=$(render_link "page.html")
assert_contains "relative URL link preserved" "[click](page.html)" "$OUT"

echo "Test: whitespace-leading javascript: scheme stripped"
OUT=$(render_link "  javascript:foo")
assert_not_contains "whitespace js href absent" "javascript:" "$OUT"
assert_contains "display text preserved" "click" "$OUT"

echo ""

# ============================================================
test_summary
