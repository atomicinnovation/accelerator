---
type: codebase-research
id: "2026-06-13-0110-surface-rcas-in-visualiser-operate-category"
title: "Research: Surfacing Root Cause Analyses in the Visualiser under a new Operate category"
date: "2026-06-13T20:42:18+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0110"
parent: "work-item:0110"
relates_to: ["codebase-research:2026-05-24-0074-per-doc-type-hues-on-detail-page", "codebase-research:2026-06-09-0082-big-glyph-hero-illustrations", "codebase-research:2026-05-15-0041-library-page-wrapper-and-overview-hub", "codebase-research:2026-06-01-0054-sidebar-search", "codebase-research:2026-06-11-0096-templates-view-auto-discovery"]
topic: "Surfacing Root Cause Analyses in the Visualiser under a new Operate category"
tags: [research, codebase, visualiser, rca, issue-research, doc-types, library, operate]
revision: "c1f316728a5bef4414f6ea25351f5f3f9ef23a2f"
repository: "build-system"
last_updated: "2026-06-13T20:42:18+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Surfacing Root Cause Analyses in the Visualiser under a new Operate category

**Date**: 2026-06-13T20:42:18+00:00
**Author**: Toby Clemson
**Git Commit**: c1f316728a5bef4414f6ea25351f5f3f9ef23a2f
**Branch**: HEAD (jj workspace: `workspaces/build-system`)
**Repository**: build-system (Accelerator plugin)

## Research Question

For work item 0110 — "Surface Root Cause Analyses in the Visualiser Under a New
Operate Category" — map every touchpoint required to surface the RCA
(`issue-research`) doc type end-to-end: a new top-level "Operate" category, a
first-class doc-type registration, a listing page, a detail page, library-hub
card, related-artifacts integration, and search. Establish what work item 0096
already landed (so this work verifies rather than re-adds it), what the
authoritative design prototype dictates, and what is still missing.

## Summary

The visualiser is **server-driven but not data-driven for the category
structure**: the five phases (DEFINE / DISCOVER / BUILD / SHIP / REMEMBER) are a
hard-coded `PHASES` static slice in the Rust server, keyed by the `DocTypeKey`
enum. Almost every browsable surface (sidebar, overview hub, listing page,
detail page) is **generic over that registry** — adding a doc type to the
registry makes it appear in all of them automatically. The real work is
extending two registries (server `DocTypeKey`, frontend `DocTypeKey` union) plus
a handful of **compile-time-exhaustive per-type asset tables** (glyph, hue,
labels, empty-state copy), which the Rust/TypeScript exhaustiveness checks will
refuse to compile until every entry is filled in — a strong safety net.

**The single most important finding** is that this one concept carries **four
distinct spellings** across the stack, and conflating them is the main hazard:

| Spelling | Where it lives | Role |
|---|---|---|
| `issue-research` | frontmatter `type:` discriminator (ADR-0033/0057) | the artifact's declared type, written into the file |
| `rca` | template stem (`templates/rca.md`), `STEM_TO_GLYPH`, config template key | template/registration stem |
| `root-cause-analyses` | frontend glyph-only key, **prototype type key + route**, colour tokens | the browsable doc-type key + URL token |
| `research_issues` | server config `doc_paths` key → `meta/research/issues/` | the on-disk directory the indexer scans |

The server classifies a file's doc type by **which configured directory it lives
in** (`meta/research/issues/`), not by its frontmatter `type:` value. So the
**wire token / route should be `root-cause-analyses`** (matching the prototype
and the existing frontend glyph-only key), the **directory key is
`research_issues`**, and the **frontmatter discriminator stays `issue-research`**
— all four coexist cleanly once you keep them in their lanes.

**State of 0096's groundwork (verified):** 0096 landed the *small-glyph
presentation layer* for RCA under the glyph-only key `root-cause-analyses` — the
fishbone icon, `STEM_TO_GLYPH` entry, `GLYPH_ONLY_DOC_TYPES` membership, and
light/dark colour tokens. It deliberately did **not** make RCA a browsable doc
type. Additionally, `research_issues` already exists as a config `doc_paths` key
mapping to `meta/research/issues/`, and a real RCA artifact already lives there
(`meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`).
So the directory plumbing and the glyph are present; **everything that makes RCA
browsable is absent** (server enum variant, `PHASES` membership, the Operate
phase itself, frontend `DocTypeKey` union entry, labels, numeric hue, BigGlyph
hero, empty-state copy, and tests/fixtures).

**One discrepancy to resolve before implementation** (see Open Questions): the
work item's acceptance criterion says the *detail page* shows the RCA BigGlyph
hero, but in both the current visualiser and the authoritative prototype the
BigGlyph hero appears only on the **empty-state** (and recovery) surface — the
populated detail page uses the small framed glyph in its eyebrow, not the
BigGlyph.

## Detailed Findings

### 1. Server: the phase/category structure is a static table keyed by an enum

The phase membership is a module-level static in
`skills/visualisation/visualise/server/src/api/library.rs:74-111`:

```rust
const PHASES: &[(&str, &str, &[DocTypeKey])] = &[
    ("define",   "Define",   &[DocTypeKey::WorkItems, DocTypeKey::WorkItemReviews]),
    ("discover", "Discover", &[DocTypeKey::DesignInventories, DocTypeKey::DesignGaps, DocTypeKey::Research]),
    ("build",    "Build",    &[DocTypeKey::Plans, DocTypeKey::PlanReviews, DocTypeKey::Validations]),
    ("ship",     "Ship",     &[DocTypeKey::PrDescriptions, DocTypeKey::PrReviews]),
    ("remember", "Remember", &[DocTypeKey::Decisions, DocTypeKey::Notes]),
];
```

- Phase order = array order; within-phase order = slice order. Both are
  preserved verbatim into the response by `build_structure`
  (`library.rs:161-181`).
- The contract is pinned by `tests/api_library_structure.rs:22-38`, which
  asserts the exact phase id list — that test must be updated to include
  `"operate"` between `"ship"` and `"remember"`.
- A doc-comment at `library.rs:74-76` calls this "the seam to extend" — it
  centralises what was previously the client-side `PHASE_DOC_TYPES` table.

**To add Operate:** insert a new tuple
`("operate", "Operate", &[DocTypeKey::RootCauseAnalyses])` between the `ship` and
`remember` entries.

The wire response model (`library.rs:13-72`): `LibraryStructureResponse { phases,
templates }`; `Phase { id, label, doc_types }` (serialises `docTypes`);
`LibraryDocType { id: DocTypeKey, label, count, filtered_count, latest,
filter_facets }`; `LatestPreviewWire { title, slug, modified_at }`. **The server
emits no glyph, route, or singular/plural/short label** — there is exactly one
`label` per type. Those are all frontend concerns.

### 2. Server: the `DocTypeKey` registry and what a new variant touches

The discriminator enum is `skills/visualisation/visualise/server/src/docs.rs:4-20`
(13 variants today, `#[serde(rename_all = "kebab-case")]` so it serialises
straight to the wire `id`). Adding `RootCauseAnalyses` touches:

1. `docs.rs:6-20` — add the enum variant.
2. `docs.rs:23-39` — add to `all()` **and bump the array type `[DocTypeKey; 13]`
   → `14`** (asserted by tests at `docs.rs:243-251`, `296-298`).
3. `docs.rs:41-62` — `config_path_key()` returns `Some("research_issues")` (the
   key already exists in config; this is the seam — see §3).
4. `docs.rs:64-80` — `label()` → "Root cause analyses".
5. `docs.rs:143-159` — `wire_str()` → `"root-cause-analyses"` (round-trip test
   at `docs.rs:255-263` + the count-13 tests above).
6. `docs.rs:82-139` — lifecycle/kanban predicates (`in_lifecycle`,
   `participates_in_lifecycle`, `in_kanban`, `is_virtual`,
   `carries_target_frontmatter`, `nested_manifest_filename`). RCA should be
   `is_virtual: false`, `in_kanban: false`, and (like Notes/Decisions) **not**
   participate in the linear lifecycle pipeline.
7. `src/api/library.rs:77-111` — the `PHASES` Operate tuple (§1).
8. `src/indexer.rs:63-69` — `facets_for` already defaults non-kanban types to
   `&["status", "clusterSlug"]`, so RCA gets a **status facet for free**.
9. `src/file_driver.rs:111-121` (`LocalFileDriver::new`) and `:527`
   (`kind_for_canonical_path`) pick up the new type **automatically** as long as
   `config_path_key()` returns a key present in `cfg.doc_paths`; classification
   is pure `path.starts_with(root)`, no per-type code.

### 3. Server: counts, "latest", the listing endpoint, and the `research_issues` seam

- **Counts & latest preview** come from `Indexer::library_aggregates`
  (`indexer.rs:640-704`), called by the structure handler
  (`library.rs:118-121`). First pass increments `per.count` per `entry.r#type`
  and tracks the greatest `mtime_ms` for "latest" (deterministic tie-break on
  lexically-smaller `rel_path`). `filtered_count` is a second pass applying the
  facet `Selection`. This is the 0041 semantics the AC references (count = number
  of artifacts; latest = most recently modified) and it applies to RCA with **no
  per-type code** once the variant exists.
- **Listing endpoint:** `GET /api/docs?type=<wire-token>` →
  `docs::docs_list` (`src/api/docs.rs:30-42`), generic: `all_by_type(kind)` →
  `Json(DocsListResponse { docs: Vec<IndexEntry> })`. Each `IndexEntry`
  (`indexer.rs:160-196`) carries `title`, `slug`, `frontmatter` (where `status`
  lives), `mtimeMs`, `workItemId`, etc. No per-type change needed.
- **The `research_issues` seam:** the config path key **already exists**. Test
  helpers and contract tests reference it — `server/tests/common/mod.rs:73`,
  `server/tests/config_contract.rs:57` (which asserts `doc_paths.len() == 13` and
  enumerates required keys including `research_issues`), and the
  `config.*.json` fixtures. But `DocTypeKey::config_path_key()` never returns it
  today, so nothing is indexed from it. The launcher
  `skills/visualisation/visualise/scripts/write-visualiser-config.sh` (path vars
  `:71-82`, jq `--arg` `:287-294`, `doc_paths` object `:310-320`) is where the
  production `doc_paths` is assembled — confirm `research_issues` →
  `meta/research/issues` is wired there (a real artifact already lives in that
  directory).

### 4. Server: search auto-includes a new doc type

Search is `GET /api/search` → `search::search`
(`server/src/api/search.rs:95-138`), reading the **same in-memory `Indexer`
snapshot** as the library (`all().await`, no per-request FS scan). It enumerates
doc types from `DocTypeKey::all()` and special-cases only `Templates`
(excluded). **Therefore a new `DocTypeKey` is automatically searchable
server-side**, provided it (a) is not virtual, (b) has a `config_path_key()`
mapped to a configured directory, and (c) its entries have a non-`None` slug
(`project()` invariant at `search.rs:86`). The result row
(`SearchResultRow`, `search.rs:27-34`) carries only `docType`, `title`, `slug`,
`mtimeMs` — label/route/glyph/hue are all derived client-side from `docType`.

### 5. Frontend: RCA exists as a glyph-only key and must be *promoted*

The frontend models browsable types via the `DocTypeKey` union and a
presentational superset `GlyphDocType = DocTypeKey | "root-cause-analyses"`.
**RCA already exists, but only as a glyph-only key `root-cause-analyses` — not a
`DocTypeKey`, and `issue-research` appears nowhere in the frontend.**

Already present (0096's work):
- `src/components/Glyph/Glyph.constants.ts:13` —
  `GLYPH_ONLY_DOC_TYPES = ["root-cause-analyses"]` with the comment "RCAs are not
  a visualiser doc type today."
- `DOC_TYPE_TOKEN_KEY` (`Glyph.constants.ts:23-38`) and `DOC_TYPE_COLOR_VAR`
  (`:42-57`) — both include `root-cause-analyses`.
- `src/components/Glyph/Glyph.tsx:39` — `ICON_COMPONENTS["root-cause-analyses"]:
  RootCauseAnalysesIcon`; the fishbone icon at
  `src/components/Glyph/icons/RootCauseAnalysesIcon.tsx`.
- `src/components/Glyph/Glyph.module.css:27` — framed-tile background for
  `data-doc-type="root-cause-analyses"`.
- `src/routes/library/template-tier.ts:64-65` — `STEM_TO_GLYPH` `rca` /
  `root-cause-analyses` → `root-cause-analyses` (resolved by
  `glyphKeyForTemplate`, `:75-88`).
- `src/styles/tokens.ts:73-76,92,149,165` — resolved hex colour tokens
  (`ac-doc-root-cause-analyses` `#ab2c96`, light/dark fg+bg), with a comment
  noting "Hue ~310 (the prototype's RCA hue)". Mirrored in
  `src/styles/global.css:119-121,137,382,397,455,468`.

Missing to make it browsable:
- `src/api/types.ts:4-17` (`DocTypeKey` union) + `:23-37` (`DOC_TYPE_KEYS`
  runtime mirror) — **no RCA entry**. This is the gate for routing
  (`isDocTypeKey`, `:40-42`), the sidebar, and `RelatedArtifacts`.
- `src/api/types.ts:68-82` (`DOC_TYPE_LABELS` plural) + `:87-101`
  (`DOC_TYPE_LABELS_SINGULAR`) — no RCA labels.
- `src/styles/tokens.ts:11-25` (`DOC_TYPE_HUE: Record<DocTypeKey, number>`) — no
  numeric hue. **There is no hue `310` anywhere**; only the resolved hex token
  whose comment mentions ~310. Add `"root-cause-analyses": 310`.
- BigGlyph hero — absent (§7).
- `src/routes/library/empty-descriptions.ts` — `TYPE_COPY` and
  `EMPTY_TYPE_PLURALS` are `Record<DocTypeKey, …>`; both need RCA entries.
- `src/api/status-variant.ts` — `statusToVariant` must learn `resolved`
  (→ green) and `monitoring` (→ indigo) or the status column renders neutral grey
  (§6, §8).
- Graduate `root-cause-analyses` **out of** `GLYPH_ONLY_DOC_TYPES`
  (`Glyph.constants.ts:13`) once it is a real `DocTypeKey`, so the
  `Record<GlyphDocType, …>` and `Record<DocTypeKey, …>` tables stay consistent.

### 6. Frontend: the listing, detail, hub, related-artifacts, and search UI are all generic

Once RCA is a `DocTypeKey`, these surfaces light up with **no per-type edit**:

- **Routes** (`src/router.ts:108-124`): `/library/$type` (listing →
  `LibraryTypeView`) and `/library/$type/$fileSlug` (detail → `LibraryDocView`)
  are generic; `parseParams` admits any `isDocTypeKey` value. No new route.
- **Listing** `src/routes/library/LibraryTypeView.tsx`: ARIA-grid with columns
  ID/Date, Title, **Status**, Slug, Modified (headers `:248-260`). The status
  cell (`:284-292`) reads `entry.frontmatter.status` via `statusValue` (`:27-31`)
  and renders `<Chip variant={statusToVariant(...)}>`. So `resolved`/`monitoring`
  render automatically — but fall to neutral grey unless `status-variant.ts`
  learns them (§5).
- **Detail** `src/routes/library/LibraryDocView.tsx`: applies hue+glyph via
  `<EyebrowLabel type={type}/>` (`:219`, renders the small framed `Glyph`),
  then `FrontmatterChips` (`:122-127`), `FrontmatterTable` (`:193-199`), and the
  markdown body. **It uses the small glyph, not the BigGlyph hero** (§8).
- **Overview hub card** `src/routes/library/LibraryOverviewHub.tsx:54-81`
  (`HubCard`): framed glyph + label + `count` + `latest` preview, driven entirely
  by the server `phases`. Appears automatically once RCA is in `PHASES`.
- **Related artifacts** `src/components/RelatedArtifacts/RelatedArtifacts.tsx`:
  rows link to `/library/${entry.type}/${slug}` (`:111-115`), glyph via
  `<Glyph docType={entry.type}/>` (`:116`), colour via
  `DOC_TYPE_COLOR_VAR[entry.type]` (`:123-128`), label via
  `DOC_TYPE_LABELS_SINGULAR[entry.type]` (`:127`). Because `entry.type` is typed
  `DocTypeKey`, RCA can only appear here **after** it joins the union and gains a
  singular label — then it works with no component change.
- **Search UI** `src/components/Sidebar/SearchResultsPanel.tsx:64-95`: builds
  `<Link to="/library/$type/$fileSlug">` from `docType`+`slug`, glyph via
  `<Glyph docType={r.docType}/>`, label via `DOC_TYPE_LABELS_SINGULAR[r.docType]`.
  Same gate: needs the union + singular-label entry.

### 7. Frontend: the BigGlyph hero must be authored

`src/components/BigGlyph/BigGlyph.tsx:27-41` defines
`BIG_GLYPHS: Record<DocTypeKey, BigGlyphDraw>` (13 entries; **no RCA**). A
BigGlyph is **not** a component — it is a `BigGlyphDraw` arrow function
`(p: BigPalette) => ReactElement` returning the inner `<g>` of an 80×80 SVG,
colouring shapes from a hue-derived palette (`bigPalette.ts:3-37`). The hue comes
from `DOC_TYPE_HUE[docType]` (`BigGlyph.tsx:81-82`), so adding the numeric hue
310 entry (§5) feeds the palette automatically.

The prototype already contains the RCA BigGlyph (a fishbone/Ishikawa diagram with
magnifier) at
`meta/research/.../prototype-full/src/big-glyphs.jsx:349-397` — it can be ported
near-verbatim into a new `src/components/BigGlyph/icons/RootCauseAnalysesBigGlyph.tsx`
and registered in `BIG_GLYPHS`. Today the hero only renders on
`src/routes/library/EmptyState.tsx:32` and
`src/routes/library/recovery/RecoverySurface.tsx:45` (both at size 96), plus the
`/big-glyph-showcase` dev page — **not** on the populated detail page.

### 8. The authoritative prototype: Operate, labels, hue, listing, detail, status

From `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`:

- **`LIBRARY_GROUPS`** (`src/data.jsx:25-32`): the `operate` group has key
  `"operate"`, label `"Operate"`, **sits between `ship` and `remember`**, and
  contains exactly `["root-cause-analyses"]`. (Note: the comment block above the
  constant at `:21-24` is stale and describes an older partitioning — the data is
  authoritative.)
- **Labels**: plural `"Root cause analyses"` (`DOC_TYPES`, `data.jsx:18`);
  singular `"Root cause analysis"` and short `"RCA"` (`TYPE_META`,
  `src/ui.jsx:303`). Stage label is the shorter `"Root cause"` (`STAGES`,
  `data.jsx:54`). The frontmatter `type:` value used in curated content is
  `root-cause-analysis` (singular kebab) — note this differs from the system's
  actual `issue-research` discriminator; the prototype is a design mock, so the
  visualiser's real frontmatter discriminator (`issue-research`) governs.
- **Hue 310**: confirmed in both `STAGES` (`data.jsx:54`, `hue: 310`) and
  `TYPE_META` (`ui.jsx:303`, `hue: 310`).
- **Route**: derived from the type key — `#/library/root-cause-analyses` and
  `#/library/root-cause-analyses/<id>` (`src/search.jsx:224`).
- **Listing** is the generic `LibraryIndex` table (`src/view-library.jsx:287-306`):
  columns ID/Date, Title, **Status**, Slug, Modified; each row renders
  `<StatusBadge status={r.status}/>`; sample data
  (`LIBRARY_INDEX["root-cause-analyses"]`, `data.jsx:1264-1268`) uses statuses
  `resolved` and `monitoring`.
- **Detail** is the generic `DocPage` (`src/view-library.jsx:323-467`): eyebrow
  `<TypeGlyph type size={16}/>` tinted at hue 310, frontmatter chips (status as a
  `StatusBadge`, plus date/author), full frontmatter table, markdown body, and a
  right aside with Related artifacts / Declared links / File / Cluster. **No
  BigGlyph on the populated detail page** — it appears only on the empty state
  (`src/view-empty.jsx:93`).
- **Status colour map** (`StatusBadge`, `src/ui.jsx:77-99`): `resolved` → green
  "Resolved"; `monitoring` → indigo "Monitoring"; `sev-1/2/3` → red/amber/neutral
  (severity is rendered as a plain frontmatter row, not a top chip);
  unmapped values (e.g. `investigating`) fall through to a neutral chip with the
  raw label.

### 9. Test infrastructure and fixtures

- **Server (Rust):** `server/tests/api_library_structure.rs` (the phase/category
  contract — `request()` helper drives the axum router via `oneshot`;
  `:22-38` asserts the phase id list; `:40-63` asserts per-type
  count/filteredCount) and `server/tests/api_types.rs:13` (asserts `/api/types`
  length — hard-codes 13 → 14). Fixtures come two ways: (1) the `seeded_cfg(tmp)`
  helper (`server/tests/common/mod.rs:16-106`) which `tempdir`s a repo,
  `create_dir_all`s doc dirs, and `fs::write`s inline-frontmatter `.md` files —
  add `meta/research/issues/` + a `doc_paths.insert("research_issues", …)`;
  (2) committed fixtures at `server/tests/fixtures/meta/<type>/*.md` (shared with
  E2E) — add `server/tests/fixtures/meta/research/issues/<date>-example-rca.md`
  mirroring `.../research/design-gaps/2026-05-26-example-gap.md`. Update
  `config_contract.rs:46-70` if the `doc_paths` count changes.
- **Frontend (Vitest):** listing template
  `frontend/src/routes/library/LibraryTypeView.test.tsx` (mock a
  `LibraryStructureResponse` literal + `IndexEntry[]`, `vi.spyOn` on
  `fetchModule.fetchDocs` / `fetchLibraryStructure`, render inside
  `QueryClientProvider` + `MemoryRouter`); hub-card template
  `LibraryOverviewHub.test.tsx` (`:99-107` asserts a card link's `href`). Add an
  RCA entry to the mock `baseStructure` and assert the card links to
  `/library/root-cause-analyses`. The glyph itself is already tested
  (`Glyph.test.tsx:205-226`).
- **E2E (Playwright):** navigation template `frontend/e2e/navigation.spec.ts:3-36`
  (`goto("/library/<type>")` → assert `[role="table"]` → click row → assert
  detail URL). The real Rust server is launched by `frontend/e2e/start-server.mjs`
  against the committed fixtures; its `docPaths` object (`:62-75`) **must gain
  `research_issues: join(fixturesDir, "research/issues")`**.
- **Visual regression:** snapshots under
  `frontend/tests/visual-regression/__screenshots__/*.spec.ts-snapshots/` are
  committed **per-platform** (`*-darwin.png` / `*-linux.png`). Adding RCA routes
  to a VR `ROUTES` array needs both baselines; linux baselines are regenerated
  via the "Update visual regression baselines" CI workflow (darwin generated
  locally).

## Code References

**Server (Rust):**
- `skills/visualisation/visualise/server/src/api/library.rs:74-111` — `PHASES` static (add the Operate tuple here)
- `skills/visualisation/visualise/server/src/api/library.rs:13-72,161-199` — wire model + structure/doc-type builders
- `skills/visualisation/visualise/server/src/docs.rs:4-20` — `DocTypeKey` enum (add `RootCauseAnalyses`)
- `skills/visualisation/visualise/server/src/docs.rs:23-39` — `all()` + `[DocTypeKey; 13]→14`
- `skills/visualisation/visualise/server/src/docs.rs:41-62,64-80,143-159` — `config_path_key` / `label` / `wire_str`
- `skills/visualisation/visualise/server/src/indexer.rs:63-69,640-704` — facets + count/latest aggregates
- `skills/visualisation/visualise/server/src/file_driver.rs:111-121,527` — directory→type classification (auto)
- `skills/visualisation/visualise/server/src/api/docs.rs:30-42` — generic listing endpoint
- `skills/visualisation/visualise/server/src/api/search.rs:95-138` — search (auto-includes new types)
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:71-82,287-320` — `doc_paths` config assembly

**Frontend (React/TS):**
- `skills/visualisation/visualise/frontend/src/api/types.ts:4-37,68-101` — `DocTypeKey` union, `DOC_TYPE_KEYS`, labels (add RCA)
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:11-25,73-76` — `DOC_TYPE_HUE` (add 310); existing colour tokens
- `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.constants.ts:13,23-57` — graduate out of `GLYPH_ONLY_DOC_TYPES`; token/colour maps
- `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx:26-41` — `ICON_COMPONENTS` (RCA icon already wired)
- `skills/visualisation/visualise/frontend/src/components/Glyph/icons/RootCauseAnalysesIcon.tsx` — existing fishbone icon
- `skills/visualisation/visualise/frontend/src/components/BigGlyph/BigGlyph.tsx:27-41` — `BIG_GLYPHS` (add RCA hero)
- `skills/visualisation/visualise/frontend/src/components/BigGlyph/bigPalette.ts:3-37` — `BigGlyphDraw`/palette shape
- `skills/visualisation/visualise/frontend/src/api/status-variant.ts:4-33` — add `resolved`/`monitoring`
- `skills/visualisation/visualise/frontend/src/routes/library/empty-descriptions.ts` — `TYPE_COPY` + `EMPTY_TYPE_PLURALS`
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx:248-292` — generic listing + status column
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:122-219` — generic detail page
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryOverviewHub.tsx:54-81` — generic hub card
- `skills/visualisation/visualise/frontend/src/components/RelatedArtifacts/RelatedArtifacts.tsx:111-128` — related-artifacts row
- `skills/visualisation/visualise/frontend/src/components/Sidebar/SearchResultsPanel.tsx:64-95` — search result row
- `skills/visualisation/visualise/frontend/src/routes/library/template-tier.ts:64-65` — `STEM_TO_GLYPH` rca entry (existing)

**Prototype (authoritative design):**
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/data.jsx:18,25-32,54,1264-1268` — `DOC_TYPES`, `LIBRARY_GROUPS`, `STAGES`, RCA listing data
- `.../prototype-full/src/ui.jsx:77-99,303,416-426,444-462` — `StatusBadge`, `TYPE_META`, RCA icon, `TypeGlyph`
- `.../prototype-full/src/big-glyphs.jsx:16-26,349-397` — `bigPalette`, RCA BigGlyph (port this)
- `.../prototype-full/src/view-library.jsx:287-467` — generic listing + `DocPage`

**Tests/fixtures:**
- `skills/visualisation/visualise/server/tests/api_library_structure.rs:22-63`, `server/tests/api_types.rs:13`, `server/tests/common/mod.rs:16-106`, `server/tests/config_contract.rs:46-70`
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.test.tsx`, `LibraryOverviewHub.test.tsx`, `frontend/e2e/navigation.spec.ts:3-36`, `frontend/e2e/start-server.mjs:62-75`

## Architecture Insights

- **Server-driven but not data-driven.** The category structure is a hard-coded
  Rust static, not config/data. "Server-driven" (0041) means the *frontend* reads
  the structure from the server; the structure itself is compiled in. So adding
  Operate genuinely requires a server code change, exactly as the work item's
  Assumptions state.
- **The registry is the lever; the surfaces are generic.** The strong pattern is
  that adding a `DocTypeKey` (server) + `DocTypeKey` union member (frontend) +
  filling the exhaustive `Record<DocTypeKey, …>` asset tables flows through to
  the sidebar, hub, listing, detail, related-artifacts, and search with no
  component edits. The compiler enforces completeness.
- **Classification is by directory, not by frontmatter.** The indexer assigns a
  `DocTypeKey` from the configured `doc_paths` directory a file lives in
  (`file_driver` `path.starts_with`). This is why the four spellings coexist
  cleanly: the route token (`root-cause-analyses`), directory key
  (`research_issues`), and frontmatter discriminator (`issue-research`) are
  independent and need not match.
- **Glyph-only keys are a deliberate halfway house.** 0096 introduced
  `GLYPH_ONLY_DOC_TYPES` precisely so RCA could be drawn (on the templates page)
  without being browsable. Promotion = moving the key from that escape hatch into
  the real `DocTypeKey` union.
- **Two glyph systems, two shapes.** Small `Glyph` = zero-arg `ComponentType`
  rendered `<Icon/>` with CSS-var theme colours (RCA already has one). BigGlyph =
  `(palette) => <g>` draw function with a runtime hue-derived HSL palette (RCA
  needs one authored). They are keyed differently (`GlyphDocType` vs strict
  `DocTypeKey`).

## Historical Context

- `meta/work/0096-templates-view-auto-discovers-templates.md` (in-progress) —
  landed the RCA small glyph + `STEM_TO_GLYPH` + glyph-only key + colour tokens;
  status lag noted by the author. This research confirms that groundwork is
  present and correct.
- `meta/work/0074-per-doc-type-hues-on-detail-page.md` (done) +
  `meta/plans/2026-05-26-0074-...` + `meta/research/codebase/2026-05-24-0074-...`
  — the per-doc-type hue/glyph registry and related-artifacts render path RCA
  extends.
- `meta/work/0041-library-page-wrapper-and-overview-hub.md` (done) +
  `meta/plans/2026-05-16-0041-...` — server-driven library/hub/list semantics
  (count + latest preview).
- `meta/work/0082-big-glyph-hero-illustrations.md` (done) +
  `meta/plans/2026-06-09-0082-...` — the BigGlyph set + draw-function convention.
- `meta/work/0054-sidebar-search.md` (done) + `meta/plans/2026-06-01-0054-...` —
  search endpoint + sidebar UI.
- `meta/work/0057-...` (in-progress epic) + `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
  + `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — the `issue-research`
  frontmatter schema and typed-linkage vocabulary RCA conforms to.
- `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`
  — the one real RCA artifact on disk today; a concrete example of what the
  listing/detail pages will render.

## Open Questions

1. **BigGlyph hero on the detail page vs. empty state.** Acceptance criterion 4
   ("its detail page renders the document with the RCA-specific glyph, hue (310),
   and BigGlyph hero") and criterion 7 ("the detail page shows the RCA BigGlyph
   hero") conflict with both the current implementation and the authoritative
   prototype, where the BigGlyph hero appears only on the **empty-state**
   surface and the populated detail page uses the small framed eyebrow glyph.
   Resolve during planning: either (a) read the AC as "the BigGlyph exists and is
   shown on the RCA empty state" (matching the prototype, lowest-risk), or
   (b) deliberately add a BigGlyph hero to the populated detail page (a new
   pattern not present for any existing doc type, a larger change). The prototype
   supports (a).
2. **Wire token / route spelling.** This research recommends the server
   `wire_str()` and route be `root-cause-analyses` (matching the prototype route
   and the existing frontend glyph-only key), with `config_path_key()` →
   `research_issues` and frontmatter discriminator `issue-research`. Confirm no
   stakeholder expects the route to be `rca` or `issue-research`.
3. **Lifecycle participation.** Should RCA appear in the lifecycle pipeline view?
   The prototype's `LIFECYCLE_OMIT` (`view-lifecycle.jsx:10`) explicitly omits
   `root-cause-analyses` (and decisions). Recommend RCA be `in_lifecycle`-false /
   non-participating, consistent with the prototype and with Operate being a peer
   category outside the linear DEFINE→REMEMBER flow.
4. **`doc_paths` count contract.** Confirm whether adding `research_issues` to the
   live `config_path_key()` mapping changes the asserted `doc_paths.len()` in
   `config_contract.rs` (the key may already be counted in fixtures but not wired
   to a `DocTypeKey`), and update the assertion accordingly.

## Related Research

- `meta/research/codebase/2026-05-24-0074-per-doc-type-hues-on-detail-page.md`
- `meta/research/codebase/2026-06-09-0082-big-glyph-hero-illustrations.md`
- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
  (+ `2026-05-16-0041-...-supplementary.md`)
- `meta/research/codebase/2026-06-01-0054-sidebar-search.md`
- `meta/research/codebase/2026-06-11-0096-templates-view-auto-discovery.md`
- `meta/research/codebase/2026-06-02-0093-extend-templates-with-typed-linkage-slots.md`
