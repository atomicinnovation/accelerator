---
date: "2026-05-10T07:49:19+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0036-sidebar-redesign.md"
work_item_id: "0036"
review_number: 1
verdict: REVISE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 1
status: complete
---

## Work Item Review: 0036 Sidebar Redesign

**Verdict:** REVISE

The work item is structurally complete, well-written, and shows clear thought about implementation — Acceptance Criteria use Given/When/Then framing, Technical Notes are dense with file:line references, and the Drafting Notes transparently flag the bundling decision. However, the same Drafting Notes acknowledgement is the loudest finding across the review: the story bundles three to four substantially independent capabilities (nav restructure + unseen-dots, search + new endpoint, activity feed + new SSE wire format + history endpoint, footer cleanup), several of which depend on backend work that the story neither claims nor declares as a blocker. Combined with several Acceptance Criteria that defer their own data sources to "options described in Technical Notes", many criteria are not yet deterministically verifiable.

### Cross-Cutting Themes

- **Hidden backend coupling** (flagged by: dependency, scope, testability) — Three to four backend changes (`SsePayload` action discriminator, `/api/activity?limit=N` history endpoint, `/api/search`, optional `/api/types` count extension) are required to satisfy the ACs but not captured in Dependencies. Either commit them to scope or list them as Blocked-by sibling stories.
- **Scope is oversized — multiple stories bundled** (flagged by: scope, testability, dependency) — The author's own Drafting Notes identify natural split points (nav + unseen / search / activity feed). Splitting would also collapse most testability findings, since per-story ACs could pin endpoints, payloads, and verbs concretely.
- **Conditional "options" / "e.g." language in ACs** (flagged by: testability, clarity) — AC2 (count source), AC5 (search response shape), AC6 (action verb mapping), AC9 (history endpoint URL) all defer their concrete contract to a "decide-later" pointer. Each becomes deterministically testable only once the option is picked.
- **Footer-status removal AC may already be satisfied by 0035** (flagged by: clarity, dependency, scope) — `SseIndicator` is in the Topbar; `Sidebar.test.tsx` already asserts no version label. Either drop the AC, fold it into 0035, or annotate the residual scope.
- **Action-verb vocabulary inconsistent across sections** (flagged by: clarity, testability) — Requirements list "created / edited / moved-to-status"; AC6 lists "created / edited / moved / deleted". Pick one canonical set and propagate.

### Findings

#### Critical

*None.*

#### Major

- 🟡 **Scope**: Story bundles four independently-deliverable capabilities
  **Location**: Requirements
  Nav restructure, search + endpoint, activity feed + new SSE wire format + history endpoint, and footer cleanup are largely orthogonal with separate backend surfaces. Drafting Notes pre-identify these as natural split points.

- 🟡 **Scope**: Story spans multiple service boundaries with no orchestration plan
  **Location**: Requirements
  Coordinated changes required across the Rust server (4 distinct surfaces) and the React frontend (4 surfaces). Assumptions hedges whether `/api/search` is in or out of scope.

- 🟡 **Dependency**: `SsePayload` action discriminator is an implied blocker not captured
  **Location**: Dependencies
  AC6 requires server-supplied action verbs; no current SsePayload field exists. Backend wire-format change is not listed as a blocker.

- 🟡 **Dependency**: Activity history endpoint is an implied blocker not captured
  **Location**: Dependencies
  AC9 requires `GET /api/activity?limit=N` (or equivalent) backed by a ring buffer or persistent log. Net-new backend work, not in Dependencies.

- 🟡 **Dependency**: `/api/search` backend coupling not reflected in Dependencies
  **Location**: Dependencies
  Assumptions hedges in/out of scope; no Blocked-by entry for a search backend story exists.

- 🟡 **Dependency**: Per-doc-type counts source change is an implied prerequisite
  **Location**: Dependencies
  AC2 ties counts to the doc-types API; Technical Notes recommend extending `GET /api/types` — a backend change not captured.

- 🟡 **Testability**: Count badge data source is conditional
  **Location**: Acceptance Criteria (AC2)
  Criterion defers to "options described in Technical Notes — extending `GET /api/types` is the recommended option". Until an option is picked, two implementations could legitimately diverge.

- 🟡 **Testability**: "Last visit" undefined for the unseen-dot trigger
  **Location**: Acceptance Criteria (AC3)
  Persistence strategy is still in Open Questions; without binding "last visit" to a concrete event/timestamp/storage key, the criterion is non-reproducible.

- 🟡 **Testability**: Activity-feed criterion depends on an unspecified wire-format change
  **Location**: Acceptance Criteria (AC6)
  Discriminator values and verb mapping aren't pinned — implementations could ship divergent verbs or wire spellings.

- 🟡 **Testability**: Initial-history endpoint named only as an example
  **Location**: Acceptance Criteria (AC9)
  "e.g. `GET /api/activity?limit=5`" leaves the endpoint, query parameters, and response schema unbound. Cannot drive a contract test.

- 🟡 **Testability**: Search keybind lacks edge-case coverage
  **Location**: Acceptance Criteria (AC4)
  Only the positive case is specified. Implementations that hijack `/` from text inputs, textareas, or contenteditable would still pass.

- 🟡 **Clarity**: Inconsistent action-verb vocabulary across sections
  **Location**: Requirements / Acceptance Criteria
  Requirements: "created / edited / moved-to-status". AC6: "created / edited / moved / deleted". Disagrees on both 'moved' semantics and whether 'deleted' is in scope.

- 🟡 **Clarity**: "Twelve doc types" justified only in Drafting Notes
  **Location**: Requirements / Drafting Notes
  Registry has 13 variants; the rationale for excluding Templates is buried in Drafting Notes. A reader of just Requirements can't resolve whether 12 is authoritative or pending.

#### Minor

- 🔵 **Testability**: Search results rendering lacks observable structure (AC5)
- 🔵 **Testability**: LIVE badge disappearance condition is one-sided (AC6/Open Questions)
- 🔵 **Testability**: Relative-timestamp format undefined (AC6/AC9)
- 🔵 **Testability**: Phase-to-doc-type mapping not enumerated (AC1)
- 🔵 **Completeness**: `/api/search` scope ambiguous in Assumptions
- 🔵 **Completeness**: Two open questions overlap (search ranking duplicated)
- 🔵 **Completeness**: No AC for the `/` keybind hint chip
- 🔵 **Dependency**: Blocks field declares 'none' despite introducing reusable primitives
- 🔵 **Dependency**: 0035 footer-status coupling may be stale
- 🔵 **Scope**: Footer-status removal AC may already be satisfied by 0035
- 🔵 **Clarity**: Ambiguous pronoun "it" in AC9
- 🔵 **Clarity**: AC6 LIVE-badge spec conflicts with an unresolved Open Question
- 🔵 **Clarity**: "Sidebar previously rendered SSE/version status" — uncertain temporal frame
- 🔵 **Clarity**: "Sidebar rectangle" is informal (Drafting Notes)

### Strengths

- ✅ All standard sections present and substantively populated; frontmatter valid; story-form Summary names actor and goals.
- ✅ ACs use Given/When/Then with concrete numeric thresholds (200 ms debounce, 100 ms render budget, 5 events, 2-char minimum).
- ✅ Empty-state behaviour for both zero-results search and zero-events activity feed is called out as distinct verifiable outcomes.
- ✅ Technical Notes are exceptionally thorough, with file:line references that disambiguate "the Sidebar", "the SSE hub", and "the doc-type registry" to single artefacts.
- ✅ Domain terminology (LIBRARY, Glyph, lifecycle phase names, SSE, SsePayload) is anchored to specific files or sibling work items.
- ✅ Upstream blockers (0033, 0035, 0037) are explicitly named with reasons.
- ✅ Drafting Notes transparently acknowledge the bundling decision and pre-identify split points.

### Recommended Changes

1. **Split the story along the Drafting-Notes seams** (addresses: scope ×2, dependency ×3, several testability findings)
   Decompose into at least three child stories: (a) Sidebar nav restructure + count badges + unseen-changes tracker, (b) Sidebar search input + `/api/search` backend, (c) Activity feed + `SsePayload` action discriminator + `/api/activity` history endpoint. Drop the footer-status AC entirely if 0035 has satisfied it.

2. **Resolve hidden backend coupling in Dependencies** (addresses: dependency ×4)
   For each backend change still required after splitting, either declare it in-scope (and own it inside the relevant child story) or list it as a Blocked-by entry pointing to a sibling backend story.

3. **Pin the contracts referenced by ACs before promoting from draft → ready** (addresses: testability ×4 major, clarity ×1 major)
   - AC2: pick option (a)/(b)/(c) for count source and bind the criterion to the chosen API.
   - AC3: define "last visit" concretely (storage key, timestamp semantics, scope per device/tab).
   - AC6: pin discriminator values and the verb-mapping table; reconcile with Requirements vocabulary.
   - AC9: fix the URL, query parameters, and response schema for the history endpoint.
   - Pick one canonical action-verb set across Requirements, ACs, and Technical Notes.

4. **Confirm 0035 status and trim accordingly** (addresses: clarity, dependency, scope minors)
   Verify whether SSE/version footer removal is fully satisfied by 0035. If yes, drop the requirement, AC, and the 0035 dependency. If no, name the residual scope.

5. **Add the small unbound items** (addresses: testability minors, completeness minors)
   Add ACs for the `/`-keybind suppression cases (focused input/textarea/contenteditable), the LIVE-badge negative case, the `/`-hint chip presence, the relative-timestamp format and tick cadence, and a phase-to-doc-type mapping table in Requirements or Technical Notes.

6. **Tidy Open Questions and minor wording** (addresses: completeness, clarity minors)
   Merge the two `/api/search` ranking questions; remove or downgrade the LIVE-badge open question now that AC6 has effectively decided it; replace "it" with "the Activity feed" in AC9; replace "sidebar rectangle" with "Sidebar component"; rephrase "Sidebar previously rendered…" to express the post-0035 invariant directly.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear, with well-structured acceptance criteria that name actors and observable outcomes, and Technical Notes that disambiguate likely implementation choices. A few clarity issues remain: the Activity feed action verb "moved-to-status" in Requirements diverges from the "created/edited/moved/deleted" set in Acceptance Criteria, the count of doc types ("all twelve") depends on an assumption that is only resolved later in Drafting Notes, and a couple of pronouns and ambiguous referents (e.g. "it", "the feed", "sidebar rectangle") could resolve to more than one entity.

**Strengths**:
- Acceptance Criteria use Given/When/Then with named actors (the user, the Sidebar, the Activity feed) and concrete observable outcomes (debounce thresholds, 100 ms render windows, badge text equals N).
- Domain terminology (LIBRARY, Glyph, lifecycle phase names, SSE, SsePayload) is anchored to specific files or sibling work items (0035, 0037), so jargon has a clear definition path.
- Requirements and Acceptance Criteria largely mirror each other section-by-section, making the intent of each requirement traceable to a verification step.
- Technical Notes preempt several ambiguity risks by naming exact files, line numbers, and existing primitives, so "the Sidebar", "the SSE hub", and "the doc-type registry" resolve to single artefacts.

**Findings**:

- 🟡 **Major / High** — *Inconsistent action-verb vocabulary for Activity feed events* (Requirements / Acceptance Criteria)
  Requirements bullet: "created / edited / moved-to-status". AC6: "created / edited / moved / deleted". Disagrees on both 'moved' vs 'moved-to-status' and on whether 'deleted' is in scope. A reader cannot tell whether the feed must surface deletions, whether 'moved' is a generic file move or specifically a status-folder transition, or which set is authoritative for the wire format change.

- 🟡 **Major / High** — *"Twelve doc types" asserted in Requirements but only justified in Drafting Notes* (Requirements / Drafting Notes)
  Registry has 13 variants; rationale for excluding Templates is buried in Drafting Notes as an assumption. Drafting-Notes caveat ("If Templates should appear, the count and phase mapping need updating") is invisible from the requirement text.

- 🔵 **Minor / Medium** — *Ambiguous pronoun "it" in Activity-feed history criterion* (Acceptance Criteria)
  "…when the Activity feed first renders, then it shows…" — "it" could refer to either the Activity feed or the app.

- 🔵 **Minor / Medium** — *"LIVE badge while connectionState is open" conflicts with Open Question on badge behaviour* (Acceptance Criteria)
  AC6 has effectively answered the open question (permanent while connected) but the Open Question is still listed as unresolved.

- 🔵 **Minor / Medium** — *"Sidebar previously rendered SSE/version status" — uncertain referent* (Acceptance Criteria)
  Technical Notes records that `SseIndicator` lives in the Topbar today and `Sidebar.test.tsx` asserts the version label is absent. "Previously" may refer to a historic state already removed by 0035.

- 🔵 **Minor / Low** — *"Sidebar rectangle" is informal and undefined* (Drafting Notes)
  Phrase is non-standard within the codebase vocabulary used elsewhere in the work item.

### Completeness

**Summary**: The work item is structurally complete and substantively populated across all expected sections for a story of this scope. Frontmatter is valid, the Summary states intent clearly, Acceptance Criteria are numerous and specific, and Context, Requirements, Dependencies, Assumptions, Open Questions, Technical Notes, and References are all present and meaningfully populated. A few minor completeness concerns relate to scope ambiguity surfaced in the Assumptions/Drafting Notes that could leave the implementer uncertain about whether `/api/search` is in scope.

**Strengths**:
- All standard sections (Summary, Context, Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions, Technical Notes, References) are present and substantively populated.
- Frontmatter integrity is solid: `type: story`, `status: draft`, `priority: high`, plus tags, parent, author, and date are all present and recognised values.
- Acceptance Criteria contain nine specific Given/When/Then bullets covering grouping, count badges, unseen-changes dot, search keybind, search debounce, activity-feed updates, footer removal, no-match empty state, and initial-history empty state.
- Context explains both the current Sidebar state and the prototype's target behaviour, providing strong motivational framing for the work.
- Technical Notes is exceptionally thorough — covers component locations, SSE wire shape limits, count-source options, persistence precedent, and net-new vs reusable primitives, giving an implementer a concrete starting point.
- Story-form Summary identifies the user ("a user navigating the visualisation app") and the goals (locate, notice updates, stay oriented), satisfying the type-specific "for whom" expectation.

**Findings**:

- 🔵 **Minor / Medium** — *Scope of `/api/search` backend left ambiguous* (Assumptions)
  Assumptions states the `/api/search` backend "is in scope for this work item (or will be coordinated as a parallel backend story)". Implementer cannot tell whether they deliver the server endpoint or only the frontend integration.

- 🔵 **Minor / Medium** — *Two open questions overlap (search ranking duplicated)* (Open Questions)
  Two near-identical entries about `/api/search` matching/ranking strategy — readers may answer one and leave the other dangling.

- 🔵 **Minor / Low** — *No acceptance criterion for the `/` keybind hint chip* (Acceptance Criteria)
  Requirements describe a `/` hint chip alongside the search input, but no AC asserts the chip is visible. Behavioural pass could ship without the visible chip.

### Dependency

**Summary**: The work item captures the major upstream blockers (0033 token system, 0035 Topbar, 0037 Glyph component) explicitly in Dependencies, and the Technical Notes do a strong job naming the server-side primitives this story depends on. However, several couplings implied by the body — a new `/api/search` backend, an `/api/activity?limit=N` history endpoint, and a server-side change to `SsePayload` to add action discriminators — are described as in-scope or required but are not captured in Dependencies as backend work that must be coordinated. The Blocks field is empty even though the work introduces new public APIs other consumers may depend on.

**Strengths**:
- Upstream story-level blockers (0033, 0035, 0037) are explicitly named in Dependencies with the reason for each coupling.
- Technical Notes make implementation-level coupling visible (specific files, hook points, existing primitives) so prerequisites are traceable.
- The work item explicitly acknowledges in Assumptions that `/api/search` may need to be coordinated as a parallel backend story, surfacing a coupling that would otherwise be hidden.

**Findings**:

- 🟡 **Major / High** — *Server-side `SsePayload` change is an implied blocker not captured in Dependencies* (Dependencies)
  AC6 requires server-supplied action discriminators (created / edited / moved / deleted); current `SsePayload` has no such field. Backend wire-format change is a hard prerequisite for AC6 but not in Dependencies.

- 🟡 **Major / High** — *Activity history endpoint is an implied blocker not captured in Dependencies* (Dependencies)
  AC9 requires up to five most-recent events from a server-provided initial-history endpoint; `SseHub` has no replay buffer. Net-new backend work, absent from Dependencies.

- 🟡 **Major / High** — *`/api/search` backend endpoint coupling not reflected in Dependencies* (Dependencies)
  Requirements and AC5 depend on a new `/api/search` endpoint; Assumptions hedges in/out of scope. No Blocked-by entry exists.

- 🟡 **Major / High** — *Per-doc-type counts source change is an implied prerequisite not captured* (Dependencies)
  AC2 ties counts to the doc-types API; Technical Notes recommend extending `GET /api/types` — server-side change to a public API not captured.

- 🔵 **Minor / Medium** — *Blocks field declares 'none' despite introducing reusable primitives* (Dependencies)
  Story introduces `/api/search`, an unseen-changes Context/Provider, and an Activity feed pattern — plausible downstream consumers may be implicitly waiting.

- 🔵 **Minor / Medium** — *Coupling to 0035 footer-status removal is ambiguous* (Technical Notes)
  Dependencies lists 0035 as Blocked-by, but Technical Notes observe `SseIndicator` is already in `Topbar.tsx:17`. Confirmation step deferred.

### Scope

**Summary**: This story bundles four substantially independent capabilities (nav restructure with unseen-dots, search, activity feed, and footer-status removal) into a single work item, and the author explicitly acknowledges natural split points in the Drafting Notes. While all four touch the Sidebar rectangle and share some SSE infrastructure, each has its own backend dependency surface and could be delivered, reviewed, and rolled back independently — making this story significantly oversized for its declared type.

**Strengths**:
- All requirements are spatially coherent — they all concern the Sidebar surface — giving the work a clear visual boundary.
- Dependencies on sibling stories (0033 token system, 0035 Topbar, 0037 Glyph) are explicitly captured, signalling the author thought about adjacent scope.
- The Drafting Notes transparently acknowledge the bundling decision and pre-identify the three natural split-points.

**Findings**:

- 🟡 **Major / High** — *Story bundles four independently-deliverable capabilities* (Requirements)
  Four largely orthogonal capabilities, each with its own backend surface. Drafting Notes pre-identify natural split points. Bundling inflates scope, couples unrelated backend changes into one delivery, and concentrates risk.

- 🟡 **Major / High** — *Story spans multiple service boundaries with no orchestration plan* (Requirements)
  Coordinated changes required across Rust server (4 distinct backend surface changes) and React frontend. Assumptions hedges whether `/api/search` is in or out of scope, leaving the boundary ambiguous and the work item unactionable as written.

- 🔵 **Minor / Medium** — *Footer-status removal AC may already be satisfied by 0035* (Acceptance Criteria)
  Technical Notes record `SseIndicator` in `Topbar.tsx:17` and `Sidebar.test.tsx:52-56` already asserts version label is absent. If true, this AC is not a unit of work belonging to this story.

### Testability

**Summary**: The work item presents a strong set of acceptance criteria, with most criteria framed as Given/When/Then with concrete thresholds (debounce timing, character minimums, response latency budgets). However, several criteria contain unverifiable elements — the count's data source is left ambiguous, the unseen-changes trigger condition lacks a clear specification of what 'last visit' means, and the initial-history endpoint is named only as an example. A handful of behaviours described in the Requirements (e.g. relative-timestamp formatting, search results rendering, keybind suppression rules) are not fully covered by measurable criteria.

**Strengths**:
- Most criteria use explicit Given/When/Then framing with concrete numeric thresholds (200 ms debounce, 100 ms render budget, 5 events, 2-character minimum).
- The search criterion specifies request frequency ("exactly once per settled query") and the no-match criterion explicitly distinguishes empty state from stale list / silent failure — both clearly testable.
- Empty-state behaviour for both zero-results search and zero-events activity feed is called out as distinct verifiable outcomes.
- The footer-removal criterion gives a concrete observable: "no SSE/version status appears in the Sidebar" — straightforward presence/absence assertion.

**Findings**:

- 🟡 **Major / High** — *Count badge criterion's data source is conditional, leaving the test ambiguous* (AC2)
  Criterion defers to "options described in Technical Notes — extending `GET /api/types` is the recommended option". Two implementers could legitimately satisfy with different APIs and divergent counts.

- 🟡 **Major / High** — *"Last visit" is undefined, making the unseen-dot trigger unverifiable* (AC3)
  "Last visit" is not defined as a concrete event (first list-view open? per-tab? cleared on logout? per device?). Open Questions explicitly flags persistence strategy as undecided.

- 🟡 **Major / High** — *Activity-feed criterion depends on a wire-format change that is not specified* (AC6)
  Discriminator values, wire-format spelling, and verb-display mapping are not pinned. Two implementers could ship divergent verb mappings or wire spellings.

- 🟡 **Major / High** — *Initial-history endpoint named only as an example, not a binding spec* (AC9)
  "e.g. `GET /api/activity?limit=5`" leaves URL, query parameters, and response schema unbound. Cannot drive a contract test.

- 🟡 **Major / High** — *Search keybind criterion lacks coverage of edge cases asserted by Requirements* (AC4)
  Only the positive case is specified; pressing `/` while typing into another input/textarea/contenteditable is implicit.

- 🔵 **Minor / High** — *"Matches are rendered inline" lacks observable structure* (AC5)
  No specification of what each match row must contain (icon? title? type? path? click target?).

- 🔵 **Minor / High** — *LIVE badge disappearance condition is one-sided* (AC6)
  Criterion asserts badge shows while `connectionState === 'open'` but does not specify what is shown otherwise.

- 🔵 **Minor / Medium** — *"Relative-timestamp string" format is not defined* (AC6 / AC9)
  Format ("5m ago", "just now", "2 hours ago"?), tick frequency, and live-update behaviour unspecified.

- 🔵 **Minor / Medium** — *Phase-to-doc-type mapping is not enumerated, leaving the grouping criterion partially unverifiable* (AC1)
  Neither Requirements nor Technical Notes pin down which doc types belong to which phase.
