---
type: work-item
id: "0148"
title: "Prompt Skills to Update Artefact Statuses on Completion"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: medium
source: "note:2026-06-22-ideas-backlog"
tags: []
last_updated: "2026-06-22T23:41:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-169
---

# 0148: Prompt Skills to Update Artefact Statuses on Completion

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Get skills to prompt for updating artefact statuses (e.g. work item status,
review status, plan status) upon completion, so statuses stay current without
relying on the user to remember.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Artefact statuses can drift when a
skill finishes its work but leaves the artefact's status unchanged.

## Requirements

- On completion, relevant skills prompt the user to update the associated
  artefact's status.
- Cover work item status, review status, plan status, and other applicable
  artefacts.

## Acceptance Criteria

- [ ] When a skill completes work tied to an artefact, then it prompts to update
      that artefact's status to the appropriate next value.

## Open Questions

- Which skills/artefacts are in scope, and what status transitions do they
  propose?
- Should the prompt auto-apply in auto-mode?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.
- Related: auto-mode for relevant skills; normalise statuses across artefacts.

## Assumptions

- Prompting (not silent auto-update) is the default behaviour.

## Technical Notes

- None identified yet.

## Drafting Notes

- None beyond faithful restatement of the source line.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
