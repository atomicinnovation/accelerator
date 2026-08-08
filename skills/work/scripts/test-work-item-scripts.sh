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

test_summary
