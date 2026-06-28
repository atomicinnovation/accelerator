---
type: work-item
id: "0160"
title: "Port skill-creator evals to the Inspect harness"
date: "2026-06-28T11:44:00+00:00"
author: Toby Clemson
status: draft
kind: task
priority: medium
tags: [evaluation, skills, inspect, testing, migration]
relates_to: ["adr:ADR-0055", "adr:ADR-0050", "adr:ADR-0052"]
last_updated: "2026-06-28T11:44:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0160: Port skill-creator evals to the Inspect harness

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Stand up the Inspect eval tier decided in ADR-0055 and migrate the existing
`skill-creator` eval task sets onto it: convert each skill's colocated
`evals/evals.json` into Inspect's `@task`/dataset form, relocate it under
`tests/evals/`, and wire the headless, threshold-gating invoke task. This makes
ADR-0055 real for the skills that already have evals; applying evals to skills
that have none is separate work (0161).

## Context

ADR-0055 adopts **UK AISI Inspect** as the skill-evaluation harness, run as a
third test tier (`tests/evals/`) via a `mise`+invoke task, excluded from the
default CI sweep, gating on **pass^k**. It records the harness choice and the
tier's shape only — not the migration.

A **partial `skill-creator` eval framework already exists**: ~18 skills carry
colocated `evals/evals.json` task sets (under `skills/work/`, `skills/design/`,
`skills/review/lenses/`, `skills/integrations/jira/`, …). ADR-0055 deliberately
places eval definitions and results under the test path, **never under
`skills/`**, so porting also relocates these out of the skill directories. The
Inspect tier itself (the `tests/evals/` layout, the in-process `eval()` invoke
task, the `pass_k` threshold gate, the `mise run eval` / `eval:skills:<skill>`
tasks) does not exist yet and must be built as part of this work.

## Requirements

- Stand up the Inspect eval tier per ADR-0055:
  - `tests/evals/skills/<skill>/` layout (`<skill>_eval.py` `@task` with
    dataset + with-skill/baseline solvers + scorer; `dataset.jsonl`;
    `results/<timestamp>.json` committed as `--log-format json`).
  - An invoke task that runs Inspect in-process with
    `epochs=Epochs(k, pass_k(k))`, reads `pass_k` off `log.results`, and exits
    non-zero below the floor (the ~3-line hand-wired gate).
  - `mise run eval:skills:<skill>` leaves plus a `mise run eval` roll-up,
    **excluded from the default `mise run` / `check` sweep**.
  - Eval files named so pytest does not collect them (Inspect `@task` files,
    not `test_*`).
- Migrate every existing `skill-creator` `evals/evals.json` task set to the
  Inspect `@task`/dataset format under `tests/evals/`, preserving the tasks.
- Remove the migrated `evals/` directories from under `skills/` once ported.
- Apply the bootstrap floor (≥ 3 tasks, pass^k ≥ 0.8, k = 3) to each migrated
  suite, or record where an existing suite already exceeds it.
- Retain `skill-creator` as an optional interactive authoring aid (not removed).

## Out of Scope

- Authoring evals for skills that have **no** existing `evals.json` — that is the
  rollout tracked by work item 0161.
- Running evals on every CI build (ADR-0055 keeps the tier opt-in).

## Acceptance Criteria

- [ ] The Inspect eval tier exists: `tests/evals/` layout, the in-process invoke
      task with the pass^k threshold gate, and `mise run eval` /
      `mise run eval:skills:<skill>` tasks excluded from the default sweep.
- [ ] Every pre-existing `skill-creator` `evals/evals.json` task set is ported to
      an Inspect `@task`/dataset under `tests/evals/skills/<skill>/`, with its
      tasks preserved.
- [ ] No `evals/` directories remain under `skills/` for the migrated skills.
- [ ] Each migrated suite is gated at the bootstrap floor (≥ 3 tasks, pass^k ≥
      0.8, k = 3) or documented as exceeding it.
- [ ] `mise run check` stays green and the eval tier does not run in the default
      sweep.

## Dependencies

- Blocks: work item 0161 (rolling Inspect evals out to skills without evals).
- Blocked by: none — but the CLI/config direction is unaffected; this is a
  test-tier and tooling change.

## Technical Notes

- ADR-0055 notes Inspect leaves the with-skill-vs-baseline A/B and
  skill-invocation detection to the author (promptfoo provides these turnkey); a
  shared eval-harness helper is worth building once and reusing across migrated
  suites.
- Inspect installs via `uv` and lints under the existing ruff/pyrefly setup
  (ADR-0048's toolchain split preserved — no new language toolchain).

## References

- `meta/decisions/ADR-0055-inspect-as-the-skill-evaluation-harness.md` — the
  harness decision this work item realises.
- `meta/decisions/ADR-0050-mise-invoke-task-runner.md` — the task runner the eval
  tier plugs into.
- `meta/decisions/ADR-0052-filesystem-as-message-bus-and-knowledge-corpus.md` —
  the committed-file model the eval definitions and results follow.
- `meta/work/0159-skill-evaluation-framework-selection.md` — the feeding spike.
</content>
