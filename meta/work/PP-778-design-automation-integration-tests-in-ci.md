---
type: "work-item"
id: "PP-778"
title: "Design-Automation Integration Tests In CI"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "done"
kind: "task"
priority: "medium"
parent: "work-item:PP-860"
tags: ["design", "ci", "testing"]
last_updated: "2026-11-22T01:00:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Done in PR #253: test:integration:design-automation joined the test:integration roll-up, so it runs and gates on both CI legs."
schema_version: 1
external_id: "PP-778"
aliases: ["0248"]
---

# PP-778: Design-Automation Integration Tests In CI

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Get the design-automation integration tests running in CI so the design
skills' browser automation is covered on every build.

## Context

Captured in the further-ideas backlog. These tests exist but are not run by
any CI job, leaving a coverage gap.

## Requirements

- Wire the design-automation integration tests into a CI lane.

## Acceptance Criteria

- [x] The design-automation integration tests run in CI and gate the build.
      *As built (PR #253):* the lane is in the `test:integration` roll-up;
      CI's integration job ran it on Ubuntu and macOS, 38 of 38 passing.

## Open Questions

- Does CI have the browser/Chromium prerequisites these tests need?

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
