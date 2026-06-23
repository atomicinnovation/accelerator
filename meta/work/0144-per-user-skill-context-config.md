---
type: work-item
id: "0144"
title: "Per-User Skill Context and Instructions"
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
external_id: PP-165
---

# 0144: Per-User Skill Context and Instructions

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add per-user configuration of additional skill context and instructions, so
individuals can supplement skills with their own context without changing team
config.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). The plugin already supports team and
personal config plus per-skill customisations; this extends per-user control
over additional context and instructions.

## Requirements

- Allow per-user configuration that injects additional context / instructions
  into skills.
- Define precedence relative to team and per-skill configuration.

## Acceptance Criteria

- [ ] A user can configure additional per-skill context / instructions that are
      applied at skill invocation, layered over team configuration.

## Open Questions

- How does this differ from / extend the existing per-skill userspace
  customisation directory?
- What is the precedence order across team, personal, and per-user layers?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.

## Assumptions

- Builds on the existing configuration extension points rather than a new
  mechanism.

## Technical Notes

- Relates to the configuration system architecture and per-skill customisation
  directory.

## Drafting Notes

- None beyond faithful restatement of the source line; overlap with existing
  per-skill userspace customisation is flagged as an open question.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
