---
title: "Migrate authentication from session cookies to JWT tokens"
type: epic
status: ready
priority: high
---

# Migrate Authentication from Session Cookies to JWT Tokens

## Summary

Replace the existing session-cookie authentication with JWT-based
authentication across the platform. This enables stateless API access for
mobile clients and third-party integrations.

## Context

The current session-cookie approach requires all API consumers to maintain
session state with the server, which is incompatible with the mobile app
and with the upcoming partner integration programme. JWT tokens will allow
stateless, cross-origin access.

## Stories

- **Issue JWT tokens on login** — Update the login endpoint to issue a
  signed JWT alongside (or instead of) the session cookie.
- **Validate JWT on protected routes** — Add JWT validation middleware to
  all protected routes in the API gateway.
- **Deprecate session-cookie auth** — Remove or disable the session-cookie
  authentication path once JWT validation is confirmed stable.
- **Rotate signing keys** — Implement a key-rotation mechanism for the
  JWT signing keys, with zero-downtime rollover.
- **Update mobile client** — Update the iOS and Android clients to store
  and transmit JWT tokens rather than session cookies.
- **Update internal API consumers** — Update the three internal services
  (reporting, data-export, admin panel) that currently call the API with
  session cookies.
- **Remove legacy session store** — Once all consumers are migrated,
  remove the Redis session store and its supporting infrastructure.
- **Load-test JWT validation middleware** — Run load tests on the new JWT
  validation middleware to confirm it meets the existing latency SLAs.

## Acceptance Criteria

- All API endpoints that currently require a session cookie accept a
  Bearer JWT token.
- The mobile client can authenticate and access the API without setting a
  cookie.
- Legacy session-cookie auth is fully removed from the codebase.
- JWT signing keys can be rotated without dropping any in-flight requests.

## Dependencies

_None identified._

## Assumptions

- The JWT signing key infrastructure will be hosted in the existing secrets
  management service.
- Session-cookie auth and JWT auth will run in parallel during the
  migration period to allow gradual rollover.
