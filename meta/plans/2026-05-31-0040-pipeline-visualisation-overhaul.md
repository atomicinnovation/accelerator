---
date: "2026-05-31T23:30:00+01:00"
type: plan
producer: create-plan
work_item_id: "0040"
status: done
id: "2026-05-31-0040-pipeline-visualisation-overhaul"
title: "Pipeline Visualisation Overhaul Implementation Plan"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-31T23:30:00+01:00"
last_updated_by: Toby Clemson
revision: "75809591931b"
repository: "ticket-management"
relates_to: ["work-item:0040", "codebase-research:2026-05-31-0040-pipeline-visualisation-overhaul", "adr:ADR-0025", "adr:ADR-0033", "adr:ADR-0034"]
---

# Pipeline Visualisation Overhaul Implementation Plan

## Overview

Replace the single-surface `PipelineDots` widget with two design-independent
visualisation components — a labelled `Pipeline` and a compact `PipelineMini` —
and surface lifecycle pipeline progress on three places that show a cluster or
work item: lifecycle cluster detail, lifecycle index cards, and kanban work
item cards. Server-side, enrich `IndexEntry` with per-entry `completeness`
and `linkedCount` so kanban cards render their own cluster's pipeline state
and an "N linked" footer without client-side joins or per-card HTTP requests.

All paths in this plan are relative to the visualiser skill root
`skills/visualisation/visualise/` unless otherwise noted.

## Current State Analysis

- `frontend/src/components/PipelineDots/PipelineDots.tsx` is a single-consumer
  widget rendering eight dots, used only on
  `frontend/src/routes/lifecycle/LifecycleIndex.tsx:99-119`.
- `frontend/src/routes/lifecycle/LifecycleClusterView.tsx` has no pipeline
  visualisation above its timeline; `cluster.completeness` is already in
  scope but unused for any chain rendering.
- `frontend/src/routes/kanban/WorkItemCard.tsx` renders ID via the legacy
  `parseWorkItemId(entry.relPath)` filename regex, has no pipeline row,
  no relation count, and stuffs mtime inside `.cardHeader` (no `__foot`).
- `server/src/clusters.rs:10-22` defines `Completeness` with eleven `has_*`
  booleans and no `present: Vec<String>` field.
- `server/src/indexer.rs:161-180` (the `IndexEntry` struct) carries no
  `completeness` and no `linked_count` field.
- `server/src/api/related.rs:31-110` (the `/api/related/{path}` endpoint)
  builds `RelatedArtifactsResponse` inline; the three resolution helpers it
  uses (`Indexer::declared_outbound`, `Indexer::reviews_by_target`,
  `Indexer::work_item_refs_by_id`, plus the inline `inferredCluster`
  derivation) are pure hashmap lookups against indexes the indexer already
  maintains.
- `server/src/config.rs:111-128` (`WorkItemConfig::extract_id`) extracts the
  work item ID from the filename only; frontmatter `work_item_id` is never
  consulted.
- `formatDocId` (`frontend/src/routes/library/doc-type-id.ts:1-12`) and
  `entry.workItemId` (`frontend/src/api/types.ts:85`) are already in place
  — `LibraryTypeView` is the existing precedent.
- Class-name convention today is CSS-module camelCase exclusively; the BEM
  hooks named in the work item's Acceptance Criteria (`.ac-kcard__top`,
  `.ac-kcard__foot`, `.ac-lcard__pipe`) do not exist in production yet and
  must be introduced as stable test/AC hooks.

## Desired End State

After this plan is complete:

- Lifecycle cluster detail (`/lifecycle/{slug}`) renders a labelled `Pipeline`
  panel between the back link and the existing timeline.
- Lifecycle index cards (`/lifecycle`) render a labelled `Pipeline` plus an
  N/8 counter in a dedicated `.ac-lcard__pipe` row, replacing `PipelineDots`.
- Kanban cards render a compact `PipelineMini` as the first child of
  `.ac-kcard__top` for non-orphan work items, render namespaced IDs via
  `formatDocId(entry.workItemId)`, and render an "{N} linked" label in a new
  `.ac-kcard__foot` alongside mtime when `entry.linkedCount > 0`.
- The server's `Completeness` carries a first-class `present: Vec<String>`
  field whose elements are `DocTypeKey` kebab-case strings (e.g. `work-items`,
  `research`, `plans`, …); `IndexEntry` carries per-entry `completeness:
  Option<Completeness>` (null on orphan entries) and `linkedCount: usize`
  derived from a shared function that also backs `GET /api/related/{path}`.
- `parseWorkItemId(entry.relPath)` is deleted alongside Phase 10's
  WorkItemCard rewrite — no production consumer remains after the
  refactor (the original "kept for `announcements.ts`" rationale was
  wrong; `announcements.ts` uses `workItemIdFromRelPath`).
- `PipelineDots` is deleted, its lone consumer is migrated, and its tests
  are split into focused `Pipeline.test.tsx` and `PipelineMini.test.tsx`.

### Verification

- All twelve Acceptance Criteria in `meta/work/0040-pipeline-visualisation-overhaul.md`
  pass against the implemented surfaces.
- Server: `cargo test` passes; new tests cover `Completeness.present`
  derivation, per-entry `IndexEntry.completeness` backfill, the
  shared `resolve_related` + `count_from_resolution` helpers, and the
  frontmatter-first
  `workItemId` resolution path.
- Frontend: `npm run typecheck` and `npm test` pass; new tests cover the
  per-surface `data-active` assertions enumerated in the work item's
  Technical Notes (`Pipeline.test.tsx`, `PipelineMini.test.tsx`,
  `WorkItemCard.test.tsx`, `LifecycleIndex.test.tsx`,
  `LifecycleClusterView.test.tsx`).
- Manual: a freshly seeded workspace renders all three surfaces; an SSE
  update that flips a cluster's stage flag propagates to all three surfaces
  on next refetch (the SSE invalidation in `api/use-doc-events.ts:107-108`
  already invalidates both `lifecycle()` and `docs('work-items')`).

### Key Decisions (locked in for implementation)

These resolve the open questions in the research document
(`meta/research/codebase/2026-05-31-0040-pipeline-visualisation-overhaul.md`,
Open Questions §1-6). They are not open during implementation.

1. **`Completeness.present` element vocabulary**: server emits **`DocTypeKey`
   kebab-case strings** verbatim (e.g. `"work-items"`, `"research"`,
   `"plans"`, `"plan-reviews"`, `"validations"`, `"pr-descriptions"`,
   `"pr-reviews"`, `"decisions"`, `"notes"`, `"design-inventories"`,
   `"design-gaps"`). Rationale: zero new vocabulary, directly mirrors
   `IndexEntry.type` serialisation, matches `LIFECYCLE_PIPELINE_STEPS[i].docType`
   on the frontend so `Pipeline` matches stage→present via that field.
   `present` lists all stages whose boolean is true, including long-tail;
   the `Pipeline` component filters to `WORKFLOW_PIPELINE_STEPS` for
   rendering.
2. **Class-name convention**: introduce BEM `.ac-*` class hooks **as additional
   class names** alongside existing CSS modules in the JSX
   (`className={`${styles.cardTop} ac-kcard__top`}`). The `.ac-*` hooks are
   stable AC/test selectors with no styling attached — all styling stays in
   the CSS modules. New components (`Pipeline`, `PipelineMini`) use the
   same pattern: a CSS-module file plus consistent BEM hooks —
   `.ac-stagechain` / `.ac-stagechain__stage` for Pipeline, and
   `.ac-stagedots` / `.ac-stagedots__dot` for PipelineMini (both blocks
   use the explicit `block__element` form, matching the AC-mandated
   `.ac-kcard__top` style). This avoids a project-wide convention shift
   while satisfying the AC text verbatim.
3. **`workItemId` source for AC-9 (frontmatter-first)**: extend `build_entry`
   in `indexer.rs` to consult `frontmatter["work_item_id"]` first when the
   entry's doc type is `WorkItems`; fall back to
   `WorkItemConfig::extract_id(filename)` only when frontmatter does not
   supply a usable string. The frontmatter value is trimmed,
   empty-string-rejected, validated against the basic shape
   `^([A-Za-z]+-)?[0-9]+$` (accepts bare digits `0042` and prefix-dash-digits
   `ENG-0042`; rejects prefix-without-dash `ENG0042`), then funnelled
   through `WorkItemConfig::normalise_id` so the canonical form matches
   the filename path. Phase 4 owns the canonical regex and helper; this
   decision summary defers to that snippet on the exact shape.
4. **`linkedCount` semantics**: `linkedCount` equals the sum of the three
   array lengths returned by `/api/related/{path}` — that is, **after**
   the endpoint's two dedup passes (inbound HashSet merge between
   `reviews_by_target` and `work_item_refs_by_id`; inferred-vs-declared
   drop). Both the field and the endpoint are produced from a single
   shared `resolve_related` helper (Phase 3), so AC-6's equality is a
   construction tautology rather than a parallel-implementation invariant.
   If a future typed-cross-linking change under 0057/0061 alters the
   `/api/related` shape, both consumers update together.
5. **Pipeline tile glyph**: reuse the existing `Glyph` component
   (`frontend/src/components/Glyph/Glyph.tsx`) keyed off
   `LIFECYCLE_PIPELINE_STEPS[i].docType`. The existing per-`DocTypeKey`
   icon set already covers every workflow stage.
6. **Per-stage hues**: introduce `--ac-stage-<docType>` design tokens
   now (eight, one per workflow stage), following the established
   `--ac-doc-*` naming precedent (no `-on/-off` suffix). The mirror
   pattern is MIRROR-A light + MIRROR-A dark in `global.css`, MIRROR-B
   mirror in `@media (prefers-color-scheme: dark)`, resolved-hex mirror
   in `tokens.ts`, parity + ≥3:1 contrast tests in `global.test.ts`.
   The stage tokens are theme-variant and therefore are **not** added
   to `prototype-tokens.json` (which covers only theme-invariant brand,
   code-surface, and syntax families). Both `Pipeline` and
   `PipelineMini` consume the same token per stage via
   `var(--ac-stage-${docType})`. The prototype's subtle chain-vs-dot
   lightness difference (`46%` vs `56%`) is collapsed to a single value
   per stage; off-state stays `var(--ac-stroke)`. This absorbs the
   work-item-flagged 0033 follow-up into this story so the prototype's
   hard-coded HSL never lands in production. The `--ac-stage-*` family
   is intentionally distinct from `--ac-doc-*` (chain hues vs per-type
   badge hues); the relationship is documented inline in `global.css`.

## What We're NOT Doing

- Making `Pipeline` interactive (no click-to-scroll, no link, no focus) —
  explicitly deferred in the work item.
- Changing the `LONG_TAIL_PIPELINE_STEPS` rendering in the "Other artifacts"
  section of `LifecycleClusterView` — unchanged.
- Refactoring the kanban announcement helper `workItemIdFromRelPath`
  (used by `announcements.ts` for screen-reader text). `parseWorkItemId`
  is a different helper and is deleted in Phase 10 as dead code.
- Deduping declared outbound vs declared inbound — see Decision 4.
- Migrating other surfaces (library/detail/etc.) to BEM `.ac-*` class
  hooks — local exception only on the surfaces touched here.
- Adding any client-side cluster join in kanban — server enrichment removes
  that need.
- Touching 0086 (kanban dnd-kit toasts) — coordinate via merge order.

## Implementation Approach

Test-driven where each new behaviour has a deterministic observable
(`data-active`, presence/absence of a DOM hook, an HTTP response field
value). For Rust changes that add fields and derived values, write the
test alongside the field, and for new components, write the test before
the JSX. Refactoring of existing tests (PipelineDots → Pipeline +
PipelineMini, WorkItemCard restructure) is folded into the phase that
introduces the production change so the test suite is never red across
phase boundaries.

Phases are sequenced for **independent merge slices**: each phase is a
complete, shippable PR with its own success criteria. Server phases
1→2→3 are linear (Phase 2's per-entry `completeness` depends on Phase 1's
`present` field; Phase 3's back-fill stores alongside Phase 2). Phase 4
is independent of 1-3. Phase 5 (tokens) is independent of all server
work. Frontend component and surface phases (6-10) consume both server
fields and tokens and have explicit `Depends on:` lines on each phase.

---

## Phase 1: Server — `Completeness.present` first-class field

### Overview

Add `present: Vec<String>` to the Rust `Completeness` struct and to the
matching TypeScript type. Populate inside `derive_completeness` from the
booleans it already computes. The values are `DocTypeKey` kebab-case
strings (Decision 1). This phase has no consumer change — it's a pure
shape extension that the later frontend phases consume.

### Changes Required

#### 1. Rust `Completeness` struct

**File**: `server/src/clusters.rs`

**Changes**: Add `pub present: Vec<String>` to `Completeness`. Populate
by iterating a single canonical ordering table (the source of truth for
stage order) and pushing each stage key whose boolean is true. The table
mirrors the frontend's `LIFECYCLE_PIPELINE_STEPS` followed by
`LONG_TAIL_PIPELINE_STEPS`; a code comment must cross-reference the
frontend constant so future re-orderings stay in lockstep.

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Completeness {
    pub has_work_item: bool,
    // ... existing booleans
    pub has_design_gap: bool,
    pub present: Vec<String>,
}

// Single source of truth for stage push order. MUST match the frontend's
// LIFECYCLE_PIPELINE_STEPS (in `frontend/src/api/types.ts`) followed by
// LONG_TAIL_PIPELINE_STEPS. Cross-reference any reordering on both sides.
const STAGE_PUSH_ORDER: &[(fn(&Completeness) -> bool, &str)] = &[
    (|c| c.has_work_item, "work-items"),
    (|c| c.has_research, "research"),
    (|c| c.has_plan, "plans"),
    (|c| c.has_plan_review, "plan-reviews"),
    (|c| c.has_validation, "validations"),
    (|c| c.has_pr_description, "pr-descriptions"),
    (|c| c.has_pr_review, "pr-reviews"),
    (|c| c.has_decision, "decisions"),
    (|c| c.has_notes, "notes"),
    (|c| c.has_design_inventory, "design-inventories"),
    (|c| c.has_design_gap, "design-gaps"),
];

fn derive_completeness(entries: &[IndexEntry]) -> Completeness {
    let mut c = Completeness {
        has_work_item: false,
        // ... existing initialisers
        has_design_gap: false,
        present: Vec::new(),
    };
    for e in entries {
        match e.r#type {
            DocTypeKey::WorkItems => c.has_work_item = true,
            // ... existing arms unchanged
        }
    }
    for (test, key) in STAGE_PUSH_ORDER {
        if test(&c) { c.present.push((*key).into()); }
    }
    c
}
```

`present` lists *all* stages whose boolean is true, including the three
long-tail stages (`notes`, `design-inventories`, `design-gaps`). The
frontend `Pipeline` component filters down to `WORKFLOW_PIPELINE_STEPS`
when rendering; long-tail consumers can rely on the full set.

#### 2. Frontend `Completeness` interface

**File**: `frontend/src/api/types.ts`

**Changes**: Add `present: string[]` to `Completeness`.

```ts
export interface Completeness {
  hasWorkItem: boolean
  // ... existing fields
  hasDesignGap: boolean
  present: string[]
}
```

#### 3. Test fixtures

**File**: `frontend/src/api/test-fixtures.ts`

**Changes**: Add a `makeCompleteness(overrides?: Partial<Completeness>)`
factory that returns an all-false, empty-`present` `Completeness`. Use
this factory inside `makeIndexEntry` and from any test that builds a
`Completeness` literal.

### Success Criteria

#### Automated Verification

- [ ] `cargo test -p accelerator-visualiser-server` passes
- [ ] New Rust unit test in `server/src/clusters.rs` `#[cfg(test)] mod tests`
      asserts: a cluster with WorkItems + Plans entries yields
      `present == vec!["work-items", "plans"]` and a cluster with only a
      WorkItem yields `present == vec!["work-items"]`.
- [ ] New Rust unit test asserts long-tail inclusion: a cluster with
      `has_notes = true` and `has_design_gap = true` yields
      `present == vec!["notes", "design-gaps"]` (long-tail keys appear
      after workflow keys when present).
- [ ] New Rust unit test asserts canonical ordering: a cluster with every
      `has_*` flag true yields `present == vec!["work-items", "research",
      "plans", "plan-reviews", "validations", "pr-descriptions",
      "pr-reviews", "decisions", "notes", "design-inventories",
      "design-gaps"]`.
- [ ] New frontend parity test
      `frontend/src/api/pipeline-step-parity.test.ts` asserts
      `WORKFLOW_PIPELINE_STEPS.map(s => s.docType).concat(
      LONG_TAIL_PIPELINE_STEPS.map(s => s.docType))` deep-equals the
      same literal vector. The Rust and TypeScript tests share the
      identical eleven-element literal so any divergence on either
      side fails its respective test, anchoring cross-language parity
      without manual cross-reading.
- [ ] `npm run typecheck` passes in `frontend/`
- [ ] `npm test` passes in `frontend/` (existing tests continue green with
      `makeCompleteness()` factory threaded through; no behavioural change
      yet)

#### Manual Verification

- [ ] `curl localhost:5173/api/lifecycle | jq '.[0].completeness.present'`
      returns a JSON array of kebab-case stage strings consistent with the
      `has*` booleans on the same object.

---

## Phase 2: Server — Per-entry `IndexEntry.completeness`

**Depends on**: Phase 1 (`Completeness.present`).

### Overview

Add `completeness: Option<Completeness>` to `IndexEntry`. Populate during
the cluster pass by building a `HashMap<slug, Completeness>` from the
freshly computed clusters and back-filling onto every entry whose
`slug.is_some()`. Orphan entries (no slug) stay `None`, which serialises
to `null` and is the signal kanban cards use for orphan rendering.

Important coherence requirements:

- The back-fill applies to both the canonical `Indexer::entries` map AND
  the `LifecycleCluster.entries` clones produced by `compute_clusters`.
  Otherwise `/api/docs/work-items` and `/api/lifecycle` disagree on the
  same entry's `completeness`. The cleanest shape is to make the back-fill
  the canonical post-step inside `compute_clusters` itself: it returns
  both the cluster list and a `HashMap<PathBuf, Completeness>` that the
  caller applies to `entries` under the same write lock as the cluster
  assignment.
- The `refresh_one` path (used by the watcher for single-file updates,
  `indexer.rs:356`) currently does not rebuild clusters. After this
  phase, `refresh_one` must trigger a full cluster recompute because
  cluster membership can shift on a single-file change. Concrete
  wiring:
  - Introduce a new `AppState::refresh_one_and_recompute(&path)`
    wrapper that takes both the `entries` and `clusters` locks (in
    that order), calls `indexer.refresh_one(path).await`, then
    invokes the same `compute_clusters` + back-fill pipeline that
    Phase 2's main path uses (returning the
    `(Vec<LifecycleCluster>, HashMap<PathBuf, Completeness>)` tuple
    and applying it under the held write locks).
  - Replace every existing caller of `indexer.refresh_one(...)` with
    `state.refresh_one_and_recompute(...)`. The two known call sites
    are `server/src/api/docs.rs:235` (after a kanban status edit)
    and `server/src/watcher.rs` (file event handler); audit by
    `rg 'refresh_one\b' server/src` before merging.
  - The watcher's full `rescan` path is unchanged — it already
    triggers `compute_clusters` + back-fill via the main pipeline.
- Scaling note: promoting `refresh_one` to a full O(N) cluster
  recompute is a deliberate coherence-over-throughput choice for this
  story. Bursts of file events serialise through the recompute and
  the cluster write-lock dominates read latency across
  `/api/lifecycle`, `/api/docs/*`, and `/api/related/*` during those
  windows. At expected workspace scale this is acceptable; if a
  future story needs to absorb higher event rates, the entry-map ↔
  cluster-clone coherence guarantee documented here is the contract
  any incremental recompute must preserve.

### Changes Required

#### 1. `IndexEntry` struct + serialisation

**File**: `server/src/indexer.rs`

**Changes**: Add `pub completeness: Option<Completeness>` to `IndexEntry`.
Default to `None` in `build_entry`. Because the field is serialised camelCase
(`completeness`), no rename annotation is needed.

#### 2. Cluster pass back-fill

**File**: `server/src/clusters.rs` (canonical post-step inside
`compute_clusters`; called from `server/src/server.rs:89`,
`server/src/watcher.rs:153-154`, and `Indexer::refresh_one` per the
coherence note above)

**Changes**: Extend `compute_clusters` to return `(Vec<LifecycleCluster>,
HashMap<PathBuf, Completeness>)`. The caller applies the map onto both
the cluster clones (`cluster.entries[i].completeness = ...`) and the
canonical entries map (`entries.get_mut(&path).completeness = ...`)
under a single write lock so the two views are atomically consistent.

Lock-ordering: take `state.clusters.write()` and `entries.write()` in
that order; release both before any reads. Document this invariant
inline at the call site.

#### 3. Frontend `IndexEntry` type

**File**: `frontend/src/api/types.ts`

**Changes**: Add `completeness: Completeness | null` to `IndexEntry`.
Either widen to `Completeness | null | undefined` (older servers omit
the field; JSON-omitted deserialises as `undefined`, not `null`) or
normalise at the API client boundary (`completeness: raw.completeness
?? null` inside `fetchDocs`). The plan picks normalisation at the API
client so consumers see a stable `Completeness | null` shape.

#### 4. Test fixtures

**File**: `frontend/src/api/test-fixtures.ts`

**Changes**: `makeIndexEntry` defaults `completeness: null` so existing
call sites pass through; tests for non-orphan work items override with
`makeCompleteness({ ... })`.

### Success Criteria

#### Automated Verification

- [ ] `cargo test -p accelerator-visualiser-server` passes
- [ ] New Rust test asserts: given a cluster with WorkItems + Plans
      entries, every entry in that cluster ends with
      `entry.completeness.as_ref().unwrap().has_work_item == true` and the
      same `present` vector as the cluster's `completeness.present`.
- [ ] New Rust test asserts: an entry whose `slug` is `None` ends with
      `entry.completeness.is_none()`.
- [ ] New Rust test asserts: two distinct entries sharing the same slug
      receive identical `completeness` values, and two entries in
      different clusters receive different `completeness` values
      (multi-entry back-fill sharing).
- [ ] New Rust test asserts cluster↔entry-map agreement: for every slug
      `s`, every `cluster.entries[i].completeness` for entries in that
      cluster equals `entries[path].completeness` for the same path
      (the canonical-snapshot invariant the back-fill guarantees).
- [ ] New Rust test asserts `refresh_one` coherence: starting from a
      seeded indexer, mutate a single file's content to move it between
      clusters; assert `entry.completeness.present` on the moved entry
      reflects the new cluster's state after the watcher event settles.
- [ ] `npm run typecheck` passes
- [ ] Existing `npm test` passes; `makeIndexEntry` defaults preserve
      behaviour.

#### Manual Verification

- [ ] `curl localhost:5173/api/docs/work-items | jq '.[0].completeness'`
      returns either a populated `Completeness` object or `null` for an
      orphan work item; the populated case mirrors the cluster's
      `completeness` for the same slug.

---

## Phase 3: Server — `IndexEntry.linkedCount` + shared resolver

**Depends on**: Phase 2 (per-entry storage location for the back-filled
count).

### Overview

Extract `/api/related/{path}`'s full resolution (including its two dedup
passes) into a single shared async function consumed by **both**
`related_get` (which serialises the lists) and the indexer's cluster
post-pass (which counts them). The count is then a `.len() + .len() +
.len()` over the same lists the endpoint returns — AC-6's equality is a
tautology, not a parallel-test invariant.

Two correctness concerns the current `related_get` resolves and that the
shared helper must preserve:
- Inbound merge dedup: paths already present from `reviews_by_target`
  are skipped when merging `work_item_refs_by_id`
  (`related.rs:79-89`).
- Inferred-vs-declared drop: same-slug siblings that also appear in any
  declared bucket are dropped from the inferred list
  (`related.rs:91-102`).

### Changes Required

#### 1. Shared async resolver

**File**: new `server/src/related.rs` sibling module. This preserves
the existing `api → domain` dependency direction (the indexer back-fill
pipeline would otherwise reach up into `api/related.rs` to call the
resolver). The handler in `server/src/api/related.rs` is reduced to an
axum entry point that calls `resolve_related` and serialises the
returned `RelatedResolution`. **Do not** place the resolver in
`indexer.rs` — that would reverse the existing `clusters → indexer`
dependency.

**Changes**: Introduce:

```rust
pub struct RelatedResolution {
    pub inferred_cluster: Vec<IndexEntry>,
    pub declared_outbound: Vec<IndexEntry>,
    pub declared_inbound: Vec<IndexEntry>,
}

pub async fn resolve_related(
    indexer: &Indexer,
    clusters: &[LifecycleCluster],
    entry: &IndexEntry,
) -> RelatedResolution {
    // Mirror related_get verbatim:
    //  - inbound = reviews_by_target ∪ work_item_refs_by_id, deduped by path
    //  - inferred = same-slug siblings minus self minus any path already
    //    in declared_outbound or declared_inbound
    // …
}

pub fn count_from_resolution(r: &RelatedResolution) -> usize {
    r.inferred_cluster.len() + r.declared_outbound.len() + r.declared_inbound.len()
}
```

Refactor `related_get` to call `resolve_related` and serialise its three
lists. The endpoint's response shape is unchanged.

#### 2. `IndexEntry.linkedCount`

**File**: `server/src/indexer.rs`

**Changes**: Add `pub linked_count: usize` to `IndexEntry` (camelCase via
existing `serde` annotation → `linkedCount`). Default to `0` in
`build_entry`. Back-fill in a **two-pass shape** as part of the same
post-cluster pipeline as Phase 2:

- Pass 1 (read-only): take a snapshot of `entries` via `entries.read()`,
  release the read lock, then iterate the snapshot and call
  `resolve_related(self, &clusters, &entry).await` for each entry. The
  resolver acquires its own brief read locks internally per call.
  Collect the resulting counts into a `HashMap<PathBuf, usize>`.
- Pass 2 (write): under a single `entries.write()` lock — taken in the
  documented order with `state.clusters.write()` — apply the map.

This avoids deadlock against the `entries.read()` taken inside
`declared_outbound` / `reviews_by_target` / `work_item_refs_by_id` (which
would re-enter if the iteration held the write lock).

**Lock-ordering and rescan_lock gating**: the entire post-cluster
pipeline (compute_clusters + Pass 1 + Pass 2) must acquire the existing
`Indexer::rescan_lock` semaphore (indexer.rs:218, 259, 367) for its
full duration, mirroring how `rescan()` and `refresh_one()` already
exclude each other. Without that gate, a concurrent
`refresh_one_and_recompute` between Pass 1 and Pass 2 can mutate
`entries` such that Pass 2 either writes stale `completeness`/
`linked_count` over freshly-refreshed entries or assigns to entries
that have been removed. As a defence-in-depth, Pass 2 should also
guard each assignment with a "path still present in `entries`" check
inside the write lock — silently skipping any path that has been
deleted since Pass 1's snapshot.

**Consistency contract**: AC-6's equality (`linkedCount == sum of
three list lengths from /api/related`) holds at the moment Pass 2
commits and is restored by the next watcher cycle if a subsequent
event drifts the maps. The contract is **point-in-time consistent at
write-apply commit**, not steady-state — readers between two commits
may observe values one cycle behind.

#### 3. `related_get` consumes the shared resolver

**File**: `server/src/api/related.rs`

**Changes**: Replace the existing inline resolution in `related_get`
with a call to `resolve_related`. Serialise the resolution's three
lists as today. Because both `related_get` and `linked_count` derive
from the same `RelatedResolution`, AC-6's equality holds by construction
— there is no parallel implementation to drift against.

#### 4. Frontend `IndexEntry` type

**File**: `frontend/src/api/types.ts`

**Changes**: Add `linkedCount: number` to `IndexEntry`. Normalise at
the API client boundary (`linkedCount: raw.linkedCount ?? 0`) so older
servers that omit the field don't surface `undefined` to consumers.

#### 5. Test fixtures

**File**: `frontend/src/api/test-fixtures.ts`

**Changes**: `makeIndexEntry` defaults `linkedCount: 0`.

### Success Criteria

#### Automated Verification

- [ ] `cargo test` passes
- [ ] New Rust integration test: seed an indexer with a cluster of three
      entries plus one cross-link from outside the cluster, fetch
      `/api/related/{path}` for one of the entries, assert
      `inferredCluster.len() + declaredOutbound.len() +
      declaredInbound.len() == entry.linked_count`.
- [ ] **Dedup integration test**: seed an entry that appears in both
      `inferred_cluster` (same-slug sibling) AND `declared_inbound` (via
      a review whose target is the entry's path). Assert the
      inferred-vs-declared drop applies (the path is dropped from
      `inferred_cluster`) and that `entry.linked_count` matches the
      endpoint's three-list sum (not the pre-dedup sum).
- [ ] **Inbound merge dedup test**: seed an entry whose path is reachable
      both from `reviews_by_target` and from `work_item_refs_by_id` (the
      same target path appears in both buckets). Assert
      `declaredInbound.len() == 1` and that `entry.linked_count` reflects
      that single inbound, not two.
- [ ] **Orphan-entry linkedCount test**: seed an entry whose `slug` is
      `None` but with one declared outbound link. Assert
      `entry.linked_count == 1`, `inferredCluster.is_empty()`, and
      `declaredOutbound.len() == 1` — covers AC-6's orphan carve-out.
- [ ] `npm run typecheck` passes
- [ ] `npm test` passes (fixture default of `0` keeps existing tests
      green)

#### Manual Verification

- [ ] In a workspace with mixed cross-links, fetch
      `/api/docs/work-items` and `/api/related/{relPath}` for a sampled
      entry: the sum of the three array lengths in the related response
      equals the entry's `linkedCount`.

---

## Phase 4: Server — Frontmatter-first `workItemId` resolution

### Overview

Extend `build_entry` to consult `frontmatter["work_item_id"]` first when
the doc type is `WorkItems`, falling back to the existing
`WorkItemConfig::extract_id(filename)` path. This satisfies AC-9 for
synced work items whose filename does not start with the namespaced
prefix.

### Changes Required

#### 1. Resolution order in `build_entry`

**File**: `server/src/indexer.rs` (around the
`work_item_cfg.extract_id(filename)` call site noted at
`indexer.rs:1032-1041` in the research)

**Changes**: For work-items entries only, try frontmatter first. The
frontmatter value is trimmed and routed through a single helper
`WorkItemConfig::normalise_id` that owns BOTH the shape check and the
canonical form. This keeps shape validity and canonical formatting in
one place — there is no separate `WORK_ITEM_ID_BASIC_SHAPE` regex
defined outside `WorkItemConfig`.

```rust
let work_item_id = if matches!(doc_type, DocTypeKey::WorkItems) {
    frontmatter
        .get("work_item_id")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .and_then(|s| work_item_cfg.normalise_id(s))
        .or_else(|| work_item_cfg.extract_id(filename))
} else {
    None
};
```

Add a new method `WorkItemConfig::normalise_id(&self, s: &str) ->
Option<String>` that:
1. Validates the input against the basic shape
   `^([A-Za-z]+-)?[0-9]+$` (accepts bare digits `0042` and
   prefix-dash-digits `ENG-0042`; rejects prefix-without-dash
   `ENG0042`, dotted suffixes `ENG-1.2`, etc.). Returns `None` on
   shape failure.
2. **If the input already carries any alphabetic prefix**, the prefix
   is preserved verbatim (`'OPS-7'` → `Some("OPS-7")`,
   `'ENG-0042'` → `Some("ENG-0042")`). This passthrough is what
   enables multi-prefix coexistence under remote sync.
3. **If the input is bare digits and `default_project_code` is set**,
   apply the configured prefix (`'42'` + `default_project_code =
   Some("ENG")` → `Some("ENG-42")`). This matches `extract_id`'s
   behaviour on a filename-derived numeric.
4. **If the input is bare digits and `default_project_code` is None**,
   return `Some("42")` (passthrough, matching `extract_id`).

Existing `extract_id` continues to handle filename extraction but
funnels its accepted string through the same shape check + step 2-4
logic (extract `extract_id`'s formatting tail into `normalise_id`).

If a frontmatter value is rejected by the shape check, emit a `warn!`
log naming the file path, the rejected value, and the expected shape
so operators can investigate silent fallback failures.

#### 2. Test coverage

**File**: `server/src/indexer.rs` (or a colocated test module that builds
`IndexEntry` from a synthetic file)

**Changes**: Add tests:

- Frontmatter `work_item_id: "ENG-0042"` with filename `0001-foo.md` →
  `entry.work_item_id == Some("ENG-0042")`.
- Frontmatter absent, filename `0042-foo.md`, `default_project_code =
  None`, `scan_regex = ^([0-9]+)-` → `entry.work_item_id == Some("0042")`.
- Frontmatter bare `work_item_id: "42"` with `default_project_code =
  Some("ENG")` → `entry.work_item_id == Some("ENG-42")` (normalisation
  applied — same canonical form as filename path).
- **Cross-prefix passthrough**: frontmatter `work_item_id: "OPS-7"`
  with `default_project_code = Some("ENG")` →
  `entry.work_item_id == Some("OPS-7")` (the workspace's default code
  is NOT applied when the frontmatter already carries a prefix; this is
  the remote-sync multi-prefix path).
- Frontmatter empty/whitespace string → fall back to filename path.
- Frontmatter shape-invalid (e.g. `"PROJ-1.2"`) → fall back to filename
  path, AND a `warn!` log line names the file and rejected value.
- Both frontmatter absent AND filename does not match `scan_regex` (e.g.
  `foo-without-number.md`) → `entry.work_item_id == None`. Exercises the
  AC-9 omit-slot rendering path server-side.
- Doc type not `WorkItems` → frontmatter ignored.

### Success Criteria

#### Automated Verification

- [ ] `cargo test` passes
- [ ] New tests above are green.
- [ ] **Corpus scan** (one-shot, before implementation): run
      `rg -nU --multiline -t md '^---\n(?:.*\n)*?work_item_id:' meta/work/`
      and verify no existing `meta/work/*.md` file has both a frontmatter
      `work_item_id` with alphabetic prefix AND a filename whose
      extracted ID would have differed. Document any conflicts in the
      PR description so the displayed-ID change is intentional.

#### Manual Verification

- [ ] Place a synthetic `meta/work/0001-foo.md` whose frontmatter has
      `work_item_id: "ENG-0042"`; restart the visualiser; confirm
      `/api/docs/work-items` returns `workItemId: "ENG-0042"` for that
      entry.

---

## Phase 5: Frontend — `--ac-stage-*` design tokens

### Overview

Introduce eight per-stage tokens following the existing `--ac-doc-*`
naming precedent (Decision 6). No `-on/-off` suffix — the existing
token system uses `--ac-doc-<key>` (fg) + `--ac-doc-bg-<key>` (bg)
with no on/off convention, and only the active hue is needed here.
Add a CSS comment block in `global.css` explaining the relationship
to `--ac-doc-*`: the stage palette is intentionally distinct (chain
hues, not per-type badge hues) and must not be unified without
design intent review. The tokens are consumed by `Pipeline` (Phase 6)
and `PipelineMini` (Phase 7); this phase has no visual change on its
own.

### Token Set

One token per workflow stage. `<key>` is the stage's `DocTypeKey`
kebab-case form, matching `LIFECYCLE_PIPELINE_STEPS[i].docType`:

- `--ac-stage-work-items` — hue 0
- `--ac-stage-research` — hue 28
- `--ac-stage-plans` — hue 220
- `--ac-stage-plan-reviews` — hue 260
- `--ac-stage-validations` — hue 160
- `--ac-stage-pr-descriptions` — hue 200
- `--ac-stage-pr-reviews` — hue 280
- `--ac-stage-decisions` — hue 355

Long-tail stages (`notes`, `design-inventories`, `design-gaps`) do not
get tokens — they are not rendered inside `Pipeline` / `PipelineMini`.
The `Pipeline`/`PipelineMini` accent lookup is guarded by an explicit
allowlist (membership test against `WORKFLOW_PIPELINE_STEPS`) so a
future long-tail rendering experiment fails loudly rather than emitting
a dangling `var(--ac-stage-notes)` reference.

### Refinement procedure

Both themes start from a hue formula (light: `hsl(<hue> 68% 46%)`;
dark: `hsl(<hue> 72% 60%)`) and must meet ≥3:1 contrast against the
theme's `--ac-bg`. The procedure is:

1. Compute starting hex from the formula.
2. Check contrast via the WCAG block in `global.test.ts`.
3. If below 3:1, reduce lightness by 4% in light theme (or raise
   lightness by 4% in dark theme) and retry until the threshold is
   met. Hue and saturation are held constant.
4. Record the final hex in `global.css`, `tokens.ts`, and the
   per-theme `--ac-doc-*` style block — there is no fixture step
   for these tokens (the `prototype-tokens.json` fixture covers only
   theme-invariant families).

Light-theme approximate starting values (refine per the procedure
above; the contrast test enforces the floor):

- `--ac-stage-work-items`:        `#c52828`
- `--ac-stage-research`:          `#c56327`
- `--ac-stage-plans`:             `#2762c5`
- `--ac-stage-plan-reviews`:      `#662cc5`
- `--ac-stage-validations`:       `#27c573`
- `--ac-stage-pr-descriptions`:   `#2796c5`
- `--ac-stage-pr-reviews`:        `#952cc5`
- `--ac-stage-decisions`:         `#c5273f`

Dark-theme values: the chain must stay coloured in dark mode (semantic
intent — `--ac-doc-*` collapses to white but the stage chain's whole
purpose is per-stage differentiation). Approximate starting values:

- `--ac-stage-work-items` (dark):       `#e26060`
- `--ac-stage-research` (dark):         `#e29560`
- `--ac-stage-plans` (dark):            `#6094e2`
- `--ac-stage-plan-reviews` (dark):     `#9560e2`
- `--ac-stage-validations` (dark):      `#60e2a3`
- `--ac-stage-pr-descriptions` (dark):  `#60c2e2`
- `--ac-stage-pr-reviews` (dark):       `#c060e2`
- `--ac-stage-decisions` (dark):        `#e26077`

### Changes Required

#### 1. `global.css` MIRROR-A light block

**File**: `frontend/src/styles/global.css`

**Changes**: Add the eight light-theme tokens to the `:root` block,
positioned after the `--ac-doc-bg-*` set (lines 122-133) with a
comment block matching the convention of the surrounding `--ac-doc-*`
tokens.

#### 2. `global.css` MIRROR-A dark block

**File**: `frontend/src/styles/global.css`

**Changes**: Add the eight dark-theme tokens to the `[data-theme="dark"]`
block, positioned analogously to MIRROR-A light.

#### 3. `global.css` MIRROR-B `prefers-color-scheme: dark`

**File**: `frontend/src/styles/global.css`

**Changes**: Hand-mirror the same eight tokens into the
`@media (prefers-color-scheme: dark) :root:not([data-theme="light"])`
block (byte-equivalent values to MIRROR-A). The existing parity helper
(`global.test.ts` parity describe) will fail otherwise.

#### 4. `tokens.ts` resolved-hex mirror

**File**: `frontend/src/styles/tokens.ts`

**Changes**: Add the eight stage tokens to both `LIGHT_COLOR_TOKENS`
and `DARK_COLOR_TOKENS` with their resolved-hex values (same as the
CSS values; no `var(--atomic-*)` brand indirection is used here).

#### 5. Contrast + parity test coverage

**File**: `frontend/src/styles/global.test.ts`

**Changes**:
- Extend the WCAG contrast describe to include the eight
  `--ac-stage-*` tokens against `--ac-bg` in both themes (≥3:1, same
  threshold as `--ac-doc-*`).
- Verify CSS↔TS parity for the new tokens.
- Verify MIRROR-A ↔ MIRROR-B parity for the new tokens.

### Success Criteria

#### Automated Verification

- [ ] `npm test -- global.test` passes (parity + contrast)
- [ ] `npm run typecheck` passes
- [ ] `npm run build` passes

#### Manual Verification

- [ ] In devtools, the eight tokens resolve to the expected hex values
      in both light and dark themes when inspecting `:root`.
- [ ] No visual change yet (no consumer until Phase 6 and 7).

---

## Phase 6: Frontend — `Pipeline` component

**Depends on**: Phase 1 (`completeness.present`), Phase 5 (tokens).

### Overview

New labelled chain component, used on lifecycle index cards and lifecycle
cluster detail. Test-driven: tests are written first; the component is
implemented to satisfy them.

### Changes Required

#### 1. Component file

**File**: `frontend/src/components/Pipeline/Pipeline.tsx`

**Changes**: Implement the labelled chain. API uses a named variant
prop (aligned with Chip) rather than a numeric size, eliminating the
off-grid Glyph problem and the `as never` cast:

```tsx
import type { Completeness } from '../../api/types'
import { WORKFLOW_PIPELINE_STEPS } from '../../api/types'
import { Glyph } from '../Glyph/Glyph'
import styles from './Pipeline.module.css'

type PipelineVariant = 'card' | 'panel'

interface Props {
  completeness: Completeness
  variant?: PipelineVariant // default 'card'
}

// Internal mapping from variant → Glyph size (must land on Glyph's 16/24/32 grid).
const GLYPH_SIZE: Record<PipelineVariant, 16 | 24> = {
  card: 16,
  panel: 24,
}

export function Pipeline({ completeness, variant = 'card' }: Props) {
  const present = new Set(completeness.present)
  return (
    <ol
      className={`${styles.chain} ac-stagechain`}
      data-variant={variant}
      aria-label={`Lifecycle pipeline, ${present.size} of 8 stages complete`}
    >
      {WORKFLOW_PIPELINE_STEPS.map((step, i) => {
        const active = present.has(step.docType)
        const nextActive = i < WORKFLOW_PIPELINE_STEPS.length - 1
          && present.has(WORKFLOW_PIPELINE_STEPS[i + 1].docType)
        const accent = `var(--ac-stage-${step.docType})`
        return (
          <li
            key={step.docType}
            className={`${styles.stage} ac-stagechain__stage`}
            data-stage={step.docType}
            data-active={String(active)}
            style={active ? { color: accent } : undefined}
          >
            <span className={styles.tile} aria-hidden="true">
              <Glyph docType={step.docType} size={GLYPH_SIZE[variant]} />
            </span>
            <span className={styles.label}>{step.label}</span>
            {i < WORKFLOW_PIPELINE_STEPS.length - 1 && (
              <span
                className={styles.connector}
                data-active={String(active && nextActive)}
                style={active && nextActive ? { background: accent } : undefined}
                aria-hidden="true"
              />
            )}
          </li>
        )
      })}
    </ol>
  )
}
```

Notes:
- The Glyph prop is `docType` (not `kind`). `size` is selected from the
  Glyph union via `GLYPH_SIZE[variant]`.
- `data-active` is coerced with `String(...)` for unambiguous serialisation.
- Off-state styling lives entirely in CSS via `[data-active="false"]`
  selectors; the inline `style` is omitted when inactive (no duplicated
  source of truth, no `var(--ac-stroke)` smuggled into JS).
- Tile size is selected via `data-variant` on the `<ol>` in the CSS
  module (`[data-variant="card"] .tile { --tile-size: 26px } …`), so
  there is no CSS custom property leaking into the React `style` prop.
- The accessible name composes `present.size` of 8 — a meaningful
  announcement rather than just a label.

Non-interactive: rendered as `<li>` not `<a>`; no `tabIndex`; no `onClick`.

#### 2. Styles

**File**: `frontend/src/components/Pipeline/Pipeline.module.css`

**Changes**: Layout the chain horizontally. Per-variant tile sizing
lives in the module via `[data-variant="card"] .tile { --tile-size:
26px; … }` / `[data-variant="panel"] .tile { --tile-size: 34px; … }`.
Tile is `var(--tile-size)` square with `border-radius: 6px`, mono
label below, connector behind the next tile.

Active/inactive state is driven entirely by `[data-active="true"]` /
`[data-active="false"]` selectors. Off-state: `border-color:
var(--ac-stroke)`, `background: var(--ac-bg-card)`, connector
`background: var(--ac-stroke)`. Active state: border-color, background
tint, and glyph contrast all derive from the inline `color` (the per-stage
accent), which CSS reads via `currentColor`.

#### 3. Tests (write first)

**File**: `frontend/src/components/Pipeline/Pipeline.test.tsx`

**Assertions**:

- Renders exactly eight `<li>` elements in `WORKFLOW_PIPELINE_STEPS` order
  with `data-stage` matching each step's `docType`.
- Each tile carries `data-active="true"` iff its `docType` is in
  `completeness.present`, else `data-active="false"`. Assertions use
  `toHaveAttribute('data-active', 'true')` / `'false'`.
- Each tile renders the step's `label` text.
- No `<a>` element appears inside the component; clicking a tile does not
  invoke any handler (no listeners).
- When `completeness.present` changes between renders, `data-active`
  updates accordingly (re-derived from prop).
- **Connector active-iff-both-adjacent rule** (the work item's
  connector-colouring rule): with `present = ['work-items', 'plans']`
  (non-adjacent in `WORKFLOW_PIPELINE_STEPS` order), assert every
  `[data-active]` on a connector is `"false"`. With
  `present = ['work-items', 'research']` (adjacent), assert exactly the
  connector following the `work-items` stage carries `data-active="true"`
  and every other connector carries `"false"`.
- **HSL-regression guard**: with an active stage rendered, the
  `<li>`'s computed `style.cssText` (or the inline `style` attribute)
  must contain `var(--ac-stage-` and must NOT contain `hsl(`. This
  defends Decision 6 — no hard-coded HSL re-enters production.
- **Variant prop**: with `variant="panel"`, `<ol>` carries
  `data-variant="panel"`; with default, `data-variant="card"`. The
  Glyph rendered inside an active stage has the expected size (16 for
  card, 24 for panel) — assert by reading the `width` (or `height`)
  attribute on the Glyph's `<svg>` and comparing to the expected value
  (`expect(svg).toHaveAttribute('width', '16')` for card,
  `'24'` for panel). The Glyph component does not emit a
  `data-glyph-size` attribute, so probe the SVG dimensions directly.
- **Computed-style smoke probe**: with `present = ['work-items']`,
  `getComputedStyle(tile).color` for the active tile is not the same
  as the inactive tile's — defends against CSS regressions where the
  data-attribute is correct but styling diverges.

### Success Criteria

#### Automated Verification

- [ ] `npm test -- Pipeline` is green
- [ ] `npm run typecheck` is green

#### Manual Verification

- [ ] No new direct consumer yet; component is exercised via the surfaces
      in Phase 8 and Phase 9.

---

## Phase 7: Frontend — `PipelineMini` component

**Depends on**: Phase 1 (`completeness.present`), Phase 5 (tokens).

### Overview

Compact unlabelled dot row for kanban embedding. TDD as for Phase 6.

### Changes Required

#### 1. Component

**File**: `frontend/src/components/PipelineMini/PipelineMini.tsx`

**Changes**: Eight dots in `WORKFLOW_PIPELINE_STEPS` order. Same
`present` matching against `step.docType`. Active dots fill and
border-colour from the per-stage `--ac-stage-<docType>` token
(Decision 6 — same token as `Pipeline`, no separate HSL formula).
Inactive dots transparent fill, `var(--ac-stroke)` border (driven from
CSS, not inline).

API note: unlike `Pipeline`, `PipelineMini` does NOT expose a
`variant` axis — kanban is its only consumer and the compact dimensions
(8px dots, 6px gap) are baked into the CSS module. If a future surface
needs an alternative size, add a `density?: 'compact' | …` axis at
that point; do not pre-emptively widen the API.

Root element is `<ol>` so the `aria-label` binds reliably (mirrors
`Pipeline` and the existing `PipelineDots`); BEM hook naming uses the
explicit `block__element` form to align with the AC-mandated
`.ac-kcard__top` style.

```tsx
interface Props {
  completeness: Completeness
}

export function PipelineMini({ completeness }: Props) {
  const present = new Set(completeness.present)
  return (
    <ol
      className={`${styles.row} ac-stagedots`}
      aria-label={`Lifecycle pipeline, ${present.size} of 8 stages complete`}
    >
      {WORKFLOW_PIPELINE_STEPS.map(step => {
        const active = present.has(step.docType)
        const accent = `var(--ac-stage-${step.docType})`
        return (
          <li
            key={step.docType}
            className={`${styles.dot} ac-stagedots__dot`}
            data-stage={step.docType}
            data-active={String(active)}
            style={active ? { background: accent, borderColor: accent } : undefined}
          />
        )
      })}
    </ol>
  )
}
```

#### 2. Styles

**File**: `frontend/src/components/PipelineMini/PipelineMini.module.css`

**Changes**: 8px dots, 6px gap, 1.5px border, optional halo via
`box-shadow: 0 0 0 2px color-mix(in oklab, currentColor 20%, transparent)`
on `[data-active="true"]`.

#### 3. Tests (write first)

**File**: `frontend/src/components/PipelineMini/PipelineMini.test.tsx`

**Assertions**:

- Renders exactly eight `<li>` dot elements inside an `<ol>` root, in
  `WORKFLOW_PIPELINE_STEPS` order.
- Each dot carries `data-active="true"` iff its `docType` is in
  `completeness.present`, else `data-active="false"`. Assertions use
  `toHaveAttribute('data-active', 'true')` / `'false'`.
- No label text rendered (each dot is empty).
- No `<a>` and no focusable elements.
- **HSL-regression guard**: with an active dot rendered, the dot's
  inline `style` (or `style.cssText`) must contain `var(--ac-stage-`
  and must NOT contain `hsl(` — defends Decision 6.
- **Computed-style smoke probe**: an active dot's `getComputedStyle`
  `background-color` differs from an inactive dot's; defends against
  CSS regressions where `data-active` is correct but styling diverges.

### Success Criteria

#### Automated Verification

- [ ] `npm test -- PipelineMini` is green
- [ ] `npm run typecheck` is green

#### Manual Verification

- [ ] Component exercised on kanban via Phase 10.

---

## Phase 8: Frontend — `Pipeline` panel on `LifecycleClusterView`

**Depends on**: Phase 6.

### Overview

Insert a `Pipeline` in a styled panel between the back link and the
existing timeline on lifecycle cluster detail.

### Changes Required

#### 1. JSX insertion

**File**: `frontend/src/routes/lifecycle/LifecycleClusterView.tsx`

**Changes**: Between the back-link `<Link>` close and the workflow
`<ol className={styles.timeline}>` open, insert:

```tsx
<section className={`${styles.pipelinePanel} ac-lcluster__pipeline`}>
  <div className={`${styles.pipelineEyebrow} ac-lcluster__pipeline-eyebrow`}>
    Pipeline
  </div>
  <Pipeline completeness={cluster.completeness} variant="panel" />
</section>
```

Both new hooks use the `block__element` BEM form (Decision 2),
mirroring the existing `.ac-lcard__pipe` shorthand on the lifecycle
index.

`cluster.completeness` is already in scope from the existing
`useQuery({ queryKey: queryKeys.lifecycleCluster(slug) })` call.

#### 2. Styles

**File**: `frontend/src/routes/lifecycle/LifecycleClusterView.module.css`

**Changes**: Add `.pipelinePanel` and `.pipelineEyebrow` rules matching
the work item's panel spec verbatim:

```css
.pipelinePanel {
  padding: 16px 20px;
  background: var(--ac-bg-sunken);
  border: 1px solid var(--ac-stroke);
  border-radius: 6px;
  margin-bottom: 8px;
}
.pipelineEyebrow {
  font-family: var(--ac-font-mono);
  font-size: 10.5px;
  color: var(--ac-fg-faint);
  letter-spacing: 0.1em;
  text-transform: uppercase;
  margin-bottom: 14px;
}
```

#### 3. Test update

**File**: `frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx`

**Changes**: Add an assertion:

- A `Pipeline` panel renders above the timeline `<ol>`. Confirm via:
  - `screen.getByText(/^Pipeline$/i)` resolves to a node inside
    `.ac-lcluster__pipeline`.
  - Eight `[data-stage]` tiles inside the panel.
  - The pipeline `<ol>` carries `data-variant="panel"`.
  - Each tile's `data-active` reflects the seeded cluster's
    `completeness.present`.
  - The panel appears before the workflow `<ol>` in document order,
    asserted via
    `panel.compareDocumentPosition(timeline) & Node.DOCUMENT_POSITION_FOLLOWING`
    being truthy (robust to wrapper insertions, unlike
    `querySelectorAll('*')` index comparison).

### Success Criteria

#### Automated Verification

- [ ] `npm test -- LifecycleClusterView` is green
- [ ] `npm run typecheck` is green
- [ ] `npm run build` succeeds

#### Manual Verification

- [ ] Open `/lifecycle/{slug}` for a cluster with mixed completeness;
      confirm the panel renders above the timeline, the eyebrow text is
      "Pipeline" in uppercase mono, the active stages have hue colouring,
      the inactive stages are outlined.
- [ ] Tab through the page; no stage tile receives focus.

---

## Phase 9: Frontend — `Pipeline` + counter on `LifecycleIndex`; delete `PipelineDots`

**Depends on**: Phase 6.

### Overview

Replace the existing `PipelineDots` on each lifecycle index card with a
`Pipeline` plus an N/8 counter in a dedicated `.ac-lcard__pipe` row.
Delete the `PipelineDots` directory and its test once the migration
lands.

### Changes Required

#### 1. JSX swap

**File**: `frontend/src/routes/lifecycle/LifecycleIndex.tsx`

**Changes**: Around the existing `PipelineDots` usage (`:112`), replace
with:

```tsx
<div className={`${styles.cardPipe} ac-lcard__pipe`}>
  <Pipeline completeness={cluster.completeness} variant="card" />
  <span className={`${styles.cardPipeCount} mono faint`}>
    {cluster.completeness.present.length}/8
  </span>
</div>
```

Remove the `PipelineDots` import.

#### 2. Styles

**File**: `frontend/src/routes/lifecycle/LifecycleIndex.module.css`

**Changes**: Add `.cardPipe` (flex row, `gap: 12px`, `align-items:
center`) and `.cardPipeCount` (mono `var(--ac-fg-faint)`).

#### 3. Delete the legacy component

**Files**:
- `frontend/src/components/PipelineDots/PipelineDots.tsx`
- `frontend/src/components/PipelineDots/PipelineDots.module.css`
- `frontend/src/components/PipelineDots/PipelineDots.test.tsx`

**Changes**: Remove the entire directory once `LifecycleIndex.tsx` no
longer imports it (verify with `rg PipelineDots frontend/src` returns no
hits — covering tests, fixtures, and other touch-points, not only
production source).

#### 4. Remove `migration.test.ts` EXCEPTIONS entries

**File**: `frontend/src/styles/migration.test.ts`

**Changes**: Remove the four EXCEPTIONS entries at lines 70-74 that
reference `components/PipelineDots/PipelineDots.module.css`. The
`EXCEPTIONS hygiene` describe block asserts every entry resolves to an
existing CSS file; leaving them in place after the directory is deleted
will fail `npm test`.

#### 5. Test update

**File**: `frontend/src/routes/lifecycle/LifecycleIndex.test.tsx`

**Changes**: Replace the existing PipelineDots assertion with:

- A `Pipeline` renders inside `.ac-lcard__pipe` on each cluster card.
- The counter renders `${present.length}/8` (matched as text inside
  `.ac-lcard__pipe`).
- Each tile's `data-active` reflects the cluster's
  `completeness.present`.

### Success Criteria

#### Automated Verification

- [ ] `npm test -- LifecycleIndex` is green
- [ ] `npm test -- migration.test` is green (EXCEPTIONS array updated)
- [ ] `npm test` (full suite) is green
- [ ] `npm run typecheck` is green
- [ ] `npm run build` succeeds
- [ ] `rg PipelineDots frontend/src` returns no hits across all files
      (production, tests, fixtures)

#### Manual Verification

- [ ] Open `/lifecycle`; each cluster card shows the chain and an N/8
      counter; the chain matches the chain shown on that cluster's
      detail page.

---

## Phase 10: Frontend — `WorkItemCard` refactor

**Depends on**: Phase 2 (per-entry `completeness`), Phase 3 (`linkedCount`),
Phase 4 (frontmatter `workItemId`), Phase 7 (`PipelineMini`).

### Overview

Restructure `WorkItemCard` into the BEM-named regions the ACs commit to:
`__top` (id + PipelineMini), `__title`, `__kind`, `__foot` (mtime + "N
linked"). Switch ID rendering from `parseWorkItemId(entry.relPath)` to
`formatDocId(entry.workItemId)`. Add `PipelineMini` and "N linked" with
the omit-on-zero semantics.

### Changes Required

#### 1. Component restructure

**File**: `frontend/src/routes/kanban/WorkItemCard.tsx`

**Changes**: Replace `parseWorkItemId` + slug-fallback with
`formatDocId`. Restructure JSX:

```tsx
import { Link } from '@tanstack/react-router'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { formatMtime } from '../../api/format'
import { fileSlugFromRelPath } from '../../api/path-utils'
import { formatDocId } from '../library/doc-type-id'
import { PipelineMini } from '../../components/PipelineMini/PipelineMini'
import type { IndexEntry } from '../../api/types'
import styles from './WorkItemCard.module.css'

export interface WorkItemCardProps {
  entry: IndexEntry
  now?: number
}

export function WorkItemCard({ entry, now }: WorkItemCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({
    id: entry.relPath,
  })
  const { role: _role, ...sortableAttributes } = attributes

  const fileSlug = fileSlugFromRelPath(entry.relPath)
  const fmKind = entry.frontmatter['kind']
  const kindLabel = typeof fmKind === 'string' && fmKind.length > 0 ? fmKind : null
  const idLabel = entry.workItemId ? formatDocId(entry.workItemId) : null

  return (
    <li className={`${styles.card} ac-kcard`} data-relpath={entry.relPath}>
      <Link
        ref={setNodeRef}
        to="/library/$type/$fileSlug"
        params={{ type: 'work-items', fileSlug }}
        className={styles.cardLink}
        style={{
          transform: CSS.Transform.toString(transform),
          transition,
        }}
        {...sortableAttributes}
        {...listeners}
      >
        <div className={`${styles.cardTop} ac-kcard__top`}>
          {entry.completeness != null && (
            <PipelineMini completeness={entry.completeness} />
          )}
          {idLabel !== null && (
            <span className={`${styles.cardId} ac-kcard__id`}>{idLabel}</span>
          )}
        </div>
        <p className={`${styles.cardTitle} ac-kcard__title`}>{entry.title}</p>
        {kindLabel !== null && (
          <p className={`${styles.cardKind} ac-kcard__kind`}>{kindLabel}</p>
        )}
        <div className={`${styles.cardFoot} ac-kcard__foot`}>
          <span className={`${styles.cardMtime} ac-kcard__mtime`}>
            {formatMtime(entry.mtimeMs, now)}
          </span>
          {entry.linkedCount > 0 && (
            <span className={`${styles.cardLinks} ac-kcard__links`}>
              {entry.linkedCount} linked
            </span>
          )}
        </div>
      </Link>
    </li>
  )
}
```

Remove the `parseWorkItemId` import.

#### 2. Delete `parseWorkItemId`

**Files**:
- `frontend/src/api/work-item.ts` — remove the `parseWorkItemId` export.
- `frontend/src/api/work-item.test.ts` — remove the `parseWorkItemId`
  test block (or delete the file if it only tested this function).

**Changes**: After Phase 10's WorkItemCard rewrite, `parseWorkItemId`
has no production consumer (`announcements.ts` uses
`workItemIdFromRelPath`, not `parseWorkItemId` — the original retention
rationale was wrong). Delete it.

#### 3. Styles

**File**: `frontend/src/routes/kanban/WorkItemCard.module.css`

**Changes**:
- Rename `.cardHeader` → `.cardTop` and remove the mtime child from this
  row.
- Add `.cardFoot` flex row (`justify-content: space-between`, mono small
  text) for mtime + linked count.
- Add `.cardId` (mono small) replacing the old `.cardNumber/.cardSlug`
  pair.
- Add `.cardLinks` (mono small, `var(--ac-fg-faint)`).

#### 4. Test rewrite

**File**: `frontend/src/routes/kanban/WorkItemCard.test.tsx`

**Changes**: Replace the existing assertions with:

- Renders `entry.workItemId` verbatim when non-null (e.g. seed
  `workItemId: 'ENG-0042'`; assert `screen.getByText('ENG-0042')` is
  inside `.ac-kcard__id`).
- Renders `'0042'` verbatim when `workItemId: '0042'` is set with no
  prefix (verifies `formatDocId`'s passthrough behaviour — bare digits
  returned unchanged, no padding applied).
- Omits the `.ac-kcard__id` slot entirely when `entry.workItemId` is
  null (assert `container.querySelector('.ac-kcard__id')` is null).
- Renders `PipelineMini` as the first child of `.ac-kcard__top` when
  `entry.completeness` is present (assert
  `container.querySelector('.ac-kcard__top > .ac-stagedots:first-child')`
  exists).
- **PipelineMini receives the correct completeness**: seed
  `entry.completeness.present = ['work-items', 'plans']`; assert the
  `[data-stage="work-items"]` and `[data-stage="plans"]` dots inside
  `.ac-kcard__top` carry `data-active="true"` and at least one other
  dot carries `data-active="false"`. Anchors the prop-passing contract
  at the consumer level (independent of the PipelineMini unit test).
- Omits `PipelineMini` entirely when `entry.completeness` is null
  (orphan).
- Renders `"{N} linked"` inside `.ac-kcard__foot` when
  `entry.linkedCount > 0`.
- Omits the linked-label when `entry.linkedCount === 0` (assert no
  `.ac-kcard__links`).
- Renders `entry.frontmatter['kind']` value inside `.ac-kcard__kind`
  when present.
- Preserves dnd-kit attributes (`data-relpath`,
  `aria-roledescription="sortable"`).

### Success Criteria

#### Automated Verification

- [ ] `npm test -- WorkItemCard` is green
- [ ] `npm test` (full suite, including the now-shrunk work-item.test.ts
      and any consumer that imported `parseWorkItemId`) is green
- [ ] `npm run typecheck` is green
- [ ] `npm run build` succeeds
- [ ] `rg parseWorkItemId frontend/src` returns no hits (function fully
      removed)

#### Manual Verification

- [ ] Open `/kanban`; for a work item with a populated cluster, the
      compact pipeline row appears above the title, the ID renders
      verbatim (matching the namespace prefix returned by the server),
      mtime sits in a footer row, and an "N linked" label appears when
      the entry has cross-links.
- [ ] For an orphan work item (no cluster slug), no pipeline row and no
      ID slot render.
- [ ] Drag-and-drop still works (dnd-kit attributes preserved).

---

## Testing Strategy

### Unit Tests (Rust)

- `clusters::tests` — `derive_completeness` exercise of `present`
  ordering and content (Phase 1).
- `clusters::tests` (or new `indexer::tests`) — per-entry
  `completeness` back-fill correctness for clustered vs orphan entries
  (Phase 2).
- `related::tests` (sibling module) — `resolve_related` against
  synthetic clusters + declared links (both inbound merge dedup and
  inferred-vs-declared drop cases); `count_from_resolution` summing
  the resolved triples; equality against `related_get`'s materialised
  response (Phase 3).
- `indexer::tests` (or colocated) — frontmatter-first `workItemId`
  resolution for the seven cases enumerated in Phase 4.

### Unit Tests (Frontend)

Per phase, as enumerated:

- Token contrast + parity coverage (Phase 5)
- `Pipeline.test.tsx` (Phase 6)
- `PipelineMini.test.tsx` (Phase 7)
- `LifecycleClusterView.test.tsx` — panel-presence + DOM-order probe
  (Phase 8)
- `LifecycleIndex.test.tsx` — Pipeline + N/8 counter in
  `.ac-lcard__pipe` (Phase 9)
- `WorkItemCard.test.tsx` — full rewrite per Phase 10 assertion list

### Cross-Surface State-Change Test

Frontend integration-style test (in `LifecycleIndex.test.tsx` or a new
`pipeline-consistency.test.tsx`):

- Seed `fetchLifecycleClusters` to return `present: ["work-items"]` and
  `fetchDocs('work-items')` to return entries whose `completeness.present
  == ["work-items"]`.
- Render `LifecycleIndex`, assert `data-active="true"` on the
  `work-items` tile across each cluster card's `Pipeline`.
- Re-seed both mocks to return `present: ["work-items", "plans"]`,
  invalidate the queries, and re-render.
- Assert `data-active="true"` on `work-items` and `plans` tiles, and
  `data-active="false"` on every other workflow stage.
- Same probe repeated against kanban's `WorkItemCard` PipelineMini
  inside a separate test, using the same fixtures.

The cross-surface test exercises the rendering reaction to fresh data
but **does not** exercise the SSE → `useDocEvents` →
`queryClient.invalidateQueries` path. That path is the responsibility
of `frontend/src/api/use-doc-events.test.tsx` — which must assert that
a `doc-changed` event with `docType: "work-items"` (or any clustered
doc type) invalidates BOTH the `lifecycle()` and `docs('work-items')`
query keys. If that assertion does not currently exist, add it as part
of this story so the manual-verification claim (SSE-driven flag-flips
reach all three surfaces) has a regression test.

### Manual Verification Steps

1. Run `cargo run -p accelerator-visualiser-server` against a seeded
   workspace; verify `/api/lifecycle`, `/api/docs/work-items`, and
   `/api/related/{path}` all return the new fields.
2. Run `npm run dev` against that server; visit `/lifecycle`,
   `/lifecycle/{slug}`, `/kanban`; verify all three Acceptance Criteria
   surface behaviours.
3. Tab-traverse `/lifecycle/{slug}`; verify no stage tile receives focus.
4. Edit a work item file to add a new stage artifact (e.g. drop a
   matching `plans/*.md`); verify the SSE-driven invalidation propagates
   the new active stage to all three surfaces on next refetch.

## Performance Considerations

- Adding `present`, `completeness`, and `linkedCount` to `IndexEntry` is
  O(N) extra work in the indexer's cluster pass — already O(N), no
  asymptotic change.
- `resolve_related` per entry is O(k) hashmap lookups (k = entry's
  outbound + inbound + same-slug sibling count); `count_from_resolution`
  on the result is O(1). Total still O(N) overall.
- `entry.completeness` is the same `Completeness` object shared across
  the cluster's entries; consider `Arc<Completeness>` if memory pressure
  becomes an issue (not anticipated at workspace scale).
- Frontend: removing per-card `useRelated` calls (never built, but the
  alternative to server enrichment) keeps kanban hot path on the single
  `fetchDocs('work-items')` request.

## Migration Notes

- `Completeness.present`, `IndexEntry.completeness`, and
  `IndexEntry.linkedCount` are additive wire fields. Older frontends
  will ignore them; older servers won't emit them. To accommodate the
  mid-deploy window, the frontend TypeScript types accept both `null`
  and `undefined` (JSON-omitted fields deserialise as `undefined`,
  not `null`), or normalise at the API client boundary
  (`completeness ?? null`, `linkedCount ?? 0`). The kanban card falls
  back to orphan rendering when `completeness` is absent.
- The wire format inlines `Completeness` on every `IndexEntry` in a
  cluster (N copies of identical data). This is a deliberate choice:
  the cohesion benefit (every entry is self-describing) outweighs the
  duplication cost at expected workspace scale, and consumers never
  need a client-side join. If payload size becomes a concern, switching
  to a slug-keyed `completenessBySlug` map is a breaking wire change
  — plan accordingly.
- `parseWorkItemId` is **deleted** in Phase 10. The previous retention
  rationale ("kept for `announcements.ts`") was incorrect —
  `announcements.ts` uses `workItemIdFromRelPath`, not
  `parseWorkItemId`. No production consumer remains after Phase 10.
- The `PipelineDots` directory is deleted in Phase 9 once the lifecycle
  index migration has landed; this is the only consumer. The four
  EXCEPTIONS entries in `frontend/src/styles/migration.test.ts:70-74`
  must be removed at the same time.
- The orphan signal (`entry.completeness == null`) collapses two
  distinct conditions: (1) a work item that genuinely belongs to no
  cluster, and (2) a work item whose slug derivation failed (filename
  doesn't match `scan_regex`, frontmatter slug missing). The kanban
  surface cannot distinguish these — a slug-derivation bug masquerades
  as an orphan. Operators seeing a kanban card with no pipeline row
  should check the indexer logs for slug-derivation warnings.
- Phase 4's frontmatter-first `workItemId` resolution is a behavioural
  reinterpretation of any existing file with both a parseable filename
  ID and an alphabetic-prefixed `work_item_id:` in frontmatter. The
  pre-implementation corpus scan in Phase 4 confirms no such conflict
  in `meta/work/` today; future synced files may shift their displayed
  ID per AC-9.
- The vocabulary coupling cost: emitting `DocTypeKey` kebab-case
  strings as `Completeness.present` means a future doc-type rename
  (e.g. `pr-descriptions` → `pull-requests`) is a coordinated change
  across the Rust enum, the wire format, the frontend
  `LIFECYCLE_PIPELINE_STEPS` constant, the CSS token name in
  `global.css`, and the resolved-hex mirror in `tokens.ts`. Accept
  the surface area when renaming.
- The `.ac-*` BEM class hooks introduced here are local to the touched
  surfaces and are not a project-wide convention shift — if a future
  story aligns the rest of the visualiser to BEM, it will inherit these
  hooks; otherwise they remain a local exception.
- The `--ac-stage-*` token family is intentionally distinct from
  `--ac-doc-*` (chain hues vs per-type badge hues; different palettes
  by design intent). Do not unify the two families without a design
  intent review. An inline comment in `global.css` documents the
  relationship for future contributors.

## References

- Work item: `meta/work/0040-pipeline-visualisation-overhaul.md`
- Review: `meta/reviews/work/0040-pipeline-visualisation-overhaul-review-1.md`
- Codebase research: `meta/research/codebase/2026-05-31-0040-pipeline-visualisation-overhaul.md`
- Design prototype (in-tree bundle):
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
- Design prototype source (off-tree convenience):
  `~/Downloads/Accelerator/src/view-lifecycle.jsx`,
  `~/Downloads/Accelerator/src/view-kanban.jsx`,
  `~/Downloads/Accelerator/src/ui.jsx`
- Related ADRs: `meta/decisions/ADR-0025-work-item-cross-ref-aggregation.md`,
  `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`,
  `meta/decisions/ADR-0034-typed-linkage-vocabulary.md`
- Sequencing dependencies: 0033 (tokens, done), 0037 (Glyph, done),
  0038 (Chip, done), 0044 (kanban scope spike — blocks Phase 10
  finalisation), 0063 (kind/type rename, done)
- Downstream consumers (blocked by this plan): 0079 (detail-page aside
  region redesign), 0083 (DevDesignSystem reference page)
