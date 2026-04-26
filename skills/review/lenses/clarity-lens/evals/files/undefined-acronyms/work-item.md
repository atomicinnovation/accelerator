---
title: "Enforce RBAC on API endpoints using OIDC tokens"
type: story
status: ready
---

# Enforce RBAC on API Endpoints Using OIDC Tokens

## Summary

The API currently has no authorisation model. This work item adds role-based
access control to all endpoints using OIDC tokens issued by the IdP.

## Context

The current API accepts requests from any authenticated user regardless of
their role. The security team has identified this as a compliance gap and
requested that all endpoints enforce RBAC using the claims embedded in OIDC
tokens. The IdP is already configured and issuing tokens with the required
`roles` claim. The SRE team has asked that any authorisation failures be
surfaced in the APM dashboard with a P99 latency SLO attached.

## Requirements

1. The API gateway must validate the OIDC token on every incoming request.
2. Each endpoint must declare its required roles in its route configuration.
3. The RBAC middleware must extract the `roles` claim from the JWT payload and
   compare it against the required roles.
4. On authorisation failure, return a 403 with a body conforming to the
   RFC 7807 schema.
5. Authorisation failures must appear as ACL_DENY events in the APM dashboard.
6. The TTL on the JWKS cache must be configurable; default to 300s.

## Acceptance Criteria

- All endpoints return 403 for requests whose OIDC token lacks the required
  role.
- ACL_DENY events appear in the APM dashboard within 30s of the failure.
- The JWKS cache TTL can be overridden via environment variable.

## Technical Notes

The IdP uses RS256 signing. JWKS rotation happens every 24 hours; the cache
TTL default of 300s ensures keys are refreshed well before rotation.
