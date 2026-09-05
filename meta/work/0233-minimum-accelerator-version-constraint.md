---
type: "work-item"
id: "0233"
title: "Minimum Accelerator Version Constraint"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
tags: ["versioning", "config"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-818"
---

# 0233: Minimum Accelerator Version Constraint

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Let a repository declare a minimum Accelerator version and alert the user
when their installed plugin is older than that constraint, prompting an
upgrade.

## Context

Captured in the further-ideas backlog. Repos evolve to depend on newer
plugin behaviour; running an older plugin against them can fail silently or
misbehave.

## Requirements

- A repository can declare a minimum required Accelerator version.
- When the installed version is below the declared minimum, the user is
  alerted and told to upgrade.

## Acceptance Criteria

- [ ] Given a repo declaring a minimum version above the installed version,
  when Accelerator runs, then the user sees an upgrade alert.
- [ ] Given a repo whose minimum version is satisfied, no alert is shown.

## Open Questions

- Where does the constraint live — team config, a dedicated file, or plugin
  metadata?
- Is the alert advisory only, or does it block skills from running?

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
