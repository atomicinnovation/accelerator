#!/usr/bin/env bash
# Check that the canonical tree fence in list-tickets/SKILL.md and
# refine-ticket/SKILL.md are byte-for-byte identical.
#
# Both SKILL.md files bracket the canonical hierarchy example with:
#   <!-- canonical-tree-fence -->
#   ...tree...
#   <!-- /canonical-tree-fence -->
#
# When called without arguments the live SKILL.md files are checked.
# Pass two file paths to check arbitrary files (used by self-tests):
#   test-hierarchy-format.sh <file-a> <file-b>
#
# Exit code: 0 if fences match, 1 on mismatch or missing/empty fence.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/.." && pwd)"

FILE_A="${1:-$REPO/skills/work/list-work-items/SKILL.md}"
FILE_B="${2:-$REPO/skills/work/refine-work-item/SKILL.md}"

extract_fence() {
  local file="$1"
  awk '/<!-- canonical-tree-fence -->/{found=1; next} /<!-- \/canonical-tree-fence -->/{found=0} found{print}' "$file"
}

check_extraction() {
  local file="$1"
  if ! grep -qF '<!-- canonical-tree-fence -->' "$file" 2>/dev/null; then
    echo "FAIL: marker missing or empty extraction — '<!-- canonical-tree-fence -->' not found in $file"
    return 1
  fi
  if ! grep -qF '<!-- /canonical-tree-fence -->' "$file" 2>/dev/null; then
    echo "FAIL: marker missing or empty extraction — '<!-- /canonical-tree-fence -->' not found in $file"
    return 1
  fi
  local content
  content=$(extract_fence "$file")
  if [[ -z "$content" ]]; then
    echo "FAIL: marker missing or empty extraction — fence block is empty in $file"
    return 1
  fi
  return 0
}

ok=true
check_extraction "$FILE_A" || ok=false
check_extraction "$FILE_B" || ok=false

if [[ "$ok" == false ]]; then
  exit 1
fi

FENCE_A=$(extract_fence "$FILE_A")
FENCE_B=$(extract_fence "$FILE_B")

if [[ "$FENCE_A" == "$FENCE_B" ]]; then
  echo "PASS: canonical tree fences match byte-for-byte"
  echo "  $FILE_A"
  echo "  $FILE_B"
  exit 0
else
  echo "FAIL: canonical tree fences differ"
  echo ""
  echo "=== $FILE_A ==="
  echo "$FENCE_A"
  echo ""
  echo "=== $FILE_B ==="
  echo "$FENCE_B"
  echo ""
  echo "=== diff (A vs B) ==="
  diff <(echo "$FENCE_A") <(echo "$FENCE_B") || true
  exit 1
fi
