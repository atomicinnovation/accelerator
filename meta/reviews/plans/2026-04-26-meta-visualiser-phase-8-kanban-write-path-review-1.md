---
date: "2026-04-27T00:00:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-26-meta-visualiser-phase-8-kanban-write-path.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, security, safety, usability]
review_pass: 3
status: complete
---

## Plan Review: Phase 8 — Kanban Write Path

**Verdict:** REVISE

The plan is structurally strong: TDD-disciplined, well-scoped, with clean
phase boundaries (pure patcher → driver → handler → mutation → UX) and
deliberate reuse of existing patterns (ETag round-trip, SSE broadcast,
inline cfg(test) modules). However, this is the visualiser's first write
path against user-authored ticket files, and several correctness, safety,
and usability concerns are under-specified or actively wrong as drafted —
notably a TOCTOU window between the etag check and the atomic rename, a
race between `refresh_one` and the watcher's debounced rescan that
defeats the self-cause filter, missing fsync/permission preservation,
the conflation of 412/428 status semantics, and silent no-op drops with
no UX feedback.

### Cross-Cutting Themes

These issues were flagged by multiple lenses and deserve the most
attention.

- **TOCTOU between etag check and atomic rename**
  (architecture, correctness, security, safety, test-coverage) —
  Two concurrent PATCHes that observe the same on-disk etag both pass
  the precondition check, both rename their tempfile, and the second
  silently clobbers the first — last-writer-wins, not 412-on-conflict.
  Step 2.9 asserts behaviour that atomic rename alone does not
  guarantee. This breaks the optimistic-concurrency contract that the
  spec relies on, and makes the test flaky/over-promising.

- **PATCH handler + watcher both broadcast for the same write**
  (architecture, correctness, safety, test-coverage) —
  After persist, the handler calls `refresh_one` and broadcasts
  `doc-changed`; independently, the watcher's notify event triggers
  a debounced full `rescan` that broadcasts again with the same etag.
  The single-consume self-cause filter suppresses only the first event,
  so the originating tab still sees a redundant invalidation/refetch —
  exactly the flicker the design tries to avoid. Worse, `refresh_one`
  does not acquire `rescan_lock`, so the two paths can interleave and
  produce a torn entries view.

- **412 vs 428 conflation for missing If-Match**
  (architecture, code-quality, correctness, security, usability) —
  Five lenses agree. The plan returns 412 with `{currentEtag}` for both
  "stale If-Match" and "no If-Match at all". RFC 7232 reserves 428
  Precondition Required for the missing-header case; conflating them
  hides distinct failure modes (client bug vs concurrency conflict),
  leaks the etag to clients that demonstrated no prior knowledge, and
  forces frontend `ConflictError` to fire on programmer errors.

- **Self-cause filter design — module-scoped, single-consume, TTL fragile**
  (architecture, code-quality, correctness, safety) —
  The `Map<string, number>` is an implicit global with no DI seam,
  breaking the existing `makeUseDocEvents(factory)` pattern and
  complicating test isolation. Single-consume semantics fail when one
  write produces multiple broadcasts (above). TTL cleanup is lazy; with
  intermittent SSE the map grows monotonically.

- **Hand-rolled YAML line patcher — under-specified edge cases**
  (architecture, code-quality, correctness) —
  Tests cover double-quoted values and inline comments, but not
  single-quoted (`'todo'`), block scalars (`|`/`>`), anchors
  (`&x todo`), flow-style mappings, or BOM. The plan duplicates fence
  detection that already exists in `frontmatter::parse` rather than
  sharing a helper, inviting drift.

- **Idempotent same-value PATCH still broadcasts and updates mtime**
  (architecture, code-quality, correctness, safety) —
  Step 3.14 accepts that no-op PATCHes still rename, refresh, and
  broadcast. This produces redundant SSE traffic, pokes the watcher
  via mtime change, and contradicts the "events mean change"
  invariant. A short-circuit when patcher returns identical bytes
  would close the loop.

- **`..` substring path check is brittle defence-in-depth**
  (security, safety) —
  `doc_rel.contains("..")` over-rejects legitimate filenames like
  `2026..04-foo.md` and provides no real protection on its own. The
  canonicalize+prefix check is the actual control; the substring check
  reads as load-bearing in step 3.10 but is not.

### Tradeoff Analysis

- **Optimistic UX vs feedback for silent no-ops** — The plan optimises
  for invisible success but leaves silent no-ops (Other-column drops,
  same-column drops, intra-column reorder) with zero feedback,
  particularly painful for keyboard/screen-reader users. Adding aria-live
  acknowledgements adds verbosity that mouse users don't need; gating
  by input modality (announce only for keyboard-initiated drops) is the
  best balance.

- **Server-side allowlist clarity vs trait abstraction**
  (architecture vs code-quality) — Encoding "tickets only" inside the
  FileDriver couples transport to v1 product policy, but lifting it
  into the handler means losing defence-in-depth at the trait boundary.
  Recommend keeping both layers but parameterising the driver with a
  writable-roots set rather than a hard-coded ticket-root check.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Correctness / Security / Safety / Test-Coverage**: TOCTOU between etag check and rename
  **Location**: Phase 2 §2 sequence + Step 2.9 test
  Read→check→patch→rename is not atomic. Two concurrent matching writes both pass the etag check and both rename, with last-writer-wins. Test 2.9 over-promises that "one returns EtagMismatch". Suggest per-path mutex inside the driver, or post-rename re-read+verify, and reframe Step 2.9 to deterministic ordering (barrier) or honest invariant.

- 🟡 **Architecture / Correctness / Safety / Test-Coverage**: PATCH handler and watcher double-broadcast
  **Location**: Phase 3 step 11 + watcher.rs:97-141
  The handler-side broadcast and the watcher's debounced rescan both fire `doc-changed` for the same write with the same etag. Single-consume self-cause filter only swallows the first. Suggest either coordinating via `rescan_lock` and emitting once, or recording the etag with a count to absorb both. Add a test pinning exactly-one-event-per-PATCH semantics.

- 🟡 **Architecture / Correctness**: refresh_one races with watcher rescan
  **Location**: Phase 2 §3 + indexer/watcher
  `refresh_one` takes `entries.write()` independently of `rescan_lock`. A concurrent `rescan` swap wholesale-replaces the map and silently overwrites `refresh_one`'s update. Suggest sharing `rescan_lock` between the two paths, or using the FileDriver's returned `FileContent.etag` as the broadcast etag instead of round-tripping through the indexer.

- 🟡 **Architecture / Code-Quality / Correctness / Security / Usability**: 412/428 conflation
  **Location**: Phase 3 §2 step 7
  Missing `If-Match` and stale `If-Match` both return 412 with `{currentEtag}`. Suggest 428 (or 400) with no body for missing header; reserve 412+`{currentEtag}` strictly for stale match. Update the frontend `ConflictError` to fire only on 412.

- 🟡 **Architecture / Code-Quality / Correctness / Safety**: Self-cause filter is a hidden global with single-consume semantics
  **Location**: Phase 4 §2 + Step 4.6-4.9
  Module-scoped `Map` breaks DI consistency, complicates test isolation, doesn't generalise to other mutations, and consumes on hit (so duplicate broadcasts aren't suppressed). Suggest a `createSelfCauseRegistry()` factory plus either count-based registration or "expire on TTL only, no consume on hit".

- 🟡 **Architecture / Code-Quality / Correctness**: Hand-rolled line-based YAML patcher under-tests its grammar
  **Location**: Phase 1 implementation note + test list
  Single-quoted values, block scalars (`|`/`>`), anchors (`&x todo`), flow-style mappings, and BOM-prefixed files aren't covered. Suggest either explicit rejection (`UnsupportedValueShape`) with tests, or hybrid validate-via-serde-then-line-replace. Also extract a shared `frontmatter::fence_offsets()` helper to deduplicate fence detection.

- 🟡 **Architecture / Code-Quality / Correctness / Safety**: Idempotent same-value PATCH still mutates state
  **Location**: Phase 1 step 1.7 / Phase 3 step 3.14
  Identical-byte writes still rename (mtime bump), refresh, and broadcast. Causes watcher feedback and SSE noise. Suggest short-circuit when `patcher::patch_status` returns input bytes: return existing etag, skip persist+refresh+broadcast.

- 🟡 **Architecture**: Ticket-only policy leaks into the FileDriver transport abstraction
  **Location**: Phase 2 §1 (FileDriver trait)
  `OnlyTicketsAreWritable` and the hard-coded tickets-root check encode v1 product policy in the transport layer. Suggest a writable-roots set on the driver, with the handler choosing which root to pass.

- 🟡 **Architecture**: Single `*path` route serving GET+PATCH hides API shape
  **Location**: Phase 3 §1 (route registration)
  matchit workaround means `/frontmatter` suffix is invisible in the route table. Suggest a doc-comment at both registration site and handler, or evaluate `/api/tickets/*path/frontmatter` as a tickets-specific route bypassing matchit's limitation.

- 🟡 **Code-Quality**: `write_frontmatter` mixes orthogonal concerns in one body
  **Location**: Phase 2 §2
  Seven sequential steps (canonicalise, ticket-root verify, read, etag check, patch, atomic-write, recompute) in one function. Suggest factoring `verify_writable_ticket_path` and `atomic_write_bytes` helpers; the public method becomes a short coordinator.

- 🟡 **Code-Quality**: Status passed as primitive string risks duplication
  **Location**: Phase 4 §1 / Phase 3 §6
  `["todo","in-progress","done"]` is duplicated across patcher constant, handler validation, frontend `KanbanColumnKey`, and `resolveTargetColumn`. Suggest a Rust enum `TicketStatus` with serde rename — single source of truth, type-safety on the wire.

- 🟡 **Test-Coverage**: SSE assertion uses `try_recv` without synchronisation
  **Location**: Phase 3 step 3.2
  Flaky if broadcast is even slightly async. Suggest `tokio::time::timeout(Duration::from_millis(100), rx.recv()).await`.

- 🟡 **Test-Coverage**: Idempotent-patch test contract is ambiguous
  **Location**: Phase 3 step 3.14
  Doesn't pin whether SSE is or isn't broadcast on no-op writes. Pin the contract one way or the other; tightly coupled to the idempotent short-circuit suggestion above.

- 🟡 **Test-Coverage**: Self-cause TTL test depends on time-source choice
  **Location**: Phase 4 step 4.9
  `vi.useFakeTimers()` only advances the comparison if the module reads `Date.now()`/`performance.now()`. Inject `now()` for testability or document the fake-timer config required.

- 🟡 **Security / Safety**: TOCTOU between canonicalize and rename target
  **Location**: Phase 2 §2 step 6
  Plan doesn't make explicit that `parent_dir` and `target` for `tempfile::persist` come from the *canonicalised* path. A symlink swap on the original path between canonicalize and persist could redirect the rename. Suggest deriving both from the canonicalised path; consider `renameat` with a captured directory fd.

- 🟡 **Safety**: File permissions and ownership lost on rewrite
  **Location**: Phase 2 §2 step 6
  `NamedTempFile` defaults to `0o600`; `persist` overwrites the original's mode. Suggest reading original `Permissions` and applying to the temp file before persist; add a Unix test asserting mode preservation.

- 🟡 **Safety**: No fsync of file or parent directory — power-loss durability not guaranteed
  **Location**: Phase 2 §2 step 6
  `tempfile::persist` does not by default fsync. Suggest `temp_file.as_file().sync_all()` before persist and a parent-dir `sync_all()` after; add comment documenting the ordering.

- 🟡 **Usability**: Other-swimlane drop is silently ignored
  **Location**: Phase 5 §3 + "What We're NOT Doing"
  Card visually rebounds with no announcement. Screen-reader users get nothing. Suggest aria-live announcement explaining Other accepts only externally-set values; visual hint via `isOver` outline.

- 🟡 **Usability**: Conflict message is misleading and not actionable
  **Location**: Phase 4 §5
  "Refresh and try again" — but SSE invalidation already refreshes. Suggest "This ticket was changed elsewhere; the board has refreshed. Try the move again if it still applies." Consider including `currentStatus` in the 412 body.

- 🟡 **Usability**: Silent no-ops for keyboard users
  **Location**: Phase 5 §3 (early returns) + Steps 5.9/5.10/5.11
  Keyboard-initiated drops with no announcement leave the user unsure whether the action registered. Suggest brief aria-live acknowledgement for keyboard-completed no-ops.

- 🟡 **Usability**: Conflict announcement uses polite, not alert
  **Location**: Phase 5 §4
  `aria-live=polite` may queue behind in-progress speech and be missed. Suggest `role="alert"` for genuine conflicts; reserve polite for non-blocking acknowledgements.

- 🟡 **Usability**: No focus management on rollback
  **Location**: Phase 4 §3 onError + Phase 5 §3
  Cache reversion remounts the card; keyboard focus is lost. Suggest `requestAnimationFrame(() => cardEl.focus())` after rollback; add Step 5.8 assertion.

- 🟡 **Usability**: Tab B mid-drag interrupted by Tab A's broadcast
  **Location**: Phase 4 (self-cause + use-doc-events)
  Inbound SSE invalidation while a drag is in progress remounts items and may abort the gesture. Suggest queueing/coalescing invalidations while `isDragging`; add a Step 5.13 covering the scenario.

#### Minor

- 🔵 **Architecture**: Idempotent PATCH still emits doc-changed (subset of major above) — short-circuit recommendation.
- 🔵 **Architecture**: Patcher duplicates fence detection — extract `frontmatter::fence_offsets`.
- 🔵 **Architecture**: Atomic-rename concurrent test over-promises (subset of major TOCTOU).
- 🔵 **Code-Quality**: Suffix-strip workaround undocumented at call site — add doc-comment with matchit issue link.
- 🔵 **Code-Quality**: `data-droppable-column` test seam leaks into production markup — prefer behavioural test.
- 🔵 **Test-Coverage**: Step 5.4 keyboard-sensor fallback assertion is vacuous — decide upfront, or move to manual smoke.
- 🔵 **Test-Coverage**: Step 5.6 hook-extraction seam couples test to internals — fold into 5.7's behavioural test.
- 🔵 **Test-Coverage**: Step 2.6 symlink test is platform-sensitive — `#[cfg(unix)]` gate with comment.
- 🔵 **Test-Coverage**: Step 3.3 has two scenarios collapsed — split into 3.3a (indexer not refreshed) and 3.3b (indexer refreshed).
- 🔵 **Test-Coverage**: Step 1.5 needs a paired test for body `---` *before* the real frontmatter close.
- 🔵 **Test-Coverage**: useMoveTicket cache assertions risk query-key brittleness — assert via `getQueryData` not literal key.
- 🔵 **Test-Coverage**: No test for handler+watcher exactly-one-event invariant.
- 🔵 **Correctness**: ETag quote-stripping is not RFC 7232 compliant (weak/list/`*` ignored) — document and reject explicitly.
- 🔵 **Correctness**: Suffix-strip can misroute `subdir/frontmatter` files; pin segment-boundary behaviour.
- 🔵 **Correctness**: Self-cause TTL grows unbounded if SSE drops — opportunistic prune or hard cap.
- 🔵 **Correctness**: Broadcast etag should be `FileContent.etag` from the write, not re-read from indexer post-refresh.
- 🔵 **Correctness**: `resolveTargetColumn` doesn't verify `over.id` is a known card — explicit `entriesByRelPath.has(...)` + test.
- 🔵 **Security**: `..` substring rejection brittle — drop in favour of canonicalize-only or per-segment check.
- 🔵 **Security**: No Origin/Referer pin on PATCH — add CSRF defence-in-depth before any future CORS misconfiguration.
- 🔵 **Security**: Body structs missing `#[serde(deny_unknown_fields)]` — silently accepts mixed bodies today.
- 🔵 **Security**: Handler joins against `project_root`, not tickets root — derive `abs` from `entry.path` for layered trust.
- 🔵 **Safety**: Cross-filesystem persist falls back to copy+delete — surface a distinct `CrossFilesystem` error variant.
- 🔵 **Safety**: CRLF preservation only tested on uniform input — add mixed-ending test.
- 🔵 **Safety**: Self-cause map size unbounded — FIFO cap + drop on SSE reconnect.
- 🔵 **Safety**: No documented recovery path for corrupted tickets — note `git checkout` in CHANGELOG/troubleshooting.
- 🔵 **Safety**: Tickets-root canonicalisation invariant should be tested with symlinked root.
- 🔵 **Usability**: Empty-column drop discoverability — visual outline on dragOver via `isOver`.
- 🔵 **Usability**: No busy state during slow PATCH — pending opacity/spinner after 250ms.
- 🔵 **Usability**: Conflict message dismissal pattern unspecified — auto-clear timeout or close button.
- 🔵 **Usability**: Other column gets droppable wired but rejects drops — disable droppable or skip registration.
- 🔵 **Usability**: PointerSensor activation distance unspecified — `{ distance: 5 }` to disambiguate click vs drag.
- 🔵 **Usability**: Card-on-card drop semantics inconsistent same- vs cross-column — document and consider always treating as "move to that card's column".

### Strengths

- ✅ Exemplary TDD discipline: every step has a named failing test before
  implementation; the test list reads as a behavioural spec.
- ✅ Clean phase boundaries with a pure functional core (patcher), an
  imperative shell (driver/handler), and clear UX wiring on top.
- ✅ Defence-in-depth on path safety, status allowlist, and ticket-only
  enforcement (handler + driver layers).
- ✅ Reuses established codebase patterns deliberately: ETag round-trip
  from `doc_fetch`, inline `cfg(test)` mod from `frontmatter.rs`,
  `vi.stubGlobal`/`vi.spyOn` from existing fetch tests, `tower::oneshot`
  integration pattern from Phase 7.
- ✅ Atomic rename via `tempfile::NamedTempFile::persist` is the correct
  primitive for crash-safe writes.
- ✅ ETag is computed from fresh on-disk bytes inside the driver, not from
  the (potentially stale) indexer cache — the right place to verify.
- ✅ Existing process-wide controls (1 MiB body limit, 30s timeout,
  host-header guard, loopback bind) cover most DoS/origin-confusion
  vectors without further work.
- ✅ Five inner phases keep build green between steps; matchit limitation
  acknowledged with concrete workaround.
- ✅ Empty-column drops addressed via `useDroppable` (a Phase-7 dead-zone
  bug).
- ✅ Self-cause SSE filter has its own dedicated tests, including the
  "still invalidates for unknown etags" regression check.
- ✅ Existing fixtures (`seeded_cfg_with_tickets`) reused; no new
  infrastructure.
- ✅ Explicit "What We're NOT Doing" prevents scope creep.

### Recommended Changes

Ordered by impact:

1. **Close the etag→rename TOCTOU** (addresses: TOCTOU finding, Step 2.9).
   Add a per-path async mutex inside `LocalFileDriver` covering the
   read→check→rename window. Update Step 2.9's assertion to a deterministic
   barrier-driven test that one of two truly-concurrent writes returns 412.

2. **Coordinate handler broadcast with the watcher** (addresses:
   double-broadcast, refresh_one race, self-cause single-consume).
   Either share `rescan_lock` between `refresh_one` and the watcher's
   `rescan`, or suppress the handler-side broadcast and rely solely on the
   watcher (simpler but adds debounce latency). Pin the contract with a
   test asserting exactly one `doc-changed` event per PATCH. Use the
   `FileContent.etag` returned by the write for both the response `ETag`
   header and the broadcast.

3. **Short-circuit idempotent PATCHes** (addresses: idempotent broadcast,
   watcher feedback, Step 3.14 ambiguity).
   When the patcher returns input bytes unchanged, skip persist, skip
   refresh_one, skip broadcast, return 204 with the existing etag. Update
   Step 3.14 to assert no SSE event is emitted.

4. **Distinguish 428 from 412** (addresses: 412/428 conflation,
   conflict-message confusion, security oracle).
   Return 428 Precondition Required (no body) for missing `If-Match`;
   reserve 412 + `{currentEtag}` strictly for stale `If-Match`. Frontend
   `ConflictError` maps only to 412. Update tests 3.3 and 3.4 accordingly.

5. **Redesign the self-cause registry** (addresses: hidden global, single
   consume, TTL fragility).
   Wrap in `createSelfCauseRegistry()` factory matching the
   `makeUseDocEvents(factory)` DI pattern. Switch from "consume on hit" to
   "expire on TTL only" (etag SHA-256 collision risk is negligible). Inject
   a `now()` time source for testability. Add hard size cap with FIFO
   eviction.

6. **Tighten the patcher grammar** (addresses: YAML edge cases, fence
   detection duplication).
   Either explicitly reject single-quoted/block-scalar/anchor `status:`
   forms with `UnsupportedValueShape` and tests, or extend the patcher to
   handle them. Extract `frontmatter::fence_offsets()` so parser and
   patcher share fence detection.

7. **Preserve permissions and fsync** (addresses: permission drift, durability).
   Read original `Permissions` and apply to the tempfile before persist.
   Call `temp_file.as_file().sync_all()` before persist; `sync_all()` on
   the parent directory after. Add tests for both.

8. **Lock the rename target to canonicalised path** (addresses:
   canonicalize→use TOCTOU).
   Specify in Phase 2 step 6 that `parent_dir` and `target` are derived
   from the canonicalised path. Optionally use `renameat` with a directory
   fd captured during the safety check.

9. **Replace `..` substring check with per-segment validation** (addresses:
   brittle path check).
   Split on `/`, reject `..`, `.`, empty, backslash, NUL segments.
   Update Step 3.10 to assert defence comes from canonicalize+prefix.

10. **Improve drag UX feedback** (addresses: silent no-ops, conflict
    messaging, focus management, tab-B-mid-drag).
    Announce no-op rejections to keyboard/screen-reader users via aria-live;
    upgrade conflict announcements to `role="alert"`; restore focus to the
    rolled-back card; suppress SSE invalidation while a drag is in progress.
    Rewrite the conflict message to reflect the auto-refresh.

11. **Add `serde(deny_unknown_fields)` and Origin pin** (addresses:
    smuggle-fields, future CORS misconfiguration).
    Cheap and locks the policy in at the moment the first write surface
    ships.

12. **Lift ticket-only policy out of FileDriver** (addresses: trait
    abstraction leak).
    Driver takes a writable-roots set; handler picks tickets root for v1.
    Anticipates v2's broader writable scope without trait churn.

13. **Tighten remaining test contracts**:
    - `tokio::time::timeout` for SSE recv (Step 3.2).
    - `#[cfg(unix)]` on Step 2.6 with comment.
    - Split Step 3.3 into a/b for indexer-refreshed vs not.
    - Add body-`---`-before-frontmatter-close test (Step 1.5 pair).
    - Drop the test-only `data-droppable-column` attribute in favour of
      behavioural assertion.
    - Decide Step 5.4 keyboard-sensor approach upfront; remove fallback.
    - Fold Step 5.6 into Step 5.7's integration test.

14. **Document recovery posture** (addresses: corrupted ticket recovery).
    CHANGELOG note: tickets are written in place; recover via
    `git checkout` if a write produces unexpected output.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is well-structured, TDD-driven, and respects the
existing architectural patterns (FileDriver trait, Indexer, SseHub
broadcast, ETag round-trip). However, it has architectural rough edges:
the FileDriver write surface adds policy concerns into a transport-level
abstraction; the handler write path races the watcher's debounced rescan,
affecting SSE ordering and cache consistency; the self-cause filter
introduces hidden cross-cutting state coupling; and the 412/428 status
conflation needs tightening.

**Strengths**: pure-function patcher; functional core / imperative shell
separation; atomic write idiom; reuse of ETag round-trip pattern; single
SSE emit point preserved; Indexer::refresh_one with extracted build_entry
helper.

**Findings**: ticket-only policy in FileDriver (major); refresh_one vs
rescan race (major); module-scoped self-cause registry (major); 412 vs
428 (major); single `*path` route hides API shape (minor); patcher
duplicates fence detection (minor); atomic-rename test over-promises
(minor); idempotent PATCH still broadcasts (minor).

### Code Quality

**Summary**: TDD-disciplined and well-structured with clear pure-function
extraction. Concerns: hand-rolled YAML patcher complexity, 412/428
semantics conflation, primitive-string typing for status, module-scoped
self-cause global, long write_frontmatter body mixing concerns,
undocumented suffix-strip workaround, accepted leaky idempotent
broadcast, and test-only DOM attribute leaking into production.

**Strengths**: TDD discipline; pure-function extraction; deliberate
pattern reuse; defence-in-depth allowlists; explicit non-goals.

**Findings**: hand-rolled YAML patcher (major); 412/428 conflation
(major); module-scoped self-cause Map (major); status primitive obsession
(minor); suffix-strip undocumented at call site (minor); write_frontmatter
mixes concerns (minor); idempotent broadcast accepted without
justification (minor); test-only data-attribute leaks (minor).

### Test Coverage

**Summary**: Exemplary TDD with named tests acting as specifications.
Coverage of error paths, edge cases, and concurrency unusually thorough.
Gaps: concurrent-write determinism, SSE assertion mechanics, self-cause
TTL, and a few mutation-coupling concerns.

**Strengths**: TDD per-step; first-class error paths; strong patcher
edge-case coverage; correct test-pyramid placement; reuse of fixtures;
explicit regression protection.

**Findings**: concurrent-write test non-deterministic (major);
`try_recv` SSE assertion flaky (major); idempotent test contract
ambiguous (major); TTL test depends on time source (major); keyboard
sensor fallback vacuous (minor); hook-extraction couples test to
internals (minor); symlink test platform-sensitive (minor); Step 3.3
collapses two scenarios (minor); body-`---` test pair missing (minor);
query-key brittleness (minor); no exactly-one-broadcast test (minor).

### Correctness

**Summary**: Strong TDD coverage and correct identification of
sensitive seams. Real correctness gaps: PATCH+watcher double-broadcast
defeats self-cause filter; refresh_one races full rescan; etag check is
TOCTOU vs atomic rename so concurrent matching writes both succeed; YAML
line editing under-specifies single-quoted/block-scalar/anchor cases.

**Strengths**: atomic rename primitive; defence-in-depth path checks;
etag verified against fresh on-disk bytes; explicit edge-case tests;
pure patcher; correct identification of matchit limitation.

**Findings**: PATCH+watcher double broadcast (major); refresh_one vs
rescan race (major); etag→rename TOCTOU (major); single-consume
self-cause filter (major); YAML edge cases under-specified (minor); 412
vs 428 (minor); etag string handling not RFC 7232 compliant (minor);
suffix-strip edge cases (minor); self-cause TTL unbounded (minor);
broadcast etag from indexer not write (minor); resolveTargetColumn
fallback (minor); idempotent PATCH triggers watcher rescan (minor).

### Security

**Summary**: Right controls broadly applied (allowlist, canonicalize+prefix,
ETag, body cap, host-header guard). Serious risks: TOCTOU between
canonicalize and rename; optimistic-concurrency check not atomic with
write; `..` substring rejection brittle and incidental. CSRF/DoS
mostly covered by loopback bind, host guard, body limit, and
non-simple PATCH preflighting — but a few defence-in-depth gaps
deserve attention.

**Strengths**: layered path-safety; minimal write surface; concurrent
test intent; atomic rename primitive; strong-validator ETag with fresh
re-read; existing process-wide controls; 412 body leaks no extra info.

**Findings**: canonicalize→rename TOCTOU (major); optimistic-concurrency
not atomic (major); `..` substring brittle (minor); 428 conflation +
etag oracle (minor); no Origin pin (minor); missing
`deny_unknown_fields` (minor); handler joins project_root not tickets
root (minor).

### Safety

**Summary**: Several thoughtful safeguards (atomic rename, If-Match,
allowlist, ticket-root). Underspecified: cross-filesystem behaviour,
no fsync of contents or parent directory, file permissions/ownership
inherited from NamedTempFile (0o600) overwrites original mode,
idempotent-patch mtime updates trigger watcher.

**Strengths**: atomic rename primitive; If-Match concurrency; layered
allowlist; symlink-escape test; concurrent-write test intent; line-by-line
patcher avoids YAML round-trip; explicit error variants; existing
debounce + rescan_lock.

**Findings**: file permissions/ownership lost (major); no fsync (major);
idempotent PATCH watcher feedback (major); etag-check vs rename TOCTOU
(major); cross-filesystem persist edge case (minor); CRLF preservation
weak test (minor); path-escape substring brittle (minor); self-cause
size unbounded (minor); no recovery-path documentation (minor);
tickets-root canonicalisation untested with symlinked root (minor).

### Usability

**Summary**: Thoughtfully scoped, with good attention to optimistic
update plumbing, ETag round-trip, and self-cause SSE filtering. UX
details in the drag-drop flow are under-specified: silent Other-swimlane
no-ops, missing announcements for client-side validation, weak conflict
messaging, ambiguous focus/keyboard behaviour on rollback, polite-aria
where alert is needed.

**Strengths**: optimistic update with rollback; self-cause flicker
prevention; empty-column droppables; respects existing DX conventions;
manual checklist exercises keyboard and Other-swimlane paths; preserved
URL contract.

**Findings**: silent Other-column drop (major); conflict message not
actionable (major); silent client-side validation no-ops (major);
polite vs alert aria-live (major); no focus management on rollback
(major); tab-B mid-drag interrupted (major); empty-column drop
discoverability (minor); no busy state during PATCH (minor); conflict
dismissal pattern (minor); 412/428 confusing message (minor); Other
column gets droppable but rejects drops (minor); PointerSensor
activation distance unspecified (minor); card-on-card drop semantics
inconsistent (minor).

## Re-Review (Pass 2) — 2026-04-27

**Verdict:** REVISE

The revision substantially closes the prior review's findings: 24 of 28
prior majors are fully resolved (per-path mutex closes etag TOCTOU,
shared `rescan_lock` closes refresh-vs-rescan race,
`recently_self_written` deduplicates handler+watcher broadcasts,
non-consuming `has()` registry handles duplicate echoes, 428 vs 412
distinction implemented, fsync + permission preservation added,
`writable_roots` lifts policy out of FileDriver, `TicketStatus` enum
threads end-to-end, `serde(deny_unknown_fields)` and Origin pin land,
DropOutcome with announcements + role=alert/status split + focus
restore + drag-suppress all wired). However, the redesign introduces
**6 new major findings** — the most consequential are two correctness
bugs in the watcher-dedup wiring (mutex release ordering and
canonical-vs-watcher-path key mismatch) that would cause the
"exactly one event per PATCH" invariant to break intermittently in
production.

### Previously Identified Issues

#### Resolved (24 of 28 prior majors)

- ✅ **Architecture**: Ticket-only policy in FileDriver — Resolved (`writable_roots` lifts policy)
- ✅ **Architecture**: Watcher race vs handler broadcast — Resolved (`recently_self_written` + shared `rescan_lock`)
- ✅ **Architecture**: Module-scoped self-cause registry — Resolved (factory + DI + context)
- ✅ **Architecture**: 412/428 conflation — Resolved
- ✅ **Code Quality**: 412/428 obscured semantics — Resolved
- ✅ **Code Quality**: Module-scoped self-cause Map — Resolved
- ✅ **Test Coverage**: Concurrent-write test non-deterministic — Resolved (Barrier + per-path mutex assertion)
- ✅ **Test Coverage**: SSE `try_recv` flaky — Resolved (`tokio::time::timeout`)
- ✅ **Test Coverage**: Idempotent contract under-specified — Resolved (mtime + no-broadcast pinned)
- ✅ **Test Coverage**: Self-cause TTL test on real timers — Resolved (injectable `now()`)
- ✅ **Correctness**: PATCH+watcher double broadcast — Resolved in design (see new ordering issue below)
- ✅ **Correctness**: refresh_one races rescan — Resolved (shared `rescan_lock`)
- ✅ **Correctness**: Etag→rename TOCTOU — Resolved (per-path mutex)
- ✅ **Correctness**: Single-consume self-cause — Resolved (non-consuming `has()`)
- ✅ **Security**: Canonicalize→rename TOCTOU — Resolved (rename target derived from canonical path)
- ✅ **Security**: Optimistic-concurrency atomicity — Resolved (per-path mutex)
- ✅ **Safety**: File permissions lost — Resolved (read-then-restore)
- ✅ **Safety**: No fsync — Resolved (file + parent dir `sync_all()`)
- ✅ **Safety**: Idempotent watcher feedback — Resolved (driver short-circuit)
- ✅ **Safety**: Etag→rename TOCTOU — Resolved
- ✅ **Usability**: Other-swimlane silent no-op — Resolved (announcement + disabled droppable)
- ✅ **Usability**: Same-column silent no-op — Partially resolved (keyboard yes, pointer no — see new finding)
- ✅ **Usability**: aria-live polite vs alert — Resolved (split into two regions)
- ✅ **Usability**: No focus management on rollback — Resolved (data-relpath + rAF)
- ✅ **Usability**: Tab B mid-drag interrupted — Resolved (drag-suppress queue)

#### Partially resolved

- ⚠️ **Architecture**: Single `*path` route GET+PATCH — Documentation
  fix only; route still serves two semantically distinct endpoints
  through one handler. Acceptable as ADR.
- ⚠️ **Code Quality**: Hand-rolled YAML patcher complexity — Bounded
  by `UnsupportedValueShape` allowlist with shared `fence_offsets`
  helper. Better, but recogniser branches will still accrete.
- ⚠️ **Code Quality**: `write_frontmatter` mixes orthogonal concerns
  — Still present (and arguably worse: redesign added writable_roots,
  per-path mutex, idempotent short-circuit, EXDEV mapping, perm
  preservation without decomposing the method). See new finding below.
- ⚠️ **Usability**: Conflict copy — New copy is better but still
  asserts "the board has refreshed" before invalidation may have
  settled. See new finding.
- ⚠️ **Usability**: Conflict dismissal pattern — `announcement` got an
  8s auto-clear; `conflict` did not. See new finding.

### New Issues Introduced

#### Major

- 🟡 **Correctness**: Race between mutex release and `recently_self_written` insert
  **Location**: Phase 2 §2 (driver releases mutex before returning) + Phase 3 §2 step 11 (handler inserts after driver returns)
  The driver releases the per-path mutex inside `write_frontmatter`, but the handler only inserts into `recently_self_written` *after* the driver returns. Between persist (which queues the inotify event) and the insert, the watcher's debounce can fire and emit a duplicate broadcast. The frontend's non-consuming filter masks user-visible damage but the "exactly one event" invariant breaks; Step 3.15 may flake. **Fix**: insert before releasing the mutex (pass the registry into the driver, or perform the insert in the handler before the driver call).

- 🟡 **Correctness**: `recently_self_written` keyed by canonical path while watcher event path may not be canonical
  **Location**: Phase 2 §4 (key) vs `watcher::on_path_changed_debounced` (event source)
  Handler inserts the canonicalised path; watcher receives the path as the notifier delivered it (project_root.join(rel_path) — not symlink-resolved). Lookup misses on macOS where `/var ↔ /private/var` etc. produce different shapes. Watcher proceeds to broadcast a duplicate. **Fix**: canonicalise the watcher event path before lookup, or store the watcher-shaped path; add a symlinked-tickets-dir test.

- 🟡 **Code Quality**: `write_frontmatter` still mixes 8 concerns
  **Location**: Phase 2 §2
  The redesign added more responsibilities to a single ~80-line method. **Fix**: extract `acquire_canonical_writable_path`, `read_and_check_etag`, `atomic_write_preserving_perms` helpers; coordinator becomes a six-line flow.

- 🟡 **Usability**: Conflict copy promises a refresh that may not have happened
  **Location**: Phase 4 §5
  "the board has refreshed" — but invalidation is async and the user can read the message before the cache settles. **Fix**: either set conflict in `onSettled`, or soften copy to "the board will update shortly".

- 🟡 **Usability**: `role="alert"` conflict has no auto-clear or dismiss affordance
  **Location**: Phase 5 §5
  `announcement` auto-clears at 8s; `conflict` only clears on next dragstart or successful mutation. Stale alerts persist indefinitely. **Fix**: 30s auto-clear on conflict, plus a visible dismiss button.

- 🟡 **Usability**: Pointer-initiated same-column drop produces zero feedback
  **Location**: Phase 5 §4 (`no-op-same-column` switch arm)
  Plan only announces for keyboard. dnd-kit's snap-back is brief and disabled under `prefers-reduced-motion: reduce`. **Fix**: always populate polite announcement on `no-op-same-column`, OR gate on `prefers-reduced-motion`.

#### Minor (highlights)

- 🔵 **Architecture**: `AppState` accruing cross-cutting write coordination state (recently_self_written, path_locks, origin guard) — extract a `WriteCoordinator` module.
- 🔵 **Architecture**: Per-path mutex map has no eviction — bounded by repo size for v1, but document or cap.
- 🔵 **Architecture**: `SelfCauseProvider` context relies on convention for new mutation hooks to register — wrap in `useDocMutation` to enforce.
- 🔵 **Code Quality**: Server self-cause set lacks the abstraction the frontend got — introduce `SelfWriteRegistry` mirroring frontend.
- 🔵 **Code Quality**: Patcher would benefit from a `classify_value` helper to keep rejection branches discoverable.
- 🔵 **Code Quality**: Focus restore via `document.querySelector` — prefer ref-based `Map<relPath, HTMLElement>`.
- 🔵 **Code Quality**: `event.activatorEvent` snippet binds undeclared `event` — spell out `(event: DragEndEvent) => { const { active, over } = event; ... }`.
- 🔵 **Test Coverage**: Step 4.10 number reused in two sections — renumber for unambiguous tracking.
- 🔵 **Test Coverage**: Focus-restore assertion races rAF in jsdom — pin `waitFor` synchronisation in step.
- 🔵 **Test Coverage**: Drag-suppress coalescing test under-specifies queue-key contract — name keying explicitly.
- 🔵 **Test Coverage**: Origin-pin test missing `Origin: null`, empty, scheme/port mismatch, `localhost` vs `127.0.0.1` cases.
- 🔵 **Test Coverage**: Step 3.15 sleep-based assertion against real watcher debounce is flake-prone — inject a controllable clock or call `on_path_changed_debounced` directly.
- 🔵 **Test Coverage**: Step 2.19 has no ordering control between `rescan` and `refresh_one` — could pass for the wrong reason.
- 🔵 **Test Coverage**: SSE reconnect-mid-drag interaction not pinned (registry reset vs queued invalidations).
- 🔵 **Test Coverage**: Body containing `---` *before* the real frontmatter close still lacks a test for `fence_offsets`.
- 🔵 **Security**: Per-path mutex map unbounded — same as architecture finding.
- 🔵 **Security**: Origin pin allows no-Origin requests through (curl ergonomics) — document the threat model decision in an ADR.
- 🔵 **Security**: `UnsupportedValueShape { reason: String }` could leak file content if reason ever interpolates raw input — constrain to closed enum of labels.
- 🔵 **Security**: If-Match parser doesn't pin trimming, empty-string, malformed-quote handling — add Test 3.4b sub-cases.
- 🔵 **Security**: No rate limiting on PATCH — document as accepted for loopback-only v1.
- 🔵 **Safety**: Server `recently_self_written` map has no FIFO cap — mirror frontend's 256.
- 🔵 **Safety**: CRLF preservation tested for uniform + status-line cases but not mixed-body — add fixture.
- 🔵 **Safety**: Cross-filesystem test ignored by default — add unit test for the error mapping.
- 🔵 **Safety**: Recovery instruction assumes git-tracked, clean working tree — caveat the CHANGELOG note.
- 🔵 **Usability**: `cssEscape` is not built-in — use `CSS.escape(cardId)` (standard DOM API).
- 🔵 **Usability**: 8s auto-clear may truncate slow screen readers — bump to 15s or clear on next interaction.
- 🔵 **Usability**: Non-conflict 4xx/5xx errors fall through to raw `FetchError.message` — add `messageFor` switch.
- 🔵 **Usability**: Other column shows no proactive read-only affordance during drags — optional `cursor: not-allowed` + badge.
- 🔵 **Usability**: No busy state during PATCH (prior #8 unaddressed) — `data-saving="true"` overlay after 250ms.

### Assessment

The plan has moved from "fundamentally racy and structurally
mis-layered" to "structurally sound with two new wiring bugs and a
handful of UX polish gaps". The two new major correctness findings
(mutex-release ordering, canonical-vs-watcher-path key mismatch) are
both narrow, mechanical fixes — change the order of two operations
and add a `canonicalize` call in the watcher consult path. The three
new major UX findings (conflict copy, conflict dismissal, pointer
no-op feedback) are all small wording or boolean changes. The
remaining major code-quality concern (`write_frontmatter` length) is
addressable in the implementation rather than blocking on plan
revision.

Recommended path forward: a third revision pass focused on the 6 new
majors (estimate: ~30 minutes of edits), after which the plan is
ready to implement. The 32 new minors are individually small and can
be triaged into "fix now" vs "track as follow-up" during that pass.

## Pass 3 — Author edits addressing the 6 new majors — 2026-04-27

**Verdict (post-edit):** APPROVE — pending implementation.

This pass is author-edits only (no fresh agent run). The 6 new
majors from Pass 2 are each addressed by a targeted plan change:

### Resolution of Pass-2 majors

- ✅ **Correctness (mutex release vs register race)**: Driver
  signature now takes an `on_committed: impl FnOnce(&Path)`
  callback that the handler supplies. The driver invokes it
  *while still holding the per-path mutex*, after persist+fsync,
  before releasing. The handler's closure calls
  `WriteCoordinator::mark_self_write(&canonical)`. By the time
  the watcher's debounce expires, the suppression entry is
  present. Broadcast-failure recovery: handler unmarks if
  broadcast fails so the watcher takes over.

- ✅ **Correctness (canonical vs watcher path mismatch)**:
  `on_path_changed_debounced` canonicalises the event path
  before consulting `WriteCoordinator::should_suppress`. New
  Step 3.16 (`#[cfg(unix)]`) creates a symlinked tickets dir
  fixture and pins the dedup contract under
  canonical-vs-watcher path divergence.

- ✅ **Code Quality (write_frontmatter mixes 8 concerns)**:
  Extracted three private helpers — `acquire_canonical_writable_path`,
  `read_and_check_etag`, `atomic_write_preserving_perms`. Public
  `write_frontmatter` is now a six-step coordinator with the
  helpers handling locking+perms, etag check, and durable write
  respectively. `WriteCoordinator` extracted as a sibling
  module (mirroring frontend `SelfCauseRegistry`'s shape).

- ✅ **Usability (conflict copy)**: Rewrote to future tense:
  "This ticket was changed elsewhere — the board will update
  shortly. Try the move again if it still applies." Plus added
  bespoke `messageFor` switch covering 428/403/400/5xx so
  non-conflict errors no longer leak raw `if-match-required` /
  `patch URL must end with /frontmatter` strings to the alert
  region.

- ✅ **Usability (conflict no auto-clear/dismiss)**: Conflict
  region now auto-clears after 30s (longer than announcement's
  15s for higher-stakes copy) and renders a visible × dismiss
  button with `aria-label="Dismiss conflict notice"`. Two new
  tests: 5.16b (auto-clear) and 5.16c (dismiss button).

- ✅ **Usability (pointer same-column no feedback)**: `no-op-same-column`
  switch arm always populates the polite announcement
  ("Card returned to {column}.") regardless of input modality —
  serves keyboard users, `prefers-reduced-motion` users, and
  mouse users alike, with zero interruption cost (role=status
  is non-blocking). Step 5.13 updated to assert both keyboard
  and pointer drags announce.

### Other improvements bundled in

- `CSS.escape(cardId)` (standard DOM API) replaces the imaginary
  `cssEscape` helper in the focus-restore selector.
- Announcement auto-clear bumped from 8s → 15s (avoids
  truncating slow screen readers).
- Step 3.15 reframed to use a synthesised watcher event call
  rather than sleep-based timing — deterministic CI behaviour.
- New Step 3.17 pins that idempotent PATCHes do not register
  self-write (driver short-circuits before invoking
  `on_committed`).
- Implementation-sequence checklist updated for new tests and
  the `WriteCoordinator` module + watcher canonicalisation
  + driver `on_committed` wiring.

### Residual minor findings (not addressed; track as follow-ups)

The 32 minors from Pass 2 are not all addressed in this pass.
The plan-level decision is to leave them for implementation-time
judgment or post-v1 cleanup:

- **Architecture**: `SelfCauseProvider` context relying on
  convention; `FrontmatterPatch` closed enum.
- **Code Quality**: Patcher `classify_value` helper; focus
  restore via ref instead of querySelector;
  `event.activatorEvent` snippet bind; useMoveTicket onSettled
  comment.
- **Test Coverage**: Step 4.10 number reuse; focus-restore rAF
  synchronisation; queue-key contract; Origin-pin edge cases;
  Step 2.19 ordering control; SSE reconnect mid-drag; missing
  body-`---`-before-fence pair test.
- **Security**: Per-path mutex map eviction; Origin no-Origin
  ADR; `UnsupportedValueShape` reason as enum; If-Match parser
  edge cases; rate limiting.
- **Safety**: Server self-write FIFO cap (now addressed —
  WriteCoordinator includes it); CRLF mixed-body fixture;
  cross-fs unit test; recovery instruction caveat.
- **Usability**: 8s → 15s announcement (now addressed);
  `cssEscape` → `CSS.escape` (now addressed); non-conflict
  error messages (now addressed); Other-column proactive
  affordance; busy-state during PATCH.

### Assessment

The plan is now ready to implement. The original critical
correctness and safety risks (etag TOCTOU, canonicalize→rename
race, double broadcast, file-permissions clobber, no fsync) are
all closed by structural mechanisms with deterministic tests.
The redesign-introduced wiring bugs from Pass 2 are now closed
by ordering discipline (register-while-locked, canonicalise
event path) with tests that pin the contracts. UX-blocking
issues (silent no-ops, conflict-message confusion, focus loss
on rollback) are all addressed.

The remaining minors are real but individually small and do not
warrant another revision pass. Implementation can proceed; the
TODO list at the bottom of the plan is the source of truth for
test sequencing.
