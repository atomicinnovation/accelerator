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
2. Consolidate token validation logic currently spread across three modules into a single `TokenValidator` class.
3. Add structured logging to all authentication events.
4. Remove the deprecated `legacyAuth` flag.

## Acceptance Criteria

- [ ] When the Redis connection fails during a login attempt, the error is caught, a structured log entry with level=error and the exception details is written, and the response returns HTTP 503 with body `{"error": "service_unavailable"}`.
- [ ] Token validation completes in under 50ms for a valid JWT (measured via the structured log's `duration_ms` field under a 10-req/s load).
- [ ] Given a valid username and password, the user is authenticated and receives a session cookie with `HttpOnly`, `Secure`, and `SameSite=Strict` attributes.
- [ ] Given an invalid token in the `Authorization` header, the middleware returns HTTP 401 with body `{"error": "invalid_token"}` and logs the failure with `level=warn`.
- [ ] Given a token that was valid when the request started but expires mid-request, the request completes using the token's original claims and a refresh is not triggered mid-flight.

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
