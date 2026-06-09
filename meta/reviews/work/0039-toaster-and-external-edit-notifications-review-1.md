---
date: "2026-05-27T00:00:00+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0039"
work_item_id: "0039"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
id: "0039-toaster-and-external-edit-notifications-review-1"
title: "0039-toaster-and-external-edit-notifications-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-27T00:00:00+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: Toaster and External-Edit Notifications

**Verdict:** REVISE

This is a strong, well-researched story: Context precisely defines its two load-bearing terms ("external change" as etag-not-in-self-cause-registry, and the `event.path === active relPath` correlation), the SSE/React-Query infrastructure it rides on is named with file paths, and six Given/When/Then acceptance criteria cover the Toaster lifecycle plus the match/non-match/self-caused branches. The blocking issue is a single under-specified detail amplified across two lenses — the SSE action enum (`created`/`edited`/`deleted`) maps to user-facing verbs (`created`/`updated`/`deleted`) with `edited`→"updated" left only as an example, never as a normative rule — plus a stated programmatic-trigger requirement (`useToast()`) that no acceptance criterion verifies.

### Cross-Cutting Themes

- **Action-to-verb mapping is exemplary, not normative** (flagged by: clarity, testability) — The displayed verb for an `edited` event ("updated") differs from the action name, but the work item gives the mapping only as an inline example. An implementer cannot derive the canonical verb per action, and AC4's "wording matching the action" gives a verifier no definitive string to assert. This is the highest-impact change.

### Findings

#### Major

- 🟡 **Clarity / Testability** (merged): Action-to-verb mapping under-specified
  **Location**: Requirements / Acceptance Criteria
  The action enum is `created`/`edited`/`deleted` but copy verbs are `created`/`updated`/`deleted`, with `edited`→"updated" a non-obvious transformation stated only as an example. AC4 ("wording matching the action") is not independently checkable per action; two implementers could ship "edited" vs "updated" and both claim conformance.

- 🟡 **Testability**: Programmatic-trigger requirement has no acceptance criterion
  **Location**: Acceptance Criteria
  Requirements state the Toaster "supports being triggered programmatically (e.g. via a `useToast()` hook)", but AC1–AC3 assume the Toaster is "triggered" without verifying the dispatch API exists. The reusable surface — the part making Toaster useful beyond external-edit — could be omitted or broken and still pass every stated criterion.

#### Minor

- 🔵 **Testability**: "Without regressing" invalidation and "no manual reload" are not independently verifiable
  **Location**: Requirements
  The final Requirement's "without regressing it, so the user sees the latest state without a manual reload" is a near-tautological no-regressions clause with no defined observable. AC4 already verifies invalidation occurs.

- 🔵 **Testability**: Auto-dismiss timing threshold has no tolerance or directionality
  **Location**: Acceptance Criteria
  AC3 ("when 5 seconds elapse") doesn't say whether dismissal is at exactly 5s, at least 5s, or within a tolerance, nor that it must still be visible just before. A timing test needs a defined boundary.

- 🔵 **Dependency**: Future doc-ID follow-up not captured in Dependencies
  **Location**: Drafting Notes
  The "switch the toast to a real document ID … add/track this as a separate work item" forward coupling lives only in Drafting Notes; Dependencies says "Blocks: none". Risk of the follow-up being forgotten once this story closes.

- 🔵 **Dependency**: Source-doc sequencing constraint silently overridden
  **Location**: Requirements
  The referenced gap analysis says Toaster/ExternalEditToast "should follow the chrome work" (Topbar/Sidebar). This story mounts Toaster at RootLayout, decoupling it — but doesn't acknowledge the override, so a planner may defer it unnecessarily.

#### Suggestions

- 🔵 **Completeness**: Toaster icon source/content unspecified
  **Location**: Requirements
  "icon" is named in Requirements and ACs but no section says what icon the external-edit toast renders or where it comes from (fixed glyph vs per-doc-type vs asset).

- 🔵 **Completeness**: Empty `parent` field despite a Related item
  **Location**: Frontmatter: parent
  `parent: ""` while References lists "Related: 0033" and Dependencies "Blocked by: 0033". Consistent if 0033 is a sibling blocker; worth a quick confirmation.

- 🔵 **Scope**: Generic Toaster could be separable, but bundling is justified
  **Location**: Requirements
  The gap analysis treats Toaster and ExternalEditToast as distinct surfaces. Drafting Notes defensibly argue external-edit is the only consumer today so splitting buys nothing. No action required; flagged for visibility.

- 🔵 **Clarity**: Domain terms used without definition
  **Location**: Summary / Context
  `SSE`, `etag`, `relPath` are used undefined but grounded in linked code. Optionally expand `SSE` (Server-Sent Events) on first use; code-linked terms are adequately anchored.

### Strengths

- ✅ Context explicitly defines the two load-bearing terms ("external change" = etag not in self-cause registry; correlation = `event.path === active relPath`), removing the ambiguities that would otherwise dominate this feature.
- ✅ Drafting Notes proactively reconcile the prototype's screenshot copy against the buildable, actor-free copy, so the reader is never confused by the reference/requirement discrepancy.
- ✅ Six concrete Given/When/Then acceptance criteria cover the Toaster lifecycle and the match / non-match / self-caused branches, giving full positive and negative coverage of the correlation logic.
- ✅ The existing infrastructure depended on (useDocEvents, SseEvent, self-cause registry, dispatchSseEvent, RootLayout, query-client) is named with file paths, so each precondition is verifiable rather than discovered mid-sprint.
- ✅ Resolved decisions (5s timeout, generic wording, all-action scope) are recorded with rationale in Drafting Notes and Open Questions; frontmatter is valid (kind: story, status: draft, priority: medium).
- ✅ Single coherent unit of work and single service boundary (all frontend), appropriately sized for a story; the stable per-doc-ID switch is explicitly deferred rather than expanding scope.

### Recommended Changes

1. **Add a normative action-to-verb mapping** (addresses: clarity/testability mapping major)
   In Requirements, state the mapping explicitly — `created`→"created", `edited`→"updated", `deleted`→"deleted" — and rewrite AC4 (or split it into per-action sub-cases) so each action's exact expected message string is independently checkable, replacing the vague "wording matching the action".

2. **Add an acceptance criterion for the programmatic trigger** (addresses: programmatic-trigger major)
   e.g. "Given a caller invokes the toast dispatcher (`useToast()`) with a heading and message, when called, then a Toaster renders with that content." This binds the reusable dispatch API to a verifiable outcome.

3. **Tighten or drop the "without regressing / no manual reload" clause** (addresses: invalidation minor)
   Either rely on AC4's existing invalidation check alone, or add a criterion naming the observable refreshed state (e.g. "the displayed document content updates to the new version without a page reload").

4. **Give AC3 a checkable timing boundary** (addresses: auto-dismiss timing minor)
   e.g. "the Toaster remains visible at 4s and is removed by 5.5s", or state an acceptable tolerance for the 5-second timer.

5. **Surface the doc-ID follow-up and sequencing override in Dependencies** (addresses: both dependency minors)
   Note in Dependencies that a follow-up item (ID-based correlation) is pending creation, and add a one-line note that Toaster mounts at RootLayout and therefore does not depend on the Topbar/Sidebar chrome work the gap analysis sequences before it.

6. **Specify the external-edit toast icon and confirm the empty `parent`** (addresses: completeness suggestions)
   Add one line on the icon source (or note it's left to implementation judgment), and confirm 0033 is a sibling blocker rather than a parent.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually clear and internally consistent: the Context section explicitly defines its two load-bearing terms ("external change" and document correlation), pronouns resolve cleanly, and Drafting Notes reconcile the prototype copy against the buildable behaviour. The one substantive clarity gap is an unresolved mapping between the SSE action enum (`created`/`edited`/`deleted`) and the user-facing verbs (`created`/`updated`/`deleted`), where `edited` silently becomes `updated` without the work item stating the rule. A few domain terms (SSE, etag, relPath) are used without definition but are grounded in linked code.

**Strengths**:
- Context explicitly defines "external change" and the document-correlation rule, removing the two dominant ambiguities.
- Drafting Notes reconcile the prototype's screenshot copy against the buildable, actor-free copy.
- Acceptance Criteria use a consistent placeholder convention (path `X` vs different path `Y`).
- Summary forward-references the precise Context definition.

**Findings**:
- 🟡 major (high): Action enum `created`/`edited`/`deleted` vs user-facing verbs `created`/`updated`/`deleted`; `edited`→"updated" mapping never stated; AC4 "wording matching the action" compounds it. Implementer cannot determine the correct verb for an `edited` event. Suggest a stated mapping referenced from the AC.
- 🔵 suggestion (medium): `SSE`, `etag`, `relPath` used without in-document definition; grounded in linked code so minor. Optionally expand `SSE` on first use.

### Completeness

**Summary**: Highly complete for its kind: identifies the consuming need, provides substantive Context defining "external" change precisely, and carries detailed Requirements, six concrete Acceptance Criteria, and well-populated Dependencies, Assumptions, and Drafting Notes. Frontmatter valid. No structural gaps rise to blocking level.

**Strengths**:
- Context explains why the work is needed and defines the load-bearing term.
- Six specific scenario-based acceptance criteria.
- Implementer-ready Requirements naming components, hooks, behaviours.
- Dependencies, Assumptions, Open Questions, Technical Notes, Drafting Notes all populated with rationale.
- Complete frontmatter with recognised kind/status/priority.

**Findings**:
- 🔵 suggestion (medium): Toaster "icon" named but its source/content unspecified for the external-edit toast.
- 🔵 suggestion (low): Empty `parent` field while References/Dependencies reference 0033; confirm 0033 is a sibling blocker not a parent.

### Dependency

**Summary**: Captures its primary upstream blocker (0033) and is explicit about the existing SSE/React-Query infrastructure it rides on, correctly treating those as assumptions/technical notes rather than blockers. Main gaps: a named future work item (stable per-doc ID) described in Drafting Notes but not captured as a forward coupling, and a sequencing constraint in the referenced gap analysis that the work item implicitly overrides without acknowledgement.

**Strengths**:
- Upstream blocker on 0033 explicitly named and consistent with the gap doc's sequencing.
- Existing infrastructure named with file paths so preconditions are verifiable.
- Assumptions records the React Query / SSE-authoritative-invalidator coupling.

**Findings**:
- 🔵 minor (high): Future doc-ID follow-up described in Drafting Notes but absent from Dependencies (Blocks: none); risk of being forgotten.
- 🔵 minor (medium): Gap-doc sequencing says Toaster should follow chrome work; this story mounts at RootLayout and decouples, but doesn't acknowledge the override.

### Scope

**Summary**: Bundles a reusable Toaster component with its first concrete consumer (the external-edit SSE subscriber). Bundling is deliberate and well-reasoned (Drafting Notes: external-edit is the only described consumer). Single-team, single-bounded-context (frontend), appropriately sized for a story.

**Strengths**:
- Coherent unit; subscriber is the sole consumer of the component; bundling explicitly justified.
- Summary, Requirements, and Acceptance Criteria describe consistent scope without drift.
- Stable per-doc ID switch explicitly deferred to a separate future item.
- Single service boundary (frontend).

**Findings**:
- 🔵 suggestion (medium): Generic Toaster is in principle independently deployable/reusable; gap doc treats them as distinct surfaces. Bundling justified — no action required; if other Toaster consumers get planned soon, reconsider splitting the generic component.

### Testability

**Summary**: Unusually well-specified for testability: six Given/When/Then criteria cover render, manual dismiss, auto-dismiss, and the positive/negative branches of external-edit correlation. Main gaps: under-specified action-to-verb mapping referenced only by example, Summary/Requirements behaviours (programmatic triggering, no-manual-reload) not bound to any verifiable criterion, and a mild ambiguity in auto-dismiss timing.

**Strengths**:
- ACs framed as concrete Given/When/Then with observable outcomes.
- External-edit logic tested across all three branches (match, mismatch, self-caused).
- "External change" definition and correlation rule precisely defined in Context.
- Open Questions records resolved timeout/wording/event-scope decisions.

**Findings**:
- 🟡 major (high): Action-to-verb mapping exemplary not enumerated; AC4 wording not independently checkable per action; `edited`→"updated" non-obvious. Add a normative mapping and split/expand AC4.
- 🟡 major (medium): Programmatic-trigger requirement (`useToast()`) has no acceptance criterion; the reusable dispatch API could be omitted/broken and still pass. Add a criterion verifying the dispatcher.
- 🔵 minor (medium): "Without regressing" invalidation and "no manual reload" not independently verifiable; near-tautological. Rely on AC4 or name the observable refreshed state.
- 🔵 minor (low): AC3 auto-dismiss timing lacks tolerance/directionality; needs a checkable boundary.

## Re-Review (Pass 2) — 2026-05-27

**Verdict:** COMMENT

Both blocking majors are resolved. The action-to-verb mapping is now normative in Requirements and enumerated with exact expected strings in the Acceptance Criteria, and a dedicated criterion verifies the `useToast()` dispatcher. The remaining findings are all minor polish — no criterion fails to verify a stated requirement at a blocking level, so the work item is ready for implementation as-is.

### Previously Identified Issues
- 🟡 **Clarity/Testability**: Action-to-verb mapping under-specified — **Resolved**. Requirements state the normative mapping (`created`→"created", `edited`→"updated", `deleted`→"deleted"); the external-edit AC enumerates the exact message string per action.
- 🟡 **Testability**: Programmatic-trigger requirement has no acceptance criterion — **Resolved**. New AC: "Given a caller invokes the toast dispatcher (`useToast()`) with a heading and message … a Toaster renders with that heading and message."
- 🔵 **Testability**: "Without regressing" / "no manual reload" not verifiable — **Resolved (downgraded)**. Tautological clause replaced with an observable outcome and a dedicated AC; a residual minor remains about naming the exact observable marker for "the new version".
- 🔵 **Testability**: Auto-dismiss timing lacks boundary — **Resolved**. AC now reads "visible at 4 seconds, removed by 5.5 seconds (±0.5s tolerance)".
- 🔵 **Dependency**: Doc-ID follow-up not in Dependencies — **Partially resolved**. Now recorded under "Followed by", but still has no work-item number (item pending creation).
- 🔵 **Dependency**: Source-doc sequencing silently overridden — **Resolved**. Dependencies now states the Toaster mounts at RootLayout and does not depend on the chrome work.
- 🔵 **Completeness**: Icon source unspecified — **Resolved (downgraded)**. Requirements now specify a fixed notification glyph; a residual minor remains about whether "fixed" means a designated vs. any-consistent glyph.
- 🔵 **Completeness**: Empty `parent` field — **Resolved**. Confirmed standalone by stakeholder; 0033 is a sibling blocker, not a parent.
- 🔵 **Scope**: Generic-Toaster separability — **No action (as accepted)**. Bundling remains justified.
- 🔵 **Clarity**: `SSE` unexpanded — **Still present**. `SSE` is still not expanded to "Server-Sent Events" on first use (minor).

### New Issues Introduced
- 🔵 **Clarity** (minor): "generic and action-aware" in the same Requirement sentence reads as a momentary tension; suggest "actor-generic but action-specific".
- 🔵 **Dependency** (minor): The docs-list-derived `relPath` resolution is an implicit data dependency (assumes docs list is loaded and its `relPath` format matches SSE `event.path`); not surfaced in Dependencies/Assumptions.
- 🔵 **Testability** (minor): The content-refresh AC doesn't name the observable marker confirming "the new version"; the verb-mapping AC bundles all three actions into one bullet (could be split or noted as requiring all three verified independently).

### Assessment
The work item is ready for implementation. Both majors that drove the REVISE verdict are fully addressed, and the new verdict is COMMENT. The remaining minor findings (SSE expansion, "generic and action-aware" phrasing, docs-list dependency note, content-refresh observable marker, splitting the verb-mapping AC, designated-vs-any glyph) are optional polish that an implementer can resolve in-flight without re-review; none block planning.

## Re-Review (Pass 3) — 2026-05-27

**Verdict:** COMMENT

The Pass-2 minors that were addressed by edits are now resolved. The lenses surfaced a few new, finer-grained minors — these are test-design and editorial nuances rather than work-item defects, and we have reached diminishing returns: each pass now trades one minor for a more specific one. The work item remains ready for implementation.

### Previously Identified Issues (from Pass 2)
- 🔵 **Clarity**: `SSE` unexpanded — **Resolved**. Now "Server-Sent Events (SSE)" on first use in the Summary.
- 🔵 **Clarity**: "generic and action-aware" tension — **Resolved**. Reworded to "actor-generic but action-specific" with a parenthetical splitting the two dimensions.
- 🔵 **Completeness/Testability**: designated-vs-any glyph ambiguity — **Resolved**. Requirements now state "a single consistent notification glyph of the implementer's choosing (no specific icon is mandated)".
- 🔵 **Testability**: content-refresh observable marker — **Resolved**. AC now asserts a known changed value appears and stale content is gone, without full-page navigation.
- 🔵 **Testability**: verb-mapping AC bundled all three actions — **Resolved**. AC now states "each action must be verified independently".
- 🔵 **Dependency**: docs-list `relPath` correlation coupling — **Resolved (in Assumptions)**. New Assumption surfaces the docs-list-loaded + path-format-match precondition and its silent-failure consequence.
- 🔵 **Dependency**: doc-ID follow-up unnumbered — **Still present (by decision)**. Recorded under "Followed by: pending creation"; stakeholder chose not to create the stub item yet.

### New Issues Introduced
- 🔵 **Clarity** (minor): "actor-generic but action-specific" lead clause still pairs opposed-sounding qualifiers before the parenthetical resolves them; could lead with the disambiguating phrasing.
- 🔵 **Clarity** (minor): "that prototype copy" demonstrative points back across a paragraph; name the antecedent. Also "the system can only tell 'not me'" uses a vaguer "the system" than the surrounding component-specific prose.
- 🔵 **Dependency** (minor): path-format/docs-list precondition is in Assumptions but a planner scanning Dependencies wouldn't see it; could be cross-referenced in Dependencies.
- 🔵 **Testability** (minor): icon AC has no stable hook (test id / decorative role) to assert against; content-refresh and programmatic-dispatch ACs could name concrete before/after fixture values; no AC covers the manual-dismiss-before-auto-dismiss timer race; positive correlation should be verified with a real resolved `relPath` vs real `event.path` rather than two equated literals.

### Assessment
Ready for implementation; verdict holds at COMMENT. The Pass-3 findings are implementation/test-authoring details (stable test hooks, concrete fixtures, a timer-race AC) that are appropriate to settle during planning or test design rather than by further work-item iteration. Recommend stopping work-item-level review here — additional passes will keep surfacing equivalent-severity editorial nuances without changing the verdict. The only standing decision is whether to create the follow-on doc-ID work item now or leave it as a pending note.

### Final Verdict — APPROVE
Marked APPROVE by stakeholder. Both blocking majors were resolved in Pass 2 and the addressed minors in Pass 3; the residual minors are test-authoring/editorial details to settle during planning. The work item is ready for implementation. Work item status transitioned draft → ready.
