---
date: "2026-05-06T00:00:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-05-add-missing-templates-to-visualiser.md"
review_number: 1
verdict: COMMENT
lenses: [correctness, test-coverage, code-quality, standards, architecture, safety]
review_pass: 2
status: complete
---

## Plan Review: Add Design Gap and Design Inventory to Visualiser

**Verdict:** REVISE

The plan is a well-scoped, additive extension of an established pattern — wiring two missing templates and two missing doc types into an already-open registration architecture. The coverage of affected sites (enum, shell script, cluster logic, frontend type registry, fixtures, test count assertions) is thorough and the two-phase structure provides independently verifiable milestones. However, a critical self-contradiction in the plan body will mislead any implementer who reads the overview before the detail steps, a `templates.len()` assertion is left unstated after Phase 1 changes the fixture count, the proposed new clusters test fails to compile as written due to an erroneous `async fn`, and a pre-existing fixture gap is neither fixed nor acknowledged as out of scope.

### Cross-Cutting Themes

- **Completeness tracking contradiction** (flagged by: Correctness, Test Coverage, Code Quality, Safety) — The Phase 2 overview states the new types "are not tracked in the `Completeness` struct," yet the detailed steps add `has_design_gap` and `has_design_inventory` fields, tracking arms, and frontend `PipelineStepKey` entries. An implementer who reads the overview and stops there will implement the wrong behaviour, producing a UI where the new lifecycle pipeline dots never illuminate.
- **Missing count assertion updates** (flagged by: Correctness, Code Quality) — The plan updates `doc_paths.len()` in `config.rs` but leaves `templates.len()` unchanged after Phase 1 extends the fixture from 5 to 8 templates. The two assertions are adjacent lines in the same inline test.

### Tradeoff Analysis

- **`review_work` fixture gap vs. scope discipline**: `review_work` is absent from both JSON fixtures but present in the live script output and the contract test key list. The plan adds two new doc paths to the fixtures without acknowledging the pre-existing missing key. Fixing it is a two-line change per fixture that would bring the static fixture and the live script into alignment, but it broadens the PR scope. The plan should explicitly decide one way or the other.

### Findings

#### Critical

- 🔴 **Correctness + Test Coverage + Code Quality + Safety**: Contradiction between stated design intent and implementation for Completeness tracking
  **Location**: Phase 2 Overview "Key Discoveries" vs Phase 2 Step 4 / Step 11
  The overview says the new types "follow the `WorkItemReviews` pattern...not tracked in the `Completeness` struct", but Phase 2 Step 4 adds `has_design_inventory: bool` and `has_design_gap: bool` to the struct, populates tracking arms in `derive_completeness()`, and Step 11 adds corresponding `PipelineStepKey` entries and `LIFECYCLE_PIPELINE_STEPS` rows. The implementation steps are internally consistent and correct; the overview claim is wrong. An implementer following the overview would write a no-op match arm and skip the struct additions, causing the lifecycle pipeline dots to never illuminate for the new types.

#### Major

- 🟡 **Correctness + Code Quality**: Missing update to `c.templates.len()` assertion in `config.rs` inline test
  **Location**: Phase 1 Step 5 / Phase 2 Step 8 — `src/config.rs` line 230
  Phase 1 adds `work-item`, `design-gap`, and `design-inventory` to `config.valid.json`, raising the template count from 5 to 8. Phase 2 Step 8 updates the adjacent `doc_paths.len()` assertion on line 226 but says nothing about the `templates.len()` assertion on line 230. After Phase 1, `cargo test -p accelerator-visualiser` will fail on `assert_eq!(c.templates.len(), 5)`.

- 🟡 **Test Coverage**: Proposed new completeness test uses `async fn` in a synchronous test module
  **Location**: Phase 2 Step 4 — `src/clusters.rs` proposed test `design_gap_and_inventory_completeness_flags_are_set`
  Every existing test in `src/clusters.rs` is a plain synchronous `fn` with `#[test]`. The proposed test uses `async fn` without a `#[tokio::test]` attribute, and `compute_clusters` is not async. This will fail to compile.

- 🟡 **Code Quality**: `review_work` doc path absent from test fixtures without acknowledgement
  **Location**: Phase 2 Steps 6/7 — `config.valid.json`, `config.optional-override-null.json`
  The live script produces `review_work` as one of its 10 `doc_paths` keys, and the contract test checks for it explicitly. Both JSON fixtures currently have 9 entries (missing `review_work`). The plan adds `design_gaps` and `design_inventories` to bring the fixtures to 11, but `review_work` remains absent and undocumented. The fixture ends up with a different key set from what the live script produces, without explanation.

#### Minor

- 🔵 **Architecture**: `canonical_rank()` ordering contradicts stated lifecycle order
  **Location**: Phase 2 Step 4 — `src/clusters.rs` `canonical_rank()`
  The plan states "design inventory logically precedes gap analysis, so it appears first in the step order," and `LIFECYCLE_PIPELINE_STEPS` correctly lists `hasDesignInventory` before `hasDesignGap`. But `canonical_rank()` assigns `DesignGaps => 9, DesignInventories => 10`, placing gaps before inventories in cluster entry lists — the reverse of the stated ordering.

- 🔵 **Test Coverage**: `api_types` test update checks count only, not properties of new types
  **Location**: Phase 2 Step 10 — `tests/api_types.rs`
  The plan renames the test and updates the count from 11 to 13 but adds no assertions that `design-gaps` and `design-inventories` are present with correct values (`virtual: false`, `inLifecycle: true`, non-null `dirPath`). A wrong `in_lifecycle()` or `is_virtual()` implementation would pass the count test.

- 🔵 **Test Coverage**: No test for canonical rank ordering of new types
  **Location**: Phase 2 Step 4 — `src/clusters.rs` `canonical_rank()`
  No test asserts that `DesignGaps` and `DesignInventories` sort correctly within a shared cluster. A transposed rank assignment or a conflict with `Notes` (rank 8) would go undetected.

- 🔵 **Standards**: Both test renames must be applied consistently
  **Location**: Phase 1 Step 2 (api_types.rs) and Phase 2 Change 3 (docs.rs)
  Both `types_returns_eleven_entries_with_virtual_flag_on_templates` and `doc_type_key_all_returns_eleven_variants` need renaming to embed the new count. The plan covers both, but implementers should confirm both are applied — if only one is renamed, the codebase has inconsistent count-in-name convention.

- 🔵 **Standards**: `PipelineStepKey` union additions should order inventory before gap
  **Location**: Phase 2 Step 11 — `frontend/src/api/types.ts`
  The Rust `Completeness` struct and `LIFECYCLE_PIPELINE_STEPS` both place inventory before gap. The `PipelineStepKey` union additions should follow the same order (`'hasDesignInventory'` before `'hasDesignGap'`) to maintain alignment across the three related declarations in types.ts.

- 🔵 **Safety**: New Completeness fields lack a serialisation name contract test
  **Location**: Phase 2 Step 4 / Step 11
  No test verifies that the Rust `has_design_inventory` / `has_design_gap` fields are serialised as the camelCase names the TypeScript interface expects. A rename on one side would silently break lifecycle cluster views.

- 🔵 **Test Coverage**: Frontend `LIFECYCLE_PIPELINE_STEPS` additions have no automated test
  **Location**: Phase 2 Step 11 — `frontend/src/api/types.ts`
  If the frontend test suite doesn't already cover `LIFECYCLE_PIPELINE_STEPS` content, a typo in `docType` or a missing `longTail: true` flag would be invisible to CI.

- 🔵 **Architecture**: Contract test line reference off by one
  **Location**: Phase 1 Step 2 — `config_contract.rs`
  The plan references line 68 for the template count assertion; the actual assertion is on line 69. Low risk but could cause problems with automated patching.

- 🔵 **Standards**: Variable names `DG` and `DI` are ambiguous abbreviations
  **Location**: Phase 1 Step 1 — `write-visualiser-config.sh`
  Both start with `D`, making them harder to distinguish at a glance than the established pattern (`PRD`, `WI`, `RES`). Consider `DGP`/`DINV` or an inline comment.

- 🔵 **Correctness**: New completeness test lacks a single-cluster guard assertion
  **Location**: Phase 2 Step 4 — proposed `design_gap_and_inventory_completeness_flags_are_set`
  Accessing `clusters[0]` is safe when both entries share the same slug, but the test should assert `clusters.len() == 1` to catch any future divergence that would make the index silently wrong.

#### Suggestions

- 🔵 **Standards**: `all_returns_every_variant_exactly_once` and `doc_type_key_all_returns_eleven_variants` serve overlapping purposes — consider consolidating now that both need updating.
- 🔵 **Architecture**: `seeded_cfg` registers new doc paths without creating the directories on disk — acceptable for the current test suite but worth a note for any future indexer test that walks those paths.

### Strengths

- ✅ Correctly identifies that `in_lifecycle()`, `in_kanban()`, and `is_virtual()` use negation patterns that automatically cover new variants without per-variant overrides.
- ✅ Two-phase structure creates independently verifiable milestones — Phase 1 can be fully tested before Phase 2 begins.
- ✅ All hardcoded count assertions across inline tests, integration tests, and the contract test are identified and listed explicitly.
- ✅ The plan correctly notes the kebab-case round-trip test needs two new pairs, ensuring wire-format correctness for both new variants.
- ✅ Naming conventions (plural kebab-case keys, snake_case config keys, PascalCase enum variants, camelCase TypeScript fields) are applied consistently and match every existing precedent.
- ✅ `longTail: true` placement is correctly justified and consistent with the `hasNotes` precedent.
- ✅ The plan proactively fixes a pre-existing test infrastructure gap (`work-item` missing from template fixture seeding) alongside the new additions.
- ✅ Frontend sidebar and library views are correctly identified as data-driven — no template changes needed beyond `types.ts`.

### Recommended Changes

1. **Remove the contradictory overview claim about Completeness tracking** (addresses: Completeness contradiction — Critical)
   In Phase 2 Overview "Key Discoveries", replace "they are not tracked in the `Completeness` struct — they are specialized docs that don't fit the standard delivery pipeline" with language matching the actual implementation: both types are tracked in `Completeness` (enabling pipeline dot illumination) but use `longTail: true` to appear outside the main workflow dots.

2. **Add `templates.len()` assertion update** (addresses: Missing templates.len() update — Major)
   In Phase 1 Step 5 (or as a new item in Phase 2 Step 8), add: "Update `src/config.rs` line 230: change `assert_eq!(c.templates.len(), 5)` → `assert_eq!(c.templates.len(), 8)`."

3. **Fix `async fn` → `fn` in the proposed clusters test** (addresses: async fn compile error — Major)
   In Phase 2 Step 4, change `async fn design_gap_and_inventory_completeness_flags_are_set` to plain `fn`, matching all other tests in `src/clusters.rs`.

4. **Resolve the `review_work` fixture gap** (addresses: review_work not acknowledged — Major)
   Either add `"review_work": "/abs/path/to/project/meta/reviews/work"` to both JSON fixtures (updating the `config.rs` `doc_paths.len()` assertion to 12 instead of 11), or add an explicit note in Phase 2 Steps 6/7 acknowledging this as a pre-existing gap intentionally left out of scope.

5. **Swap DesignGaps and DesignInventories ranks** (addresses: rank ordering contradiction — Minor)
   In Phase 2 Step 4, change to `DesignInventories => 9, DesignGaps => 10` to match the stated lifecycle order and the `LIFECYCLE_PIPELINE_STEPS` ordering.

6. **Add property assertions to the `api_types.rs` test** (addresses: count-only assertion — Minor)
   In Phase 2 Step 10, add: after updating the count, add entry lookups for `'design-gaps'` and `'design-inventories'` asserting `virtual == false`, `inLifecycle == true`, non-null `dirPath`, mirroring the existing `decisions` assertion pattern.

7. **Add canonical rank ordering test** (addresses: no rank test — Minor)
   In Phase 2 Step 4, add a test that places `DesignInventories` and `DesignGaps` entries alongside a `Notes` entry in a shared cluster and asserts they sort after Notes with inventory before gap.

8. **Clarify `PipelineStepKey` union member order** (addresses: field ordering — Minor)
   In Phase 2 Step 11, note explicitly: add `'hasDesignInventory'` before `'hasDesignGap'` in the `PipelineStepKey` union to match the ordering in `Completeness` and `LIFECYCLE_PIPELINE_STEPS`.

---

## Per-Lens Results

### Correctness

**Summary**: The plan is largely well-specified and correctly identifies the exhaustive match sites that need updating across the Rust enum, cluster ranking, and config fixtures. Two correctness gaps stand out: an inline test assertion for template count in config.rs is omitted from the update instructions, and the plan's stated design intent for how the new types relate to the Completeness struct directly contradicts the implementation it then specifies.

**Strengths**:
- Correctly identifies that `in_lifecycle()`, `in_kanban()`, and `is_virtual()` require no per-variant overrides because the existing negation patterns automatically cover the new variants.
- Correctly identifies all exhaustive match sites in clusters.rs that will require new arms, preventing compile errors.
- The doc_paths count arithmetic is correct throughout: 9+2=11 for the static fixture inline test, 10+2=12 for the contract test.

**Findings**:

**[Major / High]** Missing update to `c.templates.len()` assertion in `config.rs` inline test
Location: Phase 2, Step 8: `src/config.rs` — update doc_paths count assertion
The `parses_valid_config` test at line 230 asserts `assert_eq!(c.templates.len(), 5)`. Phase 1, Step 5 adds three new template entries to `config.valid.json`, raising the count from 5 to 8. Phase 2, Step 8 correctly addresses the `doc_paths.len()` assertion on line 226 but says nothing about updating the `templates.len()` assertion on line 230. After Phase 1 is applied, this test will fail with `left: 8, right: 5`.

**[Major / High]** Contradiction between stated design intent and specified implementation for Completeness tracking
Location: Phase 2, Step 4 vs Overview Key Discoveries
The Overview states design gaps and inventories are "not tracked in the `Completeness` struct", but Phase 2 Step 4 explicitly adds `has_design_inventory: bool` and `has_design_gap: bool` to the struct, adds tracking arms in `derive_completeness()`, and Step 11 adds them to the TypeScript `Completeness` interface and `PipelineStepKey`. An implementer following the overview would skip the struct additions.

**[Minor / Medium]** New test uses single cluster assumption without guard assertion
Location: Phase 2, Step 4 — proposed test `design_gap_and_inventory_completeness_flags_are_set`
Accessing `&clusters[0]` is safe when both entries share slug `"foo"`, but adding `assert_eq!(clusters.len(), 1)` before the completeness assertion would prevent a future slug divergence from silently passing.

---

### Test Coverage

**Summary**: The plan is methodical about updating existing count assertions across all affected test files, and provisions a new positive test for completeness flag setting. However, there is a code error in the proposed test (`async fn` in a synchronous module) that will cause a compile failure, and the updated `api_types.rs` test only checks a count rather than asserting properties of the two new doc types. Canonical rank ordering of the new types is also untested.

**Strengths**:
- Identifies all hardcoded count assertions across the codebase and updates each one.
- Phase 2, Step 4 extends the existing completeness test with negative assertions, then adds a dedicated positive test — the correct two-part pattern.
- The contract test is executed against the live shell script, providing real end-to-end validation.

**Findings**:

**[Major / High]** Proposed completeness test uses `async fn` in a synchronous test module
Location: Phase 2 Step 4 — `src/clusters.rs`
`async fn design_gap_and_inventory_completeness_flags_are_set()` without `#[tokio::test]` will fail to compile. Every existing test in `clusters.rs` is a plain synchronous `fn`. Fix: remove `async`.

**[Major / High]** Internal contradiction on Completeness tracking
Location: Phase 2 Overview vs Phase 2 Step 4 / Step 11
Same contradiction identified by the Correctness lens. An implementer following the overview would implement the wrong behaviour.

**[Minor / High]** `api_types` test update only checks count, not properties of new types
Location: Phase 2, Step 10 — `tests/api_types.rs`
The count change from 11 to 13 does not verify that `design-gaps` and `design-inventories` are present with correct `virtual`, `inLifecycle`, and `dirPath` values.

**[Minor / High]** No test covering canonical rank ordering of new types
Location: Phase 2, Step 4 — `src/clusters.rs` `canonical_rank()`
A transposed rank assignment or conflict with `Notes` (rank 8) would not be caught by any automated test.

**[Minor / Medium]** Frontend `LIFECYCLE_PIPELINE_STEPS` additions have no automated test
Location: Phase 2, Step 11 — `frontend/src/api/types.ts`
If the frontend test suite doesn't cover `LIFECYCLE_PIPELINE_STEPS` content, a typo in `docType` or omitted `longTail: true` would be invisible to CI.

---

### Code Quality

**Summary**: The plan is well-structured with a strong understanding of existing codebase patterns. The incremental two-phase approach keeps changes coherent and testable. Key issues: a direct self-contradiction about Completeness tracking, a missing assertion update when fixtures change, and a pre-existing fixture gap (missing `review_work`) neither fixed nor noted.

**Strengths**:
- Phasing is well-chosen — partial implementation leaves the codebase in a valid, passing state.
- The plan identifies all affected count assertions and lists them explicitly.
- The `canonical_rank()` and `derive_completeness()` exhaustive match constraint is correctly noted, preserving exhaustiveness checking.

**Findings**:

**[Critical / High]** Self-contradictory description of Completeness tracking for new types
Location: Phase 2 Overview and Phase 2 Section 4
The overview states the new types follow the `WorkItemReviews` pattern and "are not tracked in the `Completeness` struct". But Section 4 explicitly adds `has_design_inventory` and `has_design_gap` fields to `Completeness`, initialises them, and adds tracking arms. One description is wrong; the detailed steps are correct.

**[Major / High]** `parses_valid_config` template count assertion not updated alongside doc_paths
Location: Phase 2 Section 8 — `src/config.rs` line 230
Same issue as identified by the Correctness lens. The `templates.len()` assertion on line 230 needs updating from 5 to 8 alongside the `doc_paths.len()` update on line 226.

**[Major / High]** `review_work` doc path missing from fixtures but not noted as out of scope
Location: Phase 2 Sections 6/7 — `config.valid.json`, `config.optional-override-null.json`
The live script produces `review_work` as a doc_path key. Both fixtures omit it (9 entries vs 10 in the script). After adding the two new paths, the fixtures will have 11 entries while still missing `review_work`. The plan does not acknowledge this gap.

**[Minor / High]** Rank values 9 and 10 leave no room for future insertion between Notes and new types
Location: Phase 2 Section 4 — `src/clusters.rs` `canonical_rank()`
Minor maintenance friction — a comment noting the sequence is intentional and that 11 is the next available value would help future developers.

**[Minor / Medium]** Negative completeness assertions placed in wrong test
Location: Phase 2 Section 4 — `clusters.rs` test placement
The `!c.has_design_gap` and `!c.has_design_inventory` assertions could go in the new dedicated test as negative cases rather than being appended to `completeness_flags_track_present_types`, keeping each test's scope clear.

---

### Standards

**Summary**: The plan is largely well-aligned with established project conventions. Naming patterns for new doc types consistently follow the plural-noun, kebab-case convention. The main issues are ensuring both test renames are applied consistently, maintaining `PipelineStepKey` ordering to match related declarations, and reconsidering the two-letter shell variable names.

**Strengths**:
- Plural kebab-case doc type keys, snake_case config path keys, and PascalCase enum variants all follow established conventions.
- camelCase TypeScript field names mirror the Rust serde camelCase output.
- SKILL.md directory line additions follow the exact format used for all existing entries.

**Findings**:

**[Minor / High]** Both test renames must be applied consistently
Location: Phase 1 Step 2 (api_types.rs) and Phase 2 Change 3 (docs.rs)
Both `types_returns_eleven_entries_with_virtual_flag_on_templates` and `doc_type_key_all_returns_eleven_variants` need renaming. The plan covers both, but implementers should verify both are done.

**[Minor / High]** `PipelineStepKey` union additions should order inventory before gap
Location: Phase 2 Step 11 — `frontend/src/api/types.ts`
The Rust `Completeness` struct and `LIFECYCLE_PIPELINE_STEPS` both place inventory before gap. `PipelineStepKey` should follow the same order.

**[Minor / Medium]** Variable names `DG` and `DI` are ambiguous
Location: Phase 1 Step 1 — `write-visualiser-config.sh`
Both start with `D`, making them harder to distinguish than the established pattern. Consider `DGP`/`DINV` or a comment.

**[Suggestion / Medium]** Overlapping test purpose between `all_returns_every_variant_exactly_once` and `doc_type_key_all_returns_eleven_variants`
Location: Phase 2 Change 3 — `src/docs.rs` inline tests
Both tests assert the same thing (variant count) with different naming styles. Consider consolidating while both need updating.

---

### Architecture

**Summary**: The plan is a well-scoped extension of an established pattern. The architecture is already open for this kind of extension. The main architectural observations are: the `canonical_rank()` ordering contradicts the stated lifecycle rationale, the `Completeness` struct continues growing as a concrete field list (a pre-existing concern the plan inherits), and line number references may have drifted.

**Strengths**:
- All registration sites are updated as a coordinated set rather than piecemeal changes.
- The plan exploits existing boolean-negation patterns correctly — new variants automatically inherit the right values.
- The `longTail: true` placement decision is explicitly justified and consistent with the `hasNotes` precedent.

**Findings**:

**[Minor / High]** `canonical_rank()` ordering contradicts stated lifecycle order
Location: Phase 2, Step 4 — `src/clusters.rs` `canonical_rank()`
The plan states inventory logically precedes gap analysis and `LIFECYCLE_PIPELINE_STEPS` lists inventory first. But `canonical_rank()` assigns `DesignGaps => 9, DesignInventories => 10` — placing gaps before inventories in cluster entry lists, the reverse of the stated ordering.

**[Minor / High]** `Completeness` struct grows as a concrete field list rather than a data-driven collection
Location: Phase 2, Step 4 — `src/clusters.rs`
Each future doc type that participates in lifecycle tracking requires four coordinated changes (Rust field, TypeScript field, `PipelineStepKey` member, `LIFECYCLE_PIPELINE_STEPS` entry) with no compile-time check that they stay in sync. This is a pre-existing constraint the plan inherits, not introduces. A `HashMap<DocTypeKey, bool>` approach would be more extensible long-term.

**[Minor / Medium]** Contract test line reference off by one
Location: Phase 1 Step 2 — `config_contract.rs`
Plan references line 68 for the template count assertion; actual line is 69.

**[Suggestion / Medium]** `seeded_cfg` doesn't create directories for new doc paths
Location: Phase 2, Step 5 — `tests/common/mod.rs`
Consistent with the existing convention (several registered paths aren't created on disk), but worth noting for any future indexer test against design-gap or design-inventory documents.

---

### Safety

**Summary**: The plan proposes purely additive changes. There are no destructive operations, no data migrations, and no removals of existing behaviour. The Rust compiler's exhaustive match requirement provides inherent compile-time safety for the most critical extension point. The one safety gap worth addressing is that new Completeness fields lack a serialisation name contract test.

**Strengths**:
- All changes are strictly additive — no existing data, config keys, or behaviour is removed.
- The Rust compiler enforces exhaustive matches on DocTypeKey, so any incomplete implementation causes a compile error.
- `deny_unknown_fields` on the config struct and HashMap-based doc_paths/templates fields mean new keys cannot corrupt existing config parsing.
- Manual verification steps cover the most user-visible failure modes.

**Findings**:

**[Minor / Medium]** New Completeness fields lack a serialisation name contract test
Location: Phase 2 Step 4 / Step 11
No test verifies that Rust `has_design_inventory` serialises as `hasDesignInventory` and that the TypeScript interface name matches. A rename on one side would silently break lifecycle cluster views.

**[Suggestion / Low]** Contradictory prose on Completeness tracking
Location: Phase 2 Overview
The overview's "not tracked in the Completeness struct" claim contradicts the implementation steps. Addressed by the Critical finding above.

---

## Re-Review (Pass 2) — 2026-05-06

**Verdict:** COMMENT

### Previously Identified Issues

All eight Critical/Major findings from pass 1 were addressed. Specifically:

- 🟢 **Correctness + Test Coverage + Code Quality + Safety**: Completeness tracking contradiction — Resolved. Key Discoveries now accurately describes the new types as having tracking completeness arms and long-tail pipeline steps, explicitly distinguishing them from `WorkItemReviews`.
- 🟢 **Correctness + Code Quality**: Missing `templates.len()` assertion update — Resolved. Phase 1 Step 7 explicitly updates line 230 from 5 → 8.
- 🟢 **Test Coverage**: `async fn` in synchronous test — Resolved. Both new cluster tests are plain `#[test] fn`.
- 🟢 **Code Quality**: `review_work` fixture gap unacknowledged — Resolved. Phase 2 Step 6 now has an explicit out-of-scope note explaining the divergence.
- 🟢 **Architecture**: `canonical_rank()` ordering contradicted lifecycle order — Resolved. Swapped to `DesignInventories => 9, DesignGaps => 10`.
- 🟢 **Test Coverage**: `api_types` test count-only — Resolved. Property assertions for `virtual`, `inLifecycle`, and `dirPath` added for both new types.
- 🟢 **Test Coverage**: No canonical rank ordering test — Resolved. New `design_inventory_sorts_before_design_gap_in_cluster` test added.
- 🟢 **Standards**: `PipelineStepKey` ordering unspecified — Resolved. Phase 2 Step 11 now explicitly specifies inventory before gap.

### New Issues Introduced

- 🟡 **Test Coverage + Correctness**: `derive_completeness()` wording implies dropping `WorkItemReviews` arm
  The instruction "change from the empty `WorkItemReviews` pattern to tracking arms" can be read as replacing `WorkItemReviews => {}` with the two new arms. Since Rust requires exhaustive matches, removing the `WorkItemReviews` arm would produce a compile error. The intent is to *add* alongside it. The code block is unambiguous but the prose contradicts it.

- 🟡 **Test Coverage**: Extending `Completeness` interface will break `LifecycleIndex.test.tsx`
  Phase 2 Step 11 adds `hasDesignInventory` and `hasDesignGap` as required fields on the TypeScript `Completeness` interface. The existing `LifecycleIndex.test.tsx` defines an `empty: Completeness` object literal that will be missing these two fields — TypeScript will reject this as a compile error, failing the entire frontend test suite before any test runs. The plan must add `hasDesignInventory: false, hasDesignGap: false` to that fixture as part of Step 11.

### Remaining from Pass 1 (Minor / Unaddressed)

- 🔵 **Standards**: `DG`/`DI` variable name ambiguity — unchanged
- 🔵 **Standards**: `doc_type_key_all_returns_eleven_variants` rename target not named — new finding, name should be specified as `doc_type_key_all_returns_thirteen_variants`
- 🔵 **Safety**: Completeness camelCase wire-name contract untested — unchanged
- 🔵 **Architecture**: Phase 2 Step 9 line reference (line 49 vs actual line 45/47) — still slightly off
- 🔵 **Test Coverage**: No frontend test for long-tail pipeline step rendering — unchanged

### Assessment

The plan is in good shape for implementation. The two new major findings are both straightforward fixes: one is a prose clarification (add "alongside" to the derive_completeness wording), and one is a concrete co-change to call out (`LifecycleIndex.test.tsx` fixture update). The minor/suggestion items are low-risk and can be addressed opportunistically during implementation or deferred. The plan is acceptable to proceed with the understanding that the implementer applies the two prose fixes before starting.
