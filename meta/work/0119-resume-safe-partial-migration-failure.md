---
type: work-item
id: "0119"
title: "Resume-Safe Partial Migration Failure"
date: "2026-06-19T23:13:17+00:00"
author: Toby Clemson
producer: refine-work-item
status: ready
kind: task
priority: high
parent: "work-item:0115"
relates_to: ["work-item:0069", "work-item:0116", "work-item:0118", "work-item:0120"]
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-20T13:21:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-141
---

# 0119: Resume-Safe Partial Migration Failure

**Kind**: Task
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

When a re-run is blocked by a dirty tree whose only dirty paths are this run's
own prior-migration output, offer a guarded resume. Today the sole escape is
`ACCELERATOR_MIGRATE_FORCE=1`, which disables the single guard protecting the
corpus for *every* dirty path, not just this run's own output. This is fix E of
0115. Ownership is determined from a **per-run path manifest** of mutated paths,
written as each migration applies and read back on re-run (see Requirements). A
full staging/transaction boundary remains out of scope.

## Context

Child of 0115 — Make Interactive Migrations Satisfiable Under Agent Invocation.

The apply loop has no transaction boundary: each migration's writes land on the
tree as it runs, and `exit 1` on a later failure leaves all prior mutations in
place. On re-run, the clean-tree pre-flight sees those mutations as a dirty tree
and refuses, leaving the operator with only `ACCELERATOR_MIGRATE_FORCE=1`, which
bypasses the *entire* dirty-tree check — the sole corpus guard — not just this
run's own output.

## Requirements

- **Record ownership as migrations apply.** Maintain a per-run path manifest — a
  plain-text file, one repo-relative path per line, co-located with the
  per-migration ledger in the run's state area (exact filename to be fixed at
  planning). Append each path **at the moment it is mutated**, not only on
  whole-migration completion, so the writes of a migration that fails mid-way
  (mutates some paths, then errors) are still recorded and counted as owned. A
  path is "owned by this run" iff it appears in this manifest. (This is the
  operational definition of "this run's own prior-migration output" used
  throughout.)
- **Detect a fully-owned dirty tree.** On a blocked re-run, the clean-tree
  pre-flight computes the owned set from the manifest and determines whether
  *every* dirty path is owned.
- **Offer a guarded resume only when the dirty tree is fully owned.** When every
  dirty path is owned, the pre-flight proceeds into the apply loop without
  requiring `ACCELERATOR_MIGRATE_FORCE=1`, emitting a resume-affordance message
  to stderr that lists **every** owned dirty path it is resuming over. The
  guarded resume is reachable *only* in this fully-owned case.
- **Preserve the existing refusal otherwise.** When any dirty path is not in the
  manifest, the pre-flight still refuses and does not offer the guarded resume —
  the corpus guard is never relaxed for non-owned paths (this holds by
  construction, since resume is unreachable on a mixed tree).
- **Fail closed on an unusable manifest.** When the manifest is absent, empty,
  unreadable, or carries a different run's identity (stale), treat the owned set
  as empty so any dirty tree is refused. A missing or stale manifest must never
  be interpreted as "everything owned" — the default is the existing refusal,
  never a relaxed guard.
- Do not introduce a full staging/transaction boundary — out of scope.

## Acceptance Criteria

- [ ] Given a stub migration that mutates a known set of paths and then exits
      non-zero (or the documented 0007 partial-failure scenario), when the run
      aborts, then the per-run manifest file contains exactly the paths mutated
      before the failure — including the failing migration's partial writes —
      one repo-relative path per line (manifest correctness, asserted against the
      named artefact).
- [ ] Given a partial-run failure whose dirty paths are *all* present in this
      run's manifest, when the operator re-runs, then the pre-flight proceeds
      into the apply loop (exit 0, guarded resume) without requiring
      `ACCELERATOR_MIGRATE_FORCE=1`, and emits a resume-affordance message to
      stderr that lists every owned dirty path being resumed over.
- [ ] Given dirty paths that include at least one path *not* in this run's
      manifest, when the operator re-runs, then the pre-flight still refuses and
      does not offer the guarded resume. The refusal is observable: the run exits
      non-zero, no resume-affordance message is emitted, and the existing
      dirty-tree refusal message (the `ACCELERATOR_MIGRATE_FORCE` hint) is
      present — so "the corpus guard is unchanged" is asserted, not just narrated.
- [ ] Given the manifest is absent, empty, unreadable, or from a different run
      (stale), when the operator re-runs over a dirty tree, then the pre-flight
      fails closed with the same observable refusal as the mixed-tree case (exit
      non-zero, no resume-affordance message, dirty-tree refusal message present)
      — an unusable manifest is never treated as "everything owned".

## Open Questions

- **Resolved** — "this run's own prior-migration output" is identified via a
  **recorded per-run path manifest** of mutated paths, written as each migration
  applies (see Requirements). The session-log and migration-ledger options were
  rejected: the session log is scaffolded for interactive resume and need not
  capture all mechanical output paths, and the ledger records migration ids
  rather than the paths each migration wrote. A full staging/transaction
  boundary was also considered and rejected as out of scope.

## Dependencies

Fix-letter map (from the source research's Fix Options): **A** = 0117
(agent-decisions bridge / invoker contract), **B** = 0116 (structured stall on
no-decision input), **C** = 0118 (reconcile 0007 backfill sentinel with
validator), **D** = the 0007-split (mechanical unification vs. human-run
linkage) — **not yet decomposed into a work item**, **E** = this item (0119).

- Blocked by: none. (The ownership-identification design question is now
  resolved — see Open Questions — so the task is plannable.)
- Ordered after: 0118 (fix C) and fix D (0007-split, undecomposed). The source
  research characterises E as *secondary to C/D* — valuable hygiene that treats
  a symptom of the partial-run dirty tree those fixes address at root. Because D
  has no work item yet, the "after D" half of this ordering is unenforceable
  until D is decomposed; schedule accordingly (or create D first). This is a soft
  ordering preference, not a hard blocker — E can be built independently, but
  prioritising it ahead of C/D risks guarding a symptom the root fixes remove.
- Blocks: none.
- Relates to:
  - 0115 (parent epic) and 0069 (runner-side resumability this extends).
  - 0120 (prevention tests). 0120 covers only the no-input structured-stall and
    backfill/validator prevention tests; guarded-resume coverage is owned by this
    task's *own* tests, not 0120 — so this is a relates-to link, not a block of
    0120.
  - **Shared edit surface.** This task touches both `run-migrations.sh` (the
    manifest append in the apply loop at `:294`/`:252-297`, and the pre-flight at
    `:67-141`) and the per-migration completion region. 0116 (fix B) edits the
    same `run-migrations.sh` pre-flight/runner region; 0118 (fix C) edits `0007`
    and may touch the same per-migration completion point where this task appends
    to the manifest. Coordinate edits to both `run-migrations.sh` and the
    `0007`/completion region with 0116 and 0118 to avoid conflicts.

## Assumptions

- Existing run artifacts (session log / ledger) do **not** reliably capture the
  full set of mutated paths, so a new lightweight per-run path manifest is
  introduced as the ownership source (decision recorded in Open Questions).
- The manifest write integrates cleanly into the existing per-migration
  completion point (alongside the ledger append at `run-migrations.sh:294`)
  without a transaction boundary.
- The manifest carries enough run identity (e.g. a run id or start timestamp) to
  distinguish the current run's manifest from a stale one left by a prior run,
  enabling the fail-closed staleness check.

## Technical Notes

**Size**: M–L — two coherent halves: (1) write the per-run path manifest as
migrations apply, and (2) a manifest-driven guarded-resume branch in the
pre-flight check. Larger than the original M estimate because of the new manifest
artifact; the ownership-identification design question is now resolved, so the
task is ready to plan.

- New per-run **path manifest**: written incrementally as each migration
  completes, recording the paths it mutated. Integrate at the existing
  per-migration completion point next to the ledger append — `run-migrations.sh`
  `:294` (`atomic_append_unique`), apply loop `:252-297`.
- Clean-tree pre-flight and the `ACCELERATOR_MIGRATE_FORCE` bypass —
  `run-migrations.sh:67-141`; the FORCE guard at `:68` skips the *entire* block.
  The guarded-resume branch is gated on a fully-owned dirty tree computed from
  the manifest, leaving this bypass for genuinely foreign dirty paths.
- In-flight interactive session-log detection among dirty paths (adjacent
  resume-hint scaffolding; not the ownership source) — `run-migrations.sh:90-132`.

## Drafting Notes

- Decomposed from 0115 fix E.
- Review 1 (review-work-item) resolved the two structural majors: ownership is
  now defined via a per-run path manifest, and the guarded resume runs only on a
  fully-owned dirty tree (so the former AC3 mixed-path criterion folded into the
  refusal criterion). The path-manifest choice was considered for splitting into
  its own work item (it adds a persisted artifact), but the manifest write and
  the resume read are inseparable halves of one capability, so they stay
  together with the size bumped to M–L.
- Review 1 pass 2 (re-review) cleared all original findings but surfaced
  second-order ones from the concrete manifest design; this pass addressed them:
  manifest location/format and stderr message contract pinned down, a fail-closed
  criterion added for a missing/stale manifest (closing a guard-regression hole),
  per-mutation recording timing clarified, the fix-letter→work-item map added,
  C/D sequencing promoted to an "Ordered after" entry with fix D flagged as
  undecomposed, and 0120's guarded-resume coverage recorded as a downstream block.
- Review 1 pass 3 (re-review): all prior majors cleared; gave the refusal
  criteria (AC3/AC4) an explicit observable signal (non-zero exit, no
  resume-affordance message, FORCE-hint present) so the guard-not-relaxed
  guarantee is verifiable. Verdict APPROVE. Two follow-ups deferred out of the
  work item: (1) decompose fix D (the 0007-split) or accept E as standalone
  hygiene — an epic-0115 decision; (2) assertion-grade detail (exact stderr
  marker token, AC4 condition-splitting, manifest dedup/ordering) to be settled
  in /create-plan.

## References

- Source: `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
- Related: 0115, 0069
