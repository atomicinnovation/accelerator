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

# Run 0007 DIRECTLY (bypassing the runner) against $1 with a minimal INIT
# handshake, capturing the migration's own stderr; sets DIRECT_RC, DIRECT_ERR.
# The runner sandboxes the migration's stderr to a per-migration log it DELETES
# on success, so DIVERGE breadcrumbs are only assertable via a direct run. This
# DOES mutate the corpus (the pre-harness backfill/rewrite runs before EOF), so
# call it on a dedicated fixture, not one already migrated via run_0007.
run_0007_direct() { # $1 = repo root
  DIRECT_RC=0
  DIRECT_ERR="$(printf 'INIT\t\t\n' | (cd "$1" && PROJECT_ROOT="$1" \
    CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" MIGRATION_ID=0007-unify-meta-corpus-frontmatter \
    bash "$MIGRATION" 2>&1 >/dev/null))" || DIRECT_RC=$?
}

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
assert_validates "migrated corpus validates" "$REPO/meta"

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
assert_validates "path-normalised corpus validates" "$PL/meta"

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
assert_validates "linked corpus validates (incl. referential integrity)" "$LINK/meta"

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
# The awk is now config-aware: it classifies against the injected
# doc_type_table (-v), so the probe feeds it the DEFAULT resolved table; the
# literal type:id expectations remain the source of truth (NOT derived from the
# same table) so a regression in the lookup or id-derivation is still caught.
echo "=== Phase 6: path_to_typed id-derivation alignment (default table) ==="
# The awk takes the table as 0x1E-joined records (a newline is unusable in a -v
# value on the one-true-awk); convert the resolver's newline output accordingly.
DEFAULT_TBL="$("$PLUGIN_ROOT/scripts/config-read-doc-type-paths.sh" | tr '\n' '\036')"
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
pt_out="$(awk -v doc_type_table="$DEFAULT_TBL" -f "$FRAG" -f "$BODY" -f "$PT_PROBE" </dev/null 2>/dev/null)"
assert_eq "path_to_typed id derivation per arm (incl. prs, most-specific match)" \
  "$(printf 'work-item:0030\nplan:2026-05-13-0055-feature\nadr:ADR-0050\npr-review:2026-06-17-pr-430-review\npr-description:240-description\ncodebase-research:2026-01-01-foo')" \
  "$pt_out"

# Non-default-path probe: a CUSTOM table resolves paths under custom configured
# dirs to the right type:id (and a path under a now-unconfigured default dir is
# unmapped). Literal expectations are kept as the source of truth.
echo "=== Phase 6: path_to_typed honours custom configured dirs ==="
CUSTOM_TBL="$(printf 'work-item\tcustom/work-items\036pr-description\tmeta/pull-requests\036plan\tmeta/plans')"
PT_PROBE2="$TMP/pt-probe2.awk"
cat >"$PT_PROBE2" <<'AWK'
BEGIN {
  print path_to_typed("custom/work-items/0001-foo.md")
  print path_to_typed("meta/pull-requests/240-desc.md")
  print path_to_typed("meta/plans/2026-01-01-x.md")
  print "[" path_to_typed("meta/work/0099-x.md") "]"
}
AWK
pt2_out="$(awk -v doc_type_table="$CUSTOM_TBL" -f "$FRAG" -f "$BODY" -f "$PT_PROBE2" </dev/null 2>/dev/null)"
assert_eq "path_to_typed config-aware lookup + id derivation (custom dirs)" \
  "$(printf 'work-item:0001\npr-description:240-desc\nplan:2026-01-01-x\n[]')" \
  "$pt2_out"

# ── Phase 2: schema-driven forbidden own-id key drop + pr_title fold ─────────
echo "=== Phase 2: forbidden own-id keys dropped; pr_title folds to title ==="
P2="$TMP/phase2"
mkdir -p "$P2/meta/reviews/prs"

# A: pr_title (non-empty) + review_pass, NO title -> pr_title folds into title,
# review_pass drops. Otherwise-complete (verdict/lenses/review_number/pr_number).
cat >"$P2/meta/reviews/prs/2026-06-10-pr-100-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-10-pr-100-review"
date: "2026-06-10T00:00:00+00:00"
author: Toby
status: complete
pr_title: "Folded From PR Title"
review_pass: 1
verdict: approve
lenses: ["correctness"]
review_number: 1
pr_number: 100
tags: []
last_updated: "2026-06-10T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 100 Review
EOF

# B: pr_title (non-empty, DIFFERING) + existing title -> pr_title dropped,
# title unchanged, DIVERGE[discarded-key].
cat >"$P2/meta/reviews/prs/2026-06-11-pr-101-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-11-pr-101-review"
title: "Real Title"
date: "2026-06-11T00:00:00+00:00"
author: Toby
status: complete
pr_title: "Different PR Title"
review_pass: 1
verdict: approve
lenses: ["correctness"]
review_number: 1
pr_number: 101
tags: []
last_updated: "2026-06-11T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 101 Review
EOF

# C: pr_title EQUAL to existing title -> dropped, title unchanged (benign).
cat >"$P2/meta/reviews/prs/2026-06-13-pr-103-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-13-pr-103-review"
title: "Same Title"
date: "2026-06-13T00:00:00+00:00"
author: Toby
status: complete
pr_title: "Same Title"
review_pass: 1
verdict: approve
lenses: ["correctness"]
review_number: 1
pr_number: 103
tags: []
last_updated: "2026-06-13T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 103 Review
EOF

# D: pr_title: "" (empty) + NO title -> drops cleanly (no title: "" fold), the
# stem-derived title_default supplies the title.
cat >"$P2/meta/reviews/prs/2026-06-12-pr-102-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-12-pr-102-review"
date: "2026-06-12T00:00:00+00:00"
author: Toby
status: complete
pr_title: ""
review_pass: 1
verdict: approve
lenses: ["correctness"]
review_number: 1
pr_number: 102
tags: []
last_updated: "2026-06-12T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 102 Review
EOF

git_init "$P2"
A2="$P2/meta/reviews/prs/2026-06-10-pr-100-review.md"
B2="$P2/meta/reviews/prs/2026-06-11-pr-101-review.md"
C2="$P2/meta/reviews/prs/2026-06-13-pr-103-review.md"
D2="$P2/meta/reviews/prs/2026-06-12-pr-102-review.md"

# Red step: pr_title + review_pass report FORBIDDEN-OWN-ID before the fix.
assert_violation "Phase 2 red: pr_title/review_pass are FORBIDDEN-OWN-ID" \
  "FORBIDDEN-OWN-ID" "$A2"

run_0007 "$P2"
assert_eq "Phase 2 corpus exits 0" "0" "$RUN_RC"

# A: both forbidden keys gone; pr_title folded into title.
assert_not_contains "Phase 2 A: pr_title removed" "$(cat "$A2")" "pr_title:"
assert_not_contains "Phase 2 A: review_pass removed" "$(cat "$A2")" "review_pass:"
assert_contains "Phase 2 A: pr_title folded into title" \
  "$(fm_line "$A2" title)" 'title: "Folded From PR Title"'
assert_eq "Phase 2 A: exactly one title line (folded)" "1" \
  "$(grep -c '^title:' "$A2")"

# B: pr_title dropped, title unchanged.
assert_not_contains "Phase 2 B: pr_title removed" "$(cat "$B2")" "pr_title:"
assert_contains "Phase 2 B: existing title unchanged" \
  "$(fm_line "$B2" title)" 'title: "Real Title"'

# C: pr_title == title -> dropped, title unchanged.
assert_not_contains "Phase 2 C: pr_title removed (equal value)" "$(cat "$C2")" "pr_title:"
assert_contains "Phase 2 C: title unchanged (equal value)" \
  "$(fm_line "$C2" title)" 'title: "Same Title"'

# D: empty pr_title drops; default title supplied; exactly one title; no placeholder.
assert_not_contains "Phase 2 D: empty pr_title removed" "$(cat "$D2")" "pr_title:"
assert_eq "Phase 2 D: exactly one title line (defaulted)" "1" \
  "$(grep -c '^title:' "$D2")"
assert_not_contains "Phase 2 D: no empty title placeholder" "$(cat "$D2")" 'title: ""'

assert_validates "Phase 2 corpus validates clean" "$P2/meta"

# Idempotency.
git -C "$P2" add -A && git -C "$P2" commit -q -m migrated >/dev/null 2>&1
run_0007 "$P2"
assert_empty "Phase 2 second run is an empty meta/ diff" \
  "$(git -C "$P2" status --porcelain meta/ || true)"

# Discarded-key breadcrumb: a non-empty pr_title dropped because a differing
# title: already exists is surfaced (asserted via a direct run, since the runner
# deletes the migration stderr on success).
echo "=== Phase 2: discarded-key breadcrumb (direct run) ==="
P2BC="$TMP/phase2-breadcrumb"
mkdir -p "$P2BC/meta/reviews/prs"
cat >"$P2BC/meta/reviews/prs/2026-06-11-pr-101-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-11-pr-101-review"
title: "Real Title"
date: "2026-06-11T00:00:00+00:00"
author: Toby
status: complete
pr_title: "Different PR Title"
review_pass: 1
verdict: approve
lenses: ["correctness"]
review_number: 1
pr_number: 101
tags: []
last_updated: "2026-06-11T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 101 Review
EOF
git_init "$P2BC"
run_0007_direct "$P2BC"
assert_eq "Phase 2 direct run exits 0 (valid after drop)" "0" "$DIRECT_RC"
assert_contains "Phase 2 discarded-key breadcrumb fired" \
  "$DIRECT_ERR" "0007-DIVERGE[discarded-key]"

# Schema-driven proof: a custom SCHEMA_TSV declaring a NOVEL forbidden key drops
# it (a hard-coded implementation would not). SCHEMA_TSV is exported so the
# migration's self-validation inherits the same custom schema.
echo "=== Phase 2: forbidden-key drop is schema-driven (custom SCHEMA_TSV) ==="
BOGUS="$TMP/phase2-bogus"
mkdir -p "$BOGUS/meta/notes"
SCHEMA_BOGUS="$TMP/schema-bogus.tsv"
awk -F'\t' 'BEGIN { OFS = "\t" } NR > 1 && $2 == "note" { $6 = "bogus_key" } { print }' \
  "$PLUGIN_ROOT/scripts/templates-schema.tsv" >"$SCHEMA_BOGUS"
cat >"$BOGUS/meta/notes/2026-06-01-a-note.md" <<'EOF'
---
type: note
id: "2026-06-01-a-note"
title: "A Note"
date: "2026-06-01T00:00:00+00:00"
author: Toby
producer: create-note
status: captured
topic: "A Note"
bogus_key: "should be dropped"
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# A Note
EOF
git_init "$BOGUS"
RC_BOGUS=0
(cd "$BOGUS" && PROJECT_ROOT="$BOGUS" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$ONLY_0007" ACCELERATOR_MIGRATE_FORCE=1 \
  SCHEMA_TSV="$SCHEMA_BOGUS" bash "$DRIVER" >/dev/null 2>&1 </dev/null) || RC_BOGUS=$?
assert_eq "Phase 2 schema-driven run exits 0" "0" "$RC_BOGUS"
assert_not_contains "Phase 2 schema-driven: novel forbidden bogus_key dropped" \
  "$(cat "$BOGUS/meta/notes/2026-06-01-a-note.md")" "bogus_key"

# Header assertion (halt): a column-REORDERED SCHEMA_TSV makes the migration exit
# non-zero with zero file mutations (the file would otherwise be rewritten).
echo "=== Phase 2: reordered schema halts the migration (zero mutations) ==="
REORD="$TMP/phase2-reorder"
mkdir -p "$REORD/meta/work"
cat >"$REORD/meta/work/0001-foo.md" <<'EOF'
---
type: work-item
work_item_id: "0001"
title: "Foo"
date: "2026-06-01"
author: Toby
skill: create-work-item
kind: story
priority: high
status: ready
parent: ""
---
# Foo
EOF
SCHEMA_REORD="$TMP/schema-reorder.tsv"
awk -F'\t' 'BEGIN { OFS = "\t" } { t = $3; $3 = $4; $4 = t; print }' \
  "$PLUGIN_ROOT/scripts/templates-schema.tsv" >"$SCHEMA_REORD"
git_init "$REORD"
before_reord="$(cat "$REORD/meta/work/0001-foo.md")"
RC_REORD=0
(cd "$REORD" && PROJECT_ROOT="$REORD" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$ONLY_0007" ACCELERATOR_MIGRATE_FORCE=1 \
  SCHEMA_TSV="$SCHEMA_REORD" bash "$DRIVER" >/dev/null 2>&1 </dev/null) || RC_REORD=$?
assert_neq "Phase 2 reordered schema: migration exits non-zero" "0" "$RC_REORD"
assert_eq "Phase 2 reordered schema: zero file mutations" "$before_reord" \
  "$(cat "$REORD/meta/work/0001-foo.md")"

# Header assertion (extension tolerated): a SCHEMA_TSV with an extra TRAILING
# column (canonical 7 unchanged) is accepted by both the migration and validator.
echo "=== Phase 2: trailing schema column extension tolerated ==="
EXT="$TMP/phase2-ext"
mkdir -p "$EXT/meta/work"
cat >"$EXT/meta/work/0002-bar.md" <<'EOF'
---
type: work-item
id: "0002"
title: "Bar"
date: "2026-06-01T00:00:00+00:00"
author: Toby
producer: create-work-item
kind: story
priority: high
status: ready
tags: []
last_updated: "2026-06-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Bar
EOF
SCHEMA_EXT="$TMP/schema-ext.tsv"
awk -F'\t' 'BEGIN { OFS = "\t" } { print $0 OFS (NR == 1 ? "future_col" : "") }' \
  "$PLUGIN_ROOT/scripts/templates-schema.tsv" >"$SCHEMA_EXT"
git_init "$EXT"
RC_EXT=0
(cd "$EXT" && PROJECT_ROOT="$EXT" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$ONLY_0007" ACCELERATOR_MIGRATE_FORCE=1 \
  SCHEMA_TSV="$SCHEMA_EXT" bash "$DRIVER" >/dev/null 2>&1 </dev/null) || RC_EXT=$?
assert_eq "Phase 2 extended schema: migration accepts (exits 0)" "0" "$RC_EXT"
RC_EXTV=0
(SCHEMA_TSV="$SCHEMA_EXT" "$VALIDATOR" "$EXT/meta" >/dev/null 2>&1) || RC_EXTV=$?
assert_eq "Phase 2 extended schema: validator accepts (exits 0)" "0" "$RC_EXTV"

# ── Phase 3: unconditional ticket/ticket_id drop ────────────────────────────
echo "=== Phase 3: ticket/ticket_id dropped on any type ==="
M0001="$PLUGIN_ROOT/skills/config/migrate/migrations/0001-rename-tickets-to-work.sh"
P3="$TMP/phase3"
mkdir -p "$P3/meta/notes" "$P3/meta/work"

# A note carrying a hand-added external-tracker reference.
cat >"$P3/meta/notes/2026-06-01-noted.md" <<'EOF'
---
type: note
id: "2026-06-01-noted"
title: "A Noted Thing"
date: "2026-06-01T00:00:00+00:00"
author: Toby
producer: create-note
status: captured
topic: "A Noted Thing"
ticket: "PROJ-1234"
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# A Noted Thing
EOF

# A non-note type carrying ticket_id (regardless of value).
cat >"$P3/meta/work/0080-task.md" <<'EOF'
---
type: work-item
id: "0080"
title: "A Task"
date: "2026-06-01T00:00:00+00:00"
author: Toby
producer: create-work-item
kind: story
priority: high
status: ready
ticket_id: "LEGACY-9"
tags: []
last_updated: "2026-06-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# A Task
EOF

NOTED="$P3/meta/notes/2026-06-01-noted.md"
TASK="$P3/meta/work/0080-task.md"
git_init "$P3"

# Red step: the ticket-bearing note reports OBSOLETE-LEGACY-KEY before the fix.
assert_violation "Phase 3 red: ticket: is OBSOLETE-LEGACY-KEY" \
  "OBSOLETE-LEGACY-KEY" "$NOTED"

run_0007 "$P3"
assert_eq "Phase 3 corpus exits 0" "0" "$RUN_RC"
assert_not_contains "Phase 3 note: ticket removed" "$(cat "$NOTED")" "ticket:"
assert_not_contains "Phase 3 work-item: ticket_id removed" "$(cat "$TASK")" "ticket_id:"
assert_validates "Phase 3 corpus validates clean" "$P3/meta"

# Dropped-legacy-key breadcrumb on a non-empty value (direct run).
P3BC="$TMP/phase3-breadcrumb"
mkdir -p "$P3BC/meta/notes"
cat >"$P3BC/meta/notes/2026-06-01-noted.md" <<'EOF'
---
type: note
id: "2026-06-01-noted"
title: "A Noted Thing"
date: "2026-06-01T00:00:00+00:00"
author: Toby
producer: create-note
status: captured
topic: "A Noted Thing"
ticket: "PROJ-1234"
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# A Noted Thing
EOF
git_init "$P3BC"
run_0007_direct "$P3BC"
assert_eq "Phase 3 direct run exits 0" "0" "$DIRECT_RC"
assert_contains "Phase 3 dropped-legacy-key breadcrumb fired" \
  "$DIRECT_ERR" "0007-DIVERGE[dropped-legacy-key]"

# Integration (0001 -> 0007, same session): a meta/tickets/ file with ticket_id
# run through a dir containing BOTH migrations lands as meta/work/ with id:.
echo "=== Phase 3: 0001 -> 0007 same-session integration ==="
BOTH="$TMP/both-0001-0007"
mkdir -p "$BOTH"
cp "$M0001" "$BOTH/"
cp "$MIGRATION" "$BOTH/"
TIX="$TMP/tickets"
mkdir -p "$TIX/meta/tickets"
cat >"$TIX/meta/tickets/0070-foo.md" <<'EOF'
---
type: work-item
ticket_id: "0070"
title: "Foo Ticket"
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
# Foo Ticket
EOF
git_init "$TIX"
TIX_RC=0
(cd "$TIX" && PROJECT_ROOT="$TIX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$BOTH" ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" >/dev/null 2>&1 </dev/null) || TIX_RC=$?
assert_eq "Phase 3 two-migration run exits 0" "0" "$TIX_RC"
assert_file_exists "Phase 3 ticket landed under meta/work/" "$TIX/meta/work/0070-foo.md"
assert_dir_not_exists "Phase 3 meta/tickets/ removed" "$TIX/meta/tickets"
assert_contains "Phase 3 work_item_id -> id" \
  "$(fm_line "$TIX/meta/work/0070-foo.md" id)" 'id: "0070"'
assert_not_contains "Phase 3 no ticket_id survives the sequence" \
  "$(cat "$TIX/meta/work/0070-foo.md")" "ticket_id:"
assert_validates "Phase 3 two-migration corpus validates" "$TIX/meta"

# Combined idempotency: re-running BOTH migrations DIRECTLY (the runner ledger
# would otherwise skip applied migrations) over the migrated corpus is a no-op.
git -C "$TIX" add -A && git -C "$TIX" commit -q -m migrated >/dev/null 2>&1
(cd "$TIX" && PROJECT_ROOT="$TIX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  bash "$M0001" >/dev/null 2>&1) || true
run_0007_direct "$TIX"
assert_empty "Phase 3 combined second pass is an empty meta/ diff" \
  "$(git -C "$TIX" status --porcelain meta/ || true)"

# Integration (0001 pre-applied, cross-session): only 0007 runs over an
# already-renamed meta/work/ corpus; the drop is a no-op and the run idempotent.
echo "=== Phase 3: 0001 pre-applied, 0007-alone cross-session ==="
XS="$TMP/cross-session"
mkdir -p "$XS/meta/work" "$XS/.accelerator/state"
cat >"$XS/meta/work/0071-bar.md" <<'EOF'
---
type: work-item
work_item_id: "0071"
title: "Bar"
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
# Bar
EOF
printf '0001-rename-tickets-to-work\n' >"$XS/.accelerator/state/migrations-applied"
git_init "$XS"
run_0007 "$XS"
assert_eq "Phase 3 cross-session run exits 0" "0" "$RUN_RC"
assert_contains "Phase 3 cross-session work_item_id -> id" \
  "$(fm_line "$XS/meta/work/0071-bar.md" id)" 'id: "0071"'
assert_not_contains "Phase 3 cross-session no ticket_id" \
  "$(cat "$XS/meta/work/0071-bar.md")" "ticket_id:"
git -C "$XS" add -A && git -C "$XS" commit -q -m migrated >/dev/null 2>&1
run_0007_direct "$XS"
assert_empty "Phase 3 cross-session second run is an empty meta/ diff" \
  "$(git -C "$XS" status --porcelain meta/ || true)"

# ── Phase 4: required-extras backfill ────────────────────────────────────────
echo "=== Phase 4: required type-extras backfilled (topic/pr_number/...) ==="
P4="$TMP/phase4"
mkdir -p "$P4/meta/notes" "$P4/meta/research/codebase" "$P4/meta/reviews/prs"

# N: fenced note, has title, NO topic.
cat >"$P4/meta/notes/2026-06-20-a-fenced-note.md" <<'EOF'
---
type: note
id: "2026-06-20-a-fenced-note"
title: "A Fenced Note"
date: "2026-06-20T00:00:00+00:00"
author: Toby
producer: create-note
status: captured
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# A Fenced Note
EOF

# R: fenced codebase-research, HAS title, NO topic (proves topic derives from the
# existing title, not the empty title_default).
cat >"$P4/meta/research/codebase/2026-06-20-some-research.md" <<'EOF'
---
type: codebase-research
id: "2026-06-20-some-research"
title: "Some Research Topic"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Some Research Topic
EOF

# RV: fenced pr-review carrying EMPTY placeholders (lenses: [] / verdict: "").
cat >"$P4/meta/reviews/prs/2026-06-20-pr-200-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-20-pr-200-review"
title: "PR 200 Review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
verdict: ""
lenses: []
review_number: 1
pr_number: 200
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 200 Review
EOF

# PR430: fenced pr-review missing pr_number/review_number/verdict/lenses.
cat >"$P4/meta/reviews/prs/2026-06-17-pr-430-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-17-pr-430-review"
title: "PR 430 Review"
date: "2026-06-17T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-17T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 430 Review
EOF

# EX: fenced note that ALREADY carries topic (must not be overwritten).
cat >"$P4/meta/notes/2026-06-20-has-topic.md" <<'EOF'
---
type: note
id: "2026-06-20-has-topic"
title: "Has Topic"
date: "2026-06-20T00:00:00+00:00"
author: Toby
producer: create-note
status: captured
topic: "Existing Topic"
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Has Topic
EOF

# SEMI: fenced research whose title carries a ';' (US-channel must not truncate).
cat >"$P4/meta/research/codebase/2026-06-20-semicolon.md" <<'EOF'
---
type: codebase-research
id: "2026-06-20-semicolon"
title: "Add caching; drop the old path"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Add caching; drop the old path
EOF

# QFENCED: fenced research with NO title and a quote-bearing H1 (title_default +
# topic must both be quote-free).
cat >"$P4/meta/research/codebase/2026-06-20-quoted-fenced.md" <<'EOF'
---
type: codebase-research
id: "2026-06-20-quoted-fenced"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Quoted "Research" Title
EOF

# FL: fence-less codebase-research (exercises the widened fence-less topic seed).
printf '# Fence-less Research\n\nA pre-frontmatter research note.\n' \
  >"$P4/meta/research/codebase/2026-06-20-fenceless.md"

# QF: fence-less note with a quote-bearing H1 (title + topic must be quote-free).
printf '# A "Quoted" Note\n\nObservation.\n' \
  >"$P4/meta/notes/2026-06-20-quoted.md"

# NODEFAULT: fenced pr-review whose stem carries no derivable PR number (no pr-
# token, date-prefixed stem) — exercises the no-derivable-default backfill
# end-to-end through the runner's self_validate_structural gate (0118).
cat >"$P4/meta/reviews/prs/2026-06-20-dateonly-pr-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-20-dateonly-pr-review"
title: "Date Only PR Review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Date Only PR Review
EOF

# WIDENING: fenced plan-validation missing its required `result` extra —
# exercises the no-derivable-default backfill for a non-pr_number, non-derivable
# extra, proving the loop-level fix covers every required extra (0118).
mkdir -p "$P4/meta/validations" # P4 setup only creates meta/reviews/prs etc.
cat >"$P4/meta/validations/2026-06-20-widening-validation.md" <<'EOF'
---
type: plan-validation
id: "2026-06-20-widening-validation"
title: "Widening Validation"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Widening Validation
EOF

# HYBRID: design-inventory missing every required extra — sequence/
# screenshots_incomplete get typed bare defaults; the string/enum bundle
# (source/source_kind/source_location/crawler) gets the quoted unknown sentinel.
mkdir -p "$P4/meta/research/design-inventories"
cat >"$P4/meta/research/design-inventories/2026-06-20-hybrid-inventory.md" <<'EOF'
---
type: design-inventory
id: "2026-06-20-hybrid-inventory"
title: "Hybrid Inventory"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: draft
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Hybrid Inventory
EOF

# REVIEWPASS: plan-review missing its required review_pass — exercises the third
# numeric typed default end-to-end (must emit bare review_pass: 1, not "1") (0118).
mkdir -p "$P4/meta/reviews/plans"
cat >"$P4/meta/reviews/plans/2026-06-20-reviewpass-review.md" <<'EOF'
---
type: plan-review
id: "2026-06-20-reviewpass-review"
title: "Review Pass Review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Review Pass Review
EOF

# INCIDENT (0120 AC2): a pr-description whose filename carries an external
# tracker key (ENG-1234-description). `pr_number` has no derivable default — the
# stem has no pr/PR segment and is not numeric-leading, so the leading-numeric
# fallback returns empty (the date-prefix exclusion is never even reached) — so
# the backfill stamps the bare `unknown` sentinel on this one required extra.
# This is the exact file shape from the 0115 incident; the cross-check proves
# what the tolerant backfill emits is a state the validator accepts.
mkdir -p "$P4/meta/prs"
cat >"$P4/meta/prs/ENG-1234-description.md" <<'EOF'
---
type: pr-description
id: "ENG-1234-description"
title: "Tracker Keyed PR Description"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Tracker Keyed PR Description
EOF

# COUNTER (0120 AC2): a derivable pr-description — `pr-42-description` matches
# the `(^|-)[Pp][Rr]-?[0-9]+` segment, so pr_number derives to 42 and is NOT
# sentinel-replaced. Guards the boundary: the sentinel fires only where the
# value is genuinely underivable.
cat >"$P4/meta/prs/pr-42-description.md" <<'EOF'
---
type: pr-description
id: "pr-42-description"
title: "Derivable PR Description"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Derivable PR Description
EOF

git_init "$P4"

# Red step: the fenced note missing topic reports MISSING-EXTRA before the fix.
assert_violation "Phase 4 red: fenced note missing topic is MISSING-EXTRA" \
  "MISSING-EXTRA" "$P4/meta/notes/2026-06-20-a-fenced-note.md"

run_0007 "$P4"
assert_eq "Phase 4 corpus exits 0" "0" "$RUN_RC"

assert_contains "Phase 4 N: topic backfilled from title" \
  "$(fm_line "$P4/meta/notes/2026-06-20-a-fenced-note.md" topic)" 'topic: "A Fenced Note"'
assert_contains "Phase 4 R: topic from EXISTING title (not empty default)" \
  "$(fm_line "$P4/meta/research/codebase/2026-06-20-some-research.md" topic)" 'topic: "Some Research Topic"'
assert_contains "Phase 4 RV: empty verdict placeholder backfilled" \
  "$(fm_line "$P4/meta/reviews/prs/2026-06-20-pr-200-review.md" verdict)" 'verdict: "unknown"'
assert_contains "Phase 4 RV: empty lenses placeholder backfilled" \
  "$(fm_line "$P4/meta/reviews/prs/2026-06-20-pr-200-review.md" lenses)" 'lenses: ["unknown"]'
PR430F="$P4/meta/reviews/prs/2026-06-17-pr-430-review.md"
assert_contains "Phase 4 PR430: pr_number from pr-token (not the year)" \
  "$(fm_line "$PR430F" pr_number)" 'pr_number: 430'
assert_contains "Phase 4 PR430: review_number defaulted to 1" \
  "$(fm_line "$PR430F" review_number)" 'review_number: 1'
assert_contains "Phase 4 PR430: verdict sentinel" \
  "$(fm_line "$PR430F" verdict)" 'verdict: "unknown"'
assert_contains "Phase 4 PR430: lenses sentinel" \
  "$(fm_line "$PR430F" lenses)" 'lenses: ["unknown"]'
assert_contains "Phase 4 EX: existing topic NOT overwritten" \
  "$(fm_line "$P4/meta/notes/2026-06-20-has-topic.md" topic)" 'topic: "Existing Topic"'
assert_contains "Phase 4 SEMI: ';'-bearing title survives intact" \
  "$(fm_line "$P4/meta/research/codebase/2026-06-20-semicolon.md" topic)" 'topic: "Add caching; drop the old path"'
assert_eq "Phase 4 QFENCED: title quote-free" 'title: "Quoted Research Title"' \
  "$(fm_line "$P4/meta/research/codebase/2026-06-20-quoted-fenced.md" title)"
assert_eq "Phase 4 QFENCED: topic quote-free" 'topic: "Quoted Research Title"' \
  "$(fm_line "$P4/meta/research/codebase/2026-06-20-quoted-fenced.md" topic)"
assert_contains "Phase 4 FL: fence-less research topic seeded" \
  "$(fm_line "$P4/meta/research/codebase/2026-06-20-fenceless.md" topic)" 'topic: "Fence-less Research"'
assert_eq "Phase 4 QF: fence-less title quote-free" 'title: "A Quoted Note"' \
  "$(fm_line "$P4/meta/notes/2026-06-20-quoted.md" title)"
assert_eq "Phase 4 QF: fence-less topic quote-free" 'topic: "A Quoted Note"' \
  "$(fm_line "$P4/meta/notes/2026-06-20-quoted.md" topic)"

NODEF="$P4/meta/reviews/prs/2026-06-20-dateonly-pr-review.md"
assert_contains "Phase 4 NODEFAULT: underivable pr_number -> unknown sentinel" \
  "$(fm_line "$NODEF" pr_number)" 'pr_number: unknown'
WIDEN="$P4/meta/validations/2026-06-20-widening-validation.md"
assert_contains "Phase 4 WIDENING: underivable result -> unknown sentinel" \
  "$(fm_line "$WIDEN" result)" 'result: "unknown"'

# Derivable-path no-regression: a pr-token stem still derives the real number, so
# the sentinel must NOT be applied where a value is derivable (AC #4).
assert_not_contains "Phase 4 PR430: derivable pr_number NOT replaced by sentinel" \
  "$(fm_line "$PR430F" pr_number)" 'unknown'

INV="$P4/meta/research/design-inventories/2026-06-20-hybrid-inventory.md"
# Numeric/boolean typed defaults: assert the EXACT bare line (assert_eq, not a
# substring) so a quoted "1"/"true" type regression is caught — `sequence: 1`
# must not pass for `sequence: 10` either.
assert_eq "Phase 4 HYBRID: sequence -> bare typed default (not quoted)" \
  'sequence: 1' "$(fm_line "$INV" sequence)"
assert_eq "Phase 4 HYBRID: screenshots_incomplete -> bare bool (not quoted)" \
  'screenshots_incomplete: true' "$(fm_line "$INV" screenshots_incomplete)"
# String/enum bundle: quoted unknown sentinel (the whole bundle routes the same
# way, so pin all four to catch an extras_for_type/optional-carve-out regression).
assert_contains "Phase 4 HYBRID: source -> quoted unknown sentinel (string)" \
  "$(fm_line "$INV" source)" 'source: "unknown"'
assert_contains "Phase 4 HYBRID: source_kind -> quoted unknown sentinel" \
  "$(fm_line "$INV" source_kind)" 'source_kind: "unknown"'
assert_contains "Phase 4 HYBRID: source_location -> quoted unknown sentinel" \
  "$(fm_line "$INV" source_location)" 'source_location: "unknown"'
assert_contains "Phase 4 HYBRID: crawler -> quoted unknown sentinel" \
  "$(fm_line "$INV" crawler)" 'crawler: "unknown"'

RP="$P4/meta/reviews/plans/2026-06-20-reviewpass-review.md"
assert_eq "Phase 4 REVIEWPASS: review_pass -> bare typed default (not quoted)" \
  'review_pass: 1' "$(fm_line "$RP" review_pass)"

# ── 0120 AC2: backfill↔validator cross-check (incident-shaped fixture) ──
INCIDENT="$P4/meta/prs/ENG-1234-description.md"
# Only pr_number is a genuinely REQUIRED extra for pr-description, so the
# underivable tracker-key stem gets the BARE `unknown` sentinel. pr_url and
# merge_commit are in FM_OPTIONAL_EXTRAS (frontmatter-emission-rules.sh:74), so
# the backfill loop skips them (0007:510) and they stay ABSENT — which the
# validator accepts (MISSING-EXTRA also skips optional extras). Assert both: the
# present bare sentinel on the required extra, and the benign absence of the
# optional pair (no quoted "unknown" is ever written for them).
assert_contains "Phase 4 INCIDENT: tracker-key pr_number -> bare unknown sentinel" \
  "$(fm_line "$INCIDENT" pr_number)" 'pr_number: unknown'
# Assert absence against the WHOLE migrated file (not fm_line, which returns
# empty for an absent key and would pass vacuously) so a stray quoted-`unknown`
# stamping anywhere in the frontmatter is caught — the `assert_not_contains
# "$(cat …)"` idiom already used at :164,:169-170.
assert_not_contains "Phase 4 INCIDENT: optional pr_url left absent (not stamped)" \
  "$(cat "$INCIDENT")" 'pr_url:'
assert_not_contains "Phase 4 INCIDENT: optional merge_commit left absent (not stamped)" \
  "$(cat "$INCIDENT")" 'merge_commit:'
# AC2 names the regex `FAIL:.*MISSING-EXTRA`, but it matches NO single validator
# line: violations print `<file>: MISSING-EXTRA — <msg>` (no FAIL: prefix) and
# the only FAIL: line is the codeless summary. So assert the MEANINGFUL
# equivalent: the validator emits no MISSING-EXTRA token over the migrated
# incident fixture, proving the present `unknown` is an accepted state rather
# than a tolerated-but-rejected one. Exit-0 acceptance of this fixture is
# already covered by the corpus-wide `assert_validates "$P4/meta"` below (which
# now includes it), so no separate per-file assert_validates is needed.
INCIDENT_VOUT="$("$VALIDATOR" "$INCIDENT" 2>&1)" || true
assert_not_contains "Phase 4 INCIDENT: no MISSING-EXTRA for present sentinel" \
  "$INCIDENT_VOUT" "MISSING-EXTRA"

# ── 0120 AC2 counter: derivable pr_number is NOT sentinel-replaced ──
# Guards the pr-description / meta/prs/ derivation path specifically — distinct
# from the existing PR430 block (:1264-1298), which proves the same boundary for
# a pr-review under meta/reviews/prs/. Exact-equality positive form (not a bare
# substring) so a regression that DROPPED the line entirely also fails rather
# than passing a vacuous not_contains.
COUNTER="$P4/meta/prs/pr-42-description.md"
assert_eq "Phase 4 COUNTER: derivable pr_number from pr- segment" \
  'pr_number: 42' "$(fm_line "$COUNTER" pr_number)"
assert_not_contains "Phase 4 COUNTER: derivable pr_number NOT sentinel-replaced" \
  "$(fm_line "$COUNTER" pr_number)" 'unknown'

assert_validates "Phase 4 corpus validates clean" "$P4/meta"

# Idempotency.
git -C "$P4" add -A && git -C "$P4" commit -q -m migrated >/dev/null 2>&1
run_0007 "$P4"
assert_empty "Phase 4 second run is an empty meta/ diff" \
  "$(git -C "$P4" status --porcelain meta/ || true)"

# Breadcrumbs (direct run): sentinel backfill emits backfilled-extra; a numberless
# pr-review's underivable pr_number now gets the `unknown` sentinel via the loop's
# no-derivable-default branch (emitting a backfill-sentinel breadcrumb, not the
# removed missing-extra-no-default), so the migration runs to completion under
# set -euo pipefail rather than aborting at self_validate_structural (0118).
echo "=== Phase 4: backfill breadcrumbs + no-default (direct run) ==="
P4BC="$TMP/phase4-breadcrumb"
mkdir -p "$P4BC/meta/reviews/prs"
cat >"$P4BC/meta/reviews/prs/no-pr-number-review.md" <<'EOF'
---
type: pr-review
id: "no-pr-number-review"
title: "Numberless Review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Numberless Review
EOF
git_init "$P4BC"
run_0007_direct "$P4BC"
assert_contains "Phase 4 backfilled-extra breadcrumb fired (verdict/lenses)" \
  "$DIRECT_ERR" "0007-DIVERGE[backfilled-extra]"
# Post-fix: pr_number gets the unknown sentinel via the loop's
# no-derivable-default branch, so NO extra is left absent and the migration
# completes without aborting at self_validate_structural. The loop emits a
# backfill-sentinel breadcrumb (not the removed missing-extra-no-default) (0118).
assert_eq "Phase 4 numberless review: direct run completes (no abort)" \
  "0" "$DIRECT_RC"
assert_contains "Phase 4 backfill-sentinel breadcrumb fired (pr_number)" \
  "$DIRECT_ERR" "0007-DIVERGE[backfill-sentinel]"
# Verify the per-file AND per-extra audit contract, not just the family tag (the
# Migration Notes recovery story depends on knowing which file AND which field).
assert_contains "Phase 4 backfill-sentinel breadcrumb names the file" \
  "$DIRECT_ERR" "no-pr-number-review.md"
assert_contains "Phase 4 backfill-sentinel breadcrumb names the extra" \
  "$DIRECT_ERR" "required extra 'pr_number'"
assert_not_contains "Phase 4 no missing-extra-no-default (reconciled)" \
  "$DIRECT_ERR" "missing-extra-no-default"
# Completion-via-sentinel: review_number/verdict were still backfilled, and the
# run completed cleanly through the sentinel rather than partially mutating then
# aborting.
assert_contains "Phase 4 numberless review still backfilled (run completes)" \
  "$(fm_line "$P4BC/meta/reviews/prs/no-pr-number-review.md" review_number)" 'review_number: 1'
assert_contains "Phase 4 numberless review: pr_number -> unknown sentinel" \
  "$(fm_line "$P4BC/meta/reviews/prs/no-pr-number-review.md" pr_number)" \
  'pr_number: unknown'

# Populated lists are not clobbered and emit NO backfill breadcrumb.
echo "=== Phase 4: populated extras not clobbered (direct run) ==="
P4POP="$TMP/phase4-populated"
mkdir -p "$P4POP/meta/reviews/prs"
cat >"$P4POP/meta/reviews/prs/2026-06-20-pr-209-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-20-pr-209-review"
title: "PR 209 Review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
verdict: approve
lenses: ["security", "performance"]
review_number: 2
pr_number: 209
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 209 Review
EOF
POPF="$P4POP/meta/reviews/prs/2026-06-20-pr-209-review.md"
POP_BEFORE="$(cat "$POPF")"
git_init "$P4POP"
run_0007_direct "$P4POP"
assert_eq "Phase 4 direct run on populated review exits 0" "0" "$DIRECT_RC"
assert_eq "Phase 4 populated multi-element lenses byte-unchanged" "$POP_BEFORE" "$(cat "$POPF")"
assert_not_contains "Phase 4 no backfilled-extra breadcrumb on populated review" \
  "$DIRECT_ERR" "0007-DIVERGE[backfilled-extra]"

# Packed-channel parser edge cases (awk BEGIN{} probe over the actual system awk).
echo "=== Phase 4: packed-channel parser edge cases (awk probe) ==="
BF_PROBE="$TMP/bf-probe.awk"
cat >"$BF_PROBE" <<'AWK'
BEGIN { emit_backfill_extras(packed) }
AWK
US=$(printf '\037')
bf_empty="$(awk -f "$FRAG" -f "$BODY" -f "$BF_PROBE" -v packed="" </dev/null 2>/dev/null)"
bf_single="$(awk -f "$FRAG" -f "$BODY" -f "$BF_PROBE" -v packed="topic=Solo" </dev/null 2>/dev/null)"
bf_multi="$(awk -f "$FRAG" -f "$BODY" -f "$BF_PROBE" -v packed="topic=a=b${US}review_number=2" </dev/null 2>/dev/null)"
bf_space="$(awk -f "$FRAG" -f "$BODY" -f "$BF_PROBE" -v packed="topic=has space here" </dev/null 2>/dev/null)"
assert_empty "Phase 4 packed-probe: empty channel emits nothing" "$bf_empty"
assert_eq "Phase 4 packed-probe: single record" 'topic: "Solo"' "$bf_single"
assert_eq "Phase 4 packed-probe: =-in-value split on first = + multi record" \
  "$(printf 'topic: "a=b"\nreview_number: 2')" "$bf_multi"
assert_eq "Phase 4 packed-probe: space-bearing value intact" 'topic: "has space here"' "$bf_space"

# Pure-helper micro-assertions + frozen-migration required-extra contract guard
# (sources the migration with the test-only ACCELERATOR_0007_NO_RUN seam so the
# REAL functions are exercised, not a drift-prone re-derivation).
echo "=== Phase 4: pure-helper micro-assertions + contract guard ==="
micro_rc=0
micro_out="$(ACCELERATOR_0007_NO_RUN=1 PROJECT_ROOT="$TMP" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  SCHEMA_TSV="$PLUGIN_ROOT/scripts/templates-schema.tsv" bash -c '
    source "$0"
    fail() { echo "MICRO-FAIL: $1"; exit 1; }
    [ "$(extra_default pr_number "" 2026-06-17-pr-416-review "")" = "416" ] || fail pr416
    [ "$(extra_default pr_number "" 2026-06-17-summary "")" = "" ] || fail summary
    [ "$(extra_default pr_number "" 2026-06-17-0114-foo "")" = "" ] || fail dateid
    [ "$(extra_default pr_number "" 240-description "")" = "240" ] || fail bare240
    [ "$(extra_default pr_number "" expr-3-foo "")" = "" ] || fail expr3
    [ "$(forbidden_keys_for_type pr-review)" = "pr_title review_pass" ] || fail forbidden
    case " $(extras_for_type pr-review) " in *" pr_number "*) : ;; *) fail extras_prnum ;; esac
    fm_is_empty_val "[]" || fail emptylist
    fm_is_empty_val "\"\"" || fail emptystr
    fm_is_empty_val x && fail nonempty
    # Frozen-migration contract guard: the derived required set still carries the
    # extras 0007 backfills (fails loudly if a schema edit moves one to optional).
    required_set() {
      local out="" e
      for e in $(extras_for_type "$1"); do
        case " $FM_OPTIONAL_EXTRAS " in *" $e "*) continue ;; esac
        out="$out $e"
      done
      printf "%s" "$out"
    }
    for x in verdict lenses review_number pr_number; do
      case " $(required_set pr-review) " in *" $x "*) : ;; *) fail "contract_prreview_$x" ;; esac
    done
    for t in note codebase-research issue-research; do
      case " $(required_set "$t") " in *" topic "*) : ;; *) fail "contract_topic_$t" ;; esac
    done
    echo OK
  ' "$MIGRATION" 2>&1)" || micro_rc=$?
assert_eq "Phase 4 pure-helper + contract micro-assertions pass" "0" "$micro_rc"
assert_contains "Phase 4 micro battery reached OK" "$micro_out" "OK"

# ── Phase 5: non-canonical PR-reference linkage coercion ─────────────────────
echo "=== Phase 5: PR #N / #N / PR-N linkage tokens coerced to pr:N ==="
P5="$TMP/phase5"
mkdir -p "$P5/meta/plans" "$P5/meta/reviews/prs"

# Referenced artifacts so F_mixed's typed refs resolve in whole-corpus mode.
cat >"$P5/meta/plans/2026-05-13-0055-feature.md" <<'EOF'
---
type: plan
id: "2026-05-13-0055-feature"
title: "Feature Plan"
date: "2026-05-13T00:00:00+00:00"
author: Toby
producer: create-plan
status: done
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-05-13T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Feature Plan
EOF

cat >"$P5/meta/reviews/prs/2026-06-17-pr-430-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-17-pr-430-review"
title: "PR 430 Review"
date: "2026-06-17T00:00:00+00:00"
author: Toby
status: complete
verdict: approve
lenses: ["correctness"]
review_number: 1
pr_number: 430
tags: []
last_updated: "2026-06-17T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 430 Review
EOF

# F_combo: target "PR #416" + a relates_to list mixing every spelling variant.
cat >"$P5/meta/reviews/prs/2026-06-20-pr-500-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-20-pr-500-review"
title: "PR 500 Review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
verdict: approve
lenses: ["correctness"]
review_number: 1
pr_number: 500
target: "PR #416"
relates_to: ["PR#416", "pr #416", "PR-416", "pr-416", "#417"]
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 500 Review
EOF

# F_mixed: "PR #416" alongside two already-typed refs (must be left untouched).
cat >"$P5/meta/reviews/prs/2026-06-20-pr-504-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-20-pr-504-review"
title: "PR 504 Review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
verdict: approve
lenses: ["correctness"]
review_number: 1
pr_number: 504
relates_to: ["PR #416", "plan:2026-05-13-0055-feature", "pr-review:2026-06-17-pr-430-review"]
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 504 Review
EOF

# F_idem: already-canonical pr:N (must be byte-unchanged / not re-grabbed).
cat >"$P5/meta/reviews/prs/2026-06-20-pr-505-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-20-pr-505-review"
title: "PR 505 Review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
verdict: approve
lenses: ["correctness"]
review_number: 1
pr_number: 505
target: "pr:416"
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 505 Review
EOF

COMBO="$P5/meta/reviews/prs/2026-06-20-pr-500-review.md"
MIXED="$P5/meta/reviews/prs/2026-06-20-pr-504-review.md"
IDEM="$P5/meta/reviews/prs/2026-06-20-pr-505-review.md"
IDEM_BEFORE="$(cat "$IDEM")"
git_init "$P5"

# Red step: a non-canonical "PR #N" reports BAD-LINKAGE-SHAPE before the fix.
assert_violation "Phase 5 red: 'PR #416' is BAD-LINKAGE-SHAPE" \
  "BAD-LINKAGE-SHAPE" "$COMBO"

run_0007 "$P5"
assert_eq "Phase 5 corpus exits 0" "0" "$RUN_RC"
assert_contains "Phase 5 target 'PR #416' -> 'pr:416'" \
  "$(fm_line "$COMBO" target)" 'target: "pr:416"'
assert_contains "Phase 5 spelling variants + #N all coerced in a list" \
  "$(fm_line "$COMBO" relates_to)" 'relates_to: ["pr:416", "pr:416", "pr:416", "pr:416", "pr:417"]'
assert_contains "Phase 5 mixed: 'PR #416' coerced" \
  "$(fm_line "$MIXED" relates_to)" '"pr:416"'
assert_contains "Phase 5 mixed: typed plan ref untouched" \
  "$(fm_line "$MIXED" relates_to)" '"plan:2026-05-13-0055-feature"'
assert_contains "Phase 5 mixed: embedded pr-NNN typed ref untouched" \
  "$(fm_line "$MIXED" relates_to)" '"pr-review:2026-06-17-pr-430-review"'
assert_eq "Phase 5 already-canonical pr:416 byte-unchanged (no re-grab)" \
  "$IDEM_BEFORE" "$(cat "$IDEM")"
assert_validates "Phase 5 corpus validates clean" "$P5/meta"

# Idempotency.
git -C "$P5" add -A && git -C "$P5" commit -q -m migrated >/dev/null 2>&1
run_0007 "$P5"
assert_empty "Phase 5 second run is an empty meta/ diff" \
  "$(git -C "$P5" status --porcelain meta/ || true)"

# ── Phase 6: combined-corpus capstone ────────────────────────────────────────
# Seed one repo carrying EVERY reproduction shape (gaps 1-6 + the Phase 5
# linkage coercion + the ';'/quote edge cases), all authored so the mechanical
# passes leave the whole corpus validator-clean.
seed_combined() { # $1 = repo root
  local R="$1"
  mkdir -p "$R/meta/prs" "$R/meta/notes" "$R/meta/work" \
    "$R/meta/reviews/prs" "$R/meta/research/codebase" "$R/meta/docs"

  # gap 1 (empty type -> pr-description) + gap 5 (PR #N -> pr:N).
  cat >"$R/meta/prs/240-description.md" <<'EOF'
---
type:
id: "240-description"
title: "PR 240 Description"
date: "2026-06-01T00:00:00+00:00"
author: Toby
status: complete
relates_to: ["PR #416"]
pr_number: 240
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 240 Description
EOF

  # gap 3 (ticket dropped) + gap 4 (topic backfill) + ';'-in-title edge case.
  cat >"$R/meta/notes/2026-06-20-ticketed.md" <<'EOF'
---
type: note
id: "2026-06-20-ticketed"
title: "Ticketed; with semicolon"
date: "2026-06-20T00:00:00+00:00"
author: Toby
producer: create-note
status: captured
ticket: "PROJ-1234"
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Ticketed; with semicolon
EOF

  # gap 3 (ticket_id dropped on a non-note type).
  cat >"$R/meta/work/0080-task.md" <<'EOF'
---
type: work-item
id: "0080"
title: "A Task"
date: "2026-06-01T00:00:00+00:00"
author: Toby
producer: create-work-item
kind: story
priority: high
status: ready
ticket_id: "LEGACY-9"
tags: []
last_updated: "2026-06-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# A Task
EOF

  # gap 2 (pr_title discarded, title present) + review_pass dropped.
  cat >"$R/meta/reviews/prs/2026-06-20-pr-100-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-20-pr-100-review"
title: "Real Title"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
pr_title: "Different PR Title"
review_pass: 1
verdict: approve
lenses: ["correctness"]
review_number: 1
pr_number: 100
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 100 Review
EOF

  # gap 2 (pr_title folds into title when absent).
  cat >"$R/meta/reviews/prs/2026-06-20-pr-101-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-20-pr-101-review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
pr_title: "Folded Title"
review_pass: 1
verdict: approve
lenses: ["correctness"]
review_number: 1
pr_number: 101
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 101 Review
EOF

  # gap 4 (empty placeholders re-seeded).
  cat >"$R/meta/reviews/prs/2026-06-20-pr-102-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-20-pr-102-review"
title: "PR 102 Review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
verdict: ""
lenses: []
review_number: 1
pr_number: 102
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 102 Review
EOF

  # gap 4 (pr_number from a pr-token on a date-prefixed stem -> 430, not 2026;
  # review_number/verdict/lenses sentinels).
  cat >"$R/meta/reviews/prs/2026-06-17-pr-430-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-17-pr-430-review"
title: "PR 430 Review"
date: "2026-06-17T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-17T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# PR 430 Review
EOF

  # gap 4 (titled-but-topic-less research -> topic from existing title).
  cat >"$R/meta/research/codebase/2026-06-20-research.md" <<'EOF'
---
type: codebase-research
id: "2026-06-20-research"
title: "Some Research Topic"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Some Research Topic
EOF

  # gap 6 (freeform meta/docs/ skipped entirely).
  cat >"$R/meta/docs/logging-guide.md" <<'EOF'
---
title: Logging Guide
foo: bar
---
# Logging Guide

Freeform documentation the plugin does not own.
EOF
}

echo "=== Phase 6: combined-corpus capstone (validator-clean by construction) ==="
C="$TMP/combined"
seed_combined "$C"
git_init "$C"
run_0007 "$C"
assert_eq "Phase 6 combined corpus exits 0" "0" "$RUN_RC"
assert_contains "Phase 6 0007 recorded as applied (harness reached)" "$RUN_OUT" "applied"
assert_contains "Phase 6 ledger records 0007" \
  "$(cat "$C/.accelerator/state/migrations-applied" 2>/dev/null)" "0007-unify-meta-corpus-frontmatter"
assert_not_contains "Phase 6 prepass coexistence: no REFUSE on the multi-type corpus" \
  "$RUN_OUT" "0007-REFUSE"
assert_contains "Phase 6 empty-type pr typed pr-description" \
  "$(fm_line "$C/meta/prs/240-description.md" type)" "type: pr-description"
assert_contains "Phase 6 PR #416 coerced to pr:416" \
  "$(fm_line "$C/meta/prs/240-description.md" relates_to)" 'relates_to: ["pr:416"]'
assert_contains "Phase 6 ';'-title topic intact" \
  "$(fm_line "$C/meta/notes/2026-06-20-ticketed.md" topic)" 'topic: "Ticketed; with semicolon"'
assert_contains "Phase 6 pr-430 pr_number from token (not year)" \
  "$(fm_line "$C/meta/reviews/prs/2026-06-17-pr-430-review.md" pr_number)" 'pr_number: 430'
assert_validates "Phase 6 combined corpus validates clean (full whole-corpus)" "$C/meta"

# Combined idempotency.
git -C "$C" add -A && git -C "$C" commit -q -m migrated >/dev/null 2>&1
run_0007 "$C"
assert_empty "Phase 6 combined corpus second run is an empty meta/ diff" \
  "$(git -C "$C" status --porcelain meta/ || true)"

# Regression guard: the MECHANICAL passes alone (run_backfill + run_rewrite, NO
# harness) leave the corpus validator-clean — the core RCA fix. Driven via the
# ACCELERATOR_0007_NO_RUN sourcing seam so only the two pre-harness passes run.
echo "=== Phase 6: mechanical-passes-only regression guard ==="
RG="$TMP/regguard"
seed_combined "$RG"
git_init "$RG"
(
  export ACCELERATOR_0007_NO_RUN=1 PROJECT_ROOT="$RG" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT"
  # shellcheck disable=SC1090
  source "$MIGRATION"
  run_backfill
  run_rewrite
) >/dev/null 2>&1
assert_validates "Phase 6 run_backfill+run_rewrite alone -> zero violations" "$RG/meta"

# Broadened-namespace collision behaviour: two meta/prs/ files with the SAME
# post-rewrite id correctly REFUSE.
echo "=== Phase 6: pr-description namespace collision REFUSES ==="
COL="$TMP/collision"
mkdir -p "$COL/meta/prs"
cat >"$COL/meta/prs/a-description.md" <<'EOF'
---
type: pr-description
id: "dup-id"
title: "A"
date: "2026-06-01T00:00:00+00:00"
author: Toby
status: complete
pr_number: 1
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# A
EOF
cat >"$COL/meta/prs/b-description.md" <<'EOF'
---
type: pr-description
id: "dup-id"
title: "B"
date: "2026-06-01T00:00:00+00:00"
author: Toby
status: complete
pr_number: 2
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-01T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# B
EOF
git_init "$COL"
run_0007 "$COL"
assert_neq "Phase 6 colliding pr-description ids: run fails" "0" "$RUN_RC"
assert_contains "Phase 6 colliding pr-description ids: REFUSE emitted" "$RUN_OUT" "0007-REFUSE"

# A pr-description and a different-type artifact sharing a STEM do NOT collide
# (distinct typed refs).
echo "=== Phase 6: shared stem across types does NOT collide ==="
NOCOL="$TMP/no-collision"
mkdir -p "$NOCOL/meta/prs" "$NOCOL/meta/plans"
cat >"$NOCOL/meta/prs/2026-06-17-shared.md" <<'EOF'
---
type: pr-description
id: "2026-06-17-shared"
title: "Shared PR"
date: "2026-06-17T00:00:00+00:00"
author: Toby
status: complete
pr_number: 7
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-17T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Shared PR
EOF
cat >"$NOCOL/meta/plans/2026-06-17-shared.md" <<'EOF'
---
type: plan
id: "2026-06-17-shared"
title: "Shared Plan"
date: "2026-06-17T00:00:00+00:00"
author: Toby
producer: create-plan
status: done
tags: []
revision: "abc123"
repository: "accelerator"
last_updated: "2026-06-17T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Shared Plan
EOF
git_init "$NOCOL"
run_0007 "$NOCOL"
assert_eq "Phase 6 shared stem across types: run exits 0 (no false collision)" "0" "$RUN_RC"
assert_validates "Phase 6 shared-stem corpus validates" "$NOCOL/meta"

# ── Config-driven allowlist scope (doc-type table injection) ─────────────────
# These exercise the migration's resolve-and-inject path: default-layout
# byte-equivalence, custom-path config-aware typing, the pre-mutation fail-closed
# guard, CWD != PROJECT_ROOT, and the constant resolver-spawn count.

echo "=== Allowlist: default-layout byte-equivalence (golden) ==="
# A checked-in golden captured from the PRE-refactor migration over a fenced-only
# (VCS-independent) fixture; the post-change migration must reproduce it
# byte-for-byte, so the test cannot agree with a regressed implementation.
BE_FIX="$SCRIPT_DIR/test-fixtures/migrate-byte-equiv"
BE="$TMP/byte-equiv"
mkdir -p "$BE"
cp -R "$BE_FIX/input/." "$BE/"
git_init "$BE"
run_0007 "$BE"
assert_eq "byte-equiv corpus exits 0" "0" "$RUN_RC"
be_mismatch=""
while IFS= read -r gf; do
  rel="${gf#"$BE_FIX/golden/"}"
  cmp -s "$gf" "$BE/$rel" || be_mismatch="$be_mismatch $rel"
done < <(find "$BE_FIX/golden" -type f)
assert_empty "migrated tree is byte-identical to the pre-refactor golden" "$be_mismatch"
be_golden_n="$(find "$BE_FIX/golden/meta" -type f | wc -l | tr -d ' ')"
be_actual_n="$(find "$BE/meta" -type f | wc -l | tr -d ' ')"
assert_eq "no extra/missing files vs golden" "$be_golden_n" "$be_actual_n"
# Prepass parity: the go/no-go verdict on the default layout is unchanged (the
# config-aware out_of_scope/infer_type_from_path do not introduce a REFUSE).
assert_not_contains "default-layout prepass emits no REFUSE (parity)" "$RUN_OUT" "0007-REFUSE"

echo "=== Allowlist: custom paths.notes config-aware typing ==="
# paths.notes overridden to a non-default dir UNDER meta/. A fence-less file
# there is in scope AND typed; an equivalent file at the now-unconfigured
# default meta/notes/ is out of scope (byte-unchanged) — proving derivation is
# config-aware, not hardcoded.
CP="$TMP/custom-path"
mkdir -p "$CP/.accelerator" "$CP/meta/jottings" "$CP/meta/notes"
cat >"$CP/.accelerator/config.md" <<'EOF'
---
paths:
  notes: meta/jottings
---
EOF
printf '# A Custom Jotting\n\nObservation.\n' >"$CP/meta/jottings/2026-06-20-jotting.md"
printf '# A Default Note\n\nObservation.\n' >"$CP/meta/notes/2026-06-20-default.md"
CP_DEFAULT_BEFORE="$(cat "$CP/meta/notes/2026-06-20-default.md")"
git_init "$CP"
run_0007 "$CP"
assert_eq "custom-path corpus exits 0" "0" "$RUN_RC"
assert_contains "fence-less file under configured custom dir typed note" \
  "$(fm_line "$CP/meta/jottings/2026-06-20-jotting.md" type)" "type: note"
assert_eq "file at unconfigured default meta/notes byte-unchanged (out of scope)" \
  "$CP_DEFAULT_BEFORE" "$(cat "$CP/meta/notes/2026-06-20-default.md")"

echo "=== Allowlist: pre-mutation fail-closed guard ==="
guard_case() { # $1=label $2=bad-paths.work-value
  local G="$TMP/guard-$1"
  mkdir -p "$G/.accelerator" "$G/meta/work"
  cat >"$G/.accelerator/config.md" <<EOF
---
paths:
  work: $2
---
EOF
  cat >"$G/meta/work/0001-foo.md" <<'EOF'
---
type: work-item
work_item_id: "0001"
title: "Foo"
date: "2026-06-01"
author: Toby
skill: create-work-item
kind: story
priority: high
status: ready
parent: ""
---
# Foo
EOF
  local before
  before="$(cat "$G/meta/work/0001-foo.md")"
  git_init "$G"
  run_0007 "$G"
  assert_neq "guard ($1): migration aborts non-zero" "0" "$RUN_RC"
  assert_eq "guard ($1): zero file mutations" "$before" "$(cat "$G/meta/work/0001-foo.md")"
}
guard_case traversal ".."
guard_case absolute "/abs/work"

echo "=== Allowlist: arbitrary unconfigured subtree skipped (allowlist origin) ==="
# Not just docs/: an ARBITRARY subtree is skipped byte-unchanged, proving the
# skip is the config-driven allowlist rather than a docs-specific denylist.
ARB="$TMP/arbitrary-skip"
mkdir -p "$ARB/meta/notes" "$ARB/meta/arbitrary"
printf '# A Note\n\nx\n' >"$ARB/meta/notes/2026-06-20-n.md"
printf -- '---\nfoo: bar\n---\n# Arbitrary\n' >"$ARB/meta/arbitrary/thing.md"
ARB_BEFORE="$(cat "$ARB/meta/arbitrary/thing.md")"
git_init "$ARB"
run_0007 "$ARB"
assert_eq "arbitrary-skip corpus exits 0" "0" "$RUN_RC"
assert_eq "arbitrary unconfigured subtree skipped byte-unchanged (allowlist)" \
  "$ARB_BEFORE" "$(cat "$ARB/meta/arbitrary/thing.md")"

echo "=== Allowlist: CWD != PROJECT_ROOT (+ symlinked checkout) ==="
XR="$TMP/cwd-root"
mkdir -p "$XR/.accelerator" "$XR/meta/jottings" "$XR/meta/notes"
cat >"$XR/.accelerator/config.md" <<'EOF'
---
paths:
  notes: meta/jottings
---
EOF
printf '# Jotting\n\nx\n' >"$XR/meta/jottings/2026-06-20-j.md"
printf '# Default\n\nx\n' >"$XR/meta/notes/2026-06-20-d.md"
XR_DBEFORE="$(cat "$XR/meta/notes/2026-06-20-d.md")"
git_init "$XR"
OTHER="$TMP/elsewhere"
mkdir -p "$OTHER"
xr_rc=0
# shellcheck disable=SC2034 # captured only to harvest the exit code into xr_rc
xr_out="$(cd "$OTHER" && PROJECT_ROOT="$XR" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$ONLY_0007" ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" 2>&1 </dev/null)" || xr_rc=$?
assert_eq "CWD!=root: migration exits 0" "0" "$xr_rc"
assert_contains "CWD!=root: custom-dir file typed (config resolved against PROJECT_ROOT)" \
  "$(fm_line "$XR/meta/jottings/2026-06-20-j.md" type)" "type: note"
assert_eq "CWD!=root: default-dir file out of scope (self-validator agreed: exit 0)" \
  "$XR_DBEFORE" "$(cat "$XR/meta/notes/2026-06-20-d.md")"
# Symlinked checkout: a symlinked PROJECT_ROOT is canonicalised (pwd -P); a
# forced re-run through the symlink is an empty, idempotent diff.
XLINK="$TMP/cwd-root-link"
ln -s "$XR" "$XLINK"
git -C "$XR" add -A && git -C "$XR" commit -q -m migrated >/dev/null 2>&1
xl_rc=0
(cd "$OTHER" && PROJECT_ROOT="$XLINK" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$ONLY_0007" ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" >/dev/null 2>&1 </dev/null) || xl_rc=$?
assert_eq "symlinked PROJECT_ROOT: migration exits 0" "0" "$xl_rc"
assert_empty "symlinked PROJECT_ROOT: empty meta/ diff (canonicalised, idempotent)" \
  "$(git -C "$XR" status --porcelain meta/ || true)"

echo "=== Allowlist: constant resolver-spawn count ==="
spawn_count() { # $1=repo $2=tag -> echoes spawn count
  local repo="$1" counter="$TMP/rescount-$2" wrap="$TMP/reswrap-$2.sh"
  : >"$counter"
  cat >"$wrap" <<WRAPEOF
#!/usr/bin/env bash
echo x >>"$counter"
exec "$PLUGIN_ROOT/scripts/config-read-doc-type-paths.sh" "\$@"
WRAPEOF
  chmod +x "$wrap"
  (cd "$repo" && PROJECT_ROOT="$repo" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    ACCELERATOR_MIGRATIONS_DIR="$ONLY_0007" ACCELERATOR_MIGRATE_FORCE=1 \
    DOC_TYPE_PATHS_RESOLVER="$wrap" bash "$DRIVER" >/dev/null 2>&1 </dev/null) || true
  grep -c x "$counter"
}
SC1="$TMP/spawn1"
mkdir -p "$SC1/meta/notes"
printf '# N1\n\nx\n' >"$SC1/meta/notes/2026-06-20-n1.md"
git_init "$SC1"
SC5="$TMP/spawn5"
mkdir -p "$SC5/meta/notes"
for i in 1 2 3 4 5; do printf '# N%s\n\nx\n' "$i" >"$SC5/meta/notes/2026-06-20-n$i.md"; done
git_init "$SC5"
c1="$(spawn_count "$SC1" a)"
c5="$(spawn_count "$SC5" b)"
assert_neq "resolver spawned at least once per migration run" "0" "$c1"
assert_eq "resolver spawn count is constant across corpus sizes (1 vs 5 files)" "$c1" "$c5"

test_summary
