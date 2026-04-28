#!/usr/bin/env bash
set -euo pipefail

# Test harness for the work-item pattern compiler (work-item-pattern.sh
# CLI wrapper plus the wip_* functions in work-item-common.sh).
# Run: bash skills/work/scripts/test-work-item-pattern.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

PATTERN_CLI="$SCRIPT_DIR/work-item-pattern.sh"
COMMON_LIB="$SCRIPT_DIR/work-item-common.sh"

assert_contains() {
  local test_name="$1" needle="$2" haystack="$3"
  if printf '%s' "$haystack" | grep -qF "$needle"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected to contain: $needle"
    echo "    Actual: $haystack"
    FAIL=$((FAIL + 1))
  fi
}

assert_matches_regex() {
  local test_name="$1" regex="$2" subject="$3"
  if printf '%s' "$subject" | grep -qE "$regex"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Subject: $subject"
    echo "    Regex:   $regex"
    FAIL=$((FAIL + 1))
  fi
}

assert_not_matches_regex() {
  local test_name="$1" regex="$2" subject="$3"
  if printf '%s' "$subject" | grep -qE "$regex"; then
    echo "  FAIL: $test_name"
    echo "    Subject: $subject"
    echo "    Should not have matched: $regex"
    FAIL=$((FAIL + 1))
  else
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  fi
}

# ============================================================
echo "=== work-item-pattern.sh --validate ==="
echo ""

echo "Test: default pattern {number:04d} validates"
assert_exit_code "exits 0" 0 bash "$PATTERN_CLI" --validate "{number:04d}"

echo "Test: numeric-only pattern with bare {number} validates"
assert_exit_code "exits 0" 0 bash "$PATTERN_CLI" --validate "{number}"

echo "Test: project + number pattern validates"
assert_exit_code "exits 0" 0 bash "$PATTERN_CLI" --validate "{project}-{number:04d}"

echo "Test: width variant {number:05d} validates"
assert_exit_code "exits 0" 0 bash "$PATTERN_CLI" --validate "{number:05d}"

echo "Test: pattern with escaped {{ braces validates"
assert_exit_code "exits 0" 0 bash "$PATTERN_CLI" --validate "{{x}}-{number:04d}"

echo "Test: missing {number} token (rule 1) fails with E_PATTERN_NO_NUMBER_TOKEN"
ERR=$(bash "$PATTERN_CLI" --validate "no-number" 2>&1 >/dev/null || true)
assert_contains "stderr names rule" "E_PATTERN_NO_NUMBER_TOKEN" "$ERR"
assert_exit_code "exits 2" 2 bash "$PATTERN_CLI" --validate "no-number"

echo "Test: hostile char (rule 2) fails with E_PATTERN_HOSTILE_CHAR"
ERR=$(bash "$PATTERN_CLI" --validate "a/b-{number:04d}" 2>&1 >/dev/null || true)
assert_contains "stderr names rule" "E_PATTERN_HOSTILE_CHAR" "$ERR"
assert_exit_code "exits 2" 2 bash "$PATTERN_CLI" --validate "a/b-{number:04d}"

echo "Test: adjacent dynamic tokens (rule 3) fails with E_PATTERN_ADJACENT_TOKENS"
ERR=$(bash "$PATTERN_CLI" --validate "{project}{number:04d}" 2>&1 >/dev/null || true)
assert_contains "stderr names rule" "E_PATTERN_ADJACENT_TOKENS" "$ERR"
assert_exit_code "exits 2" 2 bash "$PATTERN_CLI" --validate "{project}{number:04d}"

echo "Test: bad format spec (rule 4) fails with E_PATTERN_BAD_FORMAT_SPEC"
ERR=$(bash "$PATTERN_CLI" --validate "{number:foo}" 2>&1 >/dev/null || true)
assert_contains "stderr names rule" "E_PATTERN_BAD_FORMAT_SPEC" "$ERR"
assert_exit_code "exits 2" 2 bash "$PATTERN_CLI" --validate "{number:foo}"

echo "Test: non-padded format spec %d (rule 4) fails"
ERR=$(bash "$PATTERN_CLI" --validate "{number:d}" 2>&1 >/dev/null || true)
assert_contains "stderr names rule" "E_PATTERN_BAD_FORMAT_SPEC" "$ERR"

echo "Test: usage with no argument fails with exit 1"
assert_exit_code "exits 1" 1 bash "$PATTERN_CLI" --validate

echo ""

# ============================================================
echo "=== work-item-pattern.sh --compile-scan ==="
echo ""

echo "Test: default {number:04d} compiles to ^([0-9]+)-"
OUT=$(bash "$PATTERN_CLI" --compile-scan "{number:04d}" "")
assert_eq "scan regex" '^([0-9]+)-' "$OUT"

echo "Test: width variant {number:05d} compiles to width-agnostic ^([0-9]+)-"
OUT=$(bash "$PATTERN_CLI" --compile-scan "{number:05d}" "")
assert_eq "scan regex unchanged by width" '^([0-9]+)-' "$OUT"

echo "Test: bare {number} compiles to ^([0-9]+)-"
OUT=$(bash "$PATTERN_CLI" --compile-scan "{number}" "")
assert_eq "scan regex" '^([0-9]+)-' "$OUT"

echo "Test: {project}-{number:04d} with PROJ compiles to ^PROJ-([0-9]+)-"
OUT=$(bash "$PATTERN_CLI" --compile-scan "{project}-{number:04d}" "PROJ")
assert_eq "scan regex" '^PROJ-([0-9]+)-' "$OUT"

echo "Test: {project}-{number:04d} with OTHER compiles to ^OTHER-([0-9]+)-"
OUT=$(bash "$PATTERN_CLI" --compile-scan "{project}-{number:04d}" "OTHER")
assert_eq "scan regex" '^OTHER-([0-9]+)-' "$OUT"

echo "Test: scan regex matches default-pattern filename"
OUT=$(bash "$PATTERN_CLI" --compile-scan "{number:04d}" "")
assert_matches_regex "matches 0042-foo.md" "$OUT" "0042-foo.md"

echo "Test: width-agnostic scan matches over-width filename"
OUT=$(bash "$PATTERN_CLI" --compile-scan "{number:04d}" "")
assert_matches_regex "matches 12345-foo.md" "$OUT" "12345-foo.md"

echo "Test: project-prefixed scan matches PROJ but not OTHER"
OUT=$(bash "$PATTERN_CLI" --compile-scan "{project}-{number:04d}" "PROJ")
assert_matches_regex "matches PROJ-0042-foo.md" "$OUT" "PROJ-0042-foo.md"
assert_not_matches_regex "does not match OTHER-0042-foo.md" "$OUT" "OTHER-0042-foo.md"

echo "Test: escape sequence {{ produces literal { in scan as \\{"
OUT=$(bash "$PATTERN_CLI" --compile-scan "{{a-{number:04d}" "")
# scan regex must escape the literal { to \{; we test that the regex matches a filename starting with {a-
assert_matches_regex "matches {a-0042-foo.md" "$OUT" '{a-0042-foo.md'

echo ""

# ============================================================
echo "=== work-item-pattern.sh --compile-format ==="
echo ""

echo "Test: default {number:04d} compiles to %04d"
OUT=$(bash "$PATTERN_CLI" --compile-format "{number:04d}" "")
assert_eq "format string" '%04d' "$OUT"

echo "Test: bare {number} defaults to %04d"
OUT=$(bash "$PATTERN_CLI" --compile-format "{number}" "")
assert_eq "format string" '%04d' "$OUT"

echo "Test: width variant {number:05d} compiles to %05d"
OUT=$(bash "$PATTERN_CLI" --compile-format "{number:05d}" "")
assert_eq "format string" '%05d' "$OUT"

echo "Test: {project}-{number:04d} with PROJ compiles to PROJ-%04d"
OUT=$(bash "$PATTERN_CLI" --compile-format "{project}-{number:04d}" "PROJ")
assert_eq "format string" 'PROJ-%04d' "$OUT"

echo "Test: escape sequence {{ produces literal { in format"
OUT=$(bash "$PATTERN_CLI" --compile-format "{{a-{number:04d}" "")
assert_eq "format string" '{a-%04d' "$OUT"

echo ""

# ============================================================
echo "=== Round-trip property ==="
echo ""

# For default {number:04d}: format(N) ; parse must recover N for digit-count transitions and width boundary
roundtrip_default() {
  local pattern="$1" project="$2" want="$3"
  local fmt scan formatted recovered
  fmt=$(bash "$PATTERN_CLI" --compile-format "$pattern" "$project")
  scan=$(bash "$PATTERN_CLI" --compile-scan "$pattern" "$project")
  formatted=$(printf "$fmt" "$want")
  # Append a slug-like suffix to mimic a real filename and parse via the scan regex.
  local subject="${formatted}-x.md"
  if [[ "$subject" =~ $scan ]]; then
    recovered="${BASH_REMATCH[1]}"
  else
    recovered="<no-match>"
  fi
  # Strip leading zeros to compare numerically
  local recovered_num=$((10#$recovered))
  assert_eq "round-trip pattern=$pattern project=$project N=$want" "$want" "$recovered_num"
}

for n in 1 9 10 99 100 999 1000 9999; do
  roundtrip_default "{number:04d}" "" "$n"
done

for n in 1 9999 10000 99999; do
  roundtrip_default "{number:05d}" "" "$n"
done

for n in 1 9999; do
  roundtrip_default "{project}-{number:04d}" "A" "$n"
  roundtrip_default "{project}-{number:04d}" "PROJ" "$n"
done

echo ""

# ============================================================
echo "=== Library function tests (sourced) ==="
echo ""

# shellcheck source=/dev/null
source "$COMMON_LIB"

echo "Test: wip_validate_pattern accepts default"
if wip_validate_pattern "{number:04d}" 2>/dev/null; then
  echo "  PASS: wip_validate_pattern returns 0 on valid"
  PASS=$((PASS + 1))
else
  echo "  FAIL: wip_validate_pattern returns 0 on valid"
  FAIL=$((FAIL + 1))
fi

echo "Test: wip_validate_pattern rejects no-number"
if wip_validate_pattern "no-number" 2>/dev/null; then
  echo "  FAIL: wip_validate_pattern should have failed"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: wip_validate_pattern returns non-zero on invalid"
  PASS=$((PASS + 1))
fi

echo "Test: wip_compile_scan returns expected regex"
OUT=$(wip_compile_scan "{project}-{number:04d}" "PROJ")
assert_eq "scan regex" '^PROJ-([0-9]+)-' "$OUT"

echo "Test: wip_compile_format returns expected format"
OUT=$(wip_compile_format "{project}-{number:04d}" "PROJ")
assert_eq "format string" 'PROJ-%04d' "$OUT"

echo "Test: wip_pattern_max_number for {number:04d} is 9999"
OUT=$(wip_pattern_max_number "{number:04d}")
assert_eq "max" "9999" "$OUT"

echo "Test: wip_pattern_max_number for {number:05d} is 99999"
OUT=$(wip_pattern_max_number "{number:05d}")
assert_eq "max" "99999" "$OUT"

echo ""

# ============================================================
echo "=== Project-value validation (rule 5) at use time ==="
echo ""

echo "Test: invalid project value '_low' rejected"
ERR=$(bash "$PATTERN_CLI" --compile-scan "{project}-{number:04d}" "_low" 2>&1 >/dev/null || true)
assert_contains "stderr names rule" "E_PATTERN_BAD_PROJECT_VALUE" "$ERR"
assert_exit_code "exits 2" 2 bash "$PATTERN_CLI" --compile-scan "{project}-{number:04d}" "_low"

echo "Test: project value with internal hyphen rejected"
ERR=$(bash "$PATTERN_CLI" --compile-scan "{project}-{number:04d}" "PROJ-FE" 2>&1 >/dev/null || true)
assert_contains "stderr names rule" "E_PATTERN_BAD_PROJECT_VALUE" "$ERR"

echo "Test: single-char project value 'A' accepted"
OUT=$(bash "$PATTERN_CLI" --compile-scan "{project}-{number:04d}" "A")
assert_eq "scan regex" '^A-([0-9]+)-' "$OUT"

echo "Test: alphanumeric multi-char project value 'ENG2' accepted"
OUT=$(bash "$PATTERN_CLI" --compile-scan "{project}-{number:04d}" "ENG2")
assert_eq "scan regex" '^ENG2-([0-9]+)-' "$OUT"

echo ""

# ============================================================
test_summary
