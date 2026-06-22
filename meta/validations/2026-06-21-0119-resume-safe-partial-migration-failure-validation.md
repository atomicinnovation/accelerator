---
type: plan-validation
id: "2026-06-21-0119-resume-safe-partial-migration-failure-validation"
title: "Validation Report: Resume-Safe Partial Migration Failure"
date: "2026-06-22T14:18:49+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
parent: "plan:2026-06-21-0119-resume-safe-partial-migration-failure"
target: "plan:2026-06-21-0119-resume-safe-partial-migration-failure"
tags: [migrate, interactive-migration, agent-invocation, tooling, manifest]
last_updated: "2026-06-22T14:18:49+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Resume-Safe Partial Migration Failure

### Implementation Status

- ✓ **Phase 1: Extract a shared scoped-dirty enumeration helper** — Fully implemented
- ✓ **Phase 2: Write the per-run path manifest (recording half)** — Fully implemented
- ✓ **Phase 3: Manifest-driven guarded-resume branch (reading half)** — Fully implemented
- ✓ **Phase 4: Reconcile the interactive session-log axis** — Fully implemented

All four phases landed as discrete commits in the planned merge order
(Phase 1 → 2 → 3 → 4), plus a fifth commit marking the plan's success
criteria complete:

- `stkqovyy` — Extract shared scoped-dirty enumeration helper in migration runner
- `nouommqt` — Record a per-run path manifest as migrations apply
- `rnrttwxm` — Add manifest-driven guarded-resume branch to the pre-flight
- `qloysqql` — Reconcile the interactive session-log axis with guarded resume
- `yonrkqkr` — Mark 0119 plan success criteria complete after implementation

### Automated Verification Results

- ✓ `bash skills/config/migrate/scripts/test-migrate.sh` — **533 passed, 0 failed**
- ✓ `bash skills/config/migrate/scripts/test-migrate-interactive.sh` — **187 passed, 0 failed**
- ✓ `bash skills/config/migrate/scripts/test-migrate-0007.sh` — **190 passed, 0 failed**
- ✓ `mise run scripts:check` — clean (shellcheck + bashisms incl. bash-3.2 floor, format, exec-bits)
- ✓ `mise run check` — **exit 0** end-to-end (format + lint + types across all four components)

The full default gate (`mise run`, with the heavy Rust/frontend rebuild)
was not re-run, consistent with the plan's own note that the change is
shell-only and the affected suites + `mise run check` are the meaningful
coverage. This matches the `[~]` markers the implementer left on those
specific criteria.

### Code Review Findings

#### Matches Plan:

- **`enumerate_scoped_dirty` (Phase 1)** — single helper at
  `run-migrations.sh:112-124`. The jj branch includes untracked; the git
  branch keeps `grep -v '^??'` and strips the porcelain status prefix plus
  the rename `s/^.* -> //` exactly as specified. Pre-flight calls it via
  `dirty=$(enumerate_scoped_dirty "$vcs")` at `:270`.
- **Manifest sidecar pair (Phase 2)** — `RUN_PATHS_FILE` / `RUN_ID_FILE`
  defined at `:31-32`; `manifest_record_delta` (`:132-140`),
  `current_base_revision` using `change_id` for jj / `HEAD` for git
  (`:149-156`), and `clear_run_manifest` (`:162-164`) all present.
  Run-identity minting + baseline capture placed after the no-pending
  early-exit, immediately before the loop (`:472-497`). Delta recorded on
  both success and failure before any `exit 1` (`:524-531`).
- **`RESUME=0` and `vcs` hoisted unconditionally** to script top
  (`:258-267`), above the FORCE gate, so the `set -u` / FORCE-path
  reasoning in the plan holds.
- **Inline `rm -f "$BASELINE_FILE"`** before every reachable `exit` (no
  EXIT trap), matching the plan's deliberate cleanup convention
  (`:513`, `:527`, `:556`).
- **`dirty_tree_fully_owned` (Phase 3)** — `:215-250`. Fail-closed
  usability gate (`-r`/`-s` on run-id, `-r` on manifest), base-revision
  staleness comparison, derived (not hard-coded) bookkeeping carve-outs,
  and the quoted-`case` literal match are all as designed. Manifest gated
  on `-r` not `-s` (the empty-manifest reachability fix for in-flight
  interactive resume).
- **`refuse_dirty_tree` single-definition** (`:170-176`); both the
  guarded-resume `else` arm and the original refusal site call it.
- **Owned-check-first reorder (Phase 4)** — `:272-357`. The session-log
  scaffold and generic refusal are now the not-owned arm, exactly as the
  Phase 4 restructure specifies.
- **`is_session_log` / `is_session_artifact` two-predicate split**
  (`:183-205`) sharing the `migrations-[0-9a-z]*-` id-class; the artifact
  predicate recognises `.jsonl`, `-stderr.log`, and `-resume-state.tmp`,
  FIFOs deliberately omitted.
- **Resume affordance** names the interactive migration, prints the
  `wc -l` decision count, the `--decisions-file` non-interactive hint, and
  reproduces the discard line — `:285-302`.
- **Custom session-log path rejection (Phase 4 §2)** — `interactive-lib.sh`
  now returns non-zero with a named error when a migration declares a
  non-canonical `migration_session_log_path` (`:498-509`).
- **0116 breadcrumb reconciled (Phase 4 §4)** — the stall text at
  `interactive-lib.sh:320-323` no longer says "tracked separately (0119)";
  it now states the partial run resumes on re-run when the base revision is
  unchanged.

#### Deviations from Plan:

- None material. The implementation tracks the plan's pseudocode and
  line-level guidance closely. Test phasing differs only in placement
  (Phase 4 interactive tests live under a "Phase 8" section header in
  `test-migrate-interactive.sh`), which is cosmetic.

#### Potential Issues:

- **Accepted residuals are as documented, not defects.** Path-only
  ownership over uncommitted hand-edits, the interrupt-on-success stale
  manifest window, the "migrations must not commit" contract, 0069
  replay-on-entry as a load-bearing contract, and the wider
  base-revision+pattern interactive ownership are all recorded under
  Limitations in the plan and backed by tests (the protocol-log assertion
  guards replay regression; the stale-revision test guards the staleness
  gate).
- **Path-with-spaces edge** (git C-quoted porcelain) remains a noted known
  edge; the kebab-case `meta/` corpus does not trigger it. No action needed.

### Manual Testing Required:

The plan lists manual spot-checks; none are blockers for correctness given
the automated coverage (AC2–AC4 are exercised under both git and jj), but
they remain available as sanity diffs:

1. Real partial failure under jj:
   - [ ] Stub that `mv`s a scoped file then `exit 1`s → confirm
     `migrations-run-paths.txt` lists the moved path repo-relative.
   - [ ] Re-run resumes without `ACCELERATOR_MIGRATE_FORCE=1` and prints the
     affordance listing owned paths; pending migrations complete.
2. Fail-closed behaviours:
   - [ ] Add one foreign hand-edited `meta/` file → re-run refuses with the
     FORCE hint, no affordance.
   - [ ] `rm .accelerator/state/migrations-run.id` → re-run refuses
     (fail-closed).
3. Interactive axis:
   - [ ] Mixed run (interactive applies, mechanical fails) → re-run resumes,
     interactive migration not re-prompted.
   - [ ] Interrupt an interactive migration mid-prompt → re-run resumes
     decided transformations rather than blocking FORCE-only.
4. git VCS parity:
   - [ ] Repeat fully-owned / mixed / fail-closed in a git repo.

### Recommendations:

- **Mergeable as-is.** All automated success criteria pass; the
  implementation faithfully realises the plan including the bash-3.2 floor,
  the single-enumeration-source coupling, and the fail-closed staleness
  gate.
- The manual steps above are worth a one-time spot-check before release
  given migrations are destructive, but the AC2–AC4 jj+git automated
  coverage already exercises the core decision matrix.
- No follow-up work items implied; the deferred boundaries are explicit and
  intentional.
