---
date: "2026-06-02T11:15:00+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0093-extend-templates-with-typed-linkage-slots.md"
work_item_id: "0093"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
---

## Work Item Review: Extend Templates With Typed-Linkage Slots

**Verdict:** REVISE

The work item is structurally complete, well-cross-referenced, and describes a single coherent unit of work with appropriate sizing as a story. Most sections are concrete and prescriptive. However, three Open Questions (universal `parent`, inverse-key exposure, ordering vs 0066) bleed into Requirements, Acceptance Criteria, Dependencies, and the testable surface — leaving the in-scope template set and the expected slot set unfixed. Until those are resolved (or explicitly deferred with a defined resolution point), an implementer or verifier cannot determine pass/fail purely from the work item.

### Cross-Cutting Themes

- **Open Questions leak into requirements and AC** (flagged by: clarity, scope, testability) — The three Open Questions each materially change what gets built and what verifies it. Clarity flags that the per-type `parent` list contradicts the open question; scope flags that landing order with 0066 changes the in-scope template count; testability flags that the unresolved questions leave the expected slot set unfixed.
- **ADR-0034 type-pair table cited as authority but not pinned as fixture** (flagged by: clarity, testability, dependency) — AC #1 and #4 defer correctness to an external table; the work item does not pin the expected per-template slot set as a stable, verifiable artefact. ADR-0034 itself appears only transitively in Dependencies (via 0061).

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: Requirement list and open question disagree on whether `parent` is universal
  **Location**: Requirements / Open Questions
  The per-type expected-additions bullets list `parent` only on some types (adr, codebase-research, rca); the Open Question asks whether `parent` should be added universally per ADR-0034's corpus-wide note. A reader cannot tell whether Requirements is the intended outcome or a placeholder pending the open question's resolution.

- 🟡 **Dependency**: Ordering constraint with 0066 acknowledged but unresolved in Dependencies
  **Location**: Dependencies / Open Questions
  Open Questions notes that 0093 should land after 0066 so it can sweep the full template surface in one pass, but Dependencies lists 0066 as a flat "Blocked by" without expressing this sequencing nuance. A scheduler reading only Dependencies may pull 0093 in before 0066 ships, forcing partial coverage or rebase.

- 🟡 **Testability**: AC #1 and #4 defer to ADR-0034's type-pair table without pinning the verifiable set
  **Location**: Acceptance Criteria
  AC #1 and #4 both delegate the pass/fail set to ADR-0034's external table. Requirements §2 lists expected additions but qualifies them with "at minimum" and "the implementer reconciles … and surfaces any divergence" — leaving the verifier without a fixed expected-set. A reviewer cannot determine pass/fail purely from the work item.

- 🟡 **Testability**: Unresolved open questions leave the testable surface ambiguous
  **Location**: Open Questions
  Two open questions directly determine which slots are expected: universal `parent` and inverse-key exposure. Until they are resolved, the expected slot set is unfixed; AC #1's pass/fail depends on which resolution is chosen.

#### Minor

- 🔵 **Clarity**: AC #4 actor is implicit
  **Location**: Acceptance Criteria
  AC #4 uses passive voice and doesn't say who verifies — automated test, human reviewer, or both. Two readers could disagree on satisfaction. Either extend AC #2 to include the negative assertion or restate AC #4 in terms of the test.

- 🔵 **Clarity**: Reference to "ADR-0034 §Design-gap inventory keys" may not resolve
  **Location**: Acceptance Criteria
  AC #5 cites a section heading the work item does not quote or anchor; if ADR-0034's section is named differently the reader cannot verify the criterion's grounding.

- 🔵 **Clarity**: "Each consuming SKILL.md" has no enumerated set
  **Location**: Requirements (third bullet)
  The skill list isn't enumerated; two implementers may derive different lists and produce partial coverage.

- 🔵 **Clarity**: TSV extension shape stated ambiguously
  **Location**: Requirements (fourth bullet) / Technical Notes
  Requirements says "new column **or** extras-list entries"; Technical Notes recommends the seventh column as "cleanest" but stops short of mandating. PR reviewers may push back either way.

- 🔵 **Dependency**: Future visualiser-graph epic named as consumer but not in Blocks
  **Location**: Context / Dependencies
  If the visualiser-graph epic is a tracked work item, it should appear in Blocks; if not yet drafted, the language should soften to "future epic (TBD)".

- 🔵 **Dependency**: ADR-0034 not surfaced as a structural dependency in its own right
  **Location**: Dependencies
  ADR-0034 appears only transitively via 0061. Promoting it to a first-class Dependencies entry (with a note that amendments mid-flight trigger re-audit) would make the binding contract explicit.

- 🔵 **Testability**: AC #3 (SKILL.md guidance) lacks a concrete verification procedure
  **Location**: Acceptance Criteria
  "Names every new slot" and "one-line note" are reviewable by reading but not scripted. Two reviewers could disagree on satisfaction. Add a grep-based check or tighten the phrasing.

- 🔵 **Testability**: Inline-comment form is exemplified but not pinned to a regex/grammar
  **Location**: Requirements
  The example comment is illustrative, not normative; AC #2(c) becomes interpretation-dependent. Specify the comment grammar normatively for single-ref and list keys.

- 🔵 **Testability**: AC #4 (negative assertion) needs an explicit test mechanism
  **Location**: Acceptance Criteria
  The described test asserts presence of expected slots but not absence of unexpected ones; AC #4 risks being unverified by automation. Add a negative-assertion sub-bullet to AC #2.

#### Suggestions

- 🔵 **Clarity**: Story 0070 reference is truncated
  **Location**: References
  `0070-...` doesn't resolve to a working path; replace with the full filename.

- 🔵 **Scope**: Consuming SKILL.md update spans many skills — verify it stays one increment
  **Location**: Requirements / Acceptance Criteria
  ~12 SKILL.md edits could quietly dominate the work. Confirm in Technical Notes that the sweep is mechanical (reusing 0065's canonical snippet), or carve into a follow-up.

- 🔵 **Scope**: Open question on landing order with 0066 affects scope surface
  **Location**: Open Questions
  Resolving the ordering question before planning lets AC #1 enumerate the in-scope template set unconditionally.

### Strengths

- ✅ Frontmatter is well-formed: kind=story, status=draft, priority=medium, parent linkage, tags, schema_version all present and recognised.
- ✅ Summary, Context, Requirements, and Acceptance Criteria all converge on the same narrow scope: empty optional slots, no corpus migration, no new ADR.
- ✅ Cross-references use stable identifiers (ADR-0034, stories 0057/0061/0065/0066/0070, AC #114) rather than vague pronouns.
- ✅ Requirements §1 specifies exact slot shape (`<key>: ""` vs `<key>: []`) and inline-comment form, enabling string-level assertions.
- ✅ Requirements §2 enumerates expected per-template additions, giving the test a concrete fixture starting point.
- ✅ Explicit non-goals (no corpus migration, no new ADR) sharpen the boundary and prevent bleed into 0066/0070.
- ✅ Dependencies names upstream blockers (0061/0065/0066) and downstream consumer (0070) with rationale.
- ✅ Parent epic 0057's specific AC (#114) is cited as the gap this story closes — a precise cross-reference.
- ✅ AC #5 carves out a specific verifiable exception (`current_inventory`/`target_inventory` retained on design-gap).
- ✅ Open Questions surfaces three real ambiguities each with a stated recommendation, rather than dangling.
- ✅ Drafting Notes gives an implementer enough context to start without follow-up, including the rationale for priority and kind.

### Recommended Changes

1. **Resolve the three Open Questions in the work item itself** (addresses: "parent universality contradicts requirements", "unresolved open questions leave testable surface ambiguous", "landing order affects scope surface")
   Pick the resolution for each and update the body accordingly: (a) decide universal `parent` vs table-driven and update Requirements §2 to match; (b) decide whether inverse slots are exposed and update §2; (c) commit to "lands after 0066" and remove the conditional language. Move the resolved questions into Context or Decisions Made; delete the Open Questions section if all three resolve.

2. **Promote the 0066 ordering constraint into Dependencies** (addresses: "ordering with 0066 unresolved in Dependencies")
   Replace the flat "Blocked by: 0066" entry with "Blocked by: 0066 (must ship first so 0093 can sweep all three review templates in one pass)" so a scheduler reading only Dependencies sees the sequencing.

3. **Pin Requirements §2 as the authoritative expected slot set** (addresses: "AC #1/#4 defer to external table without pinning")
   Drop the "at minimum" hedge and the "implementer reconciles … surfaces any divergence" line. Make Requirements §2 the canonical fixture; restate AC #1 as "every template carries exactly the slots listed in Requirements §2"; restate AC #4 to assert the test enforces this as a closed set.

4. **Add ADR-0034 as a first-class Dependencies entry** (addresses: "ADR-0034 not surfaced as structural dependency")
   Add an "External artefact" entry under Dependencies naming ADR-0034 as the authoritative table, with a note that mid-flight amendments trigger re-audit. Removes transitive dependence via 0061.

5. **Make AC #4 mechanically verifiable** (addresses: "AC #4 actor implicit", "AC #4 negative assertion needs test mechanism")
   Extend AC #2's sub-checks with an explicit "(d) no template carries a linkage key not listed in its TSV row" assertion, and remove AC #4 (or restate it as a one-line summary of (d)).

6. **Specify the inline-comment grammar normatively** (addresses: "inline-comment form not pinned to grammar")
   Replace the illustrative example with normative forms for single-ref and list keys, e.g., `# typed-linkage ref: "<source-type>:NNNN" or ""` and `# typed-linkage list: ["<source-type>:NNNN", ...] or []`. Require the test to assert exactly this shape.

7. **Decide the TSV extension shape** (addresses: "TSV extension shape stated ambiguously")
   Pick the seventh-column approach in Requirements §4 (matching Technical Notes' recommendation) and drop the "or extras-list entries" alternative.

8. **Enumerate consuming SKILL.md files** (addresses: "each consuming SKILL.md has no enumerated set", "SKILL.md update spans many skills")
   Either list the affected SKILL.md paths inline under Requirements §3, or explicitly cite 0065's plan section by name as the authoritative list. Frame the sweep as mechanical (one canonical snippet, ~12 sites).

9. **Resolve the visualiser-graph forward reference** (addresses: "future visualiser-graph epic named but not in Blocks")
   If the epic has a work-item ID, add it to Blocks; if not yet drafted, soften Summary/Assumptions to "a future epic (TBD)" so the reference doesn't imply a tracked dependency.

10. **Fix the truncated 0070 reference and the ADR-0034 section anchor** (addresses: "0070 reference truncated", "ADR-0034 §Design-gap inventory keys may not resolve")
    Replace `0070-...` with the full filename; either quote the relevant ADR-0034 line inline at AC #5, or replace the heading reference with a stable anchor.

11. **Tighten AC #3 to a scripted check** (addresses: "AC #3 lacks concrete verification procedure")
    Either add a grep-based assertion (each new slot name appears in the Populate-frontmatter section of every consuming SKILL.md), or tighten the AC phrasing to a verifiable form including required keywords like "fill" or "leave empty".

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear and well-structured, with explicit references to the authoritative ADR and predecessor stories. Most requirements name a clear actor and the scope is internally consistent. A few ambiguities remain — notably one open question whose resolution affects the requirement list, one passive construction that hides responsibility, and one section heading ambiguity for the design-gap keys.

**Strengths**:
- Summary, Context, Requirements, and Acceptance Criteria all describe the same scope.
- Cross-references use stable identifiers (ADR-0034, stories 0057/0061/0065/0066/0070, AC #114).
- Requirements §1 defines exact slot shape and comment form.
- Open Questions explicitly flags unresolved decisions.

**Findings**:
- 🟡 **Major** (high confidence): Requirement list and open question disagree on whether `parent` is universal.
- 🔵 **Minor** (high confidence): AC #4 actor is implicit.
- 🔵 **Minor** (medium confidence): Reference to "ADR-0034 §Design-gap inventory keys" may not resolve.
- 🔵 **Minor** (medium confidence): "Each consuming SKILL.md" has no enumerated set.
- 🔵 **Minor** (medium confidence): TSV extension shape stated ambiguously ("new column or extras-list entries" vs "seventh column" recommendation).
- 🔵 **Suggestion** (medium confidence): Story 0070 reference is truncated (`0070-...`).

### Completeness

**Summary**: Work item 0093 is a story that is structurally complete and substantively populated across all expected sections. Frontmatter is well-formed; summary clearly states intent; context explains motivation by tracing the gap left by predecessor stories; acceptance criteria, requirements, dependencies, assumptions, and open questions are all populated with kind-appropriate content.

**Strengths**:
- Frontmatter integrity solid (kind, status, priority, parent, tags, schema_version).
- Summary is a single unambiguous statement.
- Context enumerates what 0065/0066/0070 cover and identifies the specific gap (AC #114).
- Five distinct acceptance criteria covering template content, test coverage, SKILL.md updates, ADR conformance, design-gap preservation.
- Requirements are concrete and prescriptive with exact key names and cardinalities.
- Open Questions surfaces three real ambiguities, each with a recommendation.
- Dependencies populated with Blocked by / Blocks / Related.
- Assumptions, Technical Notes, Drafting Notes give enough context to start without follow-up.

**Findings**: _None._

### Dependency

**Summary**: The work item captures its primary couplings well: upstream blockers (0061, 0065, 0066) and downstream consumer (0070) are named explicitly with ADR-0034 cited as the authoritative source. Main gaps: the ordering ambiguity with 0066 is acknowledged in Open Questions but not resolved in Dependencies, and the future visualiser-graph epic is a named consumer that doesn't appear in Blocks.

**Strengths**:
- Upstream blockers named with rationale (0061/0065/0066).
- Downstream consumer 0070 captured in Blocks.
- Parent epic 0057 named with specific AC (#114).
- Assumptions explicitly call out ADR-0034's type-pair table as authoritative.

**Findings**:
- 🟡 **Major** (high confidence): Ordering constraint with 0066 acknowledged but unresolved in Dependencies.
- 🔵 **Minor** (medium confidence): Future visualiser-graph epic named as consumer but not listed in Blocks.
- 🔵 **Minor** (medium confidence): ADR-0034 not surfaced as a structural dependency in its own right (only transitively via 0061).

### Scope

**Summary**: Work item 0093 describes a single coherent unit: extending the existing template surface with empty typed-linkage slots per ADR-0034. Requirements, AC, and Summary all converge on the same narrow scope. Sizing as a story is appropriate, with explicit boundaries that exclude corpus migration (deferred to 0070) and new ADR work.

**Strengths**:
- Summary, Requirements, and AC describe the same scope.
- Explicit non-goals (no corpus migration, no new ADR) sharpen the boundary.
- Dependency direction is clear (blocked by 0061/0065/0066, blocks 0070).
- Sizing as `story` fits the work shape.

**Findings**:
- 🔵 **Suggestion** (medium confidence): Consuming SKILL.md update spans ~12 skills — verify it stays one increment.
- 🔵 **Suggestion** (medium confidence): Open question on landing order with 0066 affects scope surface; resolve before planning.

### Testability

**Summary**: The work item provides a strong verification surface: most acceptance criteria map to deterministic checks against template files and the existing template-shape test. However, several criteria lean on ADR-0034's type-pair table as authority without enumerating the expected per-template slot set in the AC itself, and the AC for SKILL.md updates is partially subjective.

**Strengths**:
- AC #2 ties verification to `scripts/test-template-frontmatter.sh` passing with explicit sub-checks.
- Requirements §1 specifies exact emission shape (single-ref vs list, with comment form).
- Requirements §2 enumerates expected per-template additions.
- AC #5 specifies a verifiable carve-out for `current_inventory`/`target_inventory`.
- Technical Notes §1 names the precise extension mechanism (seventh TSV column, cardinality map).

**Findings**:
- 🟡 **Major** (high confidence): AC #1 and #4 defer to ADR-0034's type-pair table without pinning the verifiable set.
- 🟡 **Major** (high confidence): Unresolved open questions (universal `parent`, inverse keys) leave the testable surface ambiguous.
- 🔵 **Minor** (medium confidence): AC #3 (SKILL.md guidance) lacks a concrete verification procedure.
- 🔵 **Minor** (medium confidence): Inline-comment form is exemplified but not pinned to a regex/grammar.
- 🔵 **Minor** (medium confidence): AC #4 (negative assertion) needs an explicit test mechanism.

## Re-Review (Pass 2) — 2026-06-02T11:07:20+00:00

**Verdict:** COMMENT

All four major findings from Pass 1 are resolved. The Decisions Made section pins the previously-deferred Open Questions (universal `parent`, inverse-key handling, landing order vs 0066). Requirements §2 is now an explicitly closed set against which the template-shape test asserts. AC #4 is folded into AC #2(d) as a closed-set negative assertion; the inline-comment grammar is normative; AC #3 names a grep-based check. ADR-0034 is promoted to a first-class Dependencies entry with a re-audit trigger. The remaining findings are all minor polish concentrated on one cross-cutting issue (the conditional handling of design-inventory/design-gap producer skills) plus two small grammar-precision concerns.

### Previously Identified Issues

- 🟡 **Clarity**: Requirement list and open question disagree on whether `parent` is universal — **Resolved** (Decisions Made pins universal `parent`; Requirements §2 enumerates it on every template type)
- 🟡 **Dependency**: Ordering constraint with 0066 acknowledged but unresolved in Dependencies — **Resolved** (Decisions Made records 0066 as done; Dependencies confirms all twelve templates reachable in one sweep)
- 🟡 **Testability**: AC #1 and #4 defer to ADR-0034's type-pair table without pinning the verifiable set — **Resolved** (Requirements §2 is the closed set; AC #1 references it explicitly)
- 🟡 **Testability**: Unresolved open questions leave the testable surface ambiguous — **Resolved** (Decisions Made pins both `parent` and inverse-key resolutions)
- 🔵 **Clarity**: AC #4 actor is implicit — **Resolved** (folded into AC #2(d) as the closed-set test assertion)
- 🔵 **Clarity**: Reference to "ADR-0034 §Design-gap inventory keys" may not resolve — **Resolved** (AC #4 now quotes the ADR-0034 line inline)
- 🔵 **Clarity**: "Each consuming SKILL.md" has no enumerated set — **Mostly resolved** (Requirements §3 enumerates ~13 SKILL.md paths; design-inventory/design-gap producers remain conditional — flagged anew below)
- 🔵 **Clarity**: TSV extension shape stated ambiguously — **Resolved** (seventh-column form is mandatory; extras-list alternative explicitly rejected)
- 🔵 **Clarity**: Story 0070 reference truncated — **Resolved** (full filename `0070-ship-meta-corpus-unified-schema-migration.md`)
- 🔵 **Dependency**: Future visualiser-graph epic named as consumer but not in Blocks — **Resolved** (softened to "TBD, not yet tracked" in Summary and Dependencies)
- 🔵 **Dependency**: ADR-0034 not surfaced as a structural dependency in its own right — **Resolved** (promoted to a first-class "External artefact" Dependencies entry with re-audit trigger)
- 🔵 **Testability**: AC #3 lacks a concrete verification procedure — **Resolved** (grep-based check named explicitly; required tokens "fill" / "leave empty" specified)
- 🔵 **Testability**: Inline-comment form not pinned to a regex/grammar — **Resolved** (Requirements §1 specifies normative grammar per cardinality with literal templates)
- 🔵 **Testability**: AC #4 negative assertion needs an explicit test mechanism — **Resolved** (folded into AC #2(d) as the closed-set assertion)
- 🔵 **Scope**: Consuming SKILL.md update spans many skills — **Partially resolved** (mechanical-sweep framing added with "per-skill divergence not permitted"; scope-size concern persists at suggestion level — see new findings)
- 🔵 **Scope**: Open question on landing order with 0066 affects scope surface — **Resolved** (0066 done; scope is unconditional)

### New Issues Introduced

- 🔵 **Clarity / Dependency / Testability** (cross-cutting): design-inventory/design-gap producer-skill identity is conditional ("as they exist; if there is no producer skill yet, the template update alone satisfies the requirement"). Three lenses flagged this independently. AC #3's grep check cannot be unambiguously evaluated for these two types without knowing whether a producer skill exists. **Suggestion**: Resolve at draft time — list the producer-skill paths explicitly, or state plainly that no producer skill exists today and the template update alone satisfies the requirement.
- 🔵 **Clarity**: "Normative comment grammar" referent in Requirements §1 is not labelled as a discrete block — the test author must infer which exact strings constitute the regex's target. **Suggestion**: Extract the literal comment templates into a small labelled block or table.
- 🔵 **Clarity**: "Consumers derive the inverse" in Decisions Made names plural unnamed actors. **Suggestion**: Add one clause clarifying that derivation is out of scope for this story (downstream consumer concern).
- 🔵 **Testability**: Comment grammar's regex tolerance for the `<source-type>` token is not specified — could be a free word, a curated type-name set, or matched to the template's own type. **Suggestion**: Fix the tolerance in one sentence in Requirements §1.
- 🔵 **Scope** (suggestion, medium): SKILL.md sweep spans ~13 sites. Bundling is defensible (slots are useless without producer guidance) but the diff surface is at the upper end of "single story". **Suggestion**: Note in Implementation Approach that templates land first and SKILL.md updates follow within the same story, so partial rollback of the sweep doesn't require reverting templates.

### Assessment

The work item is ready for implementation. All Pass 1 majors are resolved; the verdict moves from REVISE to COMMENT. The five remaining findings are minor polish (no criticals, no majors). The strongest cross-cutting concern — the design-inventory/design-gap producer-skill conditional — is worth resolving before planning starts but doesn't block the story; it's an in-flight check at most. The remaining items can be addressed during planning or implementation without re-opening the work item structurally.

## Final Pass (Pass 3) — 2026-06-02T11:15:00+00:00

**Verdict:** APPROVE

The cross-cutting design-inventory/design-gap producer-skill conditional has been resolved by inspecting the codebase and updating Requirements §3:

- `skills/design/inventory-design/SKILL.md` (design-inventory producer) — confirmed to exist
- `skills/design/analyse-design-gaps/SKILL.md` (design-gap producer) — confirmed to exist
- Both are now named explicitly in Requirements §3; the conditional parenthetical has been removed.
- Six other SKILL.md paths in the list were corrected to match actual directory layout (`skills/planning/...`, `skills/github/...`, etc. — the earlier paths were guessed at draft time and have been verified against the filesystem).
- The closed-set list now contains fifteen sites with an inline annotation per skill naming the artifact type it produces.
- Technical Notes' site-count reference updated from "~13" to "fifteen".

The remaining four polish findings (normative-grammar labelling, "consumers derive" actor naming, regex tolerance for `<source-type>`, SKILL.md sweep rollback note) are all minor / suggestion severity. They are not blockers; they can be addressed during planning or implementation if the planner finds them load-bearing.

Work item status transitions from `draft` to `ready`. Review verdict is APPROVE.
