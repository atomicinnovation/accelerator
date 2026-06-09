---
date: "2026-05-18T00:00:00+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0042"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 4
status: complete
id: "0042-templates-view-redesign-review-1"
title: "0042-templates-view-redesign-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-18T00:00:00+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: Templates View Redesign

**Verdict:** REVISE

The work item is well-structured with a clear user persona, populated sections, and mostly verifiable acceptance criteria. However, three major issues cluster around the backend etag: the backend `sha256` exposure is an implied prerequisite not captured as a blocker, AC6 verifies implementation provenance rather than an observable outcome, and the three tier-presence visual states lack measurable distinctness criteria. Minor clarity and dependency issues compound these themes.

### Cross-Cutting Themes

- **Backend etag plumbing under-specified** (flagged by: clarity, dependency, testability) — The backend `sha256` field is referenced as a hard prerequisite but not captured as a blocker; the endpoint(s) gaining the field aren't named; and AC6 ("read directly from backend response") is not externally verifiable.
- **0055 / SSE relationship is ambiguous** (flagged by: dependency, testability) — 0055 is called both a "precedent" and the basis of an open SSE question; if SSE-driven updates are adopted, dependencies expand silently and an AC is missing.
- **Visual states lack measurable definitions** (flagged by: clarity, testability) — `absent` / `present` / `present-and-winning` and the etag header format are described qualitatively; reviewers can't tell pass from fail without a token or pattern reference.

### Findings

#### Critical
*(none)*

#### Major
- 🟡 **Dependency**: Backend etag exposure is an implied prerequisite but not captured as a blocker
  **Location**: Dependencies
  Requirements and Technical Notes mandate a new backend `sha256` field on the templates endpoint, but Dependencies lists only frontend work items (0033, 0037, 0038, 0041). Planners reading Dependencies won't see the backend prerequisite.
- 🟡 **Testability**: Etag-source criterion (AC6) is not externally observable
  **Location**: Acceptance Criteria
  AC6 ("read directly from the backend response and not computed in the client") is an implementation-provenance claim that cannot be verified by inspecting the rendered DOM. It needs to be reframed as a verifiable contract (e.g., behaviour when sha256 is missing from the response) or have its verification mechanism specified.
- 🟡 **Testability**: 'Visually distinct' tier states (AC2) lack a measurable threshold
  **Location**: Acceptance Criteria
  AC2 requires `absent` to be "visually distinct from `present`" without specifying the distinguishing attribute (colour token, opacity, icon). Pass/fail becomes subjective.

#### Minor
- 🔵 **Clarity**: Inconsistent naming of the right-hand pane ("tier-preview pane" vs "right-hand pane" vs "rendered template body")
  **Location**: Requirements / Acceptance Criteria
  Three names denote the same UI region; pick one canonical term.
- 🔵 **Clarity**: "etag" / "sha256-… etag header" used without defining its role in the UI
  **Location**: Requirements / Technical Notes
  Could be misread as an HTTP ETag header rather than a visible UI label. Clarify on first use.
- 🔵 **Clarity**: "Preserve the existing active-tier marker semantics" references unspecified prior behaviour
  **Location**: Requirements (last bullet)
  Neither defined nor linked; "preserve" is underdetermined.
- 🔵 **Clarity**: Ambiguous outcome — "no interactive behaviour occurs" is broader than Requirements
  **Location**: Acceptance Criteria (final bullet)
  Requirements exclude only copy and selection; the AC could be read as forbidding all defaults. Align wording.
- 🔵 **Clarity**: Passive "Backend must expose a sha256 field" leaves the endpoint(s) implicit
  **Location**: Technical Notes
  Unclear whether index, detail, or both endpoints gain the field, and whether "per resolved template" implies one sha256 per index row.
- 🔵 **Dependency**: Relationship to 0055 is ambiguous between precedent and prerequisite
  **Location**: Dependencies
  0055 is variously called "precedent", "establishes the etag pattern", and tied to an SSE open question. Clarify whether it blocks.
- 🔵 **Dependency**: SSE-driven live update path implies an uncaptured SSE coupling
  **Location**: Open Questions
  Resolving the open question towards SSE would silently expand the dependency surface.
- 🔵 **Testability**: Etag format pattern not specified
  **Location**: Acceptance Criteria
  No statement of whether the hash is full 64-char hex, truncated, etc.
- 🔵 **Testability**: No criterion covers the "team override exists but is not winning" case
  **Location**: Acceptance Criteria
  Only `absent` and the all-default-winning case have ACs; the non-winning `present` state is unverified.
- 🔵 **Testability**: Unresolved SSE behaviour leaves a verification gap
  **Location**: Open Questions
  Static-render-only and live-updating implementations both pass current ACs.

#### Suggestions
- 🔵 **Scope**: Backend etag field bundled with frontend redesign
  **Location**: Requirements
  Crosses service boundaries; consider splitting if ownership is split.
- 🔵 **Scope**: Two visually distinct sub-features (index + detail) under one story
  **Location**: Requirements
  Each could ship independently; consider whether incremental delivery is wanted.

### Strengths

- ✅ Clear user persona ("power user customising the accelerator plugin with team-level and/or individual overrides") stated in Summary and reaffirmed in Assumptions.
- ✅ Three tier-presence states (`absent`, `present`, `present-and-winning`) are enumerated and defined inline in Requirements.
- ✅ Scope is internally consistent: Summary, Requirements, and Acceptance Criteria all describe the same two changes.
- ✅ Eight specific Given/When/Then acceptance criteria covering tier presence, layout, and etag behaviour.
- ✅ Frontmatter fully populated with recognised values; all optional sections (Dependencies, Assumptions, Open Questions, Technical Notes, Drafting Notes, References) have substantive content.
- ✅ Boundary statements explicitly preserve the existing tier model and limit scope to presentation, not resolution semantics.
- ✅ AC7 ties the etag value to a concrete computable property (sha256 of resolved winning-tier content).
- ✅ Drafting Notes capture decisions (etag source, default-tier-always-present, no copy affordance) made during enrichment.

### Recommended Changes

1. **Capture the backend `sha256` exposure as an explicit blocker** (addresses: Backend etag exposure is an implied prerequisite; Passive "Backend must expose..." leaves endpoint(s) implicit)
   Add a "Blocked by: backend exposing `sha256` on `/library/templates` response — work item TBD" entry to Dependencies, and in Technical Notes name the specific endpoint(s) gaining the field and what the value covers (winning-tier resolved content).

2. **Reframe AC6 as a verifiable contract** (addresses: Etag-source criterion is not externally observable)
   Replace "read directly from the backend response and not computed in the client" with an observable contract — e.g., "Given the backend response omits the `sha256` field, when the detail screen renders, then the etag header is not displayed."

3. **Specify the visual contract for tier-presence states** (addresses: Visually distinct states lack measurable threshold)
   Bind AC2 to concrete design tokens or to a defined Chip variant (referencing 0038's palette), e.g., "`absent` uses token X; `present` uses token Y; `present-and-winning` adds a highlight ring of token Z."

4. **Clarify the 0055 relationship and SSE scope** (addresses: 0055 ambiguity; SSE coupling; SSE verification gap)
   Decide whether 0055 is a blocker or pattern-only, and resolve the Open Question on SSE-driven updates. Either add an AC asserting static-for-page-lifetime behaviour or scope SSE in with its dependency captured.

5. **Use one canonical name for the right-hand pane** (addresses: Inconsistent pane naming)
   Pick e.g. "template preview pane" and apply across Summary, Requirements, and ACs.

6. **Define "etag header" on first use as a visible UI label** (addresses: Etag UI role undefined)
   Add a one-line definition distinguishing it from HTTP ETag semantics.

7. **Pin the active-tier marker semantics** (addresses: "Preserve" is underdetermined)
   Either link to where active-tier-marker behaviour is defined or briefly state which semantics must be preserved (visual treatment, interaction model, both).

8. **Align AC8 wording with Requirements** (addresses: "No interactive behaviour" broader than Requirements)
   Reword to "the etag header has no hover affordance, no click handler, and no copy/selection helper."

9. **Specify the etag display format** (addresses: Etag format pattern not specified)
   Add "the etag header displays the literal prefix `sha256-` followed by the full 64-character lowercase hex digest" (or the truncated form, if intended).

10. **Add an AC for the team-override-not-winning case** (addresses: missing AC for non-winning present state)
    e.g., "Given a template has both team and user overrides, when the index row renders, then the team tier is shown in `present` (non-winning) and the user tier is shown in `present-and-winning`."

11. *(Optional)* **Decide whether to split frontend / backend or index / detail** (addresses: Scope suggestions)
    Either explicitly endorse the current bundling in Drafting Notes or split into coordinated work items.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Generally clear and well-structured. A few terms (`etag`, "tier-preview pane" vs "right-hand pane", "active-tier marker semantics") are used inconsistently or without definition; some actors are obscured by passive voice.

**Strengths**:
- User persona is explicitly defined and reaffirmed in Assumptions.
- The three tier-presence states are enumerated with definitions in Requirements.
- Cross-section scope is internally consistent across Summary, Requirements, and ACs.
- Domain references (0029, 0055, 0038) are linked, not implied.

**Findings**:
- 🔵 **minor / medium**: Inconsistent naming of the right-hand pane (Requirements / Acceptance Criteria) — three names denote the same UI region.
- 🔵 **minor / medium**: "etag header" used without defining its UI role (Requirements / Technical Notes) — risks being misread as an HTTP header.
- 🔵 **minor / medium**: "Preserve the existing active-tier marker semantics" references unspecified prior behaviour (Requirements).
- 🔵 **minor / medium**: "No interactive behaviour occurs" (AC8) is broader than Requirements wording.
- 🔵 **minor / low**: Passive "Backend must expose a `sha256` field" leaves the endpoint(s) implicit (Technical Notes).

### Completeness

**Summary**: Highly complete. Clear user-framed Summary, populated Context, eight specific ACs, detailed Requirements, and well-populated Dependencies/Assumptions/Open Questions/Technical Notes/Drafting Notes. Frontmatter intact with recognised values.

**Strengths**:
- Summary leads with "As a … I want … so that …" framing.
- Context covers current state and prototype behaviour.
- Eight Given/When/Then style ACs.
- Frontmatter fully populated.
- Story-specific content well-developed.
- Optional sections substantive rather than placeholder.

**Findings**: *(none)*

### Dependency

**Summary**: Explicit upstream blockers (0033, 0037, 0038, 0041) and 0055 noted as etag-pattern precedent. The 0055 relationship is ambiguous (precedent vs prerequisite), and the backend etag field implies a backend coupling not captured as a blocker.

**Strengths**:
- Dependencies lists four blocking work items with reasons.
- Related items (0028, 0029, 0055) named in References.
- Assumptions surface the 0029 tier model coupling.
- `Blocks: none` shows downstream consumers were considered.

**Findings**:
- 🟡 **major / high**: Backend etag exposure is an implied prerequisite but not captured as a blocker (Dependencies).
- 🔵 **minor / medium**: Relationship to 0055 is ambiguous between precedent and prerequisite (Dependencies).
- 🔵 **minor / medium**: SSE-driven live update path implies an uncaptured SSE/infrastructure coupling (Open Questions).

### Scope

**Summary**: Coherent redesign of one screen with tightly related frontend changes and a small justified backend addition. The bundling of backend etag with frontend redesign is the main scope question but is reasoned. Sizing as a story is appropriate.

**Strengths**:
- Summary, Requirements, and ACs describe the same scope.
- Clear boundary statements preserving the tier model.
- Backend etag is narrowly scoped and tied to the frontend need (0055 precedent).
- Open Questions and Assumptions sharpen in/out-of-scope decisions.

**Findings**:
- 🔵 **suggestion / medium**: Backend etag field bundled with frontend redesign (Requirements) — crosses service boundaries.
- 🔵 **suggestion / low**: Two visually independent sub-features (index + detail) under one story (Requirements) — could ship independently.

### Testability

**Summary**: Mostly verifiable Given/When/Then ACs. Several criteria lack concrete state definitions, and AC6 references implementation provenance rather than user-observable output.

**Strengths**:
- Most ACs use Given/When/Then framing.
- AC8 frames non-interactivity as a verifiable behaviour.
- AC7 ties etag to a concrete computable property.
- Three tier-presence states explicitly enumerated.

**Findings**:
- 🟡 **major / high**: Etag-source criterion (AC6) is not externally observable (Acceptance Criteria).
- 🟡 **major / medium**: "Visually distinct" states (AC2) lack a measurable threshold (Acceptance Criteria).
- 🔵 **minor / high**: Etag format pattern not specified (Acceptance Criteria).
- 🔵 **minor / medium**: No criterion covers the "team override exists but is not winning" case (Acceptance Criteria).
- 🔵 **minor / medium**: Unresolved SSE behaviour leaves a verification gap (Open Questions).

## Re-Review (Pass 2) — 2026-05-18

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Dependency**: Backend etag exposure is an implied prerequisite but not captured as a blocker — **Resolved** (backend brought in-scope; Technical Notes name the detail endpoint and field; Drafting Notes record the bundling rationale)
- 🟡 **Testability**: Etag-source criterion (AC6) is not externally observable — **Resolved** (AC6 dropped and replaced with an observable "if `sha256` omitted, header not displayed" AC)
- 🟡 **Testability**: 'Visually distinct' tier states (AC2) lack a measurable threshold — **Resolved** (states bound to neutral / indigo / green Chip variants from 0038, with AC2 reworded accordingly)
- 🔵 **Clarity**: Inconsistent pane naming — **Resolved** (canonical "template preview pane" used throughout)
- 🔵 **Clarity**: "etag header" UI role undefined — **Resolved** (defined inline as a visible UI element, not an HTTP header)
- 🔵 **Clarity**: "Preserve active-tier marker semantics" underdetermined — **Resolved** (clarified — same visual treatment, same meaning, retained as-is on the detail stacked card)
- 🔵 **Clarity**: AC8 wording broader than Requirements — **Resolved** (AC enumerates no hover, no click, no copy/selection)
- 🔵 **Clarity**: Passive "Backend must expose..." leaves endpoint(s) implicit — **Partially resolved** (detail endpoint now named, but with "or equivalent" qualifier — see new finding below)
- 🔵 **Dependency**: 0055 precedent-vs-prerequisite ambiguity — **Resolved** (Drafting Notes record 0055 as pattern reference only)
- 🔵 **Dependency**: SSE-driven live update path implies uncaptured coupling — **Resolved** (SSE event scope brought in-scope and Technical Notes describe channel and payload shape)
- 🔵 **Testability**: Etag format pattern not specified — **Resolved** (literal prefix `sha256-` + full 64-character lowercase hex digest pinned in Requirements and AC)
- 🔵 **Testability**: No criterion for "team override exists but is not winning" — **Resolved** (new AC added)
- 🔵 **Testability**: Unresolved SSE behaviour — **Resolved** (open question removed; SSE-driven live update now a Requirement with a paired AC)
- 🔵 **Scope** suggestions (frontend+backend bundle; index+detail split) — **Resolved by explicit endorsement** (Drafting Notes endorse the bundling with reasoning)

### New Issues Introduced

- 🟡 **Testability**: SSE update AC lacks event payload specification (Acceptance Criteria) — Technical Notes hedges with "should match the 0055 pattern" rather than pinning the event name and `{ template, sha256 }` shape; testers can't construct a concrete event to fire.
- 🟡 **Testability**: No AC asserts the backend `sha256` field contract directly (Acceptance Criteria) — all related ACs are framed in terms of UI behaviour; a backend-only test has no acceptance gate for field name, casing, or omission-vs-null.
- 🔵 **Clarity**: "or equivalent" weakens the endpoint identifier (Technical Notes) — `GET /api/library/templates/{name}` or equivalent` re-introduces ambiguity into an otherwise precise spec.
- 🔵 **Clarity**: "etag matches sha256" AC lacks explicit actor/source (Acceptance Criteria) — could be read as a frontend, backend, or end-to-end equivalence check.
- 🔵 **Clarity**: "SSE-driven update flow described below" forward-references the next bullet (Requirements) — slows first-pass comprehension.
- 🔵 **Dependency**: 0029 (resolution model) treated as a prerequisite but only listed under Related — should be either explicitly in Dependencies or flagged as "assumed delivered".
- 🔵 **Testability**: AC7 ("etag matches sha256 of resolved content") is partially tautological without a fixture (Acceptance Criteria).
- 🔵 **Testability**: Tier-presence AC1 lacks tier ordering/count (Acceptance Criteria).
- 🔵 **Testability**: SSE-driven update lacks observable timing bound (Requirements / Acceptance Criteria).
- 🔵 **Scope** *(suggestion)*: SSE live-update could be a separable increment (Requirements) — judgement call already endorsed in Drafting Notes; flagged for awareness only.
- 🔵 **Dependency** *(suggestion)*: SSE event payload shape pinning not promoted from Technical Notes to a contractual statement (Technical Notes).
- 🔵 **Clarity** *(suggestion)*: Drafting Notes pronoun referents ("it's a single screen redesign", "that calculus") could be tightened.
- 🔵 **Clarity** *(suggestion)*: Context's "each tier shown with a present-state colour" predates the three-state model and could be aligned (Context).

### Assessment

All three original major findings are resolved, and every minor finding was addressed. The work item now has clear pane naming, defined etag header semantics, Chip-variant-bound tier states, an observable backend-omission AC, and explicit SSE scope. However, the two structural additions (backend `sha256` field and SSE event) introduced two new major testability gaps: the SSE event payload shape isn't pinned, and there's no backend contract AC. Verdict remains **REVISE** to close these gaps before implementation.

The required edits for a third pass are small and focused: (1) pin the SSE event name and payload shape in Technical Notes (not "should match"), (2) add ACs asserting the backend response includes `sha256` of the expected format, (3) drop "or equivalent" from the endpoint path, and (4) optionally add a small latency / tier-ordering expectation.

## Re-Review (Pass 3) — 2026-05-18

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability**: SSE update AC lacks event payload — **Resolved** (payload `{ template: string, sha256: string }` pinned in both AC and Technical Notes)
- 🟡 **Testability**: No AC asserts backend `sha256` contract — **Resolved** (new AC asserts response shape and digest)
- 🔵 **Clarity**: "or equivalent" endpoint qualifier — **Resolved** (dropped)
- 🔵 **Clarity**: "etag matches sha256" AC actor/source — **Resolved** (AC now names the comparison explicitly)
- 🔵 **Clarity**: "SSE-driven update flow described below" forward-reference — **Resolved** (bullet reordered and rephrased)
- 🔵 **Dependency**: 0029 treated as prerequisite but only Related — **Resolved** (added to Dependencies as "assumed delivered")
- 🔵 **Testability**: AC7 partially tautological — **Resolved** (rephrased to compare displayed value against SHA-256 of named response field)
- 🔵 **Testability**: Tier ordering not pinned — **Resolved** (AC1 fixes left-to-right order)
- 🔵 **Testability**: SSE update lacks timing bound — **Resolved** (1s latency bound added)
- 🔵 **Scope** (suggestion): SSE separable increment — **Resolved by explicit endorsement** (Drafting Notes endorse bundling)
- 🔵 **Dependency** (suggestion): SSE payload pinning — **Resolved** (Technical Notes promoted to "must")
- 🔵 **Clarity** (suggestion): Drafting Notes pronouns — **Resolved** (low-priority note tightened)
- 🔵 **Clarity** (suggestion): Context's "present-state colour" predated three-state model — **Resolved** (Context updated)

Additionally, the tier model labels were corrected during this pass — `default / team / user` → `plugin default / user override / config override` — a substantive terminology fix that propagated through Summary, Context, Requirements, ACs, Assumptions, and Drafting Notes.

### New Issues Introduced

- 🟡 **Clarity**: "etag header" overloads established HTTP terminology — the disambiguation note in Requirements isn't carried through ACs, Technical Notes, or Drafting Notes, where "etag header" / "backend etag" can be misread as an HTTP `ETag:` header.
- 🟡 **Clarity**: SSE event name referenced as `templates-change` but Technical Notes defers naming to implementation — contradiction between name-like usage in ACs and "name TBD" in notes.
- 🟡 **Dependency**: 0055's SSE `/api/events` channel infrastructure not captured as a blocker — if 0055 is the work that stands up the SSE channel and `Option<String>` etag shape, the "pattern reference only" framing understates the coupling. Either move 0055 to Blocked-by (channel parts only) or add an explicit "0055 SSE channel assumed delivered" assumption.
- 🟡 **Testability**: Winning-tier "highlighted" on index has no measurable visual contract beyond the green Chip — either remove the standalone "highlighted" wording or pin what additional visual treatment (if any) the winning row gets.
- 🟡 **Testability**: SSE event name deferred to implementation makes the AC trigger unspecified — testers can't subscribe to an unspecified event name.
- 🔵 **Clarity**: Etag label position ("above the pane" in Summary/Requirements vs "above the rendered body" in AC) is inconsistent — could be a sibling above the pane or first child inside it.
- 🔵 **Clarity** (suggestion): `Option<String>` "omitted (not null)" wire-shape semantics rely on Rust knowledge — clarify on the wire.
- 🔵 **Dependency**: 0028 (userspace customisation directory) referenced in Context for `config.md` / `config.local.md` semantics but only in Related — promote to Dependencies.
- 🔵 **Dependency**: Existing `/api/library/templates` endpoints assumed but not named in Dependencies — low impact if endpoints are baseline.
- 🔵 **Testability**: Detail-screen "left/right" layout has no DOM/viewport anchor — responsive collapse behaviour ambiguous.
- 🔵 **Testability**: Empty/missing winning-tier content path not exercised from backend side — could pass with `null` or empty string instead of omitted field.
- 🔵 **Testability**: "Within 1s of event dispatch" lacks defined measurement origin (server emit vs client receive).
- 🔵 **Testability**: "No interactive behaviour" enumeration could pin specific observable properties (cursor, hover, tooltip).
- 🔵 **Scope** (suggestion): SSE live-update could still be carved out — judgement call, already endorsed.

### Assessment

All 13 prior findings from Pass 2 were resolved, plus the tier terminology was substantively corrected. The Pass 3 majors are a mix of:

- **Genuine terminology issue surfaced by deeper review** — "etag header" vs HTTP ETag (could have been caught earlier; the disambiguation note was added in Pass 2 but doesn't propagate).
- **Tension introduced by the SSE-name decision** — using `templates-change` as a name-like token in ACs while deferring naming to implementation in Notes is internally contradictory.
- **Genuine dependency under-capture** — 0055's SSE infrastructure is more than a "pattern"; it's plumbing this work item extends.
- **AC tightening opportunities** — the "highlighted" wording duplicates the Chip-variant contract.

Verdict remains **REVISE**, but the work item is now substantially stronger than at Pass 1. Remaining fixes are small and well-scoped. Approving for implementation is defensible if the team accepts that the SSE event name will be resolved at implementation time and that "etag header" is a project-local term.

## Re-Review (Pass 4) — 2026-05-18

**Verdict:** APPROVE

### Previously Identified Issues

- 🟡 **Clarity**: "etag header" overloads HTTP terminology — **Resolved** (renamed to "content-hash label" throughout; project-local term defined in Technical Notes and noted in Drafting Notes)
- 🟡 **Clarity**: SSE event name contradiction — **Resolved** (pinned to `template-changed` consistently across ACs, Requirements, and Technical Notes)
- 🟡 **Dependency**: 0055 SSE channel coupling — **Resolved** (added to Dependencies as "assumed delivered", mirroring 0029 treatment)
- 🟡 **Testability**: "Highlighted" lacks measurable contract — **Resolved** (green Chip is the sole index signal; detail-screen accent ring pinned in a new AC)
- 🟡 **Testability**: SSE event name deferred — **Resolved** (pinned to `template-changed`)
- 🔵 **Clarity**: label position consistency — **Resolved** ("first row inside the template preview pane")
- 🔵 **Clarity**: `Option<String>` wire-shape — **Resolved** (Technical Notes explicit: present-string-or-absent, never null/empty)
- 🔵 **Dependency**: 0028 not in Dependencies — **Resolved** (added as "assumed delivered")
- 🔵 **Dependency**: existing templates endpoints unnamed — **Implicitly resolved** (endpoint paths concrete, baseline assumed)
- 🔵 **Testability**: detail-screen left/right anchor — **Resolved** (≥1024px viewport precondition; collapse out of scope)
- 🔵 **Testability**: backend empty-content omission — **Resolved** (new AC asserts field omitted, not null/empty)
- 🔵 **Testability**: SSE measurement origin — **Resolved** ("from client receipt" pinned)
- 🔵 **Testability**: interactivity property enumeration — **Resolved** (cursor, hover bg, tooltip, click handler, copy/selection all enumerated)
- 🔵 **Scope** (suggestion): SSE separable increment — **Resolved by explicit endorsement** (Drafting Notes endorse bundling)

### New Issues Introduced

No majors. All remaining findings are minor refinements or suggestions:

- 🔵 **Clarity** (minor): "stacked tier card" used as a defined term but never defined — could be one card with stacked rows or multiple stacked cards.
- 🔵 **Clarity** (minor): "the existing active-tier marker, retained as-is" has ambiguous antecedent — Context mentions a marker but doesn't confirm the existing form is an outline ring.
- 🔵 **Clarity** (minor): "the `sha256` field" referent could be qualified ("response's `sha256` field" / "JSON `sha256` field") where it might be confused with the UI label.
- 🔵 **Clarity** (suggestion): "indigo / green" Chip variants referenced by colour without confirming they are 0038's variant names.
- 🔵 **Dependency** (minor): 0055 classification as assumed-delivered may understate the hard-blocker status for SSE plumbing.
- 🔵 **Dependency** (minor): new `template-changed` event creates an uncaptured forward contract for other `/api/events` consumers.
- 🔵 **Dependency** (minor): new `sha256` field on detail endpoint creates an uncaptured API-contract coupling with other clients.
- 🔵 **Dependency** (suggestion): design-gap source document not listed as a scoping dependency (traceability gap).
- 🔵 **Testability** (minor): SSE 1s latency measurement boundary (EventSource handler vs DOM commit) not pinned.
- 🔵 **Testability** (minor): no AC asserts the index endpoint shape is unchanged.
- 🔵 **Testability** (minor): "empty or absent winning-tier content" trigger condition not specified (zero-byte body? missing file?).
- 🔵 **Testability** (minor): tier-presence state matrix not exhaustively covered — missing the "user override present, no config override" scenario.

### Assessment

All five Pass-3 majors are resolved. The remaining findings are minor refinements and traceability suggestions — the kind that surface on every review pass and never fully converge. The work item is **ready for implementation** with the verdict downgraded to COMMENT.

Notable improvements from Pass 1 → Pass 4:

| Pass | Verdict | Majors | Minors/Suggestions |
|------|---------|--------|--------------------|
| 1    | REVISE  | 3      | 11                 |
| 2    | REVISE  | 2      | 12                 |
| 3    | REVISE  | 5      | 11                 |
| 4    | COMMENT | 0      | 12                 |

The Pass-3 spike in majors reflects how addressing the prior round's findings (adding SSE scope, renaming terminology) introduced new specification surfaces; Pass 4 closed those. The minors at Pass 4 are stable refinement-level observations rather than blockers.

**Recommendation**: accept as ready for implementation. The remaining minors can be folded in opportunistically during planning or implementation rather than requiring another full revision pass.
