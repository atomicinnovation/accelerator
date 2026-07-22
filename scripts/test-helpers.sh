#!/usr/bin/env bash
# Shared bash test-harness helpers. Source from test-*.sh scripts:
#
#   source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/..(etc)../scripts/test-helpers.sh"
#
# Exposes: PASS/FAIL counters and the following assert helpers:
#   assert_eq, assert_neq, assert_empty
#   assert_contains, assert_not_contains
#   assert_matches_regex, assert_not_matches_regex
#   assert_file_exists, assert_not_exists
#   assert_file_not_exists, assert_file_content_eq
#   assert_dir_exists, assert_dir_not_exists
#   assert_exit_code, assert_file_executable, assert_stderr_empty
#   test_summary

PASS=0
FAIL=0
SKIP=0

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
  if grep -qF -- "$needle" <<<"$haystack"; then
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
  if grep -qF -- "$needle" <<<"$haystack"; then
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

# shellcheck disable=SC2329 # public assert helper invoked by the sourcing test-*.sh scripts, not within this library
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

assert_neq() {
  local test_name="$1" unexpected="$2" actual="$3"
  if [ "$unexpected" != "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected something other than: $(printf '%q' "$unexpected")"
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
    echo "    Expected empty, got: $(printf '%q' "$actual")"
    FAIL=$((FAIL + 1))
  fi
}

assert_matches_regex() {
  local test_name="$1" regex="$2" subject="$3"
  if grep -qE "$regex" <<<"$subject"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Subject: $(printf '%q' "$subject")"
    echo "    Regex:   $regex"
    FAIL=$((FAIL + 1))
  fi
}

assert_not_matches_regex() {
  local test_name="$1" regex="$2" subject="$3"
  if ! grep -qE "$regex" <<<"$subject"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Subject:                 $(printf '%q' "$subject")"
    echo "    Should not have matched: $regex"
    FAIL=$((FAIL + 1))
  fi
}

assert_file_not_exists() {
  local test_name="$1" file_path="$2"
  if [ ! -f "$file_path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name (expected file to not exist: $file_path)"
    FAIL=$((FAIL + 1))
  fi
}

assert_file_content_eq() {
  local test_name="$1" file_path="$2" expected="$3"
  local actual
  actual=$(cat "$file_path" 2>/dev/null) || {
    echo "  FAIL: $test_name (file not found: $file_path)"
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

assert_dir_exists() {
  local test_name="$1" path="$2"
  if [ -d "$path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name (expected directory: $path)"
    FAIL=$((FAIL + 1))
  fi
}

assert_dir_not_exists() {
  local test_name="$1" path="$2"
  if [ ! -d "$path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name (expected directory to not exist: $path)"
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
  if grep -qF -- "$substr" <<<"$stderr"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected stderr to contain: $(printf '%q' "$substr")"
    echo "    Actual stderr: $stderr"
    FAIL=$((FAIL + 1))
  fi
}

assert_stderr_not_contains() {
  local test_name="$1" substr="$2"
  shift 2
  local stderr
  stderr=$("$@" 2>&1 >/dev/null) || true
  if grep -qF -- "$substr" <<<"$stderr"; then
    echo "  FAIL: $test_name"
    echo "    Unexpected stderr content: $(printf '%q' "$substr")"
    echo "    Actual stderr: $stderr"
    FAIL=$((FAIL + 1))
  else
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
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

# assert_files_identical <label> <expected_file> <actual_file>
#   Byte-exact file comparison via cmp. Unlike assert_eq / assert_file_content_eq
#   (which capture via $(cat ...) and so strip trailing newlines), this proves
#   two files are byte-identical — including the terminal newline.
assert_files_identical() {
  local test_name="$1" expected_file="$2" actual_file="$3"
  if cmp -s "$expected_file" "$actual_file"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name: files differ"
    diff "$expected_file" "$actual_file" >&2 || true
    FAIL=$((FAIL + 1))
  fi
}

# assert_stdout_exact <label> <expected_file> <captured_stdout_file>
#   Thin alias around assert_files_identical for byte-exact stdout assertions
#   (the AC1 / segmentation --list cases). Capture stdout to a file, then cmp.
assert_stdout_exact() {
  assert_files_identical "$1" "$2" "$3"
}

skip_test() {
  local test_name="$1"
  local reason="$2"
  printf '  SKIP: %s (%s)\n' "$test_name" "$reason"
  SKIP=$((SKIP + 1))
}

assert_grep_empty() {
  local test_name="$1"
  local path="$2"
  local pattern="$3"
  shift 3
  if [ ! -e "$path" ]; then
    printf '  FAIL: %s — path does not exist: %q\n' "$test_name" "$path"
    FAIL=$((FAIL + 1))
    return 1
  fi
  local matches rc
  matches=$(grep -rn "$@" "$pattern" "$path" 2>&1)
  rc=$?
  case "$rc" in
    0)
      printf '  FAIL: %s — found unexpected matches for %q under %q\n' \
        "$test_name" "$pattern" "$path"
      printf '%s\n' "$matches" | sed 's/^/    /'
      FAIL=$((FAIL + 1))
      return 1
      ;;
    1)
      printf '  PASS: %s — no matches for %q under %q\n' \
        "$test_name" "$pattern" "$path"
      PASS=$((PASS + 1))
      ;;
    *)
      printf '  FAIL: %s — grep error (exit %d) searching %q under %q:\n' \
        "$test_name" "$rc" "$pattern" "$path"
      printf '%s\n' "$matches" | sed 's/^/    /'
      FAIL=$((FAIL + 1))
      return 1
      ;;
  esac
}

test_summary() {
  echo ""
  echo "=== Results ==="
  echo "Passed: $PASS"
  echo "Skipped: $SKIP"
  echo "Failed: $FAIL"
  if [ "$FAIL" -gt 0 ]; then
    return 1
  fi
  echo "All tests passed!"
}
