#!/usr/bin/env bash
set -euo pipefail

# Test harness for work item management companion scripts
# Run: bash skills/work/scripts/test-work-item-scripts.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Shared assertion helpers (assert_eq, assert_exit_code,
# assert_file_executable, assert_stderr_empty, test_summary) plus the
# PASS/FAIL counters. See scripts/test-helpers.sh for the exposed surface.
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

# next-number/resolve-id/sync-baseline/template-field-hints read config through
# the accelerator launcher; point it at the compiled binary.
accelerator_ensure_bin "$PLUGIN_ROOT"

NEXT_NUMBER="$SCRIPT_DIR/work-item-next-number.sh"
READ_STATUS="$SCRIPT_DIR/work-item-read-status.sh"
READ_FIELD="$SCRIPT_DIR/work-item-read-field.sh"

# Temporary-directory scaffolding is local to this harness because
# setup_repo encodes the .git-marker requirement of find_repo_root; it is
# not in test-helpers.sh.
TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$repo_dir/.git"
  echo "$repo_dir"
}

# ============================================================
echo "=== work-item-next-number.sh ==="
echo ""

# Test 1: No meta/work/ directory → outputs 0001
echo "Test: No meta/work/ directory"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" 2>/dev/null)
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 2: Empty meta/work/ directory → outputs 0001
echo "Test: Empty meta/work/ directory"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 3: Directory with 0003-foo.md → outputs 0004
echo "Test: Directory with 0003-foo.md"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0003-foo.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0004" "0004" "$OUTPUT"

# Test 4: Directory with gaps (0001, 0005) → outputs 0006 (uses highest)
echo "Test: Directory with gaps (uses highest)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0001-first.md"
touch "$REPO/meta/work/0005-fifth.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0006" "0006" "$OUTPUT"

# Test 5: Directory with non-work-item files only (README.md) → outputs 0001
echo "Test: Directory with non-work-item files only"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/README.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 6: Mixed work-item and non-work-item files → outputs next after highest work item
echo "Test: Mixed work-item and non-work-item files"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0002-something.md"
touch "$REPO/meta/work/README.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0003" "0003" "$OUTPUT"

# Test 7: --count 3 with highest 0002 → outputs 0003, 0004, 0005
echo "Test: --count 3 with highest 0002"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0002-something.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" --count 3)
EXPECTED=$(printf "0003\n0004\n0005")
assert_eq "outputs 0003, 0004, 0005" "$EXPECTED" "$OUTPUT"

# Test 8: --count 0 (invalid) → exits 1
echo "Test: --count 0 (invalid)"
assert_exit_code "exits 1" 1 bash "$NEXT_NUMBER" --count 0

# Test 9: --count abc (invalid) → exits 1
echo "Test: --count abc (invalid)"
assert_exit_code "exits 1" 1 bash "$NEXT_NUMBER" --count abc

# Test 10: Highest 9999 → exits 1 with "work item number space exhausted" error
echo "Test: Highest 9999 (space exhausted)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/9999-last.md"
RC=0
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" 2>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_eq "no stdout output" "" "$OUTPUT"

# Test 11: Files with 5-digit prefix (00003-foo.md, value 3) — width-agnostic
# scan picks the file up. Highest=3, next=0004 under default {number:04d}.
echo "Test: 5-digit prefix files visible via width-agnostic scan"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/00003-foo.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0004" "0004" "$OUTPUT"

# Test 12: Existing ADR-style files mixed in → ignored, outputs 0001
echo "Test: ADR-style files ignored"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/ADR-0003-something.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 13: --count with no value → exits 1
echo "Test: --count with no value"
assert_exit_code "exits 1" 1 bash "$NEXT_NUMBER" --count

# Test 14: Highest 9998 with --count 2 → outputs 9999 only and exits 1
echo "Test: Highest 9998 with --count 2 (partial overflow)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/9998-second-to-last.md"
RC=0
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" --count 2 2>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_eq "outputs 9999 only" "9999" "$OUTPUT"

# Test 15: Filename without hyphen (0001.md) → glob does not match, outputs 0001
echo "Test: Filename without hyphen ignored"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0001.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Helper to write a {project} pattern config
write_project_config() {
  local repo="$1" project_default="${2-}"
  mkdir -p "$repo/.accelerator"
  if [ -n "$project_default" ]; then
    cat >"$repo/.accelerator/config.md" <<FIXTURE
---
work:
  id_pattern: "{project}-{number:04d}"
  default_project_code: "$project_default"
---
FIXTURE
  else
    cat >"$repo/.accelerator/config.md" <<'FIXTURE'
---
work:
  id_pattern: "{project}-{number:04d}"
---
FIXTURE
  fi
}

echo ""

# ============================================================
echo "=== work-item-next-number.sh (configured pattern) ==="
echo ""

# Per-project scoping
echo "Test: --project PROJ scoping with mixed corpus"
REPO=$(setup_repo)
write_project_config "$REPO" "PROJ"
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/PROJ-0001-x.md"
touch "$REPO/meta/work/PROJ-0003-y.md"
touch "$REPO/meta/work/OTHER-0007-z.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" --project PROJ)
assert_eq "outputs PROJ-0004" "PROJ-0004" "$OUTPUT"

echo "Test: --project OTHER scoping picks OTHER's max"
REPO=$(setup_repo)
write_project_config "$REPO" "PROJ"
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/PROJ-0001-x.md"
touch "$REPO/meta/work/PROJ-0003-y.md"
touch "$REPO/meta/work/OTHER-0007-z.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" --project OTHER)
assert_eq "outputs OTHER-0008" "OTHER-0008" "$OUTPUT"

echo "Test: pattern needs {project}, no flag, no default → error"
REPO=$(setup_repo)
write_project_config "$REPO" ""
mkdir -p "$REPO/meta/work"
RC=0
ERR=$(cd "$REPO" && bash "$NEXT_NUMBER" 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_contains "stderr names rule" "$ERR" "E_PATTERN_MISSING_PROJECT"

echo "Test: default project_code from config when --project absent"
REPO=$(setup_repo)
write_project_config "$REPO" "PROJ"
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/PROJ-0042-y.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs PROJ-0043" "PROJ-0043" "$OUTPUT"

echo "Test: --project on default pattern → error"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
RC=0
ERR=$(cd "$REPO" && bash "$NEXT_NUMBER" --project PROJ 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_contains "stderr names rule" "$ERR" "E_PATTERN_PROJECT_UNUSED"

echo "Test: width change {number:05d} over 0001 corpus → 00002"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat >"$REPO/.accelerator/config.md" <<'FIXTURE'
---
work:
  id_pattern: "{number:05d}"
---
FIXTURE
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0001-foo.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 00002" "00002" "$OUTPUT"

echo "Test: --count 3 with --project PROJ"
REPO=$(setup_repo)
write_project_config "$REPO" "PROJ"
mkdir -p "$REPO/meta/work"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" --project PROJ --count 3)
EXPECTED=$(printf 'PROJ-0001\nPROJ-0002\nPROJ-0003')
assert_eq "outputs three project IDs" "$EXPECTED" "$OUTPUT"

echo "Test: overflow under {number:04d} with 9999 corpus"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/9999-foo.md"
RC=0
ERR=$(cd "$REPO" && bash "$NEXT_NUMBER" 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_contains "stderr names overflow" "$ERR" "E_PATTERN_OVERFLOW"
assert_contains "stderr names highest" "$ERR" "highest=9999"

echo "Test: overflow boundary under {number:05d}, 99998 corpus, --count 1 succeeds"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat >"$REPO/.accelerator/config.md" <<'FIXTURE'
---
work:
  id_pattern: "{number:05d}"
---
FIXTURE
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/99998-foo.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 99999" "99999" "$OUTPUT"

echo "Test: overflow boundary under {number:05d}, 99998 corpus, --count 2 fails"
RC=0
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" --count 2 2>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"

echo "Test: out-of-width legacy 12345 under {number:04d} → overflow"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/12345-foo.md"
RC=0
ERR=$(cd "$REPO" && bash "$NEXT_NUMBER" 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_contains "stderr names out-of-width file" "$ERR" "12345-foo.md"

echo ""

# ============================================================
echo "=== work-item-resolve-id.sh ==="
echo ""

RESOLVE="$SCRIPT_DIR/work-item-resolve-id.sh"

echo "Test: existing path returns absolute path"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0042-foo.md"
OUTPUT=$(cd "$REPO" && bash "$RESOLVE" "meta/work/0042-foo.md")
assert_eq "absolute path" "$REPO/meta/work/0042-foo.md" "$OUTPUT"

echo "Test: missing path exits 3"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
RC=0
(cd "$REPO" && bash "$RESOLVE" "meta/work/nope.md") >/dev/null 2>&1 || RC=$?
assert_eq "exit code 3" "3" "$RC"

echo "Test: full ID PROJ-0042 single match"
REPO=$(setup_repo)
write_project_config "$REPO" "PROJ"
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/PROJ-0042-foo.md"
OUTPUT=$(cd "$REPO" && bash "$RESOLVE" "PROJ-0042")
assert_eq "absolute path" "$REPO/meta/work/PROJ-0042-foo.md" "$OUTPUT"

echo "Test: full ID PROJ-0042 not found exits 3"
REPO=$(setup_repo)
write_project_config "$REPO" "PROJ"
mkdir -p "$REPO/meta/work"
RC=0
(cd "$REPO" && bash "$RESOLVE" "PROJ-0042") >/dev/null 2>&1 || RC=$?
assert_eq "exit code 3" "3" "$RC"

echo "Test: legacy 0042 under default pattern resolves"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0042-legacy.md"
OUTPUT=$(cd "$REPO" && bash "$RESOLVE" "0042")
assert_eq "absolute path" "$REPO/meta/work/0042-legacy.md" "$OUTPUT"

echo "Test: bare 42 (≤4 digits) zero-pads under default project code"
REPO=$(setup_repo)
write_project_config "$REPO" "PROJ"
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/PROJ-0042-foo.md"
OUTPUT=$(cd "$REPO" && bash "$RESOLVE" "42")
assert_eq "absolute path" "$REPO/meta/work/PROJ-0042-foo.md" "$OUTPUT"

echo "Test: ambiguity — legacy + project-prepended"
REPO=$(setup_repo)
write_project_config "$REPO" "PROJ"
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0042-legacy.md"
touch "$REPO/meta/work/PROJ-0042-current.md"
RC=0
ERR=$(cd "$REPO" && bash "$RESOLVE" "0042" 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 2" "2" "$RC"
assert_contains "lists legacy candidate" "$ERR" "[legacy]"
assert_contains "lists project-prepended candidate" "$ERR" "[project-prepended]"

echo "Test: ambiguity — cross-project, no default project code"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat >"$REPO/.accelerator/config.md" <<'FIXTURE'
---
work:
  id_pattern: "{project}-{number:04d}"
---
FIXTURE
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/PROJ-0042-x.md"
touch "$REPO/meta/work/OTHER-0042-y.md"
RC=0
ERR=$(cd "$REPO" && bash "$RESOLVE" "0042" 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 2" "2" "$RC"
assert_contains "lists PROJ" "$ERR" "[PROJ]"
assert_contains "lists OTHER" "$ERR" "[OTHER]"

echo "Test: ambiguity — default project + cross-project (deduplication)"
REPO=$(setup_repo)
write_project_config "$REPO" "PROJ"
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/PROJ-0042-x.md"
touch "$REPO/meta/work/OTHER-0042-y.md"
RC=0
ERR=$(cd "$REPO" && bash "$RESOLVE" "0042" 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 2" "2" "$RC"
assert_contains "lists project-prepended" "$ERR" "[project-prepended]"
assert_contains "lists OTHER" "$ERR" "[OTHER]"
# PROJ-0042-x.md must appear once with project-prepended tag, not twice
PROJ_OCCURRENCES=$(printf '%s\n' "$ERR" | grep -c "PROJ-0042-x.md")
assert_eq "PROJ-0042-x.md listed once" "1" "$PROJ_OCCURRENCES"

echo "Test: single cross-project match resolves"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat >"$REPO/.accelerator/config.md" <<'FIXTURE'
---
work:
  id_pattern: "{project}-{number:04d}"
---
FIXTURE
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/OTHER-0042-y.md"
OUTPUT=$(cd "$REPO" && bash "$RESOLVE" "0042")
assert_eq "absolute path" "$REPO/meta/work/OTHER-0042-y.md" "$OUTPUT"

echo "Test: garbage input invalid exits 1"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
RC=0
(cd "$REPO" && bash "$RESOLVE" "foo bar") >/dev/null 2>&1 || RC=$?
assert_eq "exit code 1" "1" "$RC"

echo "Test: legacy 42 under {project} pattern, no default — finds legacy file"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat >"$REPO/.accelerator/config.md" <<'FIXTURE'
---
work:
  id_pattern: "{project}-{number:04d}"
---
FIXTURE
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0042-legacy.md"
OUTPUT=$(cd "$REPO" && bash "$RESOLVE" "0042")
assert_eq "absolute path" "$REPO/meta/work/0042-legacy.md" "$OUTPUT"

echo ""

# ============================================================
echo "=== work-item-next-number.sh default-pattern golden file ==="
echo ""

GOLDEN_FIXTURE_DIR="$SCRIPT_DIR/test-fixtures"
GOLDEN_FILE="$GOLDEN_FIXTURE_DIR/work-item-next-number.golden"

if [ -f "$GOLDEN_FILE" ]; then
  while IFS= read -r line; do
    case "$line" in
      '' | '#'*) continue ;;
    esac
    SETUP="${line%%|*}"
    REST="${line#*|}"
    ARGS="${REST%%|*}"
    EXPECTED_RAW="${REST#*|}"
    # Replace literal \n with actual newlines in expected
    EXPECTED=$(printf '%b' "$EXPECTED_RAW")

    REPO=$(setup_repo)
    mkdir -p "$REPO/meta/work"
    if [ -n "$SETUP" ]; then
      IFS=',' read -ra FILES <<<"$SETUP"
      for fname in "${FILES[@]}"; do
        touch "$REPO/meta/work/$fname"
      done
    fi
    if [ -z "$ARGS" ]; then
      OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" 2>/dev/null)
    else
      # shellcheck disable=SC2086
      OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" $ARGS 2>/dev/null)
    fi
    assert_eq "golden: setup='$SETUP' args='$ARGS'" "$EXPECTED" "$OUTPUT"
  done <"$GOLDEN_FILE"
else
  echo "  SKIP: golden file not found at $GOLDEN_FILE"
fi

echo ""

# ============================================================
echo "=== Frontmatter consumer integration (quoted work_item_id) ==="
echo ""

# Source the common library so we can call wip_canonicalise_id directly
# shellcheck source=work-item-common.sh
source "$SCRIPT_DIR/work-item-common.sh"

echo "Test: read-field tolerates quoted work_item_id under default config"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat >"$REPO/meta/work/0001-foo.md" <<'FIXTURE'
---
work_item_id: "0001"
title: "Foo"
status: draft
---

# 0001: Foo
FIXTURE
OUTPUT=$(bash "$READ_FIELD" work_item_id "$REPO/meta/work/0001-foo.md")
assert_eq "quoted ID returned unquoted" "0001" "$OUTPUT"

echo "Test: read-field tolerates quoted full ID under {project} pattern"
REPO=$(setup_repo)
write_project_config "$REPO" "PROJ"
mkdir -p "$REPO/meta/work"
cat >"$REPO/meta/work/PROJ-0001-foo.md" <<'FIXTURE'
---
work_item_id: "PROJ-0001"
title: "Foo"
status: draft
---

# PROJ-0001: Foo
FIXTURE
OUTPUT=$(bash "$READ_FIELD" work_item_id "$REPO/meta/work/PROJ-0001-foo.md")
assert_eq "quoted full ID returned unquoted" "PROJ-0001" "$OUTPUT"

echo "Test: wip_is_work_item_file accepts quoted ID files"
if wip_is_work_item_file "$REPO/meta/work/PROJ-0001-foo.md"; then
  echo "  PASS: file recognised as work item"
  PASS=$((PASS + 1))
else
  echo "  FAIL: file should be recognised"
  FAIL=$((FAIL + 1))
fi

echo "Test: wip_is_work_item_file rejects file without work_item_id"
NOWI=$(mktemp "$TMPDIR_BASE/no-wid-XXXXXX.md")
cat >"$NOWI" <<'FIXTURE'
---
title: "Foo"
---
FIXTURE
if wip_is_work_item_file "$NOWI"; then
  echo "  FAIL: file should be rejected"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: file without work_item_id rejected"
  PASS=$((PASS + 1))
fi

echo "Test: wip_canonicalise_id under default pattern"
OUT=$(wip_canonicalise_id "42" "{number:04d}" "")
assert_eq "bare 42 → 0042" "0042" "$OUT"
OUT=$(wip_canonicalise_id "0042" "{number:04d}" "")
assert_eq "0042 → 0042" "0042" "$OUT"
OUT=$(wip_canonicalise_id "\"0042\"" "{number:04d}" "")
assert_eq "quoted 0042 → 0042 (quotes stripped)" "0042" "$OUT"

echo "Test: wip_canonicalise_id under {project} pattern"
OUT=$(wip_canonicalise_id "PROJ-0042" "{project}-{number:04d}" "PROJ")
assert_eq "PROJ-0042 → PROJ-0042" "PROJ-0042" "$OUT"
OUT=$(wip_canonicalise_id "42" "{project}-{number:04d}" "PROJ")
assert_eq "bare 42 + PROJ default → PROJ-0042" "PROJ-0042" "$OUT"
OUT=$(wip_canonicalise_id "\"PROJ-0042\"" "{project}-{number:04d}" "PROJ")
assert_eq "quoted PROJ-0042 → PROJ-0042" "PROJ-0042" "$OUT"

echo "Test: parent comparison via canonicalise — equality"
A=$(wip_canonicalise_id "PROJ-0042" "{project}-{number:04d}" "PROJ")
B=$(wip_canonicalise_id "42" "{project}-{number:04d}" "PROJ")
assert_eq "PROJ-0042 ≡ 42" "$A" "$B"

echo ""

# ============================================================
echo "=== work-item-read-status.sh ==="
echo ""

# Test 1: Valid frontmatter status: draft → outputs "draft"
echo "Test: Valid frontmatter status: draft"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat >"$REPO/meta/work/0001-test.md" <<'FIXTURE'
---
work_item_id: 0001
status: draft
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs draft" "draft" "$OUTPUT"

# Test 2: Valid frontmatter status: ready → outputs "ready"
echo "Test: Valid frontmatter status: ready"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat >"$REPO/meta/work/0001-test.md" <<'FIXTURE'
---
work_item_id: 0001
status: ready
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs ready" "ready" "$OUTPUT"

# Test 3: Quoted value status: "draft" → outputs "draft" (strips quotes)
echo "Test: Quoted value status: \"draft\""
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat >"$REPO/meta/work/0001-test.md" <<'FIXTURE'
---
work_item_id: 0001
status: "draft"
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs draft (strips quotes)" "draft" "$OUTPUT"

# Test 4: No space after colon status:draft → outputs "draft"
echo "Test: No space after colon"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat >"$REPO/meta/work/0001-test.md" <<'FIXTURE'
---
work_item_id: 0001
status:draft
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs draft" "draft" "$OUTPUT"

# Test 5: Trailing whitespace → outputs "draft" (stripped)
echo "Test: Trailing whitespace"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
printf -- '---\nwork_item_id: 0001\nstatus: draft  \n---\n\n# 0001: Test\n' \
  >"$REPO/meta/work/0001-test.md"
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs draft (stripped)" "draft" "$OUTPUT"

# Test 6: Missing file → exits 1
echo "Test: Missing file"
assert_exit_code "exits 1" 1 bash "$READ_STATUS" "/nonexistent/file.md"

# Test 7: File with no frontmatter → exits 1
echo "Test: File with no frontmatter"
REPO=$(setup_repo)
cat >"$REPO/no-frontmatter.md" <<'FIXTURE'
# Just a regular file

No frontmatter here.
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_STATUS" "$REPO/no-frontmatter.md"

# Test 8: Unclosed frontmatter → exits 1
echo "Test: Unclosed frontmatter"
REPO=$(setup_repo)
cat >"$REPO/unclosed.md" <<'FIXTURE'
---
work_item_id: 0001
status: draft
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_STATUS" "$REPO/unclosed.md"

# Test 9: Status in body ignored, frontmatter value returned
echo "Test: Status in body ignored, frontmatter value returned"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat >"$REPO/meta/work/0001-test.md" <<'FIXTURE'
---
work_item_id: 0001
status: draft
---

# 0001: Test

status: ready
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs draft (ignores body)" "draft" "$OUTPUT"

# Test 10: Empty status value → outputs empty string
echo "Test: Empty status value"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
printf -- '---\nwork_item_id: 0001\nstatus: \n---\n\n# 0001: Test\n' \
  >"$REPO/meta/work/0001-test.md"
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs empty string" "" "$OUTPUT"

# Test 11: No arguments → exits 1
echo "Test: No arguments"
assert_exit_code "exits 1" 1 bash "$READ_STATUS"

echo ""

# ============================================================
echo "=== work-item-read-field.sh ==="
echo ""

# Helper: create a standard work-item fixture in a temp repo
make_work_item() {
  local repo="$1"
  mkdir -p "$repo/meta/work"
  cat >"$repo/meta/work/0001-test.md" <<'FIXTURE'
---
work_item_id: 0001
kind: story
priority: high
status: draft
parent: "0001"
tags: [backend, performance]
sub.type: foo
---

# 0001: Test Work Item

kind: epic
FIXTURE
}

# Test 1: Read kind field → outputs "story"
echo "Test: Read kind field"
REPO=$(setup_repo)
make_work_item "$REPO"
OUTPUT=$(bash "$READ_FIELD" kind "$REPO/meta/work/0001-test.md")
assert_eq "outputs story" "story" "$OUTPUT"

# Test 2: Read priority field → outputs "high"
echo "Test: Read priority field"
REPO=$(setup_repo)
make_work_item "$REPO"
OUTPUT=$(bash "$READ_FIELD" priority "$REPO/meta/work/0001-test.md")
assert_eq "outputs high" "high" "$OUTPUT"

# Test 3: Read status field → outputs "draft" (works same as read-status)
echo "Test: Read status field"
REPO=$(setup_repo)
make_work_item "$REPO"
OUTPUT=$(bash "$READ_FIELD" status "$REPO/meta/work/0001-test.md")
assert_eq "outputs draft" "draft" "$OUTPUT"

# Test 4: Read parent field → outputs "0001"
echo "Test: Read parent field"
REPO=$(setup_repo)
make_work_item "$REPO"
OUTPUT=$(bash "$READ_FIELD" parent "$REPO/meta/work/0001-test.md")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 5: Read missing field → exits 1 with error
echo "Test: Read missing field"
REPO=$(setup_repo)
make_work_item "$REPO"
assert_exit_code "exits 1" 1 bash "$READ_FIELD" nonexistent "$REPO/meta/work/0001-test.md"

# Test 6: Quoted field value → strips quotes
echo "Test: Quoted field value strips quotes"
REPO=$(setup_repo)
make_work_item "$REPO"
OUTPUT=$(bash "$READ_FIELD" parent "$REPO/meta/work/0001-test.md")
assert_eq "outputs 0001 (no quotes)" "0001" "$OUTPUT"

# Test 7: Field with array value tags: [a, b] → outputs "[backend, performance]" verbatim
echo "Test: Array field value returned verbatim"
REPO=$(setup_repo)
make_work_item "$REPO"
OUTPUT=$(bash "$READ_FIELD" tags "$REPO/meta/work/0001-test.md")
assert_eq "outputs array verbatim" "[backend, performance]" "$OUTPUT"

# Test 8: Missing file → exits 1
echo "Test: Missing file"
assert_exit_code "exits 1" 1 bash "$READ_FIELD" status "/nonexistent/file.md"

# Test 9: No frontmatter (first line is not ---) → exits 1 with error
echo "Test: No frontmatter"
REPO=$(setup_repo)
cat >"$REPO/no-fm.md" <<'FIXTURE'
# Just markdown

status: draft
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_FIELD" status "$REPO/no-fm.md"

# Test 10: Unclosed frontmatter → exits 1
echo "Test: Unclosed frontmatter"
REPO=$(setup_repo)
cat >"$REPO/unclosed.md" <<'FIXTURE'
---
status: draft
kind: story
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_FIELD" status "$REPO/unclosed.md"

# Test 11: No arguments → exits 1
echo "Test: No arguments"
assert_exit_code "exits 1" 1 bash "$READ_FIELD"

# Test 12: One argument (file only, no field name) → exits 1
echo "Test: One argument only"
assert_exit_code "exits 1" 1 bash "$READ_FIELD" status

# Test 13: Field name in body ignored, frontmatter value returned
echo "Test: Body field ignored"
REPO=$(setup_repo)
make_work_item "$REPO"
OUTPUT=$(bash "$READ_FIELD" kind "$REPO/meta/work/0001-test.md")
assert_eq "outputs story (not epic from body)" "story" "$OUTPUT"

# Test 14: Duplicate key → first-match-wins
echo "Test: Duplicate key first-match-wins"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat >"$REPO/meta/work/0001-dup.md" <<'FIXTURE'
---
status: first
status: second
---
FIXTURE
OUTPUT=$(bash "$READ_FIELD" status "$REPO/meta/work/0001-dup.md")
assert_eq "returns first occurrence" "first" "$OUTPUT"

# Test 15: Prefix-collision (query `tag`, fixture has only `tags:`) → exits 1
echo "Test: Prefix collision does not match"
REPO=$(setup_repo)
make_work_item "$REPO"
assert_exit_code "exits 1" 1 bash "$READ_FIELD" tag "$REPO/meta/work/0001-test.md"

# Test 16a: Literal-match — fixture has `sub.type: foo`, query `sub.type` → outputs "foo"
echo "Test: Dots matched literally (positive)"
REPO=$(setup_repo)
make_work_item "$REPO"
OUTPUT=$(bash "$READ_FIELD" "sub.type" "$REPO/meta/work/0001-test.md")
assert_eq "outputs foo" "foo" "$OUTPUT"

# Test 16b: Negative-match — fixture has `subXtype: foo`, query `sub.type` → exits 1
echo "Test: Dots not treated as regex wildcard (negative)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat >"$REPO/meta/work/0001-nodot.md" <<'FIXTURE'
---
subXtype: foo
---
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_FIELD" "sub.type" "$REPO/meta/work/0001-nodot.md"

# Test 17: Value with trailing whitespace after closing quote → outputs "draft" (no orphan quote)
echo "Test: Trailing whitespace after closing quote"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
printf -- '---\nstatus: "draft"  \n---\n' >"$REPO/meta/work/0001-trailing.md"
OUTPUT=$(bash "$READ_FIELD" status "$REPO/meta/work/0001-trailing.md")
assert_eq "outputs draft (no orphan quote)" "draft" "$OUTPUT"

echo ""

# ============================================================
echo "=== work-item-update-tags.sh ==="
echo ""

UPDATE_TAGS="$SCRIPT_DIR/work-item-update-tags.sh"

# Helper: create a work item with specific tags content
make_tagged_work_item() {
  local repo="$1"
  local tags_line="$2"
  mkdir -p "$repo/meta/work"
  cat >"$repo/meta/work/0001-test.md" <<FIXTURE
---
work_item_id: 0001
status: draft
${tags_line}
---

# 0001: Test Work Item
FIXTURE
}

# Test 1: Add to existing flow-style array
echo "Test: Add to existing flow-style array"
REPO=$(setup_repo)
make_tagged_work_item "$REPO" "tags: [api, search]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add backend)
assert_eq "outputs new array" "[api, search, backend]" "$OUTPUT"

# Test 2: Add duplicate (no-change)
echo "Test: Add duplicate tag"
REPO=$(setup_repo)
make_tagged_work_item "$REPO" "tags: [api, search]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add api)
assert_eq "outputs no-change" "no-change" "$OUTPUT"

# Test 3: Remove existing tag
echo "Test: Remove existing tag"
REPO=$(setup_repo)
make_tagged_work_item "$REPO" "tags: [api, backend, search]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" remove backend)
assert_eq "outputs remaining tags" "[api, search]" "$OUTPUT"

# Test 4: Remove absent tag (no-change)
echo "Test: Remove absent tag"
REPO=$(setup_repo)
make_tagged_work_item "$REPO" "tags: [api, search]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" remove backend)
assert_eq "outputs no-change" "no-change" "$OUTPUT"

# Test 5: Remove last tag → []
echo "Test: Remove last tag yields empty array"
REPO=$(setup_repo)
make_tagged_work_item "$REPO" "tags: [backend]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" remove backend)
assert_eq "outputs empty array" "[]" "$OUTPUT"

# Test 6: Remove from empty [] → no-change
echo "Test: Remove from empty array"
REPO=$(setup_repo)
make_tagged_work_item "$REPO" "tags: []"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" remove backend)
assert_eq "outputs no-change" "no-change" "$OUTPUT"

# Test 7: Add to absent field → [new-tag]
echo "Test: Add to absent tags field"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat >"$REPO/meta/work/0001-test.md" <<'FIXTURE'
---
work_item_id: 0001
status: draft
---

# 0001: Test Work Item
FIXTURE
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add backend)
assert_eq "outputs single-element array" "[backend]" "$OUTPUT"

# Test 8: Add to empty [] → [new-tag]
echo "Test: Add to empty array"
REPO=$(setup_repo)
make_tagged_work_item "$REPO" "tags: []"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add backend)
assert_eq "outputs single-element array" "[backend]" "$OUTPUT"

# Test 9: Block-style detection → exit 1
echo "Test: Block-style tags rejected"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat >"$REPO/meta/work/0001-test.md" <<'FIXTURE'
---
work_item_id: 0001
tags:
  - api
  - search
---

# 0001: Test Work Item
FIXTURE
RC=0
STDERR=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add backend 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
# shellcheck disable=SC2015 # test idiom; the && branch ends in a successful echo so the || cannot spuriously fire
grep -q "block format" <<<"$STDERR" && echo "  PASS: stderr mentions block format" || {
  echo "  FAIL: stderr missing block format message"
  FAIL=$((FAIL + 1))
}

# Test 10: Non-existent file → exit 1
echo "Test: Non-existent file"
RC=0
STDERR=$(bash "$UPDATE_TAGS" "/nonexistent/file.md" add backend 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
# shellcheck disable=SC2015 # test idiom; the && branch ends in a successful echo so the || cannot spuriously fire
grep -q "file not found" <<<"$STDERR" && echo "  PASS: stderr mentions file not found" || {
  echo "  FAIL: stderr missing file not found message"
  FAIL=$((FAIL + 1))
}

# Test 11: Missing frontmatter → exit 1
echo "Test: Missing frontmatter"
REPO=$(setup_repo)
cat >"$REPO/no-fm.md" <<'FIXTURE'
# Just markdown

No frontmatter here.
FIXTURE
RC=0
STDERR=$(bash "$UPDATE_TAGS" "$REPO/no-fm.md" add backend 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"

# Test 12: Unclosed frontmatter → exit 1
echo "Test: Unclosed frontmatter"
REPO=$(setup_repo)
cat >"$REPO/unclosed.md" <<'FIXTURE'
---
work_item_id: 0001
tags: [api]
FIXTURE
RC=0
STDERR=$(bash "$UPDATE_TAGS" "$REPO/unclosed.md" add backend 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"

# Test 13: Tag containing comma is quoted
echo "Test: Tag with comma is quoted"
REPO=$(setup_repo)
make_tagged_work_item "$REPO" "tags: [api]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add "one,two")
assert_eq "outputs quoted tag" '[api, "one,two"]' "$OUTPUT"

# Test 14: Tag containing colon is quoted
echo "Test: Tag with colon is quoted"
REPO=$(setup_repo)
make_tagged_work_item "$REPO" "tags: [api]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add "key:val")
assert_eq "outputs quoted tag" '[api, "key:val"]' "$OUTPUT"

# Test 15: Tag containing hash is quoted
echo "Test: Tag with hash is quoted"
REPO=$(setup_repo)
make_tagged_work_item "$REPO" "tags: [api]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add "tag#1")
assert_eq "outputs quoted tag" '[api, "tag#1"]' "$OUTPUT"

echo ""

# ============================================================
echo "=== work-item-template-field-hints.sh ==="
echo ""

FIELD_HINTS="$SCRIPT_DIR/work-item-template-field-hints.sh"

# Test 1: Field with trailing comment → parsed values (status)
echo "Test: Status field parsed from template comment"
OUTPUT=$(bash "$FIELD_HINTS" status)
EXPECTED=$(printf "draft\nready\nin-progress\nreview\ndone\nblocked\nabandoned")
assert_eq "returns 7 status values" "$EXPECTED" "$OUTPUT"

# Test 2: Kind field parsed from template comment
echo "Test: Kind field parsed from template comment"
OUTPUT=$(bash "$FIELD_HINTS" kind)
EXPECTED=$(printf "story\nepic\ntask\nbug\nspike")
assert_eq "returns 5 kind values" "$EXPECTED" "$OUTPUT"

# Test 3: Priority field parsed from template comment
echo "Test: Priority field parsed from template comment"
OUTPUT=$(bash "$FIELD_HINTS" priority)
EXPECTED=$(printf "high\nmedium\nlow")
assert_eq "returns 3 priority values" "$EXPECTED" "$OUTPUT"

# Test 4: Unknown field with no comment → empty output
echo "Test: Unknown field returns empty output"
OUTPUT=$(bash "$FIELD_HINTS" nonexistent)
assert_eq "returns empty string" "" "$OUTPUT"

# Test 5: User-overridden template with custom values
echo "Test: User-overridden template with custom values"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator/templates"
cat >"$REPO/.accelerator/templates/work-item.md" <<'FIXTURE'
---
work_item_id: NNNN
status: open                                   # open | closed | wontfix
category: feature                              # feature | defect
priority: p1                                   # p1 | p2 | p3 | p4
---

# NNNN: Title
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$FIELD_HINTS" status)
EXPECTED=$(printf "open\nclosed\nwontfix")
assert_eq "returns custom status values" "$EXPECTED" "$OUTPUT"

# Test 6: config-read-template failure → hardcoded fallback for known fields
echo "Test: Template read failure falls back to hardcoded defaults"
# Create a repo with no template at all and set PLUGIN_ROOT to a nonexistent
# plugin to force config-read-template.sh to fail
REPO=$(setup_repo)
# Override PLUGIN_ROOT to simulate failure — run in subshell with modified env
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="/nonexistent/plugin" bash "$FIELD_HINTS" status 2>/dev/null) || true
EXPECTED=$(printf "draft\nready\nin-progress\nreview\ndone\nblocked\nabandoned")
assert_eq "returns hardcoded status values" "$EXPECTED" "$OUTPUT"

# Test 7: Field with no trailing comment → hardcoded fallback
echo "Test: Field with no comment falls back to hardcoded"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator/templates"
cat >"$REPO/.accelerator/templates/work-item.md" <<'FIXTURE'
---
work_item_id: NNNN
status: draft
kind: story
priority: medium
---

# NNNN: Title
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$FIELD_HINTS" status)
EXPECTED=$(printf "draft\nready\nin-progress\nreview\ndone\nblocked\nabandoned")
assert_eq "returns hardcoded status fallback" "$EXPECTED" "$OUTPUT"

# Test 8: Tripwire — hardcoded fallback values match shipping template's comments
echo "Test: Tripwire — hardcoded fallbacks match shipping template"
# Parse shipping template status comment directly
SHIPPING_TEMPLATE="$PLUGIN_ROOT/templates/work-item.md"
STATUS_LINE=$(grep "^status:" "$SHIPPING_TEMPLATE")
STATUS_COMMENT="${STATUS_LINE#*#}"
SHIPPING_VALUES=""
while IFS= read -r token; do
  token=$(echo "$token" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  [ -n "$token" ] && SHIPPING_VALUES="${SHIPPING_VALUES}${SHIPPING_VALUES:+$'\n'}${token}"
done < <(echo "$STATUS_COMMENT" | tr '|' '\n')
HARDCODED_VALUES=$(cd /tmp && CLAUDE_PLUGIN_ROOT="/nonexistent" bash "$FIELD_HINTS" status 2>/dev/null) || true
assert_eq "hardcoded status matches shipping template" "$SHIPPING_VALUES" "$HARDCODED_VALUES"

KIND_LINE=$(grep "^kind:" "$SHIPPING_TEMPLATE")
KIND_COMMENT="${KIND_LINE#*#}"
SHIPPING_VALUES=""
while IFS= read -r token; do
  token=$(echo "$token" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  [ -n "$token" ] && SHIPPING_VALUES="${SHIPPING_VALUES}${SHIPPING_VALUES:+$'\n'}${token}"
done < <(echo "$KIND_COMMENT" | tr '|' '\n')
HARDCODED_VALUES=$(cd /tmp && CLAUDE_PLUGIN_ROOT="/nonexistent" bash "$FIELD_HINTS" kind 2>/dev/null) || true
assert_eq "hardcoded kind matches shipping template" "$SHIPPING_VALUES" "$HARDCODED_VALUES"

PRIORITY_LINE=$(grep "^priority:" "$SHIPPING_TEMPLATE")
PRIORITY_COMMENT="${PRIORITY_LINE#*#}"
SHIPPING_VALUES=""
while IFS= read -r token; do
  token=$(echo "$token" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  [ -n "$token" ] && SHIPPING_VALUES="${SHIPPING_VALUES}${SHIPPING_VALUES:+$'\n'}${token}"
done < <(echo "$PRIORITY_COMMENT" | tr '|' '\n')
HARDCODED_VALUES=$(cd /tmp && CLAUDE_PLUGIN_ROOT="/nonexistent" bash "$FIELD_HINTS" priority 2>/dev/null) || true
assert_eq "hardcoded priority matches shipping template" "$SHIPPING_VALUES" "$HARDCODED_VALUES"

echo ""

# ============================================================
echo "=== work-item-sync-label.sh ==="
echo ""

SYNC_LABEL="$SCRIPT_DIR/work-item-sync-label.sh"

# Classification is presence-based: a non-empty external_id (after stripping
# surrounding quotes + whitespace) is synced, everything else unsynced.
echo "Test: classify non-empty external_id → synced"
assert_eq "synced" "synced" "$(bash "$SYNC_LABEL" --classify 'PROJ-0042')"

echo "Test: classify project-coded id-shape value → synced (independent of id shape)"
assert_eq "synced" "synced" "$(bash "$SYNC_LABEL" --classify 'BLA-123')"

echo "Test: classify github-style external_id → synced"
assert_eq "synced" "synced" "$(bash "$SYNC_LABEL" --classify 'atomic-innovation/accelerator#42')"

echo "Test: classify absent (empty) external_id → unsynced"
assert_eq "unsynced" "unsynced" "$(bash "$SYNC_LABEL" --classify '')"

echo "Test: classify quote-only \"\" → unsynced (normalisation strips quotes)"
assert_eq "unsynced" "unsynced" "$(bash "$SYNC_LABEL" --classify '""')"

echo "Test: classify whitespace-only → unsynced"
assert_eq "unsynced" "unsynced" "$(bash "$SYNC_LABEL" --classify '   ')"

echo "Test: classify quoted value → synced (quotes stripped, value remains)"
assert_eq "synced" "synced" "$(bash "$SYNC_LABEL" --classify '"PROJ-0042"')"

echo "Test: label synced → glyph + text"
assert_eq "synced label" "🟢 synced" "$(bash "$SYNC_LABEL" --label synced)"

echo "Test: label unsynced → glyph + text"
assert_eq "unsynced label" "⚪ unsynced" "$(bash "$SYNC_LABEL" --label unsynced)"

# All FIVE states must differ pairwise in BOTH glyph and text so the signal
# survives monochrome / glyph-blind rendering.
echo "Test: all five sync labels are pairwise distinct in glyph AND text"
FIVE_STATES="synced unsynced locally-modified remotely-modified conflict"
DISTINCT_OK=1
ALL_LABELS=""
for _s1 in $FIVE_STATES; do
  _l1=$(bash "$SYNC_LABEL" --label "$_s1")
  ALL_LABELS="$ALL_LABELS$_l1"
  for _s2 in $FIVE_STATES; do
    [ "$_s1" = "$_s2" ] && continue
    _l2=$(bash "$SYNC_LABEL" --label "$_s2")
    _g1="${_l1%% *}"
    _g2="${_l2%% *}"
    _t1="${_l1#* }"
    _t2="${_l2#* }"
    if [ "$_g1" = "$_g2" ] || [ "$_t1" = "$_t2" ]; then
      echo "  detail: '$_s1' ($_l1) collides with '$_s2' ($_l2)"
      DISTINCT_OK=0
    fi
  done
done
if [ "$DISTINCT_OK" -eq 1 ]; then
  echo "  PASS: all five labels pairwise-distinct in glyph and text"
  PASS=$((PASS + 1))
else
  echo "  FAIL: a label pair shares a glyph or text"
  FAIL=$((FAIL + 1))
fi

# Labels must be markdown-native, never ANSI escapes (output is a markdown
# table in the conversation, not a TTY). Covers all five.
echo "Test: labels emit no ANSI escape sequences"
if grep -q $'\033' <<<"$ALL_LABELS"; then
  echo "  FAIL: ANSI escape sequence present in label output"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: no ANSI escapes"
  PASS=$((PASS + 1))
fi

echo "Test: default mode classifies then renders (external_id → label)"
assert_eq "synced label" "🟢 synced" "$(bash "$SYNC_LABEL" 'PROJ-0042')"
assert_eq "unsynced label" "⚪ unsynced" "$(bash "$SYNC_LABEL" '')"

echo "Test: unknown status → exit 1"
assert_exit_code "exits 1" 1 bash "$SYNC_LABEL" --label bogus

echo ""

# ============================================================
echo "=== work-item-normalise.sh ==="
echo ""

NORMALISE="$SCRIPT_DIR/work-item-normalise.sh"
# shellcheck source=scripts/hash-common.sh
source "$PLUGIN_ROOT/scripts/hash-common.sh"

nhash() { bash "$NORMALISE" "$1" | hash_sha256_stdin; }

# A baseline work item carrying the provenance/identity fields the normaliser
# drops (the fixed IGNORE_KEYS denylist from the plan's Decisions Locked #3).
write_item() {
  cat >"$1" <<'ITEM'
---
id: "0042"
external_id: "ENG-7"
title: "Do the thing"
kind: story
status: ready
priority: medium
last_updated: "2026-06-10T00:00:00+00:00"
last_updated_by: Toby Clemson
revision: "abc123"
---

# 0042: Do the thing

## Summary

Implement the thing carefully.
ITEM
}

BASE_WI="$TMPDIR_BASE/wi-base.md"
write_item "$BASE_WI"
BASE_WI_HASH=$(nhash "$BASE_WI")

echo "Test: trailing whitespace / trailing newlines do not change the hash"
WS="$TMPDIR_BASE/wi-ws.md"
write_item "$WS"
perl -pi -e 's/$/   /' "$WS"
printf '\n\n\n' >>"$WS"
assert_eq "whitespace-only delta → identical hash" "$BASE_WI_HASH" "$(nhash "$WS")"

echo "Test: bumping last_updated / last_updated_by → identical hash"
LU="$TMPDIR_BASE/wi-lu.md"
write_item "$LU"
perl -pi -e 's/^last_updated: .*/last_updated: "2026-12-31T23:59:59+00:00"/' "$LU"
perl -pi -e 's/^last_updated_by: .*/last_updated_by: Someone Else/' "$LU"
assert_eq "restamped last_updated → identical hash" "$BASE_WI_HASH" "$(nhash "$LU")"

echo "Test: bumping revision → identical hash"
RV="$TMPDIR_BASE/wi-rev.md"
write_item "$RV"
perl -pi -e 's/^revision: .*/revision: "deadbeef"/' "$RV"
assert_eq "restamped revision → identical hash" "$BASE_WI_HASH" "$(nhash "$RV")"

echo "Test: changing external_id or id → identical hash (ignored)"
EX="$TMPDIR_BASE/wi-ex.md"
write_item "$EX"
perl -pi -e 's/^external_id: .*/external_id: "ENG-999"/' "$EX"
perl -pi -e 's/^id: .*/id: "0099"/' "$EX"
assert_eq "changed id/external_id → identical hash" "$BASE_WI_HASH" "$(nhash "$EX")"

echo "Test: a real Summary edit → different hash"
ED="$TMPDIR_BASE/wi-ed.md"
write_item "$ED"
perl -pi -e 's/Implement the thing carefully\./Implement the thing very differently./' "$ED"
assert_neq "edited Summary → different hash" "$BASE_WI_HASH" "$(nhash "$ED")"

echo "Test: determinism — same input twice → same digest"
assert_eq "stable across runs" "$(nhash "$BASE_WI")" "$(nhash "$BASE_WI")"

echo "Test: determinism — non-C caller locale → same digest"
LOCALE_HASH=$(LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 bash "$NORMALISE" "$BASE_WI" |
  hash_sha256_stdin)
assert_eq "locale-independent digest" "$BASE_WI_HASH" "$LOCALE_HASH"

echo "Test: remote projection — reordered JSON keys canonicalise to one digest"
J1='{"type":"doc","version":1,"content":[{"type":"paragraph","text":"hi"}]}'
J2='{"version":1,"content":[{"text":"hi","type":"paragraph"}],"type":"doc"}'
D1=$(printf '%s' "$J1" | jq -S . | bash "$NORMALISE" --stdin | hash_sha256_stdin)
D2=$(printf '%s' "$J2" | jq -S . | bash "$NORMALISE" --stdin | hash_sha256_stdin)
assert_eq "jq -S canonicalised projection is order-independent" "$D1" "$D2"

echo ""

# ============================================================
echo "=== work-item-sync-baseline.sh ==="
echo ""

BASELINE="$SCRIPT_DIR/work-item-sync-baseline.sh"

setup_baseline_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/bl-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator"
  cat >"$d/.accelerator/config.md" <<'CFG'
---
work:
  integration: jira
---
CFG
  echo "$d"
}

baseline() {
  local repo="$1"
  shift
  (cd "$repo" && bash "$BASELINE" "$@")
}

echo "Test: path inserts the <system>/ segment under paths.integrations"
BREPO=$(setup_baseline_repo)
BPATH=$(baseline "$BREPO" path)
assert_eq "path ends with jira/last-sync.json" \
  ".accelerator/state/integrations/jira/last-sync.json" \
  "${BPATH#"$BREPO"/}"

echo "Test: reading a non-existent baseline yields empty, not an error"
RC=0
OUT=$(baseline "$BREPO" get 0042) || RC=$?
assert_eq "get on missing file exits 0" "0" "$RC"
assert_eq "get on missing file is empty" "" "$OUT"

echo "Test: set then get round-trips an entry including remote_hash"
baseline "$BREPO" set 0042 "2026-06-01T10:00:00.000+0000" "rh-abc" "lh-xyz"
ENTRY=$(baseline "$BREPO" get 0042)
assert_eq "remote_updated_at round-trips" "2026-06-01T10:00:00.000+0000" \
  "$(printf '%s' "$ENTRY" | jq -r '.remote_updated_at')"
assert_eq "remote_hash round-trips" "rh-abc" \
  "$(printf '%s' "$ENTRY" | jq -r '.remote_hash')"
assert_eq "local_hash round-trips" "lh-xyz" \
  "$(printf '%s' "$ENTRY" | jq -r '.local_hash')"

echo "Test: baseline file is valid JSON"
BFILE=$(baseline "$BREPO" path)
assert_exit_code "jq empty parses the baseline" 0 jq empty "$BFILE"

echo "Test: set is idempotent (second identical set → no content change)"
BEFORE=$(cat "$BFILE")
baseline "$BREPO" set 0042 "2026-06-01T10:00:00.000+0000" "rh-abc" "lh-xyz"
AFTER=$(cat "$BFILE")
assert_eq "identical set leaves content unchanged" "$BEFORE" "$AFTER"

echo "Test: set-timestamp records the global epoch reference"
baseline "$BREPO" set-timestamp 1750000000
assert_eq "timestamp stored as integer epoch" "1750000000" \
  "$(jq -r '.timestamp' "$BFILE")"

echo "Test: remove deletes one entry leaving others"
baseline "$BREPO" set 0043 "2026-06-02T00:00:00.000+0000" "rh2" "lh2"
baseline "$BREPO" remove 0042
assert_eq "0042 removed" "" "$(baseline "$BREPO" get 0042)"
assert_eq "0043 retained" "lh2" \
  "$(baseline "$BREPO" get 0043 | jq -r '.local_hash')"

echo "Test: present-but-unparseable (conflict-markered) file → empty, never error"
CREPO=$(setup_baseline_repo)
CFILE=$(baseline "$CREPO" path)
mkdir -p "$(dirname "$CFILE")"
cat >"$CFILE" <<'CONFLICT'
<<<<<<< HEAD
{"timestamp": 1, "items": {}}
=======
{"timestamp": 2, "items": {}}
>>>>>>> branch
CONFLICT
RC=0
OUT=$(baseline "$CREPO" get 0042) || RC=$?
assert_eq "get on conflict-markered file exits 0" "0" "$RC"
assert_eq "get on conflict-markered file is empty" "" "$OUT"

echo "Test: crash-safety — set leaves no partial temp and a parseable file"
SREPO=$(setup_baseline_repo)
baseline "$SREPO" set 0001 "2026-06-01T00:00:00.000+0000" "rh" "lh"
SFILE=$(baseline "$SREPO" path)
SDIR=$(dirname "$SFILE")
LEFTOVER=$(find "$SDIR" -name '.atomic-write.*' 2>/dev/null | wc -l | tr -d ' ')
assert_eq "no atomic-write temp file survives a completed write" "0" "$LEFTOVER"
assert_exit_code "post-write file still parses" 0 jq empty "$SFILE"
# Structural: mutations route through atomic_write (same-dir temp + mv).
# shellcheck disable=SC2016  # grepping for the literal call, not expanding it
if grep -q 'atomic_write "$f"' "$BASELINE"; then
  echo "  PASS: baseline writes go through atomic_write"
  PASS=$((PASS + 1))
else
  echo "  FAIL: baseline writes do not use atomic_write"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== work-item-sync-label.sh — baseline-dependent label arms ==="
echo ""
assert_eq "locally-modified label" "🔵 locally modified" \
  "$(bash "$SYNC_LABEL" --label locally-modified)"
assert_eq "remotely-modified label" "🟣 remotely modified" \
  "$(bash "$SYNC_LABEL" --label remotely-modified)"
assert_eq "conflict label" "🔴 conflict" "$(bash "$SYNC_LABEL" --label conflict)"

echo ""

# ============================================================
echo "=== work-item-sync-classify.sh — change-detection engine ==="
echo ""

CLASSIFY="$SCRIPT_DIR/work-item-sync-classify.sh"

# Fixtures: a tracked local item, a baseline that matches it, and a remote body.
EFILE="$TMPDIR_BASE/eng-item.md"
write_item "$EFILE"
E_LOCAL_HASH=$(nhash "$EFILE")
R_UPDATED="2026-06-01T10:00:00.000+0000"
RBODY="$TMPDIR_BASE/eng-remote.md"
printf '# Do the thing\n\nImplement the thing carefully.\n' >"$RBODY"
E_REMOTE_HASH=$(bash "$NORMALISE" --stdin <"$RBODY" | hash_sha256_stdin)
ENTRY=$(jq -cn --arg lh "$E_LOCAL_HASH" --arg rh "$E_REMOTE_HASH" --arg ru "$R_UPDATED" \
  '{remote_updated_at: $ru, remote_hash: $rh, local_hash: $lh}')

classify() { bash "$CLASSIFY" "$@"; }

echo "Test: neither side changed → synced (remote updated-equality short-circuit)"
assert_eq "synced" "synced" \
  "$(classify --file "$EFILE" --external-id ENG-7 --baseline "$ENTRY" \
    --timestamp 0 --remote-status present --remote-updated "$R_UPDATED")"

echo "Test: local edited, remote unchanged → locally-modified"
EFILE_ED="$TMPDIR_BASE/eng-item-ed.md"
write_item "$EFILE_ED"
perl -pi -e 's/Implement the thing carefully\./Locally rewritten./' "$EFILE_ED"
assert_eq "locally-modified" "locally-modified" \
  "$(classify --file "$EFILE_ED" --external-id ENG-7 --baseline "$ENTRY" \
    --timestamp 0 --remote-status present --remote-updated "$R_UPDATED")"

echo "Test: remote edited (updated differs + body differs), local unchanged → remotely-modified"
RBODY2="$TMPDIR_BASE/eng-remote2.md"
printf '# Do the thing\n\nRemotely rewritten.\n' >"$RBODY2"
assert_eq "remotely-modified" "remotely-modified" \
  "$(classify --file "$EFILE" --external-id ENG-7 --baseline "$ENTRY" \
    --timestamp 0 --remote-status present --remote-updated "2026-12-01T00:00:00.000+0000" \
    --remote-body-file "$RBODY2")"

echo "Test: both sides changed → conflict"
assert_eq "conflict" "conflict" \
  "$(classify --file "$EFILE_ED" --external-id ENG-7 --baseline "$ENTRY" \
    --timestamp 0 --remote-status present --remote-updated "2026-12-01T00:00:00.000+0000" \
    --remote-body-file "$RBODY2")"

echo "Test: remote updated EQUAL → unchanged without a body (trusted short-circuit)"
# No --remote-body-file supplied; equality alone resolves the remote side.
assert_eq "synced (no body fetched)" "synced" \
  "$(classify --file "$EFILE" --external-id ENG-7 --baseline "$ENTRY" \
    --timestamp 0 --remote-status present --remote-updated "$R_UPDATED")"

echo "Test: remote body matches baseline hash despite a ticked updated → synced"
assert_eq "label/transition-only remote tick → synced" "synced" \
  "$(classify --file "$EFILE" --external-id ENG-7 --baseline "$ENTRY" \
    --timestamp 0 --remote-status present --remote-updated "2026-12-01T00:00:00.000+0000" \
    --remote-body-file "$RBODY")"

echo "Test: whitespace-only local + updated-only remote delta → synced (AC)"
EFILE_WS="$TMPDIR_BASE/eng-item-ws.md"
write_item "$EFILE_WS"
perl -pi -e 's/$/   /' "$EFILE_WS"
assert_eq "whitespace-equivalent local stays synced" "synced" \
  "$(classify --file "$EFILE_WS" --external-id ENG-7 --baseline "$ENTRY" \
    --timestamp 0 --remote-status present --remote-updated "$R_UPDATED")"

echo "Test: mtime pre-filter short-circuits to unchanged (pure integer compare)"
# Edited content, but mtime ≤ timestamp → advisory short-circuit declares the
# local side unchanged without hashing.
E_MTIME=$(stat -f %m "$EFILE_ED" 2>/dev/null) ||
  E_MTIME=$(stat -c %Y "$EFILE_ED")
TS_FUTURE=$((E_MTIME + 100000))
assert_eq "old mtime ≤ timestamp → local unchanged → synced" "synced" \
  "$(classify --file "$EFILE_ED" --external-id ENG-7 --baseline "$ENTRY" \
    --timestamp "$TS_FUTURE" --remote-status present --remote-updated "$R_UPDATED")"

echo "Test: no external_id → presence-only (unsynced), even with a baseline entry"
assert_eq "unsynced (5th branch)" "unsynced" \
  "$(classify --file "$EFILE" --external-id "" --baseline "$ENTRY" \
    --timestamp 0 --remote-status present --remote-updated "$R_UPDATED")"

echo "Test: tracked but absent from a successful fetch → remote-absent"
assert_eq "remote-absent" "remote-absent" \
  "$(classify --file "$EFILE" --external-id ENG-7 --baseline "$ENTRY" \
    --timestamp 0 --remote-status absent)"

echo "Test: failed/timed-out remote read → indeterminate (distinct from absent)"
assert_eq "indeterminate" "indeterminate" \
  "$(classify --file "$EFILE" --external-id ENG-7 --baseline "$ENTRY" \
    --timestamp 0 --remote-status indeterminate)"

echo "Test: first-sync (external_id, no baseline) both-ahead → conflict, not synced"
assert_eq "first-sync full contract → conflict" "conflict" \
  "$(classify --file "$EFILE" --external-id ENG-7 --baseline "" \
    --timestamp 0 --remote-status present --remote-updated "$R_UPDATED" \
    --remote-body-file "$RBODY2")"

echo ""

# ============================================================
echo "=== work-item-sync-decide.sh — (mode × state) decision table ==="
echo ""

DECIDE="$SCRIPT_DIR/work-item-sync-decide.sh"
dec() { bash "$DECIDE" decide --mode "$1" --state "$2" ${3:+--dirty "$3"}; }

echo "Test: mode resolution and the mutually-exclusive guard"
assert_eq "no flags → bidirectional" "bidirectional" "$(bash "$DECIDE" mode)"
assert_eq "--push-only" "push-only" "$(bash "$DECIDE" mode --push-only)"
assert_eq "--pull-only" "pull-only" "$(bash "$DECIDE" mode --pull-only)"
assert_exit_code "--push-only + --pull-only → error" 2 \
  bash "$DECIDE" mode --push-only --pull-only

echo "Test: synced/unsynced/indeterminate/remote-absent → noop in every mode"
for m in bidirectional push-only pull-only; do
  for s in synced unsynced indeterminate remote-absent; do
    assert_eq "$m/$s → noop" "noop" "$(dec "$m" "$s")"
  done
done

echo "Test: locally-modified pushes except under --pull-only (forbidden write)"
assert_eq "bidi local-ahead → push" "push" "$(dec bidirectional locally-modified)"
assert_eq "push-only local-ahead → push" "push" "$(dec push-only locally-modified)"
assert_eq "pull-only local-ahead → noop (no push)" "noop" "$(dec pull-only locally-modified)"

echo "Test: remotely-modified pulls except under --push-only; dirty routes safely"
assert_eq "bidi remote-ahead clean → pull" "pull" "$(dec bidirectional remotely-modified 0)"
assert_eq "pull-only remote-ahead clean → pull" "pull" "$(dec pull-only remotely-modified 0)"
assert_eq "push-only remote-ahead → noop (no pull)" "noop" "$(dec push-only remotely-modified 0)"
assert_eq "bidi remote-ahead dirty → prompt" "prompt" "$(dec bidirectional remotely-modified 1)"
assert_eq "pull-only remote-ahead dirty → skip-dirty" "skip-dirty" "$(dec pull-only remotely-modified 1)"

echo "Test: conflict prompts in bidirectional, reports+skips in directional modes"
assert_eq "bidi conflict → prompt" "prompt" "$(dec bidirectional conflict)"
assert_eq "push-only conflict → skip-conflict" "skip-conflict" "$(dec push-only conflict)"
assert_eq "pull-only conflict → skip-conflict" "skip-conflict" "$(dec pull-only conflict)"

echo "Test: resolve-conflict-token maps the destructive choice safely"
assert_eq "remote → accept-remote" "accept-remote" \
  "$(bash "$DECIDE" resolve-conflict-token '  REMOTE ')"
assert_eq "local → push-local" "push-local" \
  "$(bash "$DECIDE" resolve-conflict-token local)"
assert_eq "skip → skip" "skip" "$(bash "$DECIDE" resolve-conflict-token skip)"
assert_eq "empty → skip (never destructive)" "skip" \
  "$(bash "$DECIDE" resolve-conflict-token '')"
assert_eq "unrecognised → skip (never destructive)" "skip" \
  "$(bash "$DECIDE" resolve-conflict-token frobnicate)"

echo ""

# ============================================================
echo "=== work-item-file-dirty.sh — VCS-mode-aware overwrite guard ==="
echo ""

FILE_DIRTY="$SCRIPT_DIR/work-item-file-dirty.sh"
dirty_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/fd-XXXXXX")
  mkdir -p "$d/.git" "$d/meta/work"
  touch "$d/meta/work/0001-x.md"
  echo "$d"
}
fd_check() {
  # fd_check <repo> <mode> <status> ; echoes exit code
  local repo="$1" mode="$2" status="$3" rc=0
  (cd "$repo" && ACCELERATOR_TEST_MODE=1 WORK_DIRTY_MODE_OVERRIDE="$mode" \
    WORK_DIRTY_STATUS_OVERRIDE="$status" \
    bash "$FILE_DIRTY" "$repo/meta/work/0001-x.md") || rc=$?
  echo "$rc"
}
FDREPO=$(dirty_repo)
assert_eq "git porcelain non-empty → dirty (0)" "0" \
  "$(fd_check "$FDREPO" git ' M meta/work/0001-x.md')"
assert_eq "git porcelain empty → clean (1)" "1" "$(fd_check "$FDREPO" git '')"
assert_eq "git untracked ?? → dirty (0)" "0" \
  "$(fd_check "$FDREPO" git '?? meta/work/0001-x.md')"
assert_eq "jj path in diff → dirty (0)" "0" \
  "$(fd_check "$FDREPO" jj 'meta/work/0001-x.md')"
assert_eq "jj path absent from diff → clean (1)" "1" \
  "$(fd_check "$FDREPO" jj 'meta/work/other.md')"
# jj-colocated: a repo with BOTH .jj and .git resolves to the jj arm (never git).
COLO=$(dirty_repo)
mkdir -p "$COLO/.jj"
assert_eq "jj-colocated resolves to jj (clean diff → clean)" "1" \
  "$(cd "$COLO" && ACCELERATOR_TEST_MODE=1 WORK_DIRTY_STATUS_OVERRIDE='meta/work/other.md' \
    bash "$FILE_DIRTY" "$COLO/meta/work/0001-x.md" >/dev/null 2>&1 && echo 0 || echo 1)"
assert_eq "indeterminate VCS mode → fail-safe dirty (0)" "0" \
  "$(fd_check "$FDREPO" none '')"

# End-to-end (no override): in a REAL git linked worktree the find_repo_root →
# vcs_mode → dispatch chain must report a committed work-item file as CLEAN
# (exit 1) and a modified one as DIRTY (exit 0). Pre-fix vcs_mode returns 'none'
# in a worktree (.git is a file → the -d test fails) → fail-safe-to-dirty → the
# clean case wrongly returns exit 0. The override-driven cases above bypass the
# real vcs_mode(); this is the only coverage that exercises it. (Capture the
# exit code immediately — the guard runs under this suite's `set -e`.)
if command -v git >/dev/null 2>&1; then
  WT_PARENT=$(mktemp -d "$TMPDIR_BASE/wtp-XXXXXX")
  (cd "$WT_PARENT" && git init -q && git config user.email t@e.x &&
    git config user.name T && git commit --allow-empty -q -m init)
  WT=$(mktemp -d "$TMPDIR_BASE/wt-XXXXXX")
  rm -rf "$WT"
  (cd "$WT_PARENT" && git worktree add -q "$WT")
  printf 'original\n' >"$WT/item.md"
  (cd "$WT" && git add item.md && git commit -q -m "add item")
  rc=0
  (cd "$WT" && bash "$FILE_DIRTY" "$WT/item.md") || rc=$?
  assert_eq "worktree committed file → clean (1)" "1" "$rc"
  printf 'changed\n' >>"$WT/item.md"
  rc=0
  (cd "$WT" && bash "$FILE_DIRTY" "$WT/item.md") || rc=$?
  assert_eq "worktree modified file → dirty (0)" "0" "$rc"
else
  echo "  SKIP: git not on PATH — worktree end-to-end dirty check"
fi

echo ""

# ============================================================
echo "=== work-item-project-remote.sh — per-tracker projection seam ==="
echo ""

PROJECT="$SCRIPT_DIR/work-item-project-remote.sh"
JSHOW='{"key":"ENG-1","fields":{"summary":"Hi","description":{"type":"doc","b":1,"a":2},"updated":"2026-06-01T10:00:00.000+0000"}}'
LSHOW='{"data":{"issue":{"identifier":"BLA-1","title":"Hi","updatedAt":"2026-06-02T11:00:00.000Z","description":"Body **md**."}}}'
assert_eq "jira updated" "2026-06-01T10:00:00.000+0000" \
  "$(printf '%s' "$JSHOW" | bash "$PROJECT" --integration jira updated)"
assert_eq "linear updated" "2026-06-02T11:00:00.000Z" \
  "$(printf '%s' "$LSHOW" | bash "$PROJECT" --integration linear updated)"
# jira body canonicalises the ADF keys (jq -S), so reordered ADF hashes the same.
JSHOW2='{"key":"ENG-1","fields":{"summary":"Hi","description":{"a":2,"type":"doc","b":1},"updated":"x"}}'
PB1=$(printf '%s' "$JSHOW" | bash "$PROJECT" --integration jira body | bash "$NORMALISE" --stdin | hash_sha256_stdin)
PB2=$(printf '%s' "$JSHOW2" | bash "$PROJECT" --integration jira body | bash "$NORMALISE" --stdin | hash_sha256_stdin)
assert_eq "jira body canonicalisation is key-order-independent" "$PB1" "$PB2"
assert_contains "linear body carries the Markdown description" \
  "$(printf '%s' "$LSHOW" | bash "$PROJECT" --integration linear body)" "Body **md**."

echo ""

# ============================================================
echo "=== work-item-sync-apply.sh — pull + finalise + resumability ==="
echo ""

APPLY="$SCRIPT_DIR/work-item-sync-apply.sh"
CLASSIFY2="$SCRIPT_DIR/work-item-sync-classify.sh"

# A repo with config (work.integration: jira) so the baseline path resolves.
AREPO=$(setup_baseline_repo)
mkdir -p "$AREPO/meta/work"
LOCALFILE="$AREPO/meta/work/0050-x.md"
write_item "$LOCALFILE"
# Reconstructed post-pull content (what the SKILL would write: local frontmatter
# kept, title/body from remote).
NEWCONTENT="$TMPDIR_BASE/apply-new.md"
write_item "$NEWCONTENT"
perl -pi -e 's/Implement the thing carefully\./Pulled from remote./' "$NEWCONTENT"
# Projected, canonicalised remote body the pull wrote.
REMBODY="$TMPDIR_BASE/apply-rembody.md"
printf '# Do the thing\n\nPulled from remote.\n' >"$REMBODY"
A_RUPDATED="2026-07-01T09:00:00.000+0000"

echo "Test: apply pull overwrites the file and sets the post-overwrite baseline"
(cd "$AREPO" && bash "$APPLY" pull --id 0050 --file "$LOCALFILE" \
  --new-content-file "$NEWCONTENT" --remote-updated "$A_RUPDATED" \
  --remote-body-file "$REMBODY")
assert_contains "local file replaced from remote" "$(cat "$LOCALFILE")" "Pulled from remote."
PENTRY=$(cd "$AREPO" && bash "$BASELINE" get 0050)
assert_eq "baseline remote_updated_at recorded" "$A_RUPDATED" \
  "$(printf '%s' "$PENTRY" | jq -r '.remote_updated_at')"
assert_eq "baseline local_hash is the POST-overwrite file hash" \
  "$(bash "$NORMALISE" "$LOCALFILE" | hash_sha256_stdin)" \
  "$(printf '%s' "$PENTRY" | jq -r '.local_hash')"
assert_eq "baseline remote_hash is the projection actually written" \
  "$(bash "$NORMALISE" --stdin <"$REMBODY" | hash_sha256_stdin)" \
  "$(printf '%s' "$PENTRY" | jq -r '.remote_hash')"

echo "Test: a freshly-pulled item classifies synced on the next run"
assert_eq "post-pull → synced" "synced" \
  "$(bash "$CLASSIFY2" --file "$LOCALFILE" --external-id ENG-7 \
    --baseline "$PENTRY" --timestamp 0 --remote-status present \
    --remote-updated "$A_RUPDATED")"

echo "Test: finalise advances the global timestamp"
(cd "$AREPO" && bash "$APPLY" finalise --timestamp 1751000000)
assert_eq "timestamp persisted" "1751000000" \
  "$(jq -r '.timestamp' "$(cd "$AREPO" && bash "$BASELINE" path)")"

echo "Test: resumability — a crash between side-effect and baseline set leaves no entry"
RREPO=$(setup_baseline_repo)
mkdir -p "$RREPO/meta/work"
RFILE="$RREPO/meta/work/0060-y.md"
write_item "$RFILE"
RNEW="$TMPDIR_BASE/resume-new.md"
write_item "$RNEW"
perl -pi -e 's/Implement the thing carefully\./Resumed pull./' "$RNEW"
RC=0
(cd "$RREPO" && ACCELERATOR_TEST_MODE=1 WORK_SYNC_FAIL_AFTER=side-effect \
  bash "$APPLY" pull --id 0060 --file "$RFILE" --new-content-file "$RNEW" \
  --remote-updated "$A_RUPDATED" --remote-body-file "$REMBODY") || RC=$?
assert_eq "fault hook aborts (exit 99)" "99" "$RC"
assert_contains "side-effect DID happen (file overwritten)" "$(cat "$RFILE")" "Resumed pull."
assert_eq "baseline entry NOT set (interrupted before set)" "" \
  "$(cd "$RREPO" && bash "$BASELINE" get 0060)"
# Re-run without the fault → baseline now set (idempotent recovery).
(cd "$RREPO" && bash "$APPLY" pull --id 0060 --file "$RFILE" \
  --new-content-file "$RNEW" --remote-updated "$A_RUPDATED" --remote-body-file "$REMBODY")
assert_neq "re-run sets the baseline entry" "" \
  "$(cd "$RREPO" && bash "$BASELINE" get 0060)"

echo ""

# ============================================================
echo "=== work-item-section-diff.sh — section-grouped conflict diff ==="
echo ""

SECTION_DIFF="$SCRIPT_DIR/work-item-section-diff.sh"
SD_LOCAL="$TMPDIR_BASE/sd-local.md"
SD_REMOTE="$TMPDIR_BASE/sd-remote.md"

cat >"$SD_LOCAL" <<'EOF'
---
id: "0042"
external_id: "ENG-7"
title: "Do the thing"
last_updated: "2026-06-10T00:00:00+00:00"
---

# 0042: Do the thing

## Summary

Local summary text.

## Requirements

- local req one
- req two
EOF
cat >"$SD_REMOTE" <<'EOF'
---
id: "0042"
external_id: "ENG-7"
title: "Do the thing"
last_updated: "2026-06-10T00:00:00+00:00"
---

# 0042: Do the thing

## Summary

Remote summary text.

## Requirements

- remote req one
- req two
EOF

echo "Test: a conflict spanning two sections shows both, grouped by heading"
SD_OUT=$(bash "$SECTION_DIFF" "$SD_LOCAL" "$SD_REMOTE")
assert_contains "Summary section shown" "$SD_OUT" "=== Summary (- LOCAL / + REMOTE) ==="
assert_contains "Requirements section shown" "$SD_OUT" "=== Requirements (- LOCAL / + REMOTE) ==="
assert_contains "local Summary on the - side" "$SD_OUT" "-Local summary text."
assert_contains "remote Summary on the + side" "$SD_OUT" "+Remote summary text."

echo "Test: byte-equal (identical) frontmatter is omitted"
assert_not_contains "identical frontmatter not shown" "$SD_OUT" "=== frontmatter"

echo "Test: a whitespace-only section delta is omitted (normalised equal)"
SD_WS_LOCAL="$TMPDIR_BASE/sd-ws-local.md"
SD_WS_REMOTE="$TMPDIR_BASE/sd-ws-remote.md"
cat >"$SD_WS_LOCAL" <<'EOF'
---
id: "0042"
title: "T"
---

# T

## Summary

Same text.
EOF
cat >"$SD_WS_REMOTE" <<'EOF'
---
id: "0042"
title: "T"
---

# T

## Summary

Same text.
EOF
# Add trailing whitespace to the remote Summary only.
perl -pi -e 's/Same text\./Same text.   /' "$SD_WS_REMOTE"
SD_WS_OUT=$(bash "$SECTION_DIFF" "$SD_WS_LOCAL" "$SD_WS_REMOTE")
assert_contains "no differing sections after normalisation" "$SD_WS_OUT" \
  "no differing sections"

echo "Test: a frontmatter content change is its own section"
SD_FM_REMOTE="$TMPDIR_BASE/sd-fm-remote.md"
cat >"$SD_FM_REMOTE" <<'EOF'
---
id: "0042"
title: "Renamed thing"
---

# T

## Summary

Same text.
EOF
SD_FM_OUT=$(bash "$SECTION_DIFF" "$SD_WS_LOCAL" "$SD_FM_REMOTE")
assert_contains "frontmatter shown as its own section" "$SD_FM_OUT" \
  "=== frontmatter (- LOCAL / + REMOTE) ==="

echo ""

test_summary
