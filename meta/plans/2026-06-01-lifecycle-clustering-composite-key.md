---
type: plan
id: "2026-06-01-lifecycle-clustering-composite-key"
title: "Lifecycle Clustering Composite Key Implementation Plan"
date: "2026-06-01T21:47:39+00:00"
author: "Toby Clemson"
producer: create-plan
status: accepted
work_item_id: ""
parent: ""
reviewer: ""
tags: [visualiser, clusters, slug, typed-linkage, frontmatter]
revision: "d773efd41bb5d12c3fde4fffc25128da2708eda6"
repository: "accelerator"
last_updated: "2026-06-02T00:45:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Lifecycle Clustering Composite Key Implementation Plan

## Overview

Make the visualiser's lifecycle clustering bucket entries by a
**composite key** that walks ADR-0034 typed-linkage frontmatter first
and falls back to a tightened filename-slug derivation, so work items
and their plans/research/reviews/validations land in the same cluster
during the in-progress epic-0057 migration.

The work follows the recommended direction in
`meta/research/codebase/2026-06-01-lifecycle-clustering-slug-mismatch.md`
(Option C — composite key). Each phase is independently shippable,
each starts with failing tests, and each delivers visible value on its
own.

## Current State Analysis

`compute_clusters_with_backfill` at
[clusters.rs:63-103](skills/visualisation/visualise/server/src/clusters.rs#L63-L103)
buckets entries solely by `IndexEntry.slug`. `IndexEntry.slug` is set
in `build_entry` at
[indexer.rs:1156-1187](skills/visualisation/visualise/server/src/indexer.rs#L1156-L1187)
via the per-doc-type derivation in
[slug.rs:14-86](skills/visualisation/visualise/server/src/slug.rs#L14-L86).
The derivation is incompatible across doc types whenever the
artifact's filename embeds the work-item ID, and rejects
work-item-review filenames outright (no date prefix).

The typed-linkage frontmatter chain that would resolve this lives in
the indexer's secondary indexes already (
`work_item_by_id`, `plans_by_id`, `work_item_refs_by_target`,
`reviews_by_target`) at
[indexer.rs:217-234](skills/visualisation/visualise/server/src/indexer.rs#L217-L234),
but only `/api/related/*` consumes it
([related.rs:22-79](skills/visualisation/visualise/server/src/related.rs#L22-L79)).

Three independent failure modes (per the research):

1. **ID-prefix bleed-through** — new-convention plans/research/plan-
   reviews carry `NNNN-` after the date, so the slug differs from the
   work-item's slug. ~47 plans, 36 research files, 47 plan-reviews.
2. **Convention drift** — validations carry a non-strippable
   `-validation` suffix; PR-descriptions/PR-reviews don't follow any
   work-item-aware shape.
3. **Work-item reviews are universally orphans** — slug rule requires
   a date; corpus filenames don't have one (37/37 files dropped).

### Key Discoveries:

- `IndexEntry` struct
  ([indexer.rs:162-192](skills/visualisation/visualise/server/src/indexer.rs#L162-L192))
  uses `#[serde(rename_all = "camelCase")]`; adding `cluster_key:
  Option<String>` auto-serialises as `clusterKey`.
- `target_path_from_entry`
  ([indexer.rs:829-845](skills/visualisation/visualise/server/src/indexer.rs#L829-L845))
  is hard-gated to `DocTypeKey::PlanReviews`. Generalising it across
  WorkItemReviews / PrReviews / Validations also fixes
  `declared_outbound` / `reviews_by_target` for those types as a free
  side effect.
- `frontmatter::read_ref_keys`
  ([frontmatter.rs:305-368](skills/visualisation/visualise/server/src/frontmatter.rs#L305-L368))
  currently only understands the `work-item:` typed prefix. A central
  typed-ref parser (handles `work-item:NNNN`, `plan:<id>`, `adr:NNNN`,
  `pr:<n>`, plus repo-relative paths) is missing; both `read_ref_keys`
  and `target_path_from_entry` should share it.
- `canonicalise_refs`
  ([indexer.rs:1031-1080](skills/visualisation/visualise/server/src/indexer.rs#L1031-L1080))
  already normalises bare numerics to padded width-N and prefixes the
  default project code; the resolver reuses this so its keys match
  `work_item_by_id` exactly.
- **No clippy/fmt is enforced** — `mise.toml` and CI run only
  `invoke test` (cargo test for the server, vitest for the frontend,
  playwright e2e). Each phase's automated success criteria stop at the
  test layer; lint/fmt parity is out of scope.
- **Frontend uses `cluster.slug` as the URL identifier and React-Query
  cache key** at
  [router.ts:124-131](skills/visualisation/visualise/frontend/src/router.ts#L124-L131)
  and
  [query-keys.ts:51-52](skills/visualisation/visualise/frontend/src/api/query-keys.ts#L51-L52).
  The plan keeps `cluster.slug` as the URL anchor (now derived from the
  cluster's work-item slug when present); URLs converge on the
  work-item slug, which is the natural anchor.
- **On-disk frontmatter is messier than ADR-0034 prescribes** —
  `target:` on every disk artifact today is a repo-relative `meta/...
  md` path, never `plan:<id>`. `work_item_id:` on six plans holds a
  path instead of an ID. The resolver must accept all on-disk shapes
  before it accepts the ADR-prescribed shapes.
- **`related::resolve_related` couples to cluster identity.**
  `related::resolve_related` (server/src/related.rs:27-42) currently
  finds an entry's inferred-cluster siblings by `c.slug ==
  entry.slug`. Once cluster representative slugs diverge from member
  slugs (post-Phase-4), this invariant breaks; Phase 4 must update
  the lookup to use `cluster_key` (with slug fallback for orphan
  types) to preserve `/api/related` ↔ `/api/lifecycle` agreement.

## Desired End State

After this plan completes:

- **The plan and work item for `0040-pipeline-visualisation-overhaul`
  share a single lifecycle cluster** in the visualiser, accessed at
  `/lifecycle/pipeline-visualisation-overhaul` (the work-item slug).
  The same holds for every work-item-rooted bundle in the corpus.
- **Every plan-review and validation joins its target plan's cluster**
  by following the `target:` path in frontmatter.
- **Every work-item review joins its target work-item's cluster** by
  following the `target:` path in frontmatter.
- **All 37 files under `meta/reviews/work/`** are clustered (currently
  100% orphaned).
- **The lifecycle index** (`/lifecycle`) shows one card per work-item-
  rooted cluster instead of one per disjoint slug derivation.
- **`/api/lifecycle` JSON** carries a `clusterKey` field alongside
  `slug` on both `IndexEntry` and `LifecycleCluster`, naming the
  canonical work-item id (`"0040"`) or — for clusters with no work
  item — being `null`.
- **The lifecycle cluster view** shows a small "clustered via" tag on
  each entry exposing the typed-linkage chain (e.g.
  `parent → work-item:0040`) so users can debug clustering decisions
  during the epic-0057 migration window.
- **`/api/related/*` and `/api/lifecycle` agree on cluster membership.**
  `related::resolve_related` looks up inferred-cluster siblings via
  `cluster_key` (with slug fallback for orphan-by-design types), so
  every entry that joins a cluster in `/api/lifecycle` sees a
  populated `inferredCluster` in `/api/related/<its-path>`.
- **Path safety is preserved.** `target_path_from_entry` continues to
  thread every `TypedRef::Path` value through `normalize_target_key`,
  rejecting `..`/absolute/NUL/backslash and verifying the resolved
  path stays under `project_root`.

### How to verify

- `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib`
  passes the new test suites for `slug`, `typed_ref`, `target_resolution`,
  `cluster_key`, and `clusters::cluster_key_integration`.
- `mise run test:integration:visualiser` passes.
- `mise run test:unit:frontend` passes (covers updated wire-shape
  tests in `fetch.test.ts` and the new debug-tag assertion in
  `LifecycleClusterView.test.tsx`).
- Manual: with the dev server running, visit `/lifecycle` and confirm
  the count of clusters has decreased relative to `main` (the broken
  duplicates collapse into single cards). Visit
  `/lifecycle/pipeline-visualisation-overhaul` and confirm the work
  item, plan, research, plan-reviews, and validations are all listed.

## What We're NOT Doing

- **Migrating the corpus.** No frontmatter on disk is rewritten. Epic
  0057 owns the producer-side migration; this plan only changes how
  the visualiser consumes what's already there.
- **Implementing the deferred visualiser-graph epic.** ADR-0034 §Out of
  scope explicitly defers graph rendering to a future epic; this plan
  changes only cluster bucketing, not graph rendering.
- **Adding a configurable cluster strategy** (`strategy: slug |
  typed-linkage | composite`). The composite resolver is the single
  strategy; if a team needs slug-only later, that's a follow-up.
- **Adding clippy or rustfmt enforcement.** No standing target exists;
  introducing one is out of scope.
- **Changing the `/api/lifecycle` URL shape** beyond adding an
  optional `clusterKey` field. No new endpoint; no path scheme change.
- **Rewriting `IndexEntry.slug`'s role as the per-file URL identity.**
  `slug` keeps its role; `cluster_key` is purely a clustering decision
  surfaced for debugging.

## Implementation Approach

Five phases, each starting with failing tests and each independently
shippable:

1. **Tighten slug derivation** — narrow Option A. Fixes the most
   visible bug for filename-shaped cohorts. No dependencies on other
   phases. Closes the slug.rs gaps that fail on the `YYYY-MM-DD-NNNN-`
   and `NNNN-...-review-N` shapes.
2. **Introduce a typed-ref parser** — pure refactor with new tests
   defining the parser's contract; folds into `read_ref_keys` and
   `target_path_from_entry`. No behaviour change.
3. **Generalise target resolution** — drop `target_path_from_entry`'s
   PlanReviews-only gate; route every review/validation through the
   parser. Visible improvement in `/api/related/*` for all review types.
4. **Composite cluster-key resolver** — new module that walks the
   typed-linkage chain per ADR-0034's type-pair table; wired into
   `compute_clusters_with_backfill` (now `ClusterContext`-shaped, no
   shim) with `cluster_key.or(slug)` bucketing.
   `related::resolve_related` is updated in the same wave to look
   up inferred-cluster siblings via `cluster_key` so `/api/related`
   and `/api/lifecycle` agree. The visible clustering bug collapses
   on day one.
5. **Wire shape + frontend debug surface (optional)** — expose
   `clusterKey` on the JSON wire shape; add a small "clustered via"
   tag under each entry on the cluster detail view.

Each phase ends with both automated and manual verification, scoped so
the phase can ship without the next.

---

## Phase 1: Tighten slug derivation

### Overview

Extend the per-doc-type slug derivation in
`skills/visualisation/visualise/server/src/slug.rs` so that:

- Dated doc types (Plans, Research, Validations, Notes,
  PrDescriptions, DesignGaps, DesignInventories) strip an **optional**
  trailing work-item-id prefix after the date — recognising both the
  bare `NNNN-` shape and the project-prefixed `PROJ-NNNN-` shape by
  consulting `WorkItemConfig`.
- Reviewed doc types (PlanReviews, PrReviews, WorkItemReviews) strip
  the same optional ID-prefix between the date and the descriptive
  slug.
- WorkItemReviews accept a **no-date** `NNNN-slug-review-N.md` shape
  (this is what every file under `meta/reviews/work/` looks like
  today) and strip both the leading work-item-id and the trailing
  `-review-N`.
- Existing behaviours are preserved: legacy `YYYY-MM-DD-slug.md` files
  with no embedded ID still produce a descriptive slug; bare slugs
  unchanged.
- The ID-strip is **width-aware**: it only strips a leading segment
  when `WorkItemConfig::extract_id` recognises it as a canonical ID
  (i.e. the head equals the extracted id). Filenames like
  `2026-04-17-100-day-plan.md` (date prefix + descriptive head
  starting with three digits) are unaffected — `100` is not a valid
  4-digit ID under the default pattern.

The public `derive` signature changes to accept
`&WorkItemConfig`. All in-tree call sites (only `build_entry` in
indexer.rs) are updated to pass the config that the indexer already
holds.

### Changes Required:

#### 1. Tests (write first; must fail before implementation)

**File**: `skills/visualisation/visualise/server/src/slug.rs` (tests
module)
**Changes**: Add the following cases to the existing tests module:

```rust
#[test]
fn dated_types_strip_optional_work_item_id_after_date() {
    let cfg = WorkItemConfig::default(); // numeric \d{4} pattern
    for kind in [
        DocTypeKey::Plans,
        DocTypeKey::Research,
        DocTypeKey::Notes,
        DocTypeKey::PrDescriptions,
        DocTypeKey::Validations,
    ] {
        let cases = &[
            // ID present: stripped
            ("2026-05-31-0040-pipeline-visualisation-overhaul.md",
             Some("pipeline-visualisation-overhaul")),
            ("2026-05-05-0031-consolidate-accelerator-owned-files.md",
             Some("consolidate-accelerator-owned-files")),
            // ID absent: descriptive slug preserved (legacy shape)
            ("2026-02-22-pr-review-agents.md", Some("pr-review-agents")),
            // Boundary: leading hyphen group that is NOT digits stays
            ("2026-04-17-foo-bar.md", Some("foo-bar")),
            // Boundary: descriptive head with fewer-than-ID-width digits
            // is NOT a valid ID under the default pattern; preserve it.
            ("2026-04-17-100-day-plan.md", Some("100-day-plan")),
            // Empty descriptive tail after ID strip → None
            ("2026-05-31-0040-.md", None),
            ("2026-05-31-0040.md", None),
        ];
        for (input, expected) in cases {
            let got = derive(kind, input, &cfg);
            assert_eq!(got.as_deref(), *expected, "{kind:?} input={input}");
        }
    }
}

#[test]
fn dated_types_strip_project_prefixed_work_item_id_after_date() {
    // Project-prefixed pattern: PROJ-\d{4}
    let cfg = WorkItemConfig::with_pattern_for_test("PROJ", 4);
    let cases = &[
        ("2026-05-31-PROJ-0040-pipeline.md", Some("pipeline")),
        // Bare numeric ID does NOT match this pattern; preserve descriptor.
        ("2026-05-31-0040-pipeline.md", Some("0040-pipeline")),
        // Legacy: no ID
        ("2026-02-22-foo-bar.md", Some("foo-bar")),
    ];
    for (input, expected) in cases {
        let got = derive(DocTypeKey::Plans, input, &cfg);
        assert_eq!(got.as_deref(), *expected, "input={input}");
    }
}

#[test]
fn plan_reviews_strip_optional_work_item_id_after_date() {
    let cfg = WorkItemConfig::default();
    let cases = &[
        ("2026-05-05-0031-consolidate-accelerator-owned-files-review-1.md",
         Some("consolidate-accelerator-owned-files")),
        // ID-less legacy shape still works
        ("2026-04-18-foo-review-1.md", Some("foo")),
        // Internal -review- preserved (regression guard)
        ("2026-03-28-initialise-skill-and-review-pr-ephemeral-migration-review-1.md",
         Some("initialise-skill-and-review-pr-ephemeral-migration")),
    ];
    for (input, expected) in cases {
        let got = derive(DocTypeKey::PlanReviews, input, &cfg);
        assert_eq!(got.as_deref(), *expected, "input={input}");
    }
}

#[test]
fn work_item_reviews_accept_no_date_id_prefixed_shape() {
    let cfg = WorkItemConfig::default();
    // The shape every file under meta/reviews/work/ uses today.
    let cases = &[
        ("0030-centralise-path-defaults-review-1.md",
         Some("centralise-path-defaults")),
        ("0061-adr-typed-linkage-vocabulary-review-2.md",
         Some("adr-typed-linkage-vocabulary")),
        ("0001-three-layer-review-system-architecture-review-1.md",
         Some("three-layer-review-system-architecture")),
        // Descriptor that ends in `-review` before the numeric suffix
        // (pinning the rightmost-match behaviour of strip_suffix_review_n).
        ("0040-final-review-review-1.md", Some("final-review")),
    ];
    for (input, expected) in cases {
        let got = derive(DocTypeKey::WorkItemReviews, input, &cfg);
        assert_eq!(got.as_deref(), *expected, "input={input}");
    }
}

#[test]
fn work_item_reviews_dated_shape_still_accepted_for_back_compat() {
    let cfg = WorkItemConfig::default();
    // The previously-supported shape stays valid.
    let cases = &[
        ("2026-04-30-completeness-pass-review-1.md",
         Some("completeness-pass")),
        ("2026-05-02-foo-review-7.md", Some("foo")),
    ];
    for (input, expected) in cases {
        let got = derive(DocTypeKey::WorkItemReviews, input, &cfg);
        assert_eq!(got.as_deref(), *expected, "input={input}");
    }
}
```

**Cluster-collapse regression test** (Phase 1 ships an automated guard
that pins the visible bug-fix on the clustering layer, not just the
slug layer):

```rust
// In clusters.rs tests module.
#[test]
fn phase_1_id_prefixed_and_bare_slugs_now_cluster_into_one_bucket() {
    // Two filenames that produced disjoint slugs on `main` and produce
    // the same slug after Phase 1.
    let cfg = WorkItemConfig::default();
    let plan = entry_for_test_with_filename(
        DocTypeKey::Plans,
        "2026-05-31-0040-pipeline-visualisation-overhaul.md",
        &cfg,
    );
    let wi = entry_for_test_with_filename(
        DocTypeKey::WorkItems,
        "0040-pipeline-visualisation-overhaul.md",
        &cfg,
    );
    let (clusters, _, _) = compute_clusters_with_backfill(&[plan, wi]);
    assert_eq!(clusters.len(), 1);
}
```

#### 2. Implementation

**File**: `skills/visualisation/visualise/server/src/config.rs`
**Changes**: Add a width-aware predicate to `WorkItemConfig` that
answers "is this token a canonical work-item id?" — strictly,
respecting the `id_pattern`'s width. This is distinct from
`extract_id` (which uses the more permissive `scan_regex` and
requires a trailing `-`) and from `normalise_id` (which pads bare
digits to the canonical width). The new predicate is the right tool
for the slug helper because it admits only the exact canonical-form
strings, with no padding and no surrounding context.

```rust
impl WorkItemConfig {
    /// True iff `token` is exactly a canonical work-item id under
    /// this configuration. The width is parsed from `id_pattern`'s
    /// `{number:0Nd}` segment; tokens with the wrong digit count
    /// (or a missing/incorrect project prefix) are rejected.
    pub fn is_canonical_id_token(&self, token: &str) -> bool {
        let width = self.canonical_digit_width();
        let digits = match &self.default_project_code {
            Some(code) => match token.strip_prefix(&format!("{code}-")) {
                Some(rest) => rest,
                None => return false,
            },
            None => token,
        };
        digits.len() == width && digits.chars().all(|c| c.is_ascii_digit())
    }

    /// Returns the canonical-form digit width encoded by `id_pattern`'s
    /// `{number:0Nd}` segment, or 0 when no width specifier is present
    /// (in which case `is_canonical_id_token` accepts any length).
    fn canonical_digit_width(&self) -> usize {
        // Parses "{number:04d}" → 4, "PROJ-{number:04d}" → 4,
        // "{number}" → 0 (unanchored).
        let s = &self.id_pattern;
        let Some(i) = s.find("{number") else { return 0; };
        let rest = &s[i + "{number".len()..];
        let Some(end) = rest.find('}') else { return 0; };
        let spec = &rest[..end];
        // spec is like ":04d" or "". Strip leading ':' and trailing 'd'.
        let trimmed = spec.trim_start_matches(':').trim_end_matches('d');
        // trimmed is "04" or "" or "4"; the digit width is the parsed
        // integer (ignoring leading zero-padding flag).
        trimmed.trim_start_matches('0').parse::<usize>().ok()
            .or_else(|| if trimmed.is_empty() { Some(0) } else { trimmed.parse().ok() })
            .unwrap_or(0)
    }
}
```

Tests (in `config::tests`):

```rust
#[test]
fn is_canonical_id_token_under_default_numeric() {
    let cfg = WorkItemConfig::default_numeric();
    assert!(cfg.is_canonical_id_token("0040"));
    assert!(!cfg.is_canonical_id_token("40"));      // wrong width
    assert!(!cfg.is_canonical_id_token("00040"));   // wrong width
    assert!(!cfg.is_canonical_id_token("100"));     // wrong width — pins the descriptor-head bug
    assert!(!cfg.is_canonical_id_token("004A"));    // non-digit
    assert!(!cfg.is_canonical_id_token(""));
}

#[test]
fn is_canonical_id_token_under_project_prefixed_pattern() {
    let cfg = WorkItemConfig::with_pattern_for_test("PROJ", 4);
    assert!(cfg.is_canonical_id_token("PROJ-0040"));
    assert!(!cfg.is_canonical_id_token("0040"));    // missing prefix
    assert!(!cfg.is_canonical_id_token("PROJ-40")); // wrong width
    assert!(!cfg.is_canonical_id_token("OTHER-0040")); // wrong prefix
}
```

**File**: `skills/visualisation/visualise/server/src/slug.rs`
**Changes**: Add a width-aware helper
`strip_optional_work_item_id_prefix` that calls
`cfg.is_canonical_id_token`, and update `derive` to take
`&WorkItemConfig`. The helper strips the **shortest valid leading
id** — only when the head is itself a canonical-form id under the
configured width.

```rust
fn strip_optional_work_item_id_prefix<'a>(stem: &'a str, cfg: &WorkItemConfig) -> &'a str {
    // Walk hyphen positions left-to-right. At each candidate boundary,
    // ask the config whether the head is itself a canonical work-item
    // id (strict: exact width, exact prefix). First match wins —
    // shortest valid id prefix.
    //
    // We deliberately use `is_canonical_id_token` instead of
    // `extract_id`: extract_id's regex is variable-width by default
    // (`^([0-9]+)-`), which would mis-strip digit-only descriptor heads
    // like `100` in `2026-04-17-100-day-plan.md`. The token predicate
    // checks against `id_pattern`'s width specifier, which is the
    // canonical-form contract.
    for (i, c) in stem.char_indices() {
        if c == '-' {
            let head = &stem[..i];
            if cfg.is_canonical_id_token(head) {
                return &stem[i + 1..];
            }
        }
    }
    stem
}

// Modify the existing strip_prefix_date to chain into the helper:
fn strip_prefix_date_and_optional_id(stem: &str, cfg: &WorkItemConfig) -> Option<String> {
    let after_date = strip_prefix_date_str(stem)?;
    let trimmed = strip_optional_work_item_id_prefix(after_date, cfg);
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

// And the no-date branch for WorkItemReviews in `derive`:
DocTypeKey::WorkItemReviews => {
    // Try the dated shape first (back-compat with old fixtures).
    if let Some(slug) = strip_prefix_date_and_optional_id(stem, cfg)
        .and_then(|s| strip_suffix_review_n(&s))
    {
        return Some(slug);
    }
    // Fall back to the no-date `NNNN-slug-review-N.md` shape used
    // by every file under meta/reviews/work/ today.
    let without_id = strip_optional_work_item_id_prefix(stem, cfg);
    if without_id == stem { return None; } // require an ID prefix
    strip_suffix_review_n(without_id)
}
```

The public `derive` signature changes:

```rust
pub fn derive(kind: DocTypeKey, filename: &str, cfg: &WorkItemConfig) -> Option<String>
```

Call sites: only `build_entry` in `indexer.rs:1156-1187`, which
already holds `&self.work_item_cfg` and threads it into the call.

#### 3. Test-support helpers required by Phases 1 + 4

The new tests reference three helpers that do not exist today; the
plan adds them as small additions.

**`WorkItemConfig::default()` → alias for `default_numeric()`** in
`config.rs`. Either implement `Default` for `WorkItemConfig` calling
`default_numeric()`, or replace all `WorkItemConfig::default()` test
calls with the explicit `default_numeric()` — pick one for
consistency. The plan prefers `impl Default` because the tests read
more cleanly.

```rust
impl Default for WorkItemConfig {
    fn default() -> Self { Self::default_numeric() }
}
```

**`WorkItemConfig::with_pattern_for_test(prefix, width)`** in
`config.rs` — a #[cfg(test)] thin wrapper that builds a
project-prefixed pattern at the given digit width without going
through `RawWorkItemConfig` parsing.

```rust
#[cfg(test)]
impl WorkItemConfig {
    pub fn with_pattern_for_test(prefix: &str, width: usize) -> Self {
        let raw = format!("^({}-[0-9]{{{}}})-", regex::escape(prefix), width);
        Self {
            scan_regex: regex::Regex::new(&raw).unwrap(),
            scan_regex_raw: raw,
            id_pattern: format!("{}-{{number:0{}d}}", prefix, width),
            default_project_code: Some(prefix.to_string()),
        }
    }
}
```

**`entry_for_test_with_filename(doc_type, filename, cfg)`** in
`test_support.rs` — extends the existing `entry_for_test` so a
filename-driven slug derivation can be exercised end-to-end without
hand-computing the slug.

```rust
#[cfg(test)]
pub fn entry_for_test_with_filename(
    doc_type: DocTypeKey,
    filename: &str,
    cfg: &WorkItemConfig,
) -> IndexEntry {
    let slug = crate::slug::derive(doc_type, filename, cfg);
    let path = PathBuf::from(format!("/repo/meta/{}/{}", doc_type.dir(), filename));
    IndexEntry {
        path,
        slug,
        // ... defaults matching entry_for_test's shape (mtime, title, etc.)
        ..entry_for_test(doc_type, slug.as_deref().unwrap_or("x"), 0, "T")
    }
}
```

### Success Criteria:

#### Automated Verification:

- [ ] New slug tests pass: `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib slug::tests`
- [ ] Full server unit suite passes: `mise run test:unit:visualiser`
- [ ] Server integration suite passes: `mise run test:integration:visualiser`
- [ ] No existing slug test regresses (the prior `dated_types_strip_iso_date`, `plan_reviews_strip_date_and_review_n_suffix`, `work_item_reviews_strip_date_and_review_n_suffix` etc. all still pass).

#### Manual Verification:

- [ ] With the dev server running on the live `meta/` corpus, visit
  `/lifecycle`. Confirm that previously-duplicate cards (e.g. one for
  `0040-pipeline-visualisation-overhaul` and one for
  `pipeline-visualisation-overhaul`) collapse into one card.
- [ ] Visit `/lifecycle/centralise-path-defaults` and confirm at least
  one file from `meta/reviews/work/` now appears in the cluster
  (today: zero).
- [ ] Confirm the total cluster count on `/lifecycle` decreases vs
  `main`.

---

## Phase 2: Central typed-ref parser

### Overview

Extract a single `parse_typed_ref` helper that understands the full
ADR-0034 reference vocabulary (`work-item:NNNN`, `plan:<id>`,
`adr:NNNN`, `pr:<n>`, and bare repo-relative paths). Replace the
ad-hoc parsing in `frontmatter::read_ref_keys` and
`indexer::target_path_from_entry` so both call sites share one
implementation.

**This is a pure refactor with no behaviour change at any consumer.**
Tests pin the parser's contract; existing tests for `read_ref_keys`
and `target_path_from_entry` continue to pass unchanged.

### Changes Required:

#### 1. Tests (write first)

**File**: `skills/visualisation/visualise/server/src/typed_ref.rs`
(new module)
**Changes**: Define a new module with the parser and its tests.

```rust
//! Central parser for ADR-0034 typed-linkage reference values.
//!
//! **Path safety**: `TypedRef::Path` carries the raw, unvalidated
//! repo-relative path string. Callers that intend to resolve it
//! against the filesystem MUST pass the inner value through
//! `indexer::normalize_target_key` first (which rejects `..`,
//! absolute paths, NUL, backslash, and verifies the result stays
//! under `project_root`). The parser is purely syntactic.

use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TypedRef {
    WorkItem(String),      // raw id token (e.g. "0042", "PROJ-0042")
    Plan(String),          // plan id (filename stem)
    Adr(String),           // ADR-NNNN
    Pr(String),            // PR number
    Path(PathBuf),         // bare repo-relative path (UNVALIDATED)
}

/// Decide whether a suffix after a typed prefix is path-shaped.
/// A path-shaped suffix contains `/` or ends in `.md`.
fn looks_like_path(s: &str) -> bool {
    s.contains('/') || s.ends_with(".md")
}

pub fn parse_typed_ref(raw: &str) -> Option<TypedRef> {
    let s = raw.trim();
    if s.is_empty() { return None; }

    // Hybrid shape: typed prefix carrying a path payload (e.g.
    // `work-item:meta/work/0033-foo.md`, `plan:meta/plans/...md`).
    // These appear in the on-disk corpus during the epic-0057
    // migration window. Treat the path as authoritative and route
    // through filesystem resolution.
    if let Some(rest) = s.strip_prefix("work-item:") {
        if rest.is_empty() { return None; }
        if looks_like_path(rest) { return Some(TypedRef::Path(PathBuf::from(rest))); }
        return Some(TypedRef::WorkItem(rest.to_string()));
    }
    if let Some(rest) = s.strip_prefix("plan:") {
        if rest.is_empty() { return None; }
        if looks_like_path(rest) { return Some(TypedRef::Path(PathBuf::from(rest))); }
        return Some(TypedRef::Plan(rest.to_string()));
    }
    if let Some(rest) = s.strip_prefix("adr:") {
        if rest.is_empty() { return None; }
        return Some(TypedRef::Adr(rest.to_string()));
    }
    if let Some(rest) = s.strip_prefix("pr:") {
        if rest.is_empty() { return None; }
        return Some(TypedRef::Pr(rest.to_string()));
    }
    // No typed prefix: treat path-shaped values as paths, otherwise None.
    if looks_like_path(s) {
        return Some(TypedRef::Path(PathBuf::from(s)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_work_item_prefix() {
        assert_eq!(parse_typed_ref("work-item:0042"),
                   Some(TypedRef::WorkItem("0042".into())));
        assert_eq!(parse_typed_ref("work-item:PROJ-0042"),
                   Some(TypedRef::WorkItem("PROJ-0042".into())));
    }

    #[test]
    fn parses_plan_prefix() {
        assert_eq!(parse_typed_ref("plan:2026-05-31-0040-foo"),
                   Some(TypedRef::Plan("2026-05-31-0040-foo".into())));
    }

    #[test]
    fn parses_adr_and_pr_prefixes() {
        assert_eq!(parse_typed_ref("adr:0034"), Some(TypedRef::Adr("0034".into())));
        assert_eq!(parse_typed_ref("pr:42"),    Some(TypedRef::Pr("42".into())));
    }

    #[test]
    fn parses_repo_relative_path() {
        let r = parse_typed_ref("meta/plans/2026-05-31-0040-foo.md");
        assert_eq!(r, Some(TypedRef::Path("meta/plans/2026-05-31-0040-foo.md".into())));
    }

    #[test]
    fn empty_and_unknown_return_none() {
        assert_eq!(parse_typed_ref(""), None);
        assert_eq!(parse_typed_ref("   "), None);
        assert_eq!(parse_typed_ref("nonsense"), None);
        // Bare slug-like values (no `/`, no `.md`) are not paths.
        assert_eq!(parse_typed_ref("foo"), None);
    }

    #[test]
    fn empty_typed_suffixes_return_none() {
        // Prefix with no payload is malformed and must NOT resolve to
        // Some(*("")) — empty ids flowing into canonicalise_one would
        // silently key a cluster on "".
        assert_eq!(parse_typed_ref("work-item:"), None);
        assert_eq!(parse_typed_ref("plan:"), None);
        assert_eq!(parse_typed_ref("adr:"), None);
        assert_eq!(parse_typed_ref("pr:"), None);
    }

    #[test]
    fn typed_prefix_with_path_payload_routes_to_path() {
        // Hybrid shapes from the epic-0057 migration window: typed
        // prefix carrying a repo-relative path payload. Treat the path
        // as authoritative.
        assert_eq!(
            parse_typed_ref("work-item:meta/work/0033-foo.md"),
            Some(TypedRef::Path("meta/work/0033-foo.md".into())),
        );
        assert_eq!(
            parse_typed_ref("plan:meta/plans/2026-05-31-0040-foo.md"),
            Some(TypedRef::Path("meta/plans/2026-05-31-0040-foo.md".into())),
        );
    }

    #[test]
    fn whitespace_is_trimmed() {
        assert_eq!(parse_typed_ref("  work-item:0042  "),
                   Some(TypedRef::WorkItem("0042".into())));
    }
}
```

**File**: `skills/visualisation/visualise/server/src/lib.rs`
**Changes**: Add `pub mod typed_ref;`.

#### 2. Implementation: fold into existing call sites

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs`
**Changes**: In `read_ref_keys`, replace the inline `strip_prefix(
"work-item:")` block (around line 346-357) with `parse_typed_ref(s)`
matched on `TypedRef::WorkItem(id)`. Existing tests
(`read_ref_keys_ignores_non_work_item_target_prefix` at line 522 and
the PR prefix test at line 528) continue to pass — the parser
recognises those prefixes but `read_ref_keys` only adds the work-item
variant.

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
**Changes**: In `target_path_from_entry`
([indexer.rs:829-845](skills/visualisation/visualise/server/src/indexer.rs#L829-L845)),
replace inline prefix parsing with `parse_typed_ref`. Behaviour is
preserved: `TypedRef::Plan(id)` looks up `plans_by_id`,
`TypedRef::Path(p)` is passed through `normalize_target_key` and then
joined to `project_root`; everything else returns `None`. (The
PlanReviews-only doc-type gate stays in this phase; Phase 3 lifts
it.)

**Path-safety contract preserved**: The existing
`normalize_target_key` (server/src/indexer.rs:794) rejects `..`,
absolute paths, NUL, and backslash. The Phase 2 refactor must thread
the `TypedRef::Path(p)` value through it before joining to
`project_root`. The plan's prescribed Phase 3 implementation (below)
shows the explicit call.

### Success Criteria:

#### Automated Verification:

- [ ] New parser tests pass: `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib typed_ref::tests`
- [ ] `read_ref_keys` existing tests pass unchanged: `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib read_ref_keys`
- [ ] `target_path_from_entry` existing tests pass unchanged.
- [ ] Full server unit suite passes: `mise run test:unit:visualiser`.
- [ ] Server integration suite passes: `mise run test:integration:visualiser`.

#### Manual Verification:

- [ ] With the dev server running, hit `/api/related/<path-to-a-plan-
  review>` and confirm the response payload is byte-identical to a
  capture taken before this phase (no behaviour change is the
  contract).

---

## Phase 3: Generalise target resolution to all review/validation types

### Overview

Drop the `DocTypeKey::PlanReviews`-only gate in
`target_path_from_entry`
([indexer.rs:834](skills/visualisation/visualise/server/src/indexer.rs#L834))
and route every doc type that carries a `target:` frontmatter key
(PlanReviews, WorkItemReviews, PrReviews, Validations) through the
generalised resolver. Use the typed-ref parser from Phase 2.

The on-disk shape of every `target:` value today is a repo-relative
path (per the corpus audit); the resolver must handle both that shape
and the ADR-0034-prescribed `plan:<id>` / `work-item:NNNN` shapes.

This phase makes `/api/related/*` declared-outbound and declared-
inbound work uniformly across all review types — a visible
improvement before clustering itself is touched.

### Changes Required:

#### 1. Tests (write first)

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
(new tests module section for `target_path_from_entry`)
**Changes**: Add tests covering each doc type and each accepted
target shape.

```rust
#[cfg(test)]
mod target_path_resolution_tests {
    use super::*;

    fn plans_by_id_with(id: &str, path: PathBuf) -> HashMap<String, PathBuf> {
        let mut m = HashMap::new();
        m.insert(id.to_string(), path);
        m
    }

    fn work_item_by_id_with(id: &str, path: PathBuf) -> HashMap<String, PathBuf> {
        let mut m = HashMap::new();
        m.insert(id.to_string(), path);
        m
    }

    // Existing PlanReviews tests stay (regression guard).

    #[test]
    fn work_item_review_with_path_target_resolves_to_work_item_path() {
        let mut entry = entry_for_test(DocTypeKey::WorkItemReviews, "x", 0, "R");
        entry.frontmatter = serde_json::json!({
            "target": "meta/work/0033-design-token-system.md"
        });
        let root = PathBuf::from("/repo");
        let resolved = target_path_from_entry(
            &entry,
            &plans_by_id_with("ignored", PathBuf::from("/repo/meta/plans/x.md")),
            &root,
        );
        assert_eq!(resolved, Some(PathBuf::from("/repo/meta/work/0033-design-token-system.md")));
    }

    #[test]
    fn validation_with_path_target_resolves_to_plan_path() {
        let mut entry = entry_for_test(DocTypeKey::Validations, "x", 0, "V");
        entry.frontmatter = serde_json::json!({
            "target": "meta/plans/2026-04-21-foo.md"
        });
        let root = PathBuf::from("/repo");
        let resolved = target_path_from_entry(
            &entry,
            &HashMap::new(),
            &root,
        );
        assert_eq!(resolved, Some(PathBuf::from("/repo/meta/plans/2026-04-21-foo.md")));
    }

    #[test]
    fn plan_review_with_typed_plan_id_resolves_via_plans_by_id() {
        let mut entry = entry_for_test(DocTypeKey::PlanReviews, "x", 0, "R");
        entry.frontmatter = serde_json::json!({
            "target": "plan:2026-05-31-0040-pipeline"
        });
        let plans = plans_by_id_with(
            "2026-05-31-0040-pipeline",
            PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md"),
        );
        let resolved = target_path_from_entry(&entry, &plans, &PathBuf::from("/repo"));
        assert_eq!(resolved, Some(PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md")));
    }

    #[test]
    fn pr_review_with_path_target_resolves_to_target_path() {
        let mut entry = entry_for_test(DocTypeKey::PrReviews, "x", 0, "PRR");
        entry.frontmatter = serde_json::json!({
            "target": "meta/prs/42-foo.md"
        });
        let resolved = target_path_from_entry(
            &entry,
            &HashMap::new(),
            &PathBuf::from("/repo"),
        );
        assert_eq!(resolved, Some(PathBuf::from("/repo/meta/prs/42-foo.md")));
    }

    #[test]
    fn entry_without_target_field_resolves_to_none() {
        let entry = entry_for_test(DocTypeKey::PlanReviews, "x", 0, "R");
        let resolved = target_path_from_entry(
            &entry,
            &HashMap::new(),
            &PathBuf::from("/repo"),
        );
        assert_eq!(resolved, None);
    }

    #[test]
    fn non_target_carrying_doc_types_return_none() {
        // Plans, Research, WorkItems etc. don't carry `target:` in
        // their per-type vocabulary; even if frontmatter has it, the
        // resolver returns None for those types.
        for kind in [DocTypeKey::Plans, DocTypeKey::Research, DocTypeKey::WorkItems] {
            let mut entry = entry_for_test(kind, "x", 0, "T");
            entry.frontmatter = serde_json::json!({ "target": "meta/plans/foo.md" });
            assert_eq!(
                target_path_from_entry(&entry, &HashMap::new(), &PathBuf::from("/repo")),
                None,
                "{kind:?} should not resolve target:",
            );
        }
    }

    #[test]
    fn typed_work_item_target_returns_none_resolved_by_cluster_key_resolver() {
        // Phase 3 deliberately leaves work-item:/adr:/pr: target
        // resolution to the cluster-key resolver (Phase 4). Pin this
        // contract so the Phase 3/Phase 4 division of labour can't
        // drift silently.
        let mut entry = entry_for_test(DocTypeKey::WorkItemReviews, "x", 0, "R");
        entry.frontmatter = serde_json::json!({ "target": "work-item:0042" });
        assert_eq!(
            target_path_from_entry(&entry, &HashMap::new(), &PathBuf::from("/repo")),
            None,
        );
    }

    #[test]
    fn typed_adr_and_pr_targets_return_none() {
        for raw in ["adr:0034", "pr:42"] {
            let mut entry = entry_for_test(DocTypeKey::PrReviews, "x", 0, "PRR");
            entry.frontmatter = serde_json::json!({ "target": raw });
            assert_eq!(
                target_path_from_entry(&entry, &HashMap::new(), &PathBuf::from("/repo")),
                None,
                "raw={raw}",
            );
        }
    }

    #[test]
    fn path_target_with_traversal_is_rejected_by_normalize_target_key() {
        // Path-safety regression guard: a target like
        // `../../etc/passwd` must NOT resolve to a path outside the
        // project root.
        let mut entry = entry_for_test(DocTypeKey::PlanReviews, "x", 0, "R");
        entry.frontmatter = serde_json::json!({ "target": "../../etc/passwd" });
        assert_eq!(
            target_path_from_entry(&entry, &HashMap::new(), &PathBuf::from("/repo")),
            None,
        );
    }

    #[test]
    fn path_target_resolves_against_supplied_project_root() {
        // Pins `normalize_target_key(raw, project_root)` argument order.
        // If the args were swapped, this test fails (the swapped call
        // would reject because project_root starts with `/`).
        let mut entry = entry_for_test(DocTypeKey::PlanReviews, "x", 0, "R");
        entry.frontmatter = serde_json::json!({ "target": "meta/plans/foo.md" });
        let resolved = target_path_from_entry(
            &entry,
            &HashMap::new(),
            &PathBuf::from("/repo/alt"),
        );
        assert_eq!(resolved, Some(PathBuf::from("/repo/alt/meta/plans/foo.md")));
    }
}
```

#### 2. Implementation

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
**Changes**: Replace `target_path_from_entry`'s gate condition. The
new gate is an allow-list of doc types that carry `target:` per
ADR-0034's type-pair table:

**Vocabulary consolidation**: the set of doc types that carry a
`target:` frontmatter key is referenced in two places (this function
and `cluster_key::walk`). Add a single source of truth on
`DocTypeKey` so a future vocabulary extension can't drift between
the two:

```rust
// In src/docs.rs:
impl DocTypeKey {
    /// True iff this doc type carries a `target:` frontmatter key
    /// per ADR-0034's type-pair table. Review/validation artifacts
    /// declare their target via `target:`; everything else uses
    /// `parent:` / `work_item_id:` / no linkage.
    pub fn carries_target_frontmatter(self) -> bool {
        matches!(
            self,
            Self::PlanReviews
                | Self::WorkItemReviews
                | Self::PrReviews
                | Self::Validations,
        )
    }

    /// True iff this doc type is part of the work-item lifecycle
    /// pipeline. These types fall back to slug-bucketing when their
    /// typed-linkage walk returns no cluster_key (legacy filename
    /// shapes). Orphan-by-design types return `false` and are kept
    /// in path-keyed buckets to prevent slug-collision merges.
    pub fn participates_in_lifecycle(self) -> bool {
        matches!(
            self,
            Self::Plans
                | Self::Research
                | Self::WorkItems
                | Self::PlanReviews
                | Self::WorkItemReviews
                | Self::PrReviews
                | Self::PrDescriptions
                | Self::Validations,
        )
    }
}
```

Tests in `docs::tests`:

```rust
#[test]
fn carries_target_frontmatter_covers_only_review_and_validation_types() {
    for k in DocTypeKey::iter() {
        let expected = matches!(
            k,
            DocTypeKey::PlanReviews
                | DocTypeKey::WorkItemReviews
                | DocTypeKey::PrReviews
                | DocTypeKey::Validations,
        );
        assert_eq!(k.carries_target_frontmatter(), expected, "{k:?}");
    }
}
```

Both `target_path_from_entry` and `cluster_key::walk` dispatch
through this predicate:

```rust
fn target_path_from_entry(
    entry: &IndexEntry,
    plans_by_id: &HashMap<String, PathBuf>,
    project_root: &Path,
) -> Option<PathBuf> {
    use crate::typed_ref::{parse_typed_ref, TypedRef};
    if !entry.r#type.carries_target_frontmatter() {
        return None;
    }
    let raw = entry.frontmatter.get("target")?.as_str()?;
    match parse_typed_ref(raw)? {
        TypedRef::Plan(id) => plans_by_id.get(&id).cloned(),
        TypedRef::Path(p) => {
            // Preserve the existing path-safety contract: reject `..`,
            // absolute, NUL, backslash; verify the joined path stays
            // under `project_root`. normalize_target_key returns the
            // verified absolute path on success.
            //
            // Signature is `normalize_target_key(raw, project_root)`
            // (indexer.rs:794) — `raw` first.
            let raw_str = p.to_str()?;
            normalize_target_key(raw_str, project_root)
        }
        // work-item / adr / pr targets are resolved by the cluster-
        // key resolver in Phase 4, not by this function.
        _ => None,
    }
}
```

#### 3. Wire contract change (callout)

Generalising `target_path_from_entry` is a behaviour change at the
`/api/related/{path}` endpoint, not a free side benefit. Three doc
types — WorkItemReviews, PrReviews, Validations — currently return
empty `declaredOutbound` arrays and never appear in any plan's
`declaredInbound`. After Phase 3:

- `/api/related/{work-item-review-path}.declaredOutbound` will contain
  the target work-item entry.
- `/api/related/{validation-path}.declaredOutbound` will contain the
  target plan entry.
- `/api/related/{pr-review-path}.declaredOutbound` will contain the
  target PR-description entry.
- `/api/related/{plan-path}.declaredInbound` may now include
  Validations (and any reviews of that plan that target it by path).
- `/api/related/{work-item-path}.declaredInbound` will include
  WorkItemReviews that target the work-item.

**Frontend fixtures requiring refresh** (verified via
`grep -rn declaredOutbound skills/visualisation/visualise/frontend/`):

- `frontend/src/routes/library/LibraryDocView.smoke.test.tsx`
- `frontend/src/api/use-related.test.tsx`
- `frontend/src/api/use-doc-page-data.test.tsx`
- `frontend/src/routes/library/RelatedArtifacts.test.tsx`

Confirm during Phase 3 manual verification: each fixture above either
already tolerates extra entries or is updated to reflect the new
shape. Phase 2's "byte-identical wire capture" check applies only to
PlanReviews; for the other three types, capture-before/capture-after
diffs are *expected* and become the affirmative success criterion.

### Success Criteria:

#### Automated Verification:

- [ ] New target-resolution tests pass: `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib target_path_resolution_tests`
- [ ] Full server unit suite passes: `mise run test:unit:visualiser`.
- [ ] Server integration suite passes: `mise run test:integration:visualiser` (this is where the `reviews_by_target` integration tests live and where the broader effect on `/api/related/*` shows up).
- [ ] Frontend test suite passes: `mise run test:unit:frontend` (no frontend change in this phase; this is a regression guard).

#### Manual Verification:

- [ ] With the dev server running, open the related-artifacts panel
  (via the UI or `/api/related/{path}`) for a **validation file** in
  `meta/validations/` and confirm the panel now shows the target plan
  as `declaredOutbound` (today: empty).
- [ ] Open the related-artifacts panel for a **work-item review** in
  `meta/reviews/work/` and confirm it shows the target work item.
- [ ] Open the related-artifacts panel for the target plan itself and
  confirm the validation appears under `declaredInbound`.

---

## Phase 4: Composite cluster-key resolver

### Overview

Introduce a `cluster_key` resolver in a new module that walks the
ADR-0034 typed-linkage chain from each entry back to a canonical
work-item id. Wire it into `compute_clusters_with_backfill` with
`cluster_key.or(slug)` bucketing so:

- Typed-linkage-carrying entries cluster by their work-item id.
- Legacy filename-only entries continue to cluster by the (now
  tightened) slug.
- `IndexEntry.cluster_key` is back-filled the same way
  `IndexEntry.completeness` already is.

The cluster representative `slug` (used as the URL identity at
`/lifecycle/<slug>`) is chosen so that work-item-rooted clusters
identify themselves by the work-item's slug.

### Changes Required:

#### 1. Tests (write first)

**File**: `skills/visualisation/visualise/server/src/cluster_key.rs`
(new module)
**Changes**: Pure resolver function with tests covering each row of
ADR-0034's type-pair table.

```rust
//! Composite cluster-key resolver. Walks ADR-0034 typed-linkage
//! frontmatter back to a canonical work-item id. Falls through to
//! `None` when no chain reaches a work item; callers fall back to
//! `IndexEntry.slug`.
//!
//! Target resolution for review/validation entries delegates to
//! `indexer::target_path_from_entry` rather than re-parsing the
//! `target:` frontmatter directly. This keeps the typed-ref
//! vocabulary parser as the single source of truth for path/Plan
//! shapes; this module owns only the `WorkItem` short-circuit and
//! the recursive walk.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::warn;

use crate::config::WorkItemConfig;
use crate::docs::DocTypeKey;
use crate::indexer::{canonicalise_one_id, target_path_from_entry, IndexEntry};
use crate::typed_ref::{parse_typed_ref, TypedRef};

/// Borrowed-entry view over the corpus. `entries_by_path` holds
/// `&IndexEntry` rather than owned values to avoid a second deep
/// clone of every entry (the `entries` slice already lives in the
/// caller's frame). The lifetime `'a` is bound to that slice.
pub fn resolve_cluster_key<'a>(
    entry: &IndexEntry,
    entries_by_path: &HashMap<PathBuf, &'a IndexEntry>,
    work_item_by_id: &HashMap<String, PathBuf>,
    plans_by_id: &HashMap<String, PathBuf>,
    project_root: &Path,
    work_item_cfg: &WorkItemConfig,
) -> Option<String> {
    walk(entry, entries_by_path, work_item_by_id, plans_by_id,
         project_root, work_item_cfg, /*depth=*/0)
}

// The longest legitimate chain in today's vocabulary is
// work-item-review → plan → parent → work-item (3 hops). 8 gives
// generous headroom for transitional shapes that bounce through
// path-target intermediaries during the epic-0057 migration window
// without measurably impacting cost (each extra hop is one HashMap
// lookup). When the limit is hit, we emit a warn-log so the
// silent-fallback case is observable.
const MAX_DEPTH: u8 = 8;

fn walk<'a>(
    entry: &IndexEntry,
    entries_by_path: &HashMap<PathBuf, &'a IndexEntry>,
    work_item_by_id: &HashMap<String, PathBuf>,
    plans_by_id: &HashMap<String, PathBuf>,
    project_root: &Path,
    work_item_cfg: &WorkItemConfig,
    depth: u8,
) -> Option<String> {
    if depth >= MAX_DEPTH {
        warn!(
            entry_path = %entry.path.display(),
            entry_type = ?entry.r#type,
            entry_slug = ?entry.slug,
            depth,
            "cluster_key walk truncated at MAX_DEPTH; entry will fall back \
             to slug bucket if a slug is present, otherwise be excluded \
             from clustering",
        );
        return None;
    }
    match entry.r#type {
        DocTypeKey::WorkItems => entry.work_item_id.clone(),
        DocTypeKey::Plans | DocTypeKey::Research | DocTypeKey::PrDescriptions => {
            parent_or_legacy_id(entry, work_item_cfg)
        }
        DocTypeKey::PlanReviews
        | DocTypeKey::WorkItemReviews
        | DocTypeKey::PrReviews
        | DocTypeKey::Validations => {
            // Note: this variant list is the same as the one tested
            // by `DocTypeKey::carries_target_frontmatter`. We keep the
            // explicit `match` here for compiler-enforced exhaustiveness
            // (a new variant added to DocTypeKey forces this branch to
            // be revisited), and pin the alignment via the test
            // `cluster_key_target_arm_matches_carries_target_predicate`
            // below.
            // First: short-circuit a typed `work-item:` target without
            // a filesystem lookup. This is the canonical ADR-0034
            // shape for work-item reviews.
            let raw = entry.frontmatter.get("target").and_then(|v| v.as_str());
            if let Some(s) = raw {
                if let Some(TypedRef::WorkItem(id)) = parse_typed_ref(s) {
                    return canonicalise_one_id(&id, work_item_cfg);
                }
            }
            // Otherwise: delegate to target_path_from_entry, which
            // owns Plan(id) / Path(p) dispatch + normalize_target_key
            // path safety. Recurse on the resulting entry.
            let target_path = target_path_from_entry(
                entry, plans_by_id, project_root,
            )?;
            // entries_by_path values are `&IndexEntry`; deref through
            // `*` so the recursion receives a `&IndexEntry`, not a
            // `&&IndexEntry`.
            let target_entry: &IndexEntry = *entries_by_path.get(&target_path)?;
            walk(target_entry, entries_by_path, work_item_by_id,
                 plans_by_id, project_root, work_item_cfg, depth + 1)
        }
        DocTypeKey::Decisions
        | DocTypeKey::Notes
        | DocTypeKey::DesignGaps
        | DocTypeKey::DesignInventories
        | DocTypeKey::Templates => None,
    }
}

/// For plans/research/pr-descriptions, accept (in priority order):
/// 1. `parent: "work-item:NNNN"`  (ADR-0034 canonical)
/// 2. `parent: "NNNN"` or bare `parent: "0042"`  (transitional)
/// 3. `work_item_id: "0042"`  (legacy frontmatter)
/// 4. `work_item_id: "meta/work/0033-foo.md"`  (legacy path shape)
fn parent_or_legacy_id(entry: &IndexEntry, cfg: &WorkItemConfig) -> Option<String> {
    if let Some(raw) = entry.frontmatter.get("parent").and_then(|v| v.as_str()) {
        if let Some(id) = id_from_value(raw, cfg) {
            return Some(id);
        }
    }
    if let Some(raw) = entry.frontmatter.get("work_item_id").and_then(|v| v.as_str()) {
        if let Some(id) = id_from_value(raw, cfg) {
            return Some(id);
        }
    }
    None
}

/// Normalise a parent/work_item_id frontmatter value to a canonical
/// work-item id. Handles three shapes routed through `parse_typed_ref`:
/// - `TypedRef::WorkItem(id)` — typed canonical form
/// - `TypedRef::Path(p)` — legacy path shape, e.g. `meta/work/0033-foo.md`
/// - bare numeric/`PROJ-NNNN` token — routed via canonicalise_one_id
fn id_from_value(raw: &str, cfg: &WorkItemConfig) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() { return None; }
    match parse_typed_ref(s) {
        Some(TypedRef::WorkItem(id)) => canonicalise_one_id(&id, cfg),
        Some(TypedRef::Path(p)) => {
            // Legacy `work_item_id: meta/work/0033-foo.md` — extract
            // the id from the filename stem.
            let stem = p.file_stem()?.to_str()?;
            cfg.extract_id(&format!("{stem}.md"))
        }
        Some(_) => None, // plan:/adr:/pr: are not work-item references
        None => canonicalise_one_id(s, cfg),
    }
}
```

**Helper consolidation**: The single-string canonicaliser
`canonicalise_one_id(raw: &str, cfg: &WorkItemConfig) -> Option<String>`
is added to `indexer.rs` alongside the existing `canonicalise_refs`,
which is refactored to call it inside its loop. This avoids the
`Vec<String> → HashSet → Vec<String>` allocation that
`canonicalise_refs(vec![one_string], cfg)` would incur on every
walk hop. The original `canonicalise_refs` API is preserved for
existing multi-input callers.

Tests under `#[cfg(test)] mod tests` should cover (one test per case):

1. WorkItem returns its own `work_item_id`.
2. Plan with `parent: "work-item:0042"` resolves to `"0042"`.
3. Plan with `parent: "0042"` resolves to `"0042"`.
4. Plan with `work_item_id: "0042"` (no parent) resolves to `"0042"`.
5. Plan with path-shape `work_item_id: "meta/work/0033-foo.md"`
   resolves to `"0033"`.
6. Plan with `work_item_id: ""` and no parent resolves to `None`.
7. Plan-review with `target:` pointing at a plan path resolves
   transitively via the plan.
8. Plan-review with `target: "plan:<id>"` resolves transitively via
   `plans_by_id`.
9. Work-item-review with `target:` pointing at a work-item path
   resolves to that work-item's id.
10. Work-item-review with `target: "work-item:NNNN"` resolves directly
    via the typed short-circuit (no path lookup).
11. Validation with `target:` pointing at a plan path resolves
    transitively (two-hop).
12. Research with no parent or work_item_id resolves to `None`
    (orphan-by-design).
13. Note / DesignGap / DesignInventory / Decision / Template all
    resolve to `None`.
14. **Depth-limit boundary, positive side**: a hand-crafted chain of
    exactly `MAX_DEPTH - 1` hops (review → review → review → ... →
    work-item) resolves to the expected `work-item id`. Pins
    `MAX_DEPTH` at the lower bound.
15. **Depth-limit boundary, negative side**: a cycle of two
    plan-reviews whose targets reference each other returns `None`
    without infinite recursion. Pins cycle safety.
16. **Depth-limit upper bound**: a chain of `MAX_DEPTH + 1` hops
    returns `None` (truncates silently in cluster_key terms; a
    `tracing::warn!` is emitted but the test does not assert on
    logs).
17. **Plan whose `parent: "work-item:NNNN"` doesn't match any
    `work_item_by_id` entry still returns the canonicalised
    `"NNNN"`** — pins that cluster_key is a *logical* id, not a
    path-lookup result. A defensive implementation that gated the
    return behind `work_item_by_id.contains_key(&id)` would demote
    these entries to slug-fallback and silently break clustering.

    ```rust
    #[test]
    fn plan_parent_resolves_to_canonical_id_even_when_work_item_missing() {
        let cfg = WorkItemConfig::default();
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        plan.frontmatter = json!({ "parent": "work-item:0099" });
        // Note: empty work_item_by_id — no entry registered for 0099.
        let resolved = resolve_cluster_key(
            &plan,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &PathBuf::from("/repo"),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0099"));
    }
    ```
18. **Project-prefix canonicalisation, numeric pattern**: under
    default `WorkItemConfig`, `parent: "42"` resolves to `"0042"`.
19. **Project-prefix canonicalisation, project pattern**: under a
    `PROJ-\d{4}` pattern, `parent: "42"` resolves to `"PROJ-0042"`;
    `parent: "PROJ-0042"` passes through verbatim.
20. **Malformed empty typed prefix**: `parent: "work-item:"` resolves
    to `None` (the parser rejects empty suffixes).
21. **Vocabulary alignment**: a test
    (`cluster_key_target_arm_matches_carries_target_predicate`) iterates
    every `DocTypeKey` variant and confirms that the resolver returns
    `None` for entries with `target:` frontmatter set iff
    `DocTypeKey::carries_target_frontmatter` returns `false` for that
    variant. Locks the explicit-list / predicate alignment against
    future drift.

Wire into `lib.rs`: `pub mod cluster_key;`

#### 2. Integrate into `clusters.rs`

**File**: `skills/visualisation/visualise/server/src/clusters.rs`
**Changes**: Replace the single-arg `compute_clusters_with_backfill`
with a `ClusterContext`-shaped signature. There is no back-compat
shim: the old signature is removed and all call sites — production
(`watcher.rs:154`, `api/docs.rs:243`, `server.rs:91`) and tests — are
updated together. A repo-wide grep confirms no external crates
consume the symbol.

```rust
/// Snapshots required by the cluster-key resolver. Built once at
/// clustering time by the indexer caller from a coherent view of
/// `entries`. Tests construct an empty context via
/// `ClusterContext::empty()` when they only exercise slug-only
/// clustering.
pub struct ClusterContext<'a> {
    pub entries_by_path: HashMap<PathBuf, &'a IndexEntry>,
    pub work_item_by_id: &'a HashMap<String, PathBuf>,
    pub plans_by_id: &'a HashMap<String, PathBuf>,
    pub project_root: &'a Path,
    pub work_item_cfg: &'a WorkItemConfig,
}

impl<'a> ClusterContext<'a> {
    /// Build `entries_by_path` from the borrowed entries slice so the
    /// snapshot is trivially coherent with the slice being clustered.
    /// Zero deep clones — entries are referenced, not owned.
    pub fn from_entries(
        entries: &'a [IndexEntry],
        work_item_by_id: &'a HashMap<String, PathBuf>,
        plans_by_id: &'a HashMap<String, PathBuf>,
        project_root: &'a Path,
        work_item_cfg: &'a WorkItemConfig,
    ) -> Self {
        let entries_by_path = entries.iter().map(|e| (e.path.clone(), e)).collect();
        Self { entries_by_path, work_item_by_id, plans_by_id, project_root, work_item_cfg }
    }

}

/// Stack-allocated empty maps + config + root, used by tests that
/// only exercise the slug-fallback path. The caller owns the
/// storage and constructs a borrowing `ClusterContext` against it.
/// Conventional Rust borrow-shaped-context idiom — no `OnceLock`,
/// no surprising static state.
#[cfg(test)]
pub struct EmptyClusterFixture {
    pub wi: HashMap<String, PathBuf>,
    pub pl: HashMap<String, PathBuf>,
    pub root: PathBuf,
    pub cfg: WorkItemConfig,
}

#[cfg(test)]
impl EmptyClusterFixture {
    pub fn new() -> Self {
        Self {
            wi: HashMap::new(),
            pl: HashMap::new(),
            root: PathBuf::new(),
            cfg: WorkItemConfig::default(),
        }
    }
    pub fn ctx(&self) -> ClusterContext<'_> {
        ClusterContext {
            entries_by_path: HashMap::new(),
            work_item_by_id: &self.wi,
            plans_by_id: &self.pl,
            project_root: &self.root,
            work_item_cfg: &self.cfg,
        }
    }
}

// Test call site looks like:
//   let fx = EmptyClusterFixture::new();
//   let (clusters, _, _) = compute_clusters_with_backfill(&entries, &fx.ctx());

pub fn compute_clusters_with_backfill(
    entries: &[IndexEntry],
    ctx: &ClusterContext<'_>,
) -> (
    Vec<LifecycleCluster>,
    HashMap<PathBuf, Completeness>,
    HashMap<PathBuf, Option<String>>,
) {
    // 1. Resolve cluster_key for every non-template entry.
    let mut cluster_key_by_path: HashMap<PathBuf, Option<String>> =
        HashMap::with_capacity(entries.len());
    for e in entries {
        if matches!(e.r#type, DocTypeKey::Templates) { continue; }
        // Adapt the borrowed-entry context to the resolver's
        // owned-HashMap signature by going through the borrowed-ref
        // view; the resolver only reads.
        let key = crate::cluster_key::resolve_cluster_key(
            e, &ctx.entries_by_path, ctx.work_item_by_id, ctx.plans_by_id,
            ctx.project_root, ctx.work_item_cfg,
        );
        cluster_key_by_path.insert(e.path.clone(), key);
    }

    // 2. Single-pass bucketing. Bucket key is cluster_key when
    //    present; otherwise slug — BUT only for types that participate
    //    in the lifecycle pipeline (Plans, Research, WorkItems,
    //    PlanReviews, WorkItemReviews, PrReviews, PrDescriptions,
    //    Validations). Orphan-by-design types (Notes, Decisions,
    //    DesignGaps, DesignInventories) get their own per-path bucket
    //    so they cannot accidentally collision-merge with unrelated
    //    entries that share a slug derivation. Templates are filtered
    //    out earlier.
    //
    // Entries are pushed into buckets by clone — but the clones'
    // `cluster_key` field is NOT written here. `apply_cluster_key_
    // backfill` is the single source of truth for that field on the
    // canonical entries map, and the cluster builder (step 3) reads
    // cluster_key from `cluster_key_by_path` when constructing each
    // `LifecycleCluster.cluster_key` and rebuilding the entries
    // it embeds. One write path, no drift.
    let mut buckets: HashMap<String, Vec<&IndexEntry>> = HashMap::new();
    for e in entries {
        if matches!(e.r#type, DocTypeKey::Templates) { continue; }
        let key = cluster_key_by_path.get(&e.path).and_then(|o| o.as_deref());
        let bucket_key = match key {
            Some(k) => Some(k.to_string()),
            None if e.r#type.participates_in_lifecycle() => e.slug.clone(),
            None => Some(format!("__orphan__::{}", e.path.display())),
        };
        let Some(k) = bucket_key else { continue };
        buckets.entry(k).or_default().push(e);
    }

    // 3. Build clusters. For each bucket, pick the representative
    //    slug (see pick_representative_slug below) and read the
    //    cluster's cluster_key from cluster_key_by_path[rep.path].
    //    Each embedded entry's cluster_key is rebuilt from the
    //    backfill map at this point — so there is exactly ONE write
    //    path for IndexEntry.cluster_key (apply_cluster_key_backfill,
    //    invoked by the Indexer's existing backfill pipeline), and
    //    the LifecycleCluster's view is reconstructed from the same
    //    map at every clustering pass.
    //
    // 4. Back-fill completeness (existing logic, unchanged).
}
```

**Resolver signature note**: the resolver takes
`&HashMap<PathBuf, &IndexEntry>` rather than
`&HashMap<PathBuf, IndexEntry>` so `ClusterContext::from_entries`
doesn't have to clone every entry. The resolver code shown earlier
(`entries_by_path.get(...)?`) dereferences the borrowed value via
`*r` where it recurses; the borrow lives as long as the entries
slice passed into clustering, so the lifetime is sound.

**Cluster representative `slug` picker** (`LifecycleCluster.slug:
String` — must be non-`None`):

```rust
fn pick_representative_slug(
    bucket: &[IndexEntry],
    cluster_key: Option<&str>,
) -> String {
    // 1. WorkItems entry's slug, if Some.
    if let Some(wi_slug) = bucket
        .iter()
        .find(|e| e.r#type == DocTypeKey::WorkItems)
        .and_then(|e| e.slug.clone())
    {
        return wi_slug;
    }
    // 2. Any entry's slug (deterministic order by path).
    let mut sorted: Vec<&IndexEntry> = bucket.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    if let Some(s) = sorted.iter().find_map(|e| e.slug.clone()) {
        return s;
    }
    // 3. Last resort: the cluster_key string itself. Guarantees the
    //    cluster URL is always derivable. A bucket reaches this
    //    branch only when every entry has slug == None AND a
    //    cluster_key was resolved (impossible under today's vocabulary
    //    but defended explicitly).
    cluster_key.unwrap_or("").to_string()
}
```

- Add `pub cluster_key: Option<String>` to `IndexEntry` (default
  `None` in `build_entry` at indexer.rs:1214-1230). Already
  camelCased on the wire by virtue of the existing serde rename.
- Add `pub cluster_key: Option<String>` to `LifecycleCluster` —
  populated from `cluster_key_by_path[rep.path]` where `rep` is the
  cluster's representative entry (work-item if present, else first
  entry by path order). The cluster's embedded entries also pick up
  their `cluster_key` from `cluster_key_by_path[entry.path]` at
  build time.
- The third returned map (`cluster_key_by_path`) is consumed by the
  Indexer's existing back-fill pipeline to set
  `IndexEntry.cluster_key` on the canonical entries map (parallel to
  how the existing `Completeness` back-fill works today). This is
  the single write path for `IndexEntry.cluster_key`; the cluster
  builder above reads from the same map rather than mutating
  per-bucket clones, so there is no risk of canonical-vs-cluster
  drift if a future refactor touches one path.

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
**Changes**: Update every call site of
`compute_clusters_with_backfill` to construct a `ClusterContext` via
`ClusterContext::from_entries(&entries, &work_item_by_id, &plans_by_id,
&project_root, &work_item_cfg)`. Call sites confirmed by grep:

- `watcher.rs:154` — post-rescan clustering
- `api/docs.rs:243` — kanban status edit refresh
- `server.rs:91` — initial cluster snapshot at server start

Add an `apply_cluster_key_backfill` method on `Indexer` that mirrors
the existing `apply_completeness_backfill`: under `entries.write()`,
iterate the returned `cluster_key_by_path` map and write
`cluster_key` onto each canonical entry.

**Concurrency-parity test** (added to `indexer.rs` tests module
alongside `refresh_one_target_migration_is_atomic_under_single_writer_lock`):

```rust
#[tokio::test]
async fn apply_cluster_key_backfill_is_atomic_under_single_writer_lock() {
    // Mirrors the refresh_one_target_migration test but exercises
    // the new backfill: a reader observing IndexEntry.cluster_key
    // between snapshot and apply must see either the pre-write or
    // post-write state, never a torn read.
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("meta/plans");
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(
        work.join("0040-pipeline.md"),
        "---\nwork_item_id: \"0040\"\n---\n",
    ).unwrap();
    let plan_path = plans.join("2026-05-31-0040-pipeline.md");
    // Initial plan has no parent — cluster_key resolves to None.
    std::fs::write(&plan_path, "---\ntitle: P\n---\n").unwrap();

    let mut map = HashMap::new();
    map.insert("plans".into(), plans.clone());
    map.insert("work".into(), work);
    let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
    let idx = Arc::new(
        Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
            .await
            .unwrap(),
    );

    let canonical_plan = std::fs::canonicalize(&plan_path).unwrap();

    // Pre-arm the rendezvous: writer signals when it reaches the
    // post-cluster-key-update barrier.
    let (reached_tx, reached_rx) = tokio::sync::oneshot::channel::<()>();
    let (proceed_tx, proceed_rx) = tokio::sync::oneshot::channel::<()>();
    idx.install_post_cluster_key_update_hook(PostClusterKeyUpdateHook {
        reached: reached_tx,
        proceed: proceed_rx,
    }).await;

    // Writer: edit plan to add `parent: work-item:0040`, then
    // refresh_one. The cluster_key for the plan migrates None → "0040".
    std::fs::write(&plan_path, "---\nparent: \"work-item:0040\"\n---\n").unwrap();
    let writer_idx = idx.clone();
    let writer_plan = canonical_plan.clone();
    let writer = tokio::spawn(async move {
        writer_idx.refresh_one(&writer_plan).await.unwrap();
    });

    reached_rx.await.expect("writer reached barrier");

    // Reader attempts to read IndexEntry.cluster_key for the plan.
    // Under the single-writer lock, it must block until the writer
    // commits — never observe a partial state.
    let reader_idx = idx.clone();
    let reader_plan = canonical_plan.clone();
    let reader = tokio::spawn(async move {
        reader_idx.get_entry(&reader_plan).await.and_then(|e| e.cluster_key)
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    proceed_tx.send(()).expect("writer awaiting proceed");
    writer.await.unwrap();

    let cluster_key = reader.await.unwrap();
    assert_eq!(
        cluster_key.as_deref(), Some("0040"),
        "post-migration: plan must carry cluster_key=0040 under the lock",
    );
}
```

The `PostClusterKeyUpdateHook` is a new test-only barrier added
alongside the existing `PostSecondaryUpdateHook`, firing inside
`apply_cluster_key_backfill` after the write-guard is acquired but
before it's dropped.

**File**: `skills/visualisation/visualise/server/src/related.rs`
**Changes**: Update `resolve_related` to look up inferred-cluster
siblings via `cluster_key` rather than `slug`, preserving the
`/api/related` ↔ `/api/lifecycle` agreement invariant:

```rust
// Previously: clusters.iter().find(|c| &c.slug == slug)
// Now: prefer cluster_key match; fall back to slug match for
// entries that resolved via the slug-fallback bucket.
let inferred = match entry.cluster_key.as_deref() {
    Some(ck) => clusters.iter().find(|c| c.cluster_key.as_deref() == Some(ck)),
    None => clusters.iter().find(|c| entry.slug.as_deref() == Some(c.slug.as_str())),
};
```

Add a `resolve_related` regression test asserting that an entry
joining a cluster via `cluster_key` (where `entry.slug != cluster.slug`)
gets a populated `inferredCluster` array.

**Phase 4 wire-change note** — `/api/related/{path}.inferredCluster`
membership changes for any entry whose cluster representative slug
diverges from the entry's own slug after Phase 4. Audit candidates
on the frontend (verified via `grep -rn inferredCluster
skills/visualisation/visualise/frontend/`):

- `frontend/src/api/use-related.test.tsx`
- `frontend/src/routes/library/RelatedArtifacts.test.tsx`
- `frontend/src/api/use-doc-page-data.test.tsx`

Confirm during Phase 4 manual verification that any tests asserting
inferredCluster contents either tolerate the new sibling set or are
updated to reflect the post-Phase-4 cluster membership.

#### 3. Cluster-key integration tests

**File**: `skills/visualisation/visualise/server/src/clusters.rs`
(tests module addition)

**Test helper** — make the snapshot construction trivial and
coherent:

```rust
#[cfg(test)]
fn run_clusters(
    entries: &[IndexEntry],
    cfg: &WorkItemConfig,
) -> (
    Vec<LifecycleCluster>,
    HashMap<PathBuf, Completeness>,
    HashMap<PathBuf, Option<String>>,
) {
    // Derive snapshot maps from the entries themselves. project_root
    // is `/repo` (the existing convention in entry_for_test).
    let work_item_by_id: HashMap<String, PathBuf> = entries
        .iter()
        .filter(|e| e.r#type == DocTypeKey::WorkItems)
        .filter_map(|e| e.work_item_id.clone().map(|id| (id, e.path.clone())))
        .collect();
    let plans_by_id: HashMap<String, PathBuf> = entries
        .iter()
        .filter(|e| e.r#type == DocTypeKey::Plans)
        .filter_map(|e| {
            e.path
                .file_stem()
                .and_then(|s| s.to_str().map(|s| (s.to_string(), e.path.clone())))
        })
        .collect();
    let project_root = PathBuf::from("/repo");
    let ctx = ClusterContext::from_entries(
        entries, &work_item_by_id, &plans_by_id, &project_root, cfg,
    );
    compute_clusters_with_backfill(entries, &ctx)
}
```

**Cases** — each gets a full assertion body. Every test asserts at
minimum: cluster count, `clusters[0].slug`, `clusters[0].cluster_key`,
member doc-type presence, and per-entry `cluster_key` backfill via
the third return value.

```rust
#[test]
fn plan_with_parent_work_item_id_clusters_with_the_work_item() {
    let cfg = WorkItemConfig::default();
    let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
    wi.work_item_id = Some("0040".into());
    wi.path = PathBuf::from("/repo/meta/work/0040-pipeline.md");
    let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
    plan.path = PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md");
    plan.frontmatter = json!({ "parent": "work-item:0040" });
    let (clusters, _, cluster_key_by_path) = run_clusters(&[wi.clone(), plan.clone()], &cfg);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].cluster_key.as_deref(), Some("0040"));
    assert_eq!(clusters[0].slug, "pipeline");
    assert!(clusters[0].entries.iter().any(|e| e.r#type == DocTypeKey::Plans));
    assert!(clusters[0].entries.iter().any(|e| e.r#type == DocTypeKey::WorkItems));
    assert_eq!(cluster_key_by_path[&wi.path], Some("0040".into()));
    assert_eq!(cluster_key_by_path[&plan.path], Some("0040".into()));
}

#[test]
fn validation_with_target_path_clusters_via_plan_parent() {
    // Two-hop walk: validation → plan (target path) → work-item (parent).
    let cfg = WorkItemConfig::default();
    let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
    wi.work_item_id = Some("0040".into());
    wi.path = PathBuf::from("/repo/meta/work/0040-pipeline.md");
    let plan_path = PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md");
    let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
    plan.path = plan_path.clone();
    plan.frontmatter = json!({ "parent": "work-item:0040" });
    let mut val = entry_for_test(DocTypeKey::Validations, "pipeline", 3, "Val");
    val.path = PathBuf::from("/repo/meta/validations/2026-05-31-pipeline-validation.md");
    val.frontmatter = json!({ "target": "meta/plans/2026-05-31-0040-pipeline.md" });
    let (clusters, _, _) = run_clusters(&[wi, plan, val.clone()], &cfg);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].cluster_key.as_deref(), Some("0040"));
    assert!(clusters[0].entries.iter().any(|e| e.path == val.path));
}

#[test]
fn work_item_review_no_date_filename_clusters_via_target() {
    // Confirms the Phase 1 slug fix and the cluster_key chain agree.
    let cfg = WorkItemConfig::default();
    let mut wi = entry_for_test(DocTypeKey::WorkItems, "design-token-system", 1, "WI");
    wi.work_item_id = Some("0033".into());
    wi.path = PathBuf::from("/repo/meta/work/0033-design-token-system.md");
    let mut review = entry_for_test(DocTypeKey::WorkItemReviews, "design-token-system", 2, "R");
    review.path = PathBuf::from("/repo/meta/reviews/work/0033-design-token-system-review-1.md");
    review.frontmatter = json!({ "target": "meta/work/0033-design-token-system.md" });
    let (clusters, _, _) = run_clusters(&[wi.clone(), review.clone()], &cfg);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].cluster_key.as_deref(), Some("0033"));
    assert!(clusters[0].entries.iter().any(|e| e.path == review.path));
}

#[test]
fn plan_without_typed_linkage_falls_back_to_slug_bucket() {
    // No parent, no work_item_id. Bucket key is slug.
    let cfg = WorkItemConfig::default();
    let plan = entry_for_test(DocTypeKey::Plans, "orphan-plan", 1, "Plan");
    let (clusters, _, cluster_key_by_path) = run_clusters(&[plan.clone()], &cfg);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].slug, "orphan-plan");
    assert_eq!(clusters[0].cluster_key, None);
    assert_eq!(cluster_key_by_path[&plan.path], None);
}

#[test]
fn legacy_work_item_id_path_shape_resolves_to_work_item_cluster() {
    let cfg = WorkItemConfig::default();
    let mut wi = entry_for_test(DocTypeKey::WorkItems, "design-token-system", 1, "WI");
    wi.work_item_id = Some("0033".into());
    wi.path = PathBuf::from("/repo/meta/work/0033-design-token-system.md");
    let mut plan = entry_for_test(DocTypeKey::Plans, "tokens", 2, "Plan");
    plan.frontmatter = json!({ "work_item_id": "meta/work/0033-design-token-system.md" });
    let (clusters, _, _) = run_clusters(&[wi, plan], &cfg);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].cluster_key.as_deref(), Some("0033"));
}

#[test]
fn project_prefixed_workspace_clusters_correctly() {
    let cfg = WorkItemConfig::with_pattern_for_test("PROJ", 4);
    let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
    wi.work_item_id = Some("PROJ-0040".into());
    let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
    plan.frontmatter = json!({ "parent": "work-item:PROJ-0040" });
    let (clusters, _, _) = run_clusters(&[wi, plan], &cfg);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].cluster_key.as_deref(), Some("PROJ-0040"));
}

#[test]
fn notes_remain_orphaned_when_they_carry_no_linkage() {
    // Notes have no linkage vocabulary; bucket key falls to a
    // per-path orphan bucket. A note's cluster keeps its slug for
    // URL purposes (via pick_representative_slug) but does NOT merge
    // with other notes (or plans) that happen to share a slug.
    let cfg = WorkItemConfig::default();
    let note = entry_for_test(DocTypeKey::Notes, "random-thought", 1, "N");
    let (clusters, _, _) = run_clusters(&[note], &cfg);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].cluster_key, None);
    assert_eq!(clusters[0].slug, "random-thought");
}

#[test]
fn orphan_types_with_colliding_slugs_do_not_merge() {
    // Pins the orphan-by-design gate: two Notes whose slug derivations
    // collide (or a Note + a Decision sharing a slug) must produce
    // separate clusters, not a single merged one.
    let cfg = WorkItemConfig::default();
    let mut note_a = entry_for_test(DocTypeKey::Notes, "shared", 1, "A");
    note_a.path = PathBuf::from("/repo/meta/notes/a.md");
    let mut note_b = entry_for_test(DocTypeKey::Notes, "shared", 2, "B");
    note_b.path = PathBuf::from("/repo/meta/notes/b.md");
    let (clusters, _, _) = run_clusters(&[note_a, note_b], &cfg);
    assert_eq!(clusters.len(), 2, "orphan-type notes must not slug-merge");
}

#[test]
fn lifecycle_type_with_no_linkage_still_slug_merges_with_work_item() {
    // Counterpart to the gate: a Plan with no parent/work_item_id
    // whose filename slug matches a WorkItems' slug DOES merge —
    // this preserves the legacy slug-fallback behaviour during the
    // epic-0057 migration window.
    let cfg = WorkItemConfig::default();
    let mut wi = entry_for_test(DocTypeKey::WorkItems, "shared-slug", 1, "WI");
    wi.work_item_id = Some("0040".into());
    let plan = entry_for_test(DocTypeKey::Plans, "shared-slug", 2, "Plan");
    let (clusters, _, _) = run_clusters(&[wi, plan], &cfg);
    assert_eq!(clusters.len(), 1);
}

#[test]
fn cluster_key_is_backfilled_onto_every_clustered_entry() {
    // Mirrors backfill_map_carries_cluster_completeness — third
    // return-map carries cluster_key for every non-template entry.
    let cfg = WorkItemConfig::default();
    let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
    wi.work_item_id = Some("0040".into());
    let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
    plan.frontmatter = json!({ "parent": "work-item:0040" });
    let (_, _, cluster_key_by_path) = run_clusters(&[wi.clone(), plan.clone()], &cfg);
    assert_eq!(cluster_key_by_path[&wi.path].as_deref(), Some("0040"));
    assert_eq!(cluster_key_by_path[&plan.path].as_deref(), Some("0040"));
}

#[test]
fn cluster_without_work_item_uses_alphabetically_first_slug() {
    // Picker rule 2: no WorkItems in bucket → deterministic path-order
    // slug.
    let cfg = WorkItemConfig::default();
    let mut a = entry_for_test(DocTypeKey::Plans, "beta-slug", 1, "A");
    a.path = PathBuf::from("/repo/meta/plans/a.md");
    let mut b = entry_for_test(DocTypeKey::Research, "alpha-slug", 2, "B");
    b.path = PathBuf::from("/repo/meta/research/b.md");
    // Both carry the same parent, forming one cluster.
    a.frontmatter = json!({ "parent": "work-item:0040" });
    b.frontmatter = json!({ "parent": "work-item:0040" });
    let (clusters, _, _) = run_clusters(&[a, b], &cfg);
    assert_eq!(clusters.len(), 1);
    // Alphabetically first by path is /repo/meta/plans/a.md → slug "beta-slug".
    assert_eq!(clusters[0].slug, "beta-slug");
}
```

#### 4. End-to-end integration test on the motivating corpus

**File**: `skills/visualisation/visualise/server/tests/api_lifecycle.rs`
**Changes**: Add a test that seeds a small fixture (work-item +
work-item-review with explicit `target:` frontmatter), boots the
indexer over it, hits `/api/lifecycle/<slug>`, and asserts the
cluster contains both entries with the right `clusterKey`.

**Note**: the existing `meta/reviews/work/2026-05-26-ac2-coverage-
review-1.md` fixture carries NO `target:` field (its file header
explicitly excludes cross-reference keys for a different acceptance
criterion). The test therefore seeds its own fixture inline via
`std::fs::write` against a `tempfile::tempdir()`, matching the
convention used by the existing tests in this file (the `seeded_cfg`
+ `AppState::build` + `oneshot Request` shape).

```rust
#[tokio::test]
async fn work_item_review_with_path_target_appears_in_work_item_cluster() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // Seed: a work-item and a work-item-review targeting it by path.
    std::fs::create_dir_all(root.join("meta/work")).unwrap();
    std::fs::create_dir_all(root.join("meta/reviews/work")).unwrap();
    std::fs::write(
        root.join("meta/work/0099-ac2-coverage.md"),
        "---\nwork_item_id: \"0099\"\ntitle: AC2 Coverage\n---\n",
    ).unwrap();
    std::fs::write(
        root.join("meta/reviews/work/0099-ac2-coverage-review-1.md"),
        "---\ntarget: meta/work/0099-ac2-coverage.md\n---\n",
    ).unwrap();

    let cfg = common::seeded_cfg(root);
    let state = AppState::build(cfg).await.unwrap();
    let app = build_router(state);
    let resp = app.oneshot(
        Request::builder()
            .uri("/api/lifecycle/ac2-coverage")
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    let body: LifecycleClusterPayload = serde_json::from_slice(
        &hyper::body::to_bytes(resp.into_body()).await.unwrap()
    ).unwrap();

    assert_eq!(body.cluster.cluster_key.as_deref(), Some("0099"));
    let kinds: Vec<DocTypeKey> = body.cluster.entries
        .iter().map(|e| e.r#type).collect();
    assert!(kinds.contains(&DocTypeKey::WorkItems));
    assert!(kinds.contains(&DocTypeKey::WorkItemReviews));
}
```

This is the wiring-bug guard: a missed call site or backfill
ordering bug would pass every unit test and still ship the
motivating bug.

### Success Criteria:

#### Automated Verification:

- [ ] New `cluster_key::tests` pass (all 20 cases): `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib cluster_key`
- [ ] New `clusters::tests` cluster-key integration tests pass (all 9 cases): `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib clusters`
- [ ] New `related::tests` regression test passes: entries joining a cluster via `cluster_key` (where `entry.slug != cluster.slug`) get populated `inferredCluster`.
- [ ] New `tests/api_lifecycle.rs` end-to-end test passes (work-item-review with path target clusters with target work-item).
- [ ] New `apply_cluster_key_backfill` concurrency-parity test passes.
- [ ] Full server unit suite passes: `mise run test:unit:visualiser`.
- [ ] Server integration suite passes: `mise run test:integration:visualiser`.
- [ ] Existing `clusters::tests` that exercise only the slug-bucketing
  path (e.g. `same_slug_clusters_into_one_entry`) continue to pass via
  the updated signature (they construct a `ClusterContext::empty()`
  explicitly).
- [ ] All three production call sites of the old
  `compute_clusters_with_backfill` (`watcher.rs:154`, `api/docs.rs:243`,
  `server.rs:91`) have been updated to construct a `ClusterContext`
  via `ClusterContext::from_entries(...)`.

#### Manual Verification:

- [ ] Dev server: visit `/lifecycle` and confirm the total cluster
  count has dropped sharply vs `main` (the broken duplicates collapse).
- [ ] Visit `/lifecycle/pipeline-visualisation-overhaul` and confirm
  the work item, plan(s), research, plan-reviews, validations and any
  work-item-reviews are listed under a single cluster.
- [ ] Pick a validation file in `meta/validations/` and confirm it now
  appears in the same cluster as its target plan and that plan's work
  item.
- [ ] Pick a work-item review in `meta/reviews/work/` (e.g.
  `0033-design-token-system-review-1.md`) and confirm it appears in
  the `design-token-system` cluster (was previously orphaned).
- [ ] Confirm no cluster shows two duplicate work-item cards (would
  indicate the representative-slug picker is wrong).
- [ ] Hit `/api/related/{path-to-a-plan}` in a cluster joined via
  `cluster_key` and confirm `inferredCluster` is populated (would be
  empty if the related.rs lookup wasn't updated).
- [ ] Confirm `LifecycleClusterView`'s 404 affordance is friendly
  (link back to `/lifecycle` index) when an old, ID-prefixed cluster
  URL is hit.

---

## Phase 5: Wire shape + frontend debug surface (optional)

### Overview

Expose `clusterKey` on the JSON wire shape for both `IndexEntry` and
`LifecycleCluster`, update frontend TypeScript types, and add a small
"clustered via" debug tag under each entry on the cluster detail view
so authors can see why an entry joined a given cluster during the
epic-0057 migration window.

**Optional** — the previous phases ship the bug fix; this phase
adds debugging affordances and an API consumer contract. Skip it if
the migration is far enough along that the debug surface is not
useful.

### Changes Required:

#### 1. Server-side wire shape

The `cluster_key` field added to `IndexEntry` and `LifecycleCluster`
in Phase 4 auto-serialises as `clusterKey` via the existing
`#[serde(rename_all = "camelCase")]` attribute. Verify with a test.

**File**: `skills/visualisation/visualise/server/src/clusters.rs`
(tests module)
**Changes**:

```rust
#[test]
fn cluster_key_field_serialises_as_camelcase_on_wire() {
    let cfg = WorkItemConfig::default();
    let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
    wi.work_item_id = Some("0042".into());
    let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
    plan.frontmatter = json!({ "parent": "work-item:0042" });
    let (clusters, _, _) = run_clusters(&[wi, plan], &cfg);
    let cluster = clusters.into_iter().next().expect("one cluster");
    let json = serde_json::to_value(&cluster).unwrap();
    assert_eq!(json["clusterKey"], "0042");
    // Confirm the field also appears on every embedded IndexEntry —
    // Phase 4's apply_cluster_key_backfill writes it onto the
    // canonical entries map; the wire serialisation must surface it.
    for entry_json in json["entries"].as_array().expect("entries array") {
        assert_eq!(entry_json["clusterKey"], "0042");
    }
}

#[test]
fn cluster_key_serialises_as_null_when_absent() {
    // Slug-only fallback: a Plan with no typed linkage has no cluster_key.
    // The field is required by the wire contract (Option<String>); when
    // None it must serialise as JSON `null`, NOT be omitted, so frontend
    // type-narrowing on `clusterKey === null` works.
    let cfg = WorkItemConfig::default();
    let plan = entry_for_test(DocTypeKey::Plans, "orphan-plan", 1, "Plan");
    let (clusters, _, _) = run_clusters(&[plan], &cfg);
    let cluster = clusters.into_iter().next().expect("one cluster");
    let json = serde_json::to_value(&cluster).unwrap();
    assert_eq!(json["clusterKey"], serde_json::Value::Null);
    // Defensive: ensure no #[serde(skip_serializing_if)] was added that
    // would drop the field entirely.
    assert!(json.as_object().unwrap().contains_key("clusterKey"));
}
```

#### 2. Cache-key version bump

**File**: `skills/visualisation/visualise/frontend/src/api/query-keys.ts`
**Changes**: Add a `'v2'` segment to the cluster-prefix and
cluster-key entries so cache entries written by the old (pre-deploy)
client become unreachable on the new code path.

```ts
export const queryKeys = {
    // ...
    lifecycleClusters: () => ['lifecycle-clusters', 'v2'] as const,
    lifecycleClusterPrefix: ['lifecycle-cluster', 'v2'] as const,
    lifecycleCluster: (slug: string) =>
        ['lifecycle-cluster', 'v2', slug] as const,
};
```

Test (in `query-keys.test.ts` if present, else inline): assert the
keys include the `'v2'` segment to lock against a future refactor
that drops it accidentally.

#### 3. Frontend TypeScript types

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts`
**Changes**: Add `clusterKey: string | null` to `IndexEntry`
([types.ts:74-115](skills/visualisation/visualise/frontend/src/api/types.ts#L74-L115))
and `clusterKey: string | null` to `LifecycleCluster`
([types.ts:204-210](skills/visualisation/visualise/frontend/src/api/types.ts#L204-L210)).
Tests in `fetch.test.ts` should assert the new field appears on the
parsed payload.

**File**: `skills/visualisation/visualise/frontend/src/api/fetch.test.ts`
**Changes**: Add `clusterKey: '0040'` to existing
`fetchLifecycleClusters` / `fetchLifecycleCluster` fixtures and assert
it round-trips. Add a `clusterKey: null` case for the slug-fallback
shape.

#### 4. Frontend debug tag

**File**: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.tsx`
**Changes**: Inside `TimelineStep` (around line 185-220), beneath the
existing id chip, render a small `<span className={styles.clusteredVia}
>` showing one of:
- `clustered via: parent → work-item:<id>` (when the entry is a Plan/
  Research/PrDescription and the cluster has `clusterKey`)
- `clustered via: target → plan → parent` (when it's a Validation or
  PlanReview)
- `clustered via: target → work-item:<id>` (when it's a WorkItemReview)
- `clustered via: slug` (when the cluster has no `clusterKey`)

The exact label is derived deterministically from `entry.type` and
the relationship between `entry.clusterKey` and `cluster.clusterKey`.
Keep the tag styled as a small muted eyebrow (reuse existing tone
tokens).

**Label-selection extracted as a pure function**: keep the
deterministic derivation in `frontend/src/lib/cluster-via-label.ts`
(or co-located with `LifecycleClusterView`) so it can be unit-tested
without rendering:

```ts
export function clusterViaLabel(
    entry: { type: DocTypeKey; clusterKey: string | null },
    cluster: { clusterKey: string | null },
): string {
    if (cluster.clusterKey === null) return 'clustered via: slug';
    const wid = `work-item:${cluster.clusterKey}`;
    switch (entry.type) {
        case 'plans':
        case 'research':
        case 'pr-descriptions':
        case 'work-items':
            return `clustered via: parent → ${wid}`;
        case 'work-item-reviews':
            return `clustered via: target → ${wid}`;
        case 'plan-reviews':
        case 'validations':
            return `clustered via: target → plan → parent`;
        case 'pr-reviews':
            return `clustered via: target → pr-description → parent`;
        default:
            return 'clustered via: slug';
    }
}
```

**File**: `skills/visualisation/visualise/frontend/src/lib/cluster-via-label.test.ts`
**Changes**: Add table-driven tests pinning each label-selection
branch. Each fixture pairs an `entry.type` + cluster.clusterKey
combination with the expected label string.

```ts
const cases: Array<[
    DocTypeKey,
    { entryKey: string | null; clusterKey: string | null },
    string
]> = [
    ['plans',              { entryKey: '0040', clusterKey: '0040' }, 'clustered via: parent → work-item:0040'],
    ['research',           { entryKey: '0040', clusterKey: '0040' }, 'clustered via: parent → work-item:0040'],
    ['validations',        { entryKey: '0040', clusterKey: '0040' }, 'clustered via: target → plan → parent'],
    ['plan-reviews',       { entryKey: '0040', clusterKey: '0040' }, 'clustered via: target → plan → parent'],
    ['work-item-reviews',  { entryKey: '0040', clusterKey: '0040' }, 'clustered via: target → work-item:0040'],
    ['pr-reviews',         { entryKey: '0040', clusterKey: '0040' }, 'clustered via: target → pr-description → parent'],
    // Slug-fallback cluster: cluster.clusterKey is null.
    ['notes',              { entryKey: null,   clusterKey: null   }, 'clustered via: slug'],
    // Mismatch case: a Plan in a slug-bucket (no cluster_key).
    ['plans',              { entryKey: null,   clusterKey: null   }, 'clustered via: slug'],
];

test.each(cases)('clusterViaLabel(%s, %o) === %s', (type, keys, expected) => {
    expect(clusterViaLabel(
        { type, clusterKey: keys.entryKey },
        { clusterKey: keys.clusterKey },
    )).toBe(expected);
});
```

**File**: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx`
**Changes**: Add a single smoke test asserting that
`TimelineStep` renders a `<span>` containing the
`clusterViaLabel(entry, cluster)` output. The detailed branch
coverage lives in `cluster-via-label.test.ts`; the TSX test only
needs to confirm the wiring.

```tsx
test('TimelineStep renders the clusterVia debug tag from clusterViaLabel', () => {
    const cluster = makeClusterFixture({ clusterKey: '0040' });
    const entry = makeEntryFixture({ type: 'plans', clusterKey: '0040' });
    render(<TimelineStep cluster={cluster} entry={entry} />);
    expect(screen.getByText('clustered via: parent → work-item:0040'))
        .toBeInTheDocument();
});
```

### Success Criteria:

#### Automated Verification:

- [ ] Server wire-shape tests pass: `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib cluster_key_field_serialises`
- [ ] Frontend type / fetch tests pass: `mise run test:unit:frontend`
- [ ] Cluster view debug-tag tests pass: included in the frontend
  unit run above.
- [ ] Full CI graph passes: `mise run test`.

#### Manual Verification:

- [ ] Dev server: visit `/lifecycle/pipeline-visualisation-overhaul`
  and confirm each entry shows its "clustered via" tag.
- [ ] Open browser devtools and inspect the `/api/lifecycle` response;
  confirm both `IndexEntry` and `LifecycleCluster` carry `clusterKey`.
- [ ] Confirm that a slug-fallback cluster (find a cluster of orphaned
  notes by shared slug) shows `clustered via: slug` on its members and
  `clusterKey: null` in the JSON.

---

## Testing Strategy

### Unit Tests

Each phase adds focused unit tests that fail before implementation and
pass after. Coverage targets:

- **slug.rs**: every accepted filename shape across every doc type
  (Phase 1).
- **typed_ref.rs**: every reference vocabulary form (Phase 2).
- **indexer.rs target_path_from_entry**: every doc-type × target-shape
  combination (Phase 3).
- **cluster_key.rs**: every row of ADR-0034's type-pair table, the
  legacy `work_item_id` shapes, the depth-limit safeguard, and the
  orphan-by-design types (Phase 4).
- **clusters.rs**: end-to-end clustering with both typed-linkage
  entries and slug-fallback entries in the same corpus (Phase 4).

### Integration Tests

`mise run test:integration:visualiser` re-runs the existing indexer
build + `/api/related/*` + `/api/lifecycle` integration suite. Phase
3's generalisation of `target_path_from_entry` triggers existing
`reviews_by_target` integration tests to cover new doc-type
combinations; assert the existing test names still pass.

### Manual Testing Steps

After each phase ships, the dev server should be exercised against the
live `meta/` corpus to confirm the visible bug for that phase has
collapsed. The most striking demonstration is Phase 4: the cluster
count on `/lifecycle` drops sharply and the `pipeline-visualisation-
overhaul` cluster gathers the work item + plan + research + reviews
into a single card.

## Performance Considerations

- **Walk cost is bounded.** `compute_clusters_with_backfill` is O(N)
  entries, with one typed-ref walk per entry. Each walk has bounded
  depth (`MAX_DEPTH = 8`) and consults already-resident HashMap
  snapshots — no additional I/O. The clustering pass remains O(N log
  N) overall (the existing per-bucket sort dominates).
- **Scheduling is unchanged.** The resolver runs at clustering time
  (not at indexer-build time), so a single full-corpus run-through
  after every `rescan` or `refresh_one`. Today
  `compute_clusters_with_backfill` already runs after every such
  event (it drives the `linked_count` back-fill); the new resolver
  shares that schedule, so no extra invocations are added.
  **Caveat**: the watcher (`watcher.rs:148`) calls full `rescan` —
  not `refresh_one` — on every fs event. So a single file save
  during dev costs O(N) file reads (the existing watcher cost)
  *plus* the new O(N·depth) typed walk on top. The typed walk is a
  tiny fraction of the rescan cost.
- **Allocation cost is small but not zero.** The plan structures
  cluster context as a borrowed-entry view (`HashMap<PathBuf,
  &IndexEntry>` built via `ClusterContext::from_entries`), so there
  is **no second deep-clone of all entries** — only one `PathBuf
  ::clone()` per entry to key the map. `canonicalise_one_id`
  (extracted from `canonicalise_refs`) operates on a single string
  without `Vec`/`HashSet` allocation, avoiding the ~2,400 transient
  heap allocations per cluster pass that the naive
  `canonicalise_refs(vec![one_string], cfg)` pattern would incur.
- **Memory.** `cluster_key_by_path` adds one `Option<String>` per
  non-template entry (~200 entries × ~16 bytes = ~3 KB). Wire-shape
  growth is ~6 KB JSON per `/api/lifecycle` response (200 entries ×
  ~15 bytes for `"clusterKey":"0040",`). Gzip compression further
  reduces both. Negligible at corpus sizes up to ~1000 entries.

## Migration Notes

- **No corpus changes.** All migration concerns are owned by epic
  0057; this plan ships entirely on the consumer side.
- **URL stability.** Cluster URLs at `/lifecycle/<slug>` will change
  for any cluster whose representative slug was previously the ID-
  prefixed form. The new URL is always the work-item slug — the
  natural anchor. **This is a breaking URL contract change**: URLs
  like `/lifecycle/0040-pipeline-visualisation-overhaul` that today
  resolve (to the broken-duplicate cluster the plan fixes) will
  start 404-ing on deploy. Bookmarks, in-corpus cross-references,
  and any external links to those URLs break silently. No redirect
  or alias is added in this plan — the rationale is that the
  broken-duplicate cluster being abandoned is itself the bug the
  plan fixes, and the dev tool's audience is small enough to absorb
  the breakage. If we observe meaningful breakage in practice, a
  follow-up plan can add a thin alias layer (look up the old slug
  against `IndexEntry.slug` and 301 to the canonical cluster).
- **Active dev-server sessions.** Developers with a cluster page
  open at deploy time will see a 404 on hot-reload. The existing
  `LifecycleClusterView` 404 affordance handles this — confirm
  during Phase 4 manual verification that the affordance is friendly
  (link back to `/lifecycle` index) rather than blank.
- **React Query cache.** Cluster-page queries are cached client-side
  keyed by `cluster.slug` (`frontend/src/api/query-keys.ts:51-52`).
  After deploy, in-memory cache entries for the old (ID-prefixed)
  slugs will linger until eviction, and a returning tab may render a
  stale-but-distinct list of clusters until refetch. Phase 5 bumps
  the cluster-query-key version segment (`['lifecycle-clusters',
  'v2']` / `['lifecycle-cluster', 'v2', slug]`) so post-deploy
  queries miss the cache and refetch fresh, sidestepping the
  staleness window. Long-running browser tabs (e.g. a developer
  leaving the visualiser open across a deploy) get a clean reload
  on next interaction without a hard refresh.
- **Phase 3 → Phase 4 transient inconsistency.** Phase 3's
  generalisation of `target_path_from_entry` widens
  `reviews_by_target` for WorkItemReviews / PrReviews / Validations
  before Phase 4 introduces cluster_key clustering. Between phases,
  the related-artifacts panel will show new declared-inbound rows
  while the cluster view still shows the old (broken-duplicate)
  clustering. Each rescan rebuilds `reviews_by_target` from scratch,
  so the inconsistency is purely surface-level UX (related panel
  says A, cluster view says B) and self-resolves when Phase 4 ships.
- **Frontend behaviour during phased rollout.** Phases 1-4 are
  server-side only; the frontend reads the same wire shape it does
  today. Phase 5 introduces the `clusterKey` field; older frontends
  ignore unknown fields gracefully (TypeScript types are widened, not
  narrowed). Server-Phase-5 ahead of frontend-Phase-5 is therefore
  safe.

## References

- Original research: `meta/research/codebase/2026-06-01-lifecycle-clustering-slug-mismatch.md`
- ADR-0033 unified base frontmatter schema: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
- ADR-0034 typed linkage vocabulary: `meta/decisions/ADR-0034-typed-linkage-vocabulary.md`
- ADR-0025 work-item cross-ref aggregation: `meta/decisions/ADR-0025-work-item-cross-ref-aggregation.md`
- Epic 0057 unified-artifact-frontmatter migration: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Work-item 0040 (motivating bug): `meta/work/0040-pipeline-visualisation-overhaul.md`
- Slug rule: `skills/visualisation/visualise/server/src/slug.rs:14-86`
- Clustering: `skills/visualisation/visualise/server/src/clusters.rs:53-103`
- Build-entry slug branch: `skills/visualisation/visualise/server/src/indexer.rs:1156-1187`
- Secondary indexes the resolver reuses: `skills/visualisation/visualise/server/src/indexer.rs:217-234`
- Existing typed-linkage consumer: `skills/visualisation/visualise/server/src/related.rs:22-79`
- Frontend cluster consumers: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.tsx`, `LifecycleClusterView.tsx`, `frontend/src/router.ts:115-131`
