---
date: "2026-04-25T00:00:00+01:00"
type: plan
skill: create-plan
status: draft
---

# Meta visualiser Phase 6 — Lifecycle clusters and view

## Overview

Phase 6 turns the existing server-side cluster computation into a working
Lifecycle view. By the end of this phase, navigating to `/lifecycle` shows
a sortable list of slug-clustered work units rendered as cards with a
horizontal pipeline-of-dots indicator; navigating to `/lifecycle/:slug`
shows a single cluster as a vertical timeline with placeholders for
missing stages.

Most of the server work was completed in earlier phases — `compute_clusters`,
`/api/lifecycle`, `/api/lifecycle/:slug`, watcher-triggered cluster refresh.
This phase adds the frontend types, fetch helpers, SSE invalidation for
cluster detail queries, and the two real views, plus two small server-side
additions: a per-cluster `lastChangedMs` field so the index page can sort
by "most recently changed" without re-walking entries on the client, and
a per-entry `bodyPreview` field so the timeline cards can show a snippet
of the document body without each card making its own
`fetchDocContent` request.

The approach is test-driven throughout. Server-side: tests for the new
`last_changed_ms` field land first, then the implementation. Frontend:
Vitest + React Testing Library tests are written before each component.

## Current state

Phases 1–5 are complete. Specifically, for Phase 6 the relevant pieces are:

- **`server/src/clusters.rs`** — `compute_clusters(entries)` produces
  `Vec<LifecycleCluster>` with `slug`, `title`, `entries` (sorted by
  canonical type rank then mtime), and `completeness` (9 boolean flags
  including the spec's 8 plus an extra `has_notes` carried over from a
  prior iteration). Output is sorted alphabetically by slug. Tests pass.
- **`server/src/api/lifecycle.rs`** — exposes `GET /api/lifecycle`
  (response shape `{ "clusters": [ … ] }`) and `GET /api/lifecycle/:slug`
  (response shape: a single `LifecycleCluster` directly, or `404` if
  unknown). Integration coverage in `tests/api_lifecycle.rs`.
- **`server/src/server.rs:45`** — `AppState.clusters: Arc<RwLock<Vec<LifecycleCluster>>>`
  is seeded at startup and recomputed by the watcher (`watcher.rs:116`)
  on every filesystem-change debounce.
- **`frontend/src/api/query-keys.ts`** — already declares `lifecycle()` and
  `lifecycleCluster(slug)` keys.
- **`frontend/src/api/use-doc-events.ts`** — invalidates `lifecycle()` on
  every doc-changed/doc-invalid event. Does **not** invalidate
  `lifecycle-cluster` keys (so a cluster detail view shows stale data after
  a file edit).
- **`frontend/src/routes/lifecycle/LifecycleStub.tsx`** — placeholder
  rendered for `/lifecycle`. The router does not yet have a
  `/lifecycle/:slug` route.
- **`frontend/src/api/types.ts`** — has no `LifecycleCluster`,
  `Completeness`, or `LifecycleListResponse` types.
- **`frontend/src/api/fetch.ts`** — has no lifecycle fetch helpers.

## Desired end state

- A `lastChangedMs` field on every `LifecycleCluster` returned by
  `/api/lifecycle` and `/api/lifecycle/:slug`, equal to the maximum
  `mtimeMs` across the cluster's entries (or `0` for an empty cluster,
  which never occurs since empty buckets are filtered out).
- A `bodyPreview` field on every `IndexEntry`, holding a plain-text
  snippet (~200 chars) extracted from the document body during indexing.
  Empty string when the document has no body or the body is purely
  headings.
- Frontend `types.ts` declares `LifecycleCluster`, `Completeness`,
  `LifecycleListResponse`, and a `LIFECYCLE_PIPELINE_STEPS` constant
  describing nine pipeline stages (eight workflow plus a `Notes`
  long-tail) and a derived `WORKFLOW_PIPELINE_STEPS` helper for the
  indicator. The index card and the eight-stage portion of the detail
  timeline use the workflow subset; `Notes` renders in a separate
  long-tail section below the main timeline.
- `fetchLifecycleClusters()` and `fetchLifecycleCluster(slug)` exist and are
  tested.
- `useDocEvents` invalidates the `['lifecycle-cluster']` prefix on every
  doc-changed / doc-invalid event so open detail views refresh after a
  file edit.
- A new `/lifecycle` index page renders one card per cluster with a
  horizontal pipeline-of-dots component, a "X of 8 stages" completeness
  badge, three sort modes — `recent` (default — descending
  `lastChangedMs`), `oldest` (ascending), `completeness` (descending count
  of true booleans, mtime tiebreaker) — and a text filter input that
  narrows the visible cards by case-insensitive substring match against
  title or slug.
- A new `/lifecycle/:slug` detail page renders a vertical timeline of
  `entries`, with each present stage as a card (type · date · title ·
  link to library) and each absent stage as a faded placeholder ("no
  plan yet", "no plan-review yet", etc.).
- The router has a `lifecycleRoute` layout, a `lifecycleIndexRoute` for
  `/lifecycle`, and a `lifecycleClusterRoute` for `/lifecycle/$slug`.
  `LifecycleStub` is removed.
- All Rust tests pass (`cargo test --features dev-frontend` and the
  default-features lib pass).
- All frontend tests pass (`npm run test`).

### Verification

```bash
# Server unit + integration:
cd skills/visualisation/visualise/server
cargo test --lib                            # default features: catches feature-gating regressions
cargo test --lib --features dev-frontend
cargo test --tests --features dev-frontend

# Frontend:
cd ../frontend
npm run test
```

Manual:

1. Start the server against a real meta directory and open the URL.
2. Navigate to `/lifecycle` — see one card per cluster, ordered with the
   most-recently-modified at the top.
3. Click a card → `/lifecycle/:slug` shows the timeline; missing stages
   render as faded placeholders.
4. Edit a `.md` file on disk → within ~500ms both the index card position
   (if mtime changed) and the open detail view re-render via SSE.
5. Click a timeline card's "Open in library" link → lands on the matching
   `/library/:type/:fileSlug` page.

## What we are NOT doing

- **"Related artifacts" wiring** on library pages — Phase 9 (cross-references)
  is the home for that. This phase does not modify `LibraryDocView`.
- **"Promote inferred to explicit" affordance** — Phase 9 / post-v1.
- **Wiki-link resolution** in cluster card titles — Phase 9.
- **Filter by completeness state** beyond the three sort modes — out of
  scope for v1.
- **Lifecycle SSE-driven optimistic updates** — relying on the existing
  invalidation + refetch is sufficient for v1.
- **Scoping `bodyPreview` to lifecycle-only endpoints** — `bodyPreview`
  travels on every `IndexEntry` (so `/api/docs` carries it too), even
  though only the cluster timeline consumes it today. Accepted
  tradeoff: the universal field is simpler to populate, future doc
  views may opt-in cheaply, and the per-entry overhead (~200 chars)
  is small. If/when payload size becomes a real concern, introduce a
  slim `IndexEntrySummary` projection at the `/api/docs` boundary
  rather than splitting the type now.

---

## Implementation approach

Phase 6 follows TDD in ten steps, server first, then frontend bottom-up:

1. Server: `last_changed_ms` field on `LifecycleCluster`.
2. Server: `body_preview` field on `IndexEntry`.
3. Server: integration test for both new fields on the wire.
4. Frontend: lifecycle types in `types.ts` (and `bodyPreview` on `IndexEntry`).
5. Frontend: fetch helpers + tests.
6. Frontend: `useDocEvents` cluster-prefix invalidation + test.
7. Frontend: `PipelineDots` component (TDD).
8. Frontend: `LifecycleIndex` view (TDD).
9. Frontend: `LifecycleClusterView` detail page with body-preview cards (TDD).
10. Frontend: router wiring + router tests; remove `LifecycleStub`.

Each step is an independently committable unit; tests are written before
code in every step.

---

## Step 1: Server — `last_changed_ms` on `LifecycleCluster` (TDD)

### File: `skills/visualisation/visualise/server/src/clusters.rs`

#### 1a. Update tests to assert the new field

Add a focused test for `last_changed_ms` and update existing tests where
they touch the cluster shape. Place the new test alongside the existing
tests in the `tests` module:

```rust
#[test]
fn last_changed_ms_is_max_mtime_across_entries() {
    let entries = vec![
        entry(DocTypeKey::Tickets, "foo", 100, "T"),
        entry(DocTypeKey::Plans, "foo", 500, "P"),     // newest
        entry(DocTypeKey::PlanReviews, "foo", 300, "R"),
    ];
    let clusters = compute_clusters(&entries);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].last_changed_ms, 500);
}

#[test]
fn last_changed_ms_for_single_entry_is_that_entry_mtime() {
    let entries = vec![entry(DocTypeKey::Plans, "solo", 42, "P")];
    let clusters = compute_clusters(&entries);
    assert_eq!(clusters[0].last_changed_ms, 42);
}

#[test]
fn last_changed_ms_is_per_cluster_and_survives_slug_sort() {
    // Two clusters, intentionally inserted in non-alphabetic order so
    // the test pins (a) `last_changed_ms` is computed per-bucket (not
    // over all entries) and (b) the value follows its cluster through
    // the alphabetic-by-slug sort applied at the end of
    // `compute_clusters`.
    let entries = vec![
        // 'foo' cluster — newest is 500.
        entry(DocTypeKey::Plans,   "foo", 100, "P-foo"),
        entry(DocTypeKey::Tickets, "foo", 500, "T-foo"),
        // 'bar' cluster — newest is 900.
        entry(DocTypeKey::Plans,   "bar", 900, "P-bar"),
        entry(DocTypeKey::Tickets, "bar", 200, "T-bar"),
    ];
    let clusters = compute_clusters(&entries);
    assert_eq!(clusters.len(), 2);
    // Alphabetic sort: 'bar' first, 'foo' second.
    assert_eq!(clusters[0].slug, "bar");
    assert_eq!(clusters[0].last_changed_ms, 900);
    assert_eq!(clusters[1].slug, "foo");
    assert_eq!(clusters[1].last_changed_ms, 500);
}
```

#### 1b. Add the field to `LifecycleCluster`

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleCluster {
    pub slug: String,
    pub title: String,
    pub entries: Vec<IndexEntry>,
    pub completeness: Completeness,
    pub last_changed_ms: i64,
}
```

#### 1c. Compute it inside `compute_clusters`

In the `.map(...)` step that builds each `LifecycleCluster`, after sorting
the entries:

```rust
let last_changed_ms = entries.iter().map(|e| e.mtime_ms).max().unwrap_or(0);
let title = derive_title(&slug, &entries);
let completeness = derive_completeness(&entries);
LifecycleCluster {
    slug,
    title,
    entries,
    completeness,
    last_changed_ms,
}
```

`unwrap_or(0)` covers the impossible empty-bucket case; buckets are only
created when an entry is pushed, so the iterator is non-empty in practice.

#### Success criteria

```bash
cd skills/visualisation/visualise/server
cargo test clusters
# all 10 tests pass (existing 7 + new 3)
```

---

## Step 2: Server — `body_preview` on `IndexEntry` (TDD)

### Files: `skills/visualisation/visualise/server/src/frontmatter.rs` and `indexer.rs`

`body_preview` is computed once at index time, alongside the existing
title derivation. The frontend can render it directly without a second
HTTP round-trip per card. Snippets are plain-text only — `react-markdown`
in the cluster view is overkill for a 200-char hint, and plain text
sidesteps both XSS and visual noise from inline markdown.

#### 2a. Pure helper + tests in `frontmatter.rs`

Add a `body_preview_from(body: &str) -> String` helper next to
`title_from`. Strategy:

1. Strip leading whitespace and blank lines.
2. Skip leading heading lines (so a leading `# Title` doesn't duplicate
   the document title).
3. From what remains, take the first non-heading paragraph; join its
   lines with single spaces.
4. Treat any further heading line as a paragraph terminator — once we
   have collected content, a heading ends the preview just like a
   blank line would. This prevents a mid-document `## Section` from
   silently splicing the next paragraph onto the first.
5. Collapse internal whitespace runs (newlines, tabs, multiple spaces)
   into single spaces.
6. Truncate to 200 chars on a UTF-8 boundary, append `…` if truncated.
   The truncation predicate is `> 200`, so a body of exactly 200 chars
   is *not* truncated and gets no ellipsis.
7. Return `""` when nothing usable is left.

Write the tests first:

```rust
#[cfg(test)]
mod body_preview_tests {
    use super::body_preview_from;

    #[test]
    fn empty_body_returns_empty_string() {
        assert_eq!(body_preview_from(""), "");
        assert_eq!(body_preview_from("   \n\n   "), "");
    }

    #[test]
    fn skips_leading_h1_to_avoid_duplicating_title() {
        let body = "# The Foo Plan\n\nThis is the body of the plan.\n";
        assert_eq!(body_preview_from(body), "This is the body of the plan.");
    }

    #[test]
    fn takes_first_non_heading_paragraph() {
        let body = "## Section\n\nFirst sentence here.\n\n## Next\n\nSecond.\n";
        assert_eq!(body_preview_from(body), "First sentence here.");
    }

    #[test]
    fn collapses_internal_whitespace() {
        let body = "Line one.\nLine two.\n\tLine three.\n";
        assert_eq!(body_preview_from(body), "Line one. Line two. Line three.");
    }

    #[test]
    fn truncates_with_ellipsis_at_200_chars() {
        let long = "abcdefghij".repeat(30); // 300 chars
        let preview = body_preview_from(&long);
        // 200 + the trailing ellipsis character.
        assert_eq!(preview.chars().count(), 201);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn truncation_respects_utf8_boundaries() {
        // 'é' is 2 bytes in UTF-8. A naive byte-truncate at 200 could
        // split it; `chars().take(200)` guarantees a clean boundary.
        let body = "é".repeat(300);
        let preview = body_preview_from(&body);
        // Must round-trip as valid UTF-8.
        assert!(std::str::from_utf8(preview.as_bytes()).is_ok());
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn body_with_only_headings_returns_empty() {
        let body = "# H1\n## H2\n### H3\n";
        assert_eq!(body_preview_from(body), "");
    }

    #[test]
    fn heading_after_content_terminates_the_preview() {
        // A `## Section` immediately following the first paragraph
        // ends the preview — without this rule, the second paragraph
        // would be silently spliced onto the first.
        let body = "First para.\n## Heading\nMore text.\n";
        assert_eq!(body_preview_from(body), "First para.");
    }

    #[test]
    fn body_of_exactly_200_chars_is_not_truncated() {
        // Boundary case: the truncation predicate is `> 200`, so a
        // body of exactly 200 chars passes through unchanged with no
        // ellipsis.
        let exact = "a".repeat(200);
        let preview = body_preview_from(&exact);
        assert_eq!(preview.chars().count(), 200);
        assert!(!preview.ends_with('…'));
    }

    #[test]
    fn body_of_201_chars_truncates_with_ellipsis() {
        // Boundary case on the other side of the predicate.
        let just_over = "a".repeat(201);
        let preview = body_preview_from(&just_over);
        assert_eq!(preview.chars().count(), 201); // 200 + '…'
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn joins_multi_line_first_paragraph_with_single_spaces() {
        // A paragraph spanning multiple lines (no blank line, no
        // headings) joins with single spaces. Confirms that the
        // per-line append doesn't introduce double spaces or drop
        // content, and that the blank-line break correctly stops at
        // the paragraph boundary instead of consuming "Next paragraph."
        let body = "Line one.\nLine two.\nLine three.\n\nNext paragraph.\n";
        assert_eq!(body_preview_from(body), "Line one. Line two. Line three.");
    }
}
```

Implement `body_preview_from`:

```rust
const PREVIEW_MAX_CHARS: usize = 200;

pub fn body_preview_from(body: &str) -> String {
    // Build a single-line, heading-free preview from the first
    // paragraph of the body. Blank lines and headings both terminate
    // the preview once content has been collected; before content,
    // they're skipped.
    let mut buf = String::new();
    for line in body.lines() {
        let trimmed = line.trim();
        let is_break = trimmed.is_empty() || trimmed.starts_with('#');
        if is_break {
            if !buf.is_empty() { break; }
            continue;
        }
        if !buf.is_empty() {
            buf.push(' ');
        }
        // Collapse internal whitespace runs within the line.
        let mut last_was_space = false;
        for ch in trimmed.chars() {
            if ch.is_whitespace() {
                if !last_was_space {
                    buf.push(' ');
                    last_was_space = true;
                }
            } else {
                buf.push(ch);
                last_was_space = false;
            }
        }
    }

    // Truncate on a char boundary, append ellipsis if we cut anything.
    if buf.chars().count() > PREVIEW_MAX_CHARS {
        let truncated: String = buf.chars().take(PREVIEW_MAX_CHARS).collect();
        format!("{truncated}…")
    } else {
        buf
    }
}
```

#### 2b. Add the field to `IndexEntry` and populate it

Update `IndexEntry` in `src/indexer.rs`:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub r#type: DocTypeKey,
    pub path: PathBuf,
    pub rel_path: PathBuf,
    pub slug: Option<String>,
    pub title: String,
    pub frontmatter: serde_json::Value,
    pub frontmatter_state: String,
    pub ticket: Option<String>,
    pub mtime_ms: i64,
    pub size: u64,
    pub etag: String,
    pub body_preview: String,
}
```

In `Indexer::rescan` (around `indexer.rs:76-120`), populate the new
field next to the existing `title` line:

```rust
let title = frontmatter::title_from(&parsed.state, &parsed.body, filename_stem);
let body_preview = frontmatter::body_preview_from(&parsed.body);
```

…and pass `body_preview` into the `IndexEntry { … }` literal a few lines
below.

#### 2c. Add an indexer-level test

Append to the `tests` module in `indexer.rs`:

```rust
#[tokio::test]
async fn index_entry_carries_body_preview() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::write(
        plans.join("2026-04-25-foo.md"),
        "---\ntitle: Foo\n---\n# Foo\n\nFirst paragraph of the body.\n",
    ).unwrap();

    let mut paths = std::collections::HashMap::new();
    paths.insert("plans".to_string(), plans);
    let driver: std::sync::Arc<dyn FileDriver> =
        std::sync::Arc::new(crate::file_driver::LocalFileDriver::new(&paths, vec![]));
    let idx = Indexer::build(driver, tmp.path().to_path_buf()).await.unwrap();
    let entries = idx.all().await;
    let foo = entries.iter().find(|e| e.title == "Foo").unwrap();
    assert_eq!(foo.body_preview, "First paragraph of the body.");
}
```

#### 2d. Promote `entry()` to a shared `entry_for_test` factory

Adding a field to a public struct breaks every literal `IndexEntry { … }`
constructor across the test suite. Rather than mutate every site
individually for `body_preview` (and again for the next required field
in a future phase), promote the existing `clusters.rs::tests::entry()`
helper to a shared `entry_for_test` factory in a new
`server/src/test_support.rs` module (gated by `#[cfg(test)]`).

```rust
// server/src/test_support.rs
#![cfg(test)]

use crate::doc_types::DocTypeKey;
use crate::indexer::IndexEntry;
use std::path::PathBuf;

/// Test-only `IndexEntry` factory. New required fields default here in
/// one place; callers override only what they care about.
pub fn entry_for_test(
    doc_type: DocTypeKey,
    slug: &str,
    mtime_ms: i64,
    title: &str,
) -> IndexEntry {
    IndexEntry {
        r#type: doc_type,
        path: PathBuf::from(format!("/x/{slug}.md")),
        rel_path: PathBuf::from(format!("{slug}.md")),
        slug: Some(slug.to_string()),
        title: title.to_string(),
        frontmatter: serde_json::Value::Null,
        frontmatter_state: "parsed".to_string(),
        ticket: None,
        mtime_ms,
        size: 0,
        etag: "sha256-x".to_string(),
        body_preview: String::new(),
    }
}
```

Wire it into `lib.rs` (or wherever the crate's modules are declared):

```rust
#[cfg(test)]
pub(crate) mod test_support;
```

Then migrate the in-scope tests to use it:

- `clusters.rs::tests` — replace the inline `entry()` helper with a
  thin wrapper around `test_support::entry_for_test` (or import it
  directly).
- Any `IndexEntry { … }` literal in `watcher.rs::tests` or other
  module-level test fixtures touched by Phase 6 — switch to
  `entry_for_test(...)`.

Production has exactly one literal constructor (`indexer.rs::rescan`),
which stays as-is (it has the real values).

A sweep `grep -rn 'IndexEntry {' server/src server/tests` (note both
roots) surfaces any holdouts; the compiler will refuse to build until
each is either migrated or has `body_preview` added inline. Holdouts
that can't easily migrate (e.g. tests that need to assert a specific
non-default `frontmatter`) just add `body_preview: String::new()` to
the literal — the factory is the preferred path, not a hard rule.

#### Success criteria

```bash
cd skills/visualisation/visualise/server
cargo test frontmatter::body_preview_tests
# 11 helper tests pass
cargo test indexer
# existing tests + new index_entry_carries_body_preview pass
cargo test clusters
# 10 tests pass (Step 1's 10, after `clusters.rs::tests::entry()` is
# migrated to the shared `entry_for_test` factory)
```

---

## Step 3: Server — integration test for `lastChangedMs` and `bodyPreview`

### File: `skills/visualisation/visualise/server/tests/api_lifecycle.rs`

Append a single integration test that exercises both new fields on the
wire. The shared fixture `common::seeded_cfg` already seeds
`plans/2026-04-18-foo.md` with a `# body` block — sufficient for an
assertion that `bodyPreview` is non-empty.

```rust
#[tokio::test]
async fn lifecycle_list_carries_last_changed_ms_and_body_preview() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/lifecycle")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let foo = v["clusters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["slug"] == "foo")
        .unwrap();

    // `lastChangedMs` is the camelCase wire form of `last_changed_ms`.
    let last = foo["lastChangedMs"].as_i64().expect("lastChangedMs missing");
    assert!(last > 0, "expected a positive mtime, got {last}");

    // Every entry has `bodyPreview` and the field is a string. The
    // seeded plan body is `# body\n` (only-headings), so the preview
    // contract for that case is the empty string "". Asserting
    // `as_str()` rather than `is_some()` rules out a regression that
    // emits `bodyPreview: null` (e.g. via a future Option<String> on
    // the Rust side or a `serde(skip_serializing_if = ...)` annotation).
    for entry in foo["entries"].as_array().unwrap() {
        let preview = entry["bodyPreview"]
            .as_str()
            .expect(&format!("bodyPreview should be a string: {entry}"));
        assert_eq!(
            preview, "",
            "expected empty preview for heading-only seeded body, got {preview:?}",
        );
    }
}

#[tokio::test]
async fn lifecycle_detail_carries_last_changed_ms_and_body_preview() {
    // Same wire-shape contract for the detail endpoint. Without this
    // test, a divergent serialiser on `/api/lifecycle/:slug` (e.g.
    // a custom DTO that drops a field) could silently break the
    // cluster detail view while the list endpoint stays correct.
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/lifecycle/foo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let last = v["lastChangedMs"].as_i64().expect("lastChangedMs missing");
    assert!(last > 0, "expected a positive mtime, got {last}");
    for entry in v["entries"].as_array().unwrap() {
        let preview = entry["bodyPreview"]
            .as_str()
            .expect(&format!("bodyPreview should be a string: {entry}"));
        assert_eq!(preview, "");
    }
}
```

#### Success criteria

```bash
cargo test --tests --features dev-frontend api_lifecycle
# all 5 tests pass (existing 3 + 2 new wire-shape tests)
```

---

## Step 4: Frontend — lifecycle types and `bodyPreview` on `IndexEntry`

### File: `skills/visualisation/visualise/frontend/src/api/types.ts`

First, extend the existing `IndexEntry` interface with the new field:

```typescript
export interface IndexEntry {
  type: DocTypeKey
  path: string
  relPath: string
  slug: string | null
  title: string
  frontmatter: Record<string, unknown>
  frontmatterState: 'parsed' | 'absent' | 'malformed'
  ticket: string | null
  mtimeMs: number
  size: number
  etag: string
  bodyPreview: string  // NEW
}
```

The server always emits `bodyPreview` (empty string when no preview is
available), so the field is required, not optional.

Rather than mutate every TS test fixture by hand for `bodyPreview`
(and again for the next required field), introduce a small factory:

### File: `skills/visualisation/visualise/frontend/src/api/test-fixtures.ts` (new)

```typescript
import type { IndexEntry } from './types'

/** Test-only `IndexEntry` factory. New required fields default here
 *  in one place; callers override only what they care about via
 *  `Partial<IndexEntry>`. */
export function makeIndexEntry(overrides: Partial<IndexEntry> = {}): IndexEntry {
  return {
    type: 'plans',
    path: '/x/foo.md',
    relPath: 'foo.md',
    slug: 'foo',
    title: 'Foo',
    frontmatter: {},
    frontmatterState: 'parsed',
    ticket: null,
    mtimeMs: 0,
    size: 0,
    etag: 'sha256-x',
    bodyPreview: '',
    ...overrides,
  }
}
```

Migrate the in-scope test fixtures to use `makeIndexEntry({ ... })`
(the `LifecycleClusterView.test.tsx` `entry(...)` helper below is
already a thin local factory — it can simply call `makeIndexEntry`
under the hood). Tests under `LibraryTypeView.test.tsx` and elsewhere
that construct `IndexEntry` literals can either migrate to
`makeIndexEntry` or add `bodyPreview: ''` inline; the TypeScript
compiler will refuse to build until each fixture is satisfied.

Then append:

```typescript
/** Completeness flags. Eight workflow stages plus a `hasNotes`
 *  long-tail flag. `hasNotes` is required to match the server's
 *  `Completeness` struct, which always emits it. The pipeline-of-dots
 *  indicator on the index card renders only the eight workflow stages
 *  (see `WORKFLOW_PIPELINE_STEPS`); the timeline renders Notes in a
 *  visually distinct long-tail section below the main pipeline. */
export interface Completeness {
  hasTicket: boolean
  hasResearch: boolean
  hasPlan: boolean
  hasPlanReview: boolean
  hasValidation: boolean
  hasPr: boolean
  hasPrReview: boolean
  hasDecision: boolean
  hasNotes: boolean
}

export interface LifecycleCluster {
  slug: string
  title: string
  entries: IndexEntry[]
  completeness: Completeness
  lastChangedMs: number
}

export interface LifecycleListResponse {
  clusters: LifecycleCluster[]
}

/** Pipeline stages, in canonical order. The first eight are workflow
 *  stages; `Notes` is a long-tail stage (`longTail: true`) rendered
 *  below the main pipeline rather than inline with the workflow.
 *  `key` matches the corresponding `Completeness` field; `docType` is
 *  the matching `DocTypeKey` used to filter cluster entries; `label`
 *  is the user-visible string. Single source of truth so adding a
 *  stage is a one-line change.
 *
 *  `key` is narrowed to a literal union (rather than `keyof Completeness`)
 *  so the `as const` 9-element tuple is preserved at the type level
 *  and a future contributor can't append a stage with a key that
 *  isn't a real `Completeness` field. */
type PipelineStepKey =
  | 'hasTicket' | 'hasResearch' | 'hasPlan' | 'hasPlanReview'
  | 'hasValidation' | 'hasPr' | 'hasPrReview' | 'hasDecision'
  | 'hasNotes'

export const LIFECYCLE_PIPELINE_STEPS: ReadonlyArray<{
  key: PipelineStepKey
  docType: DocTypeKey
  label: string
  /** Copy used by the timeline when the stage has no entries. Authored
   *  per stage rather than derived from `label.toLowerCase()` so
   *  acronyms like "PR" read naturally ("no PR yet", not "no pr yet")
   *  for both sighted readers and screen-reader synthesisers. */
  placeholder: string
  longTail?: boolean
}> = [
  { key: 'hasTicket',     docType: 'tickets',      label: 'Ticket',      placeholder: 'no ticket yet' },
  { key: 'hasResearch',   docType: 'research',     label: 'Research',    placeholder: 'no research yet' },
  { key: 'hasPlan',       docType: 'plans',        label: 'Plan',        placeholder: 'no plan yet' },
  { key: 'hasPlanReview', docType: 'plan-reviews', label: 'Plan review', placeholder: 'no plan review yet' },
  { key: 'hasValidation', docType: 'validations',  label: 'Validation',  placeholder: 'no validation yet' },
  { key: 'hasPr',         docType: 'prs',          label: 'PR',          placeholder: 'no PR yet' },
  { key: 'hasPrReview',   docType: 'pr-reviews',   label: 'PR review',   placeholder: 'no PR review yet' },
  { key: 'hasDecision',   docType: 'decisions',    label: 'Decision',    placeholder: 'no decision yet' },
  { key: 'hasNotes',      docType: 'notes',        label: 'Notes',       placeholder: 'no notes yet',       longTail: true },
] as const

/** The eight workflow stages — long-tail stages (Notes) excluded.
 *  Used by `PipelineDots` (the indicator only counts workflow) and by
 *  the completeness sort, so adding a future long-tail stage doesn't
 *  silently inflate the "N of 8 stages" counter. */
export const WORKFLOW_PIPELINE_STEPS = LIFECYCLE_PIPELINE_STEPS.filter(
  s => !s.longTail,
)

/** Long-tail stages — currently just Notes. Rendered below the main
 *  workflow timeline in `LifecycleClusterView`'s "Other" section. */
export const LONG_TAIL_PIPELINE_STEPS = LIFECYCLE_PIPELINE_STEPS.filter(
  s => s.longTail,
)
```

No tests for plain interface declarations; the constant is exercised by
the `PipelineDots` and `LifecycleIndex` tests later.

---

## Step 5: Frontend — `fetchLifecycleClusters` and `fetchLifecycleCluster` (TDD)

### File: `skills/visualisation/visualise/frontend/src/api/fetch.test.ts`

Append:

```typescript
import { fetchLifecycleClusters, fetchLifecycleCluster } from './fetch'

describe('fetchLifecycleClusters', () => {
  it('unwraps the `clusters` field from the response envelope', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        clusters: [
          {
            slug: 'foo',
            title: 'Foo',
            entries: [],
            completeness: {
              hasTicket: false, hasResearch: false, hasPlan: true,
              hasPlanReview: false, hasValidation: false, hasPr: false,
              hasPrReview: false, hasDecision: false, hasNotes: false,
            },
            lastChangedMs: 1_700_000_000_000,
          },
        ],
      }),
    })
    const clusters = await fetchLifecycleClusters()
    expect(clusters).toHaveLength(1)
    expect(clusters[0].slug).toBe('foo')
    expect(clusters[0].lastChangedMs).toBe(1_700_000_000_000)
  })

  it('throws on non-200', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 500 })
    await expect(fetchLifecycleClusters()).rejects.toThrow('500')
  })
})

describe('fetchLifecycleCluster', () => {
  it('returns the single-cluster payload directly', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        slug: 'foo', title: 'Foo', entries: [],
        completeness: {
          hasTicket: false, hasResearch: false, hasPlan: false,
          hasPlanReview: false, hasValidation: false, hasPr: false,
          hasPrReview: false, hasDecision: false, hasNotes: false,
        },
        lastChangedMs: 0,
      }),
    })
    const cluster = await fetchLifecycleCluster('foo')
    expect(cluster.slug).toBe('foo')
  })

  it('url-encodes the slug', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        slug: 'foo bar', title: '', entries: [],
        completeness: {
          hasTicket: false, hasResearch: false, hasPlan: false,
          hasPlanReview: false, hasValidation: false, hasPr: false,
          hasPrReview: false, hasDecision: false, hasNotes: false,
        },
        lastChangedMs: 0,
      }),
    })
    await fetchLifecycleCluster('foo bar')
    expect(mockFetch).toHaveBeenCalledWith('/api/lifecycle/foo%20bar')
  })

  it('throws on 404', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 404 })
    await expect(fetchLifecycleCluster('missing')).rejects.toThrow('404')
  })
})
```

### File: `skills/visualisation/visualise/frontend/src/api/fetch.ts`

Add a `FetchError` class and append two helpers (also update the
imports). The class lets the cluster-detail view branch on 404 vs 5xx
without parsing free-form error strings — addressing the standing
"stringly-typed error" smell that other helpers in this file exhibit.
Existing helpers can be migrated opportunistically; the new ones
adopt it from day one.

```typescript
import type {
  DocType, DocTypeKey, DocsListResponse, IndexEntry,
  TemplateSummaryListResponse, TemplateDetail,
  LifecycleCluster, LifecycleListResponse,
} from './types'

/** Typed error thrown by fetch helpers on non-2xx responses, so
 *  callers can branch on `err instanceof FetchError && err.status === 404`
 *  rather than substring-matching the message. The message preserves
 *  the existing `'GET <path>: <status>'` shape so existing tests that
 *  match against the status code in the message keep passing. */
export class FetchError extends Error {
  constructor(public readonly status: number, message: string) {
    super(message)
    this.name = 'FetchError'
  }
}

// ... existing helpers unchanged ...

export async function fetchLifecycleClusters(): Promise<LifecycleCluster[]> {
  const r = await fetch('/api/lifecycle')
  if (!r.ok) throw new FetchError(r.status, `GET /api/lifecycle: ${r.status}`)
  const body: LifecycleListResponse = await r.json()
  return body.clusters
}

export async function fetchLifecycleCluster(slug: string): Promise<LifecycleCluster> {
  const r = await fetch(`/api/lifecycle/${encodeURIComponent(slug)}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/lifecycle/${slug}: ${r.status}`)
  return r.json()
}
```

#### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm run test -- fetch
# fetch.test.ts adds 6 new tests
```

---

## Step 6: Frontend — invalidate `lifecycle-cluster` queries on SSE events (TDD)

### File: `skills/visualisation/visualise/frontend/src/api/query-keys.ts`

First, expose the cluster-prefix as a factory entry so the SSE
dispatcher and `lifecycleCluster(slug)` consume the same source of
truth — preventing a future rename of the literal `'lifecycle-cluster'`
from silently breaking invalidation:

```typescript
export const queryKeys = {
  // ...existing entries...
  lifecycleClusterPrefix: () => ['lifecycle-cluster'] as const,
  lifecycleCluster: (slug: string) =>
    [...queryKeys.lifecycleClusterPrefix(), slug] as const,
}
```

(If `lifecycleCluster` is already defined as a literal-tuple factory in
`query-keys.ts`, refactor it to delegate to `lifecycleClusterPrefix()`
so the prefix is declared exactly once.)

### File: `skills/visualisation/visualise/frontend/src/api/use-doc-events.test.ts`

Append a new dispatch test:

```typescript
it('invalidates the lifecycle-cluster prefix on doc-changed event', () => {
  // Seed two populated cluster queries so the prefix invalidation can be
  // observed by `isInvalidated` flipping. Locks in the broad-prefix
  // invalidation strategy: SSE events do not carry slugs, so the dispatcher
  // invalidates every open cluster detail view rather than trying to
  // re-derive slugs from paths client-side.
  queryClient.setQueryData(queryKeys.lifecycleCluster('foo'), null)
  queryClient.setQueryData(queryKeys.lifecycleCluster('bar'), null)

  dispatchSseEvent(
    { type: 'doc-changed', docType: 'plans', path: 'meta/plans/x.md', etag: 'sha256-x' },
    queryClient,
  )

  expect(queryClient.getQueryState(queryKeys.lifecycleCluster('foo'))?.isInvalidated).toBe(true)
  expect(queryClient.getQueryState(queryKeys.lifecycleCluster('bar'))?.isInvalidated).toBe(true)
})

it('also invalidates the lifecycle-cluster prefix on doc-invalid event', () => {
  // The dispatcher triggers on either `doc-changed` OR `doc-invalid`,
  // so a malformed-frontmatter save must also refresh open detail
  // views. Without this assertion, a regression that gates lifecycle
  // invalidation behind only `doc-changed` would silently leave stale
  // data on screen after a `doc-invalid` event.
  //
  // Match the `doc-invalid` payload shape to whatever the existing
  // event-type definition uses — the other tests in this file already
  // dispatch `doc-invalid`, so copy the shape from there.
  queryClient.setQueryData(queryKeys.lifecycleCluster('foo'), null)

  dispatchSseEvent(
    { type: 'doc-invalid', docType: 'plans', path: 'meta/plans/x.md', error: 'malformed frontmatter' },
    queryClient,
  )

  expect(
    queryClient.getQueryState(queryKeys.lifecycleCluster('foo'))?.isInvalidated,
  ).toBe(true)
})
```

### File: `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts`

Add one line inside the `dispatchSseEvent` body, right after the existing
`lifecycle()` invalidation:

```typescript
void queryClient.invalidateQueries({ queryKey: queryKeys.lifecycle() })
// Invalidate every open cluster detail. The SSE event does not carry a
// slug; rather than re-deriving slugs client-side (duplicating server
// logic), we invalidate the whole prefix and let TanStack Query refetch
// only those views that are actually mounted. The prefix is sourced
// from `queryKeys` so it stays in lockstep with `lifecycleCluster(slug)`.
void queryClient.invalidateQueries({
  queryKey: queryKeys.lifecycleClusterPrefix(),
})
```

#### Success criteria

```bash
npm run test -- use-doc-events
# use-doc-events.test.ts adds 2 new tests
```

---

## Step 7: Frontend — `PipelineDots` component (TDD)

### File: `skills/visualisation/visualise/frontend/src/components/PipelineDots/PipelineDots.test.tsx`

Write the test first:

```typescript
import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PipelineDots } from './PipelineDots'
import type { Completeness } from '../../api/types'

const empty: Completeness = {
  hasTicket: false, hasResearch: false, hasPlan: false,
  hasPlanReview: false, hasValidation: false, hasPr: false,
  hasPrReview: false, hasDecision: false, hasNotes: false,
}

describe('PipelineDots', () => {
  it('renders all 8 pipeline stage dots', () => {
    render(<PipelineDots completeness={empty} />)
    // Query via the list semantic rather than `[data-stage]` — keeps
    // the test independent of the chosen DOM markup. `data-*`
    // attributes remain on the rendered output for styling but
    // aren't part of the test contract.
    expect(screen.getAllByRole('listitem')).toHaveLength(8)
  })

  it('marks present stages as filled and absent as unfilled via data-present', () => {
    const c: Completeness = { ...empty, hasTicket: true, hasPlan: true }
    const { container } = render(<PipelineDots completeness={c} />)
    const ticket = container.querySelector('[data-stage="hasTicket"]')!
    const plan = container.querySelector('[data-stage="hasPlan"]')!
    const research = container.querySelector('[data-stage="hasResearch"]')!
    expect(ticket.getAttribute('data-present')).toBe('true')
    expect(plan.getAttribute('data-present')).toBe('true')
    expect(research.getAttribute('data-present')).toBe('false')
  })

  it('exposes each stage label via accessible title', () => {
    const c: Completeness = { ...empty, hasPlan: true }
    render(<PipelineDots completeness={c} />)
    // `title` attribute surfaces the stage label on hover for sighted
    // users.
    expect(screen.getByTitle(/^Plan$/)).toBeInTheDocument()
    expect(screen.getByTitle(/^Plan review$/)).toBeInTheDocument()
  })

  it('exposes presence state via aria-label per dot', () => {
    // Locks in the WCAG 1.4.1 fix: the present/absent state must be
    // communicated to screen readers (which don't reliably announce
    // `title` on non-interactive list items), not via colour alone.
    const c: Completeness = { ...empty, hasPlan: true }
    render(<PipelineDots completeness={c} />)
    expect(screen.getByLabelText('Plan: present')).toBeInTheDocument()
    expect(screen.getByLabelText('Ticket: missing')).toBeInTheDocument()
  })
})
```

### File: `skills/visualisation/visualise/frontend/src/components/PipelineDots/PipelineDots.tsx`

```typescript
import type { Completeness } from '../../api/types'
import { WORKFLOW_PIPELINE_STEPS } from '../../api/types'
import styles from './PipelineDots.module.css'

interface Props {
  completeness: Completeness
}

export function PipelineDots({ completeness }: Props) {
  // `<ul>` rather than `<ol>` — the dots are not a sequence the user
  // can navigate; the per-dot `aria-label` carries the meaningful
  // information for screen readers, and dropping the ordered-list
  // semantic avoids implying a navigable order.
  return (
    <ul className={styles.pipeline} aria-label="Lifecycle pipeline">
      {WORKFLOW_PIPELINE_STEPS.map(step => {
        const present = Boolean(completeness[step.key])
        return (
          <li
            key={step.key}
            data-stage={step.key}
            data-present={present}
            // `title` is for sighted hover discoverability; screen
            // readers don't reliably announce `title` on non-interactive
            // list items, so `aria-label` carries the state explicitly.
            title={step.label}
            aria-label={`${step.label}: ${present ? 'present' : 'missing'}`}
            className={`${styles.dot} ${present ? styles.present : styles.absent}`}
          />
        )
      })}
    </ul>
  )
}
```

### File: `skills/visualisation/visualise/frontend/src/components/PipelineDots/PipelineDots.module.css`

```css
.pipeline {
  display: inline-flex;
  gap: 6px;
  list-style: none;
  margin: 0;
  padding: 0;
  align-items: center;
}

.dot {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 1.5px solid #d1d5db;
  background: transparent;
  position: relative;
}

/* Non-colour signals supplement the blue/grey distinction so the
 * indicator remains comprehensible under WCAG 1.4.1 (Use of Color):
 * present dots have a filled background plus a solid border and an
 * inner white dot; absent dots stay hollow with a dashed border. */
.present {
  background: #2563eb;
  border-color: #1d4ed8;
  border-style: solid;
}

.present::after {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: #ffffff;
  transform: translate(-50%, -50%);
}

.absent {
  background: transparent;
  border-color: #d1d5db;
  border-style: dashed;
}
```

#### Success criteria

```bash
npm run test -- PipelineDots
# 4 tests pass
```

---

## Step 8: Frontend — `LifecycleIndex` view (TDD)

### File: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.test.tsx`

Mirror the testing pattern from `LibraryTypeView.test.tsx`. Stub the
fetch helper, render with a `QueryClient` wrapper, and exercise sorts +
empty / loading / error branches.

```typescript
import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent, within } from '@testing-library/react'
import React from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from '../../components/Sidebar/test-helpers'
import { LifecycleIndex } from './LifecycleIndex'
import * as fetchModule from '../../api/fetch'
import type { LifecycleCluster, Completeness } from '../../api/types'

const empty: Completeness = {
  hasTicket: false, hasResearch: false, hasPlan: false,
  hasPlanReview: false, hasValidation: false, hasPr: false,
  hasPrReview: false, hasDecision: false, hasNotes: false,
}

const clusters: LifecycleCluster[] = [
  {
    slug: 'older',
    title: 'Older Cluster',
    entries: [],
    completeness: { ...empty, hasPlan: true },
    lastChangedMs: 1_700_000_000_000,
  },
  {
    slug: 'newer',
    title: 'Newer Cluster',
    entries: [],
    completeness: { ...empty, hasPlan: true, hasPlanReview: true, hasDecision: true },
    lastChangedMs: 1_700_500_000_000,
  },
]

function Wrapper({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

describe('LifecycleIndex', () => {
  it('renders a card per cluster with title and slug', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    expect(await screen.findByText('Older Cluster')).toBeInTheDocument()
    expect(screen.getByText('Newer Cluster')).toBeInTheDocument()
  })

  it('orders by most-recently-changed by default', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    const titles = screen.getAllByRole('heading', { level: 3 }).map(h => h.textContent)
    expect(titles[0]).toBe('Newer Cluster')
    expect(titles[1]).toBe('Older Cluster')
  })

  it('sorts by completeness when "Completeness" sort is chosen', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    fireEvent.click(screen.getByRole('button', { name: /completeness/i }))
    const titles = screen.getAllByRole('heading', { level: 3 }).map(h => h.textContent)
    // 'newer' has 3 trues, 'older' has 1 → 'newer' first.
    expect(titles[0]).toBe('Newer Cluster')
    expect(titles[1]).toBe('Older Cluster')
  })

  it('breaks completeness ties by most-recently-changed', async () => {
    // Locks in the secondary sort key: when two clusters have the
    // same completeness score, recency descending breaks the tie.
    // Without this test, removing the `b.lastChangedMs - a.lastChangedMs`
    // tiebreak would silently change user-visible order.
    const tied: LifecycleCluster[] = [
      {
        slug: 'older-equal',
        title: 'Older Equal',
        entries: [],
        completeness: { ...empty, hasPlan: true, hasDecision: true },
        lastChangedMs: 1_700_000_000_000,
      },
      {
        slug: 'newer-equal',
        title: 'Newer Equal',
        entries: [],
        completeness: { ...empty, hasPlan: true, hasDecision: true },
        lastChangedMs: 1_700_500_000_000,
      },
    ]
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(tied)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Equal')
    fireEvent.click(screen.getByRole('button', { name: /completeness/i }))
    const titles = screen.getAllByRole('heading', { level: 3 }).map(h => h.textContent)
    // Both clusters have score 2 → recency breaks the tie.
    expect(titles[0]).toBe('Newer Equal')
    expect(titles[1]).toBe('Older Equal')
  })

  it('sorts by oldest when "Oldest" sort is chosen', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    fireEvent.click(screen.getByRole('button', { name: /oldest/i }))
    const titles = screen.getAllByRole('heading', { level: 3 }).map(h => h.textContent)
    expect(titles[0]).toBe('Older Cluster')
    expect(titles[1]).toBe('Newer Cluster')
  })

  it('renders 8 pipeline dots per card', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    // Each card hosts a `<PipelineDots>` `<ul aria-label="Lifecycle pipeline">`.
    // Asserting the dots via the list semantic decouples the test
    // from the `data-stage` attribute.
    const pipelines = screen.getAllByRole('list', { name: /lifecycle pipeline/i })
    expect(pipelines).toHaveLength(2)
    pipelines.forEach(p =>
      expect(within(p).getAllByRole('listitem')).toHaveLength(8),
    )
  })

  it('shows empty state when no clusters', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue([])
    render(<LifecycleIndex />, { wrapper: Wrapper })
    expect(await screen.findByText(/no lifecycle clusters found/i)).toBeInTheDocument()
  })

  it('shows loading state while fetching', () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LifecycleIndex />, { wrapper: Wrapper })
    expect(screen.getByText(/loading/i)).toBeInTheDocument()
  })

  it('shows a generic alert on FetchError without leaking the URL', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockRejectedValue(
      new fetchModule.FetchError(500, 'GET /api/lifecycle: 500'),
    )
    render(<LifecycleIndex />, { wrapper: Wrapper })
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(/could not load lifecycle clusters/i)
    // Internal API path must not leak through to end-users.
    expect(alert.textContent).not.toMatch(/\/api\//)
  })

  it('shows a generic alert on non-FetchError rejections', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockRejectedValue(new Error('boom'))
    render(<LifecycleIndex />, { wrapper: Wrapper })
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(/something went wrong/i)
    // Even when the upstream throws a plain Error with arbitrary text,
    // its `.message` must not flow through to user-facing copy.
    expect(alert.textContent).not.toMatch(/boom/)
  })

  it('filters clusters by title or slug substring (case-insensitive)', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')

    const input = screen.getByRole('searchbox', { name: /filter clusters/i })

    // Title match (case-insensitive).
    fireEvent.change(input, { target: { value: 'NEWER' } })
    expect(screen.getByText('Newer Cluster')).toBeInTheDocument()
    expect(screen.queryByText('Older Cluster')).not.toBeInTheDocument()

    // Slug match.
    fireEvent.change(input, { target: { value: 'older' } })
    expect(screen.getByText('Older Cluster')).toBeInTheDocument()
    expect(screen.queryByText('Newer Cluster')).not.toBeInTheDocument()

    // Empty filter is a no-op — both clusters return.
    fireEvent.change(input, { target: { value: '' } })
    expect(screen.getByText('Newer Cluster')).toBeInTheDocument()
    expect(screen.getByText('Older Cluster')).toBeInTheDocument()
  })

  it('shows a no-match message when the filter excludes every cluster', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    fireEvent.change(
      screen.getByRole('searchbox', { name: /filter clusters/i }),
      { target: { value: 'zzz-no-match' } },
    )
    expect(screen.getByText(/no clusters match "zzz-no-match"/i)).toBeInTheDocument()
  })
})
```

### File: `skills/visualisation/visualise/frontend/src/api/format.ts` (new)

Extracted from the inline helper that was previously planned inside
`LifecycleIndex.tsx`. Pulled into a shared module so both the
lifecycle index and `LibraryTypeView` consume one implementation
(see migration note below). `now` is injectable so future tests can
freeze the clock without `vi.useFakeTimers`. The ladder extends past
24h via "Nd ago" / "Nw ago" thresholds before falling back to a
short date-only `toLocaleDateString()`, which avoids the jarring jump
from "23h ago" to a full localised timestamp.

```typescript
export function formatMtime(ms: number, now: number = Date.now()): string {
  if (ms <= 0) return '—'
  const diffSec = Math.floor((now - ms) / 1000)
  // Guard against future timestamps (clock skew between server and
  // client, or a freshly-written file whose mtime resolves a few
  // seconds ahead of the browser's clock). Without this, the
  // ladder's first branch would render literal "-3s ago" strings.
  if (diffSec < 0)          return 'just now'
  if (diffSec < 60)         return `${diffSec}s ago`
  if (diffSec < 3600)       return `${Math.floor(diffSec / 60)}m ago`
  if (diffSec < 86400)      return `${Math.floor(diffSec / 3600)}h ago`
  if (diffSec < 7 * 86400)  return `${Math.floor(diffSec / 86400)}d ago`
  if (diffSec < 30 * 86400) return `${Math.floor(diffSec / (7 * 86400))}w ago`
  return new Date(ms).toLocaleDateString()
}
```

### File: `skills/visualisation/visualise/frontend/src/api/format.test.ts` (new)

A small helper-level test pinning the ladder boundaries; injecting
`now` lets every assertion be deterministic without timer-mocking.

```typescript
import { describe, it, expect } from 'vitest'
import { formatMtime } from './format'

describe('formatMtime', () => {
  const NOW = 1_700_000_000_000

  it('returns em-dash for zero or negative input', () => {
    expect(formatMtime(0,   NOW)).toBe('—')
    expect(formatMtime(-1,  NOW)).toBe('—')
  })

  it('returns "just now" for future timestamps (clock skew)', () => {
    // Pinned so a future contributor cannot reintroduce the
    // "-3s ago" leak by removing the < 0 guard.
    expect(formatMtime(NOW + 5_000, NOW)).toBe('just now')
  })

  it('uses seconds, minutes, hours under a day', () => {
    expect(formatMtime(NOW - 30  * 1000, NOW)).toBe('30s ago')
    expect(formatMtime(NOW - 5   * 60_000, NOW)).toBe('5m ago')
    expect(formatMtime(NOW - 3   * 3_600_000, NOW)).toBe('3h ago')
  })

  it('uses days under a week and weeks under a month', () => {
    expect(formatMtime(NOW - 2  * 86_400_000, NOW)).toBe('2d ago')
    expect(formatMtime(NOW - 10 * 86_400_000, NOW)).toBe('1w ago')
    expect(formatMtime(NOW - 21 * 86_400_000, NOW)).toBe('3w ago')
  })

  it('falls back to a date string past 30 days', () => {
    const result = formatMtime(NOW - 60 * 86_400_000, NOW)
    // Locale-dependent but always a date-only string; assert no
    // colon (which would indicate a time component leaked through).
    expect(result).not.toMatch(/:/)
    expect(result).not.toMatch(/ago$/)
  })
})
```

### Migrate `LibraryTypeView.tsx` to the shared `formatMtime`

`frontend/src/routes/library/LibraryTypeView.tsx` has its own
`formatMtime` defined inline. Replace it with an import from
`'../../api/format'` so both the index and the library view share
one ladder. The new ladder extends past 24h (was a full
`toLocaleString()` past that threshold) — this is a deliberate
user-visible improvement and the `LibraryTypeView` tests should
continue to pass since they don't currently assert specific
timestamp text. If any do, update them to the new shape.

### File: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.tsx`

```typescript
import { useMemo, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchLifecycleClusters, FetchError } from '../../api/fetch'
import { formatMtime } from '../../api/format'
import { queryKeys } from '../../api/query-keys'
import { WORKFLOW_PIPELINE_STEPS, type LifecycleCluster } from '../../api/types'
import { PipelineDots } from '../../components/PipelineDots/PipelineDots'
import styles from './LifecycleIndex.module.css'

type SortMode = 'recent' | 'oldest' | 'completeness'

/** Count true booleans across the eight workflow stages. Long-tail
 *  stages (Notes) are excluded — including them would flatten the
 *  completeness signal that drives the sort. */
function completenessScore(c: LifecycleCluster): number {
  return WORKFLOW_PIPELINE_STEPS.reduce(
    (n, step) => (c.completeness[step.key] ? n + 1 : n),
    0,
  )
}

function sortClusters(clusters: LifecycleCluster[], mode: SortMode): LifecycleCluster[] {
  const sorted = [...clusters]
  if (mode === 'recent') {
    sorted.sort((a, b) => b.lastChangedMs - a.lastChangedMs)
  } else if (mode === 'oldest') {
    sorted.sort((a, b) => a.lastChangedMs - b.lastChangedMs)
  } else {
    // completeness: higher score first; mtime breaks ties so equally-complete
    // clusters surface in recency order.
    sorted.sort((a, b) => {
      const diff = completenessScore(b) - completenessScore(a)
      return diff !== 0 ? diff : b.lastChangedMs - a.lastChangedMs
    })
  }
  return sorted
}

/** Case-insensitive substring match on title OR slug. Empty filter is
 *  a no-op so the unfiltered list is returned as-is. */
function filterClusters(clusters: LifecycleCluster[], filter: string): LifecycleCluster[] {
  const needle = filter.trim().toLowerCase()
  if (!needle) return clusters
  return clusters.filter(c =>
    c.title.toLowerCase().includes(needle) ||
    c.slug.toLowerCase().includes(needle),
  )
}

export function LifecycleIndex() {
  const [sortMode, setSortMode] = useState<SortMode>('recent')
  const [filter, setFilter] = useState<string>('')

  const { data: clusters = [], isLoading, isError, error } = useQuery({
    queryKey: queryKeys.lifecycle(),
    queryFn: fetchLifecycleClusters,
  })

  // Memoise BEFORE conditional early returns — Rules of Hooks. Filter
  // first, then sort, so the substring match is applied to the full
  // list and the user-visible order is stable per filter+sort pair.
  const visible = useMemo(
    () => sortClusters(filterClusters(clusters, filter), sortMode),
    [clusters, filter, sortMode],
  )

  if (isLoading) return <p>Loading…</p>
  if (isError) {
    // Mirror the cluster-detail treatment from `LifecycleClusterContent`:
    // do not surface raw URLs, status codes, or developer-formatted
    // error strings to end-users. The typed `FetchError` lets us write
    // hand-authored copy; anything else falls through to a generic
    // message rather than `error.message`.
    return (
      <p role="alert" className={styles.error}>
        {error instanceof FetchError
          ? 'Could not load lifecycle clusters. Try again later.'
          : 'Something went wrong loading lifecycle clusters. Try again later.'}
      </p>
    )
  }
  if (clusters.length === 0) {
    return <p className={styles.empty}>No lifecycle clusters found.</p>
  }

  return (
    <div className={styles.container}>
      <div className={styles.toolbar}>
        <input
          type="search"
          aria-label="Filter clusters"
          placeholder="Filter…"
          className={styles.filterInput}
          value={filter}
          onChange={e => setFilter(e.target.value)}
        />
        <span className={styles.toolbarLabel}>Sort:</span>
        <SortButton current={sortMode} value="recent"       label="Recent"       onChange={setSortMode} />
        <SortButton current={sortMode} value="oldest"       label="Oldest"       onChange={setSortMode} />
        <SortButton current={sortMode} value="completeness" label="Completeness" onChange={setSortMode} />
      </div>

      {visible.length === 0 && (
        // `role="status"` makes this a polite live region so screen
        // readers announce when typing into the filter empties the
        // result list. Without it, focus stays in the input and
        // there is no auditory feedback about the empty state.
        <p role="status" className={styles.empty}>
          No clusters match "{filter}".
        </p>
      )}

      <ul className={styles.cardList}>
        {visible.map(cluster => {
          const score = completenessScore(cluster)
          return (
            <li key={cluster.slug} className={styles.card}>
              <Link
                to="/lifecycle/$slug"
                params={{ slug: cluster.slug }}
                className={styles.cardLink}
              >
                <div className={styles.cardHeader}>
                  <h3 className={styles.cardTitle}>{cluster.title}</h3>
                  <span className={styles.cardSlug}>{cluster.slug}</span>
                </div>
                <PipelineDots completeness={cluster.completeness} />
                <div className={styles.cardMeta}>
                  <span>{score} of {WORKFLOW_PIPELINE_STEPS.length} stages</span>
                  <span>{formatMtime(cluster.lastChangedMs)}</span>
                </div>
              </Link>
            </li>
          )
        })}
      </ul>
    </div>
  )
}

function SortButton({
  current, value, label, onChange,
}: {
  current: SortMode
  value: SortMode
  label: string
  onChange: (m: SortMode) => void
}) {
  const active = current === value
  return (
    <button
      type="button"
      className={`${styles.sortButton} ${active ? styles.sortButtonActive : ''}`}
      aria-pressed={active}
      onClick={() => onChange(value)}
    >
      {label}
    </button>
  )
}

// `formatMtime` is imported from `../../api/format`.
```

### File: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.module.css`

```css
.container { max-width: 900px; }

.toolbar {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.filterInput {
  flex: 0 1 220px;
  font: inherit;
  font-size: 0.85rem;
  padding: 0.3rem 0.6rem;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  margin-right: 0.5rem;
}

.filterInput:focus {
  outline: 2px solid #1d4ed8;
  outline-offset: -1px;
  border-color: #1d4ed8;
}

.toolbarLabel {
  font-size: 0.8rem;
  color: #6b7280;
  margin-right: 4px;
}

.sortButton {
  all: unset;
  font: inherit;
  font-size: 0.85rem;
  padding: 0.3rem 0.7rem;
  border: 1px solid #d1d5db;
  border-radius: 9999px;
  background: #ffffff;
  color: #374151;
  cursor: pointer;
}

.sortButton:hover,
.sortButton:focus-visible {
  border-color: #1d4ed8;
  color: #1d4ed8;
  outline: 2px solid #1d4ed8;
  outline-offset: -2px;
}

.sortButtonActive {
  background: #dbeafe;
  border-color: #1d4ed8;
  color: #1d4ed8;
  font-weight: 500;
}

.cardList {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 1rem;
}

.card {
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  background: #ffffff;
  transition: border-color 120ms, box-shadow 120ms;
}

.card:hover,
.card:focus-within {
  border-color: #1d4ed8;
  box-shadow: 0 1px 4px rgba(29, 78, 216, 0.12);
}

.card:hover .cardTitle,
.card:focus-within .cardTitle { color: #1d4ed8; }

.cardLink {
  display: block;
  padding: 1rem;
  text-decoration: none;
  color: inherit;
}

.cardHeader { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 0.6rem; }
.cardTitle { font-size: 1rem; font-weight: 600; margin: 0; color: #111827; }
.cardSlug { font-size: 0.75rem; color: #9ca3af; font-family: monospace; }

.cardMeta {
  display: flex;
  justify-content: space-between;
  margin-top: 0.6rem;
  font-size: 0.75rem;
  color: #6b7280;
}

.empty { color: #6b7280; }
.error {
  color: #991b1b; background: #fef2f2; border: 1px solid #fecaca;
  border-radius: 4px; padding: 0.5rem 0.75rem;
}
```

#### Success criteria

```bash
npm run test -- LifecycleIndex
# 12 tests pass
```

---

## Step 9: Frontend — `LifecycleClusterView` detail view (TDD)

### File: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx`

```typescript
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import React from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from '../../components/Sidebar/test-helpers'
import { LifecycleClusterContent } from './LifecycleClusterView'
import * as fetchModule from '../../api/fetch'
import { makeIndexEntry } from '../../api/test-fixtures'
import type { LifecycleCluster, Completeness, IndexEntry } from '../../api/types'

const empty: Completeness = {
  hasTicket: false, hasResearch: false, hasPlan: false,
  hasPlanReview: false, hasValidation: false, hasPr: false,
  hasPrReview: false, hasDecision: false, hasNotes: false,
}

// Test-local convenience wrapper around the shared `makeIndexEntry`
// factory; carries only the fields specific to this suite. Future
// required fields on `IndexEntry` default in `makeIndexEntry`, so
// this helper does not need to change.
function entry(
  type: IndexEntry['type'],
  rel: string,
  title: string,
  mtime: number,
  bodyPreview = '',
): IndexEntry {
  return makeIndexEntry({
    type,
    path: `/x/${rel}`,
    relPath: rel,
    title,
    frontmatter: { date: '2026-04-18' },
    mtimeMs: mtime,
    bodyPreview,
  })
}

const cluster: LifecycleCluster = {
  slug: 'foo', title: 'Foo Cluster',
  entries: [
    entry(
      'plans', 'meta/plans/2026-04-18-foo.md', 'The Foo Plan', 100,
      'A short summary of what the plan covers.',
    ),
    entry('decisions', 'meta/decisions/ADR-0007-foo.md', 'ADR Foo', 200),
  ],
  completeness: { ...empty, hasPlan: true, hasDecision: true },
  lastChangedMs: 200,
}

function Wrapper({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

describe('LifecycleClusterContent', () => {
  it('renders the cluster title and slug', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('heading', { name: 'Foo Cluster' })).toBeInTheDocument()
    expect(screen.getByText('foo')).toBeInTheDocument()
  })

  it('renders a back-link to the lifecycle index', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    const back = await screen.findByRole('link', { name: /all clusters/i })
    expect(back.getAttribute('href')).toBe('/lifecycle')
  })

  it('renders one card per present entry', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('The Foo Plan')).toBeInTheDocument()
    expect(screen.getByText('ADR Foo')).toBeInTheDocument()
  })

  it('renders a faded placeholder for each absent stage', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('The Foo Plan')).toBeInTheDocument()
    // Six absent stages: ticket, research, plan-review, validation, pr, pr-review.
    expect(screen.getByText(/no ticket yet/i)).toBeInTheDocument()
    expect(screen.getByText(/no research yet/i)).toBeInTheDocument()
    expect(screen.getByText(/no plan review yet/i)).toBeInTheDocument()
    expect(screen.getByText(/no validation yet/i)).toBeInTheDocument()
    expect(screen.getByText(/no pr yet/i)).toBeInTheDocument()
    expect(screen.getByText(/no pr review yet/i)).toBeInTheDocument()
  })

  it('present-entry cards link to the library page', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    const link = await screen.findByRole('link', { name: /the foo plan/i })
    expect(link.getAttribute('href')).toBe('/library/plans/2026-04-18-foo')
  })

  it('renders bodyPreview on cards that have one and omits the element when empty', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    // The plan card has a preview; the ADR card does not. Asserting
    // via the rendered text (a semantic query) keeps the test free of
    // any test-only DOM hooks — `data-testid` would be the alternative
    // but the project's other tests rely on Testing Library role/text
    // queries, so we follow that convention.
    expect(
      await screen.findByText(/short summary of what the plan covers/i),
    ).toBeInTheDocument()
    // No other preview text should be present. The ADR card has
    // `bodyPreview === ''`, and the component skips the preview
    // element entirely in that case (rather than rendering an empty
    // paragraph that would eat vertical space).
    const allPreviews = screen.queryAllByText(/short summary/i)
    expect(allPreviews).toHaveLength(1)
  })

  it('shows loading state while fetching', () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(screen.getByText(/loading/i)).toBeInTheDocument()
  })

  it('shows a "no such cluster" message and a back-link on 404', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockRejectedValue(
      new fetchModule.FetchError(404, 'GET /api/lifecycle/foo: 404'),
    )
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/no cluster called/i)
    // Back-link is reachable even from the error state so users can
    // bounce back to the index without using the browser controls.
    expect(screen.getByRole('link', { name: /all clusters/i })).toBeInTheDocument()
  })

  it('shows a generic error message on 5xx without leaking the URL', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockRejectedValue(
      new fetchModule.FetchError(500, 'GET /api/lifecycle/foo: 500'),
    )
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(/something went wrong/i)
    // Internal API path must not leak through to end-users.
    expect(alert.textContent).not.toMatch(/\/api\//)
  })

  it('renders Notes entries in a separate "Other" long-tail section', async () => {
    const withNotes: LifecycleCluster = {
      ...cluster,
      entries: [
        ...cluster.entries,
        entry('notes', 'meta/notes/2026-04-20-foo.md', 'A scratch note', 150),
      ],
      completeness: { ...cluster.completeness, hasNotes: true },
    }
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(withNotes)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })

    // Long-tail section is rendered with its own heading and the
    // Notes entry sits underneath, while no "no notes yet" placeholder
    // appears in the main timeline (long-tail stages are hidden when
    // empty rather than showing a placeholder).
    expect(await screen.findByRole('region', { name: /other artifacts/i }))
      .toBeInTheDocument()
    expect(screen.getByText('A scratch note')).toBeInTheDocument()
    expect(screen.queryByText(/no notes yet/i)).not.toBeInTheDocument()
  })

  it('hides the "Other" long-tail section when no long-tail entries exist', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    await screen.findByText('The Foo Plan')
    expect(screen.queryByRole('region', { name: /other artifacts/i }))
      .not.toBeInTheDocument()
  })

  it('renders multiple entries within a single stage', async () => {
    // Realistic case: two plan-reviews — first review then a review-2
    // after revision. Both must appear under the Plan review stage in
    // their canonical order. Pin this so a refactor that takes only
    // the first match per stage is detected.
    const multi: LifecycleCluster = {
      ...cluster,
      entries: [
        ...cluster.entries,
        entry('plan-reviews', 'meta/reviews/plans/foo-review-1.md', 'Foo plan review 1', 110),
        entry('plan-reviews', 'meta/reviews/plans/foo-review-2.md', 'Foo plan review 2', 130),
      ],
      completeness: { ...cluster.completeness, hasPlanReview: true },
    }
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(multi)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('Foo plan review 1')).toBeInTheDocument()
    expect(screen.getByText('Foo plan review 2')).toBeInTheDocument()
  })
})
```

### File: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.tsx`

The view is split into a router-bound shell (`LifecycleClusterView`)
and a pure renderer (`LifecycleClusterContent`). The shell does
nothing but read the typed `slug` param and forward it; the renderer
takes `slug` as a required prop and owns the query. This keeps the
production path strictly typed via the route's own `useParams` (no
`strict: false` cast, no `params.slug ?? ''` fallback, no `enabled`
guard) and lets tests render the renderer directly without router
setup. The router test below still exercises the shell end-to-end.

```typescript
import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchLifecycleCluster, FetchError } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { fileSlugFromRelPath } from '../../api/path-utils'
import {
  LIFECYCLE_PIPELINE_STEPS,
  WORKFLOW_PIPELINE_STEPS, LONG_TAIL_PIPELINE_STEPS,
  type IndexEntry, type LifecycleCluster,
} from '../../api/types'
import { lifecycleClusterRoute } from '../../router'
import styles from './LifecycleClusterView.module.css'

/** Router-bound shell. Reads the strictly-typed `slug` from the
 *  cluster route and forwards it. Production goes through this. */
export function LifecycleClusterView() {
  const { slug } = lifecycleClusterRoute.useParams()
  return <LifecycleClusterContent slug={slug} />
}

/** Pure renderer. Tests render this directly with a literal slug,
 *  avoiding any router/`useParams` setup. */
export function LifecycleClusterContent({ slug }: { slug: string }) {
  const { data: cluster, isLoading, isError, error } = useQuery({
    queryKey: queryKeys.lifecycleCluster(slug),
    queryFn: () => fetchLifecycleCluster(slug),
  })

  if (isLoading) return <p>Loading…</p>
  if (isError || !cluster) {
    // Branch on the typed error so 404 (user typo / stale link) and
    // 5xx (real failure) get distinct copy. The raw URL and status
    // code are NOT surfaced to end-users — they belong in dev tools,
    // not in the UI.
    const isNotFound = error instanceof FetchError && error.status === 404
    return (
      <div className={styles.container}>
        <Link to="/lifecycle" className={styles.backLink}>
          ← All clusters
        </Link>
        <p role="alert" className={styles.error}>
          {isNotFound
            ? <>No cluster called <code>{slug}</code> exists.</>
            : 'Something went wrong loading this cluster. Try again later.'}
        </p>
      </div>
    )
  }

  return (
    <div className={styles.container}>
      <Link to="/lifecycle" className={styles.backLink}>
        ← All clusters
      </Link>

      <header className={styles.header}>
        <h2 className={styles.title}>{cluster.title}</h2>
        <span className={styles.slug}>{cluster.slug}</span>
      </header>

      <ol className={styles.timeline}>
        {WORKFLOW_PIPELINE_STEPS.map(step => renderStage(step, cluster.entries))}
      </ol>

      {LONG_TAIL_PIPELINE_STEPS.some(
        step => cluster.entries.some(e => e.type === step.docType),
      ) && (
        <section
          className={styles.longTail}
          aria-labelledby="lifecycle-other-artifacts"
        >
          <h3
            id="lifecycle-other-artifacts"
            className={styles.longTailHeading}
          >
            Other artifacts
          </h3>
          <ol className={styles.timeline}>
            {LONG_TAIL_PIPELINE_STEPS.map(step => renderStage(step, cluster.entries))}
          </ol>
        </section>
      )}
    </div>
  )
}

type Step = (typeof LIFECYCLE_PIPELINE_STEPS)[number]

function renderStage(step: Step, entries: IndexEntry[]) {
  const stageEntries = entries.filter(e => e.type === step.docType)
  if (stageEntries.length === 0) {
    // Long-tail stages don't render an absent placeholder — the whole
    // section is hidden when no long-tail entries exist (see above).
    if (step.longTail) return null
    return (
      <li
        key={step.key}
        className={`${styles.stage} ${styles.absent}`}
        data-stage={step.key}
        data-present="false"
      >
        <span className={styles.stageLabel}>{step.label}</span>
        <span className={styles.placeholder}>{step.placeholder}</span>
      </li>
    )
  }
  return (
    <li
      key={step.key}
      className={styles.stage}
      data-stage={step.key}
      data-present="true"
    >
      <span className={styles.stageLabel}>{step.label}</span>
      <ul className={styles.entryList}>
        {stageEntries.map(e => (
          <EntryCard key={e.relPath} entry={e} />
        ))}
      </ul>
    </li>
  )
}

function EntryCard({ entry }: { entry: IndexEntry }) {
  const fileSlug = fileSlugFromRelPath(entry.relPath)
  const status = (entry.frontmatter as Record<string, unknown>).status
  const date = (entry.frontmatter as Record<string, unknown>).date
  return (
    <li className={styles.entryCard}>
      <Link
        to="/library/$type/$fileSlug"
        params={{ type: entry.type, fileSlug }}
        className={styles.entryLink}
      >
        <span className={styles.entryTitle}>{entry.title}</span>
      </Link>
      <div className={styles.entryMeta}>
        {typeof date === 'string' && <span>{date}</span>}
        {typeof status === 'string' && (
          <span className={styles.statusBadge}>{status}</span>
        )}
      </div>
      {entry.bodyPreview && (
        <p className={styles.bodyPreview}>{entry.bodyPreview}</p>
      )}
    </li>
  )
}
```

### File: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.module.css`

```css
.container { max-width: 800px; }

.backLink {
  display: inline-block;
  margin-bottom: 0.75rem;
  font-size: 0.85rem;
  color: #6b7280;
  text-decoration: none;
}

.backLink:hover,
.backLink:focus-visible {
  color: #1d4ed8;
  text-decoration: underline;
}

.header {
  display: flex;
  align-items: baseline;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
}

.title { margin: 0; font-size: 1.4rem; color: #111827; }
.slug  { font-family: monospace; font-size: 0.85rem; color: #9ca3af; }

.timeline {
  list-style: none;
  margin: 0;
  padding: 0;
  border-left: 2px solid #e5e7eb;
}

.stage {
  position: relative;
  padding: 0 0 1.25rem 1.5rem;
  margin-left: 6px;
}

.stage::before {
  content: '';
  position: absolute;
  left: -7px;
  top: 4px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #2563eb;
  border: 2px solid #ffffff;
  box-shadow: 0 0 0 1.5px #1d4ed8;
}

.absent::before {
  background: #f3f4f6;
  box-shadow: 0 0 0 1.5px #d1d5db;
}

.stageLabel {
  display: block;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: #6b7280;
  margin-bottom: 0.4rem;
}

.placeholder {
  color: #9ca3af;
  font-style: italic;
  font-size: 0.875rem;
}

.entryList { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.5rem; }

.entryCard {
  border: 1px solid #e5e7eb;
  border-radius: 4px;
  padding: 0.55rem 0.8rem;
  background: #ffffff;
}

.entryLink { text-decoration: none; color: inherit; }
.entryLink:hover .entryTitle { color: #1d4ed8; }
.entryTitle { font-weight: 500; color: #111827; }

.entryMeta {
  display: flex;
  gap: 0.6rem;
  align-items: center;
  font-size: 0.75rem;
  color: #6b7280;
  margin-top: 4px;
}

.statusBadge {
  display: inline-block;
  padding: 0.05rem 0.4rem;
  border-radius: 9999px;
  background: #e5e7eb;
  color: #374151;
}

.bodyPreview {
  margin: 0.4rem 0 0;
  font-size: 0.85rem;
  color: #4b5563;
  line-height: 1.4;
  /* Cap to ~3 lines visually so cards stay compact even when previews
   * are at the 200-char limit. The webkit-prefixed properties handle
   * Chrome/Safari/Edge; the standard `line-clamp` covers Firefox 68+;
   * the `max-height` floor prevents browsers without either from
   * letting the preview blow up to ~5 lines and break card heights. */
  display: -webkit-box;
  -webkit-line-clamp: 3;
  line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  max-height: calc(1.4em * 3);
}

.error {
  color: #991b1b; background: #fef2f2; border: 1px solid #fecaca;
  border-radius: 4px; padding: 0.5rem 0.75rem;
}

.longTail {
  margin-top: 1.75rem;
  padding-top: 1rem;
  border-top: 1px dashed #e5e7eb;
}

.longTailHeading {
  margin: 0 0 0.6rem;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #9ca3af;
  font-weight: 600;
}
```

#### Success criteria

```bash
npm run test -- LifecycleClusterView
# 12 tests pass (5 base + bodyPreview + 2 long-tail Notes + back-link + 404 + 5xx + multi-entry stage)
```

---

## Step 10: Frontend — wire routes + remove `LifecycleStub`

### 10a. Add a `LifecycleLayout` so nested routes share an Outlet

### File: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleLayout.tsx` (new)

```typescript
import { Outlet } from '@tanstack/react-router'

export function LifecycleLayout() {
  return <Outlet />
}
```

### 10b. Update `router.ts`

Replace the existing `lifecycleRoute` with a layout + index + cluster trio
(matching the library subtree's structure):

```typescript
import { LifecycleLayout } from './routes/lifecycle/LifecycleLayout'
import { LifecycleIndex } from './routes/lifecycle/LifecycleIndex'
import { LifecycleClusterView } from './routes/lifecycle/LifecycleClusterView'
// (Remove the old `LifecycleStub` import.)

export const lifecycleRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/lifecycle',
  component: LifecycleLayout,
})

export const lifecycleIndexRoute = createRoute({
  getParentRoute: () => lifecycleRoute,
  path: '/',
  component: LifecycleIndex,
})

// `lifecycleClusterRoute` is `export`ed because `LifecycleClusterView`
// (the router-bound shell) imports it to call `route.useParams()` —
// this gives the shell a strictly-typed `slug` without resorting to
// `useParams({ strict: false })`. The other two routes are exported
// for symmetry with the existing library subtree pattern.
export const lifecycleClusterRoute = createRoute({
  getParentRoute: () => lifecycleRoute,
  path: '/$slug',
  component: LifecycleClusterView,
})

// In the routeTree assembly, replace `lifecycleRoute` with:
//   lifecycleRoute.addChildren([lifecycleIndexRoute, lifecycleClusterRoute])
```

### 10c. Delete `LifecycleStub.tsx`

It is no longer referenced. Removing it (rather than leaving it as
unused dead code) keeps the route tree self-documenting.

### 10d. Add router-tree tests

### File: `skills/visualisation/visualise/frontend/src/router.test.tsx`

Append two cases:

```typescript
it('routes /lifecycle to the index view', async () => {
  vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue([])
  const router = renderAt('/lifecycle')
  await waitForPath(router, '/lifecycle')
  // The empty-state copy from LifecycleIndex confirms the route resolved
  // to the real component, not a stub.
  expect(
    await screen.findByText(/no lifecycle clusters/i),
  ).toBeInTheDocument()
})

it('routes /lifecycle/foo to the cluster detail view', async () => {
  const spy = vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue({
    slug: 'foo', title: 'Foo Cluster', entries: [],
    completeness: {
      hasTicket: false, hasResearch: false, hasPlan: false,
      hasPlanReview: false, hasValidation: false, hasPr: false,
      hasPrReview: false, hasDecision: false, hasNotes: false,
    },
    lastChangedMs: 0,
  })
  const router = renderAt('/lifecycle/foo')
  await waitForPath(router, '/lifecycle/foo')
  expect(
    await screen.findByRole('heading', { name: 'Foo Cluster' }),
  ).toBeInTheDocument()
  // Pin the slug round-trip URL → useParams (in the shell) →
  // LifecycleClusterContent prop → fetch arg. Without this assertion,
  // a regression where the shell forwards an empty/wrong slug would
  // still pass because the mock returns the same cluster regardless.
  expect(spy).toHaveBeenCalledWith('foo')
})
```

The existing `beforeEach` in `router.test.tsx` already stubs `fetchTypes`
and the templates fetches; the new tests add their own `fetchLifecycle*`
spies inline rather than polluting `beforeEach` with cases that other
tests don't need.

#### Success criteria

```bash
npm run test -- router
# router.test.tsx: pre-existing 5 tests + 2 new = 7 tests pass
```

---

## Full success criteria

### Automated verification

- [ ] `mise run test:unit` passes (visualiser server twice + frontend Vitest).
- [ ] `mise run test:integration` passes (visualiser cargo `--tests --features dev-frontend`).
- [ ] `mise run test` passes end-to-end.
- [ ] `cargo build` (default `embed-dist`) succeeds after `npm run build`.
- [ ] `npm run build` exits 0 with no TypeScript errors.

Specific suites:

- [ ] `cargo test clusters` — 10 tests (existing 7 + new `last_changed_ms` ×3);
  `clusters.rs::tests::entry()` now delegates to the shared
  `test_support::entry_for_test` factory.
- [ ] `cargo test frontmatter::body_preview_tests` — 11 tests (new helper).
- [ ] `cargo test indexer` — existing tests + `index_entry_carries_body_preview`.
- [ ] `cargo test --tests --features dev-frontend api_lifecycle` — 5 tests
  (existing 3 + new list and detail wire-shape tests).
- [ ] `npm run test -- format` — 5 new tests for `formatMtime` (shared helper).
- [ ] `npm run test -- fetch` — pre-existing 12 + new 6 = 18 tests.
- [ ] `npm run test -- use-doc-events` — pre-existing 7 + new 2 = 9 tests.
- [ ] `npm run test -- PipelineDots` — 4 tests.
- [ ] `npm run test -- LifecycleIndex` — 12 tests.
- [ ] `npm run test -- LifecycleClusterView` — 12 tests.
- [ ] `npm run test -- router` — 7 tests.

### Manual verification

- [ ] Open `http://localhost:<port>/lifecycle` — see one card per cluster.
- [ ] Cards render eight pipeline-of-dots indicators with present stages
  visibly distinct (filled, blue) from absent stages (faded, grey).
- [ ] Default sort is "Recent"; clicking "Oldest" reorders ascending by
  mtime; clicking "Completeness" sorts by the count of present stages.
- [ ] Typing in the filter input narrows the visible cards to those
  whose title or slug substring-matches (case-insensitive); clearing
  the input restores the full list.
- [ ] Click a card → land on `/lifecycle/:slug` with a vertical timeline.
- [ ] Each present stage shows a card linking to the corresponding
  `/library/:type/:fileSlug` page.
- [ ] Cards whose document has a body show a 1–3 line plain-text preview
  underneath the title; cards whose document is body-less (e.g. ADR
  frontmatter-only docs) omit the preview block entirely.
- [ ] Each absent stage shows a faded placeholder ("no plan review yet",
  "no validation yet", etc.).
- [ ] Edit any `.md` file under a watched directory on disk → within
  ~500ms the cluster's mtime updates and the open detail view rerenders
  via SSE invalidation.
- [ ] Deep link directly to `/lifecycle/<known-slug>` → renders without
  passing through the index page first.
- [ ] Deep link to `/lifecycle/does-not-exist` → renders the
  "no cluster called …" message and the back-link, no internal API
  path leaked.
- [ ] Sidebar `/lifecycle` link is highlighted as active when on
  `/lifecycle` and `/lifecycle/:slug` (the layout-with-index
  reshape did not regress active-link detection).

---

## Implementation sequence

Stop after each step and verify the named tests pass before proceeding.

1. [ ] `server/src/clusters.rs` — add `last_changed_ms` field tests, then field.
2. [ ] `server/src/test_support.rs` (new, `#[cfg(test)]`) — add the shared `entry_for_test` factory; declare the module from `lib.rs`. Migrate `clusters.rs::tests::entry()` to delegate to it (the factory's defaults include `body_preview: String::new()`, so adding the field in step 4 won't break clusters tests).
3. [ ] `cargo test clusters` — green.
4. [ ] `server/src/frontmatter.rs` — write `body_preview_from` tests, then implement.
5. [ ] `server/src/indexer.rs` — add `body_preview` field on `IndexEntry`, populate in `rescan`, add `index_entry_carries_body_preview` test.
6. [ ] Sweep `grep -rn 'IndexEntry {' server/src server/tests`. Migrate each test-side literal to `test_support::entry_for_test(...)`; leave the production `indexer.rs::rescan` literal as-is (it owns the real values).
7. [ ] `cargo build` and `cargo test --lib --features dev-frontend` — green.
8. [ ] `server/tests/api_lifecycle.rs` — add the combined `lastChangedMs` + `bodyPreview` integration test.
9. [ ] `cargo test --tests --features dev-frontend api_lifecycle` — green.
10. [ ] `frontend/src/api/types.ts` — add `bodyPreview` to `IndexEntry`, lifecycle types, `LIFECYCLE_PIPELINE_STEPS`.
11. [ ] Add `frontend/src/api/test-fixtures.ts` with `makeIndexEntry(overrides)`. Migrate in-scope mocks to it; for any remaining literals the TypeScript compiler flags, either migrate or add `bodyPreview: ''` inline.
12. [ ] `frontend/src/api/fetch.test.ts` — append 6 lifecycle fetch tests.
13. [ ] `frontend/src/api/fetch.ts` — implement `fetchLifecycleClusters` + `fetchLifecycleCluster`.
14. [ ] `npm run test -- fetch` — green.
15. [ ] `frontend/src/api/use-doc-events.test.ts` — append cluster-prefix invalidation test.
16. [ ] `frontend/src/api/use-doc-events.ts` — invalidate `['lifecycle-cluster']` prefix.
17. [ ] `npm run test -- use-doc-events` — green.
18. [ ] `frontend/src/components/PipelineDots/PipelineDots.test.tsx` — write 4 tests.
19. [ ] `frontend/src/components/PipelineDots/PipelineDots.tsx` + `.module.css` — implement.
20. [ ] `npm run test -- PipelineDots` — green.
21. [ ] `frontend/src/api/format.ts` + `frontend/src/api/format.test.ts` — extract shared `formatMtime` (with injectable `now`) and unit-test the ladder boundaries.
22. [ ] `frontend/src/routes/lifecycle/LifecycleIndex.test.tsx` — write 12 tests covering sort modes, completeness tiebreak, dot rendering, empty/loading/error states (FetchError + non-FetchError, both with URL-leak assertions), and filter behaviour (substring match + no-match).
23. [ ] `frontend/src/routes/lifecycle/LifecycleIndex.tsx` + `.module.css` — implement (importing `formatMtime` from `../../api/format`).
24. [ ] `npm run test -- LifecycleIndex format` — both green.
25. [ ] `frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx` — write 12 tests covering the body-preview render/omit case, the long-tail Notes section (rendered + hidden), the back-link, 404 vs 5xx error branches, and multi-entry-per-stage rendering.
26. [ ] `frontend/src/routes/lifecycle/LifecycleClusterView.tsx` + `.module.css` — implement the router shell + `LifecycleClusterContent` renderer; `EntryCard` renders `bodyPreview` only when non-empty.
27. [ ] `npm run test -- LifecycleClusterView` — green.
28. [ ] `frontend/src/routes/lifecycle/LifecycleLayout.tsx` — new file.
29. [ ] `frontend/src/router.ts` — replace stub with layout/index/cluster trio.
30. [ ] Delete `frontend/src/routes/lifecycle/LifecycleStub.tsx`.
31. [ ] `frontend/src/router.test.tsx` — append route-tree tests.
32. [ ] `npm run test -- router` — green.
33. [ ] `mise run test` — full suite green.
34. [ ] `npm run build` and `cargo build` — both succeed.
35. [ ] Manual smoke test in the browser against a real `meta/` directory.

---

## References

- Spec: `meta/specs/2026-04-17-meta-visualisation-design.md` §§ Views
  (Lifecycle), Data model (LifecycleCluster), Cross-cutting UX, Live updates.
- Research: `meta/research/2026-04-17-meta-visualiser-implementation-context.md`
  §§ Phase 6, D5 (reviews modelling), G3 (review-N suffix strip).
- Phase 4 plan: `meta/plans/2026-04-22-meta-visualiser-phase-4-sse-hub-and-notify-watcher.md`
  (SSE event shapes, watcher cluster recompute).
- Phase 5 plan: `meta/plans/2026-04-22-meta-visualiser-phase-5-frontend-scaffold-and-library-view.md`
  (frontend testing patterns, MemoryRouter wrapper, fetch test structure,
  Wrapper QueryClient pattern, query-keys layout).
- Server cluster module: `skills/visualisation/visualise/server/src/clusters.rs`.
- Server API endpoint: `skills/visualisation/visualise/server/src/api/lifecycle.rs`.
- Frontend types: `skills/visualisation/visualise/frontend/src/api/types.ts`.
- Frontend query keys: `skills/visualisation/visualise/frontend/src/api/query-keys.ts:9-10`.
- Frontend SSE hook: `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts`.
