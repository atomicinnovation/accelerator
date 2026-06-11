---
type: codebase-research
id: "2026-06-11-0085-humanise-detail-page-h1"
title: "Research: Humanise Detail-Page H1 Across All Doc Kinds (0085)"
date: "2026-06-11T13:00:27+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0085"
parent: "work-item:0085"
relates_to: ["codebase-research:2026-05-31-0085-humanise-detail-page-h1-fallback"]
topic: "Humanise Detail-Page H1 Across All Doc Kinds (0085)"
tags: [research, codebase, visualiser, frontmatter, title-cascade, humanise-slug, slug, indexer]
revision: "34c9bb2dc2e2d3c5db3308e07c4c311041a5ed6a"
repository: ticket-management
last_updated: "2026-06-11T13:00:27+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Humanise Detail-Page H1 Across All Doc Kinds (0085)

**Date**: 2026-06-11T13:00:27+00:00
**Author**: Toby Clemson
**Git Commit**: 34c9bb2dc2e2d3c5db3308e07c4c311041a5ed6a
**Branch**: HEAD (jj workspace `ticket-management`)
**Repository**: ticket-management

## Research Question

For work item 0085 (Humanise Detail-Page H1 Across All Doc Kinds), verify
against the *current* codebase every concrete claim the work item makes —
the `title_from` cascade in `frontmatter.rs`, the `humanise_status`
precedent in `api/library.rs`, the proposed `humanise_slug` placement and
visibility, the `DocTypeKey::all()` enumeration, the indexer call site, and
the frontend consumers of `entry.title` — and surface any reuse
opportunities or design frictions the work item leaves implicit.

## Summary

**The work item's premise is sound and the fix is genuinely small**, but
fresh research surfaces three things the work item (and the earlier
2026-05-31 research) under-state:

1. **`slug.rs` already implements the prefix-stripping the work item
   proposes to write from scratch.** The crate-root `pub mod slug`
   (`server/src/slug.rs`) already contains `strip_prefix_work_item_id`,
   `strip_prefix_date_str`, and `strip_prefix_date_and_optional_id` —
   exactly the "strip a leading numeric ID or ISO date" surgery AC2 calls
   for, already unit-tested. The work item describes reimplementing this
   inline in `humanise_slug`. This is the single most important finding:
   the helper should *compose* slug.rs, not duplicate it.

2. **`humanise_status` is a poor behavioural template, and the placement
   it implies (`api/library.rs`) is the wrong home.** `humanise_status`
   only upper-cases the first character of a single token — it neither
   splits on hyphens nor title-cases each word, so `humanise_slug` shares
   almost no logic with it. More importantly, `api::library` is a
   **private** module and calling into it from `frontmatter.rs` inverts
   the dependency direction (a low-level module reaching up into the
   HTTP/`api` layer). `slug.rs` is already `pub`, already the home of slug
   string surgery, and is the natural placement — see
   [Architecture Insights](#architecture-insights) and
   [Open Questions](#open-questions).

3. **Every line number in the work item is stale**, and they have drifted
   again since the 2026-06-06 follow-up research. The structural claims
   (one function, one call site, verbatim frontend, 13 doc kinds, no
   schema change) all still hold. Refreshed references are in
   [Code References](#code-references).

Confirmed exactly as the work item states: the three-layer cascade order,
the raw-stem fallback (the one line that changes), `DocTypeKey::all()`
returning 13 variants, the stem passed to `title_from` verbatim (prefix
intact), `IndexEntry.title` a plain non-optional `String`, and the frontend
consuming `entry.title` verbatim with no client-side humanisation anywhere.

## Detailed Findings

### `title_from` — the cascade (unchanged behaviour, moved)

`skills/visualisation/visualise/server/src/frontmatter.rs:283-304`

```rust
pub fn title_from(parsed: &FrontmatterState, body: &str, filename_stem: &str) -> String {
    if let FrontmatterState::Parsed(m) = parsed {
        if let Some(v) = m.get("title") {
            if let Some(s) = v.as_str() {
                if !s.is_empty() {
                    return s.to_string();      // layer 1: frontmatter.title
                }
            }
        }
    }
    for line in body.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("# ") {
            return rest.trim().to_string();    // layer 2: first H1
        }
    }
    filename_stem.to_string()                  // layer 3: raw stem — THE LINE 0085 CHANGES
}
```

- Order is exactly `frontmatter.title` → first H1 → raw `filename_stem`, as
  the work item states. Return type `String`; `filename_stem: &str` is the
  third positional arg.
- **Layer 3 (line 303) returns the stem verbatim** — no stripping, no
  hyphen→space, no casing. This is the single line the work item replaces
  with `humanise_slug(filename_stem)`.
- **No inline comments** document the layers today, so AC5 (one comment per
  layer) is genuinely additive.
- **`title_from` takes no `DocTypeKey` and never branches on kind** — the
  cascade is kind-independent. This is load-bearing for how AC1 should be
  read (below).
- Empty-string guard at line 291 (`if !s.is_empty()`) treats an empty
  frontmatter `title:` as absent and falls through — an existing,
  *untested* branch. Worth a fixture while in the file.

### `title_cascade_*` tests — fixture style

`skills/visualisation/visualise/server/src/frontmatter.rs:470-492`

Three tests, all driven through `parse()` then `title_from(&p.state,
&p.body, <stem>)` (parse-driven, not struct-literal-driven):

- `title_cascade_prefers_frontmatter` (470-476) — both `title:` and an H1
  present → asserts `"From FM"`.
- `title_cascade_falls_back_to_first_h1` (478-484) — no `title:`, two H1s →
  asserts the **first** H1 (`"From H1"`).
- `title_cascade_falls_back_to_filename_stem` (486-492) — no frontmatter,
  no H1, stem `"2026-04-18-my-doc"` → currently asserts the **raw** stem
  passes through. **This is the test 0085 updates** to assert the humanised
  form. Two further tests then cover AC1 (the `DocTypeKey::all()` loop) and
  AC3 (cascade layers a/b unchanged).

`mod tests` uses `use super::*;`, so any module-level `humanise_slug` added
to `frontmatter.rs` is automatically reachable from these tests.

### `humanise_status` — weak template, wrong neighbourhood

`skills/visualisation/visualise/server/src/api/library.rs:252-260`

```rust
fn humanise_status(id: &str) -> String {
    let mut chars = id.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
```

- **First-character capitalisation of a single token only.** It does NOT
  split on hyphens and does NOT title-case each segment. `"in-progress"` →
  `"In-progress"`. So `humanise_slug` shares essentially no logic with it —
  the "next to `humanise_status`, mirrors its style" framing in the work
  item overstates the kinship. (The work item's "around line 236" is stale;
  the function is at 252. Line 236 is an unrelated `facet_label` arm.)
- **Private `fn`, in a private module.** `api/mod.rs:7` declares
  `mod library;` (not `pub`), and `humanise_status` itself has no
  visibility modifier. For `frontmatter.rs::title_from` to call a
  `humanise_slug` defined here, BOTH would need promoting (module to
  `pub(crate) mod library;`, fn to `pub(crate)`), **and** it would make the
  low-level `frontmatter` module depend on the HTTP/`api` layer — an
  upward/inverted dependency. See Open Questions.
- Only one call site: `facet_option_label` (`library.rs:247`), the
  `"status"` arm. No unit tests for `humanise_status` exist.

### Closer existing patterns than `humanise_status`

- **`config.rs:304-311` — `label_from_key(key: &str)`**: does
  `key.replace('-', " ")` then capitalises the *first* character of the
  whole result. Closer than `humanise_status` (it does the hyphen split),
  but still only capitalises the first word, not every segment. Neither
  existing helper title-cases every hyphen segment, so `humanise_slug`'s
  per-segment casing is genuinely new behaviour.

- **`slug.rs` — the prefix-stripping the work item wants to reinvent**
  (`skills/visualisation/visualise/server/src/slug.rs`, declared
  `pub mod slug` at `lib.rs:26`):
  - `strip_prefix_work_item_id` (73-90) — strips a leading numeric id + dash.
  - `strip_prefix_date_str` (92-111) — strips a leading ISO `YYYY-MM-DD-`.
  - `strip_optional_work_item_id_prefix` (117-130),
    `strip_prefix_date_and_optional_id` (132-148),
    `strip_suffix_review_n` (150-158).

  These are `pub mod slug` functions, already unit-tested, and implement
  exactly AC2's "strip at most one leading numeric-ID or ISO-date prefix"
  rule. `humanise_slug` can `strip_prefix_date_and_optional_id`-style
  compose these rather than hand-rolling digit/dash predicates. This both
  removes duplication and makes `slug.rs` the obvious home for the new
  helper.

### `DocTypeKey` — 13 variants confirmed; AC1 is forward-looking only

`skills/visualisation/visualise/server/src/docs.rs:4-39`

`DocTypeKey::all()` returns `[DocTypeKey; 13]` (a fixed-size array — a
compile-time count guarantee), in declaration order: Decisions, WorkItems,
Plans, Research, PlanReviews, PrReviews, WorkItemReviews, Validations,
Notes, PrDescriptions, DesignGaps, DesignInventories, Templates. **Exactly
13**, matching the work item. Three tests already pin the count
(`doc_type_key_all_returns_thirteen_variants` at 296-298, plus two others).

**Implication for AC1**: because `title_from` takes no `DocTypeKey` and
never branches on kind, an AC1 test that loops `DocTypeKey::all()` feeding
the *same* stem will run the identical code path 13 times and produce 13
identical assertions. The loop adds **no behavioural coverage today** — its
value is purely a forward-looking invariant: if a future refactor
introduces per-kind branching, the test catches any kind that regresses to
an unhumanised stem. The work item should keep the test (as specified) but
frame it as a future-proofing guard, not per-kind verification. The
concrete-literal oracle AC1 added in review (`"0042-test-fixture"` →
`"Test Fixture"`) is what gives the test its real value-level bite.

### Indexer call site — moved substantially since last research

`skills/visualisation/visualise/server/src/indexer.rs` — inside
`build_entry` (1324-1455):

- `filename_stem = filename.strip_suffix(".md").unwrap_or(filename)` (1333).
- `title_fallback_stem = slug_filename.strip_suffix(".md").unwrap_or(filename_stem)` (1407-1408).
- `let title = frontmatter::title_from(&parsed.state, &parsed.body, title_fallback_stem);` (1409).
- Stored into `IndexEntry.title` (1443).

The stem is passed **verbatim** — prefix (numeric id / ISO date) intact,
`.md` stripped, no normalisation. So `humanise_slug` does receive e.g.
`"0042-fix-login"` with its prefix, confirming the helper must do the
stripping. The work item's cited region (1018-1047) is stale — that range
is now unrelated secondary-index helpers; even the 2026-06-06 follow-up's
~1243 has drifted to 1409.

`IndexEntry` is at `indexer.rs:162`; `pub title: String` (non-optional) at
line 171. No DTO/schema change required (matches the work item's
Assumptions). `title_from`'s only non-test call site is this one.

### Frontend consumes `entry.title` verbatim — no change needed

- `frontend/src/routes/library/LibraryDocView.tsx:116` —
  `title = entry.title;` (verbatim; the WI's "~94" is the
  document-not-found branch). Passed to `<Page title={title}>` at line 214.
- `frontend/src/components/Page/Page.tsx:33` —
  `<h1 className={styles.title}>{title}</h1>` (the WI's "~31" opens the
  header row; the `<h1>` is line 33).
- A repo-wide search for `humanis|titleCase|prettif|deslug|formatTitle|
  slugToTitle` across `frontend/src` found **no** client-side title
  humanisation in the detail render path. The only hit is an unrelated
  comment in `WorkKindBadge.tsx`.
- `IndexEntry.title: string` is declared at `frontend/src/api/types.ts:124`
  and arrives from `GET /api/docs` via `fetchDocs`/`normaliseEntry`
  (`fetch.ts:100-114`), which never touches `title`.

So AC4 (server-side only; `LibraryDocView.tsx`/`Page.tsx` untouched) holds —
there is no frontend humanisation to duplicate or conflict with.

## Code References

(Refreshed at revision `34c9bb2`. Compare to the now-stale numbers in the
work item body and the 2026-05-31 research.)

- `server/src/frontmatter.rs:283-304` — `title_from` (line 303 is the raw-stem fallback to change)
- `server/src/frontmatter.rs:470-492` — `title_cascade_*` tests (486-492 is the one to update)
- `server/src/api/library.rs:252-260` — `humanise_status` (weak template; private fn in private module)
- `server/src/api/library.rs:247` — sole `humanise_status` call site (`facet_option_label`)
- `server/src/config.rs:304-311` — `label_from_key` (hyphen-split + first-word capitalise; closer pattern)
- `server/src/slug.rs:73-158` — existing prefix/suffix strippers (`strip_prefix_work_item_id`, `strip_prefix_date_str`, `strip_prefix_date_and_optional_id`, …) — **reuse for AC2**
- `server/src/lib.rs:18,26` — `pub mod frontmatter;` and `pub mod slug;` (both crate-root, public)
- `server/src/api/mod.rs:7` — `mod library;` (private — blocks the work item's stated placement)
- `server/src/docs.rs:4-39` — `DocTypeKey` enum + `all() -> [DocTypeKey; 13]`
- `server/src/indexer.rs:162,171` — `IndexEntry` / `pub title: String`
- `server/src/indexer.rs:1407-1409` — `title_fallback_stem` derivation + `title_from` call (verbatim stem)
- `frontend/src/routes/library/LibraryDocView.tsx:116,214` — `title = entry.title` → `<Page title>`
- `frontend/src/components/Page/Page.tsx:33` — `<h1>{title}</h1>`
- `frontend/src/api/types.ts:124` — `title: string` on `IndexEntry`

## Architecture Insights

- **Single-function fix surface, confirmed.** One function (`title_from`),
  one production call site (`indexer.rs:1409`), one render prop
  (`<Page title>`). The "one-line cascade-layer swap plus a helper" framing
  is accurate.
- **Placement is the real design decision, and the work item picks the
  awkward option.** `api/library.rs` is a private module in the HTTP layer;
  `humanise_slug` belongs to slug/string-derivation, which already has a
  `pub` home in `slug.rs`. Defining it there means: (a) no visibility
  promotion of `api::library`; (b) no upward `frontmatter → api`
  dependency; (c) direct reuse of the existing `strip_prefix_*` helpers;
  and (d) co-location with the canonical slug surgery. The work item's
  Technical Notes already anticipate "extraction to a `humanise.rs`
  module … deferred until a third humaniser appears" — but `slug.rs` is a
  better-fitting existing module than a new `humanise.rs` or the `api`
  layer.
- **`humanise_status` style ≠ `humanise_slug` behaviour.** Borrow the
  *idioms* (owned `String`, stdlib-only, `char::to_uppercase().collect()`
  for the per-segment capitalise) but not the structure. The per-segment
  title-case is new; `config.rs::label_from_key` is the nearer reference
  for the split-and-case shape.
- **AC1's `DocTypeKey` loop is an invariant guard, not coverage.** Since
  `title_from` is kind-agnostic, the loop's behavioural value is zero today
  and entirely forward-looking. The concrete-literal oracle (added in
  review) is what actually guards the humanised value.
- **Line-number volatility.** This surface has moved in every file across
  three research passes (2026-05-31 → 2026-06-06 → 2026-06-11). The plan
  should reference symbols, not line numbers, and re-confirm at
  implementation time.

## Historical Context

- `meta/research/codebase/2026-05-31-0085-humanise-detail-page-h1-fallback.md`
  — the prior 0085 research (with a 2026-06-06 follow-up). Confirms the
  same structure; this pass refreshes drifted line numbers and adds the
  `slug.rs` reuse finding plus the placement re-assessment. Its Open
  Question 1 (leaf-module invariant of `frontmatter.rs`) is now moot —
  `frontmatter.rs` already couples to `crate::typed_ref`, so the relevant
  objection to a cross-module call is the *dependency direction* into the
  `api` layer, not a broken leaf invariant.
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — establishes
  `title` as a base frontmatter field on every artifact type — the
  invariant 0085's *primary* cascade path relies on. **Numbering note:** the
  work item references "ADR 0060", but there is no ADR-0060. `0060` is the
  *work item* (`meta/work/0060-adr-unified-base-frontmatter-schema.md`)
  that produced **ADR-0033**. The Dependencies/References "0060 (ADR
  establishing `title` …)" wording conflates the work-item number with the
  ADR number.
- `meta/work/0041-library-page-wrapper-and-overview-hub.md` — the `<Page>`
  wrapper that makes the H1 a single render point.
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — the source design-gap that surfaced the unhumanised-stem rendering.
- Gating dependencies: `0065` (templates, **done**), `0066` (review-skill
  inline generators, **done**), `0070` (corpus migration, **draft** as of
  the prior follow-up) — 0085 remains correctly `blocked_by: 0070`. Until
  0070 runs, legacy reviews/validations still hit the humanised-slug path
  as a *load-bearing* (not merely defensive) layer.
- `meta/work/0097-strip-redundant-doc-type-prefixes-from-titles.md` — an
  adjacent follow-up explicitly kept separate from 0085 (0085 derives the
  H1; 0097 trims redundant doc-type prefixes).

## Related Research

- `meta/research/codebase/2026-05-31-0085-humanise-detail-page-h1-fallback.md`
  — prior 0085 research + 2026-06-06 follow-up (dependency-status tracking,
  six bonus `entry.title` consumers, design-inventory `HHMMSS-` prefix
  quirk in its Open Question 2 — still unaddressed and worth carrying into
  the plan).
- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
  — the Page-wrapper render path 0085 depends on.

## Open Questions

These are choices the work item leaves implicit; resolve at plan time.

1. **Where does `humanise_slug` live?** The work item says
   `api/library.rs` next to `humanise_status`. Fresh analysis argues for
   **`slug.rs`** instead: it is already `pub`, already houses the
   `strip_prefix_*` helpers AC2 needs, avoids promoting the private
   `api::library` module, and avoids a `frontmatter → api` upward
   dependency. Recommend `slug.rs` (or a new `pub mod humanise` if a
   cleaner namespace is wanted), overriding the work item's literal
   placement. This is the one substantive deviation from the work item the
   plan should call out.

2. **Reuse vs reimplement the prefix strip.** AC2's "strip at most one
   leading numeric-ID or ISO-date prefix" is already implemented by
   `slug::strip_prefix_date_and_optional_id` and friends. Compose them
   rather than hand-rolling digit/dash predicates — fewer edge cases, and
   the single-pass semantics AC2 specifies fall out naturally.

3. **Design-inventory `HHMMSS-` prefix** (carried from the prior research's
   Open Question 2). Inventory stems are `YYYY-MM-DD-HHMMSS-rest`; AC2's
   date strip only removes `YYYY-MM-DD-`, leaving a `"123456"` first token.
   The prior research recommended accepting this (rare, normally has
   `frontmatter.title`); confirm that's still the intended behaviour or add
   a fixture pinning it.

4. **`title_from` calling across modules.** Whatever the home, `title_from`
   in `frontmatter.rs` must call it. `frontmatter.rs` already imports
   `crate::typed_ref`, so a `use crate::slug::humanise_slug;` is
   in-keeping. A `frontmatter → api::library` call is the only option that
   introduces an awkward (HTTP-layer) dependency — another reason to prefer
   `slug.rs`.

5. **Empty-title branch coverage.** `title_from` already treats an empty
   `title:` as absent (line 291) but no test covers it. Cheap to add a
   fixture while the file is open.
