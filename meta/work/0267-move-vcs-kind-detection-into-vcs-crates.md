---
type: "work-item"
id: "0267"
title: "Move VCS Kind Detection Into The VCS Crates"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0276"
tags: ["vcs", "crates", "refactor"]
last_updated: "2026-09-05T00:00:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0276 (Rust CLI Consolidation and Hardening): post-migration evolution of the cli/ Rust workspace, gathered from the audit of work items numbered above 0136."
schema_version: 1
external_id: "PP-852"
---

# 0267: Move VCS Kind Detection Into The VCS Crates

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Move all VCS kind detection (git versus jj, and workspace classification)
into the `vcs*` crates, so detection lives in one domain-owned place.

## Context

Captured in the further-ideas backlog. VCS kind detection is performed in
more than one location; the `vcs*` crates should own it.

## Requirements

- All VCS kind detection is served by the `vcs*` crates.

## Acceptance Criteria

- [ ] No detection logic for VCS kind exists outside the `vcs*` crates.
- [ ] Callers obtain the VCS kind via those crates.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
