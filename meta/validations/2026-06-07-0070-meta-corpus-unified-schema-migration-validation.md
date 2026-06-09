---
type: plan-validation
id: "2026-06-07-0070-meta-corpus-unified-schema-migration-validation"
title: "Validation Report: Ship meta/ Corpus Unified-Schema Migration"
date: "2026-06-09T12:20:05+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
parent: "plan:2026-06-07-0070-meta-corpus-unified-schema-migration"
target: "plan:2026-06-07-0070-meta-corpus-unified-schema-migration"
relates_to: ["plan-validation:2026-06-09-0070-meta-corpus-migration-dogfood"]
tags: [migration, frontmatter, schema, validation, visualiser, linkage]
last_updated: "2026-06-09T12:20:05+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Ship `meta/` Corpus Unified-Schema Migration

### Implementation Status

- ✓ **Phase 1: Corpus Frontmatter Validator** — Fully implemented
- ✓ **Phase 2: Body-Section Linkage Parser + Band Classifier + Fixtures** — Fully implemented
- ✓ **Phase 3: Migration `0007`** — Fully implemented (success-criteria checkboxes
  left unticked in the plan — see Deviations)
- ✓ **Phase 4: End-to-End Dogfood + Gap-Fix** — Fully implemented
- ✓ **Phase 5: Visualiser-Server Reader Expand + Deprecation + Template Alias** — Fully implemented

### Automated Verification Results

- ✓ `mise run test:integration:config` — 25 validator tests + linkage-parser suite, all pass
- ✓ `mise run test:integration:migrate` — 472 tests pass (`_EXPECTED_MIGRATE_SUITES = 4`)
- ✓ `skills/config/migrate/scripts/test-migrate-0007.sh` — 44 tests pass (run directly to confirm)
- ✓ `mise run test:unit:visualiser` — 412 tests pass, 0 failed
- ✓ `scripts/validate-corpus-frontmatter.sh meta/` — exit 0, 0 violations over 512 files
  (structural + whole-corpus referential integrity)
- ✓ Migration `0007` recorded in `.accelerator/state/migrations-applied` (head was `0006`)

### Code Review Findings

#### Matches Plan:

- **Phase 1.** `scripts/validate-corpus-frontmatter.sh` + `scripts/test-validate-corpus-frontmatter.sh`
  present and executable. Cross-cutting emission rules extracted to the shared
  `scripts/frontmatter-emission-rules.sh`; the single-source guard fixture proves
  *both* the validator and `test-template-frontmatter.sh` flip when the shared
  helper is tampered. Referential integrity flags dangling refs and tolerates `pr:`
  literals.
- **Phase 1 wiring.** `_EXPECTED_CONFIG_SUITES = 15` guard added (mirroring the
  migrate pattern); `_EXPECTED_MIGRATE_SUITES` bumped `3 → 4`.
- **Phase 2.** `scripts/linkage-parser.sh` + `scripts/test-linkage-parser.sh` present;
  parser suite green under `test:integration:config`.
- **Phase 3.** `0007-unify-meta-corpus-frontmatter.sh` present, `# INTERACTIVE: yes`,
  applied to the ledger. Shared `frontmatter-frag.awk` fragment and
  `status-legacy-map.tsv` data file present. Suite proves path→typed-per-doc-type,
  deterministic bare-number conversion, parent-derivation, idempotency, and the
  interactive accept/edit/skip apply path.
- **Phase 4.** Dogfood report at
  `meta/validations/2026-06-09-0070-meta-corpus-migration-dogfood.md`: result `pass`,
  0 violations, resolved-band wrong-rate **1.4% (≤5%)**, ledger-bypassed re-run is a
  byte-for-byte no-op, session log 1503 accepted / 77 edited / 43 skipped. Pre-migration
  gap-fixes (work-item `0032` `work_item_id:"0031"` collision; design-gap `0086` missing
  `current_inventory`) recorded.
- **Phase 5.** `indexer.rs` gains the unified `id:` read path (`:1280`) routed through
  normalisation; per-arm `tracing::warn!` deprecation lines in all three retained
  fallbacks with dedicated tests (`indexer.rs:3150/3169`, `frontmatter.rs:514`,
  `cluster_key.rs:282`). `work-item-review` template alias removed and the
  `work_item_id` extra dropped from its `templates-schema.tsv` row. Follow-on contract
  work item raised: `meta/work/0102-remove-visualiser-legacy-linkage-fallback-arms.md`.
- **ADR-0042** (`Reconciling Pre-Schema Status Values`) present and accepted — the
  status-vocab prerequisite.

#### Deviations from Plan:

- **Phase 3 success-criteria checkboxes are left unticked (`[ ]`) in the plan** even
  though the work is complete and the backing tests pass. This is a plan-bookkeeping
  gap, not an implementation gap — the 44-test `0007` suite and the dogfood confirm
  the criteria are met.
- **Dogfood report filename** is `2026-06-09-0070-meta-corpus-migration-dogfood.md`
  (the run date) rather than the skill's plan-stem-derived
  `…-validation.md` convention. Harmless; it carries correct typed-linkage frontmatter
  (`target: "plan:2026-06-07-0070-…"`). This report fills the stem-derived slot.
- **Test counts run slightly higher than the plan's estimates** (412 visualiser vs
  "411 default", 512 corpus files vs "487/504") — expected drift as the corpus grew;
  the plan flagged its counts as approximate and to reconcile at implementation time.
- **Migration `0005` was re-run** as part of the dogfood to reconcile 9 newer
  work-items carrying legacy `type:` with no `kind:` — a recorded reconciliation choice,
  not a plan deviation.

#### Potential Issues:

- None blocking. The retained legacy fallback arms and their pinning tests remain by
  design; their removal is correctly deferred to work item `0102`.

### Manual Testing Required:

1. Visualiser (Phase 5 manual criteria, still unticked in the plan):
   - [ ] Load this repo's migrated corpus in the visualiser; confirm work-item reviews
     cluster via the `target:` typed ref and migrated work-items resolve via the `id:` path.
   - [ ] Load a simulated un-migrated corpus; confirm it degrades gracefully with
     deprecation warnings, not broken edges.

### Recommendations:

- Tick the Phase 3 (and remaining Phase 5 manual) success-criteria checkboxes so the
  plan's recorded state matches its verified state.
- Run the Phase 5 manual visualiser smoke-load before relying on the migrated corpus in
  the visualiser epic, then close out the manual checkboxes.
- Proceed with follow-on work item `0102` (contract: remove the three legacy fallback
  arms + pinning tests) once consuming repos have migrated.
