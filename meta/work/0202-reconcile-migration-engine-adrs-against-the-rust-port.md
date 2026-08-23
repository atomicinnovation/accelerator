---
type: work-item
id: "0202"
title: "Reconcile Migration-Engine ADRs Against the Rust Port"
date: "2026-08-09T08:00:32+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: medium
parent: "work-item:0136"
relates_to: ["work-item:0172"]
tags: [rust, migration-engine, adr, reconciliation]
last_updated: "2026-08-09T08:00:32+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-732
---

# 0202: Reconcile Migration-Engine ADRs Against the Rust Port

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Work item 0172 ported the meta-directory migration framework from bash to
native Rust (`accelerator-migrate`, `cli/migrate` / `cli/migrate-adapters` /
`cli/migrate-cli`), including the optional interactive contract. The Rust
engine encodes ADR-0038's interactive-validation parameters (the two-band
predicate model — `ambiguous` vs `resolved` — and the field-set/linkage-key
vocabulary) as ordinary in-crate Rust logic
(`cli/migrate/src/migrations/m0007/`), with no replacement author-facing API,
header convention, or published hook set: a migration is now a Rust module
implementing the `Migration`/`InteractiveMigration` traits directly, not a
bash script opting into a documented contract via `# INTERACTIVE: yes`.
ADR-0037 §5's recursive supplement clause requires this kind of framework
extension be reconciled with the ADR record formally, not left as an implicit
drift between the ADRs' bash-era text and the shipped Rust reality. This item
performs that reconciliation.

## Context

Three ADRs describe the bash-era migration framework:

- **ADR-0023** (meta-directory migration framework) — the base mechanical
  contract: script discovery, the applied/skipped ledger, atomic writes,
  idempotency self-checks, the no-dry-run decision.
- **ADR-0037** (optional interactive contract, supplementing ADR-0023) — the
  `# INTERACTIVE: yes` opt-in, the four author callbacks
  (`migration_emit_transformations`/`migration_evaluate_predicate`/
  `migration_validate_edit`/`migration_apply_decision`), the FIFO/fd wire
  protocol, the write-ahead-log resumability invariant, sticky skip, and
  callback determinism (§5's own recursive supplement clause).
- **ADR-0038** (interactive validation parameters for the unified schema
  linkage migration) — the two-band (`ambiguous`/`resolved`) predicate model
  and the linkage-key vocabulary migration 0007 uses, parameterising
  ADR-0037's generic contract for that specific migration.

0172's plan (`meta/plans/2026-08-07-0172-migration-engine-subdomain.md`)
explicitly scoped ADR reconciliation out of its own work: "No new
interactive framework primitive... beyond what ADR-0037/0038 already
specify" and "No replacement author-facing migration-authoring API — a
migration becomes ordinary in-crate Rust; there is no opt-in header or
published hook set to design" — i.e. 0172 built the Rust-native shape but
did not update the ADR record to describe it. The retirement cutover
(0172's Phase 10) deleted every bash mechanism these ADRs describe
byte-for-byte (the FIFO wire protocol, the awk JSON parser, the
`# INTERACTIVE: yes` header convention, `scripts/interactive-harness.sh`),
so the ADRs' own text now describes machinery that no longer exists in the
codebase, while the design decisions they captured (predicate routing,
mandatory display elements, write-ahead-log resumability, sticky skip,
callback determinism, the two-band model) remain fully in force, just
re-expressed as Rust trait methods instead of bash callbacks and TSV frames.

## Requirements

- Read ADR-0023, ADR-0037, and ADR-0038 in full alongside the shipped Rust
  design: `cli/migrate/src/registry.rs` (`Migration`/`MigrationMeta`),
  `cli/migrate/src/interactive.rs` (`InteractiveMigration`,
  `Transformation`, `Decision`, `PredicateOutcome`), `cli/migrate/src/ports.rs`
  (`MigrationContext`, `DecisionSource`), `cli/migrate/src/engine.rs`
  (`run_interactive`), and `skills/config/migrate/SKILL.md` (already
  rewritten by 0172 to describe the Rust contract).
- For each ADR, determine which of the following applies and act on it:
  - **Deprecate** — the decision was specific to a bash mechanism that no
    longer exists (e.g. the FIFO/fd transport, the TSV frame format, the
    `harness_*` helper functions, the `# INTERACTIVE: yes` header
    convention) and has no Rust analogue because the problem it solved
    (crossing a process boundary) no longer exists.
  - **Supersede** — the decision's *intent* still holds but its *mechanism*
    changed enough that a new ADR describing the Rust-native shape is
    clearer than amending the old one in place.
  - **Amend** — the decision and its mechanism both still hold materially
    unchanged (e.g. ADR-0023's no-dry-run rule, ADR-0038's two-band model),
    and a short amending note updating file/function references is
    sufficient.
- Produce the resulting ADR set (new/superseding ADRs, deprecation markers on
  retired ones, amending notes on unchanged ones) via the `create-adr` /
  `review-adr` skills, respecting ADR immutability (only proposed ADRs are
  edited in place; accepted ADRs transition to superseded/deprecated, never
  silently rewritten).
- Update any cross-references to ADR-0023/0037/0038 elsewhere in the corpus
  (other ADRs, work items, `skills/config/migrate/SKILL.md`'s own
  cross-references section) to point at the reconciled set.

## Acceptance Criteria

- [ ] Given ADR-0023, ADR-0037, and ADR-0038, when this item completes, then
      each has an explicit disposition (deprecated, superseded, or amended)
      recorded against it — none are left silently describing bash machinery
      that 0172 deleted.
- [ ] Given a reader with no prior context who opens whichever ADR(s) result
      from this reconciliation, when they read it, then they can correctly
      describe the current Rust interactive contract (predicate routing,
      display elements, write-ahead-log resumability, sticky skip, callback
      determinism, the two-band model) without needing to cross-reference
      deleted bash source.
- [ ] Given any new or superseding ADR this item produces, when it is
      reviewed via `/accelerator:review-adr`, then it passes with no
      unresolved findings.
- [ ] Given the reconciled ADR set, when `skills/config/migrate/SKILL.md`'s
      Cross-references section is checked, then it points at ADRs that are
      not marked deprecated (or explicitly explains why a deprecated ADR is
      still linked, e.g. for historical context).

## Open Questions

- Does ADR-0038's two-band model warrant its own superseding ADR once a
  second interactive migration author (beyond 0007) exists, or is amending
  it in place sufficient for now given 0007 remains the only real consumer?

## Dependencies

- Blocked by: work-item:0172 (must be done — the Rust shape this item
  reconciles against did not stabilise until 0172's Phase 10 cutover).
- Relates to: work-item:0172.

## Assumptions

- Treating "reconcile" as requiring an explicit, recorded disposition per
  ADR (deprecate / supersede / amend) rather than a single blanket
  superseding ADR covering all three — the three ADRs cover genuinely
  different concerns (base framework, generic interactive supplement,
  one migration's specific parameterisation) and are likely to warrant
  different dispositions.

## Technical Notes

- The Rust `InteractiveMigration` trait (`cli/migrate/src/interactive.rs`)
  is the direct Rust analogue of ADR-0037's four callbacks:
  `emit_transformations` ↔ `migration_emit_transformations`,
  `evaluate_predicate` ↔ `migration_evaluate_predicate`, `validate_edit` ↔
  `migration_validate_edit`, `apply_decision` ↔ `migration_apply_decision`.
  A fifth method, `verify_applied`, and a sixth, `finalise`, exist with no
  bash callback antecedent named in ADR-0037 as written (bash's
  `migration_verify_applied` is optional and undocumented in the ADR
  itself, only in `SKILL.md`) — worth checking whether ADR-0037 needs an
  amendment either way.
- ADR-0037 §5's "recursive supplement" clause is itself the mechanism this
  item's own existence satisfies — reconciling it is not optional
  bookkeeping, it is discharging an obligation the ADR record places on
  any framework change of this shape.
- 0172's own plan documents the 30-second TTY decision timeout as
  deliberately new behaviour, not a port of anything bash had — this is a
  concrete example of a decision with no ADR coverage at all yet, which
  this item's reconciliation should surface and record somewhere (a new
  ADR, or a note on ADR-0037).

## References

- Related: work-item:0172
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
- `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md`
- `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md`
- `meta/plans/2026-08-07-0172-migration-engine-subdomain.md`
