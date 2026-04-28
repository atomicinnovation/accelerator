---
date: "2026-04-27T00:00:00Z"
type: plan
skill: create-plan
ticket: null
status: draft
---

# Phase 9 — Cross-references and wiki-links implementation plan

## Overview

Make in-body `[[ADR-NNNN]]` and `[[TICKET-NNNN]]` references render as
clickable library deep-links, and replace the placeholder "Related
artifacts" aside on every doc page with a populated list combining
slug-cluster siblings (tagged `inferred`) and declared cross-references
(tagged `declared`). The only declared cross-reference populated in the
wild today is `target:` on plan-reviews; the plan rendering is
**bidirectional** — the review's library page links to the target plan,
and the target plan's library page lists every review pointing at it.

Test-driven throughout: every step writes the failing test first, then
the implementation. The build stays green between steps except inside
the step that introduces a new failing test.

## Current State Analysis

Phase 8 left a fully-functional kanban write path. Phase 9 starts from
that base.

### Server (`skills/visualisation/visualise/server/`)

- **`Indexer` already maintains ID lookups** for ADRs and tickets. The
  field names in code are `adr_by_id` / `ticket_by_number` (not the
  research's pluralised "adrById"); both expose async lookup methods
  `Indexer::adr_by_id(id)` / `Indexer::ticket_by_number(n)` returning
  `Option<IndexEntry>`. They are populated on `rescan()` and
  `refresh_one()`, and cleaned up on deletion (see
  `src/indexer.rs:36-37, 87-91, 220-228, 282-310`).
- **No reverse declared-link index exists.** The plan-review `target:`
  field is parsed as ordinary frontmatter and stored on the review's
  `IndexEntry.frontmatter` JSON, but no map of *plan path → list of
  reviews pointing at it* is materialised. Adding this is the central
  server-side work for Phase 9.
- **Slug-cluster computation is already in place.** `compute_clusters()`
  runs on rescan and at watcher debounce; the result lives in
  `AppState.clusters: Arc<RwLock<Vec<LifecycleCluster>>>`
  (`src/clusters.rs:32-65`, `src/server.rs:75`). Phase 9 reuses this for
  the inferred half of the related list.
- **API surface lives at `/api/docs/*path`** with both GET (fetch) and
  PATCH (frontmatter) sharing the route via matchit's catch-all
  limitation (`src/api/mod.rs:27-30`). Adding another suffix on the
  same `*path` would require yet more in-handler dispatch. A separate
  route `/api/related/*path` sidesteps the issue.
- **Watcher already broadcasts `doc-changed` events on every index
  update.** Cross-doc invalidation will piggy-back on those events
  rather than introducing a new SSE event kind.
- **Test fixtures at `tests/fixtures/meta/`** already include an
  ADR (`ADR-0001-example-decision.md`), three tickets, three plans,
  and two plan-reviews. The shared `tests/common::seeded_cfg` helper
  also seeds `meta/plans/2026-04-18-foo.md` and
  `meta/reviews/plans/2026-04-18-foo-review-1.md` (with
  `target: "meta/plans/2026-04-18-foo.md"`) — perfect for the
  declared-link round-trip test.

### Frontend (`skills/visualisation/visualise/frontend/`)

- **`MarkdownRenderer` is a thin react-markdown wrapper** taking only
  `content: string`; it has no plugin slot for wiki-link rewriting yet
  (`src/components/MarkdownRenderer/MarkdownRenderer.tsx:6-21`). Its
  test file already covers headings, GFM tables, code blocks, raw-HTML
  XSS guards, and `javascript:` URL guards
  (`MarkdownRenderer.test.tsx`).
- **`LibraryDocView` renders a placeholder** `"No related artifacts
  yet."` aside (`src/routes/library/LibraryDocView.tsx:73-78`). The
  view already has the `IndexEntry` for the doc and its content
  (`docs[]` query + `docContent` query), so populating the aside only
  needs one new fetch.
- **TanStack Query is wired with SSE invalidation** via
  `useDocEvents` and `query-keys` constants (`src/api/use-doc-events.ts`,
  `src/api/query-keys.ts`). Adding a `related(relPath)` key follows the
  same pattern.
- **No remark/rehype plugin authoring precedent yet.** `MarkdownRenderer`
  uses the bundled `remark-gfm` and `rehype-highlight`. A small custom
  remark plugin (or react-markdown `components` override on text nodes)
  is the simplest insertion point.
- **Self-cause filter** (`src/api/self-cause.ts`) is for kanban writes
  only; not relevant here since Phase 9 has no write path.

### Key Discoveries

- **`Indexer::adr_by_id` and `Indexer::ticket_by_number` already exist**
  and are tested. Phase 9 adds *one* more secondary index
  (`reviews_by_target`). The three are kept as explicit
  update/remove blocks rather than abstracted behind a trait, because
  the maps are heterogeneous (`u32 → PathBuf` vs `PathBuf → BTreeSet<PathBuf>`)
  and a shared trait surface ended up empty in an earlier draft.
- **`project_root` is canonicalised once at indexer construction.**
  Phase 9 introduces this discipline: the indexer holds a canonical
  `project_root` field and every secondary-index key derivation
  routes through it. `normalize_target_key` does a *purely lexical*
  `project_root.join(raw)` plus a lexical clean — never touching the
  filesystem. Because both write side (`target:` strings joined onto
  the canonical root) and read side (`entry.path` from the canonical
  primary key) share the same canonical prefix, the secondary key is
  stable whether or not the target file exists, *and* a lookup by
  canonical entry path finds entries inserted by repo-relative
  `target:` strings — no symlink-induced drift on macOS or
  bind-mounted hosts.
- **The `target:` field is a repo-relative path** like
  `meta/plans/2026-04-18-foo.md` (verified in research §7 and the test
  fixture at `tests/fixtures/meta/reviews/plans/...-review-1.md`).
  Per-segment sanitisation rejects absolute paths, `..`, `.`, NUL,
  and backslash. Malformed values do not block the review's primary
  index entry — they just contribute no reverse-index key.
- **matchit 0.7 forbids catch-all + literal suffix on the same route.**
  Phase 8 worked around this for PATCH `/api/docs/*path/frontmatter`
  by registering a single `*path` route and stripping the suffix in
  the handler. Phase 9 picks the cleaner alternative — a separate
  prefix `/api/related/*path` — because the GET method is already
  taken on `/api/docs/*path` for `doc_fetch`, and adding suffix
  dispatch *inside* `doc_fetch` would entangle two unrelated
  responses.
- **Wiki-link resolver is pure-client.** The full ADR + ticket lists
  are already cached client-side via `fetchDocs('decisions')` and
  `fetchDocs('tickets')` (loaded for the sidebar/library); both lists
  are tiny. The resolver returns a tagged
  `{ kind: 'resolved' | 'unresolved' | 'pending' }` so the renderer
  can style cache warm-up distinctly from broken references. While
  the docs caches are still warming, every lookup returns `pending`
  and `[[…]]` references render as `wiki-link-pending` markers
  (neutral skeleton — visibly "loading"); once the caches settle,
  the resolver returns `resolved` (anchor) or `unresolved` (broken-ref
  marker), and TanStack Query re-renders so pending markers flip.
  The body itself is never gated — docs without wiki-links incur no
  perceptible delay.
- **Resolved IDs use `IndexEntry.relPath` to construct library URLs.**
  The library URL form is `/library/:type/:fileSlug` where `fileSlug`
  is the filename stem (per `path-utils::fileSlugFromRelPath`). The
  resolver returns the entry's title alongside the URL so the rendered
  anchor displays the title rather than the bracket-form.
- **Bracket-shape references that fail to resolve render as
  diagnostic spans, with a *visual distinction* between
  cache-warming (`wiki-link-pending`, neutral skeleton) and
  settled-no-match (`unresolved-wiki-link`, muted dotted-underline).**
  Authors editing docs see broken refs only after the caches have
  settled — the cold-load flicker doesn't masquerade as a
  broken-references doc.
- **`derive_completeness` is already cluster-internal.** Phase 9's
  related-artifacts response carries the IndexEntry list only — no
  completeness flags — because the consumer (`RelatedArtifacts`
  component) renders a flat list, not a pipeline.
- **Reverse-index update on review deletion needs care.** When a
  plan-review is deleted, `refresh_one`'s NotFound branch reads the
  entry being removed *before* deletion and passes it to each
  `remove_from_*` helper so the implementation can derive the
  now-stale key. The same previous-entry-read happens on the Ok
  branch (target migration) so the old key is dropped before the new
  one is inserted. Both branches hold a single `entries.write()` lock
  across the secondary-index update sequence, so readers observe a
  consistent (entries, secondary-indexes) snapshot.

## Desired End State

After this phase ships:

1. A markdown body containing `[[ADR-0017]]` renders as a clickable
   link to `/library/decisions/ADR-0017-configuration-extension-points`
   (or whatever filename matches `adr_id: ADR-0017`) when that ADR is
   in the index. The link's *display text* is the ADR's title (e.g.
   "Configuration extension points") and its hover tooltip is the
   bracket-form `[[ADR-0017]]` for source-form fallback.
2. `[[TICKET-0001]]` renders as a link to
   `/library/tickets/0001-three-layer-review-system-architecture`
   with the ticket's title as link text when ticket 1 is in the
   index.
3. When an ID's prefix matches but no entry resolves *after* both
   docs caches have settled (`[[ADR-9999]]`, typo, missing fixture),
   the bracket text renders inside a styled
   `<span class="unresolved-wiki-link">` whose tooltip is a
   diagnostic message ("No matching ADR found for ID 9999"). Authors
   get visible feedback that the syntax was recognised but the
   target wasn't found.
4. While either docs cache is still warming, recognised wiki-links
   render inside a `<span class="wiki-link-pending">` (neutral
   skeleton/italic, tooltip "Loading reference…") rather than the
   broken-reference marker — readers and authors can distinguish
   "loading" from "broken" at a glance.
5. Bare `[[0001]]` (no prefix), unknown prefixes (`[[EPIC-0001]]`),
   and case-mismatched prefixes (`[[adr-0001]]`) render as plain text
   with no special styling — the syntax is not recognised, so the
   prefix namespace stays free for future ID kinds.
5. The "Related artifacts" aside on every doc page in the library
   shows up to three visually distinct groupings:
   - **Targets** (declared) — for plan-reviews, the plan named in
     `target:`.
   - **Inbound reviews** (declared) — for plans, every review whose
     `target:` resolves to this plan.
   - **Same lifecycle** (inferred) — entries that share a slug with
     the current doc (excluding self).
   Each declared item carries a "declared" badge and a solid
   border-left modifier; each inferred item carries an "inferred"
   badge and a dashed border-left modifier. A short legend under the
   section heading explains the distinction.
6. When no slug-cluster siblings and no declared links exist, the
   aside shows the message
   `"This document has no declared or inferred relations."`.
7. When `fetchRelated` fails, the aside renders an element with
   `role="alert"` carrying the error message — matching the existing
   `LibraryDocView` error pattern.
8. Saving a plan-review with a new `target:` (or editing the
   `target:`) updates the corresponding plan's library page within
   one debounce cycle: the plan's "Inbound reviews" list reflects
   the new state. SSE-driven, no manual refresh. During the update
   window, a subtle "Updating…" hint appears on the related aside.
9. Deleting a plan-review removes it from the target plan's "Inbound
   reviews" list within one debounce cycle. Deleting a target plan
   leaves the reviews' lexical-key inbound entries intact (deferred
   materialisation: the back-reference re-materialises when the
   target is recreated).
10. The frontend never throws or warns when a `target:` field is
    missing, malformed (number, null, empty, path-escape), or points
    at a non-existent path; the entry is admitted to the primary
    index but contributes no reverse-index key.

### Verification

- `curl http://127.0.0.1:<port>/api/related/meta/plans/2026-04-18-foo.md`
  returns 200 with a JSON body whose `declaredInbound` array contains
  one entry — the plan-review whose `target:` points at the plan.
- `curl http://127.0.0.1:<port>/api/related/meta/reviews/plans/2026-04-18-foo-review-1.md`
  returns 200 with a `declaredOutbound` array containing the target
  plan.
- `curl http://127.0.0.1:<port>/api/related/meta/plans/non-existent.md`
  returns 404.
- `cargo test -p accelerator-visualiser` is green.
- `npm test` (vitest) inside `frontend/` is green.

## What We're NOT Doing

- **Promote-inferred-to-explicit affordance**. Visually distinguishing
  inferred from declared is the seed; one-click promotion is post-v1
  roadmap.
- **Bare `[[NNNN]]` resolution**. Required prefix is a Phase 9 invariant
  (per spec D6) — adding bare-form support would foreclose future ID
  kinds (`[[EPIC-NNNN]]`, etc.).
- **Resolution of `[[…]]` inside frontmatter values, code blocks, or
  inline code spans**. Only body text. The remark plugin runs on
  `text` nodes outside `code`/`inlineCode` nodes.
- **Other declared cross-reference fields** (`ticket:`, `supersedes:`,
  `related:`, etc.). Per research §7, none of these are populated in
  the wild today. The reverse index design accommodates them, but
  Phase 9 wires only `target:` end-to-end. Activation of other fields
  is a follow-up when authoring skills start populating them.
- **A new write path**. Cross-references are read-only in v1.
- **Type filters or grouping in the related-artifacts list beyond
  inferred/declared**. The list is short by design.
- **Backlink-graph view** (post-v1 roadmap).
- **Resolution caching / memoization**. The lists are tiny; rebuild
  per render is cheap.
- **Showing related artifacts on the templates pages**. Templates are
  excluded from the related view, mirroring their exclusion from
  lifecycle.

## Implementation Approach

Seven inner phases, each TDD:

1. **Server reverse index** — `Indexer.reviews_by_target` (lexical
   key built from canonicalised-once `project_root`, value
   `BTreeSet<PathBuf>` for deterministic order + dedup-by-construction).
   Three explicit secondary-index update/remove blocks in
   `rescan` / `refresh_one` / deletion, all under a single
   `entries.write()` lock for atomicity.
2. **Server related endpoint** — `GET /api/related/*path` returning
   `inferredCluster` + `declaredOutbound` + `declaredInbound` with
   explicit `#[serde(rename_all = "camelCase")]` on the response
   struct, dedup of overlapping inferred/declared entries, and
   per-segment validation that runs on the decoded path.
3. **Frontend wiki-link resolver (pure)** — bounded
   `WIKI_LINK_PATTERN`, `buildWikiLinkIndex` with explicit radix +
   kind filtering + deterministic duplicate-ID tie-breaker,
   `resolveWikiLink → ResolvedWikiLink | null` returning both
   `href` and `title`.
4. **Frontend MarkdownRenderer integration** — opt-in
   `resolveWikiLink` prop wired through a custom remark plugin that
   emits `Link` nodes for resolved refs (with title as display text),
   `<span class="wiki-link-pending">` markers for cache-warming
   refs, and `<span class="unresolved-wiki-link">` markers for
   settled-but-no-match refs. Plugin tuple is memoised on resolver
   identity so the pipeline re-runs precisely on resolver rotation.
   Plugin is pinned by an XSS test for resolver-supplied dangerous
   URLs.
5. **Frontend related fetch + types + hooks** — `fetchRelated`,
   `RelatedArtifactsResponse`, `useRelated`,
   `useWikiLinkResolver`, `useDocPageData`; SSE invalidation of the
   `related` prefix with `refetchType: 'all'`.
6. **Frontend RelatedArtifacts component + LibraryDocView wiring** —
   replace placeholder; render Targets / Inbound reviews / Same
   lifecycle groups under `<h4>`s with element-named CSS modifiers,
   a legend, an `isFetching` updating-hint, and a `role="alert"`
   error path. Body rendering is *not* gated on resolver readiness;
   wiki-links render as muted unresolved-marker spans while the docs
   caches warm and flip to anchors on settle.
7. **Fixtures + cross-cutting smoke** — second-review fixture for
   the multi-inbound case plus two frontend integration smokes
   covering the active-refetch path (Test A) and the
   inactive-cached-query refetch-on-remount path (Test B, which is
   the contract `refetchType: 'all'` exists for).

Each step writes its failing test first. Step ordering keeps the build
green except inside the step that introduces a new failing test.

---

## Phase 1: Reverse declared-link index in `Indexer`

### Overview

Add a third secondary index to `Indexer`:
`reviews_by_target: HashMap<PathBuf, BTreeSet<PathBuf>>`. Keys are
**lexically-clean** absolute paths of target plans (or any other future
target type), built against a `project_root` that is **canonicalised
once at indexer construction**. Values are sets of canonicalised
absolute paths of reviews referencing that target.

The combination of "canonical project_root" + "lexical join + lexical
clean" ensures two things at once:
- The secondary key is stable whether or not the target file exists
  (deferred materialisation works — Step 1.4).
- The secondary key matches the indexer's primary key shape (canonical
  prefix + filename), so a lookup keyed by `entry.path` (canonical)
  finds entries that were inserted via `target:` strings (lexical
  joined onto the canonical root).

Without canonicalising `project_root`, hosts that reach the project
through a symlink (macOS `/var` → `/private/var`, Linux bind-mounts,
dev-container path mappings) would silently produce keys with the
non-canonical prefix on the write side and the canonical prefix on
the read side, defeating every inbound lookup. The single
`canonicalize` at construction sits *outside* `normalize_absolute`,
which itself stays purely lexical.

The value shape `BTreeSet<PathBuf>` (rather than `Vec<PathBuf>`) gives
deterministic iteration order and dedup-by-construction, eliminating
the duplicate-on-refresh hazard and obviating an explicit sort step.

Phase 1 keeps the three secondary indexes as **separate explicit
update/remove blocks** in `rescan()`/`refresh_one()`/deletion. An
earlier draft introduced a `SecondaryIndex` trait registry, but the
three indexes are heterogeneous (`u32 → PathBuf` for the two existing
maps, `PathBuf → BTreeSet<PathBuf>` for the new one) and the shared
trait surface ended up empty. Three explicit blocks read straightforwardly
and the per-index logic is co-located with its data.

### Changes Required

#### 1. `src/indexer.rs` — canonical project_root + ReviewsByTarget

```rust
use std::collections::{BTreeSet, HashMap};
use tokio::sync::RwLock;

pub struct Indexer {
    // ... existing fields ...
    /// Canonicalised once at the top of `Indexer::build`. Every
    /// secondary-index key derivation routes through this prefix so
    /// primary keys (canonical via `build_entry`) and secondary keys
    /// (lexical `project_root.join(raw)`) share the same canonical
    /// prefix. Marked private so no caller can reset it post-construction.
    project_root: PathBuf,
    reviews_by_target: Arc<RwLock<HashMap<PathBuf, BTreeSet<PathBuf>>>>,
}

impl Indexer {
    /// Modify the existing `Indexer::build` (the project's only
    /// constructor — we do *not* introduce a second one). The first
    /// step canonicalises `project_root` so every downstream consumer
    /// — `rescan`, `refresh_one`, the secondary-index helpers, and
    /// `Indexer::reviews_by_target` — sees a canonical path. Failures
    /// are mapped into the existing `FileDriverError` channel so call
    /// sites do not learn a new error type.
    pub async fn build(
        driver: Arc<dyn FileDriver>,
        project_root: PathBuf,
    ) -> Result<Self, FileDriverError> {
        let project_root = tokio::fs::canonicalize(&project_root)
            .await
            .map_err(FileDriverError::from)?;
        // ... existing initialisation, now using the canonical
        // `project_root` for all stored fields and downstream calls ...
    }

    /// Lookup reviews whose `target:` resolves to the given path.
    /// Lock-ordering: acquires `entries.read()` *before* the secondary
    /// `reviews_by_target.read()` so reader and writer share a single
    /// canonical lock-acquisition order (entries → secondary). This
    /// avoids the writer-preferring-RwLock starvation pattern where
    /// a queued writer would block readers that already hold a
    /// secondary lock.
    pub async fn reviews_by_target(&self, target: &Path) -> Vec<IndexEntry> {
        let key = normalize_absolute(target);
        let entries = self.entries.read().await;
        let map = self.reviews_by_target.read().await;
        let Some(paths) = map.get(&key) else { return Vec::new() };
        // BTreeSet iteration order is lexical-by-path; no explicit sort.
        paths.iter().filter_map(|p| entries.get(p).cloned()).collect()
    }
}

/// Validate and normalise a `target:` frontmatter value. Returns
/// `None` for any value that:
///   - is empty;
///   - contains `..`, `.`, NUL, or backslash in any segment;
///   - starts with `/` (absolute paths bypass `project_root`);
///   - resolves outside `project_root` after lexical join.
fn normalize_target_key(raw: &str, project_root: &Path) -> Option<PathBuf> {
    if raw.is_empty() || raw.starts_with('/') { return None; }
    for segment in raw.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('\\')
            || segment.contains('\0')
        {
            return None;
        }
    }
    let joined = project_root.join(raw);
    let normalized = normalize_absolute(&joined);
    // Defence in depth: even though per-segment validation rejects
    // `..`, lexical normalisation could collapse to a path outside
    // project_root via unusual inputs. Verify the normalised form
    // still has `project_root` as a prefix.
    if !normalized.starts_with(project_root) { return None; }
    Some(normalized)
}

/// Lexically clean an absolute path: collapse `.` and `..` segments
/// without touching the filesystem. Algorithm:
///   - walk path components left to right;
///   - skip `Component::CurDir` (`.`);
///   - on `Component::ParentDir` (`..`) pop the last `Normal`
///     component if any, else discard (do not escape root);
///   - keep `Component::RootDir` and `Component::Prefix` as-is;
///   - rejoin via `PathBuf::push`.
/// Does NOT perform Unicode normalisation, case folding, trailing-
/// slash stripping, or symlink resolution. Read/write parity in the
/// reverse-index keying relies on `project_root` being canonical
/// (set once at `Indexer::build`) so both sides land at the same form.
fn normalize_absolute(path: &Path) -> PathBuf { /* ... */ }

fn target_path_from_entry(entry: &IndexEntry, project_root: &Path) -> Option<PathBuf> {
    // Phase 9 scope: `target:` is populated only on plan-reviews.
    // PR-reviews' `target:` activation is a follow-up; admitting
    // them here without a fixture would loosen the contract beyond
    // what the test set exercises.
    if entry.r#type != DocTypeKey::PlanReviews { return None; }
    let raw = entry.frontmatter.get("target")?.as_str()?;
    normalize_target_key(raw, project_root)
}
```

The `normalize_absolute` doc-comment makes the algorithm explicit
(no hidden Unicode normalisation, no filesystem touch); a dedicated
unit test (Step 1.0) locks the contract before any consumer relies
on it. Consider using the well-tested `path-clean` crate if its
behaviour matches the spec — choose the approach that produces the
shorter implementation given the Cargo workspace.

#### 2. `src/indexer.rs` — explicit update_*/remove_* helper signatures

The three secondary-index helpers share a canonical signature so the
shape is reviewable at a glance and a future fourth helper has a
clear template. All are `pub(super)` and co-located in `indexer.rs`:

```rust
/// Each helper:
///   - is async only because the per-index `RwLock::write().await`
///     acquisition is async (the body is otherwise straight map
///     mutation);
///   - takes (storage, [context], new_entry, previous) in that order;
///   - returns `()` (no value);
///   - acquires its per-index lock *while* the caller holds
///     `entries.write()` — see lock-ordering invariant below.

pub(super) async fn update_adr_by_id(
    map: &Arc<RwLock<HashMap<u32, PathBuf>>>,
    new_entry: &IndexEntry,
    previous: Option<&IndexEntry>,
) { /* derive new key; if prev key differs, remove it; insert new. */ }

pub(super) async fn update_ticket_by_number(
    map: &Arc<RwLock<HashMap<u32, PathBuf>>>,
    new_entry: &IndexEntry,
    previous: Option<&IndexEntry>,
) { /* same shape. */ }

pub(super) async fn update_reviews_by_target(
    map: &Arc<RwLock<HashMap<PathBuf, BTreeSet<PathBuf>>>>,
    project_root: &Path,
    new_entry: &IndexEntry,
    previous: Option<&IndexEntry>,
) {
    // The remove-then-insert sequence is unconditionally executed,
    // even when `prev_target == next_target`. An earlier draft
    // short-circuited on equal targets, but that masked a path-
    // change leak: if the *review's own* file is renamed (path
    // changes) while its `target:` is unchanged, the old path would
    // remain in the BTreeSet under the (still-current) target key
    // and the new path would never be inserted. Always removing
    // `previous.path` and inserting `new_entry.path` keeps the
    // contract correct even on rename. BTreeSet operations are
    // O(log n) so the redundant work is negligible at v1 scale.
    let prev_target = previous.and_then(|p| target_path_from_entry(p, project_root));
    let next_target = target_path_from_entry(new_entry, project_root);
    let mut m = map.write().await;
    if let (Some(t), Some(prev)) = (&prev_target, previous) {
        if let Some(set) = m.get_mut(t) {
            set.remove(&prev.path);
            if set.is_empty() { m.remove(t); }
        }
    }
    if let Some(t) = next_target {
        m.entry(t).or_default().insert(new_entry.path.clone());
    }
}
```

Mirror `remove_from_*` helpers exist for the deletion path; they take
the previous entry (the one being removed) and clean its keys.

#### 3. `src/indexer.rs` — `refresh_one` previous-entry read + locking discipline

For the `#[cfg(test)] test_post_secondary_update` hook used by the
concurrency tests below, declare the field on `Indexer` with this
exact shape:

```rust
#[cfg(test)]
pub(crate) struct PostSecondaryUpdateHook {
    /// Writer → test: signalled inside the critical section, after
    /// the three secondary indexes have been updated and before the
    /// `entries.write()` guard drops.
    pub reached: tokio::sync::oneshot::Sender<()>,
    /// Test → writer: awaited by the writer immediately after sending
    /// `reached`. The test holds this Sender and signals when it is
    /// safe for the writer to drop the lock.
    pub proceed: tokio::sync::oneshot::Receiver<()>,
}

pub struct Indexer {
    // ... existing fields ...
    #[cfg(test)]
    test_post_secondary_update:
        tokio::sync::Mutex<Option<PostSecondaryUpdateHook>>,
}

#[cfg(test)]
impl Indexer {
    /// Install a one-shot test rendezvous. The next call to
    /// `refresh_one`'s Ok branch will signal `reached` and await
    /// `proceed` before releasing `entries.write()`. The hook is
    /// `take`n on use, so each test installs its own pair.
    pub(crate) async fn install_post_secondary_update_hook(
        &self,
        hook: PostSecondaryUpdateHook,
    ) {
        *self.test_post_secondary_update.lock().await = Some(hook);
    }
}
```

The reverse-index update needs the entry's *previous* form
(specifically its old `target:`) before the new form is written, so
target-migration drops the stale key. In `refresh_one`'s Ok branch:

```rust
// Hold a single write lock on `entries` for the whole update so
// readers see a consistent (entries, secondary-indexes) snapshot.
// This is the Phase 9 write-discipline invariant.
let mut entries = self.entries.write().await;
let previous = entries.get(&canonical).cloned();

let new_entry = build_entry(...)?;

// Each secondary index updates *while holding `entries.write()`*,
// so cross-index reads never observe partial state.
update_adr_by_id(&self.adr_by_id, &new_entry, previous.as_ref()).await;
update_ticket_by_number(&self.ticket_by_number, &new_entry, previous.as_ref()).await;
update_reviews_by_target(
    &self.reviews_by_target,
    &self.project_root,
    &new_entry,
    previous.as_ref(),
).await;

entries.insert(canonical.clone(), new_entry);
// `entries` lock drops here; readers resuming see the new
// consistent state.

// TEST HOOK: a `#[cfg(test)]`-gated rendezvous lets concurrency tests
// (Steps 1.7b, 1.13) observe state at this exact point — *after* the
// secondary indexes are updated but *before* the `entries.write()`
// guard drops. The hook uses two `tokio::sync::oneshot` channels
// (not `Notify::notify_waiters`, which is lost-wakeup-prone): the
// writer signals "I have reached the barrier" via `reached`, then
// blocks awaiting "you may proceed" via `proceed`. Both are pre-armed
// by the test before the writer task is spawned, so there is no
// ordering race. In production builds both fields compile out.
#[cfg(test)]
if let Some(hook) = self.test_post_secondary_update.lock().await.take() {
    let _ = hook.reached.send(());
    let _ = hook.proceed.await;
}
```

For the deletion (`NotFound`) branch, the existing
`find_entry_for_deleted` is restructured to take a held write guard
rather than acquiring its own read lock. This eliminates the
read-then-write TOCTOU window and keeps the deletion path under the
same single-writer-lock invariant:

```rust
// Acquire the write guard FIRST; then perform the lookup against it.
let mut entries = self.entries.write().await;
let Some(previous) = find_entry_for_deleted(&entries, path) else { return; };

remove_from_adr_by_id(&self.adr_by_id, &previous).await;
remove_from_ticket_by_number(&self.ticket_by_number, &previous).await;
remove_from_reviews_by_target(
    &self.reviews_by_target,
    &self.project_root,
    &previous,
).await;

entries.remove(&previous.path);
```

`find_entry_for_deleted` becomes a pure function over `&HashMap<PathBuf, IndexEntry>`
(no locking inside). Its existing logic — match on canonicalised
parent + filename — is unchanged.

**Lock-ordering invariant** (load-bearing for atomicity, documented
on `Indexer`): every code path that takes both `entries` and a
secondary-index lock acquires them in the order `entries → secondary`.
Writers hold `entries.write()` then take `secondary.write()` inside
each helper. Readers (`reviews_by_target`) hold `entries.read()` then
take `secondary.read()`. Inverting the order would expose
writer-starvation under tokio's writer-preferring `RwLock` semantics.
This invariant is the contract Step 1.7b verifies via the
`test_post_secondary_update` hook above.

#### 4. `src/clusters.rs` — no changes

Cluster computation is unchanged. The related endpoint reads cluster
data from `AppState.clusters` for inferred relations; declared
relations come from the new index.

### TDD Sequence (Phase 1)

Tests live inline in `src/indexer.rs` under the existing
`#[cfg(test)] mod tests` (slug/index seeded data) and
`mod refresh_tests` (refresh-one coverage), to mirror the
existing module structure.

1. **Step 1.0** — `normalize_absolute_collapses_dot_and_dotdot_lexically`:
   parametrised over `/a/./b → /a/b`, `/a/b/../c → /a/c`,
   `/a/../../b → /b` (does not escape root), `/a//b → /a/b`,
   `/a/b/. → /a/b`. Touches no filesystem (use a path that does not
   exist on disk). Locks the contract `normalize_target_key` depends on.
2. **Step 1.1** — `reviews_by_target_populated_on_initial_scan`:
   seed a plan + a review whose `target:` points at it; build
   indexer; assert `idx.reviews_by_target(&plan_entry.path).await`
   (note: query uses the canonical entry path obtained via
   `Indexer::get`, the same form Phase 2's handler uses) returns
   exactly the review entry.
3. **Step 1.2** — `reviews_by_target_round_trips_via_canonical_root`:
   construct the indexer with a `project_root` reachable via a symlink
   (e.g., macOS `/var/folders/.../proj` → `/private/var/folders/.../proj`),
   then seed a plan + a review whose `target:` is a repo-relative
   string. Query the reverse index using `Indexer::get(&plan_relpath)`'s
   canonical `entry.path`; assert the lookup succeeds. The test seeds
   the *symlinked* root and queries the *canonical* path — exactly
   the production code path the handler exercises. Locks the
   write/read parity invariant the canonical-`project_root`-once
   discipline provides.
4. **Step 1.3** — `reviews_by_target_excludes_reviews_without_target_field`:
   seed a review with no `target:` frontmatter; assert no key for it
   exists in the map (no panic, no empty-string key).
5. **Step 1.4** — `reviews_by_target_tolerates_target_pointing_at_missing_file`:
   seed a review whose `target:` points at a non-existent plan; build
   indexer; assert no panic, and that calling `reviews_by_target` with
   the canonical form of the missing path (`project_root.join(raw)`,
   passed through `normalize_absolute`) returns the review entry. The
   back-reference materialises the moment the plan is created because
   the secondary key uses the same canonical-root prefix the new file
   will land under.
6. **Step 1.5** — `reviews_by_target_supports_multiple_reviews_per_target`:
   two reviews both pointing at one plan; lookup returns both, in
   path-sorted order (BTreeSet contract — no explicit sort needed).
7. **Step 1.5b** — `refresh_one_on_unchanged_review_keeps_set_size_one`:
   refresh the same review file twice without changing its content;
   assert the inbound set for its target has exactly one entry, not
   two. Locks the dedup-by-construction property of `BTreeSet`.
8. **Step 1.5c** — `refresh_one_on_renamed_review_with_unchanged_target`:
   review at path A targets plan P; rename the review file (move it
   to path B) without changing its `target:`; `refresh_one(B)` then
   `refresh_one(A)` (the watcher's typical add+delete sequence);
   assert plan P's inbound set contains B, not A. Locks the
   "no early-return on equal targets" contract that prevents the
   rename leak — without the unconditional remove-then-insert in
   `update_reviews_by_target`, A would remain stuck in the set.
8. **Step 1.6** — `refresh_one_adds_review_to_reverse_index`: build
   indexer with no review; write a new review file referencing an
   existing plan; `refresh_one(&new_review)`; assert
   `reviews_by_target(&plan_entry.path)` includes the new review.
9. **Step 1.7** — `refresh_one_removes_review_from_reverse_index_on_target_change`:
   review initially targets plan A; rewrite the review's `target:` to
   plan B; `refresh_one(&review)`; assert plan A's reverse list no
   longer contains the review and plan B's does. The migration *must*
   drop the stale key under plan A — a leftover entry would surface
   as a phantom "Inbound review" in the UI. Implementation requires
   reading the previous entry (Phase 1 §2) before deriving the old
   target.
10. **Step 1.7b** — `refresh_one_target_migration_is_atomic_under_single_writer_lock`:
    deterministic test using the `#[cfg(test)] test_post_secondary_update`
    rendezvous on `Indexer` (Phase 1 §3). Test setup: create two
    `tokio::sync::oneshot` channels (`reached_tx/rx`, `proceed_tx/rx`);
    `install_post_secondary_update_hook(PostSecondaryUpdateHook { reached: reached_tx, proceed: proceed_rx })`.
    Spawn a *secondary-read* task that, on receiving a separate
    `tokio::sync::oneshot` start signal from the test thread, calls
    `reviews_by_target(plan_a_canonical)` then
    `reviews_by_target(plan_b_canonical)` and reports the pair to a
    result channel. Spawn the writer task: `refresh_one` to migrate
    the review's target from A→B. The writer reaches the hook,
    sends `reached`, then awaits `proceed` *while still holding
    `entries.write()`*. The test thread awaits `reached_rx` (cannot
    be lost — `oneshot::Sender::send` always succeeds with a fresh
    receiver). On receipt, the test thread signals the secondary-read
    task to begin: both `reviews_by_target` calls block on
    `entries.read()` until the writer drops its guard. The test then
    sends `proceed_tx.send(())`; the writer's hook returns; the writer
    drops `entries.write()`; the secondary-read task unblocks. Assert
    the reported pair is the *post-migration* state (review absent in
    A, present in B). The reader cannot observe a partial state
    because it cannot acquire `entries.read()` until the writer's
    critical section completes — that *is* the locked contract.
    Crucially, no `notify_waiters()`-style lost-wakeup hazard: both
    halves of the rendezvous use buffered `oneshot` channels with
    pre-armed receivers.
11. **Step 1.8** — `refresh_one_removes_review_from_reverse_index_on_deletion`:
    review references plan A; delete the review file;
    `refresh_one(&review)`; assert plan A's reverse list is empty.
12. **Step 1.8b** — `delete_target_plan_with_inbound_reviews`: plan A
    has two inbound reviews; delete plan A; `refresh_one(&plan_a)`;
    assert (a) `reviews_by_target` still returns the two reviews when
    queried with the (now-non-existent) canonical path of plan A
    (deferred materialisation: lexical key survives target-file
    deletion), (b) the reviews' own entries are unchanged.
    Documents the chosen contract for cross-doc deletion explicitly.
13. **Step 1.9** — `reviews_by_target_survives_full_rescan`: build,
    add a review referencing a plan, force `idx.rescan()`; assert the
    reverse map still contains the back-reference (rescan clears all
    three secondary maps in its initial step, then re-populates as it
    walks the file tree).
14. **Step 1.10** — `reviews_by_target_only_admits_plan_reviews`:
    a non-review doc with a synthetic `target:` frontmatter field is
    *not* admitted; a `PrReviews` entry with `target:` is also *not*
    admitted (Phase 9 scope is plan-reviews only — broadening is a
    follow-up). Test guards against accidental scope creep.
15. **Step 1.11** — `target_path_from_entry_rejects_malformed_values`:
    parametrised over `target: ""`, `target: "../escape.md"`,
    `target: "/etc/passwd"`, `target: "foo\\bar"`, `target: "foo/../escape.md"`,
    plus non-string YAML values (`target: 42`, `target: null`,
    `target: ["a"]`). Each case asserts the entry is admitted to the
    primary index (the malformed `target` does not block indexing of
    the review document itself) but not to the reverse map. Locks the
    sanitisation contract and prevents path-escape via frontmatter.
16. **Step 1.12** — `rescan_clears_all_three_secondary_maps_before_repopulating`:
    populate `adr_by_id`, `ticket_by_number`, and `reviews_by_target`
    with stale data not present on disk; call `rescan()`; assert all
    three are rebuilt from disk only (stale entries gone). Guards the
    rescan rebuild path against a forgotten clear when a fourth map
    is added.
17. **Step 1.13** — `concurrent_rescan_and_target_migration_under_rescan_lock`:
    deterministic variant of the existing
    `refresh_one_serialises_with_concurrent_rescan` pattern. Setup:
    install the `test_post_secondary_update` rendezvous (same shape
    Step 1.7b uses — two `oneshot` channels). Drive (a) a `refresh_one`
    migrating the review A→B, and (b) an `idx.rescan()`. Both
    serialise on the existing `rescan_lock` semaphore (1 permit), so
    they cannot run concurrently. Use the rendezvous to verify the
    ordering invariant: when `refresh_one` reaches the
    post-secondary-update barrier and sends `reached`, the test
    thread immediately calls `rescan_lock.try_acquire()` and asserts
    the permit is *not* available (the writer still holds it).
    Signal `proceed`; both tasks complete; the final state has the
    review under exactly one target key (B, never both, never
    neither). The test verifies serialisation by lock state rather
    than by timing — no sampling, no flake risk, no lost-wakeup
    hazard.

### Success Criteria

#### Automated Verification

- [x] `cargo test -p accelerator-visualiser indexer::tests` passes.
- [x] `cargo test -p accelerator-visualiser indexer::refresh_tests`
  passes (existing + new coverage).
- [x] `cargo clippy -p accelerator-visualiser` is clean.
- [x] `cargo fmt --check` is clean.

---

## Phase 2: `GET /api/related/*path` endpoint

### Overview

Add one new GET route `/api/related/*path`. Handler resolves the doc,
returns a JSON body containing inferred-cluster siblings (excluding
self) and declared cross-references in both directions.

Wire format (TypeScript):

```ts
interface RelatedArtifactsResponse {
  inferredCluster: IndexEntry[]   // same-slug docs, self excluded
  declaredOutbound: IndexEntry[]  // resolved targets of self.frontmatter.target
  declaredInbound: IndexEntry[]   // reviews whose target resolves to self
}
```

The handler is read-only and trivially serialisable — no new types
beyond the response wrapper.

### Changes Required

#### 1. `src/api/related.rs` (new module)

```rust
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelatedArtifactsResponse {
    pub inferred_cluster: Vec<IndexEntry>,
    pub declared_outbound: Vec<IndexEntry>,
    pub declared_inbound: Vec<IndexEntry>,
}

pub(crate) async fn related_get(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Result<Response, ApiError> { ... }
```

The `#[serde(rename_all = "camelCase")]` attribute is mandatory and
matches the convention used by `LifecycleCluster`, `IndexEntry`, etc.
elsewhere in the server. Step 2.10 locks the behavioural contract.

Handler steps:
1. Per-segment path validation (mirrors `doc_patch_frontmatter`'s
   precise rejection — `..`, `.`, empty, backslash, NUL, leading `/`
   all rejected). Validation runs on the **decoded** form of the
   captured path so `%2F`, `%00`, etc. cannot smuggle in segments.
2. Build absolute path against `project_root`; index lookup by
   `Indexer::get(abs)`; 404 on miss.
3. Compute `inferred_cluster`:
   - Read `state.clusters.read().await`; find the cluster whose
     `slug == entry.slug` (when slug is `Some`).
   - Filter out the self entry by canonical path.
   - When entry has no slug, return empty array (no clustering).
4. Compute `declared_outbound`:
   - Delegate to `Indexer::declared_outbound(&entry).await` (new in
     Phase 1 §2 alongside `ReviewsByTarget`). The indexer owns the
     "which doc-types carry which declared-link fields" knowledge;
     the handler stays a thin orchestration layer. For Phase 9 the
     result is at most one entry (the resolved `target:` plan) for
     plan-reviews, empty for everything else.
5. Compute `declared_inbound`:
   - `state.indexer.reviews_by_target(&entry.path).await`. The
     accessor lexically normalises both sides (Phase 1), so the
     entry's canonical `path` and the secondary index's lexical key
     compare correctly.
6. **Dedup overlap**: when an entry appears in both `inferred_cluster`
   and either declared list, drop it from `inferred_cluster`. The
   declared relation is the more specific signal and the UI groups
   them separately; showing the same doc twice would be visually
   noisy and obscure the declared-vs-inferred distinction.
7. Return `200 OK` with the JSON body.

#### 2. `src/api/mod.rs`

Add the route:

```rust
.route("/api/related/*path", get(related::related_get))
```

Routing is registered *before* the catch-all `/api/*rest` not-found
fallback in `build_router_with_spa` (already the case — `mount`
happens before `api_not_found` is attached).

### TDD Sequence (Phase 2)

Tests live in a new integration test file
`server/tests/api_related.rs`, mirroring `api_lifecycle.rs`. Inline
unit tests for individual computation helpers go in `src/api/related.rs`
under `#[cfg(test)] mod tests`.

1. **Step 2.1** — `related_endpoint_returns_404_for_unknown_path`
   (integration): GET `/api/related/meta/plans/does-not-exist.md` →
   404 with JSON `{"error": "..."}` body.
2. **Step 2.2** — `related_endpoint_returns_403_for_path_escape`
   (integration): GET `/api/related/../../etc/passwd` → 403.
3. **Step 2.3** — `related_endpoint_for_plan_with_no_relations`
   (integration): GET `/api/related/meta/plans/<isolated>.md` for a
   plan with no slug-siblings and no inbound reviews → 200 with all
   three arrays empty.
4. **Step 2.4** — `related_endpoint_includes_slug_cluster_siblings`
   (integration): plan + ticket with the same slug; GET related on
   the plan; `inferredCluster[0]` is the ticket; self-exclusion
   verified.
5. **Step 2.5** — `related_endpoint_excludes_self_from_inferred`
   (integration): only the *other* same-slug entries appear; the
   self entry is never echoed back.
6. **Step 2.6** — `related_endpoint_returns_declared_outbound_for_review`
   (integration): a plan-review whose `target:` points at a plan; GET
   related on the review; `declaredOutbound[0]` is the plan;
   `declaredInbound` is empty.
7. **Step 2.7** — `related_endpoint_returns_declared_inbound_for_target_plan`
   (integration): same fixture, GET related on the plan; the plan
   has the review in `declaredInbound`; `declaredOutbound` is empty.
8. **Step 2.8** —
   `related_endpoint_returns_empty_outbound_when_target_missing`
   (integration): review's `target:` points at a non-existent file;
   GET related on the review; `declaredOutbound` is `[]` (not 404,
   not error).
9. **Step 2.9** — `related_endpoint_returns_empty_arrays_for_template_doc`
   (integration): templates have no slug and never appear in
   clusters; GET related on a template path is 404 (templates are
   not addressable by repo-relative path through this endpoint —
   they live behind `/api/templates/:name`). The 404 is the
   side-effect of `Indexer::get` not finding the template; no
   special-case code.
10. **Step 2.10** — `related_response_uses_camelcase_field_names`
    (integration): asserts the JSON object has exactly the keys
    `inferredCluster`, `declaredOutbound`, `declaredInbound` — the
    serde rename matches the wire-format contract the frontend
    types depend on.
11. **Step 2.11** —
    `related_endpoint_recovers_after_target_creation`
    (integration): write a review whose target plan does not yet
    exist; observe `declaredOutbound` is empty; create the target
    plan; (after a watcher debounce or explicit rescan) GET related
    on the review; `declaredOutbound` now contains the plan. Locks
    in Step 1.4's deferred materialisation at the HTTP boundary.
12. **Step 2.12** — `related_endpoint_dedupes_overlap_in_favor_of_declared`
    (integration): a plan-review whose target plan happens to share
    the review's slug. GET related on the review; assert the target
    plan appears in `declaredOutbound` and *not* in `inferredCluster`
    (handler step 6). Locks the dedup contract.
13. **Step 2.13** —
    `related_endpoint_returns_empty_outbound_after_target_deletion`
    (integration): review references plan A; observe `declaredOutbound`
    contains plan A; delete plan A; GET related on the review; assert
    `declaredOutbound` is `[]`. Pairs with Step 1.8b on the indexer
    side — the inbound list still surfaces via the lexical-key
    contract, but the outbound resolves through `Indexer::get` which
    returns `None` for the deleted plan.
14. **Step 2.14** — `related_endpoint_validates_decoded_path_segments`
    (integration): GET `/api/related/foo%2F..%2Fbar` (URL-encoded
    `..`) returns 403, asserting the per-segment validator runs on
    the decoded form rather than the raw capture. Pairs with the
    client's `encodeURIComponent` policy in Phase 5 §2.

### Success Criteria

#### Automated Verification

- [x] `cargo test -p accelerator-visualiser --test api_related` passes
  all fourteen cases.
- [x] `cargo test -p accelerator-visualiser` is fully green.
- [x] `cargo clippy -p accelerator-visualiser` is clean.
- [ ] Manual `curl` against a running dev server (with the seeded
  fixture) returns the expected JSON for both directions.

#### Manual Verification

- [ ] Inspect the JSON response for one plan-review and one target
  plan; confirm structure matches `RelatedArtifactsResponse`.

---

## Phase 3: Frontend wiki-link resolver (pure)

### Overview

A pure module that, given an ADR index map and a ticket index map,
resolves a wiki-link target to a `{ href, title }` pair (or `null`
when unresolved). No React, no fetching — this is the deterministic
core that the renderer plugin calls.

The resolver returns a `title` alongside the `href` so the rendered
anchor can display a meaningful label (the entry's title) rather than
the raw `[[ADR-NNNN]]` source form. Phase 4's plugin uses `title` as
the link's display text and threads the bracket-form into the
anchor's `title` attribute as a hover/source-form fallback.

### Changes Required

#### 1. `src/api/wiki-links.ts` (new file)

```ts
import type { IndexEntry } from './types'
import { fileSlugFromRelPath } from './path-utils'

export interface WikiLinkIndex {
  adrById: Map<number, IndexEntry>
  ticketByNumber: Map<number, IndexEntry>
}

export interface ResolvedWikiLink {
  href: string
  title: string
}

/** Match `[[ADR-NNNN]]` and `[[TICKET-NNNN]]` only. Bare `[[NNNN]]`
 *  is intentionally excluded so the prefix namespace stays free for
 *  future ID kinds (`[[EPIC-NNNN]]`, etc.). The digit count is
 *  bounded to 1..6 to comfortably exceed any realistic ID space
 *  while preventing pathological inputs from producing values that
 *  overflow `Number.MAX_SAFE_INTEGER`. */
export const WIKI_LINK_PATTERN = /\[\[(ADR|TICKET)-(\d{1,6})\]\]/g

/** Build the resolver maps from an unrelated docs cache. ADRs are
 *  keyed by `frontmatter.adr_id` (e.g. "ADR-0001") *or* by the
 *  numeric prefix of the filename. Tickets are keyed by the
 *  numeric prefix of the filename. Defensively filters by
 *  `entry.type` so a misuse (e.g. accidentally passing plans as
 *  tickets) cannot route `[[TICKET-N]]` to a non-ticket. On
 *  duplicate keys (two ADRs claim the same id, real authoring
 *  mistake during renames) the entry with the lexically-earliest
 *  `relPath` wins — deterministic across reloads. */
export function buildWikiLinkIndex(
  adrEntries: IndexEntry[],
  ticketEntries: IndexEntry[],
): WikiLinkIndex { ... }

/** Resolve one wiki-link target. Returns href + title or null. */
export function resolveWikiLink(
  prefix: 'ADR' | 'TICKET',
  n: number,
  idx: WikiLinkIndex,
): ResolvedWikiLink | null { ... }
```

Internally, both `buildWikiLinkIndex` and the test fixtures pass
`parseInt(prefix, 10)` with an explicit radix; all numeric
extraction in this module routes through a single helper to keep the
parsing contract uniform.

Resolved hrefs follow the existing `/library/:type/:fileSlug` shape;
`fileSlug` is derived via `fileSlugFromRelPath(entry.relPath)` so the
filename's date or numeric prefix is preserved exactly.

### TDD Sequence (Phase 3)

Tests in `src/api/wiki-links.test.ts`. No DOM, no React — pure
vitest assertions.

1. **Step 3.1** — `WIKI_LINK_PATTERN matches both ADR and TICKET
   forms`: scans `"see [[ADR-0017]] and [[TICKET-1]]"`, returns two
   matches with the correct prefix + number.
2. **Step 3.2** — `WIKI_LINK_PATTERN does not match bare numeric form`:
   `"[[0001]]"` produces zero matches.
3. **Step 3.3** — `WIKI_LINK_PATTERN does not match unknown prefix`:
   `"[[EPIC-0001]]"` produces zero matches (forward-compat is opt-in).
4. **Step 3.4** — `WIKI_LINK_PATTERN does not match
   uppercase-mismatched form`: `"[[adr-0001]]"` and `"[[Adr-0001]]"`
   both produce zero matches; the prefix is case-sensitive.
5. **Step 3.4b** — `WIKI_LINK_PATTERN rejects digit-runs longer than
   six`: `"[[ADR-9999999]]"` (seven digits) and the pathological
   `"[[ADR-` + ten thousand `'9'`s + `]]"` both produce zero matches.
   Locks the bounded-quantifier contract.
6. **Step 3.4c** — `WIKI_LINK_PATTERN boundary cases`: parametrised
   over `"[[ADR-0001]]."` (trailing punctuation — match), `"prefix[[ADR-0001]]suffix"`
   (no surrounding whitespace — match), `"[[ADR-]]"` (empty digits —
   no match), `"[[ADR-0001a]]"` (trailing non-digit before `]]` — no
   match). Locks the boundary contract against future regex tweaks.
7. **Step 3.5** — `buildWikiLinkIndex indexes ADRs by adr_id when
   present`: an entry with `frontmatter.adr_id = "ADR-0017"` is
   indexed under `17`.
8. **Step 3.6** — `buildWikiLinkIndex falls back to filename prefix
   for ADRs missing adr_id`: entry with no `adr_id` but filename
   `ADR-0042-foo.md` is indexed under `42`.
9. **Step 3.7** — `buildWikiLinkIndex prefers adr_id over filename
   when both are present and disagree`: contrived case; the
   frontmatter wins, mirroring the server's `parse_adr_id`
   precedence.
10. **Step 3.7b** — `buildWikiLinkIndex picks earliest-relPath on
    duplicate IDs`: two ADR entries with the same `adr_id`; the entry
    with the lexically-earliest `relPath` wins regardless of input
    order. Locks deterministic tie-breaking — without this, reload
    order would silently change which page `[[ADR-NNNN]]` targets.
11. **Step 3.7c** — `buildWikiLinkIndex defensively filters by entry
    type`: pass a plan entry (filename `2026-04-18-foo.md`) into
    `ticketEntries`; assert it is *not* indexed under `2026`. Locks
    the kind-restriction contract that prevents `[[TICKET-2026]]`
    from mis-resolving when a caller passes the wrong list.
12. **Step 3.8** — `buildWikiLinkIndex indexes tickets by filename
    numeric prefix`: entry with filename `0001-foo.md` is indexed
    under `1` (leading zeros stripped via `parseInt(_, 10)`).
13. **Step 3.9** — `resolveWikiLink returns null for unknown ADR id`:
    index has no entry for ADR-9999 → null.
14. **Step 3.10** — `resolveWikiLink returns href and title for known
    ADR`: entry with relPath `meta/decisions/ADR-0017-foo.md` and
    `frontmatter.title = "Configuration extension points"` resolves
    to `{ href: '/library/decisions/ADR-0017-foo', title: 'Configuration extension points' }`.
15. **Step 3.11** — `resolveWikiLink returns href and title for known
    ticket`: entry with relPath `meta/tickets/0001-foo.md` resolves
    to `{ href: '/library/tickets/0001-foo', title: <ticket.title> }`.
16. **Step 3.12** — `resolveWikiLink returns null for unknown ticket`:
    null on miss.
17. **Step 3.13** — `resolveWikiLink ignores leading zeros in input
    number`: passing `n = 17` resolves to the same entry as `n = 0017`
    would have.

### Success Criteria

#### Automated Verification

- [x] `npm test -- wiki-links` passes all seventeen cases.
- [x] `npm run typecheck` is clean.

---

## Phase 4: MarkdownRenderer wiki-link integration

### Overview

Add an opt-in `resolveWikiLink` prop to `MarkdownRenderer`. When
provided, a custom remark plugin walks the AST and rewrites text-node
substrings matching the pattern. **Four outcomes** — three correspond
to resolver return values, plus the no-bracket-shape passthrough:

- **Resolved**: resolver returns `{ kind: 'resolved', href, title }`.
  Becomes a `link` mdast node whose display text is the entry's
  `title` (with the bracket-form on the anchor's `title` attribute
  as a hover/source-form fallback).
- **Unresolved (genuine)**: resolver returns `{ kind: 'unresolved' }`.
  Becomes a marker node rendered as
  `<span class="unresolved-wiki-link" title="No matching ADR/TICKET found for ID NNN">[[…]]</span>`,
  giving authors visual feedback that the syntax was recognised but
  the target couldn't be found (typo in ID, missing fixture, etc.).
- **Pending**: resolver returns `{ kind: 'pending' }`. Becomes a
  marker node rendered as
  `<span class="wiki-link-pending" title="Loading reference…">[[…]]</span>`,
  giving readers visible *loading* feedback distinct from the
  broken-reference treatment. The Phase 5 hook returns this kind
  while either underlying docs query is pending; on settle, the
  resolver's return type rotates and `MarkdownRenderer` re-renders,
  flipping the pending markers to anchors (or to genuine unresolved
  markers if a specific ID truly has no match).
- **No bracket-shape**: text is unchanged.

When the prop is absent the plugin doesn't run — pre-Phase-9 callers
see no behavioural change.

The plugin runs only on `text` nodes. mdast represents fenced and
inline code as `code`/`inlineCode` nodes whose interior text is
*not* `text`-typed children, so the visitor never enters them.

### Changes Required

#### 1. `src/components/MarkdownRenderer/wiki-link-plugin.ts` (new)

```ts
import type { Plugin } from 'unified'
import type { Root, Text, Link, Parents } from 'mdast'
import { visit, SKIP } from 'unist-util-visit'
import { WIKI_LINK_PATTERN } from '../../api/wiki-links'

/** Resolver return shape. The three kinds map to three distinct
 *  visual treatments downstream, each rendered by the plugin via a
 *  different mdast/hast shape. The hook in Phase 5 §5 picks `pending`
 *  while either docs query is in flight, `resolved` when an entry
 *  is found, and `unresolved` when both queries have settled and
 *  no entry matches. */
export type ResolverResult =
  | { kind: 'resolved'; href: string; title: string }
  | { kind: 'unresolved' }
  | { kind: 'pending' }

export type Resolver =
  (prefix: 'ADR' | 'TICKET', n: number) => ResolverResult

export const remarkWikiLinks: Plugin<[Resolver], Root> =
  (resolve) =>
  (tree) => {
    visit(tree, 'text', (node: Text, index, parent: Parents | undefined) => {
      if (!parent || index === undefined) return
      const out = splitTextNode(node, resolve)
      if (!out) return
      parent.children.splice(index, 1, ...out)
      // SKIP unconditionally past the inserted nodes. Inserted nodes
      // are Text (already exhausted), Link (Text child is entry title,
      // not bracket-form), or marker spans (child Text *is* bracket-
      // form and would re-match if visited). SKIP prevents double-
      // rewrite for the marker case and is a no-op for the others.
      return [SKIP, index + out.length]
    })
  }

/** Returns the replacement node sequence, or null when the input
 *  contains no bracket-shape matches at all (no allocation overhead
 *  for plain prose). */
function splitTextNode(node: Text, resolve: Resolver): Array<Text | Link | MarkerNode> | null {
  // For each WIKI_LINK_PATTERN match in node.value, switch on the
  // resolver's return kind:
  //   - resolved → push prefix Text (if non-empty), Link with
  //     children=[Text(result.title)], url=result.href, and
  //     `data.hProperties.title = '[[ADR-NNNN]]'`.
  //   - unresolved → push prefix Text, then a marker with
  //     className=`unresolved-wiki-link` and a diagnostic title
  //     (`No matching ADR found for ID NNN` / `No matching TICKET…`).
  //   - pending → push prefix Text, then a marker with
  //     className=`wiki-link-pending` and title=`Loading reference…`.
  // After the loop, push trailing Text. Return null if the loop
  // produced zero matches.
  ...
}

/** A pseudo-mdast node rendered as a span with a class modifier.
 *  Two flavours: `wiki-link-pending` (cache warming) and
 *  `unresolved-wiki-link` (settled-but-no-match). The `data.hName` +
 *  `data.hProperties` shape is the unified-recommended way to emit a
 *  custom HTML element without enabling raw-HTML parsing. */
type MarkerNode = {
  type: 'wikiLinkMarker'
  data: {
    hName: 'span'
    hProperties: {
      className: 'unresolved-wiki-link' | 'wiki-link-pending'
      title: string
    }
    hChildren: [{ type: 'text'; value: string }]
  }
}
```

Both marker variants render as spans; the CSS module styles them
distinctly:
- `.wiki-link-pending` — neutral skeleton/italic, e.g., dimmed text
  with no underline; reads as "loading", not "broken".
- `.unresolved-wiki-link` — muted colour with dotted underline;
  reads as "broken reference, check your ID".

The `title` attribute carries a diagnostic message keyed to the
marker variant, not the bracket-form (which is already visible as
the span's text content).

#### 2. `src/components/MarkdownRenderer/MarkdownRenderer.tsx`

```tsx
interface Props {
  content: string
  resolveWikiLink?: Resolver
}

export function MarkdownRenderer({ content, resolveWikiLink }: Props) {
  // Memoise the plugin tuple keyed on the resolver's identity. The
  // resolver from `useWikiLinkResolver` is itself memoised (stable
  // across renders that don't change docs-cache state), so this
  // tuple is stable too — react-markdown then short-circuits its
  // pipeline re-run for content-unchanged renders. When docs caches
  // settle and `useWikiLinkResolver` rotates the resolver reference,
  // the tuple identity changes and the pipeline re-runs, flipping
  // pending markers to anchors. The flip-on-settle behaviour is
  // structural (driven by resolver identity), not incidental
  // (driven by inline-array reference change every render).
  const remarkPlugins = useMemo(
    () => resolveWikiLink
      ? [remarkGfm, [remarkWikiLinks, resolveWikiLink] as const]
      : [remarkGfm],
    [resolveWikiLink],
  )
  return (
    <div className={styles.markdown}>
      <ReactMarkdown remarkPlugins={remarkPlugins} rehypePlugins={[rehypeHighlight]}>
        {content}
      </ReactMarkdown>
    </div>
  )
}
```

The component override for anchors is unchanged — react-markdown's
default `urlTransform` strips dangerous schemes and applies to
plugin-emitted Link nodes as well as parsed-from-source ones. Step
4.10 locks this in for the plugin path.

### TDD Sequence (Phase 4)

Tests in `src/components/MarkdownRenderer/MarkdownRenderer.test.tsx`
(extending the existing file) and `wiki-link-plugin.test.ts`
(unit-level AST tests).

1. **Step 4.1** —
   `wiki-link-plugin.test.ts: leaves text without wiki-links unchanged`:
   pass mdast tree through the plugin with a pass-through resolver;
   tree is structurally identical (deep equal).
2. **Step 4.2** — `wiki-link-plugin.test.ts: rewrites resolved ref
   to a link node with title as display text`: text node
   `"see [[ADR-0001]]"` with resolver returning
   `{ kind: 'resolved', href: '/library/decisions/ADR-0001-foo', title: 'Example decision' }`
   becomes `[Text("see "), Link(url=…, children=[Text("Example decision")])]`.
   The Link's `data.hProperties.title` is `"[[ADR-0001]]"` so the
   rendered anchor exposes the source form on hover.
3. **Step 4.3** — `wiki-link-plugin.test.ts: emits unresolved marker
   when resolver returns kind=unresolved`: text node `"see [[ADR-9999]]"`
   with resolver returning `{ kind: 'unresolved' }` becomes
   `[Text("see "), MarkerNode(value="[[ADR-9999]]")]`. The marker's
   `data.hName` is `'span'`, className is `'unresolved-wiki-link'`,
   and `title` is the diagnostic `"No matching ADR found for ID 9999"`.
4. **Step 4.3b** — `wiki-link-plugin.test.ts: emits pending marker
   when resolver returns kind=pending`: text node `"see [[ADR-0001]]"`
   with resolver returning `{ kind: 'pending' }` becomes
   `[Text("see "), MarkerNode(value="[[ADR-0001]]")]`. The marker's
   className is `'wiki-link-pending'` and `title` is `"Loading reference…"`.
   Locks the warming-vs-broken visual distinction.
6. **Step 4.4** — `wiki-link-plugin.test.ts: handles multiple matches
   in one text node`: `"[[ADR-0001]] and [[TICKET-1]]"` → three nodes
   when both resolve (Link, Text, Link); or interleaved Link/marker
   variants when one or both don't resolve.
7. **Step 4.5** — `wiki-link-plugin.test.ts: does not visit inline
   code`: `"plain [[ADR-0001]] and `[[ADR-0002]]`"` — the second is
   inside backticks; only the first becomes a Link.
8. **Step 4.5b** — `wiki-link-plugin.test.ts: does not visit fenced
   code blocks`: markdown body with a triple-backtick block containing
   `[[ADR-0001]]`; assert no Link is emitted inside the block.
9. **Step 4.5c** — `wiki-link-plugin.test.ts: SKIP prevents double
   rewrite of inserted children`: feed a tree containing
   `[[ADR-0001]]` whose resolved title is `"[[ADR-0002]]"` (contrived
   collision); assert exactly one Link is emitted, not two. Locks the
   `[SKIP, index + out.length]` invariant.
10. **Step 4.6** —
    `MarkdownRenderer.test.tsx: renders wiki-link as anchor when
    resolver returns kind=resolved`: render `[[ADR-0001]]` with a stub
    resolver returning
    `{ kind: 'resolved', href: '/library/decisions/ADR-0001-foo', title: 'Example decision' }`;
    assert `screen.getByRole('link', { name: 'Example decision' }).href`
    ends with `/library/decisions/ADR-0001-foo`, and the anchor's
    `title` attribute is `"[[ADR-0001]]"`.
11. **Step 4.7** —
    `MarkdownRenderer.test.tsx: renders unresolved-wiki-link span when
    resolver returns kind=unresolved`: stub resolver returns
    `{ kind: 'unresolved' }`; assert the rendered output contains
    `<span class="unresolved-wiki-link" title="No matching ADR found for ID 9999">[[ADR-9999]]</span>`
    and no `<a>` element. The diagnostic title differentiates the
    unresolved-marker from the pending-marker.
12. **Step 4.7b** —
    `MarkdownRenderer.test.tsx: renders wiki-link-pending span when
    resolver returns kind=pending`: stub resolver returns
    `{ kind: 'pending' }`; assert the rendered output contains
    `<span class="wiki-link-pending" title="Loading reference…">[[ADR-0001]]</span>`
    and no `<a>` element. The pending span's distinct className is
    the visual signal that the cache is warming, not that the
    reference is broken.
13. **Step 4.8** —
    `MarkdownRenderer.test.tsx: omits the plugin when resolveWikiLink
    is not provided`: render `[[ADR-0001]]` without the prop; output
    contains the verbatim text and neither anchor nor marker span
    (back-compat — pre-Phase-9 callers see no behavioural change).
14. **Step 4.8b** —
    `MarkdownRenderer.test.tsx: memoised plugin tuple is stable when
    resolver identity is stable`: render twice with the same resolver
    reference; assert the `useMemo`'d `remarkPlugins` returns the
    same array reference both renders. Pair: render with two
    different resolver references; assert the array reference
    changes. Locks the resolver-identity-drives-pipeline-rerun
    contract that flip-on-settle depends on (Phase 6 §c).
15. **Step 4.9** —
    `MarkdownRenderer.test.tsx: existing XSS regression guard still
    passes with plugin enabled`: re-runs the existing
    `<script>alert('xss')</script>` and the `[click](javascript:alert(1))`
    tests with a resolver attached, proving the plugin doesn't widen
    the parsing surface.
16. **Step 4.10** —
    `MarkdownRenderer.test.tsx: resolver-supplied dangerous URL is
    sanitised by urlTransform`: stub resolver returns
    `{ kind: 'resolved', href: 'javascript:alert(1)', title: 'evil' }`;
    assert the rendered anchor either has no `href` or has a
    sanitised one (per react-markdown's default `urlTransform`).
    Locks the trust-boundary contract for plugin-emitted Link nodes.

### Success Criteria

#### Automated Verification

- [x] `npm test -- MarkdownRenderer` passes (existing + new cases).
- [x] `npm test -- wiki-link-plugin` passes.
- [x] `npm run typecheck` is clean.
- [x] `npm run lint` is clean.

---

## Phase 5: Frontend related-artifacts fetch + types + hooks

### Overview

Wire the new `/api/related/*path` endpoint into the TanStack Query
layer, and introduce two custom hooks that compose the
LibraryDocView's data fetches: `useRelated` (the related-artifacts
query), `useWikiLinkResolver` (combines the ADR + ticket caches into
a memoised resolver), and `useDocPageData` (composes
`useDocContent` + `useRelated` for the doc view's read path). Each
hook is independently testable and keeps `LibraryDocView` a thin
composition layer rather than a four-`useQuery` join point.

### Changes Required

#### 1. `src/api/types.ts`

Add:

```ts
export interface RelatedArtifactsResponse {
  inferredCluster: IndexEntry[]
  declaredOutbound: IndexEntry[]
  declaredInbound: IndexEntry[]
}
```

#### 2. `src/api/fetch.ts`

```ts
export async function fetchRelated(relPath: string): Promise<RelatedArtifactsResponse> {
  const encoded = relPath.split('/').map(encodeURIComponent).join('/')
  const r = await fetch(`/api/related/${encoded}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/related/${relPath}: ${r.status}`)
  return r.json()
}
```

#### 3. `src/api/query-keys.ts`

```ts
related: (relPath: string) => ['related', relPath] as const,
relatedPrefix: () => ['related'] as const,
disabled: (prefix: string) => [prefix, '__disabled__'] as const,
```

The `disabled(prefix)` helper consolidates the existing inline
sentinel-key idiom (`['docs', '__invalid__']`, `['related', '__off__']`)
into one place. Phase 5 adopts it for `useRelated`; existing call
sites can migrate opportunistically.

#### 4. `src/api/use-related.ts` (new)

```ts
export function useRelated(relPath: string | undefined) {
  return useQuery({
    queryKey: relPath ? queryKeys.related(relPath) : queryKeys.disabled('related'),
    queryFn: () => fetchRelated(relPath!),
    enabled: !!relPath,
  })
}
```

#### 5. `src/api/use-wiki-link-resolver.ts` (new)

Combines the ADR + ticket caches into a memoised `Resolver` suitable
for `MarkdownRenderer`'s `resolveWikiLink` prop:

```ts
import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import { fetchDocs } from './fetch'
import { queryKeys } from './query-keys'
import { buildWikiLinkIndex, resolveWikiLink, type WikiLinkIndex } from './wiki-links'
import type { Resolver, ResolverResult } from '../components/MarkdownRenderer/wiki-link-plugin'

export interface UseWikiLinkResolverResult {
  resolver: Resolver
}

export function useWikiLinkResolver(): UseWikiLinkResolverResult {
  const adrs = useQuery({
    queryKey: queryKeys.docs('decisions'),
    queryFn: () => fetchDocs('decisions'),
  })
  const tickets = useQuery({
    queryKey: queryKeys.docs('tickets'),
    queryFn: () => fetchDocs('tickets'),
  })

  // `isPending` (TanStack Query v5) is true ONLY on the initial load
  // when no cached data exists. We deliberately use this rather than
  // `isFetching` (which is true on every background refetch). On a
  // background refetch with cached data, the resolver continues to
  // serve the previous resolved/unresolved verdicts using the still-
  // present `data` — see "Refetch staleness" below for the rationale.
  const isWarming = adrs.isPending || tickets.isPending

  const wikiIndex = useMemo<WikiLinkIndex>(
    () => buildWikiLinkIndex(adrs.data ?? [], tickets.data ?? []),
    [adrs.data, tickets.data],
  )

  const resolver = useMemo<Resolver>(
    () => (prefix, n): ResolverResult => {
      if (isWarming) return { kind: 'pending' }
      const hit = resolveWikiLink(prefix, n, wikiIndex)
      return hit ? { kind: 'resolved', ...hit } : { kind: 'unresolved' }
    },
    [isWarming, wikiIndex],
  )

  return { resolver }
}
```

`isWarming` is the gating signal: on the initial cold load (before
either docs query has any data), every lookup returns
`kind: 'pending'`, which the plugin renders as a `wiki-link-pending`
marker (Phase 4). On settle, `isWarming` flips, the resolver
memoises a new bound function with stable identity until the docs
caches change again, `MarkdownRenderer`'s memoised plugin tuple
rotates, and the pipeline re-runs — flipping pending markers to
anchors (or to `unresolved-wiki-link` markers for genuine misses).

**Refetch staleness (deliberate trade)**: when an ADR is renamed,
edited, or deleted while another doc references it, the docs cache
invalidates and TanStack Query refetches in the background. During
the refetch (typically <500ms on localhost), `isPending` stays
`false` because cached data exists, so wiki-links continue to
resolve against the *previous* docs cache until the refetch
completes. This means a deleted ADR's link briefly remains clickable
to its old href until the cache settles. The alternative — defining
`isWarming` as `isFetching` — would flicker every wiki-link to
`pending` on every unrelated `doc-changed` event (every doc edit
anywhere invalidates the docs cache via `useDocEvents`), which is
strictly worse UX than a sub-second staleness window. The trade is
acknowledged here so a future maintainer doesn't "fix" this without
realising the alternative.

The hook explicitly does not shadow `resolveWikiLink` — the inner
arrow function calls the imported pure function with the index as
the third argument. Locked by Phase 6 §c's wiring test.

#### 6. `src/api/use-doc-page-data.ts` (new)

Composes the doc-view's read-path queries:

```ts
export function useDocPageData(relPath: string | undefined) {
  const content = useDocContent(relPath)
  const related = useRelated(relPath)
  return { content, related }
}
```

Trivial composition today; the join point for any future doc-view
read fanout (backlinks-graph, suggested-next-action, etc.). Tests
the gating-on-`relPath` behaviour for both children at once.

#### 7. `src/api/use-doc-events.ts`

Extend the existing SSE handler so any inbound `doc-changed` event
invalidates the entire `related` prefix:

```ts
// Why prefix-invalidate the entire related namespace rather than a
// targeted key: the set of related-of pages that depend on a given
// doc is unbounded (every doc whose cluster contains it; every plan
// if it's a review's target; transitively…). Prefix-invalidate is the
// simplest correct behaviour. The lists are tiny and the practical
// fan-out is one mounted query per library page, so the cost is
// negligible. `refetchType: 'all'` ensures unmounted-but-cached
// queries are also revalidated, so navigating to a target plan
// after deleting one of its reviews shows fresh data on mount
// rather than serving stale data until the next event.
qc.invalidateQueries({
  queryKey: queryKeys.relatedPrefix(),
  refetchType: 'all',
})
```

### TDD Sequence (Phase 5)

1. **Step 5.1** — `fetch.test.ts: fetchRelated builds the right URL
   and decodes the payload`: stubbed `fetch` mock returns a known
   payload; assert URL is `/api/related/<encoded>` and the parsed
   response equals the input.
2. **Step 5.2** — `fetch.test.ts: fetchRelated throws FetchError on
   non-2xx`: stub returns 404; assert `FetchError` with `.status === 404`.
3. **Step 5.3** — `fetch.test.ts: fetchRelated encodes path segments
   correctly`: relPath with a `#` in the filename round-trips through
   `encodeURIComponent` per segment, never double-encoding `/`. Also
   covers `%` literals (encoded to `%25`, decoded back to `%`).
4. **Step 5.4** — `query-keys.test.ts: related and relatedPrefix
   structures lock`: snapshot the key shapes; assert
   `queryKeys.disabled('related')` does not collide with
   `queryKeys.related(...)` for any plausible input.
5. **Step 5.5** — `use-doc-events.test.ts: doc-changed event
   invalidates related-prefix with refetchType: 'all'`: drive a
   `doc-changed` event; assert `invalidateQueries` is called with
   `{ queryKey: queryKeys.relatedPrefix(), refetchType: 'all' }`.
6. **Step 5.5b** — `use-doc-events.test.ts: unrelated event kinds do
   not invalidate related-prefix`: drive an event of kind other than
   `doc-changed` (e.g., `connected`); assert no related-prefix
   invalidation is issued. Pairs with 5.5 to lock the scope contract.
7. **Step 5.6** — `use-related.test.tsx: enabled is gated on relPath`:
   render the hook with `undefined` relPath; assert the query never
   fires; flip to a defined relPath; assert one fetch call.
8. **Step 5.7** — `use-wiki-link-resolver.test.tsx: returns kind=resolved
   after both docs queries settle and ID matches`: stub
   `fetchDocs('decisions')` with one ADR; render the hook; once both
   queries settle, assert `result.current.resolver('ADR', 1)` returns
   `{ kind: 'resolved', href, title }` with the expected values.
9. **Step 5.7b** — `use-wiki-link-resolver.test.tsx: returns kind=unresolved
   for unknown IDs after queries settle`: stub both queries with
   empty arrays; once settled, assert
   `result.current.resolver('ADR', 9999)` returns `{ kind: 'unresolved' }`.
10. **Step 5.8** — `use-wiki-link-resolver.test.tsx: returns kind=pending
    while either query is pending`: keep `fetchDocs('decisions')`
    pending; assert `result.current.resolver('ADR', 1)` returns
    `{ kind: 'pending' }`. Locks the warming-distinct-from-unresolved
    contract that drives the marker variant in the plugin.
11. **Step 5.8b** — `use-wiki-link-resolver.test.tsx: resolver
    reference is memo-stable across re-renders with unchanged state`:
    render the hook twice while both queries remain pending; assert
    `result1.resolver === result2.resolver` (referential equality).
    Render twice after settle with unchanged data; assert the same.
    Then flip from pending to settled; assert the resolver reference
    *does* change (the dep `[isWarming, wikiIndex]` rotated). Locks
    the memoisation contract that drives flip-on-settle in
    `MarkdownRenderer`'s memoised plugin tuple.
12. **Step 5.8c** — `use-wiki-link-resolver.test.tsx: refetch with
    cached data does NOT return resolver to pending`: stub both
    `fetchDocs` queries with one ADR each; await settle; assert
    `result.current.resolver('ADR', 1)` returns `kind: 'resolved'`.
    Trigger `queryClient.invalidateQueries({ queryKey: queryKeys.docs() })`
    (background refetch with cached data); assert the resolver still
    returns `kind: 'resolved'` (not `pending`) for the duration of the
    refetch. Locks the deliberate refetch-staleness trade documented
    in Phase 5 §5 prose.
12. **Step 5.9** — `use-doc-page-data.test.tsx: gates both children
    on relPath`: render with `undefined` relPath; assert neither
    `useDocContent` nor `useRelated` fires; flip to a value; assert
    both queries fire and produce distinct cache entries (one
    assertion that they don't collide on a shared key).

### Success Criteria

#### Automated Verification

- [ ] `npm test` is fully green.
- [ ] `npm run typecheck` is clean.
- [ ] `npm run lint` is clean.

---

## Phase 6: `RelatedArtifacts` component + LibraryDocView wiring

### Overview

Replace the placeholder `<p>No related artifacts yet.</p>` in
`LibraryDocView` with a `<RelatedArtifacts>` component that consumes
`useRelated` and renders three visually distinct groupings (Targets,
Inbound reviews, Same lifecycle) plus an explicit empty state and
explicit error state. `LibraryDocView` composes the new
`useDocPageData` and `useWikiLinkResolver` hooks (Phase 5) so the
view itself stays close to the existing thin-composition pattern.

### Changes Required

#### 1. `src/components/RelatedArtifacts/RelatedArtifacts.tsx` (new)

```tsx
interface Props {
  related: RelatedArtifactsResponse
  /** Optional — set to true ONLY when a refetch has been in flight
   *  for more than ~250ms AND will likely produce different data.
   *  Use the `useDeferredFetchingHint` helper below — passing
   *  `query.isFetching` directly causes the hint to flash on every
   *  background refetch (`refetchType: 'all'` invalidates the
   *  related prefix on every doc-changed event, including unrelated
   *  edits). */
  showUpdatingHint?: boolean
}

export function RelatedArtifacts({ related, showUpdatingHint }: Props) {
  const isEmpty =
    related.inferredCluster.length === 0 &&
    related.declaredOutbound.length === 0 &&
    related.declaredInbound.length === 0
  if (isEmpty) {
    return <p className={styles.emptyAll}>This document has no declared or inferred relations.</p>
  }
  return (
    <>
      {showUpdatingHint && <p className={styles.updating} aria-live="polite">Updating…</p>}
      <Legend />
      {related.declaredOutbound.length > 0 && (
        <RelatedGroup
          label="Targets"
          entries={related.declaredOutbound}
          kind="declared"
        />
      )}
      {related.declaredInbound.length > 0 && (
        <RelatedGroup
          label="Inbound reviews"
          entries={related.declaredInbound}
          kind="declared"
        />
      )}
      {related.inferredCluster.length > 0 && (
        <RelatedGroup
          label="Same lifecycle"
          entries={related.inferredCluster}
          kind="inferred"
        />
      )}
    </>
  )
}
```

`RelatedGroup` renders an `<h4>` heading (one level below the
parent section's `<h3>Related artifacts</h3>`) and an unordered
list of anchor items (`/library/:type/:fileSlug`) with a small
badge showing the kind tag. The container element carries a
`groupDeclared` or `groupInferred` modifier class so the visual
distinction is element-named rather than state-named.

`Legend` renders a small definition list under the section heading:

> **Declared** — explicit cross-reference in frontmatter.
> **Inferred** — shares a slug with this document.

The legend is always rendered when at least one group is present,
giving readers a self-contained explanation of why the items appear.

CSS lives in two files because the wiki-link marker classes are
emitted by the remark plugin as literal kebab-case strings (the
plugin can't compute hashed CSS-module identifiers at AST-build
time). Component-scoped styles use CSS modules (camelCase
identifiers, hashed at build); wiki-link marker styles are
deliberate globals.

`RelatedArtifacts.module.css` (CSS-module-scoped):

```css
.groupDeclared { border-left: 2px solid var(--accent-strong); }
.groupInferred { border-left: 2px dashed var(--accent-soft); }
.emptyAll { /* used only when all three arrays are empty */ }
.loading { /* used only while the related query is in flight */ }
.updating { /* the subtle "Updating…" hint during refetch */ }
```

`src/styles/wiki-links.global.css` (deliberate globals — imported
once in `main.tsx` so the kebab-case class names survive the build
unhashed):

```css
.wiki-link-pending {
  /* Neutral skeleton: italic, dimmed, no underline.
     Reads as "loading", not "broken". */
  color: var(--text-muted);
  font-style: italic;
}
.unresolved-wiki-link {
  /* Broken-reference styling: muted with dotted underline.
     Reads as "this didn't resolve, check your ID". */
  color: var(--text-muted);
  border-bottom: 1px dotted var(--text-muted);
  cursor: help;
}
```

The two marker classes are intentionally global because the remark
plugin emits them as literal `className: 'wiki-link-pending'` /
`'unresolved-wiki-link'` strings on `data.hProperties` — CSS-module
hashing would break the selector match. The globals file is the
*only* place this project uses unscoped class names; `globals.css`
in the project's existing layout is for design tokens (CSS custom
properties), so this new `wiki-links.global.css` is a sibling with
a narrower remit.

The "Same lifecycle" label matches the server-side `LifecycleCluster`
terminology rather than the more ambiguous "Same workflow".

#### 2. `src/routes/library/LibraryDocView.tsx`

Three additions:

a. Compose the doc-page-data and wiki-link-resolver hooks:

```tsx
const { content, related } = useDocPageData(entry?.relPath)
const { resolver: resolveWikiLink } = useWikiLinkResolver()
```

There is no shadowing here: `resolveWikiLink` is the destructured
hook output (a bound `Resolver` function); the imported pure helper
of the same name lives only inside `use-wiki-link-resolver.ts`.

`LibraryDocView` deliberately does *not* gate body rendering on the
docs caches. While they are still warming, the resolver returns
`null` for every call — but rather than reusing the
`unresolved-wiki-link` marker (which connotes "broken reference"),
the plugin emits a *distinct* `wiki-link-pending` marker (Phase 4 §1
spec). Pending markers carry a neutral skeleton/italic styling so
the cold-load state visibly reads as "loading" rather than "broken".
When the docs caches settle, TanStack Query re-renders the view, the
resolver memoises a new bound function (with stable identity until
the caches change again), `MarkdownRenderer` re-runs its memoised
plugin pipeline because the resolver reference changed, and pending
markers flip to anchors (or to `unresolved-wiki-link` markers if a
specific ID truly has no match). This single transition is
preferable to a full-body `Loading…` placeholder for two reasons:
(a) docs without `[[…]]` references — the dominant case — incur no
perceptible delay, and (b) `MarkdownRenderer` remains usable in
isolation by future consumers without inheriting `LibraryDocView`'s
docs-cache lifecycle.

b. Replace the placeholder block with explicit loading / error /
   data branches matching the existing `LibraryDocView` pattern
   (the doc-list and doc-content sections already use the same
   tri-state with `role="alert"`). Drive the `showUpdatingHint`
   prop through the deferred-hint helper so it doesn't flash on
   every background refetch:

```tsx
const showUpdatingHint = useDeferredFetchingHint(related)

<section>
  <h3>Related artifacts</h3>
  {related.isError && (
    <p role="alert" className={styles.error}>
      Failed to load related artifacts: {related.error.message}
    </p>
  )}
  {related.isPending && !related.isError && (
    <p className={styles.loading}>Loading…</p>
  )}
  {related.data && (
    <RelatedArtifacts related={related.data} showUpdatingHint={showUpdatingHint} />
  )}
</section>
```

The `useDeferredFetchingHint` helper (in
`src/api/use-deferred-fetching-hint.ts`) gates the hint on two
conditions:

1. The refetch has been in flight for at least ~250ms (so transient
   sub-second refetches — the common case for unrelated
   `doc-changed` invalidations under `refetchType: 'all'` — never
   surface the hint).
2. The query is `isFetching && !isPending` (i.e., a refresh of
   already-rendered data, not the initial load — initial-load is
   already covered by the `Loading…` branch above).

Sketch:

```ts
export function useDeferredFetchingHint(query: { isFetching: boolean; isPending: boolean }, delayMs = 250): boolean {
  const [show, setShow] = useState(false)
  const isRefetch = query.isFetching && !query.isPending
  useEffect(() => {
    if (!isRefetch) { setShow(false); return; }
    const id = setTimeout(() => setShow(true), delayMs)
    return () => clearTimeout(id)
  }, [isRefetch, delayMs])
  return show
}
```

c. Pass the resolver into the renderer with no body-rendering gate:

```tsx
{content.data && (
  <MarkdownRenderer content={content.data.content} resolveWikiLink={resolveWikiLink} />
)}
```

While the docs caches warm, the resolver returns `{ kind: 'pending' }`
and any `[[…]]` references render as `wiki-link-pending` markers
(neutral skeleton/italic — visibly "loading", not "broken"); once
the caches settle they flip to anchors (or `unresolved-wiki-link`
markers for genuine misses) on the next React render. Docs without
wiki-links — the common case — see no transition at all.

### TDD Sequence (Phase 6)

Tests in
`src/components/RelatedArtifacts/RelatedArtifacts.test.tsx` and
extending `src/routes/library/LibraryDocView.test.tsx`.

1. **Step 6.1** —
   `RelatedArtifacts.test.tsx: shows all-empty message when all three
   arrays are empty`: render with empty response; assert
   "This document has no declared or inferred relations." is present
   and no group headings render.
2. **Step 6.2** —
   `RelatedArtifacts.test.tsx: renders Targets group as h4 for
   declaredOutbound`: one outbound entry; assert an `h4` with text
   "Targets" is present and the anchor's href is
   `/library/<type>/<fileSlug>`.
3. **Step 6.3** —
   `RelatedArtifacts.test.tsx: renders Inbound reviews group as h4 for
   declaredInbound`: one inbound entry; `h4` "Inbound reviews"; anchor
   correct.
4. **Step 6.4** —
   `RelatedArtifacts.test.tsx: renders Same lifecycle group as h4 for
   inferredCluster`: one inferred entry; `h4` "Same lifecycle"; anchor
   correct. Locks the renamed label.
5. **Step 6.5** —
   `RelatedArtifacts.test.tsx: declared and inferred groups carry
   distinct element-named CSS modifier classes`: assert one container
   has `groupDeclared` modifier and the other has `groupInferred`.
6. **Step 6.5b** —
   `RelatedArtifacts.test.tsx: legend explains declared vs inferred`:
   assert the rendered output contains the words "Declared" and
   "Inferred" with their explanations whenever any group is rendered.
7. **Step 6.5c** —
   `RelatedArtifacts.test.tsx: shows Updating hint only when
   showUpdatingHint is true`: render with a populated response and
   `showUpdatingHint={true}`; assert "Updating…" is present in an
   `aria-live="polite"` region. Render again with `showUpdatingHint={false}`;
   assert no "Updating…" text. The component itself never reads
   `isFetching` directly — the deferred-hint helper is the gating
   point so the hint cannot flash on transient background refetches.
8. **Step 6.5d** —
   `use-deferred-fetching-hint.test.tsx: hint stays false for
   sub-250ms refetches`: drive the helper with a query where
   `isFetching` flips true then false within 100ms (using fake
   timers); assert the returned boolean stays `false` throughout.
   Then drive a refetch lasting 500ms; assert the boolean becomes
   `true` after 250ms. Locks the debounce contract that prevents
   hint-flashing under SSE-storm conditions (every unrelated
   `doc-changed` event invalidates the related prefix with
   `refetchType: 'all'`, so the helper sees frequent fast refetches).
9. **Step 6.5e** —
   `use-deferred-fetching-hint.test.tsx: hint resets to false on
   pending`: drive the helper with `isPending=true, isFetching=true`
   (the initial load); assert `false` (the `Loading…` placeholder
   covers this case, the hint is for refetches only).
8. **Step 6.6** —
   `LibraryDocView.test.tsx: fetches related on mount`: stub
   `fetchRelated` to resolve with a populated response; render the
   view; assert the relevant heading appears.
9. **Step 6.6b** —
   `LibraryDocView.test.tsx: renders error path with role=alert when
   fetchRelated fails`: stub `fetchRelated` to reject with a
   `FetchError`; assert the rendered output contains an element with
   `role="alert"` whose text mentions "Failed to load related
   artifacts" and that no `Loading…` placeholder is rendered.
10. **Step 6.6c** —
    `LibraryDocView.test.tsx: renders Loading… while related is in
    flight`: leave `fetchRelated` pending; assert the `Loading…`
    placeholder is present and no `RelatedArtifacts` content is.
11. **Step 6.7** —
    `LibraryDocView.test.tsx: real wiring resolves wiki-link to
    anchor when ADR is in cache`: stub `fetchDocs('decisions')` with
    a known ADR; render content `"[[ADR-0001]]"`; assert the rendered
    output contains an `<a>` with the entry's title as link text and
    the right library URL as href. Critically, this test does *not*
    pass a resolver via prop — it exercises the real
    `useWikiLinkResolver` → `MarkdownRenderer` wiring (the path that
    would have stack-overflowed under the original shadowing).
12. **Step 6.7b** —
    `LibraryDocView.test.tsx: body renders immediately with pending marker, then flips to anchor on settle`:
    keep `fetchDocs('decisions')` pending and render content
    `"[[ADR-0001]]"`. Assert the body renders immediately with a
    `<span class="wiki-link-pending" title="Loading reference…">[[ADR-0001]]</span>`
    (no `Loading…` placeholder, no missing body, *not* the
    unresolved-marker class). Settle the `fetchDocs('decisions')`
    promise with the matching ADR. Assert the pending span is
    replaced by an `<a>` with the entry's title and the right library
    href on the next render. Locks the deliberate
    "render-immediately, flip-on-settle" UX contract *and* the
    pending-vs-unresolved distinction that prevents cache warm-up
    from looking like a broken-references doc.
13. **Step 6.8** —
    `LibraryDocView.test.tsx: renders unresolved-wiki-link span when
    ADR is not in cache after settle`: stub `fetchDocs('decisions')`
    with an empty *settled* list; render content `"[[ADR-9999]]"`;
    assert the rendered output contains a
    `<span class="unresolved-wiki-link" title="No matching ADR found for ID 9999">[[ADR-9999]]</span>`
    and no anchor. The `unresolved-wiki-link` class is *only* used
    once both docs queries have settled, locking the typo-feedback
    affordance distinct from Step 6.7b's mid-load behaviour.

### Success Criteria

#### Automated Verification

- [ ] `npm test` is fully green.
- [ ] `npm run typecheck` is clean.
- [ ] `npm run lint` is clean.
- [ ] `npm run build` produces a clean dist (no TS errors, no
  unused-import warnings).

#### Manual Verification

- [ ] Open the library page for a plan-review with a `target:` field;
  the "Targets" group lists the target plan; clicking the link
  navigates to it.
- [ ] Open the target plan's page; the "Inbound reviews" group lists
  the review; clicking the link navigates to it.
- [ ] On a doc page whose body contains `[[ADR-0017]]` (or any
  resolvable ID), the wiki-link renders as a clickable anchor whose
  visible text is the entry's title and whose hover tooltip shows
  `[[ADR-0017]]`.
- [ ] On a hard reload of a doc page containing `[[ADR-0017]]`, the
  reference *first* renders inside a `<span class="wiki-link-pending">`
  (neutral skeleton, tooltip "Loading reference…") while the docs
  caches warm; once they settle (typically <500ms), the pending
  marker flips to a clickable anchor. The transient pending state
  must be visibly distinct from a broken reference.
- [ ] On a doc page with `[[ADR-9999]]` (unresolvable, after caches
  have settled), the text renders inside a styled
  `<span class="unresolved-wiki-link">` with a diagnostic tooltip
  ("No matching ADR found for ID 9999") — visually distinct from
  both prose and the pending-state span.
- [ ] On a doc page with `[[0001]]` (bare numeric form), the text
  renders verbatim with no anchor and no styled span (the syntax
  is not recognised).
- [ ] On a doc page where the body has `\`[[ADR-0001]]\`` (inside
  inline code), the text renders inside a `<code>` element with no
  anchor wrapping; same for triple-backtick fenced blocks.
- [ ] Edit a plan-review's `target:` to point at a different plan;
  within ~1 second, the originally-targeted plan's library page
  drops the inbound review and the new target plan's page gains it
  (SSE-driven, no manual refresh). The "Updating…" hint should
  *not* flash visibly during this window unless the refetch takes
  longer than ~250ms.
- [ ] Quickly save several unrelated docs in succession; the related
  aside on a static library page does *not* flash "Updating…"
  repeatedly (sub-250ms refetches are debounced).

---

## Phase 7: Fixtures + cross-cutting integration smoke

### Overview

The existing fixtures already cover the primary case
(`first-plan` ↔ `first-plan-review-1`). Phase 9 needs one extra
fixture pair to cover the multi-review-per-target case, plus a
genuine cross-cutting smoke that exercises the
SSE-invalidation → refetch → render round-trip in the frontend
(steps 5.5 and 6.6 each prove half of this in isolation; nothing in
CI proves they compose).

### Changes Required

#### 1. `tests/fixtures/meta/reviews/plans/2026-01-01-first-plan-review-2.md`

Same `target:` as `-review-1`, different review number; covers Step
1.5's "multiple reviews per target" assertion at the fixture level.

```markdown
---
target: "meta/plans/2026-01-01-first-plan.md"
---

Second review of the first plan.
```

#### 2. `frontend/src/routes/library/LibraryDocView.smoke.test.tsx` (new)

Two cross-cutting frontend integration tests that exercise the wiring
across phases 4–6, each locking a distinct contract:

**Test A — active refetch + DOM rerender (`smoke_active_refetch_rerenders_dom`)**:

1. Mount `LibraryDocView` for a plan with one inbound review (initial
   `fetchRelated` returns the review in `declaredInbound`).
2. Update the mock so the next `fetchRelated` returns an empty
   `declaredInbound`; dispatch a synthetic `doc-changed` SSE event
   while the view is still mounted.
3. Assert a second `fetchRelated` is called.
4. Assert the rendered "Inbound reviews" group disappears within the
   test's awaitility window.

This locks the *active* refetch path — the default `refetchType: 'active'`
would also pass it. It is the cross-phase composition smoke; the
specific `refetchType: 'all'` behaviour is locked by Step 5.5 at the
unit level.

**Test B — inactive-query revalidation on remount (`smoke_inactive_cached_query_refetches_on_remount`)**:

1. Mount `LibraryDocView` for plan A; await `fetchRelated` resolution
   (cache populated for A's related response).
2. Unmount the view (the related query becomes inactive but stays in
   the cache, per TanStack Query default `gcTime`).
3. Update the mock so the next `fetchRelated` for A returns a
   different `declaredInbound`; dispatch a synthetic `doc-changed` SSE
   event with the view unmounted.
4. Remount the view for plan A; assert that the rendered DOM reflects
   the *new* response on first paint (no stale data from the cached
   response visible).

Test B is the contract `refetchType: 'all'` exists for — without it,
the SSE invalidation marks the inactive query stale but does not
refetch, and on remount TanStack Query serves the stale cached value
until the query refetches itself. With `refetchType: 'all'`, the
cached query is refetched at event time and the remount serves fresh
data immediately.

### TDD Sequence (Phase 7)

1. **Step 7.1** — Add the second review fixture file. Run
   `cargo test -p accelerator-visualiser --test api_related` and
   confirm all fourteen Phase 2 cases still pass; plus add a
   fifteenth case `related_endpoint_returns_multiple_inbound_reviews`
   that expects two entries in `declaredInbound` for the first plan.
2. **Step 7.2a** — Add Test A (`smoke_active_refetch_rerenders_dom`).
   Locks the cross-phase composition: SSE event → invalidation →
   refetch → re-render flows end-to-end through `useDocEvents`,
   `useRelated`, `RelatedArtifacts`. Independently of `refetchType`.
3. **Step 7.2b** — Add Test B (`smoke_inactive_cached_query_refetches_on_remount`).
   Verify the test fails when `refetchType: 'all'` is omitted from
   `useDocEvents` (since the default `'active'` does not refetch
   inactive queries) and passes when present. This is the test that
   locks the unmounted-but-cached invariant the plan calls out as the
   reason for `refetchType: 'all'`.

### Success Criteria

#### Automated Verification

- [ ] `cargo test -p accelerator-visualiser` is fully green.
- [ ] `npm test` is fully green (including the new smoke test).
- [ ] `cargo clippy -p accelerator-visualiser` is clean.
- [ ] `cargo fmt --check` is clean.
- [ ] `npm run typecheck` is clean.
- [ ] `npm run lint` is clean.

#### Manual Verification

- [ ] All "Manual Verification" entries from Phase 6 pass against a
  freshly launched server with the seeded fixtures.

---

## Testing Strategy

Test counts per phase are listed in the per-phase TDD sequences
above; the summary below names the test surfaces rather than
attempting to keep arithmetic in sync.

### Unit Tests

- **Server (`src/indexer.rs`)**: covers the `reviews_by_target`
  lifecycle and the three explicit `update_*` / `remove_from_*`
  helpers — build, refresh, target migration (including the
  atomic-migration concurrency invariant via the
  `test_post_secondary_update` Notify hook), deletion, missing
  target, multi-target with dedup, malformed target sanitisation,
  plan-reviews-only kind restriction, and the rescan-vs-target-migration
  race. Also covers `normalize_absolute` directly (Step 1.0) and the
  canonical-`project_root` discipline at `Indexer::build` (Step 1.2).
- **Server (`src/api/related.rs`)**: helper-level tests for
  decoded-path validation, self-exclusion in clusters, the
  inferred/declared dedup contract, and outbound/inbound
  computation against an in-memory state.
- **Frontend (`src/api/wiki-links.test.ts`)**: pattern matching
  (including the bounded `\d{1,6}` quantifier and boundary cases),
  prefix sensitivity, ADR/ticket ID derivation with explicit radix,
  duplicate-ID tie-breaker, kind-filter defence, miss handling,
  href + title return shape.
- **Frontend (`MarkdownRenderer/wiki-link-plugin.test.ts`)**:
  AST plugin matching, splitting, code/inline-code skip, fenced
  block skip, SKIP-prevents-double-rewrite, resolved Link shape
  (display=title, hover=bracket-form), unresolved-marker shape.
- **Frontend (`use-wiki-link-resolver.test.tsx`,
  `use-doc-page-data.test.tsx`)**: settled vs pending resolver
  states, gating both children on `relPath`.

### Integration Tests

- **Server (`server/tests/api_related.rs`)**: drives the full axum
  router via `tower::ServiceExt::oneshot` against the seeded
  `tempfile` fixture — JSON wire shape (camelCase), status codes,
  declared-link round-trips, dedup overlap, target-deletion
  behaviour, decoded-path validation.
- **Frontend (`LibraryDocView.test.tsx`,
  `RelatedArtifacts.test.tsx`,
  `LibraryDocView.smoke.test.tsx`)**: visual state of the related
  aside (data, loading, error, empty), real wiring of the wiki-link
  resolver through `useWikiLinkResolver`, the render-immediately +
  flip-on-settle wiki-link transition (Step 6.7b), distinct
  `wiki-link-pending` vs `unresolved-wiki-link` markers,
  unresolved-span feedback for typos, and the cross-cutting
  SSE→invalidation→refetch→re-render round-trip (Test A and Test B).

### Manual Testing Steps

1. Launch the server (`/accelerator:visualise` or `cargo run`).
2. Open `/library/plan-reviews/2026-04-18-foo-review-1` (the seeded
   review). Confirm the "Targets" group lists `2026-04-18-foo` and
   the legend explains "Declared" vs "Inferred".
3. Open `/library/plans/2026-04-18-foo`. Confirm the "Inbound reviews"
   group lists the review.
4. Edit the review's `target:` to point at a different plan; reload
   neither — wait. Within ~1s, both library pages reflect the new
   linkage; observe the brief "Updating…" hint.
5. Open any doc that contains `[[ADR-0017]]` in the body (or write a
   throwaway note in `meta/notes/` containing the link). Confirm the
   reference renders as a link whose visible text is the ADR's title
   and whose hover tooltip is `[[ADR-0017]]`.
6. Replace `[[ADR-0017]]` with `[[ADR-9999]]`. Confirm the rendered
   form is a styled `<span class="unresolved-wiki-link">` with the
   bracket-form on hover — no anchor.
7. Wrap a wiki-link in inline backticks. Confirm it renders inside a
   `<code>` block, no anchor.
8. Stop the server while a library page is open. Confirm the related
   aside renders an error message with `role="alert"` rather than a
   permanent loading spinner.

## Performance Considerations

- The reverse-target index adds one `HashMap<PathBuf, BTreeSet<PathBuf>>`
  to `Indexer`. At project scales relevant to v1 (a few hundred docs),
  the memory cost is negligible (a few KiB). `BTreeSet` is O(log n)
  on insert/remove rather than `Vec`'s O(n) for membership, which
  keeps Phase 1's per-`refresh_one` work fast even as the inbound set
  grows.
- The related endpoint is read-only and reads from already-cached
  state (`AppState.clusters` + the indexer's secondary maps); no
  filesystem I/O on the hot path.
- The wiki-link plugin allocates only when at least one `[[…]]` match
  is found in a text node (the helper returns `null` for plain prose
  to skip the splice). The bounded `\d{1,6}` quantifier prevents
  pathological inputs from extending scan time.
- SSE prefix-invalidation of the `related` namespace with
  `refetchType: 'all'` revalidates active queries immediately and
  marks unmounted-but-cached queries stale, so navigation after a
  delete shows fresh data on mount. Practical fan-out remains one
  fetch per inbound `doc-changed` event for the active library page.

## Migration Notes

No data migration required. All new code is additive:
- The reverse index is built from existing frontmatter (`target:`)
  that's already in the wild.
- The related endpoint is a new route; no existing endpoint changes
  shape.
- The `MarkdownRenderer` prop is opt-in; pre-Phase-9 callers see no
  behavioural change.
- Frontend caches gain a new query key; no schema migration in
  storage layers (the cache is in-memory only).

## References

- Original spec: `meta/specs/2026-04-17-meta-visualisation-design.md`
  (§ "Cross-references", D6, D7).
- Research doc: `meta/research/2026-04-17-meta-visualiser-implementation-context.md`
  (Phase 9 in the implementation phasing).
- Phase 8 plan as TDD template: `meta/plans/2026-04-26-meta-visualiser-phase-8-kanban-write-path.md`.
- Existing indexer secondary-map precedent: `skills/visualisation/visualise/server/src/indexer.rs:36-37, 87-91, 220-228, 282-310`.
- Existing related placeholder: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:73-78`.
- Existing markdown renderer: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:10-21`.
- Existing fixture set: `skills/visualisation/visualise/server/tests/fixtures/meta/`.
- matchit catch-all + literal limitation: see Phase 8 plan
  "Key Discoveries" and matchit issue
  [#39](https://github.com/ibraheemdev/matchit/issues/39).
