---
date: "2026-05-26T09:20:00Z"
type: work-item-review
producer: review-work-item
target: "work-item:0088"
work_item_id: "0088"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
id: "0088-markdown-body-width-harmonisation-review-1"
title: "0088-markdown-body-width-harmonisation-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-26T09:20:00Z"
last_updated_by: Toby Clemson
---

## Work Item Review: Markdown Body Width Harmonisation

**Verdict:** COMMENT

Work item is acceptable as-is — see findings below for non-blocking
polish. The story is tightly scoped, internally consistent, and
broadly testable: it names a single CSS module, exact token values,
exact CSS values, and quantified viewport thresholds. The minor
issues cluster around measurability tolerances (the `~720px` target
and the "shrinks fluidly" qualifier) and a handful of small clarity
referents (USWDS acronym, an ambiguous `its`, summary phrasing). No
completeness gaps; dependencies are well captured with a minor open
question about coordination with 0076 on the shared file.

### Cross-Cutting Themes

- **Approximate vs measurable thresholds** (flagged by: clarity,
  testability) — Two acceptance criteria use approximate or
  qualitative language (`~720px`, `shrinks fluidly`, `its width…
  the ~260px aside is not overflowed`) that admits multiple
  interpretations at verification time. Tightening to explicit
  tolerances or computed-style assertions would make the criteria
  mechanically checkable.

### Findings

#### Critical

_None._

#### Major

_None._

#### Minor

- 🔵 **Clarity**: Undefined acronym 'USWDS' used in passing
  **Location**: Drafting Notes
  `USWDS-style` appears in Requirements and Technical Notes without
  definition. A reader outside the US design-token context cannot
  tell what convention is being invoked.

- 🔵 **Clarity**: Ambiguous referent 'its' in 1440px viewport
  criterion
  **Location**: Acceptance Criteria
  `its width caps at ~720px… and the ~260px aside is not
  overflowed` is structurally ambiguous: does it mean the aside
  itself does not overflow, or that the prose column does not
  extend into the aside?

- 🔵 **Clarity**: "Apply both across every consumer" understates
  the actual scope
  **Location**: Summary
  Summary phrasing suggests per-consumer edits; the change actually
  lives entirely in `MarkdownRenderer.module.css` and consumers
  inherit it.

- 🔵 **Testability**: Prose width target uses '~720px' without a
  tolerance
  **Location**: Acceptance Criteria (1440px viewport)
  `1ch` is font-metric-dependent so the rendered value will not be
  exactly 720px. Without a tolerance band, verifiers cannot decide
  pass/fail on a measurement like 712px or 731px.

- 🔵 **Testability**: 'Shrinks fluidly' lacks a defined check
  **Location**: Acceptance Criteria (800px viewport)
  A stuck 720px column that happens not to overflow at 800px would
  arguably pass the non-overflow half on its own.

- 🔵 **Testability**: Token-placement clause uses 'alongside'
  without a defined check
  **Location**: Acceptance Criteria (token-definition bullet)
  `alongside the existing family` is positional/aesthetic; admits
  multiple interpretations.

#### Suggestions

- 🔵 **Clarity**: "The current app" vs "the project" — implicit
  subject
  **Location**: Context
  Parallel definite-article phrasing for distinct referents makes a
  quick first pass slightly harder.

- 🔵 **Dependency**: Coordination with 0076 on shared markdown
  surface not characterised
  **Location**: Dependencies
  Both 0088 and 0076 touch `MarkdownRenderer.module.css`. The
  Dependencies note does not state whether there is an ordering
  constraint or merge-conflict risk.

- 🔵 **Dependency**: Playwright baseline refresh implies a
  CI/visual-regression workflow coupling
  **Location**: Requirements
  No external system or tooling prerequisite is named for baseline
  refresh; if any workflow state (e.g., launcher locale fix)
  matters, name it.

- 🔵 **Scope**: Body font-size change could in principle be a
  separate increment
  **Location**: Requirements
  Bundling is a judgement call, not a defect — consider noting in
  Drafting Notes why the two changes ship together.

- 🔵 **Testability**: Baseline-refresh criterion does not specify
  which routes/fixtures must update
  **Location**: Requirements / Acceptance Criteria
  A no-op suite pass would technically satisfy the wording; name
  the spec files explicitly.

### Strengths

- ✅ Summary, Context, Requirements, and Acceptance Criteria
  describe the same scope with no contradictions; Given/When/Then
  framing is consistent.
- ✅ Concrete file paths, exact token names, and exact CSS values
  (`min(var(--ac-content-max-width-prose), 100%)`,
  `var(--size-body)`) make verification mechanical.
- ✅ Negative assertions (no literal `720px` or other px width, no
  literal px `font-size`) strengthen anti-regression value.
- ✅ Out-of-scope surfaces (templates preview / `TemplateHighlight`)
  are named with rationale; consumer surfaces (`LibraryDocView`,
  `CodeSyntaxShowcase`) are enumerated.
- ✅ Upstream dependencies (0033, 0075) named with completion
  state; Blocks declared as `none` rather than omitted; Related
  0076 surfaced as a coordination point.
- ✅ Technical Notes pre-empts likely reader confusion (`1ch`
  semantics, why the cap lives on `.markdown`, near-zero width
  delta vs current cap, `min(token, 100%)` rationale).
- ✅ Frontmatter fully populated, using the current `kind:`
  schema.

### Recommended Changes

1. **Tighten the `~720px` viewport criterion** (addresses:
   Testability — '~720px' without tolerance, Clarity — ambiguous
   'its')
   Either specify an explicit tolerance band or restate as a
   computed-style assertion (e.g., "the computed `max-width` of
   `.markdown` resolves to `72ch` in its own font context, and the
   rendered prose column does not extend into the ~260px aside
   track"). Resolving this also disambiguates the `its` referent.

2. **Replace 'shrinks fluidly' with a measurable check** (addresses:
   Testability — 'shrinks fluidly' lacks defined check)
   E.g., "the prose column's rendered width equals the parent grid
   track's computed width (i.e., the `100%` branch of the `min()`
   is in effect) and does not overflow the page wrapper."

3. **Define USWDS or replace with descriptive phrase** (addresses:
   Clarity — undefined acronym)
   Expand on first use or replace `USWDS-style` with
   "unit-baked token value" (or similar) in both Requirements and
   Technical Notes.

4. **Tighten token-placement language** (addresses: Testability —
   'alongside' without defined check)
   Either restate as "declared in the same `:root` block as
   `--ac-content-max-width` and `--ac-content-max-width-narrow`" or
   drop the placement clause and rely on the value check.

5. **Sharpen the Summary's "apply across every consumer" clause**
   (addresses: Clarity — Summary understates scope)
   Reword to "so that every `MarkdownRenderer` consumer inherits a
   single source of truth for prose width and size" — keeps the
   uniformity claim, makes the single-edit-site clear.

6. **Name the Playwright spec files in the baseline criterion**
   (addresses: Testability — baseline-refresh criterion under-
   specified)
   E.g., "the commit includes updated baseline images for at least
   the `library-doc-view` and `code-syntax-showcase` Playwright
   specs."

7. **Add a one-line note on 0076 coordination** (addresses:
   Dependency — 0076 ordering not characterised)
   State whether the two work items can land in parallel, need an
   ordering, or expect a rebase against each other on
   `MarkdownRenderer.module.css`.

8. **Note in Drafting Notes why width and body-size ship together**
   (addresses: Scope — bundling rationale)
   Document the `ch`-vs-`--size-body` interdependence so future
   readers see the bundling as intentional.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually clear and internally
consistent: scope, requirements, and acceptance criteria align
tightly, and the typography/token jargon is appropriate for the
team domain. A few minor referent ambiguities and one undefined
acronym in passing reduce, but do not seriously impair,
comprehension for a reader new to the project.

**Strengths**:
- Summary, Context, Requirements, and Acceptance Criteria all
  describe the same scope with no contradictions.
- Each acceptance criterion uses a Given/When/Then structure that
  names the artefact being inspected.
- Technical Notes pre-empts likely reader confusion (`1ch` semantics,
  why the cap lives on `.markdown`, the near-zero width delta).
- Assumptions explicitly disambiguate which surfaces are in scope
  vs out.

**Findings**:
- 🔵 minor / high — Undefined acronym 'USWDS' used in passing
  (Drafting Notes / Requirements / Technical Notes)
- 🔵 minor / medium — Ambiguous referent 'its' in 1440px viewport
  criterion (Acceptance Criteria)
- 🔵 minor / medium — "Apply both across every consumer"
  understates the actual scope (Summary)
- 🔵 minor / low — "The current app" vs "the project" — implicit
  subject (Context)

### Completeness

**Summary**: Work item 0088 is a well-populated story with
substantive Summary, Context, Requirements, Acceptance Criteria,
Dependencies, Assumptions, Technical Notes, and Drafting Notes.
Frontmatter is complete with a recognised kind (story), status,
priority, and tags. No completeness gaps rise to actionable
findings for a story of this scope.

**Strengths**:
- Summary is a single, unambiguous action statement naming both the
  token and the value.
- Context explains motivation and identifies blocking groundwork
  (0033, 0075).
- Acceptance Criteria contains seven specific, Given/When/Then
  criteria covering token, renderer, viewport, migration test, and
  Playwright.
- Requirements section enumerates concrete actions sufficient for
  an implementer to start without follow-up.
- Kind-appropriate content for a story.
- Dependencies, Assumptions, Technical Notes, Drafting Notes all
  populated with substantive content.
- Frontmatter fully populated using current `kind:` schema.

**Findings**: None.

### Dependency

**Summary**: Dependencies are well captured for this story: the two
completed prerequisite work items (0033, 0075) are named with their
roles, the related work (0076) is acknowledged, and Blocks is
explicitly declared as none. The two consumer surfaces are named
and the out-of-scope surface is explicitly carved out with
rationale.

**Strengths**:
- Upstream prerequisites 0033 and 0075 named with completion state.
- Blocks explicitly declared as 'none' rather than omitted.
- Related work 0076 surfaced as a coordination point.
- Consumer surfaces enumerated; out-of-scope surface named with
  rationale.

**Findings**:
- 🔵 suggestion / medium — Coordination with 0076 on shared
  markdown surface not characterised (Dependencies)
- 🔵 suggestion / low — Playwright baseline refresh implies a
  CI/visual-regression workflow coupling (Requirements)

### Scope

**Summary**: 0088 is a tightly-scoped story that bundles two
closely-related changes to a single CSS module plus their token
additions and baseline refreshes. The two changes are coherent
because they share the same file, the same renderer, the same
consumers, and the same token-discipline rationale.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria all describe the
  same scope.
- Out-of-scope surfaces named explicitly with rationale.
- Consumers enumerated, blast radius bounded.
- Sizing matches the declared 'story' kind.

**Findings**:
- 🔵 suggestion / medium — Body font-size change could in principle
  be a separate increment (Requirements)

### Testability

**Summary**: The work item is highly testable, with concrete file
paths, exact CSS values, specific token names, and quantified
viewport measurements that admit definitive pass/fail verification.
Two criteria contain minor measurability ambiguities.

**Strengths**:
- Consistent Given/When/Then framing with concrete preconditions
  and outcomes.
- References to specific files and identifiers make verification
  mechanical.
- Exact CSS values eliminate interpretation latitude.
- Negative assertions strengthen verifiability beyond a single
  positive check.
- Quantified viewport contexts and structural reference points
  provided.
- Test-suite outcomes tied to specific suites.

**Findings**:
- 🔵 minor / high — Prose width target uses '~720px' without a
  tolerance (Acceptance Criteria, 1440px viewport)
- 🔵 minor / medium — 'Shrinks fluidly' lacks a defined check
  (Acceptance Criteria, 800px viewport)
- 🔵 minor / medium — Token-placement clause uses 'alongside'
  without a defined check (Acceptance Criteria, token-definition)
- 🔵 suggestion / medium — Baseline-refresh criterion does not
  specify which routes/fixtures must update (Requirements /
  Acceptance Criteria)

## Re-Review (Pass 2) — 2026-05-26T09:20:00Z

**Verdict:** APPROVE

Lenses re-run: clarity, dependency, scope, testability (completeness
had no findings in pass 1 so it was not re-run). The work item is in
a substantially stronger state: every original finding is at least
partially addressed, most are resolved outright. Residual items are
all minor or suggestion-level — primarily diminishing-returns
refinements around measurement tolerances and a couple of small
items pre-existing but newly noticed.

### Previously Identified Issues

- 🔵 **Clarity**: Undefined acronym 'USWDS' — **Resolved**.
  Replaced with the descriptive gloss "the token stores `72ch`,
  not the bare number `72`" in both Requirements and Technical
  Notes.
- 🔵 **Clarity**: Ambiguous referent 'its' in 1440px viewport
  criterion — **Resolved**. AC4 rewritten to refer to `.markdown`
  and the prose column explicitly; the aside-overflow clause is
  now phrased as "the prose column does not extend into the
  ~260px aside grid track".
- 🔵 **Clarity**: "Apply both across every consumer" understates
  scope — **Resolved**. Summary reworded to "applied once on
  `MarkdownRenderer` so that every consumer inherits a single
  source of truth".
- 🔵 **Clarity**: "The current app" vs "the project" — **Not
  addressed** (intentionally skipped during iteration as a low-
  confidence reader-positioning nit). Still present.
- 🔵 **Testability**: Prose width target uses '~720px' without
  tolerance — **Partially resolved**. AC4 now leads with a
  computed-style assertion (`computed max-width resolves to
  72ch`), which is the testable core. The "≈720px" figure
  survives in parentheses as an explanatory aside, and the
  "rendered prose column's width does not exceed that cap" clause
  retains some residual measurability ambiguity.
- 🔵 **Testability**: 'Shrinks fluidly' lacks defined check —
  **Resolved**. AC5 now asserts the `100%` branch is in effect and
  that rendered width equals the parent grid track's computed
  width.
- 🔵 **Testability**: Token-placement clause uses 'alongside' —
  **Resolved**. AC1 and the matching Requirement now name the
  `:root` block in `global.css` and the width-token section in
  `tokens.ts` explicitly.
- 🔵 **Testability**: Baseline criterion does not specify
  routes/fixtures — **Partially resolved**. AC7 and the
  Requirement now name `library-doc-view` and `code-syntax-
  showcase` specs, but use "at minimum"/"at least", which leaves
  the full set of mandatory updates unenumerated.
- 🔵 **Dependency**: Coordination with 0076 on shared file —
  **Resolved**. One-liner added noting no hard ordering and that a
  rebase is expected if landed in parallel.
- 🔵 **Dependency**: Playwright baseline refresh implies CI/
  tooling coupling — **Still present**. Naming the specs in AC7
  helps a verifier but does not formalise the runnability
  precondition for the visual-regression infrastructure.
- 🔵 **Scope**: Body font-size change could be a separate
  increment — **Resolved**. Drafting Notes now documents the
  `ch`-vs-`--size-body` interdependence as the bundling rationale.

### New Issues Introduced

- 🔵 **Clarity** (minor / medium): AC4 parenthetical "≈720px at
  the `--size-body` 20px body" mixes the normative computed-style
  check with an approximate pixel figure; a verifier cannot tell
  whether the px figure is asserted or merely informative.
  *Introduced by the edit that tightened the criterion.*
- 🔵 **Clarity** (minor / low): AC5 phrasing "the `100%` branch of
  the `min()` is in effect" leads with a CSS-evaluation claim
  before the observable equivalent (rendered width equals parent
  grid track). The "i.e." clause does the work; the ordering
  could be flipped.
  *Introduced by the edit that tightened the criterion.*
- 🔵 **Clarity** (minor / medium): "irreducible" used as project
  jargon in Requirements without an inline gloss (Technical Notes
  hints at it but Requirements does not).
  *Pre-existing — not flagged in pass 1 but surfaced now that
  other clarity issues are gone.*
- 🔵 **Testability** (minor / high): AC4's "the prose column does
  not extend into the ~260px aside grid track" lacks a precise
  bounding-box assertion (which element bounds against which).
  *Pre-existing — partially related to the ambiguous 'its'
  finding from pass 1; the rewrite preserved the imprecise
  measurement target.*
- 🔵 **Testability** (minor / medium): AC7's "suite passes against
  them" combined with "at least" can mask unrelated failures or
  omitted baselines.
  *Pre-existing — surfaced more sharply now that the named-specs
  fix is in place.*
- 🔵 **Dependency** (suggestion / low): vitest enforcement /
  `migration.test.ts` coupling implicit via Requirements but not
  surfaced in Dependencies.
  *Pre-existing — not flagged in pass 1.*
- 🔵 **Scope** (suggestion / medium): Baseline-refresh requirement
  could be a separable housekeeping step; bundling is defensible
  but worth a one-liner of justification.
  *Pre-existing — overlaps the resolved scope finding from pass 1
  but addresses a different bundling decision.*

### Assessment

The work item is ready for implementation. All pass-1 findings are
resolved or have a clearly documented residual that does not impede
verification. The new items are diminishing-returns refinements:
they would sharpen measurement tolerances further but are not
blockers, and adding them would risk over-specifying the
acceptance criteria for a small story.

Recommended posture: **accept the COMMENT verdict and move to
implementation**. If a future iteration is wanted, the two highest-
value targets are the AC4 parenthetical (separate the normative
`72ch` check from the explanatory ≈720px figure) and a one-line
gloss on "irreducible" in Requirements.

