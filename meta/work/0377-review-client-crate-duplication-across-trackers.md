---
type: "work-item"
id: "0377"
title: "Review Client-Crate Duplication Across Trackers"
date: "2026-11-14T09:52:01+00:00"
author: "Tom Scott"
producer: "implement-plan"
status: "draft"
kind: "spike"
priority: "low"
parent: "work-item:0181"
relates_to: ["work-item:0371"]
tags: ["work-management", "integrations"]
last_updated: "2026-11-14T09:52:01+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---
# 0377: Review Client-Crate Duplication Across Trackers

## Summary

Review what `jira-client`, `linear-client` and `shortcut-client` duplicate,
and decide what, if anything, moves into a shared crate.

## Context

Work item 0371 copied the cache's filesystem seam, the test support doubles
and the paging shape from the Linear client, because client crates may not
import each other. Three copies now exist of some of this code.

## Research Questions

- Which duplicated code is identical, and which differs on purpose?
- Would a shared crate couple the clients' release cadence to each other?
- What does a move cost in each client's tests?

## Acceptance Criteria

- [ ] A note lists the duplicated code and recommends what to share, if
      anything, with the reason.
