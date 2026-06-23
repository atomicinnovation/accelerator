---
type: codebase-research
id: "2026-06-22-0125-search-id-matching"
title: "Research: Visualiser search matching of work_item_id and external_id"
date: "2026-06-22T15:38:13+00:00"
author: "Phil Helm"
producer: research-codebase
status: complete
work_item_id: "0125"
parent: "work-item:0125"
topic: "How visualiser search ranks/matches entries, and how work_item_id and external_id flow into the search index"
tags: [research, codebase, visualiser, search, indexer]
revision: "9b00844d77584e964f34faac55fd3be02c88220d"
repository: "douala"
last_updated: "2026-06-22T15:38:13+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# Research: Visualiser search matching of work_item_id and external_id

**Date**: 2026-06-22T15:38:13+00:00
**Author**: Phil Helm
**Git Commit**: 9b00844d77584e964f34faac55fd3be02c88220d
**Branch**: douala
**Repository**: douala

## Research Question

For work item 0125 (`meta/work/0125-visualiser-search-id-matching.md`): how does
the visualiser search rank/match entries, and how do `work_item_id` and
`external_id` flow into the search index? The goal is to ground a plan that makes
`/api/search` match the work item ID and the full external (tracker) ID. Specific
questions: how `classify()` and the search handler work; how `work_item_id` and
`external_id` are populated on `IndexEntry`; **whether `external_id` is a field on
`IndexEntry` at all**; the existing test suite and fixtures; and the frontend
empty-state hint copy.

## Summary

- The search ranker, `classify()`, matches a query against exactly three fields:
  `title`, `slug`, and `body_preview`. It never consults any ID field. This is
  the whole bug.
- `work_item_id` **is** a field on `IndexEntry`, populated from the frontmatter
  `id:` key (normalised), so matching it is a small additive change to
  `classify()` mirroring the existing slug tiering.
- **`external_id` is NOT a field on `IndexEntry`, and the string `external_id`
  does not appear anywhere in `server/src`.** It is only reachable through the
  generic `frontmatter: serde_json::Value` blob carried on each entry. So
  matching `external_id` requires either reading it out of that JSON in
  `classify()` (mirroring `extract_facet_value`'s `status` path) or adding a
  dedicated indexed field.
- There is a clean, in-repo precedent for the frontmatter-JSON read:
  `extract_facet_value` reads `status` via
  `entry.frontmatter.get("status").and_then(|v| v.as_str())`, gated on
  `frontmatter_state == "parsed"`.
- The test suite (`tests/api_search.rs`) drives the real HTTP handler against a
  temp-dir corpus. A live gotcha: the shared `seeded_cfg_with_work_items`
  fixtures have **no `id:` frontmatter**, so their `work_item_id` is `None` and
  they cannot exercise ID matching — new tests must write fixtures carrying `id:`
  (and `external_id:` for that case).
- The frontend empty-state already tells users to "Try … a doc id", which the
  backend cannot currently honour.

## Detailed Findings

### Search ranking — `classify()` and the handler

`skills/visualisation/visualise/server/src/api/search.rs`

- The bucket enum (`search.rs:41-48`) is a four-tier ranking:
  `ExactSlug = 0`, `Prefix = 1`, `Interior = 2`, `Body = 3`.
- `classify()` (`search.rs:56-80`) lowercases `title` and `slug` eagerly,
  `body_preview` lazily, and matches in this order: exact slug → title/slug
  prefix → title/slug interior (substring) → body-preview substring. **No ID
  field is read.** This is the precise location of the defect.
- The handler `search()` (`search.rs:95-138`) trims/caps the query
  (`MAX_Q_LEN = 128`), lowercases it, snapshots the index, skips
  `DocTypeKey::Templates`, buckets every entry via `classify()`, sorts each
  bucket by `(mtime desc, rel_path asc)`, then projects to the wire shape via
  `project()` (`search.rs:85-93`), which drops slug-less entries.
- `SearchResultRow` (`search.rs:27-34`) is the wire shape: `doc_type`, `title`,
  `slug`, `mtime_ms` (camelCase on the wire). Note it carries no ID field today;
  matching by ID does not require changing the wire shape (the slug/title still
  identify the row), but a plan may choose to surface the matched ID.

### `work_item_id` — present, frontmatter-`id:`-derived

`skills/visualisation/visualise/server/src/indexer.rs`

- `IndexEntry.work_item_id: Option<String>` exists (`indexer.rs:170`). **Note:**
  the doc comment there currently says "filename-derived via the scan regex",
  but that is stale — see the population path below; the value is actually read
  from frontmatter `id:`.
- Population (`indexer.rs:1346-1382`): for `DocTypeKey::WorkItems`, the slug is
  derived from the filename via the scan regex (tail after the ID), while the
  identity is read from the **frontmatter `id:` key** through
  `read_fm_id("id")`, which routes the raw value through
  `WorkItemConfig::normalise_id`. For non-work-item types, `work_item_id` is
  `None` (`indexer.rs:1383-1384`).
- `normalise_id` (`config.rs:207`) with the default numeric config returns the
  bare digits unchanged (`"0042"` → `"0042"`); with a project code it yields
  `"ENG-0042"`, and passes through already-prefixed foreign keys verbatim.
- Consequence for the bug: a work item `0054-sidebar-search.md` has slug
  `sidebar-search` and `work_item_id` `Some("0054")` (only if its frontmatter
  carries `id: "0054"`). The bare ID `0054` is therefore absent from `title`,
  `slug`, and `body_preview` — confirming why an ID query never matches.

### `external_id` — NOT indexed; only in the frontmatter blob

- `grep -rn "external_id" server/src` returns **zero matches**. `external_id` is
  not a struct field and is not referenced anywhere in the server.
- The only access path is the generic frontmatter JSON on each entry:
  `IndexEntry.frontmatter: serde_json::Value` (`indexer.rs:172`), built at
  `indexer.rs:1398-1412` by copying every parsed frontmatter key/value verbatim
  into a `serde_json::Map` (so the key is exactly `external_id`, snake_case).
  When frontmatter is absent/malformed the value is `Null` and
  `frontmatter_state` is `"absent"`/`"malformed"`.
- **Precedent to mirror** — `extract_facet_value` (`indexer.rs:72-106`) reads the
  `status` facet straight from the blob:
  ```rust
  if entry.frontmatter_state != "parsed" { return None; }
  entry.frontmatter.get("status").and_then(|v| v.as_str()).map(...)
  ```
  An `external_id` read in `classify()` would use the same shape.
- Sparsity: per the work-item template and `create-work-item`, `external_id` is
  omit-when-empty and only present on synced items, and only on work items — so
  the match path must treat absence as the common case (cheap early-out).

### Test suite and fixtures

`skills/visualisation/visualise/server/tests/api_search.rs` and
`tests/common/mod.rs`

- Tests build a real `AppState` over a `tempfile::tempdir()` corpus and drive the
  axum router via `oneshot` (`api_search.rs:13-22`). Assertions inspect the JSON
  `results` array (slug/title/docType/mtimeMs).
- Existing coverage pins bucket ordering (`bucket_1_exact_slug_first`,
  `bucket_2_prefix_before_interior`, `bucket_3_title_slug_before_bucket_4_body_preview`),
  case-insensitivity, template exclusion, mtime/path tiebreaks, the length cap,
  and the "no path/relpath match" negative.
- `common::seeded_cfg` (`mod.rs:15`) seeds decisions/plans/reviews/RCA but
  `work_item: None` → the indexer falls back to `WorkItemConfig::default_numeric()`
  (`server.rs:79-81`).
- **Gotcha** `common::seeded_cfg_with_work_items` (`mod.rs:120-141`) writes
  `0001-todo-fixture.md` etc. **without an `id:` frontmatter key**, so those
  entries get `work_item_id: None` and **cannot** exercise ID matching. New tests
  must write their own work-item fixtures that include `id: "0042"` (and, for the
  external case, `external_id: "ATO-123"`). Confirmed empirically:
  `seeded_cfg_with_work_items` entries show `wid=None` in the index snapshot.
- Tests must be run with `cargo test --no-default-features --features dev-frontend`
  — the default `embed-dist` feature's build script requires a built
  `frontend/dist/index.html`.

### Frontend empty-state copy

`skills/visualisation/visualise/frontend/src/components/Sidebar/SearchResultsPanel.tsx`

- The "No matches" block (`SearchResultsPanel.tsx:99-108`) renders:
  "Nothing in `meta/` matches "{query}". Try a slug, a fragment of a title, or a
  **doc id**." Once the backend matches IDs, this hint becomes truthful; no copy
  change is strictly required, though the plan may align wording.

## Code References

- `skills/visualisation/visualise/server/src/api/search.rs:41-48` — `Bucket` ranking enum.
- `skills/visualisation/visualise/server/src/api/search.rs:56-80` — `classify()`; the matched-field set (the defect site).
- `skills/visualisation/visualise/server/src/api/search.rs:85-138` — `project()` and the `search()` handler.
- `skills/visualisation/visualise/server/src/indexer.rs:162-196` — `IndexEntry` struct (no `external_id` field; has `work_item_id`, `frontmatter`, `frontmatter_state`).
- `skills/visualisation/visualise/server/src/indexer.rs:72-106` — `extract_facet_value`; the frontmatter-JSON read pattern to mirror.
- `skills/visualisation/visualise/server/src/indexer.rs:1346-1384` — work-item slug + `work_item_id` population (frontmatter `id:` via `read_fm_id`).
- `skills/visualisation/visualise/server/src/indexer.rs:1398-1412` — frontmatter → `serde_json` map construction.
- `skills/visualisation/visualise/server/src/config.rs:207` — `normalise_id`.
- `skills/visualisation/visualise/server/src/server.rs:79-81` — default-numeric `WorkItemConfig` when unconfigured.
- `skills/visualisation/visualise/server/tests/api_search.rs:13-22,129-216` — test harness and bucket-ordering tests.
- `skills/visualisation/visualise/server/tests/common/mod.rs:120-141` — `seeded_cfg_with_work_items` (no `id:` → `work_item_id: None`).
- `skills/visualisation/visualise/frontend/src/components/Sidebar/SearchResultsPanel.tsx:99-108` — "No matches" hint copy.

## Architecture Insights

- The ranker is intentionally field-restricted (0054's design): a fixed
  four-bucket order with no per-type ID code path. Adding ID matching is additive
  — slot the ID fields into the existing tiers rather than introduce a new bucket.
- The `ExactSlug` bucket is really an "exact identity" tier; an exact ID match
  belongs there. The plan should decide whether to rename the variant or simply
  reuse it (renaming touches the bucket-ordering tests).
- Two routes for `external_id`, to be decided in planning:
  1. **Read from `entry.frontmatter` in `classify()`** (mirrors
     `extract_facet_value`'s `status`): no struct/indexer change, gated on
     `frontmatter_state == "parsed"`; cheapest, but re-reads JSON per query per
     entry and keeps `external_id` un-normalised.
  2. **Add `external_id: Option<String>` to `IndexEntry`**, populated during
     indexing like `work_item_id`: consistent with the dedicated-field treatment
     of `work_item_id`, friendlier to reuse (e.g. future facets), but touches the
     struct, the indexer, and any serialisation/tests over `IndexEntry`.
  `work_item_id` matching is unaffected by this choice — it is already a field.
- Matching style agreed on the work item: exact + prefix + interior, mirroring
  slug. Interior matching means a bare-number query substring-matches a
  zero-padded `work_item_id` (typing `42` finds `0042`) and substring-matches a
  prefixed `external_id` (typing `123` finds `ATO-123`) — accepted behaviour.

## Historical Context

- `meta/work/0054-sidebar-search.md` (done) — built `/api/search` and the
  four-bucket ranker. Its design note (line ~121) claimed work-item-by-ID lookup
  would work via the exact-slug bucket "without a separate ID-aware code path";
  this research confirms that claim never held, because the work-item slug
  excludes the bare ID. Work item 0125 fixes this gap.
- `meta/work/0041-library-page-wrapper-and-overview-hub.md` (done) — sort +
  faceted filtering, but for the Library **list views**, not the search box;
  prior art for any future "advanced search", which is currently untracked.
- `meta/work/0036-sidebar-redesign.md` (done) — parent epic of 0053/0054/0055.

## Related Research

- None found specific to search matching prior to this document.

## Open Questions

- `external_id` representation: read from the frontmatter blob in `classify()`
  (option 1) vs add an indexed field (option 2)? Recommendation leans option 1
  for a tightly-scoped bug fix, but the plan owns this call.
- Bucket placement for an exact ID hit: reuse `ExactSlug` as-is, or rename the
  variant to reflect "exact identity" (the latter touches ordering tests).
- Whether to align the frontend empty-state copy (no functional need once IDs
  match).
