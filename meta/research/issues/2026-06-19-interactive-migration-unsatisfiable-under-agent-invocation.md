---
type: issue-research
id: "2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation"
title: "Investigation: interactive migration prompt loop is unsatisfiable when the migrate skill is driven by an agent"
date: "2026-06-19T22:02:06+00:00"
author: "Toby Clemson"
producer: research-issue
status: complete
topic: "Interactive migration step (0007) aborts on its first prompt under agent-driven invocation, after earlier mechanical migrations have already dirtied the tree"
tags: [research, debugging, migrate, interactive-migration, tty, validation]
revision: "f98bfdc5fc38a0e6724b5c050dca11baad22de26"
repository: "accelerator"
last_updated: "2026-06-19T22:02:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Investigation: interactive migration prompt loop is unsatisfiable when the migrate skill is driven by an agent

**Date**: 2026-06-19 22:02 UTC
**Author**: Toby Clemson
**Git Commit**: f98bfdc5fc38a0e6724b5c050dca11baad22de26
**Branch**: HEAD
**Repository**: accelerator

## Issue Description

A run of `/accelerator:migrate` against a consumer repo failed repeatedly. The
agent driving the skill resolved two earlier blockers by hand, but the final
failure is structural: the interactive stage of migration
`0007-unify-meta-corpus-frontmatter` printed its first per-transformation prompt
and immediately aborted with `failed to obtain decision`, having recorded zero
decisions. The reporter characterises this as "a fundamental flaw in the
interactive migration model."

The session surfaced three distinct failures in sequence:

1. `0007` hard-failed with `FAIL: 2 frontmatter violation(s)` —
   `MISSING-EXTRA — required extra 'pr_number' absent` on two PR-description
   files whose names carry an external tracker key rather than a numeric PR id.
2. After the agent hand-edited those two files to add the missing key, the
   working tree was dirty (from migrations applied earlier in the same run), so
   the re-run was refused by the clean-tree pre-flight and required
   `ACCELERATOR_MIGRATE_FORCE=1` to proceed.
3. `0007` then advanced into its interactive phase and aborted on the very first
   prompt because the driver had no interactive input channel.

## Input Classification

Mixed — a structured session transcript (errors, file paths, migration ids)
plus a behavioural claim about a design flaw.

## Affected Components

- `skills/config/migrate/scripts/interactive-lib.sh:236-288` —
  `read_decision()`; the prompt-input path that reads from `/dev/tty` or fd 0.
- `skills/config/migrate/scripts/interactive-lib.sh:442-470` — the runner's
  `PROMPT` frame handler that calls `read_decision` and aborts the migration on
  its failure.
- `skills/config/migrate/scripts/run-migrations.sh:67-141` — clean-tree
  pre-flight and the `ACCELERATOR_MIGRATE_FORCE` bypass.
- `skills/config/migrate/scripts/run-migrations.sh:252-297` — the
  apply-in-order loop: no overall transaction; each migration's writes land on
  the tree as it runs.
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:193-223`
  — `extra_default()`; cannot derive `pr_number` from a tracker-key filename.
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:502-512`
  — the required-extras backfill that logs a tolerant `DIVERGE` and leaves the
  extra absent when no default is derivable.
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:771,747-786`
  — `self_validate_structural` inside the `set -e` orchestration block, which
  then hard-fails on exactly the absent extra the backfill tolerated.
- `skills/config/migrate/SKILL.md:216-224` — "Executing the migration": tells
  the invoker to run the driver, with no guidance for the interactive case.
- `skills/config/migrate/scripts/test-migrate-interactive.sh` — drives the
  interactive path **only** through `ACCELERATOR_MIGRATE_DECISIONS_FILE`; never
  through a TTY and never through the skill-invocation path.

## Timeline / Reproduction

1. `/accelerator:migrate` invoked; the agent runs `run-migrations.sh` via the
   Bash tool. Tree is clean; pre-flight passes.
2. Migrations `0001`,`0003`,`0004`,`0005`,`0006` apply and write to `meta/`.
   `0002` is a legitimate no-op (soft-defer). The tree is now dirty.
3. `0007` runs its mechanical pre-`harness_run` stages (pre-pass, backfill,
   rewrite), then `self_validate_structural` fails: two PR-description files are
   missing the required `pr_number` extra → exit 1 → whole run aborts.
4. Agent adds `pr_number: pending` to both files by hand.
5. Agent re-runs; clean-tree pre-flight refuses (tree dirty from steps 2-4);
   agent re-runs with `ACCELERATOR_MIGRATE_FORCE=1`.
6. `0007` clears its mechanical stages and reaches `harness_run`. On the first
   ambiguous-band linkage transformation it emits a `PROMPT` frame; the runner
   calls `read_decision`, which hits EOF on a non-interactive fd 0, returns
   non-zero, and the migration aborts with zero decisions recorded.

## Hypotheses

### Hypothesis 1: The interactive prompt loop has no input channel under agent invocation (primary)

- **Evidence for**: `read_decision()` (`interactive-lib.sh:258-264`) chooses its
  input source as: scripted decisions file if set, else `/dev/tty` if fd 0 is a
  TTY, else fd 0 itself. When the skill is invoked, the agent runs the driver
  through the Bash tool, where fd 0 is non-interactive (no TTY, EOF on read) and
  no decisions file is set. So `read -r line` returns non-zero immediately; the
  `PROMPT` handler (`:449-453`) prints `failed to obtain decision` and aborts.
  The agent is the "user," but it communicates by issuing tool calls, not by
  writing bytes into a blocking `read` — there is no bridge between the
  human-at-a-TTY contract and the agent-as-invoker reality. The transcript
  matches exactly: aborts on the first prompt, zero decisions recorded.
- **Evidence against**: none. A human running the driver directly in their own
  terminal would satisfy the `/dev/tty` branch — but that is not how the skill
  is invoked, and `SKILL.md` never says it must be.
- **Verdict**: Confirmed.

### Hypothesis 2: Migration 0007 self-validation contradicts its own tolerant backfill

- **Evidence for**: For a required extra with no derivable default, the rewrite
  backfill deliberately logs `0007-DIVERGE[missing-extra-no-default] … left
  absent` and continues (`0007:507-510`). `extra_default()` derives `pr_number`
  from a `pr`/`PR` filename segment or a leading numeric stem
  (`0007:201-217`); a filename keyed by an external tracker (e.g.
  `<TRACKER>-NNNN-description.md`) matches neither, so the default is empty and
  the key is left absent. Immediately afterward, still inside the `set -e`
  orchestration block, `self_validate_structural` runs the corpus validator
  (`0007:771`), which treats the same absent required extra as a hard
  `MISSING-EXTRA` violation → exit 1 → the entire run aborts. The migration
  thus rejects, one step later, precisely what it chose to tolerate.
- **Evidence against**: the contradiction only bites a corpus that actually
  contains a required-extra-bearing document whose extra cannot be auto-derived;
  corpora without such files complete cleanly. This narrows the blast radius but
  does not change the logic defect.
- **Verdict**: Confirmed.

### Hypothesis 3: Partial failure leaves a dirty tree with no safe resume path

- **Evidence for**: the apply loop (`run-migrations.sh:252-297`) has no overall
  transaction — each migration's writes persist as it runs, and state is
  recorded per-migration. When a later migration fails, the cumulative output of
  the successful ones remains on the tree, so any resume trips the clean-tree
  pre-flight (`:89-140`). The only documented escape is
  `ACCELERATOR_MIGRATE_FORCE=1`, which disables the *one* guard protecting the
  corpus. Recovery therefore required the agent to (a) hand-edit corpus files and
  (b) bypass the safety guard — exactly the improvisation the guard exists to
  deter. There is no first-class "resume after partial failure" that keeps the
  guard meaningful (the dirty-tree handler only special-cases an in-flight
  *interactive session log*, `:90-134`, not a partial mechanical run).
- **Evidence against**: VCS revert is the intended recovery model, and forcing
  is a deliberate, documented escape hatch — so this is arguably working as
  designed. But "as designed" still forces a guard-off, hand-edit recovery,
  which is the symptom.
- **Verdict**: Confirmed (as an amplifier, not the root trigger).

## Root Cause

The interactive migration contract was designed around a **human typing answers
at a terminal**, but the only supported way to invoke it — the `migrate` skill —
runs the driver through an **agent's non-interactive Bash tool**. The prompt
loop's input sources (`/dev/tty`, fd 0, or a test-only decisions file) are all
unreachable from the agent: it cannot type into `/dev/tty`, fd 0 is at EOF, and
the decisions file is undocumented and unset. So the moment any migration emits a
`PROMPT` frame, the run is structurally unsatisfiable and aborts after the
migration has already mutated the corpus (backfill + rewrite run before
`harness_run`).

This primary defect was reached only because two upstream issues each forced a
detour first: `0007`'s self-validation hard-fails on a required extra its own
backfill tolerantly left absent (H2), and a partial-run failure leaves a dirty
tree whose only resume path is to disable the safety guard (H3).

## Causal Chain

1. The `migrate` skill is invoked; the agent runs the driver via the Bash tool,
   which has no interactive stdin/TTY.
2. Mechanical migrations apply and write to `meta/`; the tree becomes dirty with
   no transactional boundary.
3. `0007`'s rewrite cannot derive `pr_number` for tracker-key-named PR files and
   tolerantly leaves the extra absent (DIVERGE).
4. `0007`'s own structural self-validation hard-fails on that absent required
   extra → the run aborts mid-corpus.
5. Resuming is blocked by the clean-tree pre-flight; recovery requires
   hand-editing files and `ACCELERATOR_MIGRATE_FORCE=1`.
6. `0007` then reaches its interactive stage and emits a `PROMPT`.
7. `read_decision` hits EOF on the non-interactive fd 0 and returns non-zero.
8. The runner aborts the migration with zero decisions recorded — the migration
   can never complete via the supported invocation path.

## Contributing Factors

- **Test coverage masks the gap.** The interactive suite drives every scenario
  through `ACCELERATOR_MIGRATE_DECISIONS_FILE`. It proves the wire protocol works
  *given a decisions file*; it never exercises the actual agent-driven
  invocation path, so the "no input channel" failure is invisible to CI.
- **The decisions-file seam is deliberately hidden.** `run-migrations.sh:14-37`
  documents it as test-only and "never documented in --help or any user-facing
  banner," so the skill author never told the agent it exists or how to populate
  it — and an agent could not populate it correctly anyway without first seeing
  the prompts (the proposed values it must decide on are only revealed by the
  prompts themselves).
- **SKILL.md addresses the migration *author*, not the *invoker*.** The
  extensive interactive contract docs explain how to *write* an interactive
  migration; nothing tells the agent *invoking* the skill how a prompt should be
  answered, or that it cannot be.
- **`0007` runs many mechanical mutations before `harness_run`**, so the
  interactive abort happens after the corpus is already changed, coupling the
  interactive defect (H1) to the dirty-tree recovery problem (H3).
- **No derivable default ≠ no breadcrumb.** The backfill leaves the extra fully
  absent rather than writing a sentinel placeholder, which is what makes the
  subsequent validator fail closed.

## Fix Options

| Option | Description                                                                                                                                                                                                                                                                                                                    | Risk | Effort |
|--------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------|--------|
| A      | Bridge decisions to the agent: have the skill collect the full list of pending transformations up front (a `--list`/dry emit mode), let the agent decide each via its own prompt-to-user or policy, write a decisions file, and pass it to the driver. Promote `ACCELERATOR_MIGRATE_DECISIONS_FILE` to a documented interface. | Med  | Med    |
| B      | Make `read_decision` detect the no-input case and emit an actionable, structured stall (list the pending keys + the exact resume command) instead of a bare `failed to obtain decision`, so the agent/user can act deterministically.                                                                                          | Low  | Low    |
| C      | Reconcile 0007's backfill with its validator: when a required extra has no derivable default, write a sentinel placeholder (e.g. `pending`) the validator accepts, instead of leaving it absent — turning a hard abort into a benign breadcrumb.                                                                               | Low  | Low    |
| D      | Split 0007: keep the mechanical frontmatter unification as a non-interactive migration that always completes; move body-section typed linkage into a *separate, optional, human-run* step (or a non-blocking deferral) so a mechanical upgrade never strands on an interactive prompt.                                         | Med  | Med    |
| E      | Add a resume-safe partial-failure mode: detect that the dirty paths are this run's own prior-migration output and offer a guarded resume, rather than forcing `ACCELERATOR_MIGRATE_FORCE=1`.                                                                                                                                   | Med  | Med    |

## Recommended Fix

Adopt **C + D** as the durable fix, with **B** as an immediate low-risk
mitigation.

- **C** removes the self-contradiction in `0007` so a real-world corpus stops
  hard-failing on un-derivable required extras — cheap, contained, and directly
  unblocks the first failure class.
- **D** addresses the fundamental flaw: a schema-upgrade migration should be able
  to complete mechanically. Body-section typed linkage is genuinely a human
  editorial judgment, so it belongs in an explicitly human-driven step that does
  not gate the mechanical upgrade. This decouples "bring the repo to the new
  schema" (must be agent-runnable) from "curate cross-document links" (needs a
  human).
- **B** is worth shipping regardless: the current failure mode is an opaque
  abort; a structured stall that names the pending decisions and the resume
  command makes every interactive migration debuggable and gives the agent
  something to relay to the user.

**A** (a full agent↔decisions bridge) is the most general answer but the most
involved, and it still asks the agent to make editorial linkage judgments that
are arguably the human's to make — so it is better as a follow-up than the first
move. **E** is valuable hygiene but treats a symptom; it is secondary to C/D.

## Prevention

- **Test the invocation path, not just the protocol.** Add a test that drives an
  interactive migration the way the skill actually does — no TTY, no decisions
  file — and assert the *intended* behaviour (a structured deferral, not an
  opaque abort). The protocol-via-decisions-file tests proved the wrong thing.
- **Forbid hard-fail-on-tolerated-state.** Any migration that tolerantly defers
  a transformation (DIVERGE/continue) must not then fail its own validation on
  that same state. A lint/test that cross-checks "what the backfill leaves
  absent" against "what the validator requires" would catch this class.
- **Keep schema-upgrade migrations non-interactive by default.** Treat any
  `# INTERACTIVE: yes` migration as a special case that must degrade gracefully
  (defer, don't block) when no decision channel exists.
- **Document the invoker contract.** SKILL.md should state plainly whether and
  how interactive migrations can be answered under agent invocation, and what
  happens when they cannot.

## Recent Changes

- `0007` is recent and actively evolving: `Backfill required type-extras in
  0007` (26d18b0b7) introduced the required-extras backfill whose
  no-default branch (H2) contradicts the self-validation. `Drop schema-forbidden
  own-id keys and fold pr_title in 0007` (4aaf2703a) and the config-aware
  classifier work (3a9917ecd, 07f48ec55) are adjacent.
- The interactive runner library is comparably young: `Add interactive migration
  harness, runner-side library, and FIFO plumbing` (3c3ce54f6) and `Wire
  predicate routing, display rendering, and the accept verb` (fa0af774e) — both
  built and tested exclusively against the scripted decisions file, never the
  agent path.

## Open Questions

- Should body-section typed linkage ever run under agent invocation at all, or is
  it inherently a human-only curation step? The answer determines whether the fix
  is "bridge the decisions" (A) or "separate the concern" (D).
- For a partial-run resume, is detecting "the dirty paths are this run's own
  output" reliable enough to offer a guarded resume, or should the framework
  adopt a real staging/transaction boundary instead?
