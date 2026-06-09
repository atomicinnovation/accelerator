---
date: "2026-05-21T10:30:00+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0078"
work_item_id: "0078"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
id: "0078-detail-page-frontmatter-table-review-1"
title: "0078-detail-page-frontmatter-table-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-21T10:30:00+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: Detail-Page Frontmatter Table

**Verdict:** COMMENT

Work item is acceptable but could be improved — see major finding below. The
work item is well-structured, well-cross-referenced, and tightly scoped to a
single coherent deliverable, with concrete CSS values, named code references,
and explicit edge-case handling. The principal concern is a headline
contradiction: the Summary says the table renders "non-null" pairs while the
Requirements and Acceptance Criteria render every key including null/empty
values (as dimmed em-dashes). Several minor refinements would improve
verifiability and dependency traceability.

### Cross-Cutting Themes

- **Headline framing vs. detailed rules disagree on "non-null"** (flagged by:
  clarity, partially echoed in testability via the empty-value AC clarity) —
  the Summary's "non-null" framing is contradicted by the Requirements/AC,
  which render every key. The reconciliation is buried in Assumptions where
  most readers will miss it.
- **0088 width-cap creates an unbounded check** (flagged by: dependency,
  testability) — the table's width depends on a sibling work item's not-yet-
  defined value, and the AC offers no measurable assertion that works
  independently of 0088's eventual choice.

### Findings

#### Major
- 🟡 **Clarity**: "Non-null" in Summary contradicts Requirements and AC
  **Location**: Summary
  The Summary says the table renders "every non-null frontmatter key/value
  pair", but Requirements and AC explicitly mandate every key including null,
  undefined, empty-string, and empty-array values (as dimmed em-dashes). A
  reader anchoring on the Summary would build the wrong mental model.

#### Minor
- 🔵 **Clarity**: "WORK*" shorthand does not match precise pattern used elsewhere
  **Location**: Summary
  The Summary's `WORK*` reads like a glob and omits the project-prefixed forms
  (`PROJ-####`) that the Requirements explicitly include.

- 🔵 **Completeness**: Open Questions section is empty
  **Location**: Open Questions
  The heading is present but unfilled. Unclear whether closure is deliberate
  or accidental.

- 🔵 **Dependency**: 0084 paired-delivery relationship does not state blocking direction
  **Location**: Dependencies
  "Paired delivery" implies scheduling coupling but does not say whether 0078
  blocks 0084, is blocked by it, or whether either can ship independently.

- 🔵 **Dependency**: 0043 spike referenced only under References, not Dependencies
  **Location**: Dependencies
  The work item relies on the spike's findings about resolver pattern
  coverage, but the upstream knowledge dependency on 0043 is not visible to
  planners scanning Dependencies.

- 🔵 **Testability**: "Every frontmatter key" criterion lacks a bounded fixture set
  **Location**: Acceptance Criteria
  Unbounded language ("every key") without a representative fixture means
  verification is sampling rather than deterministic.

- 🔵 **Testability**: Styling AC bundles multiple independently verifiable assertions
  **Location**: Acceptance Criteria
  Grid template, font, background, border, padding, and width cap are lumped
  into a single checkbox; partial failures have no place to be recorded.

- 🔵 **Testability**: Width-cap parity with markdown body lacks a measurable threshold
  **Location**: Acceptance Criteria
  The "matches 0088's width" criterion is unverifiable without a structural
  reframing that does not depend on 0088's absolute value.

#### Suggestions
- 🔵 **Clarity**: Reconciling assumption buried where readers will miss it
  **Location**: Assumptions
  The Assumptions bullet that reconciles "non-null" with the actual rendering
  rules is the only place the contradiction is resolved; fixing the Summary
  would make this assumption redundant.

- 🔵 **Clarity**: "Per prototype" references rely on Context for full styling list
  **Location**: Requirements
  A reader cannot tell whether the enumerated CSS values are the complete
  styling spec or whether the prototype implies additional unstated details
  (hover, focus, row separators).

- 🔵 **Dependency**: Downstream consumers (Blocks) not enumerated
  **Location**: Dependencies
  No "Blocks" entry — either deliberately empty or accidentally omitted; an
  explicit "Blocks: (none)" would make the intent clear.

- 🔵 **Scope**: Linkification rules slightly bundled with table rendering
  **Location**: Requirements
  Bundling table rendering with linkification semantics is reasonable given
  resolver reuse, but the decision to combine them is implicit.

- 🔵 **Testability**: Auto-linkification criterion does not specify mixed-content behaviour
  **Location**: Acceptance Criteria
  Drafting Notes mention that values with free text mixed with a WORK token
  linkify only the matching substring, but no AC captures this.

### Strengths

- ✅ Concrete, named code references (`useWikiLinkResolver`, `LibraryDocView`,
  `FrontmatterChips`, `formatChipValue`) leave no ambiguity about which
  components are involved.
- ✅ Edge-case values are explicitly enumerated (null, undefined, empty
  string, empty array, plus a note that `0` and `false` are valid) so empty
  is not left to interpretation.
- ✅ Cross-references to sibling work items (0041, 0084, 0085, 0088) are
  scoped — each says what is delegated and what remains in 0078.
- ✅ Adjacent concerns are explicitly excluded and pointed at sibling items,
  keeping this item's boundary clean.
- ✅ Drafting Notes deliberately disambiguate phrases like "always expanded"
  (no collapse UI at all) and source-order rows (no canonical sort imposed).
- ✅ Frontmatter integrity is solid: kind=story, status=draft, priority=
  medium, tags populated, author and date present.
- ✅ Styling AC names exact tokens, dimensions, and font — directly assertable.
- ✅ Drafting Notes acknowledges the ordering risk with 0088 explicitly,
  turning a potential hidden ordering constraint into documented graceful
  degradation.
- ✅ Reuse of `useWikiLinkResolver` is called out, avoiding hidden
  infrastructure work that would otherwise inflate the scope.

### Recommended Changes

1. **Fix the Summary's "non-null" framing** (addresses: "'Non-null' in Summary
   contradicts Requirements and AC", "Reconciling assumption buried where
   readers will miss it")
   Rewrite the first sentence of the Summary to say "every frontmatter
   key/value pair (with dimmed dashes for empty values)" or similar. Then
   remove the now-redundant Assumptions bullet that reconciles the term.

2. **Replace "WORK*" shorthand with precise language** (addresses: "'WORK*'
   shorthand does not match precise pattern used elsewhere")
   In the Summary, replace `WORK*` with "work-item ID values (per the
   configured `work.id_pattern`)" to match the vocabulary used elsewhere and
   include project-prefixed forms.

3. **Resolve the empty Open Questions section** (addresses: "Open Questions
   section is empty")
   Either remove the heading or add a one-line note such as "None — all
   decisions captured in Assumptions and Drafting Notes".

4. **Reframe the 0088 width-cap AC as a structural check** (addresses:
   "Width-cap parity with markdown body lacks a measurable threshold")
   Replace the absolute-width framing with "the table's computed `max-width`
   equals the markdown body's computed `max-width` at the same viewport
   width", so verification does not depend on 0088's absolute value.

5. **Decompose the styling AC** (addresses: "Styling AC bundles multiple
   independently verifiable assertions")
   Split the single styling AC into separate bullets per assertion (grid
   template, font, background, border, padding, width cap) or explicitly
   reference a visual-regression fixture that asserts all six together.

6. **Anchor "every frontmatter key" to a fixture** (addresses: "'Every
   frontmatter key' criterion lacks a bounded fixture set")
   Name a representative fixture (e.g., a canonical work-item file or an
   enumerated key set) the table is verified against.

7. **Clarify 0084 paired-delivery direction** (addresses: "0084 paired-
   delivery relationship does not state blocking direction")
   In Dependencies, state explicitly whether 0078 blocks 0084, is blocked by
   it, or both can ship independently.

8. **Add 0043 to Dependencies as informational** (addresses: "0043 spike
   referenced only under References, not Dependencies")
   Add a "Builds on: 0043 (spike establishing wiki-link resolver pattern
   coverage)" line, or note explicitly that the spike is informational-only.

9. **Add an AC for mixed-content linkification** (addresses: "Auto-
   linkification criterion does not specify mixed-content behaviour")
   Capture the Drafting Notes behaviour as an AC: "Values containing both
   free text and a work-item token render the matching substring as an
   anchor and the surrounding text as plain text."

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is mostly clear and well-cross-referenced, with
concrete CSS values, named components, and explicit handling for edge cases
like null and empty values. However, the Summary's "non-null" framing
directly contradicts Requirements and Acceptance Criteria, which render every
key including null/undefined/empty values. The Summary also uses a loose
"WORK*" shorthand that does not match the more precise `work.id_pattern`
terminology used elsewhere.

**Strengths**:
- Concrete, named code references (`useWikiLinkResolver`, `LibraryDocView`,
  `FrontmatterChips`, `formatChipValue`) leave no ambiguity.
- Edge-case values are explicitly enumerated.
- Cross-references to sibling work items are scoped explicitly.
- Drafting Notes disambiguate phrases like "always expanded" and source-order
  rows.

**Findings**:
- 🟡 (major, high) **"Non-null" in Summary contradicts Requirements and AC** —
  Summary, redefinition in Assumptions does not resolve the live
  contradiction.
- 🔵 (minor, high) **"WORK*" shorthand does not match precise pattern used
  elsewhere** — Summary; omits project-prefixed forms.
- 🔵 (suggestion, medium) **Reconciling assumption buried where readers will
  miss it** — Assumptions; would be redundant once Summary is fixed.
- 🔵 (suggestion, medium) **"Per prototype" references rely on Context for
  full styling list** — Requirements; ambiguous on whether enumerated values
  are exhaustive.

### Completeness

**Summary**: Work item 0078 is a well-structured story with substantive
content across all expected sections: Summary, Context, Requirements,
Acceptance Criteria, Dependencies, Assumptions, Technical Notes, Drafting
Notes, and References. Frontmatter is intact with a recognised kind (story)
and status (draft). The only notable structural gap is an empty Open
Questions section.

**Strengths**:
- Summary states the work clearly as a single action.
- Context explains motivation concretely with specific examples.
- Acceptance Criteria contains nine specific bullets covering all key
  behaviours.
- Frontmatter integrity is solid.
- Story-appropriate content is present.

**Findings**:
- 🔵 (minor, medium) **Open Questions section is empty** — Open Questions;
  unclear whether closure is deliberate.

### Dependency

**Summary**: The work item captures its principal couplings explicitly: an
upstream blocker (0041), related siblings (0084, 0085, 0088), a reused
resolver, and downstream consumption points. No external systems or cross-
team actions are implied. One ordering ambiguity exists around 0088's width-
cap matching, and the downstream relationship to 0084 is described as
"paired delivery" without naming whether 0078 blocks or is blocked by 0084.

**Strengths**:
- Upstream blocker is named explicitly with rationale.
- Reused infrastructure is called out under Dependencies and Technical Notes.
- Related items are enumerated with the nature of the relationship.
- Drafting Notes acknowledges the 0088 ordering risk explicitly.
- Assumptions confirms no loader changes are required.

**Findings**:
- 🔵 (minor, medium) **0084 paired-delivery relationship does not state
  blocking direction** — Dependencies; blocking direction unclear.
- 🔵 (minor, medium) **0043 spike referenced only under References, not
  Dependencies** — Dependencies; upstream knowledge dependency invisible to
  planners.
- 🔵 (suggestion, low) **Downstream consumers (Blocks) not enumerated** —
  Dependencies; no explicit "Blocks: (none)" or list of downstream consumers.

### Scope

**Summary**: Work item 0078 describes one coherent unit of work: a new
frontmatter-table component rendered above the markdown body on detail
pages. Summary, Requirements, and Acceptance Criteria are tightly aligned
around a single deliverable, and adjacent concerns are explicitly delegated
to sibling work items. The story-level sizing is appropriate.

**Strengths**:
- All sections describe the same single deliverable.
- Adjacent concerns are explicitly excluded.
- Scope is confined to a single frontend component.
- Reuse of `useWikiLinkResolver` avoids hidden infrastructure work.

**Findings**:
- 🔵 (suggestion, medium) **Linkification rules slightly bundled with table
  rendering** — Requirements; bundling is reasonable given resolver reuse but
  is implicit rather than stated.

### Testability

**Summary**: The acceptance criteria are largely specific, observable, and
verifiable, with concrete CSS values, named resolvers, and well-defined
rendering rules. A few criteria reference unbounded scope ("every
frontmatter key") without a defined fixture set, and the styling AC bundles
multiple checks into one bullet that would benefit from decomposition.

**Strengths**:
- Styling AC names exact tokens, dimensions, and font.
- Linkification is anchored to a named resolver and configured pattern.
- Empty-value rendering is unambiguous.
- Assumptions clarifies that `0` and `false` are valid values.

**Findings**:
- 🔵 (minor, medium) **"Every frontmatter key" criterion lacks a bounded
  fixture set** — Acceptance Criteria; verification is sampling not
  deterministic.
- 🔵 (minor, medium) **Styling AC bundles multiple independently verifiable
  assertions** — Acceptance Criteria; partial failures hidden behind one
  tick.
- 🔵 (minor, medium) **Width-cap parity with markdown body lacks a measurable
  threshold** — Acceptance Criteria; depends on 0088's not-yet-defined value.
- 🔵 (suggestion, low) **Auto-linkification criterion does not specify mixed-
  content behaviour** — Acceptance Criteria; Drafting Notes captures it but
  no AC does.

## Re-Review (Pass 2) — 2026-05-21T10:30:00+00:00

**Verdict:** APPROVE

### Previously Identified Issues

- 🟡 **Clarity**: "Non-null" in Summary contradicts Requirements and AC — **Resolved**. Summary rewritten to "every frontmatter key/value pair (with dimmed dashes for empty values)"; the redundant Assumptions bullet was removed.
- 🔵 **Clarity**: "WORK*" shorthand does not match precise pattern — **Resolved**. Summary now reads "work-item ID values (per the configured `work.id_pattern`, including project-prefixed forms)".
- 🔵 **Clarity**: Reconciling assumption buried where readers will miss it — **Resolved**. Assumption removed alongside Summary fix.
- 🔵 **Clarity**: "Per prototype" references rely on Context for full styling list — **Resolved**. The single "per prototype" Requirements bullet was decomposed into six explicit ACs, removing the "per prototype" framing.
- 🔵 **Completeness**: Open Questions section is empty — **Resolved**. Added "None — all decisions captured in Assumptions and Drafting Notes."
- 🔵 **Dependency**: 0084 paired-delivery direction unspecified — **Resolved**. Now stated as "can ship independently in either order; paired delivery preferred but not required".
- 🔵 **Dependency**: 0043 spike not in Dependencies — **Resolved**. "Builds on: 0043 — informational only, no scheduling dependency" added.
- 🔵 **Dependency**: Downstream consumers (Blocks) not enumerated — **Still present**. Dependency lens re-raises this as a soft block on 0084.
- 🔵 **Scope**: Linkification rules bundled with table rendering — **Still present (related)**. Scope lens now flags the specific mixed-text linkification AC as the bundling concern.
- 🔵 **Testability**: "Every frontmatter key" lacks a bounded fixture set — **Resolved**. Canonical nine-key fixture named with expected row count and order.
- 🔵 **Testability**: Styling AC bundles multiple assertions — **Resolved**. Decomposed into six separate ACs.
- 🔵 **Testability**: Width-cap parity lacks a measurable threshold — **Partially resolved**. Reframed structurally, but the conditional fallback ("if 0088 has not yet shipped") still splits the pass condition.
- 🔵 **Testability**: Mixed-content linkification not in AC — **Resolved**. New AC added covering free-text + work-item-token substrings.

### New Issues Introduced

- 🔵 **Clarity** (minor): "Every detail page" scope not explicitly defined — Summary/Requirements. Pre-existing nuance now exposed once the contradiction was cleared; could be tightened with a one-line definition.
- 🔵 **Clarity** (minor): Mixed free-text + work-item-token behaviour in AC but not Requirements — Created by the new AC; the Requirements section still describes scalar matching as whole-value.
- 🔵 **Clarity** (suggestion): "Object values render as a JSON-serialised string" underspecified — pre-existing; defers to `formatChipValue` for the exact rule.
- 🔵 **Clarity** (suggestion): "Configured project-prefixed pattern" lacks a concrete reference — pre-existing.
- 🔵 **Clarity** (suggestion): Pattern coverage assumption phrasing is mildly self-contradictory — pre-existing.
- 🔵 **Dependency** (minor): `work.id_pattern` configuration surface not captured as a coupling — pre-existing.
- 🔵 **Dependency** (suggestion): `type` → `kind` rename prerequisite not surfaced — pre-existing, mentioned only in Drafting Notes.
- 🔵 **Scope** (suggestion): Mixed free-text linkification AC may be a separable concern — created by the new mixed-content AC; resolves to "free if the resolver already supports it".
- 🔵 **Testability** (minor): Conditional fallback in width-cap AC ("if 0088 has not yet shipped") splits the pass condition — surfaced by the structural reframing.
- 🔵 **Testability** (minor): CSS-variable ACs do not specify whether to assert variable name or resolved value — exposed by the decomposition.
- 🔵 **Testability** (minor): "Dimmed em-dash" not pinned to a literal character or class — pre-existing.
- 🔵 **Testability** (minor): No fixture demonstrating a project-prefixed pattern — pre-existing.
- 🔵 **Testability** (minor): "Matches the markdown body's linkification exactly" has no AC hook — pre-existing.

### Assessment

The work item is now ready for implementation. The headline contradiction (the major finding) is fully resolved, and all but one of the originally flagged minor issues are either resolved or only partially resolved with documented behaviour. The remaining minor findings are second-order refinements — pre-existing nuances now visible because the high-level structure is cleaner — and none block planning or implementation. They could be addressed during implementation or in a follow-up polish pass without holding up the story.

