#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-custom-fields.sh
# Run: bash skills/integrations/jira/scripts/test-jira-custom-fields.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

CUSTOM_FIELDS="$SCRIPT_DIR/jira-custom-fields.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# Build a test fields.json with various schema types
FIELDS_JSON="$TMPDIR_BASE/fields.json"
jq -cn '{
  "site": "example",
  "fields": [
    {
      "id": "customfield_10016",
      "key": "customfield_10016",
      "name": "Story Points",
      "slug": "story-points",
      "schema": {"type": "number", "custom": "com.atlassian.jira.plugin.system.customfieldtypes:float"}
    },
    {
      "id": "customfield_10020",
      "key": "customfield_10020",
      "name": "Sprint",
      "slug": "sprint",
      "schema": {"type": "array", "custom": "com.pyxis.greenhopper.jira:gh-sprint"}
    },
    {
      "id": "customfield_10100",
      "key": "customfield_10100",
      "name": "Notes",
      "slug": "notes",
      "schema": {"type": "string", "custom": "com.atlassian.jira.plugin.system.customfieldtypes:textarea"}
    },
    {
      "id": "customfield_10200",
      "key": "customfield_10200",
      "name": "Due Date",
      "slug": "due-date",
      "schema": {"type": "date"}
    },
    {
      "id": "customfield_10300",
      "key": "customfield_10300",
      "name": "Priority Level",
      "slug": "priority-level",
      "schema": {"type": "option"}
    },
    {
      "id": "customfield_10400",
      "key": "customfield_10400",
      "name": "Owner",
      "slug": "owner",
      "schema": {"type": "user"}
    }
  ]
}' > "$FIELDS_JSON"

# Helper: call _jira_coerce_custom_value in a subprocess
coerce() {
  bash -c "source '$CUSTOM_FIELDS'; _jira_coerce_custom_value \"\$@\"" -- "$@"
}

# ---------------------------------------------------------------------------

echo "=== Case 1: @json: prefix — valid JSON number ==="
echo ""

RESULT_1=$(coerce "customfield_10016" "@json:42" "$FIELDS_JSON" "E_BAD_FIELD")
assert_eq "@json: literal number returned" "42" "$RESULT_1"
echo ""

# ============================================================
echo "=== Case 2: @json: invalid JSON returns non-zero ==="
echo ""

ERR_2=$(coerce "customfield_10016" "@json:not-json" "$FIELDS_JSON" "E_BAD_FIELD" 2>&1 >/dev/null || true)
assert_contains "@json: invalid JSON: error on stderr" "E_BAD_FIELD" "$ERR_2"
assert_exit_code "@json: invalid JSON exits non-zero" 1 bash -c \
  "source '$CUSTOM_FIELDS'; _jira_coerce_custom_value customfield_10016 '@json:not-json' '$FIELDS_JSON' E_BAD_FIELD"
echo ""

# ============================================================
echo "=== Case 3: @json: array literal ==="
echo ""

RESULT_3=$(coerce "customfield_10020" "@json:[42]" "$FIELDS_JSON" "E_BAD_FIELD")
PARSED_3=$(jq -e '. == [42]' <<< "$RESULT_3")
assert_eq "@json: array parsed correctly" "true" "$PARSED_3"
echo ""

# ============================================================
echo "=== Case 4: schema.type=number, raw integer ==="
echo ""

RESULT_4=$(coerce "customfield_10016" "5" "$FIELDS_JSON" "E_BAD_FIELD")
assert_eq "number coercion: integer" "5" "$RESULT_4"
echo ""

# ============================================================
echo "=== Case 5: schema.type=number, non-numeric raw value ==="
echo ""

ERR_5=$(coerce "customfield_10016" "five" "$FIELDS_JSON" "E_BAD_FIELD" 2>&1 >/dev/null || true)
assert_contains "number: non-numeric rejected" "E_BAD_FIELD" "$ERR_5"
assert_exit_code "number: non-numeric exits non-zero" 1 bash -c \
  "source '$CUSTOM_FIELDS'; _jira_coerce_custom_value customfield_10016 five '$FIELDS_JSON' E_BAD_FIELD"
echo ""

# ============================================================
echo "=== Case 6: schema.type=string, raw value ==="
echo ""

RESULT_6=$(coerce "customfield_10100" "hello" "$FIELDS_JSON" "E_BAD_FIELD")
assert_eq "string coercion produces JSON string" '"hello"' "$RESULT_6"
echo ""

# ============================================================
echo "=== Case 7: schema.type=date, raw date string ==="
echo ""

RESULT_7=$(coerce "customfield_10200" "2026-05-03" "$FIELDS_JSON" "E_BAD_FIELD")
assert_eq "date coercion produces JSON string" '"2026-05-03"' "$RESULT_7"
echo ""

# ============================================================
echo "=== Case 8: schema.type=option, raw value ==="
echo ""

RESULT_8=$(coerce "customfield_10300" "High" "$FIELDS_JSON" "E_BAD_FIELD")
PARSED_8=$(jq -e '.value == "High"' <<< "$RESULT_8")
assert_eq "option coercion produces {value: ...}" "true" "$PARSED_8"
echo ""

# ============================================================
echo "=== Case 9: schema.type=user, accountId ==="
echo ""

RESULT_9=$(coerce "customfield_10400" "5b10a2844c20165700ede21g" "$FIELDS_JSON" "E_BAD_FIELD")
PARSED_9=$(jq -e '.accountId == "5b10a2844c20165700ede21g"' <<< "$RESULT_9")
assert_eq "user coercion produces {accountId: ...}" "true" "$PARSED_9"
echo ""

# ============================================================
echo "=== Case 10: field not in cache — no schema.type ==="
echo ""

EMPTY_FIELDS="$TMPDIR_BASE/empty-fields.json"
jq -cn '{"site":"example","fields":[]}' > "$EMPTY_FIELDS"

ERR_10=$(coerce "customfield_99999" "value" "$EMPTY_FIELDS" "E_BAD_FIELD" 2>&1 >/dev/null || true)
assert_contains "missing field: error on stderr" "E_BAD_FIELD" "$ERR_10"
assert_contains "missing field: refresh hint" "refresh-fields" "$ERR_10"
assert_exit_code "missing field exits non-zero" 1 bash -c \
  "source '$CUSTOM_FIELDS'; _jira_coerce_custom_value customfield_99999 value '$EMPTY_FIELDS' E_BAD_FIELD"
echo ""

# ============================================================
echo "=== Case 11: unsupported schema.type (array) — use @json: escape ==="
echo ""

ERR_11=$(coerce "customfield_10020" "42" "$FIELDS_JSON" "E_BAD_FIELD" 2>&1 >/dev/null || true)
assert_contains "unsupported type: error on stderr" "E_BAD_FIELD" "$ERR_11"
assert_contains "unsupported type: @json: hint in error" "@json:" "$ERR_11"
assert_exit_code "unsupported type exits non-zero" 1 bash -c \
  "source '$CUSTOM_FIELDS'; _jira_coerce_custom_value customfield_10020 42 '$FIELDS_JSON' E_BAD_FIELD"
echo ""

# ============================================================
echo "=== Case 12: custom error_prefix parameter ==="
echo ""

ERR_12=$(coerce "customfield_10016" "not-a-number" "$FIELDS_JSON" "E_CREATE_BAD_FIELD" 2>&1 >/dev/null || true)
assert_contains "custom prefix: error starts with E_CREATE_BAD_FIELD" "E_CREATE_BAD_FIELD" "$ERR_12"
echo ""

# ============================================================
test_summary
