---
type: "work-item"
id: "0269"
title: "Remove Bash References From Jira And Linear Clients"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
tags: ["jira", "linear", "cleanup"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-799"
---

# 0269: Remove Bash References From Jira And Linear Clients

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Remove all references to Bash and shell exit codes from the Jira and Linear
client crates, left over from the shell-to-Rust migration.

## Context

Captured in the further-ideas backlog. The client crates still carry Bash-
and exit-code-oriented vocabulary that no longer fits the Rust
implementation.

## Requirements

- Remove Bash and exit-code references from the Jira and Linear client
  crates, replacing them with idiomatic Rust error handling.

## Acceptance Criteria

- [ ] The Jira and Linear client crates contain no Bash or exit-code
  references.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
