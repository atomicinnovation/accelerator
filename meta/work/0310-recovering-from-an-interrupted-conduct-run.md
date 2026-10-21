---
type: "work-item"
id: "0310"
title: "Recovering from an Interrupted Conduct Run"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "done"
kind: "story"
priority: "medium"
parent: "work-item:0121"
relates_to: ["work-item:0283", "work-item:0303", "work-item:0304", "work-item:0307", "work-item:0308"]
tags: ["research", "deep-research", "conduct", "recovery"]
last_updated: "2026-10-13T17:29:58+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-896"
---

# 0310: Recovering from an Interrupted Conduct Run

**Kind**: Story
**Status**: Done
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

Despite its `rate_limit` type, this is an account-wide usage limit on Claude
itself, not the failure-reason table's `rate_limited`, which a researcher
reports when a source such as OpenAlex or arXiv throttles its fetches.

The run ledger had recorded all 10 compose spawns as offered, so the same
run would never offer them again. Re-invoking `conduct` after the reset ran
`outstanding --start`, which warned:

```text
replaced the ledger of run {run}; if that run is still going it stops at its next batch, and notes its last batch writes may show as unexpected
```

and offered only the 10 compose spawns. Recovery worked, but only by
accident of the ledger's design: nothing told the user what had been lost,
and the warning reads as a concurrency hazard, not an interruption.

The interrupted run never reached step 5 (check each spawn), step 6 (loop)
or step 7 (`end-run`, tick the outline, update the manifest, summarise).
Session, weekly and spend limits are account-wide, and the orchestrator
draws on the same quota as its subagents, so its next model turn after the
batch returned was refused too. A halt-and-summarise rule inside the
interrupted run cannot help here; it applies only to failures the
orchestrator survives, which 0304 owns as its halting-failure class.

The next run never sees the dead run's errors, so it cannot tell a usage
limit from a crash or a closed terminal; the user can, from the earlier
session's output. 0304 owns the usage-limit recovery wording, and
recognises the limit only when the orchestrator survives it.

Much of the recovery already works:

- **Replaced ledger**: `outstanding --start` still holds the replaced ledger
  when it warns, including its last offered batch as spawn refs. Each spawn
  — a pair (one focus area and profile, answered by one finding) or a node
  (one level note in a pair's research tree) — has one output path derived
  from its ref.
- **Writes present at start**: the new ledger treats everything on disk as
  already seen, so these never show as unexpected. Only writes landing
  after the start, from agents of the earlier run still in flight, do.
- **Valid unchecked findings**: `outstanding` validates every finding on
  disk with the same structural pipeline as step 5 and counts those that
  pass.
- **Invalid unchecked findings**: their pair is re-planned at a new index,
  but the file stays visible. Step 3 (Clear) only quarantines a file at an
  offered path, so the stray file is never renamed and still counts towards
  the manifest's `finding_count`.
- **Invalid leftover notes**: the plan marks their node `rejected`, and
  step 3 (Clear) quarantines them when the node is offered again.
- **Outline and manifest**: step 7 ticks the outline and derives
  `round_count` and `finding_count` from disk.

## Requirements

- **Interrupted-run report**: when `outstanding --start` replaces another
  run's readable ledger, the plan carries that run's id and its last
  offered batch, each spawn classed by its one output file as written and
  valid, written but invalid, not written, or unasked. A spawn is unasked
  when no outline pair owns its stem any more (its question or profile
  left the outline); it is left in place, whatever it wrote, unless it
  left a finding that fails validation or answers a pair the outline still
  asks, which is written but invalid and quarantined, though not offered
  again. A pair's finding is valid when it answers its
  outline pair, the test step 3 (Clear) applies to a finding that does not
  complete its pair, and invalid otherwise, whether it fails validation or
  names another question or profile; a node's note is valid when the plan
  accepts it and invalid when the plan rejects it, so the node carries
  `rejected`. This data replaces
  today's "replaced the ledger of run" warning. The skill shows the report,
  one group per class, before the first batch, opening "run X did not end;
  if it is still going, it stops at its next batch, and what it writes from
  now on may show as unexpected", never asserting that the earlier run
  stopped. Every readable ledger replaced yields the report, including one
  whose last batch offered nothing, which shows the opening sentence and no
  groups. No report appears when there was no ledger to replace; a corrupt
  ledger keeps today's "replaced a corrupt run ledger" warning and yields
  no report.
- **Invalid leftover findings**: `outstanding --start` lists each finding
  the report classes as written but invalid as a path to quarantine, and
  `conduct` renames it to a `.invalid` marker straight after the report,
  before the first batch spawns, as step 5 would have done, so it no longer counts towards
  `finding_count`. Its pair is offered at a fresh index, as the planner
  already does for an invalid finding; the marker keeps the old index
  reserved.
- **Recovery guidance**: the failure-reason table gains an "unfinished run"
  row whose next step is to re-run `conduct <slug>`, which offers what the
  earlier run left unfinished, and, when the user saw an agent usage limit
  in the earlier session, to follow 0304's agent-usage-limit recovery
  first. The row does not repeat 0304's wait-or-raise detail; if this item
  lands before 0304, the row carries that step inline — re-run after the
  reset time in the message, or after raising the limit when it gives none
  — until 0304 replaces it with a reference.

## Acceptance Criteria

- [ ] Given a stale ledger whose last batch offered N spawns, pairs and
      nodes mixed, K of which left a valid output file, J an invalid one
      (including at least one rejected note) and the rest nothing, when
      `conduct` starts, then before the first batch it names the earlier
      run, lists the K spawns as written by that run, the J as written but
      invalid and offered again, marking each invalid finding quarantined,
      and the other N − K − J as not written and offered again. A spawn
      whose question left the outline is listed apart, as left in place.
- [ ] Given the same stale ledger, when `outstanding --start` plans, then
      its JSON carries the earlier run's id and the N spawn refs split into
      written and valid, written but invalid, not written, and unasked, and
      carries no "replaced the ledger of run" warning.
- [ ] Given a stale ledger, when `conduct` reports it, then the report
      opens with the sentence the requirement fixes, and makes no
      unconditional claim that the earlier run stopped, crashed, died or
      was interrupted; the conditional "if it is still going, it stops at
      its next batch" is allowed.
- [ ] Given a stale ledger whose last batch offered no spawns, when
      `conduct` starts, then before the first batch it shows the report's
      opening sentence naming the earlier run, and lists no spawns.
- [ ] Given no ledger to replace, because none existed or the previous run
      ended with `end-run`, when `conduct` starts, then no interrupted-run
      report appears.
- [ ] Given a corrupt run ledger, when `conduct` starts, then the plan
      carries no interrupted-run data, the skill shows the corrupt-ledger
      warning, and no interrupted-run report appears.
- [ ] Given two findings from the replaced ledger's last batch that do not
      answer their outline pairs, one failing validation and one valid but
      naming another question, when the new run starts, then `conduct`
      renames each to a `.invalid` marker before the first batch spawns, each pair
      is offered at a fresh index, and step 7's `finding_count` excludes
      both.
- [ ] Given a finding from the replaced ledger's last batch that answers
      its outline pair, when the new run plans, then its pair counts as answered
      and is not offered again.
- [ ] Given a stale ledger and on-disk output for part of its last batch,
      when the new run reaches step 7, then the outline checkboxes and the
      manifest's `round_count` and `finding_count` match what is on disk
      (guards existing step 7 behaviour).
- [ ] The failure-reason table has an "unfinished run" row whose next step
      says to re-run `conduct <slug>` and refers to 0304's agent-usage-limit
      recovery, or carries its wait-or-raise step inline when 0304 has not
      landed, distinct from `rate_limited`, the concurrent-subagent-limit
      row and any halting-reason rows 0304 has added.

## Dependencies

- Blocked by: none known
- Blocks: none
- Relates to:
  - 0304 owns the halting-failure class and the agent-usage-limit recovery
    wording that the new row refers to. Either may land first; the second
    reconciles the failure-reason table, and if this item lands first,
    0304 replaces the row's inline wait-or-raise step with a reference to
    its own row.
  - 0303, landed, whose concurrent-subagent-limit row sits beside the new
    row.
  - 0307 adds snapshot fields to the run ledger and refuses its node query
    for a superseded run. This item reads only the replaced ledger's
    pending spawn refs, which 0307 leaves unchanged, so neither waits on
    the other. If recovery later needs the ledger to retain more, that
    extension is this item's.
  - 0308 confines writes to the active run's offered batch. A replaced
    run's agents still in flight may write after the new run starts; 0308
    decides whether those writes are refused and, if it allows them, owns
    reporting them as the earlier run's writes. Until then they show under
    Unexpected notes, as the replaced-ledger warning says.
  - 0283, done, which introduced the run ledger, offered spawns and
    `outstanding --start`.

## Assumptions

- Session, weekly and spend limits apply to the whole account, so the
  orchestrator cannot take another turn once a subagent hits one.
- A run that dies usually leaves its ledger intact, since the plan writes
  the ledger before the batch it offers is spawned.

## Technical Notes

- Replaced ledger read and warning:
  `cli/research-cli/src/topic_command.rs:364-383`; new ledger: `:273`;
  `end-run`: `:319-348`.
- Pending batch: `cli/research/src/conduct/memory.rs:11-47`; spawn refs:
  `cli/research/src/conduct/spawn.rs:9-37`; output paths:
  `cli/research/src/topic/layout/stem.rs:66-68`,
  `cli/research/src/topic/layout/note_ref.rs:18-20`.
- New ledger seeds its seen set from disk:
  `cli/research/src/conduct/ledger.rs:56-67`; unexpected detection:
  `cli/research/src/conduct/window.rs:149-170`.
- `outstanding` finding validation:
  `cli/research-adapters/src/topic.rs:222-248`; index claiming:
  `cli/research/src/topic/plan.rs:279-309`; an invalid finding's pair
  re-planned at a new index: `:570-582`; notes judged:
  `cli/research-adapters/src/topic.rs:296-306`. All quarantining is the
  skill's; the CLI only reads `.invalid` markers.
- `skills/research/research-topic/SKILL.md`: first-plan warnings
  `:262-267`; step 3 Clear and quarantine `:297-310`; step 5 `:390-402`;
  step 7 `:418-443`; Unexpected notes `:455-458`; failure-reason table
  `:474-488`; `finding_count` rule `:611-618`.

## Drafting Notes

- Reframed from a halting-failure story to next-run recovery once it was
  clear the orchestrator shares the subagents' quota and cannot act after a
  usage limit. The halting-failure class moved to 0304, which drops the
  `blocks` link.
- That the observed run's orchestrator died is inferred from the stale
  ledger, not seen in its transcript.
- Broadened from usage limits to any interruption because the recovery
  mechanism cannot tell causes apart.
- The open questions on ledger retention and unvalidated findings were
  answered by reading the ledger and `outstanding` code. The replaced
  ledger suffices; valid unchecked findings already count; step 7 already
  reconciles from disk, so its criterion is a regression check.
- "Leftover writes" dropped: writes present at start are seen, never
  unexpected. Writes landing after the start were briefly in scope as
  "late writes", then moved to 0308, which owns reporting them if it lets
  them land at all; no fixture here could pin which spawns they cover.
  "Unchecked findings" narrowed to invalid leftovers, the only case
  `outstanding` does not already handle.
- Usage-limit recovery wording deferred to 0304, so this item no longer
  depends on Claude Code's error wording.
- Quarantine of invalid leftovers stays with the skill's step 3 rather than
  moving into `outstanding --start`: the CLI never renames files, at the
  cost of a fresh index for the re-offered pair.
- The recovery row is named "unfinished run" rather than "interrupted run"
  so it never asserts a cause the next run cannot observe.
- Codebase research
  (`meta/research/codebase/2026-10-13-0310-recovering-from-an-interrupted-conduct-run.md`)
  changed two rules. A pair's finding is now judged by whether it answers
  its outline pair rather than by schema validity alone. Under the schema
  test, a valid finding naming the wrong question was reported valid, left
  counted on disk, and its pair offered again anyway. Also, a ledger whose
  last batch was empty still yields the report, because the earlier run may
  still be live and its `end-run` would then fail as superseded.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/147
- Claude Code error reference: https://code.claude.com/docs/en/errors
- Related: 0121, 0283, 0303, 0304, 0307, 0308
