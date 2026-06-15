---
type: work-item-review
id: "0111-visualiser-frontend-closeout-fixes-review-1"
title: "Work Item Review: Visualiser Frontend Fixes for First Milestone Closeout"
date: "2026-06-15T16:06:04+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0111"
work_item_id: "0111"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: ["visualiser", "frontend", "milestone-closeout"]
last_updated: "2026-06-15T16:23:39+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Visualiser Frontend Fixes for First Milestone Closeout

**Verdict:** COMMENT

This is a high-quality, exceptionally well-specified container story. All five
lenses found it structurally complete, internally consistent, and coherently
scoped — the container-story pattern is deliberately justified, the F1 split-out
to 0112 shows mature scope hygiene, and most acceptance criteria carry concrete,
checkable values (exact tokens, opacities, font sizes, verbatim text). The work
item is acceptable as-is; the one major finding (M1's reliance on a subjective
"visually matches the prototype" check) and the minor/suggestion findings below
are improvements worth folding in before implementation, not blockers.

### Cross-Cutting Themes

- **Prototype-derived correctness vs. self-contained verifiability** (flagged by:
  testability, dependency) — Several criteria (M1, M3, L4) and the prototype
  reference encode their pass conditions as comparisons against the external
  design prototype rather than as fully self-contained, measurable assertions.
  Testability flags the subjective "visually matches"/"clearly less prominent"
  phrasing; dependency notes the criteria's correctness is gated on the named
  prototype snapshot as a frozen source of truth. Tightening the criteria to
  restate the concrete prototype-derived constants inline resolves both.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: M1 verification relies on subjective "visually matches the prototype"
  **Location**: Acceptance Criteria (M1)
  M1's pass condition is that a rendered table "visually matches the prototype's
  `.ac-md-tablewrap` / `.ac-md-table` treatment". The binding verb is a subjective
  visual comparison against an external prototype rather than self-contained
  measurable assertions, and it omits the specific tokens stated in the
  Requirements (`--ac-bg-sunken` header fill, `--ac-stroke-soft` separators, the
  clip/no-horizontal-scroll behaviour).

#### Minor

- 🔵 **Dependency**: Linux visual-regression baseline regeneration not captured in Dependencies
  **Location**: Technical Notes
  Technical Notes states the work requires "visual-regression baseline
  regeneration (darwin + linux)", but the Dependencies section does not capture
  the cross-environment baseline-update step. Linux VR baselines regenerate via a
  separate CI workflow, an out-of-band process coupling that risks discovery at PR
  time rather than being planned for.

- 🔵 **Testability**: M3 "clearly less prominent than body text" is a subjective threshold
  **Location**: Acceptance Criteria (M3)
  M3's first clause is precise (1px divider filled with `--ac-stroke`), but the
  trailing clause "clearly less prominent than body text" introduces a subjective
  comparator with no defined threshold, so it cannot produce a definitive pass/fail
  on its own.

- 🔵 **Testability**: L4 "measurably fainter" lacks the measurement procedure
  **Location**: Acceptance Criteria (L4)
  L4 gives precise sub-assertions (0.7 block opacity, additional 0.75 on the
  heading, 12.5px font-size), but its concluding "measurably fainter than other
  nav section headings and links" clause defines neither how the effective colour
  is measured nor the reference value for the comparison.

- 🔵 **Testability**: L2 "decisions still appear in related-artifacts" lacks a defined input fixture
  **Location**: Acceptance Criteria (L2)
  L2's positive assertion does not specify the precondition that the test
  lifecycle/work unit actually contains a decision (ADR). Without a fixture known
  to include one, the criterion could be vacuously "passed" against a unit that has
  no decisions at all.

#### Suggestions

- 🔵 **Clarity**: Fix label "F1" referenced before introduction
  **Location**: Technical Notes
  "F1" appears in Technical Notes and Drafting Notes (split out to 0112) but is
  never listed among this story's Requirements (M1–M3, L1–L4). A reader
  encountering "F1" mid-document has no prior referent for what it covered.

- 🔵 **Completeness**: No Open Questions section for contingent decisions
  **Location**: Open Questions
  The item has no Open Questions section even though it contains latent ones —
  e.g. the M1 Assumption notes that if horizontal scrolling for wide tables is
  wanted, "that changes the requirement". Such contingent decisions are captured
  under Assumptions rather than surfaced as questions.

- 🔵 **Dependency**: Downstream "general release" consumer is an abstract milestone, not a trackable item
  **Location**: Dependencies
  The sole downstream consumer is captured as "Blocks: General release of the first
  visualiser version" — an abstract milestone rather than a trackable work item or
  release ticket, so the unblocking relationship cannot be tracked.

- 🔵 **Dependency**: Prototype snapshot not framed as a dependency artefact
  **Location**: Requirements
  M1, L2, L3, and L4 derive their target treatment (wording strings, opacity
  factors, font sizes) from the named design prototype, which is the authoritative
  source for exact values. It is named in Context/References but not framed as a
  dependency artefact whose continued availability the criteria rely on.

- 🔵 **Scope**: L2 is the one logic-bearing fix in a bundle of cosmetic tweaks
  **Location**: Requirements (L2)
  L2 carries genuinely independent, multi-file behavioural logic (pipeline
  reclassification, `/8`→`/7` count, tests, `cluster-via-label.ts`), whereas the
  other six are CSS/text parity tweaks. Bundling raises its review/test surface
  slightly. No structural split warranted given the deliberate container framing.

- 🔵 **Scope**: "story" kind for a closeout polish bundle is a judgement call
  **Location**: Frontmatter: kind
  The item is filed as a `story` but is framed as a container of small QA fixes
  rather than a single increment of user-visible value; some teams would file this
  as a chore or task. Negligible delivery risk — a housekeeping label question.

- 🔵 **Clarity**: "QA" acronym used without expansion
  **Location**: Context
  "QA" is used in the Summary and Context without expansion on first use. Near
  universal, so stylistic rather than a genuine ambiguity.

- 🔵 **Clarity**: Design-token names used without a definition pointer
  **Location**: Requirements
  Requirements/Acceptance Criteria reference `--ac-*` and `--size-130` tokens as
  authoritative outcome values without a pointer (in those sections) to where the
  token scale is defined; the anchor only appears later in Technical Notes.

### Strengths

- ✅ Every fix carries a stable, unambiguous label (M1–M3, L1–L4) used
  consistently across Requirements, Acceptance Criteria, and Technical Notes, with
  a one-to-one Requirement↔Acceptance-Criterion mapping in Given/When/Then form.
- ✅ Outcomes are stated as concrete observable states — exact opacity values,
  font sizes, token names, and verbatim heading/subheading text — rather than
  vague desired properties (L3 verbatim strings, L4 numeric opacities, M2/M3
  observable behaviours).
- ✅ Scope hygiene is mature and explicit: the container-story pattern is justified
  (epic considered and rejected), the outsized cross-stack F1 was split to 0112,
  and 0097/0102 are deliberately held out with rationale.
- ✅ Upstream prerequisites (0033–0095, tooling 0100/0101/0108/0110) are enumerated
  and marked done, and internal cross-file coupling for L2 is traced explicitly.
- ✅ Frontmatter is complete and valid for a draft story; Context clearly explains
  the milestone-closeout motivation; Assumptions pre-empt the wide-table
  scroll-vs-clip ambiguity.

### Recommended Changes

1. **Restate M1 as discrete, measurable assertions** (addresses: M1 verification
   relies on subjective "visually matches the prototype")
   Replace "visually matches the prototype" with the concrete prototype-derived
   checks already in the Requirements — header row uses `--ac-bg-sunken` with
   uppercase Sora label text; row separators are top borders in `--ac-stroke-soft`;
   the wrapper has `overflow: hidden` so a wide table clips with no horizontal
   scrollbar — so each can be confirmed without a side-by-side judgement call.

2. **Make the M3 and L4 comparative clauses objective** (addresses: M3 "clearly
   less prominent"; L4 "measurably fainter")
   Tie M3's prominence clause to the `--ac-stroke` token vs. the body-text token,
   and reduce L4's "measurably fainter" clause to the already-stated computed
   numbers (≈0.525 net for the META label vs. ~0.75 for other section headings),
   so both reduce to definite computed-style checks.

3. **Add an L2 input-fixture precondition** (addresses: L2 lacks a defined input
   fixture)
   Add "Given a lifecycle page for a work unit that has at least one associated
   decision/ADR" so the positive (still in related-artifacts) and negative (not in
   cluster) checks both operate on a fixture that exercises them.

4. **Capture the Linux VR baseline regeneration in Dependencies** (addresses:
   Linux visual-regression baseline regeneration not captured)
   Note in Dependencies that the six visible UI fixes require a Linux
   visual-regression baseline regeneration via the dedicated CI workflow before
   merge, so the baseline-refresh step is visible as a completion prerequisite.

5. **Gloss F1 on first mention** (addresses: Fix label "F1" referenced before
   introduction)
   Add a one-clause gloss where F1 first appears (e.g. "F1 — the captured-screenshots
   feature, originally the eighth fix, split to 0112") so the orphaned label
   resolves cleanly.

## Per-Lens Results

### Clarity

**Summary**: The work item is exceptionally clear and internally consistent: each
fix is given a stable label (M1–M3, L1–L4), the Summary scope is faithfully
mirrored across Requirements, Acceptance Criteria, and Technical Notes, and
outcomes are stated as concrete, observable states. A few acronyms and domain
tokens appear without definition, and one cross-section fix-count framing has minor
friction, but none rise to true ambiguity for a reader who knows the domain.

**Strengths**:
- Every fix has a stable, unambiguous label (M1–M3, L1–L4) used consistently
  across sections, so referents never drift.
- Summary scope is consistent with Requirements and Acceptance Criteria — no scope
  contradiction between sections.
- Outcomes are stated as concrete observable states (specific opacity values, font
  sizes, token names, exact heading/subheading text) rather than vague properties.
- Scope boundaries are explicitly disambiguated: related items 0097/0112,
  out-of-scope 0102, and the F1 split-out are each called out.
- The Assumptions section pre-empts the wide-table scroll-vs-clip ambiguity.

**Findings**:
- 🔵 **suggestion** (high confidence) — _Context_ — **QA acronym used without
  expansion**: "QA" is used in the Summary and Context without expansion on first
  use. Near-universal, so a stylistic nicety rather than a genuine ambiguity;
  optionally expand on first use.
- 🔵 **suggestion** (medium confidence) — _Technical Notes_ — **Fix label "F1"
  referenced before introduction**: "F1" appears in Technical Notes and Drafting
  Notes but is never listed among the Requirements (M1–M3, L1–L4); a reader has no
  prior referent. Add a one-clause gloss of what F1 was on first mention.
- 🔵 **suggestion** (low confidence) — _Requirements_ — **Design-token names used
  without a definition pointer**: `--ac-*` and `--size-130` tokens are referenced
  as authoritative outcome values without a pointer (in those sections) to where
  the scale is defined; the anchor only appears later in Technical Notes.

### Completeness

**Summary**: An exceptionally complete container story. All structural sections
(Summary, Context, Requirements, Acceptance Criteria, Dependencies, Assumptions,
Technical Notes) are present and densely populated; each of the seven fixes has a
one-to-one Requirement and Acceptance Criterion, the Context explains the
milestone-closeout motivation, and frontmatter is valid for a draft story. No
completeness gaps rise to actionable severity.

**Strengths**:
- Every Requirement (M1–M3, L1–L4) has a matching, specific Acceptance Criterion in
  Given/When/Then form — strong one-to-one structural completeness.
- Context clearly explains the why (a QA pass against the prototype before first
  general release), with the container-story rationale spelled out.
- Frontmatter is complete and valid: kind=story, status=draft, priority=medium,
  plus relates_to and tags populated.
- Dependencies, Assumptions, and Technical Notes are all populated with
  substantive content; deferrals (F1→0112, 0097, 0102) are explicitly recorded.
- The "as a user … I want … so that" framing identifies actor and motivation.

**Findings**:
- 🔵 **suggestion** (medium confidence) — _Open Questions_ — **No Open Questions
  section for contingent decisions**: latent open questions (e.g. the M1
  wide-table scroll-vs-clip choice) are captured under Assumptions rather than
  surfaced as questions; optionally add an Open Questions section or confirm the
  Assumptions resolutions are settled.

### Dependency

**Summary**: Well dependency-mapped overall: upstream prerequisites are stated as
complete, the downstream consumer (general release) is captured as a Blocks entry,
and adjacent items 0097/0102/0112 are each explicitly named and scoped. The main
gaps are a downstream coupling stated only as an abstract milestone and a
cross-environment CI process dependency (Linux VR baseline regeneration) named in
Technical Notes but not surfaced in Dependencies. No uncaptured blocker would
prevent the work from starting.

**Strengths**:
- Upstream prerequisites are enumerated and marked done (0033–0095 plus tooling
  0100/0101/0108/0110), so there is no hidden "cannot start until X" blocker.
- Adjacent/split-out work is precisely scoped: 0097 (related), 0112 (F1 split out),
  0102 (backend, out of scope), each with rationale.
- The downstream consumer (general release) is captured as a Blocks entry rather
  than left implicit.
- Internal cross-file coupling for L2 is traced explicitly in Technical Notes.

**Findings**:
- 🔵 **minor** (medium confidence) — _Technical Notes_ — **Linux VR baseline
  regeneration not captured in Dependencies**: the work requires darwin + linux VR
  baseline regeneration; Linux baselines regenerate via a separate CI workflow, an
  out-of-band coupling not surfaced in Dependencies, risking discovery at PR time.
- 🔵 **suggestion** (medium confidence) — _Dependencies_ — **Downstream consumer is
  an abstract milestone**: "Blocks: General release of the first visualiser
  version" is a prose milestone, not a trackable work item/release ticket, so the
  unblocking relationship cannot be tracked.
- 🔵 **suggestion** (low confidence) — _Requirements_ — **Prototype snapshot not
  framed as a dependency artefact**: M1/L2/L3/L4 encode prototype-derived constants;
  if the prototype drifts or is unavailable, verification is undermined. Largely
  captured via References; optionally note the frozen prototype snapshot as the
  source of truth to diff against.

### Scope

**Summary**: 0111 is an explicitly-declared container story bundling seven small,
QA-discovered frontend fixes across markdown rendering, detail-page layout,
lifecycle, and navigation. The bundling is deliberate and well-reasoned (each fix
too small to justify its own cycle, all frontend-only, all anchored to the same
prototype, shipping together as one milestone-closeout increment). The work item
shows mature scope hygiene — F1 was correctly split to 0112 and 0097/0102 held out
— so the residual bundle is coherent and deliverable as one unit by one team.

**Strengths**:
- Explicitly names and justifies the container-story pattern, anticipating and
  rebutting the scope objection; Drafting Notes record that an epic was considered
  and rejected.
- Demonstrates active scope-boundary management: outsized cross-stack F1 extracted
  to 0112, 0097/0102 excluded, keeping the residual bundle frontend-only.
- All seven fixes share a single unified purpose and authoritative reference,
  giving genuine cohesion rather than a grab-bag.
- Sizing is self-assessed as M with explicit rationale for why it sits above S.

**Findings**:
- 🔵 **suggestion** (medium confidence) — _Requirements: L2_ — **L2 is the one
  logic-bearing fix among cosmetic tweaks**: L2 carries independent multi-file
  behavioural logic (pipeline reclassification, `/8`→`/7`, tests,
  `cluster-via-label.ts`); bundling raises its review/test surface slightly.
  Consider noting L2 as the highest-risk item for focused review/VR attention; no
  structural split warranted.
- 🔵 **suggestion** (low confidence) — _Frontmatter: kind_ — **"story" kind is a
  judgement call**: filed as a story but framed as a container of small QA fixes;
  some teams would file as chore/task. Negligible delivery risk — a housekeeping
  label question.

### Testability

**Summary**: Unusually testable: most criteria (M2, M3, L1, L3, L4) specify
concrete, observable outcomes with exact tokens, pixel values, opacity numbers, and
verbatim text strings that a verifier could check definitively. The weak spots are
the criteria that lean on "visually matches the prototype" (M1) and softer
comparative phrasing ("measurably fainter", "clearly less prominent") without an
objective, self-contained pass/fail procedure.

**Strengths**:
- L3 specifies the exact verbatim text for the eyebrow, H1, and subheading — an
  unambiguous string-equality check.
- L4 gives concrete numeric thresholds (opacity 0.7, additional 0.75, ~0.525 net,
  12.5px vs 13px) admitting a precise computed-style assertion.
- M2 specifies observable behaviours (dark thin scrollbar, no light/OS-default
  scrollbar, horizontal scrolling works, long lines do not wrap).
- M3 names the exact token (`--ac-stroke`) and dimension (1px divider).
- L2 pairs a negative assertion (no decision/ADR nodes) with a positive one
  (decisions still appear in related-artifacts).
- Each criterion is framed Given/When/Then with a stated precondition.

**Findings**:
- 🟡 **major** (high confidence) — _Acceptance Criteria_ — **M1 verification relies
  on subjective "visually matches the prototype"**: the binding verb is a
  subjective visual comparison against an external prototype rather than
  self-contained measurable assertions, and it omits specific tokens stated in the
  Requirements (`--ac-bg-sunken`, `--ac-stroke-soft`, clip/no-horizontal-scroll).
  Restate as discrete checkable assertions mirroring the Requirements.
- 🔵 **minor** (medium confidence) — _Acceptance Criteria_ — **M3 "clearly less
  prominent than body text" is a subjective threshold**: the trailing clause
  introduces a subjective comparator with no defined threshold. Drop it (implied by
  the token assertion) or make it objective against the body-text token.
- 🔵 **minor** (medium confidence) — _Acceptance Criteria_ — **L4 "measurably
  fainter" lacks the measurement procedure**: the summarising clause defines
  neither the measurement nor the reference value. Tie it to the already-stated
  numbers (≈0.525 vs ~0.75).
- 🔵 **minor** (medium confidence) — _Acceptance Criteria_ — **L2 "decisions still
  appear in related-artifacts" lacks a defined input fixture**: without a fixture
  known to contain a decision, the criterion could be vacuously passed. State the
  precondition explicitly ("a work unit that has at least one associated
  decision/ADR").

---

## Re-Review (Pass 2) — 2026-06-15T16:23:39+00:00

**Verdict:** COMMENT

Re-ran all five lenses against the edited work item. The major finding is
resolved and every surface-level finding from pass 1 is addressed. The new
findings this pass are deeper-cut, lower-severity observations — several land in
the non-binding Technical Notes rather than the Requirements/Acceptance Criteria,
and none rises to major or critical. The work item remains acceptable as-is
(COMMENT) and is materially stronger than pass 1.

### Previously Identified Issues

- 🟡 **Testability**: M1 relies on subjective "visually matches the prototype" —
  **Resolved.** M1's acceptance criterion now lists five discrete, measurable
  assertions (a–e) naming the wrapper `border-radius`/`overflow: hidden` + clip
  behaviour, 1px border, `--ac-bg-sunken` header, `--ac-stroke-soft` separators,
  and no striping/hover.
- 🔵 **Dependency**: Linux VR baseline regeneration not in Dependencies —
  **Resolved.** Added as a "Requires" entry framed as an out-of-band completion
  prerequisite.
- 🔵 **Testability**: M3 "clearly less prominent than body text" —
  **Resolved.** Reframed against the `--ac-stroke` token. (A residual "lower-
  contrast" qualifier remains, now flagged only as a low-severity suggestion.)
- 🔵 **Testability**: L4 "measurably fainter" lacks measurement procedure —
  **Resolved.** Now states the ≈`0.525` (0.7×0.75) computed opacity vs ~`0.75`
  for other headings.
- 🔵 **Testability**: L2 lacks a defined input fixture — **Resolved.** Precondition
  now requires "a work unit that has at least one associated decision/ADR".
- 🔵 **Clarity**: F1 referenced before introduction — **Resolved.** Glossed as
  "the captured-screenshots feature, originally the eighth closeout fix".
- 🔵 **Completeness**: No Open Questions section — **Resolved.** Section added with
  an explicit "None outstanding" resolution.
- 🔵 **Dependency**: Prototype snapshot not framed as a dependency — **Resolved.**
  Added a "Source of truth" entry gating M1/L2/L3/L4 on the frozen snapshot.
- 🔵 **Clarity**: QA acronym + token-definition pointer — **Resolved.** "QA
  (quality assurance)" expanded; token pointer added to Requirements.
- 🔵 **Scope**: L2 is the lone logic-bearing fix — **Resolved (documented).**
  Technical Notes now flags L2 as the highest-risk item warranting focused review.
- 🔵 **Dependency**: Abstract "general release" consumer — **Partially resolved.**
  Noted as a prose milestone with an instruction to link a tracked item if one is
  created; no release work item exists yet to reference.
- 🔵 **Scope**: "story" kind judgement call — **Not changed (by decision).** Kept
  `story`; the Drafting Notes already document this as a deliberate choice (epic
  considered and rejected).

### New Issues Introduced

None were *introduced* by the edits; the following are pre-existing latent items
the cleaner pass-1 surface allowed the lenses to surface:

- 🔵 **Testability / Clarity** (minor): M1(c) "faint colour" for the header label
  text is the one M1 sub-assertion left without a named token, while its
  neighbours cite `--ac-bg-sunken`/`--ac-stroke-soft`. Flagged independently by
  both lenses. Easiest remaining tightening: name `--ac-fg-faint` (or `-muted`).
- 🔵 **Clarity** (minor): "the prototype" serves as both the frozen diff target and
  a baseline two fixes intentionally exceed (M2 `scrollbar-color`; L2 "latest
  prototype").
- 🔵 **Clarity** (minor): Technical Notes name both `LIFECYCLE_PIPELINE_STEPS` and
  `WORKFLOW_PIPELINE_STEPS` for the L2 list without stating their relation.
- 🔵 **Clarity** (minor): Size note's 4+1+2 fix split is correct but never states
  the remaining two (M1, L2) explicitly.
- 🔵 **Dependency** (minor): 0102 (backend legacy-linkage removal) touches the same
  lifecycle/linkage area as L2 but is not captured as a Related coupling.
- 🔵 **Dependency** (minor): L2's lockstep multi-file edit (`types.ts` +
  completeness count + tests + `cluster-via-label.ts`) is described as risk but
  not as a sequencing constraint.
- 🔵 **Testability** (minor): L1 "ample space"/own-row intent is broader than the
  verifiable criterion; M2's "line wider than the viewport" trigger is
  viewport-unspecified.
- 🔵 Assorted suggestions: expand ADR on first use; quantify M3's "lower-contrast"
  clause; generic "user" beneficiary; note Darwin→Linux VR baseline ordering.

### Assessment

The work item is ready for implementation. The blocking-quality concern (the M1
major) is gone, and the acceptance criteria are now precise and individually
verifiable. The remaining findings are minor polish — most concentrated in the
Technical Notes (implementer guidance, not the contract) — and can be folded in
opportunistically during planning without another formal review pass.

**Verdict updated to APPROVE** after the highest-value remaining tweak — naming
the M1(c) header-label token as `--ac-fg-faint` (verified against the prototype
at `app.css:925`) — was applied, closing the one cross-lens minor finding. All of
M1's sub-assertions are now token-precise; the residual findings are
non-blocking and may be addressed opportunistically during planning.

---
*Review generated by /accelerator:review-work-item*
