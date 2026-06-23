---
type: plan-review
id: "2026-06-22-0125-search-id-matching-review-2"
title: "Plan Review: Visualiser Search Work-Item-ID Matching"
date: "2026-06-22T16:49:13+00:00"
author: "Phil Helm"
producer: review-plan
status: complete
parent: "plan:2026-06-22-0125-search-id-matching"
target: "plan:2026-06-22-0125-search-id-matching"
relates_to: ["plan-review:2026-06-22-0125-search-id-matching-review-1"]
reviewer: "Phil Helm"
verdict: "COMMENT"
lenses: [correctness, code-quality, test-coverage, architecture, performance, usability]
review_number: 2
review_pass: 2
tags: [visualiser, search, indexer]
last_updated: "2026-06-23T08:55:02+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Plan Review: Visualiser Search Work-Item-ID Matching

**Verdict:** REVISE

The plan has materially improved since review-1: the `numeric_key()` reduction is
logically sound across the edge cases (`00`→empty, unpadded `ops-7`, project-code
`ENG-0042`), the numeric-path flood is genuinely killed, the project-code case is
folded in config-independently, the `MAX_RESULTS` cap addresses review-1's
non-numeric flood, and the doc-comment texts and evolutionary direction are now
specified. Architecture and performance are sound and the plan's "negligible
cost" claim holds. The verdict is REVISE not because the design is wrong but
because the **test contract does not yet pin several behaviours its own Success
Criteria claim it pins** — most importantly, the headline prefix-isolation test
(item 4) relies on an mtime tiebreak the fixture never controls, so the very
mutation it advertises catching can survive. A second cluster concerns the
**non-numeric id path**, which is now the feature's weak spot: its prefix/interior
branches are untested, an unpadded full key (`eng-42`) silently misses, and the
project-code fragment flood leans entirely on the cap. None are
structural-redesign blockers; all are addressable with targeted edits to the test
plan plus two small scope/wording decisions.

### Cross-Cutting Themes

- **The test contract pins less than its Success Criteria claim** (flagged by:
  test-coverage, correctness) — Item 4 (prefix-outranks-interior) depends on the
  `(Reverse(mtime), rel_path)` within-bucket sort, but `write_work_item` never
  equalises mtimes, so with `id_prefix` deleted the newer file can still sort
  first and the mutation survives. The cap test (item 10) cannot distinguish
  "truncate before vs after bucket sorting" as claimed, because the single
  ExactSlug target lands at `results[0]` either way. And the `numeric_key()` "unit
  tests, no AppState" cannot compile where the plan implies, since the helper is
  private to `src/api/search.rs` while the tests live in the separate
  `tests/api_search.rs` crate.

- **The non-numeric id path is the new weak spot** (flagged by: test-coverage,
  correctness, usability) — Only the non-numeric *exact* tier is tested (item 9);
  the non-numeric `id_prefix`/`id_interior` branches — which back the "type a full
  prefixed key or a project-code fragment" promise — are unpinned, so deleting or
  anchoring them ships green. Behaviourally, the same branch makes `eng-42`
  silently match nothing against `ENG-0042` (unpadded tolerance is numeric-only),
  and a bare `eng` substring-matches the whole work-item corpus, bounded only by
  the cap.

- **`MAX_RESULTS` is broader and blunter than its "flood backstop" framing**
  (flagged by: correctness, architecture, performance, usability) — Applied in the
  shared projection chain, the cap silently changes the result-volume contract for
  *every* query type (title/slug/body included), not just id floods — a 0054
  contract shift bundled into an id fix. It bounds the response payload but not the
  per-query CPU (snapshot clone, per-entry lowercasing, matched-set sort remain
  O(corpus)), and its truncation is invisible to the user.

- **The frontend layer is under-modelled in the spec** (flagged by: usability) —
  The 2-char min-length gate (`use-search.ts`) makes the single-digit interior
  behaviour described in the Desired End State and a manual-verification step
  unreachable through the UI, and the matched id never appears anywhere on the
  result row (the slug shown is the tail *after* the id), so an id-matched row
  gives no confirmation of why it matched.

### Tradeoff Analysis

- **Cap generality vs contract precision**: The cap-over-heuristics decision is
  architecturally the right least-surprise call, and the plan argues it well. The
  cost is that it changes the global search contract and is non-ID-specific.
  Recommendation: keep the cap, but state explicitly that it hardens *all* query
  paths (not just id floods) and reference the `MAX_RESULTS` constant from the
  test rather than the literal `50` (the plan itself calls the value
  non-contractual).

- **Scope discipline vs explainability (invisible matched id)**: The plan already
  documents this as an accepted rough edge with a follow-up pointer — that
  decision stands. The usability lens sharpens *why* it stings (the row shows
  none of the typed digits, not merely "no highlight"). Recommendation: keep
  deferring, but consider the near-free win of rendering the full filename
  (`0042-login-form`) in the existing sub-line so the id is at least visible
  without a wire-shape change.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Test Coverage**: Item 4's prefix-isolation depends on an mtime tiebreak the fixture never pins
  **Location**: Phase 1, Section 1, item 4 (`work_item_id_prefix_outranks_interior`) + `write_work_item` sketch
  The within-bucket sort is `(Reverse(mtime_ms), rel_path)`; `rel_path` only breaks ties when mtimes are equal. `write_work_item` writes files sequentially and never calls `set_mtime_ms`, so the target and competitor get different mtimes and the newer sorts first regardless of `rel_path`. With `id_prefix` deleted, both fall to `Interior` but the order may not flip — the headline mutation the Success Criteria promises to catch can survive.

- 🟡 **Test Coverage**: Non-numeric `id_prefix`/`id_interior` branches are never pinned
  **Location**: Phase 1, Section 1, item 9 + classify() non-numeric branch
  The contract tests the non-numeric path only at the exact tier (`q=eng-0042`). No test exercises a non-numeric prefix (`eng-00`/`eng`) or interior (`ng-00`) match, so a mutation that drops or prefix-anchors those branches passes the whole suite — yet they back the "type a full prefixed key or a project-code fragment" promise.

- 🟡 **Test Coverage**: `numeric_key()` "unit tests, no AppState" cannot live in the integration crate
  **Location**: Phase 1, Section 1, final paragraph; item 9 fallback
  `numeric_key()` (and `classify()`) are private to `src/api/search.rs`; the tests live in `tests/api_search.rs`, a separate crate seeing only the `pub` surface. As written these unit tests won't compile, and the plan designates them as the pins for "the riskiest new logic". The plan should specify a `#[cfg(test)] mod tests` inside `search.rs` (or `pub(crate)` the helper).

- 🟡 **Test Coverage**: Cap test does not distinguish "truncate before vs after bucket sorting"
  **Location**: Phase 1, Success Criteria (mutation checks) + item 10
  The Success Criteria claim the cap test "fails if truncation is applied before bucket sorting." But the single ExactSlug target lands at `results[0]` whether `take()` runs before or after the per-bucket sort, so item 10's two assertions are identical under both implementations — the stated mutation is not actually caught.

- 🟡 **Usability**: Frontend 2-char min-length gate makes single-digit interior matching unreachable
  **Location**: Desired End State + Phase 1 Manual Verification
  `use-search.ts` only searches at `length >= 2`. The Desired End State and the "typing a single common digit" manual-verification step describe behaviour the UI can never trigger — a reviewer following the steps with one digit sees nothing.

- 🟡 **Usability**: Id-matched rows give zero feedback about *why* they matched
  **Location**: What We're NOT Doing — "Surfacing the matched ID"
  The row's title gets `Highlight` (no-op for an id query), and the sub-line shows `docType/slug` where slug is the tail *after* the id — so the typed number appears nowhere. The plan accepts this as a rough edge (decision stands), but the framing ("highlights nothing") understates that the row offers no confirmation at all.

#### Minor

- 🔵 **Correctness**: Unpadded full-key query (`eng-42`) silently matches nothing against `ENG-0042`
  **Location**: Desired End State / Phase 1, Section 2
  Unpadded tolerance applies only to purely-numeric queries; `eng-42` takes the non-numeric branch and fails exact/prefix/interior against `eng-0042`, even though `42` and `eng-0042` both find it. Whether it matches depends on how the id was stored (`normalise_id` keeps `ENG-42` unpadded). Document the gap or extend the reduction to a CODE-prefixed query's trailing digit run.

- 🔵 **Correctness / Architecture**: `MAX_RESULTS` silently changes the existing title/slug/body result-volume contract
  **Location**: Phase 1, Section 4 / Desired End State
  The cap sits in the shared projection chain, so it bounds every query type, not just id floods — a behavioural shift to a pre-existing concern (0054 implicitly returned all matches) bundled into an id fix. Sound, but the "existing ranking unchanged" framing under-states it. Reword to state the cap is general search-handler hardening; optionally land it as its own revertable commit.

- 🔵 **Test Coverage**: Cap test couples to the literal `MAX_RESULTS` value and risks tier bleed
  **Location**: Phase 1, Section 1, item 10
  Asserting `results.len() == 50` freezes a value the plan calls non-contractual, and the 51+ generated flood keys must be constructed so none accidentally exact/prefix-matches the query. Reference the `MAX_RESULTS` constant and generate interior-only-matching keys.

- 🔵 **Test Coverage**: All-zeros flood guard (item 7) needs fixtures' title/body explicitly free of `00`
  **Location**: Phase 1, Section 1, item 7 + `write_work_item`
  The bucket is unobservable, so the test asserts absence from results — which silently also requires neither fixture's title/body contains `00`. The helper doesn't enforce it; pin titles/bodies that exclude `00`/`0` so the test verifies the `id_active` empty-key guard, not incidental field contents.

- 🔵 **Code Quality**: The `id_*` binding block is dense and carries an undocumented empty-guard asymmetry
  **Location**: Phase 1, Section 2 — the `id_*` bindings
  Seven new bindings with inline closures pack the function's new cognitive load; the reason `id_exact` relies on `id_active` while prefix/interior re-check `!id.is_empty()` is subtle. Consider extracting `id_bucket(id_lc, q_lc) -> Option<Bucket>` (mirroring the `numeric_key` extraction), or at least an inline comment.

- 🔵 **Correctness**: Project-code test (item 9) must assert against the normalised `work_item_id`, not the filename id
  **Location**: Phase 1, Section 1, item 9
  Under a project-code config, `id: "0042"` normalises to `work_item_id: ENG-0042` while the filename stays `0042-…`. A test authored against the filename form would assert the wrong value or pass for the wrong reason. Add the item-1-style population guard (`Some("ENG-0042")`) first, then the query assertions.

- 🔵 **Performance**: `MAX_RESULTS` bounds the payload but not the snapshot clone, per-entry lowercasing, or the matched-set sort
  **Location**: Performance Considerations
  The cap runs after `state.indexer.all()` clones the corpus, after every entry is title-lowercased, and after `sort_by_cached_key` allocates a `rel_path` key per match. A bare-code flood still does O(corpus) work before truncation. Acceptable at scale; note the cap bounds the *response*, not per-query CPU.

- 🔵 **Usability**: "a doc id" empty-state hint over-promises for `None`-id document types
  **Location**: What We're NOT Doing — frontend copy
  Only work items carry a matchable id; decisions/plans/reviews/RCAs remain unmatchable. The cheapest user-facing accuracy win is to tighten the copy to "a work item number" in the same PR (one string + its snapshot) rather than defer.

- 🔵 **Usability**: Bare-project-code query returns a bounded but undifferentiated, silently-capped wall
  **Location**: Desired End State / What We're NOT Doing
  `eng` returns up to 50 near-identical rows (no on-row id) with no signal that results were truncated. Consider having the match-count meta row signal truncation ("50+ matches — narrow your query"); low priority, pairs with the id-chip follow-up.

#### Suggestions

- 🔵 **Code Quality / Performance**: Hoist `q_is_num`/`id_q` into the handler
  **Location**: Phase 1, Section 2 / Performance Considerations
  Both depend only on the query, yet are recomputed per entry inside `classify()`. Hoisting computes them once and makes `classify()` read as "match this entry against a prepared query" — clarity over a measurable win, as the plan already notes. This also makes the "computed once per query" wording literally true.

- 🔵 **Code Quality**: Put the ladder-refactor trigger at the call site, not only in the plan
  **Location**: What We're NOT Doing — tripled tiering
  The exact/prefix/interior comparison is now hand-written three times with `external_id` slated as a fourth. The deferral is fine, but add a one-line note near `classify()` itself so the next author meets the "refactor when a third matchable id field lands" threshold at the code, not in an archived plan.

- 🔵 **Architecture**: Record that the `external_id` successor needs a *per-field* numeric-reduction policy
  **Location**: What We're NOT Doing — external_id deferral
  `numeric_key` assumes the work-item id shape (trailing digit run). External ids (`PP-76`) would let a numeric query collide across both candidate fields once the ladder iterates them, so the successor needs a per-field policy for which fields participate in bare-numeric matching — not a single shared `numeric_key`.

- 🔵 **Architecture**: Tie the index-side precompute option to a concrete trigger
  **Location**: Implementation Approach / Performance Considerations
  Keeping `numeric_key` query-side is correct now. Note that moving to index-time precompute becomes worth it only if the per-query snapshot clone is eliminated (so recomputation stops being free relative to it) or candidate id fields multiply — so a future contributor knows when the boundary should move.

- 🔵 **Test Coverage**: Add an integration case for a foreign-prefix id distinct from the configured code
  **Location**: Phase 1, Section 1, item 9
  `numeric_key` covers `ops-7`→`7` at the unit level, but no integration test confirms an `OPS-7` item (distinct from the configured `ENG`) is findable by `q=7` and `q=ops-7` end-to-end — the multi-prefix coexistence `normalise_id` is designed for.

- 🔵 **Correctness**: Note that any all-zeros numeric query (`0`, `000`) is an intentional id-path no-op
  **Location**: Phase 1, Section 2 — `q_is_num` derivation
  The `00` flood-guard logic generalises: `0`/`000` also reduce to empty and match no id. Internally consistent and bounded; just note the test contract's `00` case stands in for the whole all-zeros class.

### Strengths

- ✅ The pass-1 substance is resolved: `numeric_key` traces clean across all named
  edge cases, the numeric flood is genuinely killed, and the project-code case is
  config-independent.
- ✅ Tightly scoped and well-grounded — confines matching to `classify()` with no
  struct/indexer/wire-shape change; the `numeric_key`/`classify` split keeps the
  ranking logic pure and unit-testable in the imperative shell.
- ✅ The cap-over-heuristics decision is the right least-surprise call and is
  argued on architectural grounds, aligned with the existing `MAX_Q_LEN`
  precedent; truncation after bucket ordering correctly protects exact/prefix
  hits.
- ✅ Tier placement is (mostly) pinned by relative ordering against an
  adjacent-bucket competitor rather than presence — the correct mutation-resistant
  technique, matching the existing `bucket_*` suite.
- ✅ Negative, degenerate, and `None`-id cases are covered (items 6, 7, 8), and the
  fixture-population guard (item 1) inverts the documented `seeded_cfg_with_work_items`
  gotcha so a config regression names its own cause.
- ✅ Doc-comment corrections (both the `classify()` function comment and the stale
  `indexer.rs` struct comment, including its `Some`/`None` conditions) are quoted
  in full and verified accurate against the population path.
- ✅ Performance claim is accurate: the added cost is dwarfed by the pre-existing
  full-snapshot clone, which the plan correctly leaves untouched.
- ✅ Evolutionary direction for the deferred `external_id` (dedicated-field
  successor + ladder refactor) and the `ExactSlug` overloading are documented
  rather than left implicit.

### Recommended Changes

1. **Fix the prefix/interior tier-isolation tests to control mtime**
   (addresses: "Item 4's prefix-isolation depends on an mtime tiebreak", "All-zeros
   flood guard needs fixtures free of the query")
   Have item 4 (and item 5) set both target and competitor to the *same*
   `mtime_ms` via `common::set_mtime_ms`, so the `rel_path` tiebreak deterministically
   fires and the deleted-`id_prefix`/`id_interior` mutation actually flips the
   order. State this requirement in the contract.

2. **Pin the non-numeric id prefix and interior branches**
   (addresses: "Non-numeric `id_prefix`/`id_interior` branches are never pinned")
   Add at least one non-numeric prefix-tier test (e.g. project-code config,
   `q=eng-00` against `ENG-0042` + a body-only competitor, ordering asserted) and
   ideally a non-numeric interior case, mirroring items 4–5.

3. **Specify where the `numeric_key()` unit tests live**
   (addresses: "`numeric_key()` unit tests cannot live in the integration crate")
   State that they go in a new `#[cfg(test)] mod tests` inside `src/api/search.rs`
   (same crate, can call the private helper), distinct from the integration
   contract in `tests/api_search.rs`.

4. **Correct the cap test's stated mutation guarantee**
   (addresses: "Cap test does not distinguish truncate before vs after bucket
   sorting", "Cap test couples to the literal value")
   Either drop the "applied before bucket sorting" clause from the Success
   Criteria, or add an assertion that exercises it (e.g. seed >`MAX_RESULTS`
   Prefix hits plus some Interior hits and assert the truncated set is all
   Prefix-tier). Reference the `MAX_RESULTS` constant rather than `50`.

5. **Reconcile the frontend layer with the spec**
   (addresses: "2-char gate makes single-digit matching unreachable", "a doc id
   hint over-promises")
   Reword the Desired End State and manual-verification steps to use 2+ character
   examples (or note single-digit is server-only), and tighten the empty-state
   copy to "a work item number" in the same PR.

6. **Sharpen the cap and non-numeric documentation**
   (addresses: "`MAX_RESULTS` silently changes the title/slug/body contract",
   "Unpadded full-key `eng-42` silently misses")
   State that the cap is general search-handler hardening (affects all query
   paths), and add the `eng-42`-misses gap to "What We're NOT Doing" (or extend
   the reduction to a CODE-prefixed query's trailing digit run).

7. **Fold in the low-effort notes** (addresses: the suggestion cluster)
   Per-field numeric-reduction policy for the `external_id` successor; the
   index-side precompute trigger; the call-site ladder-refactor trigger; the
   all-zeros generalisation note; and (optional) hoisting `q_is_num`/`id_q`.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The `classify()` sketch and `numeric_key()` helper are logically
sound for the canonical id shapes guaranteed by `WorkItemConfig::normalise_id`
(bare digits, `CODE-digits`, or foreign-prefix forms with the digit run always
trailing), and the boolean ladder correctly handles the degenerate `00`→empty and
`None`-id cases via the `id_active` guard. The numeric-vs-non-numeric matching
split introduces a least-surprise asymmetry: unpadded tolerance exists only for
purely-numeric queries, so an unpadded full key like `eng-42` silently matches
nothing against `ENG-0042`. The `MAX_RESULTS` cap is applied correctly (after
bucket concatenation, after `filter_map`) but silently changes title/slug/body
result volume too.

**Strengths**:
- The `00`→empty / all-zeros degenerate case is correctly neutralised: id_q="",
  id_active=false, every id_* boolean short-circuits — matched by flood test 7.
- `numeric_key()`'s trailing-digit-run extraction is correct for every shape
  `normalise_id` can produce, and the doc comment scopes the assumption and flags
  the future-id-shape risk.
- `take(MAX_RESULTS)` after `flatten().filter_map(project)` is correct: it counts
  only successfully-projected rows and drops only the lowest-relevance tail.
- `None`-id and empty-query preconditions are explicitly reasoned about; is_some_and
  guards prevent any None/empty dereference.

**Findings**:
- 🔵 **minor** (high) — Unpadded full-key query (`eng-42`) silently matches nothing
  against `ENG-0042`. *Location: Desired End State; Phase 1, Section 2.* The
  non-numeric branch fails exact/prefix/interior; whether it matches depends on
  how the id was stored. Document, or extend the reduction to a CODE-prefixed
  query's trailing digit run.
- 🔵 **minor** (high) — `MAX_RESULTS` silently changes existing title/slug/body
  result volume, not just id matches. *Location: Phase 1, Section 4.* The cap is in
  the shared projection chain; the "existing ranking unchanged" framing
  under-states its scope.
- 🔵 **minor** (medium) — `q_is_num` treats a single `0` query as numeric, making
  it match no id. *Location: Phase 1, Section 2 — q_is_num.* Internally consistent
  and bounded; note the `00` case generalises to all-zeros.
- 🔵 **minor** (medium) — Project-code config diverges fixture filename id from the
  stored `work_item_id`. *Location: Phase 1, Section 1, item 9.* Assert against the
  normalised `ENG-0042`, not the filename form; add the population guard first.

### Code Quality

**Summary**: An unusually well-reasoned small change: it isolates the single
defect site, threads the new logic through the existing bucket tiers, and
documents its accepted trade-offs explicitly. The main code-quality risks are
local readability ones — the dense `id_*` binding block packs a lot of
conditional logic inline, and the exact/prefix/interior ladder is now tripled
across title/slug/id (with `external_id` flagged as a future fourth copy), which
the plan acknowledges but defers. Naming, helper extraction, doc-comment
corrections, and testability are handled thoughtfully.

**Strengths**:
- The stale `IndexEntry.work_item_id` doc comment is correctly identified as wrong
  on both counts and a concrete, verified replacement is supplied.
- `numeric_key()` is extracted as a named, unit-tested helper with a doc comment
  stating its assumption and failure mode.
- Extends the existing bucket tiering in-place rather than building a separate id
  path; the lazy `body_preview` lowercasing is preserved in the updated comment.
- Accepted trade-offs are documented at their definition sites and in "What We're
  NOT Doing".
- `MAX_RESULTS` is justified by analogy to the existing `MAX_Q_LEN` cap.

**Findings**:
- 🔵 **minor** (medium) — The `id_*` binding block is dense and carries an
  undocumented empty-guard asymmetry. *Location: Phase 1, Section 2.* Consider an
  `id_bucket(id_lc, q_lc) -> Option<Bucket>` extraction (mirroring `numeric_key`),
  or an inline comment on why `id_exact` need not re-check emptiness.
- 🔵 **minor** (high) — The tripled matching ladder is a DRY smell the plan defers
  acting on. *Location: What We're NOT Doing.* Defensible for this scope; make the
  trigger explicit with a one-line note near `classify()` itself, not only in the
  archived plan.
- 🔵 **suggestion** (medium) — `MAX_RESULTS` test couples to the literal value the
  plan calls non-contractual. *Location: Phase 1, Section 4 / item 10.* Reference
  the constant and seed `MAX_RESULTS + N` via it.
- 🔵 **suggestion** (low) — `id_q`/`q_is_num` are query-level facts computed in the
  per-entry hot loop. *Location: Phase 1, Section 2.* Lean toward hoisting to the
  handler for conceptual clarity.

### Test Coverage

**Summary**: The plan is unusually test-forward — a 10-item integration contract
plus direct `numeric_key` unit tests, with most tiers pinned by relative ordering
against an adjacent-bucket competitor rather than mere presence, the right
mutation-resistant technique. However, several pins are weaker than claimed: the
prefix-outranks-interior test relies on an mtime tiebreak the fixture never
controls, the cap test does not distinguish the "truncate before vs after bucket
sorting" mutation the Success Criteria assert, the non-numeric id prefix/interior
branches are never exercised, and the "direct `numeric_key()` unit tests" cannot
live where the plan implies because the helper is private to a separate test
crate. Fixable contract refinements, not a flawed strategy.

**Strengths**:
- Tier placement is pinned by relative ordering against a deliberately-seeded
  adjacent-bucket competitor (items 2–5, 9), matching the existing `bucket_*`
  convention.
- Item 1 (fixture-population guard) inverts the documented
  `seeded_cfg_with_work_items` gotcha, surfacing config regressions by name.
- The contract is explicit about red-before-green and enumerates specific
  mutations to catch — a genuine mutation-testing attempt.
- Negative (item 6), all-zeros flood (item 7), and `None`-id (item 8) cases all
  guard real edge behaviours of the bare-numeric reduction.

**Findings**:
- 🟡 **major** (high) — Item 4's mutation-resistance depends on an mtime tiebreak
  the fixture never pins. *Location: Phase 1, Section 1, item 4 + write_work_item.*
  The within-bucket sort is `(Reverse(mtime), rel_path)`; sequential writes give
  unequal mtimes, so a deleted `id_prefix` may not flip the order. Set both
  fixtures to the same `mtime_ms` via `set_mtime_ms`.
- 🟡 **major** (medium) — Cap test does not distinguish "truncate before vs after
  bucket sorting" as the Success Criteria claim. *Location: Success Criteria +
  item 10.* The single ExactSlug target lands at `results[0]` either way. Drop the
  clause or add an assertion that exercises it.
- 🟡 **major** (high) — Direct `numeric_key()`/`classify()` unit tests cannot live
  in the integration test crate. *Location: Phase 1, Section 1, final paragraph.*
  Both helpers are private; the tests would not compile from `tests/api_search.rs`.
  Specify a `#[cfg(test)] mod tests` inside `search.rs`.
- 🟡 **major** (high) — Non-numeric id prefix and interior branches are never
  pinned. *Location: item 9 + classify() non-numeric branch.* Only the non-numeric
  exact tier is tested; deleting/anchoring the prefix/interior branches ships
  green. Add a non-numeric prefix (and ideally interior) tier test.
- 🔵 **minor** (high) — All-zeros flood guard needs its fixtures' title/body
  explicitly free of the query. *Location: item 7.* The absence assertion silently
  also requires no field contains `00`; pin titles/bodies that exclude `00`/`0`.
- 🔵 **minor** (medium) — Cap test couples to the `MAX_RESULTS` value and risks
  tier bleed in the flood fixtures. *Location: item 10.* Reference the constant;
  generate interior-only-matching keys.
- 🔵 **suggestion** (medium) — No integration coverage of a foreign-prefix id
  distinct from the configured project code. *Location: item 9.* Add an `OPS-7`
  item under an `ENG` config, findable by `q=7` and `q=ops-7`.

### Architecture

**Summary**: A well-scoped, additive change that slots id matching into the
existing four-bucket ranker without new structural seams, keeps the matching logic
as pure testable functions in the imperative shell, and documents its tradeoffs
and evolutionary direction thoroughly. The two genuine architectural decisions —
bounding output via `MAX_RESULTS` rather than gating input via heuristics, and
deferring `external_id` to a dedicated-field successor with a ladder refactor —
are sound and explicitly justified. The main observations are that the cap is a
cross-cutting behavioural change to the whole search contract bundled into an id
fix, and that the documented `external_id` successor will inherit a numeric-key
generalisation problem the plan should name now.

**Strengths**:
- Additive design reusing the existing tiering rather than a new bucket or
  per-type id path — minimal new structure, consistent with 0054.
- Clean functional-core / imperative-shell separation: `numeric_key()`/`classify()`
  pure and unit-testable; the volume cap in the handler.
- Evolutionary direction for `external_id` explicitly documented (dedicated-field
  option, candidate-field ladder refactor).
- The cap-vs-heuristics tradeoff is argued on architectural grounds and aligned
  with the `MAX_Q_LEN` precedent.
- Boundary-touching tradeoffs are each surfaced in "What We're NOT Doing".

**Findings**:
- 🔵 **minor** (high) — `MAX_RESULTS` is a cross-cutting search-contract change
  bundled into an id-matching fix. *Location: Phase 1, Section 4.* It truncates
  every query path, not just id floods — a 0054 contract shift. Call out the
  general scope; optionally land it as its own revertable commit.
- 🔵 **minor** (medium) — The documented `external_id` successor inherits a
  numeric-key cross-field generalisation hazard. *Location: What We're NOT Doing.*
  `numeric_key` assumes the work-item id shape; iterating candidate fields would
  let a numeric query collide across fields. Flag the need for a per-field policy.
- 🔵 **suggestion** (high) — Query-side `numeric_key` is right now; record the
  index-side trigger condition. *Location: Implementation Approach.* Tie the
  boundary move to eliminating the per-query clone or multiplying id fields.
- 🔵 **suggestion** (high) — `ExactSlug` overloading is acceptable but weakens
  domain alignment. *Location: What We're NOT Doing.* Fine to ship; fold a rename
  into the future `external_id` ladder refactor that already touches these
  branches.

### Performance

**Summary**: The Performance Considerations section is accurate and appropriately
proportionate for the stated scale (a search ranker over an in-memory snapshot of
hundreds to low-thousands of local entries). It correctly identifies the
pre-existing full-snapshot clone as the only meaningful per-query cost, leaves it
untouched, and honestly flags the per-entry recompute of numeric-query state as a
readability-not-performance choice. The new work adds negligible cost and the cap
is a net positive for broad-match payloads.

**Strengths**:
- Correctly identifies the dominant per-query cost (`state.indexer.all()` clones
  every `IndexEntry` including the frontmatter blob) and rightly leaves it out of
  scope.
- The lazy `body_preview` lowercasing optimisation is preserved in the new sketch.
- `MAX_RESULTS` is applied via `.take()` after `filter_map(project)`, genuinely
  bounding `project()` clones and JSON serialisation for broad-match queries.
- `numeric_key()` and the `q_is_num`/`id_q` derivation are borrowed-`&str` scans
  over the short query with no heap allocation.
- Honest about the recompute tradeoff, framing the hoist as clarity not a win.

**Findings**:
- 🔵 **suggestion** (high) — Per-entry recompute of `q_is_num`/`id_q` is correct to
  note but trivially hoistable. *Location: Performance Considerations; Phase 1,
  Section 2.* Both are loop invariants; hoisting removes them at zero behavioural
  cost.
- 🔵 **suggestion** (high) — `id_lc` lowercasing allocates only for the sparse
  populated subset, not every entry. *Location: Performance Considerations.* Tighten
  the wording to "per id-bearing work item".
- 🔵 **minor** (medium) — `MAX_RESULTS` bounds payload/serialisation but not the
  snapshot clone, per-entry lowercasing, or the matched-set sort. *Location:
  Performance Considerations.* A bare-code flood still does O(corpus) work before
  truncation; note the cap bounds the response, not per-query CPU.

### Usability

**Summary**: The plan delivers a genuine DX win — making work-item numbers
findable closes a long-standing least-surprise gap. The accepted-tradeoff framing
is mostly honest and well-reasoned (the cap over matching heuristics is the right
least-surprise call). However, the plan reasons about the search experience almost
entirely at the server/ranking layer and under-weights the frontend gate and
result-row presentation: a 2-character min-length gate makes part of the
documented interior fan-out and a manual-verification step unreachable, and the
"invisible matched id" is sharper than the plan implies because the row never
displays the id anywhere, so an id-matched row gives zero feedback about why it
matched.

**Strengths**:
- Closes a real least-surprise defect: typing a work item's number now works.
- Unpadded/config-independent numeric matching matches how developers refer to
  items.
- Choosing a general volume cap over per-field gating is the right least-surprise
  call and is documented.
- Bucket-ordered truncation guarantees exact/prefix hits are never dropped.
- Reuses the existing tiering rather than a parallel ranking concept.

**Findings**:
- 🟡 **major** (high) — Frontend 2-char min-length gate makes single-digit interior
  matching unreachable. *Location: Desired End State + Manual Verification.*
  `use-search.ts` only searches at `length >= 2`; the single-digit examples and
  manual step can never trigger through the UI.
- 🟡 **major** (high) — Id-matched rows give zero feedback about why they matched.
  *Location: What We're NOT Doing.* The row's title highlight no-ops and the
  sub-line shows the slug tail (after the id), so the typed number appears nowhere.
  Decision to defer stands, but the framing understates the gap.
- 🔵 **minor** (high) — "a doc id" empty-state hint over-promises for `None`-id doc
  types. *Location: What We're NOT Doing — frontend copy.* Tighten to "a work item
  number" in the same PR (one string + snapshot).
- 🔵 **minor** (medium) — Bare-project-code query returns a bounded but
  undifferentiated, silently-capped wall. *Location: Desired End State.* Consider
  signalling truncation in the match-count row; low priority.
- 🔵 **minor** (medium) — All-zeros query silently returns nothing with no
  actionable feedback. *Location: Desired End State.* Falls through to the generic
  "No matches"; acceptable, just note it is intentionally indistinguishable from a
  normal miss.

## Re-Review (Pass 2) — 2026-06-23T08:55:02+00:00

**Verdict:** COMMENT

The pass-1 (review-2) edits resolve the substance across all five re-run lenses.
Correctness traced the new `eng-42` gap, the `MAX_RESULTS` hardening note, the
all-zeros generalisation, and item 9's normalised-id assertion all clean, with no
new defects. Architecture and code-quality confirmed the new notes are sound and
internally consistent. Test-coverage confirmed the four major test-contract fixes
(item 4 mtime control, in-module `numeric_key` tests, the honest cap Success
Criteria, the constant-referencing cap fixtures) — **but caught one real defect
the edits introduced**: the new item 11 prescribed an interior-only competitor
*whose id contains but does not start with `eng-00`*, which is not constructible
(every normalised id is `eng-<digits>`, so `eng-00` can only occur at offset 0).
That single major was fixed in a follow-up edit (competitor now interior-matches
via a non-id field), along with the related clarity/consistency items. With one
major (now resolved) the verdict is COMMENT; after the follow-up edits the plan
has no outstanding actionable findings and is implementation-ready.

### Previously Identified Issues

- 🟡 **Test Coverage**: Item 4's prefix-isolation depended on an unpinned mtime
  tiebreak — **Resolved**. The "mtime control" subsection + explicit
  `set_mtime_ms` requirement make the `rel_path` tiebreak load-bearing; mechanism
  verified sound (`0014-zzz` sorts before `0042`).
- 🟡 **Test Coverage**: Non-numeric `id_prefix`/`id_interior` branches unpinned —
  **Resolved** (items 11/12 added), though item 11 as first written was not
  constructible — see New Issues; now fixed.
- 🟡 **Test Coverage**: `numeric_key()` unit tests couldn't compile in the
  integration crate — **Resolved**. In-module `#[cfg(test)] mod tests` in
  `search.rs` now specified, with a note not to widen visibility.
- 🟡 **Test Coverage**: Cap test couldn't distinguish truncate-before-vs-after
  sorting — **Resolved**. Success Criteria now candid; optional order-pinning
  assertion added (with an observability marker convention).
- 🟡 **Usability**: 2-char gate made single-digit matching unreachable —
  **Resolved**. Manual step now uses a 2+ char query and notes the `use-search.ts`
  gate; remaining single-digit mentions are correctly server-side flood-vector
  descriptions.
- 🟡 **Usability**: Matched id invisible in the result row — **Accepted/Resolved**
  (deferral decision taken; documented as an accepted rough edge with a follow-up
  pointer).
- 🔵 **Correctness**: `eng-42` silently misses — **Resolved** (documented as a
  known gap; the prose, including the unpadded-`ENG-42` subtlety, traced correct).
- 🔵 **Correctness/Architecture**: `MAX_RESULTS` changes the all-query contract —
  **Resolved** (general-hardening note added; accurate).
- 🔵 **Correctness**: single-`0`/all-zeros no-op — **Resolved** (generalisation
  documented).
- 🔵 **Test Coverage**: item 7 fixtures / item 9 normalised-id assertion / cap
  constant-coupling — **Resolved**.
- 🔵 **Code Quality**: dense `id_*` block, call-site refactor trigger, hoist —
  **Resolved** (implementer-notes block added).
- 🔵 **Architecture**: `external_id` per-field policy, index-side trigger,
  ExactSlug rename window — **Resolved** (notes added; rename folded into the
  refactor scope).

### New Issues Introduced (and addressed)

- 🟡 **major** (Test Coverage, high) — **Item 11's interior-only id competitor was
  not constructible** (`eng-00` can only be a prefix of an `eng-<digits>` id).
  *Fixed*: the competitor now interior-matches `eng-00` via a non-id field (title),
  with an id that doesn't prefix-match, and the same-mtime/`rel_path` flip
  mechanism stated.
- 🔵 **minor** (Test Coverage, high) — Item 12's "Prefix competitor on the same
  fragment" cannot be id-based. *Fixed*: now specifies the competitor prefix-matches
  via title/slug.
- 🔵 **minor** (Test Coverage, medium) — The red/green Success-Criteria bullet
  still said "items 1–10", omitting 11/12. *Fixed*: now "items 1–12".
- 🔵 **minor** (Test Coverage, low) — The optional cap-order assertion lacked a
  tier-observability mechanism. *Fixed*: added a slug/title marker convention.
- 🔵 **minor** (Usability, medium) — The widened cap's truncation is unsignalled at
  the UI meta-row. *Fixed*: documented as an accepted rough edge with a follow-up
  pointer.
- 🔵 **minor** (Code Quality, high) — The hoist was recommended more strongly in the
  implementer note than in Performance Considerations. *Fixed*: softened to
  "optionally" and cross-referenced.

### Assessment

The plan is implementation-ready. All review-2 findings are resolved, and the one
genuine defect introduced during iteration (item 11's non-constructible fixture)
was caught by the re-review and fixed. No outstanding majors. The remaining
deferred items (id surfacing, empty-state copy, UI truncation indicator) are
explicitly recorded as accepted rough edges with follow-up pointers, not silent
gaps.

---
*Re-review generated by /accelerator:review-plan*
