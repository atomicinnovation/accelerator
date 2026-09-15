---
type: "work-item"
id: "0288"
title: "Parallel Browser-Analyser Agents Share One Daemon and Tab Without State Isolation"
date: "2026-09-10T17:32:37+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
relates_to: ["work-item:0206", "work-item:0214", "work-item:0196"]
tags: ["design", "playwright", "executor"]
last_updated: "2026-09-10T17:32:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0288: Parallel Browser-Analyser Agents Share One Daemon and Tab Without State Isolation

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

In `runtime`/`hybrid` mode `inventory-design` spawns browser-analyser agents in
parallel, but they drive a single shared browser tab through one reused
Playwright daemon. Routes, theme, and search state change under each other
mid-crawl, so an agent can read or capture a screen that a sibling has navigated
away, and volatile UI state persists across navigations and agents,
contaminating observations and screenshots.

## Context

During the `claude-design-prototype` crawl the four analysers reported the hash
route changing under them (for example `#/library` → `#/library/work` →
`#/lifecycle`) between their own calls, and a `review` search query set by one
agent persisted in the shared browser's storage and reopened the search overlay
across later navigations — obscuring the sidebar in every screen captured before
it was cleared. Route-independent data (design tokens read from the stylesheet)
was unaffected, but runtime observations and captures were not reliably
attributable to the intended screen. The orchestrator worked around it by
re-capturing serially and confirming each screen by its rendered heading.

Prior art: 0206 (done) documents invocation-level daemon-reuse flag bleed; 0214
(done, spike) analyses a flock lease that admits multiple concurrent holders for
concurrent crawls. Neither covers parallel agents sharing one tab within a
single crawl.

## Requirements

- Reproduction: spawn two or more browser-analyser agents concurrently against
  one crawl; observe the active hash route and/or search query changing between
  one agent's successive calls.
- Expected: each analyser's navigation and UI state is isolated from its
  siblings; captures are deterministic and attributable to the intended screen.
- Actual: agents share one tab and daemon; navigation and volatile UI state
  (search query, theme override) bleed across agents and persist across
  navigations.
- Provide isolation — a browser context or tab per agent — or serialise the
  analysers, and reset volatile UI state before each capture.

## Acceptance Criteria

- [ ] Given multiple screen groups analysed concurrently, when one agent
  navigates or changes UI state, then no other agent's reads or captures are
  affected.
- [ ] Given any capture, then volatile UI state (an open search overlay, a theme
  override) is in a known, reset state unless the capture is deliberately
  exercising it.
- [ ] Given a crawl completes, then every screenshot is verifiably of its
  intended screen — for example confirmed by heading — without manual serial
  re-capture.

## Open Questions

- Preferred remedy: per-agent browser context/tab isolation on the shared
  daemon, a daemon-per-agent model, or serialising the analyser fan-out?
  Isolation preserves the skill's parallelism; serialisation is simpler but
  slower.

## Dependencies

- Relates to: 0206 (invocation-level daemon-reuse bleed), 0214 (flock
  concurrent-holder analysis), 0196 (foundational executor CLI).

## Technical Notes

- The `inventory-design` skill (Step 8, `runtime`/`hybrid`) instructs spawning a
  browser-analyser per screen group "in parallel"; the executor reuses a single
  daemon and browser page across those agents, so the instruction and the
  runtime model are in tension.
- Volatile state that bled during the crawl: the active hash route, the
  `data-theme` override, and the search query held in the app's storage.

## Drafting Notes

- Folded the persisted-search-state contamination into this item: it shares the
  root cause (shared, non-isolated browser state) and the same fix surface
  (isolation plus state reset).
- Filed as a bug: the skill instructs a parallel fan-out that the shared-daemon
  model cannot safely support, producing unreliable observations.
- Left the remedy open (isolation versus serialisation) as a genuine design
  question rather than pre-deciding it.

## References

- Source: `meta/research/design-inventories/2026-09-10-174311-claude-design-prototype/inventory.md` (Crawl Notes)
- Related: 0206, 0214, 0196
