---
type: "pr-description"
id: "224"
title: "[0372] Scope the sync pipeline epic and the PR #188 follow-ups"
date: "2026-11-16T11:30:05+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-963"
parent: "work-item:PP-963"
relates_to: ["work-item:PP-927", "work-item:PP-167"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/224"
pr_number: 224
tags: ["sync", "architecture", "work-items"]
revision: "ac6cb3f26977d7e3cf51769976158254f7bdd430"
repository: "accelerator"
last_updated: "2026-11-16T11:30:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0372] Scope the sync pipeline epic and the PR #188 follow-ups

## Summary

Adds epic 0372 and its seventeen child work items. The epic scopes the move
to a sync run modelled as typed stages over one entry per subject, and it
owns every fix the review of PR #188 asked for, so that PR can merge without
them. This PR adds work items only: no code, skill or CHANGELOG changes.

## Changes

Eighteen new files under `meta/work/`, all in `draft` status.

- **0372 (epic)**: the goals, the order of the children, and a list mapping
  each PR #188 review finding marked to be fixed to the item that owns it.
- **Fixes worth doing whatever happens to the architecture**:
  - 0373: `/sync-work-items` can report complete for an item that needs a
    plugin upgrade; also decides and tests what a failed create over an
    unknown-recipe entry reports (`R2-03`, `R2-05`, `R2-06`).
  - 0374: advance a watermark only when every row for the item is settled,
    and move the first-match test helpers onto `only_row` (`R2-09`).
  - 0386: correct 0352's CHANGELOG entry and plan (`R2-01`).
  - 0387: decide whether an untouched unsynced item's watermark advances.
  - 0388: the skill summarises an untouched unsynced item as declined.
  - 0389: run 0352's outstanding manual checks against the sandbox.
- **The architecture**:
  - 0375: record it as an ADR.
  - 0376: the planner plans local creates (`R2-07`).
  - 0377: pin the stdout row contract through the binary.
  - 0378: check the skill's Membership table against the engine's rows.
  - 0379: one apply loop for every planned entry.
  - 0380: imports join the plan at discovery.
  - 0381: unknown recipe becomes a fact on the entry.
  - 0382: unclassifiable items and refused orders become entries.
  - 0383: each kind of planned entry carries its own payload.
  - 0384: the engine's stages are distinct types.
  - 0385: identity pass results carry into the sync entry.

## Context

- Epic: `meta/work/PP-963-sync-run-as-a-typed-pipeline-over-one-entry-per-subject.md`,
  under 0146.
- Origin: the review of PR #188 (0352), whose fix keeps "an item is in the
  plan or in the creates, never both" as a rule held by two functions and
  their tests. Each defect and finding on 0352 sat at a seam between the
  parallel lists `prepare_run` builds.
- 0351 is an open neighbour. 0377 and 0379 reference it.

## Testing

- [x] `accelerator corpus frontmatter validate` passes for all eighteen
      files.
- [x] No ID from 0372 to 0389 exists on `main` or in any open PR.
- [ ] `mise run check` was not run locally; the change is Markdown under
      `meta/work/` only, and CI covers it.

## Notes for Reviewers

- **Nothing here is synced to Linear.** The items have no `external_id`;
  `/sync-work-items` will create them.
- **Decisions waiting on a person**: 0373 (what a failed create over an
  unknown-recipe entry reports), 0376 (what a relink-ordered item gets under
  `--pull-only`), and 0387 (whether the watermark should advance at all).
- **0382 assumes the ADR folds a refused order into its item's row.** If the
  ADR decides otherwise, that item's stdout and skill changes drop out.
- **0385 is the least certain.** It rests on reading the identity pass, not
  running it, and is expected to split once the ADR settles how a subject
  is keyed across a retirement.
- **Several items are blocked by PR #188 merging.** That is stated in their
  bodies, not as a frontmatter link, since it is the PR that has to land.
- **Code is cited by function name, not line number**, as read at PR #188's
  head `581fee36`.
