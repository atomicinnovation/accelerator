---
type: "work-item"
id: "0310"
title: "Recovering from an Interrupted Conduct Run"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0121"
relates_to: ["work-item:0283", "work-item:0303", "work-item:0304", "work-item:0307", "work-item:0308"]
tags: ["research", "deep-research", "conduct", "recovery"]
last_updated: "2026-10-12T17:39:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0310: Recovering from an Interrupted Conduct Run

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a researcher whose `conduct` run died mid-way — an agent usage limit, a
crash, a closed terminal — I want the next `conduct` to tell me what the
interrupted run left unfinished and pick it up, so that I recover without
reading ledger warnings or guessing what was lost. When a usage limit hits,
the orchestrator shares the account's quota, so the interrupted run cannot
halt or summarise itself; recovery has to come from the next run.

## Context

In a depth-3 run, the final batch of 10 composers all failed before writing
anything:

```text
Agent terminated early due to an API error: You've hit your session limit · resets 5:30pm (Europe/London) (error type rate_limit, HTTP 429 …)
```

The run ledger had recorded all 10 compose spawns as offered, so the same
run would never offer them again. Re-invoking `conduct` after the reset ran
`outstanding --start`, which warned:

```text
replaced the ledger of run {run}; if that run is still going it stops at its next batch, and notes its last batch writes may show as unexpected
```

and offered only the 10 compose spawns. Recovery worked, but only by
accident of the ledger's design: nothing told the user what had been lost,
and the warning reads as a concurrency hazard, not an interruption.

The interrupted run never reached steps 5–7. Session, weekly and spend
limits are account-wide, and the orchestrator draws on the same quota as its
subagents, so its next model turn after the batch returned was refused too.
No `end-run`, tick, manifest update or summary ran, which is why the stale
ledger surfaced only on the next `--start`. A halt-and-summarise rule inside
the interrupted run cannot help here; it applies only to failures the
orchestrator survives — 0304's WebSearch quota, or a model-specific limit
when the orchestrator runs another model — and 0304 now owns that
halting-failure class.

Claude Code wraps a subagent's terminal API error as `Agent terminated early
due to an API error: <detail>` (v2.1.199+). Every plan-quota and spend-limit
variant begins `You've hit your …`; quota limits carry `· resets <time>`,
spend limits carry none. HTTP 429 and `rate_limit_error` appear on both real
limits and transient throttles, which Claude Code already retries about ten
times, so they cannot identify a usage limit. A known bug words a 5-hour
session limit as `You've hit your monthly spend limit`, so the limit kind in
the message is not reliable.

The same recovery covers every way a run can die without ending — a crash,
a closed terminal, a killed session — not only usage limits.

## Requirements

- **Interrupted-run report**: when `outstanding --start` replaces another
  run's ledger, the plan reports that run's last offered batch in structured
  form, split into spawns whose output is on disk and spawns with none. The
  skill shows the report before the first batch, worded as "run X did not
  end; if it is still going …", never asserting that the earlier run
  stopped.
- **Leftover writes**: notes and findings the interrupted run's last batch
  wrote, which the new run did not ask for, are reported as that run's
  writes, not as `unexpected`.
- **Unchecked findings**: findings the interrupted run wrote but never put
  through step 5 are validated by the next run before being counted, and
  quarantined and re-offered when they fail.
- **Recovery guidance**: the failure-reason table gains an "interrupted run"
  row whose next step is to re-run `conduct`, which re-offers what the
  earlier run left unfinished; for an agent usage limit, wait for the reset
  time in the message, or raise the spend limit when the message gives no
  reset time.

## Acceptance Criteria

- [ ] Given a stale ledger whose last batch offered N spawns, M of which
      wrote nothing, when `conduct` starts, then its first output names the
      earlier run, lists the M spawns as re-offered, and lists the other
      N − M as written by that run.
- [ ] Given a stale ledger, when `conduct` reports it, then the wording says
      the earlier run did not end and does not assert that it stopped.
- [ ] Given notes written by the interrupted run's last batch, when the new
      run summarises, then they do not appear under Unexpected notes.
- [ ] Given a finding the interrupted run wrote that fails validation, when
      the new run plans, then the finding is quarantined and its pair
      re-offered, not counted.
- [ ] Given a run interrupted by an agent usage limit, when the user re-runs
      `conduct <slug>` after the reset, then only outstanding spawns are
      offered, and the manifest counts and outline checkboxes are brought up
      to date from disk.
- [ ] The failure-reason table has an "interrupted run" row distinct from
      `rate_limited` and from the concurrent-subagent-limit row.

## Open Questions

- Does `outstanding` count a finding on disk that no step 5 ever validated?
  If so, how do the interrupted run's findings reach validation?
- Does the replaced ledger's pending window hold enough to reconstruct the
  last batch, or must `RunLedger` retain more?

## Dependencies

- Blocked by: none known
- Blocks: none
- Relates to: 0304, which defines the halting-failure class for failures
  the orchestrator survives; 0303, landed, whose concurrent-subagent-limit
  row sits beside the new row; 0307, which changes the per-batch snapshot
  this report reads; 0308, which reads the active ledger.

## Assumptions

- Session, weekly and spend limits apply to the whole account, so the
  orchestrator cannot take another turn once a subagent hits one.
- The subagent wrapper words weekly, model and spend limits the same way as
  the observed session limit; no wrapped example of those was found.

## Technical Notes

- Ledger replacement on `--start`: `cli/research-cli/src/topic_command.rs:273`;
  its warning: `:364`; `end-run`: `:319`.
- Ledger model: `cli/research/src/conduct/ledger.rs`.
- First-plan warnings reported: `skills/research/research-topic/SKILL.md:262-266`.
- Spawn checks (step 5): `skills/research/research-topic/SKILL.md:390-401`.
- Failure-reason table: `skills/research/research-topic/SKILL.md:474-488`.
- Error wording: https://code.claude.com/docs/en/errors

## Drafting Notes

- Reframed from a halting-failure story to next-run recovery once it was
  clear the orchestrator shares the subagents' quota and cannot act after a
  usage limit. The halting-failure class moved to 0304, which drops the
  `blocks` link.
- That the observed run's orchestrator died is inferred from the stale
  ledger, not seen in its transcript.
- Broadened from usage limits to any interruption because the recovery
  mechanism cannot tell causes apart.
- Line references updated: 0303 has landed in `SKILL.md`, shifting them.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/147
- Claude Code error reference: https://code.claude.com/docs/en/errors
- Related: 0121, 0283, 0303, 0304, 0307, 0308
