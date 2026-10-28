---
type: "work-item"
id: "0367"
title: "Stop rebuilding unchanged inputs"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0361"
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["build-system", "performance", "frontend", "rust"]
last_updated: "2026-10-28T16:18:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0367: Stop rebuilding unchanged inputs

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Three tasks redo work on every run even when nothing they depend on has
changed:
- the frontend build;
- the frontend `npm ci`;
- the launcher's build script, which reruns whenever the parent repository's
  `HEAD` moves.

Each triggers further rebuilds downstream. Making them skip unchanged inputs
removes that churn.

## Context

**Frontend build (B3).** `build:frontend` declares no `sources` or `outputs`,
so it rewrites `dist/` on every run. The visualiser's `build.rs` watches the
whole `dist/` directory. So even a no-op run recompiled
`accelerator-visualiser` in `pup:check` and in the `embed-dist` pass of
`test:unit:visualiser`.

With no `build` block in `vite.config.ts`, Vite's default `emptyOutDir` also
races the stub-gated `build.rs` assert when clippy and pup run at the same
time.

**`npm ci` (B4).** `deps:install:node` runs `npm ci` unconditionally. That
deletes `node_modules`, and with it the `tsBuildInfoFile`s and the Vite
cache, so both `tsc -b` runs start cold. It costs about 5 s alone, and the
churn also feeds the IDE indexer.

**Launcher build script (B6).** The launcher's `build.rs` uses vergen-gitcl,
which watches the parent repository's `.git/HEAD`. So commits in the main jj
workspace rebuild the launcher in every configuration of every other
workspace. `build_timestamp(true)` guarantees the recompile is never a no-op.

The docs tasks are the only tasks that declare working `sources`/`outputs`
today, and they were skipped correctly in warm runs.

## Requirements

1. Give `build:frontend` `sources`/`outputs`, so an unchanged frontend
   leaves `dist/` untouched.
2. Give `deps:install:node` `sources`/`outputs` keyed on
   `package-lock.json`, so `npm ci` runs only when the lockfile changes.
3. Exclude the parent `.git/HEAD` and the build timestamp from launcher
   rebuilds in dev builds. Release builds keep the metadata.

## Acceptance Criteria

- [ ] Given no frontend change since the last run, when `mise run` runs,
      then `build:frontend` is skipped and no task recompiles
      `accelerator-visualiser`.
- [ ] Given an unchanged `package-lock.json`, when `mise run` runs, then
      `deps:install:node` is skipped and the `tsBuildInfoFile`s under
      `node_modules/.tmp` survive.
- [ ] Given a commit in the main jj workspace, when another workspace with
      no source changes runs `mise run`, then the launcher is not recompiled.
- [ ] Given a release build, when its version output is inspected, then it
      still carries the git SHA and build timestamp.

## Open Questions

- Which inputs define the frontend build's `sources`? Candidates are `src`,
  `public`, the configs and the lockfile.
- Does any test assert on the dev build's git SHA or timestamp?

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- mise's `sources`/`outputs` caching behaves for these tasks as it already
  does for the docs tasks.

## Technical Notes

- `build:frontend`: `mise.toml:205-208`.
- `deps:install:node`: `mise.toml:82-84` and `tasks/deps.py:204`.
- Directory-wide `rerun-if-changed`: `cli/visualiser/server/build.rs:20`.
- vergen setup: `cli/launcher/build.rs:18-25`.
- `tsBuildInfoFile` location: `cli/visualiser/frontend/tsconfig.app.json:3`.

## Drafting Notes

- Grouped B3, B4 and B6 because all three stop work from re-running when
  inputs are unchanged. Agreed in conversation.
- The research measured these costs only indirectly, through the recompiles
  they trigger and about 5 s for `npm ci`. So the acceptance criteria test
  behaviour rather than seconds.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Related: 0361
