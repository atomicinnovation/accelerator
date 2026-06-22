---
type: work-item
id: "0132"
title: "Incorporate graphify with a locator-style agent"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: spike
priority: medium
source: "note:2026-06-22-ideas-backlog"
tags: []
last_updated: "2026-06-22T23:41:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0132: Incorporate graphify with a locator-style agent

**Kind**: Spike
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Investigate incorporating graphify together with an agent similar to the
codebase-locator, to explore graph-driven discovery over the knowledge base.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Relates to the graph-based knowledge
base representation epic.

## Requirements

- Research question: can graphify be combined with a locator-style agent to
  navigate/query the artefact graph effectively?
- Determine feasibility, integration shape, and any prerequisites.

## Acceptance Criteria

- [ ] A spike write-up exists documenting feasibility, a recommended approach
      (or a decision not to proceed), and any follow-on work items.

## Open Questions

- What is "graphify" in this context — an existing library, tool, or internal
  concept?
- What would the locator-style agent locate within the graph?

## Dependencies

- Blocked by: None identified yet.
- Blocks: May inform the graph-based knowledge base epic.

## Assumptions

- "graphify" refers to a graph-construction capability over the knowledge base.

## Technical Notes

- Locator agents (find, no Read) are deliberately separated from analyser agents
  in this codebase; the spike should respect that pattern.

## Drafting Notes

- Kept as a spike (the source line says "Spike ...") with an explicit research
  question; time-box to be set during refinement.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
