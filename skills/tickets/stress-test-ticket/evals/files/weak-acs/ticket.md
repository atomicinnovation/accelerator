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

Refactor the authentication service to improve reliability and maintainability. The current auth service has grown organically and has several known issues with session management and token validation.

## Context

The auth service was originally written as a proof-of-concept and has never been properly refactored. Several incidents in the last quarter were traced to race conditions in the session store. The team has agreed that a proper refactor is needed before adding new auth features.

## Requirements

1. Replace the in-memory session store with a persistent Redis-backed implementation.
2. Consolidate token validation logic currently spread across three modules (`auth.js`, `middleware.js`, `session.js`) into a single `TokenValidator` class.
3. Add structured logging to all authentication events (login, logout, token refresh, failure).
4. Remove the deprecated `legacyAuth` flag that has been set to `false` in production for six months.

## Acceptance Criteria

- [ ] The system should handle errors gracefully.
- [ ] Authentication should be fast.
- [ ] The refactored code should work correctly.
- [ ] Users should be able to log in.
- [ ] Token validation should work.

## Open Questions

- Should we support multiple Redis instances for high availability?

## Dependencies

- Blocked by:
- Blocks:

## Assumptions

- Redis will be available in all deployment environments.
- The existing session format is compatible with the new Redis store.

## Technical Notes

## Drafting Notes

- Refactor scope agreed with tech lead in sprint planning.

## References

- Related: 0039, 0041
