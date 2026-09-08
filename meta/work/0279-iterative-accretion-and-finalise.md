---
type: "work-item"
id: "0279"
title: "Iterative Accretion and Finalise"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0282", "work-item:0284"]
tags: ["research", "skills", "deep-research"]
last_updated: "2026-09-08T11:42:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-863"
---

# 0279: Iterative Accretion and Finalise

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Grow the knowledgebase across multiple rounds without clobbering, and close a
subject off. Adds next-round outline appending, gap-detection `conduct`
re-invocation, wholesale synthesis rewrite under anti-changelog discipline, the
full five-state `research_status` lifecycle with reopen regression, and the
`finalise` verb.

## Context

Slice 1 (0277) runs the loop once; this makes it iterative and closable. No
visualiser work is required — round and focus-area structure becomes visible in
the Slice 6 detail page (0284), not before. The round loop runs at `depth: 1`;
automatic intra-finding recursion is 0283.

## Requirements

- `outline` gains next-round appending (a new `## Round N` checklist of focus
  areas) without clobbering earlier rounds.
- `conduct` gains gap-detection re-invocation: researches only outstanding focus
  areas, never rewrites existing findings, flips outline checkboxes as findings
  land (finding-existence is ground truth; the checkbox is reconciled to match).
- `synthesise` rewrites `synthesis.md` wholesale with anti-changelog discipline
  over the findings and synthesis (`outline.md` exempt — it is the working log).
- `outline`/`conduct`/`synthesise`/`finalise` keep `manifest.md`'s
  `research_status`, `round_count`, `finding_count` and `primary` in sync.
- The five-state lifecycle `briefed → outlined → researching → synthesised →
  complete`, with reopen regression to `researching` (marking the synthesis
  stale).
- `finalise` closes a subject: refuses a stale or absent synthesis, then sets
  `research_status: complete` — the only path to that value, reversible by a
  later `outline` or `conduct`.

## Acceptance Criteria

- [ ] Re-invoking `conduct` researches only outstanding focus areas, never
      rewrites existing findings, and `synthesise` rewrites `synthesis.md`
      wholesale (idempotent accretion), keeping `manifest.md`'s counts in sync
      across rounds.
- [ ] `outline` on an already-researched set appends a new `## Round N` checklist
      without clobbering earlier rounds, and returns a `synthesised` or
      `complete` set to `researching` (marking the synthesis stale).
- [ ] The findings and `synthesis.md` read as a single dossier with no
      round-by-round narration; `outline.md` is exempt and retains its
      round-grouped checklist.
- [ ] `finalise` refuses to run against a stale or absent synthesis, and
      otherwise sets `research_status: complete` — the only path to that value; a
      subsequent `outline` or `conduct` returns the set to `researching`.
- [ ] The five-state lifecycle holds: `brief`→`briefed`, `outline`→`outlined`,
      `conduct`→`researching`, `synthesise`→`synthesised`, `finalise`→`complete`,
      with reopen regression to `researching`.

## Open Questions

- Synthesis-staleness mechanism: the epic requires `finalise` to refuse a stale
  synthesis and reopen to mark it stale, but does not specify how staleness is
  tracked — derived (a finding whose `round` exceeds `synthesis.md`'s
  `rounds_covered`) or a stored flag. Resolve during planning.

## Dependencies

- Blocked by: 0277 (the single-round engine — recorded on 0277's `blocks`).
- Blocks: 0282 (the knob wires override flags into this round loop), 0284 (the
  detail page shows this slice's round structure); also gates 0281's
  gap-proposal (recorded on 0281's `blocked_by`).

## Assumptions

- Anti-changelog discipline applies to findings and synthesis only; `outline.md`
  remains the freely-rewritten working log.

## Technical Notes

- `synthesised` asserts only that a current dossier exists, not that every focus
  area is researched (interim synthesis is allowed); completeness is what
  `finalise` asserts.

## Drafting Notes

- The staleness mechanism is left as an Open Question rather than assumed — it is
  a genuine unknown the epic does not pin down.
- Runs at `depth: 1`; the recursion engine is the separate child 0283.
- Extracted from source documents without interactive enrichment. Acceptance
  criteria, dependencies, and kind may need refinement before promoting from
  `draft` to `ready`.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 2)
- Parent epic: 0121
