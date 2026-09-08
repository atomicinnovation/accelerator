---
type: "work-item"
id: "0283"
title: "Recursive Finding Deepening"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
tags: ["research", "skills", "deep-research"]
last_updated: "2026-09-08T11:42:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-867"
---

# 0283: Recursive Finding Deepening

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

The `depth > 1` recursion engine: a single finding drills itself recursively
without human intervention between levels, narrowing by halving at each descent
and stopping at the configured depth. This is the epic's acknowledged descope
candidate — droppable if the epic runs long, keeping the knob (0282).

## Context

`depth > 1` multiplies spend, so it lands only after the output-quality gate
(0280) validates the premise and after the knob (0282) makes the ceiling
affordable. A finding's recursion is governed by `depth` and does not consume the
round's `breadth` budget, which continues to bound focus areas only.

## Requirements

- A recursion engine within `conduct`: when `depth > 1`, a finding drills itself
  recursively, narrowing per level (the dzhng halving), stopping at the
  configured depth.
- No human intervention between levels.
- A finding's recursion does not consume the round's `breadth` budget.

## Acceptance Criteria

- [ ] At `depth: 1` no recursion occurs within a finding.
- [ ] At `depth > 1` a finding drills itself recursively without human
      intervention between levels, narrowing by halving at each descent and
      stopping at the configured depth; a finding's recursion does not consume
      the round's `breadth` budget, which continues to bound focus areas only.

## Open Questions

- None specific to this slice.

## Dependencies

- Blocked by: 0280 (the output-quality gate must validate the premise before the
  token-multiplying recursion lands — recorded on 0280's `blocks`), 0282 (needs
  the `depth` knob — recorded on 0282's `blocks`).

## Assumptions

- Keeping the default `depth: 1` makes this expensive recursion strictly opt-in.

## Technical Notes

- The worst case per round is `breadth` focus areas each expanded to at most the
  `depth`-bounded halving series; recursion happens inside a finding, not
  alongside it.

## Drafting Notes

- This is the acknowledged descope candidate. If the epic runs long, drop this
  child and keep the knob (0282) rather than deferring both.
- Extracted from source documents without interactive enrichment. Acceptance
  criteria, dependencies, and kind may need refinement before promoting from
  `draft` to `ready`.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 5, recursion half)
- Parent epic: 0121
