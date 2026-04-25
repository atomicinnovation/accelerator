---
ticket_id: "0042"
title: "User Authentication Refactor"
date: "2026-04-01T10:00:00+00:00"
author: "Test Author"
type: story
status: draft
priority: high
parent: ""
tags: [auth, backend]
---

# 0042: User Authentication Refactor

**Type**: Story
**Status**: Draft
**Priority**: High
**Author**: Test Author

## Summary

Refactor the authentication service to improve reliability and maintainability. The current auth service handles sessions in-memory and token validation is spread across multiple modules.

## Context

The auth service was originally written as a proof-of-concept and has grown organically. Several incidents in the last quarter were traced to race conditions in the session store.

## Requirements

1. Replace the in-memory session store with a persistent Redis-backed implementation.
2. Consolidate token validation logic currently spread across three modules into a single `TokenValidator` class.
3. Add structured logging to all authentication events.
4. Remove the deprecated `legacyAuth` flag.

## Acceptance Criteria

- [ ] Given a valid session, Redis returns the session data within 10ms.
- [ ] All authentication events emit a structured log entry.

## Open Questions

- Should we support multiple Redis instances for high availability?

## Dependencies

- Blocked by:
- Blocks:

## Assumptions

- Redis will be available in all deployment environments.

## Technical Notes

## Drafting Notes

## References
