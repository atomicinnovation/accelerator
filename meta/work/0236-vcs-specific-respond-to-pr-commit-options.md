---
type: work-item
id: "0236"
title: "VCS-Specific Respond-to-PR Commit Options"
date: "2026-08-31T12:11:13+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: medium
tags: [pr, vcs]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0236: VCS-Specific Respond-to-PR Commit Options

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Make the commit options offered by `/respond-to-pr` specific to the active
VCS, so the choices presented match what git or jj actually supports.

## Context

Captured in the further-ideas backlog. `/respond-to-pr` currently offers
commit options that do not map cleanly onto every VCS (e.g. staging
concepts absent under jj).

## Requirements

- The commit options presented by `/respond-to-pr` adapt to the detected
  VCS.

## Acceptance Criteria

- [ ] Under git, `/respond-to-pr` offers git-appropriate commit options.
- [ ] Under jj, it offers jj-appropriate commit options with no
  git-only concepts.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
