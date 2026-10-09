---
type: "pr-description"
id: "152"
title: "[0121] Capture the open topic-research issues as work items 0303–0311"
date: "2026-10-09T20:33:01+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0121"
parent: "work-item:0121"
relates_to: ["work-item:0301", "work-item:0303", "work-item:0304", "work-item:0305", "work-item:0306", "work-item:0307", "work-item:0308", "work-item:0309", "work-item:0310", "work-item:0311"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/152"
pr_number: 152
tags: ["research", "deep-research", "work-items"]
revision: "256259271208acdbf5f6aba06126a771abd975da"
repository: "accelerator"
last_updated: "2026-10-09T20:33:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0121] Capture the open topic-research issues as work items 0303–0311

## Summary

Turns the eight open `research-topic` GitHub issues (#140–#147) into nine
draft work items under epic 0121, so each can be refined, sized and planned
like the rest of the epic. Docs only: no code, skill or agent changes.

Stacked on #148 (0295): base is `0295-fair-arxiv-fetch-queue`, so the diff
shows only this PR's commit. Retarget to `main` once #148 merges.

## Changes

| Item | Kind | Priority | Source | Problem |
|---|---|---|---|---|
| 0303 | bug | high | #140 | Default `research.topic.concurrency` of 24 exceeds Claude Code's 20-subagent cap, so 4 spawns per full batch fail permanently |
| 0304 | story | high | #141 | WebSearch quota exhaustion silently degrades web nodes to WebFetch of known URLs |
| 0305 | bug | medium | #142 | Web researchers tier `doi.org` URLs by the resolver's domain, dropping peer-reviewed papers to tier-3 |
| 0306 | story | medium | #143 | Composers resolve a source cited at conflicting tiers three different ways; cite it once at its lowest tier |
| 0307 | story | medium | #144 | `conduct` copies each spawn's full context into every Task prompt; pass references instead |
| 0308 | story | low | #145 | Researchers reach for Bash the research guard blocks; give them `Edit` and `Grep` and confine writes to the batch |
| 0309 | story | medium | #146 | Follow-up questions drift outside what the pair's source profile can answer |
| 0310 | story | medium | #147 | Agent usage limits fail every later spawn, yet `conduct` keeps issuing batches |
| 0311 | story | low | #143 | Web-profile tiers drift for the same blog or repository; add canonical domain classes |

- **0121's `children`** gains 0303–0311, plus 0301 (the PR #148 review
  follow-ups), which declared 0121 as its parent but was missing from the
  epic's list.
- **#143 splits in two**: 0306 owns the conflict rule, 0311 the upstream
  drift that causes most conflicts.
- **0305 reframes #142**: the issue blamed `fetch openalex` tiering `search`
  and `lookup` differently, which does not reproduce; the item targets the
  web profile's DOI tiering and adds a parity regression test.

## Context

- Epic: `meta/work/0121-topic-research-skillset.md`
- Issues: #140, #141, #142, #143, #144, #145, #146, #147 — each item cites
  its source under References.

## Testing

- [x] All 9 new items parse: `accelerator work show --field parent` succeeds
      on each
- [x] `accelerator work list --hierarchy` places 0301 and 0303–0311 under 0121
- [ ] Items are unsynced to Linear; run `work sync` after merge to create
      their issues

## Notes for Reviewers

- 0310 `blocks` 0304: 0304's quota-exhaustion stop rule joins the
  halting-failure class 0310 defines. The order is a choice; if 0304 is
  built first, invert the link.
- All items are `draft`; acceptance criteria and sizing are expected to
  sharpen in `refine-work-item` before planning.
- Close #140–#147 against these items once merged, or leave them open as
  user-facing trackers — not decided here.
