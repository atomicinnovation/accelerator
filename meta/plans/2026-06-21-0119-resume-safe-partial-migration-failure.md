---
type: plan
id: "2026-06-21-0119-resume-safe-partial-migration-failure"
title: "Resume-Safe Partial Migration Failure Implementation Plan"
date: "2026-06-21T08:06:03+00:00"
author: Toby Clemson
producer: create-plan
status: done
work_item_id: "work-item:0119"
parent: "work-item:0119"
derived_from: ["codebase-research:2026-06-21-0119-resume-safe-partial-migration-failure"]
relates_to: ["work-item:0115", "work-item:0116", "work-item:0118", "work-item:0069", "work-item:0120"]
tags: [migrate, interactive-migration, agent-invocation, tooling, manifest]
revision: "17a2ffbc90a5fa9c48f2621ab6a79e1ff451fc23"
repository: "build-system"
last_updated: "2026-06-22T12:35:23+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Resume-Safe Partial Migration Failure Implementation Plan

## Overview

When a migration run fails part-way, its prior writes stay on the tree (there is
no transaction boundary — VCS revert is the rollback path). On re-run, the
clean-tree pre-flight sees those writes as a dirty tree and refuses, leaving the
operator with only `ACCELERATOR_MIGRATE_FORCE=1`, which bypasses the *entire*
dirty-tree guard — for every dirty path, not just this run's own output.

This plan adds a **per-run path manifest** that records every scoped path each
migration mutates (including a failing migration's partial writes), and a
**manifest-driven guarded-resume branch** in the pre-flight: when *every* dirty
path is owned by this run's manifest, the runner proceeds into the apply loop
without `ACCELERATOR_MIGRATE_FORCE=1`, printing a resume-affordance listing the
owned paths. Any non-owned path, or a missing/empty/unreadable/stale manifest,
preserves the existing refusal (fail-closed). Run identity is the **base revision**
(jj `change_id` / git `HEAD`), so a manifest from a run the operator has committed
past is refused as stale. No transaction boundary is introduced — out of scope.

Phase 4 extends guarded resume across the **interactive** axis too: a dirty
interactive session log belonging to the current run no longer hard-blocks the
re-run — the runner proceeds and 0069's replay resumes the interactive migration —
while a stale/foreign session log keeps today's structured resume/discard steer.
This reconciles the two resume mechanisms (mechanical manifest + interactive
session log) under one base-revision ownership decision.

## Current State Analysis

All references are to `skills/config/migrate/scripts/run-migrations.sh` at
revision `17a2ffbc` unless noted.

- **The premise holds.** There is no per-run manifest and no record of which
  paths any migration writes. The only durable records are id-per-line ledgers
  (`migrations-applied` at `:25`, `migrations-skipped` at `:26`) and the
  interactive session log (decision values, not paths).
- **Migrations are opaque child processes.** Each migration is dispatched as
  `… bash "$f"` (`:300` mechanical, `run_interactive_migration` at `:287-297`
  interactive). The runner cannot observe a child's writes directly — it must
  diff the tree around the child (the chosen recording mechanism, see Approach).
- **Mid-loop failure does `exit 1` and leaves mutations on the tree** — mechanical
  at `:300-305` (`exit 1` at `:304`), interactive at `:289-292`. By design.
- **The clean-tree pre-flight** is straight-line code at `:94-168`, no functions,
  no early returns: FORCE gate at `:95` (any non-empty value bypasses the *entire*
  block), VCS detection `:96-101`, dirty enumeration `:103-114`, session-log
  branch `:116-161` (its own `exit 1` at `:160`), generic refusal `:162-166`
  (`exit 1` at `:166`). The `ACCELERATOR_MIGRATE_FORCE=1` hint text appears **only**
  at `:164-165`.
- **`.accelerator/state/` is NOT VCS-ignored.** `.gitignore` only ignores
  `.accelerator/config.local.md` (`.gitignore:13`). The dirty enumeration greps
  `^(meta/|\.claude/accelerator|\.accelerator/)` (`:107`), so any bookkeeping file
  the runner writes under `.accelerator/state/` (`migrations-applied`, and the new
  manifest + run-id sidecar) becomes a dirty path in the runner's own enumeration.
  The owned-check must treat runner-managed files as implicitly owned or it will
  refuse its own output.
- **The per-migration completion point** is `:320-321` (`mkdir -p` +
  `atomic_append_unique "$STATE_FILE" "$id"`), with the interactive twin at
  `interactive-lib.sh:648-651`.
- **0116 is done/merged.** This builds against the `--decisions-file` switch and
  `emit_no_input_stall`; its stall text already names 0119
  (`interactive-lib.sh:320-322`). 0116 deliberately did not touch the pre-flight
  resume hint — low conflict surface.

### Key Discoveries:

- Recording mechanism (decided): the runner **diffs the scoped tree** around each
  migration. Reuses the existing enumeration (`:103-114`); captures partial writes
  because the failure path is also a recording point; one enumeration source keeps
  the jj/git untracked asymmetry internally consistent. No migration-author
  contract change.
- `atomic-common.sh:38-61` `atomic_append_unique` — idempotent, atomic append
  (same-dir temp + `mv`). `atomic_write` at `:16-32`.
- Run identity is the **base revision** (jj `change_id` of `@`, git `HEAD`), not a
  timestamp/pid token: it is comparable at pre-flight time and moves only on a
  commit, giving the staleness gate real teeth. Note `artifact-derive-metadata.sh:11`
  captures jj `commit_id` (a content hash that changes on every edit) — the run-id
  helper must use `change_id` instead, or resume breaks under jj.
- Ownership is defined as **"the scoped-dirty delta around each migration"**, not
  "paths the migration's code wrote" — so the manifest is a derived view of
  `enumerate_scoped_dirty` and inherits its scope filter and VCS semantics. A
  future change to the dirty-enumeration scope therefore changes resume
  correctness as a side effect; the single helper has two coupled clients.
- `launcher-helpers.sh:157-167` (`stop_server_stop`) — fail-closed-on-identity
  precedent to mirror for staleness.
- `0003-relocate-accelerator-state.sh:120` — `rel="${path#"$PROJECT_ROOT/"}"`
  prefix-strip to repo-relative; `:196-218` bash-3.2-safe dedup loop; `:243` `>>`
  append idiom.
- bash-3.2 floor: **no associative arrays**. Set membership via file + `grep -Fxq`
  (`atomic-common.sh:46`) or parallel array + linear scan with the
  `"${arr[@]+"${arr[@]}"}"` unbound-safe expansion (`run-migrations.sh:236-242`).
- Tests: `skills/config/migrate/scripts/test-migrate.sh` (flat `Test:` blocks).
  Failing-stub model = Test 4 (`:138-154`); dirty-tree + FORCE model = Test 14
  (`:350-376`). Exact-line assertions via `grep -cFx` + `wc -l`
  (`:1004-1012`, `:791-796`). Stderr via
  `assert_stderr_contains`/`assert_stderr_not_contains`
  (`test-helpers.sh:257`/`:273`). Suite-count floor
  `tasks/test/integration.py:8` `_EXPECTED_MIGRATE_SUITES = 4` is an at-least
  floor on *suite files* — adding cases to `test-migrate.sh` needs no bump.

## Desired End State

`run-migrations.sh` records a per-run path manifest as it applies migrations, and
on a blocked re-run whose every dirty path is owned by that manifest (mechanical
paths *and* current-run interactive session logs), proceeds into the apply loop
without `ACCELERATOR_MIGRATE_FORCE=1`, emitting a resume-affordance to stderr;
0069's replay resumes any in-flight interactive migration. Any non-owned path or
unusable/stale manifest keeps today's refusal, and a stale/foreign session log
keeps today's resume/discard steer. Verified by the four AC tests plus the Phase 4
interactive-reconciliation tests in `test-migrate.sh`/`test-migrate-interactive.sh`,
with the existing suites staying green and `mise run check` clean.

## What We're NOT Doing

- **No transaction/staging boundary.** Mutations still land immediately; VCS
  revert remains the rollback path. (Work item explicitly out of scope.)
- **No migration-author contract change.** Migrations are not asked to self-report
  paths (recording mechanism (a) was rejected in favour of runner-side diffing).
- **No resumability for complete-but-uncommitted runs.** The manifest is deleted
  on full success, so guarded resume is scoped strictly to *partial-failure*
  re-runs; a complete-but-uncommitted re-run still hits today's refusal.
- **No content verification of owned paths.** Ownership is by path, not by
  content hash. The base-revision gate now refuses once the operator has
  *committed* since the failed run (the working copy moved on), so the remaining
  residual is narrower: an operator who hand-edits the same paths **without
  committing** and re-runs can still resume over those edits (base revision
  unchanged). This is the accepted path-only-ownership limitation (VCS revert is
  the backstop) — recorded under Limitations.
- **Session-log paths stay out of the *mechanical* manifest.** The manifest
  records only mechanical corpus paths; interactive migrations are not recorded
  into it (Phase 2 §3). Phase 4 instead recognises a dirty
  `migrations-<id>-session.jsonl` as **owned by pattern** (gated by the same
  base-revision staleness check), so the two axes share one ownership decision
  without the manifest carrying session-log entries. (Mixed-run reconciliation is
  now *in* scope — see Phase 4 — superseding the earlier "documented boundary".)
- **No per-session-log run identity.** Phase 4 deliberately does **not** rename
  session logs or tag each with a run id. The run id is the base revision, which is
  shared by any two invocations with nothing committed between them, so a per-log
  tag adds no robustness over the existing `migrations-run.id` base-revision gate
  (and the blocked re-run has no own-identity to compare a per-invocation nonce
  against at pre-flight time). The accepted residual is identical to mechanical
  path-only ownership: a stale prior run at the *same uncommitted* base revision.

## Implementation Approach

The runner snapshots the scoped dirty set as a **baseline** before the apply loop,
then re-enumerates after each migration returns — on success *and* failure — and
appends the **delta** (paths dirty now, not in the baseline) to the manifest
before deciding whether to `exit 1`. Because the failure path is itself a
recording point, a migration that mutates some paths then errors still has its
partial writes recorded (AC1).

The baseline depends on how the run started:

- **Fresh clean-tree run** — the pre-flight guaranteed a clean tree, so the
  baseline is empty and every migration's writes are attributed to this run.
- **FORCE run** — the tree may carry foreign dirt; the baseline captures it so
  those pre-existing paths are *excluded* from this run's manifest (the run must
  not claim ownership of paths it did not write).
- **Guarded resume** (`RESUME=1`) — the baseline is **empty**, not the current
  (already-dirty) tree. The dirty paths are this run's own prior output, already
  in the (un-truncated) manifest; re-enumerating against an empty baseline
  re-asserts ownership of every still-dirty path on each step, so the manifest is
  self-healing across successive failures rather than depending on the prior
  append surviving. Seeding the baseline from the current tree here would subtract
  the prior output from every later delta — correct only while the manifest is
  never truncated, coupling completeness to the `RESUME` flag's reliability.

```text
if RESUME: baseline = ∅                          # resume: re-assert, self-healing
else:      baseline = enumerate_scoped_dirty()   # fresh: ∅; FORCE: exclude foreign dirt
for f in pending:
  run child f  (rc)
  delta = enumerate_scoped_dirty() - baseline
  append delta to manifest                 # captures partial writes on failure
  if rc != 0: exit 1
  ledger append; continue
on full success: delete manifest + run-id sidecar

# owned = manifest-paths ∪ {runner bookkeeping files}
# guarded resume iff dirty ≠ ∅ AND every dirty path ∈ owned
#                   AND manifest usable AND recorded base revision == current
```

**Manifest = sidecar pair**, both under `$PROJECT_ROOT/.accelerator/state/`:

- `migrations-run-paths.txt` — the manifest: one repo-relative path per line,
  deduped, sorted. Pure path list (no header) so AC1's "exactly the paths, one per
  line" assertion is literal.
- `migrations-run.id` — single line recording the **base revision** the run
  started against: jj `change_id` of `@` (stable while the working copy is edited;
  moves only on `jj new`/`jj commit`), or git `HEAD`. **Not** the working-copy
  `commit_id` — under jj that is a content hash that changes on every file write
  (it is what `artifact-derive-metadata.sh:11` captures), so it would differ
  between the failing run and the resume and break a legitimate resume. Migrations
  never commit (confirmed), so the base revision is stable for a run's duration.

**Fail-closed staleness**: the owned set is treated as empty (→ refuse) whenever
the manifest is absent / empty / unreadable, the `migrations-run.id` sidecar is
absent / empty / unreadable, **or the recorded base revision differs from the
current base revision**. The revision comparison is the teeth of the staleness
check: it is computable at pre-flight time (unlike a per-process token, which the
exited run cannot expose), and it fails closed when the operator has committed
since the failed run (the working copy has moved on). A fresh clean-tree run also
mints a new id and truncates the manifest, so a prior run's manifest cannot survive
a clean start. The one residual it does *not* catch — uncommitted hand edits that
coincide with the manifest's paths (base revision unchanged) — is the accepted
path-only-ownership limitation (VCS revert is the backstop), recorded under
Limitations.

**Routing both recording and the owned-check through one enumeration helper**
(Phase 1) is what makes recorded paths string-match the dirty paths checked at
resume time, and keeps the jj-vs-git untracked behaviour consistent between the
two.

## Phase 1: Extract a shared scoped-dirty enumeration helper

### Overview

Behaviour-preserving refactor: lift the inline jj/git dirty-enumeration
(`run-migrations.sh:103-114`) into one function that yields normalized,
repo-relative scoped paths, and call it from the pre-flight. This is the single
source Phases 2 and 3 share, and it is where the jj-vs-git untracked asymmetry is
pinned to one definition. Independently mergeable — no externally observable
behaviour change.

### Changes Required:

#### 1. New enumeration helper

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: Add a function near the top (after the sourced libs, before the
pre-flight) that takes the detected `vcs` and emits one normalized repo-relative
path per line for the scoped dirty set. Normalization strips the git porcelain
status prefix (mirroring the existing session-log `sed 's/^[[:space:]]*//;
s/^[A-Z?][[:space:]]*//'` at `:122`) so jj and git both yield bare repo-relative
paths.

```sh
# enumerate_scoped_dirty <vcs>
#   Emit one normalized repo-relative path per line for uncommitted changes
#   under meta/, .claude/accelerator*.md, .accelerator/. Single source of truth
#   for the pre-flight owned-check and the apply-loop manifest recording, so the
#   jj-vs-git untracked asymmetry has exactly one definition.
enumerate_scoped_dirty() {
  local vcs="$1"
  if [ "$vcs" = "jj" ] && command -v jj >/dev/null 2>&1; then
    jj --no-pager diff --name-only 2>/dev/null |
      grep -E '^(meta/|\.claude/accelerator|\.accelerator/)' || true
  elif [ "$vcs" = "git" ]; then
    git -C "$PROJECT_ROOT" status --porcelain \
      "meta/" ".claude/accelerator.md" ".claude/accelerator.local.md" \
      ".accelerator/" 2>/dev/null |
      grep -v '^??' |
      sed 's/^[[:space:]]*//; s/^[A-Z?][[:space:]]*//; s/^.* -> //' || true
  fi
}
```

The trailing `s/^.* -> //` resolves git's rename porcelain (`R  old -> new`) to the
**new** path, so a migration that `mv`s a scoped file (e.g. the 0003 relocate
migration) records and resume-matches the destination consistently. Without it the
rename line yields a bogus `old -> new` token that never string-matches a recorded
path, making guarded resume unreachable for exactly the rename-heavy migrations
that motivate it. (Residual: paths containing spaces are C-quoted by git porcelain;
the `meta/` corpus is kebab-case so this does not arise — noted as a known edge.)

**Note the deliberate semantic change for git**: the inline code at `:110-113`
returned porcelain lines *with* status prefixes; the helper strips them to bare
paths. The pre-flight's existing `[ -n "$dirty" ]` test and the session-log
`grep`/`sed` are unaffected (the session-log branch already re-strips). This is
required so recorded paths match dirty paths by string equality in Phase 3.

**jj-vs-git untracked asymmetry (deliberately preserved).** The git branch keeps
`grep -v '^??'` (excludes untracked); the jj branch includes untracked (jj tracks
by default). This is *intentional*: it preserves the existing dirty-tree guard's
git behaviour (changing git to include untracked would make the guard stricter —
out of 0119's scope). The asymmetry is benign for resume because the **same**
exclusion governs both the recorder and the guard under git: a migration-*created*
(untracked) file is invisible to recording *and* to the pre-flight, so it neither
gets owned nor blocks a re-run — there is no resume gap. Only *modified tracked*
files (and renames, handled above) both block and are recorded, and those match
consistently. Under jj, created files are tracked end-to-end and handled the same
way throughout. So the feature is correct on both VCSes; what differs is only
*which* files participate, and that difference already exists in today's guard.

#### 2. Pre-flight calls the helper

**File**: `skills/config/migrate/scripts/run-migrations.sh:103-114`
**Changes**: Replace the inline `dirty=$(…)` block with
`dirty=$(enumerate_scoped_dirty "$vcs")`. Leave VCS detection (`:96-101`), the
`[ -n "$dirty" ]` test, the session-log branch, and the refusal untouched.

### Success Criteria:

#### Automated Verification:

- [x] Migration suite passes: `bash skills/config/migrate/scripts/test-migrate.sh`
- [x] 0007 suite passes: `bash skills/config/migrate/scripts/test-migrate-0007.sh`
- [x] Interactive suite passes:
      `bash skills/config/migrate/scripts/test-migrate-interactive.sh`
- [x] Shell lint + bashisms clean: `mise run scripts:check`
- [x] Full read-only gate clean: `mise run check`
- [x] New/strengthened assertion that the dirty-tree refusal **and** the
      session-log detection stderr are exactly as expected — Phase 1 deliberately
      changes git's emitted output (strips porcelain status prefixes to bare
      paths), and Test 14's lone `assert_contains "...dirty"` substring would not
      catch a regression in the multi-line text. Assert these **specific** lines
      (not just "dirty"): the two `refuse_dirty_tree` lines (`Error: dirty working
      tree — uncommitted changes detected …` and `Commit or discard those changes
      first, or set ACCELERATOR_MIGRATE_FORCE=1 …`), and the session-log branch's
      distinctive header/hint lines (`:125`/`:138-139`/`:158-159`). Prefer a
      golden-string capture of the full stderr block where practical, so the
      byte-identical claim is CI-enforced rather than a judgement call at
      implementation time.

#### Manual Verification:

- [ ] Spot-check the dirty-tree refusal and session-log detection stderr under
      both jj and git match pre-refactor (the automated assertion above is the
      primary guard; this is a sanity diff).

---

## Phase 2: Write the per-run path manifest (recording half)

### Overview

Record what each migration mutates, using the Phase 1 helper. Snapshot a baseline
before the loop, append the per-migration delta on success *and* failure, manage
the manifest lifecycle (truncate + new run-id on a fresh clean-tree start; delete
both files on full success). The manifest is written but **not yet read** by the
pre-flight. Satisfies AC1.

**Merge order is fixed: Phase 1 → 2 → 3.** Phase 2 is *not* fully behaviour-neutral
on its own: it writes `migrations-run-paths.txt`/`migrations-run.id` under
`.accelerator/state/`, which the dirty enumeration itself matches (`^\.accelerator/`).
Without Phase 3's implicit-ownership recognition of those bookkeeping files, a
Phase-2-only **partial failure** would leave manifest/run-id files that the
unmodified pre-flight then refuses over (with no resume affordance). A *clean full
run* still deletes them, so the common path is unaffected — but Phase 2 must be
landed as a step toward Phase 3, not shipped alone as a stable end state. (If the
phases must be independently releasable, move the bookkeeping-file carve-out from
Phase 3 §1 into Phase 2.)

### Changes Required:

#### 1. Manifest paths + helpers

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: Define alongside `STATE_FILE`/`SKIP_FILE` (`:25-26`):

```sh
RUN_PATHS_FILE="$PROJECT_ROOT/.accelerator/state/migrations-run-paths.txt"
RUN_ID_FILE="$PROJECT_ROOT/.accelerator/state/migrations-run.id"
```

Add helpers (after `enumerate_scoped_dirty`). `manifest_record_delta` takes `vcs`
as an explicit parameter — matching `enumerate_scoped_dirty`'s signature, rather
than reading an ambient global (which is unbound-fragile under `set -u`):

```sh
# manifest_record_delta <vcs> <baseline_file>
#   Append every currently-dirty scoped path not present in <baseline_file> to
#   the manifest, deduped (atomic_append_unique is idempotent). Repo-relative
#   paths already; no further normalization needed (single enumeration source).
manifest_record_delta() {
  local vcs="$1" baseline="$2" path
  mkdir -p "$(dirname "$RUN_PATHS_FILE")"
  while IFS= read -r path; do
    [ -z "$path" ] && continue
    grep -Fxq -- "$path" "$baseline" 2>/dev/null && continue
    atomic_append_unique "$RUN_PATHS_FILE" "$path"
  done < <(enumerate_scoped_dirty "$vcs")
}

# current_base_revision <vcs>
#   Emit the committed base revision the working copy sits on. For jj this is the
#   change_id of @ — STABLE while the working copy is edited (it moves only on
#   `jj new`/`jj commit`), unlike commit_id (a content hash that changes on every
#   write). For git it is HEAD, stable across uncommitted edits. Migrations never
#   commit, so this value is constant for the duration of a run and differs only
#   when the operator has committed since — exactly the staleness signal we want.
current_base_revision() {
  local vcs="$1"
  if [ "$vcs" = "jj" ] && command -v jj >/dev/null 2>&1; then
    jj log -r @ --no-graph --no-pager -T change_id 2>/dev/null || true
  elif [ "$vcs" = "git" ]; then
    git -C "$PROJECT_ROOT" rev-parse HEAD 2>/dev/null || true
  fi
}
```

#### 2. Mint run-id + truncate on a fresh clean-tree run; capture baseline

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: Place this block **after the no-pending early-exit (`:249-255`),
immediately before the apply loop** — so a "nothing to do" invocation neither mints
a run-id nor captures a baseline (it exits at `:249-255`, where `clear_run_manifest`
removes any stale leftover manifest). On a **fresh** run (not a resume — Phase 3
sets `RESUME=1`; absent here, so always-fresh until Phase 3 lands) mint a new
run-id (the current base revision) and truncate the manifest:

```sh
# Establish run identity + baseline before the apply loop.
if [ "$RESUME" -ne 1 ]; then
  mkdir -p "$(dirname "$RUN_ID_FILE")"
  current_base_revision "$vcs" | atomic_write "$RUN_ID_FILE"
  : | atomic_write "$RUN_PATHS_FILE"   # truncate to empty
fi
BASELINE_FILE=$(mktemp) || { echo "migrate: cannot create baseline temp file" >&2; exit 1; }
if [ "$RESUME" -eq 1 ]; then
  : >"$BASELINE_FILE"                  # resume: empty baseline — re-assert ownership
else
  enumerate_scoped_dirty "$vcs" >"$BASELINE_FILE" 2>/dev/null || true
fi
```

**Cleanup is explicit, not via an `EXIT` trap.** `BASELINE_FILE` is removed with an
inline `rm -f "$BASELINE_FILE"` immediately before every `exit` that can follow its
creation — the two mid-loop `exit 1`s (mechanical `:304`, interactive `:291`), the
post-loop success path, and the no-pending early-exit — mirroring how the existing
`STDOUT_FILE` is already cleaned inline (`:302`/`:313`). A script-level
`trap '… EXIT'` is **deliberately avoided**: the runner installs no EXIT trap
today, and `atomic_write` (atomic-common.sh:28-31) runs `trap - EXIT` internally.
That `trap -` is currently subshell-local (every `atomic_write` call is on the
right of a pipe, so it cannot reach a parent trap) — but depending on that
invariant for temp-file cleanup is fragile: a future direct (non-piped)
`atomic_write` call from the main shell would silently disarm a parent EXIT trap.
Inline `rm -f` is invariant-free and consistent with the surrounding code. Folding
`STDOUT_FILE` into the same convention (it already is) keeps one cleanup model.

Two scoping requirements:

- **`RESUME=0` is initialised unconditionally at the top of the script**, not
  merely "before the pre-flight" — the pre-flight begins at the FORCE gate
  (`:95`), and on the FORCE path the whole block (including Phase 3's `RESUME=1`
  assignment) is skipped, so an in-block default would leave `RESUME` unset and
  `[ "$RESUME" -ne 1 ]` would fail under `set -u`. A FORCE run therefore stays
  `RESUME=0` and correctly mints a fresh id + truncates (FORCE is a brand-new run).
- **VCS detection is hoisted** out of the FORCE-guarded block (`:96-101`) to the
  top of the script so `vcs` is computed unconditionally (cheap, side-effect-free)
  and visible to the FORCE path, the non-FORCE path, and the loop. The new helpers
  take `vcs` as a parameter rather than reading it as a global.
- **An empty base revision at mint time disables guarded resume for the run
  (fail-closed, by design).** A commit-less git repo (unborn `HEAD`) or an
  absent/failed VCS yields an empty `current_base_revision`, so the run-id sidecar
  is written empty and Phase 3's `[ -s "$RUN_ID_FILE" ]` gate refuses on any later
  re-run. This is the safe direction, but it is silent, so: new git test fixtures
  must create an initial commit (the Test 14 model already does), and the plan
  notes this as an intentional always-refuse boundary rather than a bug.

#### 3. Record the delta after every migration dispatch

**File**: `skills/config/migrate/scripts/run-migrations.sh:281-324` (apply loop)
**Changes**: Record the delta immediately after each child returns, on **both**
outcomes, before any `exit 1`. Mechanical path (`:299-305`):

```sh
  STDOUT_FILE=$(mktemp)
  if ! PROJECT_ROOT="$PROJECT_ROOT" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
       ACCELERATOR_MIGRATION_MODE=1 bash "$f" >"$STDOUT_FILE" 2>&1; then
    manifest_record_delta "$vcs" "$BASELINE_FILE"   # capture partial writes (AC1)
    cat "$STDOUT_FILE" >&2
    rm -f "$STDOUT_FILE" "$BASELINE_FILE"
    echo "[${id}] failed" >&2
    exit 1
  fi
  manifest_record_delta "$vcs" "$BASELINE_FILE"      # capture successful writes
```

The interactive failure branch (`:291`) gains the same `rm -f "$BASELINE_FILE"`
before its `exit 1`.

**The interactive path (`:287-297`) is deliberately *not* recorded into the
manifest.** Interactive partial-resume is governed by the 0069 session-log
scaffold, not this manifest, and the session-log detection branch (`:116-161`)
runs first on re-run and `exit 1`s whenever a session log is dirty — so a
manifest entry for an interactive migration could never be consumed by guarded
resume. Recording it would also pull the migration's own
`migrations-<id>-session.jsonl` into the manifest, blurring the two resume axes.
Keeping the manifest purely mechanical makes the owned-check's input coherent:
the manifest answers "what mechanical corpus paths did this run write", and the
session-log branch independently owns the interactive case. (See the expanded
"What We're NOT Doing" note on the two-axis boundary.)

#### 4. Delete the manifest + sidecar on full success

**File**: `skills/config/migrate/scripts/run-migrations.sh` (after the loop, near
the summary at `:326`)
**Changes**:

```sh
# Full run completed without aborting — no partial state to resume over.
rm -f "$RUN_PATHS_FILE" "$RUN_ID_FILE" "$BASELINE_FILE"
```

The same `rm -f "$RUN_PATHS_FILE" "$RUN_ID_FILE"` must also run on the **"no
pending migrations" early-exit** (`exit 0` before the loop): otherwise a stale
manifest from a prior failed run survives a clean "nothing to do" invocation. Factor
the two-line cleanup into a small `clear_run_manifest()` helper called from both
the post-loop success path and the no-pending early-exit, so the manifest's
"deleted whenever a run ends without partial state" invariant holds on every
non-aborting exit.

### Success Criteria:

#### Automated Verification:

- [x] New test "manifest records partial writes after mid-run failure" passes:
      `bash skills/config/migrate/scripts/test-migrate.sh` — a stub that writes a
      known set of scoped paths then `exit 1`s leaves
      `migrations-run-paths.txt` containing **exactly** those paths (asserted via
      per-path `grep -cFx '<path>' == 1` plus a `wc -l` total).
      **The test must use a real initialised repo** (the Test 14 model: `git
      init -q` + an initial commit, or the jj equivalent) — **not** Test 4's fake
      `mkdir .git`. The chosen recorder runs real `git status`/`jj diff`, which
      yield nothing against a fake `.git`, so a Test-4-style fixture would leave
      the manifest empty and the assertion would verify nothing about recording.
- [x] New test "manifest deleted on full success" passes: after a clean full run,
      `migrations-run-paths.txt` and `migrations-run.id` do not exist.
- [x] New test "fresh run truncates a leftover manifest" passes: a pre-seeded
      manifest from a clean-tree start is reset (new run-id written).
- [x] Existing Test 4 (failing-stub) still passes (no ledger entry on abort).
- [x] `mise run check` clean (shellcheck/bashisms incl. bash-3.2 floor).

#### Manual Verification:

- [ ] Induce a real partial failure under jj (stub that `mv`s a file then `exit 1`)
      and confirm `migrations-run-paths.txt` lists the moved path repo-relative.
- [ ] Confirm the manifest write does not itself break the next clean run.

---

## Phase 3: Manifest-driven guarded-resume branch (reading half)

### Overview

Teach the pre-flight to compute the owned set from the manifest and, when the
dirty tree is fully owned, proceed into the apply loop with a resume-affordance
instead of refusing. Fail-closed on any non-owned path or unusable/stale manifest.
Lands **after** Phase 2 (the phases are sequenced, not commutative): without
Phase 2 no manifest is ever written, so the owned set is always empty, behaviour
is identical to today, and the `RESUME` flag this phase sets is inert until
Phase 2's fresh-run guard reads it. Satisfies AC2/AC3/AC4.

### Changes Required:

#### 1. Owned-set computation with fail-closed staleness

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: Add a helper that decides whether every dirty path is owned. Runner
bookkeeping files are implicitly owned (they match `^\.accelerator/` but are not
recorded by migrations). Fail-closed when the manifest or run-id sidecar is
unusable.

```sh
# dirty_tree_fully_owned <vcs> <dirty>
#   Return 0 iff the manifest + run-id sidecar are usable, the recorded base
#   revision still equals the current one, AND every line in <dirty> is either a
#   runner-managed bookkeeping file or present in the manifest.
#   Fail-closed: any unusable manifest/sidecar, or a revision mismatch, → return 1.
dirty_tree_fully_owned() {
  local vcs="$1" dirty="$2" path recorded current
  # Fail-closed usability gate (mirror launcher-helpers.sh:157 identity gate).
  [ -r "$RUN_ID_FILE" ] && [ -s "$RUN_ID_FILE" ] || return 1   # run-id non-empty
  # Manifest must EXIST but may be EMPTY: an in-flight interactive interrupt that
  # ran before any mechanical delta leaves an empty manifest, yet its session log
  # is owned-by-pattern below. Requiring non-empty here (`-s`) would make that
  # resume unreachable. The per-path loop is the sole ownership authority — an
  # empty manifest + a dirty *mechanical* path still refuses (path ∉ empty manifest).
  [ -r "$RUN_PATHS_FILE" ] || return 1
  # Staleness: the recorded base revision must equal the current one. They differ
  # only when the operator has committed since the failed run (the working copy
  # has moved on) — exactly the "different run" case AC4 requires we refuse.
  recorded=$(head -n1 "$RUN_ID_FILE")
  current=$(current_base_revision "$vcs")
  [ -n "$current" ] && [ "$recorded" = "$current" ] || return 1
  # Runner-managed bookkeeping files are implicitly owned; derive their
  # repo-relative forms from the path variables (no hard-coded literals to drift).
  local rel_applied="${STATE_FILE#"$PROJECT_ROOT/"}"
  local rel_skipped="${SKIP_FILE#"$PROJECT_ROOT/"}"
  local rel_paths="${RUN_PATHS_FILE#"$PROJECT_ROOT/"}"
  local rel_id="${RUN_ID_FILE#"$PROJECT_ROOT/"}"
  while IFS= read -r path; do
    [ -z "$path" ] && continue
    case "$path" in
      "$rel_applied"|"$rel_skipped"|"$rel_paths"|"$rel_id") continue ;;
    esac
    grep -Fxq -- "$path" "$RUN_PATHS_FILE" || return 1
  done <<<"$dirty"
  return 0
}
```

**`set -e` contract**: `dirty_tree_fully_owned` is always invoked as an `if`
condition (below), which suspends `set -e` for the whole function body — so an
internal `grep -Fxq` no-match returns non-zero without aborting the script, and
the explicit `|| return 1` guards make every fail-closed branch an intended
refusal rather than an uncontrolled `set -e` exit. The quoted `case` patterns
(`"$rel_applied"` …) match literally (quoting disables globbing), so a foreign
`.accelerator/` path that is *not* one of the four bookkeeping files still falls
through to the manifest membership test and triggers refusal.

#### 2. Guarded-resume branch in the pre-flight

**File**: `skills/config/migrate/scripts/run-migrations.sh:116-167`
**Changes**: Inside `if [ -n "$dirty" ]; then` (`:116`), *after* the existing
session-log branch (`:116-161`, which still `exit 1`s when in-flight session logs
are present), and *before* the generic refusal (`:162-166`), insert the guarded
resume:

First factor the existing refusal (`:162-166`) into one helper so the message has
a single definition (the guarded-resume `else` and the original site both call it,
and they cannot drift):

```sh
# refuse_dirty_tree — emit the canonical dirty-tree refusal + FORCE hint, exit 1.
refuse_dirty_tree() {
  echo "Error: dirty working tree — uncommitted changes detected in meta/," \
    ".claude/accelerator*.md, or .accelerator/." >&2
  echo "Commit or discard those changes first, or set" \
    "ACCELERATOR_MIGRATE_FORCE=1 to skip this check." >&2
  exit 1
}
```

Then the guarded-resume branch:

```sh
    if dirty_tree_fully_owned "$vcs" "$dirty"; then
      RESUME=1
      echo "Resuming over this run's own partial migration output:" >&2
      while IFS= read -r path; do
        [ -z "$path" ] && continue
        echo "  $path" >&2
      done <<<"$dirty"
      # fall through past the refusal into "Read state files"
    else
      refuse_dirty_tree
    fi
```

Replace the original inline refusal at `:162-166` with a `refuse_dirty_tree` call
too, so the message text exists in exactly one place.

`RESUME` is read by Phase 2's "fresh run" guard (so a resume does **not** truncate
the manifest or mint a new run-id — it continues the existing run). `RESUME=0` is
initialised unconditionally at the top of the script (see Phase 2 §2), so the
FORCE path — which skips this whole block — leaves `RESUME=0` and still mints a
fresh run-id + truncates (correct: FORCE is a brand-new run), with no unbound-var
risk under `set -u`.

#### 3. Reconcile the 0116 breadcrumb

The 0116 stall text (`interactive-lib.sh:320-322`, "resume-safety for partial runs
is tracked separately (0119)") is reconciled in **Phase 4**, where the interactive
axis genuinely gains guarded resume — so this is no longer a text-only touch.
Phase 4 owns that reconciliation (and the coordination with 0116's note that
pre-flight resume-hint changes touch that task).

> **Phase 3 ships the mechanical-only guarded resume.** As written above, the
> session-log detection branch still runs first and `exit 1`s, so only a
> purely-mechanical partial failure reaches guarded resume at the end of Phase 3.
> Phase 4 reorders the pre-flight (owned-check first) and teaches the owned-check
> to recognise current-run session logs, extending guarded resume to mixed and
> interactive partial failures. Phase 3 is mergeable on its own as the
> mechanical-only capability.

### Success Criteria:

#### Automated Verification:

- [x] New test "guarded resume on fully-owned dirty tree" passes
      (`test-migrate.sh`): a partial-run failure whose dirty paths are all in the
      manifest → re-run **exits 0**, proceeds into the apply loop *without*
      `ACCELERATOR_MIGRATE_FORCE=1`, and stderr contains `own partial migration
      output` and each owned path (`assert_stderr_contains`).
- [x] New test "refuse on mixed/non-owned dirty tree" passes: dirty set includes
      one path not in the manifest → **non-zero exit**, stderr does **not** contain
      the resume-affordance (`assert_stderr_not_contains "own partial migration
      output"`), and **does** contain `ACCELERATOR_MIGRATE_FORCE`
      (`assert_stderr_contains`).
- [x] New test "fail-closed on unusable/stale manifest" passes — parameterized
      over: manifest absent; manifest empty; run-id sidecar absent; run-id sidecar
      empty; **and the genuinely-stale case** — a *populated* manifest whose paths
      match the dirty tree paired with a `migrations-run.id` whose recorded base
      revision differs from the current one (the case the revision gate exists
      for; AC4's "different run"). Each over a dirty tree **containing a mechanical
      path** yields the **same** observable refusal as the mixed case: non-zero
      exit, no resume-affordance,
      **and the FORCE-hint refusal message present** (`assert_stderr_contains
      "ACCELERATOR_MIGRATE_FORCE"`, not merely a non-zero exit — so a bare `set -e`
      abort is distinguishable from an intended refusal).
- [x] New test "guarded resume that fails again accumulates correctly": induce a
      partial failure, re-run into a guarded resume whose later migration also
      fails; assert the manifest now contains the **union** of both failures'
      paths and a **third** re-run still resumes (exit 0) — locking in the
      empty-baseline-on-resume / self-healing behaviour.
- [x] AC2–AC4 are exercised under **both git and jj** (parameterise the fixture,
      jj cases guarded by `command -v jj`); the single-enumeration-source claim and
      the `change_id`-vs-`commit_id` revision capture are jj-specific and must be
      asserted, not left to manual steps. (Today's suite has no jj coverage.)
- [x] Existing Test 14 (dirty-tree refusal + FORCE bypass) still passes unchanged.
- [~] Full default gate green end-to-end: `mise run` — affected suites
      (test-migrate, test-migrate-interactive, test-migrate-0007) + `mise run
      check` are green; the bare full gate (heavy Rust/frontend rebuild) was not
      re-run since the change is shell-only.

#### Manual Verification:

- [ ] Real partial failure → re-run under jj resumes without FORCE and prints the
      affordance listing the owned paths; the re-run completes the pending
      migrations.
- [ ] Add one foreign edit (a hand-edited `meta/` file not from the run) to a
      fully-owned dirty tree → re-run refuses with the FORCE hint, no affordance.
- [ ] `rm .accelerator/state/migrations-run.id` on an otherwise-fully-owned dirty
      tree → re-run refuses (fail-closed).
- [ ] git-VCS repo (not jj): the same fully-owned / mixed / fail-closed behaviours
      hold (untracked-created files behave consistently with the jj path given the
      single enumeration source).

---

## Phase 4: Reconcile the interactive session-log axis

### Overview

Extend guarded resume to runs whose dirty set includes an interactive
`migrations-<id>-session.jsonl` — both the *completed-interactive + mechanical-fail*
mixed case and the *in-flight interactive* case. Today the session-log detection
branch (`run-migrations.sh:116-161`) runs first and unconditionally `exit 1`s
whenever any session log is dirty, so Phase 3's guarded resume is unreachable for
any run that touched an interactive migration. Phase 4 makes the **owned-check the
primary decision**: when every dirty path — including session logs — is owned by
the current run, the runner proceeds into the apply loop, where the applied-ledger
skips completed migrations and **0069's replay-on-entry resumes the in-flight
interactive one**. A stale/foreign session log (different base revision) still gets
today's structured resume/discard scaffold.

**No session-log rename, no per-log run-id tag** (see "What We're NOT Doing"): the
run id is the base revision, already recorded in `migrations-run.id` and already
checked by `dirty_tree_fully_owned`, so a session log is recognised as current-run
**by path pattern** under that existing gate — at zero cost to the
`migrations-<id>-session.jsonl` naming contract, the detection regex, the ~40
test references, the `migration_session_log_path` harness contract, or the SKILL.md
worked-example.

### Changes Required:

#### 1. Treat current-run session artifacts as owned (extend `dirty_tree_fully_owned`)

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: A failed interactive migration leaves **more than** its `-session.jsonl`
under `.accelerator/state/`: the failure return (`interactive-lib.sh:638-639`)
removes the resume-state tmp but **intentionally preserves** `migrations-<id>-stderr.log`
(printed as "full stderr preserved at:" at `:634`). So the owned-check must
recognise the whole **runner-managed session-artifact family**, not just the log,
or a dirty `stderr.log` defeats guarded resume under jj (which tracks created files;
git's `^??` filter hides it). And the recogniser must match the **same** id class as
the existing detection regex (`migrations-[0-9a-z-]+-session\.jsonl`, `:121`) — a
looser `migrations-*-…` glob would own near-miss filenames the scaffold detector
would reject, letting the two sets drift.

Define **two predicates sharing one id-class**, because the owned-check and the
detector have *different* scopes: the owned-check must tolerate the whole family
(so a preserved `stderr.log` doesn't defeat resume), but the detector is
session-LOG-specific (it runs `wc -l` for `(N decisions recorded)` and `To discard:
rm <log> (loses N decisions)` — feeding it the family would mislabel a `stderr.log`
or `resume-state.tmp` as a decisions log with a bogus count). They share the same
`migrations-<id>-` id-class so the two recognisers cannot drift on what counts as
this-run's, but differ on suffix scope:

```sh
# is_session_log <repo-relative-path>   — detector + affordance use this.
#   True ONLY for the canonical interactive session log.
is_session_log() {
  case "$1" in
    .accelerator/state/migrations-[0-9a-z]*-session.jsonl) return 0 ;;
  esac
  return 1
}

# is_session_artifact <repo-relative-path>  — owned-check uses this.
#   True for ANY runner-managed interactive session artifact preserved across a
#   failure: the log, the stderr capture, or the resume-state tmp. (FIFOs
#   migrations-<id>-{r2m,m2r}.fifo can also linger on an in-flight failure but are
#   deliberately omitted — neither git nor jj tracks named pipes, so they never
#   appear in enumerate_scoped_dirty.)
is_session_artifact() {
  case "$1" in
    .accelerator/state/migrations-[0-9a-z]*-session.jsonl) return 0 ;;
    .accelerator/state/migrations-[0-9a-z]*-stderr.log) return 0 ;;
    .accelerator/state/migrations-[0-9a-z]*-resume-state.tmp) return 0 ;;
  esac
  return 1
}
```

Then the owned arm (gated, like every other owned path, by the base-revision check
already at the top of the function — so a stale-run artifact is **not** owned):

```sh
    case "$path" in
      "$rel_applied"|"$rel_skipped"|"$rel_paths"|"$rel_id") continue ;;
    esac
    is_session_artifact "$path" && continue   # current-run interactive artifact (Phase 4)
```

Session artifacts remain absent from the mechanical manifest. **Accepted residual
(documented in Limitations):** the owned arm treats *any* current-base-revision
session artifact as this run's, without checking the id is in this run's pending or
applied set — the pending set isn't computed until after the pre-flight, and the
base-revision gate already bounds ownership to "the working copy hasn't moved on".
This is the same residual class as mechanical path-only ownership, widened along
the interactive axis; the resume affordance (§3) names the migration id and decision
count so an operator can spot an unexpected session before replay mutates anything.

#### 2. Reorder the pre-flight: owned-check first, session-log scaffold second

**File**: `skills/config/migrate/scripts/run-migrations.sh:116-167`
**Changes**: Restructure the `if [ -n "$dirty" ]` block so the owned-check runs
**before** the session-log detection branch. The session-log scaffold (and the
generic refusal) become the *not-owned* arm:

```sh
  if [ -n "$dirty" ]; then
    if dirty_tree_fully_owned "$vcs" "$dirty"; then
      RESUME=1
      echo "Resuming over this run's own partial migration output:" >&2
      while IFS= read -r path; do
        [ -z "$path" ] && continue
        if is_session_log "$path"; then
          # §3: name it as an interactive migration being resumed, with the
          # decision count, and preserve today's verbatim discard line.
          abs="$path"; case "$abs" in /*) ;; *) abs="$PROJECT_ROOT/$path" ;; esac
          n=0; [ -f "$abs" ] && n=$(wc -l <"$abs" 2>/dev/null | tr -d ' ' || echo 0)
          echo "  $path  (interactive migration — resuming; replays $n decided, re-prompts undecided)" >&2
          echo "    to abandon instead: rm $path  (loses $n decisions)" >&2
        else
          echo "  $path" >&2
        fi
      done <<<"$dirty"
      # fall through into the apply loop; 0069 replay resumes any in-flight
      # interactive migration, the ledger skips applied ones, mechanical tail re-runs
    else
      # NOT fully owned → today's behaviour: steer in-flight session logs to
      # structured resume/discard, else the generic refusal.
      <existing session-log detection branch :117-161>
      refuse_dirty_tree
    fi
  fi
```

This supersedes Phase 3 §2's ordering (which placed guarded resume *after* the
session-log branch). The session-log scaffold now fires only when the tree is
**not** current-run-owned — exactly the stale/foreign case for which "re-run to
resume / rm to discard" is the right steer.

Two structural musts when extracting the existing branch into the `else` arm:

- **Reuse `is_session_log`** in the detector (replace the inline `grep -E
  'migrations-[0-9a-z-]+-session\.jsonl'` at `:121`) — the detector enumerates
  *logs only* (for decision counts), while the owned-check uses the family-wide
  `is_session_artifact`. The two share the `migrations-[0-9a-z]*-` id-class so they
  agree on what is this-run's, without the detector mislabeling a `stderr.log` as a
  decisions log.
- **Preserve the detector's terminal `exit 1`** (currently `:160`, inside `if [ -n
  "$dirty_session_logs" ]`). The `else` arm is `<detector — exits 1 when session
  logs present> ; refuse_dirty_tree`; if the refactor drops that inner `exit 1`, a
  stale in-flight session log would print the resume/discard scaffold **and then**
  the generic FORCE-hint refusal — contradictory guidance. Assert in a test that a
  stale session-log tree emits the scaffold but **not** the generic refusal line.

**Custom session-log paths:** a migration may declare a non-canonical path via
`migration_session_log_path` (`interactive-harness.sh:261`). Such a log escapes both
`is_session_artifact` and the detector, so an in-flight interrupt of it falls to the
*generic* refusal (FORCE-only) — neither resume nor steer. Phase 4 constrains the
declared path to the canonical `.accelerator/state/migrations-<id>-session.jsonl`
shape (assert in the harness READY handler), so the shared predicate is total over
real session logs.

#### 3. Surface resumed session logs in the affordance (preserve the discard path)

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: Today's session-log scaffold prints a structured discard affordance —
per-log decision count plus `To discard: rm <resolved-path>  (loses N decisions)`
and a VCS-aware `status` hint — built specifically to stop a confused operator
reaching for `jj abandon`. Auto-resuming must **not** silently drop it. **Hard
requirement:** when an owned dirty path is a session artifact, the resume affordance
must (a) name it as an interactive migration being resumed, with its migration id
and `wc -l` decision count, so the operator understands 0069 replays decided
transformations and re-prompts only undecided ones; and (b) **reproduce today's
discard line verbatim** — the resolved `rm <abs-path>` and `(loses N decisions)`
count and the VCS-aware status hint.

Drop the "Ctrl-C" framing: the common owned case is a *completed* interactive
migration whose log persists on disk (AC-5) with **no running process to
interrupt** — `rm` is the abandon path, not Ctrl-C.

Set expectations for the in-flight case: a guarded resume into an interactive
migration with *undecided* transformations re-prompts (or, with no TTY/decisions
channel, re-stalls via `emit_no_input_stall` and `exit 1`s — it does **not** silently
complete). The affordance should say so and point at the `--decisions-file` form for
non-interactive contexts, so "resume" doesn't imply "finishes on its own".

Add `assert_stderr_contains` for the interactive-resume phrasing **and** the
`To discard: rm` line + decision count in the Phase 4 tests, so §3's text is
CI-enforced (mirroring Phase 1's named-line discipline).

#### 4. Reconcile the 0116 breadcrumb (now behavioural)

**File**: `skills/config/migrate/scripts/interactive-lib.sh:320-322`
**Changes**: The stall text ("resume-safety for partial runs is tracked separately
(0119)") now resolves: the interactive axis *does* gain guarded resume. Update the
wording to reflect that a partial interactive run is resumable on re-run when the
base revision is unchanged, and remove the "tracked separately" deferral. This is
the behavioural reconciliation 0116 flagged as touching its task.

### Success Criteria:

#### Automated Verification:

- [x] **New harness helper** (prerequisite for the cases below): the existing
      interactive replay helpers run with `ACCELERATOR_MIGRATE_FORCE=1` and a *fake*
      `mkdir .git` (bypassing the pre-flight), so none can exercise guarded resume.
      Add a helper that does `git init`/`jj git init` + an initial commit, seeds
      `migrations-run-paths.txt` + `migrations-run.id` against the **current base
      revision**, sets `ACCELERATOR_MIGRATIONS_DIR`, and invokes the driver
      **without** FORCE. All Phase 4 cases use it. Each case captures exit code +
      stderr from a **single** driver run (`OUT=$(… 2>&1); RC=$?`) and asserts
      post-state by reading files — a guarded resume *mutates the fixture*, so a
      second assert-driven invocation would run against a now-clean tree.
- [~] New test "mixed run: interactive applied + mechanical fail resumes":
      implemented as a *seeded-state* variant — a dirty tree with both an owned
      session log (by pattern) and an owned mechanical path (manifest) + matching
      run-id → re-run **exits 0**, affordance lists both owned paths **and the
      discard line with the EXACT count** (`assert_contains "loses $LOGLINES
      decisions"`), and the pending mechanical stub completes. Asserts the same
      observables as the spec; the prior partial run is seeded rather than
      produced by a real interactive-applied-then-mechanical-fail sequence.
- [x] New test "in-flight interactive resumes via guarded resume": an interactive
      migration interrupted **before any mechanical delta** (empty manifest, run-id
      present, session log dirty, id **not** in the applied ledger) — re-run reaches
      guarded resume (this is the empty-manifest reachability fix: the owned-check
      gates manifest on `-r`, not `-s`) and 0069 replays decided transformations.
      Assert via the protocol log (`MIGRATION_PROTOCOL_LOG_MIGRATION`,
      RESUMED_APPLIED/PROMPT counts, per the `verify-applied` model ~:940-968) that
      decided transformations are **not** re-prompted — resume, not restart.
- [x] New test "stderr.log is owned (jj)": a `command -v jj`-guarded case whose dirty
      set includes a preserved `migrations-<id>-stderr.log` alongside the session log
      → guarded resume still proceeds (exit 0). Guards the most failure-prone member
      of the family (the one §1 warns defeats resume under jj if unrecognised) — a
      future predicate edit dropping the stderr arm must fail CI.
- [x] New test "custom session-log path rejected": a fixture migration declaring a
      non-canonical `migration_session_log_path` → the READY handler refuses
      (non-zero + named error), so §2's "predicate is total over real session logs"
      claim is enforced (else such a log falls through to the generic FORCE-only
      refusal — neither resume nor steer).
- [x] New test "stale session log still steers": a dirty session log + a
      `migrations-run.id` **overwritten with a sentinel non-matching revision**
      (`printf 'stale-rev\n' > …/migrations-run.id` — isolates the revision-mismatch
      branch, distinct from AC4's empty-sidecar case) → re-run **refuses** with the
      resume/discard scaffold, **and not** the generic FORCE-hint refusal line
      (asserts the else-arm's terminal `exit 1` is preserved).
- [~] New test "block-vs-resume pivot": covered by the complementary pair rather
      than one dedicated test — the existing session-log block test exercises the
      no-run-id ⇒ fail-closed (block) path, and "in-flight interactive resumes"
      exercises the matching-run-id ⇒ resume path.
- [x] New test "near-miss filename not owned": an **otherwise fully-owned** dirty
      tree (seeded manifest paths + matching run-id) **plus** one near-miss
      `.accelerator/state/` file that is not a canonical session artifact (e.g.
      `migrations-0002-session.jsonl.bak`) → re-run **refuses**. The near-miss must
      be the *sole* non-owned path, so a loosened predicate that wrongly owned it
      would flip the result to resume and fail the test (a bare near-miss-only tree
      would refuse for the wrong reason — path simply absent from the manifest).
- [x] Existing interactive suite (`test-migrate-interactive.sh`) passes unchanged,
      including the session-log-dirty block test (now exercising no-run-id
      fail-closed) and AC-5's post-success session-log persistence.
- [~] Full default gate green end-to-end: `mise run` — `mise run check` (full
      read-only gate, all four components) + the three affected shell test suites
      are green; the bare full gate (heavy Rust/frontend rebuild + all suites) was
      not re-run since the change is shell-only.

#### Manual Verification:

- [ ] Real mixed run under jj: interactive migration applies, mechanical one fails;
      re-run resumes without FORCE and completes, with the interactive migration
      **not** re-prompted (already applied).
- [ ] Interrupt an interactive migration mid-prompt, then re-run: it resumes
      (decided transformations skipped) rather than blocking with FORCE-only.

---

## Testing Strategy

### Unit Tests:

The four AC tests and mechanical lifecycle tests live in
`skills/config/migrate/scripts/test-migrate.sh`; the **Phase 4** interactive-axis
tests (mixed-run resume, in-flight replay, stale-session-log steer) live in
`test-migrate-interactive.sh`, which already has the interactive harness, the jj
fixtures, and the protocol-log assertions those cases need. One test per AC,
modelled on existing blocks:

- **AC1 — manifest correctness after partial failure** (model: Test 4's
  heredoc-stub idiom `:138-154` + exact-line assertions `:1004-1012`, but in a
  **real initialised repo** like Test 14 — *not* Test 4's fake `mkdir .git`, which
  the diff-based recorder cannot read). New stub writes a known set of scoped
  paths (e.g. `meta/work/aaa.md`, `meta/work/bbb.md`) then `exit 1`s. Assert
  `migrations-run-paths.txt` contains exactly those paths: per-path
  `grep -cFx '<path>' == 1` plus `wc -l` equals the count. Include a case where a
  path is mutated across **two** recording points (success then failure) so
  `atomic_append_unique`'s idempotent dedup is exercised, not just distinct paths.
- **AC2 — guarded resume on fully-owned dirty tree** (model: Test 14
  `:350-376`). Real git/jj repo; induce a partial run, then re-run; assert exit 0,
  `assert_stderr_contains` the affordance token and each owned path. Also assert a
  dirty tree of owned migration paths **plus** the bookkeeping sidecars
  (`migrations-applied`/`-run-paths.txt`/`-run.id`) still resumes (the implicit-
  ownership carve-out).
- **AC3 — refusal on mixed/non-owned tree**: as AC2 plus one hand-added foreign
  dirty path; assert non-zero exit, `assert_stderr_not_contains` the affordance,
  `assert_stderr_contains "ACCELERATOR_MIGRATE_FORCE"`. Include a foreign
  `.accelerator/`-prefixed path that is *not* one of the four bookkeeping files, to
  prove the carve-out matches by exact path, not prefix.
- **AC4 — fail-closed on unusable/stale manifest** (over a dirty tree containing a
  **mechanical** path — an empty manifest refuses because that path is unowned, vs.
  the Phase 4 in-flight case where an empty manifest + only a session artifact
  resumes): loop over {manifest absent, manifest empty, run-id absent, run-id empty,
  **run-id present but recorded base revision ≠ current**}; each asserts the AC3
  observable refusal (incl. the
  FORCE-hint message, not just a non-zero exit).

Plus lifecycle tests folded into Phase 2 (manifest deleted on success; fresh run
truncates leftover manifest; **and a stale leftover manifest is cleared on the
no-pending early-exit** — a distinct code path from post-loop success) and the
resume-that-fails-again accumulation test (Phase 3). **All AC2–AC4 tests run under
both git and jj** (jj guarded by `command -v jj`, mirroring the existing
`test-migrate-interactive.sh` jj fixtures) — the suite has no jj coverage today,
yet the `change_id` revision capture and the untracked-handling are jj-specific.

**jj fixtures must invoke the runner from *within* the sandbox** (`cd "$REPO" &&
…`): the runner's jj commands (`jj diff`, `jj log -r @`) run in cwd, not against
`$PROJECT_ROOT` (unlike the git branches, which are `-C "$PROJECT_ROOT"`-anchored)
— the existing interactive suite documents this exact gotcha. A jj test that sets
only `PROJECT_ROOT=` without `cd`-ing would enumerate/measure the *outer* repo.
Also note `assert_stderr_contains`/`_not_contains` re-invoke the command, and a
guarded resume *mutates the fixture* on that invocation — so capture exit code and
stderr from a **single** run (the suite's `OUT=$(… 2>&1); RC=$?` idiom), not two
separate driver runs over a now-changed tree.

If any new helper is added to `atomic-common.sh` (not currently planned — we reuse
`atomic_append_unique`/`atomic_write`), extend
`scripts/test-atomic-common.sh:47-67`.

### Integration Tests:

- `mise run test:integration:migrate` runs the migrate suites.
- Suite-count floor `tasks/test/integration.py:8` (`_EXPECTED_MIGRATE_SUITES = 4`)
  is unaffected — no new suite file is added.

### Manual Testing Steps:

1. In a jj repo, write a stub migration that mutates a `meta/` file then `exit 1`.
   Run `/accelerator:migrate`; confirm abort + `migrations-run-paths.txt` lists the
   path.
2. Re-run; confirm it resumes without FORCE and prints the affordance.
3. Hand-edit an unrelated `meta/` file, re-run; confirm refusal + FORCE hint.
4. `rm .accelerator/state/migrations-run.id`, re-run; confirm fail-closed refusal.
5. Repeat 1–4 in a git repo.

## Performance Considerations

- `manifest_record_delta` is O(dirty × manifest) per migration via repeated
  `grep -Fxq` + `atomic_append_unique` (each rewrites the manifest). For the modest
  path counts of a real migration batch this is negligible; it matches the existing
  `atomic_append_unique` ledger cost. If a future migration mutates thousands of
  paths, batch the append (accumulate + single `sort -u` write) — not needed now.

### Limitations (accepted, documented):

- **Path-only ownership (narrowed by the revision gate).** Ownership is by path,
  not content. The base-revision staleness gate refuses once the operator has
  *committed* since the failed run (the working copy moved on), so the residual is
  only this: an operator who hand-edits the manifest's exact paths **without
  committing** and then re-runs can have the guarded resume proceed over those
  edits (base revision unchanged, paths still match). VCS revert remains the
  backstop; a content-hash or transaction boundary is explicitly out of scope.
- **Interrupt-on-success window.** If the runner is killed (SIGKILL/Ctrl-C)
  between the final migration's success and the manifest deletion, a stale manifest
  describing a fully-applied run is left behind. The applied-ledger still prevents
  re-application, so the only effect is a misleading resume affordance on the next
  dirty re-run; a subsequent clean run truncates it. Accepted.
- **"Migrations must not commit" is a load-bearing contract.** The base-revision
  staleness gate assumes `change_id`/`HEAD` is stable for the run's duration — true
  only because migrations mutate the tree but never commit. A migration that ran
  `jj commit`/`git commit` (directly or via a helper that auto-commits) would move
  the base revision mid-run, making the recorded run-id stale against itself and
  turning a legitimate resume into a fail-closed refusal — i.e. re-introducing the
  exact problem 0119 fixes, via an authoring mistake rather than an operator
  commit. This is confirmed today; recorded here so future migration authors know
  committing breaks resume.
- **0069 replay-on-entry is a load-bearing contract (Phase 4).** Phase 4 lets an
  in-flight interactive migration fall through into the apply loop on the assumption
  that the harness reads the session log on entry (`build_resume_state_file`) and
  replays decided transformations rather than restarting. If that replay regressed
  (e.g. a session-log schema bump rejected on entry), guarded resume would re-enter a
  migration that re-prompts from scratch or aborts. The Phase 4 protocol-log
  assertion (decided transformations not re-prompted) is the CI guard that fails
  closed if replay regresses; recorded here as the symmetric contract to "migrations
  must not commit".
- **Interactive ownership is by base-revision + path-pattern, wider than the
  mechanical manifest (Phase 4).** A current-base-revision session artifact is owned
  regardless of whether its migration id is in this run's pending/applied set (the
  pending set isn't known at pre-flight). The base-revision gate bounds it to "the
  working copy hasn't moved on"; the resume affordance names the id + decision count
  so an operator can spot an unexpected session before replay. Same residual class
  as path-only ownership, widened along the interactive axis. VCS revert is the
  backstop.

## Migration Notes

No data migration. The manifest + run-id sidecar are new run-scoped artifacts under
`.accelerator/state/`; they are created on demand and deleted on full success. A
manifest left by a pre-0119 runner cannot exist (the files are new), so there is no
backward-compatibility concern. The fresh-start truncation invariant guarantees a
leftover manifest from any failed run is reset on the next clean-tree start.

## References

- Original work item: `meta/work/0119-resume-safe-partial-migration-failure.md`
- Research: `meta/research/codebase/2026-06-21-0119-resume-safe-partial-migration-failure.md`
- Source issue research:
  `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
- Parent epic: `meta/work/0115-make-interactive-migrations-satisfiable-under-agent-invocation.md`
- Apply loop / pre-flight: `skills/config/migrate/scripts/run-migrations.sh:94-168,281-324`
- Recording precedents: `scripts/atomic-common.sh:16-61`,
  `0003-relocate-accelerator-state.sh:120,196-218`
- Fail-closed identity precedent: `scripts/launcher-helpers.sh:157-167`
- Test models: `skills/config/migrate/scripts/test-migrate.sh:138-154,350-376,1004-1012`
