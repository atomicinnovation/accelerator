---
date: "2026-05-17T17:00:00+01:00"
type: plan-review
producer: review-plan
target: "plan:2026-05-16-0041-library-page-wrapper-and-overview-hub"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability, compatibility]
review_pass: 4
status: complete
id: "2026-05-16-0041-library-page-wrapper-and-overview-hub-review-1"
title: "2026-05-16-0041-library-page-wrapper-and-overview-hub-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-17T17:00:00+01:00"
last_updated_by: Toby Clemson
---

## Plan Review: 0041 — Library Page Wrapper, Overview Hub, and List Views

**Verdict:** REVISE

The plan is meticulously sequenced, test-first, and shows strong fluency with existing codebase conventions (TDD layout, migration.test.ts allowlists, kebab-case serde, hand-rolled small primitives). Across seven lenses there is one critical correctness contradiction (facet-count scoping cannot be satisfied by a single static server endpoint), eight cross-cutting concerns that multiple lenses raised independently, and a number of smaller convention or correctness gaps. Most are addressable with targeted plan edits rather than a structural rewrite.

### Cross-Cutting Themes

- **Facet-count scoping is internally inconsistent** (flagged by: correctness 🔴, code-quality, test-coverage) — The FilterPill contract requires interactive "post-other-facet, pre-own-facet" counts, but `/api/library/structure` is a static one-shot endpoint that takes no selection parameter. The behaviour as written is unimplementable; either the contract must change (counts are static-totals) or the computation must move client-side (server emits the option universe; the FilterPill recomputes counts against locally-filtered entries).
- **Sidebar wiring decision is left to the implementer** (flagged by: architecture, code-quality, standards, correctness) — The plan offers two architecturally distinct options (internal `useQuery` vs prop-driven from `RootLayout`) and only "recommends" prop-driven. The existing convention is unambiguous (Sidebar already takes `docTypes` as a prop from `RootLayout`); the plan should commit. Related: where does the `templates` doc-type sit in the new phase-grouped response? Sidebar currently fetches it via `byKey.get('templates')` from `/api/types`.
- **`Page.maxWidth` raw-string prop bypasses the token convention** (flagged by: code-quality, standards, usability 🟡) — The token system the plan introduces (`--ac-content-max-width`) is defeated as soon as a caller passes `'600px'` for `LibraryTemplatesIndex`. The literal escapes `migration.test.ts` (which only scans CSS), reintroducing exactly the smell this work removes.
- **Popover keyboard / ARIA contract is incomplete** (flagged by: test-coverage, standards, usability) — Three independent gaps: (1) `role="menuitem"` is specified but trigger-side WAI-ARIA (`aria-haspopup`, `aria-expanded`, `aria-controls`) is missing; (2) Tab, Space, and typeahead behaviour are not in the test contract; (3) focus-management and positioning tests rely on jsdom's `getBoundingClientRect` and `document.activeElement`, both unreliable for these specific assertions.
- **Residual-`prs` grep verification is broken** (flagged by: code-quality, correctness, test-coverage) — `\b` inside a character class is the backspace literal, not a word boundary; `--type tsx` is not a built-in ripgrep type; the check will silently miss residuals. The rename's safety net has a hole.
- **`empty_description` is a reserved-but-unused wire field** (flagged by: architecture, standards) — Shipped on the server response but ignored by the frontend in this story. Violates YAGNI for wire contracts and has no precedent in the codebase.
- **Phase grouping "server-driven" is really "centralised in one server file"** (flagged by: architecture 🟡) — The DEFINE/DISCOVER/BUILD/SHIP/REMEMBER mapping moves from `frontend/src/api/types.ts` to `server/src/api/library.rs`, but stays hardcoded. The architectural gain is modest; either acknowledge this in the plan wording or push the configuration into `config.toml`.
- **Visual-regression rename has two coupled steps** (flagged by: test-coverage, compatibility) — `jj mv` preserves jj history, but Playwright matches snapshots by computed path derived from the test id. The plan must also update `glyph-showcase.spec.ts` (or whatever drives the snapshot id) to emit `pr-descriptions-...` paths. "Byte-identical" should be verified automatically (sha256), not by comment.

### Tradeoff Analysis

- **API cohesion vs consumer cost**: `/api/library/structure` bundles phase tree, per-type metadata, and facet options into one response that three different consumers (overview hub, Sidebar, single FilterPill) read. Splitting (e.g. `/api/library/structure` + `/api/library/types/:type/facets`) reduces coupling but adds endpoints. Acceptable at v1 scale if explicitly noted.
- **Standards consistency vs explicit wire pinning**: Adding `#[serde(rename = "pr-descriptions")]` for clarity breaks uniformity with every other variant that relies on `rename_all = "kebab-case"`. Standards says drop the redundant annotation; correctness/compatibility suggest pinning *every* variant's wire token in a serialisation contract test instead.

### Findings

#### Critical

- 🔴 **Correctness**: Server-emitted facet counts cannot satisfy "post-other-facet, pre-own-facet" scoping
  **Location**: Phase 2 (Indexer aggregator + handler) and Phase 5 §3 (FilterPill)
  `/api/library/structure` is hit once on page load with no filter parameters; `library_aggregates` produces static counts from raw entries. The FilterPill test then asserts that toggling facet B changes facet A's option counts — unimplementable as specified. The behaviour described in Phase 5 Manual Verification ("Toggle a status filter on decisions; confirm rows filter, option counts in other facets update") is the contract that breaks.

#### Major

- 🟡 **Architecture**: Phase-to-doc-type mapping moves from client to server but stays hardcoded
  **Location**: Phase 2 §2 (`build_structure`) and Phase 4 §4 (Retire `PHASE_DOC_TYPES`)
  The mapping just moved file, not concern. Either acknowledge "server-driven" means "centralised server-side" or push to typed config.

- 🟡 **Architecture**: Single endpoint conflates three orthogonal concerns
  **Location**: Phase 2 §2 (`LibraryStructureResponse` shape)
  Phase tree + per-type metadata + per-doc-type facet options are read by three different consumers; Sidebar pays for facet computation it never uses.

- 🟡 **Architecture**: Page owns title styling, creating a hidden cross-cutting contract
  **Location**: Phase 1 §2 + Phase 6 §8 (migration.test.ts REQUIRED list)
  The "every route renders an h1 with `--ac-fg-strong`" rule is now split across Page's internal CSS, the REQUIRED list, and the routes. A new non-Page route silently loses the contract.

- 🟡 **Code Quality**: `LibraryAggregates` is a data clump of five parallel maps
  **Location**: Phase 2 §1
  Five `HashMap<DocTypeKey, …>` fields with shared keyspace and implicit invariants. Prefer `HashMap<DocTypeKey, PerTypeAggregate>` colocating count/latest/facets per type.

- 🟡 **Code Quality**: `build_structure` conflates phase grouping, facet emission rules, and the threshold
  **Location**: Phase 2 §2
  Decompose into named pieces: `PHASES: &[(PhaseId, &[DocTypeKey])]` const, `facets_for(doc_type)`, `facet_kind(option_count)`.

- 🟡 **Test Coverage**: Sort tie-breaker test does not cover all three tiers / all five options
  **Location**: Phase 5 §6 (LibraryTypeView test)
  Comparator has three branches (workItemIds both present / one null / both equal → relPath) × five sort options. Single test case is insufficient — non-deterministic ordering will pass and flake.

- 🟡 **Test Coverage**: Focus-management and positioning tests rely on unreliable jsdom behaviour
  **Location**: Phase 1 §4 (Popover tests)
  `getBoundingClientRect` returns zeros in jsdom; `document.activeElement` is fragile under `userEvent`. Either pin a test strategy (user-event + explicit tabIndex + stubbed BCR) or move to Playwright for positioning.

- 🟡 **Test Coverage**: Five-route Page migration tests don't enumerate all four branches per route
  **Location**: Phase 6 §1–5
  Only LifecycleIndex spells out all branches. After `RootLayout.main` padding strip, an un-migrated loading/error branch renders bare and unpadded.

- 🟡 **Test Coverage**: "Byte-identical visual baselines" is a comment, not a check
  **Location**: Phase 3 §3
  Either verify via sha256 pre/post rename, or accept regeneration via `--update-snapshots` and visually diff. Also: linux baselines need CI regeneration.

- 🟡 **Correctness**: Latest-preview tie-break on equal mtime_ms is non-deterministic
  **Location**: Phase 2 §1 (latest update)
  `HashMap` iteration order is unspecified; strict `>` means first-iterated wins on ties. Apply the same `workItemId → relPath` tie-break as the list view.

- 🟡 **Correctness**: `LibraryAggregates::default()` is called but the struct has no `#[derive(Default)]`
  **Location**: Phase 2 §1
  Code as sketched won't compile. Also `LatestPreview::from(entry)` is called but not defined.

- 🟡 **Correctness**: Residual-`prs` verification regex is incorrect on multiple axes
  **Location**: Phase 3 Success Criteria
  `\b` inside `[]` is backspace; `--type tsx` isn't a built-in ripgrep type. Use `rg -w 'prs' -g '*.{ts,tsx,rs,css}'`.

- 🟡 **Correctness**: "ID / DATE" column has under-specified mixed semantics
  **Location**: Phase 5 §6
  Behaviour when both workItemId and frontmatter.date are present, or neither, is unspecified; tests cover only the ID-pill branch.

- 🟡 **Correctness**: `DocTypeKey` rename interacts with on-disk `meta/prs/` path display in EmptyState
  **Location**: Phase 3 §2 + Phase 5 §4
  EmptyState renders `meta/{type}/` — if `{type}` is the wire token (`pr-descriptions`), users see `meta/pr-descriptions/` while the actual directory is `meta/prs/`. Derive from `DocType.dirPath` instead.

- 🟡 **Standards**: LAYOUT_TOKENS parity test only checks `:root`, not three blocks
  **Location**: Phase 1 §1
  Plan instruction to add `--ac-content-max-width` to all three theme blocks is incorrect — existing `--ac-topbar-h` is declared only in `:root`.

- 🟡 **Standards**: Hand-rolled Popover trigger missing WAI-ARIA menu-button attributes
  **Location**: Phase 1 §4 + Phase 5 §2/3
  `aria-haspopup`, `aria-expanded`, `aria-controls` on the trigger pill are absent from the contract. First popover in the codebase — sets a precedent.

- 🟡 **Usability**: `Page.maxWidth` raw-string prop encourages bypassing the token
  **Location**: Phase 1 §2 + Phase 6 §5
  Literal `'600px'` in JSX escapes `migration.test.ts`. Narrow the prop (named variants) or introduce `--ac-content-max-width-narrow`.

- 🟡 **Usability**: Popover keyboard contract omits Tab, Space, typeahead
  **Location**: Phase 1 §4
  WAI-ARIA menu pattern expects Space to activate (like Enter) and a Tab policy.

- 🟡 **Usability**: FilterPill multi-select interaction model is unspecified
  **Location**: Phase 5 §3
  `role="menuitem"` implies single-select; FilterPill needs `role="menuitemcheckbox"` with `aria-checked` and Enter/Space-toggle-without-close.

- 🟡 **Usability**: Breaking redirect removal lacks user-facing communication
  **Location**: Phase 4 §2
  Migration Notes underplay external-bookmark impact (anyone landing on `/library` expecting decisions now sees the hub).

- 🟡 **Compatibility**: Persisted `seen-doc-types` localStorage entry keyed on `"prs"` is silently dropped
  **Location**: Phase 3 (frontend rename)
  `parseStored` filters via `isDocTypeKey` — after the rename, existing users' `"prs"` epoch is lost, and the PR-descriptions card resurfaces as unseen. A ~3-line in-place key migration removes the regression.

#### Minor

- 🔵 **Architecture**: Sidebar data-source ambiguity left to implementation
  **Location**: Phase 4 §3 — Commit to prop-driven, matching the existing `docTypes` flow.

- 🔵 **Architecture**: `empty_description` reserved-but-ignored wire field
  **Location**: Phase 2 §2 + Phase 5 §4 — Drop until a consumer exists.

- 🔵 **Architecture**: Deliberate `prs`/`pr-descriptions` asymmetry needs an inline anchor
  **Location**: Phase 3 — Add a comment at `docs.rs:52` (`PrDescriptions => Some("prs")`) citing this work item.

- 🔵 **Architecture**: Caching deferred without trigger condition
  **Location**: Performance Considerations — Add a `// PERF:` comment with the threshold for migrating to the cached pattern.

- 🔵 **Architecture**: Hand-rolled Popover focus-return assumes single active popover
  **Location**: Phase 1 §4 — Add test: opening a second Popover dismisses the first.

- 🔵 **Code Quality**: Wire response includes duplicate/redundant fields (`glyphId`, `route`)
  **Location**: Phase 2 §2 — Drop unless divergence case is in scope.

- 🔵 **Code Quality**: `FacetKind` (EnumWithSearch threshold) is presentation leaking server-side
  **Location**: Phase 2 §2 — Frontend computes `options.length > 8` locally.

- 🔵 **Code Quality**: Popover bundles four concerns
  **Location**: Phase 1 §4 — Consider extracting `useMenuKeyboard` and `useReturnFocusOnClose` alongside `useDismiss`.

- 🔵 **Code Quality**: KanbanBoard migration risks per-branch Page duplication
  **Location**: Phase 6 §1 — Lift Page above the branch conditional.

- 🔵 **Code Quality**: `formatDocId` silently widens IDs that exceed 4 digits
  **Location**: Phase 5 §1 — Add a 5+ digit test case, decide explicitly.

- 🔵 **Test Coverage**: mtime sentinel test missing "multiple entries with mtime=0" case
  **Location**: Phase 2 §1 — Tie-break non-determinism not locked down.

- 🔵 **Test Coverage**: EmptyState description-table completeness not asserted
  **Location**: Phase 5 §4 — Data-driven test iterating `DOC_TYPE_KEYS`.

- 🔵 **Test Coverage**: Filter-applied-empty branch transitions not tested
  **Location**: Phase 5 §6 — Test `Clear filters` round-trip and unreachability of filter pill from doc-type-empty.

- 🔵 **Test Coverage**: Theme verification via `data-theme` is decorative in jsdom
  **Location**: Phase 4 §1 + Phase 1 §2 — Switch to `*.module.css?raw` assertions for token-binding, or rely on visual regression.

- 🔵 **Test Coverage**: Facet-kind 8-option threshold not boundary-tested
  **Location**: Phase 2 §2 — Add 7 / 8 / 9 cases.

- 🔵 **Correctness**: Project-prefix fallback may emit empty-string project
  **Location**: Phase 2 §1 — Guard with `.filter(|(p, _)| !p.is_empty())`.

- 🔵 **Correctness**: `formatDocId` regex is ASCII-only
  **Location**: Phase 5 §1 — Broaden to `/^([^-]+)-(\d+)$/` or document.

- 🔵 **Correctness**: LifecycleIndex toolbar may overflow Page `actions` slot on narrow viewports
  **Location**: Phase 6 §2 — Consider rendering toolbar in the content slot below the divider.

- 🔵 **Correctness**: Popover focus-return may not hold under React 18 StrictMode
  **Location**: Phase 1 §4 — Guard `triggerRef.current` and gate effect on `open === false` transition.

- 🔵 **Standards**: Query key shape unspecified
  **Location**: Phase 2 §3 — Existing convention: `libraryStructure: () => ['library-structure'] as const`.

- 🔵 **Standards**: Facet ids use snake_case (`cluster_slug`) on a camelCase wire
  **Location**: Phase 2 §2 — Pick a casing convention for facet ids.

- 🔵 **Standards**: Explicit `#[serde(rename = "pr-descriptions")]` redundant with `rename_all = "kebab-case"`
  **Location**: Phase 3 §1 — Drop annotation; consider a per-variant serialisation contract test instead.

- 🔵 **Standards**: `mod library;` should be private (not `pub(crate)`)
  **Location**: Phase 2 §2 — Match `lifecycle`, `templates`, `types`.

- 🔵 **Usability**: Eyebrow prop typed as `ReactNode` obscures the Glyph+text idiom
  **Location**: Phase 1 §2 — Consider an `<Eyebrow>` sub-component for the canonical pattern.

- 🔵 **Usability**: Page title is required; loading-state convention undefined
  **Location**: Phase 1 §2 + Phase 6 §3 — Document the loading placeholder convention.

- 🔵 **Usability**: OR/AND filter semantics have no in-UI cue
  **Location**: Phase 5 §3 — Helper text or visual operator.

- 🔵 **Usability**: Search-input keyboard handoff inside enum-with-search undefined
  **Location**: Phase 5 §3 — Specify arrow-down handoff into the option list, Escape precedence.

- 🔵 **Usability**: Zero-count hub cards are still clickable
  **Location**: Phase 4 §1 — Decide explicitly (dim+disable, or keep+CTA).

- 🔵 **Compatibility**: `/library` redirect removal — bookmarks/links impact under-stated
  **Location**: Migration Notes — Broaden the bullet to cover bookmarks/docs/screenshots, not just scripts.

- 🔵 **Compatibility**: Stale-bundle reconnect to upgraded server gives transient inconsistency
  **Location**: Phase 3 — One-line Migration Notes addition ("reload tabs predating the upgrade").

- 🔵 **Compatibility**: Wire token contract not pinned by test
  **Location**: Phase 3 §1 — Per-variant serialisation assertion in `docs.rs:150` test block.

- 🔵 **Compatibility**: Visual-regression spec file rename step not explicit
  **Location**: Phase 3 §3 — Add a bullet for the `glyph-showcase.spec.ts` edit (snapshot id changes from `prs` to `pr-descriptions`).

#### Suggestions

- 🔵 **Usability**: Show active sort label + direction indicator on the closed SortPill
  **Location**: Phase 5 §6

- 🔵 **Usability**: NoResultsPanel — surface active filter chips alongside `Clear filters`
  **Location**: Phase 5 §5

- 🔵 **Usability**: Consider `<PageQuery>` helper once 5+ routes repeat the loading/error/success Page wrap
  **Location**: Phase 6 — Follow-up, not blocking.

### Strengths

- ✅ Test-first discipline applied per primitive (Page, useDismiss, Popover, SortPill, FilterPill, EmptyState, NoResultsPanel) and per aggregator method; every new component has a named test file with enumerated cases.
- ✅ Six-phase delivery sequence with a clean linear dependency graph (primitives → server endpoint → rename → consumers).
- ✅ Component file organisation, CSS-module convention, kebab-case serde rename, and `migration.test.ts` allowlist usage all match existing codebase conventions exactly.
- ✅ Server-driven phase/facet structure removes the hard-coded `PHASE_DOC_TYPES` client coupling and aligns with ADR 0024.
- ✅ Single-pass `library_aggregates` under one `entries.read().await` lock is cohesive and matches `clusters.rs` precedent.
- ✅ Hand-rolled Popover + `useDismiss` matches house style (no new external dependency).
- ✅ Explicit `What We're NOT Doing` section documents tradeoffs (URL state, caching, on-disk directory, frontmatter conventions).
- ✅ Page wrapper migration sequenced atomically with `RootLayout.main` padding removal — no partial-state visual regressions possible.
- ✅ Deliberate `prs` ↔ `pr-descriptions` wire/disk/config asymmetry is documented and reflected in the test-fixture leave-list.
- ✅ Sort tie-breaker, mtime sentinel handling, and project-prefix derivation are specified explicitly rather than left to interpretation.
- ✅ Empty-state distinction (doc-type-empty vs filter-applied-empty) modelled as two separate components with separate test files — clean cohesion.
- ✅ Phase 6 atomicity called out explicitly with documented rollback strategy.
- ✅ KanbanBoard double-padding hazard pre-emptively addressed (`padding` → `padding-block`).
- ✅ PR-descriptions rename surface comprehensively enumerated across both languages (Rust enum + serde, ClusterFlags field, frontend types, CSS tokens, icon file, 12 visual baselines, fixtures).

### Recommended Changes

In rough order of impact:

1. **Resolve the FilterPill facet-count contract** (addresses: critical correctness finding; major code-quality and test-coverage findings)
   Decide explicitly:
   - **Option A (simpler)**: Counts are static totals (computed once at page load). FilterPill displays "{label} ({count})" but counts do not update as the user toggles options. Update Phase 5 §3 test cases and Manual Verification to drop the "option counts in other facets update" claim. Also document in Phase 2 §2.
   - **Option B (matches the stated UX)**: Server emits the option *universe* (ids + labels) only; FilterPill recomputes counts client-side by filtering the entries array against all-but-this-facet selections. Drop `count: usize` from `FacetOption` (or rename to `total: usize`); add a derived-counts helper in `FilterPill.tsx` with its own test file.
   Whichever is chosen, the indexer aggregator, response shape, FilterPillProps, and test all need to agree.

2. **Fix the load-bearing Rust code in Phase 2 §1** (addresses: 2 major correctness findings)
   - Add `#[derive(Default)]` to `LibraryAggregates` (and `LatestPreview` if used).
   - Define `LatestPreview::from(&IndexEntry) -> LatestPreview` explicitly (the plan calls it but doesn't show the impl).
   - Add a deterministic tie-break on equal `mtime_ms` in `latest` (e.g. lexicographically smaller `relPath` wins) and a matching test case.

3. **Replace the broken residual-grep with a working verification** (addresses: 1 major + minor findings)
   Replace Phase 3 success criteria with `rg -w 'prs' skills/visualisation/visualise/{frontend/src,server/src} -g '*.{ts,tsx,rs,css}'` and document the expected remaining matches (`config_path_key()` arm, `meta/prs/` literal paths, fixture config keys, helper scripts).

4. **Commit to one Sidebar wiring approach and resolve `templates`'s phase placement** (addresses: 1 architecture + 1 code-quality + 1 standards + 1 correctness)
   Specify prop-driven: `RootLayout` fetches `libraryStructure` and passes `phases` to `Sidebar`, mirroring the existing `docTypes` flow. Also specify where `templates` sits in the response — either as a top-level field outside `phases`, or in a dedicated phase, and update Sidebar/test changes accordingly.

5. **Narrow `Page.maxWidth` or move the 600px override back to CSS** (addresses: 1 major + 2 minor)
   Either introduce `--ac-content-max-width-narrow: 600px` and type the prop as `maxWidth?: 'default' | 'narrow'`, OR keep the literal in `LibraryTemplatesIndex.module.css` and keep its `migration.test.ts:126` allowlist entry. Don't move the literal into JSX where it escapes the migration test.

6. **Complete the Popover keyboard / ARIA contract** (addresses: 1 standards major + 1 usability major + 1 test-coverage major)
   - Add `aria-haspopup="menu"`, `aria-expanded`, `aria-controls` to the trigger via a `triggerProps` API; assign a stable `id` to the panel.
   - Specify Tab (closes popover, default tab order proceeds), Space (activates like Enter); document typeahead as out-of-scope (the search input handles it).
   - For FilterPill specifically: `role="menuitemcheckbox"` with `aria-checked`; Enter/Space toggles without closing.
   - Re-spec the focus/positioning test strategy: `@testing-library/user-event`, stubbed `Element.prototype.getBoundingClientRect`, explicit `tabIndex` on menuitems. Note that style-resolution assertions for CSS custom properties should use `*.module.css?raw` source-assertion (matches existing pattern), not jsdom `getComputedStyle`.

7. **Drop the unused `empty_description` wire field** (addresses: 1 architecture + 1 standards)
   Reintroduce additively when a frontend consumer exists.

8. **Strengthen the test-coverage gaps in five-route migration and tie-breakers** (addresses: 2 major test-coverage)
   - For each of the five Phase 6 routes, explicitly enumerate which branches (loading / error / empty / success) the test must assert wrap in `<Page>`.
   - For LibraryTypeView sort, spell out tie-breaker test cases per sort option and per branch of the recently-modified comparator.

9. **Fix Phase 1 §1 — declare `--ac-content-max-width` once in `:root`** (addresses: 1 standards major)
   The plan's claim that the parity test enforces three-block declaration is incorrect. Match the existing `--ac-topbar-h` pattern.

10. **Add the localStorage migration for `seen-doc-types`** (addresses: 1 compatibility major)
    Three lines inside `parseStored()` in `frontend/src/api/use-unseen-doc-types.ts` rewriting `"prs"` → `"pr-descriptions"` before the `isDocTypeKey` filter. Removes the silent unseen-marker regression.

11. **Verify visual-regression rename byte-identity and update the spec file** (addresses: 1 test-coverage major + 1 compatibility minor)
    Add a `shasum -a 256` step to Phase 3 §3 to confirm the renamed PNGs are byte-identical pre/post. Add an explicit bullet for the `glyph-showcase.spec.ts` edit that drives the snapshot id from `prs` to `pr-descriptions`.

12. **Address the "ID / DATE" mixed-column semantics** (addresses: 1 major correctness)
    Decide: ID pill when `workItemId` present, else `frontmatter.date` formatted, else em-dash placeholder. Confirm `EmptyState`'s `meta/{type}/` heading reads from `dirPath` (preserving on-disk `meta/prs/`), not the wire token.

13. **Drop the redundant explicit `serde(rename)` annotation; pin all wire tokens via a contract test** (addresses: 1 standards minor + 1 compatibility minor)
    Add an inline test in `docs.rs:150` block iterating every `DocTypeKey` variant and asserting its `serde_json::to_value` token.

14. **Smaller convention tightenings** (addresses: clutch of minor findings)
    - Specify query-key shape: `libraryStructure: () => ['library-structure'] as const`.
    - Specify `mod library;` (private) in `api/mod.rs`.
    - Pick a facet-id casing convention (camelCase preferred).
    - Decompose `build_structure` into `PHASES` const + `facets_for` + `facet_kind`.
    - Lift Page wrapper above KanbanBoard's branch conditional.
    - Drop unused `glyphId` and `route` wire fields (or justify divergence cases).
    - Restructure `LibraryAggregates` as `HashMap<DocTypeKey, PerTypeAggregate>`.
    - Reconsider rendering LifecycleIndex's toolbar in the content slot rather than the `actions` slot.
    - Broaden `formatDocId`'s regex or add explicit ASCII-only documentation; add a 5+ digit test case.
    - Guard project-prefix fallback against empty prefix.

15. **Document the decisions you make** (suggestion-level cleanup)
    - Migration Notes: broaden the redirect-removal bullet to mention bookmarks/docs; add "reload tabs predating upgrade".
    - Inline comment at `config_path_key()`'s `PrDescriptions => Some("prs")` arm.
    - `// PERF:` comment on `library_aggregates` with the threshold for migrating to caching.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan demonstrates strong architectural thinking — phase-ordered dependencies, server-driven structure, hand-rolled primitives consistent with codebase house style, and acknowledged tradeoffs. The dominant architectural concerns are (1) the server hard-coding lifecycle-phase grouping inside `build_structure` re-creates a different form of the same client-side coupling `PHASE_DOC_TYPES` had — it just moves where the doc-type ↔ phase mapping lives without abstracting it; (2) the `Page` component conflates wrapper layout with title styling in a way that makes the migration test `REQUIRED` list a single point of friction; and (3) the `/api/library/structure` endpoint mixes three orthogonal concerns (phase tree, doc-type metadata, filter facet options) into one response, blurring boundaries and creating recompute cost for every facet change.

**Findings**: 3 major, 5 minor (see Cross-Cutting Themes and the curated Findings list above).

### Code Quality

**Summary**: The plan is well-structured around clear primitives (Page, Popover, useDismiss) and uses TDD throughout, which sets it up for maintainability. However, several design points raise concerns: the FilterPill's documented "post-other-facet, pre-own-facet" count scoping conflicts with the server's selection-unaware aggregator (a contract gap), `LibraryAggregates` is a data clump of five parallel maps, and Phase 2's `build_structure` hides significant phase/facet/threshold logic behind ellipses. The Popover primitive carries multiple responsibilities (positioning, focus management, key navigation, dismiss) that may benefit from further decomposition.

**Findings**: 3 major, 8 minor.

### Test Coverage

**Summary**: The plan is genuinely test-first and lays out coherent contracts for the new primitives, the server aggregator, and the refactored list view. However, several TDD test contracts have meaningful gaps around tie-breaker determinism, facet count scoping, mtime sentinel handling, focus-management on a non-jsdom-friendly Popover, and uneven branch coverage in the five-route Page migration. The visual-regression baseline strategy is also under-specified (cross-platform PNGs, byte-identical claim, mask/animation handling for new chrome).

**Findings**: 5 major, 7 minor.

### Correctness

**Summary**: The plan is meticulously sequenced and covers most boundary conditions explicitly (mtime_ms == 0 sentinel, project-prefix fallback, sort tie-breakers). However, there is a load-bearing correctness contradiction in the facet-count contract: a single server-side endpoint cannot emit per-facet counts that reflect "post-other-facet, pre-own-facet scoping" against a user's interactive selection. There are also several smaller logic issues — non-deterministic latest-preview tie-breaking, an incorrect grep verification regex, a missing `Default` derive, and an under-specified ID/date column behaviour — that should be tightened before implementation.

**Findings**: 1 critical, 5 major, 5 minor.

### Standards

**Summary**: The plan demonstrates strong fluency with existing conventions: it follows the camelCase wire-format pattern, the TDD-first component layout, the migration.test.ts allowlist mechanism, the kebab-case serde rename pattern, and the /api/<noun> endpoint shape. Two correctness gaps stand out: an incorrect claim about the LAYOUT_TOKENS parity test, and incomplete WAI-ARIA menu-button trigger attributes for the hand-rolled Popover. A scattering of smaller convention drift items (query-key shape, facet id casing, explicit serde rename redundancy, deferred Sidebar wiring decision) round out the findings.

**Findings**: 2 major, 8 minor.

### Usability

**Summary**: The plan introduces a clean Page wrapper, a hand-rolled Popover primitive, and well-scoped sort/filter pills with sensible defaults. The Page API is minimal and discoverable, but several developer-facing affordances are under-specified (typing of maxWidth, eyebrow shape, slot semantics) and end-user keyboard/UX contracts have notable gaps (no Tab handling spec, no Space-to-activate, no checkbox keybindings for multi-select filters, no announcement on filter changes). The redirect removal is a meaningful behavioural break that deserves an explicit deprecation/communication step.

**Findings**: 4 major, 5 minor, 3 suggestions.

### Compatibility

**Summary**: The plan is a single-app, single-consumer change (the visualiser frontend and server are co-versioned), so most of the breaking wire-format changes are absorbed atomically. The deliberate `prs` → `pr-descriptions` rename is well-scoped on the wire/UI side and the directory/config asymmetry is documented. The two notable compatibility concerns the plan does not address are (a) persisted browser state in `localStorage` keyed on the old `DocTypeKey` token, which will silently invalidate after the rename, and (b) externally communicated/bookmarked URLs that used to land at `/library/decisions` via the redirect. Both are low-blast-radius but worth calling out explicitly in the plan's Migration Notes.

**Findings**: 1 major, 5 minor.

---

## Re-Review (Pass 2) — 2026-05-17

**Verdict:** REVISE

The plan author worked the entire prioritised changes list. Of the previous 70 findings, **57 resolved, 9 partially resolved, 4 still present**. The critical correctness contradiction (facet-count scoping) is **resolved** end-to-end: server accepts a `selection` query parameter, two-pass aggregator implements post-other-facet / pre-own-facet scoping, React Query keys include selection, and FilterPill is a pure render layer over server-emitted counts. Standards, code-quality, and correctness lenses each report near-total resolution. Usability resolved most issues but two end-user-discoverability gaps remain (OR/AND rule, sort tie-breakers).

However, the new design surface introduced **9 new major findings** — most cluster around the new `selection` query-parameter encoding (which may not actually decode as written) and the new dual selection types. The plan moved from "1 critical + 18 majors" to "0 critical + 9 majors" — substantial progress, but still above the REVISE threshold (3 majors).

### Cross-Cutting Themes (new findings)

- **Selection wire encoding is under-specified and risky** (correctness 🟡 + compatibility 🟡 + test-coverage 🟡) — Three lenses raise distinct concerns about the new `selection[<docType>][<facetId>]=opt1,opt2` query syntax:
  - **Correctness**: axum's `Query` extractor uses `serde_urlencoded`, which does NOT decompose bracket-nested keys into a nested `HashMap`. The field-level `#[serde(deserialize_with = "deserialise_selection")]` will not be invoked for keys like `selection[decisions][status]` — the selection will arrive empty and the entire scoping behaviour silently degrades to the no-selection case.
  - **Compatibility**: Comma-as-separator inside `opt1,opt2` is unsafe for option ids derived from slugs or work-item prefixes, which can contain commas (uncommon but not forbidden). No escaping rule is documented.
  - **Test Coverage**: The hand-rolled `deserialise_selection` has no unit tests for malformed input, URL-encoded characters, empty values, or unknown doc-type keys.
- **Two parallel selection types** (architecture 🟡 + code-quality) — `LibrarySelection = Partial<Record<DocTypeKey, Record<string, string[]>>>` (wire) coexists with `FilterSelection = { [facetId: string]: string[] }` (FilterPill API); LibraryTypeView translates ad-hoc at the call site. No shared abstraction or named adapter.
- **Module-level `activePopover` singleton** (architecture + code-quality + test-coverage 🟡) — Coordinates active-popover dismissal via mutable module state. Couples test isolation and Storybook/multi-root scenarios. No `beforeEach` reset or `__resetForTesting` hook specified.
- **RootLayout / Hub data-ownership story is internally contradictory** (code-quality + compatibility) — Phase 4 §1 says LibraryOverviewHub is prop-driven; Phase 4 §4 says it "reads from the same query key via its own `useQuery` call". Implementer can't tell which is intended.
- **Testing Strategy summary contradicts resolved decisions** (test-coverage) — Lines 1344 and 1347 still mention `getComputedStyle` (rejected in Phase 1) and the `kind` field (dropped from the wire). Stale text.

### Previously Identified Issues

#### Architecture (8 prior)
- 🟡 **Phase-to-doc-type mapping moves from client to server but stays hardcoded** — Partially resolved (centralised in `PHASES` table with deferred-config evolution path documented)
- 🟡 **Single endpoint conflates three orthogonal concerns** — Still present (one endpoint serves Sidebar + Hub + FilterPill)
- 🟡 **Page owns title styling, creating a hidden cross-cutting contract** — Resolved (REQUIRED list updated; Page owns title; line 59 documents)
- 🔵 **Sidebar data-source ambiguity** — Resolved (prop-driven decision documented in Phase 4 §3)
- 🔵 **`empty_description` reserved-but-ignored** — Resolved (field dropped entirely)
- 🔵 **prs/pr-descriptions asymmetry needs an architectural anchor** — Resolved (inline comment at `config_path_key()`)
- 🔵 **Caching deferred without trigger condition** — Resolved (p95 > 50ms / >10k entries threshold documented)
- 🔵 **Popover focus-return assumes single active popover** — Partially resolved (module-level `activePopover` singleton — see new finding)

#### Code Quality (11 prior)
- 🟡 **FilterPill option-count contract incompatible with server** — Resolved
- 🟡 **LibraryAggregates data clump** — Resolved (single `HashMap<DocTypeKey, PerTypeAggregate>`)
- 🟡 **`build_structure` conflates three decisions** — Resolved (decomposed into `build_doc_type`, `build_facets`, `facet_label`, `PHASES`)
- 🔵 **Wire fields `glyphId`, `route`** — Resolved (dropped)
- 🔵 **`FacetKind` presentation leaking server-side** — Resolved (computed client-side)
- 🔵 **Sidebar wiring decision deferred** — Resolved
- 🔵 **Popover bundles concerns** — Partially resolved (`useDismiss` extracted; positioning/focus/key still combined)
- 🔵 **KanbanBoard per-branch duplication** — Resolved (lift pattern adopted)
- 🔵 **formatDocId 4-digit assumption** — Resolved (5+ digit test added)
- 🔵 **Residual-`prs` grep pattern** — Resolved (per-identifier checks)
- 🔵 **Page raw maxWidth string prop** — Resolved (closed union)

#### Test Coverage (12 prior)
- 🟡 **Sort tie-breaker matrix** — Resolved (per-option, three sub-cases for recently-modified)
- 🟡 **Facet-count coverage on wrong layer** — Resolved (server tests pin scoping)
- 🟡 **Focus-management tests jsdom-unreliable** — Partially resolved (user-event used; explicit jsdom focus-spy strategy still not documented)
- 🟡 **Branch coverage uneven across 5 migrations** — Resolved (per-route branches enumerated)
- 🟡 **Byte-identical claim by comment** — Resolved (shasum diff)
- 🔵 **mtime multi-entry sentinel** — Resolved
- 🔵 **Description-table completeness** — Resolved (data-driven iter `DOC_TYPE_KEYS`)
- 🔵 **Filter-empty transitions** — Resolved
- 🔵 **Theme via data-theme decorative** — Resolved
- 🔵 **getComputedStyle in jsdom** — Resolved
- 🔵 **>8 threshold boundary** — Partially resolved (server side tests cover 7/8/9; client-side rule only tested at 11)
- 🔵 **Residual-prs grep regex** — Resolved (`-w` flag + per-identifier checks)

#### Correctness (11 prior)
- 🔴 **Facet-count scoping** — Resolved (selection-aware endpoint, two-pass aggregator)
- 🟡 **Latest-preview tie-break non-deterministic** — Resolved (rel_path tie-break + test)
- 🟡 **`LibraryAggregates::default()` without derive** — Resolved (`#[derive(Default)]`)
- 🟡 **Residual-`prs` regex** — Resolved
- 🟡 **DOC_TYPE_KEYS rename ordering** — Resolved
- 🟡 **Mixed ID/DATE column** — Resolved (per-row fallback chain spec'd)
- 🔵 **Project-prefix empty-string fallback** — Resolved (non-empty filter, test case)
- 🔵 **ID formatter ASCII-only** — Resolved (broadened to `[^-]+`)
- 🔵 **Sidebar prop signature unresolved** — Resolved
- 🔵 **LifecycleIndex toolbar overflow** — Resolved (moved to content slot)
- 🔵 **StrictMode focus return** — Resolved (guard + transition gate)

#### Standards (10 prior)
- 🟡 **LAYOUT_TOKENS parity test claim** — Resolved (`:root` only, precedent cited)
- 🟡 **Popover trigger ARIA attributes** — Resolved (`triggerProps` with full ARIA)
- 🔵 **Query key shape** — Resolved (function-returning-readonly-tuple)
- 🔵 **Facet ids snake_case** — Resolved (camelCase consistently)
- 🔵 **Explicit `serde(rename)` redundant** — Resolved (dropped + contract test)
- 🔵 **maxWidth raw string** — Resolved (closed union)
- 🔵 **Sidebar wiring** — Resolved
- 🔵 **`config_path_key` asymmetry anchor** — Resolved (inline comment)
- 🔵 **`mod library;` visibility** — Resolved (private)
- 🔵 **Reserved field convention** — Resolved (field deleted)

#### Usability (12 prior)
- 🟡 **maxWidth typing** — Resolved (closed union)
- 🟡 **Popover keyboard contract** — Resolved (Tab/Space/Enter + typeahead deferred)
- 🟡 **FilterPill multi-select model** — Resolved (`menuitemcheckbox`, toggle-no-close)
- 🟡 **Redirect removal communication** — Partially resolved (Migration Notes broadened; no in-app one-time hint)
- 🔵 **Eyebrow prop typing** — Still present (still `ReactNode`; no typed helper)
- 🔵 **Title required during loading** — Resolved (callers pass `'Loading…'`)
- 🔵 **OR-within / AND-across rule not in UI** — Still present (only in code comments)
- 🔵 **Search-input keyboard handoff** — Resolved
- 🔵 **Zero-count card UX** — Resolved (dimmed + aria-disabled)
- 🔵 **Sort default visibility** — Resolved; tie-breakers still invisible (partially resolved)
- 🔵 **Clear filters as only recovery** — Partially resolved (summary added; per-filter chips not)
- 🔵 **Page wrapper self-composability** — Still present (no guard against nesting)

#### Compatibility (6 prior)
- 🟡 **localStorage `prs` key migration** — Resolved (parseStored migration + test)
- 🔵 **Redirect bookmarks impact** — Resolved (Migration Notes broadened)
- 🔵 **Stale-bundle reconnect** — Resolved (release-notes line specified)
- 🔵 **Redundant explicit `serde(rename)`** — Resolved (dropped + all-variants contract test)
- 🔵 **Visual-regression byte-identity** — Resolved (shasum diff)
- 🔵 **No cache headers on endpoint** — Still present (minor for local dev tool)

### New Issues Introduced

#### Major

- 🟡 **Correctness**: Bracket-nested query syntax cannot be decoded by axum's Query extractor into a nested HashMap
  **Location**: Phase 2 §2 (`LibraryStructureQuery` / `deserialise_selection`)
  `serde_urlencoded` treats `selection[decisions][status]` as a single flat key — it does not decompose into nested maps. The field-level `deserialize_with` is never invoked. The whole selection-aware feature may silently no-op at runtime. Either parse the raw query string manually inside the handler, or use a flat encoding (e.g. repeated keys, JSON-encoded `?selection=…`).

- 🟡 **Compatibility**: Comma-as-separator in `selection[type][facet]=a,b` corrupts option ids containing commas
  **Location**: Phase 2 (selection encoding)
  Option ids from `clusterSlug` and `project` derive from filenames and frontmatter; commas are uncommon but not forbidden. No escaping rule documented. Switch to repeated keys (`?…[status]=open&…[status]=blocked`) or pin a percent-encoding contract with a corpus of awkward inputs in tests.

- 🟡 **Correctness**: `frontmatter.date` is typed `unknown`; the fallback chain dereferences it without narrowing
  **Location**: Phase 5 §6 first-column fallback
  `IndexEntry.frontmatter` is `Record<string, unknown>`. `formatDate(entry.frontmatter.date)` either fails type-check or silently coerces non-string values. Add `typeof === 'string'` narrowing plus `Date.parse` guard; mirror with a test using a non-string fixture.

- 🟡 **Correctness**: Page subtitle renders `"undefined documents"` during pending state
  **Location**: Phase 5 §6 (LibraryTypeView subtitle)
  `${filteredCount} documents` with `filteredCount` undefined on initial fetch and every selection toggle produces a visible `"undefined documents"` until the response lands. Guard with `isPending ? '…' : …` or use `placeholderData: keepPreviousData`.

- 🟡 **Correctness**: `entry!` claim in LibraryDocView lift is not actually substantiated
  **Location**: Phase 6 §3
  The plan claims the `entry!` non-null assertion is no longer needed under the lift-Page-above-branch pattern, but lifting alone doesn't narrow `Entry | undefined` to `Entry` — the success branch must still be guarded by an early return or type guard. Either retain the assertion (the original pattern was fine) or sketch the narrowed branch structure explicitly.

- 🟡 **Usability**: Facet toggles trigger a server refetch with no specified loading affordance
  **Location**: Phase 5 §3 / §6
  Every FilterPill toggle changes the React Query key and triggers a refetch. No spinner/pulse on the FilterPill trigger, no policy for what counts to display during refetch. Users may interpret lag as the filter not working. Add `isFetching` indicator + `placeholderData: keepPreviousData`.

- 🟡 **Test Coverage**: End-to-end selection → refetch → counts wiring is not asserted
  **Location**: Phase 5 §6 (LibraryTypeView)
  Server scoping, FilterPill, and query-keys are tested in isolation, but no integration test exercises the full loop (toggle option → key changes → fetch called with new selection → counts re-render). A wiring break would not be caught.

- 🟡 **Test Coverage**: `deserialise_selection` query parser has no unit tests
  **Location**: Phase 2 §2
  Non-trivial hand-rolled parser; no tests for malformed input, URL-encoded option ids, empty values, unknown doc-type keys.

- 🟡 **Architecture**: Two parallel selection types (`LibrarySelection` vs `FilterSelection`)
  **Location**: Phase 2 §3 + Phase 5 §3
  Wire shape and FilterPill API are differently-shaped; LibraryTypeView translates ad-hoc. No named adapter; relationship is implicit. Pick one shape or introduce `librarySelectionFor(type, selection)`.

#### Minor

- 🔵 **Architecture**: RootLayout's `libraryStructure()` cache key collides with selection-scoped fetches in LibraryTypeView (Phase 4 §3 / Phase 2 §3)
- 🔵 **Architecture**: Module-level `activePopover` singleton couples instances through global state (Phase 1 §4)
- 🔵 **Architecture**: `Selection` type defined in `indexer.rs` but is a wire concern (Phase 2 §1)
- 🔵 **Code Quality**: `PHASES` table membership shown as `/* ... */` placeholders; not enumerated (Phase 2 §2)
- 🔵 **Code Quality**: RootLayout/Hub data-ownership story internally contradictory (Phase 4 §1 vs §4)
- 🔵 **Code Quality**: Selection shape mismatch between FilterPill and fetchLibraryStructure (Phase 5 §6)
- 🔵 **Test Coverage**: Module-level `activePopover` shared mutable state with no reset (Phase 1 §4)
- 🔵 **Test Coverage**: Client-side `>8` threshold tested only at 11 options (Phase 5 §3)
- 🔵 **Test Coverage**: Testing Strategy summary contradicts resolved decisions (lines 1344, 1347)
- 🔵 **Test Coverage**: Empty-array selection semantics not asserted at helper level (Phase 2 §1)
- 🔵 **Correctness**: Second pass collects `Vec<&IndexEntry>` per doc type — O(N·T) scan shape; PERF threshold comment may misrepresent cost
- 🔵 **Correctness**: `facets_for` uses string match, not enum match — rename safety footgun
- 🔵 **Correctness**: Selection cache-key normalisation (`selection={}` vs `{decisions:{status:[]}}`) not specified
- 🔵 **Standards**: Popover trigger render-prop omits `onKeyDown` for Enter/Space/Down to open (Phase 1 §4)
- 🔵 **Usability**: `ID / DATE` column creates heterogeneous scanning (Phase 5 §6)
- 🔵 **Usability**: NoResultsPanel summary not actionable (Phase 5 §5)
- 🔵 **Usability**: Loading/error placeholder titles announce as page H1 (Phase 6)
- 🔵 **Compatibility**: 600px literal in `global.css` may require migration.test.ts allowlist update in Phase 1
- 🔵 **Compatibility**: Response shape snapshot baseline missing (Phase 2)
- 🔵 **Compatibility**: RootLayout/Hub cache-key parity depends on identical `selection` argument shape

#### Suggestions

- 🔵 **Code Quality**: Document a test reset (e.g. `__resetActivePopover()`) for the module-level singleton
- 🔵 **Usability**: Dimmed zero-count cards may still appear clickable — add `cursor: not-allowed` or a microcopy hint

### Assessment

The plan is **substantially better** than the first pass — the load-bearing critical correctness issue is resolved and the convention/standards story is now coherent. The remaining major findings cluster around **three concrete decisions that need pinning before implementation begins**:

1. **The `selection` query-parameter encoding contract**. The current `selection[type][facet]=a,b` syntax has two distinct flaws: it may not parse through axum/serde_urlencoded as planned (correctness), and comma-as-separator is unsafe for slug/project ids (compatibility). Recommended fix: switch to repeated keys (`?selection[decisions][status]=open&selection[decisions][status]=blocked`) which both serde_urlencoded and `URLSearchParams` handle natively, and pin the contract with an HTTP-level round-trip test.

2. **The `Selection` type definition lives in two places with two shapes**. Pick `LibrarySelection` everywhere (drop `FilterSelection` and have FilterPill take a per-type slice typed via `LibrarySelection[DocTypeKey]`), or introduce a named adapter that both LibraryTypeView and tests use.

3. **The lift-Page-above-branch pattern needs concrete narrowing structure**, particularly for LibraryDocView where the `entry!` claim is unsubstantiated. Either show the narrowed structure in the plan or restore the explicit non-null assertion.

After those three fixes plus the smaller corrections (subtitle pending state, frontmatter.date narrowing, FilterPill loading affordance, Testing Strategy summary cleanup), the plan would be **APPROVE**-ready. A third pass is recommended to verify these concrete fixes; the remaining minors can be carried in implementation.

---

## Re-Review (Pass 3) — 2026-05-17

**Verdict:** REVISE

The plan continues to improve. All pass-2 majors that were structural or load-bearing are now resolved: the bracket-nested query encoding is replaced with repeated keys and a hand-rolled `parse_selection_query`; `LibrarySelection` / `LibrarySelectionPerType` unifies the type duplication; `frontmatter.date` narrowing is explicit; subtitle reads from `query.data` with `placeholderData: keepPreviousData`; LibraryDocView's lift uses real `if/else if (entry)` narrowing. End-to-end selection wiring, `parse_selection_query` parser, FilterPill `isFetching`, and `normaliseSelection` cache parity all have dedicated tests.

The trend across passes is unambiguous: **1 critical + 18 majors → 0 critical + 9 majors → 0 critical + 5 majors**. The five remaining majors are smaller-scope than before and all addressable with focused edits; none are load-bearing structural issues. Verdict remains REVISE per the threshold (3+ majors), but the plan is close to APPROVE-ready.

### Cross-Cutting Themes (pass-3 new findings)

- **`keepPreviousData` interactions are under-tested and have a cross-route bleed** (correctness 🟡 + test-coverage 🟡): When navigating between doc types (e.g. `/library/decisions` → `/library/plans`), `keepPreviousData` keeps the *previous doc type's* data while the new query is in flight — so the subtitle briefly shows `"N documents"` where N is the wrong type, and `filterFacets` momentarily render the wrong facets. Also, the `keepPreviousData` "stale rows stay on screen during refetch" promise has no dedicated test; only the cold-cache `'…'` pending state is asserted.
- **`normaliseSelection` does not sort option arrays** (compatibility 🟡): Order-of-toggling matters for cache identity — toggling `open` then `blocked` produces a different React Query cache key than `blocked` then `open`, even though the server treats them as equivalent. The pass-2 cache-parity fix was incomplete.
- **`parse_selection_query` body is `todo!()`** (code-quality 🟡): The contract is fully specified by 10 inline test cases, but the parser body is a placeholder. `DocTypeKey::from_wire_str` is referenced but not defined anywhere (correctness 🔵).
- **Cache-warm "no fetch" assertion** (test-coverage 🟡): The RootLayout/Hub coupling claims it's pinned by a hub-test that asserts zero fetcher invocations on cache hit. The promise is in the prose (Phase 4 §4); the corresponding test bullet is not in the LibraryOverviewHub test list.
- **Persistent items deliberately accepted as tradeoffs**: module-level `activePopover` singleton, `Selection` in `indexer.rs`, `facets_for` string match, `O(N·T)` two-pass aggregator, no HTTP cache headers, eyebrow as `ReactNode`, `<h1>` placeholder titles, dimmed clickable cards — all remain unchanged across passes. Acceptable v1 tradeoffs.

### Previously Identified Issues (Pass 2 → Pass 3)

#### Architecture (4 pass-2 findings)
- 🟡 **Two parallel selection types** — Resolved (`LibrarySelectionPerType` slice; single source of truth)
- 🔵 **Cache-key collision** — Resolved (`normaliseSelection` helper + tests)
- 🔵 **`activePopover` singleton** — Still present (acknowledged tradeoff)
- 🔵 **`Selection` in `indexer.rs`** — Still present (acknowledged tradeoff)

#### Code Quality (4 pass-2 findings)
- 🔵 **PHASES table stubbed** — Still present (placeholders not filled in)
- 🔵 **RootLayout/Hub story contradictory** — Still present (§1 says prop-driven; §4 still offers "OR via own useQuery")
- 🔵 **Selection shape mismatch** — Resolved
- 🔵 **`activePopover` hidden global** — Still present (acknowledged)

#### Test Coverage (6 pass-2 findings)
- 🟡 **End-to-end selection wiring** — Resolved (single integration test pinning toggle→refetch→counts→clear)
- 🟡 **`parse_selection_query` no unit tests** — Resolved (10 inline cases)
- 🟡 **`activePopover` reset** — Partially resolved (no `afterEach`/unmount assertion)
- 🔵 **`>8` threshold client-side single-length** — Partially resolved (server tests 7/8/9; client still 11 only)
- 🔵 **Testing Strategy summary contradictions** — Still present (lines 1432, 1435 reference `getComputedStyle` and `kind`)
- 🔵 **Empty-array selection helper level** — Partially resolved (parse tests, not helper tests)

#### Correctness (7 pass-2 findings)
- 🟡 **Bracket-nested query decode** — Resolved (`RawQuery` + manual parse)
- 🟡 **`frontmatter.date` narrowing** — Resolved (explicit `typeof === 'string'` + non-string fallthrough test)
- 🟡 **Subtitle "undefined documents"** — Resolved (reads `query.data` with placeholder)
- 🟡 **`entry!` claim** — Resolved (explicit `if/else if (entry)` narrowing)
- 🔵 **O(N·T) scan shape** — Still present (perf concern, not correctness)
- 🔵 **`facets_for` string match** — Still present (rename footgun unchanged)
- 🔵 **Selection cache-key normalisation** — Resolved (`normaliseSelection`)

#### Standards (1 pass-2 finding)
- 🔵 **Trigger keyboard activation** — Partially resolved (relies on consumer using `<button>` for native Enter/Space; no `onKeyDown` on trigger)

#### Usability (5 pass-2 findings)
- 🟡 **Loading affordance** — Resolved (`isFetching` pulse + `keepPreviousData`)
- 🔵 **ID/DATE heterogeneous scanning** — Still present
- 🔵 **NoResultsPanel summary not actionable** — Partially resolved (summary added, no per-filter chips)
- 🔵 **`<h1>` placeholder titles** — Still present (codified rather than fixed)
- 🔵 **Dimmed clickable cards** — Still present

#### Compatibility (5 pass-2 findings)
- 🟡 **Comma-as-separator** — Resolved (repeated keys)
- 🔵 **600px in `global.css` allowlist** — Partially resolved (Phase 1 doesn't explicitly check migration test scans `global.css`)
- 🔵 **Response shape snapshot** — Partially resolved (variant tokens pinned; full shape not snapshot-tested)
- 🔵 **RootLayout/Hub cache-key parity** — Resolved (`normaliseSelection`)
- 🔵 **No cache headers** — Still present (minor for local dev tool)

### New Issues Introduced (Pass 3)

#### Major

- 🟡 **Code Quality**: `parse_selection_query` body is `todo!()` with no concrete algorithm
  **Location**: Phase 2 §2 (lines 572-579)
  The parser is left as an implementer exercise; `DocTypeKey::from_wire_str` is referenced but never defined. A parser passing the 10 listed test cases by hard-coding them is still "green". Sketch the parser body in 10-15 lines or commit to `serde_qs` (which does support bracket syntax). Pin `DocTypeKey::from_wire_str` symmetrically with the `to_value` contract test.

- 🟡 **Test Coverage**: `placeholderData: keepPreviousData` behaviour not asserted
  **Location**: Phase 5 §6
  The pending-state subtitle test only covers cold-cache first paint. No test asserts that rows and counts stay visible during a *refetch* after a successful first load. A regression that removes `keepPreviousData` would silently blank the table on every toggle.

- 🟡 **Test Coverage**: Cache-warm "no fetch" contract claimed in prose, not in test list
  **Location**: Phase 4 §4 promises it; LibraryOverviewHub test list (Phase 4 §1) omits it
  The single most important coupling between RootLayout and the hub is unverified by the named test plan.

- 🟡 **Correctness**: `keepPreviousData` shows the previous doc type's count when navigating between types
  **Location**: Phase 5 §6
  The cache key includes `[type]: selection` — navigating `/library/decisions` → `/library/plans` makes `query.data` be the previous type's data briefly. Subtitle would show `"N documents"` for the wrong type; `filterFacets` would render the wrong facets. Either scope `placeholderData` to selection changes only (not key changes that include a new type), or guard renders on `query.data?.docTypes.find(dt => dt.id === type)`.

- 🟡 **Compatibility**: `normaliseSelection` does not sort option arrays
  **Location**: Phase 2 §3 + Phase 5 §6
  Order-of-toggling matters for cache identity: `['open', 'blocked']` and `['blocked', 'open']` produce different React Query keys but are semantically equivalent server-side. Have `normaliseSelection` sort option arrays and per-doc-type keys for a true canonical form.

#### Minor

- 🔵 **Code Quality**: Hub cards carry `filter_facets` payload they don't consume (wire bloat)
- 🔵 **Code Quality**: FilterPill `isFetching` mixes data + async-state concerns (rename to `busy`?)
- 🔵 **Code Quality**: Mutable `let` reassignment pattern in lifted Page sketch is harder to read than a small helper / IIFE
- 🔵 **Code Quality**: `PHASES` uses unlabelled tuple slice; rename a labelled struct
- 🔵 **Code Quality**: End-to-end test bundles 4 assertions into one case — split for bug-bisection
- 🔵 **Test Coverage**: RootLayout's new `useQuery` wiring has no test coverage specified
- 🔵 **Test Coverage**: Lift-then-narrow branch matrix in LibraryDocView under-specified (`query.isError` vs `!isPending && !entry` collapse)
- 🔵 **Test Coverage**: `activePopover` lifecycle has no cleanup/reset assertion
- 🔵 **Correctness**: `entry!` retention claim in prose contradicts the example (which uses real narrowing)
- 🔵 **Correctness**: `extract_facet_value(project)` for `None work_item_id` undefined
- 🔵 **Correctness**: `DocTypeKey::from_wire_str` referenced but never defined
- 🔵 **Correctness**: `normaliseSelection` handling of `{decisions: undefined}` partial Records not specified
- 🔵 **Standards**: Inconsistent `axum::extract::*` import style in new `library.rs`
- 🔵 **Usability**: Bare `'…'` subtitle placeholder is ambiguous (try `'Loading…'` + `aria-busy`)
- 🔵 **Usability**: `keepPreviousData` masks `'…'` placeholder on subsequent loads — inconsistent feedback
- 🔵 **Usability**: Pulse indicator under-specified; no `aria-busy` on trigger
- 🔵 **Compatibility**: `placeholderData: keepPreviousData` requires React Query v5 — verify against `package.json`
- 🔵 **Compatibility**: `RawQuery` silent malformed-key drop conflicts with HTTP standard expectations

#### Suggestions

- 🔵 **Architecture**: Add test that `libraryStructure({ decisions: {} })` normalises identically to `libraryStructure()`
- 🔵 **Standards**: Document the repeated-key bracket convention since no other endpoint uses one
- 🔵 **Usability**: Consider `document.title` implications of placeholder `<h1>` titles
- 🔵 **Code Quality**: Document the module-level `activePopover` test-reset story explicitly

### Assessment

The plan is in **fundamentally good shape**. The 5 remaining majors are all addressable with focused, narrow edits (~30-60 minutes of plan work):

1. **`normaliseSelection` array sort** — extend the helper spec and add a test.
2. **`keepPreviousData` cross-type guard** — add `query.data?.docTypes.find(dt => dt.id === type)` to the render contract, plus a navigation-between-types test.
3. **`parse_selection_query` body sketch** — 10-15 lines of pseudocode; define `DocTypeKey::from_wire_str` explicitly.
4. **`keepPreviousData` refetch behaviour test** — sub-case in the end-to-end integration test asserting rows stay visible during in-flight refetch.
5. **Cache-warm "no fetch" hub test** — add the assertion to the LibraryOverviewHub test list.

Plus the Testing Strategy summary cleanup (drop `getComputedStyle` and `kind` mentions) and the §1/§4 RootLayout-vs-Hub data-flow contradiction.

After these fixes a fourth review pass should land at **APPROVE**. The remaining minors are reasonable to carry into implementation and address via PR review.

---

## Re-Review (Pass 4) — 2026-05-17

**Verdict:** APPROVE

The pass-3 follow-up addressed every pass-3 major and resolved the §1/§4 contradiction. All seven lenses now produce **0 critical + 0 major** findings — clean trajectory from pass 1 (1 critical + 18 majors) → pass 2 (0 critical + 9 majors) → pass 3 (0 critical + 5 majors) → pass 4 (0 critical + 0 majors).

### Per-Lens Recommendations

| Lens | Recommendation | Findings |
|------|----------------|----------|
| Architecture | **APPROVE** | 1 suggestion (extract `useLibraryStructure` hook) |
| Code Quality | **COMMENT** | 5 minors carried; plan is implementable |
| Test Coverage | **APPROVE** | 3 minors (RootLayout query, activePopover reset, FilterPill 8/9 boundary) |
| Correctness | **APPROVE** | 1 minor + 1 suggestion (optional-chaining style, test wording) |
| Standards | **APPROVE** | 2 minors + 1 suggestion (axum import style, form_urlencoded crate, normaliseSelection impl pick) |
| Usability | **APPROVE** | 2 suggestions (Loading… could be more specific, aria-busy on isFetching) |
| Compatibility | **APPROVE** | 2 minors (Array.sort locale-insensitivity note, wire_str/serde drift) |

6 lenses recommend APPROVE; code-quality recommends COMMENT because several pass-2/3 minors persist by design (module-level `activePopover` singleton, `PHASES` tuple-slice stubs, `O(N·T)` two-pass aggregator, `facets_for` string match, end-to-end test bundles 4 assertions). These are acknowledged tradeoffs documented in the plan itself; none are load-bearing.

### Previously Identified Issues (Pass 3 → Pass 4)

#### Architecture (1 pass-3 finding)
- 🔵 **LibraryOverviewHub data-flow contradiction** — Resolved (Phase 4 §1 commits to hub-owned `useQuery` with shared key; cache-warm no-fetch test pins dedup invariant)

#### Code Quality (9 pass-2/3 findings)
- 🟡 **`parse_selection_query` body `todo!()`** — Resolved (concrete 15-line body using `form_urlencoded::parse`)
- 🔵 **`PHASES` table stubbed** — Still present (placeholders not filled in)
- 🔵 **RootLayout/Hub data-ownership** — Resolved
- 🔵 **`activePopover` singleton** — Still present (acknowledged tradeoff)
- 🔵 **Hub cards carry `filter_facets` wire bloat** — Still present (acknowledged tradeoff)
- 🔵 **FilterPill `isFetching` mixes concerns** — Still present
- 🔵 **Mutable `let` reassignment in lifted Page sketch** — Still present
- 🔵 **`PHASES` unlabelled tuple slice** — Still present
- 🔵 **End-to-end test bundles 4 assertions** — Still present

#### Test Coverage (8 pass-3 findings)
- 🟡 **`keepPreviousData` refetch behaviour** — Resolved (deferred-promise mid-flight test)
- 🟡 **Cache-warm "no fetch" contract** — Resolved (test added to hub list)
- 🔵 **`activePopover` lifecycle reset** — Partially resolved (no `afterEach` reset assertion)
- 🔵 **Client-side `>8` threshold boundary** — Partially resolved (server tests 7/8/9; client still 11 only)
- 🔵 **Testing Strategy summary contradictions** — Resolved (rewritten; no `getComputedStyle`/`kind`)
- 🔵 **Empty-array selection helper level** — Resolved (helper test added)
- 🔵 **RootLayout `useQuery` wiring** — Still present (no test list for RootLayout)
- 🔵 **LibraryDocView branch matrix** — Resolved

#### Correctness (7 pass-3 findings)
- 🟡 **`keepPreviousData` cross-type bleed** — Resolved (`currentTypeData` derivation + test)
- 🔵 **`entry!` retention claim contradiction** — Resolved (prose retracted; example uses real narrowing)
- 🔵 **`extract_facet_value(project)` for `None`** — Resolved (semantics pinned)
- 🔵 **`DocTypeKey::from_wire_str` undefined** — Resolved (helper + round-trip test)
- 🔵 **`normaliseSelection` `undefined` handling** — Resolved (canonical-form spec)
- 🔵 **`O(N·T)` scan shape** — Still present (acknowledged perf tradeoff)
- 🔵 **`facets_for` string match** — Still present (acknowledged tradeoff)

#### Standards (3 pass-3 findings)
- 🔵 **Popover trigger `onKeyDown`** — Resolved (consumer-must-be-button contract documented)
- 🔵 **`axum::extract::*` import style** — Still present (see new minor)
- 🔵 **Repeated-key bracket convention** — Partially resolved (documented why; convention established)

#### Usability (8 pass-2/3 findings)
- 🔵 **Bare `'…'` subtitle** — Resolved (`'Loading…'`)
- 🔵 **`keepPreviousData` masks placeholder** — Resolved (`currentTypeData` guard makes it surface again on cross-type nav)
- 🔵 **Pulse indicator under-specified** — Still present (no `aria-busy`)
- 🔵 **Placeholder titles in tab title** — Still present
- 🔵 **ID/DATE heterogeneous scanning** — Still present
- 🔵 **NoResultsPanel summary not actionable** — Partially resolved (summary added; no chips)
- 🔵 **`<h1>` placeholder titles** — Still present
- 🔵 **Dimmed clickable cards** — Resolved (no `<Link>` wrapper)

#### Compatibility (7 pass-3 findings)
- 🟡 **`normaliseSelection` array sort** — Resolved (canonical-form spec)
- 🔵 **`DocTypeKey::from_wire_str` undefined** — Resolved
- 🔵 **`placeholderData: keepPreviousData` requires v5** — Resolved (confirmed v5 in package.json)
- 🔵 **`RawQuery` silent malformed drop** — Still present (acknowledged for internal API)
- 🔵 **600px in `global.css` allowlist** — Still present (Phase 1 doesn't explicitly cover global.css scan)
- 🔵 **Response shape snapshot** — Still present (field-by-field assertions instead)
- 🔵 **No cache headers** — Still present (acknowledged)

### New Issues Introduced (Pass 4)

No new majors. ~12 new minors / suggestions surfaced across the seven lenses; none are blocking:

#### Minor
- 🔵 **Code Quality + Architecture**: Shared-query-key dedup between RootLayout and LibraryOverviewHub is an implicit cross-file contract. Suggestion: extract a `useLibraryStructure()` hook in `api/hooks.ts` so the shared cache key is a hook-internal detail.
- 🔵 **Code Quality**: `currentTypeData` derivation does a per-render `flatMap + concat + find` (O(N) allocation). Suggestion: server emits `docTypesById: Record<DocTypeKey, LibraryDocType>` mirror, or export a `findDocType(response, id)` helper.
- 🔵 **Code Quality**: `library_aggregates` two-pass under one lock allocates `Vec<&IndexEntry>` per doc type. Pre-group entries once at the top to avoid the N·M scan.
- 🔵 **Code Quality**: `wire_str()` duplicates serde `rename_all` derivation. Pin equivalence via `wire_str(v) == serde_json::to_value(v).as_str()` in the per-variant contract test.
- 🔵 **Code Quality**: `normaliseSelection` spec offers two equivalent implementations (`JSON.parse(JSON.stringify(...))` vs manual sorted-key iteration). Pick one.
- 🔵 **Test Coverage**: RootLayout's new `useQuery` wiring has no test coverage specified. Add `RootLayout.test.tsx` assertions for fetch invocation, prop threading, pending/error states.
- 🔵 **Test Coverage**: `activePopover` module singleton has no test-cleanup strategy. Add `__resetForTesting` or assert reset on close.
- 🔵 **Test Coverage**: Client-side `> 8` threshold tested only at length 11. Add boundary tests at 8 (no search) and 9 (search).
- 🔵 **Correctness**: `currentTypeData` mixes optional chaining (`query.data?`) with required access (`query.data.templates`). Functionally correct; suggest a guard-and-assign pattern for readability.
- 🔵 **Correctness**: Cross-bleed test described as "re-mount" may not exercise `keepPreviousData` path (which retains data across key changes within a stable mount). Phrase as "navigate via router" instead.
- 🔵 **Standards**: `library.rs` imports `Query` unused; `RawQuery` referenced via fully-qualified path. Update to `use axum::{extract::{RawQuery, State}, Json};` and unqualified handler signature.
- 🔵 **Standards**: `form_urlencoded` crate dependency referenced but no `Cargo.toml` edit in plan. Add the explicit dependency line.
- 🔵 **Compatibility**: Default `Array.prototype.sort()` lexicographic ordering is the right choice for environment-invariant cache keys, but pin this explicitly (`localeCompare` would re-introduce locale-dependent drift).
- 🔵 **Compatibility**: `wire_str` manual match could drift from serde's `rename_all` output if a future variant uses unusual casing. Cross-check via the per-variant contract test.

#### Suggestions
- 🔵 **Architecture**: Extract `useLibraryStructure()` hook to make the shared-cache contract explicit.
- 🔵 **Usability**: Subtitle `'Loading…'` is unambiguous but generic — consider `'Loading decisions…'` using the active type label.
- 🔵 **Usability**: Add `aria-busy={isFetching}` on FilterPill trigger for assistive-tech parity with sighted users.
- 🔵 **Correctness**: Define `currentTypeData` via guard-and-assign rather than mixed optional chaining.

### Assessment

The plan reaches **APPROVE**. The trajectory is clean:

| Pass | Critical | Major | Minor | Verdict |
|------|----------|-------|-------|---------|
| 1    | 1        | 18    | ~30   | REVISE  |
| 2    | 0        | 9     | ~17   | REVISE  |
| 3    | 0        | 5     | ~18   | REVISE  |
| 4    | 0        | 0     | ~14   | APPROVE |

Six lenses recommend APPROVE; code-quality recommends COMMENT because several pass-2/3 minors persist by design as documented tradeoffs (module-level singleton, `O(N·T)` aggregator, `facets_for` string match, hub-card wire bloat, bundled e2e assertion, `PHASES` tuple-slice). These are acknowledged in the plan itself and are not load-bearing.

The remaining ~14 minors are all reasonable to carry into implementation and address via PR review. The two highest-value follow-ups for the implementer to action before merge:
1. **Code Quality**: Fill in `PHASES` doc-type membership and replace tuple slice with a named struct.
2. **Standards / Code Quality**: Fix the `axum::extract` import inconsistency in `library.rs` and add the `form_urlencoded` dependency to `Cargo.toml`.

The plan is implementable as-is. Recommend proceeding to `/implement-plan`.
