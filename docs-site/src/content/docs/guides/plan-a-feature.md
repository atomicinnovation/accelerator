---
title: Plan a Feature
description: How to take a feature from an idea to an approved,
  implementation-ready plan using the research and planning skills.
---

This guide takes a feature from an idea to an approved implementation
plan. It assumes the plugin is installed (see
[Getting started](../getting-started.md)) and that you are working in a
repository Claude Code has open.

Every artefact lands in `meta/` (paths are
[configurable](configuration-cookbook.md#move-where-documents-are-written)),
so each step can start a fresh conversation without losing context.

## Steps

1. **Capture the feature as a work item** (optional but recommended).
   Run [`create-work-item`](../reference/skills/work/create-work-item.md)
   with a short description:

   ```
   /accelerator:create-work-item add CSV export to the reports page
   ```

   The skill asks a few business-context questions, proposes
   requirements and acceptance criteria, and — once you approve — writes
   `meta/work/<id>-add-csv-export-....md` with `status: draft`.

2. **Research the affected code.** Run
   [`research-codebase`](../reference/skills/research/research-codebase.md)
   with the question the plan needs answered:

   ```
   /accelerator:research-codebase how does report generation and download work?
   ```

   Parallel subagents explore the codebase and the findings are written
   to `meta/research/codebase/YYYY-MM-DD-<description>.md` with
   file-and-line references. You can ask follow-up questions; they are
   appended to the same document.

3. **Create the plan.** Run
   [`create-plan`](../reference/skills/planning/create-plan.md),
   referencing the work item and/or research document:

   ```
   /accelerator:create-plan @meta/work/0042-add-csv-export.md
   ```

   This is interactive: the skill asks the questions research could not
   answer, offers design options, and agrees a phase structure with you
   before writing the full plan to `meta/plans/YYYY-MM-DD-<description>.md`.
   Each phase carries success criteria split into automated and manual
   verification. The final plan must contain no open questions.

4. **Harden the plan before any code exists.** Two companions, use
   either or both:

   - [`review-plan`](../reference/skills/planning/review-plan.md) runs
     the plan through parallel quality lenses (architecture,
     correctness, test coverage, …) and records a verdict — APPROVE,
     REVISE, or COMMENT — in `meta/reviews/plans/`.
   - [`stress-test-plan`](../reference/skills/planning/stress-test-plan.md)
     interrogates *you* about decisions, edge cases, and assumptions.

   Update the plan in response and re-review until it earns APPROVE.

5. **Hand over to implementation.** Run
   [`implement-plan`](../reference/skills/planning/implement-plan.md)
   — in a fresh session if you like, since the plan file carries all the
   context:

   ```
   /accelerator:implement-plan @meta/plans/2026-07-10-add-csv-export.md
   ```

   The skill works phase by phase, ticking off success criteria in the
   plan file itself, and pauses after each phase for your review.

6. **Close the loop.** After implementing (and
   [committing](../reference/skills/vcs/commit.md)), run
   [`validate-plan`](../reference/skills/planning/validate-plan.md) to
   verify the code matches the plan; a passing validation marks the
   plan `status: done`.

## See also

- [Development Loop](../development-loop.md) — the concepts behind the
  research → plan → implement spine.
- [Full Workflow](../workflow.md) — where planning sits in the wider
  skill map.
- [Which skill do I need?](which-skill.md)
