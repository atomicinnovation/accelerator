---
type: "work-item-review"
id: "0278-topic-research-visualiser-doc-type-review-2"
title: "Work Item Review: Topic-Research Visualiser Doc Type and Indexer"
date: "2026-09-10T18:40:26+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0278"
work_item_id: "0278"
relates_to: ["work-item-review:0278-topic-research-visualiser-doc-type-review-1"]
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 2
review_pass: 2
tags: []
last_updated: "2026-09-10T20:48:55+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Topic-Research Visualiser Doc Type and Indexer

**Verdict:** REVISE

0278 remains a dense, exemplary story: every section is present and substantively
populated, the manifest-status collapse reads identically across Summary,
Context, Requirements, and Acceptance Criteria, and the co-land with 0277 is
reconciled carefully in both prose and frontmatter. The REVISE verdict rests on
two major findings and the two-major threshold, not on any structural weakness —
one genuinely new clarity ambiguity (the `research → RSC` short-code note against
"research glyph untouched") and one testability gap (AC4's title-fallback
branches have no exercising fixture and "humanised" is undefined). The second
major is a re-surfacing of a minor from review 1's accepted tail, now escalated;
the remaining findings are minor clarity and coverage polish. Review 1 closed at
APPROVE by reviewer override with a documented minor tail (fallback-branch
fixtures, 0280/0281 in `blocks`); two of those tail items reappear here, so much
of this pass is that tail resurfacing rather than fresh regression.

### Cross-Cutting Themes

- **The 0277 co-land coupling is load-bearing** (flagged by: scope, dependency)
  — Scope flags that neither Slice 1 half is independently deliverable (0277
  writes artifacts invisible without this indexer; this story indexes manifests
  only 0277 produces). Dependency flags that the transitive ordering guarantee
  for the shared-registration consumers 0280/0281 holds only while the soft
  `relates_to` co-land holds. Both are deliberate and were accepted in review 1;
  the theme is that the co-land invariant is enforced by process, not by the item
  boundaries.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: Ambiguous `research → RSC` short-code note vs "research glyph untouched"
  **Location**: Requirements (Library placement and rendering)
  The glyph requirement reads "glyph short code `TRS` (research → `RSC`)" while
  the relabel requirement states the existing `research` type's "wire name,
  glyph, colour, routing … are untouched". It is ambiguous whether the
  parenthetical asserts that research's short code is changing to `RSC` (which
  would contradict "untouched") or merely notes an existing code for contrast, so
  an implementer cannot tell whether to modify it. Genuinely new — not raised in
  review 1.

- 🟡 **Testability**: AC4 title-fallback branches have no specified input and "humanised directory name" is undefined
  **Location**: Acceptance Criteria (title fallback)
  AC4 asserts a three-tier title fallback (manifest `title` → first H1 →
  humanised directory name), but the Fixtures section specifies only one manifest
  carrying `title: <X>`. Nothing exercises the title-absent or title-plus-H1-
  absent branches, and "humanised" defines no transformation, so two of the three
  behaviours admit no pass/fail. This is review 1's pass-3 minor
  (fallback-branch fixtures), escalated to major here.

#### Minor

- 🔵 **Scope**: Co-land coupling with 0277 means neither half is independently deliverable
  **Location**: Dependencies
  0278 must co-land with 0277 so the vertical demo is not lost, so neither half is
  independently shippable, deployable, or roll-back-able. Deliberate and
  epic-sanctioned (each half passes `mise run check` in isolation); flagged so the
  co-land is understood as an accepted unit-of-delivery coupling to enforce by
  process (e.g. a single merge train).

- 🔵 **Clarity**: "the loop" undefined and its relation to "the engine (0277)" unstated
  **Location**: Summary
  The Summary refers to "the research sets the loop produces" without defining
  "the loop"; Context then attributes the same production to "the engine (0277)".
  A reader who has not read epic 0121 cannot be certain the two terms name one
  producer.

- 🔵 **Clarity**: "the user" refers to both the product persona and the directing stakeholder
  **Location**: Summary / Drafting Notes
  "The user" names both the Summary's product persona ("As an Accelerator user…")
  and the author/stakeholder who scoped the work ("at the user's direction", "per
  user direction"), forcing the reader to infer which is meant per section.

- 🔵 **Testability**: AC5 "none default grey" has no measurable boundary
  **Location**: Acceptance Criteria (status chip)
  AC5 requires each of the five lifecycle chip colours to be "distinct" and
  "non-grey", but "grey" has no defined boundary — unlike the precise ≥3:1
  text-contrast threshold the same criterion pins. A low-saturation tone could be
  argued either way.

#### Suggestions

- 🔵 **Scope**: Story reaches beyond the visualiser into engine/corpus/skill code for the status collapse
  **Location**: Requirements: The 0277-delta
  The collapse modifies `schema.rs`, the manifest template, the `research-topic`
  skill, and the corpus-cli fixture — none of them visualiser code. Causally
  justified (the card reads base `status`, so without the collapse every card
  reads "complete") and both halves co-land anyway; keep it here while it stays
  scoped to what the card requires. Deliberate; accepted in review 1.

- 🔵 **Dependency**: 0280/0281 shared-registration consumers absent from machine-readable `blocks`
  **Location**: Dependencies (Shared downstream bullet)
  The academic-sources (0280) and consumption (0281) slices consume this story's
  shared frontend doc-type registration but appear only in prose, not `blocks`.
  Their ordering is "transitively satisfied by 0277 blocking them" — but that
  holds only while the soft `relates_to` co-land holds. Add them to `blocks`, or
  state the transitive guarantee is contingent on the co-land. Review 1's pass-3
  tail item.

- 🔵 **Clarity**: "that same view" has two candidate antecedents
  **Location**: Context
  "the set-level detail page (0284) supersedes the shared flat `LibraryDocView`
  later, and this slice renders through that same view" offers two antecedents;
  it can be misread as rendering through the future 0284 page. Replace with the
  explicit `LibraryDocView`.

- 🔵 **Clarity**: "kind" overloaded between work-item kind and doc-type discriminator
  **Location**: Context
  "kind" names both the work item's own `kind: story` and the topic-research
  document discriminator ("kind: manifest", "the `report` kind"), and the six
  doc kinds are referenced but never enumerated. Qualify the corpus discriminator
  (e.g. "doc kind") where meant.

- 🔵 **Testability**: AC5 chip rendering exercised end-to-end only for "synthesised"
  **Location**: Acceptance Criteria (status chip)
  Only `synthesised` is rendered by a fixture; the other four states rely on the
  colour map (statically checkable but not rendered). State explicitly that the
  five-way status→chip mapping is unit-tested across all five states for
  distinctness, non-grey, and contrast.

### Strengths

- ✅ Every expected section is present and substantively populated — no empty or
  placeholder sections; frontmatter complete and correct (`kind: story`,
  `status: ready`, parent/blocks/relates_to linkage all present).
- ✅ The Summary is a well-formed As-a/I-want/so-that story naming the beneficiary
  and benefit, with a scope note delegating sub-document navigation to 0284.
- ✅ Eleven Given/When/Then acceptance criteria with precise, pre-computed pass
  conditions — ≥3:1 contrast, ≥15° hue separation with expected 104°/53°/37°
  deltas, exact `rgb(28,146,51)`, exact label strings, "exactly one entry".
- ✅ Every deliberate deviation from the prototype is explicitly reconciled rather
  than left as a silent contradiction (five distinct chip tones vs neutral/indigo;
  engine vocab `researching`/`complete` vs the prototype's `gathering`/
  `monitoring`; the corrected "darwin and linux" VR phrasing).
- ✅ The scope boundary is stated with unusual precision: the out-of-scope bullet
  enumerates exactly what 0278 does not pull in, each attributed to a separate
  design-convergence work item.
- ✅ Dependency mapping is thorough — upstream blocker, co-land simultaneity with
  explicit block-cycle avoidance, downstream blocks (0279, 0284), and the human
  design-reviewer and Docker/Linux VR completion gates — all with rationale.
- ✅ The subjective visual-fidelity gate (AC9) is honestly pinned to a recorded
  sign-off artefact (a PR approval or a work-item note naming the reviewer) and
  explicitly separated from the VR baselines' drift-guard role.

### Recommended Changes

1. **Resolve the `research → RSC` ambiguity** (addresses: Ambiguous
   `research → RSC` short-code note). State explicitly whether research's short
   code changes to `RSC` in this story or is pre-existing, and reconcile it with
   "research glyph untouched" — if it changes, the "untouched" list must exclude
   the short code; if not, drop or reword the parenthetical.

2. **Give the title-fallback branches inputs and define "humanised"** (addresses:
   AC4 title-fallback). Add fixture manifests (or documented indexer test cases)
   for the title-absent and title-plus-H1-absent branches, and define the
   humanisation transform (e.g. split on `-`, title-case) so the third branch has
   an expected output to assert.

3. **Quantify and exercise the status-chip mapping** (addresses: AC5 "non-grey"
   unbounded; AC5 single-state coverage). Pin the five exact chip fill/text values
   or define "non-grey" via a saturation threshold, and state the five-way
   status→chip mapping is unit-tested across all five states.

4. **Decide 0280/0281 machine-readability** (addresses: 0280/0281 absent from
   `blocks`). Add them to `blocks`, or state in the Shared-downstream note that
   the transitive-through-0277 guarantee is contingent on the co-land holding.

5. **Clarity polish** (addresses: "the loop", "the user", "that same view",
   "kind"). Define "the loop" or relate it to "the engine (0277)"; reserve "the
   user" for the product persona and name the requester otherwise; replace "that
   same view" with `LibraryDocView`; qualify the corpus "kind" as "doc kind".

Note: the two scope observations (co-land non-independence, 0277-delta placement)
are deliberate, epic-sanctioned, and were accepted in review 1 — no change is
recommended beyond confirming the co-land is process-enforced.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: A dense, heavily cross-referenced story that is, for its size,
unusually clear: numeric constraints (hue separations 104°/53°/37°, the ≥3:1
contrast bound, the doc-type count of 14) reconcile across Requirements and
Acceptance Criteria, deliberate deviations from the prototype are each explicitly
reconciled rather than left as silent contradictions, and coined terms like "the
0277-delta" are defined on introduction. The residual clarity risks are a handful
of undefined or shifting referents — "the loop" vs "the engine (0277)", "the
user" as both product persona and directing stakeholder — and one parenthetical
("research → RSC") that reads as a possible internal contradiction with the
"research glyph untouched" statement.

**Strengths**:
- Numeric constraints are internally consistent and restated consistently: the
  hue separations (132 vs 28/185/95 = 104°/53°/37°) match in both Requirements and
  Acceptance Criteria, and the doc-type count of 14 is used uniformly.
- Every deliberate deviation is explicitly called out and reconciled rather than
  left as a latent contradiction (five-distinct chip tones vs neutral/indigo; the
  engine vocab vs the prototype's `gathering`/`monitoring`; the corrected "darwin
  and linux" VR phrasing).
- Coined and domain terms are defined or contextualised on introduction ("the
  0277-delta"; "nested-manifest" anchored to `DesignInventories`; "VR" expanded to
  "visual-regression").
- Acceptance Criteria use Given/When/Then form naming the acting subject, keeping
  actor and observable outcome explicit.

**Findings**:
- 🟡 major / medium — **Ambiguous `research → RSC` short-code note vs "research
  glyph untouched"** (Requirements, Library placement and rendering). The glyph
  requirement reads "glyph short code `TRS` (research → `RSC`)" while the relabel
  requirement states research's "wire name, glyph, colour, routing … are
  untouched". Ambiguous whether research's short code is being changed to `RSC` or
  merely noted for contrast, so an implementer cannot tell whether to modify it.
  Suggestion: state explicitly whether research's short code changes in this story
  and reconcile with "untouched".
- 🔵 minor / medium — **"the loop" undefined and its relation to "the engine
  (0277)" unstated** (Summary). "the research sets the loop produces" does not
  define "the loop"; Context attributes the same production to "the engine
  (0277)". Suggestion: define "the loop" on first use or relate it to the engine.
- 🔵 minor / high — **"the user" refers to both the product persona and the
  directing stakeholder** (Summary / Drafting Notes). Names both the "As an
  Accelerator user…" persona and the stakeholder who scoped the work ("at the
  user's direction"). Suggestion: reserve "the user" for the persona and name the
  requester explicitly.
- 🔵 suggestion / low — **"that same view" has two candidate antecedents**
  (Context). "the set-level detail page (0284) supersedes the shared flat
  `LibraryDocView` later, and this slice renders through that same view" — can be
  misread as the future 0284 page. Suggestion: name `LibraryDocView` explicitly.
- 🔵 suggestion / low — **"kind" overloaded between work-item kind and doc-type
  discriminator** (Context). Names both `kind: story` and the topic-research
  document discriminator; the six doc kinds are referenced but never enumerated.
  Suggestion: qualify the corpus discriminator as "doc kind" where meant.

### Completeness

**Summary**: An exemplary, structurally complete story. Every expected section is
present and substantively populated: a story-formatted Summary with a clear
beneficiary and rationale, a Context that adds investigation-derived motivation
beyond restating the summary, a richly detailed Requirements section, eleven
specific Given/When/Then acceptance criteria, and fully populated Dependencies,
Assumptions, Open Questions, Technical Notes, and References. Frontmatter is
complete with a recognised `kind: story`, `status: ready`, and all required
fields. No completeness gaps were identified.

**Strengths**:
- Summary is a well-formed story: names the beneficiary, states what is built,
  gives the "so that" rationale, plus a scope note delegating sub-document
  navigation to 0284.
- Eleven specific Given/When/Then criteria covering every requirement strand —
  far exceeding the two-criterion minimum.
- Context explains the forces behind the work and adds two investigation facts
  that reshaped the draft, rather than restating the Summary.
- Requirements are implementer-ready: concrete files, methods, and values (the
  `TopicResearch` variant methods, `[Self; 14]` count bumps, the collapse across
  schema/template/skill/fixture).
- Story-kind expectations fully met; optional sections (Open Questions,
  Assumptions, scope boundaries) populated rather than placeholder.
- Frontmatter integrity sound: `kind`, `status`, `priority`, `parent`, `blocks`,
  `relates_to`, `tags`, `external_id` all present and appropriate.

**Findings**: _None._

### Dependency

**Summary**: The dependency mapping in 0278 is exceptionally thorough for a
story: the upstream blocker (0277), the co-land simultaneity constraint, the
downstream blocks (0279, 0284), the cross-team/human completion gates (design
sign-off, the pinned Docker/Linux VR harness), and the on-branch 0277-delta
integration coupling are all explicitly captured with rationale. The one genuine
gap is that two named downstream consumers of this story's shared frontend
doc-type registration (0280/0281) are argued out of the machine-readable `blocks`
field via a transitivity argument that itself rests on a soft (`relates_to`)
co-land constraint. No uncaptured external system or hard start-blocker was found.

**Strengths**:
- The upstream blocker is precisely captured: 0277 blocks 0278 with the concrete
  coupling reason (the indexer keys on the `manifest.md` the engine writes, layered
  over corpus/skill code 0277 landed on-branch).
- The co-land simultaneity is captured symmetrically and reasoned through on both
  items — `relates_to` with an explicit note that a reciprocal `blocked_by` is
  omitted to avoid a `blocks`/`blocked_by` cycle.
- Cross-team/external completion gates are named with availability implications:
  the human design reviewer and the pinned Docker/Linux VR harness, with the note
  that if either is unavailable the corresponding criteria cannot be met.
- Downstream blocks 0279 and 0284 are captured in both frontmatter and prose with
  specific consumption reasons.
- The out-of-scope boundary is enumerated so adjacent design-convergence work is
  not silently assumed to be coupled here.

**Findings**:
- 🔵 suggestion / medium — **0280/0281 shared-registration consumers absent from
  machine-readable `blocks`** (Dependencies, Shared downstream bullet). The story
  names 0280 (reputation-tier rendering) and 0281 (`report` kind) as consumers of
  its shared frontend doc-type registration, but neither is in `blocks` (only 0279
  and 0284 are); their ordering is argued as "transitively satisfied by 0277
  blocking them", which holds only because the co-land is recorded as a soft
  `relates_to`. If the co-land slips, 0277 landing would unblock 0280/0281 while
  the frontend registration they depend on is absent, invisibly. Suggestion: add
  0280/0281 to `blocks`, or state the transitive guarantee is contingent on the
  co-land holding.

### Scope

**Summary**: 0278 is a coherent, unusually well-bounded story delivering the
visualiser half of epic 0121's Slice 1 — registering the umbrella
`topic-research` doc type, indexing each set as one library entry via
`manifest.md`, and rendering cards with correct status. The Summary,
Requirements, and Acceptance Criteria describe the same scope, and the story is
explicit about what it excludes (set-level navigation deferred to 0284; the
design-convergence concerns enumerated as out of scope). The two scope signals
are the mandatory co-land coupling with 0277 (neither half is independently
deliverable) and the absorption of the engine/corpus-side `research_status` →
base-`status` collapse, which reaches beyond the visualiser into
schema/template/skill code — both deliberate, epic-sanctioned, and causally
justified.

**Strengths**:
- The scope boundary is stated with unusual precision: the "Out of scope" bullet
  enumerates exactly what 0278 does not pull in, each attributed to a separate
  design-convergence work item.
- Set-level sub-document navigation and `primary`-pointer resolution are
  explicitly deferred to 0284, keeping the story bounded to card + shared-view
  manifest rendering.
- The `research` → "Codebase research" relabel is scoped to the display label
  only (wire name, glyph, colour, routing, count untouched).
- Summary, Requirements, and Acceptance Criteria consistently describe one
  deliverable with no drift between sections.

**Findings**:
- 🔵 minor / medium — **Co-land coupling with 0277 means neither half is
  independently deliverable** (Dependencies). 0278 must co-land with 0277, so
  neither half is independently shippable, deployable, or roll-back-able — the
  planning overhead of two items with the atomicity of one. Deliberate; each half
  passes `mise run check` in isolation. Suggestion: confirm the engine/visualiser
  seam earns its keep and ensure the co-land is process-enforced (e.g. a single
  merge train).
- 🔵 suggestion / medium — **Story reaches beyond the visualiser into
  engine/corpus/skill code for the status collapse** (Requirements: The
  0277-delta). The collapse modifies `schema.rs`, the manifest template, the
  `research-topic` skill, and the corpus-cli fixture — none visualiser code. The
  Drafting Notes confirm the expansion "at the user's direction". Causally
  justified and both halves co-land anyway. Suggestion: keep it here while it
  stays scoped to what the card requires; if it grows, reconsider allocating the
  engine-side status-model change to 0277.

### Testability

**Summary**: This story's acceptance criteria are unusually strong on
testability: most are framed as Given/When/Then observable behaviours with
precise, pre-computed pass conditions (≥3:1 contrast, ≥15° hue separation with
expected 104°/53°/37° deltas, exact RGB, exact label strings, "exactly one
entry"), and the deliberately subjective design gate is honestly pinned to a
recorded sign-off artefact rather than left open-ended. The residual weaknesses
are input coverage: AC4's title-fallback branches and AC5's per-state chip
rendering are asserted but lack specified fixture inputs, and "non-grey" has no
measurable boundary.

**Strengths**:
- Quantitative criteria are precise and pre-computed: AC10 pins ≥3:1 contrast,
  ≥15° hue separation with exact 104°/53°/37° deltas and `rgb(28,146,51)`; AC5
  fixes a ≥3:1 chip-text contrast threshold.
- Most criteria are observable behaviours with concrete pass conditions: AC3
  ("exactly one library entry keyed on `manifest.md`"), AC2 (exact label strings),
  AC7 (validator passes for base `status` `briefed` and `synthesised`), AC8
  (registry compiles, `Record` maps complete, `mise run check` exits 0).
- The subjective visual-fidelity gate (AC9) is honestly scoped: the pass condition
  is an enumerable artefact (a PR approval or work-item note naming the reviewer),
  and VR baselines are explicitly separated as a drift guard.
- No unbounded language: enumerated sets (the five lifecycle states, the closed
  chip vocabulary) are closed, and scope-limiting phrases give verifiers concrete
  negatives to check.
- The criteria collectively cover the Summary's four intents — cards appear (AC1),
  open to their manifest (AC6), index as one entry per set (AC3), report real
  lifecycle progress (AC5, AC7).

**Findings**:
- 🟡 major / medium — **AC4 title-fallback branches have no specified input and
  "humanised directory name" is undefined** (Acceptance Criteria, title fallback).
  AC4 asserts a three-tier fallback (manifest `title` → first H1 → humanised
  directory name), but the Fixtures section specifies only one manifest carrying
  `title: <X>`. Nothing exercises the title-absent or title-plus-H1-absent
  branches, and "humanised" defines no transformation, so two of three behaviours
  admit no pass/fail. Suggestion: add fixtures for both fallback branches and
  define the humanisation transform.
- 🔵 minor / medium — **AC5 "none default grey" has no measurable boundary**
  (Acceptance Criteria, status chip). AC5 requires each of five chip colours to be
  "distinct" and "non-grey", but "grey" has no boundary — unlike the precise ≥3:1
  contrast the same criterion pins. Suggestion: pin the five exact chip values, or
  define "non-grey" via a saturation threshold.
- 🔵 suggestion / low — **AC5 chip rendering exercised end-to-end only for
  "synthesised"** (Acceptance Criteria, status chip). Only `synthesised` is
  rendered by a fixture; the other four rely on the colour map (statically
  checkable but not rendered). Suggestion: state the five-way status→chip mapping
  is unit-tested across all five states.

## Re-Review (Pass 2) — 2026-09-10

**Verdict:** COMMENT

Re-ran all five lenses against the enlarged work item — the pass-1 fixes (both
majors plus the clarity/testability polish) and the newly added
`research` → `codebase-research` wire-key rename requirement group. Both pass-1
majors are resolved. The verdict moves REVISE → COMMENT: one major remains — a
scope observation that the wire-key rename is independently deliverable — but it
is a deliberate, author-directed bundling and sits below the two-major REVISE
threshold. The remaining tail is precision refinement, some of it introduced by
the rename; the two clarity inaccuracies in the rename text were corrected during
this pass.

### Previously Identified Issues

- 🟡 **Testability** (AC4 title-fallback had no exercising fixture) — Resolved.
  Re-review cites the per-branch fallback fixtures and the defined humanisation
  transform as a strength.
- 🟡 **Clarity** (`research → RSC` short-code contradiction) — Resolved. The
  parenthetical was removed; re-review confirms the disambiguation and the
  three-names gloss.
- 🔵 **Testability** (AC5 non-grey unquantified; single-state) — Resolved. HSL
  saturation ≥15% and a direct five-state unit test are now required.
- 🔵 **Clarity** ("the loop", "the user", "that same view", "kind") — Resolved.
  Producer named as the engine; "the user" reserved for the persona;
  `LibraryDocView` pinned; the six doc kinds enumerated.
- 🔵 **Dependency** (0280/0281 absent from `blocks`) — Resolved. Both now on
  `blocks` and cited as a strength.
- 🔵 **Scope** (co-land non-independence; 0277-delta placement) — Still present.
  Deliberate; re-flagged as minor/suggestion, no change agreed.

### New Issues Introduced

- 🟡 **Scope** (Requirements: rename group) — the `research` → `codebase-research`
  wire-key rename is independently deliverable (it breaks the public
  `/library/research` URL, needs a `localStorage` rewrite, moves the pipeline
  `completeness.present` vocabulary across three files, and renames 10 VR
  baselines) yet is welded to the feature addition. Deliberate, author-directed
  bundling; the lens recommends splitting it into its own item. Accepted as-is
  per the reviewer's direction — recorded so the coupling is understood, not an
  oversight.
- 🔵 **Clarity** (Summary / Requirements: rename group) — the Summary grouped the
  `research_codebase` config key among surfaces "already using the
  codebase-research name", contradicting the item's own "config key … untouched".
  Fixed this pass.
- 🔵 **Clarity** (Requirements / AC hue lists) — the hue-28 neighbour was named
  `research` inside the very item that renames it. Fixed this pass to
  `codebase-research (28)`.
- 🔵 **Dependency** (0277-delta) — sibling 0277's still-published acceptance
  criteria assert `research_status`, which this collapse supersedes; the co-land
  must reconcile 0277's verification spec in lockstep. Only the epic-0121 contract
  update is recorded as a follow-up.
- 🔵 **Testability** (rename ACs) — the `localStorage` last-seen rewrite lacks an
  explicit pre/post state; the pinned `TYPE_COPY` text has no verifying criterion;
  and the new-type VR baseline count in AC1 is not pinned the way the rename's 10
  baselines are in AC4.
- 🔵 **Completeness** (Context) — the Context motivates the registration and the
  status collapse but not the wire-key rename.
- 🔵 **Suggestions** — name the status-model change as an explicit third pillar in
  the Summary; add a shared non-graph co-land marker across 0277/0278; gloss "wire
  key" on first use; drop the subjective "leaving the two unambiguous" from AC2.

### Assessment

The work item is acceptable as-is (COMMENT). Both pass-1 majors are closed and the
earlier clarity/testability/dependency tail is resolved. The one remaining major
is a deliberate scope call — bundling a breaking wire-key rename with the feature
addition — recorded so the coupling is understood; if delivery independence later
matters, splitting the rename into its own item is the lens's recommendation. The
residual minors are the diminishing-returns tail (a reconciliation note for 0277's
`research_status` ACs; testability precision on the localStorage/`TYPE_COPY`/VR-count
criteria; a Context sentence for the rename). Two clarity inaccuracies introduced
by the rename were fixed during this pass. None blocks planning.

## Post-Review Edits — 2026-09-10

**Verdict:** COMMENT (unchanged)

At the author's direction, the residual pass-2 minor/suggestion tail was applied
to the work item after the re-review; these edits were not re-verified by a
further agent pass. The scope major (bundling the wire-key rename with the
feature) was deliberately kept — the coupling stands as an accepted, documented
decision, not removed — so the verdict remains COMMENT.

Applied:

- **Completeness / Scope** — Context now motivates the wire-key rename as an
  explicit third pillar.
- **Dependency** — added a co-land reconciliation note (0277's `research_status`
  acceptance criteria must move to base `status` in lockstep) and an out-of-band
  co-land enforcement marker (shared merge train / mutual PR link, since the block
  graph cannot express the simultaneity without a cycle).
- **Testability** — the `localStorage` last-seen rewrite AC gained an explicit
  pre/post pair; a criterion now asserts the topic-research `TYPE_COPY` strings
  match the pinned values; AC1 now pins the topic-research VR coverage matrix (8
  glyph-showcase + 2 big-glyph), and the subjective "leaving the two unambiguous"
  clause was dropped from the relabel criterion.
- **Clarity** — glossed "wire key" on first use.

Not changed (deliberate): the wire-key rename stays bundled here per the author's
decision; the co-land non-independence and the two-half vertical-slice split
remain documented scope observations.
