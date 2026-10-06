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
relates_to: ["work-item:0283", "work-item:0300"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/148"
pr_number: 148
tags: ["research", "arxiv", "pacing", "fetch"]
revision: "fcb2af07f1cf29b0a50cd1d19ce0bc48db3a6e0b"
repository: "accelerator"
last_updated: "2026-10-06T22:42:40+00:00"
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

- **`waiting` status (breaking for `fetch` callers).** The new output is
  `{"status":"waiting","source":"arxiv","ticket":"<n>-<nonce>","position":…}`,
  plus a stderr summary line naming the `--ticket` to re-present. Callers
  must handle this third status. The CLI help now tells callers to treat
  any unknown status as unavailable.
- **`--ticket` and its rejections.** Re-presenting a ticket with different
  arguments exits 2 with `E_ARXIV_TICKET_MISMATCH`, which names the arguments
  the ticket was issued for. Presenting a ticket another call holds exits 2
  with `E_ARXIV_TICKET_LIVE`, and a malformed ticket exits 2 with
  `E_ARXIV_TICKET_MALFORMED`.
- **Whole-call admission.** `PacingGate::try_serve` hands out a
  `ServingTurn` that holds `arxiv.lock` from a call's first request to its
  last, so no other call's requests land between its retries.
  `fetch_arxiv` only admits an attempt that leaves a serving window: 3 s of
  spacing plus one whole request.
- **Queue domain model** (`research/src/sources/queue.rs`). It is a pure
  model, with `Ticket`, `Nonce`, `Binding`, `Queue::present` and a single
  front rule (`is_front_given`). A ticket expires 300 s after its last call
  ends, has a 900 s cap from issue, and is treated as abandoned after the cap
  plus one invocation.
- **File-backed queue** (`research-adapters/src/queue.rs`). The queue lives
  in the research scratch `arxiv-queue/` directory, with one `.json` record
  and one `.lock` file per ticket. Every write happens under `queue.lock`.
  A ticket is live while its call holds an exclusive `flock`, and probers
  take only a momentary shared lock. Records are written with the new
  `store::replace_without_sync`, which is atomic and contained but skips the
  fsyncs, because the queue is scratch state that can be rebuilt.
- **Pacing.** Spacing now runs from the last request's finish. A send left
  unfinished by a killed call is spaced 3.1 s from the send, which allows a
  margin for its unseen arrival at arXiv.
- **`lock_contention` narrowed.** It now means only that a ticket passed its
  cap or the queue could not be used. `arxiv-contention.log` lines name
  their kind (and ticket), so their counts are not comparable with earlier
  releases.
- **Prompts and docs.** The arXiv profile and `researcher` agent re-present
  `waiting` tickets until the call settles. The `research.md` Pacing section
  and the CHANGELOG describe the queue.
- **Tests and lanes.** There are domain, adapter and CLI tests, including
  real-process queue ordering in `arxiv_queue.rs`. The opt-in
  `test:integration:arxiv-queue-stress` lane runs 30 concurrent fetches on
  the real clock; it takes minutes, so it is kept out of the default task.
- **Incidental.** The PR also patches the docs site's `sharp` and
  `source-map-js` advisories, captures work item 0300 (sync pulls of tracked
  work items fail silently), refreshes the Linear sync baseline, and adds
  0295 annotations to 0283's plan and validation.

## Context

- Work item: `meta/work/0295-per-profile-batch-cap-for-arxiv-researchers.md`.
  0295 was rescoped from a per-profile batch cap to this fair queue.
- Research: `meta/research/codebase/2026-10-06-0295-fair-arxiv-fetch-queue.md`
- Plan: `meta/plans/2026-10-06-0295-fair-arxiv-fetch-queue.md`, with a
  four-pass review in `meta/reviews/plans/`
- Validation:
  `meta/validations/2026-10-06-0295-fair-arxiv-fetch-queue-validation.md`
  (result: pass)
- Follows up 0283 (recursive finding deepening), whose step 17 run lost
  nodes to arXiv lock contention.

## Testing

- [x] `mise run check` (the CI read-only set, including
  `public-api:check`) exits 0 at the branch tip
- [x] Full local CI mirror `mise run`: 4,598 CLI tests pass, 2 skipped. This
  was run during validation, on the tree before the final comment-only
  commit. For that commit, `cli:check` and the `research`,
  `research-adapters` and `accelerator-research` tests (606) pass
- [x] `mise run test:integration:arxiv-queue-stress`: 1 test, 117 s
- [x] `arxiv_pacing` passes 20 of 20 under `--stress-count 20`
- [x] Attended researcher checks, run as headless sessions against this
  tree: re-presentation, `lock_contention` and a changed ticket each pass 3
  of 3
- [x] Step 17 re-run (3 arXiv pairs at depth 3, 24 level-3 nodes): 0 of 24
  nodes lost to `lock_contention` in both runs. Run 2's 5 `waiting`
  outcomes were each re-presented once and settled `ok`. The median arXiv
  response was 0.15–0.33 s
- [ ] A review of the CHANGELOG entry, the `research.md` Pacing section and
  the 0283 annotations in place, by someone other than the implementer

## Notes for Reviewers

- **Release placement.** The CHANGELOG entry sits under `### Changed`, but
  `fetch` callers must now handle `waiting`. Decide whether it belongs under
  `### Breaking`.
- **Lock ordering.** The places to focus are `FileArxivQueue::join` /
  `step_aside` / `leave`, and the turn's lifetime in `fetch.rs`'s
  `serve_call`. The turn is consumed and dropped before `leave` or
  `step_aside`, and `is_front` takes no lock beyond its momentary probe.
- **Open validation recommendations.** These are not addressed in this PR:
  - Stale doc comments describe the pre-queue behaviour:
    `Unavailable::lock_contention()` (`research/src/sources/fetch.rs:173`),
    `CALL_BUDGET` (`research-cli/src/main.rs:74`) and `summary`'s "attempts"
    (`research-cli/src/render.rs:77`).
  - `research.md:85` says only failed or unavailable calls write a stderr
    line, but a `waiting` call writes one too.
  - `contention.rs:48` hard-codes "900 s cap" instead of deriving it from
    `queue::CAP`.
  - The arXiv profile's Unavailable bullet omits the planned mid-retry
    reason note.
- **Known edge cases** (from the validation; benign):
  - A ticket `.lock` that is a directory is never removed. Housekeeping
    reports it on every pass.
  - A failed fresh join can leave an orphaned, unlocked `<ticket>.lock`
    until the next housekeeping pass.
  - After a killed send, a call with 33.0–33.1 s left can take
    `arxiv.lock` and then step aside without sending a request.
- **Load ceiling.** Claude Code caps concurrent subagents at 20, so step 17
  loaded the queue with at most 20 researchers rather than 24.
