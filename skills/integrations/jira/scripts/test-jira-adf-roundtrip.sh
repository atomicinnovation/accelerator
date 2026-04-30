#!/usr/bin/env bash
set -euo pipefail

# Round-trip property tests for the ADF Markdown ↔ ADF compiler/renderer pair.
# Run: bash skills/integrations/jira/scripts/test-jira-adf-roundtrip.sh
#
# Three invariants per supported fixture:
#   1. render(compile(md)) == canonicalise(md)  (Markdown round-trip)
#   2. compile(render(compile(md))) == compile(md)  (ADF fixed-point, modulo localId)
#   3. Marker token counts preserved across round-trip

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

COMPILER="$SCRIPT_DIR/jira-md-to-adf.sh"
RENDERER="$SCRIPT_DIR/jira-adf-to-md.sh"
FIXTURES="$SCRIPT_DIR/test-fixtures/adf-samples"

# Canonicalise: normalise CRLF to LF and trailing whitespace (preserve hard-break markers)
canonicalise() {
  awk '{
    sub(/\r$/, "")
    if ($0 ~ /  +$/) {
      sub(/[[:space:]]+$/, "")
      printf "%s  \n", $0
    } else {
      sub(/[[:space:]]+$/, "")
      print
    }
  }'
}

# Mask non-deterministic localId values for ADF fixed-point comparison
mask_local_ids() {
  jq 'walk(if type == "object" and has("localId") then .localId = "<masked>" else . end)'
}

# ============================================================
echo "=== Invariant 1: Markdown round-trip render(compile(md)) == canonicalise(md) ==="
echo ""

for md_file in "$FIXTURES"/*.md; do
  name=$(basename "$md_file" .md)
  [[ "$name" == reject-* ]] && continue
  [[ "$name" == crlf-input ]] && continue  # CRLF-specific, not a round-trip fixture

  adf_file="$FIXTURES/$name.adf.json"
  [[ -f "$adf_file" ]] || continue  # skip compile-only fixtures

  expected=$(canonicalise < "$md_file")
  actual=$(bash "$COMPILER" < "$md_file" 2>/dev/null | bash "$RENDERER")
  assert_eq "roundtrip $name" "$expected" "$actual"
done

echo ""

# ============================================================
echo "=== Invariant 2: ADF fixed-point compile(render(compile(md))) == compile(md) ==="
echo ""

for md_file in "$FIXTURES"/*.md; do
  name=$(basename "$md_file" .md)
  [[ "$name" == reject-* ]] && continue
  [[ "$name" == crlf-input ]] && continue

  adf_file="$FIXTURES/$name.adf.json"
  [[ -f "$adf_file" ]] || continue

  out1=$(bash "$COMPILER" < "$md_file" 2>/dev/null | jq -S . | mask_local_ids)
  out2=$(bash "$COMPILER" < "$md_file" 2>/dev/null | bash "$RENDERER" | bash "$COMPILER" 2>/dev/null | jq -S . | mask_local_ids)
  assert_eq "fixed-point $name" "$out1" "$out2"
done

echo ""

# ============================================================
echo "=== Invariant 3: Marker token counts preserved (mixed-everything fixture) ==="
echo ""

markers=(
  URL_M00001 CODE_M00002 BOLD_M00003 ITALIC_M00004 CODE5_M00005
  LIST_M00006 ORD_M00007 TASK_M00008 HEAD_M00009 PARA_M00010
  HBR_M00011 LNKTEXT_M00012
)

md_file="$FIXTURES/mixed-everything.md"
rendered=$(bash "$COMPILER" < "$md_file" 2>/dev/null | bash "$RENDERER")

for marker in "${markers[@]}"; do
  expected_count=$(grep -c "$marker" "$md_file" || true)
  actual_count=$(printf '%s\n' "$rendered" | grep -c "$marker" || true)
  assert_eq "marker $marker" "$expected_count" "$actual_count"
done

echo ""

# ============================================================
test_summary
