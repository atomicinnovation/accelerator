---
type: plan
id: "2026-06-11-0085-humanise-detail-page-h1"
title: "Humanise Detail-Page H1 Across All Doc Kinds Implementation Plan"
date: "2026-06-11T13:14:35+00:00"
author: Toby Clemson
producer: create-plan
status: done
work_item_id: "work-item:0085"
parent: "work-item:0085"
derived_from: ["codebase-research:2026-06-11-0085-humanise-detail-page-h1"]
tags: [backend, detail-page, indexer, frontmatter, humanise-slug]
revision: "520427700dfc40e685bf46b0dedc8119947bcea4"
repository: ticket-management
last_updated: "2026-06-11T16:46:05+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Humanise Detail-Page H1 Across All Doc Kinds Implementation Plan

## Overview

Replace the raw `filename_stem` fallback in the server-side title cascade
(`title_from` in `frontmatter.rs`) with a humanised slug, so no detail page
ever renders an unhumanised filename stem (e.g.
`0042-templates-view-redesign-review-1`) as its `<h1>`. The change is a new
`humanise_slug` helper plus a one-line cascade-layer swap, the cascade order
documented inline, and tests covering the helper, the cascade layers, and an
all-doc-kinds invariant guard.

Post unified-schema migration (epics 0065/0066/0070) every doc kind carries
`frontmatter.title`, so this fallback is the defensive belt-and-braces layer
for hand-authored, legacy, or malformed documents that slip through — and,
until 0070 runs, a still-load-bearing layer for legacy reviews/validations.

## Current State Analysis

The title shown as H1 is computed entirely server-side by `title_from` and
shipped as `IndexEntry.title` (a plain non-optional `String`). The frontend
renders it verbatim through the shared `<Page>` wrapper — there is no
client-side humanisation anywhere in the detail render path.

**The cascade today** (`skills/visualisation/visualise/server/src/frontmatter.rs:290-307`):

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

- Order is exactly `frontmatter.title` → first H1 → raw `filename_stem`.
- **Layer 3 (line 306) returns the stem verbatim** — no stripping, no
  hyphen→space, no casing. The single line this plan replaces with
  `humanise_slug(filename_stem)`.
- `title_from` takes no `DocTypeKey` and never branches on kind — the cascade
  is kind-independent. (Load-bearing for how AC1's loop is framed below.)
- The blank-title guard at the layer-1 check (`if !s.is_empty()` today) treats
  an empty frontmatter `title:` as absent and falls through — an existing but
  **untested** branch. A whitespace-only `title: "   "` currently slips
  *through* the guard and renders verbatim as a blank `<h1>`. Phase 2 tightens
  the guard to `!s.trim().is_empty()` so whitespace-only is also treated as
  absent, and fixtures both cases.
- **No inline comments** document the layers today, so AC5 is purely additive.

**The call site** (`skills/visualisation/visualise/server/src/indexer.rs:1305-1308`):

```rust
let title_fallback_stem = slug_filename
    .strip_suffix(".md")
    .unwrap_or(filename_stem);
let title = frontmatter::title_from(&parsed.state, &parsed.body, title_fallback_stem);
```

The stem is passed **verbatim** — prefix (numeric id / ISO date) intact, only
`.md` stripped. So `humanise_slug` receives e.g. `"0042-fix-login"` *with* its
prefix and must do the stripping. No structural change to the call site.

**`slug.rs` already implements the prefix surgery AC2 needs**
(`skills/visualisation/visualise/server/src/slug.rs`, a crate-root `pub mod`):

- `strip_prefix_date_str(stem) -> Option<&str>` (line 73) — strips a leading
  `YYYY-MM-DD-` and returns the borrowed tail. **Config-independent.** Returns
  `None` for a bare 10-char date with no trailing `-remainder` (it requires
  `tail.starts_with('-')` and `stem.len() >= 11`).
- `strip_prefix_work_item_id(stem) -> Option<String>` (line 64) — strips a
  leading run of ASCII digits + `-`, owned return. **Config-independent.**

Both are **private** `fn`s — but co-locating `humanise_slug` in `slug.rs`
(the decision below) means it can call them directly with no visibility
change. The config-*aware* strippers (`strip_prefix_date_and_optional_id`,
`strip_optional_work_item_id_prefix`) are deliberately **not** used:
`humanise_slug` is config-independent per AC2.

**`DocTypeKey::all()`** (`docs.rs:23`) returns `[DocTypeKey; 13]` — a
compile-time-counted fixed array, exactly the thirteen variants the work item
enumerates.

**`humanise_status`** (`api/library.rs:236`) upper-cases the first char of a
single token — it neither splits on hyphens nor title-cases each segment, so
`humanise_slug` *as a whole* shares no structural logic with it. Its
per-segment caser `title_case_segment`, however, **is** byte-for-byte identical
to `humanise_status`; Phase 1 adds a cross-reference comment recording the
deferred unification rather than collapsing them now. `humanise_status` is a
private `fn` in a private module (`api/mod.rs` declares `mod library;`),
confirming `api/library.rs` is the wrong home.

### Key Discoveries

- **Placement decision: `slug.rs`, not `api/library.rs`** (overrides the work
  item's literal Technical Note). Decided with the user. `slug.rs` is already
  `pub`, houses the config-independent strippers AC2 needs, avoids promoting
  the private `api::library` module, and avoids a low-level `frontmatter → api`
  upward dependency. `frontmatter.rs` already does `use crate::typed_ref::…`,
  so `use crate::slug::humanise_slug;` is in keeping.
- **The bare-date trap** (drives the helper's structure). `humanise_slug("2026-05-21")`
  must yield `"2026 05 21"` (AC2 degenerate case), but `strip_prefix_date_str`
  returns `None` for a bare 10-char date. A naive
  `strip_prefix_date_str(stem).or_else(|| strip_id(stem))` composition then
  lets the numeric-id stripper greedily eat the year `2026-`, wrongly producing
  `"05 21"`. The helper must guard with a **bare-ISO-date check** before the
  numeric-id strip: a stem that is *entirely* a date- (or id-) shaped prefix
  with no descriptive tail is humanised whole. TDD catches this — the AC2
  degenerate fixture fails against the naive version.
- **AC1's `DocTypeKey` loop is a forward-looking invariant, not coverage.**
  Because `title_from` is kind-agnostic, looping `DocTypeKey::all()` with the
  same stem runs the identical path 13 times. Its value is future-proofing (a
  later per-kind refactor that regressed any kind to a raw stem would trip it).
  The concrete-literal oracle (`"0042-test-fixture"` → `"Test Fixture"`) is
  what gives the test real value-level bite and guards the test oracle itself.
- **Frontend untouched** — `LibraryDocView.tsx:116` reads `entry.title`,
  `Page.tsx:33` renders `<h1>{title}</h1>`. No client-side humanisation exists
  to duplicate or conflict with. AC4 holds by construction.
- **Line numbers are volatile** — this surface has drifted across three
  research passes. The plan references symbols; re-confirm exact lines at
  implementation time.

## Desired End State

`title_from`'s third cascade layer returns `humanise_slug(filename_stem)`
instead of the raw stem. `humanise_slug` is a public, unit-tested helper in
`slug.rs`. The cascade's three layers each carry a one-line inline comment
naming their source. No detail-page route can render an unhumanised filename
stem as its `<h1>` for any of the thirteen doc kinds. No frontend file changes.

**Verification:** `mise run test:unit:visualiser` passes with the new helper
and cascade tests; `mise run server:check` (rustfmt + clippy `-D warnings`)
is clean; the PR diff touches only `slug.rs` and `frontmatter.rs` under
`server/src/` — zero frontend files.

## What We're NOT Doing

- **No frontend changes.** `LibraryDocView.tsx` and `Page.tsx` are untouched
  (AC4). No client-side humanisation is introduced.
- **No kind-aware title synthesis.** The scope-collapse the work item records
  (no review/validation title synthesis from `target` + `review_number`) is
  honoured — `frontmatter.title` from 0065/0066/0070 is the primary path; this
  is only the fallback.
- **No `humanise.rs` module and no `DocTypeKey` parameter on `title_from`.**
  Extraction to a dedicated humanise module stays deferred until a third
  humaniser appears; `title_from` stays kind-agnostic.
- **No change to `humanise_status`** or its `api/library.rs` home. We do not
  unify the per-segment capitalise idiom across modules now (trivial 4-line
  duplication; a future extraction can unify).
- **No DTO/schema change.** `IndexEntry.title` stays a plain `String`.
- **No change to the config-aware slug strippers** or the `derive()` path.

## Implementation Approach

Two phases, each independently mergeable and green on its own, built
test-first:

1. **Phase 1** adds `humanise_slug` (and a small borrowed-stripper refactor) to
   `slug.rs` with its full AC2 unit-test suite. Mergeable alone: a new `pub fn`
   plus tests; `derive()` and every existing slug test are unchanged.
2. **Phase 2** wires `humanise_slug` into `title_from`, documents the cascade
   inline, and adds the cascade/invariant/empty-title tests. Depends only on
   Phase 1's helper existing.

Each phase compiles, passes `mise run test:unit:visualiser`, and passes
`mise run server:check` before it is considered done.

## Phase 1: `humanise_slug` helper in `slug.rs`

### Overview

Add a config-independent `humanise_slug(&str) -> String` that strips at most
one leading ISO-date or numeric-id prefix (date wins; bare-date guarded), then
splits the remainder on `-`, drops empty segments (so edge/consecutive hyphens
never emit stray spaces), and title-cases each segment (digit-led segments pass
through unchanged). Reuse the existing strippers; add a borrowed variant of the
numeric-id stripper so the helper can work on `&str` without allocating.

### Changes Required

#### 1. Borrowed numeric-id stripper (refactor, no behaviour change)

**File**: `skills/visualisation/visualise/server/src/slug.rs`
**Changes**: Factor `strip_prefix_work_item_id` into a borrowed `_str` variant
and delegate the existing owned function to it, so the `derive()` path is
byte-for-byte unchanged while `humanise_slug` gets a `&str`-returning stripper.

```rust
/// Borrowed form: returns the descriptive tail after a leading run of ASCII
/// digits + '-', or None when there is no such prefix or no tail follows.
fn strip_prefix_work_item_id_str(stem: &str) -> Option<&str> {
    let dash = stem.find('-')?;
    let (digits, tail) = stem.split_at(dash);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let rest = &tail[1..];
    if rest.is_empty() { None } else { Some(rest) }
}

fn strip_prefix_work_item_id(stem: &str) -> Option<String> {
    strip_prefix_work_item_id_str(stem).map(str::to_string)
}
```

#### 2. Bare-ISO-date predicate (extract from `strip_prefix_date_str`)

**File**: `skills/visualisation/visualise/server/src/slug.rs`
**Changes**: Extract the 10-char date-shape check into a reusable predicate so
both `strip_prefix_date_str` and `humanise_slug` share one definition. The
bare-date guard is what defends `humanise_slug("2026-05-21")` from the
year-eating numeric strip.

```rust
/// True iff the first 10 bytes form a `YYYY-MM-DD` ISO date *shape* (does not
/// require a trailing `-<tail>`; `strip_prefix_date_str` adds that).
fn is_iso_date_prefix(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 10
        && b[0..4].iter().all(|b| b.is_ascii_digit())
        && b[4] == b'-'
        && b[5..7].iter().all(|b| b.is_ascii_digit())
        && b[7] == b'-'
        && b[8..10].iter().all(|b| b.is_ascii_digit())
}
```

(Refactor `strip_prefix_date_str` to call `is_iso_date_prefix(stem)` for its
shape check; its existing `len < 11` / `tail.starts_with('-')` remainder logic
stays. Keep all existing `slug.rs` tests green.)

Two related length checks now sit side by side: `is_iso_date_prefix`'s
`len >= 10` (validates the 10-char date *shape*) and `strip_prefix_date_str`'s
`len < 11` / `starts_with('-')` (asserts a trailing `-<tail>` follows). Add a
one-line comment on `strip_prefix_date_str` making that division explicit, so a
maintainer doesn't mistake them for a redundant pair and "fix" one — breaking
the bare-date-vs-dated-tail distinction the whole change hinges on. Match the
existing `|b| b.is_ascii_digit()` closure idiom in the new predicate (rather
than `u8::is_ascii_digit`) so the digit checks read uniformly across the file.

#### 3. `humanise_slug` and the per-segment title-caser

**File**: `skills/visualisation/visualise/server/src/slug.rs`
**Changes**: Add the public helper and a private per-segment caser.

```rust
/// Humanise a filename stem for display as a fallback page title.
///
/// Strips at most one leading prefix — an ISO date (`2026-05-21-`) takes
/// priority over a numeric id (`0042-`) — then splits the remainder on '-',
/// drops empty segments (so edge/consecutive hyphens never emit stray
/// spaces), and title-cases each segment. The first character of each segment
/// is uppercased; this is a no-op for a digit-led segment (`0042`, `21`, a
/// trailing `1`), which therefore passes through unchanged. A stem that is
/// *entirely* a date- or id-shaped prefix (no descriptive tail) is humanised
/// whole, so a bare date `2026-05-21` renders `"2026 05 21"` rather than
/// losing its leading token. A design-inventory `HHMMSS-` time token (e.g.
/// `…-123456-architecture`) survives as a verbatim digit-led segment — this
/// is accepted: the input is the slug-source stem, which for nested-manifest
/// kinds is the parent directory name, not `inventory.md`.
pub fn humanise_slug(stem: &str) -> String {
    let remainder = strip_humanise_prefix(stem);
    remainder
        .split('-')
        .filter(|s| !s.is_empty())
        .map(title_case_segment)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Strip the single leading date/id prefix, returning the descriptive tail —
/// or the whole stem when it is prefix-only (guards the bare-date case).
fn strip_humanise_prefix(stem: &str) -> &str {
    if let Some(rest) = strip_prefix_date_str(stem) {
        if !rest.is_empty() {
            return rest; // `YYYY-MM-DD-<tail>`
        }
        // date strip yielded an empty tail (e.g. `2026-05-21-`) — fall through
        // to the bare-date guard rather than returning "" (which would render a
        // blank <h1>, the very thing this fallback exists to prevent).
    }
    if is_iso_date_prefix(stem) {
        return stem; // bare date (or date with empty tail) → humanise whole
    }
    if let Some(rest) = strip_prefix_work_item_id_str(stem) {
        return rest; // `<digits>-<tail>` (the stripper returns None on empty tail)
    }
    stem
}

/// Uppercase the first char, leave the rest untouched. A digit-led segment is
/// unchanged (a digit has no uppercase mapping), so it passes through as-is.
///
/// Mirrors `api::library::humanise_status`'s per-segment idiom byte-for-byte;
/// unifying the two into one shared helper is deferred until a third humaniser
/// appears (see "What We're NOT Doing").
fn title_case_segment(seg: &str) -> String {
    let mut chars = seg.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
```

#### 4. Unit tests (write FIRST — TDD)

**File**: `skills/visualisation/visualise/server/src/slug.rs` (`#[cfg(test)] mod tests`)
**Changes**: Add a table-driven `humanise_slug` test covering every AC2 case,
written and watched failing before the helper is implemented.

```rust
#[test]
fn humanise_slug_covers_ac2_cases() {
    let cases = &[
        // simple hyphen splits
        ("design-token-system", "Design Token System"),
        // leading numeric id stripped; trailing review-N kept; digit verbatim
        ("0042-templates-view-redesign-review-1", "Templates View Redesign Review 1"),
        // leading ISO date stripped
        ("2026-05-21-current-app-vs-claude-design-prototype", "Current App Vs Claude Design Prototype"),
        // mixed prefixes — single-pass strip of the LEADING match only
        ("2026-05-21-0042-foo", "0042 Foo"),   // date wins; 0042 survives as one token
        ("0042-2026-05-21-foo", "2026 05 21 Foo"), // id wins; residual date splits
        // single segment
        ("notes", "Notes"),
        // edge/consecutive hyphens: empty segments dropped, no stray spaces
        ("foo--bar", "Foo Bar"),               // double hyphen would be "Foo  Bar" unfiltered
        ("0042-", "0042"),                     // trailing dash would be "0042 " unfiltered
        // degenerate: prefix-only stem humanises its own tokens
        ("2026-05-21", "2026 05 21"),          // bare-date guard (would be "05 21" naive)
        ("2026-05-21-", "2026 05 21"),         // date + empty tail → humanise whole (not "")
        ("", ""),
        // design-inventory HHMMSS- quirk: accepted, time token emitted verbatim
        ("2026-05-21-123456-architecture", "123456 Architecture"),
    ];
    for (input, expected) in cases {
        assert_eq!(humanise_slug(input), *expected, "input={input}");
    }
}
```

### Success Criteria

#### Automated Verification:

- [x] Server unit tests pass: `mise run test:unit:visualiser`
- [x] `humanise_slug_covers_ac2_cases` passes (all cases incl. the bare-date,
      empty-string, and edge/consecutive-hyphen degenerate cases)
- [x] Existing `slug.rs` `derive()` tests still pass unchanged (borrowed-stripper
      refactor is behaviour-preserving)
- [x] Rust formatting clean: `mise run format:server:check`
- [x] Clippy clean with `-D warnings`: `mise run lint:server:check`

#### Manual Verification:

- [x] Diff touches only `slug.rs` under `server/src/` in this phase
- [x] The bare-date guard is exercised by a fixture (not just asserted in prose)

---

## Phase 2: Wire `humanise_slug` into the `title_from` cascade

### Overview

Swap the layer-3 raw-stem return for `humanise_slug(filename_stem)`, document
the three cascade layers inline (AC5), and add the cascade, all-kinds
invariant, and empty-title tests. The indexer call site and the frontend are
untouched.

### Changes Required

#### 1. Cascade swap + inline layer comments

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs`
**Changes**: Import the helper; replace the raw-stem return; add one comment
per cascade layer naming its source and position.

```rust
use crate::slug::humanise_slug;

pub fn title_from(parsed: &FrontmatterState, body: &str, filename_stem: &str) -> String {
    // Cascade layer 1/3 — frontmatter.title (verbatim when non-blank;
    // whitespace-only is treated as absent so it can't render a blank <h1>).
    if let FrontmatterState::Parsed(m) = parsed {
        if let Some(v) = m.get("title") {
            if let Some(s) = v.as_str() {
                if !s.trim().is_empty() {
                    return s.trim().to_string(); // trim to match layer 2's H1
                }
            }
        }
    }
    // Cascade layer 2/3 — first H1 in the body.
    for line in body.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("# ") {
            return rest.trim().to_string();
        }
    }
    // Cascade layer 3/3 — humanise_slug(stem): humanised fallback so no detail
    // page renders an unhumanised filename stem as its <h1>.
    humanise_slug(filename_stem)
}
```

#### 2. Update the existing stem-fallback test (layer c)

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs` (tests)
**Changes**: `title_cascade_falls_back_to_filename_stem` currently asserts the
raw stem `"2026-04-18-my-doc"`; update it to assert the humanised form.

```rust
#[test]
fn title_cascade_falls_back_to_filename_stem() {
    let raw = b("body without h1\n");
    let p = parse(&raw);
    let t = title_from(&p.state, &p.body, "2026-04-18-my-doc");
    assert_eq!(t, "My Doc"); // humanised (date prefix stripped), not the raw stem
}
```

#### 3. All-doc-kinds invariant test + concrete-literal oracle (AC1)

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs` (tests)
**Changes**: Add the import `use crate::docs::DocTypeKey;` to the test module,
then the loop test. (Do **not** import `humanise_slug` here — the loop no
longer references it; the concrete-literal oracle below is the test's bite.)

```rust
#[test]
fn title_cascade_humanises_stem_for_every_doc_kind() {
    // No frontmatter.title and no first H1 → layer 3 for every kind.
    let raw = b("body without h1\n");
    let p = parse(&raw);
    let stem = "0042-test-fixture";

    // Forward-looking invariant: title_from is kind-agnostic today, so this
    // loop runs the same path 13 times. We assert only that no kind renders
    // the raw stem — comparing against humanise_slug(stem) would be a
    // tautology (production calls the same helper). It guards against a future
    // per-kind refactor regressing any kind back to an unhumanised stem.
    for kind in DocTypeKey::all() {
        let t = title_from(&p.state, &p.body, stem);
        assert_ne!(t, stem, "kind={kind:?} must not render the raw stem");
    }

    // Concrete-literal oracle: gives the test real value-level bite, guarding
    // against a shared bug in the test oracle and the production path both
    // computing the same wrong value.
    assert_eq!(title_from(&p.state, &p.body, stem), "Test Fixture");
}
```

#### 4. Cascade layer (a)/(b) + empty-title fixtures (AC3, Open Q5)

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs` (tests)
**Changes**: Layers (a) and (b) are already covered by
`title_cascade_prefers_frontmatter` and `title_cascade_falls_back_to_first_h1`
(leave as-is). Add a fixture for the blank-`title:` branch — both empty and
whitespace-only — which must fall through to the humanised stem (the empty case
was previously untested; the whitespace case is newly handled by the
`!s.trim().is_empty()` guard).

```rust
#[test]
fn title_cascade_blank_frontmatter_title_falls_through_to_humanised_stem() {
    // Drive the layer-1 guard directly at the title_from boundary by building
    // FrontmatterState::Parsed ourselves. We do NOT round-trip through parse():
    // a quoted whitespace-only YAML scalar can trip libyml's
    // trailing-whitespace panic and surface as Malformed (see
    // `malformed_when_quoted_scalar_has_trailing_whitespace`), which would skip
    // the Parsed arm entirely and make this test pass for the wrong reason —
    // green even if the `!s.trim().is_empty()` guard were reverted.
    for blank in ["", "   "] {
        let mut m = std::collections::BTreeMap::new();
        m.insert(
            "title".to_string(),
            serde_json::Value::String(blank.to_string()),
        );
        let state = FrontmatterState::Parsed(m);
        let t = title_from(&state, "body without h1\n", "0042-test-fixture");
        assert_eq!(t, "Test Fixture", "blank title {blank:?}");
    }
}
```

This constructs the `Parsed` state directly (the test module is in-file, so the
private `FrontmatterState::Parsed(BTreeMap<String, serde_json::Value>)` variant
is accessible), guaranteeing the `!s.trim().is_empty()` guard is the code under
test rather than the parser's panic-handling path.

### Success Criteria

#### Automated Verification:

- [x] Server unit tests pass: `mise run test:unit:visualiser`
- [x] Updated `title_cascade_falls_back_to_filename_stem` asserts the humanised
      form and passes
- [x] `title_cascade_humanises_stem_for_every_doc_kind` passes (13-kind loop +
      `"Test Fixture"` literal oracle)
- [x] `title_cascade_blank_frontmatter_title_falls_through_to_humanised_stem`
      passes (both empty and whitespace-only `title:`)
- [x] Layers (a)/(b) cascade tests still pass unchanged
- [x] Rust formatting clean: `mise run format:server:check`
- [x] Clippy clean with `-D warnings`: `mise run lint:server:check`

#### Manual Verification:

- [x] PR diff under `server/src/` touches only `slug.rs` and `frontmatter.rs`;
      **no** frontend files, and in particular no changes to
      `frontend/src/routes/library/LibraryDocView.tsx` or
      `frontend/src/components/Page/Page.tsx` (AC4 — verify against the actual
      diff)
- [x] Each of the three cascade layers carries a one-line comment naming its
      source and position (AC5)
- [ ] Spot-check a real review/validation detail page in a dev server with a
      doc lacking `frontmatter.title`: the H1 renders humanised, not a raw stem

---

## Testing Strategy

### Unit Tests

- **`humanise_slug` (Phase 1):** the AC2 table — simple splits, leading
  numeric-id strip, leading ISO-date strip, both single-pass mixed-prefix
  cases, single-segment, edge/consecutive-hyphen empty-segment cases, and the
  degenerate bare-date / empty-string cases, plus the design-inventory
  `HHMMSS-` quirk pinned as accepted behaviour.
- **`title_from` cascade (Phase 2):** layer (a) frontmatter.title verbatim;
  layer (b) first H1; layer (c) humanised stem; the all-kinds invariant loop
  with the concrete-literal oracle; the blank-`title:` fall-through (empty and
  whitespace-only).

### Integration Tests

None required. `title_from` has one production call site (`indexer.rs:1308`)
that already passes the stem verbatim; no wiring changes. The behaviour change
is fully covered by unit tests at the function boundary.

### Manual Testing Steps

1. Start the dev server (`mise run dev`) against a corpus containing at least
   one document with no `frontmatter.title` and no first H1 (e.g. a legacy
   work-item-review).
2. Open that document's detail page; confirm the `<h1>` is humanised
   (`Templates View Redesign Review 1`) rather than the raw stem.
3. Confirm a document *with* `frontmatter.title` is unaffected (layer 1 still
   wins).

## Performance Considerations

Negligible. `humanise_slug` runs once per document at index-build time (the
existing `title_from` call site), only when layers 1 and 2 miss. It does a
single prefix strip, one `split('-')`, and per-segment allocation — all bounded
by stem length. No hot-path or per-request impact (`IndexEntry.title` is
computed at index time, not per render).

## Migration Notes

None. No schema/DTO change (`IndexEntry.title` stays `String`), no data
migration. The change is observed the next time the index is built. Recovery
from any regression is a VCS revert of the two-file diff.

## References

- Original work item: `meta/work/0085-humanise-detail-page-h1.md`
- Related research: `meta/research/codebase/2026-06-11-0085-humanise-detail-page-h1.md`
  (and prior `meta/research/codebase/2026-05-31-0085-humanise-detail-page-h1-fallback.md`)
- Cascade function to change: `skills/visualisation/visualise/server/src/frontmatter.rs:290-307`
  (layer-3 return at line 306)
- Existing cascade tests: `skills/visualisation/visualise/server/src/frontmatter.rs:466-487`
- Helper home + strippers to reuse: `skills/visualisation/visualise/server/src/slug.rs:64-92`
  (`strip_prefix_work_item_id`, `strip_prefix_date_str`)
- `humanise_status` precedent (per-segment idiom only): `skills/visualisation/visualise/server/src/api/library.rs:236`
- `DocTypeKey::all() -> [DocTypeKey; 13]`: `skills/visualisation/visualise/server/src/docs.rs:23`
- Verbatim call site: `skills/visualisation/visualise/server/src/indexer.rs:1305-1308`
- Frontend consumers (untouched): `frontend/src/routes/library/LibraryDocView.tsx:116`,
  `frontend/src/components/Page/Page.tsx:33`
- ADR establishing `title` as a base frontmatter field:
  `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` (the invariant
  the primary cascade path relies on; note the work item's "ADR 0060" conflates
  work-item 0060 with ADR-0033)
- Adjacent follow-up kept separate: `meta/work/0097-strip-redundant-doc-type-prefixes-from-titles.md`
