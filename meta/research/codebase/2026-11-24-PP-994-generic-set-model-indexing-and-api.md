---
type: "codebase-research"
id: "2026-11-24-PP-994-generic-set-model-indexing-and-api"
title: "Generic Set Model: Indexing and API"
date: "2026-11-24T00:27:45+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "PP-994"
parent: "work-item:PP-994"
topic: "What PP-994's generic set model builds on or changes in the visualiser server today"
tags: ["research", "codebase", "visualiser", "indexer", "file-driver", "watcher", "activity-feed", "search", "related", "corpus", "topic-research", "design-inventories"]
revision: "c7d80a5677b676f2ba20ef1a9f6f50b83186dd64"
repository: "accelerator"
last_updated: "2026-11-24T00:27:45+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Generic Set Model: Indexing and API

**Date**: 2026-11-24T00:27:45+00:00
**Author**: Toby Clemson
**Git Commit**: c7d80a5677b676f2ba20ef1a9f6f50b83186dd64
**Branch**: a detached jj working copy, not on a bookmark
**Repository**: accelerator

## Research Question

PP-994 (`meta/work/PP-994-generic-set-model-indexing-and-api.md`) adds a
generic set model to the visualiser server. Which parts of the codebase does
it build on or change? The research covers five things:

- how the server (`cli/visualiser/`) indexes doc types and serves them
  through its API;
- how topic-research sets and design inventories are handled today;
- how search, live update and the activity feed work;
- where a generic set model would fit;
- the work item's planning-time open question on nested doc roots.

All paths are relative to the workspace root. Server paths are shortened to
`server/src/...`, which means `cli/visualiser/server/src/...`.

## Summary

- **One declaration marks a set today.** That is
  `DocTypeKey::nested_manifest_filename()` (`cli/corpus/src/doc_type.rs:165-171`).
  It returns `inventory.md` for DesignInventories and `manifest.md` for
  TopicResearch. Two production sites branch on it generically:
  `file_driver.rs:258` (listing) and `indexer.rs:1514` (slug and title
  source).
- **The index holds one entry per set, and no members.** It is a flat
  `HashMap<PathBuf, IndexEntry>` (`server/src/indexer.rs:247`). The file
  driver lists only `<root>/<dir>/<root-file>`, one level deep
  (`file_driver.rs:275-327`). Member documents and assets are never listed.
- **Every index consumer works per entry and has no root-versus-member
  notion.** That covers the type listing, the counts, the library
  aggregates, search, related, clusters and the watcher diff.
- **The watcher drops every non-`.md` event** (`watcher.rs:118-121`). It
  rescans the whole index on any `.md` event and emits one event per
  changed entry from a before/after diff (`watcher.rs:280-317`).
- **The activity feed only appends.** It is a 50-slot ring buffer that never
  deduplicates (`activity_feed.rs:81-90`).
- Five findings matter most for planning, because they contradict or
  enlarge what the work item assumes:
  1. **The related response reads no `relates_to` today.** The server reads
     `work_item_id`, `target`, `parent` and `related` only
     (`server/src/frontmatter.rs:178-228`), and never calls
     `corpus::linkage`. The criterion asserting a member's `relates_to`
     relation therefore needs new linkage reading, not only attribution.
  2. **Search matches only title, slug and a 200-character `body_preview`.**
     The preview is the first prose paragraph (`frontmatter.rs:75-120`,
     `api/search.rs:57-81`), and no full-body index exists. "A phrase that
     appears only in one member document" is found only if it sits in that
     member's title, slug or first paragraph.
  3. **`GET /api/docs/{path}` already serves any file under any configured
     root, indexed or not.** That includes member documents, and assets
     served as `text/markdown` (`api/docs.rs:44-109`,
     `file_driver.rs:153-161`). The criteria that make an asset path, or a
     member of a set with no root, return not-found are therefore a
     *restriction* of current behaviour.
  4. **Nested doc roots exist.** No config check stops a doc root sitting
     under another. The e2e harness and `api_smoke.rs` both configure
     `research_codebase: meta/research`, which nests the topics and
     design-inventories roots under the Research root. The work item's
     assumption is false for these test configurations; see the
     nested-doc-root section.
  5. **The server's production source names both set types.** `PHASES` in
     `api/library.rs:78-114` lists them in the "discover" phase. That
     conflicts with the criterion that a source search for `TopicResearch`
     and `DesignInventories` finds matches only in doc-type declarations
     and tests.
- **Real design inventories hold far more than `inventory.md` plus
  `screenshots/`.** They include `assets/*.svg` and `prototype-full/` trees
  of HTML, JSX, CSS, TTF and shell files, nested three levels deep, some
  with spaces in their names.
- **No real topic-research set exists in this workspace**
  (`meta/research/topics/` is absent). The only sets are the visualiser
  fixtures and the skill's eval project.

## Detailed Findings

### 1. Doc-type declaration (`cli/corpus`)

`DocTypeKey` (`cli/corpus/src/doc_type.rs:9-25`) has 15 variants.
`DesignInventories` is at :21 and `TopicResearch` at :22.

**Methods:**
- `all()` (:29-47) returns the variants in a fixed order, which the indexer
  relies on.
- `config_path_key()` (:50-68) gives `research_design_inventories` and
  `research_topics`.
- `linkage_type_name()` (:74-92) gives `design-inventory` and
  `topic-research`.
- `label()` (:103-121).
- `wire_str()` (:174-192) gives `design-inventories` and `topic-research`,
  and is used as the URL and API token.

**Predicates:**
- `in_lifecycle` (:124-126) is true for both.
- `carries_target_frontmatter` (:129-137), `participates_in_lifecycle`
  (:140-152), `in_kanban` (:155-157) and `is_virtual` (:160-162) are false
  for both.
- A test pins that the two types share this profile (:255-274).

**Set declaration:** `nested_manifest_filename()` (:165-171) is the only
"this type is directory-shaped" declaration. It carries the root filename
and no presentation. PP-994's set declaration and presentation declaration
would replace or extend it.

**Path inference:** `infer(path, table)` (:206-224) resolves a path to a
doc type by the *longest* segment-anchored root match (`segment_match`
:226-238). It is deterministic when roots nest.

**Dependants:** eleven crates depend on `corpus`, so a change to
`DocTypeKey` reaches all of them, under ADR-0069's dependency rules:
- `corpus-adapters`, `corpus-cli`
- `migrate`, `migrate-adapters`, `migrate-cli`
- `research-adapters`, `research-cli`
- `visualiser/server`
- `work`, `work-adapters`, `work-cli`

**Slugs** (`cli/corpus/src/slug.rs:9-39`):
- TopicResearch keeps the stem verbatim (:34-36).
- DesignInventories strips the date and an optional ID prefix (:25-27,
  `strip_prefix_date_and_optional_id` :135-149). For example,
  `2026-05-06-140608-foo` becomes `140608-foo` (pinned at
  `indexer.rs:2935`).
- `slug.rs` knows nothing of directories. The indexer passes
  `"<dirname>.md"` in.

**Clusters** (`cli/corpus/src/cluster.rs`):
- The typed-linkage `walk` returns `None` for both set types (:216-223), so
  no cluster key is resolved through frontmatter.
- Neither type participates in the lifecycle, so an unbridged set root goes
  to an `__orphan__::<path>` bucket (:384-413).
- A root whose slug equals a work item's slug joins that work item's
  cluster through the slug-to-key bridge (:358-382).
- `derive_completeness` sets `has_design_inventory` for inventories (:565).

**Linkage** (`cli/corpus/src/linkage.rs:212-251`):
- `resolve_path_target` gives a design inventory the id of its parent
  directory (:235-239).
- TopicResearch falls to `_ => stem` (:243), so a manifest path resolves to
  id `manifest`. This asymmetry is latent: the server does not call
  `linkage.rs` at all.

### 2. File driver (`server/src/file_driver.rs`)

**Roots:** `LocalFileDriver::new` (:113-144) maps each `DocTypeKey` to its
canonicalised root (`roots`, :105).

**`list(kind)`** (:243-328):
- Flat types list only the root's direct `*.md` children through
  `markdown_files_in` (:554-597). The extension test `has_md_extension`
  (:543-545) is case-sensitive.
- Work items also read `drafts/` (:263-272).
- Nested types (:275-327):
  - A missing root returns an empty list (:278-280).
  - Entries that are not directories are skipped (:308-310).
  - **Dot-prefixed directories are skipped** (:311-319). That is the only
    dot rule anywhere in the server.
  - `<dir>/<root-file>` is pushed only when that file exists (:320-324).
  - Nothing else in the directory is enumerated.

**`read(path)`** (:330-397):
- It canonicalises the path, and a missing file gives `NotFound`.
- `path_is_allowed` (:153-161) only requires the path to sit under *any*
  configured root or template root.
- Size is capped at 10 MiB (`MAX_DOC_BYTES`, :28).
- It returns `FileContent { bytes, etag, mtime_ms, size }` (:19-26), with
  `etag_of` = `sha256-<hex>` (:514-518). Extensions are not filtered.

**`kind_for_canonical_path`** (:506-511) returns the *first* `starts_with`
match from a `HashMap`. It is not the longest match, so it is ambiguous
when roots nest. Its only caller is `Indexer::refresh_entry`
(`indexer.rs:509-511`), which is reached only from the work-item PATCH.
Rescans take the type from the `list(kind)` call that produced the entry.

### 3. Indexer (`server/src/indexer.rs`, `server/src/frontmatter.rs`)

**`IndexEntry`** (:143-190, serde camelCase) has these fields:
- `type`, `path` (absolute canonical), `relPath`, `slug`
- `workItemId`, `workItemKey`, `identityKey` (skipped)
- `draft`, `title`, `frontmatter`, `frontmatterState`, `workItemRefs`
- `mtimeMs`, `size`, `etag`, `bodyPreview`
- `completeness`, `linkedCount`, `clusterKey`

It has no field for a set, a member, a role or a set-relative path. It
assumes markdown content (`bodyPreview`, `frontmatter`).

**Frontmatter state:** `FrontmatterState::{Parsed, Absent, Malformed}`
(`frontmatter.rs:5-19`) is rendered as `parsed`, `absent` or `malformed`,
and parsed by `corpus_adapters::parse`. The malformed, absent and parsed
marks PP-994 needs for members already exist per entry.

**`build_entry`** (:1502-1609):
- For nested types the slug filename is `"<parent-dir-name>.md"`
  (:1512-1521).
- The slug comes from `corpus::slug::derive` (:1543-1551).
- The title falls back from frontmatter `title`, to the first H1, to the
  humanised directory stem (:1553-1560, `title_from`
  `frontmatter.rs:122-148`).
- `mtime_ms`, `size` and `etag` come from the root file only (:1601-1603).
- **Branching on the parent directory breaks for members at depth.** A
  member at `findings/x.md` would take the slug `findings`, so set-slug
  derivation needs the set directory, not the parent.

**`rescan()`** (:316-455):
- It is a full rebuild under `rescan_lock`.
- Phase 1 lists every type except Templates (:333-345).
- Phase 2 reads the files at concurrency 64 (:351-365).
- Pass A builds entries and the id maps (:389-424). Pass B builds
  `reviews_by_target` (:429-435).
- The six maps are swapped together under ordered write locks (:441-452).

**Other operations:**
- `refresh_one` and `refresh_entry` (:467-623) handle single-path refresh.
  Deletion goes through `find_entry_for_deleted` (:1144-1161).
- `get(path)` (:949-958) does an exact lookup, then a canonicalised one.

**Consumers that will need a root-versus-member rule:**
- `all_by_type` (:697-705) backs `GET /api/docs?type=`.
- `counts_by_type` (:707-713) backs `GET /api/types` counts.
- `library_aggregates` (:725-787):
  - `count` and `latest` are computed per entry. `latest` is the entry
    with the highest `mtime_ms`, tie-broken on `rel_path` (:733-751), and
    is projected through `LatestPreview::from_entry` (:50-58).
  - `filtered_count` and facets are computed per entry (:754-784).
  - The facets are `status` and `clusterSlug` for both set types
    (`facets_for` :62-68). `extract_facet_value` (:72-91) reads frontmatter
    `status` and `entry.slug`.
  - **No per-set latest time exists beyond the root's own `mtimeMs`.**
    Removals leave no record behind, so "latest time = time the removal was
    observed" needs new state held outside the entries map.

**Pinned tests:**
- `design_inventories_indexed_from_nested_directories` (:2885-2942) builds
  a tempdir with two sets, an in-flight
  `.2026-05-06-200000-inflight.tmp/`, and one directory with no root. It
  asserts two entries and the slugs `140608-foo` and `135214-bar`.
- `topic_research_indexes_one_entry_per_set_from_its_manifest`
  (:2944-3008) uses committed fixtures. It asserts four entries, that the
  member titles "Cost model" and "Summary report" are absent, and that
  every `rel_path` ends `manifest.md`.

### 4. HTTP API (`server/src/api/`)

**Router** (`api/mod.rs:26-53`), with the routes it registers:
- `/api/events`, `/api/activity`, `/api/types`, `/api/library/structure`
- `/api/docs`, and `/api/docs/*path`, which carries GET plus PATCH
  `/frontmatter`
- `/api/templates[/:name]`, `/api/lifecycle[/:slug]`
- `/api/related/*path`, `/api/search`, and the config routes

Two router constraints matter:
- matchit 0.7 forbids a catch-all followed by a suffix (:33-40). A
  path-keyed member API therefore needs its own prefix, such as
  `/api/sets/*path`, rather than `/api/docs/*path/members`.
- `server.rs:253-313` adds the healthz and info routes and a JSON 404
  catch-all.

**Errors:** `ApiError` maps NotFound to 404, PathEscape to 403 and
TooLarge/Io to 500 (`mod.rs:55-180`).

**`GET /api/docs?type=`** (`docs.rs:30-42`) returns `{ docs: IndexEntry[] }`
sorted by path. A set's `slug` is already the "set slug" PP-994 names: the
directory name for topic research, and the shortened form for inventories.

**`GET /api/docs/{path}`** (`docs.rs:44-109`):
- It rejects `..` and a leading `/` (:49-51), but does not percent-decode
  the way the related handler does.
- It looks the path up in the index. **If the path is not indexed, it falls
  back to `file_driver.read`** (:75-89), so any file under any root is
  served.
- The response is a raw body with `ETag` and
  `Content-Type: text/markdown`, whatever the extension (:92-108). It has
  no JSON envelope and no slug.
- A directory path fails inside `read` with `Io`, which gives 500, not 404.

So, for PP-994's endpoint criteria:
- A member body is **already** served today.
- An asset path must change from 200 to 404.
- A member of a set with no root must change from 200 to 404.
- The "document response carries the set's slug" criterion needs either a
  response header or a new envelope. The response is raw markdown today.

**PATCH `/api/docs/{path}/frontmatter`** (`docs.rs:144-338`) serves work
items only and needs an index entry. Its cluster recompute (:262-311) is
one of three copies of the same seven steps:
- `server.rs:116-146`
- `watcher.rs:228-255`
- `docs.rs:276-311`

**Search** (`api/search.rs`):
- The row is `{ docType, title, slug, mtimeMs }` (:27-35), and the
  response is `{ results }`.
- Buckets, in order, are ExactSlug, Prefix, Interior and Body (`classify`
  :57-81). Body matching uses `body_preview` only.
- Results are ordered by mtime descending, then `rel_path` (:123-130), with
  no limit.
- `project` drops entries without a slug (:86-94).
- Rows carry no path. A member row would need a new set-relative path field
  (to `SearchResultRow` and the frontend `SearchResult`, `fetch.ts:242-247`)
  with its `slug` holding the set's slug. Because slug is a match field,
  every member would then match a query equal to the set slug in the
  ExactSlug bucket. A ranking rule is needed.

**Related** (`related.rs`, `api/related.rs`):
- The handler (`api/related.rs:30-69`) percent-decodes the path, validates
  it per segment and requires an index entry (404 otherwise). It returns
  `{ inferredCluster, declaredOutbound, declaredInbound }`, each an
  `IndexEntry[]` (:15-21).
- `inferredCluster` (`related.rs:32-51`) is the entry's cluster, found by
  `cluster_key` or else by `slug`, minus the entry itself.
- `declaredOutbound` is `indexer.declared_outbound` (`indexer.rs:1021-1064`):
  the `target:` (types with `carries_target_frontmatter` only, resolved via
  `target_path_from_entry` :1122-1137) plus the `work_item_refs`.
- `declaredInbound` is `reviews_by_target` plus `work_item_refs_by_id`
  (:58-68).
- Declared items are removed from the inferred items (:73-81).
- `work_item_refs` come from `read_ref_keys` (`frontmatter.rs:178-228`):
  `work_item_id` (or a `target:` of the `work-item:` form), `parent`, and
  `related`. **`relates_to` is read nowhere in the server**, and a plan
  named by path in `relates_to` resolves to nothing today. PP-994's
  related criterion (a member's `relates_to` names a plan, and the plan is
  returned, and the member returned inbound) therefore needs
  `relates_to`-to-path resolution and an inbound map for it.
- Members must also stay out of clusters, since inferred items come only
  from clusters. The slug fallback (`c.slug == entry.slug`) would wrongly
  match a member that carries its set's slug in `slug`.

**Library structure** (`api/library.rs`):
- The response is `{ phases[{id,label,docTypes[]}], templates }`, where
  each `LibraryDocType` is
  `{ id, label, count, filteredCount, latest{title,slug,modifiedAt}, filterFacets }`
  (:13-73).
- `PHASES` (:78-114) is **production code** naming `DesignInventories` and
  `TopicResearch` (:88, :90).

**Doc types** (`api/types.rs:14-23`, `doc_type_view.rs:9-41`) return
`DocType { key, label, dirPath, inLifecycle, inKanban, virtual, count }`.
Nothing about set shape is exposed yet. The frontend mirror is `DocType` in
`frontend/src/api/types.ts:112-123`. The set and presentation declarations
would be added to both.

**Lifecycle** (`api/lifecycle.rs:17-35`, `clusters.rs:77-229`):
- `LifecycleCluster` is `{ slug, title, entries, completeness, lastChangedMs, clusterKey }`.
- `compute_clusters_with_backfill` wraps `corpus::cluster::compute`.
- `server/src/lifecycle.rs` is the idle-shutdown watcher, not clustering.

**Where a member API fits:**
- Add a new module under `api/` and mount it in `api::mount`
  (`mod.rs:26-53`), with `State<Arc<AppState>>`.
- The validation-then-`indexer.get`-then-404 shape of `related_get`
  (`api/related.rs:30-57`) is the template to copy.
- A slug-keyed form (`/api/sets/:type/:slug`) would mirror `lifecycle_one`
  (`api/lifecycle.rs:25-35`).
- The choice of shape is PP-994's first-stage decision. This research does
  not make it.

### 5. Live update: watcher, SSE, activity (`server/src/watcher.rs`, `sse_hub.rs`, `activity_feed.rs`)

**Watching** (`watcher.rs:36-43`, `server.rs:422-441`):
- One recursive `notify` watcher covers every doc root and template root.
  The list is canonicalised, sorted and deduplicated.
- Roots that do not exist at startup are never watched (`watcher.rs:149-158`).
- Events pass through a 1024-slot `try_send` channel, so overflow drops
  them (:45-53).

**Filter:** `is_markdown` (:118-121, :368-371) checks the extension
case-insensitively. Directory events and asset events never trigger a
rescan. There is no dot rule in the watcher.

**Debounce:**
- 100 ms per path (`Settings::DEFAULT` :26-30, `pending` map :113-137).
- A later event aborts only the pending sleep, never a rescan in progress
  (:164-189).
- Different paths do not coalesce. N paths cause N rescans, and only the
  first emits anything.

**`on_path_changed`** (:191-267):
1. Canonicalise the path, or its nearest existing ancestor (:379-402).
2. Check templates.
3. Return early on a self-write (:217-220).
4. Take a `before` snapshot, then run a full `rescan()`.
5. Recompute clusters (:228-255).
6. Diff with `rescan_changes` (:280-317), filter self-writes (:262), and
   `emit` each change.

**The diff** (`rescan_changes`):
- It is keyed on absolute path.
- An after-entry with no before-entry of equal `(etag, frontmatter_state)`
  becomes Created or Edited. A malformed one becomes `DocInvalid`
  (`payload_for_entry` :341-366).
- A missing path becomes Deleted.
- Creates and edits come first, then deletes. All changes in one rescan
  share one `Utc::now()` timestamp.

What this means for members:
- **Once members are in the index and the asset filter is relaxed, a
  member change emits exactly one event naturally, carrying the member's
  path.** The diff already gives per-path events.
- Excluded dot files stay silent if they are never indexed. They would
  still trigger harmless rescans unless the watcher also filters them.

**Renames into place:** a `.x.tmp/` to `x/` rename produces directory
events only, and those are dropped. The set appears only at the next `.md`
event anywhere (`a_change_another_files_rescan_absorbed_still_broadcasts`,
:1022-1050). PP-994's "one created event per file on rename" criterion
therefore needs directory events, or rename events, to trigger a rescan.
Whether macOS FSEvents and inotify report child paths after a directory
rename was not verified.

**Root deletion:** deleting `x/manifest.md` already drops the set from the
listing, which requires the root file (`file_driver.rs:320-324`), and
emits Deleted. With members indexed, the same rescan would drop them and
emit a Deleted event for each. That matches the criterion if listing
members stays conditional on the root existing.

**`SsePayload`** (`sse_hub.rs:15-39`):
- `doc-changed { action, docType, path, etag?, timestamp }`
- `doc-invalid { docType, path }`
- `template-changed`
- `ActionKind` is `created`, `edited` or `deleted` (:7-13). `path` is
  project-relative with `/` separators.

**`emit`** (:328-333) pushes to the ring buffer and then broadcasts, so
`/api/activity` holds the row before subscribers see it.

**Activity ring buffer** (`activity_feed.rs`):
- `CAPACITY = 50` (:26).
- A row is `{ action, docType, path, timestamp }` (:28-35). `from_payload`
  (:43-60) keeps only `doc-changed`.
- `push` (:81-90) pops the oldest row and pushes the new one to the front,
  with **no deduplication by path**. The test at `watcher.rs:953-975` pins
  three rows for three edits of one file.
- The timestamp is the observation time, not the file's mtime. The buffer
  is in-process only.

The PP-994 roll-up needs these changes:
- Replace the row by set-root path. That needs the event-to-set mapping at
  `emit` or `push` time.
- Carry the root's own last action. That is state per set, beyond the
  current row.
- Keep the root path as the row's `path`, as PP-996 requires.

**`GET /api/activity`** (`api/activity.rs:37-44`) clamps the limit to 50.
Self-writes from the PATCH broadcast directly (`docs.rs:316-324`) and never
reach the ring buffer.

**Self-write suppression** (`write_coordinator.rs`) uses a 5 s TTL keyed on
the canonical path. Only the kanban PATCH produces self-writes, so set
members are unaffected.

**Frontend consumers** (unchanged by PP-994 apart from the types):
- `use-doc-events.ts:106-152` invalidates the types, docs(type),
  docContent(path), lifecycle and related query caches on every
  `doc-changed` or `doc-invalid`. Member events will invalidate
  `docs(topic-research)` and similar queries. That is harmless, but it adds
  refetches.
- The external-edit toast (`use-external-edit-toast.ts:32-55`) matches an
  exact relPath, so no member route means no toast.
- `ActivityFeed.tsx:123-130` links rows by `fileSlugFromRelPath(path)`.
  That is the PP-996 bug, out of scope here.

**Test patterns:**
- The `Scripted` harness (`watcher.rs:779-837`) injects `notify` events
  through `ScriptedWatcher` and the `EventSink`. It provides `next()`
  (5 s `EVENT_BUDGET`, :552-562) and `assert_quiet(window)`.
- The template for "exactly one event after quiet longer than the
  debounce" is `rapid_writes_coalesce_to_one_event` (:878-889).
- End-to-end: `server/tests/sse_e2e.rs`, with `InjectedWatcher` and
  `read_until`.
- The real-OS test is
  `the_os_watcher_reports_a_write_under_a_registered_directory`
  (:979-1020), run in the `test:integration:visualiser-os-watcher` lane.

### 6. On-disk conventions the model depends on

**Topic research** (`skills/research/research-topic/SKILL.md`; names in
`cli/research/src/topic/layout.rs:10-15`). The set root is
`paths.research_topics`, default `meta/research/topics`
(`cli/config/src/catalogue.rs:73-74`). Each set is laid out as:

```
<slug>/
  manifest.md  brief.md  outline.md  synthesis.md
  .conduct-run.json                  (run ledger, inside the set)
  .tmp-XXXXXX                        (transient atomic-write staging file)
  findings/NN-<q>-<profile>.md
  findings/.NN-...md.invalid         (quarantine)
  findings/NN-...levels/{1.md,2-1.md,...}
  findings/NN-...levels/.<lineage>.md.invalid
  reports/<q>.md
```

- **Findings are flat; rounds are frontmatter stamps, not directories.**
  The path rules reject subdirectories
  (`cli/research/src/topic/layout/finding_path.rs:10-30`). PP-994's
  `findings/round-2/notes/...` fixture is a synthetic depth test, not the
  real layout. The real maximum depth is 3
  (`findings/<stem>.levels/<note>.md`).
- `.conduct-run.json`:
  - It is written by `write_run_ledger` through `AtomicWrite`
    (`cli/research-adapters/src/conduct/ledger.rs:37-42, 89-96`).
  - It is deleted by `end-run` (:103-108;
    `cli/research-cli/src/topic_command.rs:552-567`).
  - It survives a killed run.
- Every atomic write stages a `.tmp-XXXXXX` file in the target directory
  (`cli/store/src/lib.rs:111-137, 341-367`; `TEMP_PREFIX` in
  `cli/kernel/src/lib.rs:15`). It is dot-prefixed, so the dot rule covers
  it, but it is not named in the work item.
- The `.invalid` renames are done by the agent, following skill prose. The
  CLI only reads them (`cli/research-adapters/src/topic.rs:57, 392-404`).
- `.<slug>.tmp/` exists only in skill prose (`SKILL.md:190-204`), and only
  `brief` uses it. Later verbs edit the set in place, editing `manifest.md`
  last.

**Design inventories** (`skills/design/inventory-design/SKILL.md`):
- An inventory is built in `.YYYY-MM-DD-HHMMSS-<source-id>.tmp/`, then
  `inventory.md` is scrubbed of secrets and the directory is renamed into
  place (:255-326, :374-375).
- Screenshots go to `screenshots/{screen-id}-{state}.png`
  (`agents/browser-analyser.md:45, 93`).
- No Rust code writes into inventories.

**Real inventories on disk** (`meta/research/design-inventories/`, six of
them):
- `2026-05-06-140608-claude-design-prototype/` has `assets/*.svg`.
- `2026-05-21-015231-claude-design-prototype/` has
  `prototype-standalone.html` and a `prototype-full/` tree. That tree holds
  HTML files whose names contain spaces and parentheses, plus JSX, CSS and
  `assets/fonts/*.ttf`, three levels deep.
- `2026-09-10-174311-...` has a `prototype-full/` tree including
  `serve.sh`.

The work item's drafting note says the skill writes no `assets/` directory.
That is true of the skill, but real data has one. Under the "by file type"
rule all of this becomes asset members, so asset counts per inventory reach
dozens. The doc root also holds a `.gitkeep`, which is a file at the root
and is already skipped.

**Fixtures** (`server/tests/fixtures/meta/research/`):
- `topics/prompt-caching-economics/` has `manifest.md`,
  `findings/01-cost-model.md` and `reports/summary.md`. Its manifest claims
  `primary: synthesis.md`, but no such file exists.
- `topics/` also holds `mcp-transport-layer/`, `model-context-protocol/`
  and `ac2-coverage/`, plus `empty-set/notes.md` (no root) and
  `.draft-subject.tmp/manifest.md`.
- `design-inventories/` holds `2026-05-26-example/` and
  `2026-05-26-ac2-coverage/`, each with `inventory.md` only.
- No fixture has `.invalid`, `.conduct-run.json`, `.levels/` or assets.
- These fixtures are also the e2e project root
  (`frontend/e2e/start-server.mjs:75`). Indexing members will add member
  search results to e2e runs.

### 7. Nested-doc-root check (planning-time open question)

**Finding: a set doc root can sit under another configured doc root.**
Nothing forbids it, and the committed test configurations already do it.

- `config::paths::doc_type_dirs` (`cli/config/src/paths.rs:32-69`) rejects
  only unsafe paths (`is_unsafe` :118-127). Neither it nor the server
  checks for overlap.
- Under the catalogue defaults the research roots are siblings under
  `meta/research/` (`catalogue.rs:39-76`): `codebase`,
  `design-inventories`, `design-gaps`, `issues` and `topics`. So nothing
  nests by default, and this workspace's `.accelerator/config.md`
  overrides no paths.
- But two test setups write `paths: research_codebase: meta/research`, so
  the topics and design-inventories roots sit inside the Research root:
  - `frontend/e2e/start-server.mjs:82-85`, with the rationale at :68-73;
  - `server/tests/api_smoke.rs:24-27`.
- What nesting does today:
  - Listing stays disjoint, because Research is a flat type and lists only
    `meta/research/*.md` (`markdown_files_in`, `file_driver.rs:554-597`).
    So set members would never be double-listed as research documents.
  - Reads are unaffected, because `path_is_allowed` accepts any root.
  - Both roots are watched recursively, and overlapping events only cause
    extra rescans.
  - Type resolution through `kind_for_canonical_path` (`file_driver.rs:506-511`)
    is order-dependent. `corpus::doc_type::infer` (longest match) is not.

**Decision taken here** (there was no user to ask, so this records the
outcome for the author to confirm in the decisions line). The assumption
"a set doc root does not sit under another configured doc root" is false
for the e2e and smoke configurations, and is not enforced for users. The
indexing requirements still hold if two conditions are met:
- every member's type comes from the set it was listed under, as `rescan`
  already does, and never from `kind_for_canonical_path`, or that function
  switches to longest match;
- every enclosing doc type keeps a flat listing.

A recursive listing for any enclosing type would reopen the question. The
author should decide between two options:
- accept nesting, with those two rules written into the ADR;
- add a configuration guard against overlapping roots, which would break
  the e2e and smoke setups.

### 8. Where a generic set model fits

These are the seams, in the order data flows through them:

1. **Declaration** (`cli/corpus/src/doc_type.rs:165-171`): replace or
   extend `nested_manifest_filename()` with a set declaration (the root
   file) and a presentation declaration (set page or entry-point). Phase
   membership (`api/library.rs:78-114`) is the one other per-type table in
   server production code. Moving it onto `DocTypeKey` would satisfy the
   source-search criterion.
2. **Enumeration** (`file_driver.rs:275-327`): after admitting a set
   directory, walk it recursively. Skip dot-prefixed names at every depth,
   classify `.md` as a document and anything else as an asset, and record
   the set-relative path. The walk applies only when the root file exists.
   The doc-root-level dot rule already exists (:311-319).
3. **Index model** (`indexer.rs:143-190`, :247, :1502-1609): records need
   a role (root or member), the set slug (derived from the set directory,
   not the immediate parent) and the set-relative path. Assets need a
   lighter record carrying path, size and mtime. `IndexEntry`'s markdown
   fields do not fit an asset. A per-set "last observed change" time,
   which survives removals, is new state.
4. **Consumers**: each needs a root-only or member-aware rule.
   - Root only: `all_by_type`, `counts_by_type`, `library_aggregates`
     (count and facets), and cluster input at its three sites
     (`server.rs:116-146`, `watcher.rs:228-255`, `docs.rs:276-311`).
   - Member-aware: search projection (`search.rs:86-94`), `resolve_related`
     (`related.rs:22-88`) and `library_aggregates` latest time.
5. **Document endpoint** (`docs.rs:44-109`): remove the unindexed-read
   fallback, or limit it to indexed documents, so that assets and orphaned
   members return 404.
6. **Watcher** (`watcher.rs:118-121`): admit asset and directory events,
   so assets and renames trigger a rescan. Optionally drop dot paths
   early. `rescan_changes` then yields per-member events as it is.
7. **Activity** (`activity_feed.rs:43-90`, `watcher.rs:328-333`): add a
   set-aware push that replaces the row keyed on the root path, keeps the
   root's action and updates the time.
8. **Member API**: a new `api/` module (see section 4).
9. **Doc types response** (`doc_type_view.rs`, `types.ts:112-123`): expose
   both declarations.

## Code References

**Corpus crate:**
- `cli/corpus/src/doc_type.rs:9-25` - the `DocTypeKey` enum.
- `cli/corpus/src/doc_type.rs:165-171` - `nested_manifest_filename`, the
  only set declaration today.
- `cli/corpus/src/doc_type.rs:206-238` - `infer`, longest-match path to
  type.
- `cli/corpus/src/slug.rs:25-36` - slug rules for the two set types.
- `cli/corpus/src/cluster.rs:216-223, 384-413` - no cluster key for sets;
  orphan bucketing.
- `cli/corpus/src/linkage.rs:235-243` - inventory id from the directory,
  topic id from the stem (`manifest`).

**File driver:**
- `server/src/file_driver.rs:153-161` - `path_is_allowed`, which accepts
  any root.
- `server/src/file_driver.rs:275-327` - nested listing: one level deep, dot
  directories skipped, root file only.
- `server/src/file_driver.rs:330-397` - `read`, with no extension filter.
- `server/src/file_driver.rs:506-511` - `kind_for_canonical_path`, first
  match.
- `server/src/file_driver.rs:554-597` - `markdown_files_in`, flat with no
  dot filter.

**Indexer and frontmatter:**
- `server/src/indexer.rs:143-190` - the `IndexEntry` wire shape.
- `server/src/indexer.rs:247` - the entries map.
- `server/src/indexer.rs:316-455` - `rescan`.
- `server/src/indexer.rs:697-787` - `all_by_type`, `counts_by_type`,
  `library_aggregates`.
- `server/src/indexer.rs:1021-1064` - `declared_outbound`.
- `server/src/indexer.rs:1502-1609` - `build_entry`, with the nested slug
  branch at :1512-1521.
- `server/src/indexer.rs:2885-3008` - the two pinned one-entry-per-set
  tests.
- `server/src/frontmatter.rs:5-19, 75-120, 178-228` - frontmatter state,
  `body_preview` (200 characters) and the ref keys (no `relates_to`).

**API:**
- `server/src/api/mod.rs:26-53` - the router and the matchit suffix
  constraint.
- `server/src/api/docs.rs:44-109` - the document fetch, with its unindexed
  fallback at :75-89.
- `server/src/api/docs.rs:262-311` - the cluster recompute, one of three
  copies.
- `server/src/api/search.rs:27-35, 57-94` - the search row, the buckets
  and the projection.
- `server/src/related.rs:22-88`, `server/src/api/related.rs:15-69` -
  related resolution.
- `server/src/api/library.rs:78-114` - `PHASES`, which names both set
  types in production code.
- `server/src/api/types.rs:14-23`, `server/src/doc_type_view.rs:9-41` -
  the doc types response.

**Watcher and live update:**
- `server/src/watcher.rs:118-121, 368-371` - the `.md`-only event filter.
- `server/src/watcher.rs:191-267, 280-317, 328-333` - rescan, diff and
  emit.
- `server/src/watcher.rs:779-889` - the scripted watcher test harness.
- `server/src/sse_hub.rs:7-39` - the change event wire type.
- `server/src/activity_feed.rs:26-100` - the ring buffer.

**Configuration:**
- `cli/config/src/paths.rs:32-69`, `cli/config/src/catalogue.rs:39-76` -
  doc root config, with no overlap check.
- `cli/visualiser/frontend/e2e/start-server.mjs:82-85`,
  `server/tests/api_smoke.rs:24-27` - nested doc roots in the tests.

**Frontend:**
- `cli/visualiser/frontend/src/api/types.ts:112-167`,
  `frontend/src/api/fetch.ts:242-247` - the frontend response types to
  extend.

**On-disk writers:**
- `cli/research-adapters/src/conduct/ledger.rs:37-42`,
  `cli/store/src/lib.rs:341-367` - the run ledger and `.tmp-` staging
  files.

## Architecture Insights

- **The index is the only source for every read endpoint except the
  document body.** The watcher derives events by diffing index snapshots.
  If members enter the index, change events follow for free. The work is
  then mostly in teaching each consumer to filter on role.
- **Full rescans and per-path debounce are simple but coarse.** A set
  arriving with N files can trigger up to N rescans, and only the first
  emits. That is correct but costly, and the costs grow once inventories'
  dozens of assets are watched.
- **Set handling already branches generically** on
  `nested_manifest_filename()`, never on a variant. `PHASES` is the only
  per-variant production table, and the frontend glyph maps are
  frontend-only.
- **The cluster recompute is duplicated three times.** Any rule that
  excludes members from clusters must be applied at all three sites, or
  factored into one.
- **Slug is overloaded.** It is an identity for links, a search match
  field, a cluster bridge key, a facet value (`clusterSlug`) and a
  related-cluster lookup key. Putting the set's slug on members, as
  PP-994 requires for search rows, must not leak into the cluster, facet
  or related-fallback uses.

## Historical Context

- `meta/work/PP-862-topic-research-visualiser-doc-type.md`,
  `meta/research/codebase/2026-09-10-PP-862-topic-research-visualiser-doc-type-indexer.md`
  and `meta/plans/2026-09-10-PP-862-topic-research-visualiser-doc-type.md`
  introduced the topic-research type, one entry per set. This story
  reverses that pin.
- `meta/plans/2026-04-21-meta-visualiser-phase-3-file-driver-indexer-api.md`
  and `2026-04-22-meta-visualiser-phase-4-sse-hub-and-notify-watcher.md`
  are the original designs for the indexer and watcher.
- `meta/plans/2026-04-25-meta-visualiser-phase-6-lifecycle-clusters-and-view.md`
  and `2026-06-01-lifecycle-clustering-composite-key.md` cover lifecycle
  clustering.
- `meta/research/codebase/2026-05-13-PP-77-sidebar-activity-feed.md` and
  `2026-06-01-PP-76-sidebar-search.md` cover the activity feed and search.
- `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md`
  is the nearest precedent for the set ADR.
- `ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
  governs the `corpus` edge. `ADR-0034-typed-linkage-vocabulary.md` defines
  `relates_to`, which the server does not yet read.
- No ADR covers the visualiser index, watcher, activity feed, search or
  clusters. The highest ADR is ADR-0076, so the next number is still 0077.
- Sibling items: PP-995, PP-996, PP-998, PP-1000, PP-1001, PP-1002 and
  PP-134, with reviews under `meta/reviews/work/`. The reviews of PP-994
  are `meta/reviews/work/PP-994-generic-set-model-indexing-and-api-review-{1,2}.md`.

## Related Research

- `meta/research/codebase/2026-09-10-PP-862-topic-research-visualiser-doc-type-indexer.md`
- `meta/research/codebase/2026-05-13-PP-77-sidebar-activity-feed.md`
- `meta/research/codebase/2026-06-01-PP-76-sidebar-search.md`
- `meta/research/codebase/2026-05-24-PP-90-related-documents-inference-accuracy.md`
- `meta/research/codebase/2026-06-01-lifecycle-clustering-slug-mismatch.md`
- `meta/research/codebase/2026-05-27-PP-61-toaster-and-external-edit-notifications.md`

## Open Questions

- **Search depth.** Should member search stay with title, slug and the
  200-character `body_preview`, or does PP-994 add full-body matching? The
  criterion's "phrase that appears only in one member document" passes
  under the present design only if the phrase sits in the first paragraph.
- **The `relates_to` scope.** The related response reads no `relates_to`
  today. Is adding `relates_to` resolution (outbound by path or id, plus an
  inbound map) in PP-994's scope? The related criterion requires it. This
  is a size increase not counted in the Technical Notes.
- **Slug on the document response.** `GET /api/docs/{path}` returns raw
  markdown. Should the set slug go in a header, or in a new JSON envelope?
  An envelope breaks `fetchDocContent`, and the frontend change is
  confined to types.
- **The `PHASES` table.** Should phase membership move onto `DocTypeKey`,
  or should the source-search criterion admit `api/library.rs`?
- **Rename detection.** Do the macOS FSEvents and Linux inotify backends,
  as wrapped by `notify`, deliver anything for the files inside a renamed
  directory? Or must the watcher treat a directory event as a trigger for
  a rescan? Answering this needs a real-OS test.
- **The nested-doc-root outcome** (section 7). The author must choose
  between accepting nesting with the listing and type-resolution rules,
  and guarding against it.
- **Asset volume and names.** Real inventories hold dozens of assets with
  spaces and parentheses in their names, three levels deep. The member
  API's response size, and path encoding in the member API, need a
  decision.
- **Fixture blast radius.** The shared fixtures double as the e2e project.
  Indexing `prompt-caching-economics`'s members changes e2e search
  results. Existing Playwright expectations were not checked.
