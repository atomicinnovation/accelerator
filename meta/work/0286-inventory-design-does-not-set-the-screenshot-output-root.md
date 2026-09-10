---
type: "work-item"
id: "0286"
title: "inventory-design Does Not Set the Screenshot Output Root, Blocking Crawl Captures"
date: "2026-09-10T17:32:25+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "high"
relates_to: ["work-item:0196", "work-item:0262"]
tags: ["design", "playwright", "inventory-design"]
last_updated: "2026-09-10T17:32:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0286: inventory-design Does Not Set the Screenshot Output Root, Blocking Crawl Captures

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

The `inventory-design` skill never exports `ACCELERATOR_INVENTORY_OUTPUT_ROOT`
before the Playwright executor daemon is spawned, so the screenshot path-guard
fails closed and every screenshot in a `runtime` or `hybrid` crawl is refused.
The inventory is written with no visual captures unless the operator intervenes
manually.

## Context

In `runtime`/`hybrid` mode the skill starts the daemon at Step 5
(`accelerator design executor ping`) and later spawns browser-analyser agents to
capture screens. The screenshot path-guard
(`skills/design/inventory-design/scripts/playwright/lib/path-guard.js:34`) reads
`ACCELERATOR_INVENTORY_OUTPUT_ROOT` from the daemon's own `process.env` and
returns `screenshot-output-root-unset` when it is missing or empty. `SKILL.md`
sets it nowhere.

The daemon captures its environment at spawn — the spawn in
`cli/design-adapters/src/process.rs` does not `env_clear`, so it would inherit
the variable if the spawning process had it, but nothing sets it — so a
browser-analyser exporting the variable in its own shell cannot reach the
already-running daemon. Observed on build `design-1.24.0-pre.63` during the
`claude-design-prototype` crawl: all four analysers were refused every
screenshot. A prior `current-app` inventory hit the same wall (its Crawl Notes
at `meta/research/design-inventories/2026-05-21-004250-current-app/inventory.md`).

## Requirements

- Reproduction: run `/accelerator:inventory-design <source-id> <url> --crawler
  runtime` (or `hybrid`). Every `screenshot` executor call returns
  `{"error":"screenshot-output-root-unset", ...}`.
- Expected: screenshots are written under the in-progress inventory's
  `screenshots/` directory.
- Actual: no screenshots are produced; the inventory is written with an empty
  `screenshots/` directory, silently, unless the operator stops the daemon and
  restarts it with the variable set.
- The skill must export `ACCELERATOR_INVENTORY_OUTPUT_ROOT`, pointing at the
  tmp inventory's `screenshots/` directory, before the first `executor` call
  that spawns the daemon (Step 5), so the daemon — and every session reusing it
  — inherits it.

## Acceptance Criteria

- [ ] Given a `runtime` or `hybrid` crawl, when a browser-analyser captures a
  screenshot, then the PNG is written under the inventory's `screenshots/`
  directory and no `screenshot-output-root-unset` error occurs.
- [ ] Given the skill spawns the daemon, then the daemon's environment carries
  `ACCELERATOR_INVENTORY_OUTPUT_ROOT` resolving to the in-progress
  `.tmp/screenshots` directory.
- [ ] Given `ACCELERATOR_INVENTORY_OUTPUT_ROOT` is unset or empty, then the
  path-guard still fails closed — the existing security guard is preserved.
- [ ] Given the crawl finishes, then the captured screenshots move with the
  atomic `.tmp` → final rename into the published inventory directory.

## Open Questions

- Where should the variable be set — inside `SKILL.md` before Step 5, or by the
  `executor` wrapper itself given a resolved inventory tmp path? The former
  matches the existing plan; the latter would also cover direct executor use.

## Dependencies

- Relates to: 0196 (foundational design-inventory/executor CLI), 0262 (visualiser
  rendering of captured screenshots — the downstream consumer of these captures).

## Technical Notes

- Path-guard: `skills/design/inventory-design/scripts/playwright/lib/path-guard.js`.
- Daemon spawn inherits parent env (no `env_clear`):
  `cli/design-adapters/src/process.rs`.
- Prior plan specifying this plumbing:
  `meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md` (Phase 4 §3 —
  `SKILL.md` exports the env var before each browser-agent Task call); the review
  at `meta/reviews/plans/2026-05-06-design-skill-localhost-and-mcp-issues-review-1.md`
  flagged the under-specification. The plumbing was not carried into the shipped
  `SKILL.md`.
- Recovery used during the crawl that surfaced this: stop the daemon, restart it
  with the variable set inline
  (`ACCELERATOR_INVENTORY_OUTPUT_ROOT=<tmp>/screenshots accelerator design
  executor ping`), then capture serially.

## Drafting Notes

- Filed as a bug (skill defect), not a task: a documented deliverable
  (screenshots) is silently broken in the shipped skill.
- Priority high because it defeats screenshot capture for every runtime/hybrid
  crawl and fails silently — the inventory still writes, just empty of captures.
- Framed the fix as "SKILL.md sets the variable before daemon spawn" per the
  existing 2026-05-06 plan rather than moving the guard, since the guard's
  fail-closed behaviour is a deliberate security feature to keep.

## References

- Source: `meta/research/design-inventories/2026-09-10-174311-claude-design-prototype/inventory.md` (Crawl Notes)
- Plan: `meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md`
- Related: 0196, 0262
