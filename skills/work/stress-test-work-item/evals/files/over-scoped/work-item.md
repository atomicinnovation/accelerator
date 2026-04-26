---
work_item_id: "0043"
title: "Improve System Quality"
date: "2026-04-01T10:00:00+00:00"
author: "Test Author"
type: story
status: draft
priority: medium
parent: ""
tags: [quality, backend, frontend, database]
---

# 0043: Improve System Quality

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Test Author

## Summary

Improve overall system quality by addressing a backlog of technical improvements across authentication, logging, metrics, UI, and database schema.

## Context

The team has accumulated several quality improvement items across different subsystems. This ticket captures them all for the sprint.

## Requirements

1. Fix the authentication service to handle concurrent login attempts without race conditions.
2. Add structured JSON logging across all API endpoints using the winston library.
3. Instrument key business metrics (user signups, active sessions, failed logins) with Prometheus counters.
4. Update the UI theme from the legacy Bootstrap 3 components to the new design system tokens.
5. Run the pending database migrations to add the `last_active_at` column to the `users` table and add indexes to the `sessions` table.
6. Write unit tests for the `PasswordHasher` utility class which currently has 0% coverage.

## Acceptance Criteria

- [ ] Concurrent login attempts no longer cause race conditions (verified by a stress test with 50 concurrent logins).
- [ ] All API endpoint logs are valid JSON with `level`, `timestamp`, `requestId`, `method`, `path`, and `duration_ms` fields.
- [ ] Prometheus scrape endpoint at `/metrics` exposes `user_signups_total`, `active_sessions_gauge`, and `failed_logins_total` counters.
- [ ] All pages using `btn-primary`, `panel`, and `navbar` Bootstrap 3 classes are migrated to the new design system equivalents.
- [ ] `last_active_at` column exists in production `users` table and is populated on every authenticated request; `sessions` table has indexes on `user_id` and `expires_at`.
- [ ] `PasswordHasher` has 100% line coverage with tests covering bcrypt hash generation, comparison, and error cases.

## Open Questions

- Should the DB migration run automatically on deploy, or does ops need to schedule a maintenance window?

## Dependencies

- Blocked by: Design system token library must be published (no ticket yet)
- Blocks:

## Assumptions

- The bootstrap-to-design-system migration can be done component by component without a full page rebuild.

## Technical Notes

## Drafting Notes

- This is a catch-all for Q2 tech debt items.

## References

- Related: 0011, 0027, 0033
