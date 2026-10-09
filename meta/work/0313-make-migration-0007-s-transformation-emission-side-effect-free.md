---
type: "work-item"
id: "0313"
title: "Make Migration 0007's Transformation Emission Side-Effect-Free"
date: "2026-10-09T09:34:53+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "high"
parent: "work-item:0312"
relates_to: ["work-item:0202", "work-item:0172", "work-item:0117"]
tags: ["migration-engine", "m0007", "interactive-contract"]
last_updated: "2026-10-09T09:34:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0313: Make Migration 0007's Transformation Emission Side-Effect-Free

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Migration 0007 runs its whole mechanical rewrite inside
`emit_transformations`, the callback meant only to enumerate transformations.
So `accelerator migrate --list`, documented as non-mutating, rewrites the
corpus, and the next real run then refuses over that output as foreign dirt.
This converges m0007 on a side-effect-free `emit_transformations`, the
contract the ADR-0037 successor records.

## Context

`--list` and the `--decisions-file` dry-apply call `emit_transformations` to
enumerate pending prompts (`cli/migrate/src/engine.rs:120-159`). m0007 also
uses it as a pre-prompt mechanical phase (`m0007/mod.rs:1-11, 122-200`). The
module doc gives the reason: it is the only callback the engine calls
unconditionally before prompting. `skills/config/migrate/SKILL.md` contradicts
itself: L136 places mechanical work there, while L143 requires repeatable
callbacks to be side-effect-free.

## Requirements

Reproduction (verified at revision `e76a8a9b`):

1. In a VCS repo, record migrations 0001–0006 as applied and commit two work
   items whose frontmatter predates the unified schema.
2. Run `accelerator migrate --list`.
3. Run `accelerator migrate`.

Expected: step 2 leaves the working copy unchanged, and step 3 runs.
Actual: step 2 rewrites both work items and writes
`.accelerator/state/migrations-run-paths.txt`. Step 3 refuses with
`Unowned changes (2)`.

Fix:

- Compute the pre-pass, the fence-less backfill and the rewrite in memory
  inside `emit_transformations`, with no write through `MigrationContext`.
- Emit one transformation per changed file, routed `Mechanical`, whose
  `apply_decision` writes that file. These come ahead of the linkage
  transformations, so the rewrites land before any linkage applies.
- Validate the rewritten frontmatter in memory before emitting, so a
  structural failure still aborts before any prompt.
- Reconcile `SKILL.md`'s callback contract so that `emit_transformations`
  enumerates and must be side-effect-free.

## Acceptance Criteria

- [ ] Given a committed corpus that 0007 would rewrite, when `accelerator
      migrate --list` runs, then the VCS diff is empty and
      `.accelerator/state/migrations-run-paths.txt` does not exist.
- [ ] Given that state after `--list`, when `accelerator migrate` runs, then
      the pre-flight does not refuse.
- [ ] Given a decisions file that fails dry-apply validation, when the run
      aborts, then no corpus file has changed.
- [ ] Given a corpus the pre-pass refuses or whose rewrite fails structural
      validation, when the run aborts, then no corpus file has changed and
      no prompt was shown.
- [ ] Given a run interrupted after some file rewrites, when it is re-run to
      completion, then the corpus matches an uninterrupted run's.
- [ ] Given the existing m0007 and `--list` test suites, when they run,
      then they pass, and `--list` gains a test seeded with a corpus the
      rewrite changes.
- [ ] Given `skills/config/migrate/SKILL.md`, when its callback contract is
      read, then it nowhere places mutation in `emit_transformations`.

## Open Questions

None.

## Dependencies

- Blocked by: none.
- Blocks: none. The ADR-0037 successor (0202) records the rule and cites
  this item as known non-compliance, so it does not wait for this fix.

## Assumptions

- Structural validation of in-memory content needs a content-taking variant
  of `MigrationContext::validate_frontmatter`, which today reads paths from
  disk (`cli/migrate-adapters/src/context.rs:240-255`). Moving validation to
  `finalise` instead was rejected: it would let a user answer every prompt
  before a structural failure.

## Technical Notes

- Already in-memory: `prepass::precondition_prepass`, `backfill::backfill`
  and `rewrite::rewrite`. The rewrite is idempotent (`rewrite.rs:842-844`).
  The only mutations are at `m0007/mod.rs:155, 180`.
- The corpus index is path-derived (`corpus_index.rs:27-43`). Linkage type
  and parse read content, so they can run on the in-memory rewrite.
- `PredicateOutcome::Mechanical` sends a transformation to `apply_decision`
  without a prompt or a session-log record (`engine.rs:62-71`). Emission
  order is canonical.
- A `prepare` hook was considered and rejected: `--list` would have to run
  it (mutating) or skip it (enumerating against the pre-rewrite corpus).

## Drafting Notes

- Kind is `bug` because the user-visible symptom is the `--list` refusal.
  The convergence refactor is the fix.
- Priority is high: previewing with `--list`, as the documented agent
  workflow does, blocks the real run on any corpus 0007 still rewrites.

## References

- Codebase research:
  `meta/research/codebase/2026-10-09-0202-reconcile-migration-engine-adrs-against-the-rust-port.md`
  (Follow-up Research section)
- Related: 0202, 0172, 0117
