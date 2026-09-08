---
type: "work-item-review"
id: "0121-topic-research-skillset-review-3"
title: "Work Item Review: Topic Research Skillset"
date: "2026-09-08T00:33:18+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0121"
relates_to: ["work-item-review:0121-topic-research-skillset-review-1", "work-item-review:0121-topic-research-skillset-review-2"]
work_item_id: "0121"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "scope"]
review_number: 3
review_pass: 2
tags: []
last_updated: "2026-09-08T00:49:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Topic Research Skillset

**Verdict:** REVISE

This is the first review of the epic as rewritten by the 2026-09-08 workflow
redesign (seven subcommands, `finding`-named vocabulary, six slices); reviews 1
and 2 evaluated the superseded four-subcommand `topic`-named model. The epic
remains exceptionally strong on structure and boundaries — completeness returns
no findings, scope confirms one coherent capability with clean out-of-scope
seams, and the Terminology discipline is intact. The verdict is REVISE only
because two internal-consistency gaps survive the redesign at major severity, and
the configured threshold is two majors; both are localised clarity fixes rather
than design defects, and one is a single-word correction.

### Cross-Cutting Themes

- **Redesign-introduced internal inconsistencies** (flagged by: clarity) — both
  majors are seams the 2026-09-08 rewrite opened and did not fully close: the
  Summary's "three groups" miscount against a two-group set, and the unreconciled
  question of whether `synthesise`/`report` spawn a `researcher` or run inline.
  Neither is a design disagreement; both are the redesign's own text disagreeing
  with itself.
- **Slice 1 sizing recurs across all three reviews** (flagged by: scope in
  reviews 1, 2, and 3) — scope has now softened it from a review-2 major to a
  suggestion, and pairs it with the same observation on Slice 5. The epic already
  names both carve seams; the standing author position is to settle them at
  `/extract-work-items` breakdown time, not in the epic.

### Findings

#### Critical

None.

#### Major

- 🟡 **Clarity**: Summary says "three groups" but names only two
  **Location**: Summary
  The Summary states the seven subcommands are "split into three groups" but then
  names only **build** and **consume**. Requirement 2 independently describes the
  same set as "in two groups".

- 🟡 **Clarity**: Unclear whether `synthesise`/`report` run inline or via a
  spawned researcher
  **Location**: Requirements (goal 1) / Technical Notes
  Requirement 1, the Summary, and the "Generic researcher + profiles" note
  describe the generic `researcher` as spawned with a focus of "the findings to
  aggregate when synthesising" / "the question when writing a report", yet Slice 1
  says `synthesise` writes `synthesis.md` "inline", and the "Synthesis &
  citations" note states "there is no synthesis agent". The actors "synthesis
  writer" and "report writer" are named without tying either to a mechanism.

#### Minor

- 🔵 **Clarity**: "the orchestrator" not tied to a named subcommand
  **Location**: Artifact contract (contract notes: `source_profiles`)
  The `source_profiles` note calls the profile list "the set the orchestrator may
  draw on", but "the orchestrator" is never identified with a subcommand, while
  the acceptance criteria say `conduct` assigns one profile per researcher.

- 🔵 **Clarity**: Slice 4 called "no visualiser work" but registers a doc-type
  value and rendering
  **Location**: Requirements: Initial stories (vertical slices)
  The slice preamble says "Slices 2 and 4 … need no visualiser work", yet Slice 4
  "Registers the `report` `research_kind` value … and the `reports/` layout and
  rendering".

#### Suggestions

- 🔵 **Scope**: Slice 1 walking skeleton fuses the engine and full visualiser
  registration
  **Location**: Requirements: Initial stories (vertical slices) — Slice 1
  Slice 1 bundles the entire research engine with a full-stack visualiser
  doc-type registration (Rust enum, TS union, glyph, colour tokens, ~8 VR
  baselines). The epic already names the carve seam; scope recommends treating it
  as a committed split at breakdown, with both children landing together.

- 🔵 **Scope**: Slice 5 fuses the tunable-config surface with the recursion engine
  **Location**: Requirements: Initial stories (vertical slices) — Slice 5
  Slice 5 bundles the self-contained `breadth`/`depth` config knob with the
  `depth > 1` recursion engine — the epic's acknowledged descope candidate.
  Carving along the flagged seam keeps the descope clean while the knob still
  ships adjacent to the recursion it makes affordable.

- 🔵 **Clarity**: "set handle" used before it is defined
  **Location**: Acceptance Criteria
  "set handle" (and bare "handle") is used throughout Requirements and Acceptance
  Criteria, but its three accepted forms are only defined much later in the
  "Set-handle resolution" Technical Note.

### Strengths

- ✅ The Terminology section disambiguates Subject, Round, Focus area, and
  Finding, and records the deliberate `topic` → `finding` rename that ends the
  two-senses-of-topic overload.
- ✅ "Reputation tier" is defined once as the single term for source-quality
  ranking and scoped as reputation-only (venue standing, not claim correctness).
- ✅ Acronyms and coined terms are defined at or near first use (VR, INVEST,
  Claude Design, the polite pool).
- ✅ The seven-subcommand set and the five-state `research_status` lifecycle are
  named consistently across Requirements, the slice list, and Acceptance
  Criteria, with an explicit transition table.
- ✅ Every standard section is present and densely populated; Context explains the
  motivating gap (research stops at the repo boundary) and grounds the design in
  a survey of prior systems rather than restating the Summary.
- ✅ Acceptance Criteria are abundant (~28), each tagged to its slice, and
  collectively cover every subcommand, the lifecycle, reputation tiers,
  depth/breadth tunables, and visualiser integration.
- ✅ The epic is one coherent capability decomposed into six independently
  shippable vertical slices with explicit ordering constraints and a designated
  descope candidate (Slice 6) whose deferral leaves the rest shippable.
- ✅ In/out-of-scope boundaries are stated with unusual precision (external-only,
  web+academic only), with later source families and the citation pass reachable
  through a named injectable-profile seam.
- ✅ Reusable infrastructure is delivered through its first consumer rather than
  as an un-demoable horizontal story, and the relationship to `0056` is correctly
  characterised (done → constrains, not blocks).

### Recommended Changes

1. **Reconcile the `synthesise`/`report` execution model** (addresses: "Unclear
   whether `synthesise`/`report` run inline or via a spawned researcher")
   State once, explicitly, whether `synthesise` and `report` spawn a generic
   `researcher` or run inline in the skill. Then align the goal-1 researcher-focus
   wording, the "synthesis writer"/"report writer" terms, and the "there is no
   synthesis agent" note with that single decision.

2. **Fix the group miscount** (addresses: "Summary says 'three groups' but names
   only two")
   Change "three groups" to "two groups" in the Summary, matching Requirement 2.

3. **Tighten two under-identified referents** (addresses: "the orchestrator";
   Slice 4 "no visualiser work")
   Replace "the orchestrator" in the `source_profiles` note with the concrete
   actor (`conduct`), or define the role once. Reword the Slice 4 exemption to
   "no *new* visualiser doc-type or library-entry work" so it does not contradict
   Slice 4's own `report` registration and `reports/` rendering.

4. **Gloss "set handle" at first use** (addresses: "'set handle' used before it is
   defined")
   Add a one-line gloss at first use (or in Terminology) pointing to the
   "Set-handle resolution" rule.

5. **Decide the Slice 1 and Slice 5 carve seams** (addresses: the two scope
   suggestions)
   Optional and consistent with the standing author position: either commit both
   splits in the epic now, or leave the recorded carve notes to be actioned at
   `/extract-work-items` breakdown. No epic text change is strictly required.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually disciplined about terminology: a
dedicated Terminology section disambiguates Subject/Round/Focus area/Finding,
acronyms are expanded inline (VR, INVEST, Claude Design), and "reputation tier"
is pinned as a single reputation-only term. The main clarity risks are two
internal inconsistencies — a "three groups" vs "two groups" miscount in the
Summary, and an unresolved contradiction over whether synthesise/report run
inline or via a spawned generic researcher agent — plus a couple of minor
under-identified actors and forward-referenced terms.

**Strengths**:

- A dedicated Terminology section explicitly disambiguates Subject, Round, Focus
  area, and Finding, and records the deliberate topic→finding rename to kill the
  two-senses-of-topic overload.
- "Reputation tier" is defined once as the single term for source-quality ranking
  and explicitly scoped as reputation-only (venue standing, not claim
  correctness).
- Acronyms and coined terms are defined at or near first use (VR =
  visual-regression, INVEST expanded, Claude Design and the "polite pool" glossed
  inline).
- The seven-subcommand set and the five-state research_status lifecycle are named
  consistently across Requirements, the slice list, and Acceptance Criteria, with
  an explicit transition table.

**Findings**:

- **major / high** — *Summary says "three groups" but names only two* —
  Location: Summary. The Summary states the seven subcommands are "split into
  three groups" but then names and defines only two — build
  (`brief`/`outline`/`conduct`/`synthesise`/`finalise`) and consume
  (`ask`/`report`). Requirement 2 independently describes the same subcommands as
  being "in two groups." **Impact**: A reader is left hunting for a missing third
  group that does not exist, undermining confidence in the headline description.
  **Suggestion**: Change "three groups" to "two groups" in the Summary, or, if a
  third grouping was intended, name it and list its members.

- **major / medium** — *Unclear whether synthesise/report run inline or via a
  spawned researcher* — Location: Requirements: Reusable research infrastructure /
  Technical Notes. Who performs synthesis and report-writing is contradictory.
  Requirement 1 and the Summary describe the generic `researcher` agent as
  spawned with a focus of "the set of findings to aggregate when synthesising" /
  "the question when writing a report," and Technical Notes ("Generic researcher +
  profiles") repeats this — implying spawned researchers. Yet Slice 1 says
  "`synthesise` writing `synthesis.md` inline from the findings," and Technical
  Notes ("Synthesis & citations") states "there is no synthesis agent... skill +
  `outline`/`synthesise`/`report` prompts + generic `researcher`" — implying an
  inline skill prompt with no spawn. The document also introduces the actors
  "synthesis writer" and "report writer" without tying either to a mechanism.
  **Impact**: Implementers cannot tell whether synthesise/report delegate to a
  `researcher` sub-agent or execute inline in the skill — a core architectural
  distinction. **Suggestion**: State once, explicitly, whether synthesise and
  report spawn a generic `researcher` (and if so reconcile "there is no synthesis
  agent") or run inline, and make the researcher-focus wording and the "synthesis
  writer"/"report writer" terms agree with that decision.

- **minor / medium** — *"the orchestrator" not tied to a named subcommand* —
  Location: Artifact contract (contract notes: source_profiles). The contract note
  on `source_profiles` says the profile list is "the set the orchestrator may draw
  on," but "the orchestrator" is never explicitly identified with a subcommand,
  while the Acceptance Criteria separately say "`conduct` assigns one profile per
  spawned researcher." **Impact**: A reader must infer that "the orchestrator"
  means the `conduct` subcommand, leaving open whether a distinct orchestration
  component is intended. **Suggestion**: Replace "the orchestrator" with the
  concrete actor (e.g. "`conduct`"), or define "orchestrator" once as the role
  `conduct` plays.

- **minor / low** — *Slice 4 called "no visualiser work" but registers a doc-type
  value and rendering* — Location: Initial stories (vertical slices). The slice
  preamble states "Slices 2 and 4... need no visualiser work," yet Slice 4
  "Registers the `report` `research_kind` value... and the `reports/` layout and
  rendering." **Impact**: The blanket "no visualiser work" appears to contradict
  Slice 4's own registration/rendering scope, leaving a reader unsure whether
  Slice 4 touches the visualiser at all. **Suggestion**: Reconcile the wording —
  e.g. clarify that Slice 4 needs no *new* visualiser doc-type or library-entry
  work because reports reuse the umbrella type and the shared `LibraryDocView`.

- **suggestion / low** — *"set handle" used before it is defined* — Location:
  Acceptance Criteria. The term "set handle" (and bare "handle") is used
  throughout the Requirements and Acceptance Criteria, but its meaning and the
  three accepted forms (the set directory, a bare slug, or any document within the
  set) are only defined much later, in Technical Notes ("Set-handle resolution").
  **Impact**: A reader working top-to-bottom cannot resolve what a "handle"
  accepts at the point the acceptance criteria rely on it. **Suggestion**: Add a
  one-line gloss of "set handle" at first use (or in the Terminology section)
  referencing the resolution rule.

### Completeness

**Summary**: This epic is exceptionally complete against the completeness lens.
Every expected section — Summary, Context, Requirements, Acceptance Criteria,
Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes,
References — is present and densely, substantively populated, and the frontmatter
is intact with a recognised `kind: epic` and appropriate `status: draft`.
Kind-appropriate content is strong: the epic carries a full decomposition into
six independently-shippable vertical slices with ordering constraints, plus a
per-slice acceptance-criteria set covering all seven subcommands.

**Strengths**:

- The Summary states an unambiguous, single statement of intent (a
  `research-topic` skillset delivering an iterative multi-agent deep-research loop
  with seven subcommands in build/consume groups) and is not a vague placeholder.
- Kind-appropriate decomposition is fully present: the "Initial stories (vertical
  slices)" section enumerates six slices, each described as independently
  shippable/demoable/testable, with explicit ordering constraints and a
  designated descope candidate.
- Acceptance Criteria are abundant (roughly 28 criteria), each tagged to its
  slice, and collectively cover every one of the seven subcommands, the five-state
  lifecycle, reputation tiers, depth/breadth tunables, and visualiser integration.
- Context thoroughly explains the motivation (Accelerator can research a codebase
  but not the world outside it) and grounds the design in a survey of prior
  systems, going well beyond restating the Summary.
- Optional sections that could plausibly be empty — Dependencies, Assumptions,
  Open Questions — are all populated with genuinely relevant content, and the
  single remaining Open Question is explicitly scoped and deferred to Slice 6
  planning.
- Frontmatter is complete and correct: `type`, `id`, `title`, `status`, `kind`
  (recognised value `epic`), `priority`, and lineage fields are all present; the
  omission of a `children` field is explicitly explained in-body as the
  omit-when-empty convention pending child creation.

**Findings**: None.

### Scope

**Summary**: This epic describes one coherent, well-bounded capability — a
durable, citation-backed research skillset for external subjects — decomposed
into six cohesive vertical slices that each cut end-to-end and are individually
shippable, demoable, and testable. Epic is the correct kind for the scope, the
boundary is crisp, and the out-of-scope list is explicit with named seams for
later additivity. The only scope observations are two large slices (1 and 5) that
fuse separable deliverables, and in both cases the epic has already self-identified
the carve seam, so these are breakdown-time reminders rather than epic-level scope
defects.

**Strengths**:

- Single coherent capability: all six slices serve one theme (durable,
  consultable external-subject research) rather than being a grab-bag under one
  label.
- Epic kind is appropriate for the scope: a genuine multi-story effort decomposed
  into six vertical slices, neither over-decomposed into micro-tasks nor
  under-decomposed into a single oversized story.
- Reusable infrastructure is delivered through its first consumer (Slice 1)
  instead of as a standalone horizontal story — a deliberate choice recorded in
  Drafting Notes that avoids an un-demoable infrastructure-only work item.
- Crisp scope boundary: external sources only, web and academic only, with source
  families beyond those, the Crossref/Semantic Scholar profiles, the dedicated
  citation pass, the ideation skillset, and downstream consumer skills all
  explicitly out of scope and reachable through a named injectable-profile seam.
- Good scope hygiene on delivery risk: Slice 6 is identified as the sole descope
  candidate whose deferral leaves the rest fully shippable, and carve seams are
  pre-identified for the two largest slices; ordering constraints are stated.
- Relationship to related work item 0056 is clear and correctly characterised —
  0056 is done and therefore constrains (established the subcategory pattern)
  rather than blocks, and no overlapping in-flight work items exist.

**Findings**:

- **suggestion / medium** — *Slice 1 "walking skeleton" bundles the engine and
  full visualiser registration* — Location: Initial stories (vertical slices):
  Slice 1. Slice 1 bundles the entire research engine (the `research-topic` skill,
  the generic `researcher` agent, five web-only templates, the artifact contract,
  and nested-manifest indexing) with the full-stack visualiser doc-type
  registration (Rust enum, TS union, glyph, colour tokens, framed background,
  Discover-phase placement, and ~8 VR baselines across sizes and themes). The epic
  itself already flags this and names a carve seam. **Impact**: If Slice 1 becomes
  a single child work item unchanged, it risks a multi-week, hard-to-estimate
  story that undercuts the vertical-slice discipline the rest of the epic
  maintains. **Suggestion**: At epic breakdown, treat the identified seam as a
  split rather than an option — carve Slice 1 into an engine child and a thin
  visualiser-registration child that land together so the end-to-end demo is
  preserved.

- **suggestion / medium** — *Slice 5 bundles the tunable-config surface with the
  recursion engine* — Location: Initial stories (vertical slices): Slice 5. Slice 5
  bundles the config plus per-invocation override surface for `breadth`/`depth` (a
  self-contained tunable that establishes the first numeric-config precedent) and
  the `depth > 1` recursion engine (the epic's explicitly acknowledged descope
  candidate). The epic already names this as a candidate seam to carve into two
  children. **Impact**: The recursion engine is the token-multiplying, higher-risk
  component; fusing it to the low-risk config knob in one child makes the stated
  descope decision harder to exercise cleanly if the epic runs long.
  **Suggestion**: At breakdown, carve Slice 5 along the flagged seam — the tunable
  knob as one child and the recursion engine as an adjacent, droppable child.

## Re-Review (Pass 2) — 2026-09-08

**Verdict:** COMMENT

Both majors that drove the initial REVISE verdict are resolved, and the verdict
steps down to COMMENT. Clarity confirms the `synthesise`/`report` execution model
now reads cleanly ("the researcher agent is the only spawned actor") and that
"set handle" is defined. The re-review surfaced four residual clarity items and
two regressions from the pass-1 edits; the regressions were fixed on surfacing,
and the four residuals are all minor or suggestion and non-blocking. Scope
re-flagged the same two carve seams, still-present by deliberate design.

### Previously Identified Issues

- 🟡 **Clarity**: "three groups" named but only two defined — Resolved. Summary now
  reads "two groups", matching Requirement 2.
- 🟡 **Clarity**: `synthesise`/`report` inline vs spawned — Resolved. Reconciled to
  inline: the Summary, goal 1, the "Generic researcher + profiles" note, and the
  Artifact contract now state that only `conduct` spawns researchers and that
  `synthesise`/`report` run inline over the immutable findings.
- 🔵 **Clarity**: "the orchestrator" not tied to a subcommand — Resolved. The
  `source_profiles` note now names `conduct`.
- 🔵 **Clarity**: Slice 4 "no visualiser work" contradiction — Resolved. Reworded
  to "no new doc-type or library-entry work", naming the `report` value
  registration into the umbrella type.
- 🔵 **Clarity**: "set handle" used before defined — Resolved. Terminology now
  carries a Set handle entry pointing to the resolution note.
- 🔵 **Scope**: Slice 1 fuses engine + full visualiser registration — Still
  present, by deliberate design (now rated minor). The carve seam is settled at
  `/extract-work-items` breakdown.
- 🔵 **Scope**: Slice 5 fuses config knob + recursion engine — Still present, by
  deliberate design (suggestion). Carve seam settled at breakdown.

### New Issues Introduced

- 🔵 **Clarity**: "one per focus area" in goal 1 contradicted Slice 5's `depth > 1`
  recursion — Resolved on surfacing. Dropped the count from goal 1 and the
  "Generic researcher + profiles" note; "only `conduct` spawns researchers" holds
  at all depths. (Introduced by the pass-1 inline reconciliation.)
- 🔵 **Clarity**: "the set `conduct` may draw on" overloaded "set" in the
  `source_profiles` note — Resolved on surfacing. Reworded to "the profiles
  `conduct` may draw on".
- 🔵 **Clarity**: Core term "set" is used pervasively but never defined in
  Terminology — Resolved in follow-up iteration. A **Set** entry was added to
  Terminology (the on-disk collection rooted at `meta/research/topics/<slug>/`).
- 🔵 **Clarity**: Slice 5 depth-recursion leaves the recursion actor and the
  "halving" quantity unnamed — Open (minor); planning-time detail for Slice 5.
- 🔵 **Clarity**: Slice 3's "`brief.md` declares a `source_profiles` list" can
  misread as first introducing the field the contract binds in Slice 1 — Open
  (minor).
- 🔵 **Clarity**: "the model's own vocabulary" (Terminology) reads ambiguously
  between the domain model and the language model — Resolved in follow-up
  iteration. Reworded to "the skill's internal vocabulary".

### Assessment

The epic is acceptable for implementation planning. Both blocking majors are
resolved and the two edit-introduced regressions were corrected on surfacing. Of
the four residual items, two were fixed in follow-up iteration — the missing "set"
Terminology entry and the "the model" referent. The two that remain are minor and
non-blocking: the Slice 5 recursion actor/quantity detail (planning-time) and the
Slice 3 `source_profiles` wording.

## Approval — 2026-09-08

**Verdict:** APPROVE

Approved for implementation planning by Toby Clemson after the pass-2 COMMENT and
the follow-up clarity fixes. The two blocking majors were reconciled in pass 1,
the two edit-introduced regressions were corrected on surfacing, and two of the
four residual minors were closed in follow-up iteration. The two remaining items
are non-blocking: the Slice 3 `source_profiles` wording (a one-line reword) and
the Slice 5 recursion actor/quantity detail (deferred to Slice 5 planning).
