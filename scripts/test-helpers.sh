#!/usr/bin/env bash
# Shared bash test-harness helpers. Source from test-*.sh scripts:
#
#   source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/..(etc)../scripts/test-helpers.sh"
#
# Exposes: PASS/FAIL counters and assert_eq, assert_contains,
# assert_not_contains, assert_file_exists, assert_file_not_exists,
# assert_not_exists, assert_empty, assert_exit_code, assert_file_content_eq,
# assert_file_executable, assert_dir_absent, assert_stderr_empty,
# assert_stderr_contains, assert_json_eq, test_summary.

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

assert_contains() {
  local test_name="$1" haystack="$2" needle="$3"
  if echo "$haystack" | grep -qF "$needle"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected to contain: $(printf '%q' "$needle")"
    echo "    Actual: $(printf '%q' "$haystack")"
    FAIL=$((FAIL + 1))
  fi
}

assert_not_contains() {
  local test_name="$1" haystack="$2" needle="$3"
  if echo "$haystack" | grep -qF "$needle"; then
    echo "  FAIL: $test_name"
    echo "    Expected NOT to contain: $(printf '%q' "$needle")"
    echo "    Actual: $(printf '%q' "$haystack")"
    FAIL=$((FAIL + 1))
  else
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  fi
}

assert_file_exists() {
  local test_name="$1" file_path="$2"
  if [ -f "$file_path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected file to exist: $file_path"
    FAIL=$((FAIL + 1))
  fi
}

assert_file_not_exists() {
  local test_name="$1" file_path="$2"
  if [ ! -f "$file_path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected file to not exist: $file_path"
    FAIL=$((FAIL + 1))
  fi
}

assert_not_exists() {
  local test_name="$1" path="$2"
  if [ ! -e "$path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name ($path should not exist)"
    FAIL=$((FAIL + 1))
  fi
}

assert_empty() {
  local test_name="$1" actual="$2"
  if [ -z "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected empty, got: $actual"
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

assert_file_content_eq() {
  local test_name="$1" file_path="$2" expected="$3"
  local actual
  actual=$(cat "$file_path" 2>/dev/null) || {
    echo "  FAIL: $test_name"
    echo "    File not found: $file_path"
    FAIL=$((FAIL + 1))
    return
  }
  if [ "$expected" = "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected content: $(printf '%q' "$expected")"
    echo "    Actual content:   $(printf '%q' "$actual")"
    FAIL=$((FAIL + 1))
  fi
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

assert_dir_absent() {
  local test_name="$1" path="$2"
  if [ ! -d "$path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name — directory exists: $path"
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

assert_stderr_contains() {
  local test_name="$1" substr="$2"
  shift 2
  local stderr
  stderr=$("$@" 2>&1 >/dev/null) || true
  if echo "$stderr" | grep -qF "$substr"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected stderr to contain: $(printf '%q' "$substr")"
    echo "    Actual stderr: $stderr"
    FAIL=$((FAIL + 1))
  fi
}

assert_json_eq() {
  local test_name="$1" jq_filter="$2" expected="$3" json_path="$4"
  local actual
  actual=$(jq -r "$jq_filter" "$json_path" 2>/dev/null || echo '__jq_error__')
  if [ "$expected" = "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Filter:   $jq_filter"
    echo "    Expected: $(printf '%q' "$expected")"
    echo "    Actual:   $(printf '%q' "$actual")"
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
