---
type: "work-item"
id: "0373"
title: "Shortcut Parent/Child Sync"
date: "2026-11-14T09:52:01+00:00"
author: "Tom Scott"
producer: "implement-plan"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0291"
relates_to: ["work-item:0371"]
tags: ["work-management", "integrations", "shortcut", "sync"]
last_updated: "2026-11-14T09:52:01+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---
# 0373: Shortcut Parent/Child Sync

## Summary

Sync a story's epic, and a sub-task's parent story, with the local
`parent:` link, as 0291 does for the other trackers.

## Context

Work item 0371 syncs a story's or an epic's title and body only. A Shortcut
story names its epic in `epic_id`, and a sub-task names its parent story.
0291 defines parent/child sync for the tracker port; this item is its
Shortcut part.

## Acceptance Criteria

- [ ] Given a local story whose `parent:` is a work item linked to
      `epic-<id>`, when it is pushed, then the Shortcut story's `epic_id`
      is that epic.
- [ ] Given a Shortcut story in an epic, when it is pulled, then the local
      item's `parent:` names the epic's work item.
- [ ] A parent that has no local work item is reported, not dropped.
