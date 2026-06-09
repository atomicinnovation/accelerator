---
id: "0040"
title: "Pipeline Visualisation Overhaul"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
kind: story
status: done
priority: high
tags: [design, frontend, lifecycle, kanban]
type: work-item
schema_version: 1
last_updated: "2026-05-06T14:04:04+00:00"
last_updated_by: Toby Clemson
blocks: ["work-item:0079", "work-item:0040", "work-item:0083"]
source: "design-gap:2026-05-06-current-app-vs-claude-design-prototype"
relates_to: ["design-inventory:2026-05-21-015231-claude-design-prototype", "design-inventory:2026-05-06-140608-claude-design-prototype"]
---

# 0040: Pipeline Visualisation Overhaul

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a developer using the visualiser, I want to see lifecycle pipeline progress on every surface that shows a cluster or work item, so that I can spot incomplete or stalled work at a glance from the lifecycle index, cluster detail, or kanban board.

The unifying thread is to surface lifecycle pipeline progress everywhere a cluster or work item is shown, plus the minimal `WorkItemCard` chrome changes that make that pipeline readable on a kanban card. Concretely: replace the current `PipelineDots` component with two design-independent pipeline-visualisation components — a full labelled `Pipeline` (used on lifecycle cluster detail above the existing timeline, and on each cluster card in the lifecycle index) and a compact unlabelled `PipelineMini` (embedded in kanban `WorkItemCard`). Enrich `WorkItemCard` with `PipelineMini`, a server-computed "N linked" relation count, and the full namespaced work-item ID. All three surfaces share the same cluster-completeness data source.

## Context

The visualiser currently has a single `PipelineDots` component (`frontend/src/components/PipelineDots/PipelineDots.tsx`) that renders an eight-dot completeness row, used in exactly one place — the lifecycle index cluster cards (`routes/lifecycle/LifecycleIndex.tsx:112`). `PipelineDots` is not used on the lifecycle cluster detail page or in the kanban view.

The 2026-05-21 prototype (`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/`) shows the eight-stage pipeline rendered two different ways:

- **A full labelled chain** with linked stage tiles and labels (the prototype's `HexChain`, used at `size=26` on lifecycle index cluster cards inside an `.ac-lcard__pipe` flex row alongside an `N/8` counter, and at `size=34` on lifecycle cluster detail in a panel between the page header and the timeline).
- **A compact dot row** (the prototype's `StageDots`, embedded in `.ac-kcard` kanban cards next to the namespaced ID).

The current `WorkItemCard` (`routes/kanban/WorkItemCard.tsx`) renders the work-item ID, mtime, title, and an optional `frontmatter.kind` label (renamed from `type` per work item 0063). The prototype's `.ac-kcard` adds the compact pipeline row, an "N linked" relation count (`.ac-kcard__links`), and namespaced IDs of the form `PROJ-0042` / `ENG-0042` / `META-0042` — multi-prefix coexistence is real (work items created locally use the workspace's `work.default_project_code`; others can be synced in from remote work-management tools with different prefixes that still match `work.id_pattern`).

Reference screenshots: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/screenshots/lifecycle-cluster-detail.png`, `kanban-view.png`, `main-light.png`, `library-view.png`. Authoritative source files for the prototype components: `view-lifecycle.jsx`, `view-kanban.jsx`, `ui.jsx`, `app.css`.

## Requirements

- Replace `PipelineDots` with two new components in `frontend/src/components/`:
  - `Pipeline` — labelled stage tiles + connectors, configurable `size` prop (used at `26` on lifecycle index, `34` on cluster detail). Non-interactive: tiles do not link, do not scroll, do not focus. Status display only.
  - `PipelineMini` — compact unlabelled row, used in kanban `WorkItemCard`. Same data shape, different rendering.
- Both components consume cluster-completeness data via a `present: string[]` field on the `Completeness` shape — the list of stage keys whose corresponding boolean field is `true`. The existing `Completeness` shape (`frontend/src/api/types.ts:175-187`) currently exposes per-stage booleans only; this story adds `present: string[]` as a first-class field on the server-side `Completeness` type (and matching frontend type), derived at index time from the per-stage booleans. This keeps the wire contract and the component input shape aligned.
- Both components render `data-active="true"` on each tile/dot whose stage key appears in `completeness.present`, and `data-active="false"` otherwise. The attribute is derived from the current `completeness` prop on every render (i.e. re-computed when the prop changes), so the cross-surface state-change criterion has an unambiguous mechanism. This is the canonical observable signal for "active" across all surfaces and tests; visual styling (HSL accent, halo, etc.) is driven from this attribute.
- Lifecycle cluster detail (`routes/lifecycle/LifecycleClusterView.tsx`) renders a `Pipeline` in a panel between the page header and the existing vertical timeline. The panel is a styled container with `padding: 16px 20px`, `background: var(--ac-bg-sunken)`, `border: 1px solid var(--ac-stroke)`, `border-radius: 6px`, and an uppercase mono eyebrow `Pipeline` above the chain — matching the 2026-05-21 prototype's inline-styled panel in `view-lifecycle.jsx:134-137`. The panel shares `cluster.completeness` with the timeline below so the two stay in sync. The chain renders the eight `WORKFLOW_PIPELINE_STEPS` only; the three `LONG_TAIL_PIPELINE_STEPS` are excluded.
- Lifecycle index cluster cards (`routes/lifecycle/LifecycleIndex.tsx`) render a `Pipeline` plus a counter rendering `${cluster.completeness.present.length}/8` in `.ac-lcard__pipe`, replacing the current `PipelineDots`.
- Kanban cards (`routes/kanban/WorkItemCard.tsx`) embed `PipelineMini` as the first child of `.ac-kcard__top`, alongside the namespaced ID. A work item is *orphan* iff `entry.completeness == null` (loose equality — covers both `null` and `undefined`, i.e. the server has not assigned the work item to a lifecycle cluster); orphan cards omit `PipelineMini` entirely, matching the prototype. The term *orphan* is used elsewhere in this work item with this exact meaning.
- The indexer (`server/src/indexer.rs`) populates a per-entry `completeness: Completeness | null` field on `IndexEntry`, derived from the work item's lifecycle cluster (`null` when the work item is orphan). This lets kanban cards render their own cluster's pipeline state without a client-side join against `fetchLifecycleClusters`.
- The indexer computes `IndexEntry.linkedCount: number` at index time using the same resolution function that backs `GET /api/related/{path}` — i.e. `inferredCluster.length + declaredOutbound.length + declaredInbound.length`. Using a shared function (not a parallel implementation) keeps the count consistent with the endpoint, and keeps the kanban hot path on the single `fetchDocs('work-items')` request.
- Render an "{N} linked" label on `WorkItemCard` inside `.ac-kcard__foot` (alongside mtime), sourced from `entry.linkedCount`. When `linkedCount === 0`, the label is omitted entirely; when `linkedCount > 0`, the label appears in the footer.
- Switch `WorkItemCard` ID rendering from the legacy `parseWorkItemId(entry.relPath)` filename regex to `entry.workItemId` formatted via the existing `routes/library/doc-type-id.ts:formatDocId` helper. The card must render whatever namespaced prefix the server extracted, regardless of the workspace's `default_project_code` — IDs synced in from remote tools may use any prefix matching `work.id_pattern`. When `entry.workItemId` is `null`, the `.ac-kcard__id` slot is omitted entirely (the card must not fall back to the legacy filename regex).
- Preserve the existing `WorkItemCard` chrome: mtime, title, and the `frontmatter.kind` label.

## Acceptance Criteria

- [ ] Given a user opens `/lifecycle/{slug}`, when the page renders, then a `Pipeline` component appears in a panel above the existing vertical timeline, with each of the eight `WORKFLOW_PIPELINE_STEPS` shown as a labelled tile, and tiles whose stage key is in `cluster.completeness.present` carry `data-active="true"` while all other tiles carry `data-active="false"`.
- [ ] Given a user opens the lifecycle index, when each cluster card renders, then a `Pipeline` and a counter rendering `${cluster.completeness.present.length}/8` are visible in `.ac-lcard__pipe`, with each tile's `data-active` state derived from the same `cluster.completeness` data that drives the cluster detail page.
- [ ] Given a user opens the kanban view, when each `WorkItemCard` for a work item with a non-null `entry.completeness` renders, then a `PipelineMini` row appears as the first child of `.ac-kcard__top`, alongside the namespaced ID.
- [ ] Given a `WorkItemCard` for an orphan work item (`entry.completeness` is `null` or missing), when the card renders, then `PipelineMini` is omitted entirely.
- [ ] Given a `WorkItemCard`, when it renders the pipeline state, then it reads `entry.completeness` from `IndexEntry` directly (no client-side join against the lifecycle clusters query).
- [ ] Given any work item path `P` present in `fetchDocs('work-items')` after the indexer has settled (including orphan work items), when both the visualiser index and `GET /api/related/P` are queried, then `IndexEntry.linkedCount` equals `inferredCluster.length + declaredOutbound.length + declaredInbound.length` from the endpoint response. For orphan work items, `inferredCluster.length` is `0` and the count reflects only declared in/outbound relations.
- [ ] Given a `WorkItemCard` whose `entry.linkedCount > 0`, when the card renders, then a "{N} linked" label appears inside `.ac-kcard__foot` alongside mtime.
- [ ] Given a `WorkItemCard` whose `entry.linkedCount === 0`, when the card renders, then no "linked" label is rendered inside `.ac-kcard__foot`.
- [ ] Given a work item file with `work_item_id: "ENG-0042"` (or any prefix matching `work.id_pattern`) and a `relPath` filename that does not start with the same prefix (e.g. `0001-foo.md`), when its kanban card renders, then the displayed ID is exactly `ENG-0042` — i.e. sourced from `entry.workItemId`, not from the filename.
- [ ] Given a work item whose `entry.workItemId` is `null`, when the kanban card renders, then no ID is displayed in the `.ac-kcard__id` slot (the slot is omitted).
- [ ] Given a work item file with frontmatter `kind: "story"`, when its kanban card renders, then the kind label remains visible in its current chrome location (unchanged from today's behaviour).
- [ ] Given a user clicks any stage tile in the `Pipeline` on either lifecycle screen, when the click is processed, then no navigation occurs and no route change is triggered (status display only).
- [ ] Given the `Pipeline` is rendered on either lifecycle screen, when the page is tab-traversed, then no stage tile receives keyboard focus, and no tile is rendered as an `<a href>` element.
- [ ] Given the cluster detail page renders, when the `Pipeline` panel is inspected, then the container has an uppercase mono eyebrow with text `Pipeline` immediately above the chain, and the container uses `var(--ac-bg-sunken)` as its background.
- [ ] Given a cluster whose `completeness.present` changes from `["work"]` to `["work","adr"]`, when each surface is re-fetched via its production data hook (`fetchLifecycleClusters` for the lifecycle screens; `fetchDocs('work-items')` for kanban), then each surface's pipeline shows exactly the stages `work` and `adr` as `data-active="true"` and every other stage as `data-active="false"`.

## Open Questions

- None outstanding. (Interactivity of `Pipeline` stage tiles is intentionally deferred — see Drafting Notes.)

## Dependencies

- Blocked by:
  - 0033 (token system, done), 0037 (Glyph component, done), 0038 (Chip component, done) — full dependency chain unblocked for the pipeline-visualisation portion.
  - 0044 (kanban scope spike) — kanban portion only. The kanban Acceptance Criteria for `WorkItemCard` (PipelineMini embedding, namespaced ID, "N linked" footer) cannot be finalised until 0044 resolves the column model, "live" chip, and totals questions. The pipeline-visualisation portion of this story can progress in parallel.
- Builds on: 0063 (kind/type rename, done) — the kanban card chrome preserves the `frontmatter.kind` label, which assumes 0063's rename has landed.
- Coordinates with:
  - 0086 (kanban drag-and-drop with toast confirmations) — same `WorkItemCard` surface, merge-conflict risk if both progress in parallel.
  - 0057 / 0061 (typed cross-linking) — `linkedCount` semantics are bound to the current three-bucket `/api/related/{path}` shape (`inferredCluster + declaredOutbound + declaredInbound`). Any future change to the typed cross-linking model under those work items must consider this consumer; conversely, this story's contract assumes the current three-bucket shape.
- Blocks: 0079 (detail-page aside region redesign — explicitly cites 0040 as blocker), 0083 (DevDesignSystem reference page — explicitly cites 0040 as blocker).
- Sequencing: server-side `IndexEntry` enrichment (`completeness`, `linkedCount`) must land before, or in the same change as, the frontend `WorkItemCard` updates that consume those fields. The two server fields can be added independently of each other.
- May spawn follow-up to 0033: if a per-stage `--ac-stage-<key>-on` token is introduced as part of this refactor (the prototype currently hard-codes per-stage HSL values), the new tokens should be captured as a follow-up under the token system.

## Assumptions

- The existing `Completeness` shape is sufficient as the canonical data input for `Pipeline` and `PipelineMini`; no new fields on `Completeness` itself are required. (Server enrichment of `IndexEntry` to embed per-entry `completeness` and `linkedCount` is required and is captured in Requirements, not as an assumption.)
- `work.id_pattern` configuration is the authoritative grammar for namespaced IDs; the server's existing extraction logic (`server/src/config.rs:62-128`) correctly handles any prefix matching the pattern, including prefixes other than `work.default_project_code` that arrive via remote work-management synchronisation.

## Technical Notes

- **Component shape**: replace `PipelineDots` (`frontend/src/components/PipelineDots/PipelineDots.tsx:1-27`) with two new components. The current component has a single consumer (`routes/lifecycle/LifecycleIndex.tsx:112`); migration is contained.
- **Pipeline data flow on kanban**: `IndexEntry` (`api/types.ts:80-95`) currently carries `slug` but no `Completeness`. The cluster-level completeness lives only on `LifecycleCluster.completeness` returned by `fetchLifecycleClusters`. To avoid a client-side join in the kanban hot path, server-side `IndexEntry` enrichment is required — extend the indexer (`server/src/indexer.rs`) and the `IndexEntry` API type to include `completeness` keyed off the entry's cluster slug.
- **Linked count**: the existing endpoint `GET /api/related/{path}` (`server/src/api/related.rs:31-110`) returns `RelatedArtifactsResponse { inferredCluster, declaredOutbound, declaredInbound }`. The `linkedCount` field on `IndexEntry` is the sum of those three array lengths, computed at index time using the same resolution logic so the count and the endpoint stay consistent.
- **Namespaced IDs**: switch the kanban card from `parseWorkItemId(entry.relPath)` (legacy filename regex returning `number | null`) to `entry.workItemId` (server-typed `string | null` at `api/types.ts:85`) and render via `formatDocId` (`routes/library/doc-type-id.ts:1-12`). This matches the `LibraryTypeView` precedent (`routes/library/LibraryTypeView.tsx:36-37`). The legacy `parseWorkItemId` stays in use for `announcements.ts` screen-reader text and can remain.
- **Pipeline header on cluster detail**: `LifecycleClusterView.tsx:49-78` already has `cluster.completeness` in scope from `useQuery`. Inserting the new `Pipeline` in a panel above `<ol className={styles.timeline}>` is a localised edit. The chain renders the eight `WORKFLOW_PIPELINE_STEPS` only; `LONG_TAIL_PIPELINE_STEPS` (`types.ts:248-250`) are excluded from the chain (they remain rendered in the existing "Other artifacts" timeline section unchanged).
- **Connector colouring rule** (from prototype `view-lifecycle.jsx:28-49`): for each stage at index `i` in `WORKFLOW_PIPELINE_STEPS`, the link that follows it carries the stage's HSL accent when both stage `i` and stage `i+1` are active; otherwise the link uses `var(--ac-stroke)`. This must be computed in JSX (not pure CSS sibling selectors) because the link colour depends on the *next* stage's state in `WORKFLOW_PIPELINE_STEPS` order. The prototype hard-codes per-stage hue formulas (`hsl(hue 68% 46%)` for `Pipeline` active tiles, `hsl(hue 72% 56%)` for `PipelineMini` active dots) — no `--ac-stage-<key>-on` token exists. If we introduce one as part of the refactor, that's a 0033 (token system) follow-up worth flagging.
- **Note on `STAGES.work.hue` vs `TYPE_META.work.hue`** in the prototype: `STAGES.work` uses hue `0`, `TYPE_META.work` uses hue `12`. These are independent palettes; the `Pipeline` work tile uses `0`, the per-type glyph (used elsewhere) uses `12`. Don't unify these without checking design intent.
- **Tests to migrate**: split `PipelineDots.test.tsx` into `Pipeline.test.tsx` and `PipelineMini.test.tsx`; update `WorkItemCard.test.tsx`, `LifecycleIndex.test.tsx`, `LifecycleClusterView.test.tsx`. The new tests must assert at least the following:
  - `Pipeline.test.tsx`: renders exactly eight tiles in `WORKFLOW_PIPELINE_STEPS` order; each tile carries `data-active="true"` iff its stage key is in the `completeness.present` input, else `data-active="false"`; each tile renders its configured label; clicking a tile does not trigger navigation.
  - `PipelineMini.test.tsx`: renders exactly eight dots in `WORKFLOW_PIPELINE_STEPS` order; each dot carries `data-active="true"` iff its stage key is in the `completeness.present` input, else `data-active="false"`; no labels render.
  - `WorkItemCard.test.tsx`: renders `entry.workItemId` verbatim when non-null (replacing the current `#0001` assertion); omits the `.ac-kcard__id` slot when `entry.workItemId` is null; renders `PipelineMini` as the first child of `.ac-kcard__top` when `entry.completeness` is present, and omits it when null; renders "{N} linked" inside `.ac-kcard__foot` when `entry.linkedCount > 0` and omits the label when `entry.linkedCount === 0`; preserves the `frontmatter.kind` label in its existing chrome location.
  - `LifecycleIndex.test.tsx`: each cluster card renders a `Pipeline` inside `.ac-lcard__pipe` and a counter rendering `${present.length}/8`.
  - `LifecycleClusterView.test.tsx`: a `Pipeline` panel renders above the timeline showing each of the eight workflow stages; the timeline's per-stage rows below show the same active stages as the chain above.

## Drafting Notes

- Story treats the pipeline + kanban card enrichment as one cohesive story (confirmed by author). Splitting into per-surface stories would create coordination overhead with no clear value, because all three surfaces share the same data source and components.
- Renaming `HexChain` → `Pipeline` and `StageDots` → `PipelineMini` (component names independent of visual design). The names from the prototype source are visual-design-specific and not appropriate for the production component vocabulary.
- `Pipeline` is non-interactive in this story, matching the 2026-05-21 prototype. Interactivity (e.g. clicking a stage tile scrolls the timeline to that stage on cluster detail) is deferred for a follow-up decision; recorded here rather than as an Open Question so it doesn't block readiness.
- Priority bumped from `medium` to `high` because two downstream items (0079, 0083) explicitly block on 0040.
- "N linked" data is sourced from a new server-computed `IndexEntry.linkedCount` field rather than per-card `useRelated` calls (avoids N HTTP requests per kanban render). The semantic basis for "linked" is the existing `/api/related/{path}` three-bucket model (`inferredCluster` + `declaredOutbound` + `declaredInbound`).
- "N linked" label is hidden entirely when `linkedCount === 0` rather than rendered as "0 linked".
- Cluster-detail `Pipeline` shows the eight `WORKFLOW_PIPELINE_STEPS` only; long-tail stages stay in the existing "Other artifacts" timeline section.
- Multi-prefix ID coexistence is real (work items from remote sync may have prefixes other than the workspace's `default_project_code`), but no NEW infrastructure is needed — the existing server-side extraction in `config.rs:62-128` already handles any prefix matching `work.id_pattern`. The story narrows the requirement to "render whatever the server extracted, verbatim".
- The original story implied per-work-item completeness was already available; investigation showed that `IndexEntry` doesn't carry it. Server-side enrichment of `IndexEntry` is a required scope addition, captured in Requirements (not as an Assumption).
- The prototype's kanban card (`view-kanban.jsx:3-22`) does not currently render a kind/type label; author has confirmed the production card should keep its `frontmatter.kind` label and will update the prototype to match.
- "Active" stage styling has a canonical observable signal: the `data-active="true"` attribute on each tile/dot. Defined once in Requirements and asserted via the same probe across all three surfaces, so tests stay consistent and cross-surface state can be verified deterministically.
- Test-assertion expectations moved out of Acceptance Criteria into Technical Notes — they describe how implementation is verified rather than user-visible outcomes, so they belong with implementation guidance.
- Server-side `IndexEntry` enrichment (`completeness`, `linkedCount`) and the kanban namespaced-ID switch were considered for splitting into precursor or sibling stories. Both kept in this story because (a) the new server fields have no other consumer; splitting would create a dependency with no independent user-visible value, and (b) the namespaced-ID switch is what makes the existing server-side ID infrastructure visible on the kanban surface — separating it from the broader card enrichment would create two near-trivial PRs touching the same component.

## References

- Source: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Prototype (current): `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md` and `prototype-standalone.html`
- Prototype (superseded): `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md`
- Screenshots: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/screenshots/lifecycle-cluster-detail.png`, `kanban-view.png`, `main-light.png`, `library-view.png`
- Related (context only, no direct coupling): 0041 (library page wrapper — precedent for `entry.workItemId` + `formatDocId` rendering), 0045 (work-management integration epic — canonical source for `work.id_pattern`), 0078 (id_pattern auto-linkification in frontmatter table).
- Dependency-coupled work items are captured in the Dependencies section above: 0033, 0037, 0038, 0044, 0057, 0061, 0063, 0079, 0083, 0086.
