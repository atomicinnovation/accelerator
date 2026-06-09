---
date: "2026-05-16T08:44:14+01:00"
author: Toby Clemson
revision: "60812eb5fda5696e4b71499da7f36545027ae2fb"
repository: accelerator
topic: "Work item 0041 — supplementary codebase research after work-item rewrite"
tags: [research, codebase, visualiser, library, page-wrapper, five-route-migration, prs-rename, indexer, library-structure-endpoint]
status: complete
last_updated: "2026-05-16T00:00:00+00:00"
last_updated_by: Toby Clemson
extends: "meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md"
type: codebase-research
id: "2026-05-16-0041-library-page-wrapper-supplementary"
title: "Research: Work Item 0041 — Supplementary Findings"
schema_version: 1
relates_to: ["codebase-research:2026-05-15-0041-library-page-wrapper-and-overview-hub", "work-item:0041", "adr:ADR-0024", "adr:ADR-0026"]
derived_from: ["codebase-research:2026-05-15-0041-library-page-wrapper-and-overview-hub", "codebase-research:2026-05-13-0055-sidebar-activity-feed", "codebase-research:2026-05-14-0038-generic-chip-component"]
---

# Research: Work Item 0041 — Supplementary Findings

**Date**: 2026-05-16T08:44:14+01:00
**Author**: Toby Clemson
**Git Commit**: 60812eb5fda5696e4b71499da7f36545027ae2fb
**Branch**: HEAD (jj change `stlzowrnoslm`; `main` at `f5c54374`)
**Repository**: accelerator (visualisation-system workspace)

## Research Question

The 2026-05-15 research pass (commit `77c4cfe1`) resolved eight open questions and drove a substantial work-item rewrite. The rewrite expanded scope to:

1. Migrate **five** `<main>`-padded routes to the new `Page` wrapper in this story (`KanbanBoard`, `LifecycleIndex`, `LibraryDocView`, `LibraryTemplatesView`, `LibraryTemplatesIndex`) — formerly only `KanbanBoard` was named.
2. Delete `PageSubtitle` outright (formerly a thin shim was planned).
3. Introduce a new `--ac-content-max-width` design token (formerly not specified).
4. Rename `prs` → `pr-descriptions` on the wire (formerly the rename surface was lightly sketched).
5. Strip `RootLayout.main`'s `padding: var(--sp-5) var(--sp-6)` rule (formerly out of scope).

This supplementary pass — run after zero code commits to the visualiser (only hooks/test changes have landed between `77c4cfe1` and `60812e b5`) — targets the gaps the prior pass did not cover in depth: a per-route migration audit, the additive rename surface beyond what was already enumerated, the indexer extension shape for the new `/api/library/structure` endpoint, and the test surface beyond `router.test.tsx` / `LibraryTypeView.test.tsx`.

The 2026-05-15 research doc remains the primary reference. Read it first; this doc is purely additive.

## Summary

- **Five-route migration is heterogeneous.** Two routes are trivial swaps (`LibraryTemplatesView`, `LibraryTemplatesIndex`), one is moderate (`KanbanBoard` — already on `PageSubtitle` with three call sites), and **two are non-obvious** (`LifecycleIndex` has no header today; `LibraryDocView` puts its header inside a two-column grid as `grid-area: header`).
- The current `max-width` literals are **not uniform**: `LibraryDocView` 1100px, `LibraryTemplatesView` 900px, `LibraryTypeView` 900px, `LifecycleIndex` 900px, `LibraryTemplatesIndex` 600px. The **600px** outlier in `LibraryTemplatesIndex` forces a per-route override mechanism on `Page` (a `maxWidth` prop or equivalent).
- `KanbanBoard`'s `.board { padding: var(--sp-4) }` rule will **double-pad** horizontally once `Page` owns horizontal padding. Drop it as part of the migration.
- The four routes carrying a `max-width` allowlist entry in `migration.test.ts` (`:122,126,129,154`) plus the one for `LibraryTypeView` (`:134`) are forced-atomic: removing the literal without removing the allowlist entry fails the test at `migration.test.ts:327-348`.
- **`migration.test.ts` also has a "title color" requirement block at `:394-412`** that asserts each route's `.title` block declares `color: var(--ac-fg-strong)`. Migrating titles into `Page` requires updating or removing this `REQUIRED` list — not previously noted.
- **The `prs` rename has wider exposure than enumerated.** New discoveries: 12 visual-regression PNG baselines (`glyph-showcase.spec.ts-snapshots`); 5 unit-test files asserting `hasPr: false` on `ClusterFlags`; 2 server-test fixtures (`config.valid.json`, `config.optional-override-null.json`); 2 server unit tests (`config_contract.rs:62`, `api_smoke.rs:27`); plus `e2e/start-server.mjs:69`, `scripts/write-visualiser-config.sh:60,158,184`, `scripts/test-launch-server.sh:75`, and `SKILL.md:19`.
- **The `hasPr` cluster-flag field name is not in the rename spec** but is co-located with the rename (server `ClusterFlags::has_pr` at `clusters.rs:16,108,208`; frontend `hasPr` consumers in 5 test files). Planning decision needed: rename to `hasPrDescription` for consistency, or keep `hasPr` since the field semantics are unchanged.
- **`config_path_key()` semantics**: per the work item, this returns `Some("prs")` after the rename. So all `doc_paths` config-key usages (JSON fixtures, `common/mod.rs`, `config_contract.rs`, `api_smoke.rs`, `start-server.mjs`, `write-visualiser-config.sh`, `test-launch-server.sh`, `SKILL.md:19`) **stay as `prs`** by spec. Only the wire serialisation, the Rust variant name, and the human label change.
- **`AppState` exposes `cfg` and `indexer` as `pub` fields** (`server/src/server.rs:40-51`) — sufficient for a new `library_structure` handler with no AppState changes.
- **No existing reducer precedent returns multiple aggregates per `DocTypeKey` in one pass.** `counts_by_type` (`indexer.rs:326-332`) is the only template, and `clusters.rs::compute_clusters` is the single-pass-with-snapshot precedent. A safer single-call `library_aggregates(...)` method on `Indexer` (one `entries.read().await`, computes count + latest + facet buckets together) avoids the multi-call race with rescan's `entries.write()`.
- **Caching is not warranted at v1 scale.** No existing pattern caches per-handler aggregates; only `state.clusters` is cached, and that is because it's already computed every rescan to feed the SSE pipeline.
- **`mtime_ms == 0` is a valid sentinel**, not an "unknown". `clusters.rs:52` uses `.max().unwrap_or(0)` and doesn't filter — follow that precedent in `latest_by_type`.
- **`WorkItemConfig::extract_id` returns the full ID** (`PROJ-0042`), not the prefix. `WorkItemConfig` has no project-prefix accessor. For the project facet, derive from `cfg.work_item.as_ref().and_then(|w| w.default_project_code.clone())` (mirrors `api/work_item_config.rs:18-23`) when `entry.work_item_id` has no `-`, else split on the first `-`. Multi-project handling is net-new.
- **Test-surface additions beyond the prior list are minimal**: `e2e/smoke.spec.ts:3-5` (regex still passes but title and intent should update); `tests/visual-regression/tokens.spec.ts:11,13,81` (overview-hub baseline will change for the bare `/library` capture).

---

## Detailed Findings

### A. Five-route migration to `Page` — per-route audit

The work item enumerates five routes that must migrate to `Page` in this story so that `RootLayout.main`'s `padding: var(--sp-5) var(--sp-6)` rule can be removed without visual regression. The five routes are heterogeneous; complexity ranges from trivial to non-obvious.

#### Current `<main>` rule

`frontend/src/components/RootLayout/RootLayout.module.css:13-17`:

```css
.main {
  flex: 1;
  overflow: auto;
  padding: var(--sp-5) var(--sp-6);
}
```

The `padding` declaration at line 16 is what 0041 removes. `flex: 1` and `overflow: auto` stay — they belong to viewport-fill + scrolling semantics, not page chrome.

#### Per-route summary

| Route | max-width (where) | Vertical rhythm | Header today | Subtitle/actions today | Branch count | Conflict with `Page` |
|---|---|---|---|---|---|---|
| `KanbanBoard` | none on `.board` | `.board { display: flex; flex-direction: column; gap: var(--sp-4); padding: var(--sp-4) }` (`KanbanBoard.module.css:1-6`) | `PageSubtitle` at three call sites (`:143` loading, `:152` error, `:183` success) | Success branch passes `<Chip variant="indigo">live</Chip>` as `PageSubtitle` child (`:184`) | 3 | `.board { padding: var(--sp-4) }` will **double-pad** horizontally once `Page` owns horizontal padding |
| `LifecycleIndex` | `900px` on `.container` (`LifecycleIndex.module.css:1`) | `.container` only has `max-width`; `.toolbar` has `margin-bottom: var(--sp-4)`; `.cardList` has `gap: var(--sp-4)` | **None.** No `<h1>`, no eyebrow. Top of return is `<div className={styles.container}><div className={styles.toolbar}>…filter + sort buttons…</div>` (`:72-87`) | None today — but the toolbar (filter input + 3 `SortButton`s) is the natural occupant of `Page`'s actions slot | 4 (loading / error / empty / success — three return raw `<p>` outside `.container`) | Four early-return branches return bare `<p>` outside the container; introducing `Page` means a title literal must be chosen and the wrapper threaded through every branch |
| `LibraryDocView` | `1100px` on `.article` (`LibraryDocView.module.css:6`) | CSS grid: `gap: var(--sp-5) var(--sp-6)`, `grid-template-areas: "header header" "body aside"`, `grid-template-columns: 1fr 260px` | Custom `<header>` (`:70-78`): `<h1 className={styles.title}>{entry!.title}</h1><FrontmatterChips frontmatter={...} state={...} />` — NOT `PageSubtitle` | `FrontmatterChips` directly under `<h1>` inside the grid's header row — semantically the subtitle slot. No eyebrow, no actions. | many — bare `<p>` early returns at `:48,51,54-58,62-66,67,68` | The custom header is a **grid area** inside a two-column grid. Lifting it into `Page` changes the layout from "header inside grid" to "Page above grid" — verify aside column alignment still works. `.article` width 1100px ≠ likely token canonical value. |
| `LibraryTemplatesView` | `900px` on `.container` (`LibraryTemplatesView.module.css:1`) | `.title { margin: 0 0 var(--sp-5) }`; `.tiers { gap: var(--sp-5) }` | Bare `<h1 className={styles.title}>{name}</h1>` (`:40-43`) | None. No eyebrow, no subtitle, no actions. | 3 (error / loading / success) | `.title { margin: 0 0 var(--sp-5) }` overlaps with `Page`'s header-to-content gap — drop after migration |
| `LibraryTemplatesIndex` | `600px` on `.container` (`LibraryTemplatesIndex.module.css:1`) | `.title { margin: 0 0 var(--sp-5) }`; `.list { gap: var(--sp-2) }` | Bare `<h1 className={styles.title}>Templates</h1>` (`:31-33`) | None. Title is the literal `"Templates"`. | 3 (error / loading / success) | The **`600px` max-width is narrower than any other route**, so this route must pass an override to `Page` (a `maxWidth` prop) to preserve current visual output |

#### Complexity assessment

| Route | Complexity | Rationale |
|---|---|---|
| `KanbanBoard` | moderate | Already on `PageSubtitle` with three call sites and a `<Chip>` child; mechanical swap, but the three branches all need updating and `.board`'s own `padding: var(--sp-4)` must be dropped |
| `LifecycleIndex` | **non-obvious** | No header today; introducing `Page` means choosing a title literal, deciding whether the toolbar belongs in the actions slot, and threading the wrapper through four early-return branches that currently render raw `<p>`s outside `.container` |
| `LibraryDocView` | **non-obvious** | Header is a grid area inside a two-column grid; lifting into `Page` changes the layout. `1100px` differs from likely token canonical value, so probably needs an override or the canonical-value decision drives it. Multiple bare-`<p>` early returns each need wrapper treatment. |
| `LibraryTemplatesView` | trivial | Bare `<h1>` swap; drop `.title { margin-bottom }` |
| `LibraryTemplatesIndex` | trivial-ish | Same shape as TemplatesView but `600px` is narrower — must override `maxWidth` on `Page` |

#### `migration.test.ts` allowlist — entries forced-atomic with this story

Each entry is `kind: 'irreducible'`. The allowlist enforcement at `migration.test.ts:327-348` ("declared count equals observed count — no stale entries, no over-count") fails if a literal is removed from the CSS without the corresponding `EXCEPTIONS` entry also being removed.

| Line | File | Literal | Reason |
|---|---|---|---|
| `:122` | `routes/library/LibraryDocView.module.css` | `1100px` | `article max-width — no token equivalent` |
| `:123` | `routes/library/LibraryDocView.module.css` | `260px` | `aside column width — no token equivalent` (stays; column width, not max-width) |
| `:126` | `routes/library/LibraryTemplatesIndex.module.css` | `600px` | `container max-width — no token equivalent` |
| `:129` | `routes/library/LibraryTemplatesView.module.css` | `900px` | `container max-width — no token equivalent` |
| `:134` | `routes/library/LibraryTypeView.module.css` | `900px` | `container max-width — no token equivalent` |
| `:147` | `routes/lifecycle/LifecycleClusterView.module.css` | `800px` | `max-width — no spacing-scale equivalent` (NOT in 0041 scope — `LifecycleClusterView` is not in the five-route list) |
| `:154` | `routes/lifecycle/LifecycleIndex.module.css` | `900px` | `container max-width — no token equivalent` |

Of these, **four** entries get cleaned up by 0041's migration (`:122, :126, :129, :134, :154`). `:123` (260px aside column) and `:147` (`LifecycleClusterView`, out of scope) stay.

`KanbanBoard.module.css` has **no** max-width allowlist entry to clean up.

#### `migration.test.ts` title-color requirement block — not previously called out

`migration.test.ts:394-412` declares a `REQUIRED` list asserting that each route's `.title` block must declare `color: var(--ac-fg-strong)`. Three of the five migrating routes are in this list:

- `routes/library/LibraryDocView.module.css` — `.title` requirement
- `routes/library/LibraryTemplatesView.module.css` — `.title` requirement
- `routes/library/LibraryTemplatesIndex.module.css` — `.title` requirement

Once `Page` owns title styling, these per-route `.title` rules go away and the `REQUIRED` list must be updated (entries removed) or the test will fail asserting "required rule absent".

#### Conflict-of-padding summary

Today, four of the five routes rely entirely on `RootLayout.main`'s `padding: var(--sp-5) var(--sp-6)` for horizontal padding. `KanbanBoard` adds its own `padding: var(--sp-4)` on top. `LibraryDocView` likewise relies on `.main`'s padding for outer inset around its grid.

After `Page` owns horizontal padding and `.main`'s padding is removed:
- `KanbanBoard.module.css:5` (`.board { padding: var(--sp-4) }`) must drop horizontal padding — keep only vertical or change to `padding: var(--sp-4) 0` or similar.
- The other four routes need no padding change (they had none of their own); they inherit `Page`'s padding.

### B. `prs` → `pr-descriptions` rename — additional surfaces beyond prior research

The 2026-05-15 doc enumerated the core surfaces. The following surfaces were NOT in that list. Group by category.

#### B.1 Server test fixtures and config

- `server/tests/fixtures/config.valid.json:20` — `"prs": "/abs/path/to/project/meta/prs",` (config doc_paths key)
- `server/tests/fixtures/config.optional-override-null.json:20` — `"prs": "/abs/path/to/project/meta/prs",`
- `server/tests/fixtures/meta/prs/42-add-config-layer.md:3` — `type: pr-description` (frontmatter — note: kebab singular, already differs from the wire token `prs` plural)
- `server/tests/fixtures/meta/prs/99-no-frontmatter-pr.md` — file in `meta/prs/` directory (stays)
- `server/tests/fixtures/meta/reviews/prs/2026-01-15-add-config-layer-review-1.md:2` — `target: "meta/prs/42-add-config-layer.md"` (path stays)

Per spec (`config_path_key()` still returns `Some("prs")`), the JSON config keys above **stay as `prs`**. Flagged for planning to confirm.

#### B.2 Server unit/integration tests

- `server/tests/common/mod.rs:58` — `doc_paths.insert("prs".into(), meta.join("prs"));`
- `server/tests/config_contract.rs:62` — `"prs",` (in expected `doc_paths` key list)
- `server/tests/api_smoke.rs:27` — `("prs", "prs"),` (config doc_paths tuple)

Likewise stay as `prs` per the `config_path_key()` semantics.

No `DocTypeKey::Prs` constructions in server test sources outside `clusters.rs` / `docs.rs` inline tests (already covered in prior research).

#### B.3 `ClusterFlags::has_pr` and frontend `hasPr`

The cluster-flag field name `has_pr` / `hasPr` was **not in the rename spec**, but is co-located:

Server (`server/src/clusters.rs`):
- `:16` — `pub has_pr: bool,` (struct field)
- `:108` — `has_pr: false,` (initialiser)
- `:208` — `assert!(!c.has_pr);` (inline test)

Frontend (test fixtures asserting `hasPr: false`):
- `frontend/src/router.test.tsx:93,190`
- `frontend/src/api/fetch.test.ts:140,329,346`
- `frontend/src/routes/lifecycle/LifecycleIndex.test.tsx:12`
- `frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx:14`
- `frontend/src/components/PipelineDots/PipelineDots.test.tsx:8`

**Planning decision needed**: rename `has_pr` → `has_pr_description` (and `hasPr` → `hasPrDescription`) for consistency with the human label, or keep `has_pr` since the semantics are unchanged? The work item does not specify. Recommendation: **rename for consistency** — the field's meaning is "this cluster has a PR description doc", and the new label is "PR descriptions", so `has_pr_description` is the consistent form.

#### B.4 Icon component name

- `frontend/src/components/Glyph/Glyph.tsx:11` — `import { PrsIcon } from './icons/PrsIcon'`
- `frontend/src/components/Glyph/icons/PrsIcon.tsx:3` — `export function PrsIcon(): ReactElement { … }`

Rename target: file `PrDescriptionsIcon.tsx`, exported function `PrDescriptionsIcon`. Tests at `Glyph.test.tsx` and `GlyphShowcase.test.tsx` iterate `GLYPH_DOC_TYPE_KEYS` rather than referencing the icon by name, so no test-side rename is needed beyond the icon mapping table at `Glyph.tsx:55`.

#### B.5 Visual-regression PNG baselines

12 snapshot files keyed by the doc-type wire token must rename:

```
frontend/tests/visual-regression/__screenshots__/glyph-showcase.spec.ts-snapshots/
  prs-16-light-visual-regression-darwin.png  →  pr-descriptions-16-light-visual-regression-darwin.png
  prs-24-light-…                              →  pr-descriptions-24-light-…
  prs-32-light-…                              →  pr-descriptions-32-light-…
  prs-16-dark-…                               →  pr-descriptions-16-dark-…
  prs-24-dark-…                               →  pr-descriptions-24-dark-…
  prs-32-dark-…                               →  pr-descriptions-32-dark-…
  (same 6 files for -linux variant)
```

`glyph-showcase.spec.ts` derives snapshot names from `GLYPH_DOC_TYPE_KEYS` — once the union member renames, the spec will look for `pr-descriptions-*` baselines. The 12 files can either be renamed via `git mv`/`jj mv` or regenerated via `playwright test --update-snapshots`.

#### B.6 E2E and helper scripts (config-key references — stay per spec)

- `frontend/e2e/start-server.mjs:69` — `prs: join(fixturesDir, 'prs'),`
- `scripts/write-visualiser-config.sh:60,158,184` — bash variable + `--arg prs` + jq construction
- `scripts/test-launch-server.sh:75` — `assert_json_eq "config: prs" ".doc_paths.prs" "$PROJ/meta/prs" "$CFG_FILE"`
- `skills/visualisation/visualise/SKILL.md:19` — `**PRs directory**: !\`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh prs\``

All of these reference the `doc_paths` config key, which stays as `prs` per `config_path_key()`. **However**, `SKILL.md:19`'s human label says "PRs directory" — that label should update to "PR descriptions directory".

#### B.7 Recap: what stays vs what changes

**Stays as `prs`** (config-key + on-disk surface):
- Rust `config_path_key()` return value
- `meta/prs/` directory on disk
- All JSON fixtures and test config keys in `doc_paths` maps
- `e2e/start-server.mjs`, `write-visualiser-config.sh`, `test-launch-server.sh`, `config-read-path.sh prs` argument
- `frontend/src/components/Glyph/icons/PrsIcon.tsx` file path inside `config_path_key()`-coupled places (no such coupling — this file does rename)

**Changes to `pr-descriptions` (wire) / `PR descriptions` (label)**:
- Rust enum variant: `DocTypeKey::Prs` → `DocTypeKey::PrDescriptions` with `#[serde(rename = "pr-descriptions")]`
- Rust `label()` return: `"PRs"` → `"PR descriptions"`
- Frontend `DocTypeKey` union: `'prs'` → `'pr-descriptions'`
- Frontend `DOC_TYPE_LABELS['prs']` value: `'PR'` → `'PR descriptions'` (and the key changes to `'pr-descriptions'`)
- Frontend pipeline-step label at `api/types.ts:192-193`: `'PR'` → `'PR descriptions'`
- Frontend `PHASE_DOC_TYPES.ship.docTypes` entry: `'prs'` → `'pr-descriptions'`
- Frontend icon: `PrsIcon.tsx` → `PrDescriptionsIcon.tsx`, exported function renames likewise, mapping at `Glyph.tsx:55` updates
- All `--ac-doc-prs` CSS custom properties in `global.css` and `tokens.ts`: `--ac-doc-prs` → `--ac-doc-pr-descriptions`
- 12 visual-regression PNG baseline filenames (or regenerated)
- 5 frontend test files asserting `hasPr: false` — IF `has_pr` is also renamed (planning decision)
- 4 server source/test references to `has_pr` — IF renamed
- `SKILL.md:19` human label "PRs directory" — but the CLI arg `prs` stays

### C. `/api/library/structure` endpoint — server-side shape and indexer extension

#### C.1 `AppState` already exposes what's needed

`server/src/server.rs:40-51`:

```rust
pub struct AppState {
    pub cfg: Arc<Config>,             // line 41
    // …
    pub indexer: Arc<crate::indexer::Indexer>,  // line 44
    // …
}
```

Both fields are `pub`. A new handler `async fn library_structure(State(state): State<Arc<AppState>>) -> Json<…>` can directly access `state.cfg` and `state.indexer.<method>()`. No AppState changes required.

#### C.2 Indexer's existing query surface

Read-only methods on `Indexer` (`server/src/indexer.rs`):

| Method | Lines | Notes |
|---|---|---|
| `all_by_type(kind)` | `:316-324` | Filtered enumeration, `entries.read().await.values().filter(…).cloned().collect()` |
| `counts_by_type()` | `:326-332` | The only existing per-`DocTypeKey` reducer; single pass, single `entries.read().await` |
| `all()` | `:334-336` | Full enumeration, `entries.read().await.values().cloned().collect()` |
| `get(path)` | `:338-347` | Direct + canonical fallback |
| `adr_by_id(id)` | `:349-352` | Secondary-index lookup |
| `work_item_by_id(id)` | `:354-357` | Secondary-index lookup |
| `reviews_by_target(path)` | `:370-382` | Acquires `entries.read()` first, then secondary lock |
| `work_item_refs_by_id(id)` | `:386-396` | Same pattern |
| `declared_outbound(entry)` | `:401-431` | Same pattern |

**No method returns a multi-aggregate per `DocTypeKey` in one pass.** `counts_by_type` is the only template. `clusters.rs::compute_clusters` (`:34-67`) is the only single-pass-with-snapshot precedent, but it operates on a `&[IndexEntry]` slice (caller pre-snapshotted via `indexer.all().await`), not directly on the lock.

#### C.3 Recommended indexer extension: `library_aggregates`

To avoid the multi-call race (where `counts_by_type()` followed by `latest_by_type()` followed by per-facet aggregators could each acquire a different `entries.read()` snapshot if a `rescan` lands in between), add **one** method that produces everything in one pass under one lock:

```rust
pub async fn library_aggregates(&self) -> LibraryAggregates { … }
```

Where `LibraryAggregates` carries:
- `counts: HashMap<DocTypeKey, usize>`
- `latest: HashMap<DocTypeKey, IndexEntry>` (or just `(title, slug, mtime_ms)` tuples to avoid cloning the full entry — but cloning is cheap relative to the read)
- Per-facet option counts keyed by `(DocTypeKey, FacetId)` (or pre-bucketed by facet kind: `status_by_type`, `cluster_slug_by_type`, `project_by_type`)

This is **one** `entries.read().await` lock, **one** linear pass over `entries.values()`, accumulating into multiple `HashMap`s. Same complexity class as `counts_by_type` plus a constant factor.

Caveat: `IndexEntry` is `Clone` and includes the full `serde_json::Value` frontmatter plus `body_preview: String`. If `latest` ends up holding 13 entries (one per doc type), the clone cost is negligible. If a per-facet bucket holds references to entries, lifetime management is awkward; cloning small projections is simpler.

#### C.4 Performance and caching

- Index size at v1 scale: low thousands of entries across 12 non-virtual doc types.
- One linear pass over `entries.values()` is the same shape as `counts_by_type` and `compute_clusters`. Both run on every request without a complaint.
- **No caching needed at v1.** Recompute on each request (mirrors `api/types::types` pattern).
- If caching is later desired, the precedent is `state.clusters: Arc<RwLock<Vec<LifecycleCluster>>>` (`server.rs:46`), refreshed inside `AppState::build` (`server.rs:82`). That hookup costs another rescan side-effect — defer.

#### C.5 Mtime semantics

`IndexEntry.mtime_ms: i64` (`indexer.rs:30`) is populated from `FileContent.mtime_ms` (`indexer.rs:824`), which itself comes from `file_driver.rs:330-335`:

```rust
let mtime_ms = meta
    .modified()
    .ok()
    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
    .map(|d| d.as_millis() as i64)
    .unwrap_or(0);
```

**`mtime_ms == 0` is a valid sentinel** (filesystem without mtime support, or pre-1970 mtime). `clusters.rs:52` uses `.max().unwrap_or(0)` and doesn't filter — follow that precedent in `library_aggregates`. The "latest" preview line will still render with the smallest-mtime entry rather than disappear.

#### C.6 Facet derivation

- **`status`**: `entry.frontmatter.get("status").and_then(|v| v.as_str())` — pattern from `indexer.rs:844`. Respect `entry.frontmatter_state == "parsed"` (skip when state is `"absent"`/`"malformed"`).
- **`cluster_slug`**: `entry.slug.clone()` — already the slug from `slug::derive`. `clusters.rs:34` already buckets by it.
- **`project`**: Read `cfg.work_item.as_ref().and_then(|w| w.default_project_code.clone())` (mirrors `api/work_item_config.rs:18-23`). For per-entry, split `entry.work_item_id` on first `-` and take the prefix; fall back to the configured default when no `-` present. `WorkItemConfig::extract_id` (`config.rs:111-127`) returns the full ID, not the prefix — there is no existing helper, so this split is **net-new** but trivial.

#### C.7 Wire-up in `api/mod.rs`

Insertion point in `api/mod.rs::mount` (`:23-44`), right after `/api/types`:

```rust
.route("/api/types", get(types::types))
.route("/api/library/structure", get(library::library_structure))  // new
```

Plus a new `mod library;` declaration at the top of `api/mod.rs` (alphabetical insertion between `kanban_config` and `lifecycle`).

#### C.8 Recommended structure response shape

The work item's spec at `Requirements > Server-driven library structure endpoint` is well-formed. One refinement: rather than extending `DocType` itself (which would invite the warning at `docs.rs:106-111`), add a sibling type. Suggested:

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStructureResponse {
    pub phases: Vec<Phase>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Phase {
    pub id: String,
    pub label: String,
    pub doc_types: Vec<LibraryDocType>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDocType {
    pub id: DocTypeKey,
    pub label: String,
    pub glyph_id: DocTypeKey,        // same as id for non-virtual
    pub route: String,                // e.g. "/library/decisions"
    pub count: usize,
    pub latest: Option<Latest>,
    pub filter_facets: Vec<Facet>,
    pub empty_description: Option<String>,
}

#[derive(Serialize)]
pub struct Latest { pub title: String, pub slug: Option<String>, pub modified_at: i64 }

#[derive(Serialize)]
pub struct Facet { pub id: String, pub label: String, pub kind: FacetKind, pub options: Vec<FacetOption> }
```

Where the phase order is hard-coded in Rust (matches the prototype's DEFINE/DISCOVER/BUILD/SHIP/REMEMBER ordering and the per-phase doc-type membership). The `kind` enum is `Enum` / `EnumWithSearch` (the "more than 8 options" UI variant) — let the server decide which to emit per facet so the frontend has no policy.

### D. Test surface — additions beyond prior research

#### D.1 Redirect coverage — only `router.test.tsx` 40-77 require changes for the redirect removal

Confirmed via search:
- No additional unit/integration test files reference `libraryIndexRoute`, `redirect({ to: '/library`, or the `/library` → `/library/decisions` cascade.
- `frontend/src/router.test.tsx:150,152,161` and `frontend/src/components/Breadcrumbs/Breadcrumbs.test.tsx:74` visit `/library/decisions` directly — they survive the redirect removal because `/library/decisions` remains a valid type route.

#### D.2 E2E

- `frontend/e2e/smoke.spec.ts:3` — `test('app loads and redirects to /library', async ({ page }) => {`
- `frontend/e2e/smoke.spec.ts:4` — `await page.goto('/')`
- `frontend/e2e/smoke.spec.ts:5` — `await expect(page).toHaveURL(/\/library/)`

The regex `/\/library/` still matches the new `/library` overview hub URL, so the assertion passes as-is. The **test title** ("redirects to /library") still reads correctly because `/` → `/library` still redirects (via `indexRoute` at `router.ts:60`). However, the **comment intent** may need adjustment if the test was meant to assert post-`/library` content. Low-priority touch-up.

#### D.3 Visual regression

- `frontend/tests/visual-regression/tokens.spec.ts:11` — `['library', '/library'],`
- `frontend/tests/visual-regression/tokens.spec.ts:81` — `await page.goto('/library')` (dark-mode capture)

Both navigate to bare `/library`. After 0041, this URL renders the new overview hub rather than redirecting to `/library/decisions`. **Baseline screenshots will change** — needs `--update-snapshots` regeneration.

- `frontend/tests/visual-regression/tokens.spec.ts:13` — `['library-decisions', '/library/decisions'],`

This baseline still covers `/library/decisions` directly. Survives the redirect removal, but shifts if the shared `Page` wrapper changes the visual rendering of the type list view — which it will.

#### D.4 Sort-header coverage — only `LibraryTypeView.test.tsx:57-80` requires changes

Confirmed: no other tests reference `SortHeader`, `aria-sort`, `toggleSort`, or click `<th>` buttons. The 18 tests in `LibraryTypeView.test.tsx` will need a rewrite to match the new column set, sort-pill behaviour, and the redesigned empty state — but no surprise extra files.

#### D.5 `LibraryTypeView.module.css` allowlist counts may shift

`migration.test.ts:131-134` enumerates the irreducible literals in `LibraryTypeView.module.css`:
- `:131` — `2px` count `3` (border + outline widths)
- `:132` — `1px` count `2`
- `:133` — `0.4rem` count `1`
- `:134` — `900px` count `1`

When `LibraryTypeView` loses its `.sortButton` rules and column-header borders, the `2px` / `1px` / `0.4rem` counts may drop. The allowlist enforcement is exact-match; counts must update or the test fails.

#### D.6 URL state — confirmed absent

`validateSearch` does not appear anywhere in `frontend/src/`. `libraryTypeRoute` (`router.ts:98-107`) declares only `getParentRoute`, `path: '/$type'`, and `component: LibraryTypeView`. No test passes search params on `/library/$type`. Confirmed safe to remove column-header click-sort without breaking any URL contract.

---

## Code References

### Frontend — files newly identified

- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.module.css:13-17` — `.main { padding: var(--sp-5) var(--sp-6) }` (the rule being removed)
- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.module.css:1-6` — `.board { padding: var(--sp-4) }` (must drop after migration)
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.tsx:72-87` — toolbar; candidate for `Page` actions slot
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.module.css:1` — `900px` max-width
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:70-78` — custom `<header>` inside grid
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.module.css:3-9` — grid layout, `1100px` max-width, `.title` styling
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx:40-43` — bare `<h1>`
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css:1-3` — `900px` max-width, `.title { margin-bottom }`
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx:31-33` — bare `<h1>`
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.module.css:1-3` — `600px` max-width (outlier)
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:122,126,129,134,154` — max-width allowlist entries to clean up
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:131-134` — `LibraryTypeView.module.css` literal counts that may shift
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:327-348` — allowlist exact-match enforcement
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:394-412` — `.title { color: var(--ac-fg-strong) }` REQUIRED list (three of five routes)

### Frontend — `prs` rename additive surface

- `frontend/src/components/Glyph/Glyph.tsx:11` — `import { PrsIcon } from './icons/PrsIcon'`
- `frontend/src/components/Glyph/icons/PrsIcon.tsx:3` — `export function PrsIcon()`
- `frontend/src/router.test.tsx:93,190` — `hasPr: false` fixture
- `frontend/src/api/fetch.test.ts:140,329,346` — `hasPr: false`
- `frontend/src/routes/lifecycle/LifecycleIndex.test.tsx:12` — `hasPr: false`
- `frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx:14` — `hasPr: false`
- `frontend/src/components/PipelineDots/PipelineDots.test.tsx:8` — `hasPr: false`
- `frontend/tests/visual-regression/__screenshots__/glyph-showcase.spec.ts-snapshots/prs-*-*.png` (12 files)
- `frontend/e2e/smoke.spec.ts:3-5` — redirect smoke test (title may need touch-up)
- `frontend/tests/visual-regression/tokens.spec.ts:11,13,81` — `/library` and `/library/decisions` baselines

### Server — files newly identified

- `server/src/server.rs:40-51` — `AppState` definition, `pub cfg` and `pub indexer` fields
- `server/src/server.rs:82` — `AppState::build` cluster-cache wiring (caching precedent)
- `server/src/server.rs:165-170` — top-level route registration
- `server/src/indexer.rs:824` — `mtime_ms` population
- `server/src/file_driver.rs:330-335,395-401,429-434` — `mtime_ms` derivation (zero-sentinel possibility)
- `server/src/clusters.rs:16,108,208` — `has_pr` field name (planning decision)
- `server/src/clusters.rs:34-67` — `compute_clusters` (single-pass-with-snapshot precedent)
- `server/src/api/types.rs:1-21` — `/api/types` handler (full body — template for `library_structure`)
- `server/src/api/lifecycle.rs:21` — clusters cache read pattern
- `server/src/api/work_item_config.rs:18-23` — `cfg.work_item.default_project_code` access pattern (for `project` facet)
- `server/src/config.rs:65-71,111-127` — `WorkItemConfig` (no project-prefix helper exists)
- `server/src/indexer.rs:512,844` — frontmatter `.get(…).as_str()` access pattern
- `server/tests/fixtures/config.valid.json:20` — `doc_paths.prs` (stays as `prs`)
- `server/tests/fixtures/config.optional-override-null.json:20` — same
- `server/tests/common/mod.rs:58` — doc_paths seeding (stays as `prs`)
- `server/tests/config_contract.rs:62` — expected keys list (stays)
- `server/tests/api_smoke.rs:27` — config tuple (stays)
- `skills/visualisation/visualise/SKILL.md:19` — `**PRs directory**: ...config-read-path.sh prs` — human label updates, CLI arg stays

---

## Architecture Insights

- **`config_path_key()` is the asymmetry hinge**. The work item's deliberate choice to leave `config_path_key()` returning `Some("prs")` means there are three distinct identifier spaces for this doc type after the rename: (a) **wire token** `pr-descriptions`, (b) **config key** `prs`, (c) **directory name** `prs`. This asymmetry is acceptable per the work item but it does mean every developer touching this surface must mentally distinguish these three spaces.
- **`has_pr` is the inconsistency tell**. It's a single boolean field whose name reflects the old kebab token. Renaming it (or not) is a small judgment call about whether internal field names should track human labels. Recommendation: rename `has_pr` → `has_pr_description` for the same reason the label is changing — the field's semantic referent is the doc type, not the legacy short name.
- **`AppState` is already a thin shell over `Arc<Config>` and `Arc<Indexer>`**. Every API handler in the codebase accesses these via `state.cfg` / `state.indexer.<method>()`. The new `library_structure` handler should follow this exact shape with no AppState changes.
- **Per-handler single-pass-over-`entries` is the established pattern**. Both `counts_by_type` (`indexer.rs:326-332`) and `compute_clusters` (`clusters.rs:34-67`) do exactly this. The library-structure aggregator is the third instance, not a new pattern.
- **Multi-aggregate-per-pass is net-new**. No existing reducer produces more than one output type per pass. The new `library_aggregates` method introduces this shape — useful template for future per-DocTypeKey aggregations.
- **`Page` migration sequences five heterogeneous routes simultaneously**. The work item correctly bundles these to keep `RootLayout.main`'s padding-rule change atomic. The non-obvious case is `LibraryDocView` — the grid layout means slot mapping isn't a pure 1:1 replacement, and the implementation should test aside-column alignment after migration.

---

## Historical Context

- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md` — primary research; established eight resolved decisions, the surface of `LibraryTypeView`, `PageSubtitle`, the router redirect, `PHASE_DOC_TYPES`, design tokens, `describe_types` shape, and the absence of popover/checkbox primitives.
- `meta/work/0041-library-page-wrapper-and-overview-hub.md` — work item, substantially rewritten after the 2026-05-15 research to incorporate all eight resolutions; expanded scope to five-route migration and PR-descriptions rename.
- `meta/decisions/ADR-0024-visualiser-kanban-column-config.md` — server-driven kanban column config; precedent shape for server-driven library structure.
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` — token application conventions; binds `--ac-content-max-width` introduction to existing token discipline.
- Blocker work items 0033 (Design Token System), 0037 (Glyph), 0038 (Chip) — all `done`.

---

## Related Research

- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md` — extended by this doc.
- `meta/research/codebase/2026-05-13-0055-sidebar-activity-feed.md` — adjacent component pattern (icon glyph + count); similar `<Link>` + `DOC_TYPE_LABELS` consumer to migrate to server shape.
- `meta/research/codebase/2026-05-14-0038-generic-chip-component.md` — `Chip` component implementation; consumed unchanged by 0041.

---

## Open Questions

The 2026-05-15 research closed all eight open questions. This pass surfaces three smaller decisions for the planner:

1. **`ClusterFlags::has_pr` rename** — does this field rename to `has_pr_description` for consistency, or stay (since the semantics are unchanged)? Recommendation: rename for label consistency. Surface: `server/src/clusters.rs:16,108,208`; five frontend test files.
2. **Canonical value for `--ac-content-max-width`** — five routes have five different current widths (1100 / 900 / 900 / 900 / 600). The work item's Open Question on this remains open. Recommendation: 1100px canonical, with `LibraryTemplatesIndex` (600px) passing an override via `Page` prop, and the three 900px routes either aligning to 1100 or also overriding. Decide per route against the prototype screenshots.
3. **`SKILL.md:19` human label** — "PRs directory" should rename to "PR descriptions directory" while the CLI arg `prs` stays. Tiny but worth flagging.

---

## References

- Work item: `meta/work/0041-library-page-wrapper-and-overview-hub.md`
- Primary research: `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
- Blocker work items: 0033 (Design Token System), 0037 (Glyph Component), 0038 (Generic Chip Component) — all `done`.
- Sibling work items: 0042 (Templates View Redesign), 0044 (Spike: Confirm List-Screen Scope Decisions) — both `draft`.
