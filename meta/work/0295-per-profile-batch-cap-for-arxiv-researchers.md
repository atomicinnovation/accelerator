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
last_updated: "2026-10-05T10:21:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0295: Per-Profile Batch Cap for arXiv Researchers

**Kind**: Story
**Status**: In Progress
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

The lock is held for the whole response plus the spacing, so a turn on it
averaged ~4.1 s (452 s / 110 requests), not 3 s. A 100 s budget therefore
covers ~24 turns: the 24-node batch sat at the limit, and the lock's
unfair polling pushed 4 nodes past it.

## Requirements

- A source profile may declare a per-batch spawn cap; a profile that
  declares none is bounded only by `concurrency`.
- The `arxiv` profile declares a cap of 12.
- Each profile's cap lives in a Rust profile table beside the pacing
  constants it is derived from, not in the profile skill's frontmatter.
- `outstanding` offers at most a profile's cap of that profile's spawns in
  any one batch, within the overall `--limit`, and fills the freed slots
  with spawns from uncapped profiles.
- Spawns held back by a profile cap keep the spawn window's order, are
  counted in `remaining`, and the run ledger never offers one twice.
- The cap is a fixed property of the profile, not user configuration.

## Acceptance Criteria

- [ ] Given 24 outstanding `arxiv` nodes and default settings, when
      `outstanding --limit 24 --start` plans, then it offers 12 and counts
      12 in `remaining`.
- [ ] Given a batch that mixes `web` and `arxiv` spawns, when it is
      planned, then at most 12 `arxiv` spawns are offered and the free
      slots are filled with `web` spawns up to `--limit`.
- [ ] Given outstanding `web` and `openalex` nodes only, when the batch is
      planned, then the offer is identical to today's.
- [ ] Given a batch whose held-back `arxiv` spawns are offered in a later
      batch, then they appear in spawn-window order and none is offered
      twice.
- [ ] Re-running 0283's step 17 setup (3 `arxiv` pairs, depth 3, default
      concurrency) loses 0 nodes to `lock_contention`, recorded in a
      validation document.
- [ ] `research-topic`'s `lock_contention` reason row no longer tells the
      user to lower `--concurrency` as the first remedy.

## Open Questions

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- The contention rate scales with concurrent arXiv nodes, not with the
  number of pairs or the depth.
- Each arXiv researcher has at most one fetch in flight, so the worst-case
  wait on the lock is about (cap − 1) × the per-turn time.

## Technical Notes

- The planner is `window()` in `cli/research/src/conduct/window.rs`, which
  truncates the fresh spawns with `fresh.len().min(limit)`; the cap applies
  per profile before that truncation.
- The pacing gate is `FilePacingGate` in
  `cli/research-adapters/src/pacing.rs` (`SPACING` 3 s, non-FIFO `flock`
  polled every 100 ms). The 100 s `CALL_BUDGET` in
  `cli/research-cli/src/main.rs` sits below Claude Code's default Bash
  timeout, so the gate cannot wait longer.
- Cap of 12: at ~4.1 s per turn, the worst-case wait is ~45 s, leaving ~2×
  headroom in the 100 s budget for unfair lock acquisition and slow
  responses.
- Measurement method and raw figures are in
  `meta/validations/2026-09-28-0283-recursive-finding-deepening-validation.md`.

## Drafting Notes

- Raised from 0283's Phase 7 step 17, which states that a material
  contention rate becomes a follow-up for a per-profile batch cap.
- Chose a planner cap over changing the pacing gate. Lengthening the wait
  is capped by the Bash timeout, and a fair queue alone still leaves the
  24th node waiting ~100 s. A FIFO ticket lock in `FilePacingGate` is out
  of scope here, but it is a useful follow-up that would make the cap's
  headroom more predictable.
- Chose 12 over ~24 (the raw turns-per-budget figure) to leave headroom;
  this halves arXiv throughput per batch compared with the default
  `concurrency`, and accepts that trade for no re-run.
- Made the cap fixed rather than configurable, because it is derived from
  the lock's spacing and the call budget, and a user-set value could
  bring the contention back.
- Placed the cap in a Rust profile table rather than the profile skill's
  frontmatter, so it cannot drift from `SPACING` and `CALL_BUDGET`, which
  justify its value.

## References

- `meta/work/0283-recursive-finding-deepening.md`
- `meta/plans/2026-09-26-0283-recursive-finding-deepening.md`
- `meta/validations/2026-09-28-0283-recursive-finding-deepening-validation.md`
