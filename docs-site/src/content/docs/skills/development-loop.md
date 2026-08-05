---
title: Development Loop Skills
description: 'How the research → plan → implement skills fit together, and
  when to reach for each.'
---

The research → plan → implement spine, plus the review companions that
harden a plan before any code is written. For the narrative — how the
phases hand off through the `meta/` directory — see the
[Development Loop](../development-loop.md) overview.

## The spine

Run these in order for any non-trivial change:

1. [`research-codebase`](../reference/skills/research/research-codebase.md)
   spawns parallel subagents and synthesises what it finds into a
   research document in `meta/research/codebase/`. Run it first so the
   plan can cite concrete findings rather than guesses.
2. [`create-plan`](../reference/skills/planning/create-plan.md) builds a
   phased implementation plan in `meta/plans/` through interactive
   collaboration, with success criteria per phase.
3. [`implement-plan`](../reference/skills/planning/implement-plan.md)
   executes an approved plan phase by phase, checking off success
   criteria in the plan file as each phase completes.

## Hardening a plan

Two complementary skills sit between plan and implementation:

- [`review-plan`](../reference/skills/planning/review-plan.md) applies
  the fixed quality lenses of the [Review System](review-system.md) and
  iterates on the findings.
- [`stress-test-plan`](../reference/skills/planning/stress-test-plan.md)
  interrogates **your** reasoning interactively — decisions, edge
  cases, and assumptions — to find gaps before implementation begins.

Use review for systematic coverage, stress-test when the plan rests on
judgement calls you want challenged. They compose: many changes warrant
both.

## Closing the loop

[`validate-plan`](../reference/skills/planning/validate-plan.md) runs
after implementation. It verifies each success criterion against the
actual code, records a validation report in `meta/validations/`, and
surfaces anything that drifted from the plan.
