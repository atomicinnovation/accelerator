#!/usr/bin/env bash
set -euo pipefail

# Template-shape contract test. For each row in templates-schema.tsv, parses
# the YAML frontmatter block at the head of the named template file and
# asserts the unified base fields, provenance fields (when code-state-
# anchored), per-type extras, the status-comment vocabulary, and the absence
# of any legacy own-identity key. See ADR-0033 for the contract.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=test-helpers.sh
source "$SCRIPT_DIR/test-helpers.sh"
cd "$ROOT"

echo "=== Template frontmatter shape ==="

SCHEMA_TSV="$SCRIPT_DIR/templates-schema.tsv"
# Cross-check inputs (both 0065 and 0066 carry Schema Reference tables;
# the union must match the TSV exactly).
WORK_ITEM_MDS=(
  "meta/work/0065-update-artifact-templates-to-unified-schema.md"
  "meta/work/0066-update-review-skills-inline-frontmatter.md"
)

BASE_FIELDS=(type id title date author producer status tags last_updated last_updated_by schema_version)
PROVENANCE_FIELDS=(revision repository)
FORBIDDEN_PROVENANCE_FIELDS=(git_commit branch)

# Self-check: every TSV row (including the header) must have exactly six
# tab-separated fields. Header is row 1 and is skipped by the data loop
# below.
if ! awk -F'\t' 'NF != 6 { print "ERROR: " FILENAME ":" NR " has " NF " fields, expected 6"; bad=1 } END { exit (bad ? 1 : 0) }' "$SCHEMA_TSV"; then
  echo "  FAIL: templates-schema.tsv field-count self-check"
  FAIL=$((FAIL + 1))
  test_summary
  exit 1
fi
echo "  PASS: templates-schema.tsv field-count self-check"
PASS=$((PASS + 1))

extract_frontmatter() {
  local file="$1"
  # Read everything between the first two `---` lines, normalising CRLF.
  tr -d '\r' < "$file" | awk '
    BEGIN { state=0 }
    /^---[[:space:]]*$/ {
      if (state == 0) { state=1; next }
      if (state == 1) { exit }
    }
    state == 1 { print }
  '
}

assert_in_block() {
  local test_name="$1" block="$2" regex="$3"
  if grep -qE "$regex" <<< "$block"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Regex: $regex"
    FAIL=$((FAIL + 1))
  fi
}

assert_not_in_block() {
  local test_name="$1" block="$2" regex="$3"
  if grep -qE "$regex" <<< "$block"; then
    echo "  FAIL: $test_name"
    echo "    Regex: $regex (should not match)"
    FAIL=$((FAIL + 1))
  else
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  fi
}

# Iterate TSV rows, skipping the header (row 1). Process substitution
# (rather than a pipe) keeps the loop in the parent shell so PASS/FAIL
# counter increments persist.
while IFS=$'\t' read -r template_file expected_type anchored extras status_vocab forbidden_own_id_key; do
  template_path="templates/$template_file"
  if [ ! -f "$template_path" ]; then
    echo "  FAIL: $template_file — template file not found at $template_path"
    FAIL=$((FAIL + 1))
    continue
  fi

  echo "--- $template_file (type=$expected_type) ---"

  block=$(extract_frontmatter "$template_path")
  if [ -z "$block" ]; then
    echo "  FAIL: $template_file — frontmatter block is empty or missing"
    FAIL=$((FAIL + 1))
    continue
  fi

  # Every base field present (anchored key line).
  for field in "${BASE_FIELDS[@]}"; do
    assert_in_block "$template_file: $field present" "$block" "^${field}:[[:space:]]"
  done

  # `type:` literal equals expected value.
  assert_in_block "$template_file: type is '$expected_type'" "$block" "^type:[[:space:]]+${expected_type}([[:space:]]+#.*)?$"

  # `schema_version: 1` (bare integer).
  assert_in_block "$template_file: schema_version is bare integer 1" "$block" "^schema_version:[[:space:]]+1([[:space:]]+#.*)?$"

  # `id:` value is a quoted YAML string.
  assert_in_block "$template_file: id value is quoted string" "$block" '^id:[[:space:]]+"[^"]*"([[:space:]]+#.*)?$'

  # Forbidden legacy own-identity key(s) absent (when applicable). The column
  # accepts a space-separated list of keys or the `-` sentinel for "no
  # forbidden key".
  if [ "$forbidden_own_id_key" != "-" ]; then
    for fkey in $forbidden_own_id_key; do
      assert_not_in_block "$template_file: legacy own-id key '$fkey' absent" "$block" "^${fkey}:[[:space:]]"
    done
  fi

  # Provenance bundle handling.
  if [ "$anchored" = "yes" ]; then
    for pfield in "${PROVENANCE_FIELDS[@]}"; do
      assert_in_block "$template_file: provenance field '$pfield' present" "$block" "^${pfield}:[[:space:]]"
    done
  fi
  for pfield in "${FORBIDDEN_PROVENANCE_FIELDS[@]}"; do
    assert_not_in_block "$template_file: forbidden provenance field '$pfield' absent" "$block" "^${pfield}:[[:space:]]"
  done

  # Per-type extras.
  for extra in $extras; do
    assert_in_block "$template_file: extra '$extra' present" "$block" "^${extra}:[[:space:]]"
  done

  # Status vocabulary verbatim on `status:` line (grep -F against the line).
  status_line=$(grep -E '^status:[[:space:]]' <<< "$block" || true)
  if [ -z "$status_line" ]; then
    echo "  FAIL: $template_file — no status: line"
    FAIL=$((FAIL + 1))
  elif grep -qF -- "$status_vocab" <<< "$status_line"; then
    echo "  PASS: $template_file: status vocabulary verbatim ($status_vocab)"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $template_file — status line missing pinned vocabulary"
    echo "    Expected to contain: $status_vocab"
    echo "    Actual: $status_line"
    FAIL=$((FAIL + 1))
  fi

done < <(tail -n +2 "$SCHEMA_TSV")

# Cross-check: parse the work-item Schema Reference markdown table(s) and
# assert the union of templates matches the TSV exactly. Both 0065 and 0066
# carry Schema Reference tables; their union is the authoritative source.
echo "--- Cross-check: work-item Schema Reference vs templates-schema.tsv ---"
existing_count=0
for wi in "${WORK_ITEM_MDS[@]}"; do
  [ -f "$wi" ] && existing_count=$((existing_count + 1))
done
if [ "$existing_count" -eq 0 ]; then
  echo "  SKIP: no work-item Schema Reference file present"
  SKIP=$((SKIP + 1))
else
  wi_templates=$(
    for wi in "${WORK_ITEM_MDS[@]}"; do
      if [ -f "$wi" ]; then
        awk '
          /^## Schema Reference/ { in_section=1; next }
          in_section && /^## / { in_section=0 }
          in_section && /^\| `[a-z0-9-]+\.md` \| / { print $0 }
        ' "$wi" | sed -E 's/^\| `([a-z0-9-]+\.md)` \|.*$/\1/'
      fi
    done | sort
  )
  tsv_templates=$(awk -F'\t' 'NR > 1 {print $1}' "$SCHEMA_TSV" | sort)
  if [ "$wi_templates" = "$tsv_templates" ]; then
    echo "  PASS: work-item Schema Reference templates match TSV"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: work-item Schema Reference templates differ from TSV"
    echo "    Work-item (union):"
    echo "$wi_templates" | sed 's/^/      /'
    echo "    TSV:"
    echo "$tsv_templates" | sed 's/^/      /'
    FAIL=$((FAIL + 1))
  fi
fi

test_summary
