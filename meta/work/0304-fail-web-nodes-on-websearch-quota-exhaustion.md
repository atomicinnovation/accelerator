---
type: "work-item"
id: "0304"
title: "Fail Web Nodes When the WebSearch Quota Runs Out"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
relates_to: ["work-item:0282", "work-item:0283", "work-item:0303", "work-item:0309", "work-item:0310"]
tags: ["research", "deep-research", "conduct", "web-profile"]
last_updated: "2026-10-12T22:28:34+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-890"
---

# 0304: Fail Web Nodes When the WebSearch Quota Runs Out

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Deep `conduct` runs exhaust Claude Code's session-wide WebSearch quota, after
which web researchers silently fall back to WebFetch of URLs they already
know, so coverage depends on scheduling order rather than the question. Give
the web profile a depth-scaled per-node search budget to make exhaustion
rarer, and when it does happen, fail the node, tell the user, and direct
them to clear the session and re-run `conduct`. Introduce a halting-failure
class in `conduct` — failures after which every further spawn in the session
would fail too — with quota exhaustion as its first member: `conduct` stops
spawning, ends the run cleanly, and prints the recovery.

## Context

A `--depth 3` run with 8 web pairs and 2 openalex pairs (130 nodes) exhausted
the quota around node 25–35; the ~90 web nodes after that worked only from
WebFetch of known primary sources. Discovery collapsed: later notes skewed
to well-known docs and missed community writing, issue threads and newer
material.

The quota is documented: 200 `WebSearch` calls per interactive session,
shared by the main conversation and every subagent (Claude Code v2.1.212+).
`/clear` resets it; `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` raises it;
from v2.1.290 it refills at ~100 calls per hour. At the cap there is no
error — further calls do nothing and the model is told to continue with
what it has. So the web profile's Denied outcome does not fire, and nothing
reaches the orchestrator.

Falling back to known sources or background knowledge is not acceptable: a
note researched without search is lower quality and indistinguishable from a
searched one. Failing the node leaves it outstanding so a later run in a
fresh session researches it properly.

Quota exhaustion leaves the orchestrator running: it makes no `WebSearch`
calls of its own, so it can stop further batches, run step 7 and summarise.
That distinguishes it from an account-wide agent usage limit, which also
refuses the orchestrator's next turn; recovery from that is 0310's, on the
next run. The halting-failure class therefore covers only failures the
orchestrator survives. A model-specific agent usage limit (`You've hit your
Opus limit`, `… Sonnet limit`) that the orchestrator observes is one: the
orchestrator, alive to see it, is not on that model's quota, but every
further spawn on it would fail.

## Requirements

- The web profile sets a per-node `WebSearch` budget that decreases with
  depth, in the same form as the openalex profile's call cap. Level-1 nodes,
  which map the field, get the largest budget.
- The web profile adds a **Quota exhausted** outcome: when a `WebSearch` call
  returns Claude Code's session-limit notice, the researcher writes no file
  — even if it already gathered pages — and its summary says "web search
  quota exhausted". It never completes the note from WebFetch of known URLs
  or from memory.
- `conduct` records a Quota exhausted node as failed with that reason, and
  the node stays outstanding.
- **Halting-failure class**: `conduct` defines, in one place, the failure
  reasons after which every further spawn in this session would also fail,
  each with its recovery. Its members are "web search quota exhausted" and
  "agent usage limit" — a spawn that ended `Agent terminated early due to an
  API error: You've hit your …`, recorded with the message's reset time when
  it gives one.
- **Halting**: after step 5, if any spawn in the batch failed with a halting
  reason, `conduct` spawns no further batches. It still runs step 7 in full —
  `end-run`, tick from disk, manifest update — so the re-run starts with no
  stale ledger.
- **Retry rounds**: once any spawn accepted by the batch message or a
  capacity-refusal round ends with a halting failure, no further
  capacity-refusal round runs. Spawns still capacity-refused keep the
  concurrent-subagent-limit reason.
- **Summary**: a halted run's summary opens with a halt notice, above
  Failures, naming the halting reason, the affected spawns, and that
  reason's recovery. The affected spawns are also listed under Failures.
- **Recoveries**: for quota exhausted, run `/clear`, then re-run `conduct
  <slug>`, mentioning `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` to raise
  the cap. For an agent usage limit, re-run `conduct <slug>` after the reset
  time in the message, or after raising the limit when it gives none.
- The `conduct` failure-reason table has a row for each halting reason with
  its recovery, distinct from `rate_limited`.

## Acceptance Criteria

- [ ] The web profile states a maximum `WebSearch` count per node for each
      depth level, non-increasing with depth.
- [ ] Given the default depth-scaled budget, a depth-3 run of 10 web pairs
      (130 nodes) spends at most 200 `WebSearch` calls in the worst case.
- [ ] Given a web researcher whose `WebSearch` returns the session-limit
      notice, when it finishes, then it has written no file and its summary
      says "web search quota exhausted".
- [ ] Given a batch in which one or more spawns report quota exhaustion, when
      `conduct` checks the batch, then those nodes are recorded as failed
      with reason "web search quota exhausted" and no further batch is
      spawned.
- [ ] Given a batch in which a spawn ended `Agent terminated early due to an
      API error: You've hit your Opus limit · resets 3:45pm`, when `conduct`
      checks the batch, then the spawn is recorded as failed with reason
      "agent usage limit" and the reset time, and no further batch is
      spawned.
- [ ] Given a batch where other spawns succeeded alongside a halting failure,
      when `conduct` checks the batch, then the successful spawns are
      retained and validated as normal.
- [ ] Given a run stopped by a halting failure, when `conduct` ends, then
      `end-run` has removed the run's ledger, the outline checkboxes and
      manifest counts reflect disk, and the summary opens with the halt
      notice naming the reason, the affected spawns and the recovery.
- [ ] Given a batch message with capacity-refused spawns and a spawn that
      ended with a halting failure, when `conduct` handles refusals, then no
      capacity-refusal round runs and the refused spawns are recorded with
      the concurrent-subagent-limit reason.
- [ ] Given a run stopped by quota exhaustion, when the user runs `/clear`
      and re-runs `conduct <slug>`, then the new run reports no replaced
      ledger and offers the failed nodes again.
- [ ] The skill defines halting failures in one place, listing their member
      reasons, and the failure-reason table has a row for each.

## Open Questions

- What are the per-level budget numbers? With 130 nodes in the worst case,
  deeper levels get about 1 search each to fit within 200.
- Should `conduct` read `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` up front
  and warn when planned web nodes × budget exceeds the cap?

## Dependencies

- Blocked by: none known
- Blocks: none known
- Related: 0310, which recovers on the next run from interruptions the
  orchestrator does not survive. Either may land first; the second
  reconciles the failure-reason table. If 0310 lands first, its
  "unfinished run" row carries the agent-usage-limit wait-or-raise step
  inline, and this item replaces it with a reference to its own row.
- Related: 0303, landed, whose capacity-refusal rounds this item stops
  early; 0309, which takes the same no-fallback stance for background
  knowledge.

## Assumptions

- A researcher can recognise the session-limit notice in its WebSearch
  result. Claude Code documents no exact error string, so the profile must
  describe the notice rather than match text.
- An orchestrator that observes a `You've hit your …` termination survived
  it, so the limit is model-specific; every further spawn on that model
  would fail until the reset.

## Technical Notes

- Web profile outcomes: `skills/research/profiles/web-profile/SKILL.md:57-70`
  (Denied at 62).
- Budget pattern to mirror:
  `skills/research/profiles/openalex-profile/SKILL.md:39`.
- Capacity-refusal rounds: `skills/research/research-topic/SKILL.md:366-387`.
- Spawn checks (step 5): `skills/research/research-topic/SKILL.md:390-401`.
- Step 7 and its summary order: `skills/research/research-topic/SKILL.md:418-469`.
- Failure-reason table: `skills/research/research-topic/SKILL.md:474-488`.
- Claude Code docs: `code.claude.com/docs/en/tools-reference` ("Session
  search limit"), `code.claude.com/docs/en/env-vars`, and
  `code.claude.com/docs/en/errors` (subagent termination wording).

## Drafting Notes

- The issue suggested a WebFetch fallback on exhaustion; the user rejected
  it in favour of failing the node, with `/clear` plus a re-run as the
  recovery.
- Depth-scaled budget was a user decision; the numbers are left open.
- The halting-failure class moved here from 0310 once it was clear an
  account-wide usage limit also stops the orchestrator, so halting works
  only for failures the orchestrator survives. This drops the dependency on
  0310.
- Adding "agent usage limit" as a second member is my call: it costs one
  reason and one recovery, and gives the class a second member to keep its
  definition honest. Drop it if the scope should stay on the web profile.
- Running step 7 after a halt, stopping capacity-refusal rounds early, and
  placing the halt notice above Failures were agreed while reworking 0310.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/141
- Related: 0121, 0282, 0283, 0303, 0309, 0310
- External: https://code.claude.com/docs/en/tools-reference,
  https://code.claude.com/docs/en/env-vars,
  https://code.claude.com/docs/en/errors
