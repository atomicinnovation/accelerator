---
type: "work-item"
id: "0235"
title: "Reviewer Profiles"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
tags: ["review", "config"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-820"
---

# 0235: Reviewer Profiles

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Allow reviewer profiles so reviews can be triggered with different lens and
configuration presets, letting a user pick a review flavour rather than
always running the full default lens set.

## Context

Captured in the further-ideas backlog. Review skills run a fixed lens
catalogue; different situations warrant different lens selections and
settings.

## Requirements

- Define named reviewer profiles bundling a lens selection and config
  presets.
- Let a review be triggered against a chosen profile.

## Acceptance Criteria

- [ ] A user can define a reviewer profile naming its lenses and config.
- [ ] Triggering a review with a profile applies exactly that profile's
  lenses and settings.

## Open Questions

- Where are profiles stored — team config, personal config, or both?
- Do profiles apply to all review skills or a subset?

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
