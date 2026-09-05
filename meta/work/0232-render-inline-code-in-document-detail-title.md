---
type: "work-item"
id: "0232"
title: "Render Inline Code In Document Detail Page Title"
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
external_id: "PP-817"
---

# 0232: Render Inline Code In Document Detail Page Title

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The document detail page title shows inline code markup as literal
backticks instead of rendering it as a code span.

## Context

Captured in the further-ideas backlog. Titles containing inline code (e.g.
a title naming a `command`) are displayed raw on the detail page in the
visualiser.

## Requirements

- The document detail page title should render inline code spans rather
  than showing literal backtick syntax.

## Acceptance Criteria

- [ ] A document whose title contains inline code renders that fragment as
  a code span on the detail page.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
