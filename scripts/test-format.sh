#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

PASS=0
FAIL=0

pass() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL: $1"; echo "        $2"; FAIL=$((FAIL + 1)); }

echo "=== Format checks ==="

# Hyphenation guard: 'work item' (space) must not appear as part of a
# compound identifier. Check specific identifier-context patterns:
#
#   1. work item- : compound word with hyphen (should be work-item-)
#      e.g. `work item-template-field-hints.sh` is wrong
#   2. work items/ : plural as path component (should be work-items/ or work/)
#      e.g. meta/reviews/work items/ or paths.work items
#
# This check is intentionally targeted, not exhaustive. Run the broader
# `rg '\bwork item\b' skills/` sweep manually for prose audits.

PATTERN1='work item-[a-z]'  # compound-word hyphen: work item-foo
PATTERN2='work items/'      # plural as path component
PATTERN3='paths\.work items' # config key with wrong plural

HITS1=$(rg -l --no-ignore-parent "$PATTERN1" \
  skills/ scripts/ templates/ README.md CHANGELOG.md \
  --iglob '!scripts/test-format.sh' 2>/dev/null || true)
HITS2=$(rg -l --no-ignore-parent "$PATTERN2" \
  skills/ scripts/ templates/ README.md CHANGELOG.md \
  --iglob '!scripts/test-format.sh' 2>/dev/null || true)
HITS3=$(rg -l --no-ignore-parent "$PATTERN3" \
  skills/ scripts/ templates/ README.md CHANGELOG.md \
  --iglob '!scripts/test-format.sh' 2>/dev/null || true)

ALL_HITS=$(printf '%s\n%s\n%s' "$HITS1" "$HITS2" "$HITS3" | grep -v '^$' | sort -u || true)

if [ -z "$ALL_HITS" ]; then
  pass "No 'work item' (space) in identifier/path contexts"
else
  fail "Found 'work item' (space) in identifier/path contexts — use 'work-item'" \
       "$(echo "$ALL_HITS" | tr '\n' ' ')"
  echo "      Matching lines:"
  { rg -n "$PATTERN1" skills/ scripts/ templates/ README.md CHANGELOG.md \
      --iglob '!scripts/test-format.sh' 2>/dev/null || true
    rg -n "$PATTERN2" skills/ scripts/ templates/ README.md CHANGELOG.md \
      --iglob '!scripts/test-format.sh' 2>/dev/null || true
    rg -n "$PATTERN3" skills/ scripts/ templates/ README.md CHANGELOG.md \
      --iglob '!scripts/test-format.sh' 2>/dev/null || true
  } | head -20 | sed 's/^/        /'
fi

echo ""
echo "Format check results: $PASS passed, $FAIL failed"
if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
