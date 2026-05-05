#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-jql.sh / jira-jql-cli.sh
# Run: bash skills/integrations/jira/scripts/test-jira-jql.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

JQL_LIB="$SCRIPT_DIR/jira-jql.sh"
JQL_CLI="$SCRIPT_DIR/jira-jql-cli.sh"

# shellcheck source=/dev/null
source "$JQL_LIB"

# ============================================================
echo "=== jql_quote_value ==="
echo ""

echo "Test 1: simple value"
OUT=$(jql_quote_value 'simple')
assert_eq "simple quoted" "'simple'" "$OUT"

echo "Test 2: single-quote doubling"
OUT=$(jql_quote_value "don't")
assert_eq "single quote doubled" "'don''t'" "$OUT"

echo "Test 3: double quotes pass through"
OUT=$(jql_quote_value 'with "double"')
assert_eq "double quotes unchanged" "'with \"double\"'" "$OUT"

echo "Test 4: reserved word quoted"
OUT=$(jql_quote_value 'AND')
assert_eq "reserved word quoted" "'AND'" "$OUT"

echo "Test 5: empty value exits E_JQL_EMPTY_VALUE"
ERR=$(jql_quote_value "" 2>&1 >/dev/null || true)
assert_contains "names error code" "$ERR" "E_JQL_EMPTY_VALUE"
assert_exit_code "exits non-zero" 33 jql_quote_value ""

echo "Test 6: literal string EMPTY is just a value"
OUT=$(jql_quote_value 'EMPTY')
assert_eq "EMPTY string quoted normally" "'EMPTY'" "$OUT"

echo ""

# ============================================================
echo "=== jql_filter ==="
echo ""

echo "Test 7: simple field = value"
OUT=$(jql_filter status 'In Progress')
assert_eq "filter clause" "status = 'In Progress'" "$OUT"

echo ""

# ============================================================
echo "=== jql_compose IS EMPTY / IS NOT EMPTY ==="
echo ""

echo "Test 8: --empty flag"
OUT=$(jql_compose --all-projects --empty status)
assert_eq "IS EMPTY clause" "status IS EMPTY" "$OUT"

echo "Test 9: --not-empty flag"
OUT=$(jql_compose --all-projects --not-empty status)
assert_eq "IS NOT EMPTY clause" "status IS NOT EMPTY" "$OUT"

echo ""

# ============================================================
echo "=== jql_in / jql_not_in ==="
echo ""

echo "Test 10: jql_in with two values"
OUT=$(jql_in status 'In Progress' 'In Review')
assert_eq "IN clause" "status IN ('In Progress','In Review')" "$OUT"

echo "Test 11: jql_not_in with one value"
OUT=$(jql_not_in status Done)
assert_eq "NOT IN clause" "status NOT IN ('Done')" "$OUT"

echo ""

# ============================================================
echo "=== jql_split_neg ==="
echo ""

echo "Test 12: split positives and negatives"
jql_split_neg "Done" "~In Progress" "Backlog"
assert_eq "positives" "Done Backlog" "${JQL_POSITIVES[*]}"
assert_eq "negatives" "In Progress" "${JQL_NEGATIVES[*]}"

echo ""

# ============================================================
echo "=== jql_compose full composition ==="
echo ""

echo "Test 13: project + status IN/NOT IN + label"
OUT=$(jql_compose --project ENG --status 'In Progress' --status '~Done' --label bug)
assert_eq "composed query" \
  "project = 'ENG' AND status IN ('In Progress') AND status NOT IN ('Done') AND labels IN ('bug')" \
  "$OUT"

echo "Test 14: empty positives and negatives skips clause"
OUT=$(jql_compose --project ENG)
assert_eq "no status clause when none given" "project = 'ENG'" "$OUT"

echo "Test 15: no project and no --all-projects exits E_JQL_NO_PROJECT"
ERR=$(jql_compose --status Done 2>&1 >/dev/null || true)
assert_contains "names error code" "$ERR" "E_JQL_NO_PROJECT"
assert_exit_code "exits 30" 30 jql_compose --status Done

echo "Test 16: --all-projects omits project clause"
OUT=$(jql_compose --all-projects --status Done)
assert_eq "no project clause" "status IN ('Done')" "$OUT"

echo ""

# ============================================================
echo "=== Unsafe value rule ==="
echo ""

echo "Test 17: printable punctuation passes without --unsafe"
for val in 'feature/auth' 'Customer Champion?' '[brackets]' 'tag#1' \
           'email@example' '100%' '*wildcard*' 'path|pipe' 'bug;urgent'; do
  OUT=$(jql_quote_value "$val")
  assert_contains "quoted: $val" "$OUT" "'"
done

echo "Test 18: control char rejected with named byte"
ERR=$(printf 'foo\x07bar' | xargs -I{} bash -c 'source '"$JQL_LIB"'; jql_quote_value "$1" 2>&1 >/dev/null || true' _ {})
# Use a simpler approach: call jql_quote_value directly with a control char
CTRL_VAL=$'foo\x07bar'
ERR=$(jql_quote_value "$CTRL_VAL" 2>&1 >/dev/null || true)
assert_contains "names error code" "$ERR" "E_JQL_UNSAFE_VALUE"
assert_contains "names hex code or name" "$ERR" "0x07"
assert_exit_code "exits 31" 31 jql_quote_value "$CTRL_VAL"

echo "Test 18b: --unsafe overrides control char rejection"
OUT=$(jql_quote_value --unsafe "$CTRL_VAL")
assert_contains "output is quoted" "$OUT" "'"

echo ""

# ============================================================
echo "=== Fuzz sanity check ==="
echo ""

echo "Test 19: 100 random printable-ASCII inputs round-trip"
PRINTABLE='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 !"#$%&'"'"'()*+,-./:;<=>?@[\]^_`{|}~'
PRINTABLE_LEN=${#PRINTABLE}
FUZZ_FAIL=0
for i in $(seq 1 100); do
  # Generate a random length 1..40 string from printable ASCII
  LEN=$(( (RANDOM % 40) + 1 ))
  INPUT=""
  for j in $(seq 1 "$LEN"); do
    IDX=$(( RANDOM % PRINTABLE_LEN ))
    INPUT="${INPUT}${PRINTABLE:$IDX:1}"
  done
  OUT=$(jql_quote_value "$INPUT" 2>/dev/null) || { FUZZ_FAIL=$(( FUZZ_FAIL + 1 )); continue; }
  # Strip the surrounding single quotes and check no unescaped single quote remains
  INNER="${OUT:1:${#OUT}-2}"
  # Replace all '' (doubled quotes) with empty; no bare ' should remain
  STRIPPED="${INNER//\'\'/}"
  if [[ "$STRIPPED" == *"'"* ]]; then
    echo "  FUZZ FAIL on input: $(printf '%q' "$INPUT") -> $(printf '%q' "$OUT")"
    FUZZ_FAIL=$(( FUZZ_FAIL + 1 ))
  fi
done
if [[ "$FUZZ_FAIL" -eq 0 ]]; then
  echo "  PASS: fuzz sanity check (100 inputs)"
  PASS=$(( PASS + 1 ))
else
  echo "  FAIL: fuzz sanity check ($FUZZ_FAIL failures)"
  FAIL=$(( FAIL + 1 ))
fi

echo ""

# ============================================================
echo "=== --jql raw escape hatch ==="
echo ""

echo "Test 20: --jql appended verbatim with warning"
OUT=$(jql_compose --project ENG --jql 'assignee = currentUser() ORDER BY rank' 2>/dev/null)
assert_eq "raw jql appended" \
  "project = 'ENG' AND assignee = currentUser() ORDER BY rank" \
  "$OUT"
ERR=$(jql_compose --project ENG --jql 'assignee = currentUser()' 2>&1 >/dev/null || true)
assert_contains "warns about raw jql" "$ERR" "raw JQL"

echo ""

# ============================================================
echo "=== New flags: --type, --component, --reporter, --parent, --watching, --text ==="
echo ""

echo "Test 21: --type Bug"
OUT=$(jql_compose --project ENG --type Bug)
assert_eq "--type single value" "project = 'ENG' AND issuetype IN ('Bug')" "$OUT"

echo "Test 22: --type ~Bug negation"
OUT=$(jql_compose --project ENG --type '~Bug')
assert_eq "--type negation" "project = 'ENG' AND issuetype NOT IN ('Bug')" "$OUT"

echo "Test 23: --type multiple values"
OUT=$(jql_compose --project ENG --type Bug --type Story)
assert_eq "--type multi-value" "project = 'ENG' AND issuetype IN ('Bug','Story')" "$OUT"

echo "Test 24: --component with mixed positives and negatives"
OUT=$(jql_compose --project ENG --component frontend --component '~legacy')
assert_eq "--component mixed neg" \
  "project = 'ENG' AND component IN ('frontend') AND component NOT IN ('legacy')" \
  "$OUT"

echo "Test 25: --reporter"
OUT=$(jql_compose --project ENG --reporter alice)
assert_eq "--reporter single" "project = 'ENG' AND reporter IN ('alice')" "$OUT"

echo "Test 26: --parent"
OUT=$(jql_compose --project ENG --parent ENG-1)
assert_eq "--parent single" "project = 'ENG' AND parent IN ('ENG-1')" "$OUT"

echo "Test 27: --watching adds watcher = currentUser()"
OUT=$(jql_compose --project ENG --watching)
assert_eq "--watching clause" "project = 'ENG' AND watcher = currentUser()" "$OUT"

echo "Test 28: --text basic match"
OUT=$(jql_compose --project ENG --text "foo bar")
assert_eq "--text basic" 'project = '"'"'ENG'"'"' AND text ~ "foo bar"' "$OUT"

echo "Test 29: --text backslash escaped"
OUT=$(jql_compose --project ENG --text 'has\backslash')
assert_eq "--text backslash" 'project = '"'"'ENG'"'"' AND text ~ "has\\backslash"' "$OUT"

echo "Test 30: --text double-quote escaped"
OUT=$(jql_compose --project ENG --text 'has"quote')
assert_eq "--text double-quote" 'project = '"'"'ENG'"'"' AND text ~ "has\"quote"' "$OUT"

echo "Test 31: --text control character exits 31"
CTRL_VAL=$'foo\tbar'
ERR=$(jql_compose --project ENG --text "$CTRL_VAL" 2>&1 >/dev/null || true)
assert_contains "--text ctrl: error on stderr" "$ERR" "E_JQL_BAD_VALUE"
assert_exit_code "--text ctrl: exits 31" 31 jql_compose --project ENG --text "$CTRL_VAL"

echo "Test 32: --text injection resistance"
INJECT='foo" OR project = X OR text ~ "bar'
OUT=$(jql_compose --project ENG --text "$INJECT")
EXPECTED_CLAUSE='text ~ "foo\" OR project = X OR text ~ \"bar"'
assert_contains "--text injection resistance" "$OUT" "$EXPECTED_CLAUSE"

echo "Test 32b: --text backslash-quote ordering (adversarial: backslash+quote)"
OUT=$(jql_compose --project ENG --text '\"')
assert_eq "--text backslash-quote ordering" \
  'project = '"'"'ENG'"'"' AND text ~ "\\\""' \
  "$OUT"

echo "Test 33: combined flags compose correctly"
OUT=$(jql_compose --project ENG --status 'In Progress' --type Bug --component frontend --reporter alice --watching --text "API")
assert_eq "combined flags" \
  "project = 'ENG' AND status IN ('In Progress') AND issuetype IN ('Bug') AND component IN ('frontend') AND reporter IN ('alice') AND watcher = currentUser() AND text ~ \"API\"" \
  "$OUT"

echo ""

# ============================================================
test_summary
