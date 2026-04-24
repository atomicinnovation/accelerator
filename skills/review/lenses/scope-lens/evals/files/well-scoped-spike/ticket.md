---
title: "Evaluate rate-limiter libraries for the public API"
type: spike
status: ready
priority: high
---

# Evaluate Rate-Limiter Libraries for the Public API

## Summary

Evaluate three rate-limiter libraries — `rate-limiter-flexible`,
`bottleneck`, and `p-throttle` — for use in the public API, and recommend
one with a documented rationale.

## Context

The public API currently has no rate limiting, leaving it vulnerable to
abuse and making it difficult to offer tiered service plans. The team has
agreed that a Node.js-native library is preferable over a Redis-based
approach at the current traffic scale. This spike must be completed before
the "API rate limiting" feature story can be implemented.

## Requirements

1. Evaluate `rate-limiter-flexible`, `bottleneck`, and `p-throttle` against
   the following criteria: per-user limit enforcement, distributed counter
   support (Redis optional, file-based or in-memory acceptable), graceful
   degradation under load, and maintenance status.
2. Prototype a minimal integration of the top candidate in a sandbox branch
   to validate the API it exposes.

## Acceptance Criteria

- A decision memo (committed to `meta/research/`) names the recommended
  library, summarises each candidate against the evaluation criteria, and
  states the rationale for the recommendation.
- A comparison matrix (Markdown table in the memo) lists all three candidates
  against each evaluation criterion with a pass/fail/partial rating.
- The recommended default configuration (rate limit per user, window size,
  burst allowance) is documented in the memo.

## Time-box

3 working days.

## Dependencies

- None (sandbox evaluation only; no production changes in scope)

## Assumptions

- Redis is not available in the current infrastructure, so Redis-dependent
  solutions are lower priority.
- The prototype does not need to pass code review; it is exploratory only.
