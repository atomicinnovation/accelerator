#!/usr/bin/env bash
# Shared bash test-harness helpers. Source from test-*.sh scripts:
#
#   source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/..(etc)../scripts/test-helpers.sh"
#
# Exposes: PASS/FAIL counters and assert_eq, assert_exit_code,
# assert_file_executable, assert_stderr_empty, test_summary.

PASS=0
FAIL=0

assert_eq() {
  local test_name="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected: $(printf '%q' "$expected")"
    echo "    Actual:   $(printf '%q' "$actual")"
    FAIL=$((FAIL + 1))
  fi
}

assert_exit_code() {
  local test_name="$1" expected_code="$2"
  shift 2
  local stderr_file
  stderr_file=$(mktemp)
  local actual_code=0
  "$@" >/dev/null 2>"$stderr_file" || actual_code=$?
  if [ "$expected_code" -eq "$actual_code" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected exit code: $expected_code"
    echo "    Actual exit code:   $actual_code"
    if [ -s "$stderr_file" ]; then
      echo "    stderr:"
      sed 's/^/      /' "$stderr_file"
    fi
    FAIL=$((FAIL + 1))
  fi
  rm -f "$stderr_file"
}

assert_file_executable() {
  local test_name="$1" path="$2"
  if [ -x "$path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name ($path not executable)"
    FAIL=$((FAIL + 1))
  fi
}

assert_stderr_empty() {
  local test_name="$1"
  shift
  local stderr
  stderr=$("$@" 2>&1 >/dev/null)
  if [ -z "$stderr" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Unexpected stderr: $stderr"
    FAIL=$((FAIL + 1))
  fi
}

test_summary() {
  echo ""
  echo "=== Results ==="
  echo "Passed: $PASS"
  echo "Failed: $FAIL"
  if [ "$FAIL" -gt 0 ]; then
    return 1
  fi
  echo "All tests passed!"
}
