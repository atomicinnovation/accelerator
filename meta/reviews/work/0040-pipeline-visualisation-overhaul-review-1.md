---
date: "2026-05-31T03:00:00+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0040"
work_item_id: "0040"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
id: "0040-pipeline-visualisation-overhaul-review-1"
title: "0040-pipeline-visualisation-overhaul-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-31T03:00:00+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: Pipeline Visualisation Overhaul

**Verdict:** REVISE

The work item is strongly written across clarity, completeness, dependency, and scope — every section is substantively populated, frontmatter is valid, and reasoning about scope/dependencies is explicit. The two issues that push it to REVISE are both in the Acceptance Criteria: the cross-surface consistency criterion and the test-migration criterion describe activity or alignment rather than concrete observable outcomes a tester can verify. Several smaller clarity/testability findings cluster around two themes: imprecise positional language ("top row" vs "footer") and the undefined meaning of "active" stage styling.

### Cross-Cutting Themes

- **Kanban card element placement is ambiguous** (flagged by: clarity, testability) — Requirements say `PipelineMini` and the namespaced ID sit in the card top row; Acceptance Criteria places the "N linked" label "in the card footer"; no source defines either region.
- **"Active" stage styling lacks a defined observable** (flagged by: clarity, testability) — the term is used in criteria and notes but never given a DOM signal (class, attribute, computed property) that tests can probe deterministically.
- **A few criteria reference implementation rather than observable behaviour** (flagged by: testability) — "must not call `parseWorkItemId`", "same logic as `GET /api/related/{path}`", and "tests are migrated" each shift verification onto code structure or activity rather than observable outcome.

### Findings

#### Major
- 🟡 **Testability**: Cross-surface consistency criterion lacks a concrete verification probe
  **Location**: Acceptance Criteria (cross-surface consistency)
  The criterion says all three surfaces "reflect the new state consistently" without defining how the change is triggered or what "consistently" means observably. A tester has no defined procedure.
- 🟡 **Testability**: Test-migration criterion describes activity, not a verifiable outcome
  **Location**: Acceptance Criteria (test migration bullet)
  "Tests are migrated to cover the new components and assertions" is tautological — could be claimed met by any rename. Doesn't enumerate the assertions the new tests must contain.

#### Minor
- 🔵 **Clarity**: Term 'active' for stage styling is used but never defined
  **Location**: Acceptance Criteria / Requirements
  Used in "styled as active", "active tiles", "active dots" without specifying the visual or attribute signal.
- 🔵 **Clarity / Testability**: Conflicting placement of 'N linked' label (top row vs footer)
  **Location**: Requirements vs Acceptance Criteria
  Requirements describe `PipelineMini` "in the card top row alongside the namespaced ID"; Acceptance Criteria places "{N} linked" "in the card footer". No source defines either region.
- 🔵 **Clarity**: Behaviour for work items without a namespaced prefix is unspecified
  **Location**: Requirements / Acceptance Criteria (ID rendering)
  `entry.workItemId` is `string | null` — what does the card render when null or when the file matches `work.id_pattern` without a prefix?
- 🔵 **Dependency**: Implied coupling to 0063 (kind/type rename) not captured
  **Location**: Dependencies
  Context says `frontmatter.kind` was "renamed from `type` per work item 0063"; 0063 only appears in References, not Dependencies.
- 🔵 **Dependency**: Internal ordering between server enrichment and frontend work not stated
  **Location**: Requirements
  `IndexEntry` server enrichment is a prerequisite for the frontend AC, but the sequencing is implicit.
- 🔵 **Dependency**: Potential token-system follow-up to 0033 noted but not captured as a dependency
  **Location**: Technical Notes
  The conditional `--ac-stage-<key>-on` token decision is buried in Technical Notes; not surfaced anywhere actionable.
- 🔵 **Scope**: Server-side `IndexEntry` enrichment is an orthogonal sub-concern bundled into a UI story
  **Location**: Requirements
  Backend indexer change has its own deployment profile; could ship as a precursor story.
- 🔵 **Testability**: "Must not call `parseWorkItemId`" is implementation check, not observable
  **Location**: Acceptance Criteria (ID rendering)
  Phrased as implementation prohibition; better expressed positively.
- 🔵 **Testability**: Active-stage styling assertion lacks defined observable signal
  **Location**: Acceptance Criteria
  No specified class, attribute, or DOM hook for "styled as active".
- 🔵 **Testability**: "At the top of the card" is informal positional probe
  **Location**: Acceptance Criteria (kanban PipelineMini placement)
  No machine-checkable signal (DOM order, container).
- 🔵 **Testability**: "Same logic as GET /api/related/{path}" is implementation reference
  **Location**: Acceptance Criteria (linkedCount derivation)
  Verifiable in principle but framed as alignment rather than observable equality.

#### Suggestions
- 🔵 **Clarity**: 'Panel' between page header and timeline referenced but not defined
  **Location**: Requirements / Acceptance Criteria
- 🔵 **Dependency**: Several Related items (0041, 0044, 0045, 0057, 0061, 0078) listed without role
  **Location**: References
- 🔵 **Scope**: Namespaced ID switch is independently deliverable from pipeline overhaul
  **Location**: Requirements

### Strengths

- ✅ Frontmatter is complete and well-formed (kind, status, priority, tags, author, date)
- ✅ Summary opens with explicit user-role/value statement and follows with concrete component/surface description
- ✅ 12 Given/When/Then acceptance criteria with explicit triggers and outcomes — well above the two-criterion floor
- ✅ Most domain terms are anchored to file paths and line numbers (`Completeness`, `WORKFLOW_PIPELINE_STEPS`, `IndexEntry`, `parseWorkItemId`)
- ✅ Negative cases (orphan cards, `linkedCount === 0`, no-navigation) explicitly enumerated and testable
- ✅ Data-flow criteria framed against named API/type shapes so the wire contract can be verified
- ✅ Scope-adjacency risks named explicitly (long-tail stages excluded, interactivity deferred, 0086 coordination flagged)
- ✅ Upstream blockers (0033, 0037, 0038) enumerated with status; downstream consumers (0079, 0083) captured as Blocks
- ✅ Drafting Notes record substantive interpretations (priority bump rationale, multi-prefix model, prototype kanban-card divergence)

### Recommended Changes

1. **Rewrite the cross-surface consistency criterion with a concrete probe** (addresses: Testability major #1)
2. **Replace the test-migration criterion with specific assertion targets** (addresses: Testability major #2)
3. **Define "active" once and use it consistently** (addresses: Clarity #1, Testability active-styling)
4. **Resolve the top-row vs footer placement contradiction** (addresses: Clarity placement, Testability positional)
5. **Specify behaviour when `entry.workItemId` is null** (addresses: Clarity ID fallback)
6. **Promote 0063 from References into Dependencies** (addresses: Dependency #1)
7. **State server-then-frontend sequencing explicitly** (addresses: Dependency #2)
8. **Reframe behaviour-coupled criteria as observable equality / positive tests** (addresses: Testability `parseWorkItemId`, Testability "same logic")
9. **Address the orthogonal-scope concerns in Drafting Notes** (addresses: Scope #1 + #2)
10. **Define "panel"** (addresses: Clarity suggestion)
11. **Either promote Related items into Dependencies with a role, or leave them as context** (addresses: Dependency suggestion)
12. **Capture the token follow-up to 0033 explicitly** (addresses: Dependency #3)

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually precise: actors, file paths, data sources, and component responsibilities are named explicitly throughout, with little reliance on ambiguous pronouns. A few minor clarity gaps exist around the meaning of 'active', the location of the 'N linked' label, and the treatment of namespaced IDs without a numeric prefix, but none rise to a blocking concern.

**Strengths**:
- Pronouns are nearly always anchored to a named subject (component, file path, or data field), so referent ambiguity is minimal.
- Each requirement names its actor or surface (Lifecycle cluster detail, Lifecycle index cluster cards, Kanban cards, the server indexer), making responsibility explicit.
- Domain terms like `Completeness`, `WORKFLOW_PIPELINE_STEPS`, `LONG_TAIL_PIPELINE_STEPS`, `IndexEntry`, and `parseWorkItemId` are linked to concrete file paths and line numbers, eliminating guesswork about referents.
- Cross-section scope is consistent: Summary, Requirements, Acceptance Criteria, and Technical Notes all describe the same three surfaces and same data flow.
- Drafting Notes explicitly call out terminology renames (`HexChain` → `Pipeline`, `StageDots` → `PipelineMini`) so readers do not confuse prototype names with production names.

**Findings**:
- (minor, medium) Term 'active' for stage styling is used but never defined — Acceptance Criteria (first bullet) / Requirements
- (minor, high) Conflicting placement of the 'N linked' label (card top row vs. card footer) — Requirements vs. Acceptance Criteria
- (minor, medium) Behaviour for work items without a namespaced prefix is unspecified — Requirements / Acceptance Criteria (ID rendering)
- (suggestion, medium) 'Panel' between page header and timeline is referenced but not defined — Requirements / Acceptance Criteria

### Completeness

**Summary**: Work item 0040 is a thoroughly populated story with every expected section present and substantively filled in. Frontmatter is complete and valid, the Summary clearly states the user-facing intent, and Acceptance Criteria, Requirements, Context, Dependencies, Assumptions, Technical Notes, and Drafting Notes all carry meaningful content appropriate for a story of this scope. No completeness gaps would block an implementer from picking up the work.

**Strengths**: (see strengths list above)

**Findings**: None.

### Dependency

**Summary**: The work item has strong dependency capture overall: upstream blockers (0033, 0037, 0038) are listed with status, downstream consumers (0079, 0083) are named as Blocks, and a parallel-work coordination risk (0086) is explicitly flagged. The main gaps are around the implied ordering between server-side IndexEntry enrichment and the frontend card work, and a couple of related items mentioned in References (0063 in particular) that have an implied dependency relationship not surfaced in Dependencies.

**Strengths**:
- Upstream blockers explicitly enumerated with status
- Downstream consumers 0079 and 0083 captured as Blocks, with priority rationale tying back
- Parallel-work coordination risk with 0086 on same `WorkItemCard` surface explicitly named
- External-data coupling (`/api/related/{path}` resolution) named with consistency requirement

**Findings**:
- (minor, high) Implied coupling to 0063 (kind/type rename) not captured — Dependencies
- (minor, high) Internal ordering between server enrichment and frontend work not stated — Requirements
- (minor, medium) Potential token-system follow-up to 0033 noted but not captured as a dependency relationship — Technical Notes
- (suggestion, medium) Several Related items (0041, 0044, 0045, 0057, 0061, 0078) listed without role — References

### Scope

**Summary**: Work item 0040 bundles several related changes (two new pipeline components, three surfaces consuming them, server-side IndexEntry enrichment with completeness and linkedCount, and a kanban ID/format change) under a single 'pipeline visualisation overhaul' theme. The bundling is largely defensible because all surfaces share the same data source and components, but the server-side IndexEntry enrichment and the namespaced-ID switch are arguably orthogonal sub-concerns that could be delivered independently. Overall the story is coherent for its declared kind, though it is at the upper edge of what a single story should carry.

**Strengths**:
- Drafting Notes explicitly address the bundling question and justify keeping the three surfaces in one story
- Summary, Requirements, and Acceptance Criteria describe the same three-surface scope consistently
- Long-tail pipeline stages explicitly excluded from the new chain
- Interactivity of Pipeline tiles explicitly deferred to a follow-up
- Coordination risk with 0086 on same `WorkItemCard` surface named in Dependencies

**Findings**:
- (minor, medium) Server-side IndexEntry enrichment is an orthogonal sub-concern bundled into a UI story — Requirements
- (suggestion, low) Namespaced ID switch is independently deliverable from the pipeline overhaul — Requirements

### Testability

**Summary**: The work item's Acceptance Criteria are largely well-formed observable Given/When/Then behaviours with explicit triggers, surfaces, and expected outcomes, and they map closely onto the Requirements and Summary. A few criteria embed implementation instructions rather than observable outcomes, one uses unbounded scope language without a concrete probe, and one references existing tests as a migration target without specifying the new behaviour those tests must cover.

**Strengths**:
- Most acceptance criteria follow clear Given/When/Then structure with concrete preconditions and observable expected outcomes
- Negative cases explicitly enumerated and testable
- Data-flow criteria framed against named API/type shapes
- Specific numeric and identifier expectations (eight `WORKFLOW_PIPELINE_STEPS`, N/8 counter, namespaced ID format)

**Findings**:
- (major, high) Cross-surface consistency criterion lacks a concrete verification probe — Acceptance Criteria (cross-surface consistency)
- (major, high) Test-migration criterion describes activity, not a verifiable outcome — Acceptance Criteria (test migration bullet)
- (minor, medium) Negative assertion 'must not call `parseWorkItemId`' is an implementation check, not an observable outcome — Acceptance Criteria (WorkItemCard ID rendering)
- (minor, medium) Active-stage styling assertion lacks a defined observable signal — Acceptance Criteria (Pipeline on cluster detail)
- (minor, medium) 'In the card footer' is unverified — Requirements say 'top row' while Technical Notes are silent on placement — Acceptance Criteria (linked-label rendering)
- (minor, high) 'At the top of the card' is the only positional probe and is informal — Acceptance Criteria (kanban PipelineMini placement)
- (minor, low) 'Same logic as GET /api/related/{path}' is an implementation reference, not a verifiable outcome — Acceptance Criteria (IndexEntry.linkedCount derivation)

## Re-Review (Pass 2) — 2026-05-31

**Verdict:** COMMENT

Both major testability findings have been resolved, and every minor/suggestion finding from Pass 1 has either been addressed or substantively acknowledged. The work item now reads cleanly across all four re-reviewed lenses (clarity, dependency, scope, testability — completeness was skipped as it had no findings in Pass 1). No findings rise above minor severity, so the REVISE threshold (critical or ≥2 major) is no longer met. A handful of new minor observations surfaced as the previous fixes brought sharper detail into focus, but none block implementation.

### Previously Identified Issues

#### Major (both resolved)
- ✅ 🟡 **Testability**: Cross-surface consistency criterion lacks a concrete verification probe — **Resolved**. New criterion uses a concrete worked example (`["work"]` → `["work","adr"]`) with `data-active` assertions.
- ✅ 🟡 **Testability**: Test-migration criterion describes activity, not a verifiable outcome — **Resolved**. Migrated out of Acceptance Criteria into Technical Notes as a per-file assertion checklist.

#### Minor / Suggestion
- ✅ 🔵 **Clarity**: Term 'active' for stage styling — **Resolved**. `data-active="true"|"false"` is the canonical observable, defined once in Requirements.
- ✅ 🔵 **Clarity / Testability**: Top-row vs footer placement contradiction — **Resolved**. `PipelineMini` anchored to `.ac-kcard__top` first-child; "N linked" anchored to `.ac-kcard__foot`.
- ✅ 🔵 **Clarity**: `workItemId` null behaviour — **Resolved**. Both Requirements and a dedicated AC now specify the slot is omitted.
- ✅ 🔵 **Clarity**: 'Panel' definition — **Resolved** at Requirements level (concrete styling pinned).
- ✅ 🔵 **Dependency**: 0063 coupling — **Resolved**. Promoted to Dependencies as "Builds on".
- ✅ 🔵 **Dependency**: Server-then-frontend sequencing — **Resolved**. Captured as "Sequencing" bullet under Dependencies.
- ✅ 🔵 **Dependency**: 0033 token follow-up — **Resolved**. Captured as "May spawn follow-up to 0033".
- ✅ 🔵 **Dependency**: Related items without role — **Resolved**. References split into context-only (with roles) vs dependency-coupled.
- ✅ 🔵 **Scope**: Server-side IndexEntry enrichment bundling — **Partially resolved**. Drafting Notes now justify the bundling decision; re-review acknowledges this is defensible.
- 🔵 **Scope**: Namespaced ID switch independently deliverable — **Partially resolved**. Drafting Notes record the decision not to split; re-review suggests an explicit fallback note if the switch hits issues.
- ✅ 🔵 **Testability**: 'Must not call `parseWorkItemId`' — **Resolved**. Reframed as positive observable test.
- ✅ 🔵 **Testability**: Active-stage styling observable — **Resolved**. `data-active` attribute is the canonical probe.
- ✅ 🔵 **Testability**: 'In the card footer' contradiction — **Resolved**.
- ✅ 🔵 **Testability**: 'At the top of the card' informal — **Resolved**. Now anchored as first child of `.ac-kcard__top`.
- ✅ 🔵 **Testability**: 'Same logic as GET /api/related' — **Resolved**. Reframed as observable equality.

### New Issues Introduced (or Newly Surfaced)

All minor or suggestion severity.

- 🔵 **Clarity**: "Orphan work item" used as a term without explicit definition — used parenthetically, but a reader could interpret "orphan" more broadly than `entry.completeness === null`.
- 🔵 **Clarity**: `completeness.present` shape referenced as if stable but Requirements hedges with "the existing `Completeness` shape or a derived `present: string[]`". Worth pinning whether `present` is a stable contract or a derivation.
- 🔵 **Clarity**: Connector colouring rule says "its neighbour" / "the next stage" without specifying direction in `WORKFLOW_PIPELINE_STEPS` order.
- 🔵 **Clarity** (suggestion): Pronoun "It" in Context (referring to `PipelineDots`) is mildly ambiguous after intervening clauses.
- 🔵 **Clarity** (suggestion): `data-active` rendering trigger implicit — initial render only vs. re-derived on prop change.
- 🔵 **Dependency**: 0044 (kanban scope spike) noted as "should resolve before kanban AC is locked" in References but not captured in Dependencies as a soft coordination/blocker entry.
- 🔵 **Dependency** (suggestion): `linkedCount` semantics bound to the current three-bucket `/api/related` shape from 0057/0061; not explicitly noted as a coordination point.
- 🔵 **Scope** (suggestion): Summary names two distinct deliverable axes (pipeline visualisation + `WorkItemCard` enrichments). Could be tightened so the unifying thread is self-evident from Summary alone.
- 🔵 **Testability**: Non-interactivity criterion only asserts no-click-navigation; Requirements promise no focus, no link element, no scroll. Worth a companion keyboard-traversal / element-tag criterion.
- 🔵 **Testability**: Cross-surface consistency uses "next loaded" without pinning the fetch trigger (hard reload vs route re-mount vs background refetch).
- 🔵 **Testability**: Panel styling specified in Requirements (padding, sunken background, eyebrow) has no corresponding observable AC — only "appears in a panel".
- 🔵 **Testability** (suggestion): `linkedCount` equality criterion uses "any indexed work item path P" without bounding the population (orphans, pending re-index).

### Assessment

The work item is ready for implementation as a coherent story. The two major findings that triggered REVISE are both fully resolved, and every Pass 1 minor or suggestion finding has been addressed or substantively acknowledged. The new findings cluster into three small groups (orphan/`present` definition pinning, 0044 sequencing acknowledgement, additional observable test probes for non-interactivity and panel styling) — none block picking up the story. They could be addressed in a small follow-up pass or deferred to planning.

---
*Re-review generated by /review-work-item*

## Re-Review (Pass 3) — 2026-05-31

**Verdict:** COMMENT (unchanged)

Pass 2 findings are largely resolved or substantively acknowledged. No criticals, no majors. The work item is converging — pass 1 had 2 majors + 10 minors + 3 suggestions, pass 2 had 0 majors + 8 minors + 4 suggestions, pass 3 has 0 majors + 7 minors + 4 suggestions. The remaining findings are increasingly granular polish issues that won't block implementation.

### Previously Identified Issues (Pass 2)

#### Clarity
- ✅ "Orphan" definition — **Resolved**
- 🔄 `completeness.present` shape — **Partially resolved**. Definition added, but Acceptance Criteria still reference `cluster.completeness.present` as if it's a wire field while Requirements leave the location ambiguous.
- ✅ Connector colour direction — **Resolved**
- ✅ "It" pronoun — **Resolved**
- ✅ `data-active` rendering trigger — **Resolved**

#### Dependency
- 🔄 0044 sequencing — **Partially resolved**. Moved to "Coordinates with" with caveat; pass 3 suggests tightening as a conditional "Blocked by (kanban portion)".
- ✅ 0057/0061 `linkedCount` semantics — **Resolved**

#### Scope
- ✅ Summary deliverable axes — **Resolved**
- 🔄 Namespaced ID switch — **Carried over** as suggestion (weakest thematic tie); bundling defensible.

#### Testability
- ✅ Non-interactivity (focus/element-tag) — **Resolved**
- ✅ "Next loaded" fetch trigger — **Resolved**
- ✅ Panel styling AC — **Resolved** (pass 3 flags a subtle CSS-variable assertion issue)
- ✅ `linkedCount` population — **Resolved**

### New Issues (Pass 3)

#### Clarity
- 🔵 **Orphan predicate phrased two ways**: Requirements uses `entry.completeness === null` (strict equality); the orphan AC says "is `null` or missing". `null` vs `undefined` could diverge in implementation.
- 🔵 **`present` location ambiguous**: Requirements offers either-or ("component boundary or surfaced as a typed field"); ACs refer to `cluster.completeness.present` as if the field exists on the wire.
- 🔵 **Passive constructions obscure actor**: "Extend `IndexEntry` server-side..." doesn't explicitly name the indexer as the computing actor.
- 🔵 (suggestion) ID rendering bullet has a parallel-obligation phrasing that could be collapsed.

#### Dependency
- 🔵 **0044 straddles "coordinates with" vs "blocked by"**: The bullet implies a soft upstream gate, not pure coordination.

#### Scope
- 🔵 (suggestion) Story sits at the upper end of a single-increment unit — suggests documenting an internal merge-slice sequence.
- 🔵 (suggestion) Namespaced-ID switch remains the bundled item with the weakest thematic tie.

#### Testability
- 🔵 **Panel styling AC asserts a CSS variable** (`var(--ac-bg-sunken)`) rather than a class hook or resolved colour — awkward to test with `getComputedStyle`.
- 🔵 **State-change AC doesn't specify induction mechanism** — fixture swap vs mocked hook vs real fs change all "pass" the AC differently.
- 🔵 **Kind label AC defers to "current chrome location"** rather than a concrete DOM hook, coupling the spec to a moving target.

### Assessment

The work item is in a good state for planning to begin. The remaining findings are polish — three clarity nits worth fixing (orphan predicate consistency, `present` location commitment, passive→active actor naming) and three small testability tightenings (class-hook over CSS variable, induction mechanism, concrete DOM hook). The scope suggestions are advisory; bundling decisions are defensible. None block implementation.

---
*Re-review generated by /review-work-item*

## Manual Approval — 2026-05-31

**Verdict:** APPROVE (manual override from COMMENT)

The author elected to address the four highest-value pass-3 findings and accept the remaining minor/suggestion items as planning-phase or implementation-detail decisions. Post-approval edits applied to the work item:

- **Orphan predicate**: Standardised on `entry.completeness == null` (loose equality, covers both `null` and `undefined`) and aligned Requirements with the Acceptance Criterion wording.
- **`present` field commitment**: Committed to surfacing `present: string[]` as a first-class field on the server-side `Completeness` type (derived at index time from the per-stage booleans). Wire contract and component input shape now match.
- **Active actor naming**: Server-side `IndexEntry` enrichment bullets in Requirements now name the indexer (`server/src/indexer.rs`) as the actor explicitly, replacing the passive "Extend `IndexEntry` server-side..." phrasing.
- **0044 promotion**: Moved from "Coordinates with" to "Blocked by (kanban portion only)" with explicit scope distinction — the pipeline-visualisation portion can progress in parallel; the kanban Acceptance Criteria cannot be finalised until 0044 resolves.

Remaining unresolved pass-3 findings (accepted as planning/implementation decisions, not blockers):

- 🔵 Suggestion: ID rendering bullet's parallel-obligation phrasing — minor wording.
- 🔵 Scope suggestions on bundling size and namespaced-ID switch — advisory, defensible.
- 🔵 Testability nits on panel-styling CSS variable assertion, state-change induction mechanism, kind-label DOM hook — planning-phase decisions for the implementer to pin down.

Work item now considered ready for status transition to `ready`.

---
*Manual approval applied by review-work-item flow*
