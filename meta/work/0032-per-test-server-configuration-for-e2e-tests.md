---
work_item_id: "0031"
title: "Per-test server configuration and fixtures for E2E tests"
date: "2026-05-05T00:00:00+01:00"
author: Toby Clemson
type: task
status: draft
priority: medium
parent: ""
tags: [visualiser, testing, playwright, e2e]
---

# 0031: Per-test server configuration and fixtures for E2E tests

**Type**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The visualiser's Playwright E2E suite currently shares a single fixture server
started once in `global-setup.ts`. Tests that need different server configuration
(e.g. a project-prefixed ID pattern, custom kanban columns, or a differently-
seeded fixture tree) must mock API endpoints via `page.route()` rather than
running against a real configured server. This limits what the tests can assert
and means the full server-side validation path (boot-time config parsing, PATCH
validation against the configured column set) goes untested at the E2E level.

This work item introduces per-test (or per-suite) server lifecycle support so
that tests can spin up an isolated server instance with arbitrary configuration
and a clean fixture directory, exercise the full stack without mocking, and shut
it down at the end of the test.

## Context

Three E2E specs were written against the work-item terminology migration plan
that rely on mocking because they need non-default configuration:

- `frontend/e2e/project-pattern-custom-columns.spec.ts` — needs a server
  configured with `work.id_pattern: "{project}-{number:04d}"` and
  `visualiser.kanban_columns: [ready, in-progress, review, done]`
- `frontend/e2e/default-pattern.spec.ts` — partially exercises the real server
  but some assertions are limited by the shared fixture set
- `frontend/e2e/legacy-schema.spec.ts` — needs a fixture tree with a known
  set of legacy-schema files

The shared server is started in `global-setup.ts` against
`server/tests/fixtures/meta/` with the default compiled configuration.
Playwright's `globalSetup`/`globalTeardown` lifecycle does not support
per-test server instances natively, but the pattern can be implemented via
a test fixture factory that starts the server binary on a free port, waits
for it to become ready, and registers a `test.afterEach` (or `test.afterAll`)
teardown.

## Requirements

1. A Playwright test fixture (e.g. `withServer(config, fixtures)`) that:
   - Writes a `config.json` with the supplied configuration to a temp directory
   - Copies or seeds a fixture `meta/` tree in the same temp directory
   - Starts the visualiser server binary on a free port pointed at that directory
   - Waits for the server to respond to `GET /api/types` (health check)
   - Returns a `baseURL` and optionally a `request` context scoped to that server
   - Stops the server and cleans up the temp directory after the test/suite
2. The fixture must work on all four supported platforms (macOS arm64/x86_64,
   Linux arm64/x86_64) by locating the server binary via the same mechanism as
   the existing `global-setup.ts`.
3. Port allocation must be collision-safe — use `net.createServer` with port 0
   to let the OS assign a free port, then close and reuse it.
4. The three specs that currently use `page.route()` mocking must be updated to
   use the real server fixture instead, removing the mocks.
5. The `global-setup.ts` shared server must continue to work for tests that do
   not need per-test configuration.

## Acceptance Criteria

- [ ] A `withServer` (or equivalent) fixture factory is exported from
  `frontend/e2e/fixtures.ts` and accepts a partial `config.json` shape and an
  optional fixture tree seed function.
- [ ] `frontend/e2e/project-pattern-custom-columns.spec.ts` uses the real
  server with `work.id_pattern: "{project}-{number:04d}"` and four custom
  columns; all `page.route()` mocks are removed.
- [ ] `frontend/e2e/default-pattern.spec.ts` uses the real server with default
  config but a deterministic fixture set; no mocks needed.
- [ ] `frontend/e2e/legacy-schema.spec.ts` uses the real server with a fixture
  tree seeded from the 30 legacy `meta/work/` files.
- [ ] Per-test servers start and stop cleanly; no port leaks or orphaned
  processes after a test run (including after failures).
- [ ] All existing E2E specs that use the shared server continue to pass
  without modification.
- [ ] `npx playwright test` passes the full suite on CI.

## Technical Notes

- The Playwright `test.extend` API can inject per-test server state as a
  fixture. Scope as `{ scope: 'test' }` for isolation or `{ scope: 'worker' }`
  for per-file sharing (prefer per-file to avoid startup overhead on large suites).
- The server binary path is available as `process.env.VISUALISER_BIN` (set by
  `global-setup.ts`). Reuse this rather than re-deriving the path.
- `config.json` schema is defined in
  `skills/visualisation/visualise/server/src/config.rs`; the Typescript shape
  used in `global-setup.ts` is the reference for what fields to populate.
- Temp directories: use Node's `fs.mkdtempSync(path.join(os.tmpdir(), 'vis-e2e-'))`.
  Register cleanup in a `test.afterAll` (not `afterEach`) to avoid restarts
  between tests in the same file.

## References

- `frontend/e2e/global-setup.ts` — existing server startup pattern to model
- `frontend/e2e/fixtures.ts` — Playwright fixture extension point
- `frontend/e2e/project-pattern-custom-columns.spec.ts` — primary consumer
- `frontend/e2e/default-pattern.spec.ts` — secondary consumer
- `frontend/e2e/legacy-schema.spec.ts` — secondary consumer
- `skills/visualisation/visualise/server/src/config.rs` — `config.json` schema
