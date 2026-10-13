---
type: "pr-description"
id: "158"
title: "[0303] Clamp research concurrency to the subagent cap"
date: "2026-10-12T02:54:28+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0303"
parent: "work-item:0303"
relates_to: ["work-item:0330", "work-item:0331", "work-item:0310", "work-item:0307"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/158"
pr_number: 158
tags: ["research", "research-topic", "conduct", "concurrency"]
revision: "63ce5a0fcc5cf3edbbcb576d49956cb317cce30c"
repository: "accelerator"
last_updated: "2026-10-13T09:18:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0303] Clamp research concurrency to the subagent cap

## Summary

`research-topic conduct` offered batches of up to 24 spawns, but Claude Code
starts at most 20 subagents at once. In every full batch, 4 spawns were
refused and lost. This PR lowers the default to 20 and moves concurrency
resolution from skill prose into `accelerator research topic outstanding`,
which clamps the value to the harness's subagent cap. It also teaches
`conduct` to re-issue spawns the harness refuses for capacity.

## Changes

- **Default of 20.** The `research.topic.concurrency` default drops from 24
  to 20 in the config catalogue, the dump golden and the configure skill's
  table. A test ties the default to `HARNESS_DEFAULT` in
  `research-cli/src/subagent_cap.rs`, so if the harness cap changes, someone
  has to decide the default again.
- **Domain model** (`cli/research/src/conduct/concurrency.rs`):
  - `SubagentCap` holds the cap's slots; reading a setting is the CLI's job.
  - `RequestedConcurrency` is either a value `Given` by the flag or a config
    file, which `Provenance` names (`Flag`, or
    `ConfigFile(Team | Personal)`), or the `BuiltInDefault`. `resolve`
    validates a given value under 0282's positive-integer rule, then clamps
    it to the cap. The built-in default clamps without a warning.
  - `Concurrency` can be built only through resolution, so the spawn window
    can no longer be called without a bound.
- **`outstanding` resolves concurrency.**
  - ⚠️ `--limit` is renamed `--concurrency`, and `E_TOPIC_RESEARCH_LIMIT`
    gives way to `E_TOPIC_RESEARCH_CONCURRENCY`. When the flag is absent,
    `outstanding` falls back to `research.topic.concurrency`.
  - ⚠️ A malformed value no longer exits 1. Under the 0282 knob rule it
    clamps to 1 with a warning. Only an empty value exits 1, and in that
    case no ledger is written.
  - Every plan, plain queries included, reports the resolved value as
    `concurrency` and the cap as `subagent_cap`. The cap carries `slots`,
    and its `source`: the variable's name, or `default`.
  - Clamp and invalid-value warnings are raised on `--start` only, ahead of
    the replaced-ledger notice. Each names the flag, or the config file
    (`.accelerator/config.md` or `.accelerator/config.local.md`) that set
    the value. A clamp also names where the cap came from and how to raise
    it. The wording lives in `research-cli/src/concurrency_wording.rs`.
  - The set now resolves before the concurrency, so an unknown slug is
    reported first. A config error raised while resolving the set used to
    surface as `E_TOPIC_RESEARCH_UNRESOLVED`; it now reports as itself.
  - `--help` gives both defaults of 20, and says how to raise the cap.
- **The cap comes from the environment.**
  - `research-cli/src/subagent_cap.rs` reads `ACCELERATOR_SUBAGENT_CAP`,
    then `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, else 20.
  - Each setting is parsed exactly as Claude Code's JavaScript parser
    parses it: `String.prototype.trim` whitespace, an optional sign, ASCII
    digits, and a value of at least 1.
  - A value the harness would ignore falls through to the next variable. A
    set `ACCELERATOR_SUBAGENT_CAP` that falls through warns on `--start`.
  - An earlier design passed the cap as `--subagent-cap "$CLAUDE_…"`.
    Claude Code blocks auto-approval of any Bash command that expands a
    shell variable, so the CLI now reads the cap itself.
- **Coupling guard admits the cap by name.** 🔒 `lint:claude-coupling:check`
  admits `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` only in its reader and that
  reader's integration test. A line naming any other `CLAUDE_*` variable
  still fails, a test pins the admitted set, and no file is exempt as a
  whole. ADR-0071 (`accepted`) records the rule: an admitted variable must
  be optional and have a stated default, and records why an
  Accelerator-only variable was rejected.
- **Skill prose.**
  - `conduct` passes the invocation's `--concurrency` through unchanged, in
    every plan including the tick plan, and no longer resolves concurrency
    itself. It reports the first plan's `warnings` as soon as they return.
  - Step 4 re-issues capacity-refused spawns in rounds, before step 5's
    check and before any re-plan.
  - A spawn still refused once retrying stops is recorded once, with a new
    failure reason. Its remedy row orders three checks.
- **Docs and changelog.**
  - The configure skill, `docs-site/.../research.md` and the unreleased
    `CHANGELOG.md` entries describe the cap, its precedence and the clamp.
  - The configure skill points harnesses with no cap (Claude Code below
    v2.1.217, and ultracode sessions) at `ACCELERATOR_SUBAGENT_CAP`.
  - The skill and the CHANGELOG describe the retry rounds; the docs-site
    page covers only the CLI.
- **Process.** The repository's `implement-plan` instructions now ask for
  small, focused commits.

## Context

- Work item: `meta/work/0303-research-concurrency-exceeds-subagent-limit.md`
- Plan: `meta/plans/2026-10-10-0303-research-concurrency-subagent-limit.md`,
  with its review under `meta/reviews/plans/`
- Codebase research: `meta/research/codebase/2026-10-10-0303-research-concurrency-subagent-limit.md`
- Validation (result `pass`): `meta/validations/2026-10-10-0303-research-concurrency-subagent-limit-validation.md`
- Decision: `meta/decisions/ADR-0071-optional-harness-variables-in-the-cli.md`
- PR review: `meta/reviews/prs/158-review-1.md`. The responses from round 1
  are on this PR.
- Follow-ups captured during the manual runs:
  - 0330: a skill's `allowed-tools` grant lapses once a background
    subagent's completion opens a new query;
  - 0331.

## Testing

- [x] `mise run` exits 0 end to end on `63ce5a0f`, docs lane included
- [x] Unit tests in `subagent_cap.rs` cover every cap-parser case:
      whitespace, U+FEFF, U+0085, signs, non-ASCII digits, zero-padding and
      `u32` overflow
- [x] Integration tests over the `outstanding` binary clear both cap
      variables on every call. They cover:
  - team and personal config, with the file named in each diagnostic;
  - exact `warnings` and stderr, and `--start`-only warnings;
  - offered spawns and `remaining` under each clamp;
  - the empty-value exit;
  - set resolution ahead of concurrency, and a config error during set
    resolution;
  - the cap's wiring, with two values each way.
- [x] Coupling-guard tests cover admission by name and file, a banned name
      on the same line, the pinned admitted set, and the absence of each
      admitted file
- [x] Structural tests over the skill cover:
  - flag pass-through, and no shell variable in any plan;
  - the retry-round wording;
  - the reason row and the order of its checks;
  - the configure skill's sentence for harnesses with no cap;
  - knob-rule wording kept in step with the CLI.
- [x] Manual, Claude Code 2.1.296, run before review round 1: the cap
      reaches the CLI from an exported shell variable and from project,
      local and user `settings.json`. `ACCELERATOR_SUBAGENT_CAP` wins over
      Claude Code's variable, and no `accelerator` command triggered a
      permission denial.
- [x] Manual, Claude Code 2.1.296, run before review round 1:
  - full load: 108 spawns, batches of exactly 20, no capacity refusal;
  - partial hold: rounds accept one spawn each, and nothing is recorded as
    failed;
  - full hold: one fully refused round, each spawn recorded once, and a
    fresh run offers the spawns again.

## Notes for Reviewers

- ⚠️ A plain `outstanding` query, which is how `conduct` ticks the outline,
  now reads config and is bounded by the concurrency. With an empty
  `research.topic.concurrency` it exits 1, naming the file, where it used
  to ignore the value. Ticking itself is unaffected, because
  `items[].complete` comes from the round plan, not the spawn window.
- ⚠️ The clamp applies on every harness version and mode, including
  Claude Code before 2.1.217 and ultracode sessions, neither of which
  enforces a cap. A project that configured more than 20 gets a clamp
  warning until it raises `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, or, on a
  harness with no cap, sets `ACCELERATOR_SUBAGENT_CAP`.
- The manual runs predate review round 1. Round 1 changed the diagnostics'
  wording, the plan's `subagent_cap` field and the order of set
  resolution. It did not change spawning or retry behaviour.
- The retry recognises the harness's refusal by meaning; "Concurrent
  subagent limit reached" is today's text. After a harness upgrade,
  recheck that text and the cap parser.
- Worth a close look: the admission logic in `tasks/lint/claude_coupling.py`,
  and ADR-0071, accepted after its review in round 1.
- Out of scope: stopping retry rounds early on a halting failure belongs to
  0310. Making 0307's node query answer for re-issued spawns belongs to
  0307.
