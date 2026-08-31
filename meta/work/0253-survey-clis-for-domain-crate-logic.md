---
type: work-item
id: "0253"
title: "Survey CLIs For Domain-Crate Logic"
date: "2026-08-31T12:11:13+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: task
priority: medium
tags: [cli, crates, refactor]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0253: Survey CLIs For Domain-Crate Logic

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Survey all CLIs to identify logic currently living in the command layer that
should be pushed down into the domain crates.

## Context

Captured in the further-ideas backlog. CLIs accreted domain logic that
belongs in reusable crates; a survey scopes the extraction.

## Requirements

- Review each CLI for domain logic embedded in command handlers.
- Produce a list of extraction candidates.

## Acceptance Criteria

- [ ] A catalogue of CLI-resident logic that should move into domain crates
  is produced.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.
- Investigative; likely a spike feeding follow-up extraction tasks.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
