---
type: "work-item"
id: "0310"
title: "Halting Failures in Conduct, Starting with Agent Usage Limits"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0121"
blocks: ["work-item:0304"]
relates_to: ["work-item:0283", "work-item:0303"]
tags: ["research", "deep-research", "conduct"]
last_updated: "2026-10-09T17:58:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0310: Halting Failures in Conduct, Starting with Agent Usage Limits

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

When an agent stops because of an account or API usage limit, every further
spawn in the session fails the same way, yet `conduct` keeps issuing
batches and its failure table says nothing about recovery. Introduce a
halting-failure class in `conduct`: after any batch containing one, spawn no
further batches and print the recovery. Agent usage limits are its first
member, recovered by re-running `conduct` after the limit resets.

## Context

In a depth-3 run, the final batch of 10 composers all failed before writing
anything:

```text
Agent terminated early due to an API error: You've hit your session limit · resets 5:30pm (Europe/London) (error type rate_limit, HTTP 429 …)
```

The run ledger had recorded all 10 compose spawns as offered, so the same
run would never offer them again. Re-invoking `conduct` after the reset ran
`outstanding --start`, which warned that it replaced the previous run's
ledger and offered only the 10 compose spawns. Recovery worked but is
undocumented, and with more batches pending the orchestrator would have
burned them on failures.

This is distinct from a source's `rate_limited` from `fetch`, which the table
already covers and which affects one source, not the session. WebSearch
quota exhaustion (0304) has the same session-wide shape and the same
stop-reset-re-run recovery.

## Requirements

- **Halting-failure class**: `conduct` names a class of failure reasons
  meaning every further spawn in this session would also fail. After a
  batch's checks (step 5), if any spawn failed with a halting reason,
  `conduct` spawns no further batches: it finishes recording the current
  batch, prints the usual summary, then prints the halting reason, the
  affected spawns, and the recovery.
- **Agent usage limit**: a spawn whose agent terminated with an account,
  session or API usage-limit error (Claude Code's "You've hit your … limit"
  / `rate_limit` / HTTP 429 termination) is recorded as failed with reason
  "agent usage limit", including the reset time when the error gives one.
  It is a halting failure.
- **Failure-reason table**: the table gains an "agent usage limit" row,
  separate from the `fetch` `rate_limited` row, whose next step is: re-run
  `conduct` after the limit resets; the new run replaces the interrupted
  run's ledger and offers only the outstanding spawns.
- **Extensible**: the class is defined so 0304's "web search quota
  exhausted" joins it by adding a reason and a recovery, without a second
  stop rule.

## Acceptance Criteria

- [ ] Given a batch in which any spawn terminates with an agent usage-limit
      error, when `conduct` checks the batch, then that spawn is recorded as
      failed with reason "agent usage limit" (including the reset time when
      present), and no further batch is spawned.
- [ ] Given a run stopped by a halting failure, when `conduct` ends, then the
      summary names the halting reason, lists the affected spawns, and gives
      that reason's recovery.
- [ ] Given a batch where other spawns succeeded alongside a usage-limit
      failure, when `conduct` checks the batch, then the successful spawns
      are retained and validated as normal.
- [ ] Given a run stopped by an agent usage limit, when the user re-runs
      `conduct <slug>` after the reset, then the new run offers only the
      outstanding spawns, including the failed ones.
- [ ] The failure-reason table has an "agent usage limit" row distinct from
      `rate_limited`.
- [ ] The skill defines halting failures in one place, listing their member
      reasons.

## Open Questions

- Does Claude Code report the termination reason to the orchestrator
  consistently enough to match on? Other limit kinds (weekly,
  organisation) may word it differently from the observed session-limit
  text.

## Dependencies

- Blocked by: none known
- Blocks: 0304, whose stop rule joins the halting-failure class defined
  here.
- Relates to: 0303. Both edit step 5 and the failure-reason table.
  0303's capacity-exhaustion reason is not a halting failure. Either item
  may land first; the second reconciles the table. 0303's retry rounds stop
  early once any spawn the batch message or a round accepted ended with a
  halting failure; whichever item lands second adds that rule.

## Assumptions

- A usage limit hit by one agent applies session-wide, so stopping further
  batches loses no work that could have succeeded.

## Technical Notes

- Spawn checks (step 5): `skills/research/research-topic/SKILL.md:359-370`.
- Failure-reason table: `skills/research/research-topic/SKILL.md:441-455`.
- Ledger replacement on `--start`: `cli/research/src/conduct/ledger.rs`.

## Drafting Notes

- Upgraded from a documentation task to a story when the user put stopping
  behaviour in scope as a shared halting-failure class.
- Ordering against 0304 is my choice; whichever lands first can define the
  class. If 0304 is built first, invert the `blocks` link.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/147
- Related: 0121, 0283, 0303, 0304
