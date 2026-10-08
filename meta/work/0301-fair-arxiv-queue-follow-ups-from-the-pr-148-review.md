---
type: "work-item"
id: "0301"
title: "Fair arXiv Queue Follow-ups from the PR 148 Review"
date: "2026-10-08T18:02:08+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "low"
parent: "work-item:0121"
blocked_by: ["work-item:0295"]
relates_to: ["work-item:0295"]
tags: ["research", "arxiv"]
last_updated: "2026-10-08T18:02:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0301: Fair arXiv Queue Follow-ups from the PR 148 Review

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Tidy the fair arXiv fetch queue that 0295 shipped: the structural, modelling
and test-hygiene findings from the PR #148 review that were deferred because
each needs its own design or a wider change than that PR should carry. None
is a defect in the queue's behaviour today.

## Context

The PR #148 review (`meta/reviews/prs/148-review-1.md`) raised 37 minor
findings outside its inline threads. The ones local to the PR were fixed
there. These remain, grouped by the change each would need.

## Requirements

1. **Adapter structure.** Split `research-adapters/src/queue.rs` into
   submodules so the main file reads as the join/step-aside/leave protocol,
   with the record schema and lock primitives beside it. Extract the
   non-blocking `flock` retry loop that `bookkept`, `own` and
   `FilePacingGate::try_serve` each write out.
2. **Serving window.** Introduce a domain `ServingWindow` (spacing plus one
   request) and pass it to `ArxivQueue::join` in place of a raw `spacing`, so
   `admitted`, `until_settled`, `Bound::ServingWindow` and `own` stop
   re-deriving it.
3. **Module dependencies.** Break the `request` ↔ `queue` cycle: move ticket
   errors out of `RequestError` into a `TicketError` in `queue`.
4. **Outcome modelling.** Decide whether `Waiting` and `Rejected` belong in
   the shared `FetchOutcome` or in an arXiv-only outcome, and whether a
   ticket rejection (a usage error) belongs in an outcome at all. Have
   `Queue::present` return `Result<Admission, TicketRejection>` so `join` can
   housekeep once for every admitted presentation.
5. **Naming and types.** Rename the near-synonyms `served_when`,
   `serve_call` and `served`. Make `Binding` a newtype over `ArxivRequest`.
   Give `read_record` a validation that does not build a throwaway
   `Standing`. Model an absent `Contender` without a fabricated `issued_at`.
6. **Test hygiene.** Move the `own()`-patience failure tests onto
   `RecordingClock` so they no longer depend on wall-clock timing.
7. **Stress lane in CI.** Run the 30-process arXiv queue stress lane on a
   schedule (nightly or weekly).
8. **Clock steps.** Give queue stamps a wall-clock-step guard like
   `PacingState::trusted_at`, so a forward stamp cannot hold a ticket
   unexpired.
9. **Silent re-issue.** Tell the caller when a re-presented ticket had
   expired and a fresh one was issued in its place.
10. **Killed-send margin.** Measure connection-setup latency to arXiv and
    revisit the 100 ms margin a killed call's send is spaced by.

## Acceptance Criteria

- [ ] Each numbered requirement is either done or recorded here as declined,
      with its reason.
- [ ] `mise run` exits 0 after each change.

## Open Questions

- Does requirement 4 justify an `ArxivOutcome`, given only two sources exist?
- Is the stress lane's cost acceptable nightly, or only weekly?

## Dependencies

- Blocked by: 0295 merging (PR #148).

## Assumptions

- The queue's observable behaviour does not change except under
  requirements 8 and 9.

## Technical Notes

Requirements 1, 2 and 5 touch the same files and are cheapest done
together. Requirement 4 changes the domain's public API, so refresh the
`cargo-public-api` snapshot with it.

## Drafting Notes

- Filed as one task rather than ten because each item is small and the
  areas overlap; split it if the outcome-modelling decision grows.

## References

- Source: `meta/reviews/prs/148-review-1.md`
- Related: 0295
- Plan: `meta/plans/2026-10-06-0295-fair-arxiv-fetch-queue.md`
