---
type: "pr-description"
id: "148"
title: "[0295] Queue arXiv fetches fairly so a waiting call keeps its place across calls"
date: "2026-10-06T22:42:40+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0295"
parent: "work-item:0295"
relates_to: ["work-item:0283", "work-item:0300", "work-item:0301"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/148"
pr_number: 148
tags: ["research", "arxiv", "pacing", "fetch"]
revision: "950384733272de2b29cdd6b08c39346f69a3e532"
repository: "accelerator"
last_updated: "2026-10-08T18:14:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0295] Queue arXiv fetches fairly so a waiting call keeps its place across calls

## Summary

Concurrent arXiv `research fetch` calls now wait their turn in a
project-wide first-in, first-out queue. Previously they raced for
`arxiv.lock`, and under 0283's level-3 fan-out that race lost whole research
nodes to `lock_contention`. Once a call is admitted, it holds the lock for its
whole call, retries and backoffs included. A call that cannot be served
within its 100 s budget prints a `waiting` ticket and exits 0. Re-presenting
the ticket with `--ticket` resumes the call's place, so a researcher's search
is delayed rather than lost.

## Changes

- **`waiting` status.** The new output is
  `{"status":"waiting","source":"arxiv","ticket":"<n>-<nonce>","position":…}`,
  plus a stderr summary line naming the `--ticket` to re-present. `fetch`
  callers must handle this third status. The help text, `research.md` and the
  arXiv profile tell callers to treat any status other than `ok`,
  `unavailable` or `waiting` as unavailable. A waiting call can take up to
  about 1,000 s from its first call to settle.
- **`--ticket` and its rejections.** Re-presenting a ticket with different
  arguments exits 2 with `E_ARXIV_TICKET_MISMATCH`, which names the whole
  call the ticket was issued for and which arguments differ. Presenting a
  ticket another call holds exits 2 with `E_ARXIV_TICKET_LIVE`, which clears
  once that call returns. A malformed ticket exits 2 with
  `E_ARXIV_TICKET_MALFORMED`.
- **Whole-call admission.** `PacingGate::try_serve` hands out a
  `ServingTurn` that holds `arxiv.lock` from a call's first request to its
  last, so no other call's requests land between its retries.
  `fetch_arxiv` only admits an attempt that leaves a serving window: 3 s of
  spacing plus one whole request.
- **Queue domain model** (`research/src/sources/queue.rs`). It is a pure
  model, with `Ticket`, `Nonce`, `Binding`, `Queue::present` and a single
  liveness rule shared by `is_front_given` and expiry. A ticket expires 300 s
  after its last call ends, has a 900 s cap from issue, and is treated as
  abandoned after the cap plus one invocation. A ticket whose lock cannot be
  probed (`Presence::Unknown`) counts as live only until one invocation after
  its presentation. A resumed ticket the call cannot hold
  (`Joining::Unheld`) is served without the queue and still ends `waiting`.
- **File-backed queue** (`research-adapters/src/queue.rs`). The queue nests
  its own `arxiv-queue/` directory in the research scratch directory, with
  one `.json` record and one `.lock` file per ticket. Every write happens
  under `queue.lock`. A ticket is live while its call holds an exclusive
  `flock`, and probers take only a momentary shared lock. An unprobeable
  lock is reported once per invocation, with its error. Records are written
  with the new `store::replace_without_sync`, which is atomic and contained
  but skips the fsyncs, because the queue is scratch state that can be
  rebuilt.
- **Pacing.** Spacing now runs from the last request's finish. A send left
  unfinished by a killed call is spaced 3.1 s from the send, which allows a
  margin for its unseen arrival at arXiv.
- **`lock_contention` narrowed.** It now means only that a ticket passed its
  cap or a fresh call found the queue unusable. `arxiv-contention.log` lines
  name their kind (and ticket).
- **Prompts and docs.** The arXiv profile and `researcher` agent re-present
  `waiting` tickets until the call settles. The profile says the CLI settles
  a ticket within about 17 minutes and caps re-presentations at 20, ending
  as Unavailable with reason `waiting`. The `research.md` Pacing section
  describes the queue and its limits. The CHANGELOG folds the queue into the
  Added `accelerator research` entry, since no stable release shipped
  `fetch`.
- **Tests and lanes.** There are domain, adapter and CLI tests, including
  real-process queue ordering in `arxiv_queue.rs`. The opt-in
  `test:integration:arxiv-queue-stress` lane runs 30 concurrent fetches on
  the real clock; it takes minutes, so it is kept out of the default task.
  The build fails if `CALL_BUDGET` outgrows `queue::LONGEST_INVOCATION`.
- **Incidental.** The PR also patches the docs site's `sharp` and
  `source-map-js` advisories, captures work items 0300 (sync pulls of
  tracked work items fail silently) and 0301 (deferred review findings),
  refreshes the Linear sync baseline, and adds 0295 annotations to 0283's
  plan and validation.

## Context

- Work item: `meta/work/0295-per-profile-batch-cap-for-arxiv-researchers.md`.
  0295 was rescoped from a per-profile batch cap to this fair queue.
- Research: `meta/research/codebase/2026-10-06-0295-fair-arxiv-fetch-queue.md`
- Plan: `meta/plans/2026-10-06-0295-fair-arxiv-fetch-queue.md`, with a
  four-pass review in `meta/reviews/plans/`
- Validation:
  `meta/validations/2026-10-06-0295-fair-arxiv-fetch-queue-validation.md`
  (result: pass)
- PR review: `meta/reviews/prs/148-review-1.md` (verdict: comment). Its
  deferred findings are in
  `meta/work/0301-fair-arxiv-queue-follow-ups-from-the-pr-148-review.md`.
- Follows up 0283 (recursive finding deepening), whose step 17 run lost
  nodes to arXiv lock contention.

## Testing

- [x] Full local CI mirror `mise run` (formatters, every lint and type-check,
  `docs:check` and the whole test suite) exits 0 at the branch tip in 485 s
- [x] `public-api:check`, `cli:check`, the structure tests (57 in
  `test_research_structure.py`) and the `research`, `research-adapters` and
  `accelerator-research` tests (with `test-loopback`) pass after each
  review-response commit
- [x] Review-response tests were watched failing first, or checked by
  mutation: the lookup-ticket round trip, the unlocked pacing fallback, the
  unprobeable-ticket bound and its once-only report, the mismatch messages,
  and `Joining::Unheld`
- [x] During validation: `mise run test:integration:arxiv-queue-stress`
  (1 test, 117 s) and `arxiv_pacing` 20 of 20 under `--stress-count 20`
- [x] Attended researcher checks, run as headless sessions against the
  validated tree: re-presentation, `lock_contention` and a changed ticket
  each pass 3 of 3
- [x] Step 17 re-run (3 arXiv pairs at depth 3, 24 level-3 nodes): 0 of 24
  nodes lost to `lock_contention` in both runs. Run 2's 5 `waiting`
  outcomes were each re-presented once and settled `ok`. The median arXiv
  response was 0.15–0.33 s
- [ ] The stress lane, attended checks and step 17 re-run have not been
  repeated since the review responses
- [ ] A review of the CHANGELOG entry, the `research.md` Pacing section and
  the 0283 annotations in place, by someone other than the implementer

## Notes for Reviewers

- **Lock ordering.** The places to focus are `FileArxivQueue::join` /
  `step_aside` / `leave`, and the turn's lifetime in `fetch.rs`'s
  `serve_call`. The turn is consumed and dropped before `leave` or
  `step_aside`, and `is_front` takes no lock beyond its momentary probe.
- **Review responses.** Every inline thread from the first review has a
  reply and is resolved. The general findings have one top-level reply:
  16 fixed here, 11 deferred to 0301, and 4 left unchanged with reasons.
  The behavioural changes are the `Unknown` liveness bound and
  `Joining::Unheld`. The rest are wording, tests and small refactors.
- **Known edge cases** (left as they are):
  - A ticket `.lock` that is a directory is never removed. Its ticket
    stops blocking one invocation after its presentation, and its record
    expires as usual; the leftover lock is reported once per invocation.
  - A failed fresh join can leave an orphaned, unlocked `<ticket>.lock`
    until the next housekeeping pass.
  - After a killed send, a call with 33.0–33.1 s left can take
    `arxiv.lock` and then step aside without sending a request.
  - An `Unheld` call cannot record its last retryable reason, so a later
    over-cap result reports `lock_contention` rather than that reason.
- **Load ceiling.** Claude Code caps concurrent subagents at 20, so step 17
  loaded the queue with at most 20 researchers rather than 24.
