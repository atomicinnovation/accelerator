---
type: "pr-description"
id: "36"
title: "Close out 0167, 0168 and 0182 now their code has landed"
date: "2026-08-02T18:57:09+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0136", "work-item:0167", "work-item:0168", "work-item:0182", "work-item:0169", "work-item:0186", "work-item:0187"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/36"
pr_number: 36
tags: ["work-items", "bookkeeping", "status"]
revision: "aec98af456425821e21bd94c0049e94d4bb4fb46"
repository: "accelerator"
last_updated: "2026-08-02T18:57:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Close out 0167, 0168 and 0182 now their code has landed

## Summary

Three epic-0136 work items shipped their implementation to `main` but were never transitioned out of `ready`. That is not cosmetic: 0186, 0187 and 0169 all declare these items as blockers, so the dependency graph showed three startable tasks as blocked by work that was already complete. This marks all three `done`.

## Changes

Three one-line frontmatter edits, `status: ready` → `status: done`:

- **0167** — Built-in `config` Command and Invocation-Contract Migration
- **0168** — Fold the Visualiser into the `cli/` Workspace
- **0182** — `bin/accelerator` requires `CLAUDE_PLUGIN_ROOT` in the environment

## Context

Each item's code is verifiable on `main` today:

- **0167** — both of its plans are fully executed (`2026-07-22-0167-config-command-refactoring.md` has 62/62 checked). Its remaining unchecked items are manual or release-gated: `--help` snapshot review, live-session skill invocations, and cold-start latency measurement. 0169's Dependencies section explicitly asks for this closeout *before* that story starts, rather than at its acceptance, "or the edge stays stale throughout".
- **0168** — `cli/visualiser/{frontend,server}` exists in the workspace and the plan is already marked `status: done`. Its nine unchecked items are E2E, visual and release-gated checks.
- **0182** — `bin/accelerator` exports `ACCELERATOR_PLUGIN_ROOT`; 104 of 109 plan items are checked and the five outstanding are manual pre-release checks against a clean install of a signed artifact.

## Testing

- [x] `./scripts/validate-corpus-frontmatter.sh meta` — exit 0; `done` is a valid status in the work-item vocabulary and the three files still validate.
- [x] Blocker edges re-checked: with these three `done`, 0186 (blocked by 0182), 0187 (blocked by 0168) and 0188 (blocked by 0179, already `done`) have no outstanding blockers, and 0169's 0167 prerequisite is discharged.
- [ ] CI on this PR. Frontmatter-only; no code, build-system or shell files are touched.

## Notes for Reviewers

**All three are being closed with manual checks still unticked, deliberately.** Between them, 0167, 0168 and 0182 have 37 unchecked plan items, and every one is a manual UI check, a live-session check, or a check gated on cutting a signed release. Holding three `done`-in-substance items open on a release cut would have kept 0186, 0187 and 0169 looking blocked indefinitely. 0186's work item already anticipates this and states its 0182 edge is "discharged when 0182's `bin/accelerator` and entrypoint-suite changes are on `main`, not when 0182 reaches `complete`". If you would rather any of the three stayed open until its release gate clears, this is the PR to say so on.

**0182 changes status twice across this stack** — PR #34 moves it `in-progress` → `ready`, this PR moves it `ready` → `done`. Two steps of one closeout, not churn.

**`last_updated` was not bumped on any of the three.** The status edits were made by hand rather than through `/accelerator:update-work-item`, so the three files now read as last touched on their previous dates. Harmless, but worth a deliberate decision — happy to refresh them if you would rather the corpus stay strictly consistent.

**Stack position.** Last of three: #34 (the 0169 split) → #35 (jj pin bump) → **#36**. Independent of both parents in content; merge order is free once the bases retarget.
