---
type: "work-item"
id: "0287"
title: "Playwright Executor Omits Documented Interaction Commands and Snapshot Refs"
date: "2026-09-10T17:32:31+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
relates_to: ["work-item:0208", "work-item:0196"]
tags: ["design", "playwright", "executor"]
last_updated: "2026-09-10T17:32:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0287: Playwright Executor Omits Documented Interaction Commands and Snapshot Refs

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The design executor's forwarding allowlist exposes only `ping, navigate,
snapshot, screenshot, evaluate, links, daemon-stop`. The interaction commands
`click`, `type`, and `wait_for` are absent, and `snapshot` returns no element
references. Browser-analyser agents therefore cannot actuate interaction-only
states during a crawl and must fall back to `evaluate` DOM manipulation.

## Context

Observed on build `design-1.24.0-pre.63` during the `claude-design-prototype`
runtime crawl. `accelerator design executor click '{...}'` hangs to timeout;
`accelerator design executor --help` lists only the seven commands above;
`snapshot` output carries no refs to feed a subsequent action. As a result the
analysers could not click a document card, expand a cluster, toggle theme, or
type into the search box; theme flip, search entry, and viewer navigation were
reproduced through `evaluate` (setting `data-theme`, injecting the search input
via the native value setter) rather than real input.

Work item 0208 ("Runtime Test Lane Absent From Every Build") records the same
class of defect from the 0196 migration validation on 2026-08-13 — "a forwarding
allowlist omitting four documented commands" — as one of six defects the missing
CI lane failed to catch. 0208 is scoped to wiring that CI lane and does not own
restoring the commands, so nothing currently owns this fix.

## Requirements

- Reproduction: `accelerator design executor click '{...}'` hangs;
  `executor --help` shows a seven-command allowlist with no `click`/`type`/
  `wait_for`; `snapshot` output has no element refs.
- Expected: the executor exposes the documented interaction commands, and
  `snapshot` returns stable element references usable by a subsequent
  `click`/`type`.
- Actual: interaction commands are absent and snapshots are ref-less, so
  interaction-dependent screens can only be approximated via `evaluate`.
- Confirm the exact set of omitted commands against the executor's documented
  command surface: 0208 cites "four"; this crawl observed three (`click`,
  `type`, `wait_for`).

## Acceptance Criteria

- [ ] Given a runtime crawl must actuate an interaction, when the analyser
  issues a documented interaction command (`click`/`type`/`wait_for`), then the
  executor performs it rather than timing out.
- [ ] Given a `snapshot`, then returned elements carry stable references that a
  subsequent action command can target.
- [ ] Given the documented command surface, then the executor's forwarding
  allowlist matches it — no documented command is silently omitted.

## Open Questions

- Is the omission a migration regression (documented-but-dropped) or an
  intentional decision to keep crawls non-interactive? 0208's "omitting four
  documented commands" framing implies a regression.
- What is the exact set of omitted commands, and is there a fourth beyond
  `click`, `type`, and `wait_for`?

## Dependencies

- Relates to: 0208 (records the omitted-commands defect and the CI lane that
  would catch it), 0196 (established the executor command surface).

## Technical Notes

- The executor is invoked as `accelerator design executor <command> [json]`; the
  command is checked against the forwarding allowlist before dispatch to the
  Node runner.
- `snapshot` currently exposes no `ref` ids, so even where a target can be
  located there is no handle to pass to an action command — the two gaps
  compound.

## Drafting Notes

- Filed as a new bug per the user's decision rather than folding into 0208,
  which is CI-wiring-scoped and out of scope for the executor commands.
- 0208 linked as `relates_to`, not parent or blocker: it is prior evidence and
  the regression guard, not the fix owner. Caveat recorded: 0208 says four
  commands, this crawl saw three — the exact set needs confirmation.
- Kept as a bug on the assumption the commands are documented-but-omitted; if
  the omission turns out to be intentional this becomes a docs/spec correction
  instead (see Open Questions).

## References

- Source: `meta/research/design-inventories/2026-09-10-174311-claude-design-prototype/inventory.md` (Crawl Notes)
- Related: 0208, 0196
