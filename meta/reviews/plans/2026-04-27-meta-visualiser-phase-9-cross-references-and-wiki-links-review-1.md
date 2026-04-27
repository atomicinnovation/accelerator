---
date: "2026-04-28T15:00:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-27-meta-visualiser-phase-9-cross-references-and-wiki-links.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, security, standards, usability]
review_pass: 4
status: complete
---

## Plan Review: Phase 9 — Cross-references and wiki-links

**Verdict:** REVISE

The plan is methodical, follows the established Phase 8 TDD template, and re-uses existing indexer/secondary-map and route-shape conventions cleanly. However, three distinct critical issues — a self-recursive callback in `LibraryDocView`, a write/read canonicalisation mismatch in the reverse index that defeats the deferred-materialisation behaviour the plan explicitly tests, and a missing previous-target read in `refresh_one` that leaves phantom inbound entries on target migration — would all ship broken if implemented as sketched. Several majors also cluster around silent contract gaps (missing `serde(rename_all)`, unbounded regex, undeduped `Vec`, untested error/loading paths) that are cheap to fix at the plan stage but expensive once the test sequence is locked.

### Cross-Cutting Themes

- **Name shadowing in `LibraryDocView` resolver wiring** (flagged by: architecture, code-quality, correctness, standards, test-coverage, usability) — Six of seven lenses independently caught the `const resolveWikiLink = useCallback(...)` shadowing the imported `resolveWikiLink`. The closure body recurses into itself and stack-overflows on the first wiki-link render. None of the proposed Phase 6 tests would catch this because they inject a stub resolver via props rather than exercising the real wiring.
- **Reverse-index write/read canonicalisation drift** (flagged by: architecture, code-quality, correctness) — `target_path_from_entry` stores the canonicalised path when the target file exists and the raw `project_root.join(raw)` when it doesn't (Step 1.4's "deferred materialisation"). The lookup sketch is a bare `map.get(target)` keyed on raw bytes, even though Step 1.2 claims canonicalisation happens "at read time". The two halves only agree when no symlinks are involved, which silently fails on macOS dev environments (`/var` ↔ `/private/var`).
- **Schema/contract gaps that only one test catches** (flagged by: correctness, standards) — Phase 2's response struct is sketched without `#[serde(rename_all = "camelCase")]`, the only safety net being Step 2.10. The plan should make the convention explicit alongside the handler so implementers don't have to reverse-engineer it from a behavioural test.
- **Frontmatter `target:` is untrusted but unsanitised** (flagged by: security, test-coverage) — `entry.frontmatter.get("target")?.as_str()?` accepts any string, including absolute paths and `..`-laden relative paths, and there is no test for malformed values (number, null, empty, escape).
- **Frontend error/loading state for `fetchRelated`** (flagged by: test-coverage, usability) — The Phase 6 sketch renders `Loading…` whenever `data` is undefined, conflating in-flight with errored. The other sections of `LibraryDocView` already have a `role="alert"` error pattern that this section abandons.
- **`Vec<PathBuf>` ordering + dedup contract** (flagged by: code-quality, correctness) — Step 1.5 asserts "in path order" but the implementation pushes in iteration order; on repeated `refresh_one` calls duplicates accumulate. `BTreeSet<PathBuf>` would solve both problems.
- **Unbounded `\d+` in `WIKI_LINK_PATTERN`** (flagged by: correctness, security) — Pathologically long digit runs scan freely; `parseInt` overflow opens a Map-key collision space (low practical risk, trivial fix).

### Tradeoff Analysis

- **SSE invalidation scope (architecture vs correctness)** — Architecture accepts the coarse `relatedPrefix()` invalidation as cost-effective at v1 scale; Correctness flags that, because TanStack Query keeps unmounted query data fresh by default, navigating to a previously-cached related page after a delete still serves stale data until the next event. Recommendation: accept the coarse policy but add `staleTime: 0` (or `refetchType: 'all'`) so re-mounting always revalidates.
- **Custom hook vs inline `useQuery` (standards vs code-quality)** — Standards notes that `useRelated` departs from the existing inline-`useQuery` convention used throughout the routes layer; Code Quality and Architecture appreciate the separation of concerns the hook provides. Pick one and document the choice; the cheaper option is to inline since there's only one consumer today.

### Findings

#### Critical

- 🔴 **Architecture / Code Quality / Correctness / Standards / Test Coverage / Usability**: `resolveWikiLink` shadows itself, creating infinite recursion
  **Location**: Phase 6 §2(a) "Pull the ADR + ticket lists into the resolver"
  The `useCallback` binds a local `resolveWikiLink` whose body calls `resolveWikiLink(prefix, n, wikiIndex)` — that's the closure itself, not the imported pure function. Stack overflow on first render. None of the Phase 6 tests catch it (they inject a stub resolver via props). Fix: rename one identifier (e.g. `import { resolveWikiLink as resolveWikiLinkPure }`) and add a Phase 6 test that exercises the real wiring through a `[[ADR-0001]]` body.

- 🔴 **Correctness / Architecture / Code Quality**: Reverse-index lookup never canonicalises the query side, defeating deferred materialisation
  **Location**: Phase 1 — `target_path_from_entry` and `Indexer::reviews_by_target`
  Write side stores canonical-when-it-exists / raw-`project_root.join(raw)`-otherwise. Read side does `map.get(target)` with no canonicalisation, even though Step 1.2's narrative claims "both are canonicalized at read time." Step 2.11's "recovers after target creation" test will silently coincidence-pass (no symlinks in tempfile fixtures) but real macOS dev paths (`/var` ↔ `/private/var`) will permanently miss. Pick one normalisation: lexically-clean `project_root.join(raw)` on both sides, with `canonicalize` reserved for the entry primary key. Make the secondary-index key derive from `entry.rel_path` rather than `entry.path`.

- 🔴 **Correctness**: `refresh_one` does not read the previous target before overwriting, leaving phantom inbound entries
  **Location**: Phase 1 TDD Step 1.7 "refresh_one_removes_review_from_reverse_index_on_target_change"
  The Step 1.7 narrative requires the migration to drop the stale key, but the sketched implementation only computes `target_path_from_entry(&new_entry, …)` on the freshly-built entry. There's no `let old = self.entries.read().get(&canonical).cloned();` step to capture the previous target before the new one overwrites. The test name is correct; the implementation sketch underspecifies how it would pass. Add explicit "read old entry → derive old target → remove from reverse map → insert new" sequence to the Phase 1 changes-required, mirroring how `find_entry_for_deleted` handles the deletion branch.

#### Major

- 🟡 **Correctness / Standards**: Response struct sketch omits `#[serde(rename_all = "camelCase")]`
  **Location**: Phase 2 §1 — `src/api/related.rs`
  Step 2.10 asserts camelCase keys behaviourally, but the plan never shows the Rust struct. Default serde produces snake_case. Make the struct explicit alongside the handler signature so the convention is visible at the same granularity as the test that locks it.

- 🟡 **Code Quality / Correctness**: `Vec<PathBuf>` is unsorted and not deduped
  **Location**: Phase 1 §1 (field type) + TDD Step 1.5
  Step 1.5 asserts "in path order" but iteration order through `entries.read()` is randomised. Repeated `refresh_one` on an unchanged review pushes duplicates. Use `HashMap<PathBuf, BTreeSet<PathBuf>>` (auto-sorted, auto-deduped) or a remove-then-insert helper, and add a "refresh same review twice keeps single entry" test.

- 🟡 **Security**: `target:` value is path-joined and the canonicalize-fallback bypasses path-escape protection
  **Location**: Phase 1 — `target_path_from_entry`
  `Path::join` discards `project_root` when `raw` is absolute; `..`-laden values bypass canonicalisation when the file doesn't exist. Apply the same per-segment validation that `doc_patch_frontmatter` applies (reject leading `/`, `..`, `.`, NUL, backslash) before joining, and verify the joined result starts with `project_root`.

- 🟡 **Test Coverage**: Malformed `target:` frontmatter values are not tested
  **Location**: Phase 1 TDD Sequence
  No test covers `target: 42`, `target: ""`, `target: null`, `target: ["a","b"]`, or `target: "../../etc/passwd"`. The path-escape case is the security-adjacent one — Phase 1 should pin "no panic, no entry added, no path-escape leakage" with explicit cases.

- 🟡 **Correctness**: Unbounded `\d+` in `WIKI_LINK_PATTERN`
  **Location**: Phase 3 §1 — `WIKI_LINK_PATTERN`
  `[[ADR-99999999999999999999]]` overflows `Number.MAX_SAFE_INTEGER` and could collide with another entry's key in the resolver Map. Bound to `\d{1,6}` (or `\d{1,8}`) — comfortably exceeds any realistic ID space. Add a regression test for the over-long case.

- 🟡 **Correctness**: SSE prefix-invalidation does not refetch unmounted queries
  **Location**: Phase 5 §5 — `use-doc-events.ts`
  `qc.invalidateQueries({ queryKey: queryKeys.relatedPrefix() })` marks active queries stale but does not invalidate cached-but-unmounted ones. Navigating to a target plan after deleting one of its reviews shows the deleted review until the next unrelated event. Use `refetchType: 'all'` or set `staleTime: 0` on `useRelated`.

- 🟡 **Correctness**: `buildWikiLinkIndex` ticket key derivation lacks radix and kind validation
  **Location**: Phase 3 §1 — `buildWikiLinkIndex`
  `parseInt('001abc-foo.md')` silently succeeds; if `ticketEntries` is misused (e.g., a date-prefixed plan slips through), `[[TICKET-2026]]` would mis-resolve. Pass radix `10` to `parseInt`, and either filter by `entry.type === 'tickets'` defensively or document the precondition.

- 🟡 **Test Coverage**: No concurrency test for `refresh_one` updating reverse index during `rescan`
  **Location**: Phase 1 TDD Sequence
  Existing `refresh_one_serialises_with_concurrent_rescan` covers primary index races; the new third secondary index has more involved invariants (target migration reads-then-writes) and deserves its own concurrent-target-migration test.

- 🟡 **Test Coverage / Usability**: Frontend error and loading paths for `fetchRelated` are untested and merged
  **Location**: Phase 5 + Phase 6 §c
  Sketched as `{related ? <RelatedArtifacts/> : <p>Loading…</p>}` — same UI for in-flight and errored. Existing `LibraryDocView` (lines 46–58) uses a `role="alert"` error pattern. Restore consistency: destructure `{ data, isError, error }`, render the error path, and add tests for both.

- 🟡 **Test Coverage**: Phase 7's "cross-cutting smoke" is one extra backend integration case, not a round-trip
  **Location**: Phase 7 — Fixtures + cross-cutting smoke test
  Steps 1.7 and 5.5 each prove half of the SSE round-trip in isolation; nothing in CI proves they compose. Either add a frontend integration test that mounts `LibraryDocView`, fires a synthetic `doc-changed` event, and asserts the second fetch landed, or be explicit that the round-trip is verified manually only.

- 🟡 **Test Coverage**: Cross-doc deletion invariant (delete a plan with inbound reviews) is untested
  **Location**: Phase 1 + Phase 2
  Plan deletion is foreseeable. After deletion, `reviews_by_target(&deleted_plan_path)` semantics are unverified, and `GET /api/related/<review>` for a review whose target was deleted should return `declaredOutbound: []` — also untested. Add at least one indexer test and one integration test pinning the chosen behaviour.

- 🟡 **Test Coverage**: Inferred ↔ declared overlap is unspecified and untested
  **Location**: Phase 2 + Phase 6
  A review whose target plan is also in its slug cluster would appear in both `inferredCluster` and `declaredOutbound`. The plan does not specify whether the response dedupes or whether the UI hides the overlap. Pick a contract; lock it with a test.

- 🟡 **Test Coverage**: `buildWikiLinkIndex` behaviour with duplicate IDs is unspecified
  **Location**: Phase 3 TDD Sequence
  Two ADRs claiming the same `adr_id` (real authoring mistake during renames) silently last-write-wins. Lock a deterministic tie-breaker (e.g. first-by-`relPath`) with a test.

- 🟡 **Security**: No explicit test that resolver-supplied URLs cannot smuggle dangerous schemes
  **Location**: Phase 4 — wiki-link-plugin and MarkdownRenderer integration
  The XSS regression test exercises markdown source, not plugin-emitted Link nodes. Add a unit test where the resolver returns `'javascript:alert(1)'` and assert the rendered anchor either has no `href` or has a sanitised one — locks in react-markdown's `urlTransform` for plugin-emitted nodes.

- 🟡 **Usability**: Wiki-link syntax fails silently for common typos
  **Location**: Phase 3/4 — silent rejection of `[[0001]]`, `[[adr-0017]]`, `[[EPIC-0001]]`
  Authors get zero feedback when their refs don't render as links. Add at least a dev-mode `console.warn` for `[[…]]`-shaped strings that match the bracket pattern but fail resolution; or document syntax rules in a help popover.

- 🟡 **Usability**: Race between docs cache load and body render produces transient unresolved wiki-links
  **Location**: Phase 6 §a — wiki-link resolver wiring
  On cold navigation, `adrs`/`tickets` default to `[]` until two `useQuery`s resolve. `[[ADR-NNNN]]` renders as plain text on first paint, then flips to a link. Either gate the renderer on the docs queries settling or accept the flicker explicitly with a placeholder.

- 🟡 **Usability**: Empty-state copy "No related artifacts yet." is misleading when only one half is empty
  **Location**: Phase 6 §1 `RelatedArtifacts` component
  Show only when all three arrays empty; otherwise readers can't tell "this kind doesn't apply" from "this kind hasn't been populated yet". Either render every group always with per-group empty messages, or rephrase to drop the "yet".

#### Minor

- 🔵 **Architecture**: Kind whitelist hardcoded in `target_path_from_entry` couples reverse index to two doc types
  **Location**: Phase 1 §1
  Adding a future declared field will require parallel reverse maps and parallel cleanup paths. Consider a small registry of `(kind, field) → key extractor` rules, even just as a ticketed follow-up.

- 🔵 **Architecture**: Two consistency horizons feed one response (clusters debounced, secondary maps synchronous)
  **Location**: Phase 2 §1 — handler computation
  Inferred and declared groupings can disagree by one debounce window. Either derive `inferredCluster` from a synchronous `Indexer::cluster_for(slug)` accessor, or document the asymmetry in "What We're NOT Doing".

- 🔵 **Architecture**: SSE prefix-invalidation works today but couples Phase 9 to global event volume
  **Location**: Phase 5 §5 — invalidation policy
  Add a comment in `use-doc-events.ts` flagging the upgrade path so the coarse policy is recognised as deliberate.

- 🔵 **Architecture / Code Quality**: `LibraryDocView` accumulates four orthogonal responsibilities
  **Location**: Phase 6 §2 — LibraryDocView wiring
  Doc fetch + ADR/ticket fetches + related fetch + frontmatter chips all in one component. Consider extracting `useWikiLinkResolver()` and/or `useDocPageData(relPath)` hooks.

- 🔵 **Code Quality**: Three secondary-index update sites invite the next add to be missed
  **Location**: Phase 1 §1 — rescan / refresh_one / deletion paths
  Extract a small `update_secondary_indexes(&self, entry)` / `remove_secondary_indexes(&self, entry)` pair so adding a fourth map only changes one place.

- 🔵 **Code Quality**: Outbound resolution lives in handler instead of indexer
  **Location**: Phase 2 §1 step 4
  `Indexer::declared_outbound(&self, entry) -> Vec<IndexEntry>` would consolidate the type-switch logic next to the secondary-index field rather than in the API module.

- 🔵 **Code Quality**: Sentinel `'__off__'` query key is a third unexplained occurrence
  **Location**: Phase 5 §4 — `use-related.ts`
  Pull into a shared `disabledQueryKey(prefix)` helper or document the idiom once.

- 🔵 **Code Quality**: `splitTextNode` returns `Array | null` overloads "no match" with "no replacement"
  **Location**: Phase 4 §1 — wiki-link-plugin
  Future "render unresolved refs differently" will need to unwind this. Consider `{ replaced: boolean; nodes }` shape.

- 🔵 **Code Quality**: Prefix-invalidate intent should be documented at the call site
  **Location**: Phase 5 §5
  Add a 3–4 line comment in `use-doc-events.ts` explaining why prefix-invalidate is correct (transitive membership) and cheap (one mounted query at a time).

- 🔵 **Test Coverage**: Wiki-link plugin not tested against fenced code blocks
  **Location**: Phase 4 TDD Step 4.5
  Step 4.5 covers inline code only. Add a one-line case for triple-backtick blocks.

- 🔵 **Test Coverage**: `WIKI_LINK_PATTERN` boundary cases (trailing punctuation, empty digits, trailing non-digit) untested
  **Location**: Phase 3 TDD Sequence
  Cheap parametrised test enumerating boundary inputs.

- 🔵 **Test Coverage**: SSE invalidation test does not verify scope (no negative case)
  **Location**: Phase 5 Step 5.5
  Tighten by asserting which prefixes are not invalidated, and add a counter-test that an unrelated event kind does not trigger.

- 🔵 **Test Coverage**: "in path order" assertion has no canonical reference
  **Location**: Phase 1 Step 1.5
  Sort explicitly in the implementation snippet and rename the test (`..._sorted_by_path`).

- 🔵 **Test Coverage**: Testing-Strategy counts disagree with TDD step counts
  **Location**: Testing Strategy section
  E.g., "five cases" for wiki-link-plugin but Phase 4 lists nine. Align or drop the numeric claims.

- 🔵 **Correctness**: `visit()` SKIP semantics undocumented
  **Location**: Phase 4 §1 — wiki-link-plugin
  Add a comment explaining why inserted nodes are intentionally not re-visited; add a regression test for double-rewrite.

- 🔵 **Correctness / Standards**: `target_path_from_entry` admits `PrReviews` despite scope
  **Location**: Phase 1 §1
  Either restrict to `PlanReviews` (matching scope) or extend tests + fixtures so the broader admission is intentional.

- 🔵 **Correctness**: Per-segment `encodeURIComponent` round-trip not tested for `%`/`#` filenames
  **Location**: Phase 5 §2 — `fetchRelated`
  Add a fetch.test case or document the input constraint.

- 🔵 **Security**: Client/server URL-encoding disagreement on `%2F` is not pinned by a test
  **Location**: Phase 2 + Phase 5
  Add a server-side test for `/api/related/foo%2F..%2Fbar` asserting the path-escape rejection runs after decoding.

- 🔵 **Security**: 404 response body echoes attacker-supplied path
  **Location**: Phase 2 Step 2.1
  On localhost benign; if logs are centrally aggregated, raw user input in error bodies is a hygiene issue. Pick a stance and document.

- 🔵 **Standards**: `useRelated` custom hook departs from inline `useQuery` pattern
  **Location**: Phase 5 §4
  Either inline (matches existing `LibraryDocView`) or document the divergence.

- 🔵 **Standards**: Sub-group heading levels for `RelatedArtifacts` unspecified
  **Location**: Phase 6 §1
  Specify `<h4>` (one level below the `<h3>Related artifacts</h3>` parent) and assert in a test.

- 🔵 **Standards**: CSS modifier names `.declared`/`.inferred` are state-only, not element-named
  **Location**: Phase 6 — CSS module
  Existing modules name the element (`.empty`, `.aside`). Consider `.groupDeclared`/`.groupInferred`.

- 🔵 **Usability**: "Same workflow" label is opaque for slug-cluster siblings
  **Location**: Phase 6 §1
  Prefer "Same lifecycle" (matches `LifecycleCluster` server-side terminology) or "Slug siblings" with an `<abbr>` explainer.

- 🔵 **Usability**: Declared/inferred distinction lacks tooltip or helper text
  **Location**: Phase 6 §1
  Border style + badge alone don't explain the categories. Add a brief `<dl>` legend or `title` attribute.

- 🔵 **Usability**: 1-second SSE update window has no UI hint
  **Location**: Phase 6 — Manual Verification §7
  Surface `isFetching` as a subtle "Updating…" hint on the related aside.

- 🔵 **Usability**: Loading-state phrasing inconsistent with body's bare `<p>`
  **Location**: Phase 6 §c
  `styles.empty` for a `Loading…` state is semantically odd. Introduce `styles.loading` or reuse the body's pattern.

- 🔵 **Usability**: Resolved link display preserves `[[ADR-0001]]` text
  **Location**: Phase 4 Step 4.2
  Unconventional vs Obsidian/Foam/MediaWiki precedent. Consider rendering the entry's title with a `title` attribute carrying the bracketed source form, or call out the deliberate choice in the spec rationale.

#### Suggestions

- 🔵 **Architecture**: Doc view becomes the join point for three independent concerns — extract hooks if scope allows.
- 🔵 **Code Quality**: Document prefix-invalidate intent in `use-doc-events.ts`.

### Strengths

- ✅ Reverse-target index follows established `adr_by_id`/`ticket_by_number` precedent: same lifecycle hooks, same locking discipline, same secondary-map shape — minimises new architectural surface.
- ✅ Clean separation of pure resolver (`wiki-links.ts`), AST plugin (`wiki-link-plugin.ts`), fetch hook, and presentation component — testable layers with explicit dependencies.
- ✅ New `/api/related/*path` route avoids overloading `/api/docs/*path`, sidestepping matchit's catch-all + suffix limitation while keeping the related-data response cohesive.
- ✅ Opt-in `resolveWikiLink` prop on `MarkdownRenderer` preserves back-compat for pre-Phase-9 callers.
- ✅ Bidirectional declared-link semantics materialise reverse data at the right architectural layer (the indexer) rather than recomputing per-request.
- ✅ Self-cause / SSE invalidation reuses existing `useDocEvents` infrastructure rather than introducing a new event kind.
- ✅ Step 4.9 retains the `<script>alert('xss')</script>` regression test with the plugin attached, ensuring the new remark plugin doesn't widen the parsing surface.
- ✅ Bare `[[NNNN]]` and unknown prefixes are intentionally unresolved — preserves the prefix namespace for future ID kinds.
- ✅ Phase 2 explicitly mirrors `doc_patch_frontmatter`'s per-segment validation and includes a path-escape integration test (Step 2.2).
- ✅ Each TDD step's assertion is tied to a specific behaviour (target migration, kind-restriction, deletion cleanup) rather than a coverage goal — proportional rigour.
- ✅ Self-exclusion in `inferredCluster` is a separate explicit step (Step 2.5), removing a common cluster off-by-one bug.

### Recommended Changes

Ordered by impact. Each addresses one or more findings.

1. **Fix the `LibraryDocView` resolver shadowing** (addresses: cross-cutting critical) — Rename either the import (`import { resolveWikiLink as resolveWikiLinkPure }`) or the local binding (`const resolver = useCallback(...)`). Add a Phase 6 test that exercises the real wiring, not a stub-via-prop, by rendering `[[ADR-0001]]` content with a populated `fetchDocs('decisions')` mock and asserting the rendered anchor's href.

2. **Fix the reverse-index canonicalisation contract** (addresses: critical "Reverse-index lookup never canonicalises…") — Define a single `normalize_target_key(raw, project_root)` helper that produces a stable, lexically-clean form (no filesystem call); use it on both write and read sides. Reserve `std::fs::canonicalize` for the entry primary key. Update Step 1.2 to assert against this helper rather than implying read-time canonicalisation.

3. **Specify the previous-target read in `refresh_one`** (addresses: critical "refresh_one does not read the previous target…") — Add an explicit "read existing entry → derive old target → remove from reverse map → insert new entry → derive new target → add to reverse map" sequence to Phase 1 §1's changes-required, plus an assertion in Step 1.7 that the migration is atomic (no transient state where both old and new keys hold the review).

4. **Show the Rust response struct with `#[serde(rename_all = "camelCase")]`** (addresses: serde camelCase major) — Make Phase 2 §1 list the struct definition next to the handler, mirroring `LifecycleCluster` precedent.

5. **Sanitise and validate `target:` frontmatter** (addresses: security path-escape major + test-coverage malformed-target major) — In `target_path_from_entry`, reject leading `/`, `..`, `.`, NUL, backslash before `Path::join`. Verify the joined result starts with `project_root` lexically. Add Phase 1 tests for `target: 42`, `target: ""`, `target: null`, `target: "../escape.md"`, `target: "/etc/passwd"`.

6. **Switch `Vec<PathBuf>` to `BTreeSet<PathBuf>` (or document remove-then-insert)** (addresses: ordering/dedup major) — Solves Step 1.5's ordering assertion and the duplicate-on-refresh hazard in one type change. Add an explicit "refresh same review twice keeps single entry" test.

7. **Bound `WIKI_LINK_PATTERN` digit count** (addresses: unbounded `\d+` major) — `\d{1,6}` (or `\d{1,8}`). Add a regression test for over-long input.

8. **Make SSE invalidation refetch unmounted queries** (addresses: SSE staleness major) — `qc.invalidateQueries({ queryKey: queryKeys.relatedPrefix(), refetchType: 'all' })` or `staleTime: 0` on `useRelated`. Document the chosen mechanism.

9. **Pin radix and entry-kind validation in `buildWikiLinkIndex`** (addresses: ticket parseInt major) — `parseInt(prefix, 10)`; filter by `entry.type === 'tickets'` defensively or document the precondition.

10. **Render `RelatedArtifacts` error path with `role="alert"`** (addresses: error/loading state major) — Match the existing `LibraryDocView` pattern. Destructure `{ data, isError, error }`. Add tests for both branches.

11. **Add the cross-doc deletion test, the inferred/declared overlap test, the duplicate-ID test, and the concurrent-target-migration test** (addresses: four test-coverage majors) — Cheap test-list additions that close real gaps without changing implementation scope.

12. **Add a wiki-link XSS test for plugin-emitted Link nodes** (addresses: security XSS major) — Resolver returns `javascript:alert(1)`; assert the rendered anchor is sanitised.

13. **Surface unresolved `[[…]]` to authors** (addresses: silent-typo usability major) — Dev-mode `console.warn` is the lowest-cost option; an "unresolved" badge on bracket-shaped strings that fail resolution is the higher-fidelity option.

14. **Decide the docs-cache-load race policy** (addresses: transient flicker usability major) — Either gate on settled queries or accept the flicker explicitly. Test the chosen branch.

15. **Rephrase the partial-empty state** (addresses: empty-state usability major) — Either drop "yet" from the all-empty copy, or render per-group empty messages.

16. **Specify heading levels and rename the inferred-group label** (addresses: standards heading + usability label minors) — `<h4>` for sub-groups; "Same lifecycle" or "Slug siblings" instead of "Same workflow".

17. **Decide on `useRelated` vs inline `useQuery`** (addresses: standards custom-hook minor) — Inline matches the existing pattern with one consumer; if the hook stays, document the new precedent.

18. **Align Testing-Strategy counts with TDD steps** (addresses: testing-strategy count minor) — Drop the numeric claims or update them.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan extends the existing indexer secondary-map pattern cleanly and keeps the related-artifacts data flow read-only and additive. Boundaries between server (reverse index, related endpoint), pure resolver, AST plugin, fetch hook, and presentation component are well-separated and follow Phase 8's conventions. However, two architectural concerns warrant attention: a path-canonicalisation inconsistency in the reverse-index design that risks deferred-materialisation correctness, and a name-shadowing/coupling issue in `LibraryDocView` where the resolver wiring conflates the join responsibility with cross-cutting query concerns.

**Strengths**: Reverse-target index follows established precedent; clean pure-vs-effect layering; `/api/related/*path` route avoids matchit limitations; opt-in `resolveWikiLink` preserves back-compat; bidirectional materialisation at the right layer; reuses `useDocEvents`; explicit tradeoff acknowledgement.

**Findings**: Canonicalisation inconsistency (major), name shadowing (major), kind whitelist coupling (minor), two consistency horizons (minor), coarse SSE invalidation (minor), doc view join-point accumulation (suggestion).

### Code Quality

**Summary**: Plan follows established codebase patterns well, but several concrete code smells appear in proposed snippets: `LibraryDocView` self-recursive useCallback, `Vec<PathBuf>` reverse map without dedup, fallback non-canonical key. Most issues are easy to fix at plan stage.

**Strengths**: Re-uses existing lifecycle hooks; pure logic separated from IO; opt-in renderer prop; sibling route choice; per-step behaviour-tied assertions.

**Findings**: Name shadowing (critical), dual-key invariant (major), Vec dedup (major), three-site duplication (minor), feature-envy in handler (minor), sentinel query key (minor), null-overload helper (minor), prefix-invalidate intent comment (suggestion).

### Test Coverage

**Summary**: Strong unit-level coverage of the new reverse index and resolver, ~50 tests across seven phases. Several real risk areas are under-tested: concurrency, cross-doc deletion invariants, error paths, dedup overlap, Phase 7's "smoke" not actually being end-to-end.

**Strengths**: TDD ordering; full lifecycle coverage of `reviews_by_target`; wire-format contract test (Step 2.10); deferred-materialisation lock (Step 2.11); case-sensitivity and precedence tests; XSS regression guard.

**Findings**: Concurrency (major), error/loading paths (major), Phase 7 smoke (major), cross-doc deletion (major), malformed target (major), inferred/declared overlap (major), duplicate IDs (major), fenced code blocks (minor), regex boundaries (minor), shadow not caught (minor), invalidation scope (minor), ordering assertion (minor), count mismatch (suggestion).

### Correctness

**Summary**: High-level structure is sensible but several correctness traps lurk in the reverse-index lifecycle and frontend wiring. Most damaging are the LibraryDocView shadowing, un-canonical lookup, missing previous-target read in refresh_one, and missing serde camelCase. None are unfixable.

**Strengths**: Reverse-index lifecycle modelled on existing precedent; graceful degradation; explicit self-exclusion test; deferred materialisation as documented contract.

**Findings**: Resolver shadowing (critical), reverse-index lookup canonicalisation (critical), refresh_one previous target (critical), serde rename_all (major), Vec ordering/dedup (major), unbounded \d+ (major), SSE staleness (major), parseInt radix + kind validation (major), URL encoding round-trip (minor), visit SKIP semantics (minor), PrReviews admitted (minor).

### Security

**Summary**: Localhost developer tool with read-only API, small residual surface. Main concrete gaps: target_path_from_entry canonicalize-fallback bypassing path-escape; no explicit test for resolver-supplied dangerous URLs.

**Strengths**: Mirrors doc_patch_frontmatter validation; read-only data flow; remark plugin scoped to text nodes; XSS regression test retained; bare/unknown prefixes intentionally unresolved.

**Findings**: target path-escape via frontmatter (major), no resolver-URL XSS test (major), client/server URL encoding disagreement (minor), unbounded regex ReDoS (minor), 404 echoes path (minor).

### Standards

**Summary**: Adheres closely to project conventions: route shape, error envelope, status codes, file naming. Gaps: missing serde rename_all on response struct, useRelated departs from inline useQuery, sub-group heading levels unspecified.

**Strengths**: Route choice consistent; error shape matches ApiError envelope; type reuse via IndexEntry; kebab-case + PascalCase folder conventions; query key shape mirrors existing; per-segment validation precedent.

**Findings**: Missing serde rename_all (major), useRelated divergence (minor), heading levels unspecified (minor), name shadowing (minor), PrReviews admitted (minor), CSS modifier names (suggestion).

### Usability

**Summary**: Solid plumbing but notable author/reader UX and DX gaps. Wiki-link syntax fails silently for several plausible cases; related-artifacts UI lacks affordances explaining declared/inferred; "Same workflow" label is opaque; partial-empty handling needs revisiting.

**Strengths**: Graceful degradation on unresolved refs; opt-in MarkdownRenderer prop; preserved empty-state copy; bidirectional SSE acknowledged; visual distinction not colour-only.

**Findings**: Silent typo failures (major), docs-cache load race (major), partial-empty state (major), error swallowed by Loading (major), "Same workflow" label (minor), declared/inferred distinction lacks affordance (minor), 1s SSE window no hint (minor), loading-state phrasing inconsistency (minor), shadow shadowing (minor), bracketed display text (minor).

---

## Re-Review (Pass 2) — 2026-04-27

**Verdict:** REVISE

### Previously Identified Issues

**Critical (prior pass)**
- 🔴 **`resolveWikiLink` self-recursion in `LibraryDocView`** — Resolved (new `useWikiLinkResolver` hook destructures the bound function; Step 6.7 exercises real wiring).
- 🔴 **Reverse-index canonicalisation drift** — Partially resolved (lexical-key contract introduced via `normalize_target_key`, but a new critical surfaced: canonical `entry.path` and lexical-joined `target:` only unify if `project_root` is canonicalised at indexer construction, which the plan does not specify).
- 🔴 **`refresh_one` missing previous-target read on migration** — Resolved (explicit Phase 1 §3 sequence + Step 1.7).

**Major (prior pass)**
- 🟡 serde camelCase, BTreeSet ordering/dedup, `target:` sanitisation, `WIKI_LINK_PATTERN` bound, `parseInt(_, 10)` + kind filter, cross-doc deletion test, overlap dedup, duplicate-ID tie-breaker, resolver-URL XSS test, silent-typo affordance, partial-empty state, `role="alert"` error path, `<h4>` heading hierarchy, "Same lifecycle" label, decoded-path validation — Resolved.
- 🟡 **SSE prefix invalidation refetches unmounted queries** — Partially resolved. `refetchType: 'all'` added (Phase 5 §7) and locked at unit level by Step 5.5, but the Phase 7 smoke test (Step 7.2) describes a scenario where the related query is *active* at event time, so it cannot distinguish `refetchType: 'all'` from the default `'active'`. The cross-cutting CI assertion is unreliable.
- 🟡 **Concurrency tests for `refresh_one` + reverse index** — Partially resolved. Steps 1.7b and 1.13 added, but both use a reader-loops-and-asserts-no-bad-state pattern that can pass probabilistically without proving atomicity. The plan's "atomic from reader's perspective" claim is also incoherent — the reader makes two separate `reviews_by_target` calls (two read locks) so transient inconsistency is observable even when the writer's update is per-index atomic.

### New Issues Introduced

**Critical**
- 🔴 **Correctness**: Lexical-only normalisation does not unify canonical `entry.path` with lexical `target:` key when `project_root` is reached via symlink (macOS `/var` ↔ `/private/var`). The plan does not specify that `project_root` is canonicalised once at indexer construction; without that, Step 1.2's symlink test will either fail or pass only because `normalize_absolute` secretly canonicalises, contradicting its own "never touches the filesystem" doc-comment. One-line fix to Phase 1 §1 plus an assertion in Step 1.2 that uses `Indexer::get` to obtain the canonical entry path before calling `reviews_by_target`.

**Major**
- 🟡 **Architecture / Code Quality**: `SecondaryIndex` trait + `Vec<Box<dyn>>` registry over-engineered for three heterogeneous indexes (`u32→PathBuf` numeric vs `PathBuf→BTreeSet<PathBuf>` set-valued). The shared trait surface is empty — every implementation diverges. Drop the trait and accept three explicit blocks, or use a concrete enum with inherent methods.
- 🟡 **Code Quality**: `normalize_absolute` body specified only as `/* ... */` despite being load-bearing for the entire lexical-key contract. Behaviour on `..`-escape-from-root, trailing slashes, `//` runs, and Unicode normalisation is undefined. Specify the algorithm or use `path-clean`.
- 🟡 **Architecture**: Atomicity claim over the registry is incoherent — there is no registry-level write lock; each `SecondaryIndex` impl owns its own `RwLock`. Step 1.7b's reader makes two separate `reviews_by_target` calls so cross-call atomicity is not (and cannot be) guaranteed.
- 🟡 **Architecture / Usability**: Body-rendering gate on `resolverReady` regresses cold-load TTFV for the common case (docs without wiki-links) and couples `MarkdownRenderer`'s reusability to an unrelated docs-cache lifecycle. Render the body immediately and let unresolved-spans flip to anchors when the resolver settles, or pre-scan the body for `[[…]]` and gate only when a match exists.
- 🟡 **Test Coverage**: Smoke test (Step 7.2) does not exercise the unmounted-query path that makes `refetchType: 'all'` load-bearing. Restructure the smoke (mount → unmount → event → remount) or rename to "active refetch + DOM rerender" and rely on Step 5.5 as the `refetchType: 'all'` lock.
- 🟡 **Test Coverage**: Atomic-migration tests (1.7b, 1.13) rely on probabilistic sampling. Replace with a deterministic interposed barrier or a stress-test (N≥10000 iterations) and document the trade.
- 🟡 **Code Quality**: `UnresolvedNode` pseudo-mdast type with hand-rolled `data.hName`/`hChildren`/`hProperties` shape — TypeScript silently accepts the invalid mdast node, and a misspelt `hName` falls back to default rendering with no compile error. Step 4.7's render-level test mitigates this; consider a small integration assertion that the rendered HTML carries `class="unresolved-wiki-link"`.
- 🟡 **Usability**: Legend uses "Declared / Inferred" vocabulary not present in any heading ("Targets" / "Inbound reviews" / "Same lifecycle"). First-time readers can't connect the abstract dimensions to the concrete labels. Include group labels in the legend or attach explanatory captions to each `<h4>`.

**Minor**
- 🔵 **Architecture**: `Indexer::declared_outbound` pulls "which doc-types carry which declared-link fields" knowledge into a structural index; consider a `DeclaredLinks` module owning the schema.
- 🔵 **Architecture**: Three near-identical path-validators (`doc_fetch`, `doc_patch_frontmatter`, `normalize_target_key`); extract a single `parse_repo_relative_path` helper.
- 🔵 **Architecture**: `queryKeys.disabled(prefix)` collapses different "disabled reasons" under one helper; consider TanStack Query v5+'s `skipToken`.
- 🔵 **Code Quality**: `useWikiLinkResolver` silently absorbs query errors via `?? []`; surface `error` through the hook.
- 🔵 **Code Quality**: SSE invalidation fan-out applies to all `doc-changed` events including notes; narrow to related-relevant doc kinds.
- 🔵 **Code Quality**: Atomicity comment in Phase 1 §3 refers to "the registry's `RwLock::write`" which doesn't exist; clarify it's the per-index lock.
- 🔵 **Code Quality**: Two simultaneous `Loading…` placeholders on cold load; consider hoisting a single placeholder.
- 🔵 **Test Coverage**: Step 2.14 covers `%2F` only; parametrise over `%00`, `%5C`, `%2e%2e`.
- 🔵 **Test Coverage**: Step 4.10 doesn't pin the `title` attribute or surrounding shape after sanitisation.
- 🔵 **Test Coverage**: Step 6.7b doesn't test the resolver × content state matrix (4 combinations).
- 🔵 **Test Coverage**: Step 3.7b's tie-breaker test should run with both input orderings.
- 🔵 **Test Coverage**: No memoisation-stability test for `useWikiLinkResolver`'s returned function.
- 🔵 **Security**: `normalize_target_key` lexical check is bypassable by a writable symlink under `project_root`; comment that the lexical key is not a filesystem-access authorisation.
- 🔵 **Security**: `doc_fetch` retains the looser path-validation idiom; extract a shared `validate_rel_path` helper.
- 🔵 **Security**: Unresolved-marker `title` attribute echoes the regex match; reconstruct from `(prefix, n)` instead.
- 🔵 **Security**: `Indexer::declared_outbound` should explicitly route through `target_path_from_entry` / `normalize_target_key` (single sanitisation source).
- 🔵 **Security**: Defence-in-depth: assert `resolved.href.startsWith('/library/')` in the plugin before emitting the Link.
- 🔵 **Standards**: Two sentinel-key idioms (`__invalid__`, `__disabled__`) will coexist; pick one.
- 🔵 **Standards**: New custom-hooks precedent unspecified (file location, naming, JSDoc); document.
- 🔵 **Standards**: Response envelope diverges from `{ docs: [...] }` / `{ clusters: [...] }`; document the deliberate departure.
- 🔵 **Standards**: List-element semantics for related groups not specified; pin `<ul>`/`<li>` in a TDD step.
- 🔵 **Standards**: Per-item declared/inferred badge mentioned but not asserted in tests.
- 🔵 **Standards**: New public `MarkdownRenderer` prop + new endpoint should get a CHANGELOG entry.
- 🔵 **Standards**: ARIA on the `Loading…` placeholder is inconsistent with the `aria-live="polite"` updating hint.
- 🔵 **Usability**: Error path lacks recovery affordance (no Retry button or auto-retry).
- 🔵 **Usability**: Silent fallthrough for `[[adr-0001]]` and `[[EPIC-0001]]` gives no author feedback; consider treating "almost-matches" as unresolved-marker.
- 🔵 **Usability**: Malformed `target:` values silently rejected; log a warning at the indexer level.
- 🔵 **Usability**: `Updating…` hint flashes on every background refetch; consider a 200ms delay.
- 🔵 **Usability**: Bracket-form on hover may not be discoverable; consider dropping or documenting rationale.

### Assessment

The plan is substantially stronger than the first pass — most surface-level gaps are closed, and the test sequence has grown from ~50 to ~70 cases with explicit coverage of error paths, malformed inputs, dedup, and cross-doc invariants. Two structural issues prevent a clean APPROVE:

1. **The lexical-key contract has a load-bearing missing piece.** `project_root` must be canonicalised once at indexer construction, otherwise the canonical/lexical mismatch is exactly the same class of bug as the original critical. One-line fix to Phase 1 plus an assertion in Step 1.2.
2. **Two contracts the plan claims to lock are not actually locked by the proposed tests.** `refetchType: 'all'` (Step 7.2 doesn't exercise the unmounted-query path) and the atomic-migration invariant (Steps 1.7b/1.13 are probabilistic). Both can be tightened without major restructuring.

Beyond those two, the `SecondaryIndex` trait registry is over-engineered for three heterogeneous indexes — worth simplifying to a concrete enum or dropping entirely — and the `resolverReady` body gate creates a worse UX for the dominant no-wiki-links case than the flicker it was designed to prevent.

A short third iteration should close these out:
1. Canonicalise `project_root` once at indexer construction; update Step 1.2 to seed via a symlinked-root fixture and query via `Indexer::get(...).path`.
2. Either restructure the smoke test to mount → unmount → event → remount, or rename it to lock the active-refetch contract honestly.
3. Tighten Steps 1.7b/1.13 to deterministic synchronisation (interposed barrier or stress-iteration).
4. Drop the `SecondaryIndex` trait; replace with three explicit blocks or a concrete enum.
5. Drop the `resolverReady` body gate or scope it to docs containing `[[…]]`.

---

## Re-Review (Pass 3) — 2026-04-28

**Verdict:** REVISE

### Previously Identified Issues

**Critical (Pass 2)**
- 🔴 **Lexical-key contract canonicalisation drift** — Resolved at the design level (canonicalise `project_root` once at construction). However, see new critical below: the constructor change is incomplete in the plan.

**Major (Pass 2)**
- 🟡 SecondaryIndex over-engineering — Resolved (dropped, replaced with three explicit free functions).
- 🟡 `normalize_absolute` body unspecified — Resolved (algorithm spelled out).
- 🟡 Atomicity claim incoherent — Resolved at the design level (single `entries.write()` discipline). However, see new critical: the deterministic atomicity test is unimplementable as written.
- 🟡 Body-rendering gate on `resolverReady` regressed cold-load TTFV — Resolved (gate dropped). New major: the cold-load behaviour now visually conflates "warming" with "broken refs".
- 🟡 Smoke test didn't lock `refetchType: 'all'` — Resolved with Test A/Test B split. Minor caveat about `staleTime: 0` defaults remains.
- 🟡 Legend vocabulary mismatch — Partially resolved (legend present, "shares a slug" still jargon).

### New Issues Introduced

**Critical**
- 🔴 **Correctness/Standards**: Plan introduces `Indexer::new` but doesn't retire/modify the existing `Indexer::build`. The existing constructor is async, takes a `driver`, returns `Result<_, FileDriverError>`, and stores `project_root` as-passed. If `build` remains alongside `new`, every existing call site (including production `AppState::build` in `server.rs`) bypasses the canonicalisation discipline. The structural fix collapses to a per-call-site discipline. **Fix: modify `Indexer::build` to canonicalise `project_root` as its first step (mapping `io::Error` into `FileDriverError`) — do not introduce a new `Indexer::new`.**
- 🔴 **Correctness/Test Coverage**: Steps 1.7b and 1.13 cannot pass as written. Step 1.7b says "acquire `entries.read()` to capture a snapshot... then have a separate task call `reviews_by_target(...)`" — but that accessor *also* takes `entries.read()`, and tokio's RwLock is write-preferring: once the writer queues for `entries.write()`, new readers block to avoid writer starvation, deadlocking the test. Step 1.13 has the analogous issue. **Fix: restructure both tests to use a `#[cfg(test)]` `Notify` barrier inside the production code's critical section, so the writer signals the reader at known points within its own lock hold. The plan must explicitly specify this hook surface.**

**Major**
- 🟡 **Architecture**: Lock ordering inverted between writers (`entries → secondary`) and readers (`secondary → entries`). Under tokio's writer-preferring RwLock, queued writers can starve readers holding a secondary read. Fix: hoist all locking to a single canonical order (readers also acquire `entries.read()` first, then secondary), or collapse to one composite RwLock for `entries + secondary`.
- 🟡 **Code Quality**: `find_entry_for_deleted` currently takes `entries.read()` internally and drops it. The new single-`entries.write()` discipline conflicts with this — either the deletion path deadlocks or reopens a TOCTOU window. Fix: restructure `find_entry_for_deleted` to take `&HashMap<PathBuf, IndexEntry>` (held write guard) instead of acquiring its own lock.
- 🟡 **Code Quality**: `update_*` free function shape under-specified — visibility, parameter ordering, async-ness, return type left to implementer. Fix: sketch one full canonical signature and state the rule.
- 🟡 **Code Quality**: `normalize_absolute` algorithm under-specifies edge cases (empty input, trailing-slash stripping contradicting `Components` semantics, Windows `Prefix` without `RootDir`, drive-relative `..`). Either tighten spec to POSIX-only inputs or pin Windows behaviour.
- 🟡 **Test Coverage**: Step 1.2 references macOS `/var` ↔ `/private/var` symlink but Linux CI runners would silently degrade to a tautology. Fix: explicitly construct the symlink via `std::os::unix::fs::symlink`; gate Windows with `#[cfg(unix)]`.
- 🟡 **Documentation drift**: Phase 6 §2.a still describes `useWikiLinkResolver` as exposing an `isReady` flag — but Phase 5 §5 returns only `{ resolver }`. Testing Strategy still references the dropped `SecondaryIndex` registry and the dropped `resolverReady` body-render gate. Pure cleanup.
- 🟡 **Usability**: Cold-load behaviour visually conflates "cache warming" with "broken reference". On a doc with many `[[…]]` refs, first paint shows every ref styled muted-as-broken until cache settles. Fix: emit a different marker class (`wiki-link-pending` neutral skeleton) while pending; reserve `unresolved-wiki-link` for resolver-returned-null after settle.
- 🟡 **Usability**: "Updating…" hint flashes on every `doc-changed` event due to `refetchType: 'all'` invalidating all cached related queries — including for unrelated docs being edited elsewhere. Fix: suppress when refetch produces byte-identical data, or debounce with 200ms delay.
- 🟡 **Correctness**: Resolver-flip-on-settle relies on react-markdown re-running its pipeline when `remarkPlugins` array reference changes — works today because the array is constructed inline every render, but a future memo optimisation breaks it silently. Fix: memoise `remarkPlugins` in `MarkdownRenderer` keyed on `[resolveWikiLink]` so resolver identity is the actual trigger.

**Minor**
- 🔵 Plan references `mod refresh_tests` which doesn't exist in current `indexer.rs` (only `mod tests`).
- 🔵 `disabled(prefix)` claims to consolidate inline sentinel-key idioms that don't exist in the codebase. Reframe as "introduces".
- 🔵 No retry affordance on related-fetch error path (carry-over from pass 2).
- 🔵 Unresolved-marker `title` attribute duplicates visible text; could carry a diagnostic message ("No matching ADR found for ID 9999").
- 🔵 Legend wording "shares a slug" is jargon for non-technical readers; consider "follow the same lifecycle (matching filename pattern)".
- 🔵 `update_reviews_by_target` empty-key cleanup described in prose but not locked by a test.
- 🔵 `normalize_target_key`'s defence-in-depth `starts_with` check has no test (either unreachable or per-segment validation is incomplete — debug_assert or specific test case).
- 🔵 Test B in Phase 7 may pass without `refetchType: 'all'` due to TanStack Query's default `staleTime: 0` + `refetchOnMount: true`. Configure non-zero `staleTime` in the test to suppress remount-triggered refetches.
- 🔵 Step 1.13 doesn't specify the test-only `Notify` hook surface area in production code.
- 🔵 Step 4.5/4.5b code-block exclusion doesn't cover the indented (4-space) form.
- 🔵 Step 5.9 doesn't verify `useDocContent` and `useRelated` use distinct query keys.
- 🔵 No automated test for the bidirectional update contract (both old target and new target plan pages refresh on a `target:` edit).
- 🔵 `target_path_from_entry` hard-codes `DocTypeKey::PlanReviews`; consider a `DECLARED_LINK_FIELDS` table for future extension (acknowledged scope-fence; comment-pointer suffices for v1).
- 🔵 Group ordering rationale (Targets > Inbound reviews > Same lifecycle) brittle to refactor; add a comment + ordering test.
- 🔵 Single-writer-lock discipline coarsens lock granularity vs the existing per-map locking — worth a brief tradeoff note in Performance Considerations.
- 🔵 Decoded-path validation (Step 2.14) covers `%2F` only; parametrise over `%00`, `%0A`, `%0D`, `%5C` for completeness.

### Assessment

The plan converged on the right design. The remaining issues fall into three buckets:

1. **The new criticals are both fixable in <10 lines of plan changes**: modify `Indexer::build` (don't add a new constructor) and reformulate Steps 1.7b/1.13 to use a `#[cfg(test)]` `Notify` hook inside `refresh_one`'s critical section.

2. **Documentation drift** — three places (Phase 6 §2.a `isReady`, Testing Strategy `SecondaryIndex`, Testing Strategy `resolverReady`) still reference removed concepts. Pure cleanup.

3. **Two material UX issues** — cold-load visually conflates warming with broken; "Updating…" hint flashes on unrelated edits. Both have clean fixes (distinguish warming-marker from unresolved-marker; debounce or diff-suppress the hint).

The lock-ordering and `find_entry_for_deleted` concerns are real but lower-impact; both can be addressed by spelling out the locking discipline more carefully without major restructuring.

A focused fourth iteration to address:
1. Replace `Indexer::new` references with modifying the existing `Indexer::build`
2. Rewrite Steps 1.7b/1.13 to use a `#[cfg(test)]` `Notify` hook (and specify the hook surface in §2)
3. Hoist reader lock order to match writer (`entries → secondary`) or collapse to one composite RwLock
4. Restructure `find_entry_for_deleted` to take a held write guard
5. Strip the three documentation-drift references (Phase 6 §2.a `isReady`, Testing Strategy × 2)
6. Distinguish warming-marker from unresolved-marker (Phase 4 + Phase 6)
7. Suppress/debounce the "Updating…" hint on duplicate refetch results
8. Memoise `remarkPlugins` in `MarkdownRenderer`

---

## Re-Review (Pass 4) — 2026-04-28

**Verdict:** COMMENT (with three majors worth addressing)

The pass-4 changes resolved every prior critical and most majors cleanly. **No new criticals were introduced.** Three majors surfaced — one is a genuine correctness bug (resolver doesn't return to `pending` on cache invalidation), one is a concurrency-test soundness issue (`notify_waiters()` lost-wakeup hazard), and one is a logic bug in `update_reviews_by_target` that would surface only on review file rename. The remaining issues are precision-and-polish rather than design soundness.

### Previously Identified Issues

**Critical (Pass 3)**
- 🔴 Incomplete constructor migration — Resolved (`Indexer::build` modified in place).
- 🔴 Unimplementable concurrency tests — Resolved structurally via `#[cfg(test)] test_post_secondary_update` Notify hook (but see new lost-wakeup major below).

**Major (Pass 3)**
- 🟡 Lock ordering inverted — Resolved.
- 🟡 `find_entry_for_deleted` lock conflict — Resolved.
- 🟡 `update_*` shape under-specified — Resolved (canonical signature spelled out).
- 🟡 `normalize_absolute` edge cases — Resolved.
- 🟡 Step 1.2 macOS-only symlink test — Resolved.
- 🟡 Documentation drift (`isReady` / `SecondaryIndex` / `resolverReady`) — Resolved.
- 🟡 Cold-load conflates warming with broken — Resolved (`wiki-link-pending` vs `unresolved-wiki-link`).
- 🟡 "Updating…" hint flashes on every event — Resolved (`useDeferredFetchingHint` debounce).
- 🟡 `MarkdownRenderer` plugin re-run incidental — Resolved (memoised `remarkPlugins`).

### New Issues Introduced

**Major**
- 🟡 **Correctness**: Resolver does not return to `pending` on cache invalidation/refetch. `isWarming = adrs.isPending || tickets.isPending` is true only on initial fetch (no cached data); on `invalidateQueries` + refetch with cached data, `isPending` stays `false`. Stale anchors/markers visible until refetch completes; for a deleted ADR that another doc references via `[[ADR-NNNN]]`, the user could see a clickable link to a no-longer-extant page. Step 5.8b only verifies `pending → settled`, not the back-to-pending direction. **Fix:** define `isWarming` to include the refetch-with-cached-data case, or accept the staleness explicitly and document it.
- 🟡 **Correctness**: `notify_waiters()` is lost-wakeup-prone. The hook calls `notify_waiters()` (does not buffer); if the test thread reaches `.notified().await` after the writer reaches the hook, the notification is lost and the test deadlocks or false-passes. Step 1.7b also conflates writer→test and test→writer signal directions on a single Notify field. **Fix:** use `notify_one()` (buffers one permit) or two `oneshot::channel`s — one for writer→test, one for test→writer. Update §3 field declaration to hold both halves explicitly.
- 🟡 **Correctness**: `update_reviews_by_target` early-return on `prev_target == next_target` leaks stale path on review file rename. If the review file is renamed but its `target:` is unchanged, `previous.path != new_entry.path` while targets match — the early return leaves the old path stale in the BTreeSet without inserting the new path. **Fix:** path-aware check (`prev_target == next_target && previous.map(|p| &p.path) == Some(&new_entry.path)`), or drop the optimisation entirely.
- 🟡 **Standards**: Plugin-emitted class names are kebab-case globals (`unresolved-wiki-link`, `wiki-link-pending`) while `RelatedArtifacts` CSS module uses camelCase. CSS modules' default mode hashes class names; plugin-emitted literal kebab-case classes won't match a module-scoped selector. **Fix:** declare these as deliberate global classes in `globals.css` with a one-line rationale, or thread the resolved CSS-module class names into the plugin configuration.

**Minor** — see review file for full list (architecture: helper signature asymmetry, cfg(test) field on production type, SSE fan-out, hardcoded prefix→doctype mapping; code quality: cfg(test) field shape, debounce primitive reuse, `pub(super)` convention, `isPending` vs `isFetching` comment, MarkerNode docstring; test coverage: one-sided atomicity assertion, plugin-rerun verification, error-path body assertion, smoke-test scaffolding, invalidation-scope test, mixed-resolution parametrisation, populated-cache miss case, primary-index content assertion, marker distinguishability test; correctness: hint-helper inline-derivation comment, `canonicalize` precondition, empty Text nodes; security: diagnostic title sanitisation, CanonicalRoot newtype; standards: `useDeferredFetchingHint` placement, `parseInt(prefix, 10)` confusing description, TDD step numbering, legend element type, `declared_outbound` contract location; usability: tooltip parity, layout reflow on flip, legend jargon, dual loading affordances, keyboard accessibility, manual debounce verification).

### Assessment

The plan is now implementable. The three correctness majors are all worth a focused fifth pass:

1. **Resolver doesn't return to `pending` on invalidation** — pick a definition for `isWarming` and document; lock the rotation in Step 5.8b.
2. **`notify_waiters()` lost-wakeup risk** — switch to `notify_one()` or `oneshot` channels; specify both signal directions.
3. **`update_reviews_by_target` rename leak** — drop the optimisation, or make the equality path-aware.

The standards major (CSS globals vs modules) is a one-line clarification.

Verdict has moved from REVISE to **COMMENT**: the design is sound, the test sequence is comprehensive, and the prior critical issues are all resolved. The three new correctness gaps are real but localised; they should be fixed before implementation but do not require structural rework. The minor list is long but every item is a precision/polish concern, not a blocker.
