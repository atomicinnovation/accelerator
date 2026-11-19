---
type: "work-item"
id: "PP-989"
external_id: "PP-989"
aliases: ["0395", "draft-vh87k5"]
title: "Shortcut Status and Priority Mapping"
date: "2026-11-14T09:52:01+00:00"
author: "Tom Scott"
producer: "implement-plan"
status: "draft"
kind: "spike"
priority: "medium"
parent: "work-item:PP-701"
relates_to: ["work-item:PP-990", "work-item:PP-876"]
tags: ["work-management", "integrations", "shortcut"]
last_updated: "2026-11-14T09:52:01+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---
# PP-989: Shortcut Status and Priority Mapping

## Summary

Map a story's workflow state and its priority custom field to the local
`status` and `priority`, as input to PP-876's field mapping.

## Context

Work item PP-990 syncs title and body only. Shortcut keeps state per
workflow, with a state type (`unstarted`, `started`, `done`), and keeps priority
in a custom field that each workspace defines. PP-876 needs to know these shapes
to design a mapping that works for all three trackers.

## Acceptance Criteria

- [ ] A note records how each local `status` maps to a Shortcut state type,
      and how a workspace's priority field could map to `priority`.
- [ ] The note names what PP-876 must support for Shortcut.
