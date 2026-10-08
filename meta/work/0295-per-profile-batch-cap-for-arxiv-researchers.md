---
type: "work-item"
id: "0295"
title: "Per-Profile Batch Cap for arXiv Researchers"
date: "2026-09-28T09:03:54+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "in-progress"
kind: "story"
priority: "medium"
parent: "work-item:0121"
relates_to: ["work-item:0283"]
tags: ["research", "deep-research", "arxiv"]
last_updated: "2026-09-28T09:03:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0295: Per-Profile Batch Cap for arXiv Researchers

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an Accelerator user researching a subject with `arxiv` pairs at depth
above 1, I want `conduct` to cap how many arXiv researchers it spawns in
one batch, so that a deepened round does not lose nodes to arXiv lock
contention.

## Context

All arXiv fetches in a project share one lock, spaced 3 s apart, with a
100 s budget per call. Recursive deepening (0283) lets up to `concurrency`
arXiv nodes queue on that lock at once; the default is 24. 0283's attended
verification ran 3 `arxiv` pairs at depth 3 with default concurrency, so
the level-3 batch held exactly 24 arXiv nodes:

| Measure | Value |
|---|---|
| Contention per fetch | 4 of 110 requests (3.6%) |
| Nodes failed as `lock_contention` | 4 of 24 (17%) |
| `rate_limited` without `lock_contention` | 0 |
| Wall-clock, batch issued to last note | 452 s |

A failed node is recoverable, because a re-run researches only the missing
nodes. But one node in six failing at the default setting makes a full
arXiv round routinely need a second `conduct`.

## Requirements

- `outstanding` offers at most a per-profile limit of `arxiv` spawns in any
  one batch, within the overall `--limit`, and holds the rest for later
  batches.
- The limit defaults to a value at which the 0283 measurement
  (24 level-3 arXiv nodes) loses no node to `lock_contention`.
- `web` and `openalex` spawns stay bounded only by `concurrency`.
- Spawns held back by the profile limit keep the spawn window's order, and
  the run ledger never offers one twice.

## Acceptance Criteria

- [ ] Given 24 outstanding `arxiv` nodes and default settings, when
      `outstanding --limit 24 --start` plans, then it offers at most the
      arXiv limit and counts the rest in `remaining`.
- [ ] Given a batch that mixes `web` and `arxiv` spawns, when it is planned,
      then the free slots are filled with `web` spawns up to `--limit`.
- [ ] Re-running 0283's step 17 setup loses no node to `lock_contention`,
      recorded in a validation document.
- [ ] `research-topic`'s `lock_contention` reason row no longer tells the
      user to lower `--concurrency` as the first remedy.

## Open Questions

- Should the limit be configurable (`research.topic.arxiv_concurrency`), or
  a fixed domain constant derived from the lock's spacing and budget?
- Should the cap live in the planner, or should the pacing gate hold
  queued fetches past the 100 s budget instead?

## Dependencies

- Blocked by: nothing; builds on 0283's spawn window.
- Blocks: nothing.

## Assumptions

- The contention rate scales with concurrent arXiv nodes, not with the
  number of pairs or the depth.

## Technical Notes

- The spawn window is `cli/corpus/src/topic_research/spawn_window.rs`; the
  pacing gate is `cli/research-adapters/src/pacing.rs`.
- Measurement method and raw figures are in
  `meta/validations/2026-09-28-0283-recursive-finding-deepening-validation.md`.

## Drafting Notes

- Raised from 0283's Phase 7 step 17, which states that a material
  contention rate becomes a follow-up for a per-profile batch cap.

## References

- `meta/work/0283-recursive-finding-deepening.md`
- `meta/plans/2026-09-26-0283-recursive-finding-deepening.md`
