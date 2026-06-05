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

# Pin locale so the [A-Za-z0-9-] ranges, tolower/sort, and regex matching
# behave deterministically regardless of the host locale (parity with the
# project's existing LANG=C discipline).
export LC_ALL=C

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

# Cardinality lookup by linkage-key name. case-based (not `declare -A`)
# so the script keeps running on bash 3.2, the macOS default (a `declare -A`
# aborts the whole script under `set -euo pipefail` there). Echoes `single`,
# `list`, or empty (unknown key).
linkage_cardinality() {
  case "$1" in
    parent | superseded_by | target | source) echo single ;;
    supersedes | blocks | blocked_by | derived_from | relates_to) echo list ;;
    *) echo "" ;;
  esac
}

# Curated source-type set used inside the comment regex. Kept as a
# pipe-joined string so it can be interpolated into ERE patterns.
SOURCE_TYPE_RE='work-item|plan|adr|pr|codebase-research|issue-research|pr-description|design-inventory|design-gap|plan-validation|plan-review|work-item-review|pr-review'

# The blocked_by inverse-key guidance line. It lives on its own full-line
# comment beneath the slot, so it never breaks the list regex's `\[\]$`
# end-anchor; the post-check greps for it across the whole block.
INVERSE_GUIDANCE_LINE='# inverse of blocks — producers SHOULD prefer writing blocks: on the canonical side'

# Union of all linkage-vocabulary key names (used by the closed-set check).
# Keep aligned with linkage_cardinality(). superseded_by is listed as a guard
# even though no template carries it, so the closed-set check rejects any
# template that adds it.
LINKAGE_VOCABULARY=(parent superseded_by target source supersedes blocks blocked_by derived_from relates_to)

# rc 0 = slot shape+comment valid (and, for blocked_by, the standalone
# inverse-guidance line is present); 1 = rejected; 2 = unknown key. The
# inverse-guidance check lives HERE (not in the live loop) so the
# negative-fixture self-test exercises the same code path.
check_linkage_slot() {
  local block="$1" key="$2" regex
  case "$(linkage_cardinality "$key")" in
    single) regex="^${key}:[[:space:]]+\"\"[[:space:]]+#[[:space:]]+typed-linkage[[:space:]]+ref:[[:space:]]+\"(${SOURCE_TYPE_RE}):[A-Za-z0-9-]+\"[[:space:]]+or[[:space:]]+\"\"$" ;;
    list) regex="^${key}:[[:space:]]+\\[\\][[:space:]]+#[[:space:]]+typed-linkage[[:space:]]+list:[[:space:]]+\\[\"(${SOURCE_TYPE_RE}):[A-Za-z0-9-]+\",[[:space:]]+\\.\\.\\.\\][[:space:]]+or[[:space:]]+\\[\\]$" ;;
    *) return 2 ;;
  esac
  grep -qE "$regex" <<<"$block" || return 1
  if [ "$key" = blocked_by ]; then
    grep -qF -- "$INVERSE_GUIDANCE_LINE" <<<"$block" || return 1
  fi
  return 0
}

# rc 0 = no spurious linkage key; 1 = a vocabulary key is present in the
# block but absent from $keys and not exempt via $extras. The extras
# exemption: design-inventory carries a foreign-source `source:` (an extra,
# not a typed-linkage edge); without it the name-based walk would misclassify
# it and FAIL a template the plan leaves untouched.
check_closed_set() {
  local block="$1" extras="$2" keys="$3" vkey
  for vkey in "${LINKAGE_VOCABULARY[@]}"; do
    grep -qE "^${vkey}:[[:space:]]" <<<"$block" || continue
    case " $extras " in *" $vkey "*) continue ;; esac # declared extra
    case " $keys " in *" $vkey "*) continue ;; esac   # declared slot
    return 1                                          # spurious
  done
  return 0
}

# Self-check: every TSV row (including the header) must have exactly seven
# tab-separated fields. Header is row 1 and is skipped by the data loop
# below.
if ! awk -F'\t' 'NF != 7 { print "ERROR: " FILENAME ":" NR " has " NF " fields, expected 7"; bad=1 } END { exit (bad ? 1 : 0) }' "$SCHEMA_TSV"; then
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
  tr -d '\r' <"$file" | awk '
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
  if grep -qE "$regex" <<<"$block"; then
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
  if grep -qE "$regex" <<<"$block"; then
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
while IFS=$'\t' read -r template_file expected_type anchored extras status_vocab forbidden_own_id_key typed_linkage_keys; do
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

  # Typed-linkage slots: shape + comment grammar per cardinality (and, for
  # blocked_by, the standalone inverse-guidance line). check_linkage_slot is
  # rc-returning (no counter mutation); this loop is the ONLY place the live
  # run mutates PASS/FAIL for it. `rc=0; ... || rc=$?` keeps the non-zero
  # paths from tripping `set -e`.
  for lkey in $typed_linkage_keys; do
    rc=0
    check_linkage_slot "$block" "$lkey" || rc=$?
    case "$rc" in
      0)
        echo "  PASS: $template_file: linkage slot '$lkey' shape+comment"
        PASS=$((PASS + 1))
        ;;
      2)
        echo "  FAIL: $template_file — unknown linkage key '$lkey'; add it to linkage_cardinality() (and LINKAGE_VOCABULARY) or correct the row"
        FAIL=$((FAIL + 1))
        ;;
      *)
        echo "  FAIL: $template_file: linkage slot '$lkey' bad shape/comment (or missing inverse-guidance line)"
        FAIL=$((FAIL + 1))
        ;;
    esac
  done

  # Closed-set: no linkage-vocabulary key may appear in the block unless it is
  # a declared slot (typed_linkage_keys) or a declared extra (the
  # design-inventory `source:` exemption).
  if check_closed_set "$block" "$extras" "$typed_linkage_keys"; then
    echo "  PASS: $template_file: closed-set (no spurious linkage keys)"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $template_file: closed-set violated (a linkage key not in the TSV row)"
    FAIL=$((FAIL + 1))
  fi

  # Status vocabulary verbatim on `status:` line (grep -F against the line).
  status_line=$(grep -E '^status:[[:space:]]' <<<"$block" || true)
  if [ -z "$status_line" ]; then
    echo "  FAIL: $template_file — no status: line"
    FAIL=$((FAIL + 1))
  elif grep -qF -- "$status_vocab" <<<"$status_line"; then
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
          in_section && /^\|[[:space:]]+`[a-z0-9-]+\.md`[[:space:]]+\|/ { print $0 }
        ' "$wi" | sed -E 's/^\|[[:space:]]+`([a-z0-9-]+\.md)`[[:space:]]+\|.*$/\1/'
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

# ---------------------------------------------------------------------------
# Negative-fixture self-test. Because this script's deliverable IS assertions,
# a green run against only-valid templates does not prove they can reject bad
# input (an inert regex produces zero FAIL, indistinguishable from success).
# Feed known-bad blocks through the SAME pure functions the live loop wraps
# and assert each is rejected. Runs on every invocation (guarded inline, not a
# sibling file, so CI always exercises it). Gated on the exact PASS count.
# ---------------------------------------------------------------------------
echo "--- Self-test: negative fixtures (each new assertion must reject bad input) ---"
selftest_pass=0
st_reject() {
  # $1 = description, $2 = rc from a check (non-zero = rejected, as expected).
  local desc="$1" rc="$2"
  if [ "$rc" -ne 0 ]; then
    echo "  PASS: self-test rejects $desc"
    selftest_pass=$((selftest_pass + 1))
  else
    echo "  FAIL: self-test did NOT reject $desc (assertion is inert)"
    FAIL=$((FAIL + 1))
  fi
}

# 1. list slot carrying a single-ref value — wrong cardinality.
fixture=$'blocks: ""                                   # typed-linkage list: ["work-item:NNNN", ...] or []'
rc=0
check_linkage_slot "$fixture" blocks || rc=$?
st_reject "a list slot with a single-ref value" "$rc"

# 2. slot with a malformed comment.
fixture=$'parent: ""                                   # see ADR-0034'
rc=0
check_linkage_slot "$fixture" parent || rc=$?
st_reject "a slot with a malformed comment" "$rc"

# 3. blocked_by slot missing its standalone inverse-guidance line.
fixture=$'blocked_by: []                               # typed-linkage list: ["work-item:NNNN", ...] or []'
rc=0
check_linkage_slot "$fixture" blocked_by || rc=$?
st_reject "a blocked_by slot missing the inverse-guidance line" "$rc"

# 4. block carrying a vocabulary key absent from its TSV row (spurious slot).
fixture=$'relates_to: []                               # typed-linkage list: ["work-item:NNNN", ...] or []'
rc=0
check_closed_set "$fixture" "" "parent" || rc=$?
st_reject "a block carrying a linkage key absent from its TSV row" "$rc"

# 5. declared slot absent from the frontmatter (no matching line).
fixture=$'title: "x"'
rc=0
check_linkage_slot "$fixture" parent || rc=$?
st_reject "a declared slot absent from the frontmatter" "$rc"

# 6. comment whose source-type token is outside the curated set.
fixture=$'parent: ""                                   # typed-linkage ref: "ticket:NNNN" or ""'
rc=0
check_linkage_slot "$fixture" parent || rc=$?
st_reject "a comment with an out-of-vocabulary source-type token" "$rc"

if [ "$selftest_pass" -eq 6 ]; then
  echo "  PASS: negative-fixture self-test count (6)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: negative-fixture self-test count is $selftest_pass, expected 6"
  FAIL=$((FAIL + 1))
fi

# ---------------------------------------------------------------------------
# Vocabulary-drift structural guard: every LINKAGE_VOCABULARY entry must have
# a non-empty cardinality. Catches the two hand-maintained lists drifting (a
# key in one but not the other turns the suite red immediately). Gated on the
# exact count.
# ---------------------------------------------------------------------------
echo "--- Self-test: vocabulary-drift guard (every vocabulary key has a cardinality) ---"
vocab_pass=0
for vkey in "${LINKAGE_VOCABULARY[@]}"; do
  if [ -n "$(linkage_cardinality "$vkey")" ]; then
    echo "  PASS: vocabulary key '$vkey' has a cardinality"
    vocab_pass=$((vocab_pass + 1))
  else
    echo "  FAIL: vocabulary key '$vkey' has no cardinality (LINKAGE_VOCABULARY and linkage_cardinality drifted)"
    FAIL=$((FAIL + 1))
  fi
done
if [ "$vocab_pass" -eq 9 ]; then
  echo "  PASS: vocabulary-drift guard count (9)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: vocabulary-drift guard count is $vocab_pass, expected 9"
  FAIL=$((FAIL + 1))
fi

test_summary
