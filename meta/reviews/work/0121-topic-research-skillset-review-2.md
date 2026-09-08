---
type: "work-item-review"
id: "0121-topic-research-skillset-review-2"
title: "Work Item Review: Topic Research Skillset"
date: "2026-09-07T21:39:26+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0121"
relates_to: ["work-item-review:0121-topic-research-skillset-review-1"]
work_item_id: "0121"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "scope"]
review_number: 2
review_pass: 2
tags: []
last_updated: "2026-09-08T00:13:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Topic Research Skillset

**Verdict:** COMMENT

Epic 0121 has absorbed review 1's findings well: the artifact contract is now
specified inline at field-name level, acceptance criteria are slice-attributed,
Slice 5's in/out status is settled with a named descope candidate, `breadth` is
stated as a hard ceiling, and the synthesis writer is named as the `conduct`
orchestrator. This second pass (clarity, completeness, scope) surfaces one major
and eight lower-severity findings, all documentation refinements rather than
design gaps — the epic is acceptable but could be improved. The one recurring
substantive issue is Slice 1's size, which review 1 also raised and the work
item deliberately kept whole; scope now frames it as a planning-time call rather
than a defect.

### Cross-Cutting Themes

- **Slice 1's breadth persists across both reviews** (flagged by: scope in
  reviews 1 and 2) — Slice 1 still bundles the research engine and a full
  VR-baselined visualiser doc-type registration as one child. The work item now
  carries an explicit rationale for keeping it whole (the loop is only
  observable end to end). Scope accepts the engine-plus-library-entry argument
  but notes it does not obviously require the full doc-type registration to land
  in the same story, and recommends revisiting the seam at Slice 1 planning.

- **Localised terminology ambiguities cluster in the naming layer** (flagged by:
  clarity) — three minor findings all concern names rather than structure: the
  inverted `research-topic`/`topic-research` pair, a frontmatter field named
  `topic` that reintroduces the disambiguated term, and `source/focus profile`
  obscuring whether one or two profile axes exist. The Terminology section
  removed the big ambiguity; these are the residue.

### Findings

#### Critical

None.

#### Major

- 🟡 **Scope**: Slice 1 walking skeleton bundles the engine and full visualiser
  registration as one child
  **Location**: Requirements: Initial stories (vertical slices) — Slice 1
  Slice 1 is slated to become a single child work item, yet it spans the config
  path key, the template set, the `manifest.md` aggregate root, the generic
  `researcher` agent, the web profile, the skill with two subcommands and
  orchestrator prompt, and a complete visualiser doc-type registration (Rust
  enum, TS union, glyph, colour tokens, framed background, VR baselines on
  darwin+linux, indexer wiring). The whole-loop rationale covers the engine plus
  a library entry but does not obviously require the full VR-baselined doc-type
  registration in the same story.

#### Minor

- 🔵 **Clarity**: Near-identical inverted names `research-topic` and
  `topic-research`
  **Location**: Requirements: Artifact contract / Summary
  The skill is `research-topic`, the directory `meta/research/topics/`, and the
  doc-type/`type` value `topic-research`. Terminology explains why the skill and
  directory keep colloquial forms but never why the doc type inverts, leaving a
  reader unsure whether the inversion is deliberate or a slip.

- 🔵 **Clarity**: Frontmatter field named `topic` reintroduces the disambiguated
  term
  **Location**: Requirements: Artifact contract (topic row)
  The topic document lists `research_kind: topic`, `round`, `topic`,
  `source_profile`. A field literally named `topic` inside a `research_kind:
  topic` document reintroduces the overloading the Terminology section removed,
  and the contract never states what the field holds.

- 🔵 **Clarity**: `source/focus profile` obscures whether there are one or two
  profile axes
  **Location**: Summary / Requirements (goal 1) / Technical Notes
  The reusable abstraction is described three times as an injectable
  "source/focus profile", but the artifact contract only ever names
  `source_profile`/`source_profiles`, and "focus" never appears independently or
  with a definition. This sits on the epic's load-bearing reusability thesis.

#### Suggestions

- 🔵 **Scope**: Slice 4 fuses the tunable-config surface with the recursion
  engine
  **Location**: Requirements: Initial stories — Slice 4
  Slice 4 bundles the `--breadth`/`--depth` config surface with the `depth > 1`
  recursion engine, and its own text states a "ship the knob, drop the
  recursion" fallback — a signal the two concerns can be delivered
  independently. Pre-splitting at planning turns the acknowledged fallback into
  a clean boundary rather than a mid-slice cut.

- 🔵 **Scope**: Reusable-infrastructure generality has only one in-scope
  consumer
  **Location**: Requirements: High-level goals and themes — goal 1
  Goal 1 elevates reusable research infrastructure to a first-class deliverable,
  but the only in-scope consumer is `research-topic`; reuse is proven only by
  out-of-scope future skills. Keep the contract scoped to what `research-topic`
  needs and treat generalisation beyond that as out of scope until the first
  future consumer lands.

- 🔵 **Completeness**: Referenced `children` frontmatter field does not yet exist
  **Location**: Frontmatter: children
  Requirements states the author records each child's id in frontmatter under
  `children`, but no `children` key exists yet. Reasonable at draft with no
  children created; adding `children: []` now makes the referenced tracking
  mechanism exist.

- 🔵 **Clarity**: Bare `docs.rs` collides with the well-known Rust site name
  **Location**: Requirements: Initial stories (Slice 1)
  Slice 1 references "(+ `docs.rs` config_path_key)" where `docs.rs` means the
  file `server/src/docs.rs` (spelled out later in Technical Notes). Standing
  alone, the token reads first as the Rust documentation host.

- 🔵 **Clarity**: `primary` flip ownership stated in both Slice 1 and Slice 2
  **Location**: Requirements: Initial stories (Slice 2)
  Slice 1's criterion says `conduct` sets `primary` "(flipped to
  `synthesis.md`)"; Slice 2 also describes `conduct`/`expand` keeping `primary`
  in sync with the same flip. Reconcilable, but the Slice 2 parenthetical
  re-describes the Slice 1 event, leaving momentary doubt about which slice owns
  the flip.

### Strengths

- ✅ The Terminology section defines "subject", "topic", and "round" up front and
  explicitly acknowledges the two colloquial senses of "topic", pre-empting the
  most likely reader confusion.
- ✅ "Reputation tier" is defined precisely and pinned as reputation-only (venue
  standing, not claim correctness), closing review 1's four-names-for-one-concept
  finding.
- ✅ The Artifact contract now specifies frontmatter fields and required sections
  per document shape at binding field-name level — the pillar deliverable review
  1 found unspecified.
- ✅ Acceptance criteria are extensive (24) and each is tagged to the slice that
  delivers it, giving every slice its own definition of done.
- ✅ Vertical slicing is INVEST-aligned with explicit ordering constraints and a
  designated descope candidate (Slice 5) whose deferral leaves the rest
  shippable.
- ✅ In-scope/out-of-scope boundaries are stated with unusual precision
  (external-only, web+academic only, Crossref/Semantic Scholar/dedicated
  citation pass deferred through the injectable-profile seam).
- ✅ Acronyms and external concepts are defined on first use (VR, INVEST, Claude
  Design, the polite pool), and Drafting Notes flag where an earlier resolution
  was superseded so stale content is not read as authoritative.

### Recommended Changes

1. **Decide Slice 1's visualiser seam at planning, or note it explicitly**
   (addresses: "Slice 1 walking skeleton bundles the engine and full visualiser
   registration")
   Either record in the Slice 1 text that the doc-type registration + indexer
   wiring is a candidate thin follow-on story once the engine writes conforming
   artifacts, or state that Slice 1 will be planned as a small sub-epic. The
   whole-loop rationale can stand while still naming the seam.

2. **Resolve the three naming ambiguities** (addresses: the inverted
   `research-topic`/`topic-research` names; the `topic` frontmatter field; the
   `source/focus profile` axis question)
   Add one sentence stating `topic-research` is the intentional doc-type value
   paired with the `research-topic` skill. State what the `topic` field holds and
   consider a name that resolves the sense. Pick one term for the profile
   abstraction, or state explicitly that a profile captures both a source and a
   focus dimension and reflect that in the contract's field naming.

3. **Consider pre-splitting Slice 4** (addresses: "Slice 4 fuses the
   tunable-config surface with the recursion engine")
   Split Slice 4 into the tunable-config story and the recursion-engine story at
   planning, so the acknowledged "drop the recursion engine" fallback becomes a
   clean boundary. Keep them ordered adjacently — the knob makes the recursion
   affordable to try.

4. **Tidy the residual referents** (addresses: "`children` field does not yet
   exist"; "bare `docs.rs`"; "`primary` flip stated twice")
   Add `children: []` to the frontmatter now. Write `server/src/docs.rs` in Slice
   1 as it appears in Technical Notes. In Slice 2, refer to keeping the
   already-flipped `primary` in sync rather than re-describing the flip.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: This is an unusually clear and internally consistent work item: a
dedicated Terminology section pre-empts the obvious subject/topic/round
ambiguity, "reputation tier" is explicitly scoped, acronyms are defined on first
use, and actors (orchestrator, skill, indexer, user) are consistently named with
concrete observable outcomes. The clarity concerns that remain are localised
naming ambiguities rather than structural contradictions: two near-identical
inverted names (`research-topic` vs `topic-research`), a frontmatter field that
reintroduces the overloaded term the Terminology section worked to disambiguate,
and a slash-construction ("source/focus profile") that obscures whether one or
two profile dimensions exist.

**Strengths**:

- A dedicated Terminology section defines "subject", "topic", and "round" up
  front and explicitly acknowledges the two colloquial senses of "topic".
- "Reputation tier" is defined precisely and its scope pinned as reputation-only
  (venue standing, not claim correctness).
- Acronyms are handled well: VR expanded on first use, INVEST spelled out inline,
  and non-obvious external concepts like "Claude Design" and the "polite pool"
  defined in place.
- Requirements and Acceptance Criteria name the acting component for each
  behaviour and state outcomes as observable frontmatter/file states.
- Drafting Notes explicitly flag where an earlier resolution was superseded and
  point to the current statement.

**Findings**:

- **minor / medium** — *Near-identical inverted names `research-topic` and
  `topic-research`* — Location: Requirements: Artifact contract / Summary. The
  work item uses three closely related but non-identical names: the skill
  `research-topic`, the directory `meta/research/topics/`, and the
  document/frontmatter `type: "topic-research"`. Terminology and Drafting Notes
  explain why the skill and directory keep their colloquial forms but never
  explain why the visualiser doc type inverts, leaving a reader unsure whether
  the inversion is deliberate. **Impact**: A reader or implementer may misread
  one for the other or introduce a mismatched identifier. **Suggestion**: Add one
  sentence stating that `topic-research` is the intentional doc-type/`type` value
  paired with the `research-topic` skill, or align the names.

- **minor / medium** — *Frontmatter field named `topic` reintroduces the
  disambiguated term* — Location: Requirements: Artifact contract (topic row).
  The topic document's binding frontmatter is `research_kind: topic`, `round`,
  `topic`, `source_profile`. A field literally named `topic` inside a
  `research_kind: topic` document reintroduces the exact overloading the
  Terminology section removed, and the contract never states what the `topic`
  field holds (the sub-question text? a slug? the parent topic it drills?).
  **Impact**: Because field names are declared binding, an ambiguous field name
  propagates into templates and code. **Suggestion**: State what the `topic`
  field contains, and consider a name that resolves the sense.

- **minor / medium** — *"source/focus profile" obscures whether there are one or
  two profile axes* — Location: Summary / Requirements (goal 1) / Technical
  Notes. The central reusable abstraction is described three times as an
  injectable "source/focus profile", but the artifact contract only ever names
  `source_profile`/`source_profiles`, and "focus" never appears independently or
  with a definition. **Impact**: This ambiguity sits on the epic's load-bearing
  "reusable infrastructure" thesis, so downstream planning could build a phantom
  second axis or miss an intended one. **Suggestion**: Pick one term
  consistently, or explicitly state that a profile captures both a source and a
  focus dimension and reflect that in the contract's field naming.

- **suggestion / low** — *Bare `docs.rs` collides with the well-known Rust site
  name* — Location: Requirements: Initial stories (Slice 1). Slice 1 references
  "(+ `docs.rs` config_path_key)", where `docs.rs` means the file
  `server/src/docs.rs` (as Technical Notes later spells out). Standing alone, the
  token reads first as the Rust documentation host. **Impact**: A momentary
  mis-parse for any Rust-literate reader. **Suggestion**: Write
  `server/src/docs.rs` in Slice 1 as it appears in Technical Notes.

- **suggestion / low** — *`primary` flip ownership stated in both Slice 1 and
  Slice 2* — Location: Requirements: Initial stories (Slice 2). The Slice 1
  criterion says `conduct` sets `primary` "(flipped to `synthesis.md`)", while
  Slice 2 also describes `conduct`/`expand` keeping `primary` in sync with the
  same flip. The two are reconcilable, but the Slice 2 parenthetical re-describes
  the Slice 1 event. **Impact**: A reader tracing slice ownership may be briefly
  unsure where the flip is delivered versus maintained. **Suggestion**: In Slice
  2, refer to keeping the already-flipped `primary` in sync rather than
  re-describing the flip.

### Completeness

**Summary**: This epic is exceptionally complete from a structural and
informational standpoint: every expected section (Summary, Context,
Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions,
Technical Notes, References) is present and substantively populated, and the
epic-required decomposition strategy is delivered as five well-defined vertical
slices with explicit ordering constraints. Frontmatter is valid — `kind: epic`,
`status: draft`, and `priority: high` are all present and recognised. The only
notable gap is a frontmatter mechanism the body references (a `children` field)
that is not yet reflected in the frontmatter itself, which is acceptable at draft
stage but worth noting.

**Strengths**:

- Every standard section is present and densely populated — Context explains the
  motivating gap (research stops at the repo boundary; nothing accretes), and the
  Summary states an unambiguous deliverable.
- The decomposition strategy is fully realised as five independently shippable
  vertical slices, each with scope, rationale, and explicit ordering constraints.
- Acceptance Criteria are extensive (24) and each is tagged to the slice that
  delivers it, giving each slice its own definition of done.
- A binding Artifact contract plus a Terminology section that disambiguates
  "subject" vs "topic" vs "round" pre-empt a large class of implementer
  follow-up questions.
- Open Questions, Dependencies, and Assumptions are all genuinely populated, and
  Drafting Notes record the resolution history of prior review passes.

**Findings**:

- **suggestion / low** — *Referenced `children` frontmatter field does not yet
  exist* — Location: Frontmatter: children. The Requirements section states that
  "the author records each child's id in this epic's frontmatter under
  `children` as it is created," but the frontmatter currently contains no
  `children` key. This is reasonable while the epic is in `draft` with no child
  work items yet created. **Impact**: A reader tracking decomposition progress
  has no single frontmatter field to consult, though the five in-body slices
  already convey the full plan. **Suggestion**: Add an empty `children: []` key
  now so the referenced tracking mechanism exists and is populated as children
  are created.

### Scope

**Summary**: This epic describes one coherent capability — a durable,
citation-backed external topic-research skillset — and decomposes it into five
vertical slices that each serve that theme, with unusually clear
in-scope/out-of-scope boundaries (external-only, web+academic only, future
consumer skills and extra source families explicitly deferred). The epic kind is
appropriate for the multi-slice, cross-layer scope, and the decomposition is
coherent rather than a grab-bag. The principal scope tension is that Slice 1, as
a single child work item, bundles the entire research engine, the generic agent,
the skill, config plumbing, templates, and a full visualiser doc-type
registration — a breadth that may itself warrant being a sub-epic — and a couple
of slices bundle separable concerns whose own text acknowledges a "ship one,
drop the other" fallback.

**Strengths**:

- The epic is a genuinely coherent capability: all five slices serve the single
  theme, and the future consumer skills are explicitly held out of scope.
- The in-scope/out-of-scope surface is stated with unusual precision, so a reader
  can state exactly what is and is not in the unit of work.
- Vertical slicing is explicitly INVEST-aligned with stated ordering constraints
  and a designated descope candidate (Slice 5) whose deferral leaves the rest
  shippable.
- The bundling of reusable infrastructure with its first consumer is handled
  deliberately (surfaced through Slice 1 rather than as a standalone horizontal
  story with no standalone value).

**Findings**:

- **major / medium** — *Slice 1 "walking skeleton" bundles two separately
  shippable increments across four toolchains* — Location: Requirements: Initial
  stories (vertical slices) — Slice 1. Slice 1 is slated to become a single child
  work item, yet it bundles the config path key + default, the template set, the
  `manifest.md` aggregate root, the generic `researcher` agent, the web profile,
  the skill with two subcommands and orchestrator prompt, AND a complete
  visualiser doc-type registration (Rust enum, TS union, glyph, colour tokens,
  framed background, VR baselines on darwin+linux, nested-manifest indexer
  wiring). The whole-loop defence covers the engine plus a library entry but does
  not obviously require the full VR-baselined doc-type registration in the same
  story. **Impact**: A single child story spanning skill, agent, template,
  config, Rust server and React frontend with VR baselines risks being
  multi-week and hard to plan/verify as one increment, undermining the INVEST
  "small" property. **Suggestion**: At Slice 1 planning, revisit whether the
  visualiser doc-type registration and indexer wiring form a natural seam that
  can ship as a thin follow-on story once the engine writes conforming artifacts,
  or whether Slice 1 should itself be planned as a small sub-epic.

- **suggestion / medium** — *Slice 4 bundles the tunable-config surface with the
  recursion engine* — Location: Requirements: Initial stories (vertical slices) —
  Slice 4. Slice 4 bundles the config-tunable `--breadth`/`--depth` surface
  (schema, config-read plumbing, override flags, docs) and the `depth > 1`
  recursion engine, and its own text states a fallback of shipping the knob and
  dropping the recursion engine "if the epic runs long". That explicit drop-one
  option is a signal the two concerns can be delivered independently. **Impact**:
  Keeping them fused risks either over-running the slice or exercising the
  descope under time pressure rather than as a planned boundary. **Suggestion**:
  Consider pre-splitting Slice 4 into the tunable-config story and the
  recursion-engine story at planning, so the acknowledged fallback becomes a
  clean scope boundary. The rationale for shipping them together can still order
  them adjacently.

- **suggestion / low** — *Reusable-infrastructure generality has only one
  in-scope consumer* — Location: Requirements: High-level goals and themes —
  goal 1. Goal 1 elevates "reusable research infrastructure" to a first-class
  deliverable, but the only in-scope consumer is `research-topic`; reuse is
  proven only by future consumer skills that are explicitly out of scope.
  Designing generality ahead of a second concrete consumer can inflate an epic's
  scope. **Impact**: Effort spent hardening a "stable contract" and spawn-time
  profile seam for hypothetical future skills could expand the epic without a
  second consumer to validate the abstraction, and mis-guesses become costly
  given the immutability rule on topic files. **Suggestion**: Keep the
  infrastructure scoped to exactly what `research-topic` needs, and treat
  contract generalisation beyond that as out of scope until the first future
  consumer lands.

## Re-Review (Pass 2) — 2026-09-07

**Verdict:** COMMENT

The pass-1 edits resolved every clarity finding and the mechanical fixes cleanly.
Two new findings surfaced on the edited text — one genuine, fixable clarity gap
(`conduct` vs `expand` boundary) and one mis-directed cross-reference — plus the
scope lens re-raised Slice 1/4 as majors, now arguing the carve seams should be
committed in the epic rather than deferred. That last point is a standing design
disagreement: the author deliberately chose to make the split an
epic-breakdown-time (`/extract-work-items`) decision, so it is recorded as an
accepted position rather than an open defect. Mechanically this pass carries two
major findings, but with the scope major settled by decision the substantive
open item is the single `conduct`/`expand` clarity gap, leaving the verdict at
COMMENT.

### Previously Identified Issues

- 🟡 **Scope**: Slice 1 bundles engine + full visualiser registration — Still
  present, by deliberate design. Slice 1 now carries an epic-breakdown-time carve
  note; the scope lens would prefer the split committed in the epic, but the
  author chose to settle it at `/extract-work-items`.
- 🔵 **Clarity**: Inverted `research-topic`/`topic-research` names — Resolved. A
  Terminology sentence now states the inversion is deliberate.
- 🔵 **Clarity**: Frontmatter field `topic` reintroduced the disambiguated term —
  Resolved. Renamed to `question`, with a contract note stating it holds the
  sub-question text.
- 🔵 **Clarity**: `source/focus profile` axis ambiguity — Resolved. Summary,
  goal 1, and Technical Notes now state the two orthogonal axes (source profile +
  focus) explicitly.
- 🔵 **Scope**: Slice 4 fuses tunable-config with recursion engine — Partially
  resolved. An epic-breakdown-time carve note was added; the scope lens still
  flags the fusion as a minor and prefers the split committed now.
- 🔵 **Scope**: Reusable-infrastructure generality has one consumer — Resolved
  (not re-flagged this pass).
- 🔵 **Completeness**: `children` frontmatter field absent — Resolved as
  not-a-defect. An empty `children: []` was briefly added, but
  `corpus frontmatter validate` rejects it under the omit-when-empty rule for
  typed-linkage keys, so the key was removed. The field's original absence was
  correct: `children` is added only when populated, as child ids are recorded at
  extraction time. The body already documents that mechanism.
- 🔵 **Clarity**: Bare `docs.rs` — Resolved. Now `server/src/docs.rs` in Slice 1.
- 🔵 **Clarity**: `primary` flip stated twice — Resolved. Slice 2 now maintains
  the already-flipped field rather than re-describing the flip.

### New Issues Introduced

- 🟡 **Clarity**: The functional boundary between `conduct` and `expand` is never
  stated. Both are described as appending topics to `brief.md`, so a reader
  cannot tell whether `expand` researches the topics it adds (spawns
  researchers) or only widens the brief for a later `conduct`. One sentence in
  Terminology or Slice 2 fixes it. (Pre-existing gap, newly surfaced — not caused
  by the pass-1 edits.)
- 🔵 **Clarity**: A Drafting Notes cross-reference reads "superseded in review
  pass 3 — see the pass-3 bullet above", but the pass-3 bullet sits below that
  line (the pass bullets run 3-before-2), sending the reader the wrong way.
  Change "above" to "below" or reorder the pass bullets chronologically.

### Assessment

The epic is in strong shape and remains at COMMENT — acceptable for
implementation planning. The one substantive open item is the `conduct` vs
`expand` clarity gap, which is a single-sentence fix requiring only the author's
statement of what `expand` does that `conduct` does not. The scope lens's
preference to commit the Slice 1/4 splits in the epic is noted but overridden by
the author's deliberate choice to resolve them at epic-breakdown time.

### Post-Review Finding (surfaced in collaborative iteration)

- 🟡 **Completeness**: The Artifact contract specified every `research_status`
  transition but never stated what the base `status` field — which the contract
  also mandates on all four shapes — holds. **Location**: Requirements: Artifact
  contract. The two fields sit side by side on `manifest.md`, where they read as
  redundant, and an implementer would guess the base value per shape. This is the
  same class of defect as review 1's "artifact contract named but not specified"
  major, on the one contract field that survived that pass unspecified. Neither
  lens pass of this review caught it; it was surfaced by an author question about
  why both fields exist.

  **Resolution**: Resolved. The contract now carries a base-`status` note giving
  the per-shape values (topic files `complete` from first write; `synthesis.md`
  `complete` after each rewrite; `brief.md` `draft` then `complete`;
  `manifest.md` `complete` once it exists) and states that set progress lives
  solely in `research_status`, never inferred from the manifest's base `status`.
  The `research_status` note was updated to state it is manifest-only and to
  cross-reference the base field.

- 🔵 **Clarity**: The `conduct` acceptance criterion took "a brief path", while
  `conduct`/`expand`/`finalise` all operate on the whole set (reading the brief,
  writing topic files and `synthesis.md`, mutating `manifest.md`). **Location**:
  Acceptance Criteria (Slice 1 `conduct`; Slice 2 `expand`, `finalise`).
  Addressing one sub-document as the handle is inconsistent with the
  manifest-as-aggregate-root decision, reads wrong for `finalise` (which never
  touches the brief), and is unstable because the `primary` sub-document flips
  from `brief.md` to `synthesis.md` after the first round. Surfaced by an author
  question about whether the argument should be the set directory instead.

  **Resolution**: Resolved. The three mutating subcommands now take a single set
  handle, and a new "Set-handle resolution" Technical Note defines the three
  accepted forms resolving to the set root (`meta/research/topics/<slug>/`): the
  set directory, a bare slug (resolved against `paths.research_topics`), or any
  document within the set — mirroring the `work resolve` precedent. The Slice 4
  override-flag references were updated from `<path>` to the set-handle
  positional accordingly. `brief` remains the exception (takes a subject, creates
  the set).

- 🟡 **Scope / Clarity**: The `conduct`/`expand` overlap the pass-2 lens review
  and follow-up questions kept circling was, at root, a workflow-shape problem —
  `conduct` conflated researching a round with proposing the next, and `expand`
  duplicated the proposal path. **Location**: Requirements (Initial stories, ACs,
  Terminology, Artifact contract). Surfaced by author questions, then
  investigated with a fresh web survey of the referenced deep-research systems
  (Sagan, dzhng/deep-research, Anthropic), which found that none surfaces a
  user-editable plan, a separate steering command, or read-only consumption of
  the corpus.

  **Resolution**: Resolved by a full workflow redesign (2026-09-08). `expand`
  was dropped; the build loop is now `brief` → `outline` → `conduct` →
  `synthesise` → `finalise` (plan/execute split, matching Accelerator's
  create-plan/implement-plan idiom); two read-only consume verbs `ask` and
  `report` were added; the per-item unit was renamed `topic` → `finding` (ending
  the two-senses-of-topic overload); `research_status` gained `outlined` and
  `synthesised`; focus areas + per-round progress moved to a new `outline.md`
  (round-grouped checkboxes); findings live under `findings/`, reports under
  `reports/`; and the slice set grew to six. The survey and the divergence
  rationale are recorded in the work item's Context, Drafting Notes, and
  References. This redesign supersedes several earlier review findings and
  resolutions that assumed the four-subcommand `topic`-named model; the work
  item's Drafting Notes carry the supersession trail.
