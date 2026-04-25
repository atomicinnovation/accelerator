---
ticket_id: "0099"
title: "Fix Null Pointer in Session Cleanup"
date: "2026-04-01T10:00:00+00:00"
author: "Test Author"
type: bug
status: draft
priority: high
parent: ""
tags: [auth, backend]
---

# 0099: Fix Null Pointer in Session Cleanup

**Type**: Bug
**Status**: Draft
**Priority**: High
**Author**: Test Author

## Summary

A null pointer exception occurs in the session cleanup job when a session record has a null `user_id` field.

## Context

Observed in production logs on 2026-03-28. Session cleanup runs hourly and processes ~5000 sessions.

## Requirements

1. Handle null `user_id` in session cleanup gracefully (skip the session and log a warning).
2. Add a regression test for the null `user_id` path.

## Acceptance Criteria

- [ ] Given a session with a null `user_id`, the cleanup job skips the session and emits a `level=warn` log entry containing the session ID.
- [ ] The cleanup job processes all remaining valid sessions after encountering a null `user_id` session.

## Open Questions

## Dependencies

- Blocked by:
- Blocks:

## Assumptions

- Sessions with null `user_id` are rare but valid data.

## Technical Notes

## Drafting Notes

## References
