#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-auth.sh / jira-auth-cli.sh
# Run: bash skills/integrations/jira/scripts/test-jira-auth.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

assert_contains() {
  local test_name="$1" needle="$2" haystack="$3"
  if printf '%s' "$haystack" | grep -qF "$needle"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected to contain: $(printf '%q' "$needle")"
    echo "    Actual:              $(printf '%q' "$haystack")"
    FAIL=$((FAIL + 1))
  fi
}

assert_not_contains() {
  local test_name="$1" needle="$2" haystack="$3"
  if ! printf '%s' "$haystack" | grep -qF "$needle"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected NOT to contain: $(printf '%q' "$needle")"
    echo "    Actual:                  $(printf '%q' "$haystack")"
    FAIL=$((FAIL + 1))
  fi
}

AUTH_CLI="$SCRIPT_DIR/jira-auth-cli.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# Create a minimal fake-VCS repo fixture (no real git/jj needed for most tests)
setup_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.claude"
  echo "$d"
}

# Create a real git repo (required for VCS-tracking tests)
setup_git_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/git-XXXXXX")
  mkdir -p "$d/.claude"
  (cd "$d" && git init -q && git config user.email "t@t.com" && git config user.name "T")
  echo "$d"
}

write_team_config() {
  local repo="$1" site="${2:-https://myorg.atlassian.net}" email="${3:-user@example.com}"
  local extra_field="${4:-}"
  {
    echo "---"
    echo "jira:"
    echo "  site: $site"
    echo "  email: $email"
    [ -n "$extra_field" ] && printf '%s\n' "$extra_field"
    echo "---"
  } > "$repo/.claude/accelerator.md"
}

# extra_line should be indented (e.g. "  token: tok")
write_local_config() {
  local repo="$1" extra_line="${2:-}"
  {
    echo "---"
    if [ -n "$extra_line" ]; then
      echo "jira:"
      printf '%s\n' "$extra_line"
    fi
    echo "---"
  } > "$repo/.claude/accelerator.local.md"
  chmod 600 "$repo/.claude/accelerator.local.md"
}

# ---------------------------------------------------------------------------
echo "=== resolution chain — token source precedence ==="
echo ""

echo "Test 1: ACCELERATOR_JIRA_TOKEN env var wins over all config sources"
REPO=$(setup_repo)
write_team_config "$REPO" "https://team.atlassian.net" "team@example.com" "  token: team-token"
OUTPUT=$(cd "$REPO" && ACCELERATOR_JIRA_TOKEN=env-token bash "$AUTH_CLI")
assert_contains "token=env-token in stdout" "token=env-token" "$OUTPUT"

echo "Test 2: ACCELERATOR_JIRA_TOKEN_CMD resolves with trailing whitespace trimmed"
REPO=$(setup_repo)
write_team_config "$REPO"
# printf outputs tok-cmd + CR + LF + two more LF + trailing spaces
OUTPUT=$(cd "$REPO" && ACCELERATOR_JIRA_TOKEN_CMD='printf "tok-cmd\r\n\n\n  "' bash "$AUTH_CLI")
assert_eq "token=tok-cmd (stripped)" "token=tok-cmd" "$(printf '%s\n' "$OUTPUT" | grep '^token=')"

echo "Test 3: accelerator.local.md jira.token wins over accelerator.md jira.token"
REPO=$(setup_repo)
write_team_config "$REPO" "https://team.atlassian.net" "team@example.com" "  token: team-token"
write_local_config "$REPO" "  token: local-token"
OUTPUT=$(cd "$REPO" && bash "$AUTH_CLI")
assert_contains "local token wins" "token=local-token" "$OUTPUT"

echo "Test 4: accelerator.local.md jira.token_cmd wins over accelerator.md jira.token"
REPO=$(setup_repo)
write_team_config "$REPO" "https://team.atlassian.net" "team@example.com" "  token: team-token"
write_local_config "$REPO" "  token_cmd: echo cmd-token"
OUTPUT=$(cd "$REPO" && bash "$AUTH_CLI")
assert_contains "local token_cmd wins" "token=cmd-token" "$OUTPUT"

echo "Test 5: accelerator.md jira.token used when nothing else is set"
REPO=$(setup_repo)
write_team_config "$REPO" "https://team.atlassian.net" "team@example.com" "  token: shared-token"
OUTPUT=$(cd "$REPO" && bash "$AUTH_CLI")
assert_contains "shared token" "token=shared-token" "$OUTPUT"

echo "Test 6: accelerator.md jira.token_cmd is ignored — E_TOKEN_CMD_FROM_SHARED_CONFIG + E_NO_TOKEN"
REPO=$(setup_repo)
write_team_config "$REPO" "https://team.atlassian.net" "team@example.com" "  token_cmd: echo ignored"
EXIT_CODE=0
STDERR=$(cd "$REPO" && bash "$AUTH_CLI" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_contains "E_TOKEN_CMD_FROM_SHARED_CONFIG in stderr" "E_TOKEN_CMD_FROM_SHARED_CONFIG" "$STDERR"
assert_contains "E_NO_TOKEN in stderr" "E_NO_TOKEN" "$STDERR"
if [ "$EXIT_CODE" -ne 0 ]; then
  echo "  PASS: exits non-zero"
  PASS=$((PASS + 1))
else
  echo "  FAIL: expected non-zero exit, got 0"
  FAIL=$((FAIL + 1))
fi

echo "Test 7: empty resolution exits non-zero with E_NO_TOKEN, nothing on stdout"
REPO=$(setup_repo)
write_team_config "$REPO"  # no token fields
EXIT_CODE=0
STDOUT=$(cd "$REPO" && bash "$AUTH_CLI" 2>/dev/null) || EXIT_CODE=$?
STDERR=$(cd "$REPO" && bash "$AUTH_CLI" 2>&1 1>/dev/null) || true
assert_contains "E_NO_TOKEN in stderr" "E_NO_TOKEN" "$STDERR"
if [ "$EXIT_CODE" -ne 0 ]; then
  echo "  PASS: exits non-zero"
  PASS=$((PASS + 1))
else
  echo "  FAIL: expected non-zero exit, got 0"
  FAIL=$((FAIL + 1))
fi
assert_eq "stdout is empty" "" "$STDOUT"

echo "Test 8: stdout has exactly three lines — site=, email=, token= — no interleaved logging"
REPO=$(setup_repo)
write_team_config "$REPO" "https://team.atlassian.net" "user@example.com" "  token: tok"
OUTPUT=$(cd "$REPO" && bash "$AUTH_CLI")
LINE_COUNT=$(printf '%s\n' "$OUTPUT" | wc -l | tr -d ' ')
assert_eq "exactly 3 lines" "3" "$LINE_COUNT"
assert_contains "site= line present" "site=" "$OUTPUT"
assert_contains "email= line present" "email=" "$OUTPUT"
assert_contains "token= line present" "token=" "$OUTPUT"

echo "Test 9: token redaction — sentinel not present in --debug stderr"
REPO=$(setup_repo)
write_team_config "$REPO" "https://team.atlassian.net" "user@example.com" "  token: tok-SENTINEL-xyz123"
DEBUG_STDERR=$(cd "$REPO" && bash "$AUTH_CLI" --debug 2>&1 1>/dev/null || true)
assert_not_contains "sentinel absent from debug stderr" "tok-SENTINEL-xyz123" "$DEBUG_STDERR"
# Confirm sentinel IS on stdout
DEBUG_STDOUT=$(cd "$REPO" && bash "$AUTH_CLI" --debug 2>/dev/null)
assert_contains "sentinel on stdout token= line" "token=tok-SENTINEL-xyz123" "$DEBUG_STDOUT"

echo "Test 10: token_cmd failure exits with E_TOKEN_CMD_FAILED on stderr"
REPO=$(setup_repo)
write_team_config "$REPO"
write_local_config "$REPO" "  token_cmd: exit 1"
EXIT_CODE=0
STDERR=$(cd "$REPO" && bash "$AUTH_CLI" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_contains "E_TOKEN_CMD_FAILED in stderr" "E_TOKEN_CMD_FAILED" "$STDERR"
if [ "$EXIT_CODE" -ne 0 ]; then
  echo "  PASS: exits non-zero"
  PASS=$((PASS + 1))
else
  echo "  FAIL: expected non-zero exit"
  FAIL=$((FAIL + 1))
fi

echo "Test 11: site/email use only file precedence — ACCELERATOR_JIRA_SITE is not consulted"
REPO=$(setup_repo)
write_team_config "$REPO" "https://team.atlassian.net" "user@example.com" "  token: tok"
OUTPUT=$(cd "$REPO" && ACCELERATOR_JIRA_SITE=https://injected.atlassian.net bash "$AUTH_CLI")
assert_contains "site from config, not env" "site=https://team.atlassian.net" "$OUTPUT"
assert_not_contains "injected site absent" "injected.atlassian.net" "$OUTPUT"

echo "Test 12: token_cmd whitespace handling — CRLF, multiple newlines, spaces all stripped"
REPO=$(setup_repo)
write_team_config "$REPO"
write_local_config "$REPO" '  token_cmd: printf "tok\r\n\n\n  "'
OUTPUT=$(cd "$REPO" && bash "$AUTH_CLI")
assert_eq "token stripped to tok" "token=tok" "$(printf '%s\n' "$OUTPUT" | grep '^token=')"

echo ""

# ---------------------------------------------------------------------------
echo "=== accelerator.local.md permissions — fail-closed ==="
echo ""

echo "Test 13: mode 0644 — exits 29 with E_LOCAL_PERMS_INSECURE, no token on stdout"
REPO=$(setup_repo)
write_team_config "$REPO"
write_local_config "$REPO" "  token: local-token"
chmod 644 "$REPO/.claude/accelerator.local.md"
EXIT_CODE=0
STDERR=$(cd "$REPO" && bash "$AUTH_CLI" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_eq "exit code 29" "29" "$EXIT_CODE"
assert_contains "E_LOCAL_PERMS_INSECURE in stderr" "E_LOCAL_PERMS_INSECURE" "$STDERR"
STDOUT=""
STDOUT=$(cd "$REPO" && bash "$AUTH_CLI" 2>/dev/null) || true
assert_eq "no token leaked on stdout" "" "$STDOUT"

echo ""

# ---------------------------------------------------------------------------
echo "=== ACCELERATOR_ALLOW_INSECURE_LOCAL dual-gate (14a-14f) ==="
echo ""

echo "Test 14a: mode 0644 + env var set + no marker file → exits 29"
REPO=$(setup_git_repo)
write_team_config "$REPO"
write_local_config "$REPO" "  token: tok"
chmod 644 "$REPO/.claude/accelerator.local.md"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_ALLOW_INSECURE_LOCAL=1 bash "$AUTH_CLI" 2>/dev/null 1>/dev/null) || EXIT_CODE=$?
assert_eq "14a: exits 29" "29" "$EXIT_CODE"

echo "Test 14b: mode 0644 + env var set + tracked marker → proceeds with downgrade warning"
REPO=$(setup_git_repo)
write_team_config "$REPO"
write_local_config "$REPO" "  token: tok"
chmod 644 "$REPO/.claude/accelerator.local.md"
touch "$REPO/.claude/insecure-local-ok"
(cd "$REPO" && git add .claude/insecure-local-ok 2>/dev/null)
EXIT_CODE=0
STDERR=$(cd "$REPO" && ACCELERATOR_ALLOW_INSECURE_LOCAL=1 bash "$AUTH_CLI" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_eq "14b: exits 0" "0" "$EXIT_CODE"
assert_contains "14b: downgrade warning on stderr" "Warning:" "$STDERR"

echo "Test 14c: mode 0644 + tracked marker + env var unset → exits 29"
REPO=$(setup_git_repo)
write_team_config "$REPO"
write_local_config "$REPO" "  token: tok"
chmod 644 "$REPO/.claude/accelerator.local.md"
touch "$REPO/.claude/insecure-local-ok"
(cd "$REPO" && git add .claude/insecure-local-ok 2>/dev/null)
EXIT_CODE=0
(cd "$REPO" && bash "$AUTH_CLI" 2>/dev/null 1>/dev/null) || EXIT_CODE=$?
assert_eq "14c: exits 29 without env var" "29" "$EXIT_CODE"

echo "Test 14d: mode 0644 + env var set + untracked marker → exits 29"
REPO=$(setup_git_repo)
write_team_config "$REPO"
write_local_config "$REPO" "  token: tok"
chmod 644 "$REPO/.claude/accelerator.local.md"
touch "$REPO/.claude/insecure-local-ok"  # exists but NOT git-added
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_ALLOW_INSECURE_LOCAL=1 bash "$AUTH_CLI" 2>/dev/null 1>/dev/null) || EXIT_CODE=$?
assert_eq "14d: exits 29 with untracked marker" "29" "$EXIT_CODE"

echo "Test 14e: mode 0644 + env var set + symlink marker → exits 29"
REPO=$(setup_git_repo)
write_team_config "$REPO"
write_local_config "$REPO" "  token: tok"
chmod 644 "$REPO/.claude/accelerator.local.md"
ln -s /dev/null "$REPO/.claude/insecure-local-ok"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_ALLOW_INSECURE_LOCAL=1 bash "$AUTH_CLI" 2>/dev/null 1>/dev/null) || EXIT_CODE=$?
assert_eq "14e: exits 29 with symlink marker" "29" "$EXIT_CODE"

echo "Test 14f: mode 0644 + env var set + directory marker → exits 29"
REPO=$(setup_git_repo)
write_team_config "$REPO"
write_local_config "$REPO" "  token: tok"
chmod 644 "$REPO/.claude/accelerator.local.md"
mkdir -p "$REPO/.claude/insecure-local-ok"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_ALLOW_INSECURE_LOCAL=1 bash "$AUTH_CLI" 2>/dev/null 1>/dev/null) || EXIT_CODE=$?
assert_eq "14f: exits 29 with directory marker" "29" "$EXIT_CODE"

echo ""
test_summary
