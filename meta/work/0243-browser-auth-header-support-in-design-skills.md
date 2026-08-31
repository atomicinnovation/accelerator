---
type: "work-item"
id: "0243"
title: "Browser Auth Header Support In Design Skills"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
tags: ["design", "browser"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-773"
---

# 0243: Browser Auth Header Support In Design Skills

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add full support for `ACCELERATOR_BROWSER_AUTH_HEADER` in the design
skills, so browser-driven design capture can reach pages behind an
auth-header gate.

## Context

Captured in the further-ideas backlog. The env var exists but is not fully
honoured across the design skills' browser automation.

## Requirements

- All design skills that drive a browser pass
  `ACCELERATOR_BROWSER_AUTH_HEADER` through to the browser session.

## Acceptance Criteria

- [ ] Given `ACCELERATOR_BROWSER_AUTH_HEADER` is set, design skills capture
  pages that require that header.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
