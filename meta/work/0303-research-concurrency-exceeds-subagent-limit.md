---
type: "work-item"
id: "0303"
title: "Default Research Concurrency Exceeds the Subagent Limit"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "bug"
priority: "high"
parent: "work-item:0121"
relates_to: ["work-item:0282", "work-item:0283"]
tags: ["research", "deep-research", "conduct"]
last_updated: "2026-10-09T17:58:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0303: Default Research Concurrency Exceeds the Subagent Limit

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

`research-topic conduct` defaults `research.topic.concurrency` to 24, but
Claude Code refuses more than 20 concurrent subagents, so every full batch
loses 4 spawns that become permanent `unfinished` failures and whose deeper
levels are never researched. Default to 20, bound the knob by the harness
cap, and tell the skill how to recover a capacity-refused spawn.

## Context

The skill says to spawn "exactly the offered spawns, never more: every Task
call issued together in one message". The run ledger records the whole batch
as offered and never offers a spawn twice. In a `--depth 3` run on
`1.24.0-pre.74` with default config, 5 of 7 batches had 4 spawns refused
with "Concurrent subagent limit reached. You can run 20 subagents at once.
Do not retry." The orchestrator only avoided losing them by deviating from
the skill and starting them as slots freed.

The default comes from the config catalogue. The skill resolves the knob in
prose, with an explicit "There is no upper bound", and passes it to
`outstanding` as `--limit`.

## Requirements

Reproduction:
1. Default config (no `research.topic.concurrency` override, no
   `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`).
2. `conduct <slug> --depth 3` on a brief whose batches exceed 20 spawns.

Expected: every offered spawn in a batch runs.
Actual: 4 spawns per full batch are refused; on re-plan they return as
`unfinished` ("wrote no note") and their subtrees are never researched.

Fix:
- The built-in default for `research.topic.concurrency` is 20.
- The skill's knob-resolution rule bounds concurrency by the harness
  subagent cap: `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` when set, otherwise
  20, read at invocation via the `!` preprocessor. A resolved value above
  the cap clamps to it with a warning.
- When the harness refuses a spawn for capacity, the orchestrator starts it
  once a slot frees, still within the same batch, before re-planning — it
  never re-plans with that spawn unfinished.
- The concurrency knob's documentation states the harness cap and the clamp.

## Acceptance Criteria

- [ ] Given no concurrency override, when the config catalogue is queried for
      `research.topic.concurrency`, then it returns 20.
- [ ] Given `research.topic.concurrency: 24` and no
      `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, when `conduct` resolves its
      knobs, then concurrency is 20 and the skill warns, naming both the
      configured value and the cap.
- [ ] Given `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=30` and
      `research.topic.concurrency: 24`, when `conduct` resolves its knobs,
      then concurrency is 24 with no warning.
- [ ] Given `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=8` and no concurrency
      override, when `conduct` resolves its knobs, then concurrency is 8 with
      a warning.
- [ ] Given `--concurrency 50` on the invocation, when `conduct` resolves its
      knobs, then the flag value is clamped to the cap like a configured one.
- [ ] Given a batch where the harness refuses a spawn with "Concurrent
      subagent limit reached", when a slot frees, then the orchestrator
      starts that spawn before calling `outstanding --spawned`, and the
      re-plan does not list it as `unfinished`.
- [ ] The concurrency knob's documentation states the harness cap, the env
      var, and the clamp.

## Open Questions

- Does the `!` preprocessor's shell see `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`
  when it is set via `settings.json` `env` rather than the launching shell?
  If not, the clamp misses user-raised caps and falls back to 20.

## Dependencies

- Blocked by: none known
- Blocks: none known

## Assumptions

- Re-spawning a capacity-refused spawn within the same batch is consistent
  with the run ledger: the spawn was already offered and has not yet been
  reported via `--spawned`.

## Technical Notes

- Default: `cli/config/src/catalogue.rs:393`, plus its pinned test at
  `cli/config/src/catalogue.rs:541`.
- Knob resolution and the "no upper bound" rule:
  `skills/research/research-topic/SKILL.md:59-87`.
- Batch spawn and re-plan: `skills/research/research-topic/SKILL.md:234-253`.
- Clamping in skill prose leaves `outstanding --limit` unbounded; a caller
  bypassing the skill can still pass more than 20, which is acceptable since
  the skill is the only orchestrator.

## Drafting Notes

- Clamping in the skill, not in `outstanding`, was a user decision — it
  keeps the Claude Code env var out of the CLI.
- Capacity-refusal recovery was explicitly put in scope by the user to cover
  a runtime cap lower than the resolved value.
- The clamp replaces "There is no upper bound" for concurrency only; breadth
  and depth remain unbounded.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/140
- Related: 0121, 0282, 0283
