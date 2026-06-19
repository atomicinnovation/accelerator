---
type: work-item
id: "0115"
title: "Make Interactive Migrations Satisfiable Under Agent Invocation"
date: "2026-06-19T22:59:40+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: story
priority: high
parent: "work-item:0057"
relates_to: ["work-item:0069", "work-item:0092", "work-item:0114"]
source: "issue-research:2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation"
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-19T22:59:40+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0115: Make Interactive Migrations Satisfiable Under Agent Invocation

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a developer running `/accelerator:migrate` against a repo, I want interactive
schema-upgrade migrations to complete under agent invocation, so that a
migration is never stranded mid-corpus on a prompt the agent structurally cannot
answer.

The interactive migration contract was designed around a human typing answers at
a terminal, but the only supported invocation path — the `migrate` skill driven
by an agent's non-interactive Bash tool — cannot satisfy a single `PROMPT`. The
moment any migration emits one, the run aborts with `failed to obtain decision`
after it has already mutated the corpus, leaving a dirty tree whose only
documented resume disables the corpus safety guard.

## Context

A real run against a consumer repo surfaced three failures in sequence
(`0007` self-validation hard-fail → forced dirty-tree bypass → interactive abort
on the first prompt). Root cause analysis is in
`meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`.

The primary defect is structural: `read_decision()` chooses its input as a
scripted decisions file (test-only, undocumented, unset), else `/dev/tty` (the
agent cannot type into it), else fd 0 (at EOF under the Bash tool). All three are
unreachable from an agent, so an interactive migration can never complete via the
supported path.

This item amends the **invoker** side of the optional interactive contract
(ADR-0037, work item `0092`) and extends the runner-level interactive/resume
machinery built in `0069`. The chosen direction is to **bridge** decisions to the
agent (research option A), explicitly *not* to split 0007 into a mechanical step
plus a human-run linkage step (research option D, rejected).

## Requirements

Deliver the following four fixes plus the prevention work, as one story
(decomposition into children may follow via `/refine-work-item`):

- **A — Agent↔decisions bridge.** Add a `--list` / dry-emit mode to the migration
  driver that surfaces every pending interactive transformation (key + proposed
  value + context) up front, before any prompt blocks. The agent decides each
  (via its own prompt-to-user or a policy), writes a decisions file, and passes
  it to the driver to complete the run. Promote
  `ACCELERATOR_MIGRATE_DECISIONS_FILE` from a deliberately hidden test-only seam
  to a documented, user-facing interface, and document the **invoker** contract
  in `SKILL.md` (how an agent answers prompts, and what happens when it cannot).

- **B — Structured stall on no input.** `read_decision()` detects the no-input
  case (non-interactive fd 0, no decisions file, no TTY) and emits an actionable,
  structured stall — listing the pending decision keys and the exact resume
  command — instead of the bare `failed to obtain decision` abort. This ships as
  the immediate low-risk mitigation and makes every interactive migration
  debuggable.

- **C — Reconcile 0007 backfill with its validator.** When a required type-extra
  (e.g. `pr_number`) has no derivable default — such as a PR-description file
  named with an external tracker key rather than a numeric PR id — the backfill
  writes an accepted sentinel placeholder (e.g. `pending`) rather than leaving the
  extra absent. This stops `self_validate_structural` from hard-failing on
  precisely the state the backfill chose to tolerate. Scoped to the
  no-derivable-default path only; the broader 0007 backfill completeness is owned
  by `0114`.

- **E — Resume-safe partial failure.** Detect that the dirty paths blocking a
  re-run are this run's own prior-migration output, and offer a guarded resume,
  rather than forcing `ACCELERATOR_MIGRATE_FORCE=1` (which disables the single
  guard protecting the corpus). Path-ownership detection is sufficient for now; a
  full staging/transaction boundary is out of scope.

- **Prevention.** Add a test that drives an interactive migration the way the
  skill actually does — no TTY, no decisions file — and asserts the intended
  structured deferral (not the protocol-via-decisions-file path, which proved the
  wrong thing). Add a check that cross-validates "what a tolerant backfill leaves
  absent" against "what the validator requires" so the hard-fail-on-tolerated-
  state class cannot reappear.

### Child work items

- 0116 — Structured Stall on No Decision Input
- 0117 — Agent-Decisions Bridge and Documented Invoker Contract
- 0118 — Reconcile 0007 Backfill Sentinel With Its Validator
- 0119 — Resume-Safe Partial Migration Failure
- 0120 — Prevention Tests for the Agent-Invocation Path

## Acceptance Criteria

- [ ] Given the `migrate` skill is invoked by an agent (no TTY, no decisions
      file) and a migration emits a `PROMPT`, when no input channel exists, then
      the run produces a structured, actionable stall naming the pending decision
      keys and the resume command — not an opaque `failed to obtain decision`
      abort. *(B)*
- [ ] Given an interactive migration with pending transformations, when the agent
      runs the documented list → decide → resume flow, then the migration
      completes with the decisions applied from the passed decisions file. *(A)*
- [ ] Given `ACCELERATOR_MIGRATE_DECISIONS_FILE` and the `--list` mode, when a
      developer reads `SKILL.md`, then the invoker contract (how prompts are
      answered under agent invocation, and what happens when they cannot be) is
      documented. *(A)*
- [ ] Given a corpus file whose required type-extra cannot be auto-derived from
      its filename, when 0007's backfill runs, then it writes an accepted sentinel
      and 0007's self-validation passes rather than hard-failing on the absent
      extra. *(C)*
- [ ] Given a partial-run failure whose only dirty paths are this run's own
      prior-migration output, when the operator re-runs, then a guarded resume is
      offered without requiring `ACCELERATOR_MIGRATE_FORCE=1`. *(E)*
- [ ] Given the test suite, when it runs, then it drives the actual agent
      invocation path (no TTY, no decisions file) and asserts the structured
      deferral. *(prevention)*
- [ ] Given a migration that tolerantly defers a transformation, when its own
      validation runs, then a check prevents it from hard-failing on that same
      tolerated state. *(prevention)*

## Open Questions

- Should A/B's change to the invoker side of the interactive contract be recorded
  as an amendment to ADR-0037 (work item `0092`), or is it an implementation
  detail under the existing decision?

## Dependencies

- Blocked by: none.
- Relates to: `0069` (runner-side interactive path + resumability this extends),
  `0092` / ADR-0037 (the interactive contract this amends on the invoker side),
  `0114` (0007 backfill completeness; C is scoped around it), `0070` (shipped
  the 0007 migration this makes satisfiable under agent invocation), `0062`
  (ADR that established interactive validation for corpus migration), `0103`
  (skill-emission side of the unified-schema validator contract), `0105`
  (corpus-validator blind spots; the validator C reconciles against).

## Assumptions

- Bridging decisions to the agent (A) is the chosen answer to "should
  body-section typed linkage run under agent invocation?" — it should, via the
  decisions bridge. Splitting the concern (D) is explicitly rejected. If this
  reverses, A's scope changes materially.
- C is still required despite `0114` being complete, because the research (dated
  today) shows the no-derivable-default contradiction live in the current tree.

## Technical Notes

**Size**: L — four scripts touched (`interactive-lib.sh`, `run-migrations.sh`,
the 0007 driver, `SKILL.md`), net-new `--list` driver surface, a cross-script
0007↔validator reconciliation, and two new test classes; decomposes into five
tasks (B/C low, A/E medium, prevention non-trivial).

- `read_decision()` — `skills/config/migrate/scripts/interactive-lib.sh:236-288`;
  the `PROMPT` frame handler that aborts on its failure is at `:442-470`.
- Clean-tree pre-flight and the `ACCELERATOR_MIGRATE_FORCE` bypass —
  `skills/config/migrate/scripts/run-migrations.sh:67-141`; the apply-in-order
  loop with no transaction boundary is at `:252-297`.
- 0007 `extra_default()` (`:193-223`), the tolerant required-extras backfill
  (`:502-512`), and `self_validate_structural` inside the `set -e` block
  (`:747-786`) — the C contradiction lives here.
- The existing interactive suite
  (`skills/config/migrate/scripts/test-migrate-interactive.sh`) drives only the
  decisions-file path, never a TTY or the skill-invocation path — the gap the
  prevention test must close.
- B has **two** emit sites, not one: the `PROMPT` handler at
  `interactive-lib.sh:450` and the VALIDATE_ERR re-prompt at `:485`. Both print
  the opaque message and both need the structured-stall treatment.
- A's `--list` mode is genuinely net-new driver surface — the driver exposes
  only `--skip`/`--unskip` today (`run-migrations.sh:43-65`).
- The decisions file is **positional and keyless**: newline-delimited
  `accept` / `skip` / `edit <value>` verbs matched by emission order, CRLF-
  tolerant, blank lines skipped (`interactive-lib.sh:266-287`); opened on
  literal fd 9 in `run_interactive_migration` (`:308-316`). `--list` output must
  preserve consumption order so the agent can map entries to verbs.
- C's sentinel must clear **both** validator gates: `MISSING-EXTRA`
  (`scripts/validate-corpus-frontmatter.sh:345`) and `EMPTY-PLACEHOLDER`, which
  rejects only literal `""` / `[]` (`:348-359`) — so a non-empty token like
  `pending` passes. The hard-fail propagates via `set -e` from the
  `self_validate_structural` call at `0007:771`.
- The 0007 unit-test seam `ACCELERATOR_0007_NO_RUN=1` (`0007:742`) lets the
  helpers be tested without triggering the orchestration block — useful for the
  prevention cross-check.

## Drafting Notes

- Kind chosen as `story` over `bug` because A and E are net-new capability, not
  just a defect repair.
- C is scoped narrowly to the no-derivable-default sentinel path; `0114` (status
  considered complete) owns the rest of 0007's backfill normalisation.
- Kept as a single story by request; `B`+`C` are low-effort, `A`+`E` are
  medium-effort, so `/refine-work-item` may later split it.
- Priority set to High: this is a structural blocker — the only supported
  invocation path for interactive migrations is unsatisfiable.

## References

- Source: `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
- Related: 0069, 0092, 0114
