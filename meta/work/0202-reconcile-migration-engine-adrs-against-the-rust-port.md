---
type: "work-item"
id: "0202"
title: "Supersede Migration-Engine ADRs with Rust-Native Successors"
date: "2026-08-09T08:00:32+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "ready"
kind: "task"
priority: "medium"
parent: "work-item:0312"
relates_to: ["work-item:0172", "work-item:0070", "work-item:0119", "work-item:0241", "work-item:0298", "work-item:0115", "work-item:0116", "work-item:0117", "work-item:0214", "work-item:0157", "work-item:0136", "work-item:0313", "work-item:0318"]
tags: ["rust", "migration-engine", "adr", "reconciliation"]
last_updated: "2026-10-09T09:34:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-732"
---

# 0202: Supersede Migration-Engine ADRs with Rust-Native Successors

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Work item 0172 ported the meta-directory migration framework from bash to
native Rust (`accelerator-migrate`, `cli/migrate` / `cli/migrate-adapters` /
`cli/migrate-cli`), including the optional interactive contract. A migration
is now a Rust module implementing the `Migration`/`InteractiveMigration`
traits directly, not a bash script opting into a contract via a header, and
migration 0007's interactive-validation parameters live as ordinary in-crate
logic (`cli/migrate/src/migrations/m0007/`). The three migration-framework
ADRs (ADR-0023, ADR-0037, ADR-0038) still describe the bash-era shape.

This item supersedes all three with Rust-native successors, each accounting
for every decision its predecessor made. Each successor also records
decisions the ADR record has never held: the ADR-0023 successor records the
guarded-resume pre-flight from work-item:0119 and migration-level skip; the
ADR-0037 successor — the bulk of the effort and the critical-path
acceptance — records the interactive mechanics that until now lived only in
`skills/config/migrate/SKILL.md` and 0172's plan; and the ADR-0038 successor
records how migration 0007 fills the engine-owned session-log record. Live
documents that cite a superseded ADR are repointed at its successor.

## Context

Three ADRs describe the bash-era migration framework, all `accepted`:

- **ADR-0023** (meta-directory migration framework) — the base mechanical
  contract. Its text pins the ledger at `meta/.migrations-applied` (a stale
  ledger path since migration 0003 moved it to
  `.accelerator/state/migrations-applied`, flagged by work-item:0070) and
  names `run-migrations.sh`, `hooks/migrate-discoverability.sh`, and the
  `[0-9][0-9][0-9][0-9]-*.sh` discovery glob. Its strict clean-tree
  pre-flight has since been relaxed by work-item:0119 into guarded resume,
  with no ADR recording the change.
- **ADR-0037** (optional interactive contract, supplementing ADR-0023) — an
  abstract contract: an opt-in trigger predicate, three mandatory display
  elements, a declared resumability persistence artefact (session log and
  resume state), and accept / edit / skip semantics. It deliberately
  declines to commit to a callback ABI, process model, or persistence
  format. Its only concrete machinery reference is the runner
  `skills/config/migrate/scripts/run-migrations.sh`. §5 requires any
  extension to the interactive contract (a new control verb, display
  element, or resumability guarantee) to be recorded in a new ADR rather
  than left implicit.
- **ADR-0038** (interactive validation parameters for the unified schema
  linkage migration) — parameterises ADR-0037 for migration 0007. Line 122
  defines the migration ID as "the `NNNN-<slug>` form of its file in
  `skills/config/migrate/migrations/`, assigned at the time the migration
  script is added"; line 124 fixes the session-log record fields.

The bash implementation of the interactive contract added mechanics that
were never ADR decisions. 0172's Phase 10 cutover deleted the bash
artefacts: the `# INTERACTIVE: yes` header, the four `migration_*` author
callbacks, the FIFO/fd wire protocol, and `run-migrations.sh`. The
behavioural rules those artefacts implemented survive in Rust: the
write-ahead-log invariant, sticky skip, source drift, and callback
determinism. Today they are documented only in
`skills/config/migrate/SKILL.md` (Interactive contract section) and 0172's
plan.

0172's plan (`meta/plans/2026-08-07-0172-migration-engine-subdomain.md`)
scoped ADR reconciliation out of its own work: "No replacement author-facing
migration-authoring API — a migration becomes ordinary in-crate Rust; there
is no opt-in header or published hook set to design." It also added
behaviour with no bash precedent: the 30-second TTY decision timeout, and the
`verify_applied` and `finalise` trait methods. The session log is now owned
by the engine (the `SessionLog` port in `cli/migrate/src/ports.rs`), not by
each migration.

work-item:0117 decided that the agent decisions bridge (`--list`,
`--decisions-file`, the structured stall from work-item:0116) is a
driver-level implementation detail, not an ADR-0037 §5 extension, because
ADR-0037 is neutral on how the runner is invoked.

ADR-0031 (skill-level ADR immutability, `accepted`) permits no content edits
to a non-`proposed` ADR; an accepted ADR changes only by moving to
`superseded` (via `create-adr --supersedes`) or `deprecated` (via
`review-adr`). Recent changes to accepted ADRs have followed that path
(work-item:0214, work-item:0157), so in-place amendment is not available.
Supersession also discharges ADR-0037 §5: the successor is the new ADR that
records the extensions, and supersession rather than a supplement is needed
because ADR-0037's own text names a deleted runner.

ADR-0047, ADR-0052, and ADR-0053, also cited by 0172's plan, name none of
the deleted machinery or the stale ledger path and are out of scope.

### Terms

- **Holds** — an ADR's text holds when it still accurately describes the
  shipped system.
- **Predicate routing** — each emitted transformation is sent to the
  interactive path when its trigger predicate fires and applied mechanically
  otherwise.
- **Sticky skip** — a transformation skipped on a prior run stays skipped on
  every later run unless its source drifts.
- **Source drift** — a recorded decision whose `proposed_value` differs from
  the live emission is discarded and the transformation re-prompted. Used
  only in this session-log sense.
- **Proposed transformation** — the display element showing the artefact
  mutation that would apply if accepted (ADR-0037's term). Distinct from the
  `proposed_value` session-log field, which records only the value.
- **Migration lag** — in an Accelerator-managed repo, the highest applied
  migration ID is lower than the highest bundled migration ID. This is
  ADR-0023's rule, unchanged in `cli/migrate-cli/src/discoverability.rs`.
- **Stall** — the structured halt (`MIGRATION STALLED: no decision input
  available`) an interactive run emits when it has no decision source.
- **Run base** — the VCS revision a migration run records when it starts.
- **Path manifest** — the per-run record of every path the run has written,
  appended at write time by `MigrationContext::write`.
- **Own dirt / foreign dirt** — a dirty path is the run's own dirt when it
  is listed in the current run's path manifest, and foreign dirt otherwise.
- **Guarded resume** — the pre-flight proceeds over a dirty tree when every
  dirty path is own dirt and the run base is unchanged (work-item:0119).
- **Force bypass** — `ACCELERATOR_MIGRATE_FORCE=1`, which skips the dirty-tree
  pre-flight only.
- **Repeatable callbacks** — `emit_transformations`, `evaluate_predicate`,
  `validate_edit`, and `verify_applied`, which the engine may invoke more
  than once per run (`verify_applied` from both the dry-apply validation and
  the live run in `engine.rs`). `apply_decision` runs once per accepted or
  edited decision and `finalise` once per run, so neither is in this set.
- **Supplements / parameterises** — a successor supplements or parameterises
  another ADR when its title names that ADR's ID, its frontmatter
  `relates_to` lists it, and its Context names it. A successor that
  supplements the ADR-0023 successor "not ADR-0023" fails if its title
  names ADR-0023; mentions of ADR-0023 elsewhere are permitted only as the
  superseded predecessor.

## Requirements

- Read ADR-0023, ADR-0037, and ADR-0038 in full alongside the shipped Rust
  design: `cli/migrate/src/registry.rs` (`Migration`/`MigrationMeta`),
  `cli/migrate/src/interactive.rs` (`InteractiveMigration`,
  `Transformation`, `Decision`, `PredicateOutcome`), `cli/migrate/src/ports.rs`
  (`MigrationContext`, `DecisionSource`, `SessionLog`),
  `cli/migrate/src/engine.rs` (`run_interactive`), and
  `skills/config/migrate/SKILL.md`.
- Before drafting the first successor, check that each ADR's triggering
  text in the Dispositions table no longer holds, and set that row's
  Confirmed column to `yes`. If any triggering text does hold, stop and
  re-scope this item with Toby Clemson before drafting anything.
- Produce the successors in this order, accepting each before drafting the
  next:
  1. **ADR-0023 successor** (base framework) — states the ledger location
     `.accelerator/state/migrations-applied`, compiled-in migration
     registration via the `Migration` trait and registry (replacing script
     discovery), and the pre-flight as it now behaves: refusal over foreign
     dirt, guarded resume, and the force bypass. It states that migration
     lag is surfaced at session start without naming the hook that delivers
     it, so it holds whichever way work-item:0298 lands. It also records
     migration-level skip, which ADR-0037 §4 already attributes to the base
     framework: `--skip` and `--unskip`, the
     `.accelerator/state/migrations-skipped` ledger, that a skipped
     migration never runs and does not block other pending migrations, that
     applied takes precedence when an ID is in both ledgers, and that the
     force bypass does not unskip.
  2. **ADR-0037 successor** (interactive contract), supplementing the
     ADR-0023 successor — maps each contract primitive onto a named
     `InteractiveMigration` method or engine port, and states:
     - predicate routing between the interactive and mechanical paths
     - the three mandatory display elements by ADR-0037's names: the
       proposed transformation, the source location, and the trigger
       predicate's evaluated value
     - accept, edit, and skip, each by its artefact effect and session-log
       effect: accept applies the proposed transformation and records
       `outcome: accepted`; edit applies the user's value once `validate_edit` accepts
       it and records `outcome: edited` with `user_value`; skip leaves the
       artefact untouched, records `outcome: skipped`, and never calls
       `apply_decision`
     - that the engine owns the session log, and its record fields:
       `transformation_key`, `schema_version`, `outcome`, `proposed_value`,
       `user_value` (edited only), `timestamp`
     - the write-ahead-log invariant: the session-log record is durably
       persisted before `apply_decision` mutates the artefact
     - sticky skip and source drift
     - that the repeatable callbacks must be deterministic and
       side-effect-free (`emit_transformations` enumerates and never
       mutates; m0007's mechanical phase is recorded as known
       non-compliance tracked by work-item:0313)
     - that an unanswered TTY prompt fails the run after 30 seconds
     - the purpose of `verify_applied` and of `finalise` (see Technical
       Notes)
     - that band design is migration-owned (no framework-level band model)
     - that how the runner is invoked (`--list`, `--decisions-file`, the
       no-input stall) is outside the contract, per work-item:0117
     - a recursive-extension clause equivalent to ADR-0037 §5, applying to
       the successor itself

     Sources: `skills/config/migrate/SKILL.md` (Interactive contract
     section), 0172's plan, and `cli/migrate/src/interactive.rs` /
     `engine.rs` / `ports.rs`.
  3. **ADR-0038 successor** (migration 0007 parameters), parameterising the
     ADR-0037 successor — restates the two-band model and cites ADR-0034
     for the linkage-key vocabulary rather than relisting it, identifies migration 0007 by its `MigrationMeta` id rather
     than a script file, and states how 0007 fills the engine-owned
     `transformation_key`, `proposed_value`, and `user_value` fields. It
     classifies each of ADR-0038 line 124's eight fields (`artifact_path`,
     `source_anchor`, `inferred_key`, `inferred_target`, `band`, `decision`,
     `edited_key`, `edited_target`) exactly once as carried, renamed to a
     named field, folded into a named engine field (its information is
     encoded inside that field's value), or dropped with a reason.
- Give each successor a "Carried-forward decisions" section with two
  subsections. "Restated" restates each still-valid predecessor decision
  against the Rust shape. "Superseded design" lists each dropped decision
  with its reason. Outside the Context section, the bash-era literals listed
  in the Acceptance Criteria may appear only in "Superseded design". Every
  "Restated" entry cites the Rust type, method, or `SKILL.md` section that
  shows it holds. Every item on the predecessor's checklist below appears in
  exactly one of the two subsections:
  - **ADR-0023** — from its Decision section: Registry (ordered
    registration, per-migration description — now
    `MigrationMeta::description`, project-root access — now via
    `MigrationContext`, per-migration idempotency, atomic writes); State file (newline-delimited ledger, unknown-ID
    preservation); Driver steps 1–5 (clean-tree pre-flight with force
    bypass, pending computation, one-line preview, ordered apply with abort
    and no partial ledger write, end-of-run summary table); Pinned-path
    preservation; Collision guard; Discoverability hook (the lag rule and
    the Accelerator-repos-only guard); First migration. From its chosen
    Considered Options: a skill-driven framework, the newline ledger, no
    dry-run (pre-flight plus preview instead), VCS-revert-only rollback,
    SessionStart discoverability.
  - **ADR-0037** — Decision §1–§5 (trigger predicate, display elements,
    resumability mechanism, accept / edit / skip controls, recursive
    supplement clause), and chosen Option 5 (opt-in; migrations that do not
    opt in run unchanged on the mechanical path).
  - **ADR-0038** — each Decision subsection: band design, application
    shape, trigger predicate parameterisation, display elements beyond the
    mandatory three, edit mutation targets, resumability persistence
    artefact, parser accuracy fixes, `Source:` prose on plans as `parent`,
    broader-workstream linkage.
- Draft each successor with `/accelerator:create-adr --supersedes` and have
  Toby Clemson take it to `accepted` with `/accelerator:review-adr`,
  revising it while `proposed` rather than rejecting it. Each successor
  records the behaviour that shipped; a dispute over whether a newly
  recorded decision is right (for example the 30-second timeout value)
  becomes a follow-up work item rather than holding the successor open.
- Fill the Dispositions table's Successor column with each accepted
  successor's ID.
- As each successor is accepted, repoint every citation of its predecessor
  in the body text of live documents, so each filled Successor cell marks a
  consistent stopping point. Replace the citation; keep the old ID only
  where the sentence is about the superseded design itself, written as
  `ADR-NNNN (superseded by ADR-MMMM)`. Before closing, re-run the sweep to
  catch citations other items added during delivery.
  - In scope: body text under `skills/` and work items under `meta/work/`
    whose status is not `done`, other than this one. Today that is
    `skills/config/migrate/SKILL.md` (line 68 and the Cross-references
    section), all of which are replaced rather than annotated.
  - `docs-site/src/content/docs/reference/skills/config/migrate.md` is
    gitignored and generated from `SKILL.md` by the docs build; it is
    regenerated, not edited.
  - Out of scope: frontmatter linkage values (resolved through the
    superseded ADR's `superseded_by`), other ADRs' bodies (immutable), and
    `done` work items, plans, and research, which are point-in-time
    historical records.

## Acceptance Criteria

- [ ] Given the Dispositions table, when this item completes, then every
      row's Confirmed column is `yes` and its Successor column names an
      accepted ADR ID.
- [ ] Given ADR-0023, ADR-0037, and ADR-0038, when this item completes, then
      each has `status: superseded` and a `superseded_by` link to the
      successor named in its Dispositions row, which has `status: accepted`.
- [ ] Given each successor ADR, when its "Carried-forward decisions" section
      is compared with its predecessor's checklist in Requirements, then
      every checklist item appears in exactly one of "Restated" or
      "Superseded design", every "Restated" entry cites the Rust type,
      method, or `SKILL.md` section that shows it holds, and every
      "Superseded design" entry gives a reason.
- [ ] Given each predecessor ADR, when its diff across this item is
      inspected, then only its frontmatter `status`, `superseded_by`, and
      timestamp fields changed.
- [ ] Given the ADR-0023 successor, when it is read, then it states:
      - the ledger path `.accelerator/state/migrations-applied`
      - that migrations are registered at compile time, not discovered from
        scripts
      - that the pre-flight refuses over foreign dirt
      - that guarded resume proceeds when every dirty path is own dirt and
        the run base is unchanged
      - that `ACCELERATOR_MIGRATE_FORCE=1` bypasses the dirty-tree pre-flight
      - that migration lag is surfaced at session start, without naming the
        delivering hook
      - that `--skip` and `--unskip` maintain
        `.accelerator/state/migrations-skipped`; that a skipped migration
        never runs and does not block others; that applied takes precedence
        when an ID is in both ledgers; and that the force bypass does not
        unskip
- [ ] Given the ADR-0037 successor, when it is read, then it:
      - supplements the ADR-0023 successor, not ADR-0023
      - maps each contract primitive onto a named `InteractiveMigration`
        method or engine port
      - routes each transformation to the prompt loop or the mechanical path
        by its predicate outcome
      - names the proposed transformation, the source location, and the
        trigger predicate's evaluated value as the mandatory display
        elements
      - states that accept applies the proposed transformation and records
        `outcome: accepted`; that edit applies the user's value once `validate_edit`
        accepts it and records `outcome: edited` with `user_value`; and that
        skip leaves the artefact untouched, records `outcome: skipped`, and
        never calls `apply_decision`
      - lists the session-log record fields `transformation_key`,
        `schema_version`, `outcome`, `proposed_value`, `user_value`,
        `timestamp`, as engine-owned
      - states that the session-log record is durably persisted before
        `apply_decision` mutates the artefact
      - states that `emit_transformations`, `evaluate_predicate`,
        `validate_edit`, and `verify_applied` must be deterministic and
        side-effect-free
      - states that a skipped transformation stays skipped unless its
        source drifts, and that a drifted record is discarded and
        re-prompted
      - states that an unanswered TTY prompt fails the run after 30 seconds
      - states that `verify_applied` confirms on resume that a recorded
        accept or edit is still applied, re-prompting when it is not, and
        that `finalise` runs once after all applications for whole-corpus
        validation
      - states that band design is migration-owned
      - states that `--list`, `--decisions-file`, and the no-input stall are
        outside the contract
      - carries a recursive-extension clause applying to itself
- [ ] Given the ADR-0038 successor, when it is read, then it parameterises
      the ADR-0037 successor (not ADR-0037), states the two-band model,
      cites ADR-0034 for the linkage-key vocabulary, identifies migration 0007 by its
      `MigrationMeta` id, states how 0007 fills `transformation_key`,
      `proposed_value`, and `user_value`, and classifies each of the eight
      ADR-0038 line 124 fields exactly once as carried, renamed, folded, or
      dropped with a reason.
- [ ] Given the three successor ADRs, when searched case-sensitively for
      the literals `run-migrations.sh`, `migrate-discoverability.sh`,
      `hooks/`, `meta/.migrations-applied`, `# INTERACTIVE`,
      `# DESCRIPTION`, `PROJECT_ROOT`, `FIFO`, `migration_emit`,
      `migration_evaluate`, `migration_validate`, `migration_apply`,
      `[0-9][0-9][0-9][0-9]-*.sh`, and `skills/config/migrate/migrations/`,
      then every hit lies in the ADR's Context section or its "Superseded
      design" subsection.
- [ ] Given `skills/config/migrate/SKILL.md`, when this item completes,
      then each citation of ADR-0023, ADR-0037, or ADR-0038 it held before
      this item now cites that ADR's successor from the
      Dispositions table, with no `superseded by` annotation.
- [ ] Given body text under `skills/` and work items
      under `meta/work/` whose status is not `done` (excluding
      work-item:0202), when this item completes, then every remaining
      occurrence of `ADR-0023`, `ADR-0037`, or `ADR-0038` there has the
      form `ADR-NNNN (superseded by ADR-MMMM)`, with MMMM the successor in
      that ADR's Dispositions row.

## Dispositions

| ADR | Disposition | Triggering text | Confirmed | Successor |
|---|---|---|---|---|
| ADR-0023 | Supersede | `meta/.migrations-applied`, `run-migrations.sh`, `hooks/migrate-discoverability.sh`, `*.sh` discovery glob | TBD | TBD |
| ADR-0037 | Supersede | `skills/config/migrate/scripts/run-migrations.sh` | TBD | TBD |
| ADR-0038 | Supersede | line 122: `skills/config/migrate/migrations/`, "the migration script" | TBD | TBD |

## Open Questions

None.

## Dependencies

- Blocked by: work-item:0172 (done — the Rust shape this item reconciles
  against stabilised at 0172's Phase 10 cutover).
- Blocks: completion of parent work-item:0312, which closes only when every
  child reaches a terminal status.
- Approval: Toby Clemson accepts each successor. The three acceptances are
  sequential gates on the critical path, and each predecessor sits in
  `superseded` from its successor's draft until that acceptance.
- Relies on: the supersession behaviour of
  `skills/decisions/create-adr/SKILL.md` and
  `skills/decisions/review-adr/SKILL.md` described in Technical Notes; a
  change to either during delivery invalidates the sequencing and
  no-reject rules here.
- Relates to:
  - work-item:0070 — its stale-ledger-path flag is resolved by the ADR-0023
    successor.
  - work-item:0119 (done) — introduced the guarded resume the ADR-0023
    successor records.
  - work-item:0241 (ready) — changes how the jj dirty-path computation
    agrees with `jj status`, not the pre-flight rule; the ADR-0023 successor
    states the rule, so 0241 landing before or after does not invalidate it.
    An ADR-0023 citation it adds before this item closes is caught by the
    closing sweep.
  - work-item:0298 (draft) — consolidates the SessionStart hooks; the
    ADR-0023 successor states only that lag is surfaced at session start, so
    0298 landing before or after does not invalidate it. An ADR-0023
    citation it adds before this item closes is caught by the closing sweep.
  - work-item:0115, work-item:0116, work-item:0117 (done) — shaped
    `DecisionSource` and the invocation surfaces the ADR-0037 successor
    places outside the contract.
  - work-item:0214, work-item:0157 — supersession precedents.

## Assumptions

- Three per-ADR successors rather than one blanket ADR: the three cover
  genuinely different concerns (base framework, generic interactive
  contract, one migration's parameterisation), and keeping them separate
  preserves the generic/specific split ADR-0037 and ADR-0038 established.
- One item rather than three children: each successor depends on the one
  before it being accepted, and the Successor column of the Dispositions
  table provides the intermediate checkpoints.
- Kind stays `task`: the work produces documents only, with no code change.

## Technical Notes

- The `InteractiveMigration` trait (`cli/migrate/src/interactive.rs`) is the
  Rust analogue of the bash implementation's four callbacks:
  `emit_transformations` ↔ `migration_emit_transformations`,
  `evaluate_predicate` ↔ `migration_evaluate_predicate`, `validate_edit` ↔
  `migration_validate_edit`, `apply_decision` ↔ `migration_apply_decision`.
- `verify_applied` (optional, default `true`) is consulted on resume before
  replaying an accepted or edited record; `false` discards the record and
  re-prompts the transformation, as source drift does. It is never consulted
  for a skipped record.
- `finalise` (optional, default no-op) runs exactly once per run, after
  every transformation decided in that run has been applied, and is where
  whole-corpus post-apply validation belongs.
- 0172's plan documents the 30-second TTY decision timeout as deliberately
  new behaviour; bash's TTY read had no timeout.
- The Rust session log is `.accelerator/state/migrations-<id>-session.jsonl`
  — still line-delimited JSON, so ADR-0038's format claim holds; the record
  fields are what need comparing.
- `create-adr --supersedes` flips the predecessor to `superseded` when the
  successor is drafted, while the successor is still `proposed`. Rejecting
  that successor in `review-adr` is terminal and would strand the
  predecessor, and `create-adr` refuses to supersede an ADR that is not
  `accepted`, so the tooling cannot repair it. Successors are therefore
  revised while `proposed` and never rejected.
- Superseded ADRs keep their bodies verbatim; only `status` and
  `superseded_by` change, per ADR-0031.

## Drafting Notes

- The original draft offered an "Amend" disposition (short in-place note
  updating references). ADR-0031 forbids content edits to non-`proposed`
  ADRs. An errata-block convention was considered and rejected: it would
  require superseding ADR-0031 and changing `create-adr`/`review-adr`, a
  lifecycle-wide change disproportionate to stale references.
- An earlier draft let the implementer choose between Supersede, Deprecate,
  and Retain per ADR. Review found all three ADRs name deleted machinery, so
  the dispositions are fixed in the Dispositions table. A later draft
  allowed a per-row "Not superseded" fallback, but it could not be applied
  to one row without invalidating the successors that supplement or
  parameterise it; a contrary finding now pauses the item for re-scoping
  before any successor is drafted.
- The ADR-0023 successor records migration-level skip because ADR-0037 §4
  already cites it as a base-framework mechanism. Other shipped
  base-framework mechanics — the `NoOpPending` soft skip, the run-level
  advisory lock, and the exact set of paths the pre-flight scans — are
  implementation detail documented in `skills/config/migrate/SKILL.md`, not
  ADR decisions.
- The 30-second TTY timeout and the `verify_applied`/`finalise` methods are
  folded into the ADR-0037 successor rather than given standalone ADRs —
  they are facets of the same interactive contract.
- The two-band model stays in the ADR-0038 successor; ADR-0037 already held
  that band design is a migration concern, and promoting it to the generic
  contract would constrain future interactive migrations. The session-log
  record shape moves the other way, to the ADR-0037 successor, because the
  engine now owns the log.
- The carried-forward checklists are closed and keyed to each predecessor's
  Decision headings plus its chosen Considered Options, because ADR-0023
  records no-dry-run, rollback, and discoverability as chosen options rather
  than in its Decision section.
- ADR-0023's stale ledger path and the 0119 guarded resume sit outside the
  Rust port but are in scope: the staleness test is whether the text still
  holds, whatever the cause, and the successor restates both anyway.
- The review criterion is "successor reaches `status: accepted`" rather than
  "passes with no unresolved findings", because `review-adr` findings are
  advisory; the content criteria carry the quality bar.

## References

- Related: work-item:0172, work-item:0070, work-item:0119, work-item:0241,
  work-item:0298, work-item:0115, work-item:0116, work-item:0117,
  work-item:0214, work-item:0157, work-item:0136
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md`
- `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md`
- `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md`
- `meta/plans/2026-08-07-0172-migration-engine-subdomain.md`
- `cli/migrate/src/registry.rs`, `cli/migrate/src/interactive.rs`,
  `cli/migrate/src/ports.rs`, `cli/migrate/src/engine.rs`
- `skills/config/migrate/SKILL.md`
- `docs-site/src/content/docs/reference/skills/config/migrate.md`
- `skills/decisions/create-adr/SKILL.md`
- `skills/decisions/review-adr/SKILL.md`
