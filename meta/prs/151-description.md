---
type: "pr-description"
id: "151"
title: "[0202] Research the migration-engine ADR supersession and open the improvements epic"
date: "2026-10-09T18:08:29+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0202"
parent: "work-item:0202"
relates_to: ["work-item:0312", "work-item:0313", "work-item:0314", "work-item:0315", "work-item:0316", "work-item:0317", "work-item:0318", "work-item:0319", "work-item:0136"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/151"
pr_number: 151
tags: ["migration-engine", "adr", "research", "work-items"]
revision: "5bfb87a194a6b24cf6ad52b84a4529beef249831"
repository: "accelerator"
last_updated: "2026-10-09T18:08:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0202] Research the migration-engine ADR supersession and open the improvements epic

## Summary

Readies work item 0202, which supersedes ADR-0023, ADR-0037 and ADR-0038
with Rust-native successors, and adds the codebase research that grounds it.
The research shows the Rust migration engine diverges from its recorded
contract in several places. This PR opens a Migration Engine Improvements
epic (0312) for those divergences, with six child items, and moves 0202
under it. It changes documents only; no code is touched.

## Changes

- **0202 readied** (`meta/work/0202-…`):
  - Retitled to "Supersede Migration-Engine ADRs with Rust-Native Successors" and set to `ready` after five review passes. The final verdict is APPROVE (`meta/reviews/work/0202-…-review-1.md`).
  - In this PR's later commits, re-parented from 0136 to 0312.
  - Its purity requirement now states option a: `emit_transformations` enumerates and never mutates, and m0007's non-compliance is tracked by 0313.
- **Research** (`meta/research/codebase/2026-10-09-0202-…`):
  - The gate check: all three ADRs' triggering text no longer holds.
  - A mapping of every contract primitive onto `InteractiveMigration` methods and engine ports.
  - A classification of ADR-0038's eight session-log fields.
  - Proposed Restated/Superseded splits.
  - A follow-up showing `emit_transformations` can be made pure.
- **Epic 0312, Migration Engine Improvements** (standalone): scope covers the engine crates, m0001–m0010 and `skills/config/migrate/SKILL.md`. Children:
  - **0313** (bug, high): make m0007's emission side-effect-free. This fixes `--list` mutating the corpus.
  - **0314** (story, low): resolve the ADR-0038 affordances that never shipped. These are the display extras, path-form edits, accept-degraded and type-pair validation.
  - **0315** (bug, medium): an unknown applied ID silences the SessionStart lag advisory.
  - **0316** (bug, low): the stall lists already-decided and mechanical transformations as pending.
  - **0317** (task, low): add test guards for registry order and ID uniqueness.
  - **0318** (story, medium): give `verify_applied` read access to the corpus, so resume can verify an applied decision.
- **0319** (story, under 0146): add `--id` to `accelerator work create`.
- **0136**: removes 0202 from its child list and Children line.

## Context

- Work item: `meta/work/0202-reconcile-migration-engine-adrs-against-the-rust-port.md`
- Research: `meta/research/codebase/2026-10-09-0202-reconcile-migration-engine-adrs-against-the-rust-port.md`
- Work items in this PR are numbered from 0312 to avoid collisions with
  items in flight on other branches. That collision risk is also why 0319
  exists: today `work create` always allocates max+1 locally.

## Testing

- [x] `accelerator corpus frontmatter validate` passes for all 12 changed files.
- [x] `accelerator work list --parent 0312` lists 0202 and 0313–0318.
- [x] Reproduced 0313's bug against `accelerator-migrate` built at `e76a8a9b`. Running `--list` on a committed corpus that 0007 rewrites changed two work items and wrote `migrations-run-paths.txt`. The next run then refused with `Unowned changes (2)`.
- [x] Reproduced 0315's bug: with `9999-from-a-newer-plugin` in the applied ledger, the discoverability hook emitted nothing although nine migrations were unapplied.
- [ ] 0316 was found by reading the code (`engine.rs:252-261` against the `Reporter` doc at `ports.rs:550-553`). I didn't reproduce it.
- [ ] `mise run check` and `mise run` were not run; this PR contains no code or skill changes.

## Notes for Reviewers

- ⚠️ **Hand-written work items:** the eight new items were written directly rather than through `accelerator work create`, so the numbering could start at 0312. They aren't synced to Linear yet; run `/sync-work-items` after merge.
- **Decisions still open for 0202:**
  - Whether `create-adr --supersedes` may rewrite the predecessor's body `**Status**:` line. 0202's AC currently allows only frontmatter changes.
  - Whether `superseded_by` takes the bare form (`"ADR-0070"`) or the typed form (`"adr:ADR-0070"`).
  - For each unshipped ADR-0038 affordance, whether to restore it or record it as dropped (0314).
- **Further candidates:** the research lists items not yet captured as work items. These are the missed ADR-0054 and ADR-0026 supersession flips, unrecognised TTY input becoming an edit, `ACCELERATOR_MIGRATE_FORCE` accepting any non-empty value, and the force-does-not-unskip test being gated behind `bash-parity`.
