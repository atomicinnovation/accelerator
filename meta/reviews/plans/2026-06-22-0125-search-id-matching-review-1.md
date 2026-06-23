---
type: plan-review
id: "2026-06-22-0125-search-id-matching-review-1"
title: "Plan Review: Visualiser Search Work-Item-ID Matching"
date: "2026-06-22T15:49:14+00:00"
author: "Phil Helm"
producer: review-plan
status: complete
target: "plan:2026-06-22-0125-search-id-matching"
reviewer: "Phil Helm"
verdict: "COMMENT"
lenses: [correctness, code-quality, test-coverage, architecture, performance, usability]
review_number: 1
review_pass: 2
tags: [visualiser, search, indexer]
last_updated: "2026-06-22T16:03:30+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Plan Review: Visualiser Search Work-Item-ID Matching

**Verdict:** REVISE

This is a tightly-scoped, well-grounded, additive change that correctly
localises the fix to the single defect site (`classify()`) and reuses the
existing four-bucket ranker — architecture and performance are sound, and the
plan's "negligible cost" claim holds (the per-query cost is dominated by the
pre-existing full-snapshot clone, not the new ID lowercasing). The plan needs
revision in two areas before implementation: the **test suite as specified does
not pin the behaviour the plan promises** (the two tests assert match *presence*
but never the bucket/tier placement that the Desired End State is entirely
about, and the "exact" test does not isolate the exact tier), and **interior
substring matching on zero-padded numeric IDs floods results for short numeric
queries** while the matched ID is invisible in the result row — so a user cannot
tell why a row matched. None of the findings are structural-redesign blockers;
they are addressable with targeted edits to the test plan and one
behaviour/scope decision on interior matching.

### Cross-Cutting Themes

- **Tests don't verify the ranking behaviour the plan is about** (flagged by:
  test-coverage, correctness) — The Desired End State makes three tier claims
  (exact→`ExactSlug`, prefix→`Prefix`, interior→`Interior`), but both proposed
  tests only assert `results.iter().any(...)`. There is no prefix-tier test, the
  "exact" test's query `0042` also satisfies the prefix and interior branches
  (so deleting the exact branch leaves it green), and no test pins ordering. The
  feature's whole point — *where* an ID match ranks — is unverified.

- **Short numeric queries flood the Interior bucket** (flagged by: correctness,
  usability) — `id_interior = id_lc.contains(q_lc)` against bare zero-padded IDs
  means a two-digit query (`00`, `02`) substring-matches a large fraction of all
  work items. The frontend gates queries at length ≥ 2 (`use-search.ts:8`), so
  the single-digit `0`/`1` case the correctness agent raised cannot reach the
  API — but two-digit numeric queries are common and the backend applies no
  result cap. Compounded by the next theme, these hits are also unexplained.

- **Matched ID is invisible / config-dependent** (flagged by: usability,
  architecture) — `SearchResultRow` carries no ID field, so an ID match shows a
  title/slug containing none of the query digits (the `Highlight` component
  highlights nothing). Separately, "unpadded tolerance" relies on the *string
  form* of the normalised ID: under a project-code config the ID is `ENG-0042`,
  the exact tier needs the full `eng-0042` typed, and the plan's tests only cover
  the default-numeric config.

- **Tripled per-tier ladder is reaching its limit** (flagged by: code-quality,
  architecture) — `classify()` now repeats exact/prefix/interior across three
  fields, with `external_id` explicitly slated to become a fourth. Acceptable at
  three fields; worth a note that the next field should trigger a small refactor
  to iterate candidate fields per tier.

### Tradeoff Analysis

- **Scope discipline vs explainability**: The plan deliberately leaves the wire
  shape (`SearchResultRow`) untouched. That keeps the change minimal, but
  usability argues an ID match is unexplained without surfacing the matched ID.
  Recommendation: keep the wire shape out of scope for this fix, but record the
  invisible-ID rough edge explicitly as a known limitation rather than a closed
  decision, and reconsider it alongside the flooding decision below.

- **Interior-match simplicity vs result noise**: Reusing slug-style interior
  matching gives unpadded tolerance "for free", but produces the short-numeric
  flood. Recommendation: decide and *document* the intended behaviour — e.g.
  strip leading zeros and match the unpadded form (so `42`→`0042` works but `00`
  doesn't fan out), or gate interior ID matching on query length/numeric form —
  and pin it with a test. A bespoke normalisation here is a reasonable cost.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Test Coverage**: Tests assert match presence but never pin the bucket/tier the plan promises
  **Location**: Phase 1, Section 1: Failing tests / Testing Strategy
  Both tests assert only `results.iter().any(|r| slug == "login-form")` — never which bucket the match lands in nor its order relative to other entries. The plan's central behavioural promise (tier placement) is therefore untested; mutations that mis-tier the ID match survive.

- 🟡 **Test Coverage**: The "exact" test does not isolate the exact tier — query `0042` also matches prefix and interior
  **Location**: Phase 1, Section 1: Failing tests
  `work_item_id_exact_match` queries `0042`, which is simultaneously an exact, prefix, and interior match of `0042`. The test passes even if the new `id_lc == Some(q_lc)` ExactSlug branch is deleted entirely — the exact branch has no test that fails when it is removed.

- 🟡 **Test Coverage**: No test for the prefix tier of `work_item_id`
  **Location**: Phase 1, Section 2: Extend classify() / Testing Strategy
  The plan adds an `id_prefix` term and lists it in the Desired End State, but no test exercises a prefix-only ID query (e.g. `004` against `0042`). Deleting the `id_prefix` term survives the suite.

- 🟡 **Usability / Correctness**: Short numeric queries flood the Interior bucket with unexplained hits
  **Location**: Desired End State / Phase 1, Section 2: id_interior branch
  Two-digit numeric queries substring-match a large swathe of zero-padded IDs (`00` matches every `00xx`); the frontend min-length floor of 2 blocks only the single-digit case, and the backend applies no result cap. The intended item is buried among hits whose visible title/slug don't contain the query.

- 🟡 **Usability**: Matched ID is invisible in the result row, so ID matches look unexplained
  **Location**: What We're NOT Doing / Phase 1, Section 2
  `SearchResultRow` carries only `doc_type`/`title`/`slug`/`mtime_ms`. When a user types `0042`, the row shows "Login form" / `login-form` — nothing containing `0042`, and the `Highlight` component highlights nothing. The user cannot tell why the row matched. Either surface the ID or record this as a known UX rough edge rather than a closed decision.

#### Minor

- 🔵 **Correctness**: Exact-id tier is unreachable for prefixed IDs unless the user types the full lowercased key
  **Location**: Phase 1, Section 2: Extend classify() — exact-id branch under project-code configs
  Under a project-code config `work_item_id` is `eng-0042`, so the exact branch only fires for `eng-0042`; a natural `0042` falls through to Interior. The plan's tests use only the default-numeric config, leaving this divergence unverified.

- 🔵 **Architecture**: "Unpadded tolerance" guarantee is config-dependent and untested for project-code IDs
  **Location**: Implementation Approach / Desired End State
  The interior-match strategy couples ID-search ergonomics to the incidental string form of a normalised ID (`0042` vs `ENG-0042`). The "no bespoke normalisation needed" claim holds only for the default-numeric mode. Narrow the guarantee in the Desired End State, or add a project-code fixture test.

- 🔵 **Test Coverage**: No negative test for an ID that should NOT match
  **Location**: Phase 1, Section 1: Failing tests / Testing Strategy
  No test that a non-matching numeric query (e.g. `0099` against a `0042` item) returns no false positive. A regression making ID matching over-broad would not be caught.

- 🔵 **Test Coverage**: No coverage that `work_item_id: None` entries are handled safely and don't false-match
  **Location**: Phase 1, Section 2: Extend classify()
  Non-work-item entries (plans/decisions/RCAs) all have `work_item_id: None` — the common case across the corpus — yet no test confirms they neither panic nor false-match on a numeric query.

- 🔵 **Test Coverage**: Fixture-population assumption is load-bearing but only implicitly verified
  **Location**: Phase 1, Section 1: Failing tests
  The whole suite rests on `write_work_item` producing `work_item_id: Some("0042")` under default-numeric normalisation — the inverse of the documented `seeded_cfg_with_work_items` gotcha. Add a one-line snapshot assertion or comment so a future config change surfaces the root cause clearly.

- 🔵 **Code Quality**: `classify()` doc-comment update is named but its replacement text is unspecified
  **Location**: Phase 1, Section 2: Extend classify()
  Step 2 says "Update the function doc comment" but the code block omits it. The existing comment's eager/lazy lowercasing claim is now incomplete (it doesn't mention `work_item_id`). Without the replacement text the implementer may recreate exactly the kind of stale comment this change fixes elsewhere.

- 🔵 **Code Quality**: Indexer doc-comment correction is under-specified
  **Location**: Phase 1, Section 3: Correct the stale struct doc comment
  The stale comment at `indexer.rs:167-170` has *two* wrong claims — the "filename-derived" source and the `Some`/`None` conditions. The plan directs fixing the source but not the conditions. Quote the full replacement.

- 🔵 **Architecture**: `ExactSlug` reuse leaves a domain-clarity gap
  **Location**: What We're NOT Doing — ExactSlug reuse
  Reusing the variant (declining the rename) is the right churn trade, but the name now misleads about its semantics. Add a one-line doc comment on the variant noting it is the "exact identity" tier covering exact slug OR exact `work_item_id`.

#### Suggestions

- 🔵 **Code Quality / Architecture**: Tripled exact/prefix/interior tiering invites a small extract as the field set grows
  **Location**: Phase 1, Section 2: Extend classify()
  Acceptable at three fields, but `external_id` is already slated as a fourth. Note in the plan that the next field should trigger a refactor to iterate a `[Option<&str>]` of candidate fields per tier, so fields slot in by data rather than triplicated edits.

- 🔵 **Architecture**: Record the evolutionary direction for the deferred `external_id`
  **Location**: What We're NOT Doing — external_id deferral
  This change sets a precedent (dedicated typed fields drive matching). Add a sentence noting the dedicated-field approach is the consistent successor for `external_id`, so the deferred work inherits clear intent.

- 🔵 **Performance**: Eager `id_lc` lowercasing for a sparse field is harmless — keep as-is
  **Location**: Performance Considerations
  `Option::map` over `None` allocates nothing and IDs are tiny; the plan's "negligible" claim is accurate. No change recommended; the real per-query lever (out of scope) is the full-snapshot clone in the handler, not ID lowercasing.

- 🔵 **Usability**: Empty-state hint says "doc id" but the feature matches only work-item IDs
  **Location**: What We're NOT Doing — frontend copy
  Decisions/plans/reviews have `work_item_id: None`, so searching their identifiers still yields nothing despite the generic "doc id" phrasing. Either keep the vague copy (acceptable) and note the limitation, or tighten toward "a work item number".

- 🔵 **Usability**: Exact-ID hit not guaranteed to rank first within the shared top bucket
  **Location**: Desired End State
  An exact ID shares `ExactSlug` with exact-slug hits, ordered only by `(mtime desc, rel_path asc)`, so it is top-*bucket* but not necessarily the top *row*. Acceptable for a scoped fix; note that exact-ID is co-equal with exact-slug so the expectation is explicit.

### Strengths

- ✅ Tightly scoped and well-grounded: confines the change to the single defect
  site (`classify()`) with no struct, indexer, or wire-shape change — minimal
  blast radius, and the research backing is thorough.
- ✅ Reuses the existing four-bucket tiering rather than inventing a new bucket
  or per-type ID code path, consistent with the deliberately field-restricted
  ranker from work item 0054.
- ✅ Genuinely test-first with a credible red state (neither `0042` nor `42`
  appears in any seeded field today), and the new fixture is well-isolated so a
  passing assertion can only come from the new `work_item_id` path.
- ✅ Correctly identifies and avoids the documented fixture gotcha
  (`seeded_cfg_with_work_items` yields `work_item_id: None`) by writing its own
  id-bearing fixture.
- ✅ Preserves the deliberate lazy `body_preview` lowercasing optimisation and
  proactively corrects the stale `indexer.rs` struct doc comment while in the
  area.
- ✅ The "What We're NOT Doing" boundaries are explicit and well-justified
  (deferring `external_id`, declining the bucket rename, leaving the wire shape
  untouched), and tradeoffs are acknowledged rather than hidden.
- ✅ Performance claim is accurate: the added cost is dwarfed by the pre-existing
  snapshot clone, which the plan correctly leaves untouched.

### Recommended Changes

1. **Strengthen the test plan to pin tier placement, not just presence**
   (addresses: "Tests assert match presence…", "The 'exact' test does not
   isolate the exact tier", "No test for the prefix tier", "No negative test",
   "No coverage that `work_item_id: None`…")
   Revise Phase 1 Section 1 so the tests mirror the existing
   `bucket_1_exact_slug_first` / `bucket_2_prefix_before_interior` patterns:
   seed a competing entry and assert the exact-ID item ranks at `results[0]`;
   add a prefix-only test (`004`→`0042`); add a negative test (`0099` returns no
   match); and add an assertion that `None`-id entries seeded by `seeded_cfg`
   never appear for a numeric query.

2. **Decide and document the short-numeric interior-match behaviour**
   (addresses: "Short numeric queries flood the Interior bucket")
   Choose between (a) strip leading zeros and match the unpadded form, (b) gate
   interior ID matching on query length/numeric form, or (c) accept the flood
   and cap results — then state the choice in the Desired End State and pin it
   with a test (e.g. `42`→`0042` matches but `00` does not broadly fan out).

3. **Resolve the invisible-matched-ID UX, even if deferring**
   (addresses: "Matched ID is invisible in the result row")
   Either bring surfacing `work_item_id` in `SearchResultRow` into scope, or add
   it to "What We're NOT Doing" explicitly as a known UX rough edge with a
   follow-up pointer — not silently.

4. **Specify the doc-comment replacement texts**
   (addresses: "`classify()` doc-comment update… unspecified", "Indexer
   doc-comment correction… under-specified")
   Quote the full replacement for both the `classify()` function comment
   (covering eager `work_item_id` lowercasing) and the `indexer.rs:167-170`
   struct comment (correcting both the source *and* the `Some`/`None`
   conditions).

5. **Address project-code-config behaviour**
   (addresses: "Exact-id tier is unreachable for prefixed IDs", "'Unpadded
   tolerance' guarantee is config-dependent")
   Either narrow the Desired End State to state the unpadded-tolerance and
   exact-tier guarantees apply to the default-numeric config, or add a
   project-code fixture test confirming the `0042`/`42` behaviour against
   `ENG-0042`.

6. **Add forward-looking notes (low effort)**
   (addresses: "ExactSlug reuse… domain-clarity gap", "Tripled tiering",
   "Record the evolutionary direction for `external_id`")
   A one-line doc comment on the `ExactSlug` variant, and a sentence in "What
   We're NOT Doing" noting the per-tier ladder should be refactored to iterate
   candidate fields when `external_id` lands via the dedicated-field approach.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The core change — slotting `work_item_id` into the existing
exact/prefix/interior tiers of `classify()` — is logically sound: the exact-tier
ordering (id checked before slug, both returning `ExactSlug`) is consistent,
empty/whitespace ids cannot spuriously match because `normalise_id` rejects them
and the handler rejects empty queries, and the `Option`/`as_deref` plumbing
mirrors the existing slug handling correctly. The principal correctness risk is
the unbounded interior substring match against zero-padded numeric ids: short
numeric queries (`0`, `1`) will substring-match a large fraction of all work
items, which the plan neither acknowledges nor tests. The two proposed tests do
genuinely isolate id-only matching (the chosen slug/title/body avoid the query
tokens), but they only assert presence, not the bucket/ranking the Desired End
State claims.

**Strengths**:
- Exact-tier ordering is correct: `id_lc == q_lc` and `slug == q_lc` both return
  `Bucket::ExactSlug`, so there is no ambiguity when an id equals a slug —
  within-bucket ordering is the existing mtime/rel_path sort.
- Empty/whitespace `work_item_id` cannot produce a false match: `normalise_id`
  returns `None` for empty/whitespace input, and the handler rejects
  empty/whitespace queries before `classify()`.
- The two proposed tests correctly isolate id-only matching — the fixture's slug,
  title and body contain neither `0042` nor `42`, so a passing assertion proves
  the match came from `work_item_id`.
- The `Option`/`as_deref`/`is_some_and` plumbing for `id_lc` faithfully mirrors
  the existing slug handling, so absent ids short-circuit without panics.

**Findings**:
- 🟡 **major** (high) — Short numeric queries interior-match nearly every
  zero-padded work-item id. *Location: Phase 1, Section 2: Extend classify() —
  id_interior branch; Desired End State.* `id_interior = id_lc.contains(q_lc)`
  runs unconditionally against zero-padded numeric IDs; a single-or-double-digit
  query dumps most work items into Interior, swamping relevant title/slug
  results. Decide and document intended behaviour for short numeric queries and
  add a test pinning it. *(Reviewer note: the frontend gates at length ≥ 2, so
  the single-digit case cannot reach the API; two-digit floods remain.)*
- 🔵 **minor** (high) — Tests assert presence but not the bucket/ranking the
  plan promises. *Location: Phase 1, Section 1; Desired End State.* Both tests
  assert only `.any(...)`; a regression that mis-tiers an exact id into Interior
  would still pass. Seed a competing entry and assert `results[0]`.
- 🔵 **minor** (medium) — Exact-id tier is unreachable for prefixed ids unless
  the user types the full lowercased key. *Location: Phase 1, Section 2 —
  exact-id branch under project-code configs.* Under a project code,
  `work_item_id` is `eng-0042`; a `0042` query falls through to Interior. Accept
  and document, or normalise the query before comparison; add a prefixed-config
  test.

### Code Quality

**Summary**: The plan is a small, well-scoped additive change that correctly
preserves the lazy `body_preview` lowercasing optimisation. The main quality
concern is that the proposed `classify()` rewrite triples the
exact/prefix/interior tiering across three near-identical fields, pushing
cognitive load up just as the matched-field set is likely to keep growing
(`external_id` is a deferred follow-on). Two specification gaps reduce
maintainability: the `classify()` doc-comment update and the indexer struct
doc-comment correction are both named as required but neither supplies the
replacement text.

**Strengths**:
- Preserves the lazy `body_preview` lowercasing optimisation — `id_lc` is
  lowercased eagerly alongside title/slug (all short), body lazily on
  fall-through.
- Reuses the existing four-bucket tiering rather than inventing a new ID bucket
  or per-type code path (KISS/YAGNI; interior match gives unpadded tolerance for
  free).
- Correctly identifies and plans to fix the stale `indexer.rs` struct doc
  comment while in the area.
- Scope boundaries are explicit and disciplined (no wire-shape change, no bucket
  rename, `external_id` deferred).

**Findings**:
- 🔵 **minor** (high) — `classify()` doc-comment update is named but its
  replacement text is unspecified. *Location: Phase 1, Section 2.* The existing
  comment's eager/lazy claim omits the new `work_item_id` lowercasing; without
  the replacement the implementer may recreate a stale comment. Specify the new
  text.
- 🔵 **minor** (medium) — Indexer doc-comment correction under-specified given
  the actual stale text. *Location: Phase 1, Section 3.* The stale comment's
  `Some`/`None` conditions are also wrong (gated on frontmatter `id:`, not
  filename match), and the plan does not call this out. Quote the full
  replacement.
- 🔵 **minor** (medium) — Tripled exact/prefix/interior tiering invites a small
  extract as the field set grows. *Location: Phase 1, Section 2.* Readable at
  three fields, but `external_id` is slated next; consider a `&[Option<&str>]`
  candidate-fields iteration per tier — only if it stays simpler than the
  explicit form.

### Test Coverage

**Summary**: The plan is test-driven and the two proposed integration tests
correctly target the previously-unmatched path, and the red-green claim is sound.
However, the tests assert only match presence (`.any(...)`) and never pin the
bucket/ordering that the Desired End State explicitly promises, so several
mutations of the new code survive. There are also no tests for the prefix tier,
an isolated exact-vs-interior distinction, the negative (non-matching ID) case,
or `work_item_id: None` entries.

**Strengths**:
- Genuinely test-first: tests fail before the change, and the red state is
  credible since neither `0042` nor `42` appears in any seeded field.
- The new fixture helper is well-isolated — a passing assertion can only come
  from the new `work_item_id` path.
- Drives the real axum handler over a temp corpus (integration level at the
  right boundary), consistent with the existing suite.
- Correctly avoids the documented fixture gotcha by writing its own id-bearing
  fixture; `seeded_cfg` already maps `doc_paths['work']` and `id:"0042"`
  normalises to `Some("0042")` under default-numeric.

**Findings**:
- 🟡 **major** (high) — Tests assert match presence but never pin the bucket/tier
  the Desired End State promises. *Location: Phase 1, Section 1 / Testing
  Strategy.* Mutations survive (moving the ExactSlug branch into Prefix/Interior,
  collapsing to a single `contains`). Add a test seeding a competing entry and
  asserting `results[0]`.
- 🟡 **major** (high) — The "exact" test does not isolate the exact tier — query
  `0042` also matches prefix and interior. *Location: Phase 1, Section 1.* The
  test passes even if the exact branch is deleted; the name overstates what is
  verified. Pin via bucket placement.
- 🟡 **major** (high) — No test for the prefix tier of `work_item_id`. *Location:
  Phase 1, Section 2 / Testing Strategy.* Deleting the `id_prefix` term survives
  the suite. Add a strict-prefix test (`004`→`0042`).
- 🔵 **minor** (high) — No negative test for an ID that should NOT match.
  *Location: Phase 1, Section 1 / Testing Strategy.* Add `0099` returns no false
  positive.
- 🔵 **minor** (medium) — No coverage that `work_item_id: None` entries are
  handled safely and don't false-match. *Location: Phase 1, Section 2.* Add an
  assertion that a numeric query returns only the id-bearing work item.
- 🔵 **minor** (medium) — Fixture-population assumption is load-bearing but only
  implicitly verified. *Location: Phase 1, Section 1.* Add a one-line snapshot
  assertion that the fixture entry's `work_item_id` is `Some`, or a comment
  noting the default-numeric reliance.

### Architecture

**Summary**: A tightly-scoped, additive change that correctly localises the fix
to `classify()` without touching the struct, indexer, or wire shape — a sound
evolutionary move that respects the existing field-restricted ranker design. The
decisions to reuse `ExactSlug` and defer `external_id` are explicitly reasoned
and defensible. The main concern is that slotting a third match-field into the
hand-rolled per-tier boolean ladder repeats a pattern showing strain, and the
doc-comment/bucket-naming choices leave latent domain-clarity debt.

**Strengths**:
- Confines the change to a single cohesive site with no struct/indexer/wire-shape
  change — minimal blast radius.
- Reuses existing tiering rather than a new bucket or per-type code path,
  consistent with work item 0054's design.
- "What We're NOT Doing" boundaries are explicit and well-justified.
- Proactively corrects the stale `work_item_id` doc comment.
- Tradeoffs acknowledged rather than hidden.

**Findings**:
- 🔵 **minor** (high) — The revised `classify()` repeats the exact/prefix/interior
  ladder across three fields with `external_id` slated as a fourth; the structure
  does not scale open-closed. *Location: Phase 1, Section 2.* Note that once
  `external_id` lands the per-tier logic should iterate a `[&str]` of candidate
  fields per tier.
- 🔵 **minor** (medium) — `ExactSlug` reuse leaves a domain-alignment gap; the
  variant name no longer matches the concept it represents. *Location: What We're
  NOT Doing — ExactSlug reuse.* Keep the name but add a one-line doc comment that
  it is the "exact identity" tier (exact slug OR exact `work_item_id`).
- 🔵 **minor** (medium) — The unpadded-query tolerance is config-dependent
  (`ENG-0042` under a project code) and untested. *Location: Implementation
  Approach / Desired End State.* Narrow the guarantee to default-numeric, or add
  a project-code fixture test.
- 🔵 **suggestion** (low) — The plan does not record which `external_id`
  evolutionary path this fix biases toward; the change sets a dedicated-field
  precedent. *Location: What We're NOT Doing — external_id deferral.* Add a
  sentence noting the dedicated-field approach is the consistent successor.

### Performance

**Summary**: A small additive change to a search ranker over an in-memory
snapshot of a developer's local `meta/` corpus (hundreds to low-thousands of
entries, not a high-scale endpoint). The plan's "negligible" claim is correct:
the per-entry cost is dominated by the pre-existing full-snapshot clone in the
handler. The only proportionate observation is that `id_lc` is lowercased eagerly
even though `work_item_id` is `None` for most entries, but at this scale this is
immaterial and not worth changing.

**Strengths**:
- Reuses existing tiering — no new passes over the data; stays single-pass O(n)
  per query.
- Interior substring matching gives unpadded tolerance without a precomputation
  pass.
- Preserves lazy lowercasing of `body_preview` (the largest allocation).
- Correctly scoped to `classify()` with no new I/O or change to the
  snapshot/locking model.

**Findings**:
- 🔵 **suggestion** (high) — Eager `id_lc` lowercasing for a sparse field is
  harmless but could be lazy/borrowed. *Location: Performance Considerations;
  Phase 1, Section 2.* Keep as written for readability/consistency; do not
  change. The real lever (if ever needed) is the snapshot clone, not ID
  lowercasing.
- 🔵 **minor** (high) — Per-query cost is dominated by the full-snapshot clone,
  not `classify()`. *Location: Performance Considerations (handler context the
  plan does not change).* `state.indexer.all()` deep-copies every IndexEntry
  including the serde_json frontmatter blob. No action for this work item;
  context only — confirms the plan's "negligible" claim.

### Usability

**Summary**: A well-scoped, additive change that closes a genuine discoverability
gap and honours the existing "try a doc id" empty-state hint. The main concerns:
interior substring matching on zero-padded numeric IDs makes short numeric queries
noisy and unexplained because the result row never shows the matched ID; and
exact-ID hits share the top bucket with exact-slug hits ordered only by mtime, so
an exact ID is not guaranteed to appear first.

**Strengths**:
- Closes a real discoverability gap; the "Try … a doc id" hint becomes truthful.
- Reuses existing tiering — least-surprise extension.
- Unpadded-query tolerance matches how people type IDs.
- Manual verification covers both padded and unpadded queries plus a regression
  check.

**Findings**:
- 🔴/🟡 **major** (high) — Matched ID is invisible in the result row, so ID
  matches look unexplained. *Location: What We're NOT Doing / Phase 1, Section 2.*
  `SearchResultRow` carries no ID; the `Highlight` component highlights only the
  title, so an ID hit highlights nothing. Surface the ID, or call it out as a
  known rough edge rather than a closed decision.
- 🟡 **major** (high) — Short numeric interior matches flood results with
  unexplained hits. *Location: Desired End State / Phase 1, Section 2.* The
  frontend min-length floor of 2 blocks single-digit floods, but two-digit
  numeric queries are common and the backend applies no result cap. Gate ID
  interior matching on a numeric/longer-query heuristic, anchor to the unpadded
  form, and/or cap results.
- 🔵 **minor** (medium) — Exact-ID hit not guaranteed to rank first within the
  shared top bucket. *Location: Desired End State.* Ordered only by
  `(mtime desc, rel_path asc)`; note that exact-ID is co-equal with exact-slug.
- 🔵 **minor** (medium) — Empty-state hint says "doc id" but the feature matches
  only work-item IDs. *Location: What We're NOT Doing — frontend copy.*
  Decisions/plans/reviews have `work_item_id: None`. Keep the vague copy and note
  the limitation, or tighten toward "a work item number".

## Re-Review (Pass 2) — 2026-06-22T16:03:30+00:00

**Verdict:** COMMENT

The revisions resolved the bulk of pass 1: the numeric-form reduction
(`numeric_key`) is logically sound (traced clean across all named edge cases),
the short-numeric flood is genuinely killed (`00` → empty → no match), the
matched-ID-invisibility and `external_id` direction are documented, the
doc-comment texts are specified, and the project-code case is folded in
config-independently. Two pass-1 themes recurred in a new form and warrant
attention but sit **below the configured REVISE threshold** (3+ majors): a
**test-contract defect** (the prefix-only test does not actually isolate the
prefix branch) and a **new flood vector** (a non-numeric project-code fragment
like `eng` substring-matches every work item). Both are concrete and cheap to
fix; the plan is otherwise implementation-ready.

### Previously Identified Issues

- 🟡 **Test Coverage**: Tests assert presence not bucket placement —
  **Resolved**. The 9-item contract pins tiers via ordering against a
  lower-bucket competitor (sound: cross-bucket order is by bucket index, robust
  to temp-dir mtime ties).
- 🟡 **Test Coverage**: "Exact" test doesn't isolate the exact tier —
  **Resolved**. Item 2 (`exact_ranks_above_body`) pins exact via ordering.
- 🟡 **Test Coverage**: No prefix-tier test — **Partially resolved**. A
  prefix test was added, but as specified it does not isolate the prefix branch
  — see new issue below.
- 🟡 **Usability / Correctness**: Short numeric queries flood — **Resolved**
  for the numeric path (`numeric_key` reduces `00` to empty). A non-numeric
  flood variant surfaced — see new issue below.
- 🟡 **Usability**: Matched ID invisible in result row — **Resolved** (now a
  documented, accepted rough edge with a follow-up pointer; re-confirmed minor).
- 🔵 **Correctness**: Exact-id unreachable for prefixed IDs — **Resolved**.
  `numeric_key` makes `42`/`0042` exact-match `ENG-0042`; full key `eng-0042`
  also matches.
- 🔵 **Architecture**: Unpadded tolerance config-dependent/untested —
  **Resolved**. Reduction is config-independent; test item 9 covers project-code.
- 🔵 **Test Coverage**: No negative / no `None`-id test — **Resolved** (items 6
  and 8).
- 🔵 **Test Coverage**: Fixture-population assumption unverified — **Resolved**
  (item 1 asserts `work_item_id == Some(...)`).
- 🔵 **Code Quality**: `classify()` and indexer doc-comment texts unspecified —
  **Resolved** (both quoted; indexer `Some`/`None` conditions corrected).
- 🔵 **Architecture**: `ExactSlug` domain-clarity gap — **Resolved** (one-line
  "exact identity tier" doc comment specified).
- 🔵 **Architecture/Code Quality**: Tripled tiering / `external_id` direction —
  **Resolved** as documented deferral (recurs only as an accepted minor).

### New Issues Introduced

- 🟡 **major** (Correctness + Test Coverage, high) — **Prefix-only test does not
  isolate the prefix branch.** For `004` vs `0042` the reduced forms are `4` vs
  `42`; `4` is *both* a prefix and an interior substring of `42`, so deleting
  `id_prefix` demotes the hit to `Interior` but it is **still returned**. The
  presence-only assertion passes, so the Success-Criteria "mutation check"
  guarantee is false. *Fix*: pin tier via ordering against an interior-only
  competitor (as items 2/3 do), and apply the same to the interior-only test
  (item 5) so it pins `Interior` not just presence.
- 🟡 **major** (Usability, high) — **Non-numeric project-code fragment floods.**
  The non-numeric branch does `full_id.contains(q_lc)`, so under a project-code
  config typing `eng` (or `e`, `-`) substring-matches every `ENG-…` id and
  returns the whole work-item corpus in Interior — the same flood class the
  numeric path was redesigned to avoid. *Fix*: gate the non-numeric id path on
  the query containing a digit, or require a prefix (start-anchored) match for
  the non-numeric id form; document the decision alongside the numeric guard.
- 🔵 **minor** (Usability, high) — Short interior **numeric** matches are
  unintuitive for ID lookup (`2` surfaces `0042`; `42` surfaces `0042` and
  `0420`). Consider dropping interior matching for the numeric id path (exact +
  prefix only) so a number behaves like an identifier lookup, or document the
  fan-out as intentional in the Desired End State.
- 🔵 **minor** (Test Coverage, medium) — `numeric_key` edge cases and unpadded
  prefixed IDs (`normalise_id` passes `OPS-7` through *unpadded*) are untested;
  add cheap direct unit tests over `numeric_key()`.
- 🔵 **minor** (Test Coverage, medium) — Mixed-alphanumeric queries (`eng-42`)
  take neither reduction and so won't match `ENG-0042`; pin or document this
  boundary.
- 🔵 **minor** (Correctness + Architecture, medium/low) — `numeric_key` relies on
  the digits being the trailing run (an invariant enforced in `config.rs`);
  add a one-line note documenting the cross-module assumption.
- 🔵 **minor** (Code Quality, high) — The `id_*` binding block is dense (the
  empty-guard is split across `id_active` and three `!id.is_empty()` checks);
  consider extracting an `id_bucket(id_cmp, id_q) -> Option<Bucket>` helper.
- 🔵 **suggestion** (Performance, high) — Performance Considerations says `id_q`
  is "computed once per query"; in the sketch it is recomputed per entry inside
  `classify()` (cheap, allocation-free). Reword, or hoist `q_is_num`/`id_q` into
  the handler so the claim is literally true.
- 🔵 **minor** (Test Coverage, medium) — Project-code test's unit-test fallback
  would bypass `normalise_id`; if taken, add a population assertion under a
  project-code config.
- 🔵 **suggestion** (Architecture, medium) — When the deferred `external_id`
  refactor lands, prefer precomputing searchable id forms at *index* time so
  `classify()` becomes pure literal matching, rather than keeping `numeric_key`
  query-side.

### Assessment

The plan is in good shape and close to implementation-ready. With 2 major
findings it falls below this project's REVISE threshold (3), so the verdict is
COMMENT — but both majors are worth addressing before implementation because
they are cheap and concrete: the prefix-test fix is a one-competitor ordering
assertion, and the non-numeric flood needs a single accept/reject decision
(digit-gate or start-anchor the non-numeric id path) mirroring the numeric guard
already made. The minor cluster (numeric-path interior fan-out, `numeric_key`
unit tests, the dense `id_*` block, the per-query/per-entry wording) can be
folded in opportunistically or deferred.

---
*Re-review generated by /accelerator:review-plan*
