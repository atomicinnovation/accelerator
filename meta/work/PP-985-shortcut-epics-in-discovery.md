---
type: "work-item"
id: "PP-985"
external_id: "PP-985"
aliases: ["0394", "draft-eqdpcp"]
title: "Shortcut Epics in Discovery"
date: "2026-11-14T09:52:01+00:00"
author: "Tom Scott"
producer: "implement-plan"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:PP-701"
blocked_by: ["work-item:PP-876"]
relates_to: ["work-item:PP-990"]
tags: ["work-management", "integrations", "shortcut", "sync"]
last_updated: "2026-11-14T09:52:01+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---
# PP-985: Shortcut Epics in Discovery

## Summary

Discovery finds untracked Shortcut stories only. This item lets it find
untracked epics too, so a pull can create local epic work items.

## Context

Work item PP-990 searches stories by team through `/stories/search`. Epics
are not owned by one team in the same way, so discovering them needs its own
scope rule. PP-876's field mapping decides how an epic's fields map to a local
item, so this item waits for it.

## Acceptance Criteria

- [ ] Given an epic created in Shortcut, when a bidirectional sync runs,
      then it is reported as an untracked remote with key `epic-<id>`.
- [ ] Epic discovery honours the configured team scope, or documents why it
      cannot.
