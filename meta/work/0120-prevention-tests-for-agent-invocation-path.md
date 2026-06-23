---
type: work-item
id: "0120"
title: "Prevention Tests for the Agent-Invocation Path"
date: "2026-06-19T23:13:17+00:00"
author: Toby Clemson
producer: refine-work-item
status: done
kind: task
priority: high
parent: "work-item:0115"
relates_to: ["work-item:0116", "work-item:0117", "work-item:0118", "work-item:0119"]
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-20T13:48:16+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-142
---

# 0120: Prevention Tests for the Agent-Invocation Path

**Kind**: Task
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Close the two test gaps that let this class of failure ship: add a test that
drives an interactive migration the way the skill actually does (no TTY, no
decisions file) and asserts the **structured stall** (0116's term for the
actionable, non-zero halt that replaces the bare `failed to obtain decision`
abort), and add a check that cross-validates what a tolerant backfill emits for
an un-derivable required extra against what the validator accepts. This is the
prevention work of 0115.

## Context

Child of 0115 — Make Interactive Migrations Satisfiable Under Agent Invocation.

The existing interactive suite drives prompts *exclusively* via a decisions
file and always with `ACCELERATOR_MIGRATE_FORCE=1`, so the no-input branch of
`read_decision()` (the real agent-invocation path) is never exercised — exactly
the gap that hid the unsatisfiability. Separately, nothing cross-checks that the
state a tolerant backfill leaves behind is a state the validator accepts, which
is how the 0007 hard-fail-on-tolerated-state contradiction reached the tree.

## Requirements

- Add a test that invokes an interactive migration with no TTY and no decisions
  file (the actual skill-invocation path) and asserts 0116's structured stall —
  explicitly *not* the protocol-via-decisions-file path, which proved the wrong
  thing. The assertion must check the stall's concrete, observable signal (per
  0116's contract): a non-zero exit, output that names the current pending
  decision key and a resume command (in which `ACCELERATOR_MIGRATE_DECISIONS_FILE`
  is assigned a non-empty path, the migration id appears as a literal substring,
  and `run-migrations.sh` is the invoked driver), and the *absence* of the old
  bare string `failed to obtain decision`.
- Add an end-to-end check in the 0007 suite that cross-validates what a tolerant
  backfill emits for an un-derivable required extra (the `unknown` sentinel 0118
  writes — a "tolerated state". A *required type-extra* is a frontmatter key the
  corpus schema mandates for a given document type; *derivable* means the
  backfill can compute its value from the filename — e.g. `pr_number` from a
  numeric stem or a `pr`/`PR` segment. The tolerated state is a required
  type-extra the backfill cannot derive a real value for) against what the
  corpus validator accepts, so the
  hard-fail-on-tolerated-state class cannot reappear. The check must run against
  a triggering fixture that actually exhibits the state — a PR-description file
  whose filename carries an external tracker key (`<TRACKER>-NNNN-description.md`)
  so `pr_number` cannot be auto-derived.

## Acceptance Criteria

- [ ] Given an interactive migration that emits a `PROMPT` with no decisions
      file, no TTY, and fd 0 at EOF (the agent-invocation path, set up per
      Assumptions), when the no-input test runs, then the run exits non-zero and
      its output:
      - (a) contains the exact decision key the fixture's first `PROMPT` frame
        emits, asserted as a literal substring against a key the test fixture
        fixes and knows in advance (not merely "some non-empty token");
      - (b) contains a resume command in which `ACCELERATOR_MIGRATE_DECISIONS_FILE`
        is assigned a non-empty path, the migration id appears as a literal
        substring, and `run-migrations.sh` is the invoked driver;
      - (c) does **not** contain the string `failed to obtain decision`.

      The test must reach this via the no-input branch, not by supplying a
      decisions file; the structured-stall output of (a)–(b) is itself the
      positive evidence that `read_decision()`'s bare fd-0 branch
      (`interactive-lib.sh:262`) was traversed rather than the decisions-file or
      `/dev/tty` branch.
- [ ] Given a corpus file whose required type-extra cannot be auto-derived from
      its filename (a `pr_number` on a `<TRACKER>-NNNN-description.md` file), when
      0007 runs in full in the e2e cross-check, then 0007 exits 0, no output line
      matches the regex `FAIL:.*MISSING-EXTRA`, and the value the validator sees
      for that extra is the accepted `unknown` sentinel — accepted because the
      validator raises neither `MISSING-EXTRA` (the extra is present) nor
      `EMPTY-PLACEHOLDER` (the validator rejects only literal `""`/`[]`, which the
      non-empty `unknown` token is not).
- [ ] Given both new tests, when CI runs them in headless mode, then each
      explicitly establishes the no-input precondition (closes fd 0 / redirects
      from `/dev/null`, with `ACCELERATOR_MIGRATE_DECISIONS_FILE` unset per
      Assumptions), and each affected suite — `test-migrate-interactive.sh` (the
      no-input test) and the 0007 suite (the cross-check) — runs to completion
      exiting 0 with a reported test count at or above the floor that suite
      already asserts in CI (the comparison is delegated to the suite's existing
      recorded floor, so it is deterministic at run time without hard-coding a
      count here, and a silently-skipped or shrunk suite does not satisfy this),
      without provisioning a pseudo-TTY.

## Open Questions

- ~~Should the cross-validation check live in the 0007 suite, a shared migration
  test helper, or as a standalone lint over backfill/validator pairs?~~
  **Resolved**: the cross-check lives in the existing 0007 suite as an
  end-to-end assertion against a tracker-key-named fixture. This keeps the task
  bounded to the single failure class (M sizing). The generalised
  standalone-lint-over-all-pairs option was rejected here as scope-widening; if
  that broader guard is ever wanted, it is a separate work item.

## Dependencies

- Blocked by: 0116 (the structured stall the no-input test asserts), 0118
  (the backfill/validator reconciliation the cross-check guards).
- Blocks: none.
- Relates to: 0115 (parent), 0116, 0117, 0118, 0119. 0117 (agent↔decisions
  bridge / invoker contract) carries the other two prevention items from the
  research ("keep schema-upgrade migrations non-interactive" and "document the
  invoker contract"); it is a traceability link, not an ordering dependency of
  these tests. 0119 (resume-safe partial-migration failure) owns the
  guarded-resume behaviour and its *own* tests — guarded-resume coverage is
  deliberately **out of scope** here, so 0119 is a relates-to link, not a
  blocker of these two tests.
- This "Blocked by: 0116" edge is mirrored reciprocally — 0116 records
  "Blocks: 0120" — and is real: the no-input test asserts behaviour 0116
  introduces.
- The resume-command shape AC1 asserts is owned by the pre-flight session-log
  resume hint (`run-migrations.sh:90-132`), not by this task. Per 0116's own
  Dependencies, a future change to that hint format — including via 0119 — must
  be treated as touching this test's assertion.

## Assumptions

- The no-input path can be exercised in CI by closing fd 0 / redirecting from
  `/dev/null` and leaving `ACCELERATOR_MIGRATE_DECISIONS_FILE` unset, without a
  pseudo-TTY.

## Technical Notes

**Size**: M — two new test classes (the no-input agent-invocation path plus a
backfill-vs-validator cross-check); the cross-check design is non-trivial and the
task is sequenced after 0116 and 0118.

- The existing suite drives only the decisions-file path, always
  FORCE-bypassed, never the no-input branch —
  `skills/config/migrate/scripts/test-migrate-interactive.sh` (decisions-file
  pattern e.g. `:381`, `:396`; `ACCELERATOR_MIGRATE_FORCE=1` e.g. `:232`,
  `:395`).
- No-input branch under test: `read_decision()` bare fd-0 read at
  `interactive-lib.sh:262`; the structured stall this asserts comes from 0116.
- 0007 unit-test seam `ACCELERATOR_0007_NO_RUN=1` (`0007:742`) lets helpers be
  tested without the full orchestration; the cross-check guards the
  backfill (`0007:507-510`) vs validator (`validate-corpus-frontmatter.sh:345`)
  contradiction.

## Drafting Notes

- Decomposed from 0115 prevention work. Sequenced after 0116 and 0118 because
  the tests assert the behaviour those tasks introduce.
- The two tests are intentionally kept together as one task: although they have
  disjoint blockers (no-input test → 0116; cross-check → 0118) and could in
  principle land independently, both are small and share the single purpose of
  closing the regression gaps for this one failure class. They are not split
  into separate children of 0115.

## References

- Source: `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
- Related: 0115, 0116, 0118
