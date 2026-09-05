---
type: "work-item"
id: "0234"
title: "Resolvable Corpus Cross-References"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
tags: ["corpus", "documentation"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-819"
---

# 0234: Resolvable Corpus Cross-References

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Improve corpus cross-references used in code comments and documentation by
either banning them outright or making them fully resolvable, so references
never go stale or dangle.

## Context

Captured in the further-ideas backlog. Cross-references to corpus artefacts
(ADRs, work items, plans) in comments and docs can rot as artefacts move or
are renumbered.

## Requirements

- Decide a policy: ban corpus cross-references in comments/docs, or make
  every such reference resolvable and validated.
- If resolvable: provide a mechanism to resolve and verify references.

## Acceptance Criteria

- [ ] A policy is chosen and enforced consistently.
- [ ] If references are permitted, an unresolvable reference is detected by
  tooling.

## Open Questions

- Ban versus resolve — which policy do we adopt?
- Note: existing guidance discourages stale corpus references in comments;
  reconcile with that.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
