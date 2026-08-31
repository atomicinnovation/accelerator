---
type: "work-item"
id: "0273"
title: "Domain Crate For Linear And Jira Subcommands"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0276"
tags: ["jira", "linear", "crates", "refactor"]
last_updated: "2026-09-05T00:00:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0276 (Rust CLI Consolidation and Hardening): post-migration evolution of the cli/ Rust workspace, gathered from the audit of work items numbered above 0136."
schema_version: 1
external_id: "PP-858"
---

# 0273: Domain Crate For Linear And Jira Subcommands

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Introduce a domain crate for the `linear` and `jira` subcommands, moving
their logic out of the command layer into a reusable, testable crate.

## Context

Captured in the further-ideas backlog. The tracker subcommands hold domain
logic in the command surface rather than a dedicated crate.

## Requirements

- Extract `linear`/`jira` subcommand logic into a domain crate.
- Have the subcommands delegate to it.

## Acceptance Criteria

- [ ] The `linear` and `jira` subcommands delegate to a domain crate.
- [ ] The domain crate is independently tested.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
