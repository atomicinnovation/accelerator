---
type: "plan-validation"
id: "2026-10-06-0295-fair-arxiv-fetch-queue-validation"
title: "Validation Report: Fair arXiv Fetch Queue Implementation Plan"
date: "2026-10-06T20:18:21+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-06-0295-fair-arxiv-fetch-queue"
tags: ["research", "arxiv", "pacing", "fetch"]
last_updated: "2026-10-06T21:11:39+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Fair arXiv Fetch Queue Implementation Plan

Result: pass. All five phases are implemented, every automated criterion
passes on the current tree, and the Phase 5 attended checks and step 17 run
pass. The deviations below are small, and none changes a behaviour the plan
or work item specifies.

### Implementation Status

✓ Phase 1: Whole-Call Admission Under the Serving Lock - Fully implemented
✓ Phase 2: Queue Domain Model - Fully implemented
✓ Phase 3: File-Backed Queue Adapter - Fully implemented
✓ Phase 4: `fetch` Queues arXiv Calls - Fully implemented
✓ Phase 5: Attended Checks, Validation Run and Annotations - Fully implemented

Every test the plan names exists, by name, by an obvious rename, or through a
replacement the Phase 4 table lists.

### Automated Verification Results

Run on the tree after the send-margin commit:

✓ Full local CI mirror: `mise run` (4,598 CLI tests pass, 2 skipped)
✓ Read-only CI set: `mise run check`
✓ Public API snapshots: `mise run public-api:check`
✓ Prompt structure and lane wiring: `uv run pytest tests/unit/tasks/test_research_structure.py tests/unit/tasks/test_mise.py` (106 passed)
✓ Guard differential: `mise run test:integration:research`
✓ Generated docs current: `mise run docs:generate` leaves no diff
✓ Docs lane: `mise run docs:check`
✓ Stress lane: `mise run test:integration:arxiv-queue-stress` (1 test, 117 s)
✓ Real-time pacing: `arxiv_pacing` 20 of 20 under `--stress-count 20`
✓ Validation frontmatter: `accelerator corpus frontmatter validate`

### Code Review Findings

#### Matches Plan:

- `PacingGate { spacing, try_serve }`, `ServingTurn::paced` and
  `OutOfBudget` match the Ports section. The backoff sleeps while the turn
  is alive, and `confirmations.recall` runs under it.
- Admission is 33 s for 32 s / 33 s and 39 s for a 6 s backoff, from
  `admits_attempt_after(now, wait + spacing)`.
- `Queue::present` applies Join, mismatch, live, cap and Resume in the
  planned order. Elapsed time saturates, and abandonment only ever caps an
  end.
- `FileArxivQueue` probes with a shared non-blocking lock on a read-write
  open without create. `own` retries every 1 ms within 500 ms, and every
  record write, removal and ticket-lock creation happens under
  `queue.lock`.
- No lock-order violation was found. The turn is consumed and dropped
  before `leave` or `step_aside`, and `is_front` takes no lock beyond its
  momentary probe.
- `upstream_reason_or_contention` is the only place contention is
  recorded.
- The `waiting` JSON, its summary line and both rejection lines match the
  plan's wording.
- The profiles, reason rows, `research.md`, CHANGELOG and stress-lane
  wiring are as planned.

#### Deviations from Plan:

- **Send margin.** A send with no finish after it is spaced 3.1 s
  (`cli/research-adapters/src/pacing.rs:58-61`). Recorded in the plan's
  Implementation Notes.
- **Lock bounds carry small margins.** `join`'s `queue.lock` bound keeps
  `spacing + 10 ms` in reserve, and `step_aside` and `leave` stop with
  10 ms left rather than at zero (`cli/research-adapters/src/queue.rs:546-548`).
- **Own ticket probed.** `step_aside` and `leave` probe their own ticket
  rather than marking it `Live`. A fresh descriptor's shared probe reads
  `Held` against the process's own exclusive lock, so the result is the
  same.
- **Position after a failed write.** When `step_aside`'s record write
  fails, it returns the recomputed position rather than the join position
  (`queue.rs:505-515`).
- **Shared store helpers.** `store::replace_without_sync` shares
  `contained_target`, `stage` and `publish` with `atomic_write`, not just
  `ensure_contained` and the staging name. `atomic_write`'s two fsyncs stay
  inline and unflagged, as the plan requires.
- **Abandoned live ticket.** Re-presented, it reads as `OverCap` or `Join`
  rather than `AlreadyLive`. This follows from the abandonment rule;
  `a_live_abandoned_ticket_presented_again_is_not_already_live` covers it.
- **Unavailable bullet shortened.** The arXiv profile's Unavailable bullet
  omits the planned note that a ticket whose last call ran out mid-retry
  reports that retry's reason. The required "Any other status" sentence is
  present.
- **Phase 5 call count.** The attended re-presentation runs show 5 `fetch`
  invocations, not 4, because each researcher made one further legitimate
  search; see Phase 5 Evidence.

#### Potential Issues:

- **Stale doc comments.** These describe the pre-queue behaviour:
  - `Unavailable::lock_contention()` still reads "Another process held the
    source for longer than the deadline allowed"
    (`cli/research/src/sources/fetch.rs:174`);
  - `CALL_BUDGET` says a throttled source "reports itself unavailable"
    (`cli/research-cli/src/main.rs:74-75`);
  - `summary` says it names attempts, which the waiting line omits
    (`cli/research-cli/src/render.rs:77-78`).
- **Docs gap.** `research.md:84-86` says only failed or unavailable calls
  write a stderr line; a `waiting` call writes one too.
- **Duplicated constant.** `contention.rs:48` writes "900 s cap" literally
  rather than from `queue::CAP`.
- **Margin edge.** The domain admits on 3 s spacing, but after a killed
  send the gate needs 3.1 s. A call with 33.0–33.1 s left can take
  `arxiv.lock` and then step aside without a request, which fits the plan's
  "first attempt checks the clock again" guarantee.
- **Undeletable lock path.** A ticket `.lock` that is a directory is never
  removed; housekeeping reports it each pass, while its record is still
  pruned by age.
- **Orphaned lock file.** A fresh join whose `own` or record write fails
  leaves an orphaned `<ticket>.lock` until the next housekeeping pass. The
  lock is released, so nothing waits on it.

### Manual Testing Required:

1. Review by someone other than the implementer:
  - [ ] Read the CHANGELOG entry and the `research.md` Pacing section
  - [ ] Read the 0283 annotations in place
2. Release placement:
  - [ ] Decide whether the CHANGELOG entry belongs under `### Breaking`
        rather than `### Changed`, since `fetch` callers must now handle
        `waiting`

### Recommendations:

- Update the three stale doc comments and the `research.md` stderr
  sentence before merge.
- Derive the contention-log cap text from `queue::CAP`.
- Restore the mid-retry note to the arXiv profile's Unavailable bullet, or
  record its omission in the plan.
- Consider raising `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` to 24 for a
  future step 17 run, so the queue sees the full planned load.

### Phase 5 Evidence

#### Method

Each check ran as a headless `claude -p` session (Claude Code 2.1.292,
`claude-opus-5-5`) with `--plugin-dir` pointed at this tree, using the
contributor launcher override (`ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER=1`,
`ACCELERATOR_LAUNCHER_BIN`, and `ACCELERATOR_<SUB>_BIN` for each sub-binary
built from this tree) and `--permission-mode bypassPermissions`. The
installed plugin was disabled through `--settings`. Each session ran in its
own fresh scratch git repository, so no other session or older binary shared
its `arxiv.lock` or queue.

- **Attended checks.** One `arxiv` pair at `--depth 1`, so each session
  spawned one researcher. `ACCELERATOR_RESEARCH_BIN` pointed at a stub that
  `exec`s the real `accelerator-research` for every subcommand but `fetch`,
  so the research guard hook and `topic outstanding` ran for real. `fetch`
  answered from a per-repository call counter and logged its arguments.
- **Step 17.** Three `arxiv` pairs at `--depth 3` with default concurrency.
  For each pair, `1.md` was seeded with 4 follow-ups and every `2-k.md` with
  2, all 12 distinct from each other and from the pair question. Seeded
  notes took their path, `id` and `question` from `outstanding --depth 3`,
  and validate. `ACCELERATOR_RESEARCH_BIN` pointed at a timing wrapper that
  runs the real binary unchanged and logs each `fetch` invocation's start,
  end, arguments and printed status.
- **Response time.** For each invocation that settled `ok` or
  `unavailable`, its last request's send time comes from
  `arxiv-requests.log`, and its response time is the invocation's end minus
  that send.
- **Host.** Each step 17 run ran under `caffeinate -i`, so the host stayed
  awake.
- **Cost.** 9 attended sessions, about $4.65; 2 step 17 runs, about $18.33.

#### Attended researcher checks

| Check | Result | Evidence |
|---|---|---|
| Re-presentation | pass 3 of 3 | invocations 2–4 carry `--ticket 7-a1b2c3` with unchanged query and `--limit`; 1 note each |
| `lock_contention` | pass 3 of 3 | 1 invocation; no note; summary gives the reason row's re-run remedy |
| Changed ticket | pass 3 of 3 | invocations 2–3 carry `--ticket 7-a1b2c3`, invocation 4 exactly one `--ticket 8-d4e5f6`; 1 note each |

In all 6 runs that reached `ok`, the researcher went on to make one further
search with a new query and no `--ticket`, within the profile's 3-search
cap. The plan's "4 `fetch` invocations" assumed the researcher would stop
after the settled search. The property the check targets holds in every
run: each waiting ticket was re-presented at once with unchanged arguments,
the latest ticket replaced the earlier one, no ticket leaked into a later
call, and exactly one note was written. Every written note and manifest
validates.

#### Planner guard

`outstanding --start --limit 24` over the seeded step 17 set offers exactly
24 level-3 spawns, all `arxiv`, with `remaining` 0, and
`research.topic.concurrency` resolves to 24. In both runs the orchestrator's
first plan offered 24 level-3 nodes and it spawned all 24 as batch 1; in
run 2 the 24th waited for a subagent slot, as Observations describes.
`a_24_limit_offers_24_arxiv_spawns_and_counts_the_rest` and the catalogue
default assertion remain the enforcing tests.

#### Step 17

| Measure | Run 1 | Run 2 |
|---|---|---|
| Median arXiv response | 0.33 s | 0.15 s |
| `waiting` outcomes / re-presentations | 0 / 0 | 5 / 5 |
| Verdict | not evidence | judged |
| Nodes lost to `lock_contention` | 0 of 24 | 0 of 24 |
| Other losses | 0 | 0 |
| Un-re-presented `waiting` tickets | 0 | 0 |
| `fetch` invocations / arXiv requests | 134 / 134 | 133 / 128 |
| `arxiv-contention.log` lines | 0 | 0 |
| Wall-clock, first fetch to last level-3 note | 498 s | 454 s |
| Fetch window | 459 s | 428 s |
| Peak concurrent `fetch` invocations | 19 | 20 |
| Longest / median invocation | 71.5 s / 37.9 s | 74.8 s / 36.5 s |
| Notes / findings written | 24 / 3 | 24 / 3 |

The median response time was well under the 3 s threshold in both runs, and
was read before either outcome.

Run 2's 5 `waiting` outcomes each stepped aside after about 67 s of queueing,
at position 2 or 3, without sending a request. Each was re-presented once
with unchanged arguments and returned `ok`. No call ran out of budget
mid-call, so no search was repeated. After the run, `arxiv-queue/` held only
`queue.lock`, no run ledger remained, and whole-corpus `accelerator corpus
frontmatter validate` exits 0.

For information:

- **arXiv idle time.** About 33 s over run 2's 128 requests, summing each
  gap minus the median response and the 3 s spacing. The longest gap was
  18.7 s; the shortest 3.03 s.
- **Re-presentations per node.** 5 over 24 nodes, about 0.2.
- **Repeated lookups.** 9 arXiv IDs were looked up by more than one
  researcher. These are separate nodes citing the same preprint, not
  repeats after mid-call exhaustion.
- **Researcher tokens.** The transcripts report usage for the orchestrator
  only; the whole run cost about $9.16.

`waiting` counts are not comparable with 0283's contention-log entries: a
`waiting` outcome keeps its place and is re-presented, where a 0283
contention entry lost its node.

#### Observations

- 🟡 **Claude Code caps concurrent subagents at 20.** In run 2 the
  orchestrator hit the limit and launched the 24th researcher once a slot
  freed, and the peak of 20 concurrent fetches reflects it. The queue was
  therefore loaded by at most 20 researchers at once rather than 24. 0283's
  step 17 ran on 2.1.282; whether it met the same cap is not recorded.
  `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` raises it.
- 🟡 **Researchers fill truncated abstracts from memory.** The orchestrator
  reported that 10 of 24 researchers said they supplied mechanism details
  from their own knowledge because the CLI's abstracts are truncated, and
  that most notes do not flag it. This is outside 0295's scope.
</content>
</invoke>
