---
work_item_id: "0100"
title: "Configurable Visualiser Auto-Shutdown"
date: "2026-06-02T13:37:32+00:00"
author: Toby Clemson
type: story
status: draft
priority: low
parent: ""
tags: [visualiser, server, configuration, lifecycle]
---

# 0100: Configurable Visualiser Auto-Shutdown

**Type**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Make the visualiser server's idle auto-shutdown timeout configurable, and
raise the default from 30 minutes to 8 hours so a working session doesn't
outlive the server.

## Context

The visualiser server currently auto-exits after 30 minutes idle (also on
launching-process exit or explicit `stop`). 30 minutes is short for a
review session left open in a browser tab — the server dies mid-review and
must be relaunched. The timeout is hard-coded; there is no override.

## Requirements

- Raise the default idle auto-shutdown from 30 minutes to 8 hours.
- Make the timeout configurable (mechanism TBD — see Open Questions).

## Open Questions

- Configuration mechanism: env var, `.accelerator/config.md` (`visualiser:`
  block), CLI flag, or some combination — and precedence between them.
- Units and format for the configured value (minutes, duration string).
- Whether a value of 0 / "never" to disable auto-shutdown should be
  supported.

## Dependencies

- Related: visualiser server lifecycle (auto-exit on idle / launching-process
  exit / explicit stop).

## Drafting Notes

- Captured as a stub for later refinement; Requirements and Acceptance
  Criteria are deliberately thin pending the configuration-mechanism
  decision.
