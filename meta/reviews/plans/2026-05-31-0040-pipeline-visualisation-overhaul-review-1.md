---
date: "2026-06-01T00:00:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-31-0040-pipeline-visualisation-overhaul.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, compatibility, usability]
review_pass: 3
status: complete
---

## Plan Review: Pipeline Visualisation Overhaul

**Verdict:** REVISE

The plan is unusually thorough — it sequences ten phases for independent
merge slices, locks in six explicit decisions to resolve research-document
open questions, names deterministic observables for every behaviour, and
preserves test green at every phase boundary. However, several concrete
code snippets are wrong in ways the plan presents as canonical: the shared
`compute_linked_count` resolver won't compile against the existing async
indexer and breaks AC-6 by skipping `related_get`'s dedup; the `Pipeline`
JSX references a Glyph prop name that does not exist and forces an
off-grid Glyph size; and three "additive" compatibility claims
(`parseWorkItemId` retention, `prototype-tokens.json` snapshot extension,
deletion verification by `rg`) won't hold against the actual codebase.
Server architecture also has an unaddressed coherence story between the
indexer's entry map and the cluster snapshot's entry clones during
watcher-driven refreshes.

### Cross-Cutting Themes

- **`compute_linked_count` shared resolver is not actually shared**
  (flagged by: architecture, code-quality, correctness, compatibility) —
  The plan's helper computes `inferred + outbound + inbound` without
  applying the two dedup passes `related_get` performs (`related.rs:80-89`
  merging inbound; `related.rs:91-102` dropping inferred entries that
  appear in declared). It also calls `async fn` methods synchronously,
  so it won't compile. AC-6's observable-equality contract therefore
  fails by construction on any workspace with overlapping cross-links.
- **Glyph API mismatch in Phase 6 JSX**
  (flagged by: code-quality, standards, compatibility, usability) — The
  plan writes `<Glyph kind={step.docType} size={Math.round(size * 0.6)} />`,
  but the production prop is `docType` and the size union is `16 | 24 | 32`.
  At `Pipeline size=34`, the computed inner size is 20 — off-grid.
- **Hand-maintained parallel stage orderings**
  (flagged by: code-quality, test-coverage, standards) — Stage order
  lives in (1) Rust `derive_completeness` push ladder, (2) Rust `has_*`
  match arms, (3) frontend `WORKFLOW_PIPELINE_STEPS`, (4) the
  `--ac-stage-*-on` token block. No automated test enforces cross-language
  agreement; mutations in any one source drift silently.
- **Three "additive" claims won't hold against the codebase**
  (flagged by: compatibility, standards) — `parseWorkItemId` retention
  cites `announcements.ts` as a consumer that uses a different function
  (`workItemIdFromRelPath`); `prototype-tokens.json` is theme-invariant-only
  and cannot host the theme-variant stage tokens without restructuring;
  the `rg PipelineDots` post-deletion check misses four EXCEPTIONS
  entries in `frontend/src/styles/migration.test.ts`.

### Tradeoff Analysis

- **Wire-shape duplication vs payload bloat**: Code-quality and
  architecture both flag that per-entry inlined `Completeness` clones
  N times within a cluster. Migration Notes proposes `Arc<Completeness>`
  as a memory-only fallback, but the wire payload is already locked in.
  Recommendation: accept the duplication for this story (cohesion
  benefit outweighs cost at expected scale) and explicitly document the
  tradeoff in Migration Notes so a future `completenessSlug` indirection
  isn't surprising.
- **`data-active` as both styling and test hook**: Usability flags
  that visual regressions can pass tests when the attribute is correct
  but CSS rules drift; correctness flags that `data-active={active}`
  serialisation depends on React's boolean→string handling. Pinning the
  observable to a single attribute is the right call for cross-surface
  consistency, but at least one computed-style smoke probe per component
  would close the gap.

### Findings

#### Critical
- 🔴 **Correctness, Compatibility**: `compute_linked_count` diverges from `related_get`'s dedup; AC-6 equality will fail
  **Location**: Phase 3 — `compute_linked_count` snippet
  The helper sums raw bucket lengths without applying the inbound HashSet dedup or the inferred-vs-declared drop that `related_get` performs at `related.rs:79-102`. Any cross-linked entry that also sits in the same cluster will make `entry.linkedCount` exceed the endpoint's array-length sum.

- 🔴 **Correctness**: Helper calls async indexer methods without `.await`; will not compile
  **Location**: Phase 3 — `compute_linked_count` Rust snippet
  `declared_outbound`, `reviews_by_target`, and `work_item_refs_by_id` are all `async fn` returning futures. The plan's `.len()` call on a future is a type error; the helper signature must be `async` (or take pre-resolved snapshot inputs to avoid the deadlock risk if `entries.write()` is held during the back-fill).

- 🔴 **Code Quality, Standards, Compatibility, Usability**: Pipeline JSX uses wrong Glyph prop name and out-of-union size
  **Location**: Phase 6 — `Pipeline.tsx` component snippet
  Uses `<Glyph kind={step.docType} size={Math.round(size * 0.6)} />` but the production prop is `docType` and size is the literal union `16 | 24 | 32`. With `size = 34` (Phase 8 cluster detail), inner = 20 — off-grid. Both errors fail TypeScript. Glyph.tsx explicitly warns: "For off-grid sizes, widen the union with a documented specimen — do not cast."

#### Major
- 🟡 **Architecture, Correctness**: Dual storage of `IndexEntry` (indexer map vs cluster.entries clones) without a coherence story
  **Location**: Phase 2 — Cluster pass back-fill
  `compute_clusters` already clones each entry into `cluster.entries`. After Phase 2 lands, the cluster snapshot's clones carry default-zero `completeness`/`linked_count` while the indexer map is back-filled. `refresh_one` updates entries without rebuilding clusters, so single-file changes leave entries stale until the next rescan. Watcher concurrency exacerbates this: there is no documented lock-ordering for `state.clusters.write()` vs `entries.write()`.

- 🟡 **Correctness**: `WORK_ITEM_ID_BASIC_SHAPE` regex rejects bare digits, contradicting plan prose
  **Location**: Phase 4, Decision 3 — `^[A-Za-z]+-?[0-9]+$`
  The regex requires at least one `[A-Za-z]`, so `"0042"` fails — but the plan documents a "bare digits" branch and asserts a test case `work_item_id: "0042"` should round-trip. The regex also doesn't trim whitespace or reject empty strings explicitly.

- 🟡 **Correctness**: Frontmatter value bypasses `default_project_code` normalisation
  **Location**: Phase 4 — frontmatter-first resolution
  A frontmatter `work_item_id: "42"` with `default_project_code = Some("ENG")` stores `"42"` raw, while the filename path stores `"ENG-42"`. `work_item_by_id` becomes dual-keyed and cross-refs silently fail to resolve. Mirror `canonicalise_refs` normalisation (`indexer.rs:653`) on the accepted frontmatter value.

- 🟡 **Correctness**: `work_item_id` borrow shape in back-fill loop is inconsistent
  **Location**: Phase 3 — back-fill `entry.linked_count = compute_linked_count(self, &clusters, &entry)`
  Taking `&entry` and then mutating `entry.linked_count` on the same binding doesn't borrow-check. The plan does not specify whether the iteration is over a snapshot or live entries, and the implementer will have to invent the two-pass shape.

- 🟡 **Code Quality**: Parallel-array between `has_*` booleans and `present.push` ladder
  **Location**: Phase 1 — `derive_completeness` snippet
  The eleven-arm `match` and the eleven-line `if c.has_x { c.present.push("x".into()); }` ladder are hand-synchronised. A future stage rename or insertion now touches at least three places (struct, match, push). `has_*` and `present` become near-redundant; consider deriving both from a single iteration.

- 🟡 **Test Coverage**: No test covers the connector active-iff-both-adjacent rule
  **Location**: Phase 6 — `Pipeline.test.tsx` assertion list
  The work item's Technical Notes treat connector colouring (`active && nextActive`) as load-bearing. The listed assertions only cover tile-level `data-active`. A mutation from `&&` to `||` or `i+1` to `i-1` would pass all listed tests.

- 🟡 **Test Coverage**: No test prevents raw `hsl(...)` literals from re-entering production
  **Location**: Phase 6/7 — component test assertion lists
  Decision 6 commits "the prototype's hard-coded HSL never lands in production." Listed assertions only check `data-active`; a regression inlining `hsl(0 68% 46%)` would still pass. Add a probe asserting the active tile's `style.cssText` contains `var(--ac-stage-` and does not contain `hsl(`.

- 🟡 **Test Coverage**: Missing test case in Phase 4 for filename also unresolvable
  **Location**: Phase 4 — test coverage subsection
  The four documented cases omit the fall-through where both frontmatter is absent AND the filename does not match `scan_regex`. This is the path that produces `entry.workItemId == null` and triggers AC-9's omit-slot rendering — it should be exercised server-side.

- 🟡 **Test Coverage**: No test enforces server `present` order matches frontend `WORKFLOW_PIPELINE_STEPS`
  **Location**: Phase 1 / Phase 6 — schema parity
  Decision 1 commits canonical-ordering parity, but the Rust test asserts a literal vector and the frontend uses Set-membership (order-insensitive). A reorder of one side drifts silently.

- 🟡 **Standards**: `-on` token suffix does not follow the `--ac-doc-*` precedent
  **Location**: Decision 6 / Phase 5 — token naming
  Existing tokens are `--ac-doc-<key>` (fg) + `--ac-doc-bg-<key>` (bg); there is no `-on/-off` convention anywhere. Either drop the `-on` suffix or document why this family adopts it.

- 🟡 **Standards**: `prototype-tokens.json` is theme-invariant-only — Phase 5 step 5 won't work
  **Location**: Phase 5, section 5 — snapshot fixture
  `prototype-tokens.json` only contains theme-invariant families (brand, code surface, syntax). The new stage tokens are theme-variant. Adding them will fail the parity test (`global.test.ts:207-222` describes "theme-invariant families"). Drop the fixture step.

- 🟡 **Standards, Usability**: `aria-label` on roleless `<span>` in PipelineMini not reliably announced
  **Location**: Phase 7 — PipelineMini root element
  `<span aria-label="Lifecycle pipeline">` with no role; per ARIA in HTML, `aria-label` only binds on elements with a name-from-author role. Use `<ol>`/`<ul>` to mirror `Pipeline` (and the existing `PipelineDots`), or add `role="img"`.

- 🟡 **Compatibility**: `migration.test.ts` EXCEPTIONS still reference PipelineDots after deletion
  **Location**: Phase 9 — Success Criteria (`rg PipelineDots`)
  Four EXCEPTIONS entries in `frontend/src/styles/migration.test.ts:70-74` reference `components/PipelineDots/PipelineDots.module.css`. The "EXCEPTIONS hygiene" test asserts each entry resolves to a real file — deleting the directory without updating the array goes red.

- 🟡 **Compatibility**: `parseWorkItemId` retention rationale is wrong
  **Location**: Desired End State / Migration Notes / What We're NOT Doing
  The plan says `parseWorkItemId` is retained for `announcements.ts`, but that file uses a different function (`workItemIdFromRelPath`). After Phase 10, `parseWorkItemId` has no production consumer — only its own test. Either delete it alongside Phase 10 or correct the rationale to name the actual consumer.

- 🟡 **Compatibility**: Existing corpus may silently shift displayed IDs
  **Location**: Phase 4 — frontmatter-first resolution behaviour change
  No corpus scan validates that no existing `meta/work/*.md` file has a parseable filename ID AND a different alpha-prefixed `work_item_id:`. The behaviour change should be guarded by a one-shot scan + Phase 4 success criterion.

- 🟡 **Usability**: `size: number` prop inconsistent with codebase conventions and forces off-grid Glyph
  **Location**: Phase 6 — Pipeline component API
  Glyph uses a literal union; Chip uses named variants. Only two call sites (26, 34). Replace with `variant?: 'card' | 'panel'`, each mapped to a tile size that lands on a Glyph-supported grid value. Eliminates the `as never` CSS-custom-property cast as a side effect.

#### Minor
- 🔵 **Architecture**: Server-phase ordering claim understates Phase 2's dependency on Phase 1
  **Location**: Implementation Approach
  "Server phases (1-4) can land in any order" is false — Phase 2/3 depend on Phase 1's `present` field. Adjust to "1→2→3 sequenced; 4 independent."

- 🔵 **Architecture**: Helper placement in `indexer.rs` blurs `indexer` ↔ `clusters` layering
  **Location**: Phase 3 — file placement
  Today `clusters → indexer`. Placing `compute_linked_count` in `indexer.rs` while it takes `&[LifecycleCluster]` reverses the dependency. Prefer `clusters.rs` or a new `related.rs`.

- 🔵 **Architecture**: Raw `DocTypeKey` kebab-case strings on the wire couple three concerns
  **Location**: Decision 1
  A doc-type rename now touches Rust enum, wire format, frontend constant, CSS token name, and prototype fixture. Acceptable but capture the renaming cost in Migration Notes.

- 🔵 **Architecture**: Orphan signal collapses two distinct conditions
  **Location**: Phase 10 — orphan detection
  `completeness == null` mixes "no cluster" with "slug derivation failed". Document the contract or distinguish.

- 🔵 **Architecture**: Per-entry `Completeness` clone duplicated N times within a cluster
  **Location**: Phase 2 — memory shape
  Wire payload size scales with cluster density. Performance section flags `Arc` for memory but the wire shape is locked. Document.

- 🔵 **Code Quality**: Connector colour computed twice (inline style + `data-active`)
  **Location**: Phase 6 — Pipeline component
  Pick one mechanism. Either drive all colour from CSS via `[data-active="true"]` + custom property, or all inline. Mixed is drift-prone.

- 🔵 **Code Quality**: `['--stage-size' as never]` hides a typing problem
  **Location**: Phase 6 — inline style
  Either widen `React.CSSProperties` once via ambient declaration or cast as `React.CSSProperties` on the whole style object — drop `as never`.

- 🔵 **Code Quality**: PipelineMini Overview hard-codes `hsl(hue 72% 56%)` while snippet uses token
  **Location**: Phase 7 — Overview vs Component sections
  Inconsistent guidance; drop the HSL formula and reference Decision 6.

- 🔵 **Code Quality**: `WORK_ITEM_ID_BASIC_SHAPE` introduces a second ID grammar
  **Location**: Phase 4 — validation regex
  The project already has `work.id_pattern`. Validate frontmatter via `WorkItemConfig` (extract `accept_id`) rather than a parallel ad-hoc shape.

- 🔵 **Code Quality**: Phase 5 hex starting values are approximations without a refinement rule
  **Location**: Phase 5 — token values
  Either compute final values now or state the rule (e.g. "reduce lightness by 4% iteratively until ≥3:1").

- 🔵 **Code Quality**: Phase 10 test description conflates "passthrough" with "zero-padded"
  **Location**: Phase 10 — WorkItemCard test list
  `formatDocId('0042')` returns `'0042'` verbatim — that is passthrough, not padding. Reword.

- 🔵 **Test Coverage**: Phase 1 long-tail inclusion not tested
  **Location**: Phase 1 — Success Criteria
  Only work-items + plans are exercised. Add `hasNotes=true, hasDesignGap=true` case.

- 🔵 **Test Coverage**: WorkItemCard test asserts PipelineMini presence but not data-active correctness
  **Location**: Phase 10 — WorkItemCard.test.tsx
  Add a probe that the dots inside `PipelineMini` reflect `entry.completeness.present`.

- 🔵 **Test Coverage**: Phase 2 back-fill test doesn't cover sharing across multiple entries
  **Location**: Phase 2 — Success Criteria
  Add assertion that all entries in a cluster receive identical `completeness`, and that entries from different clusters differ.

- 🔵 **Test Coverage**: Orphan-entry linkedCount path not explicitly tested
  **Location**: Phase 3 — Success Criteria
  AC-6 carves out orphans (`inferredCluster.length == 0`). Add an orphan test.

- 🔵 **Test Coverage**: DOM-order probe via `querySelectorAll('*')` is fragile
  **Location**: Phase 8 — LifecycleClusterView test
  Use `compareDocumentPosition` instead — robust to wrapper insertions.

- 🔵 **Correctness**: Connector colour uses current stage's accent, not next
  **Location**: Phase 6 — connector JSX
  Pin the rule explicitly and verify against work item's Technical Notes.

- 🔵 **Correctness**: `data-active` boolean serialisation depends on React's toString
  **Location**: Phase 6/7 — JSX + tests
  Standardise on `data-active={String(active)}` and `toHaveAttribute('data-active', 'true')`.

- 🔵 **Correctness**: Push order claim vs all-eleven-stage reality
  **Location**: Phase 1 — derive_completeness body
  Drive `present` from a single source of truth iteration; assert HashSet equality in tests rather than Vec equality.

- 🔵 **Standards**: `<ol>` vs `<ul>` choice unexplained vs PipelineDots precedent
  **Location**: Phase 6
  Document the rationale (lifecycle has a canonical progression order).

- 🔵 **Standards**: Singular/plural mismatch between `.ac-stagedot`/`.ac-stagedots` and `.ac-hexchain__stage`
  **Location**: Phases 6/7 — BEM hook naming
  Adopt consistent BEM (`.ac-stagedots__dot` and `.ac-hexchain__stage`).

- 🔵 **Standards**: `present` ordering is a fourth hand-maintained canonical order
  **Location**: Phase 1
  Drive from a single Rust-side ordering constant, or at minimum cross-reference `LIFECYCLE_PIPELINE_STEPS` in a comment.

- 🔵 **Standards**: Off-state colour smuggled into inline JS rather than CSS module
  **Location**: Phases 6/7
  Move off-state styling fully into CSS via `[data-active="false"]` selectors.

- 🔵 **Compatibility**: TypeScript type should accept `undefined` for mid-deploy compatibility
  **Location**: Phase 1/2 — `Completeness | null` types
  Older servers omit fields → JSON `undefined` not `null`. Either widen the type or normalise at the API client boundary.

- 🔵 **Compatibility**: Long-tail stages get no token but Glyph reuse implies a colour binding
  **Location**: Phase 5 — token set
  Either pre-define long-tail tokens or guard `Pipeline`/`PipelineMini` with an explicit allowlist.

- 🔵 **Usability**: `aria-label="Lifecycle pipeline"` adds screen-reader noise without state
  **Location**: Phase 6/7
  Either drop the label and rely on visible context, or compose meaningful text: `"Lifecycle pipeline, ${present.length} of 8 stages complete"`.

- 🔵 **Usability**: Hard-coded English eyebrow "Pipeline" has no i18n layer
  **Location**: Phase 8 — eyebrow text
  Acknowledge the localisation debt or accept the existing English-literal precedent explicitly.

- 🔵 **Usability**: `WORK_ITEM_ID_BASIC_SHAPE` rejects common namespacing (dots, multi-part)
  **Location**: Phase 4
  Widen the regex (e.g. `^[A-Za-z][A-Za-z0-9_.-]*[0-9]$`) or log a warning when frontmatter IDs are rejected.

- 🔵 **Usability**: Eight new stage tokens form parallel family to `--ac-doc-*` without documented relationship
  **Location**: Phase 5
  Add a CSS comment block explaining why these are separate, which surfaces consume which, and that long-tail intentionally has no stage token.

- 🔵 **Usability**: Shared styling/test hook (`data-active`) risks visual regressions passing tests
  **Location**: Phase 6/7
  Add at least one `getComputedStyle` probe per component.

- 🔵 **Usability**: `"N linked"` label is ambiguous without affordance
  **Location**: Phase 10 — WorkItemCard footer
  Either reword to "N related" / "N cross-refs", or add a tooltip with breakdown.

#### Suggestions
- 🔵 **Test Coverage**: Cross-surface test re-renders rather than exercising SSE invalidation
  **Location**: Testing Strategy
  Cite `use-doc-events.test.tsx` as the regression test for the invalidation path, or thin-assert that both `lifecycle()` and `docs('work-items')` are invalidated on `doc-changed`.

### Strengths
- ✅ Test-driven phasing with deterministic observables (`data-active`, DOM hooks) makes each phase independently verifiable.
- ✅ Server-side enrichment (`IndexEntry.completeness`, `IndexEntry.linkedCount`) is the right architectural choice — eliminates client-side joins on the kanban hot path with O(N) cost paid once.
- ✅ Decision 1's `DocTypeKey` kebab-case vocabulary matches `LIFECYCLE_PIPELINE_STEPS[i].docType` exactly — verified against `types.ts:212-242` and `clusters.rs`.
- ✅ Decisions (1-6) explicitly document divergence from established conventions; every tradeoff is acknowledged in writing.
- ✅ Phase 4 enumerates four resolution cases (frontmatter present / absent / empty / non-WorkItems) — a complete decision-table.
- ✅ Phase 5 reuses the established `--ac-doc-*` token testing pattern (≥3:1 contrast, CSS↔TS parity, MIRROR-A↔MIRROR-B parity).
- ✅ Phase boundaries designed for independent merge slices with green tests at every step; wire fields are additive.
- ✅ Test-fixture factories (`makeCompleteness`, `makeIndexEntry`) localise churn for the new wire fields.
- ✅ Migration Notes addresses mixed-deploy compatibility explicitly.
- ✅ Frontmatter shape regex is conservative — bare numerics fall through to filename extraction (the documented Phase 4 test case for `"0042"` notwithstanding — see the regex finding).
- ✅ `Completeness` derives `Clone` (verified at `clusters.rs:8`), so the planned `map.get(s).cloned()` back-fill is type-correct.
- ✅ SSE invalidation (`use-doc-events.ts:104-124`) confirms `doc-changed` fans out to `docs()`, `lifecycle()`, and `lifecycleClusterPrefix()` — the plan's cross-surface refresh claim holds.
- ✅ Explicit non-interactive contract in "What We're NOT Doing" + AC-12/AC-13 — no ambiguity.

### Recommended Changes

1. **Rewrite the `compute_linked_count` snippet** (addresses C1, C2, M3, multiple minors)
   Make the helper an `async fn` (or take pre-resolved snapshot inputs); apply the same inbound HashSet dedup and inferred-vs-declared drop as `related.rs:79-102`. Prefer extracting a single resolver that returns the three deduped lists; have both `related_get` and `compute_linked_count` consume it, so AC-6 is a tautology rather than a test invariant. Document the back-fill lock-ordering as a two-pass (snapshot read → write-lock apply) to avoid deadlock against `entries.read()` inside the called methods.

2. **Fix the `Pipeline` JSX Glyph invocation** (addresses C3)
   Rename `kind` → `docType`. Replace `size?: number` (and the `Math.round(size * 0.6)` derivation) with a named variant — e.g. `variant?: 'card' | 'panel'` defaulting to `'card'` — each mapping internally to a tile size that lands on a `Glyph`-supported grid value (16/24/32). This eliminates the off-grid problem, the `['--stage-size' as never]` cast, and aligns with Glyph/Chip conventions.

3. **Address the indexer dual-storage and watcher concurrency story** (addresses M1)
   Specify whether the back-fill is the canonical post-step inside `compute_clusters` (returning a `HashMap<PathBuf, (Completeness, usize)>` applied under one write lock), or whether `LifecycleCluster.entries` becomes `Vec<PathBuf>` with consumers resolving against `indexer.entries`. Add an explicit phase note covering `refresh_one` so single-file updates also refresh per-entry enrichment. Add a Rust test asserting that the cluster snapshot's and the indexer map's `completeness` agree on the same slug.

4. **Fix the frontmatter resolution: regex + normalisation + corpus scan** (addresses M2, M14, code-quality M_ID_grammar)
   Change `WORK_ITEM_ID_BASIC_SHAPE` to `^([A-Za-z]+-)?[0-9]+$` (prefix optional), trim whitespace, reject empty strings. Apply `default_project_code` normalisation to the accepted frontmatter value (or validate through `WorkItemConfig` directly). Add a one-shot pre-implementation corpus scan to confirm no existing file changes its displayed ID.

5. **Fix the three "additive" compatibility gaps** (addresses M10, M12, M13)
   - Drop Phase 5 step 5 (prototype-tokens.json snapshot extension) — the fixture is theme-invariant-only.
   - Add `frontend/src/styles/migration.test.ts:70-74` EXCEPTIONS-array updates to Phase 9 changes.
   - Either delete `parseWorkItemId` alongside Phase 10 or correct the retention rationale (`announcements.ts` uses `workItemIdFromRelPath`, not `parseWorkItemId`).

6. **Add the missing test assertions** (addresses M5, M6, M7, M8)
   - `Pipeline.test.tsx`: connector active-iff-both-adjacent rule (non-adjacent stages → no active connector; adjacent → exactly one active).
   - `Pipeline.test.tsx` + `PipelineMini.test.tsx`: active style.cssText contains `var(--ac-stage-` and not `hsl(`.
   - Phase 4: add filename-also-unresolvable test → `entry.work_item_id == None`.
   - Phase 1: add Rust test comparing `present` order against a literal vector matching frontend `WORKFLOW_PIPELINE_STEPS` + long-tail tail.

7. **Refine the token naming and Phase 1 derivation** (addresses M4, M9, multiple minors)
   - Drop the `-on` suffix or document the convention. If kept, add an explicit note in `global.css`.
   - Drive `derive_completeness` push order from a single ordering constant (or extend `canonical_rank`). At minimum, cross-reference `LIFECYCLE_PIPELINE_STEPS` in a code comment.

8. **Fix the accessibility regression in PipelineMini** (addresses M11)
   Switch `<span aria-label="Lifecycle pipeline">` → `<ol aria-label="…">` (or `<ul>`) to mirror `Pipeline` and the existing `PipelineDots`. Optionally compose a meaningful label: `aria-label={`Lifecycle pipeline, ${present.length} of 8 stages complete`}`.

9. **Tighten the Implementation Approach phase-ordering claim**
   Adjust "Server phases (1-4) can land in any order" → "Phases 1→2→3 are sequenced; Phase 4 and Phase 5 are independent of those and of each other." Add explicit `Depends on:` lines on Phases 2 and 3.

10. **Minor follow-ups (batched)**
    Apply the minor findings as a single pass: consistent BEM hook naming, off-state colour in CSS not inline, `String(active)` for `data-active`, `compareDocumentPosition` for DOM-order, Phase 10 test rewording, long-tail token allowlist guard, undefined-tolerant TypeScript types at the API boundary, and accessibility-label semantics.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound at the level of component
boundaries and the tradeoffs (BEM local exception, per-stage tokens
folded in, raw kebab-case vocabulary on the wire) are explicitly
acknowledged. Three structural concerns deserve attention: the
`compute_linked_count` helper's signature is incompatible with the
async-locked indexer and silently diverges from `related_get`'s dedup;
back-filling enriched fields onto `IndexEntry` creates dual storage
(canonical entries map vs cluster-entry clones) with no story for
keeping them coherent across `refresh_one`/SSE paths; and the
phase-parallelism claim for server phases 1-4 isn't quite right given
Phase 2 depends on Phase 1's `present` field.

**Strengths**: clean component separation; server-side enrichment is
the correct architectural choice; shared resolver is the correct
anti-drift design (in intent); Decisions document divergence
explicitly; phase boundaries designed for independent merge; frontmatter
resolution at `build_entry` not the API layer; Glyph reuse keyed on
`DocTypeKey`.

**Findings**: 2 major (helper async/dedup, dual storage), 6 minor
(phase ordering claim, helper placement, vocabulary coupling, orphan
signal, per-entry clone duplication, frontmatter regex grammar mismatch).

### Code Quality

**Summary**: Unusually thorough on test assertions, naming, file
locations, and per-phase merge boundaries. Concrete code snippets
contain quality issues: incorrect Glyph prop and out-of-union size;
connector colour computed twice; parallel-array maintenance between
booleans and `present.push` ladder; the `compute_linked_count`
extraction leaves a parallel implementation in place.

**Strengths**: TDD with deterministic observables; shared helper is
the right abstraction; test-fixture factories; Decision 1 vocabulary
alignment; BEM hooks as additive class names; design-token elevation
out of inline HSL; pragmatic `parseWorkItemId` retention.

**Findings**: 3 major (Glyph prop, shared-resolver duplication,
parallel-array), 8 minor (connector duplication, `as never` cast,
PipelineMini HSL inconsistency, second ID grammar, payload clone bloat,
async borrow checker, hex starting-value refinement rule, Phase 10 test
wording).

### Test Coverage

**Summary**: Explicitly TDD with strong test surfaces and a smart
cross-surface integration probe. Gaps: no connector colouring rule
assertion (load-bearing per work item); no `hsl(...)` regression guard
on shipped output; missing Phase 4 fallback case; no cross-language
ordering parity test.

**Strengths**: deterministic observables; AC-6 endpoint equality
assertion; Phase 4 four-case decision table; reuses `--ac-doc-*`
testing pattern (contrast, parity, drift); cross-surface state-change
test; centralised fixtures; PipelineDots-removal mechanical
verification.

**Findings**: 4 major (connector rule, HSL regression guard, Phase 4
fallback, cross-language ordering parity), 5 minor (long-tail
inclusion, WorkItemCard data-active, multi-entry back-fill share,
orphan linkedCount, DOM-order fragility), 1 suggestion (SSE invalidation
test citation).

### Correctness

**Summary**: Core data model is sound and the kebab-case `present`
vocabulary aligns exactly with `LIFECYCLE_PIPELINE_STEPS[i].docType`.
However, `compute_linked_count` has multiple correctness problems
(skips `related_get`'s dedup, calls async without `.await`, borrow
inconsistencies), and the frontmatter-first `WORK_ITEM_ID_BASIC_SHAPE`
contradicts the plan's prose by rejecting bare digits.

**Strengths**: vocabulary alignment with frontend; `Completeness`
derives `Clone`; components re-derive `Set` on every render;
SSE invalidation is comprehensive; orphan→null serialisation gives a
clean predicate.

**Findings**: 2 critical (compute_linked_count dedup, async without
await), 4 major (regex rejects bare digits, no default_project_code
normalisation, borrow shape, watcher concurrency), 4 minor (inferred
dedup duplicate-of-critical, connector colour rule, data-active
serialisation, push-order all-eleven-stage scope).

### Standards

**Summary**: Largely follows established visualiser conventions
(kebab-case DocTypeKey wire strings, three-mirror token pattern,
aria-label on pipeline container) and documents the new `.ac-*` BEM
hook as a deliberate local exception. Concrete divergences: Glyph
prop misquoted, `-on` token suffix doesn't follow `--ac-doc-*`,
prototype-tokens.json is theme-invariant-only, `<span aria-label>` is
roleless.

**Strengths**: vocabulary alignment with `DocTypeKey` serialisation;
BEM hook scope is explicitly local; three-mirror token discipline
preserved; aria-label on container preserves PipelineDots convention;
inline custom property pattern matches existing precedent.

**Findings**: 4 major (Glyph prop name, -on suffix, prototype fixture
mismatch, span aria-label), 4 minor (ol vs ul rationale, BEM
singular/plural mismatch, present hand-maintained order, off-state
colour inline).

### Compatibility

**Summary**: Frames wire-shape additions as additive and notes the
mixed-deploy fallback, but several risks are unaddressed. Pipeline
won't compile against the existing Glyph API; the proposed
`compute_linked_count` contradicts `related_get`'s dedup; `parseWorkItemId`
retention rationale is wrong; corpus scan for frontmatter-first
behaviour change is missing.

**Strengths**: Migration Notes addresses mixed-deploy; conservative
frontmatter regex; camelCase via existing serde; valid CSS token
identifiers; deliberate vocabulary reuse.

**Findings**: 2 critical (Glyph prop, compute_linked_count dedup),
3 major (migration.test.ts EXCEPTIONS, parseWorkItemId rationale,
corpus scan), 2 minor (undefined vs null TypeScript, long-tail token
allowlist).

### Usability

**Summary**: Clean two-component split with a well-chosen primary
prop (`completeness`) and a single canonical observable. Developer-
facing API friction points: free-form numeric `size` prop conflicts
with Glyph/Chip conventions; `--stage-size` inline `as never` cast;
Glyph `kind` prop name wrong. End-user usability also flags screen-reader
noise, hard-coded English label, and an ambiguous "N linked" label.

**Strengths**: single observable signal; minimal prop surface; vocabulary
alignment; client-side join eliminated; centralised fixtures; explicit
non-interactive contract.

**Findings**: 2 major (size prop convention, Glyph kind prop), 7
minor (as never cast, aria-label noise, i18n debt, ID regex narrowness,
parallel token family, data-active dual hook risk, "N linked"
ambiguity).

## Re-Review (Pass 2) — 2026-06-01

**Verdict:** REVISE

Substantively much improved. All three pass-1 critical findings are
resolved by structural changes (shared `resolve_related` makes AC-6
tautological, Glyph prop name + grid-aligned sizing, async helper with
two-pass back-fill). All major pass-1 findings are resolved or
documented as deliberate tradeoffs. However, three new major
correctness concerns surfaced during re-review — two are clean-up
items I missed during editing (a stale regex in Decision 3 that
contradicts the updated Phase 4 snippet, and a stray `--ac-stage-*-on`
selector left in Phase 5's contrast-test bullet), and one is a real
design gap (the Phase 2 commitment that `refresh_one` triggers a full
cluster recompute is asserted but the call-site wiring is not
specified; the rescan_lock gating across the two-pass back-fill is
also unstated). Once these are addressed the plan is implementation-
ready.

### Previously Identified Issues

#### Critical (pass 1) — all resolved
- 🟢 **Correctness, Compatibility**: `compute_linked_count` dedup divergence — **Resolved**
  via shared `resolve_related` returning the same three deduped lists `related_get` serialises.
- 🟢 **Correctness**: Async-without-await — **Resolved**: helper is `async`,
  two-pass back-fill (snapshot read → write-lock apply) documented.
- 🟢 **Code Quality, Standards, Compatibility, Usability**: Glyph prop / off-grid size —
  **Resolved**: `docType` prop; `Record<PipelineVariant, 16 | 24>` map; variant API.

#### Major (pass 1)
- 🟢 **Architecture, Correctness**: Dual storage / watcher concurrency — **Resolved**
  via `compute_clusters` returning a back-fill map applied under single write lock;
  `refresh_one` path explicitly addressed (but see new finding below on wiring).
- 🟢 **Correctness**: Bare-digit regex contradiction — **Resolved** in Phase 4 snippet
  (but introduced new contradiction with Decision 3 — see new findings).
- 🟢 **Correctness**: Borrow-shape inconsistency — **Resolved** by two-pass shape.
- 🟢 **Correctness**: default_project_code normalisation bypass — **Resolved**
  via `WorkItemConfig::normalise_id`.
- 🟢 **Code Quality**: Parallel-array (booleans + push ladder) — **Resolved**
  via `STAGE_PUSH_ORDER` const table.
- 🟢 **Test Coverage**: Connector active-iff-both-adjacent rule — **Resolved**
  (adjacent + non-adjacent assertions added).
- 🟢 **Test Coverage**: HSL regression guard — **Resolved** (positive + negative
  cssText assertions in Pipeline + PipelineMini).
- 🟢 **Test Coverage**: Phase 4 fallback case — **Resolved**.
- 🟢 **Test Coverage**: Cross-language ordering parity — **Resolved**
  (Rust literal + frontend parity test).
- 🟢 **Standards**: `-on` token suffix — **Resolved** (dropped; matches `--ac-doc-*`).
- 🟢 **Standards**: prototype-tokens.json mismatch — **Resolved** (step removed).
- 🟢 **Standards, Usability**: PipelineMini aria-label binding — **Resolved**
  (now `<ol>` root).
- 🟢 **Compatibility**: migration.test.ts EXCEPTIONS — **Resolved**
  (Phase 9 step 4 added).
- 🟢 **Compatibility**: parseWorkItemId retention rationale — **Resolved** (deleted;
  rationale corrected in three places).
- 🟢 **Compatibility**: Corpus scan — **Resolved** (added to Phase 4 success criteria).
- 🟢 **Usability**: size:number prop — **Resolved** via variant API.

#### Minor (pass 1) — sample
- 🟢 Server-phase ordering claim — **Resolved**.
- 🟢 Helper placement (`indexer.rs`) — **Resolved** (explicit "do not place in
  indexer.rs"; the indexer-vs-api/related.rs choice still open — see new finding).
- 🟢 `as never` cast — **Resolved** via `[data-variant]` selector in CSS.
- 🟢 `data-active` boolean serialisation — **Resolved** via `String(active)`.
- 🟢 DOM-order probe — **Resolved** via `compareDocumentPosition`.
- 🟢 PipelineMini hard-coded HSL — **Resolved** via shared token.
- 🟢 Phase 10 test wording — **Resolved**.
- 🟢 Phase 1 long-tail inclusion test — **Resolved**.
- 🟢 Multi-entry sharing test, orphan linkedCount test, refresh_one coherence test — **Resolved**.
- 🟢 Stage-token parallel family documented — **Resolved** (three places).
- 🟢 aria-label meaningful — **Resolved**.
- 🟡 English-only eyebrow "Pipeline" — **Still present** (not in scope for this story per
  consistent visualiser precedent; flag for future i18n pass).
- 🟡 `WORK_ITEM_ID_BASIC_SHAPE` regex narrowness — **Still present** (deliberate;
  prefer adding indexer `warn!` log for rejected values rather than widening).
- 🟡 "N linked" label ambiguity — **Still present** (polish concern; could add
  `title` attribute as one-line fix).

### New Issues Introduced

#### Major
- 🟡 **Correctness, Standards, Compatibility**: Decision 3 regex contradicts Phase 4 snippet
  **Location**: Key Decisions §3 (line 130) vs Phase 4 implementation (line 625)
  Decision 3 still reads `^[A-Za-z]+-?[0-9]+$` (old form) while the Phase 4
  snippet specifies `^([A-Za-z]+-)?[0-9]+$` (new form). The two differ in
  admission: the old form rejects bare digits and accepts `ENG0042`; the new
  form does the opposite. An implementer following Decision 3 as the
  normative spec will encode the wrong regex. Fix: update Decision 3 to
  match Phase 4.

- 🟡 **Correctness**: `refresh_one` cluster recompute is asserted but not wired into call sites
  **Location**: Phase 2 — Cluster pass back-fill (refresh_one paragraph)
  Phase 2 commits `refresh_one` to a full cluster recompute, but
  `Indexer::refresh_one` has no access to `state.clusters`. The plan does
  not describe how the recompute is invoked at the two call sites outside
  the watcher (`api/docs.rs:235` after a kanban status edit, and the test
  harness). Without wiring, a kanban status edit leaves `completeness` /
  `linkedCount` stale despite Phase 2's coherence test passing in
  isolation. Fix: add a sub-bullet specifying `refresh_one`'s new signature
  (or an `AppState::refresh_one_and_recompute` wrapper) and the
  `api/docs.rs:235` refactor.

- 🟡 **Correctness**: Two-pass back-fill does not specify `rescan_lock` gating
  **Location**: Phase 3 — Two-pass back-fill
  The plan documents the snapshot-read → write-lock apply pattern but
  never mentions the existing `Indexer::rescan_lock` semaphore. Without
  that gate across both passes, a concurrent `refresh_one` between
  Pass 1 and Pass 2 can produce stale writes or assignments to deleted
  entries. Fix: document that the post-cluster pipeline holds `rescan_lock`
  for the duration of compute_clusters + Pass 1 + Pass 2.

#### Minor
- 🔵 **Standards, Compatibility**: Stray `--ac-stage-*-on` selector in Phase 5 contrast-test bullet
  **Location**: Phase 5, Contrast + parity test coverage
  Decision 6 drops the `-on` suffix everywhere except this one stale
  reference. Update to `--ac-stage-*`.

- 🔵 **Standards**: `.ac-pipeline-panel` / `.ac-eyebrow` deviate from BEM block__element
  **Location**: Phase 8 — panel JSX
  Decision 2 commits all new hooks to block__element form. These two are
  flat. Rename to `.ac-lcluster__pipeline-panel` and
  `.ac-lcluster__pipeline-eyebrow` (or pick a parent block name).

- 🔵 **Test Coverage, Compatibility**: `[data-glyph-size]` attribute doesn't exist on Glyph
  **Location**: Phase 6 variant test — Glyph size probe
  The test list offers `[data-glyph-size]` as one option, but Glyph emits
  no such attribute. Drop the option; standardise on
  `expect(svg).toHaveAttribute('width', '24')`.

- 🔵 **Correctness**: `normalise_id` cross-prefix behaviour unspecified
  **Location**: Phase 4 — normalise_id helper
  Test cases cover bare-numeric and matching-prefix but not cross-prefix
  (`'OPS-7'` with `default_project_code = Some('ENG')`). The multi-prefix
  intent suggests passthrough; pin it with a test.

- 🔵 **Correctness, Code Quality**: Linked-count helper named two different ways
  **Location**: Phase 3 (`count_from_resolution`) vs Verification/Testing Strategy (`compute_linked_count`)
  Pick one name and update references.

- 🔵 **Architecture**: `resolve_related` placement choice deferred
  **Location**: Phase 3 — file placement
  Plan offers `api/related.rs` OR `server/src/related.rs` sibling. The
  sibling is the correct choice (preserves `api → domain` direction); pick
  it explicitly.

- 🔵 **Architecture**: TOCTOU window between Pass 1 read snapshot and Pass 2 write apply
  **Location**: Phase 3 — two-pass back-fill
  Document explicitly that the contract is point-in-time at write-apply
  commit, not steady-state (the next watcher cycle catches up). Related to
  the rescan_lock finding above — fixing one resolves most of the other.

- 🔵 **Architecture**: `refresh_one` now triggers O(N) cluster recompute on every file event
  **Location**: Phase 2 — refresh_one note
  Acknowledge the watcher's scaling profile change and note that
  debounce/coalescing is a separate concern.

- 🔵 **Code Quality**: `WORK_ITEM_ID_BASIC_SHAPE` regex still a second grammar
  **Location**: Phase 4 — frontmatter resolution
  The shape check is now used only as a pre-filter, but the regex is
  still defined outside `WorkItemConfig`. Consider folding the shape check
  into `normalise_id` so a single helper owns both validity and canonical
  formatting.

- 🔵 **Code Quality**: Shared resolver returns full `Vec<IndexEntry>` even for count-only callers
  **Location**: Phase 3 — `resolve_related` signature
  The back-fill clones N×3 IndexEntry records to compute three integers.
  Consider a lighter-weight `resolve_related_paths` returning
  `Vec<PathBuf>` triples consumed by the back-fill, with `resolve_related`
  layered on top for the endpoint. Optional.

- 🔵 **Code Quality**: `STAGE_PUSH_ORDER` fn-pointer pattern unusual in Rust
  **Location**: Phase 1 — derive_completeness const table
  Optional readability improvement: convert to an inherent method
  `Completeness::stages_in_canonical_order()` that pattern-matches each
  field internally.

- 🔵 **Standards**: `ac-hexchain` block name implies hexagonal geometry the production
  component does not carry
  **Location**: Decision 2 + Phase 6
  Consider renaming to `ac-stagechain` to match `ac-stagedots` and reflect
  what the component renders rather than its prototype shape.

- 🔵 **Test Coverage**: Cross-language ordering parity not pinned to a named frontend test file
  **Location**: Phase 1 success criteria
  Pin the frontend parity test to a specific file (e.g.
  `pipeline-step-parity.test.ts`) with the same literal.

- 🔵 **Usability**: `PipelineMini` lacks the variant axis `Pipeline` gained
  **Location**: Phase 7 — PipelineMini API
  Mild asymmetry; either note the deferral explicitly or pre-emptively add
  a `density?: 'compact'` axis. Low priority.

### Assessment

The plan has materially improved. All critical findings and the
substantive majors are resolved. The three new majors are:

1. **Two clean-up references I missed during editing** (Decision 3 regex
   vs Phase 4 snippet; stray `--ac-stage-*-on` in Phase 5) — trivial to
   fix, one Edit each.
2. **Two real design gaps** (`refresh_one` wiring at `api/docs.rs:235`;
   `rescan_lock` gating across two-pass back-fill) — solvable with two
   short sub-bullets in Phase 2/3.

Once these are addressed, the plan is implementation-ready. The minor
findings are polish items the team can apply alongside the major fixes
or defer to follow-up. Verdict stays REVISE because of the three new
majors, but the next pass should converge quickly.

---

## Pass 3 — 2026-06-01 (fixes applied)

**Verdict:** APPROVE

All three pass-2 majors and the substantive minors were applied without
re-running the lens agents (the fixes are narrow, mechanical, and target
named lines):

### Pass-2 majors — resolved

- 🟢 **Decision 3 vs Phase 4 regex contradiction** — Decision 3 rewritten
  to defer to Phase 4 for the canonical regex; both now read
  `^([A-Za-z]+-)?[0-9]+$` (accepts bare digits + prefix-dash-digits,
  rejects prefix-without-dash). Decision 4 also tightened to reference
  the shared `resolve_related` helper rather than the pre-edit "no dedup"
  text that had drifted.
- 🟢 **`refresh_one` cluster recompute wiring** — Phase 2 now specifies a
  new `AppState::refresh_one_and_recompute(&path)` wrapper, names both
  callers to update (`api/docs.rs:235` and `watcher.rs`), and documents
  the lock order. The O(N)-per-event scaling change is acknowledged.
- 🟢 **`rescan_lock` gating** — Phase 3 now states the entire post-cluster
  pipeline holds `rescan_lock` for its duration; Pass 2 also guards each
  assignment with a "path still present" check. Consistency contract
  pinned as point-in-time at write-apply commit.

### Pass-2 minors — resolved

- 🟢 Stray `--ac-stage-*-on` in Phase 5 contrast bullet → `--ac-stage-*`.
- 🟢 `[data-glyph-size]` test option dropped → `expect(svg).toHaveAttribute('width', …)`.
- 🟢 `.ac-pipeline-panel` / `.ac-eyebrow` → `.ac-lcluster__pipeline` /
  `.ac-lcluster__pipeline-eyebrow` (BEM block__element).
- 🟢 Resolver placement → `server/src/related.rs` sibling (explicit).
- 🟢 Helper name unified → `resolve_related` + `count_from_resolution`
  everywhere (Verification, Testing Strategy, Performance Considerations
  all updated).
- 🟢 Cross-language parity pinned to
  `frontend/src/api/pipeline-step-parity.test.ts`.
- 🟢 `WORK_ITEM_ID_BASIC_SHAPE` folded into `WorkItemConfig::normalise_id`
  — no separate regex; one helper owns validity AND canonical form.
- 🟢 Cross-prefix passthrough test added (`"OPS-7"` with
  `default_project_code = Some("ENG")` → `"OPS-7"`).
- 🟢 `warn!` log on shape-invalid frontmatter values (addresses pass-1
  usability "silent fallback is worst error mode" concern).
- 🟢 `PipelineMini` single-variant intent documented inline.
- 🟢 `ac-hexchain` renamed to `ac-stagechain` (matches `ac-stagedots`,
  reflects what the component renders rather than its prototype shape).
- 🟢 TOCTOU window documented (Phase 3 consistency contract).
- 🟢 `refresh_one` O(N) scaling profile acknowledged (Phase 2 scaling note).

### Deferred (intentional)

- 🟡 English-only eyebrow "Pipeline" — no i18n precedent in the
  visualiser; consistent with surrounding code, deferred to a future
  i18n pass.
- 🟡 "N linked" label ambiguity — polish-grade; could add a `title`
  attribute later. Not blocking.
- 🔵 `STAGE_PUSH_ORDER` fn-pointer style — table form is clear enough;
  the inherent-method refactor is an optional readability tweak.
- 🔵 `resolve_related` returning full `IndexEntry` vectors — clones N×3
  records to compute three integers. The "shared resolver" framing
  benefits from this for the endpoint; a `resolve_related_paths`
  variant for the count path is a follow-up if profiling shows
  measurable cost.

### Assessment

Plan is implementation-ready. Critical resolution path (`resolve_related`
+ `count_from_resolution`) makes AC-6 a tautology; back-fill coherence
across `Indexer::entries`, `LifecycleCluster.entries`, and
`refresh_one` is documented end-to-end with lock-ordering and
gating spelled out; `WorkItemConfig::normalise_id` owns both shape
validity and canonical formatting with explicit multi-prefix support;
BEM hooks consistent (`block__element` throughout); token family
relationship documented in code, decisions, and migration notes;
component APIs symmetric where it matters (`Pipeline` variant axis)
and asymmetric where deliberate (`PipelineMini` single-variant).

---
*Pass 3 generated by /review-plan*
