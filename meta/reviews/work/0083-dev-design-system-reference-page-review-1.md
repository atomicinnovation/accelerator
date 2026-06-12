---
type: work-item-review
id: "0083-dev-design-system-reference-page-review-1"
title: "Work Item Review: DevDesignSystem Reference Page"
date: "2026-06-12T16:57:17+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0083"
work_item_id: "0083"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-06-12T22:01:23+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: DevDesignSystem Reference Page

**Verdict:** REVISE

This is a strong, unusually well-populated story: every section is present and densely substantive, the 24-section content inventory in Technical Notes gives an implementer a near-complete spec, and the Acceptance Criteria are mostly framed as observable Given/When/Then behaviours. The findings that warrant a REVISE are not about absence — they are about three reconcilable tensions: (1) the machine-readable `blocked_by` field disagrees with the Dependencies prose about which stories block, (2) the work item's own Drafting Notes flag that the scope has grown to epic size and that kind/priority are unsettled, yet Open Questions reads "None outstanding", and (3) the rich section inventory that should serve as the verification oracle is not wired into the fidelity acceptance criteria, which still lean on subjective "renders correctly" / "verified against the prototype" phrasing. None is a blocker to *understanding* the work; each is a reconciliation the work item should carry before it is planned.

### Cross-Cutting Themes

- **`blocked_by` frontmatter contradicts the Dependencies prose** (flagged by: clarity, dependency) — The frontmatter lists three blockers (`0037`, `0038`, `0033`) while the prose names four more (`0035`, `0039`, `0040`, `0041`) and then hedges that they "may need adding once their IDs are confirmed" — though the IDs are already stated. The structured blocker graph and the human-readable one cannot both be relied on, and a planner reading only the frontmatter would conclude the work is unblocked sooner than the prose implies.
- **Scope / sizing / kind is self-flagged as unsettled but not surfaced where decisions live** (flagged by: scope, completeness) — The Drafting Notes explicitly call the work "materially larger than the original draft implied — flagged for sizing", question whether `kind` should be `task`, and note `priority: low` "may warrant revisiting" — yet Open Questions says "None outstanding". The scope lens independently reads the Requirements as bundling several independently-deliverable workstreams (page + content fidelity, activation infrastructure, scroll-spy, theme toggle, five-route retirement + VR migration).
- **The fidelity oracle exists but isn't wired into the criteria** (flagged by: testability) — Technical Notes enumerates exact per-section variant counts (Surfaces 8, brand palette 19, Chips 6×2, Status badges 8+4, …), but AC #1, #2 and the VR-migration AC quantify over un-enumerated reference sets ("every primitive shown by the showcases", "verified section-by-section against the prototype", "no primitive loses coverage"), leaving the pass/fail to manual diligence.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity + Dependency**: `blocked_by` frontmatter contradicts the Dependencies prose on which items block
  **Location**: Frontmatter: `blocked_by` / Dependencies
  The frontmatter lists exactly `0037`, `0038`, `0033`, but the Dependencies prose adds "the other primitives delivered by 0035, 0039, 0040, 0041" and says these "may need adding to `blocked_by` once their IDs are confirmed" — yet those IDs are already stated explicitly in both Dependencies and References. The machine-readable blocker graph that scheduling relies on is therefore incomplete, and the two enumerations of prerequisites disagree.

- 🟡 **Testability**: "Verified section-by-section against the prototype" relies on manual comparison without a defined procedure
  **Location**: Acceptance Criteria (#2)
  AC #2 requires each section to render "the full variant set specified by the prototype `view-dev.jsx`, verified section-by-section". The mechanism is a manual eyeball diff against a separate file; it gives examples for three sections but not the other ~21, and does not say what counts as a match given the explicit instruction to port to live tokens (so pixels will differ). A dropped variant in an un-exemplified section (Type's 7 ramps, Spacing's 11 steps, Icons' ~33 names) could pass unnoticed.

- 🟡 **Testability**: "Renders correctly" in the theme AC has no defined pass/fail procedure
  **Location**: Acceptance Criteria (#6)
  AC #6 states the page "renders correctly under both the light and dark themes" and that "switching theme restyles all sections". "Renders correctly" is subjective with no oracle; two reviewers could disagree. (The reviewing agent noted the second half — Colours swatches reflecting active token values — is already measurable, so the AC is partly testable.)

- 🟡 **Scope**: Story bundles several independently-deliverable concerns under one unit of work
  **Location**: Requirements
  The Requirements combine at least five separable workstreams: (1) the 24-section page at full fidelity, (2) net-new activation infrastructure (hash routing, cross-browser keychord, sidebar-foot element + triple-click), (3) net-new scroll-spy with `IntersectionObserver` + hash sync, (4) theme toggle behaviour, and (5) retiring five routes + migrating each one's VR spec/baselines. The page-build is the cohesive core, but route retirement and the activation/scroll-spy infrastructure could each be built, deployed, or rolled back independently.

- 🟡 **Scope**: Self-flagged scope expansion makes the `story` sizing questionable
  **Location**: Drafting Notes
  The Drafting Notes record the scope was expanded from two showcases to five (with VR migration) and further to add full content fidelity, deep-linking + hash sync, theme coverage, and dev-page chrome — concluding it is "materially larger than the original draft implied — flagged for sizing". Combined with ten ACs spanning rendering, three activation paths, cross-browser verification, theme switching, 404 behaviour, VR migration, and README updates, `kind: story` sits at the upper edge of a single increment and may warrant an epic.

#### Minor

- 🔵 **Dependency**: Conditional "missing primitive" blocker named in Assumptions is absent from Dependencies
  **Location**: Assumptions
  Assumptions states that "Any primitive the prototype shows that does not yet exist in the live app surfaces a dependency to resolve before this section can reach fidelity" — a potential upstream blocker not reflected in Dependencies, risking a mid-sprint surprise blocker.

- 🔵 **Testability**: VR-migration AC asserts "no primitive loses coverage" without an enumerated mapping
  **Location**: Acceptance Criteria (VR migration)
  "No primitive loses VR coverage" is a negative, set-spanning claim with no showcase→section mapping in the AC, so a verifier cannot deterministically confirm the migrated assertions cover everything the five old specs asserted.

- 🔵 **Testability**: AC #1 "every primitive currently shown by the five retired showcases appears" lacks an enumerated reference set
  **Location**: Acceptance Criteria (#1)
  "Every primitive" quantifies over a set defined only in external showcase source, so a definitive pass/fail depends on a reference the verifier must reconstruct themselves.

- 🔵 **Clarity**: Section-slug referent ambiguous — "Colours" label vs `#dev/colors` deep-link
  **Location**: Summary / Requirements / Acceptance Criteria
  The deep-link example uses American spelling (`#dev/colors`) while the section is labelled "Colours". It is unstated whether `<section>` derives from the display label (→ `colours`) or a separate prototype slug (→ `colors`), so an implementer could derive the wrong hash segment.

- 🔵 **Completeness**: Story does not explicitly identify the user or system whose need is met
  **Location**: Context
  Context and Summary describe what is built without naming the audience; the Drafting Notes acknowledge "no direct end-user benefit", but the "for whom" (developers/designers maintaining the visualiser) is never stated in Context.

- 🔵 **Completeness**: Unresolved kind/priority/sizing tension is noted in Drafting Notes but not surfaced as an Open Question
  **Location**: Drafting Notes / Open Questions
  The author's own live decisions (sizing, `priority: low`, story-vs-task) are flagged in Drafting Notes but Open Questions reads "None outstanding", so a refiner scanning the canonical place would miss them.

- 🔵 **Scope**: Declared kind (`story`) is self-acknowledged as arguably a `task`
  **Location**: Frontmatter: `kind`
  The Drafting Notes record `kind` was "left as `story` this session; it remains arguably a `task` (developer tooling, no direct end-user benefit)". A housekeeping concern, but it should settle once the sizing/decomposition question resolves.

- 🔵 **Dependency**: Cross-browser verification depends on four browser engines whose behaviour is not noted as a coupling
  **Location**: Acceptance Criteria (#7)
  Only the Chromium `Cmd/Ctrl+Shift+D` reservation is named as a coupling; if the suggested fallback `Cmd/Ctrl+Shift+L` collides with a reserved shortcut in Edge/Firefox/Safari, implementation could stall on an unrecorded external constraint.

- 🔵 **Scope**: Delivery depends on simultaneous completion of multiple parallel primitive stories
  **Location**: Dependencies
  Full fidelity requires every primitive to already exist, gating delivery on a broad set of upstream completions (0037, 0038, 0033, plus 0035/0039/0040/0041). Consider whether the page can land section-by-section as primitives arrive, or confirm this is an accepted capstone-consolidation property.

#### Suggestions

- 🔵 **Clarity**: SSE acronym used without definition
  **Location**: Technical Notes (Topbar inventory)
  The "SSE indicator" is the only undefined acronym in an otherwise self-defining document; expand on first use (e.g. "SSE (server-sent events) indicator").

- 🔵 **Testability**: AC #4 deep-link does not specify a scroll-position tolerance
  **Location**: Acceptance Criteria (#4, #5)
  "Lands on and scrolls to that section" / "scrolled into view" lack a threshold; Technical Notes records the prototype's `rootMargin`/`threshold`, which could anchor a deterministic assertion (heading within the observer's active region; TOC entry carries active state).

### Strengths

- ✅ Subject identity is stable throughout — "the prototype `view-dev.jsx`", "the live app", "DevDesignSystem" are introduced once and referenced consistently; the Summary's scope maps cleanly onto Requirements and Acceptance Criteria with no inter-section contradictions.
- ✅ Every expected section for a story is present and densely substantive (no placeholders), and the frontmatter is complete and valid (`kind`, `status`, `priority`, `tags`, `blocked_by`, `source` all set).
- ✅ The Technical Notes content inventory is an exceptional verification oracle — 24 sections with exact per-section variant counts — so an implementer can reach fidelity without reconstructing the spec.
- ✅ Acceptance Criteria are numerous (ten) and mostly framed as observable behaviours with named variant sets; AC #5 explicitly demands an automated test and pins the negative condition ("never pinned to a single section"); the 404 and README criteria are fully deterministic.
- ✅ Scope boundaries are explicitly drawn (24 named sections, five named routes; extension/user-custom keybindings explicitly out of scope), and the route retirement is anchored to an authoritative in-repo designation (`router.ts:161-162`).
- ✅ The Chromium "bookmark all tabs" reservation on `Cmd/Ctrl+Shift+D` is named and resolved, with the alternative chord gated behind a cross-browser verification AC; Open Questions records prior questions as resolved rather than leaving the section blank.
- ✅ The Drafting Notes transparently surface the sizing/kind/priority tension, showing the scope risk was consciously considered rather than overlooked.

### Recommended Changes

1. **Reconcile `blocked_by` frontmatter with the Dependencies prose** (addresses: "blocked_by contradicts Dependencies prose"; "Conditional missing-primitive blocker absent from Dependencies")
   Decide whether `0035`, `0039`, `0040`, `0041` are true blockers. If so, add `work-item:0035`, `work-item:0039`, `work-item:0040`, `work-item:0041` to frontmatter `blocked_by` (the IDs are stated, so the "once confirmed" caveat is satisfied) and drop the hedge from the prose. If not, state in Dependencies that they are References-only, not prerequisites. Also add a Dependencies note that content fidelity is contingent on every prototype primitive already existing in the live app, so a discovered gap surfaces a tracked blocker.

2. **Settle the scope / sizing / kind question and record it in Open Questions** (addresses: "Story bundles independently-deliverable concerns"; "Self-flagged scope expansion"; "Declared kind arguably a task"; "kind/priority/sizing not surfaced as an Open Question"; "Delivery depends on parallel primitive stories")
   Either (a) decompose into an epic with cohesive child stories — e.g. the reference page + theme coverage as the core, activation triggers as a second, route retirement + VR migration as a third — or (b) keep it as one story with an explicit indivisibility rationale (the VR migration depends on the page sections existing). Whichever way, settle `kind` and revisit `priority: low`, and move the live decisions from Drafting Notes into Open Questions (or mark them resolved) so they are visible where refiners look.

3. **Wire the Technical Notes inventory into the acceptance criteria as the verification oracle** (addresses: "Verified section-by-section relies on manual comparison"; "Renders correctly has no defined procedure"; "VR-migration AC lacks enumerated mapping"; "AC #1 lacks an enumerated reference set")
   Promote the per-section variant counts into the ACs as countable assertions ("an automated test asserts the rendered swatch/chip/glyph/ramp count per section equals the inventory figure"). Replace "renders correctly" with a defined check (light + dark VR snapshot per section; computed `--ac-*` surface token values change on theme toggle, asserted via computed styles for at least the Colours swatches). Add an explicit five-row showcase→section mapping (glyph→Icons/Doc-type glyphs, big-glyph→Empty-state glyphs, chip→Chips, code-syntax→Code blocks, kanban-card→Cards) and phrase the migration AC as a row-by-row audit.

4. **Resolve the section-slug ambiguity** (addresses: "Section-slug referent ambiguous")
   State what `<section>` resolves to (e.g. "the prototype `DEV_SECTIONS` id, not the display label") and either align the `#dev/colors` example with the "Colours" label or note that the slug intentionally differs.

5. **Minor polish** (addresses: "Story does not identify the user/system"; "SSE acronym"; "AC #4 scroll tolerance"; "Cross-browser coupling")
   Name the page's audience in Context; expand "SSE" on first use; add a scroll-position tolerance to AC #4/#5 anchored on the recorded `rootMargin`/`threshold`; note in Dependencies that the chosen chord is coupled to browser-engine reserved-shortcut behaviour across all four target browsers.

## Per-Lens Results

### Clarity

**Summary**: The work item is largely unambiguous and internally coherent: subjects ("the user", "DevDesignSystem", "the prototype") are stable across sections, the 24-section count and the five retired routes are named consistently, and the Summary's scope matches the Requirements and Acceptance Criteria. The main clarity gaps are a mismatch between the frontmatter `blocked_by` list and the Dependencies prose, an unresolved spelling/slug ambiguity between section labels ("Colours") and the deep-link example ("#dev/colors"), and one undefined acronym (SSE).

**Strengths**:
- Subject identity is stable: "the prototype view-dev.jsx", "the live app", and "DevDesignSystem" are introduced once and referenced consistently, so referents never shift mid-document.
- The Summary's scope (24 sections, dual-theme, three activation triggers, retire five routes and migrate VR coverage) maps cleanly onto the Requirements and Acceptance Criteria with no contradictions between sections.
- Actors are explicitly named in the Acceptance Criteria (the user navigates, presses, triple-clicks, scrolls), avoiding actor-obscuring passive constructions for the interactive behaviours.
- The "visual-regression" term is spelled out before being abbreviated to "VR", and the excluded/suggested keychords are stated unambiguously throughout.

**Findings**:
- 🟡 **major** (high) — _Dependencies_ — **Frontmatter blocked_by contradicts Dependencies prose on which items block**: The frontmatter `blocked_by` lists exactly three items (`0037`, `0038`, `0033`), but the prose states "Blocked by: 0037, 0038, 0033, and the other primitives delivered by 0035, 0039, 0040, 0041" and then hedges that those "may need adding to `blocked_by` once their IDs are confirmed". The two enumerations of blockers disagree, and a reader cannot tell whether 0035/0039/0040/0041 are actual blockers. Suggestion: reconcile so both describe the same blocker set.
- 🔵 **minor** (medium) — _Summary_ — **Section-slug referent ambiguous: "Colours" label vs "#dev/colors" deep-link**: The deep-link example uses American spelling (`#dev/colors`) while the section is labelled "Colours". It is unstated whether `<section>` derives from the display label or a separate prototype slug, so an implementer could derive the wrong hash segment. Suggestion: state what `<section>` resolves to and align the example or note the intentional divergence.
- 🔵 **suggestion** (medium) — _Technical Notes_ — **SSE acronym used without definition**: The "SSE indicator" in the Topbar inventory is the only undefined acronym in an otherwise self-defining document. Suggestion: expand on first use or link to its definition.

### Completeness

**Summary**: An exceptionally well-populated story: every expected section is present and densely substantive, and the frontmatter is complete and valid. The 24-section content inventory, ten acceptance criteria, and detailed Context give an implementer everything needed to start without follow-up. The only gaps are minor: Context frames the work as developer tooling without explicitly naming the user/system whose need is served, and the author's own Drafting Notes flag an unresolved kind/priority/sizing tension that has not been carried into Open Questions.

**Strengths**:
- Every expected section for a story is present and substantively populated — Summary, Context, Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes, References — with real content, not placeholders.
- Frontmatter is complete and valid: kind (story), status (draft), priority (low), tags, blocked_by, source all present and recognised.
- Acceptance Criteria are numerous (ten) and each maps cleanly to a distinct requirement.
- Technical Notes provides an unusually thorough content inventory (24 sections with per-section variant counts).
- Open Questions explicitly records that prior questions were resolved rather than leaving the section blank.

**Findings**:
- 🔵 **minor** (medium) — _Context_ — **Story does not explicitly identify the user or system whose need is met**: Context and Summary describe what is built without naming the audience; the Drafting Notes acknowledge "no direct end-user benefit", yet the "for whom" is never stated in Context. Suggestion: add a sentence naming the consumer (developers/designers maintaining the visualiser).
- 🔵 **minor** (medium) — _Drafting Notes_ — **Unresolved kind/priority/sizing tension is noted but not surfaced as an Open Question**: The Drafting Notes raise the sizing/priority and story-vs-task questions, but Open Questions states "None outstanding", so these live decisions are not captured where a refiner would look. Suggestion: promote them into Open Questions or resolve them.

### Dependency

**Summary**: The work item captures its principal upstream couplings well — it names the prototype as authoritative content spec, identifies the browser keychord reservation, and points to the router comment designating this work item as the consolidation home. However, there is a material mismatch between the prose Dependencies (which names 0035, 0039, 0040, 0041) and the frontmatter `blocked_by` (only 0037, 0038, 0033), leaving four implied upstream blockers out of the machine-readable record. A conditional "missing primitive" blocker named in Assumptions is also uncaptured in Dependencies.

**Strengths**:
- The Chromium "bookmark all tabs" reservation on `Cmd/Ctrl+Shift+D` is an external-system coupling explicitly named and resolved, with the alternative chord gated behind a cross-browser verification AC.
- Context and Technical Notes name `src/router.ts:161-162` as the comment designating this work item the consolidation home and warning to migrate (not delete) the VR specs/baselines — the inbound coupling is captured.
- The prototype `view-dev.jsx` and its supporting sources are explicitly named as the authoritative content specification.
- "Blocks: none" is defensible — the retired routes are reached by direct URL with no in-app links, so no downstream consumer is left implied.

**Findings**:
- 🟡 **major** (high) — _Frontmatter: blocked_by_ — **blocked_by frontmatter omits four upstream blocker stories named in Dependencies prose**: The frontmatter lists only `0037`, `0038`, `0033`, but the prose names 0035, 0039, 0040, 0041 as primitive providers, stating they "may need adding once their IDs are confirmed" — yet the IDs are already stated. The blocker graph scheduling relies on is incomplete. Suggestion: add the four IDs to `blocked_by`, or explicitly downgrade them to References-only.
- 🔵 **minor** (high) — _Assumptions_ — **Conditional "missing primitive" blocker named in Assumptions is absent from Dependencies**: Assumptions states any prototype primitive not yet in the live app "surfaces a dependency to resolve before this section can reach fidelity", a potential blocker not reflected in Dependencies. Suggestion: add a Dependencies note that fidelity is contingent on every primitive already existing.
- 🔵 **minor** (medium) — _Acceptance Criteria_ — **Cross-browser verification depends on four browser engines whose behaviour is not noted as a coupling**: Only the Chromium case is named; if the fallback chord collides with a reserved shortcut in another engine, implementation could stall on an unrecorded constraint. Suggestion: note the chord is coupled to browser-engine reserved-shortcut behaviour across the four targets.

### Scope

**Summary**: This story bundles a genuinely cohesive core (a single consolidated DevDesignSystem reference page) with several independently-deliverable concerns: net-new activation infrastructure, net-new scroll-spy/hash-sync, theme toggle behaviour, and the retirement-plus-VR-migration of five separate showcase routes. The deliverable is materially larger than a typical story — a point the work item's own Drafting Notes flag — and its kind (story vs task) and priority are self-acknowledged as unsettled. The work is logically related through the single page, but the multi-route retirement and activation/scroll-spy infrastructure are separable enough to warrant either decomposition into an epic or an explicit indivisibility justification.

**Strengths**:
- Scope boundaries are unusually well-drawn: in scope (24 named sections, five named routes) and out (extension/user-custom keybindings explicitly excluded) are both stated.
- Route retirement is anchored to an authoritative in-repo designation (`router.ts:161-162`), so the bundling is non-arbitrary.
- The Drafting Notes already surface the sizing tension transparently, showing the scope risk was consciously considered.
- All five retired primitives are claimed to be fully represented within the 24 sections, giving a coherence argument for bundling route retirement with the page build.

**Findings**:
- 🟡 **major** (medium) — _Requirements_ — **Story bundles several independently-deliverable concerns under one unit of work**: At least five separable workstreams (page + fidelity, activation infrastructure, scroll-spy + hash sync, theme toggle, five-route retirement + VR migration); route retirement and activation/scroll-spy could each be built/deployed/rolled back independently. Suggestion: consider an epic with child stories, or state the indivisibility rationale explicitly.
- 🟡 **major** (medium) — _Drafting Notes_ — **Self-flagged scope expansion makes "story" sizing questionable**: The Drafting Notes call the work "materially larger than the original draft implied — flagged for sizing"; combined with ten wide-ranging ACs, `kind: story` is at the upper edge of a single increment. Suggestion: decompose into an epic, or trim to the indivisible core and split out activation infrastructure and route retirement.
- 🔵 **minor** (medium) — _Frontmatter: kind_ — **Declared kind (story) is self-acknowledged as arguably a task**: The Drafting Notes note `kind` "remains arguably a `task` (developer tooling, no direct end-user benefit)". Suggestion: settle once the sizing/decomposition question resolves.
- 🔵 **minor** (low) — _Dependencies_ — **Delivery depends on simultaneous completion of multiple parallel primitive stories**: Full fidelity requires every primitive to already exist, gating delivery on a broad set of upstream completions. Suggestion: consider incremental section-by-section delivery, or confirm the capstone-consolidation property is intended.

### Testability

**Summary**: This story is unusually testable for its size: most of the 10 acceptance criteria are framed as observable Given/When/Then behaviours with concrete, named variant sets, and an explicit per-section content inventory in Technical Notes serves as the verification oracle. The main gaps are the recurring reliance on subjective "renders correctly" / "verified section-by-section against the prototype" phrasing for the visual-fidelity criteria, and the cross-browser chord verification AC which mixes a testable automated assertion with a property (a browser not reserving the chord) that cannot be exhaustively verified.

**Strengths**:
- Most criteria are framed as observable behaviours (navigation triggers activation, scrolling updates TOC + URL hash, retired path returns 404) with clear preconditions and outcomes.
- The per-section content inventory enumerates exact variant counts, giving AC #2 a concrete verification oracle rather than leaving "full variant set" undefined.
- AC #5 explicitly demands the scroll-spy behaviour be "asserted by an automated test" and pins the negative condition ("never pinned to a single section").
- AC #9 (404, no redirect) and AC #10 (README lists DevDesignSystem, drops the five showcases) are fully deterministic.
- The activation-chord AC bounds its own scope ("browser built-in shortcuts; extension/user-custom bindings out of scope").

**Findings**:
- 🟡 **major** (high) — _Acceptance Criteria_ — **"Verified section-by-section against the prototype" relies on manual comparison without a defined procedure**: AC #2's verification is a manual diff against a separate file; it exemplifies three sections but not ~21 others, and does not say what counts as a match given the port to live tokens. A dropped variant in an un-exemplified section could pass unnoticed. Suggestion: promote the Technical Notes inventory into a countable assertion (rendered count per section equals the inventory figure).
- 🟡 **major** (high) — _Acceptance Criteria_ — **"Renders correctly" in the theme AC has no defined pass/fail procedure**: AC #6's "renders correctly under both themes" is subjective with no oracle. (The reviewing agent noted the later half — Colours swatches reflecting active token values — is already measurable.) Suggestion: replace with light/dark VR snapshots per section and computed-style assertions on `--ac-*` surface tokens.
- 🔵 **minor** (medium) — _Acceptance Criteria_ — **Cross-browser "not a reserved shortcut" property is not exhaustively verifiable as stated**: The keydown-delivery part is testable per browser, but "not reserved" can only be confirmed for the versions tested, and the suggested chord is not locked in as the thing under test. Suggestion: split into (a) an automated test that the chosen chord's handler fires and `preventDefault` is honoured, and (b) a recorded manual matrix naming the exact chord and browser+version+OS.
- 🔵 **minor** (medium) — _Acceptance Criteria_ — **VR-migration AC asserts "no primitive loses coverage" without an enumerated mapping**: A negative, set-spanning claim with no showcase→section mapping in the AC. Suggestion: add an explicit five-row mapping and phrase the AC as a row-by-row audit.
- 🔵 **minor** (medium) — _Acceptance Criteria_ — **AC #1 "every primitive currently shown by the five retired showcases appears" lacks an enumerated reference set**: "Every primitive" quantifies over a set defined only in external source. Suggestion: reference the same per-section inventory used for AC #2, or add a short per-showcase primitive list.
- 🔵 **suggestion** (low) — _Acceptance Criteria_ — **AC #4 deep-link does not specify a scroll-position tolerance**: "Lands on and scrolls to that section" / "scrolled into view" lack a threshold; the prototype's recorded `rootMargin`/`threshold` could anchor a deterministic assertion. Suggestion: state the verifiable target (heading within the observer's active region; TOC entry carries active state).

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-12

**Verdict:** COMMENT

All five previously-identified major findings are resolved; the verdict improves from REVISE to COMMENT. Re-running all five lenses confirms the edits hold: the blocker graph is reconciled, the fidelity criteria now carry concrete oracles, the slug/audience/acronym gaps are closed, and the scope/sizing/kind decisions are recorded where refiners look. One new major surfaced as a direct consequence of sharpening AC #2 — the count-equality assertion is only fully enumerated for 3 of 24 sections — plus a cluster of minor polish items. With a single major and no critical findings, the work item is acceptable to plan; the new major is worth a follow-up edit.

### Previously Identified Issues

- 🟡 **Clarity + Dependency**: `blocked_by` frontmatter contradicts Dependencies prose — **Resolved**. Frontmatter now lists all nine done prerequisites and the Dependencies prose names them by title and marks them satisfied; both lenses now cite this as a strength. (Dependency notes a residual *suggestion* that the field lists nine `done` items as blocking edges — this is the deliberate provenance-record decision; fine if planning tooling distinguishes done from open blockers.)
- 🟡 **Testability**: "Verified section-by-section against the prototype" relied on manual comparison — **Resolved**. AC #2 now demands an automated count-equality assertion against the Technical Notes inventory, cited as a strength. (See new major below: the inventory figures are only partially enumerated.)
- 🟡 **Testability**: "Renders correctly" theme AC had no defined procedure — **Partially resolved**. AC #6 now specifies per-section light/dark VR snapshots + computed-style checks (a strength), but the Requirements wording still says "renders correctly", and VR verifies stability rather than correctness with the computed-style check scoped to Colours only.
- 🟡 **Scope**: Story bundles several independently-deliverable concerns — **Resolved** (major → minor). The recorded indivisibility rationale and the explicit Open-Questions sizing decision moved this to a strength; a residual minor notes the rationale covers VR + fidelity but not the net-new activation/scroll-spy/hash infrastructure strands.
- 🟡 **Scope**: Self-flagged scope expansion made `story` sizing questionable — **Resolved** (major → minor). Now "a defensible deliberate call given the recorded indivisibility rationale… not a delivery-blocking scope defect."
- 🔵 **Dependency**: Conditional missing-primitive blocker absent from Dependencies — **Resolved**. Now explicitly captured in Dependencies and Assumptions (cited as a strength).
- 🔵 **Testability**: VR-migration AC lacked an enumerated mapping — **Resolved**. The showcase→section mapping in Technical Notes is now the row-by-row oracle for AC #9.
- 🔵 **Testability**: AC #1 "every primitive" lacked an enumerated reference set — **Resolved**. AC #1 now resolves to the finite written checklist via the mapping (cited as a strength).
- 🔵 **Clarity**: Section-slug ambiguity (`Colours` vs `#dev/colors`) — **Resolved**. The slug-vs-label divergence is explicitly reconciled in Requirements and AC.
- 🔵 **Completeness**: Story did not identify the user/system — **Resolved**. Context now names the visualiser's maintainers as the audience.
- 🔵 **Completeness / Scope**: kind/priority/sizing not surfaced as Open Questions; `kind` arguably a task — **Resolved**. Both decisions are recorded in Open Questions and Drafting Notes with dated rationale.
- 🔵 **Dependency**: Cross-browser verification coupling not noted — **Resolved**. The browser-engine reserved-shortcut coupling is now named in Dependencies.
- 🔵 **Scope**: Delivery depends on parallel primitive stories — **Resolved / moot**. All upstream stories are `done`, so 0083 is unblocked; not re-raised.
- 🔵 **Clarity**: SSE acronym undefined — **Resolved**. Expanded to "SSE (server-sent events)".
- 🔵 **Testability**: AC #4 deep-link lacked a scroll tolerance — **Resolved**. Now anchored to the observer's active region per the documented `rootMargin` (cited as a strength).

### New Issues Introduced

- 🟡 **Testability** (major): **AC #2 per-section variant counts are only partially enumerated.** The count-equality assertion gives explicit integers for only 3 of 24 sections (Colours, Status badges, Cards); the rest must be reconstructed from prose, and several are not single integers — Icons "~33", Colours doc-type hues "per `TYPE_META`", Chips "6 tones × sm/md". A strict equality test has no fixed target for ~18 sections. Suggestion: add an explicit per-section count table (composite sections stated as the resolved product, e.g. Chips = 12; `ICON_NAMES` as its exact length), and state whether `TYPE_META`-derived counts assert against the live length or a frozen number.
- 🔵 **Clarity** (minor): **`Glyph` vs `TypeGlyph` naming.** The doc-type glyph component is called `TypeGlyph` in the inventory/Assumptions but `Glyph` in the new showcase→section mapping; state they are the same component.
- 🔵 **Clarity** (minor): **Doc-type glyph count 12 vs 13.** The mapping describes `/glyph-showcase` as "12 doc-type Glyphs" but `/big-glyph-showcase` as "13 doc-type heroes"; reconcile (or explain why the hero set differs by one).
- 🔵 **Completeness** (suggestion): **No AC for the in-page theme-toggle affordance.** Requirements call for a theme toggle, but the ACs verify rendering in each theme without confirming the toggle exists and switches the theme.
- 🔵 **Dependency** (minor ×2): The **five existing VR specs** and the **frontend README** are shared artefacts this work mutates but are not named as couplings in Dependencies (both are captured in Requirements/ACs, so this is record-completeness, not a missing coupling).
- 🔵 **Testability** (minor ×3): AC #7 "exit to app" / AC #3 "activates" lack a defined observable post-condition; the marquee/footer keybind hint is only checked as "not ⌘⇧D" rather than equalling the bound chord; AC #5's "enters view" is not bound to the observer active region the way AC #4 is.

### Assessment

The work item is ready to plan. Every major finding from the first pass is resolved and the verdict improves to **COMMENT**. The one new major (AC #2 count enumeration) and the small cluster of clarity/testability minors it sits within are worth a quick follow-up — they all converge on producing one explicit per-section variant-count table (with the `Glyph`/`TypeGlyph` and 12-vs-13 reconciliations folded in) — but none blocks planning.

### Post-Re-Review Edits (2026-06-12)

The pass-2 findings were addressed by direct edits to the work item (prototype sources were read to resolve exact counts; not re-verified by a further lens pass):

- 🟡 **AC #2 count enumeration (major)** — added a **per-section variant-count oracle table** to Technical Notes covering all 24 sections. Dynamic counts bind to the live source constant (`ICON_NAMES.length` = 33, `STAGES.length` = 9, doc-type counts via `TYPE_META`) rather than frozen integers; compositional sections assert presence of each named variant. AC #2 now references the table.
- 🔵 **`Glyph` vs `TypeGlyph`** — inventory now states they are the same component.
- 🔵 **12-vs-13 doc-type count** — explained as two independently-sized sets in the live app (`/glyph-showcase` rendered 12, `/big-glyph-showcase` 13); each section's count binds to its own component's source set. "~33" corrected to exact 33.
- 🔵 **Theme-toggle AC** — added an acceptance criterion that the in-page toggle exists and switches the rendered theme; the theme Requirement reworded from "renders correctly" to layout-without-breakage + theme-appropriate token values with the VR baselines as the correctness oracle.
- 🔵 **AC #3 / #5 / #7 post-conditions** — AC #3 now names the observable activation signal (marquee + TOC + `#dev` hash); AC #5 binds "enters view" to the observer active region (matching the deep-link criterion); AC #7 states the exit end-state and requires the keybind hint to equal the bound chord constant.
- 🔵 **Dependency record** — added a line noting the five VR specs + baselines and the frontend README as shared artefacts this work mutates.
- 🔵 **Summary wording** — "non-conflicting modifier keychord" tightened to "a modifier keychord that is not a reserved browser shortcut".

The remaining dependency observation (nine `done` items retained in `blocked_by`) is the deliberate provenance-record decision and was left as-is.

### Approval (2026-06-12)

**Verdict: APPROVE.** Both review passes' findings are resolved or consciously accepted; the work item is approved for planning. (Verdict set by the reviewer after the post-re-review edits; the frontmatter `verdict` reflects this final decision.)
