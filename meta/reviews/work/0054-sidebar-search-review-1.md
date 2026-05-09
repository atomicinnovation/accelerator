---
date: "2026-05-11T20:20:02+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0054-sidebar-search.md"
work_item_id: "0054"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
---

## Work Item Review: Sidebar Search Input and API Search Endpoint

**Verdict:** COMMENT

The work item is in strong shape: completeness is clean with no findings, scope is well-justified as a vertical slice, and acceptance criteria use solid Given/When/Then framing. No critical or major issues were raised — the verdict is COMMENT because eleven minor findings cluster around three closely related themes (the "settled query" definition, the no-match empty state signal, and the unspecified result ordering) that would each be a small polish before implementation.

Work item is acceptable but could be improved — see minor findings below.

### Cross-Cutting Themes

- **"Settled query" / debounce semantics underdefined** (flagged by: clarity, testability) — Both lenses flagged that the third acceptance criterion uses undefined terms ("settled query", "keystroke run") and that the behaviour on query revisit (type `ab` → backspace to `a` → retype `ab`) is ambiguous.
- **No-match empty state lacks a queryable signal** (flagged by: testability; partially clarity) — The criterion requires a "designated no-match empty state" but pins neither text nor role nor test id, so a verifier cannot distinguish it from an empty panel.
- **Ranking/ordering is deferred but downstream gating is missing** (flagged by: dependency, testability) — Result ordering is committed to in Technical Notes (`mtime_ms` desc) but neither lifted into an acceptance criterion nor tracked as a follow-up.

### Findings

#### Minor

- 🔵 **Clarity**: 'Twelve LIBRARY doc types' is referenced but never enumerated
  **Location**: Summary / Requirements
  Both sections refer to "the twelve LIBRARY doc types" as canonical but never list them; the closest hint is the parenthetical "every `DocTypeKey` variant except `Templates`", which defines the set by exclusion against an external enum.

- 🔵 **Clarity**: 'The user' role shifts between end-user and implementer
  **Location**: Summary
  The Summary opens with a user-story framing but the second paragraph switches to implementer voice ("Add a new server endpoint…", "wire the Sidebar search input slot…") within the same section.

- 🔵 **Clarity**: 'Settled query' is used without prior definition
  **Location**: Acceptance Criteria (third bullet)
  "Settled query" and "keystroke run" are introduced for the first time in the third criterion and never defined; a reader cannot tell whether this means "after the 200 ms debounce", "after typing stops", or "deduped by query-key cache".

- 🔵 **Clarity**: 'Or inline the same pattern' leaves the helper's existence ambiguous
  **Location**: Technical Notes (Frontend wiring)
  The debounce bullet offers two structurally different outcomes (a named `useDebouncedValue` helper vs inline code) without indicating which is required, and the acceptance criteria do not constrain the choice.

- 🔵 **Clarity**: 'Suppressed' versus 'focus does not change' uses different vocabulary for the same condition
  **Location**: Requirements (second bullet) / Acceptance Criteria (first bullet)
  Requirements describe the keybind as "suppressed" while editable elements have focus; Acceptance Criteria describe the same condition as "the `/` character is inserted normally … and Sidebar search focus does not change". A reader could interpret "suppressed" as "no listener attached" rather than "the listener early-returns".

- 🔵 **Dependency**: Net-new global `/` hotkey and debounce infrastructure has no captured downstream consumers
  **Location**: Dependencies
  Technical Notes flag the `/` keybind and `useDebouncedValue` as net-new reusable primitives ("first `useEffect` in `RootLayout`"), but Blocks is `none` and no downstream story is named.

- 🔵 **Dependency**: `DocTypeKey` enum coupling to the server crate is not captured as a dependency
  **Location**: Requirements
  The response shape pins `docType: DocTypeKey` and the Templates exclusion is defined against the enum, coupling the contract to a type whose source module is not named in Dependencies.

- 🔵 **Testability**: 100 ms render window has no defined measurement endpoints
  **Location**: Acceptance Criteria
  The fourth criterion requires rendering "within 100 ms" of response arrival but does not define when the timer starts (fetch resolution, React state update, paint) or how to measure it, risking flaky verification.

- 🔵 **Testability**: Result ordering not specified, so multi-row tests cannot assert sequence
  **Location**: Acceptance Criteria
  Technical Notes commit to `mtime_ms` desc ordering but this is not lifted into an acceptance criterion; multi-row tests cannot assert sequence, weakening regression coverage.

- 🔵 **Testability**: No-match empty state content is not pinned
  **Location**: Acceptance Criteria (no-match empty state)
  The seventh criterion requires "a no-match empty state message" but does not specify text, role, or test id — a verifier cannot distinguish "empty-state rendered" from "nothing rendered".

- 🔵 **Testability**: 'Exactly once per settled query' is ambiguous on query revisit
  **Location**: Acceptance Criteria (debounce criterion)
  Typing `ab` → deleting to `a` → retyping `ab` could be one request (cache key shared) or two (each settled state independent); the criterion does not disambiguate.

#### Suggestions

- 🔵 **Dependency**: Open ranking-strategy question has no captured downstream gating
  **Location**: Open Questions
  Ranking is deferred and `body_preview` matching is out of scope at first cut, but no follow-up story is named — the deferred ranking work risks being orphaned.

- 🔵 **Scope**: Net-new frontend infrastructure (debounce + global hotkey) is bundled with feature delivery
  **Location**: Requirements
  Co-locating both primitives is reasonable given their declared size (~10 LOC each, single call-site each). Consider noting that if either grows a second consumer during implementation, the generalisation should be split as a follow-up chore rather than absorbed here.

- 🔵 **Testability**: No backend-only acceptance criterion for the `/api/search` route
  **Location**: Acceptance Criteria (server endpoint)
  All seven criteria are browser-facing; the server route's contract (Templates exclusion, missing/empty `q` behaviour) has no dedicated criterion, so a backend integration test must infer from Technical Notes.

### Strengths

- ✅ Acceptance criteria use Given/When/Then with named actors and concrete observable outcomes (focus moves, exactly-once requests, results within 100 ms).
- ✅ API contract pinned in both Requirements and Acceptance Criteria — response shape is concrete enough for a contract test.
- ✅ Keybind suppression is split into two complementary criteria (focused-input vs not-focused), eliminating the common global-hotkey ambiguity.
- ✅ Sub-2-character behaviour is enumerated as an explicit negative criterion (no request, no results), preventing silent under-specification.
- ✅ Technical Notes disambiguate net-new infrastructure (debounce, hotkey) from precedented patterns, and explicitly note the pre-existing `LifecycleIndex` search is unrelated.
- ✅ Open Questions narrow what is still open ("only the internal ranking algorithm remains open"), preventing readers from assuming endpoint/shape are also unresolved.
- ✅ Vertical-slice scoping is appropriate: search needs both endpoint and consumer; the bundling decision is pre-empted explicitly in Assumptions.
- ✅ Epic decomposition into 0053 (layout), 0054 (search), 0055 (activity) cleanly separates by feature; sibling boundaries named in Context.
- ✅ Frontmatter is fully populated with recognised type/status, parent reference, tags, author, and timestamp.

### Recommended Changes

1. **Define "settled query" and resolve the revisit ambiguity** (addresses: 'Settled query' is used without prior definition; 'Exactly once per settled query' is ambiguous on query revisit)
   Replace "settled query" in the third Acceptance Criterion with an explicit definition such as "the query value present 200 ms after the most recent keystroke". Add a concrete example clarifying revisit behaviour, e.g. "typing `ab` → deleting to `a` → retyping `ab` within the same session results in exactly one `/api/search?q=ab` request because the React Query cache key is shared" (or the opposite — pick the design intent).

2. **Pin a queryable signal for the no-match empty state** (addresses: No-match empty state content is not pinned)
   Specify either a role + text combination (e.g. `role="status"` containing "No matches") or a stable test identifier (e.g. `data-testid="search-empty"`) so the seventh criterion can be asserted unambiguously.

3. **Lift result ordering into an acceptance criterion or mark it explicitly out of scope** (addresses: Result ordering not specified)
   Either add "Given matches exist across multiple doc types, when results render, then rows are ordered by `mtime_ms` descending" or state explicitly that ordering is out of scope and defer to the open-question follow-up.

4. **Define how the 100 ms render budget is measured — or drop the threshold** (addresses: 100 ms render window has no defined measurement endpoints)
   Specify measurement endpoints (e.g. "from fetch promise resolution to the results panel being present in the DOM, asserted via `findByRole`") or replace the numeric bound with an ordering assertion ("results appear after response arrival and before further user input is processed").

5. **Add 1–2 backend-only acceptance criteria** (addresses: No backend-only acceptance criterion for `/api/search`)
   Cover Templates exclusion against a fixture and behaviour on missing/empty `q`, e.g. "Given the indexer contains entries across LIBRARY doc types plus Templates, when `GET /api/search?q=<title-substring>` is called, then the response contains zero entries with `docType: Templates`" and "Given `q` is absent or empty, when the route is called, then the response is `{ results: [] }`" (or a defined 4xx).

6. **Enumerate or link the twelve LIBRARY doc types** (addresses: 'Twelve LIBRARY doc types' is referenced but never enumerated)
   Add a file:line reference to `DocTypeKey`'s definition in Context or Technical Notes, or list the twelve variants once so the referent is closed within the work item.

7. **Pick one shape for the debounce helper** (addresses: 'Or inline the same pattern' leaves the helper's existence ambiguous)
   Replace "co-located with `useSearch`, or inline the same pattern" in Technical Notes with a single committed shape (e.g. "introduce `useDebouncedValue` as a named helper") so reviewers know what to expect.

8. **Align "suppressed" wording with Acceptance Criteria** (addresses: 'Suppressed' versus 'focus does not change' uses different vocabulary)
   Rephrase the Requirements bullet as "the activator early-returns (does not steal focus or call `preventDefault`) while a text input, textarea, or contenteditable element has focus" to match the AC vocabulary.

9. **Split the Summary into user-story and scope paragraphs** (addresses: 'The user' role shifts between end-user and implementer)
   Either keep paragraph 1 in user-story voice and rewrite paragraph 2 in third person ("This story adds a new server endpoint…"), or label them as two distinct sub-sections.

10. **Note `DocTypeKey` coupling and rank-question follow-up in Dependencies** (addresses: `DocTypeKey` enum coupling not captured; Open ranking-strategy question has no captured downstream gating)
    Add a one-line note in Dependencies (or Technical Notes' Server route section) acknowledging the contract is coupled to `DocTypeKey` and that any taxonomy change must revisit `/api/search`. If no follow-up story is planned for ranking refinement, state explicitly that no follow-up is currently tracked so the deferral is visible.

11. **Acknowledge net-new infrastructure follow-up posture in Dependencies/Technical Notes** (addresses: Net-new global `/` hotkey and debounce infrastructure has no captured downstream consumers; bundled infrastructure scope suggestion)
    Either name any backlog story that will consume the new hotkey/debounce primitives, or confirm Blocks: none is accurate, and note that if either grows a second consumer during implementation the generalisation should be split as a follow-up chore.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely unambiguous: actors are named, the API contract is pinned, and acceptance criteria use clear Given/When/Then phrasing. A few minor referent ambiguities remain (notably "the user" as both end-user and developer, "twelve LIBRARY doc types" used without enumeration, and the term "settled query" used without definition), but none are critical.

**Strengths**:
- Acceptance criteria use Given/When/Then with explicit actors and concrete observable outcomes.
- API contract is pinned with an explicit response shape.
- Technical Notes explicitly disambiguate net-new infrastructure versus precedented patterns and call out that the pre-existing `LifecycleIndex` search is unrelated.
- Open Questions explicitly narrow what is still open ("only the internal ranking algorithm remains open").
- Cross-references to sibling/parent stories (0036, 0053, 0055) and specific file:line locations make referents concrete.

**Findings**:
- 🔵 minor / high — "Twelve LIBRARY doc types" never enumerated (Summary / Requirements)
- 🔵 minor / medium — "The user" role shifts between end-user and implementer (Summary)
- 🔵 minor / medium — "Settled query" used without prior definition (Acceptance Criteria, third bullet)
- 🔵 minor / medium — "Or inline the same pattern" leaves helper's existence ambiguous (Technical Notes)
- 🔵 minor / low — "Suppressed" vs "focus does not change" uses different vocabulary (Requirements / Acceptance Criteria)

### Completeness

**Summary**: The work item is comprehensively populated against the completeness lens: all expected sections for a story are present and substantively filled, with a clear user-framed Summary, Context that explains both the parent relationship and current codebase state, detailed Requirements, seven Acceptance Criteria covering happy paths and edge cases, and rich Technical Notes. Frontmatter is fully populated with a recognised type and status. No notable gaps in section presence or content density were observed.

**Strengths**:
- Summary framed as unambiguous user story with a clear "so that" motivation.
- Seven specific Given/When/Then acceptance criteria covering activation, suppression, debouncing, rendering, navigation, minimum-length guard, and empty state.
- Context explains parent relationship and sibling boundaries and grounds the work in current codebase state.
- Type-appropriate content for a Story: clearly identified user and well-formed criteria.
- Frontmatter complete and consistent.
- Optional sections (Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes, References) substantively populated.

**Findings**: none.

### Dependency

**Summary**: The dependency map is well-formed for the most part: 0053 (input slot/layout) and 0037 (Glyph) are correctly captured as upstream blockers, and the parent epic 0036 and sibling 0055 are named in Context. However, the work item introduces a new server-side contract (the `/api/search` endpoint and its response shape including `DocTypeKey`) that is implicitly coupled to the Indexer module structure and `AppState.templates` store, and the relationship to 0055 is informational only. The most notable gap is that downstream consumers of the new `queryKeys.search`, `fetchSearch`, and `useSearch` primitives, and any future stories that depend on the `/` global hotkey infrastructure introduced here, are not named as Blocks.

**Strengths**:
- Upstream blockers 0037 and 0053 explicitly captured with the specific artefact each provides.
- Parent epic 0036 and siblings 0053/0055 named in Context.
- Assumptions explicitly state `/api/search` is implemented within this story.
- Technical Notes pin specific source locations making coupling points concrete.

**Findings**:
- 🔵 minor / medium — Net-new global `/` hotkey and debounce infrastructure has no captured downstream consumers (Dependencies)
- 🔵 minor / medium — `DocTypeKey` enum coupling to the server crate is not captured (Requirements)
- 🔵 suggestion / low — Open ranking-strategy question has no captured downstream gating (Open Questions)

### Scope

**Summary**: This story describes a single coherent vertical slice — search functionality — that necessarily spans a thin backend endpoint and the frontend wiring that consumes it. The bundling of server and client work is justified by the explicit assumption that the endpoint is not coordinated as parallel backend work, and the slice is sized M with small contained pieces. The epic decomposition (0053 layout, 0054 search, 0055 activity) appears clean and orthogonal along feature lines.

**Strengths**:
- Vertical-slice scoping is appropriate; splitting would create deploy-order coupling without standalone value.
- Assumptions section pre-empts the natural scope challenge by addressing the bundling decision.
- Epic-level decomposition cleanly separates by feature.
- Summary, Requirements, and Acceptance Criteria describe the same scope without drift.
- Open Questions explicitly bounded to ranking algorithm only.

**Findings**:
- 🔵 suggestion / medium — Net-new frontend infrastructure (debounce + global hotkey) is bundled with feature delivery (Requirements)

### Testability

**Summary**: The Acceptance Criteria are unusually well-structured for a frontend story: each criterion uses Given/When/Then framing, names precise inputs (the `/` key, the trimmed query length, the 200 ms debounce window), and specifies observable outcomes (focus shift, exactly one network call, inline rendering, no-match empty state). A few criteria leave verification thresholds soft (the 100 ms render window has no defined measurement point, and the search ranking/ordering and the no-match message text are not pinned), but most criteria are directly executable as integration tests.

**Strengths**:
- Every criterion uses explicit Given/When/Then framing with named preconditions and observable post-conditions.
- Keybind suppression split into two complementary criteria.
- Debounce contract is testable as a count assertion.
- Sub-2-character behaviour enumerated as a negative criterion.
- Response shape pinned in both Requirements and Acceptance Criteria.

**Findings**:
- 🔵 minor / high — 100 ms render window has no defined measurement endpoints (Acceptance Criteria)
- 🔵 minor / high — Result ordering not specified (Acceptance Criteria)
- 🔵 minor / medium — No-match empty state content is not pinned (Acceptance Criteria)
- 🔵 minor / medium — "Exactly once per settled query" is ambiguous on query revisit (Acceptance Criteria)
- 🔵 suggestion / medium — No backend-only acceptance criterion for `/api/search` (Acceptance Criteria)

## Re-Review (Pass 2) — 2026-05-11T20:36:46+00:00

**Verdict:** COMMENT

All eleven recommendations from Pass 1 have been addressed in the work item edits. Re-running clarity, dependency, scope, and testability surfaced one new major issue I introduced — `mtime_ms` is referenced in the new ordering criterion (AC5) but is not part of the pinned response shape — plus a cluster of new minor refinements (loading state, error handling, click target semantics, cache lifetime). No criticals; one major; many minor. Verdict remains COMMENT (one major is below the revise threshold of two).

### Previously Identified Issues

- ✅ **Clarity**: 'Twelve LIBRARY doc types' not enumerated — **Resolved** (Context now references `DocTypeKey` with file path)
- ✅ **Clarity**: 'The user' role shifts between end-user and implementer — **Resolved** (Summary split; second paragraph in third-person voice)
- ✅ **Clarity**: 'Settled query' used without definition — **Resolved** (defined inline in AC3 as "trimmed query value present 200 ms after the most recent keystroke")
- ✅ **Clarity**: 'Or inline the same pattern' ambiguity — **Resolved** (committed to named `useDebouncedValue` helper)
- ✅ **Clarity**: 'Suppressed' vs 'focus does not change' vocabulary mismatch — **Resolved** (Requirements wording aligned with AC)
- ✅ **Dependency**: Net-new hotkey/debounce infrastructure has no captured downstream consumers — **Resolved** (Dependencies explicitly notes no current consumers; Technical Notes adds "follow-up posture")
- ✅ **Dependency**: `DocTypeKey` enum coupling not captured — **Resolved** (Context + Dependencies both note the coupling)
- ✅ **Dependency**: Open ranking-strategy question has no captured downstream gating — **Resolved** (Dependencies explicitly states no follow-up is currently scheduled)
- ✅ **Scope**: Net-new infrastructure bundled with feature delivery — **Resolved** (Technical Notes "follow-up posture" bullet pre-empts scope creep)
- ✅ **Testability**: 100 ms render window has no defined measurement — **Resolved** (replaced with ordering assertion: "after response resolves and before further user input is processed")
- ✅ **Testability**: Result ordering not specified — **Resolved** (AC5 added pinning `mtime_ms` desc ordering — but see new issue below)
- ✅ **Testability**: No-match empty state content not pinned — **Resolved** (pinned to `role="status"` + literal text `No matches`)
- ✅ **Testability**: 'Exactly once per settled query' ambiguous on revisit — **Resolved** (concrete `ab` → `a` → `ab` example added with cache-key explanation)
- ✅ **Testability**: No backend-only AC for `/api/search` — **Resolved** (AC8 covers Templates exclusion; AC9 covers absent/empty `q`)

### New Issues Introduced

#### Major
- 🟡 **Clarity**: `mtime_ms` introduced in AC5 ordering rule but not present in the response shape declared in Requirements and AC4 — implementer cannot tell whether to extend the response, sort server-side and discard, or surface it. Same point also flagged independently by testability ("AC5 ordering depends on `mtime_ms` but response shape does not include it"). **This issue was introduced by Pass-1 recommendation #3 (lift ordering into an AC) and should be resolved before implementation.**

#### Minor (from new edits)
- 🔵 **Clarity**: "any stable order" in AC5 tie-break clause is ambiguous (stable w.r.t. what input order?). Suggest "any order" or anchor to `Indexer::all()` iteration order.
- 🔵 **Clarity**: AC3 dedup behaviour ("within the same session") silently depends on React Query `staleTime` policy, which is not pinned.
- 🔵 **Clarity**: Requirements says `useSearch(query)` is "keyed by query string" — ambiguous whether the cache key is the raw or settled query.
- 🔵 **Testability**: AC7 asserts the empty state is "distinct from a loading state" but no AC defines the loading-state output.
- 🔵 **Testability**: No AC covers `/api/search` 4xx/5xx or network-error UX (Technical Notes mention `FetchError` but no observable outcome is pinned).
- 🔵 **Testability**: AC3 trailing-edge debounce semantics on rapid alternations (e.g. `ab` → `abc` → `ab` within 200 ms) underspecified.
- 🔵 **Testability**: AC6 click target underspecified — URL pattern, modifier-click / middle-click, keyboard-Enter, `<a href>` element semantics not pinned.

#### Minor (pre-existing but newly raised)
- 🔵 **Clarity**: Sidebar described as "purely presentational today" while also being given "its first local state"; `searchInputRef` ownership offered as "RootLayout prop or SearchContext" without decision — pick one or mark deferred.
- 🔵 **Dependency**: Parent epic 0036 captured in frontmatter and Context but not surfaced as a "Part of" entry in Dependencies.
- 🔵 **Dependency**: Sibling 0055 modifies the same Sidebar surface; no coordination/ordering note in Dependencies.
- 🔵 **Scope**: Backend + frontend bundling is defensible as a vertical slice but could be split into 0054a/0054b — flagged as a sizing judgement call, not a structural problem.
- 🔵 **Testability** (suggestion): `/` hint chip dynamic behaviour during interaction not covered — clarify ownership (0053 only) or add an AC.

### Assessment

The work item is substantially improved by the Pass-1 edits — all fourteen prior findings are resolved cleanly. **One real new major issue was introduced by lifting the ordering rule into AC5**: `mtime_ms` needs to either be added to the response shape (in Requirements and AC4) or AC5 needs to explicitly state that ordering is verified server-side over a non-returned internal field. Recommend addressing the `mtime_ms` shape consistency before implementation; the remaining minor refinements are polish that can land alongside implementation review.

## Re-Review (Pass 3) — 2026-05-11T20:50:10+00:00

**Verdict:** COMMENT

The Pass-2 major (`mtime_ms` response-shape inconsistency) is resolved. All Pass-2 new minors are also resolved, with two partial resolutions that pass-3 lenses re-raise from a different angle (the AC5 tie-break is now "any order" rather than ambiguous-stable but is now tautological; AC6 click target pins `<a href>` and modifier-click semantics but does not pin the URL pattern itself). No criticals, no majors. Remaining findings are diminishing-returns polish — recommend treating the work item as ready for implementation.

### Previously Identified Issues

- ✅ **Major / Clarity+Testability**: `mtime_ms` in AC5 ordering but not in response shape — **Resolved** (added `mtime_ms: number` to response shape in Requirements, AC4, AC5)
- ✅ **Clarity**: AC5 "any stable order" tie-break — **Partially resolved** (now "any order"; pass-3 testability flags this is tautological — see new findings)
- ✅ **Clarity**: AC3 cache lifetime / staleTime unpinned — **Resolved** ("while the React Query cache entry for `queryKeys.search('ab')` is still live")
- ✅ **Clarity**: "keyed by query string" raw-vs-settled ambiguity — **Resolved** (explicit "cache key is always the settled value, not raw")
- ✅ **Testability**: AC7 loading-state distinctness undefined — **Resolved** (dedicated in-flight AC added)
- ✅ **Testability**: No error / network failure AC — **Resolved** (new error AC); **partially**: pass-3 flags console.error payload shape not pinned
- ✅ **Testability**: AC3 debounce edge cases — **Resolved** (trailing-edge + `ab`→`abc`→`ab` example)
- ✅ **Testability**: AC6 click target / URL pattern — **Partially resolved**: `<a href>`, keyboard Enter, modifier-click pinned; URL template itself still not specified (re-flagged)
- ✅ **Clarity**: Sidebar state ownership ambiguity — **Resolved** (Sidebar owns query state + results panel; RootLayout owns ref + listener)
- ✅ **Dependency**: Parent 0036 not in Dependencies — **Resolved** ("Part of: 0036" entry added)
- ✅ **Dependency**: Sibling 0055 coordination not in Dependencies — **Resolved** ("Coordinates with: 0055" entry added)
- ✅ **Testability**: `/` hint chip dynamic behaviour — **Resolved** (chip noted as owned by 0053; this story does not modify it)

### New Issues Introduced

#### Minor (introduced by Pass-2 edits)
- 🔵 **Clarity**: "The chip" / "it is purely informational visual state" — pronoun antecedent ambiguous in the new hint-chip bullet.
- 🔵 **Clarity**: "before any further user input is processed" in AC4 is operationally vague (introduced when 100 ms threshold was replaced with ordering assertion).
- 🔵 **Testability**: AC5 tie-break "any order" is tautological — tests cannot detect non-deterministic ordering regressions. Consider deterministic secondary sort (e.g., `path` ascending) or forbid equal-`mtime_ms` fixtures.
- 🔵 **Testability**: Error AC's `console.error` requirement is unverifiable without a pinned payload shape.

#### Minor (pre-existing, raised in Pass 3)
- 🔵 **Clarity**: "activator" / "global keydown listener" / "the listener" used interchangeably without a canonical name.
- 🔵 **Clarity**: "LOC" acronym in Size note used without expansion.
- 🔵 **Clarity**: Context paragraph 3 closes with "is unrelated" without naming what it is unrelated to.
- 🔵 **Dependency**: `DocTypeKey` type-bridge regeneration (Rust → TypeScript) not explicitly captured as part of the implementation flow.
- 🔵 **Dependency**: Indexer availability prerequisite (`state.indexer.all()`) not named in Assumptions.
- 🔵 **Testability**: Templates-exclusion AC presupposes a fixture state without naming it — add a concrete worked example.
- 🔵 **Testability**: Initial matching algorithm (case-insensitive substring on title+slug) lives in Technical Notes only; positive-match ACs depend on it being lifted to a criterion.
- 🔵 **Testability**: AC6 does not pin the document-detail URL template, so `href` cannot be asserted as a literal.
- 🔵 **Testability**: `/` keybind modifier-key suppression (Cmd/Ctrl/Alt/Meta + `/`) is in Technical Notes only; no AC asserts it.

#### Suggestion
- 🔵 **Dependency**: Deferred ranking algorithm has no formal downstream tracker (a placeholder follow-up spike could be created), though the explicit "no follow-up currently scheduled" wording is acceptable if the team has a routine to revisit closed stories' open questions.

### Assessment

The work item is converging — Pass-2's single major is fixed, every Pass-2 new minor is at least partially resolved, and the remaining findings are incremental polish (naming consistency, vague phrasings, AC-level vs Technical-Notes placement of constraints). No critical or major issues remain. The work item is ready for implementation; the remaining minor items can either be tightened during implementation review or absorbed into the test design where they overlap with TDD fixture decisions (matching algorithm, URL template, fixture shape, console.error payload).
