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
relates_to: ["work-item:0282", "work-item:0283", "work-item:0304", "work-item:0307", "work-item:0310"]
tags: ["research", "deep-research", "conduct"]
last_updated: "2026-10-10T21:22:58+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-889"
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
levels are never researched. Default to 20, have `outstanding` resolve the
knob and bound it by the harness cap, and tell the skill how to recover a
capacity-refused spawn.

## Context

"The harness" here is the Claude Code runtime that runs the skill and its
subagents. A full batch is a batch of `concurrency` spawns. A capacity
refusal is the harness declining to start a spawn because the cap is
reached; it is distinct from an agent that starts and declines its task.
The run ledger, offered spawns, `outstanding --spawned` and `unfinished`
are defined in the 0283 plan
(`meta/plans/2026-09-26-0283-recursive-finding-deepening.md`). The re-plan
is the `outstanding --spawned` call that follows step 5.

The skill's spawn step (step 4) says to spawn "exactly the offered spawns,
never more: every Task call issued together in one message". The run ledger
records the whole batch as offered and never offers a spawn twice within
one run; a fresh `conduct` run starts a new ledger. In a
`--depth 3` run on `1.24.0-pre.74` with default config, 5 of 7 batches had
4 spawns refused with "Concurrent subagent limit reached. You can run 20
subagents at once. Do not retry." The orchestrator only avoided losing them
by deviating from the skill and starting them as slots freed.

The default comes from the config catalogue. The skill resolves the knob in
prose, with an explicit "There is no upper bound", and passes it to
`outstanding` as `--limit`.

## Requirements

Reproduction:
1. Claude Code v2.1.217 or later, which enforces the 20-subagent cap.
2. Default config (no `research.topic.concurrency` override, no
   `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`).
3. No other subagents or forks running in the session.
4. `conduct <slug> --depth 3` on a brief whose batches exceed 20 spawns,
   such as the brief from the original run (issue #140).

Expected: every offered spawn in a batch runs.
Actual: 4 spawns per full batch are refused; on re-plan they return as
`unfinished` ("wrote no note") and their subtrees are never researched.

Fix:
- The built-in default for `research.topic.concurrency` is 20.
- `accelerator research topic outstanding` resolves concurrency itself, and
  the skill no longer resolves it in prose:
  - An optional `--concurrency N` flag replaces `--limit N`. The flag wins;
    otherwise the CLI reads `research.topic.concurrency` from config, noting
    whether the value came from a config file or the built-in default.
  - The 0282 knob validation runs first, now in the CLI for concurrency: an
    empty value exits 1 with `E_TOPIC_RESEARCH_CONCURRENCY`, and a zero,
    negative or non-integer value clamps to 1 with a warning.
  - The value it produces is then compared with the harness subagent cap:
    `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` when set, otherwise 20. The cap
    is parsed as the harness parses it: trimmed, matching `^[+-]?\d+$`, and
    at least 1; any other value is ignored silently and the cap is 20.
  - A value above the cap clamps to it, with a warning when the value came
    from the flag or a config file and silently when it is the built-in
    default. A value equal to the cap is not clamped.
  - Concurrency warnings join the plan's `warnings` and stderr on `--start`
    only, so a run's re-plans never repeat them. Every plan reports the
    resolved value as `concurrency`.
  - `conduct` passes the invocation's `--concurrency N` through to every
    `outstanding` plan unchanged, or omits it, and reports the first plan's
    warnings as soon as they appear.
- When the harness capacity-refuses spawns with "Concurrent subagent limit
  reached", the spawn step (step 4) re-issues them before the per-spawn
  check (step 5) and before re-planning. A round is one follow-up message
  of re-issued spawns; the original batch message is not a round. Like any
  batch message, a round returns once its accepted spawns have finished.
  - The first round always runs, carrying every spawn the batch message
    capacity-refused, with no added delay or sleep.
  - Each later round re-issues the spawns the latest round capacity-refused,
    and runs only when that latest round had at least one of its spawns
    accepted.
  - The skill states that this retry deliberately overrides the harness's
    "Do not retry".
  - Once a round has every spawn capacity-refused, retrying stops. Each
    spawn still capacity-refused is recorded as failed in the batch's
    failure record, the same record step 5 writes, with the reason
    `refused by the concurrent subagent limit — other subagents held the slots, or the harness cap is below the resolved concurrency`.
  - The re-plan then reports such a spawn as `unfinished`; the orchestrator
    does not record that entry as a second failure.
- Step 5's no-file case already lists "a refusal" among its reasons,
  meaning an agent that declined its task. The change: it states that a
  capacity refusal is not one of these and is handled by the spawn step.
- The failure-reason table gains a row for the exhaustion reason, whose next
  step is: re-run `conduct` once other subagents have finished, and the new
  run offers the spawn again; if it recurs with no other subagents running,
  make sure `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` is set where both Claude
  Code and its Bash tool see it, or lower the concurrency below the
  `concurrency` the summary reports, with a smaller `--concurrency` or
  `research.topic.concurrency`. The final summary states the `concurrency`
  the plans reported beside any such failure.
- The changelog records the new default, the clamp and the `--limit` to
  `--concurrency` rename beside the knob's and the run flags' original
  entries, and the golden dump fixture pins the default of 20.
- The concurrency knob's documentation states the default of 20, that the
  cap comes from `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` or is otherwise 20,
  that a configured or flag value above the cap clamps with a warning, and
  that the built-in default clamps silently. The `outstanding` reference in
  `docs-site/src/content/docs/research.md` documents `--concurrency`, the
  `concurrency` field and `E_TOPIC_RESEARCH_CONCURRENCY` in place of
  `--limit` and `E_TOPIC_RESEARCH_LIMIT`.

## Acceptance Criteria

"SKILL.md" alone means `skills/research/research-topic/SKILL.md`. "Plans
with `--start`" means `accelerator research topic outstanding ... --start`.
The criteria are verified in three ways:
- Rust tests pin concurrency resolution: unit tests over the domain rule
  and integration tests over the `outstanding` binary, which set
  `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` and config files explicitly.
- Structural tests pin the skill prose for the spawn step, step 5, the
  reason table and the configure docs. This is the only check for the
  retry criteria the manual runs do not exercise, an accepted risk because
  the retry lives in skill prose, not code.
- The manual runs exercise the behaviour end to end: the cap reaching the
  CLI from the session, plus the full-load, partial-hold and full-hold runs.

Each concurrency diagnostic names its `{source}`: `--concurrency` when the
value came from the flag, otherwise `research.topic.concurrency`. The clamp
warning reads
`{source} is {value}, above the subagent cap of {cap} — clamping to {cap}`
and the invalid-value warning reads
`{source} must be a positive integer, got '{value}' — clamping to 1`.
The empty-value error names its source the same way.
Each appears verbatim in the plan's `warnings` and, prefixed `warning: `, on
stderr.

- [ ] Given no concurrency override, when the config catalogue is queried for
      `research.topic.concurrency`, then it returns 20.
- [ ] Given `research.topic.concurrency: 24` and no
      `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, when `outstanding` plans with
      `--start`, then `concurrency` is 20 and the clamp warning names the
      value 24 and the cap 20.
- [ ] Given `research.topic.concurrency: 20` and no
      `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, when `outstanding` plans with
      `--start`, then `concurrency` is 20 with no warning.
- [ ] Given `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=30` and
      `research.topic.concurrency: 24`, when `outstanding` plans with
      `--start`, then `concurrency` is 24 with no warning.
- [ ] Given `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=8` and no concurrency
      override, when `outstanding` plans with `--start`, then `concurrency`
      is 8 with no warning, because only an explicitly set value warns when
      clamped.
- [ ] Given `--concurrency 50` and no
      `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, when `outstanding` plans with
      `--start`, then `concurrency` is 20 and the clamp warning names
      `--concurrency`, the value 50 and the cap 20.
- [ ] Given `--concurrency 50` and `research.topic.concurrency: 6`, when
      `outstanding` plans with `--start`, then the flag wins and the clamp
      warning names `--concurrency` and the value 50.
- [ ] Given `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` set to `007`, `+8`, or
      ` 8 `, and no concurrency override, when `outstanding` plans with
      `--start`, then `concurrency` is 7, 8, or 8 respectively, matching the
      harness parser.
- [ ] Given `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` set to a value the harness
      ignores (`0`, `-5`, `abc`, `20.5`, `1e2`, `1,000`, empty) and no
      concurrency override, when `outstanding` plans with `--start`, then
      `concurrency` is 20 and no warning of any kind is emitted.
- [ ] Given `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` set to each of those
      ignored values and `research.topic.concurrency: 24`, when
      `outstanding` plans with `--start`, then `concurrency` is 20 and the
      clamp warning names the cap 20. A misparse such as reading `1e2` as
      100 would give 24 with no warning.
- [ ] Given `--concurrency 0`, when `outstanding` plans with `--start`, then
      `concurrency` is 1 with the invalid-value warning naming
      `--concurrency` and `'0'`.
- [ ] Given `--concurrency 50.5` and no
      `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, when `outstanding` plans with
      `--start`, then `concurrency` is 1 with the invalid-value warning and
      no clamp warning, showing validation ran before the cap clamp.
- [ ] Given `research.topic.concurrency` set to an empty value, or
      `--concurrency ""`, when `outstanding` plans, then it exits 1 with
      `E_TOPIC_RESEARCH_CONCURRENCY` naming the empty source, and prints no
      plan.
- [ ] Given `research.topic.concurrency: 24` and no
      `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, when `outstanding` continues a
      run with `--run` and `--spawned`, or plans with neither run flag, then
      `concurrency` is 20 and no concurrency warning is emitted.
- [ ] Given a set with 25 outstanding spawns and no concurrency override,
      when `outstanding` plans with `--start`, then it offers 20 spawns and
      reports `remaining` 5.
- [ ] Given `outstanding --limit 2`, when it runs, then it exits non-zero:
      `--limit` no longer exists.
- [ ] Given a batch where the harness refuses spawns with "Concurrent
      subagent limit reached", including a batch where it refuses every
      spawn, when the batch returns, then the orchestrator re-issues the
      refused spawns in a follow-up message within step 4, before the step 5
      check and before calling `outstanding --spawned`.
- [ ] Given a round in which some spawns are accepted and some refused,
      when that round returns, then the refused ones are re-issued in a
      further round.
- [ ] Given a spawn accepted on re-issue that writes a valid file, when
      step 5 checks the batch, then it is not recorded as failed and the
      re-plan does not list it as `unfinished`.
- [ ] Given a round in which every spawn is refused, when that round
      returns, then retrying stops with no added delay, each remaining
      spawn is recorded as failed once with the reason
      `refused by the concurrent subagent limit — other subagents held the slots, or the harness cap is below the resolved concurrency`,
      and the re-plan's `unfinished` entry for it adds no second failure.
- [ ] Given a run in which a spawn was recorded as failed with the capacity
      reason, when `conduct <slug>` is run again, then that spawn is offered
      again.
- [ ] SKILL.md no longer injects or validates concurrency, and a structural
      test in `tests/unit/tasks/test_research_structure.py` pins it: the
      knob block carries no `research.topic.concurrency` `!` line, the
      knob-resolution rule states that `outstanding` resolves concurrency
      and bounds it by the subagent cap, `conduct` passes
      `{concurrency flag}` (the invocation's `--concurrency N`, or nothing)
      to every plan, and no `--limit` appears in `conduct`. The
      misplaced-flag rule still names `--concurrency` on `outline`.
- [ ] SKILL.md's spawn step states, and a structural test pins: the first
      round always runs; later rounds run while the latest round accepted
      at least one spawn; the retry explicitly overrides the harness's "Do
      not retry"; step 5's refusal case excludes capacity refusals; and the
      exhaustion reason verbatim. Beyond the verbatim reason, the test
      asserts these sentences inside the spawn step:
      `This retry deliberately overrides the harness's "Do not retry".`,
      `The first round always runs.` and
      `A later round runs only when the latest round had at least one spawn accepted.`
      It also asserts the sentence
      `A capacity refusal is not one of these; the spawn step handles it.`
      inside step 5.
- [ ] SKILL.md's failure-reason table has a row for the exhaustion reason
      whose next step re-runs `conduct`, and a structural test pins it.
- [ ] `skills/config/configure/SKILL.md` states, and a structural test
      pins: the default of 20, `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` as the
      cap's source with 20 otherwise, and that values above the cap clamp.
- [ ] `CHANGELOG.md` records the default of 20 and the clamp, and the golden
      dump fixture pins 20.

Each manual run captures its evidence from the session transcript:
- the `concurrency` each `outstanding` plan reports;
- the spawns each `outstanding` plan offers, per batch;
- where each round sits relative to step 5's checks and the re-plan's
  `outstanding --spawned` call;
- every warning `conduct` prints;
- the harness's refusal results for the batch message and each round;
- the failure list in `conduct`'s final summary;
- the re-plan's `unfinished` list.

A holder is a background subagent told to stay running until a named stop
file exists; the run removes the holders by creating that file. Before a
hold run, the tester issues a probe spawn to validate the fixture: with the
holders running, it is accepted in the partial-hold run and
capacity-refused in the full-hold run. The probe must finish before
`conduct` starts, so it holds no slot during the run. The tester starts the
holders, issues the probe, runs `conduct` and creates the stop file, all in
one session. A run whose probe gives the other result, or whose first batch
offers fewer spawns than the run requires, is invalid rather than failed.

- [ ] Manual run, cap reaches the CLI: run `conduct` under each
      configuration below and check the first plan's `concurrency` and the
      warnings `conduct` reports. Each run may stop once its first
      `outstanding` call and any warnings have appeared.
      - `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=8` exported in the launching
        shell, no override: 8, no warning.
      - `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=8` in the project
        `.claude/settings.json` `env`, no override: 8, no warning.
      - env unset, `--concurrency 50`: 20, a clamp warning naming 50 and
        cap 20, reported before the first batch is spawned.
- [ ] Manual run, full load: with default config and no holders, run
      `conduct <slug> --depth 3` on the brief from the original run (issue
      #140), or any brief whose batches offered more than 20 spawns before
      the fix. At least one batch offers exactly 20 spawns, all accepted by
      the batch message; a run with no such batch is invalid rather than
      passed. Every plan reports `concurrency` 20, no capacity refusal
      appears, and no `unfinished` entry follows a capacity refusal.
- [ ] Manual run, partial hold: with
      `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=4` and 3 holders running, run
      `conduct <slug> --depth 2` on a brief whose first batch offers 4
      spawns. The batch
      message has 1 spawn accepted and 3 refused. Rounds then accept 1 spawn
      each until none is left. The final summary has no capacity failure,
      and the re-plan lists none of the re-issued spawns as `unfinished`.
- [ ] Manual run, full hold: with `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=4`
      and 4 holders running, run `conduct <slug> --depth 2` on the same
      brief. The batch message and exactly
      one round are fully refused. The final summary lists each spawn once
      with the exhaustion reason. After the holders stop, a fresh `conduct`
      run offers them again.

## Dependencies

- Blocked by: none known
- Blocks: none known
- External: the Claude Code harness's concurrent-subagent contract. It has
  three parts: the default cap of 20 (enforced from v2.1.217), the
  `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` parser, and the refusal text
  "Concurrent subagent limit reached". All three were verified on 2.1.296.
  A change to the cap or the parser requires revisiting the clamp. A change
  to the refusal text silently disables the retry.
- Relates to: 0310. Both edit step 5 and the failure-reason table.
  Capacity exhaustion is not a halting failure: it is transient and
  confined to the slots other subagents hold. Either item may land first;
  the second reconciles the table. Retry rounds stop early once any spawn
  the batch message or a round accepted ended with a halting failure, and
  the spawns still capacity-refused are recorded with the capacity reason.
  Whichever item lands second adds that rule.
- Relates to: 0304. Both add a row to the failure-reason table. Once 0304
  joins 0310's halting-failure class, a WebSearch quota failure also stops
  retry rounds early.
- Relates to: 0307. Its per-response ceiling and its "current offered
  batch" node query assume step 4 issues a batch in one message. Rounds are
  follow-up messages within the same offered batch. 0307 must make its
  node query answer for spawns re-issued in rounds; 0307's Dependencies
  already record this, and this item needs no change for it.
- Supersedes: 0283's default of 24, its "batches of 24 and 6" example and
  its `--limit` flag on `outstanding`. It also overrides 0282's
  no-upper-bound rule and its prose resolution, for concurrency only;
  breadth and depth stay resolved in prose.

## Assumptions

- Re-spawning a capacity-refused spawn within the same batch is consistent
  with the run ledger: the spawn was already offered and has not yet been
  reported via `--spawned`.
- The harness cap counts every running Agent-tool subagent in the session,
  including background, resumed and forked ones. A fork is a subagent
  spawned with `subagent_type: "fork"`, inheriting the parent's context.
  Once concurrency is clamped, a capacity refusal therefore means other
  subagents hold slots, or the harness parsed the cap differently from the
  clamp. The exhaustion reason names both causes.
- A `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` value below 20 lowers the cap
  and one above raises it. The harness (2.1.296, `M.int({min:1,
  digitsOnly:true})`) trims the value, requires `^[+-]?\d+$` and at least 1,
  and otherwise ignores it, so `007` is 7 and `+8` is 8. A later harness
  changing this parser can make the clamp miss; the retry path is the
  backstop.
- A spawn recorded as failed in one run is offered again by a fresh
  `conduct` run, so rerunning recovers it. This was observed in 0310's
  usage-limit run, where the new run replaced the ledger and offered only
  the failed spawns. The full-hold manual run confirms it for capacity
  failures.
- A capacity refusal returns immediately, so retrying stops at the first
  round with no acceptance rather than after a fixed count. No CLI wait is
  added. The harness blocks a foreground `sleep` command, and a wait would
  sidestep that guard. A fixed delay also cannot know when slots free.
- A batch message, and each round, returns only once its accepted spawns
  have finished, so the slots they held are free when the next round is
  issued. The partial-hold run's expected sequence depends on this.
- The clamp to 20 also applies where the harness has no such cap: Claude
  Code below v2.1.217 (the plugin floor is v2.1.144), and ultracode
  sessions, Claude Code's opt-in multi-agent orchestration mode, which the
  harness exempts from the cap. Throughput there is
  lower, which is accepted.
- The default stays at the cap, with no headroom; the retry path covers
  slots held by other agents.
- The `!` preprocessor sees `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` whether
  it is set in the launching shell, a project `.claude/settings.json`
  `env`, or a `--settings` file's `env`. Verified on Claude Code 2.1.296
  with a probe skill under each source and an unset control. The
  `outstanding` calls run through the Bash tool, not the preprocessor; the
  same sources are assumed to reach them, since the launcher execs
  `accelerator-research` with the inherited environment. The
  cap-reaches-the-CLI manual run confirms this for the shell and project
  settings sources.

## Open Questions

- Can a holder stay running until its stop file appears without tripping
  the foreground-`sleep` guard? The hold runs depend on it; the probe spawn
  detects a holder that has exited.
- Is the brief from the original run (issue #140) still retrievable? If
  not, the full-load run uses any brief whose unclamped batches exceed 20
  spawns.

## Technical Notes

- Default: `cli/config/src/catalogue.rs:393`, plus its pinned test at
  `cli/config/src/catalogue.rs:541`.
- Knob resolution and the "no upper bound" rule:
  `skills/research/research-topic/SKILL.md:59-87`; the plans passing
  `--limit {concurrency}`: `SKILL.md:245,253`.
- `outstanding` flags: `cli/research-cli/src/cli.rs:93-116`, parsed by
  `OutstandingFlags::parse` at `cli/research-cli/src/topic_command.rs:100-131`
  and fed to `research::conduct::window::window` as `limit`. Config
  provenance comes from `ConfigAccess::effective(..).is_from_config()`; the
  environment port is `config::credentials::Environment` with
  `config_adapters::credentials::SystemEnvironment`.
- Spawn step: `skills/research/research-topic/SKILL.md:305-357`. Per-spawn
  check: `skills/research/research-topic/SKILL.md:359-370`. Re-plan:
  `skills/research/research-topic/SKILL.md:372`. Failure-reason table:
  `skills/research/research-topic/SKILL.md:441`.
- Clamping in the CLI bounds every caller of `outstanding`, including a
  hand run, so no plan can offer more spawns than the harness admits.
- Knob documentation: `skills/config/configure/SKILL.md:438`.
- Golden fixture pinning the default:
  `cli/launcher/tests/fixtures/dump/dump.golden:18`.
- Structural tests over the knob prose:
  `tests/unit/tasks/test_research_structure.py:188`.
- Changelog entry beside the knob's original entry at `CHANGELOG.md:139`.
- Harness cap reference: the "Concurrent subagent limit" section of
  https://code.claude.com/docs/en/sub-agents.md.

## Drafting Notes

- Resolving and clamping concurrency in `outstanding`, not in skill prose,
  was a user decision at planning time, reversing the earlier choice to
  keep the Claude Code env var out of the CLI. It turns the harness-parser
  replica and the clamp into code under unit test, and removes the need
  for a non-`accelerator` `!` line with its own permission grant.
- Capacity-refusal recovery was explicitly put in scope by the user. It
  covers slots held by other subagents or forks, and a harness parser that
  disagrees with the clamp.
- The default, the clamp and the retry ship together. The clamp alone still
  loses spawns whenever other subagents hold slots, so shipping it without
  the retry leaves the data loss open.
- The clamp replicates the harness parser exactly rather than accepting any
  positive integer. A divergent parser makes the clamp disagree with the
  real cap, which is the class of bug this item fixes.
- The clamp replaces "There is no upper bound" for concurrency only; breadth
  and depth remain unbounded.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/140
- Related: 0121, 0282, 0283, 0304, 0307, 0310
