---
title: "Add per-client rate limiting to the public API"
type: story
status: ready
priority: high
---

# Add Per-Client Rate Limiting to the Public API

## Summary

The public API currently has no rate limiting, allowing a single client to
make unlimited requests and potentially degrade service for all users. This
work item adds per-client rate limiting at the API gateway layer, enforced using
a sliding window algorithm with limits configurable per API key.

## Context

The operations team has observed several incidents where a single client's
runaway polling loop consumed 40% of API capacity for 15-minute windows. The
team has agreed to implement rate limiting using a sliding window algorithm.
Redis is already deployed as a shared cache and will serve as the rate-limit
counter store. Limits will be applied per API key rather than per IP address,
because many clients share a NAT gateway.

## Requirements

1. The API gateway must check a rate-limit counter in Redis before forwarding
   each request.
2. If the counter exceeds the configured limit, the API gateway must return
   HTTP 429 with a `Retry-After` header specifying the seconds until the
   window resets.
3. The default limit is 1000 requests per 60-second sliding window; this must
   be configurable per API key.
4. The API gateway must increment the counter atomically using a Redis Lua
   script to prevent race conditions under concurrent load.
5. The operations team must be able to adjust per-key limits without
   redeploying the API gateway.

## Acceptance Criteria

- When a client sends more than 1000 requests within any 60-second window, the
  API gateway returns HTTP 429 with a `Retry-After` header on all subsequent
  requests until the window resets.
- When a client is below the rate limit, the API gateway forwards requests
  normally with no added latency above 2ms at the 99th percentile.
- An operator can change the default rate limit by updating a configuration
  value without redeploying the API gateway.
- Per-key overrides can be added or changed without restarting any service.

## Dependencies

- Redis cluster (already deployed, owned by: infrastructure team)
- API gateway deployment pipeline (owned by: platform team)

## Assumptions

- The Redis cluster has sufficient throughput to handle the added counter
  operations (estimated at 2× current request volume).
- API keys are already issued and stored in the gateway's key registry.

## Technical Notes

Use the `INCR` and `EXPIRE` commands inside a Lua script for atomicity. If the
Redis version does not support sorted sets, a fixed window with a 2× safety
factor is an acceptable approximation of the sliding window.
