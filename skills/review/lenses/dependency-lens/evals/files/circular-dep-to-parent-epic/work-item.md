---
title: "Validate JWT on protected routes"
type: story
status: ready
priority: high
---

# Validate JWT on Protected Routes

## Summary

Add JWT validation middleware to all protected routes in the API gateway
so that requests carrying a valid Bearer token are accepted without a
session cookie.

## Context

This story is one of eight child stories in epic #0042 "Migrate
authentication from session cookies to JWT tokens". The epic cannot be
marked complete until all eight child stories ship; this story specifically
covers the gateway-side validation layer that all other stories depend on
delivering first.

## Requirements

1. Implement a `validateJwt` middleware function that verifies the
   signature and expiry of a Bearer token supplied in the `Authorization`
   header.
2. Mount the middleware on all routes currently protected by the
   `requireSession` middleware.
3. Return HTTP 401 for requests with an invalid, expired, or absent JWT.
4. Pass the decoded JWT claims (user ID, role) to the route handler via
   `request.user`.

## Acceptance Criteria

- A valid JWT grants access to a protected route without a session cookie.
- An expired JWT returns HTTP 401.
- A JWT signed with an unknown key returns HTTP 401.
- A request with no `Authorization` header to a protected route returns
  HTTP 401.
- Existing session-cookie auth continues to work alongside JWT auth
  (parallel operation during migration).

## Dependencies

- Blocked by: #0042 (parent epic — cannot ship child stories until the
  epic is approved)

## Assumptions

- The JWT signing key is available from the secrets management service at
  deploy time; no runtime key fetch is required.
- The `Authorization: Bearer <token>` header format is used by all clients.
