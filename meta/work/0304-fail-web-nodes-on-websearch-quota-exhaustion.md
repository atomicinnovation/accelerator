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
relates_to: ["work-item:0282", "work-item:0283", "work-item:0309"]
tags: ["research", "deep-research", "conduct", "web-profile"]
last_updated: "2026-10-09T17:58:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
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
them to clear the session and re-run `conduct`.

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
- Quota exhausted is a halting failure (0310): when any spawn in a batch
  reports it, `conduct` spawns no further batches, tells the user the
  session's WebSearch quota is exhausted, names the failed nodes, and gives
  the recovery: run `/clear`, then re-run `conduct <slug>`. The recovery
  also mentions `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` to raise the cap.
- The `conduct` failure-reason table documents Quota exhausted and its
  recovery.

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
      with reason "web search quota exhausted", no further batch is spawned,
      and the user is told to run `/clear` and re-run `conduct <slug>`.
- [ ] Given a run stopped by quota exhaustion, when the user runs `/clear`
      and re-runs `conduct <slug>`, then the new run offers the failed nodes
      again.
- [ ] The `conduct` failure-reason table has a Quota exhausted row with its
      recovery.

## Open Questions

- What are the per-level budget numbers? With 130 nodes in the worst case,
  deeper levels get about 1 search each to fit within 200.
- Should `conduct` read `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` up front
  and warn when planned web nodes × budget exceeds the cap?

## Dependencies

- Blocked by: 0310 (halting failures in `conduct`), whose class this
  outcome joins.
- Blocks: none known
- Related: 0309, which takes the same no-fallback stance for background
  knowledge.

## Assumptions

- A researcher can recognise the session-limit notice in its WebSearch
  result. Claude Code documents no exact error string, so the profile must
  describe the notice rather than match text.
- Re-running `conduct` after `/clear` replaces the interrupted run's ledger
  and offers only outstanding spawns, as observed in the 0310 source run.

## Technical Notes

- Web profile outcomes: `skills/research/profiles/web-profile/SKILL.md:58-70`
  (Denied at 62–63).
- Budget pattern to mirror:
  `skills/research/profiles/openalex-profile/SKILL.md:39`.
- Spawn checking and failure recording:
  `skills/research/research-topic/SKILL.md:359-364`.
- Claude Code docs: `code.claude.com/docs/en/tools-reference` ("Session
  search limit") and `code.claude.com/docs/en/env-vars`.

## Drafting Notes

- The issue suggested a WebFetch fallback on exhaustion; the user rejected
  it in favour of failing the node, with `/clear` plus a re-run as the
  recovery.
- Depth-scaled budget was a user decision; the numbers are left open.
- Stopping further batches follows from the halting-failure class in 0310:
  every later web node in the same session would fail too.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/141
- Related: 0121, 0282, 0283, 0309, 0310
- External: https://code.claude.com/docs/en/tools-reference,
  https://code.claude.com/docs/en/env-vars
