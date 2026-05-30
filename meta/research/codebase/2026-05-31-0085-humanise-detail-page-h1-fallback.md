---
date: 2026-05-31T22:54:27+01:00
author: Toby Clemson
git_commit: 88b3cab89687dd2ca32bc4e38fb603198e46eb92
branch: HEAD
repository: accelerator
topic: "Humanise Detail-Page H1 Across All Doc Kinds (0085)"
tags: [research, codebase, visualiser, frontmatter, title-cascade, indexer, humanise-slug]
status: complete
last_updated: 2026-06-06
last_updated_by: Toby Clemson
last_updated_note: "Added follow-up research re-verifying live codebase state and dependency statuses; 0065 and 0066 now done, 0070 still draft."
---

# Research: Humanise Detail-Page H1 Across All Doc Kinds (0085)

**Date**: 2026-05-31T22:54:27+01:00
**Author**: Toby Clemson
**Git Commit**: 88b3cab89687dd2ca32bc4e38fb603198e46eb92
**Branch**: HEAD (jj workspace `visualisation-system`)
**Repository**: accelerator

## Research Question

What is the current state of the codebase touched by work item 0085 — i.e.
the title cascade in `frontmatter.rs`, the `humanise_status` precedent in
`api/library.rs`, the `DocTypeKey::all()` enumeration, the indexer's
title-resolution call site, and the frontend consumers of `entry.title` — and
what is the status of the gating dependencies (0065, 0066, 0070) and related
header-surface work items (0074, 0078, 0080, 0084)?

## Summary

The work item's premise checks out across every reference point it names.
The fix is genuinely small:

- **One function** to change: `title_from` at `server/src/frontmatter.rs:278`
  (an 18-line, three-layer cascade with no inline comments today).
- **One helper** to add: `humanise_slug` sitting alongside `humanise_status`
  at `server/src/api/library.rs:236` (a 7-line, stdlib-only first-char
  capitaliser — the precedent is much simpler than what `humanise_slug`
  needs to do, so the new helper will be larger but stylistically aligned).
- **One enumeration** to drive table-driven tests: `DocTypeKey::all()` at
  `server/src/docs.rs:23` returns `[DocTypeKey; 13]` — the work item's count
  is exact.
- **No frontend change**. `entry.title` is rendered verbatim by
  `LibraryDocView.tsx:95` → `Page.tsx:31`. Six other surfaces (library
  index table, lifecycle clusters, kanban cards, kanban a11y announcements,
  wiki-link anchors/tooltips, related-artifacts list) will also pick up
  the humanised title for free.

The gating dependencies are **not yet shipped**: 0065 is `ready`, 0066 and
0070 are still `draft`. Of the related header-surface work, **0078 is
done**, **0074 and 0084 are in-progress**, and **0080 is draft**. The work
item correctly identifies itself as ordering-independent of 0074/0084
(server-side `entry.title` value vs. header markup/styling — disjoint
surfaces).

A few non-obvious implementation choices surface from the research that the
work item leaves implicit; they are flagged in **Open Questions** below.

## Detailed Findings

### `title_from` — current cascade

`skills/visualisation/visualise/server/src/frontmatter.rs:278-295`

The function is a single resolver chain with **no inline comments** and **no
per-kind branching**:

```rust
278  pub fn title_from(parsed: &FrontmatterState, body: &str, filename_stem: &str) -> String {
279      if let FrontmatterState::Parsed(m) = parsed {
280          if let Some(v) = m.get("title") {
281              if let Some(s) = v.as_str() {
282                  if !s.is_empty() {
283                      return s.to_string();
284                  }
285              }
286          }
287      }
288      for line in body.lines() {
289          let line = line.trim_start();
290          if let Some(rest) = line.strip_prefix("# ") {
291              return rest.trim().to_string();
292          }
293      }
294      filename_stem.to_string()
295  }
```

Layer-by-layer behaviour:

- **Layer 1** (`frontmatter.title`, lines 279–287): only consulted when
  `FrontmatterState::Parsed`; rejects non-strings and empty strings (both
  fall through). Returns the raw value — no trimming, no normalisation.
- **Layer 2** (first H1, lines 288–293): scans `body.lines()`,
  `trim_start()`s each line, and matches a literal `"# "` prefix (hash +
  single space). Returns the post-prefix remainder `.trim()`'d. Stops on
  first match.
- **Layer 3** (lines 294): unconditional `filename_stem.to_string()` — no
  transformation. **This is the line the work item replaces.**

The module is a "leaf" module: its only top-level import is
`use std::collections::BTreeMap;` (line 1). It has **zero internal-crate
coupling** — no `crate::` imports at all. Bringing in `humanise_slug` from
`api/library.rs` would establish the first cross-module dependency from
`frontmatter` into `api`.

### `title_cascade_*` tests — fixture style

`skills/visualisation/visualise/server/src/frontmatter.rs:420-442`

Three existing tests, all driven through the public `parse()` entry point
via a single `b(s: &str) -> Vec<u8>` helper (lines 363–365). No struct
literals; no direct `FrontmatterState::Parsed(BTreeMap::new())`.

```rust
420  #[test]
421  fn title_cascade_prefers_frontmatter() {
422      let raw = b("---\ntitle: From FM\n---\n# H1 Body\n");
423      let p = parse(&raw);
424      let t = title_from(&p.state, &p.body, "fallback");
425      assert_eq!(t, "From FM");
426  }
428  #[test]
429  fn title_cascade_falls_back_to_first_h1() {
430      let raw = b("---\nstatus: done\n---\n# From H1\n# Second\n");
431      let p = parse(&raw);
432      let t = title_from(&p.state, &p.body, "fallback");
433      assert_eq!(t, "From H1");
434  }
436  #[test]
437  fn title_cascade_falls_back_to_filename_stem() {
438      let raw = b("body without h1\n");
439      let p = parse(&raw);
440      let t = title_from(&p.state, &p.body, "2026-04-18-my-doc");
441      assert_eq!(t, "2026-04-18-my-doc");
442  }
```

The third test is the natural one to **update**: currently it asserts the
stem passes through unchanged; under 0085 it must assert the humanised form.
Two new tests then satisfy AC1 (per-doc-kind table-driven test over
`DocTypeKey::all()`) and AC3 (cascade layer (a) and (b) remain unchanged).

`mod tests` begins at line 361 with `use super::*;` — any module-level
helper added to `frontmatter.rs` is automatically reachable from tests.

### `humanise_status` precedent

`skills/visualisation/visualise/server/src/api/library.rs:236-242`

```rust
236  fn humanise_status(id: &str) -> String {
237      let mut chars = id.chars();
238      match chars.next() {
239          Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
240          None => String::new(),
241      }
242  }
```

Characteristics relevant to 0085:

- **Private** (`fn`, no `pub`/`pub(crate)`). For `humanise_slug` to be
  callable from `frontmatter.rs`, visibility must be widened to
  `pub(crate)` — or `humanise_slug` defined locally in `frontmatter.rs`
  (see Open Question 1).
- Returns owned `String` (not `Cow`).
- Pure stdlib — `char::to_uppercase()` + `Chars::as_str()` tail concat.
- Currently called from a single site:
  ```rust
  230  match facet_id {
  231      "status" => humanise_status(id),
  232      _ => id.to_string(),
  233  }
  ```
- **No tests exist** for `humanise_status`. The `#[cfg(test)] mod tests`
  block at lines 244–343 contains 11 tests, all exclusively targeting
  `parse_selection_query`.

Test-style precedent inside that module: `use super::*;` (line 246), free
`#[test] fn ...()` functions with descriptive snake_case names phrased as
assertion sentences. No parametrisation; no fixture helpers; bare
`assert_eq!`. Adding `humanise_slug` unit tests at the end of that block
(e.g. after line 342) matches the precedent.

### `api/library.rs` module shape and dep audit

Five contiguous bands top-to-bottom (1–9 imports; 11–70 wire types; 72–109
static phase table; 111–147 axum handler + query parser; 149–242 private
helper cluster ending in `humanise_status`; 244–343 `mod tests`).
`humanise_slug` lands naturally as a sibling of `humanise_status` (insert
between line 242 and 243).

**No string-manipulation crates currently imported** in this file: no
`heck`, `convert_case`, `inflections`, `titlecase`, `Inflector`,
`unicode-segmentation`. The only relevant `Cargo.toml` deps are
`regex = "1"` (already on the crate, unused in this file) and
`form_urlencoded = "1"` (for query parsing only). Implementing
`humanise_slug` with stdlib only — to mirror `humanise_status`'s style —
adds no new dependency. The "leading numeric ID / ISO date" prefix strip
needed by AC2 can be done with stdlib pattern matching (single-pass: check
ISO-date prefix first via fixed-width `chars().take(10)` digit/`-`
predicates, then numeric-ID prefix via leading-digit + `-` check), or with
`regex::Regex` (already on the crate). The work item is silent on which.

### `DocTypeKey` enum — 13 variants confirmed

`skills/visualisation/visualise/server/src/docs.rs:4-39`

Enum derives `Copy`, has serde `rename_all = "kebab-case"`. Confirmed 13
variants in declaration order, matching the work item exactly:

```
Decisions, WorkItems, Plans, Research,
PlanReviews, PrReviews, WorkItemReviews, Validations,
Notes, PrDescriptions, DesignGaps, DesignInventories,
Templates
```

`all()` returns `[DocTypeKey; 13]` (fixed-size array, not a Vec/iterator).
This makes the AC1 table-driven test a literal `for kind in
DocTypeKey::all() { ... }` loop. There is **no `Display` impl** and **no
`as_str()`** on `DocTypeKey` — there is a `wire_str()` (lines 108–124,
kebab-case) and a `label()` (lines 62–78, sentence-case English label).
Neither is needed for the AC1 test, which only iterates the enum to ensure
every variant resolves identically through `title_from`.

### Indexer integration

`skills/visualisation/visualise/server/src/indexer.rs:1016-1047`

The actual range is slightly wider than the work item's stated 1018–1047.
The title-resolution flow:

```rust
1016  let parsed = frontmatter::parse(&content.bytes);
1017  let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
1018  let filename_stem = filename.strip_suffix(".md").unwrap_or(filename);
1019
1020  // Nested-manifest branch (DesignInventories): rewrite slug_filename
1021  // to "<parent-dir>.md" so the title fallback uses the dated parent
1022  // dir name instead of the literal "inventory" manifest name.
1023  let slug_filename: String = if kind.nested_manifest_filename().is_some() { ... }
1024                              else { filename.to_string() };
1032  // WorkItems branch: regex-driven slug + work_item_id extraction.
1033  // Does NOT affect title_fallback_stem (computed from slug_filename
1034  // above, which is unconditional w.r.t. this branch).
1042  // Title fallback uses the slug-source stem so nested kinds (where
1043  // the manifest filename is just "inventory") get a meaningful default.
1044  let title_fallback_stem = slug_filename
1045      .strip_suffix(".md")
1046      .unwrap_or(filename_stem);
1047  let title = frontmatter::title_from(&parsed.state, &parsed.body, title_fallback_stem);
```

Critical observation: `title_fallback_stem` is passed **verbatim** —
no lowercasing, no normalisation. So the string `humanise_slug` will
receive looks like:

- Decisions / plans / research / reviews / etc.:
  `"2026-05-31-0085-humanise-detail-page-h1"`,
  `"0042-templates-view-redesign-review-1"`
- Design inventories: `"2026-05-29-120000-some-id"` (parent dir, includes
  a 6-digit time component after the date)
- Notes / templates: arbitrary slug shapes including bare names like
  `"notes"`

This confirms the AC2 prefix-handling examples are the right shape, and
exposes a subtle case the work item doesn't mention explicitly: design
inventories pass `"YYYY-MM-DD-HHMMSS-..."` — the ISO-date prefix strip
defined in the AC handles only the `YYYY-MM-DD-` prefix, so the `HHMMSS-`
remainder will appear as a `"123456"`-shaped first word after humanising.
See **Open Question 2**.

### `IndexEntry` wire shape

`skills/visualisation/visualise/server/src/indexer.rs:161-180`

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    ...
    pub title: String,
    ...
}
```

`title: String` is non-optional and always populated by `title_from`
(line 1047). No schema change required (matches the work item's
Assumptions section).

### Frontend consumers of `entry.title` — verbatim everywhere

The work item's assertion is confirmed; the frontend changes nothing.

**Detail-page H1 path**:

- `frontend/src/routes/library/LibraryDocView.tsx:95` —
  `title = entry.title` (verbatim assignment, no transformation).
- `frontend/src/components/Page/Page.tsx:31` —
  `<h1 className={styles.title}>{title}</h1>`.

**Other surfaces that will also benefit** (verbatim today; will inherit a
humanised title for free):

| File | Line | Surface |
|---|---|---|
| `LibraryDocView.tsx` | 95 | Detail-page H1 |
| `LibraryTypeView.tsx` | 248 | Library index table title cell (also sorted by title at 63/67) |
| `LifecycleClusterView.tsx` | 134 | Lifecycle cluster entry list |
| `WorkItemCard.tsx` | 54 | Kanban card body title |
| `announcements.ts` | 25 | Kanban drag-drop ARIA announcements |
| `RelatedArtifacts.tsx` | 97 | Related artifacts list (`entry.title \|\| entry.relPath`) |
| `LibraryOverviewHub.tsx` | 83 | "Latest · …" overview line |
| `api/wiki-links.ts` | 192 | Anchor text + `title=` hover tooltip on wiki links |

**Document/browser `<title>` is unaffected**: `frontend/index.html:6`
ships the static literal `<title>Accelerator Visualiser</title>`; no
`document.title` / `useTitle` / `Helmet` is used anywhere in the
frontend. Breadcrumbs do not render `entry.title` either — the eyebrow
above the H1 is the doc-type label via `<EyebrowLabel type={type} />`
(`LibraryDocView.tsx:159`).

### Dependency status

| ID | Title | Status | Notes |
|---|---|---|---|
| **0065** | Update All Artifact Templates to Unified Schema | `ready` | Closest to landing; gates 0066 (for `validation.md`) and 0070. |
| **0066** | Move Review/Validation Skills' Frontmatter into Templates | `draft` | Blocked by 0060, 0061, 0065. |
| **0070** | Ship `meta/` Corpus Unified-Schema Migration | `draft` | Blocked by 0060–0066 chain. |
| **0074** | Per-Doc-Type Hues on Detail Page | `in-progress` | Header surface (eyebrow + aside icons), not title. |
| **0078** | Detail-Page Frontmatter Table | `done` | Independent of 0085. |
| **0080** | Detail-Page Header Actions | `draft` | Header surface (actions slot), not title. |
| **0084** | Detail-Page Chip Strip Cap (Status, Date, Author) | `in-progress` | Header surface (subtitle slot), not title. |

**Implication**: none of the three gating dependencies (0065/0066/0070)
are shipped yet, so 0085's humanised-slug path is currently the **primary**
derivation for three doc kinds (work-item-reviews, plan-reviews,
validations) that lack both `frontmatter.title` and a first H1. The work
item's framing of the fallback as a "defensive belt-and-braces layer"
holds *eventually* (once 0065+0066+0070 land), but **on the day 0085
lands it will be the load-bearing path for those three kinds**. This
strengthens the case for the work-item's own gating clause ("Blocked by:
0065, 0066, 0070") — if 0085 lands before those, its tests still pass
but the user-visible improvement on reviews/validations is the headline
effect, not the edge-case backstop.

## Code References

- `skills/visualisation/visualise/server/src/frontmatter.rs:278-295` — `title_from` (the function to change)
- `skills/visualisation/visualise/server/src/frontmatter.rs:420-442` — existing `title_cascade_*` tests (one to update, two unchanged)
- `skills/visualisation/visualise/server/src/frontmatter.rs:1` — single top-level import (`BTreeMap`); no `crate::` coupling
- `skills/visualisation/visualise/server/src/frontmatter.rs:361-365` — `mod tests` header + `b(s)` fixture helper
- `skills/visualisation/visualise/server/src/api/library.rs:236-242` — `humanise_status` precedent (visibility/style template)
- `skills/visualisation/visualise/server/src/api/library.rs:230-233` — only existing call site for `humanise_status` (facet label)
- `skills/visualisation/visualise/server/src/api/library.rs:244-343` — `mod tests`; insertion point for `humanise_slug` tests
- `skills/visualisation/visualise/server/Cargo.toml:52` — `regex = "1"` already on the crate
- `skills/visualisation/visualise/server/src/docs.rs:4-20` — `DocTypeKey` enum (13 variants, serde kebab-case)
- `skills/visualisation/visualise/server/src/docs.rs:23-39` — `DocTypeKey::all()` returns `[DocTypeKey; 13]`
- `skills/visualisation/visualise/server/src/indexer.rs:1016-1047` — title-resolution call site (`title_fallback_stem` derivation)
- `skills/visualisation/visualise/server/src/indexer.rs:161-180` — `IndexEntry` (wire shape with `title: String`)
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:95` — `title = entry.title` (verbatim, the H1 source)
- `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx:31` — `<h1>{title}</h1>`

## Architecture Insights

- **Single-function fix surface**. The detail-page H1 cascade is one
  function (`title_from`), called from one site (`indexer.rs:1047`),
  rendered through one prop (`<Page title={entry.title} />`). The
  work item's "one-line cascade-layer swap" framing is accurate.
- **Leaf-module isolation of `frontmatter.rs`**. The module has zero
  `crate::` imports. Pulling `humanise_slug` from `api/library.rs` would
  introduce the first such coupling — feasible but worth flagging. The
  alternative (define `humanise_slug` inside `frontmatter.rs`) keeps the
  module standalone at the cost of physical co-location with
  `humanise_status`. See **Open Question 1**.
- **Test-fixture style is parse-driven, not constructor-driven**. Every
  `title_cascade_*` test calls `parse(&b("..."))` to materialise inputs.
  AC1's table-driven test over `DocTypeKey::all()` does **not** need to
  vary the body — only the `DocTypeKey` (and even that doesn't currently
  affect `title_from` since the function takes no `kind` parameter). The
  AC1 test therefore exists primarily as a **future-proofing** guard:
  if a later refactor introduces per-kind branching in the cascade, the
  test catches any kind that regresses to an unhumanised stem.
- **`DocTypeKey` is the right enumeration knob**. Iterating
  `DocTypeKey::all()` for AC1 doesn't actually drive `title_from` (which
  is kind-agnostic) — it asserts the invariant *across kinds* that no
  detail-page route can render an unhumanised stem. The same loop
  structure could become a stronger test if/when `title_from` grows
  kind-awareness, which is precisely the regression hazard the AC1 test
  is defending against.
- **Six bonus consumers** of `entry.title` (library table, lifecycle
  clusters, kanban cards, kanban a11y, wiki links, related artifacts)
  will inherit the humanised title automatically. This isn't in scope for
  0085 but is worth mentioning in the PR description, as it's the
  user-visible "halo" of the change.
- **Design inventory time-stamp quirk**. The slug source for design
  inventories includes a 6-digit time component
  (`YYYY-MM-DD-HHMMSS-...`). The AC2 prefix strip rules as written only
  strip the date — see Open Question 2.

## Historical Context

- `meta/decisions/0033-base-frontmatter-schema.md` (referenced by 0085 as
  ADR-0060 in the work item's References; ADR numbering may have shifted
  during the unified-schema epic) — establishes `title` as a base field
  on every artifact type, which is the invariant 0085's primary cascade
  path relies on.
- `meta/work/0041-page-wrapper.md` — Page wrapper standardised across
  detail routes (`<Page title={title}>` → `<h1>`). The "single rendering
  point" assertion in 0085 depends on this work.
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — original source that surfaced the unhumanised-stem rendering bug.
- No prior research document exists in `meta/research/codebase/` for the
  title cascade, frontmatter helpers, or `DocTypeKey` enumeration —
  this is the first codebase-research artifact in that area.

## Related Research

- `meta/research/codebase/2026-03-18-meta-management-strategy.md` — broader
  meta-directory architecture; tangential.
- `meta/research/codebase/2026-04-08-ticket-management-skills.md` — ticket
  lifecycle work that produced the unified-frontmatter epic the gating
  dependencies belong to; tangential.

## Open Questions

These are choices the work item leaves implicit; flagging here so they
can be resolved at plan time rather than during code review.

1. **Where does `humanise_slug` live, and what visibility?**
   The work item says "next to `humanise_status` in `api/library.rs:236`".
   That requires widening `humanise_status` and `humanise_slug` to at
   least `pub(crate)`, and adding `use crate::api::library::humanise_slug;`
   to the top of `frontmatter.rs` (which currently has zero `crate::`
   coupling). Three options:
   - **(a)** Add `pub(crate) fn humanise_slug` in `api/library.rs`,
     import in `frontmatter.rs`. Matches work-item literal text. Breaks
     `frontmatter.rs`'s leaf-module invariant.
   - **(b)** Define `humanise_slug` in `frontmatter.rs` (private to the
     module). Preserves leaf invariant but diverges from work-item
     instruction; future extraction to a `humanise.rs` module (deferred
     per the work item's Technical Notes) becomes a one-file move.
   - **(c)** Create `humanise.rs` now, move both helpers. The work item
     explicitly defers this until a third humaniser appears, so this is
     against intent — but is the cleanest end state.
   Recommend **(a)** to match work-item intent literally; flag the leaf
   coupling for reviewer.

2. **Design-inventory time-stamp prefix.** Inventory stems have shape
   `YYYY-MM-DD-HHMMSS-rest` (e.g. `2026-05-29-120000-source-id`). AC2's
   prefix rules only strip leading `YYYY-MM-DD-`. After the strip:
   `"120000 Source Id"` becomes the humanised title. Three options:
   - **(a)** Treat as out-of-scope — the inventory parent-dir scheme is
     unusual, and these documents normally have `frontmatter.title`
     populated already; the humanised-slug fallback is the defensive
     layer that this case rarely hits in practice.
   - **(b)** Extend the leading-ISO-date strip to optionally swallow a
     trailing `HHMMSS-` segment, so the visible result is `"Source Id"`.
   - **(c)** Strip *any* leading dash-delimited segment that is purely
     digits and at least 4 digits long, repeatedly (single-pass would
     drop both date and time). AC2 explicitly forbids this — see the
     `"2026-05-21-0042-foo"` example which expects `"0042 Foo"`, not
     `"Foo"`.
   Recommend **(a)** — silent acceptance of `"120000 Source Id"` for the
   rare malformed-inventory case. The cleanup work belongs to the
   nested-manifest branch in `indexer.rs` (or to a separate work item)
   rather than to slug humanisation.

3. **Stdlib vs `regex` for prefix detection.** `regex` is already on the
   crate (`Cargo.toml:52`) but unused in `library.rs`. AC2's prefix
   matching is small enough to do in stdlib (~10 lines:
   `s.split_once('-')` plus digit predicates). `humanise_status`'s
   stdlib-only style argues for stdlib; reaching for `regex` would be
   the heavier-handed choice for one helper. Recommend stdlib.

4. **Single-segment slug capitalisation rule.** AC2 says
   `humanise_slug("notes") == "Notes"`. The `humanise_status`-style
   first-char-uppercase achieves this. But `humanise_slug` also needs
   per-segment capitalisation, so the natural implementation is
   `split('-').map(capitalise_first).collect::<Vec<_>>().join(" ")`. A
   `capitalise_first` private helper inside `humanise_slug` (or extracted
   to a sibling fn) would let both helpers share the casing primitive —
   marginal cleanup, no behaviour change. Not required by the AC but
   worth noting for the simplify pass.

5. **AC1 test value-add given that `title_from` is kind-agnostic.** As
   the function takes no `kind` parameter today, iterating
   `DocTypeKey::all()` and feeding the same stem through `title_from`
   produces 13 identical assertions. The test's value is regression
   detection if `title_from` grows kind-awareness in the future. Worth
   keeping (as the work item specifies) but the PR description should
   frame it as a forward-looking invariant test, not a current
   per-kind verification.

## Follow-up Research 2026-06-06T18:06:59+00:00

**Git Commit**: 34c156e056e923bc9cd6687eb3704b10e4437dab
**Trigger**: re-verify the research now that some of 0085's dependency
work items have progressed.

### Headline: dependency status has moved, but 0085 is still gated

Two of the three gating dependencies have shipped since 2026-05-31; the
third has not. **0085 remains blocked** on its own terms.

| ID | Title | Status 2026-05-31 | Status 2026-06-06 | Change |
|---|---|---|---|---|
| **0065** | Update All Artifact Templates to Unified Schema | `ready` | **`done`** | ✅ shipped |
| **0066** | Move Review/Validation Skills' Frontmatter into Templates | `draft` | **`done`** | ✅ shipped |
| **0070** | Ship `meta/` Corpus Unified-Schema Migration | `draft` | `draft` | — unchanged |
| **0074** | Per-Doc-Type Hues on Detail Page | `in-progress` | **`done`** | ✅ shipped |
| **0078** | Detail-Page Frontmatter Table | `done` | `done` | — unchanged |
| **0080** | Detail-Page Header Actions | `draft` | `draft` | — unchanged |
| **0084** | Detail-Page Chip Strip Cap | `in-progress` | **`done`** | ✅ shipped |
| **0057** | Unified Artifact Frontmatter & Typed Cross-Linking (epic, parent of 0065/0066/0070) | *not tracked* | `in-progress` | newly noted |
| **0085** | (this work item) | `ready` | `ready` | — unchanged |

Notes on the status read (YAML `status:` is authoritative):

- Several work items have a **stale body `**Status**:` line** that still
  reads "Ready" while the YAML `status:` reads `done` (0065, 0074, 0078,
  0084). The table above uses the YAML value.
- **0080** still carries the un-migrated `type: story` discriminator
  rather than `kind:` — a corpus artifact that 0070's migration would
  normalise.

### Revised implication — the fallback is now *partly* defensive

The original snapshot's key implication (see **Dependency status**
above) was that, with none of 0065/0066/0070 shipped, 0085's
humanised-slug fallback would be the **primary** title-derivation path
for the three frontmatter-less, H1-less kinds (work-item-reviews,
plan-reviews, validations). That has shifted:

- **0065 (templates) and 0066 (review/validation inline generators) are
  now done.** So **newly produced** reviews and validations are born
  with `frontmatter.title` populated → cascade **layer 1** resolves for
  them, and 0085's fallback is genuinely the belt-and-braces layer the
  work item describes.
- **0070 (corpus migration of existing `meta/` documents) is still
  `draft`.** So the **existing** review/validation documents already in
  the indexed corpus still lack `frontmatter.title` and still fall
  through to the stem fallback. For those legacy docs, 0085's
  humanised-slug path is **still load-bearing**, not merely defensive.

Net: the "load-bearing on the day it lands" caveat from the original
snapshot now applies only to the **legacy corpus**, and only **until
0070 runs**. Because 0085 is still `Blocked by: 0070`, the work item's
own gating is internally consistent — landing it before 0070 still
yields the user-visible improvement on legacy reviews/validations as the
headline effect (not just the edge-case backstop). No change to the work
item's dependency clause is warranted.

### Codebase re-verification — behaviour unchanged, line numbers drifted

The implementation surface 0085 touches is **behaviourally identical**
to the 2026-05-31 snapshot — no humanise step has been added anywhere,
the frontend still consumes `entry.title` verbatim, and `DocTypeKey`
still has exactly 13 variants. Line numbers have moved in two files. The
fix is still the same small, single-function change.

**`frontmatter.rs`** — `title_from` is still at **lines 278-295**,
unchanged: three-layer cascade, raw `filename_stem.to_string()` fallback
(layer 3), no inline comments, same signature. The empty-string guard at
line 282 (`if !s.is_empty()`) flagged during re-verification is **not
new** — it was already present in the original snapshot. No
`humanise_slug` exists or is imported.
- The `title_cascade_*` tests have shifted down: `mod tests` header is
  now at **line 370** (was 361), the `b(s)` helper at **374-376** (was
  363-365), and the three tests at **431-453** (was 420-442). The third
  test still asserts the **raw stem** passes through unchanged
  (`assert_eq!(t, "2026-04-18-my-doc")` at line 452) — i.e. AC-3(c) is
  not yet satisfied. No `DocTypeKey`-driven table test has been added.

**`frontmatter.rs` leaf-module invariant is now STALE — affects Open
Question 1.** The original snapshot's central architectural caveat — that
`frontmatter.rs` has *zero* `crate::` coupling, so importing
`humanise_slug` from `api::library` would establish the *first*
cross-module dependency — **no longer holds**. The module now references
`crate::typed_ref` inline at **lines 351-352** (inside a `read_ref_keys`
`target:` fallback, added by the typed-linkage / 0057 work). Consequences
for Open Question 1's recommendation:
- Option **(a)** (import `pub(crate) humanise_slug` from `api::library`)
  no longer *breaks* a leaf-module invariant — that invariant is already
  broken. The "flag the leaf coupling for reviewer" caveat can be dropped.
- The rationale for option **(b)** (define `humanise_slug` locally to
  preserve the leaf invariant) is correspondingly weakened, since there
  is no longer a leaf invariant to preserve.
- Recommendation stands at **(a)** to match work-item literal text, now
  with *less* architectural friction than the original snapshot implied.

**`api/library.rs`** — `humanise_status` unchanged at **lines 236-242**,
still private `fn`, still no unit tests; `mod tests` (244-343) still only
exercises `parse_selection_query`. No `humanise_slug`. No
string-manipulation crate (heck/convert_case/inflections/titlecase/
Inflector) imported. The `regex` crate remains available on the crate but
unused here. (Open Questions 1, 3, 4 are unaffected and still stand.)

**`docs.rs`** — `DocTypeKey` still **13 variants** at lines 4-20 in the
same declaration order; `all()` still returns `[DocTypeKey; 13]` at lines
23-39. Two tests now assert the count of 13 explicitly
(`all_returns_every_variant_exactly_once`,
`doc_type_key_all_returns_thirteen_variants`). The work item's count is
still exact. (Open Question 5 — AC1 being a forward-looking invariant
test — is unaffected.)

**`indexer.rs` — title-resolution call site MOVED.** The original
snapshot recorded it at lines 1016-1047. The derivation and call now live
inside `build_entry`: `title_fallback_stem` is derived at **lines
1240-1242** (`slug_filename.strip_suffix(".md").unwrap_or(filename_stem)`)
and `frontmatter::title_from(&parsed.state, &parsed.body,
title_fallback_stem)` is called at **line 1243**. The stem is still passed
**verbatim** (no humaniser). `IndexEntry` has grown (now **lines
162-198**, with added `completeness`, `linked_count`, `cluster_key`
fields) but `pub title: String` remains, now at **line 173**. No schema
change to the DTO; Assumptions section still holds.

**Frontend consumers of `entry.title` — still verbatim everywhere.** No
per-route humanisation was introduced; **0085 still requires no frontend
change** (AC-4 holds). Line numbers drifted and two files moved into
per-component directories:

| File | Line 2026-05-31 | Line 2026-06-06 | Note |
|---|---|---|---|
| `LibraryDocView.tsx` (`title = entry.title`) | 95 | **103** | `<Page title>` at 191; `<EyebrowLabel>` at 190 (was 159) |
| `Page.tsx` (`<h1>{title}</h1>`) | 31 | 31 | unchanged |
| `LibraryTypeView.tsx` (table cell) | 248 | 248 | sort comparators still 63/67 |
| `LifecycleClusterView.tsx` | 134 | **208** | page-level `<Page>` uses `cluster.title` (87), not `entry.title` |
| `WorkItemCard.tsx` | 54 | 54 | unchanged |
| `announcements.ts` | 25 | 25 | ARIA sentence composition (unchanged in character) |
| `RelatedArtifacts.tsx` | 97 | **94** | moved to `components/RelatedArtifacts/RelatedArtifacts.tsx`; rendered at 118 |
| `LibraryOverviewHub.tsx` | 83 | **84** | unchanged behaviour |
| `api/wiki-links.ts` | 192 | 192 | unchanged |

`frontend/index.html` still ships the static
`<title>Accelerator Visualiser</title>` (line 6); no `document.title` /
`Helmet` / `useTitle` anywhere in `frontend/src`.

### Bottom line

- **Dependencies**: 2 of 3 gating deps done (0065, 0066); **0070 still
  `draft`** — 0085 remains correctly `Blocked by: 0070`. Sibling header
  work 0074 and 0084 also landed (both `done`), confirming the original
  "ordering-independent of 0074/0084" claim caused no conflict.
- **Code**: behaviourally unchanged; the fix is still a one-function
  cascade-layer swap plus a `humanise_slug` helper. Only `indexer.rs`
  (call site moved into `build_entry`, ~line 1243) and the
  `frontmatter.rs` test block (now ~431-453) need their line references
  refreshed at plan time.
- **One stale premise to correct in the plan**: `frontmatter.rs` is no
  longer a zero-`crate::`-coupling leaf module (it now uses
  `crate::typed_ref`), so Open Question 1's leaf-invariant caveat is
  moot and option (a) is the clean, low-friction choice.
