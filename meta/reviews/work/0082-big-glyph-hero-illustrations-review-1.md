---
type: work-item-review
id: "0082-big-glyph-hero-illustrations-review-1"
title: "Work Item Review: BigGlyph Hero Illustration Set"
date: "2026-06-09T18:21:39+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0082"
work_item_id: "0082"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-06-09T18:25:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: BigGlyph Hero Illustration Set

**Verdict:** COMMENT

This is a strong, well-disciplined story. All five lenses found the work item
substantively complete, coherently scoped, and clearly written: the deliverable
(a type-specific `BigGlyph` hero), its single integration point
(`EmptyState.tsx`), and its provenance (the prototype's `big-glyphs.jsx`) are all
named unambiguously, and the Drafting Notes proactively record how earlier
ambiguities and out-of-scope items were resolved. No critical or major findings
were raised; the verdict is COMMENT. The findings cluster around two improvable
seams — the unenumerated "thirteen `DocTypeKey` values" set and the relationship
between the seven-tone palette and the `pr-reviews` diff-tint exception — neither
of which blocks implementation.

### Cross-Cutting Themes

- **The "thirteen `DocTypeKey` values" set is asserted but never enumerated**
  (flagged by: clarity, testability) — Both lenses independently flagged that
  the work item repeatedly requires a hero "for each of the thirteen
  `DocTypeKey` values" yet only names `templates` as an example, pointing at
  `src/api/types.ts` for the full list. A reader cannot confirm set membership
  from the work item alone, and a verifier has no checklist to confirm all
  thirteen are covered (a key silently falling through to `DEFAULT_BIG` would
  pass unnoticed). This is the single most reinforced finding.

- **The seven-tone palette vs. `pr-reviews` diff-tint relationship is
  underspecified** (flagged by: clarity, testability) — Clarity notes it is
  ambiguous whether the green/red diff tints are members of the seven tones or
  extra constants layered on for one doc type; testability notes the expected
  diff-tint values are unspecified, weakening the "no per-doc-type tone is
  hard-coded" check. Same seam, two angles.

### Findings

#### Critical

_None._

#### Major

_None._

#### Minor

- 🔵 **Clarity / Testability**: "Thirteen `DocTypeKey` values" asserted but not enumerated
  **Location**: Requirements / Acceptance Criteria (AC1, AC5)
  The work item requires a bespoke hero for each of thirteen `DocTypeKey`
  values but never lists them (only `templates` is named), pointing only at
  `src/api/types.ts`. A reader cannot verify set membership and a verifier has
  no exhaustive coverage checklist. (Merged: clarity medium-confidence +
  testability high-confidence.)

- 🔵 **Dependency**: 0083 is a downstream consumer of BigGlyph but recorded only as `relates_to`, not `blocks`
  **Location**: Dependencies
  Context, Technical Notes, and Dependencies all describe 0083 as the
  DevDesignSystem page that "showcases BigGlyph" — i.e. 0083 cannot land its
  showcase until this story ships the component — yet 0083 sits under Related /
  `relates_to` while Blocks is "none". When 0082 closes, no Blocks edge signals
  0083 is unblocked. (high confidence)

- 🔵 **Testability**: "distinct, bespoke" illustrations lack a defined pass/fail check
  **Location**: Acceptance Criteria (AC1)
  "Distinct" and "bespoke … traced from the prototype" are quality judgements
  with no defined verification procedure, so AC1 could be argued met regardless
  of fidelity to `big-glyphs.jsx`. (medium confidence)

- 🔵 **Testability**: AC5 does not specify a baseline-approval gate
  **Location**: Acceptance Criteria (AC5)
  A visual-regression spec that merely "captures" snapshots passes trivially on
  first run (any render becomes the baseline). The criterion does not require
  that correct baselines for all 26 hero × theme combinations be established and
  approved. (medium confidence)

- 🔵 **Testability**: Summary's "recognisable / on-brand" intent has no measurable criterion
  **Location**: Summary
  The story's stated reason — heroes that are "recognisable, type-specific" and
  read "on-brand" — has no corresponding AC. A set could satisfy every criterion
  yet fail the intent (technically distinct but not type-recognisable). (medium
  confidence)

- 🔵 **Testability**: `pr-reviews` diff-tint exception has unspecified expected colours
  **Location**: Requirements / Acceptance Criteria (AC2)
  The sanctioned green/red diff tints are exempted from hue derivation but their
  expected values are unstated, so a verifier checking "no per-doc-type tone is
  hard-coded" cannot cleanly distinguish them from a stray hard-coded colour.
  (low confidence)

#### Suggestions

- 🔵 **Clarity**: Seven-tone palette vs. diff-tint relationship is described inconsistently
  **Location**: Requirements
  Sections variously call it a "seven-tone palette" and "seven tones: six
  hue-derived plus a fixed white", then add the `pr-reviews` diff tints as
  further constants — leaving implicit whether `bigPalette` returns exactly
  seven values for every type or seven-plus-extras for `pr-reviews`. (medium
  confidence)

- 🔵 **Completeness**: No explicit Open Questions section
  **Location**: Open Questions
  Prior open questions were resolved and recorded in Drafting Notes instead; an
  empty section is genuinely not applicable here. Optionally add an explicit
  "Open Questions: none (resolved — see Drafting Notes)" line for scanners. (low
  confidence)

- 🔵 **Scope**: Thirteen bespoke illustrations are a sizeable but indivisible payload
  **Location**: Acceptance Criteria
  Tracing thirteen heroes plus `DEFAULT_BIG`, the `bigPalette` helper, and the
  integration is substantial, but the illustrations share one component, one
  palette mechanism, and one integration point and have no standalone value.
  Flagged only to confirm sizing was considered — no decomposition warranted.
  (low confidence)

### Strengths

- ✅ `BigGlyph` and the small `Glyph` (0037) are consistently distinguished
  throughout — including Drafting Notes that flag where earlier drafts conflated
  them — so the two component scales are never confused.
- ✅ The integration point is named unambiguously (`EmptyState.tsx`) and mapped
  to the prototype's `LibraryIndexEmpty`, removing guesswork about which surface
  changes; the change is reduced to a single concrete element swap.
- ✅ Every piece of jargon (`bigPalette`, `DEFAULT_BIG`, `DocTypeKey`,
  `PaperFold`) is anchored to a concrete source location.
- ✅ Both upstream blockers (0073, 0074) are named with their `done` status, and
  the existing hue plumbing (`copy.hue` → `--ac-empty-page-hue`) is correctly
  classified as an available prerequisite rather than an open blocker; no
  external or cross-team coupling exists.
- ✅ Scope is actively pruned — the landing empty-card variant and the dev
  design-system showcase (deferred to 0083) are explicitly removed — and
  Summary, Requirements, and Acceptance Criteria describe the same unit of work.
- ✅ Five concrete Acceptance Criteria cover component behaviour, palette
  derivation, integration point and size, accessibility, and visual-regression
  coverage; AC3/AC4 are fully observable and AC5 names a concrete mechanism.
- ✅ Drafting Notes record how prior open questions (shape provenance, palette
  accessibility, landing-card scope) were investigated and resolved, leaving no
  dangling unknowns.

### Recommended Changes

1. **Enumerate (or authoritatively cite) the thirteen `DocTypeKey` values**
   (addresses: "Thirteen `DocTypeKey` values asserted but not enumerated")
   List the thirteen keys once in Context or Requirements, or state explicitly
   that the prototype's `big-glyphs.jsx` table (reconciled against
   `src/api/types.ts`) is the authoritative complete set the heroes and the AC5
   snapshots must cover. This resolves the strongest cross-cutting finding and
   gives both reader and verifier a coverage checklist.

2. **Add 0083 as a `blocks` edge**
   (addresses: "0083 is a downstream consumer recorded only as relates_to")
   Move 0083 from Related into Blocks (and add a `blocks` frontmatter edge),
   since it consumes the `BigGlyph` component this story delivers; keep
   0037/0041 as Related (peer / owner, not consumer).

3. **Tighten the palette / diff-tint specification**
   (addresses: clarity palette-inconsistency suggestion; testability
   `pr-reviews` diff-tint finding)
   State explicitly whether the green/red `pr-reviews` diff tints are members of
   the seven tones or separate structural constants used only by that
   illustration, and note their expected values (or that they match the
   prototype's `pr-reviews` tints), so the "no per-doc-type tone is hard-coded"
   check has a definite reference.

4. **Strengthen AC1 and AC5 verification framing (optional)**
   (addresses: "distinct/bespoke lacks a check"; "AC5 baseline-approval gate";
   "recognisable/on-brand has no criterion")
   Replace the subjective "distinct" with the AC5 per-doc-type snapshots as the
   distinctness check; clarify AC5 is met only when correct, approved baselines
   exist for all 26 hero × theme combinations; and note that design sign-off on
   those baselines operationalises the Summary's "recognisable / on-brand"
   intent. These sharpen verification but do not block implementation.

## Per-Lens Results

### Clarity

**Summary**: This work item communicates its intent with strong clarity: it
consistently distinguishes BigGlyph from the small Glyph, names the integration
point (EmptyState.tsx) unambiguously, and the Drafting Notes proactively resolve
terminology that earlier drafts got wrong. The single recurring ambiguity is the
count and identity of the doc types being illustrated, where "thirteen" is
asserted alongside a parenthetical that lists only one example. Domain terms are
anchored to a named prototype source file, so jargon is well-handled overall.

**Strengths**:
- BigGlyph and the small Glyph (0037) are consistently and explicitly
  distinguished throughout, including in Drafting Notes that flag where earlier
  drafts conflated them.
- The integration point is named unambiguously (EmptyState.tsx) and Context maps
  it to the prototype's LibraryIndexEmpty.
- Domain-specific identifiers (bigPalette, DEFAULT_BIG, DocTypeKey, PaperFold)
  are each tied to a concrete source location.
- Scope boundaries are stated affirmatively and negatively — the dev
  design-system page is explicitly assigned to 0083 and called out as out of
  scope.

**Findings**:
- 🔵 minor (medium confidence) — **Requirements**: The work item repeatedly
  asserts "thirteen" doc types and qualifies the count with a single
  parenthetical example, but never enumerates the full set or links to where the
  thirteen `DocTypeKey` values are listed beyond a file path. AC1 cannot be
  checked against an explicit list. Suggestion: enumerate the thirteen values
  once, or state that `big-glyphs.jsx` is the authoritative source of the set.
- 🔵 suggestion (medium confidence) — **Requirements**: The seven-tone palette
  is described inconsistently — "seven-tone palette" vs. "seven tones: six
  hue-derived plus a fixed white" vs. the green/red `pr-reviews` diff tints as
  further constants — leaving implicit whether `bigPalette` returns exactly seven
  values for every type or seven-plus-extras for `pr-reviews`. Suggestion: state
  explicitly whether the diff tints are members of the seven tones or separate
  constants used only by `pr-reviews`.

### Completeness

**Summary**: This is a highly complete story. Frontmatter is fully populated
with a recognised kind (story) and valid status; every expected section is
present and substantively filled. The story identifies its user, explains why
the work is wanted, and defines done via five specific criteria. No structural
gaps warrant a critical or major finding.

**Strengths**:
- Summary is a clear user-story statement naming the actor, the want, and the
  deliverable.
- Context thoroughly explains the motivating gap (prototype ships per-doc-type
  heroes; current app has only a generic PaperFold hero).
- Five concrete Acceptance Criteria cover component behaviour, palette
  derivation, integration point, accessibility, and visual-regression coverage.
- Frontmatter is complete and correct (kind, status, priority, blocked_by /
  relates_to / source edges all populated).
- Drafting Notes capture how prior open questions were resolved, leaving no
  dangling unknowns.

**Findings**:
- 🔵 suggestion (low confidence) — **Open Questions**: The story has no Open
  Questions section; Drafting Notes instead record that prior open questions
  were investigated and resolved. This is an acceptable substitute and an empty
  section is genuinely not applicable. Suggestion: optionally add an explicit
  "Open Questions: none (see Drafting Notes)" line for scanners.

### Dependency

**Summary**: Dependency capture is strong: both upstream blockers (0073, 0074)
are named with their done status, the existing in-app integration points are
documented as already-available rather than as blockers, and there are no
external systems or cross-team actions implied. The one gap is a downstream
consumer — 0083 (DevDesignSystem) is described as showcasing BigGlyph yet is
recorded only as relates_to, not as a Blocks edge.

**Strengths**:
- Both upstream blockers (0073 brand palette tokens, 0074 per-doc-type hues) are
  explicitly named with done status, and Drafting Notes justify keeping them as
  blocked_by edges.
- The integration coupling to EmptyState.tsx is captured precisely — the numeric
  copy.hue / --ac-empty-page-hue plumbing already exists, correctly classified as
  an available prerequisite.
- External and cross-team coupling is genuinely absent — the work is
  self-contained within the frontend, drawing shapes from an in-repo prototype.
- The scope boundary with 0083 is stated explicitly in both Technical Notes and
  Requirements.

**Findings**:
- 🔵 minor (high confidence) — **Dependencies**: 0083 is described as the
  DevDesignSystem page that "showcases BigGlyph" — it cannot land its showcase
  until this story ships the component — yet 0083 is listed only under Related /
  relates_to while Blocks is "none". A genuine downstream coupling is invisible:
  when 0082 closes, no Blocks edge signals 0083 is unblocked. Suggestion: add
  0083 as a Blocks entry (and a blocks frontmatter edge); keep 0037/0041 as
  Related.

### Scope

**Summary**: This is a well-scoped, coherent story: every requirement serves the
single deliverable of a type-specific BigGlyph hero illustration wired into the
per-type empty state. The author has actively pruned scope (removing the landing
card and the dev design-system showcase, deferred to 0083), and the Summary,
Requirements, and Acceptance Criteria all describe the same unit of work. The
`story` kind is appropriate for the bounded, single-team, single-surface
increment described.

**Strengths**:
- Single unified purpose: component implementation, palette derivation, and
  EmptyState integration all serve one deliverable, with no "and also" bundling.
- Explicit out-of-scope boundaries: the landing empty-card variant is removed and
  the dev design-system showcase is deferred to 0083.
- Summary, Requirements, and Acceptance Criteria describe the same scope
  consistently — AC bullets map one-to-one onto the requirements.
- Single surface and single team: one integration point (EmptyState.tsx) within
  the frontend, no cross-service or cross-ownership boundaries.

**Findings**:
- 🔵 suggestion (low confidence) — **Acceptance Criteria**: The story requires
  hand-tracing thirteen bespoke SVG illustrations plus a DEFAULT_BIG fallback,
  the bigPalette helper, and the EmptyState integration — substantial for one
  story, though the illustrations share one component, one palette mechanism, and
  one integration point and ship as a set with no standalone value. Suggestion:
  keep as one story — the illustrations are genuinely indivisible; flagged only
  to confirm sizing was considered.

### Testability

**Summary**: This story's Acceptance Criteria are mostly concrete and verifiable:
each references named artefacts, specific pixel sizes (96px), a defined doc-type
set, and a named verification mechanism (Playwright visual-regression). The main
gaps are the unbounded "thirteen" set lacking an enumeration to check against,
the subjective "distinct/bespoke" quality with no defined comparison procedure,
and the Summary's "recognisable, on-brand" intent not being captured in any
measurable criterion.

**Strengths**:
- AC3 and AC4 are fully testable: 96px size, the PaperFold-to-BigGlyph swap, the
  aria-hidden attribute, and the retained title/lede/footer copy are all directly
  observable.
- AC5 names a concrete verification mechanism (Playwright visual-regression
  across light and dark themes).
- Requirements pin the verification inputs precisely — the hue source (copy.hue →
  --ac-empty-page-hue, numeric 0–360) and the palette derivation.

**Findings**:
- 🔵 minor (high confidence) — **Acceptance Criteria**: "thirteen DocTypeKey
  values" is not enumerated, so coverage cannot be checked exhaustively. A
  missing or mis-typed key (silently falling through to DEFAULT_BIG) could pass
  review. Suggestion: list the thirteen values, or reference src/api/types.ts as
  the canonical enumeration to reconcile against.
- 🔵 minor (medium confidence) — **Acceptance Criteria**: "distinct, bespoke"
  illustrations lack a defined pass/fail check; "distinct" and "traced" are
  quality judgements with no procedure. Suggestion: state the verification basis
  (SVG path equivalence to big-glyphs.jsx at 80×80) and rely on AC5 baselines as
  the distinctness check.
- 🔵 minor (medium confidence) — **Summary**: the "recognisable / on-brand"
  intent is not captured in any measurable criterion; a set could satisfy every
  AC yet fail the intent. Suggestion: note that AC5 baselines (with design
  sign-off) operationalise "on-brand", or add an approval criterion.
- 🔵 minor (medium confidence) — **Acceptance Criteria**: AC5 does not specify a
  baseline-approval gate, so "captures heroes" is ambiguous — a spec that records
  whatever rendered passes trivially. Suggestion: clarify the criterion is met
  only when correct baselines exist for all 26 hero × theme combinations and the
  spec passes against them.
- 🔵 minor (low confidence) — **Requirements**: the pr-reviews diff-tint
  exception is verifiable but its expected colours are unspecified, so a verifier
  cannot distinguish the sanctioned tints from a stray hard-coded colour.
  Suggestion: note the expected diff-tint colours (or that they match the
  prototype's pr-reviews tints).

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-09

**Verdict:** APPROVE

Re-ran the three lenses whose findings were addressed (clarity, dependency,
testability). All five originally-flagged findings are resolved. The work item
is now in strong shape; the residual items are polish-level (minor/suggestion),
and all of them have since been edited away in this pass.

### Previously Identified Issues

- 🔵 **Clarity / Testability**: "Thirteen `DocTypeKey` values" not enumerated —
  **Resolved.** Requirements now enumerate all thirteen keys and cite
  `DOC_TYPE_KEYS` in `src/api/types.ts` as authoritative; AC1 references that set.
- 🔵 **Dependency**: 0083 recorded only as `relates_to`, not `blocks` —
  **Resolved.** 0083 moved to a `blocks` frontmatter edge and the Dependencies
  "Blocks" entry, with rationale; no dependency findings remain this pass.
- 🔵 **Testability**: "distinct, bespoke" lacks a pass/fail check — **Resolved.**
  AC1 now anchors distinctness to the snapshot baselines and traces fidelity to
  the prototype rather than relying on subjective judgement.
- 🔵 **Testability**: AC5 had no baseline-approval gate — **Resolved.** AC5 now
  requires design-approved baselines for all 26 hero × theme combinations and a
  passing spec, not merely captured snapshots.
- 🔵 **Testability**: "recognisable / on-brand" had no measurable criterion —
  **Resolved.** AC5 names baseline sign-off as what operationalises the intent.
- 🔵 **Clarity / Testability**: `pr-reviews` diff-tint relationship/values —
  **Resolved.** Requirements and AC2 now state the diff tints are not palette
  members and must equal the prototype's exact diff-tint constants.

### New Issues Introduced

All four were surfaced this pass and then fixed in the same pass:

- 🔵 **Clarity** (suggestion): "AC5" positional label pointed at an unnumbered
  criterion — **Fixed** (replaced with "final criterion below").
- 🔵 **Clarity** (suggestion): "the table" referred to an inline list — **Fixed**
  (changed to "the thirteen-key set").
- 🔵 **Testability** (minor): "no per-doc-type tone is hard-coded" lacked a stated
  verification procedure — **Fixed** (AC2 now specifies unit-test on `bigPalette`
  HSL hue + code-review check for stray literals).
- 🔵 **Testability** (minor): "visually equivalent" / "matching the prototype"
  lacked a comparison procedure — **Fixed** (AC1 specifies baseline sign-off
  against side-by-side prototype renders; AC2 requires exact prototype diff-tint
  constants).

### Assessment

The work item is ready for implementation. Every finding from the initial review
and every new item raised in this pass has been resolved in the work item text,
with no critical or major findings at any point and no outstanding blocker. The
verdict is therefore promoted to **APPROVE** for this pass.
