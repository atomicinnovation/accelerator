---
type: "work-item"
id: "0376"
title: "One Exhaustive Tracker Registry"
date: "2026-11-14T09:52:01+00:00"
author: "Tom Scott"
producer: "implement-plan"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0181"
relates_to: ["work-item:0371"]
tags: ["work-management", "integrations"]
last_updated: "2026-11-14T09:52:01+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---
# 0376: One Exhaustive Tracker Registry

## Summary

Replace the string matches on `work.integration` with one enum, so adding a
tracker fails to compile until every site decides how to handle it.

## Context

Work item 0371 had to find and pin each site that matches on the
integration name: the registry, the scope key, the create preview, the
finaliser, tracker-owned ids, a migration and the baseline realigner. A
`match` on a string has a wildcard arm, so a missed site compiles and does
the wrong thing silently.

## Acceptance Criteria

- [ ] A `Tracker` enum parses `work.integration`, and every site matches on
      it with no wildcard arm.
- [ ] Adding a variant makes each site fail to compile until it handles the
      variant.
