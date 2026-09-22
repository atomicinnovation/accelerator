---
type: "pr-description"
id: "130"
title: "[0282] Tunable research depth and breadth"
date: "2026-09-22T15:48:03+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0282"
parent: "work-item:0282"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/130"
pr_number: 130
tags: ["research", "config", "skills"]
revision: "b4e89221daa3052aa73fe851be8f3c1d0bacb661"
repository: "accelerator"
last_updated: "2026-09-22T18:08:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0282] Tunable research depth and breadth

## Summary

Makes the `/accelerator:research-topic` breadth and depth bounds configurable
instead of hard-coded, each resolved `flag > personal > team > built-in
default`. `breadth` (default 8) is a live ceiling on focus areas per `outline`
round; `depth` (default 1) ships dormant — threaded through `conduct` but with
no behavioural effect until the recursion engine (work item 0283) lands. To let
the skill read those defaults, `accelerator config get` now resolves the
built-in catalogue default and takes its override as a `--default` flag, a
breaking change to the CLI grammar.

## Changes

### Research knobs

- **`outline` gains `--breadth N`, `conduct` gains `--depth N`** in
  `research-topic`; each verb resolves its knob `flag > personal
  (config.local.md) > team (config.md) > built-in default`.
- **Validation is uniform**: a value that is not an integer of 1 or more clamps
  to 1 with a warning naming the offending value; an empty (unreadable) config
  stops the verb rather than guessing; a flag meant for the other verb is
  ignored with a note. There is no upper bound — breadth is the per-round cost
  guard.
- **`depth` is inert**: `conduct` always spawns one researcher per focus area
  whatever the resolved depth, printing a notice when the resolved value
  exceeds 1.
- **`configure` documents the `research.topic.*` namespace** — the
  `breadth`/`depth` keys, their integer contract, and the resolution order.
  The knobs are scoped under `research.topic` (the `research-topic` skill),
  kept distinct from the `paths.research_*` output-directory keys.

### `config get` grammar (breaking)

- **Built-in default on a cross-level miss**: `accelerator config get <key>`
  now returns the key's catalogue default when it is unset at both levels and
  read without `--level`, rather than an empty line.
- **Override moves to a flag**: the caller default moves from a bare positional
  (`config get <key> <default>`) to `config get <key> --default <value>`. A
  non-empty `--default` wins over the built-in; `--default ""` falls through to
  it (matching `config path`); a single-level `--level` read applies no
  built-in default; `--fail-safe` is unchanged.
- **New catalogue keys**: `RESEARCH_KEYS` registers `research.topic.breadth`
  (`8`) and `research.topic.depth` (`1`), taking the catalogue to 65 keys
  across seven groups; exported in the public-API fixture.
- **In-repo callers migrated**: e.g. `init-jira` drops its `config get
  jira.site ""` positional to `config get jira.site`.

### 0282 planning corpus (`meta/`)

- Adds the codebase research, the approved plan, the plan and work-item
  reviews, and the validation record for 0282; advances the work item to done.

### Incidental

- Adds a **`raise-pr` skill** (`.claude/skills/raise-pr/SKILL.md`) — unrelated
  to 0282, riding on the same branch.

## Context

- Implements work item 0282, `meta/work/0282-tunable-depth-and-breadth.md`
  (child of epic 0121, high priority).
- Plan: `meta/plans/2026-09-20-0282-tunable-depth-and-breadth.md`. Validation:
  `meta/validations/2026-09-20-0282-tunable-depth-and-breadth-validation.md`.
- `depth` lands dormant ahead of the sibling recursion engine, work item 0283,
  which 0282 blocks.

## Testing

- [x] `mise run check` — the read-only CI mirror (format, lint, and types
      across frontend, server, cli, and scripts) passes.
- [x] Catalogue unit tests assert the two research defaults and the updated
      65-key / seven-group count (`cli/config/src/catalogue.rs`).
- [x] `config get` resolution covered by `cli/launcher/tests/config_read.rs`
      (built-in default on a cross-level miss, `--default` override, `--level`
      suppression), with the config-adapter parity suite and the `dump` golden
      updated.
- [x] Edge cases: malformed / sub-1 clamp, empty-config stop, misplaced-flag
      note, and `--default ""` fall-through — specified and exercised at the
      skill layer.
- [ ] Recursive `depth` behaviour — out of scope here; dormant until 0283.

## Notes for Reviewers

- **Breaking CLI change.** `config get`'s positional default becomes
  `--default`; every in-repo caller is migrated in this PR, but any external
  caller relying on the positional will break. Flagged in `CHANGELOG.md`.
- **Review `depth` as plumbing, not behaviour** — resolution, validation, and
  the notice only. The recursion engine that gives it effect is 0283.
- **The `raise-pr` skill is a separate addition** that rode on the same branch;
  it is not part of 0282's scope. Worth a glance, but review it on its own
  terms.
- **Knob validation lives in the skill prompt, not Rust** — the clamp and the
  empty-config stop are prose rules, verified by reading rather than a unit
  test.
