---
type: "pr-description"
id: "54"
title: "Renumber the VCS migration follow-ups and close the story"
date: "2026-08-07T10:10:44+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0169", "work-item:0199", "work-item:0200"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/54"
pr_number: 54
tags: ["meta", "work-items", "vcs"]
revision: "1b24e4a10b1b87ca9bf74faff9fff70dca5d2cd0"
repository: "accelerator"
last_updated: "2026-08-07T10:10:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Renumber the VCS migration follow-ups and close the story

## Summary

Corpus housekeeping in the wake of the VCS subdomain migration. The two follow-up work items opened while landing `0169` were allocated the IDs `0192` and `0193`, which PR #49 had already handed to the documentation epic and its docs-site story — so they are renumbered to `0199` and `0200` and every inbound reference is repointed. With PR #51 merged on 2026-08-06, `0169` itself is marked done.

## Changes

**Resolve the ID collision (`0192` → `0199`, `0193` → `0200`)**

- Renamed `meta/work/0192-retire-vcs-common-sh-residue-and-launcher-link-refresh.md` → `0199-…` and `meta/work/0193-decide-vcs-guard-log-diff-blocklist-membership.md` → `0200-…`, updating each file's `id:` and body H1 to match. `0199` and `0200` were the next free IDs — `0198` is the highest previously allocated work item.
- Repointed the inbound references in `meta/work/0169-vcs-subdomain-and-hooks-migration.md`: the `relates_to:` list and the two resolution links in the Dependencies section that name the follow-ups by filename.
- Repointed the narrative references in the derived artifacts — the Phase 10 hand-off criterion in `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` and the Phase 10 verdict in `meta/validations/2026-08-05-0169-vcs-subdomain-and-hooks-migration-validation.md`.
- Updated `meta/prs/51-description.md` — both its `relates_to:` list and the hand-offs bullet — so the merged PR's record points at the surviving IDs. This edits an already-merged PR's on-disk description; it is a record-keeping correction, not a change to what #51 shipped.

**Close `0169`**

- Flipped `status: ready` → `status: done` in the frontmatter and `**Status**: Ready` → `**Status**: Done` in the body of `meta/work/0169-vcs-subdomain-and-hooks-migration.md`.

## Context

- Work item `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — the story being closed; shipped by PR #51 (merged 2026-08-06, `3612820`).
- Follow-ups `meta/work/0199-retire-vcs-common-sh-residue-and-launcher-link-refresh.md` (the `scripts/vcs-common.sh` residue plus `hooks/launcher-link-refresh.sh`) and `meta/work/0200-decide-vcs-guard-log-diff-blocklist-membership.md` (whether `log`/`diff` belong in the guard's blocked set), both still `draft`. The third hand-off, `0198`, was unaffected.
- PR #49 is the direct precedent — it moved the documentation items off `0178`/`0179` and onto `0192`/`0193`, which is what made these IDs collide when `0169`'s follow-ups were created a month later.

## Testing

- [x] `grep -rn -E '0192|0193' meta/` returns only the documentation epic `0192-documentation.md`, the story `0193-make-the-docs-amazing.md`, their derived plan and research documents, and PR #49's description — no stale references to the retired follow-up IDs survive.
- [x] `grep -rn -E '0199|0200' meta/` shows every inbound reference (from `0169`, its plan, its validation, and `51-description.md`) resolving to the renamed files.
- [x] Both renamed files' `id:` and body H1 agree with their new filenames; their `parent:`, `relates_to:`, and `derived_from:` slots already pointed at `0136`/`0169`/the `0169` plan and needed no change.
- [x] PR #51 confirmed merged via `gh pr view 51`, so `status: done` on `0169` reflects reality rather than anticipating it.
- [ ] `mise run check` not run — the change is confined to `meta/*.md`, which sits outside all four component check surfaces (frontend, server, cli, build-system, scripts). CI will confirm.

## Notes for Reviewers

- **`last_updated` on `0169` was not bumped** — it still reads `2026-08-06T02:00:00+00:00`. PR #49 set the precedent of leaving `last_updated` alone for a pure renumber, but this commit also flips `0169`'s status, which is a content change. If you read that field as "last touched", it wants bumping to today.
- **This is the second collision in the same numbering range.** IDs are allocated by scanning `meta/work/` for the highest number, so any two branches that create work items concurrently will collide — #49's renumber and `0169`'s follow-up creation did exactly that, a month apart. Nothing here fixes the allocation mechanism; if that is worth owning, it needs its own work item.
- Both follow-ups remain `draft` and unscheduled. Renumbering deliberately changes identity only — no scope, criteria, or timestamps were touched.
