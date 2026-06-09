---
type: plan-validation
id: "2026-06-09-0070-meta-corpus-migration-dogfood"
title: "Dogfood Validation: meta/ Corpus Unified-Schema Migration"
date: "2026-06-09T10:00:00+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
parent: "work-item:0070"
target: "plan:2026-06-07-0070-meta-corpus-unified-schema-migration"
tags: [migration, dogfood, validation, frontmatter, schema]
last_updated: "2026-06-09T10:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Dogfood Validation: `meta/` Corpus Unified-Schema Migration

**Result**: pass — migration `0007` applied to this repo's `meta/` corpus; the
migrated corpus passes the unified-schema validator with **zero** violations
(structural + referential), the resolved-band wrong-rate is **1.4% (≤5%)**, and
the migration is idempotent (a ledger-bypassed re-run is a byte-for-byte no-op).

## Run summary

- **Sequence**: two pre-fix commits → re-run migration `0005` → migration `0007`.
- **Scope**: 504 in-scope `meta/` artifacts migrated; `meta/specs/` (1) and
  `meta/talks/` (1) deliberately out of scope (annotated below).
- **Migration runner**: exit 0; ledger records `0007`. Zero `0007-REFUSE` /
  `0007-MALFORMED` (AC-1).
- **Validator** (`scripts/validate-corpus-frontmatter.sh meta`): exit 0, 0
  violations — structural and referential integrity (AC-1).
- **Idempotency** (AC-17): a direct re-invocation (ledger bypassed, resumed from
  the session log) produced no `meta/` changes.

## Pre-migration corpus pre-fixes (gap-fix log)

Two genuine pre-existing data gaps were corrected before `0007` (they cannot be
mechanically derived):

1. **work-item `0032`** carried `work_item_id: "0031"` (and an H1 `# 0031:`),
   colliding with work-item `0031` — corrected to `0032`. (The migration's
   precondition pre-pass turns this latent collision into a loud REFUSE.)
2. **design-gap `0086`** (Kanban Convergence Record) was missing the required
   `current_inventory` extra — pointed at the `2026-05-21-004250-current-app`
   inventory (the current-app baseline it converges from).

## Migration `0005` re-run

`0005` (`type:`→`kind:`) had not been re-applied since 9 newer work-items were
added carrying a legacy `type: story`/`bug` with no `kind:`. Re-running `0005`
renamed those 9, and dropped the redundant `type: work-item` discriminator from
12 already-unified work-items (12 expected, recorded `divergent type/kind`
warnings — all dropping `type=work-item`); `0007` re-infers `type: work-item`
for all of them from location. This was the chosen reconciliation for the
type/kind stragglers (vs. a one-off corpus fix), so the corpus is brought fully
into line with `0005`'s contract.

## Resolved-band wrong-rate (AC-8)

- **Procedure (reproducible)**: enumerate every resolved-band body-section
  inference over the corpus (the deterministic mechanical path), stratify by the
  five header types, and draw a stratified sample of ≥150. The resolved-band
  population is **212** (≥150), so the **full population** was classified (no
  sampling error). The draw is deterministic (even-spaced per stratum, no RNG) —
  see `scripts`-adjacent sampler procedure in the work log.
- **Stratification**: `## Dependencies` 130, `## References` 70, `## Historical
  Context` 7, `## Related Research` 5, `## Source References` 0.
- **Classification**: each inference is `correct` when its emitted `doc-type:id`
  target resolves to a real artifact (existence) **and** the target's id token
  appears in the source body (faithfulness); `wrong` when it does not resolve.
- **Result**: correct **209**, wrong **3**, uncertain **0** → **wrong-rate
  1.4%** (≤5% threshold met).
- **The 3 wrong**: all `blocked_by: "work-item:2026"` on work-items `0062`,
  `0092`, `0093` — the `2026` was a year extracted from a prose-embedded date
  (`…/2026-05-24-0068-…md`) in a long `Blocked by:` line (spike 0068's
  `bare-id-misresolved` pattern). **All three were caught by the migration's
  resolved-inference existence-check and skipped** (`0007-DIVERGE[reverse-orphan]`)
  — none were written. Applied-linkage accuracy is therefore effectively 100%,
  and the corpus has zero dangling references.

## Interactive session (AC-9)

The body-section parser produced **1623 ambiguous-band** references (routed to
the 0069 interactive hook). Curation policy: accept references whose proposed
target is a real typed `doc-type:id` (or `pr:`); recover bare numbers that
resolve to a real work-item by editing to the typed form; skip the rest.

- **Session-log terminal states**: 1503 `accepted`, 77 `edited`, 43 `skipped`
  — **every routed reference reached `APPLIED_CONFIRM`; none left in `PROMPT` /
  `VALIDATE_ERR` / `DRIFT`** (AC-9).
- The 77 `edited` are bare `relates_to: "NNNN"` references recovered to
  `work-item:NNNN` (the parser left them ambiguous because a bare number's type
  is undecidable; all 77 resolve to real work-items).
- The 43 `skipped` are unrecoverable: 17 bare numbers matching no artifact and
  ~26 stale/mis-typed body-prose references that resolve to nothing. Each skip
  is an accountable decision (the relationship, if any, remains in body prose);
  no dangling edge was written.

## Annotated DIVERGE decisions

- `0007-DIVERGE[reverse-orphan]` — resolved inferences whose target does not
  resolve (the `work-item:2026` year-mis-parses) are skipped, not written.
- `0007-DIVERGE[parent-conflict]` — competing single-valued-key inferences:
  the first-set value wins (set-if-absent), the conflicting candidate is kept
  out and logged. This keeps single-key application idempotent.
- **Legacy timestamps** — a handful of `date:` values were space-separated or
  carried a timezone abbreviation (e.g. `2026-03-15 14:39:41 GMT`); these are
  normalised to the date at midnight UTC (the unrepresentable time is dropped).
- **`priority` default** — work-items lacking `priority:` are defaulted to
  `medium` (the `create-work-item` template default) — a recorded decision.
- **Author/revision** — backfilled and fenced-but-incomplete artifacts resolve
  `author`/`revision` from VCS; a genuine absence yields `Unknown` with a
  counted diagnostic.

## Out of scope (deliberate)

- `meta/specs/` (1 file) and `meta/talks/` (1 file) are excluded from the
  migration and validator per epic 0057. `type: talk`/`spec` are not added to
  the schema. No artifact under these paths was modified.

## Follow-on

- **Phase 5b** (visualiser template/schema alias): the `work-item-review`
  `work_item_id` transitional alias can now be removed (the corpus carries no
  residual review alias after migration).
- **Follow-on contract story**: the visualiser-server fallback arms
  (`read_ref_keys` `work-item:`, `cluster_key` legacy branch, `indexer.rs`
  filename fallback) remain retained-and-deprecated this release; their removal
  + the migration-completion grep gate ship in the follow-on once every
  consuming repo has migrated.

## References

- Plan: `meta/plans/2026-06-07-0070-meta-corpus-unified-schema-migration.md`
- Work item: `meta/work/0070-ship-meta-corpus-unified-schema-migration.md`
- Spike (wrong-rate procedure precedent):
  `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
- Decisions: ADR-0033, ADR-0034, ADR-0037, ADR-0038, ADR-0040, ADR-0042
