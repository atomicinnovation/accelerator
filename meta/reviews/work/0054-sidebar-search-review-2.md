---
date: "2026-06-01T13:11:07+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0054"
work_item_id: "0054"
review_number: 2
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
id: "0054-sidebar-search-review-2"
title: "0054-sidebar-search-review-2"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-06-01T13:11:07+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: Sidebar Search Input and API Search Endpoint

**Verdict:** REVISE

The work item remains in strong shape overall — completeness is essentially clean, scope is well-justified as a vertical slice with self-aware infrastructure follow-up posture, and acceptance criteria are uniformly Given/When/Then with worked examples. However, three concerns reach `major` severity this pass: a wire-contract contradiction between Requirements (`mtime_ms`, `slug: string`) and AC4 (`mtimeMs`, `slug: string | null`) flagged independently by clarity, completeness, and testability; an AC4 "within the same React render" clause that is not deterministically verifiable; and the absence of any AC for the modifier-key suppression rule (Cmd/Ctrl/Alt/Meta + `/`) that Technical Notes commits to. Two majors plus reinforcement from a third triggers REVISE under the configured threshold.

Prior review (review-1) closed at COMMENT after three passes with the work item judged "ready for implementation"; this pass surfaces issues that escaped prior attention because they live in cross-section contract consistency and a single missing AC, rather than the polish dimensions the prior passes converged on.

### Cross-Cutting Themes

- **Wire-contract drift between Requirements, AC4, and Technical Notes** (flagged by: clarity, completeness, testability) — the response row's `mtime_ms` vs `mtimeMs` field casing and `slug: string` vs `slug: string | null` nullability disagree between the Requirements bullet and AC4; Technical Notes confirms the camelCase/nullable shape, but a reader of Requirements alone will build a different contract than a reader of AC4. This is the single highest-impact theme this pass.
- **Detail-view URL contract is implicit** (flagged by: dependency, testability) — AC10 requires navigation to "the document detail view for that row's `docType` + `slug`" but neither AC10 nor Dependencies nor Technical Notes pins the URL template or names the existing route helper. Two equally plausible `href` shapes can both pass the AC as currently written.
- **Ranking algorithm placement straddles Requirements and ACs** (flagged by: scope, clarity) — the four-bucket ranking strategy is repeated across Requirements, AC5–AC8, Technical Notes, and Drafting Notes, with subtle wording drift (e.g., "exact-slug" appears in Requirements without the case-insensitive qualifier that AC5/AC6 add). The repetition increases the surface where drift can hide.

### Findings

#### Major

- 🟡 **Clarity + Completeness + Testability**: Response field naming and nullability contradict between Requirements and Acceptance Criteria
  **Location**: Requirements / Acceptance Criteria (AC4)
  Requirements declares the row as `{ docType, title, slug: string, path, mtime_ms: number }` (snake_case `mtime_ms`, non-nullable `slug`); AC4 declares the same row as `{ docType, title, slug: string | null, path, mtimeMs: number }` (camelCase `mtimeMs`, nullable `slug`). Technical Notes agrees with AC4 (camelCase via `#[serde(rename_all = "camelCase")]`, `slug: Option<String>`), leaving Requirements as the sole outlier. AC prose then refers to `mtime_ms` repeatedly as if it were the JSON field, compounding the ambiguity. A reader implementing from Requirements would produce a different wire contract than one implementing from AC4.

- 🟡 **Testability**: "Within the same React render" clause in AC4 is hard to verify deterministically
  **Location**: Acceptance Criteria (AC4)
  AC4 requires results to render "inline beneath the input within the same React render in which the response resolves." Whether a row appears in "the same render" versus "the next render" depends on React internals (concurrent batching, microtask ordering) and is not observable from a DOM or RTL assertion — only the steady-state DOM is observable. An implementation that renders results one render later would satisfy every observable assertion while technically violating this clause, and a reviewer cannot distinguish the two without inspecting React's commit cycle.

- 🟡 **Testability**: No AC covers modifier-key suppression of the `/` keybind
  **Location**: Acceptance Criteria (missing AC)
  Technical Notes commits to suppressing the `/` keybind when Cmd, Ctrl, Alt, or Meta are held ("no modifier keys — Cmd, Ctrl, Alt, or Meta held suppresses activation and lets the default browser behaviour proceed"), but no AC encodes this. AC1 and AC2 cover the focused-vs-unfocused dispatch but say nothing about modifier state. An implementation that fires the keybind on Cmd+`/` (clobbering a common browser/IME shortcut) would pass every current AC.

#### Minor

- 🔵 **Clarity**: Ranking-bucket wording subtly differs between Requirements and AC5/AC6
  **Location**: Requirements / Acceptance Criteria
  Requirements states bucket-1 as "exact-slug" without a case qualifier; AC5/AC6 and Drafting Notes describe the same bucket as "exact-slug match (case-insensitive)." A reader implementing from Requirements alone could plausibly build a case-sensitive matcher.

- 🔵 **Clarity**: AC9 parenthetical mixes two distinct rules
  **Location**: Acceptance Criteria (AC9)
  AC9 states the substring-free → absent rule and then adds a parenthetical that separately enumerates the searched fields (`title`, `slug`, `body_preview`) and excluded fields (`path`, `rel_path`). The "fields searched" rule is load-bearing for several other ACs but is buried as a parenthetical inside AC9.

- 🔵 **Clarity**: 'LIBRARY' (caps) is used as a defined term without an explicit definition
  **Location**: Context / Requirements
  'LIBRARY' appears throughout in all-caps as if a defined label. Context glosses it as "the twelve non-`Templates` variants of `DocTypeKey`" but never introduces the term itself, so the all-caps token reads as an unfamiliar acronym rather than a defined label.

- 🔵 **Clarity**: 'Shared ref' wording in Technical Notes risks contradicting 'no new context'
  **Location**: Technical Notes (Frontend wiring)
  Technical Notes describes `RootLayout` as focusing the input "via a shared ref" and later commits to passing the ref by prop with "no new context." The phrase "shared ref" could be read as an additional shared object (context, module-level singleton).

- 🔵 **Dependency + Testability**: Document-detail navigation URL pattern is unspecified
  **Location**: Acceptance Criteria (AC10) / Dependencies
  AC10 requires navigation to "the document detail view for that row's `docType` + `slug`" without naming the URL template or the route helper. The `<a href>` cannot be asserted as a literal and any future change to the detail-route shape silently breaks search-result links because the coupling is not recorded.

- 🔵 **Dependency**: Follow-up ranking-quality story is named but not tracked
  **Location**: Assumptions
  Assumptions says "that work is tracked as a follow-up story" but no ID is given and Blocks/Coordinates-with does not name one. The downstream dependant is gestured at but not discoverable.

- 🔵 **Testability**: AC14 'in every LIBRARY type' fixture is underspecified
  **Location**: Acceptance Criteria (AC14)
  AC14 requires a substring matching at least one entry per LIBRARY type and one Templates entry, but does not enumerate the twelve types or provide a candidate query — the fixture-construction cost is hidden, and a tester could omit one or two types and still claim AC14 passes.

- 🔵 **Testability**: AC3 cache-hit example depends on React Query cache lifetime not pinned by any AC
  **Location**: Acceptance Criteria (AC3)
  Example (a) ("while the React Query cache entry for `queryKeys.search('ab')` is still live yields exactly one network request") depends on `staleTime` / `gcTime` configuration that no AC, Requirement, or Technical Note pins. A verifier cannot deterministically know how long "still live" lasts.

- 🔵 **Testability**: No AC covers the case where the query falls below 2 chars after results were shown
  **Location**: Acceptance Criteria (missing AC)
  AC11 covers "fewer than 2 chars → no request, no results" but only as a starting state. AC12 covers in-flight transitions between two ≥2-char queries. Neither covers: type `ab` (rows render), delete to `a` — should the previously rendered rows be cleared?

- 🔵 **Testability**: AC5 'Fixtures may rely on this ordering' is a permission, not a verification criterion
  **Location**: Acceptance Criteria (AC5)
  The trailing clause is a meta-comment to test authors, not an observable behaviour. Mixing it with the system-behaviour assertion in the same bullet makes it unclear what a verifier asserts.

#### Suggestions

- 🔵 **Dependency**: `IndexEntry.body_preview` field availability could be cross-referenced more visibly
  **Location**: Technical Notes
  The ranking strategy reads `body_preview` from `IndexEntry`; the coupling is captured inside a long Technical Notes paragraph rather than surfaced as a structural dependency. A future indexer refactor that removes the field would silently regress the body-preview bucket.

- 🔵 **Scope**: Server ranking strategy could plausibly be a separable story
  **Location**: Requirements / Acceptance Criteria
  The four-bucket ranking algorithm consumes five of the fifteen ACs and is somewhat orthogonal to "wire a search input to a backend endpoint." Bundling is defensible as a vertical slice; flagged only as a judgement call.

- 🔵 **Scope**: Two net-new frontend primitives in one story
  **Location**: Technical Notes / Requirements
  `useDebouncedValue` plus the first global `keydown` listener in `RootLayout.tsx` are both net-new infrastructure. The story already reasons about this via the "Infrastructure follow-up posture" paragraph; worth flagging only as upper-edge sizing for M.

### Strengths

- ✅ Frontmatter is fully populated with a valid kind/status/priority/parent/tags/author/date.
- ✅ Summary opens with a user-perspective statement ("As a user… I want… so I can…") followed by concrete what-is-being-built enumeration, satisfying both "what" and "why" demands.
- ✅ Fifteen Given/When/Then acceptance criteria cover keybind dispatch, debounce semantics, ranking buckets, rendering, transitional state, error handling, modifier-clicks, and backend-only contract behaviour — well above any minimum.
- ✅ Worked examples disambiguate trailing-edge debounce (`ab`→`a`→`ab` cache-hit; `ab`→`abc`→`ab` intermediate-keystroke) — eliminating a common source of misreading.
- ✅ Ranking is decomposed into four named buckets with explicit tie-breakers (`mtime_ms` desc, `path` asc); fixtures can target each bucket boundary independently.
- ✅ Negative criteria are explicit: AC9 (field-search closed set), AC11 (sub-2-char), AC13 (error UX with `FetchError` + URL), AC14 (Templates exclusion), AC15 (empty query → `{ results: [] }`).
- ✅ Dependencies names parent, siblings, prior blockers (with status), a coordinating sibling (0083), and forward `DocTypeKey` enum coupling.
- ✅ Scope is self-aware about net-new infrastructure: explicit "Infrastructure follow-up posture" defers generalisation until a second consumer appears.
- ✅ Technical Notes pins file:line references (`indexer.rs:570-572`, `api/docs.rs:30-41`, `Sidebar.tsx:22-33`) so implementers can trace the surface area without rediscovery.
- ✅ State ownership is stated explicitly (`Sidebar.tsx` owns query/results; `RootLayout.tsx` owns listener/ref), avoiding the usual cross-component "who owns what" ambiguity.

### Recommended Changes

1. **Resolve the response-shape contradiction in one place** (addresses: Response field naming and nullability contradict)
   Update the Requirements bullet to match AC4 and Technical Notes: change `mtime_ms: number` to `mtimeMs: number` and `slug: string` to `slug: string | null`. Then audit AC prose (AC4, AC5, AC8) — anywhere `mtime_ms` is used to refer to the JSON field, replace with `mtimeMs`; keep `mtime_ms` only where it refers to the underlying `IndexEntry` field (Rust side). Optionally add a single "Response schema" code block that Requirements and AC4 both link to, to prevent future drift.

2. **Replace the 'same React render' clause with a steady-state assertion** (addresses: 'Within the same React render' clause hard to verify)
   Reword AC4 to assert observable steady state, e.g. "after the response resolves, the results area contains exactly the response rows in response order, queryable via `findAllByRole('link')`." If the intent is to forbid an intermediate empty-state flash, encode that in AC12-style transitional-content language rather than coupling to React's commit cycle.

3. **Add a modifier-key suppression AC** (addresses: No AC covers modifier-key suppression of the `/` keybind)
   Add: "Given no text input, textarea, or contenteditable element has focus, when the user presses `/` while holding any of Cmd, Ctrl, Alt, or Meta, then focus does not move to the Sidebar search input and `preventDefault` is not called (the keystroke is delivered with native browser semantics)."

4. **Pin the document-detail URL contract in AC10** (addresses: Document-detail navigation URL pattern is unspecified)
   Either name the URL template inline (e.g., "the row's `href` is `/library/<docType>/<slug>` where `docType` is the kebab-case wire form") or reference the existing route helper / constant the implementation should use. Add a one-line Technical Notes entry pointing to the detail-route module so the coupling is recorded.

5. **Pin React Query cache lifetime for AC3 cache-hit example** (addresses: AC3 cache-hit depends on cache lifetime not pinned)
   Either state in Requirements/Technical Notes that the global QueryClient defaults are used (with the implication that "still live" means within the default 5-minute `gcTime`), or bound the AC3 example timing explicitly ("within 1 second of the first response").

6. **Add an AC for the query-drops-below-2 case** (addresses: No AC covers query falling below 2 chars after results were shown)
   Add: "Given previously rendered results are visible, when the user edits the query so that the settled query falls below 2 characters, then the results area is cleared (no rows, no `No matches` element)."

7. **Lift the field-search closed-set rule out of AC9's parenthetical** (addresses: AC9 parenthetical mixes two distinct rules)
   Promote the "fields searched are `title`, `slug`, `body_preview`; `path` and `rel_path` are not searched" statement into its own Requirements bullet or its own AC, leaving AC9 to focus on the substring-free → absent rule.

8. **Move 'Fixtures may rely on this ordering' out of AC5** (addresses: 'Fixtures may rely on this ordering' is a permission, not a criterion)
   Move it to Drafting Notes or Technical Notes; leave AC5 as a pure behaviour assertion.

9. **Reduce ranking-bucket wording drift** (addresses: Ranking-bucket wording subtly differs between Requirements and AC5/AC6)
   Add "case-insensitive" to the bucket descriptors in the Requirements bullet and Drafting Notes so all three places state the same matching rule.

10. **Enumerate or link the LIBRARY doc types for AC14** (addresses: AC14 fixture underspecified; 'LIBRARY' used as defined term without definition)
    Add a Technical Notes line (or a single Context line) listing the twelve LIBRARY doc types by stable name. Optionally introduce the term "LIBRARY" explicitly in Context ("we refer to the twelve non-`Templates` `DocTypeKey` variants collectively as the LIBRARY doc types").

11. **Tighten 'shared ref' wording in Technical Notes** (addresses: 'Shared ref' risks contradicting 'no new context')
    Replace "via a shared ref" with "via the `searchInputRef` prop passed from `RootLayout` to `Sidebar`" so wording matches the explicit plumbing decision.

12. **Resolve the follow-up ranking story phrasing** (addresses: Follow-up ranking-quality story is named but not tracked)
    Either drop the "tracked as a follow-up story" phrasing in Assumptions or, if a placeholder is intended, create the work item and reference its ID under Blocks/Coordinates-with.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually precise and internally consistent: terms like 'settled query', 'twelve LIBRARY doc types', 'the keybind listener', and the four-bucket ranking strategy are explicitly defined and used consistently across Context, Requirements, and Acceptance Criteria. Two minor inconsistencies stand out — the response field naming flips between `mtime_ms` (Requirements) and `mtimeMs` (AC4) and between `slug: string` (Requirements) and `slug: string | null` (AC4) — which together create a real ambiguity about the wire contract. A small number of undefined/team-domain terms (LIBRARY, Glyph, FetchError) are likely fine for an in-team reader but worth noting.

**Strengths**:
- Pronouns and referents are tightly controlled.
- 'Twelve LIBRARY doc types' is defined in Context with a file reference.
- Actor and trigger are named for each behavioural rule.
- AC3 debounce semantics are disambiguated with worked examples.
- State ownership is stated explicitly.

**Findings**:
- 🟡 major / high — Response field naming and nullability contradict between Requirements and AC4 (Requirements / AC)
- 🔵 minor / high — Ranking-bucket wording subtly differs (Requirements omits case-insensitive)
- 🔵 minor / medium — AC9 parenthetical mixes two distinct rules
- 🔵 minor / medium — 'LIBRARY' used as defined term without explicit definition
- 🔵 minor / medium — 'Shared ref' wording risks contradicting 'no new context'

### Completeness

**Summary**: The work item is comprehensively populated against the completeness lens: all expected sections for a story are present and substantively filled. No structural gaps were identified; the only observation is the wire-contract naming inconsistency, also surfaced by clarity and testability.

**Strengths**:
- Frontmatter is complete and valid.
- Summary covers both "what" and "why".
- Fifteen acceptance criteria cover happy paths and edge cases.
- Optional sections (Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes, References) all substantively populated.

**Findings**:
- 🔵 minor / medium — Response field naming inconsistency (mtime_ms vs mtimeMs, slug vs slug | null)

### Dependency

**Summary**: Dependency capture is unusually thorough for a story: the parent epic, sibling deliverables, prior blockers, a coordinating sibling, and a forward-looking enum coupling are all explicitly named. The few gaps are minor — an implicit dependency on the existing document-detail routing target for result-row navigation, and forward-looking follow-up work mentioned in Assumptions without a tracking ID.

**Strengths**:
- Dependencies names parent (0036), siblings (0053, 0055), prior blockers (0037, 0053, 0055).
- `DocTypeKey` enum coupling captured in Context and Blocks.
- Coordinates with 0083 proactively named.
- Internal touchpoints named with file/line precision.
- Assumptions explicitly state `/api/search` is in-scope for this story.

**Findings**:
- 🔵 minor / medium — Document-detail navigation target is an implicit dependency (AC10 / Dependencies)
- 🔵 minor / medium — Follow-up ranking-quality story is named but not tracked (Assumptions)
- 🔵 suggestion / low — `IndexEntry.body_preview` field availability could be cross-referenced more visibly (Technical Notes)

### Scope

**Summary**: A well-scoped story that delivers a single user-visible capability — sidebar search — by combining the new `/api/search` endpoint and frontend wiring. Summary, Requirements, and ACs describe the same scope; the story is consciously sized M to accommodate small net-new infrastructure (debounce hook, global `/` keybind) that exists solely in service of this feature.

**Strengths**:
- Single coherent purpose.
- Clear cross-boundary orchestration with explicit ownership note.
- Explicit boundaries against siblings 0053 and 0055.
- Net-new infrastructure intentionally introduced as single-consumer primitives with stated follow-up posture.
- Sizing well-justified with enumerated contributing pieces.

**Findings**:
- 🔵 suggestion / medium — Server ranking strategy could plausibly be a separable story (Requirements / AC)
- 🔵 suggestion / low — Two net-new frontend primitives in one story (Technical Notes / Requirements)

### Testability

**Summary**: The Acceptance Criteria are unusually testable — most are framed as concrete Given/When/Then scenarios with named procedures, specific thresholds (200 ms, 2-char minimum), and worked examples. However, a contract inconsistency between Requirements and AC4 over field casing and slug nullability creates ambiguity about which schema a verifier should assert against, AC4's "same React render" clause introduces an implementation-coupled assertion that is difficult to verify deterministically, and the modifier-key suppression rule for `/` appears only in Technical Notes with no corresponding AC.

**Strengths**:
- Uniform Given/When/Then framing with named preconditions and observable post-conditions.
- AC3 trailing-edge debounce pinned with concrete worked examples.
- Ranking broken into four named buckets with explicit tie-breakers.
- Negative criteria explicit (sub-2-char, Templates exclusion, empty query, field-search closed set).
- AC12 specifies loading-transition contract.
- AC13 names exact failure-mode observable (`console.error` + `FetchError` + URL).

**Findings**:
- 🟡 major / high — Response shape contradiction makes contract assertions ambiguous (Requirements / AC4)
- 🟡 major / medium — 'Within the same React render' clause is hard to verify deterministically (AC4)
- 🟡 major / high — No AC covers modifier-key suppression of the `/` keybind (missing AC)
- 🔵 minor / medium — AC14 'in every LIBRARY type' fixture is underspecified (AC14)
- 🔵 minor / medium — AC3 cache-hit example depends on React Query cache lifetime not pinned (AC3)
- 🔵 minor / high — No AC covers the empty-results case when query falls back below 2 chars (missing AC)
- 🔵 minor / medium — AC5 'Fixtures may rely on this ordering' is a permission, not a verification criterion (AC5)
- 🔵 minor / low — AC10 'document detail view' has no specified URL or route shape (AC10)

## Re-Review (Pass 2) — 2026-06-01T13:11:07+00:00

**Verdict:** COMMENT

All three Pass-1 majors and all ten Pass-1 minors are resolved by the edits. Re-running clarity, completeness, dependency, and testability surfaces zero critical or major issues; nine new minor polish items remain (a typo, an `<a href>` equality predicate question, and a few "this could be more verifiable" notes that map naturally to test-design choices). Verdict moves from REVISE to COMMENT.

### Previously Identified Issues

#### Pass-1 Majors
- ✅ **Clarity + Completeness + Testability**: Response field naming and nullability contradiction — **Resolved** (Requirements updated to `slug: string | null` and `mtimeMs: number`; case-insensitive note added; field-search closed set lifted to Requirements)
- ✅ **Testability**: 'Within the same React render' clause in AC4 — **Resolved** (replaced with observable steady-state assertion using `await waitFor(() => screen.findAllByRole('link'))`)
- ✅ **Testability**: No AC covers modifier-key suppression of `/` keybind — **Resolved** (new AC added covering Cmd/Ctrl/Alt/Meta+`/` → no activation, no `preventDefault`)

#### Pass-1 Minors
- ✅ **Clarity**: Ranking-bucket "case-insensitive" missing from Requirements — **Resolved**
- ✅ **Clarity**: AC9 parenthetical mixed two distinct rules — **Resolved** (fields-searched lifted to Requirements; AC9 now points to it)
- ✅ **Clarity**: 'LIBRARY' undefined — **Resolved** (Context introduces the term explicitly and enumerates all twelve variants + kebab-case wire forms; also corrected stale `doc_type.rs` path to `docs.rs:6-20`)
- ✅ **Clarity**: 'Shared ref' wording — **Resolved** (replaced with explicit `searchInputRef` prop language; "no module-level singleton" added)
- ✅ **Dependency + Testability**: Document-detail navigation URL pattern — **Resolved** (AC10 now pins `<a href="/library/<docType>/<slug>">` and references `libraryDocRoute` in `frontend/src/router.ts`; null-slug rows explicitly not rendered as links)
- ✅ **Dependency**: Follow-up ranking story phrasing — **Resolved** ("No follow-up story is currently tracked; capture a new work item at that point")
- ✅ **Testability**: AC14 fixture underspecified — **Resolved** (Context now enumerates all twelve LIBRARY variants, providing the fixture checklist)
- ⚠️ **Testability**: AC3 cache lifetime not pinned — **Partially resolved**: cache lifetime is now pinned in Technical Notes (global QueryClient defaults, gcTime 5 min, ~1s retest window); Pass-2 testability re-flags this as worth promoting into the AC body itself (see new findings)
- ✅ **Testability**: Query-drops-below-2 AC missing — **Resolved** (new AC added covering settled-query falling below 2 chars clears results)
- ✅ **Testability**: AC5 'Fixtures may rely on this ordering' as permission, not criterion — **Resolved** (moved to Drafting Notes with deterministic-ordering rationale)

### New Issues Introduced

#### Minor (from Pass-1 edits)
- 🔵 **Clarity**: Typo "the the" in Technical Notes → Frontend wiring (the cache-lifetime sentence reads "re-trigger the the debounce settled-query criterion example"). Cosmetic, but worth a quick fix.
- 🔵 **Clarity**: Path forms drift — `Sidebar.tsx` vs `Sidebar/Sidebar.tsx` are both used for what is the same file. Standardise to one form (likely `frontend/src/Sidebar/Sidebar.tsx`).
- 🔵 **Clarity**: "no new context" parenthetical in Requirements is briefly ambiguous between React Context and English "context". Technical Notes uses the clearer "no new React Context, module-level singleton, or other shared object" form — mirror that wording in Requirements.
- 🔵 **Clarity**: "see plumbing decision above" in Technical Notes → Sidebar input host is a softer cross-reference than the rest of the document. Replace with an explicit anchor like "see the `/`-keybind listener plumbing decision earlier in Frontend wiring".
- 🔵 **Testability**: Cache-liveness window for the debounce dedup example lives in Technical Notes but not in the AC body — a verifier reading only the ACs cannot reproduce deterministically. Suggest appending "(within ~1 second of the first response, well inside React Query's default `gcTime`)" to the AC example.
- 🔵 **Testability**: "exactly the response rows in response-array order" lacks an equality predicate (Glyph element? doc-type label text? `href`?). Recommend restating as a structural assertion against `<a href>` values or pinning the doc-type label source.
- 🔵 **Testability**: Templates-exclusion AC's Given clause ("indexer contains entries across LIBRARY doc types plus Templates") cannot be produced by production code paths because Technical Notes says Templates never enter `entries`. Either reframe against the `Indexer::all()` snapshot output or drop the Templates-entry precondition.
- 🔵 **Testability**: `console.error` failure assertion in the error AC doesn't pin a verification approach (spy + call-args check vs partial match). Optionally append a concrete verification signal.
- 🔵 **Testability**: Prototype source is unreadable (`src/search.jsx` does not exist, standalone HTML doesn't embed it). Since the four-bucket ranking is fully pinned in the ACs, this is largely mitigated, but worth adding "ACs are authoritative where they differ from observed prototype behaviour" as a tiebreaker note.

### Assessment

The work item is ready for implementation. All three majors and all ten minors from Pass 1 are resolved; the nine new minor findings are diminishing-returns polish — a typo, two cross-reference cleanups, and a cluster of testability suggestions that map naturally to test-design choices during implementation (cache-window timing, `<a href>` equality predicate, `console.error` spy shape). None are structural; none block planning or implementation. Recommend treating the work item as approve-ready and absorbing the remaining items into implementation review.

## Pass 3 — 2026-06-01T13:11:07+00:00 — APPROVE

All 8 Pass-2 polish items have been addressed in the work item.

**Clarity**:
- ✅ Path forms standardised to `frontend/src/Sidebar/Sidebar.tsx` throughout
- ✅ "no new context" → "no new React Context, no module-level singleton"
- ✅ "see plumbing decision above" → explicit anchor to the `/`-keybind listener plumbing decision

**Testability**:
- ✅ Cache-liveness window lifted into the debounce AC body ("within ~1 second of the first response, well inside React Query's default `gcTime` of 5 minutes")
- ✅ Non-empty response AC now asserts `href` equality structurally — "exactly `results.length` link elements whose `href` attributes equal the per-row `/library/<docType>/<slug>` values in response-array order"
- ✅ Templates-exclusion AC reframed against the `Indexer::all()` snapshot output with regression-guard rationale
- ✅ Error AC pins verification approach: `vi.spyOn(console, 'error')` + `err instanceof FetchError && err.message.includes('/api/search')`
- ✅ Prototype reference now states "ACs and Requirements are authoritative" as a tiebreaker

No outstanding findings. Work item approved for implementation.
