---
type: work-item
id: "0152"
title: "Collaboration (PR) Skills"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: epic
priority: medium
source: "note:2026-06-22-ideas-backlog"
tags: []
last_updated: "2026-06-22T23:41:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0152: Collaboration (PR) Skills

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add a suite of collaboration skills covering pull-request lifecycle operations,
including stacked-PR variants.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Complements the existing
describe/respond/review PR skills and the collaboration group restructuring.

## Requirements

Candidate child work items captured for decomposition during refinement:

- `/open-pr` skill.
- `/merge-pr` skill.
- `/list-prs` skill, with `--mine`.
- `/open-pr-stack` skill (or `/open-prs --stacked`, possibly with `--describe`
  to bundle describing on open).
- `/describe-pr-stack` skill (or `/describe-prs --stacked`).
- `/respond-to-pr-stack` skill (or `/respond-to-prs --stacked`).
- `/review-prs-stack` skill (or `/review-prs --stacked`).

## Acceptance Criteria

- [ ] The PR-lifecycle collaboration skills above exist and operate through the
      collaboration integration layer.

## Open Questions

- Single-PR-plus-`--stacked`-flag vs. dedicated `*-stack` skills — which command
  shape is preferred?
- Should `--describe` bundling on open be in scope for the first cut?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.
- Related: decouple collaboration skills from GitHub integration.

## Assumptions

- Built on top of the (decoupled) collaboration integration layer.

## Technical Notes

- Stacked-PR variants imply awareness of PR/branch stacks.

## Drafting Notes

- Treated as a single epic per the user's instruction, with the source
  sub-bullets (including the command-shape alternatives) captured as child
  candidates under Requirements rather than as separate work items.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
