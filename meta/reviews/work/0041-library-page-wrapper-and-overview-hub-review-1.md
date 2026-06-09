---
date: "2026-05-15T16:25:27+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0041"
work_item_id: "0041"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
id: "0041-library-page-wrapper-and-overview-hub-review-1"
title: "0041-library-page-wrapper-and-overview-hub-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-15T16:25:27+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: Library Page Wrapper, Overview Hub, and List Views

**Verdict:** REVISE

The work item is structurally well-formed and substantively populated — every required section is present, the Drafting Notes explicitly flag the size and the split-vs-keep call, and most Acceptance Criteria use Given/When/Then framing. The strongest theme across lenses is that the work item bundles two qualitatively different deliverables (server contract change + six frontend sub-features) without fully resolving how the server side is owned, sequenced, or verified, and several Requirements (mtime format, long-facet threshold, zero-doc overview behaviour) drift away from their corresponding Acceptance Criteria. Sharpening recent edits already pinned several thresholds, but they created new Requirements↔AC inconsistencies that need reconciling.

### Cross-Cutting Themes

- **Server contract is a hard prerequisite that is treated as "likely related"** (flagged by: dependency, scope, testability) — Requirements and AC assume a server-driven phase/facet shape, but the server work has no concrete blocker reference, no committed contract, and testability flags three majors asking how "server-driven" is verified from outside.
- **Story bundles independently shippable pieces** (flagged by: scope, dependency, completeness) — six sub-features plus a server endpoint plus an implicit Sidebar migration; ordering between Page wrapper, popover primitive, and the dependent UI is unstated; KanbanBoard migration is left undecided.
- **Requirements and Acceptance Criteria drift apart in places** (flagged by: clarity, testability, completeness) — mtime format (`1mo ago` in Requirements vs `<n>w ago` fallback in AC), long-facet threshold (vague in Requirements, ">8" in AC), zero-document overview card behaviour (in Requirements as recommendation, absent from AC), filter-applied empty state (in Open Questions only).
- **Implementation-property ACs lack observable verification procedures** (flagged by: testability ×3) — "server-provided phase structure", "server-driven facets", "renders correctly in both themes" describe properties of the codebase rather than observable behaviours a verifier can pass/fail.

### Findings

#### Major

- 🟡 **Dependency**: Server endpoint for phase/doc-type metadata is a hard prerequisite but only listed as 'likely related'
  **Location**: Dependencies
  Requirements/AC mandate server-driven phase structure and filter facets but the server contract is demoted to "likely related — may warrant a separate work item." Either commit it as a Blocked-by entry (with a concrete work-item id, splitting it out) or state explicitly that the server work is in-scope and remove the soft framing.

- 🟡 **Scope**: Story bundles six features plus a server contract change
  **Location**: Requirements
  Six independently shippable deliverables under one work item (Page wrapper, overview hub, list-view chrome, sort pill, filter pill, empty states) plus server-side phase/facet metadata. Author already flagged the size as L and considered splitting. Suggest splitting along the existing Requirements headings (e.g., wrapper + hub + endpoint / list chrome + sort / filter + facets / empty states).

- 🟡 **Dependency**: Uncaptured internal ordering between Page wrapper and dependent sub-features
  **Location**: Requirements / Dependencies
  Page wrapper is structurally prerequisite to the overview hub, list-view chrome, and empty-state card; the popover primitive is prerequisite to sort and filter pills. Neither ordering nor the popover primitive itself appears in Dependencies, so parallel work risks rebasing against an unstable wrapper API (actions-slot still an Open Question).

- 🟡 **Testability**: "Renders correctly in both light and dark themes" has no measurable outcome
  **Location**: Acceptance Criteria — Overview hub
  Subjective oracle. Anchor to the named screenshots already in References (`library-view-updated-light.png`, `library-view-updated-dark.png`) for backgrounds, card surfaces, eyebrow/H1/subtitle text colours, and Glyph colours.

- 🟡 **Testability**: Server-driven phase structure criterion has no defined verification procedure
  **Location**: Acceptance Criteria — Overview hub
  Describes an implementation property, not an observable outcome. Reframe with an input/output pair: "Given server returns phases [A: [t1], B: [t2,t3]], the page renders two groupings A then B with named cards."

- 🟡 **Testability**: Server-driven facets criterion is unverifiable as written
  **Location**: Acceptance Criteria — Filter
  Same shape as the phase-structure issue. Replace with behavioural pairs: "Given doc type metadata declares [STATUS, CLUSTER SLUG], the filter menu shows exactly those two facet sections in that order."

- 🟡 **Testability**: Filter count and OR/AND semantics lack input specifications
  **Location**: Acceptance Criteria — Filter
  Counts have no defined scope (pre-filter vs post-other-facet-filter vs post-own-filter); OR/AND semantics need at least one worked example pairing selections to expected rows.

- 🟡 **Clarity**: Long-facet threshold contradicts Requirements wording
  **Location**: Acceptance Criteria — Filter
  Requirements say "when a facet's option list is long"; AC now says "more than 8 options" after the sharpen pass. Reconcile by moving "> 8 options" into Requirements (or stating it as an implementation decision in Technical Notes).

- 🟡 **Clarity**: Modified-timestamp format differs between Requirements and Acceptance Criteria
  **Location**: Requirements — List view chrome / Acceptance Criteria — List view + sort
  Requirements use `2m ago`, `1h ago`, `1mo ago` (months unit); AC uses `<n>s/m/h/d/w ago` with a "locale date for values older than one week" fallback. Pick one canonical format and replicate (or cross-reference).

#### Minor

- 🔵 **Clarity**: Inconsistent rendering of the doc-type path heading
  **Location**: Requirements / Acceptance Criteria — Empty / zero states
  Requirements use `meta/notes/`, AC uses `meta/{type}/`. State once that `{type}` is replaced with the doc-type slug, then keep the placeholder form throughout.

- 🔵 **Clarity**: 'doc count' position ambiguous relative to other card elements
  **Location**: Requirements — Library overview hub
  Add a short layout sentence, or make the screenshot reference normative.

- 🔵 **Clarity**: Empty-state copy example uses an undefined term ('hallway captures')
  **Location**: Assumptions
  Project-specific jargon. Either gloss in parentheses or scope it as "example copy for the `notes` doc type".

- 🔵 **Clarity**: `PR descriptions` vs `PRs` referent is genuinely ambiguous
  **Location**: Open Questions
  Resolve before implementation, or explicitly state implementer should treat whatever the server returns as authoritative.

- 🔵 **Clarity**: Pill display copy not consistent with AC
  **Location**: Requirements — Sort pill and menu
  Requirements show `↕ Recently modified ⌄` with glyphs; AC only requires the pill label update. Note that glyphs are illustrative (defer to 0037) or add an AC asserting the glyph affordances.

- 🔵 **Completeness**: Zero-document overview card behaviour described only as a recommendation
  **Location**: Requirements > Library overview hub / Acceptance Criteria > Overview hub
  Once Open Question #3 is resolved, promote the agreed behaviour into a dedicated AC.

- 🔵 **Completeness**: Filter-applied empty state has no requirement or acceptance coverage
  **Location**: Requirements > Filter pill and menu / Acceptance Criteria > Filter
  After answering the open question, add a Requirement + AC, or mark explicitly out of scope.

- 🔵 **Dependency**: Sidebar migration to server-driven phase structure is a coupled change not captured as a dependency
  **Location**: Technical Notes / Dependencies
  Sidebar.tsx PHASE_DOC_TYPES migration must happen in lockstep. Either add as an in-scope deliverable / AC, or capture as a Blocks entry.

- 🔵 **Dependency**: PageSubtitle / KanbanBoard coupling left undecided in dependency terms
  **Location**: Technical Notes / Dependencies
  Decide explicitly whether KanbanBoard migrates to `Page` in this story or in a follow-up; either add an AC or add a Blocks entry.

- 🔵 **Dependency**: Router test files are coupled changes not surfaced in Dependencies
  **Location**: Requirements / Dependencies
  `router.test.tsx` and `router-with-crumb.test.ts` are coupled changes; either add an AC for the test updates or surface in Dependencies.

- 🔵 **Scope**: Server contract acknowledged as possible separate work item
  **Location**: Open Questions
  Resolve before sprint acceptance: commit it here, or extract as a blocking dependency work item.

- 🔵 **Scope**: Generic Page wrapper has reuse beyond library implied but not scoped
  **Location**: Requirements
  Explicitly state whether KanbanBoard is migrated in this story or deferred, and reflect that in AC or an out-of-scope note.

- 🔵 **Testability**: "Enforces consistent" max width/padding/spacing has no numeric threshold
  **Location**: Acceptance Criteria → Page wrapper
  Anchor to specific tokens from 0033, or state that consuming screens render with identical computed values for those properties.

- 🔵 **Testability**: Doc-type-specific description sentence is referenced but not verifiable
  **Location**: Acceptance Criteria → Empty / zero states
  Either provide an inline table of doc-type → description sentence, or defer until Open Question #5 is resolved and reference the resolved source.

- 🔵 **Testability**: ID monospace pill format and STATUS chip variants are not enumerated
  **Location**: Acceptance Criteria → List view + sort
  Reference `statusToChipVariant` as the canonical mapping, and pin ID format (e.g., `PROJ-0001`).

- 🔵 **Testability**: Facet section scrollable max-height is approximate
  **Location**: Acceptance Criteria → Filter
  Replace "roughly 8 option rows" with an exact pixel value or precise rule (e.g., 8 × option-row-height).

- 🔵 **Testability**: Zero-document overview card behaviour absent from criteria
  **Location**: Acceptance Criteria → Overview hub
  Add a criterion after Open Question #3 is resolved.

- 🔵 **Testability**: Sort ordering not specified for ties or for the 'latest' overview preview
  **Location**: Acceptance Criteria → List view + sort
  Add a tie-breaker rule (e.g., secondary sort by ID ascending).

#### Suggestions

- 🔵 **Completeness**: Parent field is empty despite multiple related work items
  **Location**: Frontmatter: parent
  Confirm whether a parent epic exists for the library redesign track; if so, populate `parent`.

### Strengths

- ✅ All standard Story sections present and substantively populated (Summary, Context, Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes, References).
- ✅ Acceptance Criteria mirror the Requirements sub-feature groupings, making coverage auditable.
- ✅ Most ACs use Given/When/Then with named inputs and observable outputs.
- ✅ Concrete thresholds are pinned where it matters (breakpoints 640px/1024px, default sort selection, exact empty-state footer copy, sort option list enumerated).
- ✅ "Modified" semantics explicitly defined in Sort requirements rather than left implicit.
- ✅ Canonical vs superseded screenshots disambiguated.
- ✅ Filter combination semantics (OR within facet, AND across facets) stated explicitly.
- ✅ Open Questions and Drafting Notes capture areas needing confirmation rather than leaving them implicit.
- ✅ Upstream blockers (0033, 0037, 0038) confirmed merged in Technical Notes.
- ✅ Removal of column-header sort is stated as a negative criterion — directly testable.
- ✅ Drafting Notes proactively raises the split-vs-keep decision and invites review feedback.

### Recommended Changes

Ordered by impact:

1. **Resolve the server-contract scope question** (addresses: Server endpoint demoted to 'likely related'; Server contract acknowledged as possible separate work item; three testability server-driven ACs)
   Pick one of: (a) commit the server work to this story (add `/api/library/structure` shape as a Requirement, list endpoint changes as ACs), or (b) extract it as a new blocking dependency work item (add Blocked-by entry with concrete id). Either way, rewrite the three "server-driven" ACs as observable Given/When/Then pairs that name a sample server response and assert what renders.

2. **Make a split-or-stay decision and justify it** (addresses: Story bundles six features; Uncaptured internal ordering; PageSubtitle/KanbanBoard coupling; Sidebar migration coupling)
   Either split into the seams already visible (wrapper + hub + endpoint / list chrome + sort / filter + facets / empty states), or strengthen the Drafting Notes justification beyond "every piece consumes the same Page wrapper" and add an explicit ordering note + popover-primitive dependency + Sidebar/KanbanBoard scope decisions to Dependencies and AC.

3. **Reconcile Requirements ↔ AC drift** (addresses: Long-facet threshold; Mtime format; Zero-doc overview card; Filter-applied empty state; ID format; doc-type path heading)
   Pass through Requirements and Acceptance Criteria sections together: (a) move "> 8 options" into Requirements; (b) pick one canonical mtime format wording and replicate; (c) once Open Questions #3 and the filter-applied empty state are resolved, promote to Requirements + matching ACs (or mark out of scope); (d) state once that `{type}` is replaced with the doc-type slug then keep placeholder form.

4. **Replace subjective AC oracles with anchored ones** (addresses: 'Renders correctly in both themes'; 'Enforces consistent' Page wrapper)
   Anchor "correctly" to the named reference screenshots; anchor "consistent" to specific design tokens (e.g., `--size-content-max`, `--space-page-x`) or to identical computed values across consumers.

5. **Specify filter-count scope and tie-breakers** (addresses: Filter count/OR-AND semantics; Sort ordering tie-breakers)
   State whether option counts are pre-filter / post-other-facet-filter / post-own-filter; add a worked example for OR-within-facet + AND-across-facets; add a tie-breaker rule (e.g., ID ascending on mtime tie).

6. **Resolve remaining clarity nits** (addresses: 'hallway captures' jargon; 'PR descriptions' vs 'PRs'; doc count position; sort pill glyph affordances)
   Inline glosses, normative screenshot references, or explicit "defer to server-returned shape" statements as the open questions are resolved.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Generally well-written with strong cross-referencing, explicit terminology, and clear screenshots-to-requirements mapping. A few ambiguities around empty-state copy sourcing, inconsistent "Modified" fallback semantics between Requirements and ACs, and shifting use of `meta/{type}/` as both literal path and styled rendering token.

**Strengths**:
- Defines 'Modified' semantics explicitly
- Disambiguates canonical from superseded design
- Calls out filter combination semantics (OR within facet, AND across facets)
- Ties each requirement to a specific screenshot
- Open Questions section explicitly flags uncertain intent

**Findings**: 2 major (long-facet threshold contradiction, mtime format mismatch), 5 minor (path heading rendering, card layout ambiguity, hallway captures jargon, PR descriptions referent, sort pill glyph copy).

### Completeness

**Summary**: Highly complete for a Story of this scope. Every standard section present and substantively populated, with rich Requirements broken down by sub-feature and comprehensive ACs mirroring those groupings.

**Strengths**:
- All Story sections present with substantive content
- ACs extensive and organised under same sub-feature groupings as Requirements
- Frontmatter well-formed with recognised values
- Context clearly explains motivation
- Open Questions and Drafting Notes explicitly capture unresolved areas

**Findings**: 2 minor (zero-doc overview behaviour only a recommendation, filter-applied empty state no AC), 1 suggestion (parent field empty).

### Dependency

**Summary**: Captures primary frontend blockers (0033, 0037, 0038) and one downstream consumer (0042), with server-endpoint flagged as both open question and 'likely related'. However, the server-side metadata endpoint is a hard prerequisite yet not promoted to a formal blocker, and ordering constraints between bundled pieces are uncaptured.

**Strengths**:
- Dependencies section names upstream blockers and confirms merged status
- Downstream consumer 0042 captured as Blocks entry
- Server-side coupling at least surfaced rather than left implicit
- Technical Notes call out specific cross-cutting consumers (Sidebar migration)

**Findings**: 2 major (server endpoint as 'likely related', uncaptured Page-wrapper ordering), 3 minor (Sidebar migration coupling, PageSubtitle/KanbanBoard coupling, router test files).

### Scope

**Summary**: Bundles six distinct deliverables plus a server-side endpoint change. Author already considered splitting and noted it as a Drafting Note for review. Pieces share a common Page wrapper and metadata dependency, but several components are independently deliverable. Sizing is borderline; bundling is justified but warrants explicit reviewer confirmation.

**Strengths**:
- Drafting Notes explicitly raises split-vs-keep decision
- Requirements genuinely connect through shared Page wrapper and server-driven metadata
- Summary, Requirements, and ACs describe the same scope consistently
- Dependencies and Blocks are explicit

**Findings**: 1 major (story bundles six features + server contract), 2 minor (server contract acknowledged as possible separate WI, generic Page wrapper reuse beyond library implied but not scoped).

### Testability

**Summary**: Most ACs use Given/When/Then framing with concrete UI elements and exact copy/thresholds. Several ACs, however, contain subjective or unspecified outcomes (light/dark 'correctly', server-driven facets without verification procedure, filter counts without input specification) that would leave a verifier guessing. A few testable thresholds appear in Requirements but not in ACs.

**Strengths**:
- Most criteria use explicit Given/When/Then framing
- Concrete thresholds pinned (640px/1024px breakpoints, >8 options, default sort, exact empty-state footer)
- Sort options enumerated; relative-timestamp buckets spelled out with fallback rule
- Removal of column-header sort stated as a negative criterion

**Findings**: 4 major (light/dark 'correctly', server-driven phase verification, server-driven facets verification, filter count/OR-AND semantics), 6 minor (Page wrapper 'consistent', doc-type description sentence, ID/STATUS chip variants, facet scrollable height approximate, zero-doc overview absent from ACs, sort tie-breakers).

## Re-Review (Pass 2) — 2026-05-15T16:20:28+00:00

**Verdict:** REVISE

Substantial improvement: 9 major findings → 4 major findings; 18 minor → 14 minor. All five cross-cutting themes from pass 1 (server contract under-specified, scope bundling, Requirements↔AC drift, subjective oracles) are materially addressed. The four remaining major findings are narrower and largely judgement calls or deliberate trade-offs: classifying a deliberately-bundled work item as story vs epic, anchoring visual conformance via screenshots vs enumerated tokens, the breakpoint hedge that was added during sharpen, and one missing post-condition AC for the `Clear filters` button. Completeness lens has zero findings on this pass.

### Previously Identified Issues

#### Critical / Major from Pass 1

- 🟡 **Dependency**: Server endpoint listed only as 'likely related' — **Resolved** (server contract now in-scope; full endpoint shape in Requirements + 5 ACs).
- 🟡 **Scope**: Bundles six features plus server contract — **Partially resolved** (decision committed, Drafting Notes strengthened with three concrete reasons; scope lens now raises a softer "epic classification?" finding).
- 🟡 **Dependency**: Uncaptured ordering between Page wrapper and dependent sub-features — **Resolved** (internal ordering note in Dependencies; popover primitive surfaced as prerequisite).
- 🟡 **Testability**: "Renders correctly in both themes" — **Partially resolved** (now anchored to named screenshots; testability lens flags that screenshot-match still lacks a numeric oracle / tolerance).
- 🟡 **Testability**: Server-driven phase structure AC — **Resolved** (rewritten as observable G/W/T with sample server response).
- 🟡 **Testability**: Server-driven facets AC — **Resolved** (rewritten as G/W/T with two contrasting server responses).
- 🟡 **Testability**: Filter count + OR/AND semantics — **Resolved** (count scope pinned to post-other-facet-filter; worked example added).
- 🟡 **Clarity**: Long-facet threshold contradicts Requirements wording — **Resolved** (">8 options" now in both Requirements and AC).
- 🟡 **Clarity**: Mtime format differs between Requirements and AC — **Resolved** (Requirements aligned to `formatMtime` wording).

#### Minor from Pass 1 (selected)

- 🔵 Path heading `meta/{type}/` notation — **Resolved** (notation pinned).
- 🔵 'doc count' card position ambiguous — **Resolved** (explicit layout sentence; screenshot ref normative).
- 🔵 'hallway captures' jargon — **Resolved** (example replaced).
- 🔵 `PR descriptions` vs `PRs` — **Resolved** (PR descriptions canonical everywhere).
- 🔵 Sort pill glyph copy — **Resolved** (illustrative; defers to 0037).
- 🔵 Zero-document overview behaviour — **Resolved** (promoted to Requirement + AC).
- 🔵 Filter-applied empty state — **Resolved** (panel with Clear filters action; Requirement + AC).
- 🔵 Sidebar migration coupling — **Resolved** (in-scope as AC).
- 🔵 PageSubtitle/KanbanBoard coupling — **Resolved** (shim + Blocks follow-up).
- 🔵 Router test files coupling — **Resolved** (in AC).
- 🔵 Page wrapper "consistent" — **Resolved** (tokens enumerated; testability lens now wants the measurement DOM node pinned).
- 🔵 ID/STATUS chip variants — **Resolved** (canonical mapping referenced; ID format pinned).
- 🔵 Facet scrollable max-height approximate — **Resolved** (exact 8-row rule).
- 🔵 Sort tie-breakers — **Resolved** (ID ascending secondary sort).

### New Issues Introduced

#### Major (Pass 2)

- 🟡 **Scope**: Story scope plausibly warrants epic classification
  **Location**: Frontmatter: type / Drafting Notes
  Confidence: medium. Soft restatement of the pass-1 scope finding — author has explicitly chosen story-with-justification, so this is partly a normative call about team conventions.

- 🟡 **Testability**: Theme-match criterion relies on screenshot comparison without defined tolerance
  **Location**: AC — Overview hub
  Screenshot anchoring resolved the "subjective" framing but introduced a verification-procedure question: there is no pixel-diff threshold or token-by-token assertion list. Anchoring to specific tokens (e.g., `--ac-surface-1`, `--ac-text-muted`, `--ac-glyph-color-{doc-type}`) would harden this.

- 🟡 **Testability**: Breakpoint criterion hedged by "subject to design-token alignment"
  **Location**: AC — Overview hub
  The "subject to design-token alignment with 0033 — confirm during implementation" hedge means the criterion can be argued passed against multiple incompatible breakpoint sets. Pin the breakpoints to specific 0033 tokens before acceptance, or move the values to Open Questions.

- 🟡 **Testability**: `Clear filters` action lacks an explicit post-condition assertion
  **Location**: AC — Filter
  The button is mentioned in the filter-applied empty-state AC but its effect is not asserted. Add a dedicated criterion: "when `Clear filters` is clicked, all facet checkboxes return to unchecked, the empty-state panel is removed, and the full unfiltered list re-renders."

#### Minor (Pass 2 selected, full list in per-lens output)

- 🔵 **Clarity**: `ADR-style doc types` term used without enumerating which doc types qualify.
- 🔵 **Clarity**: `latest.modified_at` vs `mtime_ms` field-naming inconsistency — clarify whether the response carries epoch-ms or an ISO string.
- 🔵 **Clarity**: `PR descriptions` rename scope (which identifiers, where) is not enumerated and not in any AC.
- 🔵 **Clarity**: `Glyph` referent overloaded (component vs identifier vs system) — define on first use.
- 🔵 **Clarity**: `ID / DATE` column — the `DATE` half is never defined.
- 🔵 **Clarity**: Filter `{facet-noun}` placeholder source unstated (server field? derived?).
- 🔵 **Dependency**: `formatMtime` and `statusToChipVariant` reused artefacts not in Dependencies (only in Technical Notes).
- 🔵 **Dependency**: Sidebar migration coupling worth a Dependencies-level callout in addition to the Coupled migrations subsection.
- 🔵 **Dependency**: Indexer `mtime_ms` field reliance not surfaced as upstream coupling.
- 🔵 **Scope**: `PR descriptions` rename is orthogonal to the wrapper/hub theme — justify the lockstep or split it out.
- 🔵 **Scope**: Frontend↔server orchestration not explicitly described (which merges first; feature-flag strategy).
- 🔵 **Testability**: Page wrapper "identical computed values" lacks a defined measurement DOM node.
- 🔵 **Testability**: Server-driven endpoint criterion does not pin URL, HTTP method, or status code (deliberately, per Open Question).
- 🔵 **Testability**: Option-search input lacks an explicit "typing filters the options" assertion.
- 🔵 **Testability**: Locale-date fallback format/timezone not specified.
- 🔵 **Testability**: `latest` semantics testable but no worked-fixture example.

### Assessment

The work item is materially ready for implementation: structural concerns are resolved, all six original Open Questions are decided, the server contract is committed, and the Requirements/AC sections are internally consistent. The remaining four major findings break into two groups:

1. **Deliberate trade-offs the author has already made explicit**: the story-vs-epic classification (author's call, documented justification), and the breakpoint hedge added during sharpen (intentional pending 0033 alignment). These are not "fix before implementation" issues — they are visible choices the team can ratify or revise.

2. **Specific, addressable AC tightening**: the `Clear filters` post-condition AC is a clear quick fix; the screenshot-vs-token verification anchor for theme rendering is a 30-minute exercise of listing the relevant tokens (`--ac-surface-*`, `--ac-text-*`, `--ac-glyph-color-*`) once 0033 is consulted.

Recommended next step: address the two specific testability gaps (`Clear filters` AC + token-level theme assertion), then accept the remaining scope/breakpoint findings as ratified trade-offs and proceed to `/create-plan`. Alternatively, accept the current state — none of the remaining issues block planning, and several of them are arguably implementation details that the plan itself will pin down.

## Re-Review (Pass 3) — 2026-05-15T16:25:27+00:00

**Verdict:** COMMENT

The two specific testability fixes from pass 2 (`Clear filters` post-condition AC + token-level theme verification) have been applied. Major findings dropped from 4 → 1: only the deliberate story-vs-epic classification finding remains, which the author has explicitly chosen and justified in Drafting Notes. With 1 major (threshold is 2) and 13 minor findings, the verdict tips from REVISE to COMMENT — the work item is acceptable as-is for planning, with minor refinements available if the author wishes.

### Previously Identified Issues (Pass 2 → Pass 3)

- 🟡 **Testability**: Theme-match without defined tolerance — **Resolved** (now enumerates `--ac-bg-card`, `--ac-bg-app`, `--ac-fg-muted`, `--ac-fg-strong`, `--ac-stroke`, plus the Glyph component's per-doc-type mapping; screenshot retained as sanity-check only).
- 🟡 **Testability**: `Clear filters` lacks post-condition — **Resolved** (new AC asserts all facet checkboxes unchecked, panel removed, pill returns to unselected label, full unfiltered list re-renders).
- 🟡 **Testability**: Breakpoint hedge — **Partially resolved** (testability lens reclassified this from major to minor in pass 3; the parenthetical hedge is still present but no longer blocking).
- 🟡 **Scope**: Epic-classification — **Unchanged** (intentionally; author's explicit call, justification in Drafting Notes).
- 🔵 All other pass-2 minors remain in similar form (terminology nits, dependency-altitude callouts) — no regressions introduced.

### New Issues Introduced

Light surface area only — five new minors emerged from the closer reading the lenses do on a passing work item:

#### Clarity
- 🔵 ADR acronym not expanded on first use.
- 🔵 `PR descriptions` (label) vs `pr_descriptions` (identifier) conflated in the rename description.
- 🔵 `subtitle/count` slot — is it one slot or two? Public API of `Page` is ambiguous.
- 🔵 'Modified semantics' uses an ambiguous "This" pronoun.
- 🔵 `{type-plural}` notation defined without examples.
- 🔵 'jump into a view' in Summary has no concrete referent.

#### Dependency
- 🔵 Server endpoint → frontend consumer ordering not echoed in the Dependencies internal-ordering bullet (only Page wrapper + popover are mentioned there).
- 🔵 PR descriptions rename coupling (which call sites?) not enumerated.
- 🔵 (suggestion) `/library` redirect removal not echoed as a behaviour-change-others-should-know-about note.

#### Scope
- 🟡 **Major**: Story-typed work item spans 9 distinct deliverables — soft restatement of pass-2 finding; author has consistently chosen story-with-justification.
- 🔵 PR descriptions rename is orthogonal to wrapper/hub theme.
- 🔵 Frontend↔backend ownership note not stated.

#### Testability
- 🔵 Breakpoint hedge in AC (now minor, was major).
- 🔵 Screenshot "sanity check" phrasing in theme AC has no defined verification procedure (could simply be deleted now that tokens are enumerated).
- 🔵 Per-doc-type empty-state description sentences not enumerated in the work item.
- 🔵 Faceted-count scoping AC lacks a numeric worked example (parallel to the OR/AND example that does have one).
- 🔵 `formatMtime` locale-date fallback boundary phrased as "matches the locale date" — defers to helper without pinning the format.

### Assessment

Pass 3 hits the COMMENT threshold: 1 major (deliberate trade-off) + 13 minor. The work item is **ready for `/create-plan`** as-is.

The remaining minor findings cluster into three categories, all optional:

1. **Cheap polish** (10-20 minutes each): expand "ADR" on first use, split label-vs-identifier statement for PR descriptions, fix "subtitle/count" slot ambiguity, drop the screenshot "sanity check" sentence, list 1-2 example `{type-plural}` cases.
2. **Optional rigor**: numeric worked example for faceted-count scoping; per-doc-type empty-state sentence table; redundant ordering bullet in Dependencies.
3. **Deliberate trade-offs**: scope bundling (story-not-epic), breakpoint hedge pending 0033 alignment, frontend/backend ownership (likely a single team).

None of the remaining items block planning. The author can ratify pass 3 as-is and move on, or sweep the cheap polish in a 30-minute pass before planning. No further review pass is necessary.
