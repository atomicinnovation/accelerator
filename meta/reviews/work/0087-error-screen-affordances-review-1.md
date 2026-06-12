---
type: work-item-review
id: "0087-error-screen-affordances-review-1"
title: "Work Item Review: 404 / Error Screen with Affordances"
date: "2026-06-12T16:09:02+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0087"
relates_to: ["work-item:0054"]
work_item_id: "0087"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-06-12T16:46:31+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: 404 / Error Screen with Affordances

**Verdict:** REVISE

This is a strong, well-bounded story: it explicitly disentangles the three
branches the current inline message conflates, names the performing component
for every behaviour, resolves both prior open questions inline, and populates
every optional section with genuine content. The verdict tips to REVISE only on
two related **testability** majors — the suggestion-ranking criterion has no
worked fixture to assert ordering against, and the fetch-error state's copy has
no measurable distinction from the 404 copy beyond the absent suggestions block.
Both are pin-the-example fixes rather than structural rework.

### Cross-Cutting Themes

- **Fetch-error state is under-specified** (flagged by: completeness, testability)
  — The story mandates a distinct error state with "distinct, honest copy" that
  omits the suggestions block, but never says which affordances it retains
  (back-to-library? retry?) nor gives any checkable property distinguishing its
  copy from the 404 copy. Completeness sees a content gap; testability sees an
  unverifiable criterion. The same fix resolves both.
- **Copy/voice conventions are stated but not pinned for verification** (flagged
  by: clarity, testability) — The monospace-slug span, sentence-case, terminal
  period, and the literal "Document not found" H1 are specified in Requirements
  but (a) the H1 is ambiguous for the catch-all `/garbage` case and (b) none of
  the copy conventions appear in any Acceptance Criterion, so "done" wouldn't
  require them.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Suggestion-ordering criterion lacks a defined fixture, so the ranking assertion is not deterministic
  **Location**: Acceptance Criteria
  The ranking rule (substring/prefix quality, mtime tie-break) is stated in prose
  but pinned to no input corpus or expected output, so a verifier must invent a
  fixture and adjudicate ties themselves — two testers could both claim a pass on
  different fixtures even with an incorrect prefix-boost or tie-break.

- 🟡 **Testability**: The fetch-error copy criterion has no measurable distinction from the 404 copy
  **Location**: Acceptance Criteria
  The criterion requires copy that "reflects a fetch/loading failure rather than a
  missing document," but the only observable property is the absence of the
  suggestions block — "honest" and "reflects a fetch failure" admit no pass/fail
  check, so near-identical 404 wording could be argued as passing.

#### Minor

- 🔵 **Clarity**: Literal H1 "Document not found" conflicts with the catch-all use case
  **Location**: Requirements
  Requirements keep the literal H1 "Document not found" for the 404 case, and the
  same surface mounts as the router-level `notFoundComponent` for `/garbage`-style
  URLs — which are not missing *documents*. The story never says whether the
  catch-all reuses that H1 or a route-not-found heading.

- 🔵 **Dependency**: BigGlyph / DEFAULT_BIG hero glyph reuse implies an uncaptured dependency on a net-new illustration system
  **Location**: Requirements
  Requirements direct the implementer to "reuse" the `.ac-empty-page` hero layout
  and `DEFAULT_BIG` fallback glyph, but the source design-gap lists BigGlyph as a
  net-new feature absent from the current app — there is no current-app component
  to reuse, and Dependencies names only 0041.

- 🔵 **Dependency**: Per-doc-type hue tokens on the detail surface are a net-new coupling not named in Dependencies
  **Location**: Technical Notes
  The visual conventions call for the prototype's type-tinted glyph treatment, but
  the design-gap establishes that the current app does not surface the per-doc-type
  `--ac-doc-*` hue tokens on the detail surface — a separate net-new feature
  (TypeColourCoding) the story may silently depend on.

- 🔵 **Testability**: First AC's "when any exist" overlaps the dedicated suggestion ACs without stating the no-suggestion render
  **Location**: Acceptance Criteria
  The first criterion folds the conditional suggestion case into a compound
  assertion ("…and suggestions when any exist"); the precise present/absent
  outcomes are only pinned in the two dedicated suggestion criteria, so the first
  AC alone is verifiable only for the always-present links.

- 🔵 **Testability**: Copy-voice requirements (monospace span, sentence case, terminal period) are not reflected in any Acceptance Criterion
  **Location**: Requirements
  Requirements mandate concrete, checkable copy properties (monospace slug span,
  sentence case, terminal period, no apologies), but none appear in the ACs — so
  "done" as defined by the criteria would not require the voice/visual intent that
  motivates the story.

#### Suggestions

- 🔵 **Clarity**: Multiple interchangeable names for the central artefact
  **Location**: Requirements
  The new artefact is called "not-found surface", "404 surface", "not-found
  screen", and "the new surface", and contrasted with an "error state". The
  mapping is inferable but the reader must reconstruct it; pick one canonical term
  per variant.

- 🔵 **Clarity**: Ranking convention cites two precedents without saying they agree
  **Location**: Requirements
  The ranking requirement cites both `/api/search`'s `classify()` and the
  prototype's `rankCorpus` joined by "and", without stating whether they define
  the same algorithm or which is authoritative if they differ.

- 🔵 **Clarity**: mtime tie-breaker source not tied to the named slug source
  **Location**: Assumptions
  The tie-breaker is "mtime, most-recent first" while Assumptions only commit to
  slugs via `IndexEntry.slug`; the story never states that the same `IndexEntry`
  records carry the mtime, leaving the sort-key source implicit.

- 🔵 **Completeness**: Fetch-error state copy and affordances not fully specified
  **Location**: Requirements
  The distinct error state is mandated to omit suggestions and use "distinct,
  honest copy", but the section never says which affordances it retains (e.g.
  back-to-library present, retry offered) or gives a one-line copy intent, while
  the 404 surface is fully specified by contrast.

- 🔵 **Scope**: Fetch-error/404 split is a latent correctness fix bundled with the 404 feature
  **Location**: Requirements
  The story bundles the new 404 affordance surface with a latent correctness fix
  (splitting fetch-error branches out of the shared "Document not found" copy).
  The bundle is defensible — both touch the same `LibraryDocView` branches and
  share the surface — so no action is needed unless the fix has independent urgency.

- 🔵 **Dependency**: Sibling redesign work items that also touch LibraryDocView are not noted as ordering-adjacent
  **Location**: Dependencies
  Several sibling design-gap stories edit the same `LibraryDocView.tsx`; if any
  land concurrently, a Related note flagging the shared touch-point would make
  sequencing a conscious choice rather than a merge surprise.

### Strengths

- ✅ The Context section explicitly enumerates the three current branches and
  states which one is the true 404, removing the ambiguity that "Document not
  found" currently spans three distinct conditions.
- ✅ Requirements name the performing component for each behaviour
  (`LibraryDocView.tsx` for the true-404 branch, `router.ts` `notFoundComponent`
  for the catch-all), so responsibility is never left passive.
- ✅ Both Open Questions are marked resolved with the chosen option and rationale
  inline; the Assumptions section actively corrects an earlier mistaken
  assumption (single global slug endpoint).
- ✅ Seven Given/When/Then Acceptance Criteria cover each branch, the suggestion
  cap, the empty-suggestions omission, the router catch-all, and the fetch-error
  split — strong structural testability.
- ✅ Clear in/out-of-scope boundaries: Levenshtein deferred, mtime bounded to a
  tie-breaker, suggestions capped at five; the story maps cleanly to a single
  design-gap driver paragraph and sits within one frontend bounded context.
- ✅ Scope-expansion decisions (router catch-all, fetch-error/404 split) are
  consciously recorded in Drafting Notes with rationale rather than silently
  widening the story.

### Recommended Changes

1. **Add a worked ranking fixture to the suggestion-ordering AC** (addresses:
   "Suggestion-ordering criterion lacks a defined fixture") — Pin one concrete
   example: given a specific set of slugs with stated mtimes and a specific
   missing slug, state the exact expected order. This makes the prefix-boost and
   mtime tie-break checkable rather than prose-only.

2. **Give the fetch-error state a checkable copy property** (addresses: "The
   fetch-error copy criterion has no measurable distinction"; "Fetch-error state
   copy and affordances not fully specified") — Specify what distinguishes the
   error state observably: e.g. a distinct heading string (not "Document not
   found") that references retry/loading, and state which affordances it retains
   (back-to-library present, no suggestions). One fix closes both the
   completeness gap and the testability major.

3. **Resolve the catch-all H1** (addresses: "Literal H1 'Document not found'
   conflicts with the catch-all use case") — State whether the router-level
   not-found surface keeps "Document not found" or uses a route-not-found
   heading, so the H1 is unambiguous for both mount points.

4. **Add an AC for the testable copy conventions** (addresses: "Copy-voice
   requirements … not reflected in any Acceptance Criterion") — Add a criterion
   asserting the failed slug renders inside a monospace element and the body is
   sentence-case with a terminal period, so the voice/visual intent is part of
   "done".

5. **Clarify the glyph/hue dependency** (addresses: "BigGlyph / DEFAULT_BIG …
   uncaptured dependency"; "Per-doc-type hue tokens … net-new coupling") — State
   whether the surface re-creates a minimal glyph itself (no dependency) or
   consumes the net-new BigGlyph / TypeColourCoding systems, and if the latter,
   add those work items to Dependencies as blockers.

6. **(Optional) Canonicalise terminology and cross-reference the first AC**
   (addresses: clarity suggestions, "First AC's 'when any exist' overlaps…") —
   Pick one term per artefact variant and narrow the first AC to the
   always-present affordances, deferring suggestions to the dedicated ACs.

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually precise: it explicitly disambiguates the
three current branches, names the actor/component for each behaviour, and
resolves prior open questions inline. Most pronouns and referents resolve
cleanly. The principal clarity risks are a terminological cloud around the
central artefact ("not-found surface" / "404 surface" / "error state" /
"not-found screen") and an apparent tension between reusing the literal H1
"Document not found" and applying that same surface to truly-unmatched
non-document URLs.

**Strengths**:
- The Context section explicitly enumerates the three current branches and
  states which one is the true 404.
- Requirements name the performing component for each behaviour, so
  responsibility is never left passive.
- Both Open Questions are marked "(Resolved)" with the chosen option and
  rationale inline.
- The Assumptions section actively corrects an earlier mistaken assumption
  (single global slug endpoint).

**Findings**:
- 🔵 **minor** (confidence: medium) — _Literal H1 "Document not found" conflicts
  with the catch-all use case_ (Requirements). Requirements keep the literal H1
  for the 404 case, and the same surface mounts as the router-level
  `notFoundComponent` for `/garbage`-style URLs — not missing documents. The
  story never says whether the catch-all reuses that H1 or a route-not-found
  heading, so an implementer could ship a factually wrong heading or invent one
  with no guidance.
- 🔵 **suggestion** (confidence: medium) — _Multiple interchangeable names for
  the central artefact_ (Requirements). "not-found surface", "404 surface",
  "not-found screen", "the new surface", contrasted with "error state". The
  mapping is inferable but the reader must reconstruct it; pick one canonical
  term per variant, perhaps with a one-line glossary.
- 🔵 **suggestion** (confidence: low) — _Ranking convention cites two precedents
  without saying they agree_ (Requirements). The rule cites both `classify()`
  and `rankCorpus` joined by "and" without stating whether they are equivalent
  or which is authoritative if they differ.
- 🔵 **suggestion** (confidence: low) — _mtime tie-breaker source not tied to the
  named slug source_ (Assumptions). The tie-breaker is mtime while Assumptions
  only commit to `IndexEntry.slug`; the story never states the same records carry
  the mtime, leaving the sort-key source implicit.

### Completeness

**Summary**: This story is exceptionally complete for its kind: a clear
user-framed Summary, a Context that explains the inline-message problem and the
three branches it conflates, specific actionable Requirements, seven testable
Acceptance Criteria, plus populated Dependencies, Assumptions, Open Questions,
Technical Notes, and Drafting Notes. Frontmatter is well-formed with a
recognised kind and appropriate status. No critical or major completeness gaps;
the only observation is a minor refinement.

**Strengths**:
- Summary is framed as a clear user-need statement and names the concrete
  deliverable.
- Context substantively explains the motivation and grounds the
  design-consistency rationale in the prototype's building blocks.
- Acceptance Criteria provides seven specific Given/When/Then criteria covering
  each branch, the suggestion cap, the empty-suggestions case, the router
  catch-all, and the fetch-error split.
- Requirements are detailed enough to start work without follow-up.
- Optional sections are all genuinely populated; both Open Questions are marked
  resolved with the chosen option recorded.
- Frontmatter is complete and well-formed (kind=story, status=draft, priority,
  blocked_by, source, relates_to), consistent with the body.

**Findings**:
- 🔵 **suggestion** (confidence: medium) — _Fetch-error state copy and
  affordances not fully specified_ (Requirements). The distinct error state must
  omit suggestions and use "distinct, honest copy," but the section does not say
  which affordances it carries (back-to-library present? retry?) nor what the
  copy should read, while the 404 surface is fully specified by contrast. An
  implementer could deliver the error state with materially less recovery
  affordance, needing a follow-up. State explicitly which affordances it retains
  and give a one-line copy intent, or note that the "Given any not-found state"
  back-to-library criterion is intended to cover it.

### Dependency

**Summary**: The work item captures its primary upstream blocker (0041 Page
wrapper) consistently in both frontmatter and the Dependencies section, and
correctly relegates 0054 to a non-blocking "Related" entry with clear rationale.
The main gap is that several visual building blocks the Requirements lean on (the
BigGlyph hero illustration and DEFAULT_BIG fallback glyph, the per-doc-type hue
tokens) are net-new features per the source design-gap that do not yet exist in
the current app, yet no dependency on the work items introducing them is named.

**Strengths**:
- The 0041 blocker is captured consistently in frontmatter `blocked_by`, the
  Dependencies section, and Technical Notes.
- 0054 is correctly classified as Related rather than Blocked-by, with explicit
  rationale.
- "Blocks: none" is an appropriate, honest downstream assessment for a
  low-priority leaf error surface.
- The Assumptions section explicitly captures the client-side-slug-aggregation
  coupling, surfacing what would otherwise be a hidden data-availability
  dependency.

**Findings**:
- 🔵 **minor** (confidence: medium) — _BigGlyph / DEFAULT_BIG hero glyph reuse
  implies an uncaptured dependency on a net-new illustration system_
  (Requirements). Requirements direct the implementer to "reuse" the
  `.ac-empty-page` hero layout and `DEFAULT_BIG` fallback glyph, but the
  design-gap lists BigGlyph as net-new and absent from the current app — there is
  no component to reuse, and Dependencies names only 0041. Clarify whether the
  surface re-creates a minimal glyph itself or consumes a shared BigGlyph system;
  if the latter, add that work item as a blocker.
- 🔵 **minor** (confidence: medium) — _Per-doc-type hue tokens on the detail
  surface are a net-new coupling not named in Dependencies_ (Technical Notes).
  The visual conventions call for the prototype's type-tinted glyph treatment, but
  the design-gap establishes the current app does not surface the `--ac-doc-*`
  hue tokens on the detail surface (a separate net-new feature, TypeColourCoding).
  State whether the surface needs type-tinted hues (and name that work item) or
  note that the fallback glyph requires no per-type tokens.
- 🔵 **suggestion** (confidence: low) — _Sibling redesign work items that also
  touch LibraryDocView are not noted as ordering-adjacent_ (Dependencies). Several
  sibling design-gap stories edit the same `LibraryDocView.tsx`; if any land
  concurrently, a Related note flagging the shared touch-point would make
  sequencing a conscious choice rather than a merge surprise.

### Scope

**Summary**: A coherent, well-bounded story: every requirement serves the single
purpose of turning dead/erroring URLs into a recoverable not-found surface, and
the Summary, Requirements, and Acceptance Criteria describe the same scope
without drift. It sits within one frontend bounded context (no cross-service
coordination), is appropriately sized for a "story" kind, and the author has
consciously documented its scope-expansion decisions in Drafting Notes. The only
mild signal is the bundling of a latent fetch-error-vs-404 correctness fix
alongside the new 404 affordances, but the two are tightly coupled through the
same `LibraryDocView` branches.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria are tightly aligned — no section
  claims scope another omits.
- Clear in/out-of-scope boundaries: Levenshtein deferred, mtime bounded to a
  tie-breaker, suggestions capped at five.
- Sits within a single frontend bounded context with no cross-service or
  multi-team orchestration.
- Maps cleanly to a single driver paragraph in the source design-gap, honouring
  that document's instruction to treat each paragraph as a discrete work item.
- Scope-expansion decisions are consciously recorded in Drafting Notes with
  rationale.

**Findings**:
- 🔵 **suggestion** (confidence: medium) — _Fetch-error/404 split is a latent
  correctness fix bundled with the 404 feature_ (Requirements). The story bundles
  the new affordance surface with a latent correctness fix (splitting fetch-error
  branches out of the shared "Document not found" copy). Impact is minor — the two
  concerns touch the same branches and share the surface, so coupling is high and
  splitting would add coordination overhead. No action needed if the author
  accepts the bundle (Drafting Notes already justify it); consider a fast-follow
  chore only if the fix has independent urgency.

### Testability

**Summary**: The Acceptance Criteria are unusually strong for testability: each
is framed as Given/When/Then with concrete observable outcomes (specific links to
specific routes, the up-to-five cap, the empty-block-omission rule, the catch-all
behaviour, and the 404/error split). The main verification gaps are the
suggestion-ranking criterion (ordering is hard to assert without a defined
fixture) and the absence of any measurable threshold for the "distinct, honest"
error copy.

**Strengths**:
- Every AC is framed as Given/When/Then with explicit precondition, action, and
  observable outcome.
- The conditional back-to-type affordance is testable on both branches (present
  for known type, absent for catch-all).
- The empty-suggestions criterion specifies omission (not an empty block) — a
  concrete, distinguishable DOM outcome.
- The fetch-error vs 404 split is its own criterion with a verifiable negative
  assertion (suggestions block absent).
- Scope is bounded with explicit numeric/algorithmic limits, removing
  unbounded-matching ambiguity.

**Findings**:
- 🟡 **major** (confidence: high) — _Suggestion-ordering criterion lacks a
  defined fixture, so the ranking assertion is not deterministic_ (Acceptance
  Criteria). The rule (substring/prefix quality, mtime tie-break) is specified but
  given no input corpus or expected output, so a verifier must invent a fixture
  and adjudicate ties — two testers could both claim a pass on different fixtures
  even with an incorrect implementation. Add a worked example: e.g. given slugs
  `[error-handling (mtime T2), error-screen (mtime T1), screen-errors]` and
  missing slug `error-scr`, the order is `error-screen, error-handling,
  screen-errors`.
- 🟡 **major** (confidence: medium) — _The fetch-error copy criterion has no
  measurable distinction from the 404 copy_ (Acceptance Criteria). The criterion
  requires copy that "reflects a fetch/loading failure rather than a missing
  document", but the only observable property is the absence of the suggestions
  block — "honest" and "reflects a fetch failure" have no defined check, so
  near-identical 404 wording could pass. Specify a checkable property: the error
  heading/body differs from "Document not found" and references retry/loading, or
  pin the exact error H1.
- 🔵 **minor** (confidence: medium) — _First AC's "when any exist" overlaps the
  dedicated suggestion ACs without stating the no-suggestion render_ (Acceptance
  Criteria). The first criterion folds the conditional suggestion case into a
  compound assertion; the precise present/absent outcomes are only pinned in the
  dedicated criteria, so the first AC alone is verifiable only for the
  always-present links. Narrow the first AC to the always-present affordances or
  cross-reference the dedicated ACs.
- 🔵 **minor** (confidence: medium) — _Copy-voice requirements (monospace span,
  sentence case, terminal period) are not reflected in any Acceptance Criterion_
  (Requirements). These properties are objectively testable (DOM assertion on the
  slug element, string checks), yet "done" as defined by the ACs would not require
  them, risking the voice/visual intent shipping unverified. Add an AC asserting
  the failed slug renders inside a monospace element and the body is sentence-case
  with a terminal period.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-12T16:37:31+00:00

**Verdict:** REVISE

Re-ran the four lenses that drove pass-1 edits (clarity, completeness,
dependency, testability); scope was skipped as its sole pass-1 finding was an
explicit no-action suggestion. Net progress is strong — every pass-1 finding is
resolved or downgraded — but the pass surfaced two new testability majors (one
introduced by the pass-1 AC8 edit), so the verdict remains REVISE pending those
two quick fixes.

### Previously Identified Issues

- 🟡 **Testability**: Suggestion-ordering criterion lacks a defined fixture —
  **Resolved**. The worked example (`error-screen` → `error-screen-v2`,
  `error-screens`, `legacy-error-screen`; `error-handling` omitted) makes the
  bucket-then-mtime order deterministically verifiable.
- 🟡 **Testability**: Fetch-error copy has no measurable distinction —
  **Partially resolved**. The negative check (heading ≠ `Document not found`)
  is now concrete; the positive "names a load/fetch failure" remains subjective
  (downgraded to a minor below).
- 🔵 **Clarity**: Catch-all H1 conflict — **Resolved**. `Document not found`
  (unknown slug) vs `Page not found` (catch-all) split accepted.
- 🔵 **Dependency**: BigGlyph / DEFAULT_BIG uncaptured — **Resolved**. 0082
  recorded in `blocked_by` and Dependencies as a satisfied prerequisite.
- 🔵 **Dependency**: Per-doc-type hue tokens uncaptured — **Resolved**. 0074
  recorded likewise.
- 🔵 **Testability**: Copy-voice conventions absent from ACs — **Resolved**.
  AC8 added (monospace slug, sentence case, terminal period) — though it
  introduced a new major (see below).
- 🔵 **Clarity**: Dual ranking precedents (`classify()`/`rankCorpus`) —
  **Resolved**. `classify()` is now authoritative; `rankCorpus` illustrative.
- 🔵 **Clarity**: mtime tie-breaker source — **Resolved**. Now tied to
  `classify()`'s `sort_by_cached_key((Reverse(mtime_ms), rel_path))`.
- 🔵 **Completeness**: Fetch-error affordances unspecified — **Resolved**.
  Affordances (back-to-library, conditional back-to-type) enumerated.
- 🔵 **Testability**: First AC compound overlap — **Still present**. Re-flagged
  as the AC1 umbrella that restates AC2–AC5 without an independent check.
- 🔵 **Clarity**: Multiple names for the central artefact — **Still present**.
  Re-flagged as the "the surface" referent shifting between the shared
  not-found surface, the 404 instance, and the fetch-error state.
- 🔵 **Dependency**: Sibling LibraryDocView edits — **Not re-flagged** (dropped).
- 🔵 **Scope**: Fetch-error/404 bundle — not re-run (no-action suggestion).

### New Issues Introduced

- 🟡 **Testability** (Acceptance Criteria, AC8): `"contains no apology"` is not
  mechanically verifiable — there is no deterministic check for an open-ended
  semantic category. Introduced by the pass-1 AC8 edit. Fix: drop the clause
  from the AC (keep it as a non-AC voice note) or pin to a substring check.
- 🟡 **Testability** (Requirements / AC4–AC5): the candidate-match rule defines
  no minimum missing-slug length, so a 1-char slug `contains`-matches nearly
  every candidate and the AC5 "empty → omitted" boundary cannot be constructed.
  Fix: adopt the existing search threshold (`use-search.ts` gates on
  `length >= 2`).
- 🔵 **Completeness** (Frontmatter: blocked_by): `blocked_by` lists 0082/0074
  while the body marks them satisfied — frontmatter and body disagree.
  Introduced by the pass-1 edit.
- 🔵 **Dependency** (Technical Notes / Assumptions): the `classify()`
  behavioural coupling and the TanStack-Query cache-availability assumption are
  not surfaced in the Dependencies section.
- 🔵 **Testability** (Requirements): the `rel_path` secondary tie-break is
  unexercised by any acceptance criterion.
- 🔵 **Clarity** (Summary): "Replace the bare inline `Document not found`
  message" reads as contradicting the retained true-404 H1.

### Assessment

The work item is materially stronger than at pass 1 — all original findings are
resolved or downgraded. The remaining REVISE is driven solely by two trivial,
self-contained testability majors with obvious fixes. Once the unverifiable
"no apology" clause is removed and the `≥ 2`-char threshold is adopted, the
item should clear to APPROVE; the residual minors (AC1 umbrella, terminology,
the two dependency-note adds, the frontmatter/body `blocked_by` reconciliation)
are polish that does not block implementation.

## Re-Review (Pass 3) — 2026-06-12T16:46:31+00:00

**Verdict:** REVISE

Verification pass over the two lenses that still carried open majors at pass 2
(testability) or an introduced minor (completeness). Completeness came back
clean; testability confirmed both pass-2 majors resolved but surfaced two new
majors — latent AC-coverage gaps (requirement clauses with no asserting
criterion). All four testability items (2 majors, 2 minors) were addressed in
this pass; completeness needs no change.

### Previously Identified Issues

- 🟡 **Testability**: `"contains no apology"` not verifiable — **Resolved**.
  AC8 now demarcates the mechanically-checkable parts (monospace slug, sentence
  case, terminal period) and flags the no-apology rule as a non-AC design note.
- 🟡 **Testability**: Candidate-match minimum-length threshold — **Resolved**.
  Requirements adopt the `>= 2`-char search minimum, and AC5 names the sub-2-char
  case as a way to reach the empty branch.
- 🔵 **Completeness**: `blocked_by` lists satisfied deps (frontmatter/body
  disagreement) — **Resolved**. Body reframes 0082/0074 as satisfied blockers
  retained as real dependency edges, with only 0041 open; frontmatter and body
  now agree.
- 🔵 **Dependency**: `classify()` / cache couplings not in Dependencies —
  **Resolved** (pass-2 follow-up). Captured as a "Behavioural coupling (not
  blocking)" line.
- 🔵 **Clarity**: Summary "replace" vs kept H1 — **Resolved**. Summary now says
  the inline *rendering* is replaced and the H1 retained.
- 🔵 **Testability**: AC1 umbrella overlap — **Resolved**. AC1 narrowed to the
  routing + H1 assertion; affordances deferred to AC2–AC5.
- 🔵 **Clarity**: Terminology cloud — **Resolved**. A glossary line fixes the
  "the surface" referent.

### New Issues Introduced

- 🟡 **Testability** (Acceptance Criteria): the **five-suggestion cap** was never
  exercised by a criterion with more than five matches. **Addressed** — new AC
  asserts the top five in ranked order with the sixth omitted.
- 🟡 **Testability** (Acceptance Criteria): **case-insensitive matching** had no
  asserting criterion (all ACs used lowercase slugs). **Addressed** — new AC with
  a mixed-case missing slug (`Error-Screen`) matching a lowercase candidate.
- 🔵 **Testability** (Acceptance Criteria): `rel_path` final tie-break
  unexercised. **Addressed** — folded a same-bucket/same-mtime `rel_path`-ascending
  assertion into the AC4 worked example.
- 🔵 **Testability** (Acceptance Criteria): fetch-error trigger under-specified.
  **Addressed** — AC7 now names the concrete triggers (doc-list query 5xx/network
  failure; doc-content query rejects).
- 🔵 **Completeness** (Open Questions): section holds only resolved items
  (low-confidence suggestion) — not actioned; the resolved entries are useful
  provenance and "no open questions" is inferable.

### Assessment

The work item is now ready for implementation. All majors across all three passes
are resolved, the ranking/matching requirements have full acceptance-criteria
coverage (ordering, cap, case-insensitivity, tie-breaks, empty case, and
fetch-error triggers), and the dependency and clarity concerns are closed. The
single remaining item is a low-confidence completeness suggestion about the Open
Questions section, which is provenance-only and does not block work. A further
testability pass would yield diminishing returns — recommend treating this as
effectively APPROVE-ready.

**Final verdict: APPROVE** (set by the reviewer after the pass-3 fixes landed).
Every major across all three passes is resolved; the only outstanding item is a
low-confidence, provenance-only completeness suggestion. The frontmatter
`verdict` reflects this final APPROVE; the per-pass `REVISE` verdicts above are
retained as the chronological record of what each pass found before its fixes.
