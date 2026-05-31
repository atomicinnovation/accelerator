---
date: "2026-05-31T23:08:12+01:00"
author: Toby Clemson
git_commit: 88b3cab89687dd2ca32bc4e38fb603198e46eb92
branch: HEAD
repository: accelerator
topic: "Pipeline Visualisation Overhaul (work item 0040) — codebase landing readiness"
tags: [research, codebase, visualiser, pipeline, lifecycle, kanban, indexer, work-item-0040]
status: complete
last_updated: 2026-05-31
last_updated_by: Toby Clemson
---

# Research: Pipeline Visualisation Overhaul (0040)

**Date**: 2026-05-31T23:08:12+01:00
**Author**: Toby Clemson
**Git Commit**: 88b3cab89687dd2ca32bc4e38fb603198e46eb92
**Branch**: HEAD (jj-colocated)
**Repository**: accelerator

## Research Question

How does work item `0040 — Pipeline Visualisation Overhaul` land in the
current codebase? Specifically: what is the production shape of every
component, type, route, and server endpoint named by the story; where
exactly the new `Pipeline` / `PipelineMini` components and server-side
`IndexEntry` enrichments must hook in; what diverges between the
2026-05-21 design prototype and what is shipped today; and what tests
and fixtures must move.

## Summary

The story is implementable as written, but **almost every file path in
the work item is relative-to-the-skill-root, not repo-root**. The
production code lives under
`skills/visualisation/visualise/{frontend,server}/...`, not
`frontend/...` or `server/...`. Plans and implementation prompts must
account for that.

Headline findings, scoped to the four worked surfaces:

- **PipelineDots is a single-consumer, single-file widget** with no
  external test fixtures or extra wiring; the migration to `Pipeline` +
  `PipelineMini` is a self-contained refactor (PipelineDots.tsx,
  PipelineDots.module.css, PipelineDots.test.tsx all go).
- **The class names the story names** (`.ac-kcard`, `.ac-kcard__top`,
  `.ac-kcard__id`, `.ac-kcard__foot`, `.ac-lcard__pipe`) **do not
  exist in production yet**. The current frontend uses CSS-module
  camelCase keys (`styles.card`, `styles.cardHeader`, `styles.cardMtime`,
  …); the `ac-` prefix appears only on design tokens. Either the new
  components introduce the `ac-` BEM class hooks (matching the prototype)
  or the ACs must be retargeted at the existing camelCase classes. The
  Acceptance Criteria explicitly name `.ac-kcard__top` /
  `.ac-kcard__foot` / `.ac-lcard__pipe` so the implementer will need to
  introduce those classnames on the production cards as part of the
  refactor.
- **The current kanban card has no `__foot`**: `cardMtime` sits inside
  the `cardHeader` flex row alongside the ID, not in a separate footer.
  AC-7 / AC-8 ("'N linked' label alongside mtime in `.ac-kcard__foot`")
  implies introducing a new footer row and moving mtime into it.
- **`PipelineDots` currently has only 8 dots already**: it iterates
  `WORKFLOW_PIPELINE_STEPS` (which derives from `LIFECYCLE_PIPELINE_STEPS`
  by filtering out the three `longTail: true` entries), so the
  workflow/long-tail split the story relies on is already in place on
  both frontend and server.
- **Server-side `Completeness` already has eleven `hasX: boolean`
  fields and no `present: string[]`**. Adding `present` requires:
  (a) a new Rust field on `Completeness` (`server/src/clusters.rs:8-22`);
  (b) populating it inside `derive_completeness`
  (`clusters.rs:101-133`) from the boolean fields it already computes;
  (c) mirroring the field on the frontend `Completeness` interface
  (`types.ts:175-187`); (d) updating three duplicated test fixtures
  plus the `LifecycleCluster` literal builders.
- **`IndexEntry` currently has no `completeness` field**. Adding
  per-entry `completeness: Completeness | null` is straightforward: the
  cluster computation already groups entries by `entry.slug`, so the
  indexer can produce a `HashMap<slug, Completeness>` during cluster
  build and back-fill it onto every entry that shares the slug. The
  cluster pass runs once at server startup and after every watcher
  event, so this is already an O(n) pass.
- **`linkedCount` is cheap**: `/api/related/*path` is already pure
  in-memory hashmap lookups against indexes the indexer builds (no
  per-request disk reads). The three resolution helpers
  (`indexer.declared_outbound`, `indexer.reviews_by_target`,
  `indexer.work_item_refs_by_id` plus the inline `inferredCluster`
  derivation from `state.clusters`) can be called per-entry during the
  same indexer build pass. **One gotcha**: declared outbound and
  declared inbound are not deduped against each other inside
  `related_get`, so the same artifact could be double-counted; the
  worked AC arithmetic
  (`inferredCluster.length + declaredOutbound.length + declaredInbound.length`)
  matches the endpoint's own response shape and is the contract the
  story commits to.
- **`workItemId` extraction is filename-only and project-code-bound**:
  the production `WorkItemConfig::extract_id`
  (`server/src/config.rs:111-128`) only handles ASCII-digit fallbacks
  unless the `scan_regex` itself captures a non-numeric prefix.
  Assumption #2 of the work item ("the server's existing extraction
  logic correctly handles any prefix matching the pattern") is **true
  only if `scan_regex` is configured to capture that prefix**. For a
  workspace whose `scan_regex` is the default `^([0-9]+)-`, a synced
  `ENG-0042-foo.md` file will return `None`. This is worth pinning
  during planning: either the assumption is correct because real
  remote-sync workflows must configure `scan_regex`, or extraction
  needs to be widened.
- **Frontmatter is never consulted for an entry's own
  `work_item_id`**. AC-9 ("a work-item file with
  `work_item_id: \"ENG-0042\"` … displayed ID is exactly `ENG-0042`")
  is **not satisfied by the current extraction** when the `scan_regex`
  does not match the filename — frontmatter is only read for
  `work_item_refs`, never for the entry's own ID. Either AC-9 needs
  rewording to a filename-derived case, or extraction must be extended
  to consult `frontmatter['work_item_id']` first.
- **No `--ac-stage-*` tokens exist**; per-stage hues are hard-coded
  inline in the 2026-05-21 prototype as `hsl(hue 68% 46%)` (Pipeline
  tiles) and `hsl(hue 72% 56%)` (PipelineMini dots). The closest
  existing token family is per-doc-type
  `--ac-doc-<kind>` / `--ac-doc-bg-<kind>` (light + dark) in
  `frontend/src/styles/global.css`. Introducing
  `--ac-stage-<key>-on` is a 0033 follow-up the story already flags.
- **All three test files using `Completeness`** (`PipelineDots.test.tsx`,
  `LifecycleIndex.test.tsx`, `LifecycleClusterView.test.tsx`) duplicate
  the same `empty: Completeness` literal. Introducing
  `present: string[]` will cascade through these files plus
  `makeIndexEntry` (`test-fixtures.ts:6-23`). A shared
  `makeCompleteness()` factory would localise the churn.

## Detailed Findings

### Path correction: visualiser code lives in a skill subtree

The work item references `frontend/src/...` and `server/src/...` but
the production code is at
`skills/visualisation/visualise/frontend/src/...` and
`skills/visualisation/visualise/server/src/...`. Every subsequent
file:line reference in this document uses the canonical skill-relative
path. The user-memory note `feedback_workspaces_dir_is_jj_not_code`
already encodes the rule: `workspaces/` is a jj-workspace mirror, not
a code dup.

### Existing PipelineDots (the surface being replaced)

- `skills/visualisation/visualise/frontend/src/components/PipelineDots/PipelineDots.tsx`
  — single component, single prop `completeness: Completeness`, root
  is `<ul className={styles.pipeline} aria-label="Lifecycle pipeline">`,
  iterates `WORKFLOW_PIPELINE_STEPS`. Each `<li>` carries
  `data-stage={step.key}`, `data-present={present}` (boolean
  stringified), `title={step.label}`, and `aria-label="${label}: ${present
  ? 'present' : 'missing'}"`.
- `skills/visualisation/visualise/frontend/src/components/PipelineDots/PipelineDots.module.css`
  — 14×14 dots, 1.5px stroke, `--ac-accent` fill on `.present`,
  `dashed --ac-stroke` on `.absent`.
- Sole consumer:
  `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.tsx:99-119`.
  The card body is `<Link>` containing `cardHeader` → `<PipelineDots>`
  → `cardMeta` (which holds the `${score} of ${WORKFLOW_PIPELINE_STEPS.length}
  stages` counter and `formatMtime`). **No `.ac-lcard__pipe` row
  exists today**; the story's AC-2 requires introducing that flex
  row plus a counter rendering `${present.length}/8`.
- Data source: `useQuery({ queryKey: queryKeys.lifecycle(), queryFn:
  fetchLifecycleClusters })` (`LifecycleIndex.tsx:50-53`); each
  `cluster.completeness` is a `Completeness` value.
- Migration: delete the directory, replace with
  `frontend/src/components/Pipeline/` and
  `frontend/src/components/PipelineMini/`; rewrite the LifecycleIndex
  card markup to introduce `.ac-lcard__pipe` (or the BEM equivalent in
  CSS modules).

### LifecycleClusterView injection point

- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.tsx`
  — success-branch JSX:

  ```tsx
  <Page eyebrow={<>LIFECYCLE</>} title={cluster.title} subtitle={cluster.slug}>
    <Link to="/lifecycle" className={styles.backLink}>← All clusters</Link>
    <ol className={styles.timeline}>
      {WORKFLOW_PIPELINE_STEPS.map(step => renderStage(step, cluster.entries))}
    </ol>
    {LONG_TAIL_PIPELINE_STEPS.some(...) && (
      <section className={styles.longTail} ...>
        <h3>Other artifacts</h3>
        <ol className={styles.timeline}>
          {LONG_TAIL_PIPELINE_STEPS.map(step => renderStage(step, cluster.entries))}
        </ol>
      </section>
    )}
  </Page>
  ```

- `cluster.completeness` is already in scope via
  `useQuery({ queryKey: queryKeys.lifecycleCluster(slug), queryFn: () =>
  fetchLifecycleCluster(slug) })` (`LifecycleClusterView.tsx:19-22`).
- Insertion point for the new `Pipeline` panel: between line 53 (back
  link close) and line 55 (workflow `<ol>` open). No panel container
  exists today; the closest pattern is `.entryCard`
  (`LifecycleClusterView.module.css:80-85` — `1px solid var(--ac-stroke-soft)`,
  `var(--ac-bg-card)` background, `var(--radius-sm)` radius). The
  story's requirement (`var(--ac-bg-sunken)` + `1px solid var(--ac-stroke)`
  + `border-radius: 6px` + uppercase mono "Pipeline" eyebrow) inherits
  from the prototype, not from `.entryCard`. `--ac-bg-sunken` already
  exists in production tokens (`frontend/src/styles/global.css:80-208`).
- Tokens already in use on this page: `--ac-stroke-soft`,
  `--ac-stroke`, `--ac-bg-sunken`, `--ac-bg-card`, `--ac-accent`,
  `--ac-fg-{strong,muted,faint}`, `--ac-err`, `--sp-1..4`,
  `--radius-sm`, `--size-{xxs,xs}`.
- The chain renders `WORKFLOW_PIPELINE_STEPS` only; long-tail stages
  stay in the "Other artifacts" section unchanged — this matches the
  existing split.

### Kanban WorkItemCard (the surface needing the most rework)

- `skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx:15-58`
  — current JSX:

  ```tsx
  <li className={styles.card} data-relpath={entry.relPath}>
    <Link ref={setNodeRef} to="/library/$type/$fileSlug" ...>
      <div className={styles.cardHeader}>
        <span className={number !== null ? styles.cardNumber : styles.cardSlug}>{idChip}</span>
        <span className={styles.cardMtime}>{formatMtime(entry.mtimeMs, now)}</span>
      </div>
      <p className={styles.cardTitle}>{entry.title}</p>
      {kindLabel !== null && <p className={styles.cardKind}>{kindLabel}</p>}
    </Link>
  </li>
  ```

  ID rendering today (`WorkItemCard.tsx:26-32`):

  ```ts
  const number = parseWorkItemId(entry.relPath)
  const fileSlug = fileSlugFromRelPath(entry.relPath)
  const idChip = number !== null
    ? `#${String(number).padStart(4, '0')}`
    : fileSlug
  ```

  The fallback to the file slug is not what AC-10 wants — it wants the
  slot omitted when `entry.workItemId == null`.

- `parseWorkItemId` lives at
  `skills/visualisation/visualise/frontend/src/api/work-item.ts:4-10`,
  regex `/^(\d+)-/`. Other consumers: only the kanban card itself plus
  its unit tests. `announcements.ts` defines its own parallel parser
  `workItemIdFromRelPath` (`/(\d{4})-/`) — that file is unaffected by
  the AC-7 switch to `entry.workItemId`. The story's Technical Notes
  permit `parseWorkItemId` to stay in the codebase for the
  announcements path; we should treat the kanban card switch as
  a single-call-site change.
- Structural gap vs the prototype: **no `__foot` exists**. The current
  card has the mtime inside `.cardHeader`. To satisfy AC-7 ("N linked"
  + mtime inside `.ac-kcard__foot") the implementer must:
  1. Split the card body into `__top` (ID + PipelineMini) and `__foot`
     (mtime + optional "N linked"); move mtime out of cardHeader.
  2. Decide where `__title` and `__kind` sit — the prototype puts them
     between `__top` and `__foot`, no `__kind` exists in the prototype
     (the story says explicitly that production keeps it).
- The card is rendered inside a dnd-kit sortable; the link must keep
  carrying `aria-roledescription="sortable"`, `data-relpath`, the
  transform/transition styles, and the listeners.

### Server-side IndexEntry, Completeness, and cluster derivation

- `IndexEntry` —
  `skills/visualisation/visualise/server/src/indexer.rs:161-180`. Fields
  today: `type, path, relPath, slug, workItemId, title, frontmatter,
  frontmatterState, workItemRefs, mtimeMs, size, etag, bodyPreview`
  (camelCase via `#[serde(rename_all = "camelCase")]`). **No
  `completeness`, no `linkedCount`** — both are new.
- `Completeness` —
  `skills/visualisation/visualise/server/src/clusters.rs:8-22`. Eleven
  boolean fields (`hasWorkItem … hasDesignGap`). **No `present`** —
  it's new.
- `derive_completeness` (`clusters.rs:101-133`) does
  `match e.r#type { DocTypeKey::WorkItems => c.has_work_item = true,
  … }` — explicit per-`DocTypeKey` arms, with `WorkItemReviews` and
  `Templates` matched-and-ignored. The canonical stage→DocTypeKey
  mapping lives only here (no parallel `WORKFLOW_PIPELINE_STEPS`
  constant on the server side).
- `compute_clusters` (`clusters.rs:34-67`) groups entries by
  `e.slug.clone()`, calls `derive_completeness(&entries)`, sorts within
  cluster by `canonical_rank` then mtime. Called once at server
  startup (`server.rs:89`) and again from the watcher after every fs
  event (`watcher.rs:153-154`).
- Plan for `Completeness.present`: derive at the end of
  `derive_completeness` by inspecting each boolean and pushing the
  matching stage key — but the key namespace on the wire today is
  `hasWorkItem` / `hasResearch` / … (the boolean field names). The
  story's worked example uses `present: ["work", "adr"]` — a *third*
  namespace (the prototype's `STAGES` keys). The implementer must
  pick: stage keys (matches the prototype, requires the new
  Pipeline component to consume them), or the existing `hasX` keys
  (matches today's `Completeness` field names, requires the test
  fixtures to use them). Either way the choice must be coherent with
  `PipelineStepKey` in `types.ts:207-210`, which already uses the
  `hasX` form.
- Plan for `IndexEntry.completeness`: in the indexer build, materialise
  a `HashMap<String, Completeness>` keyed by cluster slug as a
  side-effect of `compute_clusters` (or run a second pass that consumes
  the freshly-built clusters), then back-fill `completeness =
  cluster_completeness.get(&entry.slug).cloned()` onto every entry
  whose `slug.is_some()`. Orphan entries (`slug.is_none()`) get
  `completeness = None`, which serialises to `null` — exactly the
  orphan signal the kanban card needs (`entry.completeness == null`).

### `/api/related` and the cost of computing `linkedCount` at index time

- `skills/visualisation/visualise/server/src/api/related.rs:31-110`.
  Response shape: `RelatedArtifactsResponse { inferredCluster,
  declaredOutbound, declaredInbound }`.
- Resolution helpers, all in `indexer.rs`:
  - `Indexer::declared_outbound(&self, entry: &IndexEntry) -> Vec<IndexEntry>`
    (`indexer.rs:637`) — reads `self.entries` and
    `self.work_item_by_id`.
  - `Indexer::reviews_by_target(&self, target: &Path) -> Vec<IndexEntry>`
    (`indexer.rs:606`) — reads `self.entries` and the
    `reviews_by_target` reverse index.
  - `Indexer::work_item_refs_by_id(&self, id: &str) -> Vec<IndexEntry>`
    (`indexer.rs:622`) — reads `self.entries` and
    `work_item_refs_by_target` reverse index.
  - `inferredCluster` is computed inline (`related.rs:56-71`) by
    scanning `state.clusters.read().await` for the entry whose `slug`
    matches and filtering out self.
- **No filesystem reads in any of the three resolutions** beyond the
  one `std::fs::canonicalize` fallback in `Indexer::get`
  (`indexer.rs:579`), which is bypassable. Calling these per-entry
  during indexer build is O(1)/O(k) per entry against already-built
  hashmaps; for N≈thousands of entries it's still cheap.
- **Dedup story**: `related_get` dedupes `inferredCluster` against the
  union of declared paths (`related.rs:94-102`). Within each declared
  bucket dedup is enforced. **Declared outbound vs declared inbound
  are *not* deduped against each other** — the same artifact can
  appear in both, in which case the AC's arithmetic
  (`inferredCluster.length + declaredOutbound.length + declaredInbound.length`)
  will count it twice. This matches the endpoint, so consumers (the
  card and any future caller of /api/related) see the same number; but
  if dedup is what users actually expect, the spec needs revisiting.
  The story explicitly commits to the three-bucket sum so this is the
  contract.
- Plan for `IndexEntry.linkedCount`: extract the three-bucket sum into
  a shared helper (e.g.
  `compute_linked_count(entries, work_item_by_id, reviews_by_target,
  work_item_refs_by_target, clusters, entry) -> usize`), call it from
  both `related_get` (replacing the current inline summing) and from
  the indexer's post-cluster pass. This is the "same resolution
  function" requirement of AC-6.

### Frontend types and stage constants

- `IndexEntry` (`api/types.ts:78-94`) already exposes `workItemId:
  string | null` at line 85; this is exactly what AC-9/10 want the
  kanban card to consume.
- `Completeness` (`api/types.ts:175-187`) has only the eleven booleans.
  Adding `present: string[]` is a typed-field addition with no
  optionality — the server is the only producer.
- `LIFECYCLE_PIPELINE_STEPS` (`api/types.ts:212-242`) — master list of
  11 entries, each with `key`, `docType`, `label`, `placeholder`,
  optional `longTail: true`. Filtered into:
  - `WORKFLOW_PIPELINE_STEPS` (8 entries, all `!longTail`) —
    `types.ts:244-246`.
  - `LONG_TAIL_PIPELINE_STEPS` (3 entries, `notes`, `design-inventories`,
    `design-gaps`) — `types.ts:248-250`.
  **No `hue` field** on the production stage records — hues live only
  in the prototype's `STAGES` constant. Adding hue metadata to the
  production constant (or to a sibling constant on the new `Pipeline`
  component) is a planning-phase decision.
- `PipelineStepKey` (`types.ts:207-210`) is a union of the eleven
  `hasX` strings — currently the only "stage key" namespace on the
  frontend.
- React-query keys (`api/query-keys.ts:46-52`):
  `queryKeys.docs(type)`, `queryKeys.lifecycle()`,
  `queryKeys.lifecycleCluster(slug)`,
  `queryKeys.lifecycleClusterPrefix()`. SSE invalidation in
  `use-doc-events.ts:107-108` invalidates both `lifecycle()` and the
  cluster-prefix on doc events — this guarantees the cross-surface
  consistency AC ("re-fetched via its production data hook") is honest:
  the kanban side gets a fresh `fetchDocs('work-items')` invalidation
  on the same event the lifecycle side gets `fetchLifecycleClusters`.

### 2026-05-21 design prototype (Pipeline / PipelineMini source)

The authoritative prototype reference lives in-tree at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
— a 2.6 MB self-contained bundle with all JSX, CSS, fonts, and assets
inlined. The inventory directory also holds `inventory.md` and the
`screenshots/` folder. The component-level details below were
originally extracted from the unbundled JSX checkout at
`~/Downloads/Accelerator/src/` (`view-lifecycle.jsx`, `view-kanban.jsx`,
`ui.jsx`, `app.css`, `data.jsx`), but every CSS rule and class hook
named below is present verbatim inside the standalone HTML (`grep
'ac-hexchain'`, `grep 'ac-stagedot'`, etc. all hit), so the in-tree
bundle is sufficient for the implementer.

- **HexChain** (→ `Pipeline`) — `view-lifecycle.jsx:28-49`. Props:
  `present` (array of stage keys), `stages` (defaults to
  `window.STAGES`), `size` (defaults to 28). Per stage: wrapper `<div
  className="ac-hexchain__stage on?">` with inline `color: hsl(hue 68%
  46%)` and `--chain-tile-h: round(size/2)px`; nested `StageTile` (a
  square 6px-radius tile with the per-doc-type SVG glyph at
  `round(size*0.16)` padding); a label below the tile in mono 9.5px;
  and a connector behind the next tile.
- **Connector colouring** (lines 28-49): for each stage at index `i`,
  the link is rendered only when `i < stages.length - 1`. Background
  is `(on && nextOn) ? hsl(stage.hue 68% 46%) : var(--ac-stroke)`.
  Must be computed in JSX because the colour depends on the next
  stage's state in `WORKFLOW_PIPELINE_STEPS` order (pure CSS sibling
  selectors don't have a "two-ahead" combinator).
- **Cluster-detail panel** (`view-lifecycle.jsx:134-137`): inline
  `padding: 16px 20px`, `background: var(--ac-bg-sunken)`, `border: 1px
  solid var(--ac-stroke)`, `borderRadius: 6, marginBottom: 8`;
  eyebrow `<div>` with `var(--ac-font-mono)`, `fontSize: 10.5`,
  `color: var(--ac-fg-faint)`, `letterSpacing: 0.1em`,
  `textTransform: uppercase`, `marginBottom: 14`, text "Pipeline";
  then `<HexChain present={cluster.present} size={34} />`.
- **lcard pipe row** (`view-lifecycle.jsx:85-90`): flex row containing
  `<HexChain size={26}>` and a `<div class="mono faint">` counter
  rendering `${present.length}/${STAGES.length}` (i.e. N/8).
- **StageDots** (→ `PipelineMini`) — `ui.jsx:98-117`. Props: `present`,
  `stages`, `compact` (bool). Size 6/8px (compact/regular), gap 4/6.
  Each dot is a `<span>` with inline
  `background: on ? hsl(s.hue 72% 56%) : transparent`,
  `borderColor: on ? hsl(s.hue 72% 56%) : var(--ac-stroke)`. Halo on
  `.on` from CSS: `box-shadow: 0 0 0 2px color-mix(in oklab,
  currentColor 20%, transparent)` (`app.css:666-674`).
- **STAGES** (`data.jsx:43-52`): eight entries with `key`, `short`,
  `label`, `hue` — `work/0`, `research/28`, `plans/220`,
  `plan-reviews/260`, `validations/160`, `pr-descriptions/200`,
  `pr-reviews/280`, `decisions/355`. **The prototype's stage keys
  differ from the production `PipelineStepKey` union** (which uses
  `hasWorkItem` / `hasResearch` / …). The implementer must pick one
  vocabulary and stick with it; the story uses the prototype's
  short-key form (`["work"]`, `["work","adr"]`) in its worked example
  — but ADR `adr` is *not* a STAGES key in the prototype either
  (decisions is `decisions`). This is a small inconsistency in the
  story's example values that warrants a planning-phase clarification.
- **kcard markup** (`view-kanban.jsx:3-22`): the prototype's structure
  is `__top` (id + StageDots compact) → `__title` → `__slug` → `__foot`
  (`__links` + `__mtime`). **The prototype has no `__kind`** — the
  story's Drafting Notes explicitly say the production card keeps the
  `frontmatter.kind` label and the prototype will be updated, so this
  is acknowledged divergence.
- **Per-stage hue formulas** confirmed: `hsl(stage.hue 68% 46%)` for
  tile fill/border (`view-lifecycle.jsx:6`,
  `view-lifecycle.jsx:14-15`); `hsl(s.hue 72% 56%)` for active dots
  (`ui.jsx:110-111`). Off-state tile uses
  `hsl(stage.hue 78% 95%)` background + `hsl(stage.hue 40% 82%)`
  border. Dark theme override
  (`app.css:1128-1131`) recomputes off-state with `color-mix(in oklab,
  currentColor 14%, transparent)`. No CSS variable for these — pure
  inline JSX.

### `formatDocId` and the LibraryTypeView precedent

- `skills/visualisation/visualise/frontend/src/routes/library/doc-type-id.ts:1-12`:

  ```ts
  export function formatDocId(workItemId: string | null | undefined): string {
    if (!workItemId) return ''
    const match = workItemId.match(/^([^-]+)-(\d+)$/)
    if (!match) return workItemId
    const [, prefix, digits] = match
    return `${prefix}-${digits.padStart(4, '0')}`
  }
  ```

  Null/empty input → `''`. No-prefix input → returned verbatim. The
  function is `prefix-zeropaddedDigits`; an input like `0042` will
  fail the regex (no `-`) and fall through to the verbatim branch.
- Call site: `LibraryTypeView.tsx:33-44` (`firstColumnContent`)
  returns `{kind: 'id', value: formatDocId(entry.workItemId)}` when
  `entry.workItemId` is truthy. Rendered at lines 231-246 as
  `{first.kind === 'empty' ? '—' : first.value}` — **note: the library
  view does *not* omit the column when workItemId is null**; it falls
  through to a date branch and finally an em-dash. The story's AC-10
  asks the kanban card to *omit* the slot — a different behaviour from
  this precedent. Either pattern is fine, but the implementer should
  know they're not just copying the library behaviour.

### Server work.id_pattern extraction

- `skills/visualisation/visualise/server/src/config.rs:111-128`
  (`WorkItemConfig::extract_id`):

  ```rust
  pub fn extract_id(&self, filename: &str) -> Option<String> {
      if let Some(cap) = self.scan_regex.captures(filename) {
          let digits = cap.get(1)?.as_str();
          return Some(match &self.default_project_code {
              Some(code) => format!("{code}-{digits}"),
              None => digits.to_string(),
          });
      }
      let code = self.default_project_code.as_deref()?;
      let dash = filename.find('-')?;
      let prefix = &filename[..dash];
      if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_digit()) {
          return None;
      }
      Some(format!("{code}-{prefix}"))
  }
  ```

- Inputs: filename only (no frontmatter). Outputs:
  `Some("CODE-DIGITS")` when scan_regex matches, or `None`.
- Fallback path requires both a configured `default_project_code` *and*
  an all-digits pre-dash prefix.
- A workspace with `scan_regex = "^([0-9]+)-"` (the default for
  numeric-only filenames) and no project code returns
  `Some("0042")` for `0042-foo.md` and `None` for `ENG-0042-foo.md`.
  A workspace with `scan_regex = "^[A-Z]+-([0-9]+)-"` and
  `default_project_code = "PROJ"` returns `Some("PROJ-0042")` for
  `ENG-0042-foo.md` — note the captured code is *not* `ENG`; it's the
  project code from config. To get verbatim multi-prefix behaviour,
  the scan_regex must capture the *whole* `PREFIX-NUMBER` form, e.g.
  `^([A-Z]+-[0-9]+)-`.
- This is exactly the gap that makes Assumption #2 of the work item
  subtly fragile: "the server's existing extraction logic correctly
  handles any prefix matching the pattern" is true only if the
  workspace's `scan_regex` is configured to capture the prefix. The
  default is *not* configured that way. For AC-9 ("`work_item_id:
  \"ENG-0042\"` frontmatter, filename without `ENG` prefix → displayed
  ID is exactly `ENG-0042`") to hold, *either*:
  (a) the test workspace's `scan_regex` matches the filename; *or*
  (b) extraction must consult `frontmatter['work_item_id']` first.
  The current code does (a) only. AC-9 reads as if it's testing (b).
  Worth resolving in planning.

### Tests, fixtures, and the migration footprint

**Files to migrate, in priority order:**

1. `frontend/src/components/PipelineDots/PipelineDots.test.tsx` —
   delete; split into `Pipeline.test.tsx` and `PipelineMini.test.tsx`.
   Existing assertions to retire: `data-present`, per-dot `title`,
   per-dot `aria-label`. New: `data-active="true|false"` only.
2. `frontend/src/routes/kanban/WorkItemCard.test.tsx` — heavy diff:
   - Retire `#0001`/`#0029` numeric tests (use namespaced IDs via
     `entry.workItemId` instead).
   - Retire "falls back to the file slug" test (slot is omitted when
     `workItemId == null` instead).
   - Add: PipelineMini rendered as first child of `.ac-kcard__top`
     when `entry.completeness != null`; omitted on orphan.
   - Add: "{N} linked" inside `.ac-kcard__foot` when `linkedCount > 0`,
     omitted on `linkedCount === 0`.
   - Add: namespaced ID renders verbatim regardless of relPath prefix.
   - Keep: kind label, sortable role-description, data-relpath,
     library link href.
3. `frontend/src/routes/lifecycle/LifecycleIndex.test.tsx` — replace
   the "8 dots" test with "Pipeline + counter present.length/8".
   Fixtures must gain `present: string[]` on every completeness
   literal.
4. `frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx` — add
   a Pipeline-panel test (eyebrow text, eight tiles, `data-active`
   reflecting `cluster.completeness.present`); fixtures gain
   `present: string[]`.
5. `frontend/src/api/test-fixtures.ts:6-23` — `makeIndexEntry` must
   default new fields: `completeness: null` (orphan-safe),
   `linkedCount: 0`. Otherwise every existing call site will need
   overrides.
6. Three duplicated `empty: Completeness` literals (PipelineDots:6-11,
   LifecycleIndex:10-15, LifecycleClusterView:12-17). Either update
   all three or introduce a shared `makeCompleteness()` factory.

**No MSW or in-flight mocking infrastructure** in any of these tests
— all use `vi.spyOn(fetchModule, ...)`. The Wrapper pattern (fresh
QueryClient + MemoryRouter) is duplicated inline in both lifecycle
tests; reusable verbatim for new tests.

### Tokens and dependencies

- Production tokens:
  `skills/visualisation/visualise/frontend/src/styles/global.css:80-208`
  (`:root`), `:314-365` (`[data-theme="dark"]`), with
  `--ac-bg-sunken`, `--ac-stroke`, `--ac-stroke-soft`, `--ac-bg-card`,
  `--ac-accent`, `--ac-fg-*` all present. Per-doc-type:
  `--ac-doc-<kind>` + `--ac-doc-bg-<kind>` for every DocTypeKey.
- **No `--ac-stage-*` tokens exist anywhere**. The closest analogue
  is the `--ac-doc-*` family. Introducing per-stage tokens (as the
  story's Technical Notes flag) is a 0033 follow-up; for this story,
  hard-coded HSL formulas matching the prototype are acceptable.
- Glyph (dep 0037):
  `skills/visualisation/visualise/frontend/src/components/Glyph/` —
  `Glyph.tsx`, `Glyph.constants.ts`, `Glyph.module.css`,
  `Glyph.test.tsx`, plus 13 per-DocTypeKey icon files. Reusable as
  the per-stage icon source if the `Pipeline` tiles render glyphs
  (likely, per the prototype's `StageTile`).
- Chip (dep 0038):
  `skills/visualisation/visualise/frontend/src/components/Chip/` —
  not directly needed by `Pipeline` / `PipelineMini` but useful for
  the cluster-detail eyebrow if framed.

## Code References

### Frontend (skills/visualisation/visualise/frontend/src/)

- `components/PipelineDots/PipelineDots.tsx:1-27` — the component being replaced.
- `components/PipelineDots/PipelineDots.module.css:1-41` — current dot styles.
- `components/PipelineDots/PipelineDots.test.tsx:1-43` — test to split.
- `routes/lifecycle/LifecycleIndex.tsx:99-119` — current PipelineDots consumer; site of `.ac-lcard__pipe` introduction.
- `routes/lifecycle/LifecycleIndex.tsx:50-53` — `useQuery` source for cluster data.
- `routes/lifecycle/LifecycleClusterView.tsx:51-76` — insertion point for `Pipeline` panel (between line 53 back link and line 55 timeline open).
- `routes/lifecycle/LifecycleClusterView.tsx:19-22` — `useQuery` for cluster data.
- `routes/lifecycle/LifecycleClusterView.module.css:80-85` — `.entryCard` panel pattern (precedent).
- `routes/kanban/WorkItemCard.tsx:15-58` — full component; the JSX restructure target.
- `routes/kanban/WorkItemCard.tsx:26-32` — current `parseWorkItemId` ID rendering (to be removed in favour of `entry.workItemId` + `formatDocId`).
- `routes/kanban/WorkItemCard.module.css:1-27` — current card styles (no `__foot`).
- `routes/kanban/announcements.ts:10-13` — `workItemIdFromRelPath` (independent helper, untouched by this story).
- `api/work-item.ts:4-10` — `parseWorkItemId` (kept in codebase for announcements path).
- `api/types.ts:78-94` — `IndexEntry` (gains `completeness`, `linkedCount`).
- `api/types.ts:85` — `workItemId: string | null` (already present).
- `api/types.ts:175-187` — `Completeness` (gains `present: string[]`).
- `api/types.ts:189-199` — `LifecycleCluster`.
- `api/types.ts:207-210` — `PipelineStepKey` union.
- `api/types.ts:212-242` — `LIFECYCLE_PIPELINE_STEPS` master list.
- `api/types.ts:244-246` — `WORKFLOW_PIPELINE_STEPS` (8 stages).
- `api/types.ts:248-250` — `LONG_TAIL_PIPELINE_STEPS` (3 stages).
- `api/fetch.ts:66-71` — `fetchDocs`.
- `api/fetch.ts:116-121` — `fetchLifecycleClusters`.
- `api/query-keys.ts:46-52` — react-query keys.
- `api/use-doc-events.ts:107-108` — SSE invalidation across both queries.
- `api/test-fixtures.ts:6-23` — `makeIndexEntry` factory.
- `routes/library/doc-type-id.ts:1-12` — `formatDocId`.
- `routes/library/LibraryTypeView.tsx:33-44` — `formatDocId` call-site precedent.
- `styles/global.css:80-208` — token definitions (no `--ac-stage-*`).
- `components/Glyph/Glyph.tsx` — Glyph component (dep 0037).

### Server (skills/visualisation/visualise/server/src/)

- `indexer.rs:161-180` — `IndexEntry` struct (gains `completeness`, `linked_count`).
- `indexer.rs:574-668` — `Indexer::{get, declared_outbound, reviews_by_target, work_item_refs_by_id}` (resolution helpers reusable for `linkedCount`).
- `indexer.rs:1009-1083` — `build_entry` (the per-file entry builder; new fields plumb in here or in a post-pass).
- `indexer.rs:1032-1041` — `work_item_cfg.extract_id(filename)` call site.
- `clusters.rs:8-22` — `Completeness` struct (gains `present`).
- `clusters.rs:24-32` — `LifecycleCluster` struct.
- `clusters.rs:34-67` — `compute_clusters` (the post-build pass; back-fills `IndexEntry.completeness`).
- `clusters.rs:69-85` — `canonical_rank` (in-cluster sort, unrelated to stage keys but useful context).
- `clusters.rs:101-133` — `derive_completeness` (where `present` would be materialised).
- `api/related.rs:16-22` — `RelatedArtifactsResponse`.
- `api/related.rs:31-110` — `related_get` (the three-bucket assembler; shared resolution helper extracted from here).
- `api/lifecycle.rs:17-35` — list and detail endpoints (already returning camelCase clusters).
- `api/docs.rs:25-41` — docs list endpoint (already returning IndexEntry[]; auto-picks up new fields).
- `config.rs:49-71` — `RawWorkItemConfig`/`WorkItemConfig` definitions.
- `config.rs:58-60` — `default_id_pattern` (`"{number:04d}"`).
- `config.rs:111-128` — `WorkItemConfig::extract_id` (relevant to AC-9 prefix-fidelity).
- `server.rs:89` — initial `compute_clusters(&indexer.all().await)` seed.
- `watcher.rs:153-154` — watcher re-runs `compute_clusters` after every fs event.

## Architecture Insights

- **Single source of truth for the stage→DocTypeKey map**: today it
  lives in `derive_completeness` (Rust) and in `LIFECYCLE_PIPELINE_STEPS`
  (TypeScript) — these are kept in sync by convention, not by code.
  The new `present: string[]` field gives the frontend a server-derived
  view of the same data, which removes one duplication risk: the
  client no longer has to know how booleans map to stage keys; the
  server tells it.
- **Indexer is the single producer of `Completeness` and (newly) of
  `linkedCount`**. The watcher rebuilds clusters after every fs event,
  so the SSE-driven invalidation already triggers a re-read of both
  surfaces — meaning the cross-surface consistency AC ("`["work"]` →
  `["work","adr"]`") works out of the box once `present` and
  `completeness` plumb through.
- **Performance**: the indexer build is O(N) over entries, the cluster
  build is O(N) over entries plus O(C) clusters. Adding `linkedCount`
  per entry adds O(N · k) where k is the average number of hashmap
  lookups per entry — still O(N) overall and dwarfed by the file I/O
  in Phase 2.
- **Class-name conventions are inconsistent today**: production uses
  CSS-module camelCase class keys exclusively (no BEM, no `ac-`
  prefix). The prototype uses BEM (`ac-kcard__top`, `ac-lcard__pipe`,
  `ac-hexchain__stage`). The work item's ACs commit to the BEM names.
  Introducing BEM-style hooks on production cards is a small but
  *visible* convention change that planning should call out — either
  globally (the rest of the visualiser eventually moves to BEM) or as
  a local exception for the kanban/lifecycle cards. The `ac-` prefix
  is already used for tokens, so adding it as a class hook is a small
  scope drift.
- **The 2026-05-21 prototype is in-tree as a self-contained bundle**
  at `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`.
  All CSS rules and class hooks referenced above are inlined in that
  file. The unbundled JSX checkout at `~/Downloads/Accelerator/src/`
  is a convenience for browsing component-level source but is not
  required to implement the story.

## Historical Context

- `meta/decisions/ADR-0025-work-item-cross-ref-aggregation.md` — sets
  the model for `/api/related` and the three-bucket shape that
  `linkedCount` consumes. AC-6 of the story is implicitly coupled to
  this ADR.
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` and the
  unfinished 0061 — describe the next iteration of the linkage model.
  The story's Dependencies section notes that any change to the
  three-bucket shape under those work items must consider this
  consumer.
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` and
  the in-flight 0057 — touch the frontmatter contract that feeds
  `IndexEntry`. `workItemId` extraction sits adjacent: 0057 may
  consolidate where the ID is sourced (frontmatter vs filename),
  which would unblock AC-9 cleanly.
- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
  — prior look at the same `/api/related` resolution graph; confirms
  the hashmap-lookup hot path.
- `meta/research/codebase/2026-05-12-0037-glyph-component.md` and
  `meta/plans/2026-05-12-0037-glyph-component.md` — the Glyph
  component the Pipeline tiles will likely render inside.
- `meta/research/codebase/2026-05-14-0038-generic-chip-component.md` —
  Chip component is dep but probably not directly used by Pipeline.
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — the gap analysis that produced this story.
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md`
  — the inventory (screenshots + HTML bundle) referenced by the work
  item.
- `meta/reviews/work/0040-pipeline-visualisation-overhaul-review-1.md`
  — three-pass review. Major findings (cross-surface consistency
  probe, test-migration concreteness) were all resolved; the work item
  was manually approved on 2026-05-31 with `present: string[]`
  promoted to a first-class wire field, the orphan predicate
  standardised on `entry.completeness == null`, and 0044 promoted to
  a "blocked by (kanban portion only)" dependency.
- `meta/work/0044-spike-list-screen-scope-decisions.md` — open spike;
  blocks kanban portion of 0040.
- `meta/work/0063-rename-work-item-type-to-kind.md` — done; provides
  the `frontmatter.kind` label the kanban card preserves.
- `meta/plans/2026-04-26-meta-visualiser-phase-7-kanban-read-only.md` —
  origin of the current kanban implementation; useful for the chrome
  layout context.

## Open Questions

These are decisions for planning (not blockers for implementation):

1. **Stage-key namespace for `Completeness.present`**: prototype short
   keys (`work`, `research`, `plans`, …) or production `PipelineStepKey`
   form (`hasWorkItem`, `hasResearch`, …)? The story's worked example
   uses `["work","adr"]`; production `PipelineStepKey` uses `hasX` and
   `LIFECYCLE_PIPELINE_STEPS` exposes `key` in `hasX` form. The new
   server-derived `present` field's element type needs to be pinned
   before frontend wiring begins.
2. **Class-name convention**: introduce BEM `.ac-kcard__top` etc.
   alongside the existing CSS-module camelCase, or retarget the
   work-item ACs at the existing class shape? The ACs commit to BEM;
   planning should pin whether this is a local exception or a wider
   convention shift.
3. **AC-9 source of `workItemId`**: today, extraction is filename-only
   and project-code-bound. For the "frontmatter `work_item_id:
   \"ENG-0042\"` with a non-matching filename" scenario to work,
   `WorkItemConfig::extract_id` must learn to consult frontmatter
   first. Either widen extraction in this story, or constrain AC-9 to
   the filename-derived case. Story Assumption #2 papers over this
   gap.
4. **Declared inbound/outbound dedup**: AC-6 commits to the three-bucket
   sum, which can double-count an artifact that appears in both
   declared buckets. If consumers expect a unique-count, the spec and
   the endpoint both need a dedup step. Worth confirming.
5. **Whether `Pipeline` tiles render Glyphs**: the prototype's
   `StageTile` renders an SVG glyph keyed off `stage.key`. Production
   `Glyph` is per-DocTypeKey, not per-stage. If `Pipeline` uses
   Glyph, the stage→DocTypeKey map (already in
   `LIFECYCLE_PIPELINE_STEPS[i].docType`) becomes the glyph key
   source. Worth a planning decision: reuse Glyph (saves icon
   maintenance) vs inline per-stage SVG (simpler, no Glyph coupling).
6. **Per-stage tokens vs inline HSL**: the prototype hard-codes
   `hsl(hue 68% 46%)` / `hsl(hue 72% 56%)` formulas. The story flags
   `--ac-stage-<key>-on` as a 0033 follow-up. For this story, inline
   HSL is acceptable; planning can decide whether to ship the token
   follow-up as part of 0040 or split it.

## Related Research

- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
  — independent investigation of the `/api/related` resolution path.
- `meta/research/codebase/2026-05-12-0037-glyph-component.md` —
  Glyph component implementation (dep 0037).
- `meta/research/codebase/2026-05-14-0038-generic-chip-component.md` —
  Chip component implementation (dep 0038).
- `meta/research/codebase/2026-05-06-0033-design-token-system.md` —
  Design token system foundation (dep 0033).
- `meta/research/codebase/2026-05-20-0063-rename-work-item-type-to-kind.md`
  — `frontmatter.kind` rename that the kanban card preserves.
- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md`
  — foundational visualiser context including indexer + cluster
  architecture.
