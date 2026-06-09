---
date: "2026-05-26T08:55:08+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0084"
work_item_id: "0084"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 1
status: complete
id: "0084-detail-page-chip-strip-cap-review-1"
title: "0084-detail-page-chip-strip-cap-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-26T08:55:08+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: Detail-Page Chip Strip Cap (Status, Date, Author)

**Verdict:** APPROVE (initial review returned COMMENT; all 10 recommendations applied to the work item — see below)

Work item 0084 is in strong shape across all five lenses. Scope is tight and atomic (single-component change with explicit out-of-scope boundaries), all named upstream prerequisites are `status: done`, and the seven Given/When/Then acceptance criteria define observable outcomes for the cap, omission rules, and date/author precedence. The findings are uniformly minor: a handful of measurability gaps around the zero-chip "empty container" behaviour, a couple of clarity nits around the overloaded `date` term, and some optional refinements to dependency direction and frontmatter alignment with ADR-0033. Work item is acceptable as-is but could be improved — see findings below.

### Cross-Cutting Themes

- **Zero-chip "empty container" behaviour is not measurably defined** (flagged by: clarity, testability) — "preserves header rhythm" and "H1 no longer sits on the divider" describe a visual outcome without an observable threshold (min-height, fixed class, or DOM marker). Two implementers could legitimately disagree on whether a 0-height-but-present container satisfies the rule.
- **12-doc-kind verification mechanism is not specified** (flagged by: clarity, testability) — the final AC enumerates the 12 kinds but does not say whether verification is by a parameterised unit test, per-kind fixtures, Playwright snapshots, or component-layer assertion only.

### Findings

#### Minor

- 🔵 **Clarity**: Overloaded use of 'date' as both key name and chip name
  **Location**: Acceptance Criteria
  The term `date` is used as a frontmatter key name, a chip label, and a semantic concept ("creation-anchored"). ACs like "the `date` chip uses `date` (creation-anchored)" force the reader to disambiguate three referents of the same token in one sentence.

- 🔵 **Clarity** / **Testability** (merged theme): "Empty container that still occupies the subtitle slot" / "H1 no longer sits on divider" lacks an observable definition
  **Location**: Requirements (3rd bullet); Acceptance Criteria (4th bullet)
  "Header rhythm" and "sitting on the divider" are visual-design idioms without an observable threshold (min-height, fixed CSS class, computed height). Two implementers could each satisfy the requirement with different DOM/CSS shapes.

- 🔵 **Clarity** / **Testability** (merged theme): "Verified across the 12 doc kinds" — verification mechanism is implicit
  **Location**: Acceptance Criteria (last bullet)
  The parenthetical "verified across" is ambiguous: it could mean the enumeration is the scope, or that verification mechanically iterates the 12. Without a defined procedure (parameterised test, per-kind fixtures, component-layer assertion), the criterion risks being claimed met via spot-check.

- 🔵 **Completeness**: No Open Questions section despite an explicit unresolved follow-up
  **Location**: Open Questions (missing)
  The "future schema-alignment story (TBD)" is mentioned in Dependencies and Drafting Notes but not surfaced in a single Open Questions section, so readers must scan multiple sections to assemble unresolved items.

- 🔵 **Completeness**: Frontmatter omits several ADR-0033 base fields
  **Location**: Frontmatter
  ADR-0033 (cited from this item) mandates a unified base set including `id`, `type`, `schema_version`, `last_updated`, `last_updated_by`, `producer`. This item's frontmatter omits them while explicitly referencing the ADR.

- 🔵 **Dependency**: Schema-alignment follow-up story relationship is one-directional but recorded only as "Related"
  **Location**: Dependencies
  The future schema-alignment story's user-visible 3-chip outcome is gated on this work item shipping first, but Blocks is `none` and the entry is filed as neutral "Related" — understating the directional coupling.

- 🔵 **Testability**: Whitespace-only string handling not specified
  **Location**: Assumptions / Acceptance Criteria (2nd bullet)
  The missingness rule covers absent / null / empty string but not whitespace-only (`author: "   "`). Mixed-template corpus makes this likely to occur in practice.

- 🔵 **Testability**: No criterion asserts ordering when only two of the three keys are present
  **Location**: Acceptance Criteria
  Ordering is fixed for the three-chip case and for the date+author notes case, but not explicitly for status+date or status+author pairs. The "never reorders" property in Requirements is not surfaced as a directly testable AC.

- 🔵 **Clarity**: Prototype cap restated slightly more strongly than the source supports
  **Location**: Context
  "The prototype's chip row is hard-limited to four chips" is slightly stronger than the source design-gap document's "max four chips" / "harmonise the chip-strip cap to four chips max". Low-impact cross-document precision nit.

#### Suggestions

- 🔵 **Dependency**: ADR-0033 schema coupling not surfaced in Dependencies
  **Location**: Dependencies
  ADR-0033 governs the canonical key set this story's whitelist effectively pins. It appears in References but not Dependencies, so the schema coupling is invisible to a reader scanning upstream constraints.

### Strengths

- ✅ Single coherent purpose: every requirement and AC describes capping/whitelisting chips in `FrontmatterChips` — no bundled refactor or unrelated concern.
- ✅ Chip order and contents are stated unambiguously and repeated consistently across Summary, Requirements, and Acceptance Criteria (status → date → author).
- ✅ Explicit out-of-scope boundaries: the schema-alignment follow-up, verdict relocation, and StatusBadge changes are each acknowledged and deferred.
- ✅ Upstream blockers explicitly cross-checked: 0038, 0078, 0081 each named with `status: done`, justifying the empty Blocked-by.
- ✅ Divergence from the prototype's four-chip cap is named and justified (two coloured-tone slots competing for attention) in Context and Drafting Notes.
- ✅ "Missing" definition is pinned down in Assumptions (absent, null, empty string), and `0` / `false` edge cases are explicitly excluded for the canonical set.
- ✅ Acceptance Criteria use Given/When/Then framing with concrete preconditions and observable outcomes; the precedence rules for `last_updated`/`date` and `last_updated_by`/`author` are isolated into independent ACs.
- ✅ Implementation point (`FrontmatterChips.tsx`) named in Technical Notes without leaking implementation detail into the behavioural criteria.
- ✅ Frontmatter `kind:` adoption is reconciled against ADR-0033 in Drafting Notes and References, eliminating a likely reader question.

### Recommended Changes

1. **Add a measurable threshold for the zero-chip container** (addresses: Clarity/Testability "Empty container" theme)
   In the third Requirements bullet and the fourth AC, replace "preserves header rhythm without leaving the H1 sitting on the divider" with an observable condition — e.g. "the chip-strip container renders with the same vertical height as a one-chip strip" or "the container element is present in the DOM with a fixed minimum height equal to chip line-height".

2. **Specify the 12-doc-kind verification mechanism** (addresses: Clarity/Testability "verification across 12 kinds" theme)
   Add a sub-bullet to the final AC such as: "a parameterised test in `FrontmatterChips.test.tsx` iterates the 12 kinds with fixture frontmatter containing one extra non-canonical key each, asserting that key never appears as a chip."

3. **Disambiguate the overloaded `date` term** (addresses: Clarity "Overloaded use of 'date'")
   On first use in each AC, write "`date` chip (sourced from the `date` frontmatter key)" or similar. Optionally introduce a one-line glossary in Drafting Notes naming the three slots (`status`/`date`/`author` chips) versus the frontmatter keys.

4. **Add an explicit ordering AC for subsets** (addresses: Testability "No criterion asserts ordering for subsets")
   Add: "Given any subset of {status, date, author} present, when the page renders, then the chips appear in canonical order with no gaps, regardless of which keys are missing."

5. **Extend the missingness rule to cover whitespace-only strings** (addresses: Testability "Whitespace-only string handling")
   Update the Assumption to either treat trimmed-empty strings as missing (and add an AC), or state that whitespace renders literally.

6. **Promote the schema-alignment follow-up to a directional entry** (addresses: Dependency "Schema-alignment story relationship")
   Reword the Related entry to "enables (once created): a future schema-alignment story (TBD)" so the directional coupling is visible. When that story gets an ID, populate Blocks.

7. **Surface unresolved items in a dedicated Open Questions section** (addresses: Completeness "No Open Questions section")
   Add a short section listing the schema-alignment TBD and the ADR-0033 frontmatter-alignment question (recommendation 8) in one place.

8. **Either add the ADR-0033 base frontmatter fields or note the deferral** (addresses: Completeness "Frontmatter omits ADR-0033 base fields")
   Add at minimum `id`, `type: work-item`, `schema_version: 1` to the frontmatter, or add a Drafting Note explicitly deferring full alignment to the corpus migration epic.

9. **Optionally surface ADR-0033 under Dependencies** (addresses: Dependency suggestion)
   Add "Depends on (schema): ADR-0033 (unified base frontmatter schema)" so the schema-source coupling is visible alongside work-item couplings.

10. **Soften the prototype-cap framing** (addresses: Clarity prototype-cap nit)
    Change "The prototype's chip row is hard-limited to four chips" to "The prototype's chip row caps at four chips" to mirror the source design-gap wording exactly.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item communicates its intent with high precision: it names the component, defines the allowed chip set, fixes the order, and reconciles its divergence from the prototype explicitly. A few minor referent and consistency issues exist — notably the ambiguous `date` chip semantics where `date` is used both as a frontmatter key and a chip label, and a small drift between the Context's stated three-chip cap and the source document's four-chip cap.

**Strengths**:
- Chip order and contents are stated unambiguously and repeated consistently across Summary, Requirements, and Acceptance Criteria.
- The divergence from the prototype's four-chip cap is explicitly named and justified.
- The "missing" definition is pinned down in Assumptions.
- The `kind`/`type` rename is reconciled in Drafting Notes and References.
- Caller-vs-component responsibility is stated actively.

**Findings**:
- Minor / high: Overloaded use of "date" as both key name and chip name (Acceptance Criteria).
- Minor / medium: "Empty container that still occupies the subtitle slot" lacks an observable definition (Requirements).
- Minor / medium: "Verified across the 12 doc kinds" conflates enumeration with verification mechanism (Acceptance Criteria).
- Minor / low: Prototype cap restated inconsistently between source doc and work item (Context).

### Completeness

**Summary**: Work item 0084 is structurally complete for a story: all expected sections are present and substantively populated, with a clear user-voiced summary, kind-appropriate context, seven specific acceptance criteria, populated dependencies and assumptions, and technical notes that point to the implementation surface. Minor gaps exist around a missing Open Questions section and the absence of several base fields specified by ADR-0033.

**Strengths**:
- Summary in proper story form with a follow-on action statement.
- Context with concrete examples and prototype-divergence justification.
- Seven Given/When/Then ACs covering the cap, ordering, omission, zero-chip case, precedence rules, and corpus-wide verification.
- Assumptions explicitly address what "missing" means.
- Dependencies enumerate blockers/related/blocks with `status: done` cross-check.
- Drafting Notes capture rationale.
- Frontmatter is well-formed with recognised kind/status/priority.

**Findings**:
- Minor / medium: No Open Questions section despite an explicit unresolved follow-up.
- Minor / medium: Frontmatter omits several ADR-0033 base fields.

### Dependency

**Summary**: The work item handles its dependencies competently: the three named prerequisites (0038, 0078, 0081) are all listed with their `status: done` confirmed, leaving Blocked-by correctly empty. The only borderline gap is the directionality of the relationship to the future schema-alignment story.

**Strengths**:
- Upstream blockers explicitly cross-checked with `status: done`.
- The future schema-alignment story is named in Dependencies and Drafting Notes; its out-of-scope boundary is stated in Assumptions.
- No third-party APIs, vendor services, or cross-team actions introduced.
- ADR-0033 named in References as rationale for the `kind`/`type` migration.

**Findings**:
- Minor / medium: Schema-alignment follow-up story relationship is one-directional but recorded only as "Related" (Dependencies).
- Suggestion / low: ADR-0033 schema coupling not surfaced in Dependencies.

### Scope

**Summary**: Work item 0084 describes a single, tightly bounded change to `FrontmatterChips`: enforce a three-chip whitelist with a fixed order. Scope is coherent — every requirement, AC, and technical note serves the same purpose — and explicit out-of-scope notes further sharpen the boundary. Sizing is appropriate for a story.

**Strengths**:
- Single coherent purpose: every requirement and AC describes capping/whitelisting chips in `FrontmatterChips`.
- Explicit out-of-scope notes for schema-alignment, verdict-in-frontmatter-table, and StatusBadge changes.
- Cap enforced inside the component itself; no caller opt-in.
- Cleanly delineated against sibling stories (0038, 0078, 0081).
- Single team / single bounded context.
- Deliberate divergence from the source design-gap doc documented in Context and Drafting Notes.

**Findings**: none.

### Testability

**Summary**: The work item is well-framed for verification: criteria are written in Given/When/Then form, define observable outcomes, and specify concrete inputs. One criterion uses partially unbounded scope but is bounded by an enumerated list of 12 doc kinds. A small number of measurability gaps remain.

**Strengths**:
- Given/When/Then framing throughout with concrete preconditions and observable outcomes.
- Non-canonical-key exclusion criterion enumerates the 12 doc kinds.
- "Missing" defined explicitly (absent, null, empty string), with `0` / `false` edge cases excluded.
- Precedence rules (`last_updated` vs `date`, `last_updated_by` vs `author`) isolated into independent ACs.
- Implementation-detail point kept in Technical Notes rather than ACs.

**Findings**:
- Minor / high: "H1 no longer sits on the divider" lacks a measurable threshold (Acceptance Criteria 4th bullet).
- Minor / medium: "Verified across 12 doc kinds" does not specify the verification procedure (Acceptance Criteria last bullet).
- Minor / medium: Whitespace-only string handling not specified (Assumptions / Acceptance Criteria 2nd bullet).
- Minor / low: No criterion asserts ordering when only two of the three keys are present (Acceptance Criteria).
