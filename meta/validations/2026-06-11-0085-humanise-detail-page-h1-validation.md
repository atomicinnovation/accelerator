---
type: plan-validation
id: "2026-06-11-0085-humanise-detail-page-h1-validation"
title: "Validation Report: Humanise Detail-Page H1 Across All Doc Kinds"
date: "2026-06-12T01:02:45+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: "pass"
parent: "plan:2026-06-11-0085-humanise-detail-page-h1"
target: "plan:2026-06-11-0085-humanise-detail-page-h1"
tags: [backend, detail-page, indexer, frontmatter, humanise-slug]
last_updated: "2026-06-12T01:02:45+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Humanise Detail-Page H1 Across All Doc Kinds

### Implementation Status

✓ Phase 1: `humanise_slug` helper in `slug.rs` — Fully implemented
✓ Phase 2: Wire `humanise_slug` into the `title_from` cascade — Fully implemented

Both phases landed as two separate commits:

- `ywsmyxpo` — "Add humanise_slug helper for filename-stem title fallback" (Phase 1)
- `tkpwsvqx` — "Humanise the filename-stem fallback in the title cascade" (Phase 2)

### Automated Verification Results

✓ Unit tests pass: `mise run test:unit:visualiser` — 421 passed, 0 failed
✓ `humanise_slug_covers_ac2_cases` passes (all 13 AC2 cases incl. bare-date,
  empty-string, edge/consecutive-hyphen, and `HHMMSS-` quirk)
✓ `title_cascade_falls_back_to_filename_stem` passes (asserts humanised `"My Doc"`)
✓ `title_cascade_humanises_stem_for_every_doc_kind` passes (13-kind loop +
  `"Test Fixture"` literal oracle)
✓ `title_cascade_blank_frontmatter_title_falls_through_to_humanised_stem` passes
  (both empty `""` and whitespace-only `"   "` cases)
✓ Existing `slug.rs` `derive()` tests still pass (borrowed-stripper refactor is
  behaviour-preserving)
✓ Rust formatting clean: `mise run format:server:check`
✓ Clippy clean with `-D warnings`: `mise run lint:server:check`

### Code Review Findings

#### Matches Plan:

- **Phase 1 — borrowed stripper** (`slug.rs:83-101`): `strip_prefix_work_item_id_str`
  added exactly as specified; the owned `strip_prefix_work_item_id` now delegates
  to it via `.map(str::to_string)`, so the `derive()` path is byte-for-byte
  unchanged.
- **Phase 1 — bare-ISO-date predicate** (`slug.rs:103-132`): `is_iso_date_prefix`
  extracted as planned; `strip_prefix_date_str` refactored to call it, with the
  one-line comment distinguishing the shape check (`len >= 10`) from the
  trailing-tail check (`len < 11` / `starts_with('-')`).
- **Phase 1 — `humanise_slug` + `strip_humanise_prefix` + `title_case_segment`**
  (`slug.rs:181-239`): match the plan verbatim, including the bare-date guard
  ordering (date strip → bare-date guard → numeric-id strip) and the doc comment
  recording the deferred `humanise_status` unification.
- **Phase 1 — AC2 test table** (`slug.rs:245-278`): all 13 cases present exactly
  as planned.
- **Phase 2 — cascade swap + inline layer comments** (`frontmatter.rs:284-310`):
  layer 3 returns `humanise_slug(filename_stem)`; all three layers carry a
  one-line comment naming source and position (AC5). The layer-1 guard was
  tightened to `!s.trim().is_empty()` with `s.trim().to_string()` as planned.
- **Phase 2 — import** (`frontmatter.rs:1`): `use crate::slug::humanise_slug;`
  added at module level as planned.
- **Phase 2 — cascade/invariant/blank-title tests** (`frontmatter.rs:476-545`):
  all present and matching the plan, including the direct `FrontmatterState::Parsed`
  construction in the blank-title test to avoid the libyml panic path.
- **Call site untouched** (`indexer.rs:1407-1413`): the stem is still passed
  verbatim; no structural change (AC matches plan's "no change to the call site").
- **Frontend untouched** (AC4): combined diff touches only `slug.rs`,
  `frontmatter.rs`, and the plan markdown — zero frontend files. `LibraryDocView.tsx`
  and `Page.tsx` are unchanged.

#### Deviations from Plan:

- **`is_iso_date_prefix` uses `u8::is_ascii_digit` (method reference), not the
  planned `|b| b.is_ascii_digit()` closure** (`slug.rs:108-112`). The plan
  explicitly instructed matching the closure idiom "so the digit checks read
  uniformly across the file." The implementation correctly deviated: under
  clippy `-D warnings`, the `redundant_closure_for_method_calls` lint would
  reject the closure form, so the plan's stated instruction would have failed
  `lint:server:check`. The method-reference form is the only one that compiles
  green. This is a justified, behaviour-preserving deviation — the plan's
  guidance was simply wrong on this point.
- **`DocTypeKey` import is function-local, not module-level**
  (`frontmatter.rs:502`, `use crate::docs::DocTypeKey;` inside the test fn). The
  plan said "add the import … to the test module." Placing it inside the single
  consuming test is functionally identical and arguably cleaner (no unused import
  if the test is later removed). Cosmetic, no impact.

#### Potential Issues:

- None identified. The change is a pure additive helper plus a one-line cascade
  swap, fully covered by unit tests at the function boundary. No DTO/schema
  change, no migration, no hot-path impact.

### Manual Testing Required:

1. Dev-server spot-check (the one unchecked box in the plan, low risk given unit
   coverage):
  - [ ] Start `mise run dev` against a corpus containing a document with no
    `frontmatter.title` and no first H1 (e.g. a legacy work-item-review); confirm
    its detail-page `<h1>` renders humanised (e.g. `Templates View Redesign
    Review 1`) rather than the raw stem.
  - [ ] Confirm a document *with* `frontmatter.title` is unaffected (layer 1 still
    wins).

### Recommendations:

- Safe to merge. All automated success criteria pass; both phases are
  independently green.
- Consider updating the plan's Phase 1 §2 note (the `|b| b.is_ascii_digit()`
  closure instruction) in any future revision, since it contradicts the
  clippy-clean implementation — though as the plan is now `done`, this is purely
  informational.
- The single remaining manual spot-check is optional verification, not an
  automated gate; the 13-case helper suite and the all-kinds invariant test
  already exercise the behaviour the spot-check would observe.
