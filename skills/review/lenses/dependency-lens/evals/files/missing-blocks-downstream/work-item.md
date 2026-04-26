---
title: "Introduce versioned API schema file"
type: story
status: ready
priority: high
---

# Introduce Versioned API Schema File

## Summary

Add a machine-readable schema file (`api-schema.json`) to the repository
that describes every public API endpoint, its request and response shapes,
and its current version. This file will enable further work across the
engineering team.

## Context

Currently there is no single authoritative description of the public API
surface. Each team maintains its own informal understanding of the endpoint
contracts, which causes integration bugs whenever a consuming service's
assumptions drift from the provider's implementation.

Adding a versioned schema file will:
- Enable client-side type generation in the web and mobile front-ends
- Allow the platform team to add automated contract-change detection to CI
- Let the documentation team auto-generate API reference pages from a
  single source of truth

Three follow-up work items are already drafted and waiting for this story to
merge before they can proceed: client type generation (#1041), contract-
change CI check (#1042), and API reference auto-generation (#1043).

## Requirements

1. Create `api-schema.json` at the repository root following the OpenAPI
   3.1 specification.
2. Document every currently-public endpoint: path, method, request body
   schema, response schemas (success and error), and authentication
   requirements.
3. Add a CI step that validates the schema file is syntactically valid
   OpenAPI on every pull request.

## Acceptance Criteria

- `api-schema.json` exists at the repository root and passes
  `openapi-generator validate` without errors.
- Every public endpoint listed in the existing handwritten API reference
  docs is present in the schema file.
- The CI validation step fails the build if `api-schema.json` is
  syntactically invalid.

## Dependencies

- Blocked by: none

## Assumptions

- The existing handwritten API reference docs are the authoritative list
  of public endpoints for this story; undocumented internal endpoints are
  out of scope.
- OpenAPI 3.1 is acceptable to all downstream consumers (web, mobile,
  platform, docs teams).
