---
type: "work-item"
id: "0263"
title: "Rename ForeignDirt"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "done"
kind: "task"
priority: "medium"
parent: "work-item:0276"
tags: ["naming", "refactor"]
last_updated: "2026-09-23T23:00:23+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Done as part of 0241: `ForeignDirt` became `PreflightError::UnownedChanges`, carrying the unowned paths, and `Ownership::Foreign` became `Ownership::Unowned`."
schema_version: 1
external_id: "PP-793"
---

# 0263: Rename ForeignDirt

**Kind**: Task
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Rename the `ForeignDirt` type to a name without negative connotations,
better expressing its domain role.

## Context

Captured in the further-ideas backlog. The current name carries an
unhelpful negative framing.

## Requirements

- Choose a domain-appropriate replacement name and rename all usages.

## Acceptance Criteria

- [x] `ForeignDirt` is renamed and no references to the old name remain.

## Open Questions

- What is the intended replacement name? Resolved: `UnownedChanges`
  (`PreflightError::UnownedChanges`) and `Ownership::Unowned`; the domain
  speaks of owned and unowned changes.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
