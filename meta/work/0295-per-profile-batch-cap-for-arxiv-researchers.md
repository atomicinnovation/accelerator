---
type: "work-item"
id: "0295"
title: "Fair arXiv Fetch Queue That Waits Across Calls"
date: "2026-09-28T09:03:54+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "done"
kind: "story"
priority: "medium"
parent: "work-item:0121"
relates_to: ["work-item:0283", "work-item:0161"]
tags: ["research", "deep-research", "arxiv"]
last_updated: "2026-10-06T21:30:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-882"
---

# 0295: Fair arXiv Fetch Queue That Waits Across Calls

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an Accelerator user researching a subject with `arxiv` pairs at depth
above 1, I want arXiv fetch requests to queue fairly and keep their place
across invocations, so that a deepened round loses no nodes to arXiv lock
contention unless an unexpired ticket is re-presented more than 900 s after
issue, or the queue itself is unusable.

## Context

A pair is a (focus area, source profile) combination, and a node is one
unit of research work within a pair's tree, handled by one researcher; both
are defined in 0283. All arXiv requests in a project share one lock, spaced
3 s apart, with a 100 s budget per `fetch` invocation. Recursive deepening
(0283) lets up to `concurrency` arXiv nodes queue on that lock at once; the
default is 24. 0283's attended verification ran 3 `arxiv` pairs at depth 3
with default concurrency, so the level-3 batch held exactly 24 arXiv nodes:

| Measure | Value |
|---|---|
| Contention-log entries over arXiv requests | 4 of 110 (3.6%) |
| Nodes failed as `lock_contention` | 4 of 24 (17%) |
| Nodes failed as `rate_limited` for any other cause | 0 |
| Wall-clock, batch issued to last note | 452 s |

A failed node is recoverable, because a re-run researches only the missing
nodes. But one node in six failing at the default setting makes a full
arXiv round routinely need a second `conduct`.

The lock is held for the whole response plus the spacing, so a lock turn
averaged ~4.1 s (452 s / 110 requests), not 3 s. A 100 s budget therefore
covers ~24 lock turns. arXiv's terms (one connection at a time, 3 s apart)
fix that throughput, so the risk is not the number of waiters but that all
of an invocation's waiting must fit inside its budget. One fetch request can
also need several lock turns: a search or lookup, then one withdrawal
confirmation per uncached candidate, each taking the lock separately while
other invocations interleave. A node's single `unavailable` outcome writes
no note, so one lost race fails the node.

This item began as a per-profile batch cap and was rescoped to a fair
queue; the Drafting Notes say why.

## Terms

- **Request**: one HTTP call to arXiv: a search, a lookup, or a withdrawal
  confirmation.
- **Attempt**: one try of a request. A request makes up to four attempts,
  separated by retry backoffs.
- **Withdrawal confirmation**: an OAI-PMH request that checks whether a
  candidate record has been withdrawn. Its verdict is cached, so each
  candidate is confirmed once.
- **Fetch request**: one search or lookup together with its withdrawal
  confirmations, identified by its `fetch` arguments.
- **Invocation**: one `fetch` process, with one budget (`CALL_BUDGET`,
  100 s). "Budget" below always means this.
- **Ticket**: a place in the queue for one fetch request. It is issued on
  the fetch request's first invocation and bound to its arguments. A later
  invocation re-presents it by passing it to `fetch` with the same
  arguments; a ticket passed with different arguments is rejected. A
  different fetch request takes a new ticket.
- **Researcher**: the agent that handles one node and invokes `fetch`.
- **Serving lock**: the existing per-project `arxiv.lock`, held by the one
  invocation currently making arXiv requests. It is kept, not replaced.
- **Ticket lock**: a `flock` on the ticket itself, held by the live
  invocation presenting it.
- **Admitted**: an invocation at the front of the queue passes the budget
  check and is then served. An invocation that fails the check is never
  served.
- **Served**: an invocation is served when it acquires the serving lock.
- **Serving window**: room for one attempt, 33 s: the 30 s
  `REQUEST_BUDGET` plus the 3 s spacing.
- **Live / absent**: a ticket is live while an invocation holds its ticket
  lock, and absent otherwise.
- **Invocation end**: the moment the ticket lock is released, whether the
  invocation exits or is killed.
- **Expired**: an absent ticket whose most recent invocation ended more
  than 300 s ago. An expired ticket counts toward no position and is never
  served.
- **Lock turn**: one request's time under the serving lock, whoever holds
  it.

## Requirements

- arXiv fetch requests wait in a first-in, first-out queue of tickets,
  shared by every arXiv `fetch` invocation in the project, including
  invocations from concurrent `conduct` runs.
- Order holds among live tickets. An unexpired absent ticket is skipped, so
  it never stalls live tickets, and keeps its priority when re-presented.
- A live invocation holds its ticket lock, so an invocation killed mid-wait
  or mid-call frees its place without waiting for expiry; the next live
  ticket is served within 1 s.
- An invocation is admitted only while its remaining budget covers a
  serving window. Once admitted, it holds the serving lock for all of its
  requests and every retry backoff between their attempts, so no other
  invocation's request interleaves with them. Before every attempt after
  admission, including the first attempt of each later request (with no
  backoff), the remaining budget must cover that attempt's backoff plus a
  serving window. Spacing between requests is unchanged.
- An invocation whose remaining budget falls below a serving window before
  it is admitted exits with a `waiting` outcome carrying its ticket and
  queue position, having made no request. The position is 1-based and
  counts every unexpired ticket ahead, live or absent.
- An admitted invocation whose remaining budget cannot cover its next
  attempt releases the serving lock and returns `waiting`. Its ticket keeps
  its priority but is skipped while absent like any other. On
  re-presentation its search is repeated, while confirmation verdicts
  already cached are reused.
- Re-presenting a ticket within 300 s of its most recent invocation end
  resumes its place, unless the 900 s cap below applies. A ticket absent for longer expires, and re-presenting
  it rejoins the back with a fresh ticket.
- An unexpired ticket re-presented more than 900 s after issue returns
  `rate_limited` with `cause: lock_contention`, enforced in Rust. The
  exception is a ticket whose most recent invocation ran out of budget while
  retrying a retryable failure: it returns `unavailable` with that failure's
  reason, with no `lock_contention` cause and no contention-log entry, so a
  persistently failing upstream is not reported as queue contention. Expiry is
  checked before the cap, so an expired ticket rejoins with a fresh ticket
  and a reset clock. The cap is checked only on re-presentation; an
  invocation already waiting when its ticket crosses 900 s runs on until
  served or out of budget. The cap is per ticket: a node making several
  fetch requests can wait longer in total.
- The arXiv researcher re-presents a `waiting` ticket until the outcome is
  anything other than `waiting`, and writes no note while it is `waiting`.
- If the queue itself cannot be used (its lock cannot be opened, locked
  or taken within the budget), an arXiv fetch is admitted unqueued. Having
  no place to keep, it returns `rate_limited` with
  `cause: lock_contention` rather than `waiting` when its budget runs out,
  so the researcher's loop stays bounded.
- After this change the 900 s cap and an unusable queue are the only paths
  to `lock_contention`. The `lock_contention` cause stays distinct from the
  throttling cause of `rate_limited`. `arxiv-contention.log` is written
  only when one of them fires.
  `research-topic`'s reason row names re-running `conduct` as the remedy
  instead of lowering `--concurrency`.
- `conduct` has no per-profile arXiv spawn cap today, and this story adds
  none; the default `concurrency` stands.
- OpenAlex and web fetches are unaffected and never return `waiting`.

## Acceptance Criteria

Ordering criteria are observed as the order in which requests reach a stub
arXiv.

- [x] Given three live tickets that join the queue in order A, B, C, when
      the serving lock frees, then they are served in order A, B, C, even
      when C polls before A and B.
- [x] Given absent unexpired ticket A ahead of live ticket B, when the
      serving lock frees, then B is served without waiting for A; when A is
      re-presented, it is served before every ticket issued after it.
- [x] Given two `conduct` runs in the same project, when both issue arXiv
      fetch requests, then their tickets are served from one queue in issue
      order.
- [x] Given a ticket issued for one search, when it is re-presented with
      different `fetch` arguments, then `fetch` returns a rejection outcome
      naming the mismatched arguments, makes no request, and leaves the
      queue unchanged.
- [x] Given an invocation needing a search and two withdrawal
      confirmations, and a stub arXiv that returns a retryable error on the
      first confirmation, when it is served while other invocations wait,
      then its requests run consecutively, each at least 3 s after the
      previous request, and no other invocation's request reaches the stub
      before that confirmation's retry completes.
- [x] Given an invocation at the front with 32 s of budget left, when it
      would be admitted, then it exits with a `waiting` outcome carrying
      its ticket and position, and makes no request; given 33 s left, it is
      admitted and makes its first request.
- [x] Given an admitted invocation whose attempt failed with a retryable
      error and a 6 s backoff pending, when it has 38 s of budget left, then
      it releases the serving lock and returns `waiting`; given 39 s left, it
      backs off holding the serving lock and retries.
- [x] Given an invocation with a 40 s budget behind a live ticket that holds
      the serving lock for 20 s, when its remaining budget falls below 33 s,
      then it exits with a `waiting` outcome carrying its ticket and
      position 2, and makes no request.
- [x] Given expired ticket A, absent unexpired ticket B and live ticket C
      ahead of ticket D, when D returns `waiting`, then it reports
      position 3.
- [x] Given an admitted invocation whose remaining budget cannot cover its
      second confirmation, when it reaches that point, then it releases the
      serving lock and returns `waiting`; while it is absent a later live
      ticket is served; when re-presented, it is served before every ticket
      issued after it, repeats the search, and re-issues no confirmation
      already cached.
- [x] Given a `waiting` ticket re-presented exactly 300 s after its most
      recent invocation end, when the invocation runs, then it keeps its
      place ahead of every ticket issued after it; re-presented at 301 s,
      it rejoins at the back with a fresh ticket.
- [x] Given a ticket issued 950 s ago whose most recent invocation ended
      400 s ago, when it is re-presented, then it rejoins at the back with a
      fresh ticket rather than returning `rate_limited`.
- [x] Given an invocation killed mid-wait or mid-call, when the serving
      lock next frees, then the next live ticket is served within 1 s,
      without waiting for any expiry, and its first request still reaches
      the stub at least 3 s after the previous request; when the killed
      ticket is re-presented within 300 s, it keeps its place.
- [x] Given an unexpired ticket re-presented exactly 900 s after issue,
      when the invocation runs, then it is queued as usual; re-presented
      901 s after issue, it returns `rate_limited` with
      `cause: lock_contention`, makes no request, and writes one entry to
      `arxiv-contention.log`.
- [x] Given an unexpired ticket whose most recent invocation stepped aside
      while retrying an upstream error, when it is re-presented more than
      900 s after issue, then it returns `unavailable` with reason
      `upstream_error` and no cause, makes no request, and writes nothing
      to `arxiv-contention.log`.
- [x] Given an arXiv queue whose lock cannot be used and a serving lock
      held past the invocation's budget, when an arXiv fetch runs, then it
      returns `rate_limited` with `cause: lock_contention` rather than
      `waiting`, makes no request, and writes one entry to
      `arxiv-contention.log`.
- [x] Given an admitted invocation whose stub arXiv returns 429 on every
      attempt, when its attempts are exhausted, then it returns
      `rate_limited` with a cause other than `lock_contention`; given a
      `Retry-After` that pushes the next backoff plus a serving window past
      the remaining budget, it returns `waiting`.
- [x] Given an invocation waiting when its ticket crosses 900 s since
      issue, when the serving lock frees for it within its budget, then it
      is served.
- [x] Given 30 concurrent fetch requests, each re-presenting its ticket
      until a non-`waiting` outcome, against a stub arXiv with 1 s response
      latency, no errors and search results with no withdrawal candidates,
      when all complete, then every fetch request succeeds, none returns
      `lock_contention`, at least one returns `waiting` before succeeding,
      and no two stub requests overlap or start less than 3 s apart.
- [x] Given an attended `conduct` run with one `arxiv` node whose stubbed
      `fetch` issues ticket T with `waiting`, returns `waiting` twice more
      and then succeeds, when the researcher finishes, then its transcript
      shows the three `fetch` invocations after the first presenting T with
      unchanged arguments, no later invocation presenting any ticket, and
      the node writing exactly one note; given a stub that returns
      `rate_limited` with `cause: lock_contention`, the node writes no note.
      Both pass in 3 of 3 runs.
- [x] Given the arXiv serving lock held and live arXiv tickets queued, when
      an OpenAlex or web fetch runs, then it completes without returning
      `waiting` and the arXiv queue is unchanged.
- [x] As a regression guard on the unchanged planner: given at least 24
      pending arXiv nodes and default `concurrency` (24), when `conduct`
      spawns a batch, then it spawns exactly 24 arXiv nodes.
- [x] Re-running 0283's step 17 setup (3 `arxiv` pairs, depth 3, default
      concurrency) loses 0 nodes to `lock_contention`, recorded in a new
      0295 validation document under `meta/validations/`:
      - The document records the wall-clock time, the median arXiv
        response time, and the count of `waiting` outcomes and
        re-presentations. A run with none of either is not evidence and is
        repeated.
      - A run whose median arXiv response time exceeds 3 s is abnormal and
        repeated rather than judged; the threshold is applied before the
        outcome is read.
      - Any node lost with a `waiting` ticket that was not re-presented
        fails the run.
      - Nodes lost to other causes are listed with their outcome. More than
        2 such losses of 24 makes the run inconclusive.
      - At most 3 runs are made. If none is valid evidence, the item is
        recorded as unvalidated, not passed.
      - No wall-clock bound applies.
- [x] `research-topic`'s `lock_contention` reason row names re-running
      `conduct` as the remedy and no longer tells the user to lower
      `--concurrency` first.
- [x] 0283's plan and validation documents carry a note that 0295 was
      rescoped from a per-profile batch cap to this fair queue.

## Open Questions

- What is the on-disk form of the queue and tickets, and what is the shape
  of the `waiting` and rejection outcomes and the `fetch` argument that
  re-presents a ticket, and how is `fetch` stubbed for the attended
  researcher criterion? To settle during planning, before any implementation phase
  starts. Whatever form is chosen must let tests set ticket issue and
  invocation-end times, so the 300 s and 900 s criteria run without real
  waits.

## Dependencies

- Blocked by: 0283 (recursive finding deepening) — satisfied; merged in
  PR #136. 0283's frontmatter still shows `ready`; its plan and
  validation name 0295 as a per-profile batch cap and are annotated as an
  acceptance criterion here.
- Blocks: none.
- Relates to: 0161's recursion eval, which covers `conduct`'s
  orchestration contracts. Whether it relies on today's `lock_contention`
  remedy text is unchecked; to check during planning. Any eval change it
  needs is out of scope here and raised as its own item.
- External: the arXiv search API and OAI-PMH endpoint, under their current
  terms (one connection at a time, 3 s apart). The 900 s cap and the
  validation run assume those terms and typical latency, and the
  validation run needs both endpoints reachable.
- Platform: the 120000 ms Bash `timeout` the arXiv profile passes, which
  bounds the 100 s budget; a researcher subagent's ability to re-invoke
  `fetch` repeatedly; and advisory `flock` on a local filesystem, which the
  OS releases when the holding process exits.
- Consumers: of `waiting`, the arXiv profile and `agents/researcher.md`; of
  the changed `lock_contention` remedy, the reason row (the table row
  mapping an unavailable cause to its user-facing remedy) in
  `skills/research/research-topic/SKILL.md`; of the narrowed
  `arxiv-contention.log`, anyone measuring contention as 0283 did. The 0295
  validation counts `waiting` outcomes instead of contention-log entries,
  so its figures are not directly comparable with the 0283 baseline.
- Validation: an attended `conduct` session for a 24-node depth-3 arXiv
  round, possibly repeated, with the model budget that needs.

## Assumptions

- The contention rate scales with concurrent arXiv nodes, not with the
  number of pairs or the depth.
- Nothing stops a researcher issuing parallel fetch requests, each of which
  takes its own ticket.
- A researcher can re-invoke `fetch` repeatedly without hitting a limit of
  its own before a ticket's 900 s cap is reached.
- The gap between a researcher's consecutive `fetch` invocations is one
  agent turn, well under 300 s.
- `conduct` reads only the notes researchers write, not `fetch` outcomes,
  so `waiting` needs no handling outside the researcher beyond a reason row
  for a researcher that ends without re-presenting its ticket.

## Technical Notes

- No planner cap can remove the risk while all of a fetch request's waiting
  must fit in one budget: arXiv throughput is fixed at ~1 request per
  4.1 s, so any arrangement that queues more than ~24 lock turns ahead of an
  invocation can fail it. A ticket that outlives the invocation moves the
  waiting out of the budget. Wall-clock may rise above the 452 s baseline:
  backoff held under the serving lock, searches repeated after mid-call
  exhaustion, and the delay before a killed invocation's ticket lock is
  noticed each add idle lock time.
- Whole-call admission is part of this story, not a follow-up. Without it,
  a fetch request's confirmations re-enter contention with every other
  ticket between requests, so a multi-request fetch has no FIFO guarantee
  and can still fail its node.
- The serving window is one attempt, not one request with its full retry
  schedule: four 30 s attempts plus 3, 6 and 12 s backoffs (`RetrySchedule`
  in `cli/research/src/sources/schedule.rs`) total 141 s, and up to 210 s
  when `Retry-After` raises each backoff to its 30 s ceiling, so no
  invocation within the 100 s budget could ever be admitted.
  `Attempts::until_settled` already checks the deadline before each retry;
  admission extends that check to the queue.
- Under skip-while-absent, expiry costs no stalled lock time, only stale
  entries and inflated positions for tickets behind; hence the generous
  300 s. Skip-while-absent also makes a position a poor predictor of
  wait, so `waiting` carries no estimate.
- The pacing gate is `FilePacingGate` in
  `cli/research-adapters/src/pacing.rs` (`SPACING` 3 s, non-FIFO `flock`
  polled every 100 ms, held through the response). `PacingGate::paced`
  wraps one attempt, so admission for a whole fetch request needs the gate
  to span every attempt `fetch_arxiv` makes
  (`cli/research/src/sources/fetch.rs`); retry backoff currently sleeps
  outside the lock.
- The 100 s `CALL_BUDGET` in `cli/research-cli/src/main.rs` is bounded by
  the 120000 ms Bash `timeout` the arXiv profile passes, not by Claude
  Code, which allows up to 600000 ms. Raising the budget to ~550 s would
  serve the 0283 batch within one invocation; it was rejected in favour of
  tickets, which keep each invocation short and have no queue-length
  ceiling. Tests can already shorten the budget through the loopback
  override (`loopback::call_budget`).
- The serving lock is per project (`arxiv.lock` under
  `paths.tmp/research`) and used only by researcher `fetch` invocations;
  OpenAlex is unpaced (`NoPacing`) and web has no Rust fetch path.
- Contention today returns `Unavailable::lock_contention()`
  (`reason: RateLimited, cause: LockContention`) and logs to
  `arxiv-contention.log`.
- The researcher's handling of an `unavailable` outcome is in
  `skills/research/profiles/arxiv-profile/SKILL.md`; the `waiting` outcome
  and its re-presentation belong there and in `agents/researcher.md`. The
  reason row is in `skills/research/research-topic/SKILL.md`.
- The story is one increment because no part delivers value alone, but the
  plan can phase it so each phase is verifiable: whole-call admission under
  the existing lock, then the ticket queue and `waiting`, then expiry and
  the cap, then the researcher's re-presentation and the reason row, then
  the validation run.
- Measurement method and raw figures are in
  `meta/validations/2026-09-28-0283-recursive-finding-deepening-validation.md`.

## Drafting Notes

- Raised from 0283's Phase 7 step 17, which states that a material
  contention rate becomes a follow-up for a per-profile batch cap. This
  item replaces that follow-up and fully discharges it.
- Rescoped from a per-profile batch cap in the `conduct` planner to a fair
  queue in the pacing gate. A stress test found the cap's derivation
  assumed one lock turn per fetch request, which withdrawal confirmations
  break, and that no cap removes the risk while waiting is bounded by one
  invocation's budget. The earlier objection to changing the gate (a fair
  queue alone still leaves the 24th node waiting ~100 s) no longer holds
  once a ticket survives across invocations.
- Dropped the cap of 12, which had no derivation once multi-request fetches
  were accounted for. A cap for round duration was considered and
  rejected: longer rounds are accepted in exchange for no lost nodes.
- Kept the file name and id; only the title changed.
- Plan review added two paths to the `lock_contention` contract. An
  unusable queue ends a call in `lock_contention`, because an unpersisted
  ticket could otherwise be re-presented forever. A cap that fires after an
  upstream failure reports that failure, because a slow outage would
  otherwise read as contention after 900 s.

## References

- `meta/work/0283-recursive-finding-deepening.md`
- `meta/plans/2026-09-26-0283-recursive-finding-deepening.md`
- `meta/validations/2026-09-28-0283-recursive-finding-deepening-validation.md`
- `meta/work/0161-roll-out-inspect-evals-across-remaining-skills.md`
