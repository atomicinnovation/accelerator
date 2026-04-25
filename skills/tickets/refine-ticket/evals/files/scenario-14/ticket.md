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

Refactor the authentication service to improve reliability and maintainability.

## Context

The auth service was originally written as a proof-of-concept.

## Requirements

1. Replace the in-memory session store with Redis.
2. Consolidate token validation.

## Acceptance Criteria

- [ ] Given a valid session, Redis returns data within 10ms.

## Open Questions

## Dependencies

- Blocked by:
- Blocks:

## Assumptions

- Redis will be available.

## Technical Notes

The session store is implemented in `src/auth/session.ts` using a plain JavaScript Map. Token validation is spread across `src/auth/tokens.ts` (JWT signing), `src/auth/middleware.ts` (cookie parsing), and `src/routes/auth.ts` (validation logic). The three modules share no common interface.

See also `src/auth/legacyAuth.ts` for the deprecated flag handling.

## Drafting Notes

## References
