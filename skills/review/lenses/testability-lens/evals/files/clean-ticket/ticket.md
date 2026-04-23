---
type: story
status: ready
priority: high
---

## Summary
Rate-limit the public API to 100 requests per minute per API key

## Context
The public API currently has no rate limiting. Two customers are running automated polling loops that are generating over 2,000 requests/minute each, degrading response times for all other customers. Introducing a per-API-key limit of 100 req/min will cap runaway clients while keeping normal usage well within limits (p99 usage is 12 req/min per key).

## Requirements
1. Each API key may make at most 100 requests per 60-second sliding window
2. Requests that exceed the limit must be rejected with HTTP 429 and a `Retry-After` header indicating when the limit resets
3. Requests within the limit must complete with no added latency overhead above 5ms at p99
4. The current rate-limit counters must be readable via the existing `/admin/api-keys/{id}` endpoint

## Acceptance Criteria
- Given an API key that has made 100 requests in the last 60 seconds, when a 101st request arrives, then the API responds with HTTP 429 and a `Retry-After` header value between 1 and 60 seconds inclusive
- Given an API key that has made 99 requests in the last 60 seconds, when a 100th request arrives, then the API responds with HTTP 200 (or the normal response for that endpoint)
- Given a key at its limit, when 60 seconds have elapsed since the oldest request in the window, then subsequent requests succeed
- When `GET /admin/api-keys/{id}` is called, the response includes `rate_limit.requests_this_window` and `rate_limit.window_resets_at` fields
- When 1,000 sequential requests are made against a rate-limited key using a load test, p99 latency overhead compared to a non-rate-limited baseline does not exceed 5ms

## Dependencies
- Redis (existing cluster at `redis.internal`) — for sliding-window counters
- API gateway middleware (existing `src/middleware/`) — insertion point for rate-limit check

## Assumptions
- API key identity is already available in request context before middleware runs
- The Redis cluster can handle the additional write load (capacity confirmed with infrastructure team)
