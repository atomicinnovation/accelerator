---
id: "0100"
title: "Configurable Visualiser Auto-Shutdown"
date: "2026-06-02T13:37:32+00:00"
author: Toby Clemson
kind: story
status: done
priority: low
tags: [visualiser, server, configuration, lifecycle]
type: work-item
schema_version: 1
last_updated: "2026-06-02T13:37:32+00:00"
last_updated_by: Toby Clemson
relates_to: ["work-item:0055"]
---

# 0100: Configurable Visualiser Auto-Shutdown

**Kind**: Story
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

Make the visualiser server's idle auto-shutdown timeout configurable via a
duration string in the `visualiser:` config block (with an environment-variable
override), and raise the default from 30 minutes to 8 hours so a working
session doesn't outlive the server.

## Context

The visualiser server currently auto-exits after 30 minutes idle (also on
launching-process exit or explicit `stop`). 30 minutes is short for a
review session left open in a browser tab — the server dies mid-review and
must be relaunched. The timeout is hard-coded; there is no override.

Here "idle" means no incoming HTTP request has been received since the last
recorded server-side activity timestamp; the idle timer resets on each
request. An open browser tab that issues no requests still counts as idle.

## Requirements

- Raise the default idle auto-shutdown timeout from 30 minutes to **8 hours**,
  expressed internally as the same duration-string format used for
  configuration. This default raise is independently valuable and may land
  before the configurability machinery if that work needs to be deferred.
- Read the timeout from a new `idle_timeout` key in the `visualiser:` block of
  `.accelerator/config.md` (team-committed) or `.accelerator/config.local.md`
  (personal), accepting a **duration string** (e.g. `"8h"`, `"30m"`,
  `"1h30m"`). The canonical config form is the nested YAML block (mirroring
  the existing `visualiser.binary` key):

  ```yaml
  visualiser:
    idle_timeout: "8h"
  ```

  Referred to below as `visualiser.idle_timeout`.
- Honour an environment-variable override
  `ACCELERATOR_VISUALISER_IDLE_TIMEOUT` for one-shot, shell-scoped use,
  following the existing `ACCELERATOR_VISUALISER_BIN` pattern.
- Precedence: env var > config key (`visualiser.idle_timeout`) > built-in
  default (8h).
- Support a disable value (`"never"` or `0`, matched case-insensitively) that
  turns off idle auto-shutdown entirely. The server still exits on
  launching-process exit and on explicit `stop`.
- Reject an unparseable or invalid timeout value at launch with a clear error
  (fail fast) rather than silently falling back to the default.

### Out of scope

- A CLI flag on `/accelerator:visualise` — configuration is via the
  `visualiser:` block and the env-var override only.
- Changing the other two shutdown triggers (launching-process exit, explicit
  `stop`).

## Acceptance Criteria

Timeout-boundary criteria are verified against an injectable/shortened clock
(see Technical Notes) rather than by waiting the full real-time interval, and
"shuts down after D" means "shuts down within a small tolerance of D" (e.g.
±5s of the configured boundary).

- [ ] Given no timeout is configured, when the server starts and sits idle,
  then it auto-shuts-down at the 8-hour boundary (verified via the injectable
  clock, within tolerance).
- [ ] Given `visualiser.idle_timeout: "30m"` configured and no env-var
  override, when the server starts and sits idle, then it shuts down at the
  30-minute boundary — i.e. the configured value beats the 8h default
  (config-over-default precedence).
- [ ] Given a compound duration `visualiser.idle_timeout: "1h30m"`, when the
  server starts and sits idle, then it shuts down at the 90-minute boundary
  (verified via the injectable clock, within tolerance) — confirming
  compound-duration parsing.
- [ ] Given `ACCELERATOR_VISUALISER_IDLE_TIMEOUT=2h` is set and a different
  `visualiser.idle_timeout` in config, when the server starts, then the 2h
  env-var value takes effect (env-over-config precedence).
- [ ] Given `visualiser.idle_timeout: "never"` (or `0`), when the server is
  idle for an interval exceeding the previous 30-minute default (verified via
  the injectable clock), then it does not auto-shut-down — but still exits
  when the launching process exits or `stop` is invoked.
- [ ] Given a mixed-case disable token (e.g. `visualiser.idle_timeout:
  "Never"`), when the server starts, then idle auto-shutdown is disabled —
  confirming case-insensitive matching.
- [ ] Given the numeric disable token `visualiser.idle_timeout: 0`, when the
  server is idle for an interval exceeding the previous 30-minute default
  (verified via the injectable clock), then it does not auto-shut-down — i.e.
  `0` disables rather than meaning "shut down immediately".
- [ ] Given an unparseable value (e.g. `visualiser.idle_timeout: "soon"`),
  when the server starts, then it exits with a clear error naming the bad
  value and does not start with a silent default.

## Open Questions

- Should the disable token set be broadened beyond `"never"` and `0` to also
  accept `"off"` / `"none"`? (Current Assumption: no.)
- Are the env-var name (`ACCELERATOR_VISUALISER_IDLE_TIMEOUT`) and config key
  (`visualiser.idle_timeout`) final, or are these provisional names to be
  confirmed at implementation?

## Dependencies

- Related: visualiser server lifecycle (auto-exit on idle / launching-process
  exit / explicit stop).
- Builds on the existing idle-timeout activity tracker in
  `server/src/activity.rs` (located via work item 0055).
- Introduces a new third-party crate dependency on the server for
  duration-string parsing (`humantime` or equivalent) — visible at planning
  time for supply-chain/licensing review.
- Coordinates with any user-facing configuration documentation: the new
  `visualiser.idle_timeout` key and the changed default (30m → 8h) may need
  reflecting in a config reference, schema, or changelog. (If no such artefact
  exists, this entry confirms the check was made.)

## Assumptions

- Disable tokens are `"never"` and `0`, matched case-insensitively. If
  `"off"`/`"none"` should also be accepted, this needs broadening.
- An invalid or unparseable timeout value fails fast at launch rather than
  warning and falling back — consistent with treating committed config as
  code.

## Technical Notes

- The idle tracker lives in `server/src/activity.rs` (an `AtomicI64`
  request-middleware timestamp). The configured duration feeds the
  idle comparison there. That comparison is the natural verification seam:
  injecting a short timeout or a controllable clock lets the boundary
  criteria be tested without a real-time 8-hour wait.
- Duration-string parsing: the `humantime` crate is the natural fit for the
  Rust server (parses `"8h"`, `"30m"`, `"1h30m"`).
- Env-var name (`ACCELERATOR_VISUALISER_IDLE_TIMEOUT`) and config key
  (`visualiser.idle_timeout`) are chosen by analogy with existing overrides;
  final naming is an implementation call.

## Drafting Notes

- Interpreted "follow existing" as the `visualiser:` config-key plus env-var
  override pattern (mirroring `ACCELERATOR_VISUALISER_BIN`), with
  env > config > default precedence and no CLI flag.
- Chose the env-var name and config key by analogy; flagged in Technical
  Notes as an implementation-level call.

## References

- Related: 0055 (`meta/work/0055-sidebar-activity-feed.md`) — locates the
  existing idle-timeout subsystem at `server/src/activity.rs`.
