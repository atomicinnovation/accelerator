---
type: work-item
id: "0161"
title: "Roll out Inspect evals across the remaining skills"
date: "2026-06-28T11:44:00+00:00"
author: Toby Clemson
status: draft
kind: task
priority: medium
external_id: "PP-180"
tags: [evaluation, skills, inspect, testing, coverage]
blocked_by: ["work-item:0160"]
relates_to: ["adr:ADR-0055"]
last_updated: "2026-06-28T11:44:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0161: Roll out Inspect evals across the remaining skills

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Extend Inspect skill-evaluation coverage to the skills that have no eval suite
yet — authoring `@task` datasets and scorers for them on the tier stood up in
0160 — so skill quality is gated for the whole catalogue, not just the skills
that already carried `skill-creator` evals.

## Context

ADR-0055 adopts Inspect as the skill-evaluation harness and gates on **pass^k**.
Work item 0160 stands up the `tests/evals/` tier and migrates the ~18 skills that
already have `skill-creator` `evals/evals.json` task sets. Many skills have no
evals at all — for example `configure`, the VCS skills, and the GitHub/PR skills.
This work item covers authoring Inspect evals for those, prioritising by risk and
real-failure history rather than attempting the whole catalogue at once.

## Requirements

- Identify skills with no Inspect eval suite after 0160 and prioritise them
  (highest-value / highest-risk / most-real-failures first).
- For each prioritised skill, author an Inspect `@task` + `dataset.jsonl` under
  `tests/evals/skills/<skill>/`, with a with-skill/baseline A/B and a scorer,
  starting at the bootstrap floor (≥ 3 tasks, pass^k ≥ 0.8, k = 3).
- Grow suites toward the 20–50 real-failure-derived tasks ADR-0055 commits to as
  real failures accumulate, raising k as token budget allows.
- Commit eval definitions and results under `tests/evals/` (never under
  `skills/`), per ADR-0055.

## Out of Scope

- The tier infrastructure and the migration of existing evals — both owned by
  0160.
- Running evals on every CI build (the tier stays opt-in).

## Acceptance Criteria

- [ ] A prioritised list of un-evalled skills exists, with rationale.
- [ ] Each skill taken on in this work item has an Inspect eval suite under
      `tests/evals/skills/<skill>/` gated at the bootstrap floor.
- [ ] Coverage gaps that remain (skills deliberately deferred) are recorded so
      the rollout is auditable rather than silently partial.
- [ ] `mise run check` stays green; the eval tier stays out of the default sweep.

## Dependencies

- Blocked by: work item 0160 (the Inspect tier and migration must land first).

## Technical Notes

- Reuse the shared eval-harness helper from 0160 for the with-skill-vs-baseline
  setup and skill-invocation detection rather than re-building per skill.
- `configure` is a natural early target (ADR-0055's original worked example), but
  it is no longer privileged as "first" — sequence by value/risk.

## References

- `meta/decisions/ADR-0055-inspect-as-the-skill-evaluation-harness.md` — the
  harness and bootstrap-floor decision.
- `meta/work/0160-port-skill-creator-evals-to-inspect-harness.md` — the tier
  setup and existing-eval migration this work depends on.
</content>
