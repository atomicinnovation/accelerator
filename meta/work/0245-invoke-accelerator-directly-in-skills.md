---
type: work-item
id: "0245"
title: "Invoke Accelerator Directly In Skills"
date: "2026-08-31T12:11:13+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: task
priority: medium
tags: [skills, cli]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0245: Invoke Accelerator Directly In Skills

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Have all skills invoke the `accelerator` binary directly rather than through
any intermediate wrapper, for a uniform and permission-friendly call
convention.

## Context

Captured in the further-ideas backlog. Skills vary in how they reach the
CLI; direct invocation is the preferred convention.

## Requirements

- Every skill that shells out to Accelerator invokes the binary directly.

## Acceptance Criteria

- [ ] No skill invokes Accelerator via a wrapper; all call the binary
  directly.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
