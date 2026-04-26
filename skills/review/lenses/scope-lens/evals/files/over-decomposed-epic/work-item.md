---
title: "Rename legacy config variables to use snake_case"
type: epic
status: ready
priority: low
---

# Rename Legacy Config Variables to use snake_case

## Summary

Rename all legacy config variables in the settings module from camelCase to
snake_case to comply with the new naming convention.

## Context

The settings module has twelve variables that still use camelCase names from
the original implementation. A decision was made to use snake_case throughout
the codebase.

## Requirements

1. Rename each camelCase config variable to its snake_case equivalent.
2. Update all references in application code and tests.

## Stories

- Rename `maxRetries` to `max_retries`
- Rename `connectionTimeout` to `connection_timeout`
- Rename `enableDebug` to `enable_debug`
- Rename `logLevel` to `log_level`
- Rename `cacheSize` to `cache_size`
- Rename `requestTimeout` to `request_timeout`
- Rename `retryDelay` to `retry_delay`
- Rename `apiBaseUrl` to `api_base_url`
- Rename `maxConcurrent` to `max_concurrent`
- Rename `sessionExpiry` to `session_expiry`
- Rename `enableMetrics` to `enable_metrics`
- Rename `defaultLocale` to `default_locale`

## Acceptance Criteria

- All twelve config variables use snake_case naming.
- No references to the old camelCase names remain in application code or tests.
- CI passes.

## Dependencies

- None
