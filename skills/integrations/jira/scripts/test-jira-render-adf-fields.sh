#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-render-adf-fields.sh
# Run: bash skills/integrations/jira/scripts/test-jira-render-adf-fields.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

RENDER="$SCRIPT_DIR/jira-render-adf-fields.sh"
TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# ---------------------------------------------------------------------------
# ADF test documents (inline)

SIMPLE_ADF='{"type":"doc","version":1,"content":[{"type":"paragraph","content":[{"type":"text","text":"hello world"}]}]}'

RICH_ADF='{"type":"doc","version":1,"content":[{"type":"heading","attrs":{"level":2},"content":[{"type":"text","text":"Section Title"}]},{"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Item with "},{"type":"text","text":"bold","marks":[{"type":"strong"}]},{"type":"text","text":" and "},{"type":"text","text":"italic","marks":[{"type":"em"}]},{"type":"text","text":" and "},{"type":"text","text":"a link","marks":[{"type":"link","attrs":{"href":"http://example.com"}}]}]}]}]}]}'

CODE_ADF='{"type":"doc","version":1,"content":[{"type":"paragraph","content":[{"type":"text","text":"First paragraph."}]},{"type":"codeBlock","attrs":{"language":"python"},"content":[{"type":"text","text":"print(42)"}]},{"type":"paragraph","content":[{"type":"text","text":"Second paragraph."}]}]}'

# ---------------------------------------------------------------------------
# Helper: create a test issue JSON with a given ADF description
make_issue() {
  local adf="$1"
  jq -cn --argjson adf "$adf" '{"key":"ENG-1","id":"10001","fields":{"summary":"Test issue","description":$adf}}'
}

# ---------------------------------------------------------------------------

echo "=== Case 1a: single paragraph description rendered ==="
echo ""

ISSUE=$(make_issue "$SIMPLE_ADF")
OUT=$(printf '%s' "$ISSUE" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
DESC=$(printf '%s' "$OUT" | jq -r '.fields.description')
assert_eq "simple paragraph rendered" "hello world" "$DESC"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 1b: heading + bullet list + inline marks ==="
echo ""

ISSUE=$(make_issue "$RICH_ADF")
OUT=$(printf '%s' "$ISSUE" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
DESC=$(printf '%s' "$OUT" | jq -r '.fields.description')
assert_contains "heading marker present" "$DESC" "## "
# BSD grep rejects patterns starting with '-'; use grep -- to bypass
if printf '%s' "$DESC" | grep -qF -- "- "; then
  echo "  PASS: list item marker present"
  PASS=$((PASS + 1))
else
  echo "  FAIL: list item marker present"
  FAIL=$((FAIL + 1))
fi
assert_contains "bold marker present" "$DESC" "**"
assert_contains "italic marker present" "$DESC" "*"
assert_contains "link target present" "$DESC" "](http"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 1c: code block + multiple paragraphs ==="
echo ""

ISSUE=$(make_issue "$CODE_ADF")
OUT=$(printf '%s' "$ISSUE" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
DESC=$(printf '%s' "$OUT" | jq -r '.fields.description')
assert_contains "fenced code block marker" '```' "$DESC"
assert_contains "first paragraph text" "$DESC" "First paragraph."
assert_contains "second paragraph text" "$DESC" "Second paragraph."
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 2: null description preserved ==="
echo ""

NULL_ISSUE='{"key":"ENG-2","id":"10002","fields":{"summary":"No desc","description":null}}'
OUT=$(printf '%s' "$NULL_ISSUE" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
DESC_TYPE=$(printf '%s' "$OUT" | jq -r '.fields.description | type')
assert_eq "null description type preserved" "null" "$DESC_TYPE"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 3: environment field rendered ==="
echo ""

ENV_ISSUE=$(jq -cn --argjson adf "$SIMPLE_ADF" \
  '{"key":"ENG-3","id":"10003","fields":{"summary":"Env test","environment":$adf}}')
OUT=$(printf '%s' "$ENV_ISSUE" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
ENV=$(printf '%s' "$OUT" | jq -r '.fields.environment')
assert_eq "environment field rendered" "hello world" "$ENV"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 4: comment bodies rendered ==="
echo ""

COMMENT_ISSUE=$(jq -cn \
  --argjson adf1 "$SIMPLE_ADF" \
  --argjson adf2 "$SIMPLE_ADF" \
  '{"key":"ENG-4","id":"10004","fields":{"summary":"Comments","comment":{"comments":[
    {"id":"1","body":$adf1,"created":"2026-01-01T00:00:00.000+0000"},
    {"id":"2","body":$adf2,"created":"2026-01-02T00:00:00.000+0000"}
  ]}}}'
)
OUT=$(printf '%s' "$COMMENT_ISSUE" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
BODY0=$(printf '%s' "$OUT" | jq -r '.fields.comment.comments[0].body')
BODY1=$(printf '%s' "$OUT" | jq -r '.fields.comment.comments[1].body')
assert_eq "first comment body rendered" "hello world" "$BODY0"
assert_eq "second comment body rendered" "hello world" "$BODY1"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5: search response — descriptions rendered ==="
echo ""

SEARCH=$(jq -cn \
  --argjson adf1 "$SIMPLE_ADF" \
  --argjson adf2 "$SIMPLE_ADF" \
  '{"issues":[
    {"key":"ENG-1","fields":{"description":$adf1}},
    {"key":"ENG-2","fields":{"description":$adf2}}
  ]}'
)
OUT=$(printf '%s' "$SEARCH" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
D0=$(printf '%s' "$OUT" | jq -r '.issues[0].fields.description')
D1=$(printf '%s' "$OUT" | jq -r '.issues[1].fields.description')
assert_eq "first issue description rendered" "hello world" "$D0"
assert_eq "second issue description rendered" "hello world" "$D1"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 6: custom textarea field rendered ==="
echo ""

# Create a fields.json cache with a textarea custom field
FIELDS_CACHE="$TMPDIR_BASE/fields-textarea.json"
jq -cn '{
  "site":"example",
  "fields":[
    {"id":"customfield_10100","key":"customfield_10100","name":"Design Notes",
     "slug":"design-notes","schema":{"custom":"com.atlassian.jira.plugin.system.customfieldtypes:textarea"}}
  ]
}' > "$FIELDS_CACHE"

CF_ISSUE=$(jq -cn --argjson adf "$SIMPLE_ADF" \
  '{"key":"ENG-5","id":"10005","fields":{"summary":"CF test","customfield_10100":$adf}}')
OUT=$(printf '%s' "$CF_ISSUE" | \
  ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_JIRA_FIELDS_CACHE_PATH_TEST="$FIELDS_CACHE" \
  bash "$RENDER")
CF_VAL=$(printf '%s' "$OUT" | jq -r '.fields.customfield_10100')
assert_eq "custom textarea field rendered" "hello world" "$CF_VAL"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 7: custom non-textarea field NOT rendered ==="
echo ""

# Cache with a textfield (not textarea) custom field
FIELDS_CACHE7="$TMPDIR_BASE/fields-textfield.json"
jq -cn '{
  "site":"example",
  "fields":[
    {"id":"customfield_10200","key":"customfield_10200","name":"Short Text",
     "slug":"short-text","schema":{"custom":"com.atlassian.jira.plugin.system.customfieldtypes:textfield"}}
  ]
}' > "$FIELDS_CACHE7"

CF_ISSUE7=$(jq -cn --argjson adf "$SIMPLE_ADF" \
  '{"key":"ENG-6","id":"10006","fields":{"summary":"Non-CF","customfield_10200":$adf}}')
OUT7=$(printf '%s' "$CF_ISSUE7" | \
  ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_JIRA_FIELDS_CACHE_PATH_TEST="$FIELDS_CACHE7" \
  bash "$RENDER")
CF_TYPE=$(printf '%s' "$OUT7" | jq -r '.fields.customfield_10200 | type')
assert_eq "non-textarea field not rendered (still object)" "object" "$CF_TYPE"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 8: empty issues array round-trips unchanged ==="
echo ""

EMPTY_SEARCH='{"issues":[],"nextPageToken":"tok123"}'
OUT8=$(printf '%s' "$EMPTY_SEARCH" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
ISSUE_LEN=$(printf '%s' "$OUT8" | jq '.issues | length')
TOKEN=$(printf '%s' "$OUT8" | jq -r '.nextPageToken')
assert_eq "empty issues array preserved" "0" "$ISSUE_LEN"
assert_eq "nextPageToken preserved" "tok123" "$TOKEN"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 9: missing fields block round-trips unchanged ==="
echo ""

NO_FIELDS='{"key":"ENG-7","id":"10007","summary":"No fields"}'
OUT9=$(printf '%s' "$NO_FIELDS" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
KEY=$(printf '%s' "$OUT9" | jq -r '.key')
assert_eq "key preserved when no fields" "ENG-7" "$KEY"
HAS_FIELDS=$(printf '%s' "$OUT9" | jq 'has("fields")')
assert_eq "no fields key added" "false" "$HAS_FIELDS"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 10: bad JSON exits 90 ==="
echo ""

assert_exit_code "bad JSON exits 90" 90 bash -c "printf '%s' 'not-json' | ACCELERATOR_TEST_MODE=1 bash '$RENDER'"
ERR10=$(printf '%s' 'not-json' | ACCELERATOR_TEST_MODE=1 bash "$RENDER" 2>&1 >/dev/null || true)
assert_contains "E_RENDER_BAD_INPUT on stderr" "$ERR10" "E_RENDER_BAD_INPUT"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 11a: idempotent — byte-identical second pass ==="
echo ""

ISSUE11=$(make_issue "$SIMPLE_ADF")
PASS1=$(printf '%s' "$ISSUE11" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
PASS2=$(printf '%s' "$PASS1"   | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
assert_eq "second pass byte-identical" "$PASS1" "$PASS2"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 11b: renderer not re-spawned on second pass ==="
echo ""

# Create a stub renderer — the render script picks it up via the test seam
# ACCELERATOR_JIRA_ADF_RENDERER_TEST (gated by ACCELERATOR_TEST_MODE=1).
# The stub increments STUB_COUNTER_FILE on each invocation and emits a
# plain string so the second pass won't try to re-render it.
STUB_DIR=$(mktemp -d "$TMPDIR_BASE/stub-XXXXXX")
COUNTER_FILE=$(mktemp "$TMPDIR_BASE/counter-XXXXXX")

cat > "$STUB_DIR/jira-adf-to-md.sh" <<'STUBEOF'
#!/usr/bin/env bash
if [[ -n "${STUB_COUNTER_FILE:-}" ]]; then
  _c=$(cat "${STUB_COUNTER_FILE}" 2>/dev/null || echo 0)
  echo $((_c + 1)) > "${STUB_COUNTER_FILE}"
fi
printf '%s\n' "rendered markdown"
STUBEOF
chmod +x "$STUB_DIR/jira-adf-to-md.sh"

# First pass on ADF issue: renderer should be called once (description)
echo 0 > "$COUNTER_FILE"
ISSUE11B=$(make_issue "$SIMPLE_ADF")
PASS1_OUT=$(printf '%s' "$ISSUE11B" | \
  ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_JIRA_ADF_RENDERER_TEST="$STUB_DIR/jira-adf-to-md.sh" \
  STUB_COUNTER_FILE="$COUNTER_FILE" \
  bash "$RENDER")
COUNT_AFTER_PASS1=$(cat "$COUNTER_FILE")
assert_eq "renderer called once on first pass" "1" "$COUNT_AFTER_PASS1"

# Second pass on already-rendered output: type-predicate gate must
# short-circuit before spawning the renderer (strings are not ADF docs)
echo 0 > "$COUNTER_FILE"
printf '%s' "$PASS1_OUT" | \
  ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_JIRA_ADF_RENDERER_TEST="$STUB_DIR/jira-adf-to-md.sh" \
  STUB_COUNTER_FILE="$COUNTER_FILE" \
  bash "$RENDER" > /dev/null
COUNT_AFTER_PASS2=$(cat "$COUNTER_FILE")
assert_eq "renderer not called on second pass" "0" "$COUNT_AFTER_PASS2"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 12: missing comment.comments path is a no-op ==="
echo ""

NO_COMMENTS='{"key":"ENG-8","id":"10008","fields":{"summary":"No comment block","description":null}}'
OUT12=$(printf '%s' "$NO_COMMENTS" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
KEY12=$(printf '%s' "$OUT12" | jq -r '.key')
assert_eq "key preserved when no comment block" "ENG-8" "$KEY12"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 13: ACCELERATOR_TEST_MODE=true (not '1') ignores cache override ==="
echo ""

# A fields cache that WOULD activate customfield_10100 rendering
FIELDS_CACHE13="$TMPDIR_BASE/fields-gate-test.json"
jq -cn '{
  "site":"example",
  "fields":[
    {"id":"customfield_10100","key":"customfield_10100","name":"Design Notes",
     "slug":"design-notes","schema":{"custom":"com.atlassian.jira.plugin.system.customfieldtypes:textarea"}}
  ]
}' > "$FIELDS_CACHE13"

CF_ISSUE13=$(jq -cn --argjson adf "$SIMPLE_ADF" \
  '{"key":"ENG-9","id":"10009","fields":{"summary":"Gate test","customfield_10100":$adf}}')

# ACCELERATOR_TEST_MODE=true (not "1") — gate should NOT activate the cache override
OUT13=$(printf '%s' "$CF_ISSUE13" | \
  ACCELERATOR_TEST_MODE=true \
  ACCELERATOR_JIRA_FIELDS_CACHE_PATH_TEST="$FIELDS_CACHE13" \
  bash "$RENDER" 2>/dev/null || true)

# When gate is inactive, the walker has no fields.json to read, so the
# custom field should NOT be rendered (ADF object remains untouched)
CF_TYPE13=$(printf '%s' "$OUT13" | jq -r '.fields.customfield_10100 | type')
assert_eq "custom field not rendered when gate inactive" "object" "$CF_TYPE13"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 14: comment-list shape — bodies rendered ==="
echo ""

COMMENT_LIST=$(jq -cn \
  --argjson adf1 "$SIMPLE_ADF" \
  --argjson adf2 "$SIMPLE_ADF" \
  '{startAt:0, maxResults:50, total:2,
    comments:[
      {"id":"1","body":$adf1,"author":{"accountId":"u1","displayName":"Alice"},"created":"2026-01-01T00:00:00.000+0000"},
      {"id":"2","body":$adf2,"author":{"accountId":"u2","displayName":"Bob"},"created":"2026-01-02T00:00:00.000+0000"}
    ]}')
OUT14=$(printf '%s' "$COMMENT_LIST" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
assert_eq "comment-list: first body rendered"  "hello world" "$(printf '%s' "$OUT14" | jq -r '.comments[0].body')"
assert_eq "comment-list: second body rendered" "hello world" "$(printf '%s' "$OUT14" | jq -r '.comments[1].body')"
assert_eq "comment-list: startAt preserved"    "0"           "$(printf '%s' "$OUT14" | jq -r '.startAt')"
assert_eq "comment-list: total preserved"      "2"           "$(printf '%s' "$OUT14" | jq -r '.total')"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 15: single-comment shape — body rendered ==="
echo ""

SINGLE_COMMENT=$(jq -cn \
  --argjson adf "$SIMPLE_ADF" \
  '{id:"42", body:$adf, author:{"accountId":"u1","displayName":"Alice"}, created:"2026-01-01T00:00:00.000+0000"}')
OUT15=$(printf '%s' "$SINGLE_COMMENT" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
assert_eq "single-comment: body rendered" "hello world" "$(printf '%s' "$OUT15" | jq -r '.body')"
assert_eq "single-comment: id preserved"  "42"          "$(printf '%s' "$OUT15" | jq -r '.id')"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 16: already-rendered comment-list is idempotent ==="
echo ""

PASS1_16=$(printf '%s' "$COMMENT_LIST" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
PASS2_16=$(printf '%s' "$PASS1_16"     | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
assert_eq "comment-list idempotent" "$PASS1_16" "$PASS2_16"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 17: single-comment with non-ADF body passes through unchanged ==="
echo ""

PLAIN_COMMENT='{"id":"42","body":"plain text body","author":{"accountId":"u1","displayName":"Alice"}}'
OUT17=$(printf '%s' "$PLAIN_COMMENT" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
assert_eq "non-ADF body preserved" "plain text body" "$(printf '%s' "$OUT17" | jq -r '.body')"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 18: single-issue with top-level startAt+comments dispatches to single-issue ==="
echo ""

# Regression guard: a shape that matches has("comments") and has("startAt") but ALSO
# has("fields") must NOT dispatch to the comment-list branch.
ISSUE_WITH_COMMENTS=$(jq -cn \
  --argjson adf "$SIMPLE_ADF" \
  '{key:"ENG-1", startAt:0, fields:{description:$adf, summary:"Test"}, comments:[{id:"1"}]}')
OUT18=$(printf '%s' "$ISSUE_WITH_COMMENTS" | ACCELERATOR_TEST_MODE=1 bash "$RENDER")
assert_eq "issue+comments: description rendered (single-issue branch used)" \
  "hello world" "$(printf '%s' "$OUT18" | jq -r '.fields.description')"
assert_eq "issue+comments: top-level comments array preserved" \
  "1" "$(printf '%s' "$OUT18" | jq '.comments | length')"
echo ""

# ---------------------------------------------------------------------------
test_summary
