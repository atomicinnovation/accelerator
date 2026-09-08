---
type: "work-item"
id: "0282"
title: "Tunable Depth and Breadth"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0283"]
tags: ["research", "skills", "config"]
last_updated: "2026-09-08T11:42:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-866"
---

# 0282: Tunable Depth and Breadth

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Control `breadth` and `depth` via config (project + user) and per-invocation
override flags. `breadth` is the hard ceiling on focus areas a round may
commission (bounding `outline`); `depth` is the recursion limit within one
finding (bounding `conduct`), defaulting to 1. This knob child is independently
shippable; the `depth > 1` recursion engine is the sibling child 0283.

## Context

This establishes the config system's first numeric tunables — today it carries
only paths, templates, agents and work integration. The knob is what makes the
recursion affordable to try, so it lands adjacent to and before the recursion
engine (0283).

## Requirements

- Config-schema extension for numeric `depth` / `breadth` in
  `scripts/config-defaults.sh` (the numeric-tunable precedent) + config-read
  plumbing.
- Per-invocation override flags: `--breadth` on `outline`, `--depth` on
  `conduct`, alongside the set-handle positional.
- Resolution order flag > config (project/user) > hardcoded default
  (`breadth: 8`, `depth: 1`).
- `configure help` docs for both knobs and their defaults.

## Acceptance Criteria

- [ ] `breadth` (focus areas per round) and `depth` (recursion levels within one
      finding) resolve from config and are overridable per-invocation via
      `--breadth` (on `outline`) and `--depth` (on `conduct`) (flag > config >
      hardcoded default); the hardcoded defaults `breadth: 8` and `depth: 1` are
      documented in `configure help`.

## Open Questions

- None specific to this slice.

## Dependencies

- Blocked by: 0279 (the flags wire into the round loop Slice 2 establishes —
  recorded on 0279's `blocks`).
- Blocks: 0283 (the recursion engine needs the depth knob; they land
  adjacently, knob first).

## Assumptions

- `breadth` is a hard ceiling, not a target: the effort-scaling rubric operates
  at or beneath it, and a user who wants the rubric's 10+ band raises `--breadth`
  deliberately.

## Technical Notes

- The two knobs bound different things and multiply rather than contradict:
  `breadth` bounds focus areas per round, `depth` bounds recursion levels within
  a single finding. Both numbers should be quoted together when reasoning about
  cost.

## Drafting Notes

- This knob half relaxes the epic's Slice 5 → Slice 3 dependency: config + flags
  do not token-multiply, so it needs only 0279, not the 0280 output-quality gate.
  Only the recursion engine (0283) carries the 0280 edge. Flagged for the user in
  the decomposition; revert to a 0280 dependency if that relaxation is unwanted.
- Extracted from source documents without interactive enrichment. Acceptance
  criteria, dependencies, and kind may need refinement before promoting from
  `draft` to `ready`.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 5, knob half)
- Parent epic: 0121
