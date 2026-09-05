---
type: "work-item"
id: "0231"
title: "Fix Inline Code Font Size In Headings"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "bug"
priority: "medium"
tags: ["visualiser", "styling"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-761"
---

# 0231: Fix Inline Code Font Size In Headings

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Inline code spans rendered inside headings use the wrong font size, so
monospaced fragments in a heading look out of proportion with the
surrounding heading text.

## Context

Captured in the further-ideas backlog. The visualiser renders Markdown
headings that may contain inline `code` spans; the code font size does not
scale to the heading level.

## Requirements

- Inline code within a heading should render at a size proportional to the
  heading it sits in, not the body default.

## Acceptance Criteria

- [ ] Inline code in a heading of any level renders at a size that matches
  the heading's text size.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
