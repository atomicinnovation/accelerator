---
type: plan
id: "2026-06-22-0125-search-id-matching"
title: "Visualiser Search Work-Item-ID Matching Implementation Plan"
date: "2026-06-22T15:42:50+00:00"
author: "Phil Helm"
producer: create-plan
status: draft
work_item_id: "work-item:0125"
parent: "work-item:0125"
derived_from: ["codebase-research:2026-06-22-0125-search-id-matching"]
tags: [visualiser, search, indexer]
revision: "9b00844d77584e964f34faac55fd3be02c88220d"
repository: "douala"
last_updated: "2026-06-22T16:03:30+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# Visualiser Search Work-Item-ID Matching Implementation Plan

## Overview

Make the visualiser's `/api/search` match a query against each entry's
`work_item_id`, so typing a work item's number finds the item. This is a small,
additive change to the ranking function `classify()`, fixing a gap against work
item 0054's stated-but-non-functional design (ID lookup via the exact-slug
bucket). Scope is the local `work_item_id` only.

## Current State Analysis

`classify()` in
[`skills/visualisation/visualise/server/src/api/search.rs`](../../skills/visualisation/visualise/server/src/api/search.rs)
(lines 56–80) ranks each `IndexEntry` against the lowercased query using only
three fields — `title`, `slug`, and `body_preview` — across four buckets
(`ExactSlug`, `Prefix`, `Interior`, `Body`). It never reads any ID field.

A work item's `slug` is the filename tail *after* the ID
(`0054-sidebar-search.md` → `sidebar-search`), and its numeric identity lives in
`IndexEntry.work_item_id` (populated from the frontmatter `id:` key, normalised —
`indexer.rs:1346–1382`). Because the bare ID is in none of the three searched
fields, an ID query can never match → "No matches".

`work_item_id` is **already a field** on `IndexEntry` (`indexer.rs:170`), so no
struct or indexer change is needed. Full grounding:
[`meta/research/codebase/2026-06-22-0125-search-id-matching.md`](../research/codebase/2026-06-22-0125-search-id-matching.md).

### Key Discoveries:

- `classify()` matched-field set is the sole defect site — `search.rs:56-80`.
- `IndexEntry.work_item_id: Option<String>` is already populated — `indexer.rs:170`, `indexer.rs:1346-1382`.
- The struct doc comment at `indexer.rs:167-170` is **stale** — it says the value
  is filename-derived, but it is actually read from frontmatter `id:`. Worth
  correcting while here.
- Test harness drives the real axum handler over a temp corpus — `tests/api_search.rs:13-22`.
- **Fixture gotcha**: `seeded_cfg_with_work_items` (`tests/common/mod.rs:120-141`)
  writes work items with **no `id:`** → `work_item_id: None`; it cannot exercise
  ID matching. New tests must write fixtures carrying `id:`.
- Tests require `cargo test --no-default-features --features dev-frontend` (the
  default `embed-dist` build script needs a built frontend).

## Desired End State

Typing a work item's number into the visualiser search box surfaces that item.
Specifically, in `/api/search`:

- A query equal to a work item's id — in full (`0042`), unpadded (`42`), or,
  under a project-code config, the full key (`ENG-0042`) — ranks in the top
  (`ExactSlug`) "exact identity" bucket.
- **Numeric queries are matched against the id's bare numeric form** (leading
  zeros and any project-code prefix stripped). So `42` finds `0042` *and*
  `ENG-0042` (config-independent), while `00` — which unpads to empty — matches
  no id at all. A prefix of that numeric form ranks in `Prefix`; an interior
  substring ranks in `Interior`.
- **Non-numeric queries substring-match the full lowercased id**, so `eng-0042`
  still finds `ENG-0042` across the exact/prefix/interior tiers.
- **The result set is capped at `MAX_RESULTS`**, applied *after* bucket sorting.
  This bounds any broad-match query (e.g. a single-letter or project-code
  fragment like `eng` that prefixes every id) to a fixed-size response, while the
  bucket order guarantees the truncation only ever drops the lowest-relevance
  tail — exact/prefix hits are never cut. This is the general flood backstop; we
  deliberately do **not** add per-field matching heuristics to suppress floods.
  Note this is **general search-handler hardening, not id-specific**: the cap sits
  in the shared projection chain, so it now bounds title/slug/body queries too —
  a behavioural change from the previously-unbounded result set (a corpus with
  >`MAX_RESULTS` legitimate title matches now truncates). Bucket ordering makes
  this safe (only the lowest-relevance tail is dropped), but the contract change
  is wider than the id feature that motivates it. One accepted rough edge: the
  truncation is currently unsignalled at the UI — the results meta-row shows a
  bare count (`{N} matches`), so a user whose broad query legitimately exceeds the
  cap sees exactly `MAX_RESULTS` with no "showing first N" affordance. A
  truncation indicator is the natural follow-up (alongside the id-chip work),
  out of scope here.
- Existing title/slug/body ranking and template exclusion are unchanged; all
  existing `api_search` tests still pass.

Verified by the new and existing tests in `tests/api_search.rs` and by a manual
check in the running visualiser.

## What We're NOT Doing

- Matching `external_id` / remote tracker keys (not indexed; needs new plumbing —
  deferred to a future advanced-search work item). When that work lands, the
  consistent successor is the **dedicated-field** approach (option 2 in the
  research: an `external_id: Option<String>` on `IndexEntry` populated during
  indexing, mirroring `work_item_id`), and the per-tier matching ladder in
  `classify()` should at that point be refactored to iterate a slice of
  candidate id fields per tier rather than gaining a fourth hand-written copy.
  That refactor needs a **per-field numeric-reduction policy**, not a single
  shared `numeric_key`: external keys (e.g. `PP-76`) also have a trailing digit
  run, so iterating candidate fields with one shared reduction would let a numeric
  query (`76`) collide across `work_item_id` *and* `external_id`. Decide per field
  which participate in bare-numeric matching. That refactor is also the natural
  window to rename `ExactSlug` → an "exact identity" variant: it already churns
  the ordering tests, so the rename the bucket-reuse decision defers (see below)
  pays its cost in a change that touches those tests anyway.
- Tag search, document-type scoping, a dedicated results view, or result sorting
  (advanced-search scope, untracked).
- **Gating which fields/queries can match to suppress floods** (e.g. requiring a
  numeric query to contain a digit, or anchoring the non-numeric id match to a
  prefix). A short fragment legitimately matching many items is *bounded* by the
  `MAX_RESULTS` cap rather than *prevented* by a matching heuristic — the cap is
  general (it catches flood vectors we haven't enumerated) and avoids the
  least-surprise risk of a heuristic silently refusing a query a user typed.
  Accepted consequence: under a project-code config, typing the bare code (`eng`)
  prefix-matches and returns up to `MAX_RESULTS` work items.
- **Closing the unpadded-tolerance gap for mixed-alphanumeric full keys.** Unpadded
  tolerance is *numeric-only*: a purely-numeric query is reduced via `numeric_key`,
  but a mixed query like `eng-42` takes the non-numeric branch and substring-matches
  the **full** lowercased id, so it does **not** match a zero-padded `ENG-0042`
  (`eng-42` is neither a substring nor prefix of `eng-0042`). The bare number `42`
  and the exact padded key `eng-0042` both find it; only the unpadded *prefixed*
  form misses. Accepted as a known gap for this scope — closing it would mean
  applying numeric reduction to a CODE-prefixed query's trailing digit run, which
  widens the matching behaviour. (Note `normalise_id` can also store an id
  *unpadded* as `ENG-42`, in which case `eng-42` matches exactly — so whether a
  mixed query works depends on how the id was stored; documented here rather than
  smoothed over.)
- **Surfacing the matched ID in results** — `SearchResultRow` keeps its current
  wire shape (`docType`/`title`/`slug`/`mtimeMs`). This is a **known UX rough
  edge**: an id-matched row shows only its title/slug, so a user who typed a
  number sees no on-row indication of *why* it matched (the `Highlight`
  component highlights the title only, so it highlights nothing). Accepted for
  this tightly-scoped fix; surfacing a highlightable id chip is a follow-up
  (touches the wire shape, the frontend row, and their tests).
- Renaming the `ExactSlug` bucket variant (an exact ID reuses it as the
  "exact identity" tier; renaming would churn the ordering tests for no
  behavioural gain). Instead, add a one-line doc comment on the variant stating
  it is the "exact identity" tier — exact slug **or** exact `work_item_id` — so
  the overloaded semantics are documented at the definition site.
- Changing the frontend "No matches" hint copy. It mentions "a doc id", which
  becomes truthful for **work items**; note that decisions/plans/reviews/RCAs
  carry `work_item_id: None` and so remain unmatchable by id, so the generic
  "doc id" wording slightly over-promises. Left as-is (vague enough); tightening
  toward "a work item number" is an optional follow-up.

## Implementation Approach

Single phase, test-driven (per the project's create-plan instructions: TDD where
applicable; phases independently mergeable — there is one self-contained phase).
Add `work_item_id` into `classify()` alongside `slug`, reusing the existing
exact/prefix/interior tiering, with one piece of bespoke normalisation, and add a
result cap as a general flood backstop:

- **Numeric queries match the id's bare numeric form.** For a purely-numeric
  query, both the query and the id are reduced to their **bare numeric form** —
  leading zeros and any non-digit (project-code) prefix stripped — before the
  exact/prefix/interior comparison. `42` and `0042` both reduce to `42` and
  exactly match `0042`/`ENG-0042`. This gives unpadded tolerance, makes it
  config-independent (the project-code path needs no separate handling), and
  ranks the genuinely-relevant hit highly: an exact number is `ExactSlug`, a
  prefix is `Prefix`. `00` reduces to empty and matches no id — cleaner than
  returning an arbitrary capped slice for a degenerate query.
- **Non-numeric queries substring-match the full lowercased id**, preserving the
  ability to type a full prefixed key (`eng-0042`) or a project-code fragment.
- **Cap the result set at `MAX_RESULTS`, applied after bucket sorting.** Liberal
  substring matching means a short fragment can match many entries (a bare
  project code like `eng` prefixes every id; a single digit interior-matches many
  numeric keys). Rather than gate matching with per-field heuristics — which risk
  silently refusing a query a user legitimately typed — bound the *volume*: the
  cap truncates the concatenated, bucket-sorted results, so it only ever drops
  the lowest-relevance tail (`Interior`/`Body`) and never an exact/prefix hit.
  This is general (it bounds flood vectors we haven't enumerated) and matches the
  defensive spirit of the existing `MAX_Q_LEN` cap in the same handler.

A small `numeric_key()` helper does the reduction. Correct the stale struct doc
comment in the same change. The sketches below are the intended shape; exact
mechanics are settled under TDD against the test contract in Section 1.

## Phase 1: Match `work_item_id` in `classify()`

### Overview

Extend the ranker to consult `work_item_id`, driven by failing tests first.

### Changes Required:

#### 1. Failing tests (write first)

**File**: `skills/visualisation/visualise/server/tests/api_search.rs`
**Changes**: Add a local fixture helper that writes a work item carrying an
`id:` frontmatter key (so `work_item_id` is populated), and a test contract that
pins **tier placement** (not just presence) plus the negative and flood cases.
Use `common::seeded_cfg` (which yields the default-numeric work-item config) and
create `meta/work` in the temp dir.

The bucket is not observable in the wire shape, so tier is pinned the same way
the existing `bucket_*` tests do it — by seeding a competitor that lands in a
strictly lower bucket and asserting the relative order (buckets are concatenated
in `ExactSlug < Prefix < Interior < Body` order, so a higher-tier hit always
precedes a lower-tier one regardless of the within-bucket mtime/rel_path sort).

**mtime control for the same-bucket-fallback test (item 4)**: item 4 relies on
the *within-bucket* `rel_path` tiebreak to flip the order when `id_prefix` is
deleted (the deletion drops both entries into the *same* `Interior` bucket, and
the `rel_path` ordering must then decide). The within-bucket sort key is
`(Reverse(mtime_ms), rel_path)`, so `rel_path` only breaks ties when mtimes are
equal. `write_work_item` writes files sequentially (giving them different
mtimes), so item 4 **must** pin both the target and competitor to the same
`mtime_ms` via `common::set_mtime_ms` — otherwise the later-written file sorts
first regardless of `rel_path` and the mutation does not reliably flip the order.
Items 2/3/5/9 pin a *cross-bucket* order (or detect the mutation as a
disappearance) and are robust to mtime ties without this; only item 4 needs it.

```rust
fn write_work_item(
    dir: &Path,
    id: &str,
    slug_tail: &str,
    title: &str,
) -> std::path::PathBuf {
    let filename = format!("{id}-{slug_tail}.md");
    let content =
        format!("---\nid: \"{id}\"\ntitle: \"{title}\"\n---\n# body\n");
    let path = dir.join(&filename);
    std::fs::write(&path, content).unwrap();
    path
}
```

The contract (test names indicative; each builds an `AppState` over a temp
corpus and inspects the JSON `results`, as the existing suite does):

1. **`work_item_id_fixture_is_populated`** — guards the load-bearing assumption
   the research flagged (the inverse of the `seeded_cfg_with_work_items` gotcha):
   after building the state, assert via the index snapshot that the fixture
   entry's `work_item_id` is `Some("0042")` and not `None`. If this regresses
   (e.g. a config change requires a project code), this test names the real cause
   rather than letting the search assertions fail opaquely.

2. **`work_item_id_exact_ranks_above_body`** — seed `0042-login-form` *and* a
   competitor whose **body** contains `0042` (so it lands in `Body`). Query
   `0042`; assert `results[0].slug == "login-form"` (exact identity outranks a
   body hit). The competitor proves the match is genuinely tiered, not incidental.

3. **`unpadded_numeric_query_is_exact`** — query `42` against `0042`; assert the
   item is returned and ranks `ExactSlug` (above a body-only competitor on `42`),
   since `42` reduces to the same numeric key as `0042`.

4. **`work_item_id_prefix_outranks_interior`** — query `004` (reduces to `4`)
   against target `0042` (`42` → prefix-matches `4`) **and** an interior-only
   competitor `0014-zzz` (`14` → contains `4`, does not start with it). Assert
   the target ranks **above** the competitor. This isolates the prefix branch:
   `4` is *also* an interior substring of `42`, so a presence-only assertion
   would not catch a removed `id_prefix` (the target would merely fall to
   `Interior`). The competitor's `rel_path` (`0014-…`) sorts before the target's
   (`0042-…`), so if `id_prefix` is deleted both land in `Interior` and the
   competitor sorts *first* — flipping the order and failing the test. (The
   target's title/slug must not prefix-match `004`/`4`, so the only prefix signal
   is the id.) **Pin both fixtures to the same `mtime_ms` via
   `common::set_mtime_ms`** (see "mtime control" above) — without it the
   within-bucket order is decided by mtime, not `rel_path`, and the mutation may
   not flip.

5. **`work_item_id_interior_outranks_nothing_but_below_prefix`** — query `2`
   (reduces to `2`) against target `0042` (`42` → interior-matches `2`, not
   prefix/exact) **and** a prefix competitor `0020-aaa` (`20` → prefix-matches
   `2`). Assert the target is returned and ranks **below** the competitor
   (`Interior` < `Prefix`). Isolates the interior branch: deleting `id_interior`
   drops the target to `Body` (no body match) so it disappears entirely, failing
   the "target is returned" assertion.

6. **`numeric_query_does_not_match_unrelated_id`** (negative) — query `0099`
   against `0042` (`99` is absent from `42`); assert the item is **not** in
   results.

7. **`all_zero_query_does_not_flood`** (flood guard) — seed two id-bearing work
   items (e.g. `0042`, `0001`); query `00` (reduces to empty); assert neither is
   returned. Because the bucket is unobservable, this is an *absence* assertion,
   so it also silently requires that neither fixture's title or body contains the
   substring `00` — choose titles and bodies that demonstrably exclude `00` (the
   `write_work_item` helper's fixed `# body` is fine; pass titles free of `00`),
   and comment that the assertion specifically guards the `id_active` empty-key
   branch (not incidental field contents). Pins the anti-flood behaviour.

8. **`none_id_entry_is_not_matched_by_number`** — write a second work item
   *without* an `id:` key (→ `work_item_id: None`) and content free of the query
   digits; assert a numeric query returns only the id-bearing item and never the
   `None`-id entry (and does not panic). Covers the common `None` case across the
   corpus.

9. **`project_code_config_matches_numeric_and_full_key`** — build a
   `WorkItemConfig` with a project code so the fixture's `work_item_id` is
   `ENG-0042`. Note the fixture's *filename* stays `0042-…` while the stored
   `work_item_id` is `ENG-0042` (normalisation prepends the code); the test must
   assert against the **normalised** `ENG-0042`, not the filename form. **First**
   assert (item-1 style) that the populated `work_item_id` is `Some("ENG-0042")`
   so the config→`work_item_id` wiring is pinned explicitly — then assert both
   `q=42` (numeric tail) and `q=eng-0042` (full key, case-folded) return the item,
   and `q=0042` returns it via the numeric tail (`numeric_key("eng-0042") == "42"
   == numeric_key("0042")`, not a direct string match). Confirms the unpadded
   tolerance is config-independent. (Prefer the integration form. If seeding a
   project-code config proves heavyweight and this drops to a unit test over
   `classify()`/`numeric_key()`, keep the population assertion under a
   project-code config so the wiring — the real integration risk — is still
   covered.)

10. **`results_are_capped_and_preserve_top_hits`** — seed one exact-match target
    plus more than `MAX_RESULTS` entries that only interior-match the same query
    (e.g. many work items whose numeric keys share an interior digit); query so
    all match; assert `results.len() == MAX_RESULTS` **and** the exact-match
    target is `results[0]`. Pins both the cap value and that bucket ordering
    protects high-relevance hits from truncation. Reference the `MAX_RESULTS`
    constant in the assertion (not the literal `50`, which the plan treats as
    non-contractual), and construct the >`MAX_RESULTS` flood entries so each
    *only* interior-matches the query (distinct numeric keys sharing one interior
    digit, none exact/prefix-matching it) so none perturbs `results[0]` or the
    tier split.

11. **`non_numeric_id_prefix_outranks_interior`** (non-numeric prefix branch) —
    under a project-code config (so ids are `ENG-…`), query `eng-00` against
    target `ENG-0042` (`eng-0042` starts with `eng-00` → `id_prefix`). The
    interior competitor cannot interior-match `eng-00` via its **id** — every
    normalised id has shape `eng-<digits>`, so `eng-00` can only occur at offset 0
    (a prefix), never as a non-leading substring. So the competitor must
    interior-match `eng-00` via a **non-id field**: e.g. a title containing
    `releng-00x` (and an id that does *not* prefix-match `eng-00`, e.g.
    `ENG-7777`). Assert the target ranks **above** the competitor, pinning the
    non-numeric `id_prefix` branch — note `eng-0042` also *contains* `eng-00`, so
    deleting `id_prefix` leaves the target in `Interior` (via `id_interior`)
    alongside the competitor, and the order then flips. Pin both fixtures to the
    same `mtime_ms` and make the competitor's `rel_path` sort first (same
    mechanism as item 4) so the flip is deterministic. Without this test the
    non-numeric path is exercised only at the exact tier (item 9), so a dropped or
    prefix-anchored non-numeric branch would ship green.

12. **`non_numeric_id_interior_matches`** (non-numeric interior branch) — under a
    project-code config, query an *interior* fragment of the full key (e.g.
    `ng-00` against `ENG-0042`, which contains but does not start with `ng-00`)
    against the target **and** a `Prefix` competitor. The competitor's `Prefix`
    placement must come from its **title/slug** (e.g. a title starting `ng-00…`),
    not its id — no normalised id starts with `ng-`. Assert the target is returned
    and ranks **below** the competitor (`Interior` < `Prefix`, cross-bucket —
    robust to mtime). Deleting `id_interior` for the non-numeric path drops the
    target to `Body` (no body match) so it disappears, failing the "target is
    returned" assertion.

Also add cheap **unit tests over `numeric_key()`** directly (no `AppState`):
`"0042"`→`"42"`, `"eng-0042"`→`"42"`, `"ops-7"`→`"7"` (note `normalise_id` passes
foreign prefixes through *unpadded*), `"0000"`/`""`→`""`, and an all-letters
input. These pin the riskiest new logic independently of the integration
fixtures.

Optionally extend item 9 (or add a sibling) to seed a **foreign-prefixed** id
distinct from the configured code (e.g. `OPS-7` under an `ENG` config —
`normalise_id` stores it verbatim) and assert both `q=7` and `q=ops-7` surface it.
This lifts the multi-prefix-coexistence coverage from the `numeric_key` unit level
to the end-to-end search-behaviour level.

**Location**: `numeric_key()` (and `classify()`) are private to
`src/api/search.rs`, so these unit tests **cannot** live in `tests/api_search.rs`
— that is a separate crate that sees only the library's `pub` surface. Put them
in a new `#[cfg(test)] mod tests { use super::*; … }` block inside
`src/api/search.rs` (same crate, can call the private helpers). The integration
contract (items 1–12) stays in `tests/api_search.rs`. Do **not** widen
`numeric_key`/`classify` visibility to `pub(crate)` just to test them — the
in-module test block is the idiomatic Rust approach and keeps the surface
private.

Run the new tests first; confirm the matching/ordering assertions FAIL before
the `classify()`/cap changes (the id path and cap do not exist yet), then make
them pass.

#### 2. Extend `classify()`

**File**: `skills/visualisation/visualise/server/src/api/search.rs`
**Changes**: Add a `numeric_key()` helper, and consult `work_item_id` across the
exact/prefix/interior tiers using the numeric reduction for numeric queries and
the raw lowercased id otherwise. Update the function doc comment to match.

```rust
/// Bare numeric form of an id, used for padding-tolerant numeric matching:
/// leading non-digit prefix and leading zeros stripped.
/// `"0042"` -> `"42"`, `"eng-0042"` -> `"42"`, `"0000"`/`""` -> `""`.
/// Assumes the normalised-id shape from `WorkItemConfig::normalise_id` — bare
/// digits or `CODE-digits` with an alphabetic prefix — so the digits are the
/// trailing run; a future id shape with digits elsewhere would mis-key.
fn numeric_key(id_lc: &str) -> &str {
    id_lc
        .rsplit(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap_or("")
        .trim_start_matches('0')
}

/// Classify a single entry into a ranking bucket against the lowercased query.
/// Returns `None` when no field matches.
///
/// Matched fields: `title`, `slug`, `work_item_id`, then (lazily) `body_preview`.
/// `title`, `slug` and `work_item_id` are short and lowercased eagerly;
/// `body_preview` is lowercased only on the fall-through path, avoiding the
/// largest allocation when matches come from the cheaper fields.
///
/// `work_item_id` matching: a purely-numeric query is compared against the id's
/// bare numeric form (see `numeric_key`), so `42`/`0042` exactly match
/// `0042`/`ENG-0042` and `00` (which reduces to empty) matches nothing; any other
/// query substring-matches the full lowercased id (so `eng-0042` matches
/// `ENG-0042`).
fn classify(entry: &IndexEntry, q_lc: &str) -> Option<Bucket> {
    let title_lc = entry.title.to_ascii_lowercase();
    let slug_lc = entry.slug.as_deref().map(str::to_ascii_lowercase);
    let id_lc = entry.work_item_id.as_deref().map(str::to_ascii_lowercase);

    // For a numeric query, compare query and id in bare numeric form; otherwise
    // compare against the full lowercased id. A numeric query of all zeros
    // reduces to empty and must match no id (`id_active` guards that).
    let q_is_num = q_lc.bytes().all(|b| b.is_ascii_digit());
    let id_q: &str = if q_is_num { q_lc.trim_start_matches('0') } else { q_lc };
    let id_cmp: Option<&str> =
        id_lc.as_deref().map(|id| if q_is_num { numeric_key(id) } else { id });
    let id_active = !id_q.is_empty();
    let id_exact = id_active && id_cmp == Some(id_q);
    let id_prefix =
        id_active && id_cmp.is_some_and(|id| !id.is_empty() && id.starts_with(id_q));
    let id_interior =
        id_active && id_cmp.is_some_and(|id| !id.is_empty() && id.contains(id_q));

    if slug_lc.as_deref() == Some(q_lc) || id_exact {
        return Some(Bucket::ExactSlug);
    }
    let title_prefix = title_lc.starts_with(q_lc);
    let slug_prefix = slug_lc.as_deref().is_some_and(|s| s.starts_with(q_lc));
    if title_prefix || slug_prefix || id_prefix {
        return Some(Bucket::Prefix);
    }
    let title_interior = title_lc.contains(q_lc);
    let slug_interior = slug_lc.as_deref().is_some_and(|s| s.contains(q_lc));
    if title_interior || slug_interior || id_interior {
        return Some(Bucket::Interior);
    }
    let body_lc = entry.body_preview.to_ascii_lowercase();
    if body_lc.contains(q_lc) {
        return Some(Bucket::Body);
    }
    None
}
```

(`q_lc` is non-empty here — the handler rejects empty/whitespace queries before
`classify()`, so a non-numeric query always yields a non-empty `id_q`.) Also add
the one-line "exact identity tier" doc comment on the `Bucket::ExactSlug` variant
(per *What We're NOT Doing*).

Implementer notes (settle under TDD; none change behaviour):
- **Hoist the query-level facts.** `q_is_num`/`id_q` depend only on the query, not
  the entry; optionally compute them once in the `search()` handler and pass them
  in, so `classify()` reads as "match this entry against a prepared query" (and the
  Performance section's "once per query" framing becomes literally true). The
  sketch's per-entry form is the un-hoisted starting point; this is a clarity-only
  choice (see Performance Considerations), not a measurable win.
- **Consider an `id_bucket(id_cmp, id_q) -> Option<Bucket>` extraction** if the
  `id_*` binding block reads densely — it folds the empty-guard logic (why
  `id_exact` leans on `id_active` while prefix/interior re-check `!id.is_empty()`)
  into one tested unit, mirroring the `numeric_key` extraction. Only if it stays
  simpler than the inline form.
- **Record the ladder-refactor trigger at the call site**, not only in the plan:
  a one-line comment near `classify()` noting "if a third matchable id field is
  added, refactor to iterate candidate fields per tier" so the next author meets
  the threshold in the code.

#### 3. Correct the stale struct doc comment

**File**: `skills/visualisation/visualise/server/src/indexer.rs` (lines 167–170)
**Changes**: The current comment is wrong on **two** counts — the source ("via
the scan regex") *and* the `Some`/`None` conditions ("when the filename
matches"). Replace the whole comment, e.g.:

```rust
/// Work-item identity, read from the frontmatter `id:` key and normalised via
/// `WorkItemConfig::normalise_id`. `Some(id)` only when the entry is a work item
/// whose frontmatter carries an `id:`; `None` for non-work-item types and for
/// work items lacking a frontmatter `id:`.
work_item_id: Option<String>,
```

Confirm against the population path (`indexer.rs:1346–1384`) before settling the
exact wording.

#### 4. Cap the result set

**File**: `skills/visualisation/visualise/server/src/api/search.rs`
**Changes**: Add a `MAX_RESULTS` constant and truncate the bucket-sorted results
in the `search()` handler. The truncation must come **after** the buckets are
concatenated in order, so it drops only the lowest-relevance tail:

```rust
/// Hard cap on returned rows. Bounds the response for broad-match queries (a
/// short fragment can match many entries); applied after bucket ordering so only
/// the lowest-relevance tail is dropped, never an exact/prefix hit.
const MAX_RESULTS: usize = 50;

// in search(), the projection chain:
let results: Vec<SearchResultRow> = buckets
    .into_iter()
    .flatten()
    .filter_map(|e| project(&e))
    .take(MAX_RESULTS)
    .collect();
```

Pick the constant to comfortably exceed any plausible intentional result set
while still bounding a flood (50 is a reasonable starting point; it is not a
behavioural contract beyond "bounded"). `take` is applied after `filter_map` so
slug-less entries don't consume a slot.

### Success Criteria:

#### Automated Verification:

- [x] The new tier/negative/flood/cap/non-numeric tests (contract items 1–12
      above) plus the `numeric_key` unit tests fail before the `classify()`/cap
      changes (red), pass after (green):
      `cd skills/visualisation/visualise/server && cargo test --no-default-features --features dev-frontend --test api_search`
- [x] Mutation checks hold: removing `id_prefix` flips the order in the
      prefix-outranks-interior test (item 4, with equal mtimes so the `rel_path`
      tiebreak decides); removing `id_interior` makes the target disappear in item
      5; removing the *non-numeric* `id_prefix`/`id_interior` branches is caught by
      items 11/12; the all-zeros flood-guard fails if `00` matches via the id
      path; and the cap test (item 10) fails if truncation is removed. Note: item
      10 does **not** distinguish "truncate before vs after bucket sorting" — the
      lone exact target lands at `results[0]` either way. To pin truncation
      *order*, add an assertion that seeds >`MAX_RESULTS` `Prefix` hits plus some
      `Interior` hits and checks the surviving rows are all `Prefix`-tier (fails
      if truncation precedes bucket concatenation); since the tier is not in the
      wire shape, distinguish the two groups by a slug/title marker (e.g. `pfx-NN`
      vs `int-NN`) and assert every surviving slug carries the `Prefix` marker.
      Otherwise drop the order-sensitivity claim, since the cap truncates an
      already-ordered `flatten()`.
- [x] The whole `api_search` suite passes (no regression to bucket ordering or
      template exclusion).
- [x] Server check is green (format + clippy + types): `mise run server:check`

#### Manual Verification:

- [x] In a running visualiser (`mise run dev`), focusing the sidebar search and
      typing a work item's number (e.g. `0125`) surfaces that item; typing the
      unpadded number (e.g. `125`) also surfaces it.
- [x] A broad-match query (e.g. a common two-digit fragment, or the project code
      under a project-code config) returns a bounded list rather than the whole
      corpus. (The frontend gates searches at ≥ 2 characters — `use-search.ts` —
      so a single digit never reaches the API; use a 2+ character query here.)
- [x] Title / slug / body searches behave exactly as before.

---

## Testing Strategy

### Unit / Integration Tests:

The contract in *Phase 1 → Section 1* (items 1–12): fixture-population guard,
exact-above-body, unpadded-numeric-is-exact, prefix-outranks-interior,
interior-below-prefix, the non-matching negative, the all-zeros flood guard, the
`None`-id entry, the project-code config, the result cap, and the non-numeric
prefix/interior branches — plus direct `numeric_key()` unit tests (in an
in-module `#[cfg(test)]` block, since the helper is private). Tiers are pinned by
relative ordering against an adjacent-bucket competitor (with a `rel_path`
tiebreak — backed by equal mtimes — that flips the order if a branch is
mis-tiered), not just presence, so a deleted or mis-tiered branch fails a test.

- Existing suite guards against regressions (bucket ordering, case-insensitivity,
  template exclusion, length cap, no path/relpath match).

### Manual Testing Steps:

1. `mise run dev`, open the visualiser.
2. Press `/`, type a known work item number → the item appears.
3. Type its unpadded number → still appears.
4. Type a title fragment and a slug → unchanged behaviour.

## Performance Considerations

`work_item_id` is a short `Option<String>` already on the entry; lowercasing it
adds one tiny allocation per *id-bearing work item* (the `Option::map` is a no-op
on the `None` majority — non-work-item types and id-less work items), in line with
the existing eager title/slug lowercasing. The `numeric_key` reduction and the `q_is_num`/`id_q` derivation are
borrowed-`&str` operations (no allocation); they run inside `classify()` so they
are recomputed per entry, but each is a trivial scan of the short (≤`MAX_Q_LEN`)
query string. (If desired, `q_is_num`/`id_q` could be hoisted into the handler
and passed in, computing them once per query — a readability/clarity choice, not
a measurable win.) All of it is dwarfed by the pre-existing full-index snapshot
clone in the handler (`state.indexer.all()` clones every `IndexEntry`,
frontmatter blob included), which is the only meaningful per-query cost and is
intentionally left unchanged here. The `MAX_RESULTS` cap additionally bounds the
response payload and the work done by `project()` and JSON serialisation for
broad-match queries — but note it bounds the *response*, not per-query CPU: it
runs after the snapshot clone, after every non-template entry has been
title-lowercased, and after the matched-set sort (which allocates a `rel_path`
key per match), so a flood still does O(corpus) work before truncation. At this
scale that floor is small and is the intentionally-untouched snapshot cost above.

## Migration Notes

None — no schema, data, or wire-shape changes.

`numeric_key()` is kept **query-side** (recomputed per entry per query) rather than
precomputing a searchable numeric form on `IndexEntry` at index time: that keeps
the indexer and the serialised `IndexEntry` shape untouched and colocates all
matching logic in `classify()`. The index-side precompute becomes worth revisiting
only if the per-query full-index clone is eliminated (so per-entry recomputation
stops being free relative to it) or the candidate id-field set multiplies (see the
`external_id` note in *What We're NOT Doing*) — tie the boundary move to one of
those forces rather than treating it as an open option.

## References

- Original work item: `meta/work/0125-visualiser-search-id-matching.md`
- Related research: `meta/research/codebase/2026-06-22-0125-search-id-matching.md`
- Origin of the search endpoint / non-functional design decision: `meta/work/0054-sidebar-search.md`
- Defect site: `skills/visualisation/visualise/server/src/api/search.rs:56-80`
