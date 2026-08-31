---
type: "work-item"
id: "0237"
title: "Mark Migrated To Latest On First Init"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
tags: ["migration", "init"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-767"
---

# 0237: Mark Migrated To Latest On First Init

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

When a repo is first initialised, record it as already migrated to the
latest migration, so a fresh repo does not appear to have pending
migrations.

## Context

Captured in the further-ideas backlog. A newly initialised repo starts at
the current schema; without stamping the latest migration, migration
tooling may report or attempt to apply migrations that do not apply.

## Requirements

- First init stamps the repo at the latest available migration.

## Acceptance Criteria

- [ ] Given a freshly initialised repo, `accelerator migrate` reports no
  pending migrations.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
