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
revision: "bd078d35facd3670e3d97e6bd0ee0ac7a774e9c5"
repository: "accelerator"
last_updated: "2026-10-12T02:54:28+00:00"
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
  table. A test ties the default to `SubagentCap::HARNESS_DEFAULT`, so if
  the harness cap changes, someone has to decide the default again.
- **Domain model** (`cli/research/src/conduct/concurrency.rs`):
  - `SubagentCap` parses a cap setting exactly as Claude Code's JavaScript
    parser does: `String.prototype.trim` whitespace, an optional sign, ASCII
    digits, and a value of at least 1.
  - `RequestedConcurrency::resolve` validates the requested value under
    0282's positive-integer rule, then clamps it to the cap. The warning it
    raises records whether the value came from a flag or a config file. The
    built-in default clamps without a warning.
  - `Concurrency` can be built only through resolution, so the spawn window
    can no longer be called without a bound.
- **`outstanding` resolves concurrency.**
  - ⚠️ `--limit` is renamed `--concurrency`. When the flag is absent,
    `outstanding` falls back to `research.topic.concurrency`.
  - Every plan reports the resolved value as `concurrency`, plain queries
    included.
  - Clamp and invalid-value warnings are raised on `--start` only, ahead of
    the replaced-ledger notice.
  - An empty value exits 1 with `E_TOPIC_RESEARCH_CONCURRENCY`, and no
    ledger is written.
- **The cap comes from the environment.**
  - `research-cli/src/subagent_cap.rs` reads `ACCELERATOR_SUBAGENT_CAP`,
    then `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, else 20.
  - A value the harness would ignore falls through to the next variable.
  - An earlier design passed the cap as `--subagent-cap "$CLAUDE_…"`, but
    Claude Code blocks auto-approval of any Bash command that expands a
    shell variable, so the CLI now reads the cap itself.
- **Coupling guard admits the cap by name.** 🔒 `lint:claude-coupling:check`
  admits `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` only in its reader and that
  reader's integration test. A line naming any other `CLAUDE_*` variable
  still fails, the admitted set is pinned by a test, and no file is exempt
  as a whole. ADR-0071 (`proposed`) records the rule: an admitted variable
  must be optional and have a stated default.
- **Skill prose.**
  - `conduct` passes the invocation's `--concurrency` through unchanged, in
    every plan including the tick plan, and no longer resolves concurrency
    itself.
  - Step 4 re-issues capacity-refused spawns in rounds, before step 5's
    check and before any re-plan.
  - A spawn still refused once retrying stops is recorded once, with a new
    failure reason and remedy row.
- **Docs and changelog.** The configure skill, `docs-site/.../research.md`
  and the unreleased `CHANGELOG.md` entries describe the cap, its
  precedence, the clamp and the retry rounds.
- **Process.** The repository's `implement-plan` instructions now ask for
  small, focused commits.

## Context

- Work item: `meta/work/0303-research-concurrency-exceeds-subagent-limit.md`
- Plan: `meta/plans/2026-10-10-0303-research-concurrency-subagent-limit.md`,
  with its review under `meta/reviews/plans/`
- Codebase research: `meta/research/codebase/2026-10-10-0303-research-concurrency-subagent-limit.md`
- Validation (result `pass`): `meta/validations/2026-10-10-0303-research-concurrency-subagent-limit-validation.md`
- Decision: `meta/decisions/ADR-0071-optional-harness-variables-in-the-cli.md`
- Follow-ups captured during the manual runs: 0330, where a skill's
  `allowed-tools` grant lapses once a background subagent's completion
  opens a new query, and 0331.

## Testing

- [x] `mise run` exits 0 end to end, docs lane included (374s on the
      validation run)
- [x] Domain tests over every cap-parser case: whitespace, U+FEFF, U+0085,
      signs, non-ASCII digits, zero-padding and `u32` overflow
- [x] Integration tests over the `outstanding` binary. Both cap variables
      are cleared on every call; the tests cover team and personal config,
      exact `warnings` and stderr, offered spawns and `remaining` under each
      clamp, `--start`-only warnings and the empty-value exit
- [x] Coupling-guard tests: admission by name and file, a banned name on the
      same line, the pinned admitted set, and a missing reader
- [x] Structural tests over the skill: flag pass-through, no shell variable
      in any plan, the retry-round wording, the reason row, and knob-rule
      wording kept in step with the CLI
- [x] Manual, Claude Code 2.1.296: the cap reaches the CLI from an exported
      shell variable and from project, local and user `settings.json`,
      `ACCELERATOR_SUBAGENT_CAP` wins over Claude Code's variable, and no
      `accelerator` command triggered a permission denial
- [x] Manual, Claude Code 2.1.296:
  - full load: 108 spawns, batches of exactly 20, no capacity refusal;
  - partial hold: rounds accept one spawn each, and nothing is recorded as
    failed;
  - full hold: one fully refused round, each spawn recorded once, and a
    fresh run offers the spawns again

## Notes for Reviewers

- ⚠️ A plain `outstanding` query, which is how `conduct` ticks the outline,
  now reads config and is bounded by the concurrency. With an empty
  `research.topic.concurrency` it exits 1, where it used to ignore the
  value. Ticking itself is unaffected, because `items[].complete` comes
  from the round plan, not the spawn window.
- ⚠️ The clamp applies on every harness version and mode, including Claude
  Code before 2.1.217 and ultracode sessions, which enforce no cap. A
  project that configured more than 20 gets a clamp warning until it
  raises `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`.
- The retry recognises the harness's refusal by meaning, with "Concurrent
  subagent limit reached" as today's text. After a harness upgrade,
  recheck that text and the cap parser.
- Worth a close look: the admission logic in `tasks/lint/claude_coupling.py`
  and ADR-0071, which is still `proposed`.
- Out of scope: stopping retry rounds early on a halting failure belongs to
  0310. Making 0307's node query answer for re-issued spawns belongs to
  0307.
