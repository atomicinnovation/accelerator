---
work_item_id: "0042"
title: "User Auth Rework"
date: "2026-04-01T10:00:00+00:00"
author: "Test Author"
type: epic
status: draft
priority: high
parent: ""
tags: [auth, backend]
---

# 0042: User Auth Rework

**Type**: Epic
**Status**: Draft
**Priority**: High
**Author**: Test Author

## Summary

Rework the authentication system covering session store, token signing, and middleware.

## Context

Three separate auth incidents in Q1 motivated a cohesive rework.

## Requirements

1. Replace in-memory session store with Redis.
2. Consolidate JWT signing/verification into a single module.
3. Refactor the auth middleware to use the new token validator.

## Acceptance Criteria

- [ ] All Requirements above are implemented.

## Open Questions

## Dependencies

- Blocked by:
- Blocks:

## Assumptions

- Redis available in all environments.

## Technical Notes

## Drafting Notes

## References
