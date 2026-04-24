#!/usr/bin/env bash
# Verify that all lens boundary_benchmark.json files exist and record 100% pass rates.
# Each boundary eval is a negative-output regression guard — it verifies a lens does NOT
# produce findings in a peer lens's domain when run against that peer's fixtures.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LENSES_DIR="$SCRIPT_DIR/../skills/review/lenses"

LENSES=(clarity completeness dependency scope testability)
FAILURES=0

for lens in "${LENSES[@]}"; do
  benchmark_file="$LENSES_DIR/${lens}-lens/evals/boundary_benchmark.json"

  if [[ ! -f "$benchmark_file" ]]; then
    echo "FAIL: $lens boundary_benchmark.json not found at $benchmark_file"
    FAILURES=$((FAILURES + 1))
    continue
  fi

  # Count expectations and failed expectations using Python.
  result=$(python3 - "$benchmark_file" <<'PYEOF'
import json, sys

with open(sys.argv[1]) as f:
    data = json.load(f)

total = 0
failed = 0
for run in data.get("runs", []):
    for exp in run.get("expectations", []):
        total += 1
        if not exp.get("passed", False):
            failed += 1
            print(f"  FAIL expectation in eval '{run.get('eval_name', '?')}': {exp.get('text', '?')}")

print(f"SUMMARY:{total}:{failed}")
PYEOF
)

  summary_line=$(echo "$result" | grep '^SUMMARY:')
  total=$(echo "$summary_line" | cut -d: -f2)
  failed=$(echo "$summary_line" | cut -d: -f3)

  if [[ "$failed" -gt 0 ]]; then
    echo "FAIL: $lens boundary evals — $failed/$total expectations failed"
    echo "$result" | grep -v '^SUMMARY:'
    FAILURES=$((FAILURES + 1))
  else
    echo "PASS: $lens boundary evals — $total/$total expectations passed"
  fi
done

echo ""
if [[ "$FAILURES" -gt 0 ]]; then
  echo "FAIL: $FAILURES lens(es) have boundary eval failures"
  exit 1
else
  echo "PASS: All lens boundary evals pass"
fi
