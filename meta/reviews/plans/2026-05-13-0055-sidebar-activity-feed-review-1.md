---
date: "2026-05-14T09:30:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-13-0055-sidebar-activity-feed.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, performance, compatibility]
review_pass: 2
status: complete
---

## Plan Review: 0055 — Sidebar Activity Feed and SSE Action Discriminator

**Verdict:** REVISE

The plan is structurally sound and TDD-disciplined, with strong attention to ordering (ring-buffer push before broadcast, subscribers before self-cause drop) and a good AC→test mapping for most acceptance criteria. However, two material correctness/compatibility gaps and several test-coverage weaknesses warrant revision before implementation: (1) a non-test Rust production call site (`api/docs.rs:242`) and roughly 23 TypeScript test fixtures will fail to compile after the wire-format change but are not enumerated in the plan, (2) the refetch-on-reconnect path will produce duplicate rows because retained `live` state overlaps with the refetched initial history, and (3) AC11 (self-cause inclusion) is verified at the hook layer but the component-level test is logically vacuous and AC3's "exactly one re-render" claim is not actually asserted. Architectural concerns around watcher dual-sink duplication and the `subscribe`-vs-`onEvent` fan-out divergence are also worth addressing now while the API surface is still malleable.

### Cross-Cutting Themes

- **Wire-format migration audit is incomplete** (flagged by: compatibility, correctness) — The plan enumerates Rust test fixtures and acknowledges the type-level change, but misses (a) the production `api/docs.rs:242` `SsePayload::DocChanged` constructor in the PATCH handler, and (b) the ~23 existing TypeScript test fixtures (in `use-doc-events.test.ts`, `LibraryDocView.smoke.test.tsx`, `use-unseen-doc-types.test.ts`) that build `{ type: 'doc-changed', ... }` literals. After promoting `action`/`timestamp` to required fields, neither the Rust crate nor the frontend typecheck will compile cleanly. The "additive" framing in Migration Notes is true at the JSON level but not at the type level.

- **AC11 self-cause verification is shallow at the component layer** (flagged by: test-coverage, correctness) — The Phase 6 component test invokes the captured `subscribe` listener directly, bypassing `self-cause.ts` entirely. The test proves "any event reaches the row," which is logically equivalent to AC1. AC11's actual verification rests entirely on Phase 5 hook test #4; if anyone deletes that test as redundant, AC11 silently loses coverage.

- **Watcher coupling to two sinks with duplicated payload construction** (flagged by: architecture, performance) — The watcher now constructs both `SsePayload::DocChanged` and a parallel `ActivityEvent` per emit site, with the plan acknowledging "cloning the payload twice is fine here." This duplicates schema awareness across both sinks; any future field addition must be threaded into two places in lockstep with nothing structural to enforce it.

- **React key includes array index on a prepend list** (flagged by: code-quality, correctness) — `key={\`${r.timestamp}-${r.path}-${i}\`}` defeats DOM reconciliation: every prepend shifts every existing row's index, causing all five rows to remount on every SSE event. Functionally correct but architecturally backwards for a prepend list.

### Findings

#### Critical

(none)

#### Major

- 🟡 **Compatibility**: Plan misses production call site `api/docs.rs:242` constructing SsePayload::DocChanged
  **Location**: Phase 1, Section 1 + Phase 3, Section 3 (production fixture audit)
  Plan enumerates 5 watcher tests + sse_hub test + `api/events.rs` test + new `api/activity.rs` test, but `server/src/api/docs.rs:242` is a non-test PATCH handler that also constructs the variant and will fail to compile until `action` (likely `Edited`) and `timestamp: Utc::now()` are added. Phase 1's "atomic" framing is wrong without this.

- 🟡 **Correctness / Compatibility**: Promoting `action`/`timestamp` to required TS fields breaks ~23 existing test fixtures
  **Location**: Phase 4, Section 1 (SseDocChangedEvent extension) + Migration Notes
  ~23 sites across `use-doc-events.test.ts` (19), `LibraryDocView.smoke.test.tsx` (2), `use-unseen-doc-types.test.ts` (1), and as-cast helpers construct `{ type: 'doc-changed', ... }` literals without the new fields. Phase 4's typecheck gate will fail immediately. Plan must either enumerate every fixture migration site or keep the fields optional in TS with defensive guards at the consumer.

- 🟡 **Correctness**: Duplicate rows on SSE reconnect — refetched `initial` overlaps with retained `live` state
  **Location**: Phase 6, ActivityFeed component (`rows = [...live, ...(initial ?? [])].slice(0, LIMIT)`)
  Plan deliberately excludes `'activity'` from `SESSION_STABLE_QUERY_ROOTS` so the query refetches on reconnect, but `live` is never cleared. The refetched first-5 rows from the server's ring buffer will overlap with events already accumulated in `live`, producing duplicate visible rows until ≥ LIMIT new events arrive. Needs either a dedup pass on `rows` or clearing `live` on reconnect.

- 🟡 **Test Coverage / Correctness**: AC11 component test bypasses self-cause semantics
  **Location**: Phase 6, ActivityFeed.test.tsx — AC11 case
  The component test dispatches an event via the captured listener; the plan's own note acknowledges "the component never consults the registry, so any event reaches the row." This test is logically equivalent to AC1 and adds no AC11-specific coverage. AC11 is verified only at Phase 5 hook test #4 — fragile if that test is later removed.

- 🟡 **Test Coverage**: AC3 "exactly one re-render per tick" is not actually asserted
  **Location**: Phase 6 — AC3 ticker test outline
  The outlined test advances the clock and re-reads text content, proving the text updated but not that it updated *once*. With `setTick(t => t + 1)` plus unrelated re-renders, the text can update twice and the test still passes. Need a render-count probe or vi.fn-wrapped `formatRelative` counter.

- 🟡 **Test Coverage**: Delete-branch ring-buffer push not directly tested
  **Location**: Phase 1 + Phase 3 — watcher integration tests
  Phase 3 extends the create-test to assert `activity_feed.recent(1)` has a `Created` entry, but `file_deletion_produces_doc_changed_without_etag` is not similarly extended. The deletion branch is a distinct push site; a regression there has no test net.

- 🟡 **Test Coverage**: AC8 — timing dependency on `pre.is_some()` not explicit
  **Location**: Phase 1, Section 5 — file_change_produces_doc_changed_event extension
  AC8 hinges on `pre` being populated at modify-time. If the existing test setup writes-and-modifies in close succession before the indexer settles, the watcher's `pre` may be `None`, producing `Created` instead of `Edited`. The plan asserts `Edited` without explicitly pinning the create-and-index-then-modify ordering.

- 🟡 **Test Coverage**: Concurrent-push test claimed in narrative but absent from test list
  **Location**: Phase 2 — "cover capacity, ordering, eviction, and concurrent pushes" narrative
  None of the five listed unit tests exercise concurrent pushes. Either remove the claim or add a multi-task/multi-thread test that fires N pushes and asserts all are accounted for.

- 🟡 **Architecture**: Module mixes domain data structure with HTTP transport
  **Location**: Phase 2 + Phase 3 (`api/activity.rs` co-locates `ActivityRingBuffer` with handler)
  Watcher (a domain producer) imports from `crate::api::activity`, inverting the conventional transport-depends-on-domain direction. The naming-collision rationale (avoiding `crate::activity`) pushed it under `api/`, but the constraint is lexical, not architectural. Consider moving `ActivityRingBuffer` to `server/src/activity_feed.rs` (crate root) and keeping `api/activity.rs` for the handler alone.

- 🟡 **Architecture**: Watcher writes to two sinks with hand-duplicated payload construction
  **Location**: Phase 3, Section 3
  The watcher constructs `SsePayload::DocChanged` and a parallel `ActivityEvent` whose fields are a strict subset, with manual mirroring per emit site. A single fan-out emit function (e.g. an `EventBus::emit` that derives the `ActivityEvent` from the payload via a `From` impl) would consolidate the coupling.

- 🟡 **Architecture**: `subscribe()` is added alongside `onEvent` rather than unifying the fan-out surface
  **Location**: Phase 5 — `DocEventsHandle.subscribe` extension
  Two parallel fan-out paths with different visibility rules (subscribers fire pre-self-cause-drop; `onEvent` fires post-drop) and different cardinality (multi vs single) coexist. Future maintainers must remember which to use. Consider `subscribe(listener, { includeSelfCaused? })` as a single unified API with `useUnseenDocTypes` migrated to subscribe with `includeSelfCaused: false`.

- 🟡 **Code Quality**: Sibling `formatMtime` + `formatRelative` helpers risk silent drift
  **Location**: Phase 4, Section 4
  Both share four identical s/m/h/d branches and differ only at the `≥ 7d` boundary; their negative-elapsed handling already differs (`'just now'` vs `'0s ago'`). Extract the shared s/m/h/d ladder into a private helper; let the two functions differ only at the high-bound fallback.

- 🟡 **Code Quality / Correctness**: React `key` includes array index on a prepend list
  **Location**: Phase 6, Section 1 — `key={\`${r.timestamp}-${r.path}-${i}\`}`
  Every prepend shifts every existing row's index, causing React to remount all 5 rows on each new event. Functionally correct but defeats reconciliation and will break any future row-level transient state (animation, focus). Drop `${i}`; if collision concerns remain, key on `${r.timestamp}-${r.path}-${r.action}` or a stable client-side id assigned at receive time.

- 🟡 **Code Quality**: `console.warn` + continue swallows listener exceptions silently
  **Location**: Phase 5, Section 2 — subscriber fan-out in onmessage
  A buggy subscriber leaves only a console message that may never surface. The fault-isolation policy is correct (one bad listener must not break others), but visibility should escalate — `console.error` at minimum, and ideally a single observability seam (telemetry/reportError hook).

#### Minor

- 🔵 **Architecture**: Inlining `payload_for_entry` removes the only DocChanged/DocInvalid abstraction
  **Location**: Phase 1, Section 4
  The malformed-frontmatter rule moves into a debounce-coroutine arm, mixed with action-mapping and the ring-buffer push. Consider evolving the helper's signature instead — `payload_for_entry(entry, rel, pre, now) -> SsePayload` — preserving the pure-function seam.

- 🔵 **Architecture**: ActionKind value-rename strategy duplicates the discriminator vocabulary in TS
  **Location**: Phase 1, Section 2 + Phase 4, Section 1
  Rust enum and TS union must be kept in sync manually; variant additions can ship one-sided. Consider a generated source-of-truth or at least a round-trip integration test.

- 🔵 **Architecture**: Double-wrapping `<section>` in Sidebar creates ambiguous structural responsibility
  **Location**: Phase 7 — Sidebar mount
  Two sections sharing `aria-labelledby="activity-heading"` is invalid ARIA. Decide now: `ActivityFeed` is either presentational (Sidebar owns the section wrapper) or owns its own section (Sidebar passes layout via className).

- 🔵 **Architecture**: AppState ends up with `activity` and `activity_feed` fields with unrelated meanings
  **Location**: Phase 3 — AppState field addition
  Cheap rename opportunity to disambiguate the HTTP-activity tracker from the file-change ring buffer at the field level too (e.g. `http_activity` and `activity_feed`, or `idle_tracker` and `recent_events`).

- 🔵 **Architecture / Performance**: `std::sync::Mutex` inside async watcher coroutine
  **Location**: Phase 2, Section 1
  Critical sections are currently O(1) and never `.await`, so the choice is sound, but the read path's `iter().take(limit).cloned()` holds the lock while cloning up to 50 small structs. Acceptable at expected load; document the invariant explicitly in a module doc-comment.

- 🔵 **Code Quality**: `expect("activity ring poisoned")` panics on Mutex poisoning
  **Location**: Phase 2, Section 1
  Activity feed is non-essential; a poison panic would take down the watcher or HTTP handler. Consider `unwrap_or_else(|p| p.into_inner())` or document the panic as intentional.

- 🔵 **Code Quality**: `as any` cast in mockReturnValue bypasses the new DocEventsHandle contract
  **Location**: Phase 6, Section 2 — mountWith mock harness
  Defeats the type safety the new API was meant to deliver. Use a typed factory `function mockHandle(overrides: Partial<DocEventsHandle>): DocEventsHandle` instead.

- 🔵 **Code Quality**: Module-level mutable `capturedListener` is hidden cross-test state
  **Location**: Phase 6, Section 2 — `let capturedListener` at module scope
  Reset in `beforeEach` or return the listener from `mountWith` to localise it.

- 🔵 **Code Quality**: Silent omission when `isGlyphDocTypeKey` narrowing fails
  **Location**: Phase 6, Section 1 — Glyph render guard
  Future doc types added without Glyph coverage render icon-less with no warning. Either widen Glyph to accept the full `DocTypeKey` with a fallback, or add a dev-only assert when narrowing fails.

- 🔵 **Code Quality**: "Cloning the payload twice is fine here" risks becoming a precedent
  **Location**: Phase 3, Section 3
  The example code only clones `path` (other fields are `Copy`), so the comment is misleading. Either drop the note or replace with a specific justification.

- 🔵 **Test Coverage**: `formatRelative` boundary values not tested
  **Location**: Phase 4 — format.test.ts
  Add explicit cases for `diffSec == 0`, `60`, `3600`, `86400` to catch `<` vs `<=` mutations.

- 🔵 **Test Coverage**: Loading-state behaviour not tested
  **Location**: Phase 6 — ActivityFeed empty/loading distinction
  `isEmptyHistory` depends on `isSuccess`; the in-flight state is never tested. Add a never-resolving mock test.

- 🔵 **Test Coverage**: AC6 assertion `before <= timestamp <= after` is stricter than AC6's 1s wording
  **Location**: Phase 1, Section 5
  Either accept the tighter bound with a comment, or add a second assertion pinning the AC's 1s tolerance verbatim.

- 🔵 **Test Coverage**: `?limit=0` and `?limit=foo` handler behaviour not tested
  **Location**: Phase 3 — handler tests
  Add tests for `?limit=0` (status 200, empty events) and invalid `?limit=foo` (Axum's Query parse failure; assert and document the 400).

- 🔵 **Test Coverage**: "Restart → empty state" is manual-only despite being a documented contract
  **Location**: Phase 3 — Manual Verification
  Promote the empty-state handler test (already in the plan) to explicitly serve as the "fresh state → empty" assertion with a contract-pinning comment.

- 🔵 **Test Coverage**: No test for create→edit→delete chain on a single path
  **Location**: Phase 1 — watcher tests
  Add an integration-style test that performs create-then-edit-then-delete with debounce waits, captures three SSE events, and asserts the ring buffer contains three entries `[Deleted, Edited, Created]`.

- 🔵 **Correctness**: `now` captured before async `indexer.get` lookup; AC7's 1-second tolerance bounds drift
  **Location**: Phase 1, Section 4
  Either accept the immaterial drift and document, or capture `now` separately inside each match arm immediately before `hub.broadcast(...)`.

- 🔵 **Correctness**: Valid→malformed transitions emit `DocInvalid` with no `action='edited'` — feed silently drops them
  **Location**: Phase 1, Section 4 — malformed-frontmatter branch
  AC8 wording does not carve out malformed-edit. Document explicitly that DocInvalid events are not surfaced in the Activity feed, or consider adding `action`/`timestamp` to DocInvalid too.

- 🔵 **Correctness**: `Date.parse` truncates chrono's RFC-3339 nanosecond precision (acceptable, worth noting)
  **Location**: Phase 6, Section 1
  Negligible in practice; the negative-elapsed clamp in `formatRelative` already handles small clock skew. Worth a brief inline comment.

- 🔵 **Performance**: Subscriber fan-out runs even for self-caused events the dispatcher will discard
  **Location**: Phase 5, Section 2 — onmessage subscriber loop ordering
  Intentional per AC11, but scales with subscriber count. Mention in the plan that subscriber callbacks should be cheap and non-blocking.

- 🔵 **Compatibility**: No upper bound on `?limit` exposes the contract to malformed values
  **Location**: Phase 3, Section 1 — `LimitParam`
  Either cap `limit.min(CAPACITY)` or document the contract for negative / oversized values.

- 🔵 **Compatibility**: New-TS-frontend-against-old-server produces `undefined` action labels and `NaNd ago`
  **Location**: Migration Notes
  Dev-only papercut during partial rebuilds. Defensively coerce missing `timestamp`/`action` in ActivityFeed (~2 LOC).

#### Suggestions

- 🔵 **Code Quality**: Lift the action-mapping into a tiny named helper `action_for(pre_present, post_present) -> ActionKind`
  **Location**: Phase 1, Section 4
  Opens a unit-test seam for the pure mapping and shrinks the match arm.

- 🔵 **Performance**: `Date.parse(r.timestamp)` runs per row per render (trivial at LIMIT=5)
  **Location**: Phase 6, Section 1
  Optional precomputation if LIMIT ever grows substantially.

- 🔵 **Performance**: `setLive` allocates a new array per SSE event (trivial at LIMIT=5)
  **Location**: Phase 6, Section 1
  No change; revisit if LIMIT grows.

### Strengths

- ✅ TDD-first phasing — each phase opens with failing tests pinning the new contract; AC→test mapping is explicit and largely thorough.
- ✅ Correct decision to push the ring buffer **synchronously before** `hub.broadcast(...)` (avoids the lossy-broadcast trap that subscribing to the channel would create).
- ✅ Correct decision to NOT add `'activity'` to `SESSION_STABLE_QUERY_ROOTS` — refetch on reconnect rebuilds the rolling window deterministically.
- ✅ Correct decision to NOT add a `'moved'` action variant — preserves the existing "pre/post comparison only" invariant in the watcher.
- ✅ Sensible module placement to avoid the existing `crate::activity` namespace collision, with rationale documented inline.
- ✅ JSON wire-format change is genuinely additive for consumers that ignore unknown fields; existing dispatcher branches only on `type`.
- ✅ Per-enum `#[serde(rename_all = "lowercase")]` on `ActionKind` correctly identified as required (container's kebab-case rename does not apply to field values).
- ✅ Chrono feature flags are minimal and correct (`std`, `clock`, `serde` — nothing superfluous).
- ✅ Multi-subscriber `subscribe(listener) → unsubscribe` API is the right shape and composes with the existing `onEvent` slot.
- ✅ Capacity is encoded as a named constant `CAPACITY: usize = 50` rather than scattered magic numbers.
- ✅ Server-only Phases 1–3 and frontend-only Phases 4–7 can ship as two independent PRs.
- ✅ Plan explicitly recognises performance considerations inline, distinguishing hot from cold paths.

### Recommended Changes

The major findings cluster into a small number of concrete edits to the plan. In suggested order of impact:

1. **Extend the wire-format migration audit** (addresses: missed `api/docs.rs:242` production site; ~23 TS fixture sites)
   - Add `server/src/api/docs.rs:242` to Phase 1's Changes Required with an explicit `action` choice for PATCH (`Edited` likely) and `timestamp: Utc::now()`.
   - In Phase 4, decide between (a) enumerating every TS fixture site with a sentinel `action` and `timestamp`, or (b) keeping the TS fields optional with defensive coercion in ActivityFeed. Document the choice and its cross-server-version-drift implications.
   - Add a "grep checklist" to Phase 1's Success Criteria: `grep -rn 'SsePayload::DocChanged' server/src/` and `grep -rn "doc-changed" frontend/src/` returning zero unmigrated literal sites.

2. **Address duplicate rows on SSE reconnect** (addresses: live + refetched initial overlap)
   - In Phase 6, either dedupe `rows` by `(timestamp, path, action)` before slicing, or clear `live` in a `connectionState` transition effect.
   - Add an explicit test: mock a reconnect sequence (mount with events in both `live` and the refetched initial that overlap) and assert no duplicate rows.

3. **Strengthen AC3 and AC11 test verification** (addresses: ticker re-render count not asserted; component AC11 test is logically vacuous)
   - AC3: wrap rendered relative-time in a `vi.fn()` probe or render-count counter; assert exactly +1 per `advanceTimersByTime(60_000)`.
   - AC11: either cross-reference Phase 5 hook test #4 as the canonical AC11 evidence and remove the Phase 6 case, or strengthen the component test to exercise an integration path with a real `makeUseDocEvents` + populated self-cause registry.

4. **Add the missing test coverage cases** (addresses: delete-branch ring-buffer push, AC8 timing, concurrent pushes)
   - Extend `file_deletion_produces_doc_changed_without_etag` to assert the ring buffer contains a `Deleted` entry after the event.
   - In `file_change_produces_doc_changed_event` make the create-and-index-then-modify ordering explicit (drain the first `Created` event from `rx` before the second write).
   - Either add a concurrent-push unit test to Phase 2 or remove the "concurrent pushes" claim from the narrative.

5. **Fix the React key on the prepend list** (addresses: every-row-remount on each event)
   - Change `key={\`${r.timestamp}-${r.path}-${i}\`}` to `key={\`${r.timestamp}-${r.path}-${r.action}\`}` or assign a stable client-side id at receive time.

6. **Consolidate the watcher's dual-sink emit point** (addresses: hand-duplicated payload construction)
   - Define `impl From<&SsePayload> for Option<ActivityEvent>` (None for DocInvalid) or add a single `emit(payload, &activity_feed)` helper. Watcher then constructs one literal per branch and the projection lives in one place.

7. **Decide on `subscribe` vs `onEvent` unification** (addresses: parallel fan-out paths with divergent semantics)
   - Either ship `subscribe(listener, { includeSelfCaused?: boolean })` as a single fan-out API and migrate `useUnseenDocTypes`, or explicitly document the divergence on the `DocEventsHandle` interface so the contract is visible at the type level.

8. **Resolve the formatRelative duplication risk** (addresses: drift between sibling helpers)
   - Extract the shared s/m/h/d ladder into a private helper called by both `formatMtime` and `formatRelative`. Unifies negative-elapsed handling too.

9. **Escalate listener-error visibility** (addresses: silent `console.warn` swallow)
   - Promote `console.warn` to `console.error` in the subscriber catch, with a clear "[doc-events] subscriber threw" prefix.

10. **Tighten the minor concerns** (addresses: minor/suggestion findings collectively)
    - Add boundary cases to `formatRelative` tests (0, 60, 3600, 86400).
    - Add a loading-state test (never-resolving fetch mock; no rows, no empty-state).
    - Cap `?limit` at `CAPACITY` or document the contract.
    - Decide the Sidebar `<section>` wrapping (presentational ActivityFeed vs self-sectioning).
    - Consider renaming the existing `AppState.activity` field to disambiguate from `activity_feed`.
    - Replace the `as any` mock cast with a typed `mockHandle()` factory.
    - Reset or localise `capturedListener` to avoid cross-test state.
    - Document the activity-feed silent-drop of DocInvalid (valid→malformed transitions) in "What We're NOT Doing".

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound overall: respects the established Arc/Mutex shared-state pattern, places the new module under `api::` to avoid the `crate::activity` namespace collision, and correctly identifies that the ring buffer must be pushed synchronously (not as a broadcast subscriber). However, the chosen module placement co-locates the ring-buffer data structure with its HTTP handler, mixing transport and domain concerns; the watcher now writes to two parallel sinks with a duplicated payload construction that is fragile under future schema evolution; and the new `subscribe` API is added beside (not unifying) the existing `onEvent` slot.

**Strengths**:
- Avoids subscribing the ring buffer to the lossy broadcast channel; push-before-broadcast is the right call and is explicitly documented.
- Namespaces the new module as `crate::api::activity` to avoid the `crate::activity` HTTP-tracker collision.
- Wire-format change is genuinely additive at the JSON layer.
- Correctly omits `'activity'` from `SESSION_STABLE_QUERY_ROOTS`.
- Avoids introducing a `'moved'` action variant, preserving the watcher's "pre/post comparison only" invariant.
- TDD phasing is well-architected: server-only Phases 1–3 ship independently of frontend Phases 4–7.

**Findings**:
- 🟡 Major: Module mixes domain data structure with HTTP transport — Phase 2/3 placement under `api/activity.rs`.
- 🟡 Major: Watcher writes to two sinks with hand-duplicated payload construction — Phase 3, Section 3.
- 🟡 Major: `subscribe()` added alongside `onEvent` rather than unifying fan-out — Phase 5.
- 🔵 Minor: Inlining `payload_for_entry` removes the only DocChanged/DocInvalid abstraction — Phase 1, Section 4.
- 🔵 Minor: ActionKind value-rename duplicates discriminator vocabulary in TS — Phase 1/4.
- 🔵 Minor: Double-wrapping `<section>` creates ambiguous structural responsibility — Phase 7.
- 🔵 Minor: AppState has two fields named with `activity*` prefix — Phase 3.
- 🔵 Minor: `std::sync::Mutex` in async-rich call path — Phase 2.

### Code Quality

**Summary**: Well-structured, TDD-disciplined, and largely follows established codebase conventions. Several code-quality decisions warrant attention: panic-on-poison vs graceful handling on the ring-buffer Mutex, a swallow-and-continue `console.warn` in the subscriber fan-out loop, sibling `formatRelative`/`formatMtime` helpers that risk drifting apart, an `as any` test cast that bypasses the new handle type, an index-in-key React reconciliation hazard, and silent narrowing failures for Glyph rendering.

**Strengths**:
- Strong TDD framing with explicit test-changes-alongside-implementation-changes.
- Sensible module placement decision with rationale documented inline.
- Correct decision to inline `payload_for_entry`.
- `Utc::now()` captured once and shared across branches plus the ring-buffer push.
- Multi-subscriber `subscribe(listener) → unsubscribe` API is the right shape.
- Synchronous ring-buffer push avoids the lossy-broadcast trap.
- Pragmatic acknowledgement of broadcast as the hot path.
- Capacity encoded as a named constant.

**Findings**:
- 🟡 Major: Sibling `formatMtime` + `formatRelative` risk silent drift — Phase 4, Section 4.
- 🟡 Major: React `key` includes array index, defeats reconciliation — Phase 6, Section 1.
- 🟡 Major: `console.warn` swallows listener exceptions silently — Phase 5, Section 2.
- 🔵 Minor: `expect("activity ring poisoned")` panics — Phase 2, Section 1.
- 🔵 Minor: `as any` cast in mockReturnValue — Phase 6, Section 2.
- 🔵 Minor: Module-level `capturedListener` — Phase 6, Section 2.
- 🔵 Minor: Silent omission on Glyph narrowing failure — Phase 6, Section 1.
- 🔵 Minor: "Cloning twice is fine" note risks becoming precedent — Phase 3, Section 3.
- 🔵 Suggestion: Lift action-mapping into a tiny helper — Phase 1, Section 4.

### Test Coverage

**Summary**: Broadly thorough — every server AC has a dedicated test, the ring buffer has good unit-level coverage, and the format/fetch/hook/component layers all have explicit test sketches. However, several specific assertions are weaker than the AC text requires: AC3 is not actually asserted as "exactly one re-render", AC11 is not meaningfully verified at the component layer because the test only proves any event reaches the row, and AC8 has no dedicated edit-on-disk integration test (piggybacks an existing test). A handful of edge cases (formatRelative boundaries, ring-buffer concurrent push despite being mentioned, delete-branch ring-buffer push, loading-state distinction) are also unaddressed.

**Strengths**:
- TDD-first sequencing with explicit AC→test mapping per phase.
- Wire-format contract pinned with concrete JSON-shape assertions.
- Ring buffer has dedicated unit tests covering AC10, AC12, AC13.
- Handler test pattern mirrors `api/events.rs`.
- Hook-layer tests are well-decomposed.
- `formatRelative` has a dedicated test extension separate from `formatMtime`.

**Findings**:
- 🟡 Major: AC3 "exactly one re-render per tick" not asserted — Phase 6.
- 🟡 Major: AC11 component test bypasses self-cause semantics — Phase 6.
- 🟡 Major: Delete-branch ring-buffer push not directly tested — Phase 1+3.
- 🟡 Major: AC8 lacks dedicated on-disk edit assertion — Phase 1.
- 🟡 Major: Concurrent-push test claimed in narrative but absent — Phase 2.
- 🔵 Minor: `formatRelative` boundary values not tested — Phase 4.
- 🔵 Minor: Loading-state behaviour not tested — Phase 6.
- 🔵 Minor: AC6 assertion bound vs AC text — Phase 1.
- 🔵 Minor: `?limit=0`/invalid handler behaviour not tested — Phase 3.
- 🔵 Minor: Restart-empty-state is manual-only — Phase 3.
- 🔵 Minor: No create→edit→delete chain test — Phase 1.

### Correctness

**Summary**: Largely correct in action-mapping logic, ring-buffer semantics, and synchronous-push-before-broadcast ordering. Two genuine correctness gaps: (1) on SSE reconnect the refetched initial history overlaps with retained `live` state, producing duplicate rows; (2) the TS type-level change to `SseDocChangedEvent` (`action`/`timestamp` required) will break ~20 existing test fixtures that the plan does not mention migrating. Several minor concerns around timestamp capture distance, AC11 verification scope, malformed-state transitions, and React key collisions.

**Strengths**:
- Pre/post action mapping is sound across all three transitions.
- Ring buffer pushed synchronously immediately before broadcast.
- Plan correctly excludes DocInvalid from ring-buffer push.
- Subscriber dispatch ordering explicitly placed before line-155 with documented divergence.
- `payload_for_entry` has exactly one caller — inline-and-delete is safe.
- Capacity-based correctness fully covered by unit tests.
- Per-enum `rename_all = "lowercase"` correctly identified.

**Findings**:
- 🟡 Major: Duplicate rows on SSE reconnect — Phase 6.
- 🟡 Major: Making `action`/`timestamp` required breaks ~20 TS fixtures — Phase 4.
- 🔵 Minor: `now` captured before async lookup; AC7's 1s tolerance bounds drift — Phase 1.
- 🔵 Minor: AC11 component test bypasses self-cause — Phase 6.
- 🔵 Minor: Including index `i` in React key defeats stable-key benefit — Phase 6.
- 🔵 Minor: Valid→malformed transitions surface as DocInvalid; feed silently drops them — Phase 1.
- 🔵 Minor: `Date.parse` truncates chrono's nanosecond precision — Phase 6.

### Performance

**Summary**: Performance posture is appropriate for the expected scale: file-change events are debounced to ~100ms, the ring buffer is capped at 50, and the UI surfaces only 5 rows. Hot-path costs (one extra small clone per event, one O(1) mutex section, one extra Set-iteration) are negligible. The choice of `std::sync::Mutex` inside the async debounce coroutine is defensible given the coroutine is single-task per path and critical sections are O(1), but deserves an explicit note in the plan.

**Strengths**:
- Ring buffer bounded and uses `VecDeque` with O(1) `pop_back`/`push_front`.
- Synchronous push avoids the lossy broadcast-channel rebroadcast pattern.
- 60s ticker is a single `setInterval` per ActivityFeed instance.
- Listener Set iteration cost in onmessage is bounded.
- Plan explicitly recognises hot vs cold paths in "Performance Considerations".

**Findings**:
- 🔵 Minor: `std::sync::Mutex` inside async debounce coroutine — Phase 2/3.
- 🔵 Suggestion: Two-pass payload + ActivityEvent construction clones `path` per event — Phase 3.
- 🔵 Suggestion: `Date.parse` on every render of every row — Phase 6.
- 🔵 Suggestion: `setLive` array allocation per SSE event — Phase 6.
- 🔵 Minor: Subscriber fan-out runs even for self-caused events — Phase 5.

### Compatibility

**Summary**: Plan correctly treats the JSON change as additive (forward-compat for consumers that ignore unknown keys) and uses the minimum chrono feature set. However, the plan understates the TypeScript fixture migration: there are ~23 sites across four test files that construct `{ type: 'doc-changed', ... }` literals and will all fail typecheck once `action` and `timestamp` are promoted to required fields. It also misses a non-test Rust call site (`api/docs.rs:242`) that constructs `SsePayload::DocChanged` and must be updated. ActivityEvent's deliberate omission of `etag` is consistent with AC9's deletion-signal contract.

**Strengths**:
- JSON wire-format change is genuinely additive at the JSON layer.
- Chrono feature flags are minimal and correct.
- `etag` retained as `Option<String>` with `skip_serializing_if`, keeping legacy delete signal intact.
- Decision to NOT add `'activity'` to `SESSION_STABLE_QUERY_ROOTS` is correct.

**Findings**:
- 🟡 Major: Plan misses production call site `api/docs.rs:242` — Phase 1+3.
- 🟡 Major: Promoting `action`/`timestamp` to required TS fields breaks ~23 fixtures — Phase 4.
- 🔵 Minor: New-TS-frontend-against-old-server produces undefined action labels — Migration Notes.
- 🔵 Minor: Silent glyph omission for future virtual DocTypeKey — Phase 6.
- 🔵 Minor: No upper bound on `?limit` — Phase 3.

## Re-Review (Pass 2) — 2026-05-14

**Verdict:** COMMENT

### Previously Identified Issues

**Architecture (8)**
- 🟡 **Module mixes domain + transport** — Resolved (Phase 2 moves `ActivityRingBuffer` to crate-root `activity_feed.rs`; handler stays in `api/activity.rs`)
- 🟡 **Watcher dual-sink duplication** — Resolved (single `emit` helper + `From<&SsePayload>` projection)
- 🟡 **subscribe/onEvent unification** — Partially resolved (deferred as deliberate trade-off; documented in TSDoc and "What We're NOT Doing")
- 🔵 **Inlining `payload_for_entry`** — Resolved (helper signature evolved instead)
- 🔵 **ActionKind value-rename TS drift** — Still present (pre-existing minor concern; no codegen/round-trip test added)
- 🔵 **Double-wrapping Sidebar section** — Resolved (Phase 7 mounts ActivityFeed directly)
- 🔵 **Two AppState `activity*` fields** — Resolved (rename to `http_activity`)
- 🔵 **`std::sync::Mutex` in async path** — Resolved (Mutex discipline + poisoning policy documented)

**Code Quality (9)**
- 🟡 **formatMtime/formatRelative drift** — Resolved (shared `formatElapsedShort` ladder)
- 🟡 **React key index** — Resolved (`timestamp|path|action` key)
- 🟡 **console.warn swallowing** — Resolved (escalated to `console.error` with explicit message)
- 🔵 **expect poison panic** — Resolved (`unwrap_or_else(|p| p.into_inner())`)
- 🔵 **`as any` cast** — Resolved (typed `mockHandle()` factory)
- 🔵 **Module-level `capturedListener`** — Resolved (per-mount `getListener()`)
- 🔵 **Glyph silent omission** — Partially resolved (documented; widening Glyph deferred)
- 🔵 **"Cloning twice fine" precedent** — Resolved (single `emit` helper supersedes)
- 🔵 **action-mapping helper** — Resolved (folded into evolved `payload_for_entry`)

**Test Coverage (11)** — All 11 Resolved
- AC3 exactly-once tick, AC11 hook-layer cross-reference, delete-branch ring-buffer push, AC8 ordering, concurrent-push test, formatRelative boundary tests, loading-state test, AC6 verbatim assertion, `?limit=0`/`foo`/`999999` tests, automated restart-empty-state, create→edit→delete chain — all addressed with concrete plan edits.

**Correctness (7)** — All 7 Resolved
- Dedup-on-reconnect, ~23 TS fixture migration, `now` capture distance, AC11 cross-reference, React key, valid→malformed transition policy, Date.parse truncation note — all addressed.

**Performance (5)**
- 🔵 **`std::sync::Mutex` in async** — Resolved (discipline documented)
- 🔵 **Two-pass clone** — Resolved (single projection via `emit`)
- 🔵 **Date.parse per render** — Still present (acceptable at LIMIT=5; no change needed)
- 🔵 **setLive allocation per event** — Still present (acceptable at LIMIT=5; no change needed)
- 🔵 **Subscriber fan-out for self-caused** — Resolved (deliberate per AC11)

**Compatibility (5)** — All 5 Resolved or Mitigated
- `api/docs.rs:242` enumerated, ~23 TS fixtures enumerated, defensive coercion added, glyph silent-omission documented, `?limit` capped at `CAPACITY` server-side.

### New Issues Introduced

#### Major

- 🟡 **Correctness**: `formatElapsedShort` extraction risks regressing `formatMtime`'s `'just now'` negative-elapsed contract
  **Location**: Phase 4, Section 4 — shared `formatElapsedShort` helper
  The helper returns `null` for `diffSec < 0` ("caller decides the clamp"), but the plan's claim "existing callers and tests are unaffected" requires the refactored `formatMtime` to add its own `if (diffSec < 0) return 'just now'` guard before delegating. The plan does not show the refactored `formatMtime` body — a naive implementation `return formatElapsedShort(diffSec) ?? <weeks/locale fallback>` would route negative-elapsed inputs to the weeks-fallback path, breaking the existing 'just now' branch consumed by Library mtime rendering. Either show the refactored `formatMtime` body explicitly, or restructure `formatElapsedShort` to take a `negativeClamp` parameter so the divergence is encoded at the call site.

#### Minor

- 🔵 **Architecture**: `emit` helper placement in `sse_hub.rs` inverts the typical leaf-module direction
  **Location**: Phase 3, Section 3
  Placing `emit` in `sse_hub.rs` means the SSE hub now imports `activity_feed::{ActivityEvent, ActivityRingBuffer}`, growing its responsibility from "broadcast" to "broadcast + side-effect dispatcher". A future SSE consumer cannot reuse the hub without dragging the activity feed in. Consider moving `emit` to `watcher.rs` (its sole caller) or a small `dispatch.rs` glue module.

- 🔵 **Architecture**: `From<&SsePayload> for Option<ActivityEvent>` is stylistically idiosyncratic
  **Location**: Phase 2, Section 1
  Conflates "convert" and "filter" under a `From` impl that returns `None` for a structurally valid input. Prefer an inherent method `ActivityEvent::from_payload(payload: &SsePayload) -> Option<Self>` — same testability, clearer call sites, filter semantic visible at the use site.

- 🔵 **Architecture**: TSDoc-only divergence relies on discipline, not types
  **Location**: Phase 5, Section 1
  Subscribers and `onEvent` consumers receive the same `SseEvent` type with different self-cause semantics — a typed wrapper or naming distinction (`subscribeRaw` vs `subscribeFiltered`) would survive future refactors better than prose. Non-blocking.

- 🔵 **Code Quality**: dedup key + React key duplicate the same `(timestamp, path, action)` triple
  **Location**: Phase 6, Section 1 — `dedupeRows` and row `key` prop
  Two consumers rely on the same equality predicate for different reasons. Extract a single helper `activityRowId(event)` used by both the React key and the dedup `Set`.

- 🔵 **Code Quality**: Defensive coercion silently drops events with no diagnostic
  **Location**: Phase 6, Section 1 — subscribe handler `if (!ev.action || !ev.timestamp) return`
  In dev-mode drift the activity feed will appear broken with zero console signal. Add a `console.warn('[activity-feed] dropping event missing action/timestamp — likely cross-version dev drift', ev)` (gated by a once-per-session flag if noisiness is a concern). Aligns with the Phase 5 listener-error policy ("silent swallowing here would mask real bugs").

- 🔵 **Code Quality**: Small `as SseDocChangedEvent` cast inside the component
  **Location**: Phase 6, Section 1 — `const ev = event as SseDocChangedEvent`
  After `if (event.type !== 'doc-changed') return`, TypeScript's discriminated-union narrowing should already yield `SseDocChangedEvent` without an explicit cast. Drop the cast to match the "no unsafe casts" rhythm established in the test harness.

- 🔵 **Test Coverage**: AC3 render-count probe via `vi.spyOn(formatRelative)` won't intercept ES module bindings
  **Location**: Phase 6, Section 2 — AC3 test outline
  Module-imported function references are live bindings; `vi.spyOn` on the imported binding does not redirect calls inside the component. Specify the child-component render-counter approach explicitly (extract `Row` into a tiny subcomponent and count its render calls), or require `vi.mock('../../api/format', ...)` so the spy is intercepted at module load.

- 🔵 **Test Coverage**: Dedup-on-reconnect test synthesises duplication rather than driving a real reconnect
  **Location**: Phase 6, Section 2 — test #10
  Replays an event through the subscribe listener; the actual production scenario (connectionState transition triggers `useQuery` refetch returning a superset that overlaps with retained `live` rows) is not exercised end-to-end. Add a second test that invalidates the activity query while live rows are present and asserts no duplicates after refetch resolves.

- 🔵 **Test Coverage**: AC6 assertion uses `num_seconds().abs()` which truncates toward zero
  **Location**: Phase 1, Section 5
  Minor sensitivity around the truncation direction — `num_milliseconds().abs() < 1000` is a less-truncation-sensitive expression of the same contract. The strict-containment assertion catches the realistic case; cosmetic improvement only.

- 🔵 **Correctness**: `dedupeRows` does not re-sort by timestamp on interleave
  **Location**: Phase 6, Section 1
  Combined order is `live (newest-first by prepend) ++ initial (newest-first from server)`. In the rare case where subscribe fires before `useQuery` resolves and the server's ring already has events newer than the live ones, the rendered order is `[stale-live, newer-initial]` rather than time-sorted. Sort by `timestamp.localeCompare` descending after dedup, before `.slice(0, LIMIT)`.

- 🔵 **Correctness**: Defensive coercion fires on `action: ""` indistinguishably from missing
  **Location**: Phase 6, Section 1
  A server emitting an empty `action` string is a server bug, not a wire-format omission, but the guard handles both identically with no diagnostic. Tighten to `if (typeof ev.action !== 'string' || !ev.timestamp)` and `console.warn` on the empty-string case so server-side regressions in `ActionKind` serialisation surface in dev.

- 🔵 **Compatibility**: AppState rename audit grep scope is `src/` only — integration tests under `server/tests/` ignored
  **Location**: Phase 3, Section 2
  Today none of the 12+ integration tests dereference `state.activity.<method>` (only construct `AppState::build(cfg, activity)`), so the rename is contained — but a future `state.activity.foo()` reaching `tests/` would be missed by the as-written grep. Broaden to `server/{src,tests}/`.

- 🔵 **Compatibility**: `?limit` clamp at `CAPACITY` is undocumented in the response
  **Location**: Phase 3, Section 1
  A client requesting `?limit=200` receives up to 50 events with no signal of the clamp (no `X-Total-Count`, no 4xx). Document in the handler TSDoc and Migration Notes ("clients requesting `?limit > 50` receive the buffer's current size, not an error"), or echo the effective limit in the response envelope.

### Assessment

The revisions resolve all 14 prior majors and 21 of 23 prior minors. Two prior minors remain by deliberate decision (the Glyph silent-omission is documented; the ActionKind TS/Rust vocabulary duplication is a pre-existing concern outside this work item's scope) and two performance suggestions are accepted as trivial at LIMIT=5. The plan is in significantly better shape and architecturally ready to implement.

**One new major** (the `formatElapsedShort` extraction risks silently regressing `formatMtime`'s `'just now'` branch if the refactor is implemented as the helper signature suggests) is worth addressing before Phase 4 lands — a 3-line plan edit to either show the refactored `formatMtime` body or restructure the helper to carry a `negativeClamp` parameter.

The 13 new minors are tractable polish — most resolve in a few lines each during implementation, and several are non-blocking documentation/diagnostic improvements.

Recommended verdict: **COMMENT** — plan is acceptable as-is, but the new major and several minors merit a quick second pass.
