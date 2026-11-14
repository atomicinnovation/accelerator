---
type: "work-item"
id: "0375"
title: "Shortcut Status and Priority Mapping"
date: "2026-11-14T09:52:01+00:00"
author: "Tom Scott"
producer: "implement-plan"
status: "draft"
kind: "spike"
priority: "medium"
parent: "work-item:0181"
relates_to: ["work-item:0371", "work-item:0290"]
tags: ["work-management", "integrations", "shortcut"]
last_updated: "2026-11-14T09:52:01+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---
# 0375: Shortcut Status and Priority Mapping

## Summary

Map a story's workflow state and its priority custom field to the local
`status` and `priority`, as input to 0290's field mapping.

## Context

Work item 0371 syncs title and body only. Shortcut keeps state per workflow,
with a state type (`unstarted`, `started`, `done`), and keeps priority in a
custom field that each workspace defines. 0290 needs to know these shapes to
design a mapping that works for all three trackers.

## Acceptance Criteria

- [ ] A note records how each local `status` maps to a Shortcut state type,
      and how a workspace's priority field could map to `priority`.
- [ ] The note names what 0290 must support for Shortcut.
