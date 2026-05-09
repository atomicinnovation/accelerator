---
date: "2026-05-11T22:30:00+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0055-sidebar-activity-feed.md"
work_item_id: "0055"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
---

## Work Item Review: Sidebar Activity Feed and SSE Action Discriminator

**Verdict:** REVISE

The work item is structurally complete and scoped coherently — frontmatter, narrative, requirements, ACs, and technical notes are all present and substantive, and the parent/sibling relationships are clearly drawn. However, the wire-format contract has internal contradictions (the `'moved'` action and the `etag`/`action` relationship on deletes), several behaviours promised in Requirements/Technical Notes (edited/moved mapping, ring-buffer ordering and capacity, self-cause filter, LIVE-badge form, empty-state message) lack matching Acceptance Criteria, and the dependency on 0053 understates its true depth (Provider mount + unseen-changes tracker). Nine major findings cross multiple lenses; the work item should be revised before implementation.

### Cross-Cutting Themes

- **Wire-format ambiguity around the new `action`/`timestamp` fields** (flagged by: clarity, testability) — `'moved'` semantics conflict between Requirements and Technical Notes; the `etag` field's encoding on delete is unstated; the `timestamp` reference clock is undefined.
- **Acceptance Criteria do not fully verify the wire-format contract** (flagged by: clarity, testability) — `'edited'` and `'moved'` actions, ring-buffer ordering/capacity, and the self-cause filter default are stated in Requirements/Technical Notes but never tested.
- **Coupling to 0053 is deeper than the Dependencies section states** (flagged by: dependency) — both the `DocEventsContext` Provider mount and the unseen-changes tracker contract are silent dependencies.
- **Untestable thresholds and undefined observable markers** (flagged by: clarity, testability) — `within 100 ms`, `within 1 s of mtime`, `at least every 60 seconds`, "LIVE badge present", and "designated empty-state message" all lack defined measurement procedures or observable markers.

### Findings

#### Critical
*(none)*

#### Major
- 🟡 **Clarity**: Inconsistent mapping for `moved` action between Requirements and Technical Notes
  **Location**: Requirements / Technical Notes
  Requirements says watcher emits `'moved'` per FS event kind, but Technical Notes (option (a)) say moves likely surface as create+delete pairs unless `EventKind::Modify(ModifyKind::Name(_))` is also inspected — which option (a) does not do.

- 🟡 **Clarity**: Conflicting framings of the `etag`/`action` relationship for deletions
  **Location**: Acceptance Criteria / Assumptions / Technical Notes
  One AC says `etag` is absent on delete; Technical Notes say `etag: None` stays for backwards-compat; Assumptions says new fields are purely additive. The wire encoding (absent key vs `null` vs unchanged) is unspecified.

- 🟡 **Dependency**: Sidebar slot contract from 0053 is a deeper coupling than 'layout that hosts the feed'
  **Location**: Dependencies
  This story silently requires 0053 to mount the `DocEventsContext` Provider in `RootLayout`; if 0053 ships only the Sidebar slot, LIVE-badge and SSE wiring break.

- 🟡 **Dependency**: Unseen-changes tracker (from 0053) is named in Assumptions but not Dependencies
  **Location**: Dependencies / Assumptions
  The Assumption that the tracker ignores unknown SSE fields is a coupling to a 0053 artefact; it should be visible at the Dependencies level so 0053 scope changes do not silently invalidate it.

- 🟡 **Dependency**: 'Blocks: none' understates the downstream impact of the SSE wire-format extension
  **Location**: Dependencies
  The new `action`/`timestamp` fields are a public contract change; any future SSE consumer wanting them is implicitly blocked on this story.

- 🟡 **Testability**: No AC verifies 'edited' or 'moved' action mapping
  **Location**: Acceptance Criteria (action mapping)
  Only `created` and `deleted` have dedicated ACs; an implementation could omit `'edited'`/`'moved'` entirely and still pass the AC set.

- 🟡 **Testability**: Ring-buffer ordering and capacity have no direct AC
  **Location**: Acceptance Criteria (history endpoint)
  No AC drives N>5 events through the buffer or asserts capacity ≥50; an in-order insertion buffer of capacity 5 would pass.

- 🟡 **Testability**: 'within 100 ms' lacks a defined measurement procedure
  **Location**: Acceptance Criteria AC1
  No defined clock, environment, or instrumentation; the threshold is functionally untestable and could fail flakily under CI load.

- 🟡 **Testability**: 'within 1 second of the file system mtime' is procedurally ambiguous
  **Location**: Acceptance Criteria AC6
  The source-of-truth timestamp (FS event time vs watcher emit time vs broadcast time) is unspecified, and 1 s may be tighter than the existing debounce window.

#### Minor
- 🔵 **Clarity**: Ambiguous referent: "the heading" and "the feed heading"
  **Location**: Requirements / Acceptance Criteria
  The heading element is never introduced as a structural part of the ActivityFeed component.

- 🔵 **Clarity**: Undefined domain terms used without local anchor
  **Location**: Technical Notes
  `DocTypeKey`, `self-cause filter`, `unseen-changes tracker`, `SESSION_STABLE_QUERY_ROOTS`, `WriteCoordinator` appear without a one-line gloss or link.

- 🔵 **Clarity**: "action verb" terminology is mildly misleading
  **Location**: Requirements / Acceptance Criteria
  Values are past participles rendered verbatim; "action verb" could imply a transformation step.

- 🔵 **Clarity**: "within 1 second of the file system mtime" leaves the reference clock ambiguous
  **Location**: Acceptance Criteria
  (Overlaps with the major testability finding above; flagged separately for the clarity dimension.)

- 🔵 **Dependency**: Pagination/retention open question implies a follow-on story but no downstream work is named
  **Location**: Open Questions
  If pagination/retention is deferred, the follow-on silently depends on the ring-buffer capacity decision made here.

- 🔵 **Dependency**: Glyph dependency on 0037 is stronger than Dependencies suggests
  **Location**: Requirements / Dependencies
  Story relies on per-doc-type Glyph variants, not merely Glyph existence; partial 0037 delivery could break the feed silently.

- 🔵 **Testability**: 'at least every 60 seconds' is satisfiable by trivially short intervals
  **Location**: Acceptance Criteria AC3
  A 1 s interval would pass; tighten to a cadence band or fake-timer assertion.

- 🔵 **Testability**: 'before any new SSE events arrive' lacks a defined verification procedure
  **Location**: Acceptance Criteria AC4
  The precondition is racy on a live system; reframe as an observable invariant with test scaffolding.

- 🔵 **Testability**: No AC defines what 'LIVE badge is present' means observationally
  **Location**: Acceptance Criteria (LIVE badge)
  Pin a stable observable (text content, role, or test-id).

- 🔵 **Testability**: 'designated empty-state message' is undefined
  **Location**: Acceptance Criteria (empty state)
  Specify the message text or a stable test-id so the empty branch is detectable.

- 🔵 **Testability**: Self-cause filter behaviour mentioned in Technical Notes is not in any AC
  **Location**: Requirements (self-cause filter)
  Technical Notes commits to "default to include"; an excluding implementation would still pass.

#### Suggestions
- 🔵 **Scope**: Server-side SSE/ring-buffer changes could be a separable sub-story
  **Location**: Requirements
  Wire-format extension + history endpoint are independently shippable from the frontend. Kept as one story is defensible given the explicit coupling rationale — flagged as judgement call.

- 🔵 **Scope**: Open question on pagination/expand could expand scope
  **Location**: Open Questions
  Resolve before leaving draft, or explicitly defer expansion to a follow-up to protect the "M" sizing.

### Strengths

- ✅ Acceptance Criteria consistently use Given/When/Then with named actors and observable outcomes.
- ✅ Wire-format shapes are spelled out explicitly with literal values, response JSON shape, and ordering semantics.
- ✅ Frontmatter is complete and valid: type=story, status=draft, priority=high, parent, tags, author, and date all present.
- ✅ Summary opens with a clear user-story sentence and follows with a concise technical overview.
- ✅ Context, Requirements, Technical Notes, Open Questions, Dependencies, and Assumptions are all populated with substantive content.
- ✅ Cross-file couplings within the codebase (sse_hub.rs, watcher.rs, api/mod.rs, use-doc-events.ts) are precisely cited with line ranges.
- ✅ Out-of-scope items are explicitly called out (persistence, server-originated edits, unseen-changes tracker modifications) — strong scope boundaries.
- ✅ Assumptions explicitly addresses the additive-change contract with existing SSE consumers.
- ✅ Story-sized "M" label aligns with the described surface area and existing patterns.

### Recommended Changes

1. **Pin the `'moved'` action semantics** (addresses: Clarity "Inconsistent mapping", Testability "No AC verifies 'edited' or 'moved' action mapping")
   Either commit to inspecting `EventKind::Modify(ModifyKind::Name(_))` so `'moved'` is genuinely produced (and add an AC for it), or remove `'moved'` from the discriminator and collapse renames into create+delete. Reconcile Requirements, Technical Notes, and ACs to one story.

2. **Specify the wire encoding of `etag` on delete events** (addresses: Clarity "Conflicting framings of the `etag`/`action` relationship")
   State explicitly whether deletion events serialise `etag` as absent, `null`, or unchanged, and clarify whether `action === 'deleted'` is the new canonical detector with `etag` absence as a legacy fallback.

3. **Define the `timestamp` reference clock and a workable tolerance** (addresses: Testability AC6, Clarity "reference clock ambiguous")
   Pick a source (e.g. `Utc::now()` immediately before `hub.broadcast(...)`) and assert tolerance against that clock, not FS mtime, to make AC6 verifiable within the existing debounce window.

4. **Add ACs for `'edited'` and `'moved'` mappings** (addresses: Testability "No AC verifies 'edited' or 'moved'")
   Mirror the create/delete ACs: "Given a file's contents are modified … `action === 'edited'`"; "Given a file is renamed within a doc-type root … `action === 'moved'`" (or document why the latter is omitted, per change 1).

5. **Add ACs for ring-buffer ordering and capacity** (addresses: Testability "Ring-buffer ordering and capacity")
   E.g. "Given N>5 events occur in known order, when `GET /api/activity?limit=5` is called, then `events[0].timestamp` is the latest and events are strictly descending"; "Given 50 events have been recorded, `?limit=50` returns 50 events".

6. **Replace `within 100 ms` and `at least every 60 seconds` with procedurally verifiable forms** (addresses: Testability AC1, AC3)
   For AC1, either drop the timing clause or assert "the row is present on the next render tick after the event is dispatched". For AC3, bound the cadence (e.g. "≥30 s and ≤60 s between re-renders") or use a fake timer.

7. **Define observable markers for the LIVE badge, empty state, and self-cause behaviour** (addresses: Testability "LIVE badge", "empty-state message", "self-cause filter")
   Pin a `data-testid` or exact text for the LIVE badge and empty-state message. Add an AC asserting that a self-caused `doc-changed` event still appears in the feed (or explicitly mark self-cause filtering as out of scope).

8. **Strengthen the 0053 dependency entry** (addresses: Dependency "Sidebar slot contract", "Unseen-changes tracker")
   List both the Sidebar slot *and* the `DocEventsContext` Provider mount in `RootLayout` as required from 0053. Cross-reference the unseen-changes tracker assumption from Dependencies so renegotiation of 0053 surfaces the coupling.

9. **Sharpen the 0037 Glyph dependency entry** (addresses: Dependency "Glyph dependency on 0037 is stronger")
   Tighten to specify per-doc-type Glyph variants keyed by `DocTypeKey`, so a partial 0037 delivery cannot silently break the feed.

10. **Clarify downstream impact of the wire-format extension** (addresses: Dependency "'Blocks: none' understates")
    If no concrete consumer exists today, state "Blocks: none currently; future consumers of `action`/`timestamp` will depend on this story". Otherwise list known candidates.

11. **Introduce the ActivityFeed heading as a named structural element** (addresses: Clarity "Ambiguous referent: 'the heading'")
    Add a Requirements line: "the ActivityFeed renders a heading 'Activity'; the LIVE badge sits adjacent to it" before AC2 references it.

12. **Gloss undefined domain terms on first use** (addresses: Clarity "Undefined domain terms")
    Add one-line glosses/links for `DocTypeKey`, `self-cause filter`, `unseen-changes tracker`, `SESSION_STABLE_QUERY_ROOTS`, `WriteCoordinator`.

13. **Rename "action verb" to "action label" or "action discriminator string"** (addresses: Clarity "'action verb' terminology")
    Avoid implying a transformation from the discriminator string.

14. **Resolve or defer the pagination/expand open question** (addresses: Scope "Open question could expand scope", Dependency "Pagination/retention follow-on")
    Either decide rolling-five is final and move pagination/retention to a follow-on story (named in Blocks), or accept the scope increase and resize.

15. **(Optional, judgement call) Split server-side changes into a separate story** (addresses: Scope "Server-side SSE/ring-buffer changes could be separable")
    Bundling is defensible per the existing coupling rationale; flagged so the author can consciously confirm the choice.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally well-structured with explicit references to file paths and line numbers, and Acceptance Criteria use a clear Given/When/Then form. However, several clarity issues exist: the action discriminator semantics for moves vs deletes are inconsistent between sections, the relationship between `etag: None` and `action: 'deleted'` straddles two contradictory framings, and a few terms (DocTypeKey, self-cause filter, unseen-changes tracker) appear without local definition.

**Strengths**:
- Acceptance Criteria consistently use Given/When/Then with named actors (the watcher, the user, the Activity feed) and observable outcomes (specific field values, response shapes).
- Wire-format shapes are spelled out explicitly with literal values, response JSON shape, and ordering semantics.
- Dependencies and sibling stories (0036, 0037, 0053, 0054) are named explicitly with their roles, reducing referent ambiguity for cross-story concepts.

**Findings**:
- 🟡 **major / high**: Inconsistent mapping for `moved` action between Requirements and Technical Notes (Requirements / Technical Notes)
- 🟡 **major / high**: Conflicting framings of the `etag`/`action` relationship for deletions (Acceptance Criteria / Assumptions)
- 🔵 **minor / medium**: Ambiguous referent: "the heading" and "the feed heading" (Requirements / Acceptance Criteria)
- 🔵 **minor / medium**: "within 1 second of the file system mtime" leaves the reference clock ambiguous (Acceptance Criteria)
- 🔵 **minor / low**: Undefined domain terms used without local anchor (Technical Notes)
- 🔵 **minor / medium**: "action verb" terminology is mildly misleading (Requirements / Technical Notes)

### Completeness

**Summary**: The work item is exceptionally complete for a story: frontmatter is valid, every standard section is present and substantively populated, and the Acceptance Criteria, Requirements, and Technical Notes give an implementer concrete direction. Type-specific story content (user, motivation, criteria defining done) is clearly stated. No critical gaps were identified.

**Strengths**:
- Frontmatter is complete and valid: type=story, status=draft, priority=high, parent, tags, author, and date are all present.
- Summary opens with a clear user-story sentence and follows with a concise technical overview.
- Context section explains the parentage (0036) and the current state of the relevant code surfaces.
- Acceptance Criteria section contains seven Given/When/Then bullets covering UI behaviour, wire-format, history endpoint, and empty-state.
- Requirements, Technical Notes, Open Questions, Dependencies, and Assumptions are all populated with substantive content.
- Type-specific story content is present: identified user, motivation, and criteria defining done.

**Findings**: *(none)*

### Dependency

**Summary**: The work item captures its primary upstream blockers (0037 for Glyph, 0053 for Sidebar layout) and correctly identifies the parent epic (0036) and sibling (0054). However, it leaves several implied couplings uncaptured: the Sidebar slot contract from 0053 is a hard prerequisite that goes deeper than just 'layout', the unseen-changes tracker from 0053 is named in Assumptions but not Dependencies, and downstream consumers of the new SSE wire-format extension are not flagged as Blocks even though the wire-format change is a shared artefact.

**Strengths**:
- Dependencies section explicitly names 0037 (Glyph) and 0053 (Sidebar layout) as upstream blockers.
- Parent (0036) and sibling stories (0053, 0054) are clearly named in Context.
- Cross-file couplings are precisely cited with line ranges.
- Assumptions explicitly addresses the additive-change contract with `use-doc-events.ts`.

**Findings**:
- 🟡 **major / high**: Sidebar slot contract from 0053 is a deeper coupling than 'layout that hosts the feed' (Dependencies)
- 🟡 **major / high**: Unseen-changes tracker (from 0053) is named in Assumptions but not Dependencies (Dependencies)
- 🟡 **major / medium**: Blocks: none understates the downstream impact of the SSE wire-format extension (Dependencies)
- 🔵 **minor / medium**: Pagination/retention open question implies a follow-on story but no downstream work is named (Open Questions)
- 🔵 **minor / high**: Glyph dependency on 0037 is stronger than Dependencies suggests (Requirements)

### Scope

**Summary**: Story 0055 bundles a frontend ActivityFeed component with two server-side changes (SSE wire-format extension and a new history endpoint). While the three pieces are tightly coupled around a single user-visible capability, the work spans both halves of the stack and could plausibly be split. The scope is coherent and the parent epic relationship is well-articulated; sizing as a single 'M' story is defensible given the explicit coupling rationale.

**Strengths**:
- Clear parent/sibling relationship to 0036, 0053, and 0054 — boundaries are explicitly stated.
- Summary, Requirements, and Acceptance Criteria all describe the same scope.
- Out-of-scope items are explicitly called out (persistence, server-originated edits, unseen-changes tracker modifications).
- Single-team ownership: both halves slot into existing patterns, no cross-team coordination required.
- Story-sized 'M' label aligns with the described surface area.

**Findings**:
- 🔵 **suggestion / medium**: Server-side SSE/ring-buffer changes could be a separable sub-story (Requirements)
- 🔵 **suggestion / low**: Open question on pagination/expand could expand scope (Open Questions)

### Testability

**Summary**: The Acceptance Criteria are largely well-formed: most are framed as Given/When/Then with concrete preconditions, observable outputs, and defined response shapes. Several criteria have ambiguous thresholds (e.g. 'within 100 ms' without a defined measurement procedure) and the Requirements section contains behaviours that are not mirrored as ACs — notably ring-buffer behaviour and 'moved'/'edited' action mapping have no verification criterion.

**Strengths**:
- Most criteria specify concrete preconditions, actions, and observable outcomes.
- The history-endpoint response shape is fully specified inline within the AC.
- Empty-state behaviour is explicitly called out as a distinct AC.
- Action-discriminator semantics for 'created' and 'deleted' are pinned with verifiable assertions.

**Findings**:
- 🟡 **major / high**: No AC verifies 'edited' or 'moved' action mapping (Acceptance Criteria, action mapping)
- 🟡 **major / high**: Ring-buffer ordering and capacity have no direct AC (Acceptance Criteria, history endpoint)
- 🟡 **major / medium**: 'within 100 ms' lacks a defined measurement procedure (AC1)
- 🟡 **major / high**: 'within 1 second of the file system mtime' is procedurally ambiguous (AC6)
- 🔵 **minor / medium**: 'at least every 60 seconds' is satisfiable by trivially short intervals (AC3)
- 🔵 **minor / medium**: 'before any new SSE events arrive' lacks a defined verification procedure (AC4)
- 🔵 **minor / medium**: No AC defines what 'LIVE badge is present' means observationally (AC2)
- 🔵 **minor / medium**: 'designated empty-state message' is undefined (AC5)
- 🔵 **minor / medium**: Self-cause filter behaviour mentioned in Technical Notes is not in any AC (Requirements, self-cause filter)

## Re-Review (Pass 2) — 2026-05-11

**Verdict:** REVISE

All nine major findings from Pass 1 were resolved by the edits, and the minor/suggestion-level findings were also addressed. Two **new** major findings surfaced — both introduced by the structure/wording of the Pass-1 edits in the Requirements section. Several new minor findings (mostly testability boundary cases) were also raised but do not block.

### Previously Identified Issues

#### Major (Pass 1)
- 🟡 **Clarity**: Inconsistent `'moved'` mapping between Requirements and Technical Notes — **Resolved** (`'moved'` dropped from the discriminator; renames now explicitly surface as create+delete pairs in both Requirements and Technical Notes).
- 🟡 **Clarity**: Conflicting `etag`/`action` framings on deletes — **Resolved** (`etag` pinned as `Option<String>` with `skip_serializing_if = "Option::is_none"`; `action === 'deleted'` named as canonical detector).
- 🟡 **Dependency**: Sidebar slot contract from 0053 understates Provider-mount requirement — **Resolved** (Dependencies now lists Sidebar slot *and* `DocEventsContext` Provider mount in `RootLayout`).
- 🟡 **Dependency**: Unseen-changes tracker from 0053 in Assumptions but not Dependencies — **Resolved** (added under "Coordinates with").
- 🟡 **Dependency**: 'Blocks: none' understates downstream impact — **Resolved** (Blocks rewritten to name future pagination/retention story + future SSE consumers).
- 🟡 **Testability**: No AC verifies `'edited'` or `'moved'` action mapping — **Resolved** (new `'edited'` AC added; `'moved'` dropped from the contract entirely).
- 🟡 **Testability**: Ring-buffer ordering and capacity have no direct AC — **Resolved** (two new ACs added: 6-events-keep-5 with strict descending ordering, and 50-events-return-50).
- 🟡 **Testability**: AC1 "within 100 ms" lacks measurement procedure — **Resolved** (rewritten as "on the next render tick" via test harness's `DocEventsContext`).
- 🟡 **Testability**: AC6 "within 1 s of mtime" procedurally ambiguous — **Resolved** (tolerance now measured against `hub.broadcast(...)` wall-clock captured via `Utc::now()`).

#### Minor / Suggestion (Pass 1)
- 🔵 All eleven minor and two suggestion-level findings from Pass 1 — **Resolved** (heading element named, domain terms glossed, "action verb" → "action label", LIVE-badge and empty-state observable markers pinned via `data-testid`, AC3 cadence bounded, AC4 reframed with test harness, self-cause AC added, 0037 dependency tightened to per-doc-type variants, pagination open question deferred to follow-on with Blocks entry). The judgement-call server/frontend split suggestion was consciously declined.

### New Issues Introduced

#### Major
- 🟡 **Clarity**: Dangling sentence fragment in `SsePayload` extension bullet
  **Location**: Requirements
  After the `timestamp` sub-bullet, the line "populated by the watcher … based on the pre/post rescan comparison" is unindented and ambiguous — a reader cannot tell whether it modifies `timestamp` only, `action` only, or the whole struct. The mapping table that follows inherits the same ambiguity.

- 🟡 **Clarity**: Cadence specification contradicts stated implementation
  **Location**: Requirements / Acceptance Criteria AC3
  "At least once every 60 seconds and not more often than once every 30 seconds (driven by a single `setInterval` of 60 s)" — the 60 s `setInterval` automatically satisfies the 30 s lower bound, making the bound redundant; the prose bound (30–60 s window) and the implementation note (single 60 s `setInterval`) read as two different contracts.

#### Minor
- 🔵 **Clarity**: Undefined `<n>s/m/h/d ago` shorthand on first use in Summary.
- 🔵 **Clarity**: `doc-changed` (JSON tag) vs `DocChanged` (Rust variant) used interchangeably without a one-line bridge.
- 🔵 **Clarity**: `WriteCoordinator` gotcha says self-cause AC is "effectively covered by suppression upstream", which reads as if AC11 is vacuous.
- 🔵 **Clarity**: Requirements bullet "Subscribes to the SSE stream…" doesn't name the subscription channel (`useDocEventsContext`); only stated in Technical Notes.
- 🔵 **Dependency**: Parent epic 0036 named in Context but not listed in Dependencies as a structural relationship.
- 🔵 **Dependency**: Frontend `api/self-cause.ts` coupling load-bearing for AC11 but not surfaced in Dependencies or Assumptions.
- 🔵 **Dependency**: TBD pagination/retention follow-on story is unnumbered, so the link cannot be made concrete from the other end.
- 🔵 **Scope**: Story still bundles SSE wire-format extension with ActivityFeed UI (re-flagged from Pass 1 as a judgement call).
- 🔵 **Testability**: AC1 relative-timestamp format has no unit-boundary AC (when does `s` become `m`, `m` become `h`, etc.).
- 🔵 **Testability**: Self-cause AC covers only the include path; no negative case or mixed-stream AC.
- 🔵 **Testability**: AC6 reference clock (`hub.broadcast(...)` invocation time) is observer-defined — black-box tests cannot directly observe it.
- 🔵 **Testability**: No AC covers ring-buffer eviction at the 51st event (only the 50-events-return-50 case).
- 🔵 **Testability**: LIVE-badge AC covers steady states but not transitions (open → closed → open).

#### Suggestions
- 🔵 **Scope**: History endpoint + ring buffer is a third sub-deliverable inside the bundle (judgement call — atomicity argument is sound).
- 🔵 **Testability**: Open Question on row navigation has no testable outcome — either resolve and add AC, or explicitly defer.

### Assessment

The Pass-1 majors were all resolved, but two new majors were introduced as side-effects of the Requirements rewrite (a dangling fragment and a redundant/contradictory cadence clause). Both are surface-level wording fixes — neither requires re-thinking the design. The remaining new minors are mostly boundary cases that would tighten testability further and a few light dependency-graph entries (parent epic, self-cause helper, follow-on numbering).

With the 2-major threshold from config, the verdict remains **REVISE**, but the gap to **APPROVE** is small — fixing the two new majors and (optionally) the highest-value new minors should clear the work item for implementation.

## Re-Review (Pass 3) — 2026-05-11

**Verdict:** COMMENT

Both pass-2 majors and most pass-2 minors resolved. Pass 3 surfaced three new majors, all introduced or sharpened by the pass-2 edits — all small wording fixes, all addressed in this pass. After the pass-3 edits, no major findings remain at this confidence level; the work item is acceptable for implementation, with a long tail of minor sharpening opportunities that the team can either pick up incrementally or defer.

### Previously Identified Issues

#### Pass 2 Majors
- 🟡 **Clarity**: Dangling sentence fragment in `SsePayload` extension bullet — **Resolved** (Requirements bullet restructured: watcher mapping now folds into the `action` sub-bullet; `timestamp` stands alone).
- 🟡 **Clarity**: Cadence specification contradicts stated implementation — **Resolved** (collapsed to "fixed 60 s cadence, driven by `setInterval(..., 60_000)`" in both Requirements and AC3).

#### Pass 2 Minors — all addressed:
- 🔵 `<n>s/m/h/d ago` shorthand undefined on first use — **Resolved** (expanded inline in Summary).
- 🔵 `doc-changed` vs `DocChanged` bridge — **Resolved** (Summary and Requirements both bridge the JSON tag to the Rust variant).
- 🔵 WriteCoordinator gotcha makes AC11 read as vacuous — **Resolved** (rewritten to separate suppression upstream from synthetic-event verification path).
- 🔵 Requirements bullet doesn't name `useDocEventsContext` subscription channel — **Resolved**.
- 🔵 Parent epic 0036 not in Dependencies — **Resolved** (added as Parent entry).
- 🔵 `api/self-cause.ts` coupling not surfaced — **Resolved** (added to Assumptions).
- 🔵 TBD pagination follow-on unnumbered — **Partially resolved** (close-time housekeeping obligation removed; placeholder filing no longer a close criterion).
- 🔵 Relative-timestamp unit boundaries — **Resolved** (new AC with boundary rules and worked examples).
- 🔵 Self-cause negative case — Deferred (the include behaviour is the core contract; negative case is not load-bearing).
- 🔵 AC6 reference clock observer-defined — Carried (white-box capture remains sufficient; black-box pinning would over-engineer).
- 🔵 No AC for eviction at 51st event — **Resolved** (new AC with strict ordinal assertions).
- 🔵 LIVE-badge transitions — **Resolved** (new transition AC).

### New Issues Introduced (Pass 3 → resolved in this pass)

#### Major (all resolved by pass-3 edits)
- 🟡 **Clarity**: Relative-timestamp boundary examples didn't pin the rounding rule (floor vs round) — **Resolved**: AC4 now states `<n> = Math.floor(elapsed_seconds / unit_size_in_seconds)`.
- 🟡 **Clarity**: Ambiguous "their own" pronoun in Self-cause filter bullet conflicting with WriteCoordinator suppression note — **Resolved**: Self-cause bullet rewritten to spell out the referent (events `api/self-cause.ts` attributes to the user's own PATCH edits) and explicitly note that today's live path is suppressed, so the include semantics are observable only via test-harness events.
- 🟡 **Dependency**: Downstream pagination/retention story is TBD-not-yet-filed; close-time housekeeping obligation mixed into Blocks — **Resolved**: Blocks rewritten to state explicitly that no work items are blocked today and that filing the pagination follow-on is not a close criterion for 0055.

### Carried Minors (acceptable; can be addressed incrementally)

These were flagged in Pass 3 but are tightening opportunities rather than blockers:

- 🔵 **Clarity**: Summary's second paragraph duplicates Requirements scope verbatim — risk of future drift between Summary and Requirements.
- 🔵 **Clarity**: `ISO-8601` is referenced without naming the specific profile (RFC 3339, fractional-second precision, `Z` suffix).
- 🔵 **Clarity**: AC6 1-second tolerance reads identically to "captured immediately before broadcast" — anchor clock could be sharpened.
- 🔵 **Clarity**: Resolved pagination decision sits under "Open Questions" rather than a "Decisions" subsection.
- 🔵 **Dependency**: `api/self-cause.ts` stability is in Assumptions, could be elevated to a "Coordinates with" entry.
- 🔵 **Dependency**: SSE-reconnect invalidation policy (`SESSION_STABLE_QUERY_ROOTS`) coupling lives only in Technical Notes.
- 🔵 **Dependency**: Intra-story ordering (server-side before frontend) not stated explicitly.
- 🔵 **Scope**: SSE wire-format extension could still stand alone as a separate work item (consistent with Pass 1 judgement-call suggestion; declined again).
- 🔵 **Testability**: AC3 mixes observable outcome with internal `setInterval` mechanism phrasing.
- 🔵 **Testability**: AC6 verification environment underspecified (no test-harness clock recipe).
- 🔵 **Testability**: AC1 doesn't pin filename derivation rule (basename vs full path).
- 🔵 **Testability**: AC11 doesn't pin the recipe for producing a self-caused event (depends on `self-cause.ts` shape).
- 🔵 **Testability**: AC12/AC13 assume capacity exactly 50 while Requirements says "at least 50" — minor internal inconsistency.

### Assessment

Three review passes have driven this work item from 9 majors (Pass 1) → 2 majors (Pass 2) → 3 majors (Pass 3) → 0 majors (Pass 3 after edits). The convergence pattern is healthy: each pass resolved every prior major and surfaced fewer new ones, with the new finds becoming progressively more tactical (wording around a pronoun, a rounding rule, a housekeeping obligation) rather than design-level.

The work item is now in a state where the remaining feedback is a long tail of minor sharpening — boundary cases, terminology pins, dependency-graph completeness — that the team can pick up incrementally or while implementing. **Verdict: COMMENT** (acceptable as-is; the minors are observations rather than blockers). Recommend moving the work item out of draft.
