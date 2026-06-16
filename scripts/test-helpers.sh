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

# Directory holding this library and the config-read-*.sh scripts it launches
# in bash mode. Computed from BASH_SOURCE so run_sut resolves the scripts
# regardless of the sourcing suite's own SCRIPT_DIR.
_TEST_HELPERS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# --- System-under-test (SUT) switch: bash scripts vs. the a9r binary --------
#
# The config-read suites must run unchanged against either the original bash
# scripts (default) or the ported `a9r` binary (when A9R_BIN names it), proving
# byte-for-byte parity. Call sites invoke `run_sut <key> args…` where <key> is
# the abstract command (read-path / read-value); run_sut dispatches to bash or
# a9r. It is a plain function (NEVER exec) so it composes both under $(…)
# capture and as the trailing argument of assert_exit_code / assert_stderr_*.

# Initialise SUT mode. Pass the a9r-mode executed-assertion floor as $1.
# Fails the suite loudly if A9R_BIN is set but not an executable file, so an
# a9r-mode run can never silently degrade to testing bash twice.
sut_mode_init() {
  A9R_RUN_SUT_FLOOR="${1:-0}"
  if [ -n "${A9R_BIN:-}" ]; then
    if [ ! -x "$A9R_BIN" ]; then
      echo "run_sut: A9R_BIN is set but not an executable file: $A9R_BIN" >&2
      exit 1
    fi
    # Subshell-safe executed-assertion counter: call sites run run_sut inside
    # $(…) command substitutions, so an in-memory counter would never reach the
    # parent. Each a9r invocation appends one byte to this file; test_summary
    # counts its bytes in the parent shell.
    A9R_RUN_SUT_COUNT_FILE="$(mktemp)"
  fi
}

# Dispatch a config-read invocation to bash or a9r. Returns the SUT exit code.
run_sut() {
  local key="$1"
  shift
  local script subcommand
  case "$key" in
    read-path)
      script="config-read-path.sh"
      subcommand="config-read-path"
      ;;
    read-value)
      script="config-read-value.sh"
      subcommand="config-read-value"
      ;;
    read-context)
      script="config-read-context.sh"
      subcommand="config-read-context"
      ;;
    read-agents)
      script="config-read-agents.sh"
      subcommand="config-read-agents"
      ;;
    read-skill-context)
      script="config-read-skill-context.sh"
      subcommand="config-read-skill-context"
      ;;
    read-skill-instructions)
      script="config-read-skill-instructions.sh"
      subcommand="config-read-skill-instructions"
      ;;
    *)
      echo "run_sut: unknown subcommand key: $key" >&2
      return 2
      ;;
  esac
  if [ -n "${A9R_BIN:-}" ]; then
    printf '.' >>"$A9R_RUN_SUT_COUNT_FILE"
    "$A9R_BIN" "$subcommand" "$@"
  else
    bash "$_TEST_HELPERS_DIR/$script" "$@"
  fi
}

# Guard a block that sources the bash library and exercises an internal
# function with no a9r-binary analogue. In a9r mode it logs one accounted SKIP
# and returns 0 (true → caller skips the block); in bash mode returns 1 (false
# → caller runs it). Usage: `if ! skip_unless_bash_mode "reason"; then … fi`.
skip_unless_bash_mode() {
  local reason="$1"
  if [ -n "${A9R_BIN:-}" ]; then
    printf '  SKIP: %s\n' "$reason"
    SKIP=$((SKIP + 1))
    return 0
  fi
  return 1
}

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

  # SUT-mode banner + executed-assertion floor. The floor fails the suite
  # directly (not only via task wiring) if too few assertions actually reached
  # the a9r path — so guarding everything out of the a9r branch is caught here.
  if [ -n "${A9R_BIN:-}" ]; then
    local sut_count=0
    if [ -n "${A9R_RUN_SUT_COUNT_FILE:-}" ] && [ -f "$A9R_RUN_SUT_COUNT_FILE" ]; then
      sut_count=$(wc -c <"$A9R_RUN_SUT_COUNT_FILE" | tr -d ' ')
    fi
    echo "SUT MODE: a9r=$A9R_BIN (executed $sut_count a9r assertions)"
    if [ "$sut_count" -lt "${A9R_RUN_SUT_FLOOR:-0}" ]; then
      echo "FAIL: a9r-mode executed only $sut_count assertions, below the" \
        "floor of ${A9R_RUN_SUT_FLOOR:-0} — config-read sites were guarded" \
        "out of the a9r path." >&2
      return 1
    fi
  else
    echo "SUT MODE: bash"
  fi

  if [ "$FAIL" -gt 0 ]; then
    return 1
  fi
  echo "All tests passed!"
}
