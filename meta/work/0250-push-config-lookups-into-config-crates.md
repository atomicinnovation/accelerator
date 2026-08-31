---
type: "work-item"
id: "0250"
title: "Push Config Lookups Into Config Crates"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
tags: ["config", "crates", "refactor"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-780"
---

# 0250: Push Config Lookups Into Config Crates

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Move all configuration lookups down into the config crates, so callers ask
the config layer rather than reading or interpreting config themselves.

## Context

Captured in the further-ideas backlog. Config lookups are scattered across
CLIs and skills instead of centralised.

## Requirements

- All config lookups go through the config crates' public API.

## Acceptance Criteria

- [ ] No caller reads or parses configuration directly; all use the config
  crates.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
