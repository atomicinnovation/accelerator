---
type: work-item
id: "0116"
title: "Structured Stall on No Decision Input"
date: "2026-06-19T23:13:17+00:00"
author: Toby Clemson
producer: refine-work-item
status: done
kind: task
priority: high
parent: "work-item:0115"
relates_to: ["work-item:0069", "work-item:0117", "work-item:0118", "work-item:0119"]
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-20T12:45:43+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-138
---

# 0116: Structured Stall on No Decision Input

**Kind**: Task
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

Replace the bare `failed to obtain decision` abort with an actionable,
structured stall when an interactive migration emits a prompt but no input
channel exists. This is the immediate, low-risk mitigation — **fix B** (the
structured-stall mitigation) — for 0115; it makes every interactive migration
debuggable even before **fix A** (the full agent↔decisions bridge, 0117) lands.

A **structured stall**, as used here, is a deliberate non-zero halt (it
*replaces* the existing abort's message, not its exit semantics — the run still
cannot proceed without input) that, in place of the opaque one-line failure,
emits a structured, parseable block naming the pending decision key(s) and the
exact resume command. It is distinct from the current abort in message content
and machine-detectability, not in whether the run stops.

## Context

Child of 0115 — Make Interactive Migrations Satisfiable Under Agent Invocation.

`read_decision()` selects its input source as decisions file → `/dev/tty` (only
when fd 0 is a TTY) → bare fd 0. Under agent invocation all three are
unreachable: fd 0 is non-interactive and at EOF, so the read returns non-zero
immediately. The failure surfaces as an opaque `failed to obtain decision`
abort emitted by the PROMPT frame handler — after the corpus has already been
mutated.

There are **two** emit sites that must both be covered, each with its own
current opaque message: the PROMPT frame handler (which prints `failed to obtain
decision`) and the VALIDATE_ERR re-prompt path (which prints `failed to obtain
re-decision`). A **PROMPT frame** is the interactive-harness protocol message a
migration emits to request a decision; the **VALIDATE_ERR re-prompt** is the
path taken when a supplied decision fails validation and a fresh decision is
requested for the same key.

**Actors** referenced below:

- **driver** — `run-migrations.sh`, the top-level runner the invoker executes.
- **runner / frame handler** — `interactive-lib.sh`, which emits the PROMPT /
  VALIDATE_ERR frames and calls `read_decision()`; it is what emits the stall.
- **invoker** — whoever runs the `migrate` skill. In the failure case this is an
  **agent** driving the Bash tool (no TTY, fd 0 at EOF).
- **agent / user** — the party that consumes the stall and acts on it; the agent
  relays the stall to the human and re-runs the driver with the resume command.

## Requirements

- Detect the no-input case explicitly: non-interactive fd 0 (not a TTY), no
  `ACCELERATOR_MIGRATE_DECISIONS_FILE`, no usable `/dev/tty`.
- On detection, emit a structured, actionable stall in place of the bare
  message. The stall must:
  - **Exit non-zero**, halting the run as the current abort does — the stall is
    a better-described halt, not a clean continuation.
  - **Name the pending decision(s)**: at minimum the current decision key being
    prompted for, and additionally any other keys accumulated so far when that
    set is cheaply derivable at the emit point (see Assumptions).
  - **Print the exact resume command** in the same form as the in-flight
    session-log resume hint produced by the pre-flight check
    (`run-migrations.sh:90-132`) — i.e. an invocation that re-runs the driver
    with `ACCELERATOR_MIGRATE_DECISIONS_FILE` set, carrying the migration id and
    the decisions-file path so the run can be resumed deterministically.
- Apply the same treatment at both emit sites: the PROMPT frame handler
  (`interactive-lib.sh:450`, currently `failed to obtain decision`) and the
  VALIDATE_ERR re-prompt path (`:485`, currently `failed to obtain
  re-decision`).
- Preserve existing behaviour when a decisions file or TTY *is* present — the
  stall is reached only when no input channel exists.

## Acceptance Criteria

- [ ] Given an interactive migration emits a `PROMPT` with no decisions file,
      no TTY, and fd 0 at EOF, when `read_decision()` fails, then the run exits
      non-zero having emitted a structured stall that (a) names **at least the
      current** pending decision key and (b) contains a resume command in which
      `ACCELERATOR_MIGRATE_DECISIONS_FILE` is assigned a non-empty path, the
      failing migration's id appears as a literal substring, and
      `run-migrations.sh` is the invoked driver — and the output does **not**
      contain `failed to obtain decision`.
- [ ] Given a decision is supplied that fails validation and the re-prompt is
      emitted while the same no-input conditions hold (no decisions file, no
      TTY, fd 0 at EOF), when re-decision fails on the VALIDATE_ERR path, then
      it exits non-zero having emitted the same structured stall (current key +
      resume command in the form above), and the output does **not** contain
      `failed to obtain re-decision`.
- [ ] Given a decisions file is supplied that answers every prompt, when the
      migration runs, then it consumes each decision and completes with the same
      exit status and recorded-decision count as before this change — and the
      existing interactive suite (`test-migrate-interactive.sh`) passes
      unchanged.

> Note: listing **all** accumulated pending keys (not just the current one) is
> a best-effort enrichment, not an acceptance criterion — its deliverability
> depends on whether the full set is in scope at the emit point, which a
> black-box verifier cannot evaluate. The firm, testable guarantee is the
> current key (criteria 1-2); the all-keys behaviour is delivered when cheap
> and is otherwise out of scope for 0116.

## Open Questions

- None.

## Dependencies

- Blocked by: none.
- Blocks: 0120 (the prevention no-input test asserts the structured stall this
  task introduces, so it must land after this). 0117 builds on the same code
  region but does not require this.
- Relates to: 0115 (parent); 0069 (runner-side interactive path); 0117, 0118,
  0119 (the coordinated fix set decomposed from 0115 — fix A bridge, fix C
  0007 backfill/validator reconcile, fix E resume-safe partial failure). This
  task is one of several mitigations for the same agent-invocation failure and
  should be scheduled with that set in view.
- Content dependency on the pre-flight resume-hint format
  (`run-migrations.sh:90-132`): the resume command this stall prints must match
  that hint's shape, which this task does not own. A future change to that hint
  format (including via 0119) must be treated as touching this task.
- Ordering relative to 0117: 0116 is intended to land before, or independently
  of, 0117. Both edit `interactive-lib.sh` in the same region, so if 0117 is
  scheduled first, coordinate merge order to avoid conflicts and to preserve
  0116's standalone-mitigation value.
- Functional precondition via 0118: the stall added here is only reachable
  end-to-end in a real corpus once 0118 (fix C) lets 0007 progress past its
  structural self-validation into the interactive phase. This is an
  integration-ordering relationship for end-to-end validation, not a code-level
  blocker (the stall logic and its unit-level tests stand alone).
- Soft coupling to 0119: the resume command this stall prints reuses the
  existing pre-flight session-log resume hint. That hint covers an in-flight
  *interactive session log*, not a *partial mechanical run*; making the
  partial-run resume safe is 0119's concern. The stall's resume command is
  therefore fully actionable for the interactive-session-log path, and a
  best-effort breadcrumb otherwise until 0119 lands.

## Assumptions

- The current decision key being prompted for is always available at the emit
  point, so naming it is unconditional. The *full* set of accumulated pending
  keys may or may not be cheaply derivable there; when it is, the stall lists
  all of them, otherwise it lists only the current key. The current-key
  guarantee is what the first two acceptance criteria assert; the all-keys
  behaviour is the optional best-effort enrichment described in the note under
  Acceptance Criteria, not a criterion.
- The resume-command shape is borrowed from the existing pre-flight hint rather
  than defined here; if 0119 changes the resume mechanism, this command's text
  will need reconciliation (see Dependencies).

## Technical Notes

**Size**: S — no-input detection plus a structured stall message at two emit
sites (`interactive-lib.sh:450`, `:485`); localized to one file.

- `read_decision()` input-source selection — `interactive-lib.sh:236-288`
  (decisions file → `/dev/tty` gated on `[ -t 0 ]` at `:259` → bare fd 0 at
  `:262`).
- Emit sites to convert: PROMPT frame handler `interactive-lib.sh:450`;
  VALIDATE_ERR re-prompt `:485`. Both currently print the opaque message.
- The resume command shape should match the in-flight session-log resume hint
  already produced by the pre-flight check (`run-migrations.sh:90-132`).

## Drafting Notes

- Decomposed from 0115 fix B. Kept separate from 0117 (fix A) so it can ship as
  the standalone low-risk mitigation, though both touch the same file region.

## References

- Source: `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
- Related: 0115, 0069
