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
  20, read at invocation via the `!` preprocessor. The cap is parsed as the
  harness parses it: trimmed, matching `^[+-]?\d+$`, and at least 1;
  any other value is ignored and the cap is 20. The existing knob
  validation runs first; a valid resolved value above the cap then clamps
  to it, with a warning when the value came from a flag or config file and
  silently when it is the built-in default.
- When the harness refuses spawns for capacity, the orchestrator re-issues
  them in a follow-up message within step 4, before the step 5 check and
  before re-planning, for as long as the previous round had at least one of
  them accepted, without waiting between rounds. The skill states that this
  retry deliberately overrides the harness's "Do not retry", and step 5's
  refusal case excludes capacity refusals. Once a round refuses every
  re-issued spawn, each remaining one is recorded as failed, with its
  capacity reason.
- The concurrency knob's documentation states the harness cap and the clamp.

## Acceptance Criteria

The structural tests and the manual reproduction verify this item; the
behavioural criteria are the specification SKILL.md's prose must express.

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
      no warning, because only an explicitly set value warns when clamped.
- [ ] Given `--concurrency 50` on the invocation, when `conduct` resolves its
      knobs, then the flag value is clamped to the cap with a warning, like a
      configured one.
- [ ] Given `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` set to `007`, `+8`, or
      ` 20 `, and no concurrency override, when `conduct` resolves its knobs,
      then the cap is 7, 8, or 20 respectively, matching the harness parser.
- [ ] Given `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` set to a value the harness
      ignores (`0`, `-5`, `abc`, `20.5`, `1e2`, `1,000`, empty) and no
      concurrency override, when `conduct` resolves its knobs, then the cap
      is 20, concurrency is 20, and no warning names the env var.
- [ ] Given `--concurrency 0`, when `conduct` resolves its knobs, then the
      knob-validation rule clamps it to 1 before the cap clamp applies.
- [ ] Given a batch where the harness refuses a spawn with "Concurrent
      subagent limit reached", when the batch returns, then the orchestrator
      re-issues the refused spawns in a follow-up message within step 4,
      before the step 5 check and before calling `outstanding --spawned`,
      and the re-plan does not list them as `unfinished`.
- [ ] Given a retry round in which some re-issued spawns are accepted and
      some refused, when that round returns, then the refused ones are
      re-issued again.
- [ ] Given a spawn accepted on retry that writes a valid file, when step 5
      checks the batch, then it is not recorded as failed.
- [ ] Given a retry round in which every re-issued spawn is refused, when
      that round returns, then retrying stops without waiting, each remaining
      spawn is recorded as failed with the reason
      `refused by the concurrent subagent limit — another subagent or fork held the slots`,
      and the re-plan's `unfinished` entry for it is not recorded a second
      time.
- [ ] SKILL.md's knob-resolution rule states, and a structural test in
      `tests/unit/tasks/test_research_structure.py` pins: the cap and its
      source (`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, otherwise 20), the
      harness's parsing (trimmed, matching `^[+-]?\d+$`, at least 1,
      otherwise ignored with a cap of 20), validation before the cap clamp,
      that clamping the built-in default is silent, and the clamp warning
      verbatim:
      `Warning: research.topic.concurrency is {value}, above the subagent cap of {cap} — clamping to {cap}`.
- [ ] SKILL.md's spawn step states, and a structural test pins: refused
      spawns are retried in follow-up messages within step 4 while the
      previous round accepted at least one of them, this retry explicitly
      overrides the harness's "Do not retry", step 5's refusal case excludes
      capacity refusals, and the exhaustion reason verbatim.
- [ ] The concurrency knob's documentation states the harness cap, the env
      var, and the clamp.
- [ ] Manual reproduction: with default config, `conduct <slug> --depth 3`
      on a brief that produces full batches completes with no spawn refused
      for capacity and no `unfinished` entry caused by a refusal.

## Dependencies

- Blocked by: none known
- Blocks: none known

## Assumptions

- Re-spawning a capacity-refused spawn within the same batch is consistent
  with the run ledger: the spawn was already offered and has not yet been
  reported via `--spawned`.
- The harness cap counts every running Agent-tool subagent in the session,
  so once concurrency is clamped, a capacity refusal means something else
  holds slots (a background or resumed agent, or a lowered cap).
- A `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` value below 20 lowers the cap
  and one above raises it. The harness (2.1.296, `M.int({min:1,
  digitsOnly:true})`) trims the value, requires `^[+-]?\d+$` and at least 1,
  and otherwise ignores it, so `007` is 7 and `+8` is 8. A later harness
  changing this parser can make the clamp miss; the retry path is the
  backstop.
- A spawn recorded as failed in one run is offered again by a fresh
  `conduct` run, so rerunning recovers it. Not yet verified.
- A capacity refusal returns immediately, so retrying stops at the first
  round with no acceptance rather than after a fixed count; no CLI wait is
  added, because it would sidestep the harness's foreground-`sleep` guard
  and a fixed delay cannot know when slots free.
- The clamp to 20 applies even where the harness has no such cap — Claude
  Code below v2.1.217 (the plugin floor is v2.1.144) and ultracode
  sessions — accepting lower throughput there.
- The default stays at the cap, with no headroom; the retry path covers
  slots held by other agents.
- The `!` preprocessor sees `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` whether
  it is set in the launching shell, a project `.claude/settings.json`
  `env`, or a `--settings` file's `env`, so the clamp honours user-raised
  caps. Verified on Claude Code 2.1.296 with a probe skill under each
  source and an unset control.

## Technical Notes

- Default: `cli/config/src/catalogue.rs:393`, plus its pinned test at
  `cli/config/src/catalogue.rs:541`.
- Knob resolution and the "no upper bound" rule:
  `skills/research/research-topic/SKILL.md:59-87`.
- Batch spawn and re-plan: `skills/research/research-topic/SKILL.md:234-253`.
- Clamping in skill prose leaves `outstanding --limit` unbounded; a caller
  bypassing the skill can still pass more than 20, which is acceptable since
  the skill is the only orchestrator.
- Knob documentation: `skills/config/configure/SKILL.md:438`.
- Golden fixture pinning the default:
  `cli/launcher/tests/fixtures/dump/dump.golden:18`.
- Structural tests over the knob prose:
  `tests/unit/tasks/test_research_structure.py:188`.
- Changelog entry beside the knob's original entry at `CHANGELOG.md:139`.
- Harness cap reference: the "Concurrent subagent limit" section of
  https://code.claude.com/docs/en/sub-agents.md.

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
