---
type: "work-item"
id: "0258"
title: "Help Should Show Subcommands"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "bug"
priority: "medium"
parent: "work-item:0276"
tags: ["cli", "help"]
last_updated: "2026-09-05T00:00:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0276 (Rust CLI Consolidation and Hardening): post-migration evolution of the cli/ Rust workspace, gathered from the audit of work items numbered above 0136."
schema_version: 1
external_id: "PP-843"
---

# 0258: Help Should Show Subcommands

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`accelerator help` does not list available subcommands, leaving users
without a discoverable command index.

## Context

Captured in the further-ideas backlog. Help output omits the subcommand
list.

## Requirements

- `accelerator help` lists the available subcommands.

## Acceptance Criteria

- [ ] Running `accelerator help` shows all top-level subcommands.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
