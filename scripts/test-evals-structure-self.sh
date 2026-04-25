#!/usr/bin/env bash
# Self-test: verifies that test-evals-structure.sh classifies fixture
# directories correctly, and that test-hierarchy-format.sh classifies
# its fixture pairs correctly.
#
# Exit code: 0 if all fixture expectations pass, 1 if any fail.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

EVALS_SCRIPT="$SCRIPT_DIR/test-evals-structure.sh"
HIERARCHY_SCRIPT="$SCRIPT_DIR/test-hierarchy-format.sh"
EVALS_FIXTURES="$SCRIPT_DIR/test-evals-structure-fixtures"
HIERARCHY_FIXTURES="$SCRIPT_DIR/test-hierarchy-format-fixtures"

echo "=== Self-test: test-evals-structure.sh fixtures ==="
echo ""

assert_exit_code "valid-pair exits 0" 0 \
  bash "$EVALS_SCRIPT" --fixture-root "$EVALS_FIXTURES/valid-pair"

assert_exit_code "missing-benchmark exits non-zero" 1 \
  bash "$EVALS_SCRIPT" --fixture-root "$EVALS_FIXTURES/missing-benchmark"

assert_exit_code "scenario-name-mismatch exits non-zero" 1 \
  bash "$EVALS_SCRIPT" --fixture-root "$EVALS_FIXTURES/scenario-name-mismatch"

assert_exit_code "low-pass-rate exits non-zero" 1 \
  bash "$EVALS_SCRIPT" --fixture-root "$EVALS_FIXTURES/low-pass-rate"

assert_exit_code "malformed-json exits non-zero" 1 \
  bash "$EVALS_SCRIPT" --fixture-root "$EVALS_FIXTURES/malformed-json"

echo ""
echo "=== Self-test: test-hierarchy-format.sh fixtures ==="
echo ""

assert_exit_code "matched-fences exits 0" 0 \
  bash "$HIERARCHY_SCRIPT" \
    "$HIERARCHY_FIXTURES/matched-fences/file-a.md" \
    "$HIERARCHY_FIXTURES/matched-fences/file-b.md"

assert_exit_code "mismatched-fences exits non-zero" 1 \
  bash "$HIERARCHY_SCRIPT" \
    "$HIERARCHY_FIXTURES/mismatched-fences/file-a.md" \
    "$HIERARCHY_FIXTURES/mismatched-fences/file-b.md"

assert_exit_code "missing-marker exits non-zero" 1 \
  bash "$HIERARCHY_SCRIPT" \
    "$HIERARCHY_FIXTURES/missing-marker/file-a.md" \
    "$HIERARCHY_FIXTURES/missing-marker/file-b.md"

echo ""
test_summary
