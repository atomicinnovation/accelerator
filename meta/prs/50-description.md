---
type: "pr-description"
id: "50"
title: "Split 0170/0194 tracker crate and remote sync engine work items"
date: "2026-08-06T00:31:44+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
parent: "work-item:0136"
relates_to: ["work-item:0170", "work-item:0194", "work-item:0171"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/50"
pr_number: 50
tags: []
revision: "08de11b010aee84301b90f03b63ad26e5a6b653e"
repository: "accelerator"
last_updated: "2026-08-06T00:31:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Split 0170/0194 tracker crate and remote sync engine work items

## Summary

Splits work item 0170 (Work-Item Subdomain and Sync Engine) into a lean lifecycle-CRUD story (0170) and a new tracker-crate-and-sync-engine story (0194), then reviews and refines 0194 through all five work item lenses, then moves `--push` support out of 0170 and into 0194 so the two stories carry no dependency on each other beyond one narrow slice. Net effect: 0170 and 0194 can now be implemented fully in parallel, with only 0194's `--push`-wiring acceptance criteria waiting on 0170's `create`/`update` commands existing.

## Changes

- Splits 0170 into 0170 (lifecycle CRUD: create/show/update/resolve/diff) and a new 0194 (the `tracker` crate's `RemoteTracker` port, the sync state machine, and the `accelerator work sync` command), following work item review 1 of 0170, which found the combined story epic-scale and the two halves independently deliverable.
- Updates the 0136 epic's decomposition list and 0171's dependency/relates_to references to point at 0194 for the `RemoteTracker` port instead of 0170.
- Reviews 0194 through all five work item lenses (clarity, completeness, dependency, scope, testability); the only major finding (AC1's resumability contract had no concrete verification procedure) and several minor/clarity gaps are addressed directly in the item, then verified resolved on re-review — verdict APPROVE.
- Decouples 0170 from 0194 entirely: moves `--push` support for `create`/`update` (and the `work-item-create-remote.sh`, `work-item-update-remote.sh`, and `work-item-push-decide.sh` scripts that implement it) out of 0170's scope and into 0194's. 0170 now ships as pure local-only CRUD with zero remaining blockers; 0194 gains two new acceptance criteria for the `--push` wiring and becomes `blocked_by: 0170` for that one slice only — the `tracker` crate and `sync` command remain independently deliverable.
- Adds two new review artifacts (`meta/reviews/work/0170-...-review-1.md`, `meta/reviews/work/0194-...-review-1.md`) documenting both review passes, findings, and resolutions.

## Context

Parent epic: 0136 (Migrate Shell Scripts into a Rust CLI). Directly restructures work items 0170, 0194, and touches 0171 and 0136 for cross-references. No source code changes — this PR is entirely `meta/work/` and `meta/reviews/work/` planning documents.

## Testing

- [x] All six changed files are markdown planning documents (work items and work item reviews) under `meta/`; there is no code change, so the standard `mise run check`/`mise run test:*` suites don't apply.
- [x] Cross-checked frontmatter typed-linkage fields (`blocks`/`blocked_by`/`relates_to`) for consistency across all four touched work items (0136, 0170, 0171, 0194) — the blocking relationship between 0170 and 0194 correctly reverses for the `--push`-wiring slice while the rest of each item's dependency graph stays accurate.
- [x] Grepped for stale cross-references to the removed `0170 --push`/`RemoteTracker` coupling across all four files after the decoupling edit; none found.
- [ ] No manual/UI verification applicable (documentation-only change).

## Notes for Reviewers

- The three commits read as a sequence: split → review-and-approve 0194 → decouple 0170 from 0194. Each is independently reviewable.
- The most consequential judgment call is in the third commit: moving `--push` support (and its three source bash scripts) from 0170 into 0194 reverses part of the dependency direction established in the first commit. Worth double-checking that 0194's new Acceptance Criteria for `create --push`/`update --push` (mirroring the old 0170 ones) and the updated Dependencies sections on both items still read coherently together.
- 0170's Acceptance Criteria for `create`/`update` are now considerably simpler (local-only, no remote-call/retry semantics) — the removed complexity moved to 0194's two new ACs, not lost.
