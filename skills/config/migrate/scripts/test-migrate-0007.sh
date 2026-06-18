#!/usr/bin/env bash
set -euo pipefail

# Fixture suite for migration 0007 (meta/ corpus unified-schema migration),
# mechanical-rewrite + backfill + precondition halves. The interactive
# body-section linkage path is driven by test-migrate-interactive.sh.
# Run: bash skills/config/migrate/scripts/test-migrate-0007.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

export LC_ALL=C
MIGRATION="$PLUGIN_ROOT/skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh"
DRIVER="$SCRIPT_DIR/run-migrations.sh"
VALIDATOR="$PLUGIN_ROOT/scripts/validate-corpus-frontmatter.sh"
FRAG="$SCRIPT_DIR/frontmatter-frag.awk"
BODY="$SCRIPT_DIR/0007-frontmatter-rewrite.awk"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# A migrations dir containing only 0007 (so the runner applies nothing else).
ONLY_0007="$TMP/only-0007"
mkdir -p "$ONLY_0007"
cp "$MIGRATION" "$ONLY_0007/"

git_init() { # $1 = repo root
  git -C "$1" init -q
  git -C "$1" config user.email test@example.com
  git -C "$1" config user.name "Fixture Author"
  git -C "$1" add -A
  git -C "$1" commit -q -m "seed" >/dev/null 2>&1
}

# Run 0007 via the runner against $1; sets RUN_RC, RUN_OUT.
run_0007() {
  RUN_RC=0
  RUN_OUT="$(cd "$1" && PROJECT_ROOT="$1" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    ACCELERATOR_MIGRATIONS_DIR="$ONLY_0007" ACCELERATOR_MIGRATE_FORCE=1 \
    bash "$DRIVER" 2>&1 </dev/null)" || RUN_RC=$?
}

fm_line() { grep -E "^$2:" "$1" | head -1; } # $1=file $2=key

# Assert the corpus dir (or file list) validates clean. Wraps the inline
# validator-clean idiom so the suite has one gate implementation.
assert_validates() { # $1=test_name $2..=dir|files
  local name="$1"
  shift
  local out rc=0
  out="$("$VALIDATOR" "$@" 2>&1)" || rc=$?
  if [ "$rc" -eq 0 ]; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name (validator reported violations)"
    printf '%s\n' "$out" | sed 's/^/    /'
    FAIL=$((FAIL + 1))
  fi
}

# Assert the standalone validator reports a targeted violation code over the
# given PRE-migration fixture file(s). Runs the validator DIRECTLY (NOT via
# run_0007, which under set -euo pipefail mutates then aborts at
# self_validate_structural, leaving a half-migrated tree whose surfaced code may
# differ) so the red step asserts exactly the shape under test.
assert_violation() { # $1=test_name $2=code $3..=files
  local name="$1" code="$2"
  shift 2
  local out rc=0
  out="$("$VALIDATOR" "$@" 2>&1)" || rc=$?
  if [ "$rc" -ne 0 ] && grep -qF -- "$code" <<<"$out"; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name (expected violation $code pre-fix)"
    printf '%s\n' "$out" | sed 's/^/    /'
    FAIL=$((FAIL + 1))
  fi
}

# ── Happy-path corpus (post-0005/0006 shapes) ───────────────────────────────
echo "=== Mechanical rewrite + backfill (happy path) ==="
REPO="$TMP/happy"
mkdir -p "$REPO/meta/work" "$REPO/meta/plans" "$REPO/meta/decisions" "$REPO/meta/notes"

cat >"$REPO/meta/work/0099-sample.md" <<'EOF'
---
type: work-item
work_item_id: "0099"
title: "Sample Work Item"
date: "2026-01-01"
author: Toby
skill: create-work-item
kind: story
priority: high
status: ready
parent: ""
external_id: ""
---
# Sample Work Item
EOF

cat >"$REPO/meta/plans/2026-02-03-0042-some-plan.md" <<'EOF'
---
type: plan
work_item_id: "0042"
title: "Some Plan"
date: "2026-02-03T10:00:00+00:00"
author: Toby
skill: create-plan
status: accepted
git_commit: deadbeef123
branch: ticket-management@
repository: accelerator
reviewer: ""
---
# Some Plan
EOF

cat >"$REPO/meta/decisions/ADR-0050-some-decision.md" <<'EOF'
---
type: adr
adr_id: ADR-0050
title: "Some Decision"
date: "2026-02-04T00:00:00+00:00"
author: Toby
producer: create-adr
status: accepted
decision_makers: []
---
# ADR-0050: Some Decision
EOF

# Fence-less note → note backfill (anchored).
printf '# A Loose Note\n\nAn observation captured before the schema existed.\n' \
  >"$REPO/meta/notes/2026-04-01-a-loose-note.md"
# Fence-less pre-convention plan → plan backfill (anchored).
printf '# Legacy Plan\n\nA hand-written plan from before frontmatter.\n' \
  >"$REPO/meta/plans/2026-03-01-legacy-plan.md"

git_init "$REPO"
run_0007 "$REPO"
assert_eq "runner exits 0 on happy corpus" "0" "$RUN_RC"
assert_contains "0007 recorded as applied" "$RUN_OUT" "applied"
assert_contains "ledger contains 0007" "$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null)" "0007-unify-meta-corpus-frontmatter"

WI="$REPO/meta/work/0099-sample.md"
assert_contains "work-item own-id -> quoted id" "$(fm_line "$WI" id)" 'id: "0099"'
assert_not_contains "work-item work_item_id removed" "$(cat "$WI")" "work_item_id:"
assert_contains "skill -> producer" "$(fm_line "$WI" producer)" "producer: create-work-item"
assert_contains "date-only normalised to ISO" "$(fm_line "$WI" date)" 'date: "2026-01-01T00:00:00+00:00"'
assert_contains "tags backfilled" "$(fm_line "$WI" tags)" "tags: []"
assert_contains "schema_version backfilled" "$(fm_line "$WI" schema_version)" "schema_version: 1"
assert_not_contains "empty parent placeholder omitted" "$(cat "$WI")" "parent:"
assert_not_contains "empty external_id placeholder omitted" "$(cat "$WI")" "external_id:"

PLAN="$REPO/meta/plans/2026-02-03-0042-some-plan.md"
assert_contains "plan status accepted -> done" "$(fm_line "$PLAN" status)" "status: done"
assert_contains "git_commit -> revision" "$(fm_line "$PLAN" revision)" 'revision: "deadbeef123"'
assert_not_contains "branch dropped" "$(cat "$PLAN")" "branch:"
assert_contains "plan keeps foreign work_item_id" "$(fm_line "$PLAN" work_item_id)" 'work_item_id: "0042"'
assert_contains "plan id derived from stem" "$(fm_line "$PLAN" id)" 'id: "2026-02-03-0042-some-plan"'

ADR="$REPO/meta/decisions/ADR-0050-some-decision.md"
assert_contains "adr_id -> quoted id" "$(fm_line "$ADR" id)" 'id: "ADR-0050"'
assert_not_contains "adr_id removed" "$(cat "$ADR")" "adr_id:"

NOTE="$REPO/meta/notes/2026-04-01-a-loose-note.md"
assert_contains "note backfilled type" "$(fm_line "$NOTE" type)" "type: note"
assert_contains "note backfilled status captured" "$(fm_line "$NOTE" status)" "status: captured"
assert_contains "note backfilled id from stem" "$(fm_line "$NOTE" id)" 'id: "2026-04-01-a-loose-note"'
assert_contains "note backfilled revision (VCS)" "$(cat "$NOTE")" "revision:"

LEG="$REPO/meta/plans/2026-03-01-legacy-plan.md"
assert_contains "legacy plan backfilled type" "$(fm_line "$LEG" type)" "type: plan"
assert_contains "legacy plan id from stem" "$(fm_line "$LEG" id)" 'id: "2026-03-01-legacy-plan"'

echo "=== Validator passes over the migrated corpus ==="
vrc=0
"$VALIDATOR" "$REPO/meta" >/tmp/0007-val.out 2>&1 || vrc=$?
if [ "$vrc" -eq 0 ]; then
  echo "  PASS: migrated corpus validates"
  PASS=$((PASS + 1))
else
  echo "  FAIL: migrated corpus has violations"
  sed 's/^/    /' /tmp/0007-val.out
  FAIL=$((FAIL + 1))
fi

echo "=== Idempotency: direct re-invocation is an empty diff ==="
git -C "$REPO" add -A && git -C "$REPO" commit -q -m migrated >/dev/null 2>&1
# Re-run the migration script directly (ledger bypassed), feeding a minimal
# INIT handshake; assert no file changes.
printf 'INIT\t\t\n' | (cd "$REPO" && PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  MIGRATION_ID=0007-unify-meta-corpus-frontmatter bash "$MIGRATION" >/dev/null 2>&1) || true
diff_out="$(git -C "$REPO" status --porcelain meta/ || true)"
assert_empty "re-run produces no working-tree changes (idempotent)" "$diff_out"

# ── Precondition pre-pass: REFUSE on a work-item missing kind ────────────────
echo "=== Precondition pre-pass refuses, zero mutations ==="
BAD="$TMP/bad"
mkdir -p "$BAD/meta/work"
cat >"$BAD/meta/work/0001-nokind.md" <<'EOF'
---
type: work-item
work_item_id: "0001"
title: "No Kind"
date: "2026-01-01T00:00:00+00:00"
author: Toby
status: ready
---
# No Kind
EOF
git_init "$BAD"
before="$(cat "$BAD/meta/work/0001-nokind.md")"
run_0007 "$BAD"
assert_neq "runner fails when a work-item lacks kind:" "0" "$RUN_RC"
assert_contains "REFUSE diagnostic emitted" "$RUN_OUT" "0007-REFUSE"
assert_eq "refused file left unmutated" "$before" "$(cat "$BAD/meta/work/0001-nokind.md")"
assert_not_contains "ledger does not record a refused migration" "$(cat "$BAD/.accelerator/state/migrations-applied" 2>/dev/null || true)" "0007"

# ── frontmatter-frag.awk parity (genuine two -f form, BSD awk) ───────────────
echo "=== frag.awk quoting/refusal parity (two -f form) ==="
probe="$TMP/probe.awk"
cat >"$probe" <<'AWK'
BEGIN {
  printf "%s\n", fm_normalise_value("bare")
  printf "%s\n", fm_normalise_value("\"already\"")
  printf "%d\n", fm_refuses("has#hash")
  printf "%d\n", fm_refuses("\"safe\"")
  printf "%d\n", fm_is_fence("---")
}
AWK
parity="$(awk -f "$FRAG" -f "$probe" </dev/null)"
assert_eq "frag two -f form produces expected battery" \
  "$(printf '"bare"\n"already"\n1\n0\n1')" "$parity"

# ── Protocol hygiene: no non-frame bytes precede READY on stdout ─────────────
echo "=== Protocol hygiene: first stdout frame is READY ==="
HY="$TMP/hygiene"
mkdir -p "$HY/meta/work"
cat >"$HY/meta/work/0001-clean.md" <<'EOF'
---
type: work-item
id: "0001"
title: "Clean"
date: "2026-01-01T00:00:00+00:00"
author: Toby
producer: create-work-item
kind: story
priority: high
status: ready
tags: []
last_updated: "2026-01-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Clean
EOF
git_init "$HY"
hy_out="$(printf 'INIT\t\t\n' | (cd "$HY" && PROJECT_ROOT="$HY" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  MIGRATION_ID=0007-unify-meta-corpus-frontmatter bash "$MIGRATION" 2>/dev/null) || true)"
first_line="$(printf '%s\n' "$hy_out" | head -1)"
assert_matches_regex "first stdout line is the READY frame" '^READY' "$first_line"

# ── Pre-existing frontmatter linkage: path-shape → typed ─────────────────────
echo "=== Frontmatter linkage normalisation: path-shape -> typed ==="
PL="$TMP/pathlink"
mkdir -p "$PL/meta/work" "$PL/meta/plans"
cat >"$PL/meta/work/0030-target.md" <<'EOF'
---
type: work-item
work_item_id: "0030"
title: "Target"
date: "2026-01-01T00:00:00+00:00"
author: Toby
producer: create-work-item
kind: story
priority: high
status: ready
tags: []
last_updated: "2026-01-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Target
EOF
cat >"$PL/meta/plans/2026-05-13-0055-feature.md" <<'EOF'
---
type: plan
work_item_id: "0030"
title: "Feature Plan"
date: "2026-05-13T00:00:00+00:00"
author: Toby
producer: create-plan
status: done
parent: "meta/work/0030-target.md"
relates_to: ["meta/plans/2026-05-13-0055-feature.md"]
revision: "abc123"
repository: "accelerator"
last_updated: "2026-05-13T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Feature Plan

A code sample mentioning `meta/work/0099-nonexistent.md` in the body must not change.
EOF
# Child work-item with a deterministic bare-number parent → work-item:0030.
cat >"$PL/meta/work/0031-child.md" <<'EOF'
---
type: work-item
work_item_id: "0031"
title: "Child"
date: "2026-01-02T00:00:00+00:00"
author: Toby
producer: create-work-item
kind: story
priority: high
status: ready
parent: "0030"
tags: []
last_updated: "2026-01-02T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Child
EOF
git_init "$PL"
run_0007 "$PL"
assert_eq "path-shape corpus exits 0" "0" "$RUN_RC"
assert_contains "bare-number parent -> work-item:0030 (deterministic)" "$(fm_line "$PL/meta/work/0031-child.md" parent)" 'parent: "work-item:0030"'
PLAN2="$PL/meta/plans/2026-05-13-0055-feature.md"
assert_contains "path parent -> work-item:0030 (bare number)" "$(fm_line "$PLAN2" parent)" 'parent: "work-item:0030"'
assert_contains "path relates_to plan -> full stem" "$(fm_line "$PLAN2" relates_to)" 'relates_to: ["plan:2026-05-13-0055-feature"]'
assert_contains "body path-shape mention left untouched" "$(cat "$PLAN2")" 'meta/work/0099-nonexistent.md'
plvrc=0
"$VALIDATOR" "$PL/meta" >/tmp/0007-pl-val.out 2>&1 || plvrc=$?
if [ "$plvrc" -eq 0 ]; then
  echo "  PASS: path-normalised corpus validates"
  PASS=$((PASS + 1))
else
  echo "  FAIL: path-normalised corpus has violations"
  sed 's/^/    /' /tmp/0007-pl-val.out
  FAIL=$((FAIL + 1))
fi

# ── Interactive body-section linkage: resolved mechanical + ambiguous accept ─
echo "=== Interactive linkage: resolved mechanical + ambiguous applied ==="
LINK="$TMP/linkrepo"
mkdir -p "$LINK/meta/work"
# Target work-item (exists, so references resolve referentially).
cat >"$LINK/meta/work/0061-target.md" <<'EOF'
---
type: work-item
work_item_id: "0061"
title: "Target"
date: "2026-01-01T00:00:00+00:00"
author: Toby
producer: create-work-item
kind: story
priority: high
status: ready
tags: []
last_updated: "2026-01-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Target
EOF
# Source work-item: one resolved (Blocks: → blocks work-item:0061) and one
# ambiguous (unhinted ## References path → relates_to, accepted).
cat >"$LINK/meta/work/0060-source.md" <<'EOF'
---
type: work-item
work_item_id: "0060"
title: "Source"
date: "2026-01-01T00:00:00+00:00"
author: Toby
producer: create-work-item
kind: story
priority: high
status: ready
tags: []
last_updated: "2026-01-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Source

## References

- See `meta/work/0061-target.md`

## Dependencies

- Blocks: 0061
EOF
git_init "$LINK"
DEC="$TMP/decisions.txt"
printf 'accept\n' >"$DEC" # one decision for the single ambiguous reference
LRC=0
# shellcheck disable=SC2034  # captured only to harvest the exit code into LRC
LOUT="$(cd "$LINK" && PROJECT_ROOT="$LINK" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$ONLY_0007" ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DEC" \
  bash "$DRIVER" 2>&1 </dev/null)" || LRC=$?
assert_eq "interactive run exits 0" "0" "$LRC"
SRC="$LINK/meta/work/0060-source.md"
assert_contains "resolved Blocks applied mechanically" "$(cat "$SRC")" 'blocks: ["work-item:0061"]'
assert_contains "ambiguous relates_to applied on accept" "$(cat "$SRC")" 'relates_to: ["work-item:0061"]'
SESSION="$LINK/.accelerator/state/migrations-0007-unify-meta-corpus-frontmatter-session.jsonl"
assert_file_exists "session log written" "$SESSION"
assert_contains "session log records an accepted ambiguous decision" "$(cat "$SESSION" 2>/dev/null)" '"outcome":"accepted"'
lvrc=0
"$VALIDATOR" "$LINK/meta" >/tmp/0007-link-val.out 2>&1 || lvrc=$?
if [ "$lvrc" -eq 0 ]; then
  echo "  PASS: linked corpus validates (incl. referential integrity)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: linked corpus has violations"
  sed 's/^/    /' /tmp/0007-link-val.out
  FAIL=$((FAIL + 1))
fi

# ── Phase 1: meta/prs/ typing + meta/docs/ skip + single-sourced classification ─
echo "=== Phase 1: meta/prs/ -> pr-description, meta/docs/ skipped ==="
P1="$TMP/phase1"
mkdir -p "$P1/meta/prs" "$P1/meta/docs"

# Typeless (empty type:) pr-description, otherwise complete (anchored type, so
# revision/repository present; pr_number required extra present — Phase 4
# exercises that backfill separately). relates_to carries a path-shape ref to a
# second pr-description, exercising the broadened identity namespace end-to-end.
cat >"$P1/meta/prs/240-description.md" <<'EOF'
---
type:
id: "240-description"
title: "PR 240 Description"
date: "2026-06-01T00:00:00+00:00"
author: Toby
status: complete
relates_to: ["meta/prs/416-summary.md"]
tags: []
pr_number: 240
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 240 Description
EOF

cat >"$P1/meta/prs/416-summary.md" <<'EOF'
---
type: pr-description
id: "416-summary"
title: "PR 416 Summary"
date: "2026-06-02T00:00:00+00:00"
author: Toby
status: complete
tags: []
pr_number: 416
revision: "def456"
repository: "accelerator"
last_updated: "2026-06-02T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 416 Summary
EOF

# Freeform plugin-unowned doc (non-conforming frontmatter, no schema type) — must
# be skipped entirely (byte-unchanged, never validated).
cat >"$P1/meta/docs/logging-guide.md" <<'EOF'
---
title: Logging Guide
foo: bar
---
# Logging Guide

Freeform documentation the plugin does not own.
EOF

DOCS_BEFORE="$(cat "$P1/meta/docs/logging-guide.md")"
git_init "$P1"

# Red step: pre-migration, the typeless pr file reports INVALID-TYPE.
assert_violation "Phase 1 red: typeless meta/prs/ file is INVALID-TYPE" \
  "INVALID-TYPE" "$P1/meta/prs/240-description.md"

run_0007 "$P1"
assert_eq "Phase 1 corpus exits 0" "0" "$RUN_RC"
assert_contains "Phase 1 typeless pr -> pr-description" \
  "$(fm_line "$P1/meta/prs/240-description.md" type)" "type: pr-description"
assert_eq "Phase 1 pr type line is unique (no duplicate)" "1" \
  "$(grep -c '^type:' "$P1/meta/prs/240-description.md")"
assert_contains "Phase 1 pr relates_to path -> pr-description:<stem>" \
  "$(fm_line "$P1/meta/prs/240-description.md" relates_to)" 'relates_to: ["pr-description:416-summary"]'
assert_eq "Phase 1 meta/docs/ byte-unchanged" "$DOCS_BEFORE" \
  "$(cat "$P1/meta/docs/logging-guide.md")"

# Standalone whole-corpus mode over a corpus containing meta/docs/ exits 0
# (isolates the validator-side out_of_scope skip from the migration file-list path).
assert_validates "Phase 1 whole-corpus validates (meta/docs/ skipped)" "$P1/meta"

# Idempotency: a second run leaves an empty meta/ diff.
git -C "$P1" add -A && git -C "$P1" commit -q -m migrated >/dev/null 2>&1
run_0007 "$P1"
assert_empty "Phase 1 second run is an empty meta/ diff" \
  "$(git -C "$P1" status --porcelain meta/ || true)"

# Both surfaces single-source path classification (no local definitions remain).
assert_not_contains "migration defines no local infer_type_from_path" \
  "$(grep -E '^infer_type_from_path\(\)' "$MIGRATION" || true)" "infer_type_from_path()"
assert_not_contains "validator defines no local out_of_scope" \
  "$(grep -E '^out_of_scope\(\)' "$VALIDATOR" || true)" "out_of_scope()"
assert_contains "migration sources doc-type-inference.sh" \
  "$(cat "$MIGRATION")" "doc-type-inference.sh"
assert_contains "validator sources doc-type-inference.sh" \
  "$(cat "$VALIDATOR")" "doc-type-inference.sh"

# Linkage-target alignment (table-driven, full id): the awk path_to_typed encodes
# the id-derivation halves the shared helper does not. Pin one representative path
# per arm to its full doc-type:id, including the prs arm in step with the helper.
echo "=== Phase 1: path_to_typed id-derivation alignment ==="
PT_PROBE="$TMP/pt-probe.awk"
cat >"$PT_PROBE" <<'AWK'
BEGIN {
  print path_to_typed("meta/work/0030-target.md")
  print path_to_typed("meta/plans/2026-05-13-0055-feature.md")
  print path_to_typed("meta/decisions/ADR-0050-some-decision.md")
  print path_to_typed("meta/reviews/prs/2026-06-17-pr-430-review.md")
  print path_to_typed("meta/prs/240-description.md")
  print path_to_typed("meta/research/codebase/2026-01-01-foo.md")
}
AWK
pt_out="$(awk -f "$FRAG" -f "$BODY" -f "$PT_PROBE" </dev/null 2>/dev/null)"
assert_eq "path_to_typed id derivation per arm (incl. prs)" \
  "$(printf 'work-item:0030\nplan:2026-05-13-0055-feature\nadr:ADR-0050\npr-review:2026-06-17-pr-430-review\npr-description:240-description\ncodebase-research:2026-01-01-foo')" \
  "$pt_out"

test_summary
