---
date: "2026-05-19T01:30:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-18-0042-templates-view-redesign.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, performance, standards]
review_pass: 3
status: complete
---

## Plan Review: Templates View Redesign

**Verdict:** REVISE

The plan is well-structured and TDD-shaped, reuses established
primitives appropriately, and decomposes work into focused phases.
However, the Phase 2 watcher wiring concentrates several
correctness, architectural, and coverage risks: a concurrent-rebuild
race that can silently regress the displayed hash, a
canonicalisation mismatch that breaks live updates on macOS / under
symlinks, a state-transition gap that strands the UI when a winning
tier file is emptied, and several event-routing precedence and
shared-tier edge cases that the test plan does not exercise. These
blockers must be addressed before implementation; the broader
architectural and standards concerns are improvements rather than
blockers.

### Cross-Cutting Themes

- **Watcher template-tier branch is doing too much** (flagged by:
  architecture, code-quality, correctness, test-coverage) — the
  branch is bolted into an already-overloaded
  `on_path_changed_debounced`, broadens `watcher::spawn`'s signature
  by three parameters, mixes precedence with the existing doc flow
  via early `return`, and the new logic isn't exercised for debounce,
  rapid edits, or shared-path cases. Extracting a small
  `TemplateChangeHandler` (or per-template refresher) consolidates
  the fix.
- **Path canonicalisation is inconsistent end-to-end** (flagged by:
  correctness, performance) — event paths are canonicalised, but
  `cfg.templates` tier paths and `watch_dirs.dedup()` are not. On
  macOS or under symlinks, lookups silently miss and watches
  duplicate.
- **Full resolver rebuild on every tier-file change** (flagged by:
  architecture, code-quality, performance) — every tier edit
  re-reads every template's three tier files and re-hashes content
  on every API request. A per-name refresh and a precomputed
  cached sha256 are the natural fixes; current scale tolerates it,
  but the structural coupling is worth flagging.
- **Live-update path coverage gap** (flagged by: code-quality,
  test-coverage, correctness) — the Phase 6 live-update test
  bypasses `dispatchSseEvent`, AC11's "label equals backend
  sha256" is split across two halves never joined in one test,
  and the empty-content state transition produces no SSE event so
  AC10 cannot be satisfied via the live path.
- **Bare-hex vs sha256-prefixed hash representation** (flagged by:
  architecture, standards) — the new top-level `sha256` field
  deliberately drops the `sha256-` prefix used everywhere else in
  the codebase, and the UI re-prepends it on render. The
  divergence is documented but real and worth reconsidering.
- **TierPresenceRow encodes a backend invariant as a UI fallback**
  (flagged by: code-quality, correctness, standards) — the
  `?? (source === 'plugin-default')` fallback duplicates a server
  guarantee and would silently lie if the invariant ever changes.

### Tradeoff Analysis

- **Per-name refresh vs whole-resolver rebuild**: A per-name
  refresh reduces watcher cost from O(templates × tiers) to O(1)
  per change but adds API surface to `TemplateResolver` and a
  small interior-mutability concern. At current scale (~10
  templates) the whole-resolver rebuild is cheap; the recommendation
  is to keep the current approach but precompute and cache the
  sha256 on the resolver (one-line win) and flag the per-name
  refresh as a future evolution lever in "What we're NOT doing".
- **Bare-hex vs prefixed `sha256` on the wire**: Bare hex separates
  the new field from HTTP-`ETag:` semantics and from the existing
  per-tier `etag`. The cost is two superficially-identical hash
  fields on the same response in different shapes. The simpler
  alternative — keep the `sha256-` prefix on the wire, render
  verbatim — eliminates the UI's prepend responsibility and the
  inconsistency. Either is defensible; the plan should pick one
  consciously and document the rejection of the other.

### Findings

#### Critical

- 🔴 **Correctness**: Lost-update race on concurrent `ArcSwap::store`
  from independent debounced rebuilds
  **Location**: Phase 2, Section 6: Watcher — template-tier branch
  Two rapid tier-file edits each spawn an independent
  `TemplateResolver::build(...).await` + `templates.store(...)`.
  The later-completing rebuild can store an older snapshot,
  silently regressing the displayed hash until the next change.

- 🔴 **Correctness**: Canonicalisation mismatch between event path
  and config-held tier paths will silently drop events
  **Location**: Phase 2, Section 6: Watcher — `name_for_tier_path`
  Event paths are canonicalised, but `cfg.templates` tier paths
  are not. On macOS (`/var` vs `/private/var`), under symlinks, or
  with relative-vs-absolute mismatches, no template will match and
  the watcher silently falls through.

#### Major

- 🟡 **Architecture**: Template branch precedence can shadow
  doc-changed events for overlapping paths
  **Location**: Phase 2, Section 6
  Template tier paths can coincide with `cfg.doc_paths`. The
  template branch's unconditional `return` swallows the event,
  breaking the activity feed / clusters / library cache for that
  file.

- 🟡 **Architecture**: `watcher::spawn` parameter explosion (8 → 11)
  **Location**: Phase 2, Sections 1, 4, 5, 6
  Adding `templates`, `cfg`, `driver` couples the watcher to four
  major subsystems and pushes it toward god-object territory.
  Suggested: introduce a `TemplateChangeHandler` collaborator.

- 🟡 **Code Quality**: Template branch overloads
  `on_path_changed_debounced` rather than extracting a dispatcher
  **Location**: Phase 2, Section 6
  Function already carries `#[allow(clippy::too_many_arguments)]`;
  branched contract with `return` mid-async-fn is easy to misread
  and risk-prone to future additions.

- 🟡 **Code Quality**: Bare `return` in async fn skips
  clusters/indexer without explicit control-flow contract
  **Location**: Phase 2, Section 6 (pseudo-Rust)
  Hidden control flow in a long async function with `.await`
  side-effects. Extract a handler function or use explicit
  `if/else` with a comment.

- 🟡 **Correctness**: Emptying the winning tier file produces no
  SSE event, leaving the UI stuck on the previous hash
  **Location**: Phase 2, Section 6 + Phase 6 manual verification
  Broadcast is gated on `Some(sha256)`. Truncation to 0 bytes →
  `sha256` becomes None → no broadcast → UI never invalidates.
  AC10 fails via SSE; manual verification step would not pass.

- 🟡 **Correctness**: Template branch's early `return` skips doc
  flow even when tier path overlaps with a doc path
  **Location**: Phase 2, Section 6
  Files that are both tier and doc emit only `template-changed`,
  starving the indexer/clusters/activity feed of the change.

- 🟡 **Correctness**: `name_for_tier_path` returns `Option<String>`
  — templates sharing a tier file get inconsistent updates
  **Location**: Phase 2, Section 6
  Two templates pointing at the same plugin-default file: only
  the first matched template gets a `template-changed` event.

- 🟡 **Correctness**: Non-recursive watch on tier-parent
  directories misses nested tier files and editor temp-renames
  **Location**: Phase 2, Section 5
  `file_driver::template_extra_roots` returns immediate parents;
  watcher uses `RecursiveMode::NonRecursive`. Editor atomic-write
  patterns and nested tier layouts will not produce events.

- 🟡 **Test Coverage**: SSE-to-DOM live-update path is not
  end-to-end exercised
  **Location**: Phase 6 — live-update test
  Test calls `qc.invalidateQueries(...)` directly instead of
  dispatching a real `template-changed` event through
  `dispatchSseEvent`. A reducer regression would pass this test.

- 🟡 **Test Coverage**: AC11 (label value equals backend sha256)
  is not asserted end-to-end
  **Location**: Phase 6 — content-hash label tests
  Backend test re-derives the digest; frontend test uses a literal
  `'a'.repeat(64)`. No test computes the digest in the fixture
  setup and asserts the rendered label matches.

- 🟡 **Test Coverage**: Watcher template branch lacks debounce
  and rapid-edit coverage
  **Location**: Phase 2 — watcher tests
  No test covers two rapid edits coalescing into a single
  `template-changed`, or delete-then-write races, or that debounce
  actually applies to the new branch.

- 🟡 **Test Coverage**: First-row-position assertion is fragile
  to DOM structure changes
  **Location**: Phase 6 — preview-pane test
  `pane.firstElementChild === ...` breaks on benign wrapper
  insertions; use `compareDocumentPosition` to pin user-observable
  ordering instead.

- 🟡 **Standards**: Detail endpoint path mismatch left unresolved
  **Location**: Desired End State — "Backend HTTP surface"
  Work item says `/api/library/templates/{name}`, actual mount is
  `/api/templates/:name`. Plan flags but does not resolve. Tests
  use the existing mount; convention favours `/api/templates/:name`.

#### Minor

- 🔵 **Architecture**: Store-then-load creates a small consistency
  window between swap and broadcast
  **Location**: Phase 2, Section 6
  Compute `detail` on the local `new_resolver` before storing,
  rather than `store → load → detail`, so the broadcast's sha256
  is causally paired with the rebuild it triggered.

- 🔵 **Architecture**: Full resolver rebuild on every tier-file
  change couples per-template cost to total template count
  **Location**: Phase 2, Section 6
  Worth flagging as a known evolutionary constraint even if
  current scale tolerates it.

- 🔵 **Architecture**: `name_for_tier_path` linear scan over
  `cfg.templates` on every fs event
  **Location**: Phase 2, Section 6
  Precompute a `HashMap<PathBuf, String>` at watcher spawn time.

- 🔵 **Architecture**: Live-update path refetches full
  `/api/templates/{name}` payload to refresh one hash field
  **Location**: Phase 6 + Phase 3
  Accept as simplest design and document, or consider an
  optimistic `setQueryData` patch using the SSE payload's sha256.

- 🔵 **Architecture / Standards**: Two hash representations on the
  same response (`etag: "sha256-…"` vs `sha256: "<hex>"`)
  **Location**: Phase 1, Section 1
  Either rename to `contentHash` to signal the shape distinction,
  or keep the `sha256-` prefix on the wire and drop the UI's
  prepend responsibility.

- 🔵 **Code Quality**: `TierPresenceRow` smuggles the
  always-present-plugin-default invariant into a fallback
  **Location**: Phase 4, Section 2
  Trust the backend contract (`?? false`), or assert/log when the
  invariant breaks.

- 🔵 **Code Quality**: CSS-module text grep tests pin
  implementation rather than behaviour
  **Location**: Phase 4 and Phase 5/6 route tests
  Anchor regexes to specific selectors or replace with DOM-attribute
  / `getComputedStyle` assertions where viable.

- 🔵 **Code Quality**: Winning-tier selection re-derived in
  `TemplatePreviewPane` (active+present) vs `TierPanel`
  (source === activeTier)
  **Location**: Phase 6, Section 2
  Pick one canonical derivation or extract a `getWinningTier(data)`
  helper.

- 🔵 **Code Quality**: Dispatch reducer + `queryKeysForEvent`
  duplicate the template invalidation set
  **Location**: Phase 3, Section 2
  Factor out a single `templateKeysForEvent` and share between
  the direct and drag-deferred paths.

- 🔵 **Code Quality**: Watcher's resolver-rebuild responsibility
  leaks `cfg` + `driver` collaborators into the watcher
  **Location**: Phase 2, Sections 5–6
  Add `TemplateResolver::rebuild_into(...)` or a `TemplateRefresher`
  type owning `(cfg, driver, ArcSwap)`.

- 🔵 **Test Coverage**: Tristate matrix missing "user-only" case
  **Location**: Phase 4 — index tests
  Add a fourth fixture row with `user-override` winning and no
  `config-override`.

- 🔵 **Test Coverage**: Reducer negative-assertion flattens query
  keys loosely
  **Location**: Phase 3 — dispatch reducer tests
  Assert exact call count + deep-equal each call's queryKey
  instead of `.not.toContain(...)` on a flattened array.

- 🔵 **Test Coverage**: SSE e2e test doesn't pin the new event's
  sha256 / shape
  **Location**: Phase 2 — `sse_e2e.rs` template test
  Parse the chunk as JSON and assert `sha256 ==
  hex(Sha256(b"v2"))` plus 64-char lowercase shape.

- 🔵 **Test Coverage**: CSS-regex assertions pass with looser
  declarations than intended
  **Location**: Phase 5–6 — CSS regression assertions
  Anchor to the specific selector, e.g.
  `/\.twoColumn\s*\{[^}]*grid-template-columns:\s*minmax/`.

- 🔵 **Test Coverage**: Non-interactivity test's `onclick === null`
  check doesn't catch React's synthetic `onClick`
  **Location**: Phase 6 — non-interactivity test
  Use `userEvent.click(label)` + side-effect assertions instead.

- 🔵 **Correctness**: Invalid-UTF-8 winning content produces no
  sha256 but `present: true` — behaviour untested
  **Location**: Phase 1, Section 1
  Add an explicit test asserting `sha256.is_none()` with
  `present: true`, or hash bytes regardless of UTF-8 validity.

- 🔵 **Correctness**: `resolver.detail(name).is_none()` arm in
  watcher silently drops the broadcast
  **Location**: Phase 2, Section 6
  `expect` / log when the invariant ("every cfg name is in the
  resolver") breaks.

- 🔵 **Correctness**: `state.cfg.clone()` into watcher captures a
  snapshot; future config hot-reload would diverge
  **Location**: Phase 2, Section 5
  Pass `Arc<AppState>` (or `Arc<Config>`) instead of cloning.

- 🔵 **Correctness**: Drag-deferred path's template-changed
  invalidation depends solely on `queryKeysForEvent` extension
  **Location**: Phase 3, Section 2
  Add an explicit during-drag test for `template-changed` to lock
  the behaviour.

- 🔵 **Correctness**: Empty-string sha256 omission has no
  rationale comment
  **Location**: Phase 1, Section 1
  Add a one-line comment naming AC10 as the source of truth.

- 🔵 **Performance**: sha256 recomputed on every
  `/api/templates/:name` request
  **Location**: Phase 1, Section 1
  Cache the digest on the swapped resolver entry; one-line win.

- 🔵 **Performance**: No coalescing of `template-changed`
  broadcasts across multiple tier files for the same template
  **Location**: Phase 2, Section 6
  Dedup by template name (suppress if sha256 unchanged) or key
  debounce by template name.

- 🔵 **Performance**: Live updates refetch full per-tier content
  payload to refresh one label
  **Location**: Phase 6, Section 1
  Consider `detail()` returning content only for the active tier.

- 🔵 **Standards**: `TemplateChanged` omits a per-variant
  `rename_all = "camelCase"`
  **Location**: Phase 2, Section 2
  Apply it for forward-compatibility on multi-word fields.

- 🔵 **Standards**: Content-hash label lacks
  semantic/accessibility framing
  **Location**: Phase 6, Section 2
  Add `aria-label="Content hash"` or a visually-hidden prefix.

- 🔵 **Standards**: Phase 5's empty `aria-hidden` placeholder div
  is a phase-boundary smell
  **Location**: Phase 5, Section 2
  Either drop the placeholder (CSS grid leaves the slot empty) or
  fold Phases 5+6 into a single phase.

#### Suggestions

- 🔵 **Code Quality**: Hoist `TIER_LABELS`, `TIER_SHORT_LABELS`,
  `TIER_ORDER` to a shared module (Phase 4)
- 🔵 **Test Coverage**: Standardise bare-hex shape regex to
  reject uppercase A–F across both Rust tests (Phase 1)
- 🔵 **Test Coverage**: Add a small ArcSwap reader-during-swap
  stress test (Phase 2)
- 🔵 **Performance**: Canonicalise paths before
  `watch_dirs.sort()/dedup()` to avoid double-watching the same
  physical directory (Phase 2, Section 5)

### Strengths

- ✅ Reuses `#[serde(tag = "type", rename_all = "kebab-case")]`
  so the new SSE variant is wire-correct with zero manual wiring
- ✅ ArcSwap chosen appropriately for the read-heavy / write-rare
  resolver access pattern
- ✅ Bare-hex vs prefixed-hash divergence is made explicit with a
  doc comment requirement (a tradeoff on record rather than
  hidden)
- ✅ Template-tier branch keeps templates out of the indexer /
  clusters / activity-feed flow (templates aren't real docs)
- ✅ Pure `dispatchSseEvent` reducer extended via early-return
  branch, preserving testability and the functional-core /
  imperative-shell separation
- ✅ Strong AC-to-test mapping for most criteria, with explicit
  per-AC test file and assertion plans
- ✅ Sha256 field uses `skip_serializing_if = Option::is_none`
  with tests asserting absence rather than null/empty string —
  sound JSON contract
- ✅ Edge cases for the sha256 field (empty content, absent
  winning tier) explicitly covered with both inline unit tests
  and HTTP integration tests
- ✅ Test cross-check re-derives the digest from fixture content
  rather than hard-coding, staying robust to fixture edits
- ✅ All design tokens, Chip variants (neutral / indigo / green),
  CSS module conventions, and kebab-case SSE event naming align
  with established standards (0033, 0038, 0055)
- ✅ Outline-vs-border choice for the active-tier ring is
  justified inline (geometry stability) — the kind of micro-
  decision worth capturing for future readers
- ✅ TanStack Query invalidation (not manual cache mutation)
  means concurrent invalidations from rapid SSE events
  naturally coalesce into a single in-flight refetch per query
  key

### Recommended Changes

Ordered by impact:

1. **Serialise resolver rebuilds and fix the lost-update race**
   (addresses: Lost-update race on concurrent `ArcSwap::store`)
   Route all template-tier change notifications through a single
   dedicated rebuild task fed by an `mpsc::channel`, or attach a
   monotonic sequence number per scheduled rebuild and only
   commit on `store` if the sequence is the latest.

2. **Canonicalise `cfg.templates` tier paths at AppState build
   time**
   (addresses: Canonicalisation mismatch silently drops events)
   Precompute canonical PathBufs once at startup (with a
   fallback to the original when canonicalize fails — matches
   `tokio::fs::canonicalize(&path).await.unwrap_or_else(...)`
   pattern already used in the watcher). Use these for
   `name_for_tier_path` matching. Apply the same canonicalisation
   to `watch_dirs.sort()/dedup()`.

3. **Always broadcast `template-changed` for a tier-file change,
   carrying `Option<String> sha256`**
   (addresses: Emptying winning tier produces no event;
   single-template match misses shared tier files)
   Change broadcast condition from `if let Some(sha256)` to
   unconditional broadcast on rebuild success, with sha256 as
   `Option<String>`. Frontend reducer invalidates regardless of
   sha256 presence, satisfying AC10 via SSE. Also widen
   `name_for_tier_path` to return all matching template names
   (`Vec<String>`) and broadcast once per match.

4. **Extract a `TemplateChangeHandler` (or similar) from
   `watcher::spawn`**
   (addresses: Template branch precedence shadowing doc events;
   `watcher::spawn` parameter explosion; bare `return` control
   flow; `on_path_changed_debounced` overload; cfg+driver leaking
   into watcher)
   Introduce a small collaborator that owns
   `(templates, cfg, driver, hub)` and exposes a single `async fn
   try_handle(path) -> bool` returning whether the path was
   handled as a template change. `on_path_changed_debounced`
   becomes: handle template (additive — log + broadcast), then
   fall through to the existing doc flow when the path is also a
   doc path. This addresses precedence, signature growth,
   control-flow opacity, and locality of the rebuild concern.

5. **Watch tier parents recursively (or watch tier files
   directly)**
   (addresses: Non-recursive watch misses nested tier files /
   editor temp-renames)
   Switch the tier-parent watches to `RecursiveMode::Recursive`,
   relying on the `is_markdown` filter + `name_for_tier_path`
   check to scope work to actual tier files. Alternatively
   register per-file watches.

6. **Cache the sha256 digest on the resolver entry**
   (addresses: sha256 recomputed on every request; full rebuild
   structural coupling)
   Precompute `sha256: Option<String>` inside
   `TemplateResolver::build` and store it alongside the per-name
   tier vec, so `detail()` returns the cached string. The hash
   is computed exactly once per rebuild.

7. **Drive the live-update test through `dispatchSseEvent` and
   compute the digest in the test fixture**
   (addresses: SSE-to-DOM live-update path not end-to-end
   exercised; AC11 not asserted end-to-end)
   In Phase 6's live-update test, call `dispatchSseEvent({...
   template-changed ...}, qc)` instead of
   `qc.invalidateQueries(...)`. In another Phase 6 test, set
   `mockDetail.sha256 = hex.encode(sha256(content))` computed at
   test setup and assert the rendered label equals
   `sha256-${that}`.

8. **Add watcher coverage for debounce + rapid edits +
   shared-tier paths**
   (addresses: Watcher template branch lacks debounce / rapid-edit
   coverage; shared-tier-file edge case)
   Write the tier file twice in 20ms and assert exactly one
   broadcast carrying the final hash. Configure two templates
   pointing at the same plugin-default file and assert both
   receive a broadcast.

9. **Resolve the detail endpoint URL in the plan to
   `/api/templates/:name`**
   (addresses: Detail endpoint path mismatch left unresolved)
   Update the "Backend HTTP surface" section to use
   `/api/templates/:name` and note the work-item phrasing as a
   labelling error. Do not add a redundant `/api/library/...`
   alias.

10. **Decide bare-hex vs prefixed on the wire**
    (addresses: two hash shapes on one response)
    Either (a) rename the field to `contentHash` to signal the
    shape distinction, or (b) keep `sha256-` prefix on the wire
    and render verbatim (dropping the UI's prepend).
    Recommendation: option (b) — fewest divergences from existing
    convention.

11. **Tighten test assertions** (small batch)
    (addresses: First-row-position fragility; CSS-regex
    looseness; reducer negative assertion; `onclick === null`
    weakness; e2e SSE shape; user-only tristate case)
    Apply the specific suggestions in the minor findings; these
    are quick edits and improve regression safety materially.

12. **Drop the `TierPresenceRow` fallback for plugin-default**
    (addresses: invariant smuggled into UI fallback)
    Trust the backend contract (`?? false`) or assert/log when
    the invariant breaks.

13. **Document deferrals in "What we're NOT doing"**
    Add explicit notes for: per-name resolver refresh as a
    future evolution, payload reduction (active-tier-content-
    only `detail()`), broadcast dedup by template name. Keeps
    future maintainers informed without expanding current scope.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Well-grounded in existing patterns — composes over the
established `SsePayload` tag/serde model, the per-tier etag
pattern, the TanStack Query invalidation reducer, and the Chip /
Page / MarkdownRenderer primitives, with a single new dependency
(`arc-swap`) and a justified divergence between bare-hex `sha256`
and prefixed per-tier `etag`. The main architectural risks are
concentrated in Phase 2's watcher wiring.

**Findings**: Template branch precedence shadowing (major); watcher
signature explosion (major); store-then-load consistency window
(minor); full resolver rebuild (minor); `name_for_tier_path` linear
scan (minor); live-update full-payload refetch (minor); two hash
representations (minor).

### Code Quality

**Summary**: Well-structured, follows existing conventions, focused
phases. Main risks: Phase 2 watcher overload (function bloat, bare
`return`, expanded signature, leaked collaborators) and several
tests asserting against CSS module source text rather than
behaviour.

**Findings**: `on_path_changed_debounced` overload (major); bare
`return` in async fn (major); `TierPresenceRow` smuggled invariant
(minor); CSS-module text grep tests (minor); winning-tier re-
derived in preview pane (minor); dispatch reducer duplication
(minor); cfg+driver leaking into watcher (minor); live-update test
bypasses reducer (minor); tier label duplication (suggestion).

### Test Coverage

**Summary**: Testing-forward and TDD-shaped, mapping cleanly to most
ACs. Under-covered: AC11 end-to-end, AC12's SSE-to-DOM via the real
dispatcher, watcher debounce/rapid-edit, several implementation-
fragile assertions.

**Findings**: SSE-to-DOM not end-to-end (major); AC11 not end-to-end
(major); watcher debounce/rapid-edit (major); first-row-position
fragile (major); CSS-text regex looseness (minor); missing
user-only tristate case (minor); negative-assertion loose (minor);
SSE e2e doesn't pin sha256 (minor); `onclick === null` doesn't
catch React's synthetic (minor); bare-hex regex permissiveness
(suggestion); ArcSwap reader-during-swap stress test (suggestion).

### Correctness

**Summary**: Well-structured but contains several correctness gaps
centred on concurrency and path matching. Two critical issues
(rebuild race, canonicalisation mismatch); several majors around
the empty-content state transition, doc-overlap precedence, and
shared-tier behaviour; several edge cases needing explicit
handling or documentation.

**Findings**: ArcSwap lost-update race (critical); canonicalisation
mismatch (critical); empty → None produces no SSE event (major);
template/doc precedence collision (major); single-template match
on shared tier file (major); non-recursive watch misses (major);
invalid-UTF-8 winning content (minor); detail returns None silent
drop (minor); TierPresenceRow fallback assumption (minor); drag-
deferred path coverage (minor); empty-string sha256 rationale
(minor); cfg snapshot vs hot-reload (minor).

### Performance

**Summary**: Hot-path choices are mostly sound — ArcSwap is the right
primitive, resolver is small enough that full rebuilds are
tolerable, watcher debounces per-path. Two easy wins: cache
sha256 on the resolver instead of recomputing per request, and
dedup `template-changed` broadcasts by template name.

**Findings**: sha256 recomputed per request (minor); full O(N×3)
resolver rebuild per change (minor); no per-template broadcast
coalescing (minor); live update refetches full per-tier content
(minor); `watch_dirs` dedup by raw PathBuf rather than canonical
form (suggestion).

### Standards

**Summary**: Aligns well with established standards — kebab-case
SSE event names, `Option<String> + skip_serializing_if` for
nullable JSON, camelCase Rust→JSON via serde, Chip palette
(neutral/indigo/green) and design tokens all correctly applied.
Two real issues: detail endpoint path mismatch left unresolved;
new `sha256` field departs from project-wide `sha256-<hex>`
convention.

**Findings**: Endpoint path mismatch unresolved (major); bare-hex
vs prefixed convention departure (minor); `TemplateChanged` missing
`rename_all = "camelCase"` (minor); content-hash label lacks ARIA
(minor); empty `aria-hidden` placeholder div (minor); hardcoded
plugin-default fallback (minor).

## Re-Review (Pass 2) — 2026-05-19

**Verdict:** COMMENT

The revisions cleanly address every Pass 1 finding — both criticals
(lost-update race, canonicalisation mismatch) and all 13 majors —
through structurally sound mechanisms: `TemplateChangeHandler` +
`TierPathIndex` extraction, single-consumer mpsc-serialised
rebuilds, build-snapshot-store ordering, `Option<String>` sha256 in
the SSE payload with skip-when-absent, additive doc-overlap
precedence, recursive watch, prefixed `sha256-<hex>` on the wire,
shared frontend helpers (`templateKeysForEvent`, `getWinningTier`,
`template-tier.ts`), digest-from-fixture in tests, and real
`dispatchSseEvent` driving the live-update test.

Two new major correctness concerns surface in the revised design,
both pointing at the same architectural seam (the rebuild-queueing
boundary): the `TierPathIndex` canonicalisation falls back to raw
paths for tier files that don't exist at startup (the common case
for user/config overrides added later), and the bounded mpsc with
`try_send` silently drops the terminal rebuild event under
sustained burst with no log signal. Both are addressable with
focused, small edits.

The remaining new findings are minor (recursive-watch canonicalize
syscall cost, AC12 1s test budget coupling real bugs to CI
slowness, AC7 work-item wording not flagged for follow-up alongside
AC8) or suggestions (drop the unused `async` on `try_handle`, add
dedicated `TierPathIndex` unit tests, hoist a shared
canonicalise-or-self helper).

### Previously Identified Issues

**Pass 1 Criticals**
- 🟢 **Correctness**: Lost-update race on concurrent ArcSwap::store — **Resolved** (single-consumer mpsc serialises rebuilds end-to-end)
- 🟢 **Correctness**: Canonicalisation mismatch between event path and config-held tier paths — **Resolved** (TierPathIndex canonicalises at startup); see new finding for the absent-at-startup variant

**Pass 1 Majors**
- 🟢 **Architecture**: Template branch precedence shadowing doc-changed events — **Resolved** (additive `try_handle` returning bool, no early-return)
- 🟢 **Architecture**: `watcher::spawn` parameter explosion 8→11 — **Resolved** (TemplateChangeHandler collaborator, +1 param)
- 🟢 **Code Quality**: Template branch overloads on_path_changed_debounced — **Resolved** (handler extraction)
- 🟢 **Code Quality**: Bare `return` in async fn — **Resolved** (additive call, no early return)
- 🟢 **Correctness**: Emptying winning tier produces no SSE event — **Resolved** (Option<String> sha256 in payload, unconditional broadcast on rebuild)
- 🟢 **Correctness**: Early `return` skips doc flow for tier+doc overlap — **Resolved** (additive precedence)
- 🟢 **Correctness**: `name_for_tier_path` `Option<String>` misses shared tier files — **Resolved** (Vec<String> via TierPathIndex)
- 🟢 **Correctness**: Non-recursive watch misses nested / temp-rename — **Resolved** (RecursiveMode::Recursive)
- 🟢 **Test Coverage**: SSE-to-DOM live-update not end-to-end — **Resolved** (Phase 6 test drives real dispatchSseEvent)
- 🟢 **Test Coverage**: AC11 not asserted end-to-end — **Resolved** (digest computed from fixture via @noble/hashes)
- 🟢 **Test Coverage**: Watcher branch lacks debounce/rapid-edit coverage — **Resolved** (7 new watcher unit tests added)
- 🟢 **Test Coverage**: First-row-position assertion fragile — **Resolved** (compareDocumentPosition)
- 🟢 **Standards**: Detail endpoint path mismatch — **Partially resolved** (plan narrative resolves; work item AC7 wording follow-up not flagged in Migration Notes alongside AC8)

**Pass 1 Minors and suggestions** — all 20+ resolved or partially-resolved with the partial cases acknowledged as acceptable tradeoffs at current scale.

### New Issues Introduced

**Major**

- 🟡 **Correctness**: Canonicalisation fallback when tier file is absent at startup
  **Location**: Phase 2 §6 — `TierPathIndex::build`
  `canonicalize.unwrap_or(raw)` keys the index on the raw path
  when the tier file doesn't exist. After the user creates the
  file, event-path canonicalisation succeeds → canonical key
  differs → silent miss. User-override and config-override files
  are routinely absent at startup — this affects the primary use
  case.

- 🟡 **Correctness**: Silent event drop on bounded mpsc with no log signal
  **Location**: Phase 2 §6 — `TemplateChangeHandler::try_handle`
  `let _ = self.tx.try_send(...)` discards both the event and the
  error. Under sustained burst (slow IO, NFS, antivirus) the
  consumer stalls, the queue saturates, and the **terminal** state
  event drops — UI stuck on the previous hash with no diagnostic.
  Departs from the existing watcher's `tracing::warn!` overflow
  convention (watcher.rs:43).

**Minor (cross-cutting theme: operability of silent-drop)**

- 🔵 **Architecture / Code Quality / Performance / Standards**: All four lenses flagged the silent-drop pattern from different angles — at minimum, log on `TrySendError::Full`/`Closed` to match the existing watcher convention.

**Minor**

- 🔵 **Correctness**: Non-existent tier-parent directory silently skipped (pre-existing watcher behaviour, propagated to template path)
- 🔵 **Correctness**: Per-path debounce keyed on raw path, not canonical — same logical file via different representations bypasses coalesce
- 🔵 **Test Coverage**: mpsc full-channel back-pressure path is unasserted
- 🔵 **Test Coverage**: Recursive-watch scoping (non-markdown sibling files trigger no rebuild) is unasserted
- 🔵 **Test Coverage**: Build-snapshot-store causal pairing unasserted (burst A→broadcast→B→broadcast each with own digest)
- 🔵 **Test Coverage**: AC12 1-second budget asserted via `findByText` timeout — couples real-bug failure with CI slowness
- 🔵 **Code Quality**: sha256 derivation expression duplicated between `build()` and tests — extract a `content_sha256(&str) -> Option<String>` helper
- 🔵 **Code Quality**: TemplateChangeHandler::spawn 5-arg signature borderline data-clump
- 🔵 **Code Quality**: Watcher tests assert end-to-end broadcast behaviour from inside `watcher.rs` — consider relocating to a handler module
- 🔵 **Architecture**: `try_handle` is `async` but does no awaits — signal of design-intent ambiguity around `try_send` vs `send().await`
- 🔵 **Architecture**: Recursive watch makes template-changed implicitly inherit the `is_markdown` doc filter — couples template tiers to `.md` extension by convention
- 🔵 **Performance**: Per-fs-event `canonicalize` syscall on the recursively-widened watch surface for non-tier markdown files
- 🔵 **Performance**: Across-tier broadcast multiplicity for the same template (two tier edits = two broadcasts with identical sha256) only partially resolved
- 🔵 **Standards**: AC7 work-item phrasing (`/api/library/templates/{name}`) still in work item; only AC8 wording follow-up flagged in Migration Notes
- 🔵 **Standards**: `canonical_or_self` helper introduced in `server.rs`; pre-existing inline canonicalisation in watcher uses a slightly different idiom — hoist a single shared helper
- 🔵 **Standards**: New types' file-location convention is implicit (consolidate in `watcher.rs` or extract `template_watcher.rs`)

**Suggestion**

- 🔵 **Test Coverage**: `TierPathIndex` has no dedicated unit test — currently exercised only indirectly via watcher tests
- 🔵 **Test Coverage**: Drag-deferred `template-changed` test spec lacks the explicit "exactly once each key" / "zero calls before drag release" precision applied to the direct-dispatch test
- 🔵 **Performance**: Full TemplateResolver re-allocation per rebuild — note the allocation-shape next to the existing latency note as future-design context
- 🔵 **Architecture**: `Arc<ArcSwap<T>>` double-wrap is slightly redundant (cosmetic)

### Assessment

The plan is in materially better shape than Pass 1 — every blocker is closed, the design now has explicit serialisation guarantees and a clean collaborator boundary, and the test plan exercises both halves of every AC end-to-end. **COMMENT** verdict rather than APPROVE because two real correctness gaps remain (absent-at-startup canonicalisation + silent mpsc drop) and are worth a small follow-up edit. Neither is a structural rework; both are localised:

1. In `TierPathIndex::build`: canonicalise the **parent directory** + join the filename when the file itself doesn't exist (the parent is reliably present because the watcher already gates on it).
2. In `TemplateChangeHandler::try_handle`: match on `TrySendError`, emit `tracing::warn!` on `Full` and `tracing::error!` on `Closed`. Optionally, swap the bounded `mpsc` for a `tokio::sync::watch` ("latest dirty path wins") since the consumer rebuilds the whole resolver anyway — eliminates overflow entirely.

The remaining items are minor improvements and can land either now or in a follow-up plan revision without blocking implementation.

## Pass 2 Follow-up Edits — 2026-05-19

The two new Pass 2 majors and several minors were addressed in a
targeted edit pass (no fresh lens review run; the edits below are
the implementation of the Pass 2 recommendations):

- 🟢 **Resolved**: Canonicalisation fallback for absent-at-startup
  tier files — `TierPathIndex::build` now uses a
  `canonicalise_path_or_ancestor` helper that walks up to the
  nearest existing ancestor, canonicalises it, and re-appends the
  descendant components. New `tier_path_index_tests` C, D, E lock
  the absent-at-startup, macOS, and walk-up cases.

- 🟢 **Resolved (via redesign)**: `try_send` silent drop on bounded
  mpsc — replaced the bounded mpsc with a `tokio::sync::Notify`
  + per-template sha256 diffing in the consumer. No bounded queue
  exists, so no overflow / silent-drop failure mode is possible.
  As a side benefit: the diffing dedups across-tier broadcast
  multiplicity (identical-bytes edits and non-winning-tier edits
  produce zero broadcasts), addressing the Pass 1 §3 performance
  partial-resolution too. `try_handle` becomes synchronous; the
  watcher call site drops `.await` and records `is_template` on
  the tracing span.

- 🟢 **Resolved**: AC7 wording follow-up — added alongside the
  existing AC8 bullet in Migration Notes.

- 🟢 **Resolved**: `content_sha256(content)` helper extracted in
  `templates.rs`, used by `build()` and reusable in any future
  consumer; eliminates the format-string duplication.

- 🟢 **Resolved**: New tests for previously-unasserted paths added
  to Phase 2 §9 — burst coalescing (F), back-pressure recovery
  (F1), no-op suppression (G), causal pairing (J), recursive
  non-tier sibling no-rebuild (N), and dedicated `TierPathIndex`
  tests (A-E).

- 🟡 **Remaining**: A handful of small items not addressed (AC12
  timing-vs-budget test refinement, file-location convention for
  the new types, `canonical_or_self` idiom inconsistency, pre-
  existing non-existent-parent watch-skip, `Arc<ArcSwap<T>>`
  double-wrap). All are minors or suggestions and don't gate
  implementation.

The plan is now in a shape where it can be implemented as written.
A Pass 3 lens review can be run to verify the redesign holds up
under a fresh look, but is optional.

## Re-Review (Pass 3) — 2026-05-19

**Verdict:** COMMENT

The Pass 2 follow-up edits hold up cleanly under a fresh six-lens
look. Notify-based coalescing, per-template sha256 diffing, the
`canonicalise_path_or_ancestor` walk-up helper, the
`content_sha256` extraction, and the expanded Phase 2 §9 test plan
all read correctly and resolve their target findings.

Two new majors surface from the fresh look, both with small,
localised fixes. Several minor follow-ups appear around test-seam
specification, tracing convention, and edge cases in path
handling. Performance is essentially all-resolved.

### Previously Identified Issues

**Pass 2 Majors**
- 🟢 **Correctness**: Canonicalisation fallback for absent-at-startup tier files — **Resolved on the index side** (`canonicalise_path_or_ancestor` walks up to nearest existing ancestor); see new finding for the symmetric event-path gap
- 🟢 **Correctness**: Silent event drop on bounded mpsc — **Resolved** via Notify-based design (no bounded queue exists)

**Pass 2 Minors** — all closed or accepted (e.g., AC12 1s budget remains as documented tradeoff; `try_handle` is now sync; test seams added; content_sha256 extracted; AC7 wording bullet added).

### New Issues Introduced

**Major**

- 🟡 **Correctness**: Event-path canonicalisation still uses naive fallback
  **Location**: Phase 2 §7 — watcher `try_handle` call site
  `let canonical = tokio::fs::canonicalize(&path).await.unwrap_or_else(|_| path.clone());` falls back to the raw event path when the file no longer exists. On a **delete** of a winning-tier file (`git checkout`, `rm`, editor atomic-rename transient deletes), `canonicalize` fails because the inode is gone; the raw path isn't symlink-resolved; the index keyed via `canonicalise_path_or_ancestor` (resolved) doesn't match the event's naive raw path. The deletion never produces a `template-changed` broadcast — AC10 silently fails for the deletion variant.
  **Fix**: Use `canonicalise_path_or_ancestor(&path).await` in the watcher event path too. Add a unit test pinning the delete case: write tier content, await first broadcast, `remove_file`, assert a second broadcast with `sha256: None`.

- 🟡 **Architecture**: Unsupervised consumer task is a silent SPOF
  **Location**: Phase 2 §6 — `TemplateChangeHandler::spawn`
  The `tokio::spawn`'d consumer holds the only path from `Notify` permits to SSE broadcasts. A panic inside `TemplateResolver::build` (or any future `.unwrap()` regression) kills the task; nothing restarts it; `try_handle` continues to store Notify permits no one consumes. The `ArcSwap` keeps serving the last-known-good resolver via the API, masking the failure entirely. Live updates silently stop for the lifetime of the process.
  **Fix**: Either wrap the loop body in `catch_unwind` with `tracing::error!` and a brief retry, or document explicitly that `TemplateResolver::build` is infallible by construction and crashing the consumer is an accepted tradeoff. At minimum, add a `Drop` guard or final `tracing::error!` so the task's death is observable.

**Minor (cross-cutting)**

- 🔵 **Code Quality / Standards**: `tracing::Span::current().record("is_template", …)` at the watcher call site departs from the server crate's uniform event-style tracing (`tracing::debug!(field = …, "…")`). No `#[instrument]` spans exist in the server crate, so the field record may also silently drop at runtime. Fix: switch to `tracing::debug!(file = %canonical.display(), is_template, "watcher dispatched fs event")`.

**Minor**

- 🔵 **Correctness**: Relative paths in `cfg.templates` short-circuit the ancestor walk to raw, missing index matches. Fix: normalise to absolute via `current_dir()` join before the walk, or validate at startup.
- 🔵 **Correctness**: Paths containing `..` components are not lexically normalised when canonicalisation fails. Edge case for configs shared across monorepos.
- 🔵 **Correctness**: Consumer task has no shutdown handle. Today benign (runtime cancels on drop); pre-empts a future graceful-shutdown extension breaking silently.
- 🔵 **Correctness**: `canonicalise_path_or_ancestor` runs one wasted syscall for single-component paths (`foo`) before the empty-cursor break fires.
- 🔵 **Code Quality**: `TemplateChangeHandler::spawn` body has grown — extract `compute_broadcasts(&new_resolver, &mut previous, names) -> Vec<(String, Option<String>)>` so the consumer loop reads as build → compute → store → broadcast.
- 🔵 **Test Coverage**: Test F's "bounded number of rebuilds" is too loose. Tighten to `<= 2` for 200 notifications fired before first wake.
- 🔵 **Test Coverage**: F1 / N / K reference test-only hooks (barrier, rebuild counter) not specified in §6's design. Sketch them as `pub(crate)` test seams or `#[cfg(test) fn for_test(...)]` so the test depends on a committed API.
- 🔵 **Test Coverage**: "Two tier files for the same template with identical winning content → zero broadcasts" — design intent stated in §6 but no test pins it.
- 🔵 **Test Coverage**: AC12 1s timeout still couples real-bug failure with CI slowness (carried over from Pass 2; accepted).
- 🔵 **Architecture**: Walk-up canonicalisation misses if an intermediate component becomes a **symlink** after startup (the ancestor-canonical key won't match the symlink-resolved event canonical). Narrow edge case; document explicitly.
- 🔵 **Architecture**: "Templates have no frontend write path" invariant lives only in plan prose; add a code comment at the `try_handle` call site so a future PR adding template editing notices the WriteCoordinator coupling.
- 🔵 **Architecture / Performance wording**: "permit-counting" — actually saturating-at-1; reword to "Notify holds a single pending permit; repeated notify_one before notified().await coalesce to one wake-up."
- 🔵 **Standards**: `tokio::sync::Notify` is a fresh primitive in the server crate. Add a one-line comment on the `notify` field documenting why it's the right primitive here so future maintainers don't add a second variant by reflex.

**Suggestion**

- 🔵 **Test Coverage**: ArcSwap stress test K should pin concrete bounds (N=32 readers ≥500ms, ≥100 writes) or use `loom`
- 🔵 **Test Coverage**: Drag-deferred test should also assert `expect(spy).not.toHaveBeenCalled()` before drag-release, symmetric with the direct-dispatch test
- 🔵 **Architecture**: `previous` initialisation correct but worth a one-line comment about ordering with the first `notify_one`
- 🔵 **Correctness**: `previous.get(name).cloned().flatten()` collapses "name absent" into "name present with None sha" — invariant correct today but breaks under future cfg hot-reload; assert or document the symmetry

### Assessment

The plan is materially complete and the two new majors are both
small, localised fixes — neither is a structural rework. The
correctness gap (event-path canonicalisation) is the more
important of the two because it silently breaks a normal user
action (deleting a winning-tier file); the architecture gap
(unsupervised SPOF) is a lower-probability but high-blast-radius
operability concern.

If both are addressed in a follow-up pass, the plan moves to
**APPROVE**-shape with only minors and suggestions remaining.
Alternatively, both are small enough to land alongside the
implementation rather than as another plan revision — the lens
findings document the intent precisely enough to be followed
during code review.

## Pass 3 Follow-up Edits — 2026-05-19

The two new Pass 3 majors plus the cross-cutting tracing concern
and several minors were addressed in a targeted edit pass:

- 🟢 **Resolved**: Event-path canonicalisation now uses
  `canonicalise_path_or_ancestor` in the watcher's
  `on_path_changed_debounced` (Phase 2 §7), symmetric with the
  index-side fix. Helper is `pub(crate)` so the watcher can call
  it. New test I1 pins the deletion case (write → broadcast →
  remove_file → broadcast with `sha256: None`).

- 🟢 **Resolved**: Consumer task panic isolation — `TemplateResolver::build`
  runs inside an inner `tokio::spawn`; panics surface as
  `JoinError`, get logged via `tracing::error!`, and the consumer
  loop continues. New test K1 pins the panic-survival invariant
  using a test-only `FileDriver` that panics on its second read.

- 🟢 **Resolved**: `tracing::Span::current().record(...)` replaced
  with the project's uniform event-style convention:
  `tracing::debug!(file = %canonical.display(), is_template,
  "watcher dispatched fs event")`. Matches the existing watcher
  call sites at `watcher.rs:55,124,151,166`.

- 🟢 **Resolved**: Test F's loose "bounded number" tightened to
  "≤2 rebuilds" with explicit gate-based timing control.

- 🟢 **Resolved**: Test seams (`gate_consumer()`,
  `rebuild_counter()`) now committed in §6 as `#[cfg(test)
  pub(crate)` API so F/F1/K/K1/N depend on a documented seam, not
  ad-hoc implementer choices.

- 🟢 **Resolved**: "Two tier files for the same template with
  unchanged winning content → zero broadcasts" now has dedicated
  test J1.

- 🟢 **Resolved**: Drag-deferred test (Phase 3) now spells out the
  "zero before drag release, exactly-once after" symmetric shape.

- 🟢 **Resolved**: `tokio::sync::Notify` field on
  `TemplateChangeHandler` now carries an inline doc comment
  explaining the choice over mpsc, matching the standards lens's
  "document the convention departure" suggestion.

- 🟢 **Resolved**: "Templates have no frontend write path"
  invariant captured as a code comment at the `try_handle` call
  site so a future template-edit feature notices the
  WriteCoordinator coupling.

- 🟢 **Resolved**: "permit-counting" wording corrected — Notify
  holds a single pending permit, repeated notify_one calls
  coalesce.

- 🟢 **Resolved**: ArcSwap stress test K now pins concrete bounds
  (N=32 readers, ≥500ms, ≥100 writes).

- 🟢 **Resolved**: `canonicalise_path_or_ancestor` doc comment
  now flags the intermediate-symlink-after-startup edge case
  explicitly.

- 🟡 **Remaining (minor, not addressed)**: Relative-path
  normalisation in `cfg.templates`, lexical `..` normalisation,
  consumer-task shutdown handle, walk-up loop empty-cursor
  micro-optimisation, `previous` HashMap "name absent vs None
  sha" comment, AC12 1s-budget split into functional + perf
  tests, `compute_broadcasts` extraction inside `spawn`. All are
  minor improvements or suggestions; none gate implementation.

The plan is now in **APPROVE**-adjacent shape: every major and
critical from three review passes is resolved, with only minor
polish remaining. Suitable for implementation as written.
