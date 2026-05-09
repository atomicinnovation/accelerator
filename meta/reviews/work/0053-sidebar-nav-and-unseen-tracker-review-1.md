---
date: "2026-05-11T15:44:14+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0053-sidebar-nav-and-unseen-tracker.md"
work_item_id: "0053"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
---

## Work Item Review: Sidebar Nav and Unseen-Changes Tracker

**Verdict:** REVISE

Work item 0053 is well-structured and substantively populated across all expected sections, with explicit actors, concrete storage keys, and a clean linkage to its parent epic 0036 and siblings 0054/0055. The dominant concern is a cross-cutting ambiguity about the SSE event's timestamp source — flagged by three lenses — which threatens both the dependency ordering with 0055 and the testability of AC3. A secondary cluster of gaps in the unseen-tracker's state-machine ACs (first-run, event-during-view, event-after-clear) leaves the most-hit code paths without verifiable outcomes.

### Cross-Cutting Themes

- **SSE timestamp source is ambiguous and likely couples to 0055** (flagged by: clarity, dependency, testability) — AC3 compares a "server-supplied timestamp" against the stored T, but the parent epic 0036 explicitly places the `SsePayload::DocChanged.timestamp` field in 0055's scope. The story's Assumptions only carve out the `action` discriminator and are silent on `timestamp`. Without resolution, AC3 is either untestable or implies an uncaptured `Blocked by: 0055`.
- **Unseen-tracker state-machine edges are underspecified** (flagged by: clarity, testability) — Three realistic transitions have no defined expected outcome: (a) first-run when no entry for X exists in `localStorage`, (b) event arrives while the user is on X's list view, (c) event arrives strictly after the dot was cleared. Compounded by a Requirements/AC4 inconsistency between "last visited" and "current time" semantics for T.

### Findings

#### Major

- 🟡 **Dependency**: Unseen tracker depends on a server-supplied SSE timestamp that is not captured as a prerequisite
  **Location**: Requirements / Assumptions
  Requirements and AC3 compare a "server-supplied timestamp" against T, but parent epic 0036 places the `SsePayload.timestamp` extension in 0055. Either add 0055 as `Blocked by`, confirm in Assumptions that today's payload already carries a server timestamp, or fall back to client-received-time and adjust AC3.

- 🟡 **Testability**: Unseen-changes AC does not specify behaviour when no prior timestamp T exists
  **Location**: Acceptance Criteria (AC3)
  AC3 is conditioned on a stored T for doc type X, but says nothing about the first-run case where no entry for X exists yet. This is the most common real-world state and has no defined expected outcome.

- 🟡 **Testability**: No AC covers the "event arrives while list view is open" or "event arrives after dot cleared" cases
  **Location**: Acceptance Criteria (AC3, AC4)
  Two adjacent transitions in the tracker's state machine — event-during-view and event-after-clear — have no verifiable expected outcome. Both are realistic flows directly implied by the design.

#### Minor

- 🔵 **Clarity**: 'Glyph' used as a proper noun without inline definition
  **Location**: Requirements / Acceptance Criteria
  "Glyph" is capitalised throughout but never defined inline — a reader unfamiliar with 0037 must context-switch to discover whether it's an icon component, an SVG asset, or something else. Add a one-line gloss on first use.

- 🔵 **Clarity**: Ambiguous referent for "server-supplied timestamp" relative to T
  **Location**: Acceptance Criteria (AC3)
  AC3's timestamp source is unclear given the story's own Assumption that today's payload is sufficient — could be client receipt time or a server field that doesn't yet exist. State explicitly which source is used in this story's scope.

- 🔵 **Clarity**: Inconsistency: "last visited" vs "current time" for clear-on-visit semantics
  **Location**: Requirements vs Acceptance Criteria (AC4)
  Requirements describe T as the "last visited" timestamp; AC4 specifies updating T to "the current time" on mount. These are not necessarily the same instant. Align the wording.

- 🔵 **Dependency**: Backend `GET /api/types` change is a self-contained coupling but not surfaced in Dependencies
  **Location**: Requirements / Technical Notes
  AC2 reads the count from `GET /api/types`'s new `count` field, creating an internal ordering: the server change must merge or deploy before the frontend feature is usable end-to-end. Add a note on the ordering or feature-gate strategy.

- 🔵 **Dependency**: Blocks entries for 0054 / 0055 do not name the specific shared artefacts
  **Location**: Dependencies
  This story introduces a `useMarkSeen` hook, a `phase-to-doc-type` constant, and the `ac-seen-doc-types` storage key — none explicitly listed as artefacts 0054/0055 may consume. Confirm no shared primitives leak, or name them.

- 🔵 **Scope**: Story title and Summary name two concerns (nav + unseen-changes tracker)
  **Location**: Summary
  The title joins two concerns with "and"; the tracker is a self-contained subsystem with its own persistence and `LibraryTypeView` touchpoint. Either rename to emphasise a single unifying concept (e.g. "Sidebar nav with per-type change indicators") or split into a fourth child story.

- 🔵 **Testability**: "Within one render cycle" is not a deterministically verifiable threshold
  **Location**: Acceptance Criteria (AC4)
  Rephrase as an observable outcome at a defined point — e.g. "after `LibraryTypeView` for X has finished its mount effects, querying the Sidebar nav item for X shows no unseen-changes dot".

- 🔵 **Testability**: Server-supplied timestamp comparison semantics unspecified
  **Location**: Acceptance Criteria (AC3)
  AC3's verifiability depends on whether the SSE timestamp field already exists or arrives with 0055. Pin the format (e.g. ISO-8601 UTC from `SsePayload`) and confirm availability or list 0055 as a blocker for this portion.

- 🔵 **Testability**: AC1 doesn't cover Templates exclusion or intra-phase ordering
  **Location**: Acceptance Criteria (AC1)
  0036 specifies both Templates exclusion and intra-phase display order, but AC1 does not bind them. A passing implementation could list Templates in LIBRARY or shuffle within a phase. Add explicit clauses.

- 🔵 **Testability**: Count badge AC does not define behaviour for count = 0 or missing count
  **Location**: Acceptance Criteria (AC2)
  Zero-count and degraded-response cases have no expected outcome. Specify whether the badge is hidden or shown as "0" for N=0, and behaviour when `count` is absent.

- 🔵 **Testability**: "Adjacent to" is an under-specified spatial relationship
  **Location**: Acceptance Criteria (AC5)
  Pin the relationship — e.g. "the chip is rendered as the search input's end-adornment within the same flex row" — or reference a specific screenshot region.

#### Suggestions

- 🔵 **Clarity**: 'N' used as both the document count and the badge text label
  **Location**: Acceptance Criteria (AC2)
  Reword AC2 to make `count` from `GET /api/types` the sole authoritative source for the badge text and drop the "N documents in the list view" framing.

- 🔵 **Scope**: Search input slot is a cross-story scaffold and could be called out as such
  **Location**: Requirements
  Add a note recording the trade-off: 0053 ships an inert input container if 0054 slips. Either pair the merges or visually hide the slot until 0054 lands.

### Strengths

- ✅ Frontmatter is complete with valid type/status/priority and explicit parent linkage to 0036.
- ✅ Summary is user-centric ("As a user… I want… so that…") and the six ACs map one-to-one with Requirements.
- ✅ Actors and code locations are anchored to concrete file paths and line ranges (`server/src/docs.rs:90-99`, `LibraryTypeView.tsx:41-54`, `Sidebar.test.tsx:7-21`).
- ✅ Epic decomposition is principled: 0053 / 0054 / 0055 split along genuine seams; backend scope is tightly bounded to the `count` extension.
- ✅ Persistence contract is precisely specified (`localStorage` key `ac-seen-doc-types`, shape `Record<DocTypeKey, ISO-8601 string>`), making the tracker directly inspectable.
- ✅ Phase-to-doc-type table is referenced rather than restated, eliminating drift risk.
- ✅ Out-of-scope behaviour (search wiring) is explicitly called out in AC5, preventing ambiguous test failures.
- ✅ Sibling boundaries are clean: 0055's tracker non-modification and 0054's search-slot consumption are stated.

### Recommended Changes

1. **Resolve the SSE timestamp source** (addresses: Dependency major, Clarity AC3, Testability AC3 minor)
   Pick one path and update Assumptions + AC3 to match:
   (a) Add `0055` (or specifically the `SsePayload.timestamp` field) to `Blocked by` and keep AC3 as written.
   (b) Confirm an existing server timestamp field on today's payload and cite it.
   (c) Fall back to client-received-time, restate AC3 to compare client receipt time vs T, and note 0055 may later promote this to a server timestamp.

2. **Fill the unseen-tracker AC gaps** (addresses: Testability AC3 first-run, Testability event-during-view / after-clear)
   Add ACs covering:
   - "Given no entry for X in `localStorage[ac-seen-doc-types]`, when a `doc-changed` event for X arrives, then the dot is [shown | not shown]" (decide and pin).
   - "Given `LibraryTypeView` for X is mounted, when a `doc-changed` event for X arrives, then T for X is bumped and no dot is shown."
   - "Given the user cleared the dot at T_new, when a later event arrives with timestamp > T_new, then the dot is shown again."

3. **Align Requirements and AC4 wording for T** (addresses: Clarity Requirements vs AC4)
   Replace "last visited" in Requirements with the same wording AC4 uses, defining T as "the wall-clock time at which `LibraryTypeView` last mounted for that doc type".

4. **Tighten AC1, AC2, AC4, AC5** (addresses: Testability AC1 ordering, AC2 zero/missing, AC4 render cycle, AC5 adjacency)
   - AC1: add "Templates does not appear in LIBRARY" and "within each phase, items appear in 0036's display order".
   - AC2: define behaviour for `count = 0` and missing `count` field.
   - AC4: rephrase "within one render cycle" as an observable outcome after mount effects flush.
   - AC5: pin the chip's spatial relationship to the input (e.g. end-adornment within the same row).

5. **Add a one-line Glyph gloss on first use** (addresses: Clarity Glyph)
   First-mention parenthetical, e.g. "(a typed icon component delivered by 0037)".

6. **Note the server/frontend ordering for the `count` field** (addresses: Dependency minor on `GET /api/types`)
   Add a Technical Notes bullet: server change must deploy before the frontend renders, or feature-gate the badge on `typeof count === 'number'`.

7. **Consider a story rename or split** (addresses: Scope minor)
   Either rename to "Sidebar nav with per-type change indicators" so it reads as one capability, or split the tracker into a fourth child of 0036.

8. **Record the search-slot scaffolding trade-off** (addresses: Scope suggestion)
   One-line note in Assumptions: 0053 ships the slot inert; 0054 wires behaviour. If 0054 slips, the slot ships as dead UI.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Work item 0053 is generally clear, with named actors, explicit storage keys, and references to concrete code locations and a parent canonical table. Most requirements and acceptance criteria have unambiguous referents and observable outcomes. A few minor issues exist: an undefined term ('Glyph'), a potential pronoun-style ambiguity around 'doc-changed' event timestamps versus 'last visited' timestamps in AC3, and one subtle inconsistency between Requirements and Acceptance Criteria for the clear-on-visit behaviour.

**Strengths**:
- Pronouns are largely scoped — "this story", "the tracker", and "the Sidebar" resolve to named referents.
- Actors are explicit for almost every action.
- Cross-section scope is consistent across Summary, Requirements, and Acceptance Criteria.
- Domain terms anchored to explicit file paths and line ranges.
- Canonical phase-to-doc-type table is referenced rather than restated.

**Findings**:
- 🔵 minor / medium — "Glyph" used as a proper noun without inline definition (Requirements / Acceptance Criteria)
- 🔵 minor / medium — Ambiguous referent for "server-supplied timestamp" relative to T (AC3)
- 🔵 minor / high — Inconsistency: "last visited" vs "current time" for clear-on-visit semantics (Requirements vs AC4)
- 🔵 suggestion / medium — "N" as both the document count and the badge text label (AC2)

### Completeness

**Summary**: Work item 0053 is a well-structured story with all expected sections substantively populated. Frontmatter is complete and uses recognised values. Type-appropriate content for a story is present: user-centric summary, context explaining motivation through parent epic, and six specific acceptance criteria.

**Strengths**:
- Frontmatter complete with valid type, status, priority, parent linkage, and tags.
- Summary opens with "As a user… I want… so that…".
- Six Given/When/Then-shaped ACs mapping one-to-one with Requirements.
- Context explains relationship to parent epic and siblings plus current Sidebar state.
- Dependencies, Assumptions, Technical Notes, and Open Questions all substantively populated.

**Findings**: (none)

### Dependency

**Summary**: Dependencies are well captured overall: 0053 explicitly names 0033 and 0037 as blockers, lists 0054 and 0055 as Blocks, and the parent epic 0036 confirms the same ordering. Couplings to localStorage, SSE `doc-changed` events, and the `GET /api/types` endpoint extension are stated. One uncaptured upstream coupling deserves attention: the SSE timestamp behaviour the unseen tracker relies on.

**Strengths**:
- Upstream blockers (0033, 0037) explicitly named and match the parent epic.
- Downstream consumers (0054 search slot, 0055 activity feed) explicitly listed as Blocks.
- Cross-story coupling with 0055 addressed in Assumptions for the `action` discriminator.
- Phase-to-doc-type mapping referenced back to canonical location in 0036.

**Findings**:
- 🟡 major / high — Unseen tracker depends on a server-supplied SSE timestamp not captured as a prerequisite (Requirements / Assumptions)
- 🔵 minor / medium — Backend `GET /api/types` change is a self-contained coupling but not surfaced in Dependencies (Requirements / Technical Notes)
- 🔵 minor / medium — Blocks entries for 0054 / 0055 do not name the specific artefacts being depended on (Dependencies)

### Scope

**Summary**: Story 0053 is reasonably scoped as the foundational child of the 0036 epic and sits coherently alongside its siblings 0054 (search) and 0055 (activity feed). Its three concerns are tightly coupled to the Sidebar surface, and the lone backend change is minimal. The most defensible scope question is whether the unseen-changes tracker — a self-contained SSE+localStorage subsystem — should be its own story.

**Strengths**:
- Epic decomposition is principled, split along genuine seams.
- Backend scope tightly bounded to the `count: usize` extension.
- Search input slot inclusion justified as single-pass chrome layout.
- Templates exclusion, phase mapping, and persistence strategy anchored to parent epic.
- Sibling boundaries are clean with no overlapping ownership.

**Findings**:
- 🔵 minor / medium — Story title and Summary name two concerns (nav + unseen-changes tracker) (Summary)
- 🔵 suggestion / medium — Search input slot is a cross-story scaffold and could be called out as such (Requirements)

### Testability

**Summary**: The story's Acceptance Criteria are largely well-framed as Given/When/Then observable behaviours with concrete preconditions, actions, and outcomes. The main testability gaps are an unbounded-language risk in the unseen-tracker AC (no behaviour specified when no prior timestamp T exists), a missing AC for the SSE-event-after-mount/clear case, and one AC whose 'within one render cycle' threshold is hard to verify deterministically.

**Strengths**:
- Each AC names a concrete observable (badge text equals N, presence of dot, presence of slot).
- Persistence contract precisely specified for direct inspection.
- AC1 anchors phase grouping to canonical table for fixed mapping assertions.
- Out-of-scope behaviour (search wiring) explicitly called out in AC5.

**Findings**:
- 🟡 major / high — Unseen-changes AC does not specify behaviour when no prior timestamp T exists (AC3)
- 🟡 major / high — No AC covers the "event arrives while list view is open" or "event arrives after dot cleared" cases (AC3, AC4)
- 🔵 minor / medium — "Within one render cycle" is not a deterministically verifiable threshold (AC4)
- 🔵 minor / medium — Server-supplied timestamp comparison semantics unspecified (AC3)
- 🔵 minor / high — "Every doc type appears under the phase specified" has no AC covering Templates exclusion or ordering (AC1)
- 🔵 minor / high — Count badge AC does not define behaviour for count = 0 or missing count (AC2)
- 🔵 minor / medium — "Adjacent to" is an under-specified spatial relationship (AC5)

## Re-Review (Pass 2) — 2026-05-11

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Dependency**: Unseen tracker depends on server-supplied SSE timestamp — **Resolved** (Assumptions states tracker uses client receipt time; 0055 may later promote without breaking AC).
- 🟡 **Testability**: AC silent on no-prior-T first run — **Resolved** (new AC4 seeds T to now and shows no dot).
- 🟡 **Testability**: No AC for event-during-view / event-after-clear — **Resolved** (new AC5 and AC6).
- 🔵 **Clarity**: "Glyph" undefined — **Resolved** (inline gloss).
- 🔵 **Clarity**: AC3 server-supplied timestamp ambiguous — **Resolved** (now client receipt time).
- 🔵 **Clarity**: "last visited" vs "current time" — **Resolved** (T defined as wall-clock time of last mount).
- 🔵 **Clarity**: AC2 N-vs-count framing — **Resolved** (`count` is sole authoritative source).
- 🔵 **Dependency**: `GET /api/types` rollout ordering — **Resolved** (Technical Notes bullet).
- 🔵 **Dependency**: Shared frontend primitives — **Resolved** (Technical Notes enumeration).
- 🔵 **Scope**: Title bundles two concerns — **Resolved** (renamed to "Sidebar Nav with Per-Type Change Indicators").
- 🔵 **Scope**: Search-slot scaffolding trade-off — **Resolved** (Assumptions note).
- 🔵 **Testability**: "Within one render cycle" — **Resolved** ("after mount effects have finished").
- 🔵 **Testability**: AC3 server timestamp semantics — **Resolved** (Assumptions defines source).
- 🔵 **Testability**: AC1 Templates / ordering — **Resolved**.
- 🔵 **Testability**: AC2 zero / missing count — **Resolved**.
- 🔵 **Testability**: AC5 "adjacent to" — **Resolved** ("end-adornment within the same row").

### New Issues Introduced

#### Major

- 🟡 **Testability**: Race between AC5 (event-during-view) and AC7 (bump-on-mount) lacks tie-breaking
  **Location**: Acceptance Criteria (AC5 vs AC7)
  Equality at the millisecond ("greater than" vs "greater than or equal") and the mount-window definition (first render vs mount-effect completion) are unspecified. Add a tie-breaking clause and state that the comparison in AC3/AC6 is strictly greater-than.

- 🟡 **Testability**: "Client receipt time" is not defined as an observable quantity
  **Location**: Acceptance Criteria (AC3, AC5, AC6) / Assumptions
  `Date.now()` in the SSE `onmessage` handler? Event timestamp? Pin the canonical clock source so tests can deterministically construct comparable values; align T's "current time" with the same source.

#### Minor

- 🔵 **Clarity**: T's "last mounted" definition mildly conflicts with the bump-on-in-mount-event rule (Requirements). Reword T as "most recent time the user can be assumed to have seen changes" and list the three T-advancing events.
- 🔵 **Dependency**: Parent epic 0036's phase-to-doc-type table is a fixed input but 0036 is not listed in Dependencies. Add a line stating "Consumes the frozen phase-to-doc-type table from 0036 Technical Notes — assumed stable" (or surface as Assumption).
- 🔵 **Dependency**: Server/frontend `count` rollout ordering noted in Technical Notes but not surfaced in Dependencies. Add a brief Dependencies note so planners scanning only Dependencies see the intra-story ordering.
- 🔵 **Testability**: AC8 search-slot needs a structural identifier (e.g. `data-testid="sidebar-search"` + `data-testid="sidebar-search-keybind-hint"`) to be testable beyond visual intent.
- 🔵 **Testability**: AC9 Glyph-presence has no observable identification of the Glyph (e.g. `data-glyph={DocTypeKey}`).
- 🔵 **Testability**: AC2 doesn't bound N for malformed values or very large counts — pin the format function (e.g. `String(N)` with no upper cap, or "malformed → treated as absent").

#### Suggestions

- 🔵 **Clarity**: AC3's "at time T" Given implies a single fixed T set by opening the list view; restate as "Given `localStorage[ac-seen-doc-types][X]` holds an ISO-8601 timestamp T (set per the lifecycle in Requirements)".
- 🔵 **Clarity**: Unseen-tracker Requirements bullet packs 5 rules into one sentence — break into sub-bullets for skim-readability.
- 🔵 **Scope**: Note the rollback path if 0054 is cancelled — remove inert slot in a follow-up chore.

### Assessment

The work item improved substantially: every prior finding was resolved. The two new majors are a different class of issue from the originals — they pin down millisecond-level tie-breaking and the canonical clock source rather than capture missing scope or dependencies. Closing them needs only a few precise additions to the ACs and Assumptions, but they are real — without them, tests around the mount window are non-deterministic. The story is close to ready; one more focused pass on AC5/AC7 timing semantics and the clock-source definition should clear the bar for implementation.

## Re-Review (Pass 3) — 2026-05-11 — testability only

**Verdict:** COMMENT

### Previously Identified Issues (Pass 2 majors)

- 🟡 **Testability**: Race between AC5 and AC7 lacked tie-breaking — **Resolved** (AC5 now defines the mount window as "first render through unmount, regardless of whether the AC7 mount-effect bump has yet completed"; Assumptions adds strict-greater-than comparison semantics).
- 🟡 **Testability**: "Client receipt time" not defined as observable — **Resolved** (Assumptions pins both client receipt time and "current time" to `Date.now()` with named call sites, plus ISO-8601 serialisation path).

### New Issues Introduced

#### Minor

- 🔵 **Testability**: AC7 still uses "after `LibraryTypeView` has finished its mount effects" without naming a concrete observable (AC). Could be pinned to "after the `useMarkSeen(X)` mount-effect has executed (verifiable post-`act()` flush)". Behavioural correctness is covered by AC5 either way; this is a verification-ergonomics nit.

#### Suggestions

- 🔵 **Testability**: AC4 seed-on-first-event could clarify that the seeded T uses the same `Date.now()` reading as the event's client receipt time, so the strict-greater rule deterministically yields no dot for the seeding event itself. Currently implied but not stated.

### Assessment

Both Pass 2 majors are resolved. The story now satisfies the testability bar for implementation. No criticals or majors remain across any lens; the open items are minor wording tightening that can be addressed during planning or in a small follow-up. Verdict moves to COMMENT — the work item is acceptable as-is and ready for `/create-plan`.
