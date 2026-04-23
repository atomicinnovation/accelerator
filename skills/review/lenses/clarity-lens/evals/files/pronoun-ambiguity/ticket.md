---
title: "Cache invalidation on config updates"
type: story
status: ready
---

# Cache Invalidation on Config Updates

## Summary

When the configuration service updates a value, it should notify downstream
consumers. The cache should be invalidated and it should reload its entries
from the source. When a consumer registers for updates, it receives a
notification via the event bus. It processes the notification and requests the
updated value. It should complete within 100ms.

## Context

The system currently caches configuration values for performance. When an admin
updates a value through the config API, downstream services continue using stale
values until the next poll interval (5 minutes). This causes feature flag
divergence and causes it to apply incorrect rate limits.

## Requirements

1. The configuration service must publish an update event to the event bus when
   any value changes.
2. The cache must subscribe to update events and invalidate the affected entry.
3. It must request the fresh value from the configuration service within 100ms
   of receiving the event.
4. The event bus must guarantee at-least-once delivery.

## Acceptance Criteria

- When an admin updates a config value, downstream consumers receive an update
  within 5 seconds.
- The cache hit rate does not drop below 95% during normal operation.
- It completes invalidation and reload within 100ms.

## Technical Notes

The event bus is already deployed and supports pub/sub. It and the cache share
the same internal network.
