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
echo "=== work-item-sync-label.sh ==="
echo ""

SYNC_LABEL="$SCRIPT_DIR/work-item-sync-label.sh"
SYNC_LABEL_GOLDEN="$SCRIPT_DIR/test-fixtures/work-item-sync-label.golden"
LABEL_CLASSIFY_EXPECTED_ROWS=11
LABEL_LABEL_EXPECTED_ROWS=8
LABEL_DEFAULT_EXPECTED_ROWS=2

label_unescape() {
  if [ "$1" = "(empty)" ]; then
    printf ''
  else
    printf '%s' "$1"
  fi
}

echo "Test: classify, label and the default composed mode against the shared golden"
LABEL_SECTION=""
LABEL_CLASSIFY_RAN=0
LABEL_LABEL_RAN=0
LABEL_DEFAULT_RAN=0
# Read by redirect from the file directly, never a pipeline, so PASS/FAIL
# updates are not lost to a subshell.
while IFS= read -r LABEL_LINE; do
  case "$LABEL_LINE" in
    \#* | "") continue ;;
    "[CLASSIFY]")
      LABEL_SECTION="classify"
      continue
      ;;
    "[LABEL]")
      LABEL_SECTION="label"
      continue
      ;;
    "[DEFAULT]")
      LABEL_SECTION="default"
      continue
      ;;
  esac

  case "$LABEL_SECTION" in
    classify)
      IFS='|' read -r L_RAW L_EXPECTED <<<"$LABEL_LINE"
      LABEL_CLASSIFY_RAN=$((LABEL_CLASSIFY_RAN + 1))
      L_INPUT=$(label_unescape "$L_RAW")
      assert_eq "classify '$L_RAW'" "$L_EXPECTED" \
        "$(bash "$SYNC_LABEL" --classify "$L_INPUT")"
      ;;
    label)
      IFS='|' read -r L_STATUS L_EXIT L_EXPECTED <<<"$LABEL_LINE"
      LABEL_LABEL_RAN=$((LABEL_LABEL_RAN + 1))
      if [ "$L_EXIT" = "0" ]; then
        assert_eq "label $L_STATUS" "$L_EXPECTED" \
          "$(bash "$SYNC_LABEL" --label "$L_STATUS")"
      else
        assert_exit_code "label $L_STATUS exits $L_EXIT" "$L_EXIT" \
          bash "$SYNC_LABEL" --label "$L_STATUS"
      fi
      ;;
    default)
      IFS='|' read -r L_RAW L_EXPECTED <<<"$LABEL_LINE"
      LABEL_DEFAULT_RAN=$((LABEL_DEFAULT_RAN + 1))
      L_INPUT=$(label_unescape "$L_RAW")
      assert_eq "default '$L_RAW'" "$L_EXPECTED" \
        "$(bash "$SYNC_LABEL" "$L_INPUT")"
      ;;
  esac
done <"$SYNC_LABEL_GOLDEN"

if [ "$LABEL_CLASSIFY_RAN" -eq "$LABEL_CLASSIFY_EXPECTED_ROWS" ] &&
  [ "$LABEL_LABEL_RAN" -eq "$LABEL_LABEL_EXPECTED_ROWS" ] &&
  [ "$LABEL_DEFAULT_RAN" -eq "$LABEL_DEFAULT_EXPECTED_ROWS" ]; then
  echo "  PASS: ran $LABEL_CLASSIFY_RAN classify, $LABEL_LABEL_RAN label," \
    "$LABEL_DEFAULT_RAN default rows (expected" \
    "$LABEL_CLASSIFY_EXPECTED_ROWS/$LABEL_LABEL_EXPECTED_ROWS/$LABEL_DEFAULT_EXPECTED_ROWS)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: ran $LABEL_CLASSIFY_RAN classify, $LABEL_LABEL_RAN label," \
    "$LABEL_DEFAULT_RAN default rows (expected" \
    "$LABEL_CLASSIFY_EXPECTED_ROWS/$LABEL_LABEL_EXPECTED_ROWS/$LABEL_DEFAULT_EXPECTED_ROWS)"
  FAIL=$((FAIL + 1))
fi

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
echo "=== work-item-sync-classify.sh — change-detection engine ==="
echo ""

CLASSIFY="$SCRIPT_DIR/work-item-sync-classify.sh"
CLASSIFY_FIXTURE="$SCRIPT_DIR/test-fixtures/work-item-sync-classify.json"
CLASSIFY_EXPECTED_BASH_CASES=14

classify() { bash "$CLASSIFY" "$@"; }

# Shared table with cli/work/tests/sync_classify.rs — one oracle read by
# both implementations, so a row can't be edited on one side and left stale
# on the other. Content is fixed across every row; only the symbolic hash
# choice, the timestamps and the mtime offset vary between cases.
CLASSIFY_LOCAL_FILE="$TMPDIR_BASE/classify-local.md"
CLASSIFY_REMOTE_FILE="$TMPDIR_BASE/classify-remote.md"
jq -r '.local_content' "$CLASSIFY_FIXTURE" >"$CLASSIFY_LOCAL_FILE"
jq -r '.remote_body' "$CLASSIFY_FIXTURE" >"$CLASSIFY_REMOTE_FILE"

CLASSIFY_BASE_REPORTED=$(jq -r '.remote_updated_reported' "$CLASSIFY_FIXTURE")
CLASSIFY_BASE_TICKED=$(jq -r '.remote_updated_ticked' "$CLASSIFY_FIXTURE")
CLASSIFY_LOCAL_FROM_CONTENT=$(nhash "$CLASSIFY_LOCAL_FILE")
CLASSIFY_REMOTE_FROM_CONTENT=$(bash "$NORMALISE" --stdin <"$CLASSIFY_REMOTE_FILE" | hash_sha256_stdin)
CLASSIFY_STALE_HASH="stale-hash-that-can-never-match-anything-real"

CLASSIFY_REAL_MTIME=$(stat -f %m "$CLASSIFY_LOCAL_FILE" 2>/dev/null) ||
  CLASSIFY_REAL_MTIME=$(stat -c %Y "$CLASSIFY_LOCAL_FILE")

# Resolves one of the three symbolic hash values against the shared content.
# ⚠️ Fabricated digests are not executable by bash: "from-content" always
# runs the real recipe, never a literal placeholder string.
classify_resolve_hash() {
  case "$1" in
    absent) printf '' ;;
    stale) printf '%s' "$CLASSIFY_STALE_HASH" ;;
    from-content) printf '%s' "$2" ;;
    *)
      echo "classify_resolve_hash: unknown symbol: $1" >&2
      return 1
      ;;
  esac
}

# Resolves a {kind, value} timestamp object to bash's flat --remote-updated
# string. not_reported and not_read both collapse to empty — bash cannot
# distinguish them, which is exactly why the rows pinning that distinction
# are rust-only.
classify_resolve_timestamp() {
  local kind value
  kind=$(printf '%s' "$1" | jq -r '.kind')
  case "$kind" in
    reported)
      value=$(printf '%s' "$1" | jq -r '.value')
      # The fixture's own literal placeholder tokens, not shell expansion.
      # shellcheck disable=SC2016
      case "$value" in
        '$R') printf '%s' "$CLASSIFY_BASE_REPORTED" ;;
        '$T') printf '%s' "$CLASSIFY_BASE_TICKED" ;;
        *) printf '%s' "$value" ;;
      esac
      ;;
    not_reported | not_read) printf '' ;;
    *)
      echo "classify_resolve_timestamp: unknown kind: $kind" >&2
      return 1
      ;;
  esac
}

CLASSIFY_RAN=0
# Read by redirect, never a pipeline: `jq … | while read` runs the loop body
# in a subshell, discarding every PASS/FAIL update this harness makes.
while IFS= read -r CLASSIFY_CASE; do
  CLASSIFY_APPLIES_BASH=$(printf '%s' "$CLASSIFY_CASE" |
    jq -r 'any(.applies_to[]; . == "bash")')
  [ "$CLASSIFY_APPLIES_BASH" = "true" ] || continue
  CLASSIFY_RAN=$((CLASSIFY_RAN + 1))

  CLASSIFY_NAME=$(printf '%s' "$CLASSIFY_CASE" | jq -r '.name')
  CLASSIFY_EXTERNAL_ID=$(printf '%s' "$CLASSIFY_CASE" | jq -r '.external_id')
  CLASSIFY_PRESENCE=$(printf '%s' "$CLASSIFY_CASE" | jq -r '.presence')
  CLASSIFY_EXPECT=$(printf '%s' "$CLASSIFY_CASE" | jq -r '.expect')

  if [ "$CLASSIFY_PRESENCE" != "present" ]; then
    CLASSIFY_ACTUAL=$(classify --file "$CLASSIFY_LOCAL_FILE" \
      --external-id "$CLASSIFY_EXTERNAL_ID" --baseline "" --timestamp 0 \
      --remote-status "$CLASSIFY_PRESENCE")
    assert_eq "$CLASSIFY_NAME" "$CLASSIFY_EXPECT" "$CLASSIFY_ACTUAL"
    continue
  fi

  CLASSIFY_REMOTE_UPDATED_JSON=$(printf '%s' "$CLASSIFY_CASE" | jq -c '.remote_updated')
  CLASSIFY_REMOTE_UPDATED=$(classify_resolve_timestamp "$CLASSIFY_REMOTE_UPDATED_JSON")

  CLASSIFY_MTIME_OFFSET=$(printf '%s' "$CLASSIFY_CASE" | jq -r '.mtime_offset')
  CLASSIFY_TIMESTAMP=$((CLASSIFY_REAL_MTIME - CLASSIFY_MTIME_OFFSET))

  CLASSIFY_WITH_REMOTE_BODY=$(printf '%s' "$CLASSIFY_CASE" |
    jq -r '.with_remote_body // false')

  CLASSIFY_BASELINE_UPDATED_JSON=$(printf '%s' "$CLASSIFY_CASE" |
    jq -c '.baseline.remote_updated_at')
  CLASSIFY_BASELINE_UPDATED=$(classify_resolve_timestamp "$CLASSIFY_BASELINE_UPDATED_JSON")
  CLASSIFY_BASELINE_REMOTE_SYMBOL=$(printf '%s' "$CLASSIFY_CASE" |
    jq -r '.baseline.remote_hash')
  CLASSIFY_BASELINE_LOCAL_SYMBOL=$(printf '%s' "$CLASSIFY_CASE" |
    jq -r '.baseline.local_hash')
  CLASSIFY_BASELINE_REMOTE_HASH=$(classify_resolve_hash \
    "$CLASSIFY_BASELINE_REMOTE_SYMBOL" "$CLASSIFY_REMOTE_FROM_CONTENT")
  CLASSIFY_BASELINE_LOCAL_HASH=$(classify_resolve_hash \
    "$CLASSIFY_BASELINE_LOCAL_SYMBOL" "$CLASSIFY_LOCAL_FROM_CONTENT")

  CLASSIFY_ENTRY=$(jq -cn \
    --arg ru "$CLASSIFY_BASELINE_UPDATED" \
    --arg rh "$CLASSIFY_BASELINE_REMOTE_HASH" \
    --arg lh "$CLASSIFY_BASELINE_LOCAL_HASH" \
    '{remote_updated_at: $ru, remote_hash: $rh, local_hash: $lh}')

  if [ "$CLASSIFY_WITH_REMOTE_BODY" = "true" ]; then
    CLASSIFY_ACTUAL=$(classify --file "$CLASSIFY_LOCAL_FILE" \
      --external-id "$CLASSIFY_EXTERNAL_ID" --baseline "$CLASSIFY_ENTRY" \
      --timestamp "$CLASSIFY_TIMESTAMP" --remote-status present \
      --remote-updated "$CLASSIFY_REMOTE_UPDATED" \
      --remote-body-file "$CLASSIFY_REMOTE_FILE")
  else
    CLASSIFY_ACTUAL=$(classify --file "$CLASSIFY_LOCAL_FILE" \
      --external-id "$CLASSIFY_EXTERNAL_ID" --baseline "$CLASSIFY_ENTRY" \
      --timestamp "$CLASSIFY_TIMESTAMP" --remote-status present \
      --remote-updated "$CLASSIFY_REMOTE_UPDATED")
  fi

  assert_eq "$CLASSIFY_NAME" "$CLASSIFY_EXPECT" "$CLASSIFY_ACTUAL"
done < <(jq -c '.cases[]' "$CLASSIFY_FIXTURE")

if [ "$CLASSIFY_RAN" -eq "$CLASSIFY_EXPECTED_BASH_CASES" ]; then
  echo "  PASS: ran $CLASSIFY_RAN bash-applicable classify rows (expected $CLASSIFY_EXPECTED_BASH_CASES)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: ran $CLASSIFY_RAN bash-applicable classify rows, expected $CLASSIFY_EXPECTED_BASH_CASES"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== work-item-sync-decide.sh — (mode × state) decision table ==="
echo ""

DECIDE="$SCRIPT_DIR/work-item-sync-decide.sh"
DECIDE_GOLDEN="$SCRIPT_DIR/test-fixtures/work-item-sync-decide.golden"
DECIDE_EXPECTED_ROWS=24
TOKEN_EXPECTED_ROWS=5
dec() { bash "$DECIDE" decide --mode "$1" --state "$2" --dirty "$3"; }

echo "Test: mode resolution and the mutually-exclusive guard"
assert_eq "no flags → bidirectional" "bidirectional" "$(bash "$DECIDE" mode)"
assert_eq "--push-only" "push-only" "$(bash "$DECIDE" mode --push-only)"
assert_eq "--pull-only" "pull-only" "$(bash "$DECIDE" mode --pull-only)"
assert_exit_code "--push-only + --pull-only → error" 2 \
  bash "$DECIDE" mode --push-only --pull-only

# Shared table with cli/work/tests/sync_decide.rs. [DECIDE_RUST_ONLY] rows are
# skipped here: bash's only dirtiness test is `[ "$dirty" = "1" ]`, so the
# correct "unknown decides as dirty" behaviour is inexpressible in bash.
decide_unescape_token() {
  local raw="$1"
  if [ "$raw" = "(empty)" ]; then
    printf ''
  else
    printf '%s' "${raw//(nbsp)/$(printf '\xc2\xa0')}"
  fi
}

echo "Test: the (mode × state × dirty) table and the token resolver"
DECIDE_RAN=0
TOKEN_RAN=0
DECIDE_SECTION=""
# Read by redirect from the file directly, never a pipeline, so PASS/FAIL
# updates are not lost to a subshell.
while IFS= read -r DECIDE_LINE; do
  case "$DECIDE_LINE" in
    \#* | "") continue ;;
    "[DECIDE]")
      DECIDE_SECTION="decide"
      continue
      ;;
    "[DECIDE_RUST_ONLY]")
      DECIDE_SECTION="rust_only"
      continue
      ;;
    "[TOKEN]")
      DECIDE_SECTION="token"
      continue
      ;;
    "[TOKEN_RUST_ONLY]")
      DECIDE_SECTION="token_rust_only"
      continue
      ;;
  esac

  case "$DECIDE_SECTION" in
    decide)
      IFS='|' read -r D_MODE D_STATE D_DIRTY D_EXPECTED <<<"$DECIDE_LINE"
      DECIDE_RAN=$((DECIDE_RAN + 1))
      assert_eq "$D_MODE/$D_STATE/dirty=$D_DIRTY" "$D_EXPECTED" \
        "$(dec "$D_MODE" "$D_STATE" "$D_DIRTY")"
      ;;
    rust_only | token_rust_only) ;;
    token)
      IFS='|' read -r T_RAW T_EXPECTED <<<"$DECIDE_LINE"
      TOKEN_RAN=$((TOKEN_RAN + 1))
      T_TOKEN=$(decide_unescape_token "$T_RAW")
      assert_eq "token '$T_RAW'" "$T_EXPECTED" \
        "$(bash "$DECIDE" resolve-conflict-token "$T_TOKEN")"
      ;;
  esac
done <"$DECIDE_GOLDEN"

if [ "$DECIDE_RAN" -eq "$DECIDE_EXPECTED_ROWS" ]; then
  echo "  PASS: ran $DECIDE_RAN decide rows (expected $DECIDE_EXPECTED_ROWS)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: ran $DECIDE_RAN decide rows, expected $DECIDE_EXPECTED_ROWS"
  FAIL=$((FAIL + 1))
fi
if [ "$TOKEN_RAN" -eq "$TOKEN_EXPECTED_ROWS" ]; then
  echo "  PASS: ran $TOKEN_RAN token rows (expected $TOKEN_EXPECTED_ROWS)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: ran $TOKEN_RAN token rows, expected $TOKEN_EXPECTED_ROWS"
  FAIL=$((FAIL + 1))
fi

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

test_summary
