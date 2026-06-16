#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-body-input.sh
# Run: bash skills/integrations/jira/scripts/test-jira-body-input.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

BODY_INPUT="$SCRIPT_DIR/jira-body-input.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# ---------------------------------------------------------------------------
# Helper: run jira_resolve_body in a subprocess, inheriting caller's stdin.
# Piped stdin is passed through: printf 'text' | resolve_body --allow-stdin
resolve_body() {
  bash -c "source '$BODY_INPUT'; jira_resolve_body \"\$@\"" -- "$@"
}

# Helper: run jira_resolve_body with stdin treated as a terminal (test seam).
# Use for cases that test EDITOR invocation, which requires stdin not to appear piped.
resolve_body_no_stdin() {
  ACCELERATOR_TEST_MODE=1 JIRA_BODY_STDIN_IS_TTY_TEST=1 \
    bash -c "source '$BODY_INPUT'; jira_resolve_body \"\$@\"" -- "$@"
}

# ---------------------------------------------------------------------------

echo "=== Case 1: --body inline wins over --body-file ==="
echo ""

FILE_1="$TMPDIR_BASE/file-1.txt"
printf 'file contents\n' >"$FILE_1"
RESULT_1=$(resolve_body --body "inline value" --body-file "$FILE_1")
assert_eq "inline --body wins over --body-file" "inline value" "$RESULT_1"
echo ""

# ============================================================
echo "=== Case 2: --body-file over piped stdin ==="
echo ""

FILE_2="$TMPDIR_BASE/file-2.txt"
printf 'file body\n' >"$FILE_2"
RESULT_2=$(printf 'piped stdin\n' | resolve_body --body-file "$FILE_2" --allow-stdin)
assert_eq "--body-file wins over piped stdin" "file body" "$RESULT_2"
echo ""

# ============================================================
echo "=== Case 3: stdin when --allow-stdin is set ==="
echo ""

RESULT_3=$(printf 'piped content' | resolve_body --allow-stdin)
assert_eq "stdin read when --allow-stdin" "piped content" "$RESULT_3"
echo ""

# ============================================================
echo "=== Case 4: stdin disallowed — piped but --allow-stdin not set ==="
echo ""

ERR_4=$(printf 'piped' | resolve_body 2>&1 >/dev/null || true)
assert_contains "stdin disallowed: E_BODY_STDIN_DISALLOWED on stderr" "$ERR_4" "E_BODY_STDIN_DISALLOWED"
assert_exit_code "stdin disallowed: exits 3" 3 bash -c \
  "printf 'piped' | bash -c \"source '$BODY_INPUT'; jira_resolve_body\""
echo ""

# ============================================================
echo "=== Case 5: \$EDITOR tempfile ==="
echo ""

EDITOR_STUB_5="$TMPDIR_BASE/editor-5.sh"
cat >"$EDITOR_STUB_5" <<'EOF'
#!/usr/bin/env bash
printf 'edited body\n' >"$1"
EOF
chmod +x "$EDITOR_STUB_5"

RESULT_5=$(EDITOR="$EDITOR_STUB_5" resolve_body_no_stdin --allow-editor)
assert_eq "EDITOR invoked and body captured" "edited body" "$RESULT_5"
echo ""

# ============================================================
echo "=== Case 6: \$EDITOR disallowed — no source, no --allow-editor ==="
echo ""

ERR_6=$(resolve_body_no_stdin 2>&1 >/dev/null || true)
assert_contains "no source: E_BODY_NONE_PROVIDED on stderr" "$ERR_6" "E_BODY_NONE_PROVIDED"
assert_exit_code "no source exits 5" 5 \
  env ACCELERATOR_TEST_MODE=1 JIRA_BODY_STDIN_IS_TTY_TEST=1 \
  bash -c "source '$BODY_INPUT'; jira_resolve_body"
echo ""

# ============================================================
echo "=== Case 7: \$EDITOR exits non-zero ==="
echo ""

ERR_7=$(EDITOR=false resolve_body_no_stdin --allow-editor 2>&1 >/dev/null || true)
assert_contains "editor failed: E_BODY_EDITOR_FAILED on stderr" "$ERR_7" "E_BODY_EDITOR_FAILED"
assert_exit_code "editor failure exits 4" 4 \
  env ACCELERATOR_TEST_MODE=1 JIRA_BODY_STDIN_IS_TTY_TEST=1 EDITOR=false \
  bash -c "source '$BODY_INPUT'; jira_resolve_body --allow-editor"
echo ""

# ============================================================
echo "=== Case 8: Empty body permitted — --body \"\" returns empty stdout, exit 0 ==="
echo ""

RESULT_8=$(resolve_body --body "")
assert_empty "empty --body returns empty stdout" "$RESULT_8"
assert_exit_code "empty --body exits 0" 0 bash -c \
  "source '$BODY_INPUT'; jira_resolve_body --body ''"
echo ""

# ============================================================
echo "=== Case 9: Empty file permitted — --body-file /tmp/empty exits 0 ==="
echo ""

EMPTY_FILE="$TMPDIR_BASE/empty.txt"
: >"$EMPTY_FILE"
RESULT_9=$(resolve_body --body-file "$EMPTY_FILE")
assert_empty "empty file returns empty stdout" "$RESULT_9"
assert_exit_code "empty file exits 0" 0 bash -c \
  "source '$BODY_INPUT'; jira_resolve_body --body-file '$EMPTY_FILE'"
echo ""

# ============================================================
echo "=== Case 10: Missing file — --body-file /nonexistent exits 2 ==="
echo ""

MISSING="/tmp/does-not-exist-$$-body-input-test"
rm -f "$MISSING"
ERR_10=$(resolve_body --body-file "$MISSING" 2>&1 >/dev/null || true)
assert_contains "missing file: E_BODY_FILE_NOT_FOUND on stderr" "$ERR_10" "E_BODY_FILE_NOT_FOUND"
assert_exit_code "missing file exits 2" 2 bash -c \
  "source '$BODY_INPUT'; jira_resolve_body --body-file '$MISSING'"
echo ""

# ============================================================
echo "=== Case 11: Multiple --body flags rejected with E_BODY_BAD_FLAG ==="
echo ""

ERR_11=$(resolve_body --body "first" --body "second" 2>&1 >/dev/null || true)
assert_contains "duplicate --body: E_BODY_BAD_FLAG on stderr" "$ERR_11" "E_BODY_BAD_FLAG"
assert_exit_code "duplicate --body exits 1" 1 bash -c \
  "source '$BODY_INPUT'; jira_resolve_body --body first --body second"
echo ""

# ============================================================
echo "=== Case 12: Stdin TTY detection — no pipe falls through to \$EDITOR ==="
echo ""

EDITOR_STUB_12="$TMPDIR_BASE/editor-12.sh"
cat >"$EDITOR_STUB_12" <<'EOF'
#!/usr/bin/env bash
printf 'tty-fallthrough\n' >"$1"
EOF
chmod +x "$EDITOR_STUB_12"

# Verify that when stdin appears to be a TTY, --allow-stdin does not consume it;
# instead the EDITOR fallback is invoked. Use the JIRA_BODY_STDIN_IS_TTY_TEST seam.
RESULT_12=$(EDITOR="$EDITOR_STUB_12" resolve_body_no_stdin --allow-stdin --allow-editor)
assert_eq "TTY detection: stdin skipped, EDITOR invoked" "tty-fallthrough" "$RESULT_12"

# Also verify that in a piped context, stdin is consumed before EDITOR
RESULT_12_PIPED=$(printf 'piped wins' | resolve_body --allow-stdin --allow-editor)
assert_eq "piped stdin wins over EDITOR" "piped wins" "$RESULT_12_PIPED"
echo ""

# ============================================================
echo "=== Case 13: EDITOR value with disallowed characters rejected ==="
echo ""

ERR_13=$(EDITOR='rm -rf /tmp/x' resolve_body_no_stdin --allow-editor 2>&1 >/dev/null || true)
assert_contains "bad EDITOR: E_BODY_EDITOR_INVALID on stderr" "$ERR_13" "E_BODY_EDITOR_INVALID"
assert_exit_code "bad EDITOR exits 6" 6 \
  env ACCELERATOR_TEST_MODE=1 JIRA_BODY_STDIN_IS_TTY_TEST=1 \
  bash -c "source '$BODY_INPUT'; EDITOR='rm -rf /tmp/x' jira_resolve_body --allow-editor"

# Assert the rm command was NOT executed (no side effect)
assert_not_exists "rm not executed (no side effect)" "/tmp/x"
echo ""

# ============================================================
echo "=== Case 14: --body value beginning with '--' ==="
echo ""

RESULT_14=$(resolve_body --body "--summary foo")
assert_eq "--body value with leading -- returned verbatim" "--summary foo" "$RESULT_14"
echo ""

# ============================================================
test_summary
