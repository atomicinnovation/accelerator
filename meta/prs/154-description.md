---
type: "pr-description"
id: "154"
title: "[0219] Split warm-dispatch latency tracking into a measurement epic"
date: "2026-10-10T19:05:24+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0219"
parent: "work-item:0219"
relates_to: ["work-item:0136", "work-item:0205", "work-item:0217", "work-item:0225", "work-item:0299"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/154"
pr_number: 154
tags: []
revision: "da08ccd7244ae0ab40126e2730a787a6ce026341"
repository: "accelerator"
last_updated: "2026-10-10T19:05:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0219] Split warm-dispatch latency tracking into a measurement epic

## Summary

0219 started as one task that owned recurring warm-dispatch latency
measurement. Its first review returned **REVISE**, partly because the task
bundled four separately deliverable workstreams plus a tuning step that has to
wait for merges. This PR revises 0219 against that review, promotes it to a
standalone epic, and decomposes it into eight child stories, 0322–0329. It
also moves the earlier measurement items, 0205 and 0217, under the epic, out of
0136. Only planning documents under `meta/` change; no code changes.

## Changes

- **0219 becomes an epic.**
  - `kind` changes from `task` to `epic`. `parent: work-item:0136` is replaced
    by a `relates_to` link to 0136, following the 0312 precedent.
  - Context, Vocabulary, the platform table and the per-platform versus
    per-platform-backend granularity stay on the epic as its shared glossary.
  - The detailed requirements become five goals plus a child list.
  - The acceptance criteria become epic-level: every child reaches a terminal
    status, every measured platform records each prerelease without human
    action, every platform is calibrated or has a recorded reason, and 0189's
    criterion is amended.
- **Eight child stories, 0322–0329.** Each carries its slice of 0219's revised
  requirements and acceptance criteria, mostly verbatim, plus Technical Notes
  with `path:line` references from codebase analysis. The `blocks:` links run
  0322 → 0323 → 0324 → 0325 → 0326 → 0327 → 0328 → 0329, with a few extra
  edges that skip ahead.

  | Item | Story | Gated on |
  | --- | --- | --- |
  | 0322 | Record every measurement session's outcome | — |
  | 0323 | Pair a prerelease against its predecessor in the harness | 0322 |
  | 0324 | Measure warm-up sessions on uncalibrated platforms | 0322, 0323 |
  | 0325 | Measure each prerelease in a CI lane | 0323, 0324, measurement App |
  | 0326 | Raise regression, ceiling and invalidation alerts | 0325, App `issues: write` |
  | 0327 | Detect cross-merge latency drift | 0326 |
  | 0328 | Propose calibration from warm-up sessions | 0326, 0327 |
  | 0329 | Tune the GitHub-hosted platform entries | 0328, 20 merges per platform |

- **0205 and 0217 move from 0136 to 0219.** Their `parent` fields and the
  parent lines in their bodies now name 0219. 0217 no longer `blocks` 0219,
  since it is now its child.
- **0136 drops 0205, 0217 and 0219.** They are removed from the Decomposition
  list and the Children line, and a Drafting Note records why. 0189 stays
  under 0136, because its subject is the launcher's at-most-once guarantee
  rather than measurement.
- **Cross-links from the review revision.** 0225 now relates to 0219, whose
  open-or-comment issue step 0225 can reuse. 0299 now relates to 0219: it
  takes its before-and-after figures on the same harness revision as 0219.
- **Review artefact.** `meta/reviews/work/0219-own-the-recurring-absolute-budget-check-review-1.md`
  records the three-pass, five-lens review this revision answers.

## Context

- Epic: `meta/work/0219-own-the-recurring-absolute-budget-check.md`
- Former parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Precedent for a standalone epic split out of 0136:
  `meta/work/0312-migration-engine-improvements.md`
- Harness plan the original task derived from:
  `meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md`

Codebase analysis during decomposition found four things the original task
did not account for, and they shaped the split:

1. **Most invalidated sessions write no record.** Every
   `PreconditionFailureError` or `Exit` raised inside the session propagates
   past the record write (`tasks/measure.py:1829-1842`), which is why 0322
   exists as its own story.
2. **Uncalibrated platforms fail before the raise the task pointed to.**
   Without an entry, the fast farm has no `sha256sum`, so `assert_backends`
   raises at `tasks/measure.py:1402`, ahead of the documented raise at
   `tasks/measure.py:1750`. 0324 owns this.
3. **The harness assumes jj.** `build_rig` recovers the baseline with the
   default `engine="jj"`, so it cannot run on CI's git-only checkout. 0323
   owns this.
4. **Every session builds the tree with `cargo test --release`**, through
   `close_the_budget`. 0323 drops that build from the paired mode.

## Testing

- [x] `accelerator corpus frontmatter validate` passes on all 15 changed
      files.
- [x] `accelerator work list --hierarchy` renders 0219 with the ten expected
      children, 0205, 0217 and 0322–0329, and with no parent of its own.
- [x] No other branch on `origin` (127 checked) contains work items numbered
      0322–0329. Numbering starts at 0322 deliberately, so that 0320 and 0321
      stay free for other branches.
- [ ] `mise run check` was not run. The diff touches only `meta/`, which none
      of its component checks read.

## Notes for Reviewers

- **Scope decisions in the children worth checking:**
  - **0322 is new.** No requirement in the original task maps to it; codebase
    analysis surfaced it.
  - **0323 drops several figures from the paired mode.** It doesn't recover
    `B`, compute C5/C6 or decompose budget terms, because no lane signal reads
    them. The single-version mode keeps all of these.
  - **0324 owns the runner-identity tuple on `Calibration`.** The tuple
    decides which validity rule a session runs under, so it can't wait for
    tuning in 0328.
- **Open question.** The choice of change-point detector moved from 0219 to
  0327, keeping its default of an in-house windowed test.
- **Not synced to Linear yet.** 0219 (PP-748), 0205 (PP-735) and 0217 (PP-746)
  have local changes that haven't been pushed, and 0322–0329 have no Linear
  issues yet. Run `/accelerator:sync-work-items` once this PR merges.
- **External blocker.** The measurement GitHub App that 0325 and 0326 need
  does not exist yet.

https://claude.ai/code/session_019iM7ztGkEwHECo6dVjnDGU
