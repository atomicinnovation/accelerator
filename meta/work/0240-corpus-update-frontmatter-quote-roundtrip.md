---
type: "work-item"
id: "0240"
title: "Corpus Update Frontmatter Quote Roundtrip"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "bug"
priority: "medium"
tags: ["corpus", "frontmatter"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0240: Corpus Update Frontmatter Quote Roundtrip

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`accelerator corpus update` does not preserve quoting on frontmatter values
across a roundtrip, producing output the model then rejects.

## Context

Captured in the further-ideas backlog. Related to the canonical frontmatter
quoting work; the update path re-emits frontmatter without honouring the
canonical quoting standard.

## Requirements

- `accelerator corpus update` must roundtrip frontmatter values with their
  quoting intact, matching the canonical quoting standard.

## Acceptance Criteria

- [ ] Given a document with quoted frontmatter values, when
  `accelerator corpus update` runs, then quoting is preserved and the result
  passes validation.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
