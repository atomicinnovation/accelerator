---
type: work-item
id: "0150"
title: "Rename GitHub Skill Group to Collaboration"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: task
priority: medium
source: "note:2026-06-22-ideas-backlog"
tags: []
last_updated: "2026-06-22T23:41:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-171
---

# 0150: Rename GitHub Skill Group to Collaboration

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Rename the GitHub skill group to "collaboration", reflecting that the group is
about collaboration workflows rather than a specific provider.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Precursor to decoupling
GitHub-specific behaviour from the collaboration skills.

## Requirements

- Rename the `github` skill group/category to `collaboration`.
- Update `plugin.json` registration, paths, and any references.

## Acceptance Criteria

- [ ] The skill group is renamed to `collaboration` and all registrations /
      references are updated, with skills still resolving correctly.

## Open Questions

- Are there external references (docs, config) that also need updating?

## Dependencies

- Blocked by: None identified yet.
- Blocks: Decouple collaboration skills from GitHub integration.

## Assumptions

- This is a rename/reorganisation task with no behavioural change.

## Technical Notes

- Relates to plugin organisation and the skill category structure.

## Drafting Notes

- Kept as a task (mechanical rename/reorganisation) rather than a story.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
