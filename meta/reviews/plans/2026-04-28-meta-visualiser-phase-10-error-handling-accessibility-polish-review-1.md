---
date: "2026-04-28T17:17:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-28-meta-visualiser-phase-10-error-handling-accessibility-polish.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, security, usability, compatibility, standards]
review_pass: 3
status: complete
---

## Plan Review: Phase 10 — Error handling, accessibility, polish

**Verdict:** REVISE

The plan is well-structured, TDD-shaped throughout, and decomposes the
work into seven independently shippable inner phases that respect the
existing project conventions. Strong points include explicit test
seams (factory injection, pure `computeBackoff`, `Deps` thunks),
typed errors via `thiserror`, a sensible choice of `file-rotate` for
size-based rotation, and a serious WCAG audit table with concrete
ratios. However, three convergent issues across four lenses prevent
APPROVE: the `version_header` middleware is wired *innermost* rather
than outermost in the tower stack (so the spec-mandated header will
not land on guard-rejected responses); the launcher's existing
`>>"$LOG_FILE"` redirect collides with the new in-process file-rotate
writer, producing interleaved bytes and silently defeating the size
cap; and the `ReconnectingEventSource` state machine has three
overlapping flags whose interactions don't match the wrapper's stated
contract (post-`close()` resurrection, first-open-after-error storm,
re-entrant `onerror`). Phase 10.1's bash test additions also
reference helpers that don't exist (`make_stub_binary`) and would
turn every existing happy-path test red without an explicit fix to
`make_project()`.

### Cross-Cutting Themes

- **Layer ordering for `version_header`** (flagged by:
  architecture, correctness, compatibility) — the plan asserts
  outermost behaviour but the snippet wires innermost. Guard
  rejections (403 from `host_header_guard` / `origin_guard`) will
  arrive without the version header, contradicting the plan's own
  manual-verification bullet, and the existing tests don't cover
  this path. Three lenses independently identify this.
- **`ReconnectingEventSource` state machine** (flagged by:
  code-quality, correctness) — three flags (`wasErrored`,
  `hasEverConnected`, `state`) over-specify one piece of
  information; `scheduleReconnect()` is not idempotent under
  re-entrant `onerror`; `close()` doesn't set a guard that prevents
  later resurrection; the `onopen` handler uses the wrong gate to
  decide whether to fire `onReconnect`.
- **File-rotate plumbing** (flagged by: correctness, compatibility,
  security, code-quality) — the call site uses a syntactically
  invalid `#[cfg(unix)] Some(0o600)` argument; rotated segments may
  not inherit `0o600` on multi-user hosts; the launcher's
  `>>"$LOG_FILE"` continues to write to the same path as the
  in-process writer; and the test's "5.1 MB synchronous write"
  assertion has not been validated against the actual file-rotate
  0.7 API.
- **Global tracing subscriber in tests** (flagged by: code-quality,
  test-coverage, correctness, compatibility) — `init_for_test`
  calls `try_init` on the process-global subscriber, so test
  ordering determines which test's writer is bound; subsequent
  tests' assertions are flaky and may pass for the wrong reason.
- **Bash test additions break the harness** (flagged by:
  test-coverage, code-quality, correctness) — `make_stub_binary` is
  not a real helper; `make_project()` doesn't seed
  `meta/tmp/.gitignore`, so adding the sentinel rejects every
  existing happy-path test until the helper is updated; `assert_eq`
  argument order in the new tests is inverted relative to the
  existing harness; the sentinel placement before the reuse
  short-circuit can lock users out of an already-running server.
- **`/api/info` mounting and queryKey casing diverge from
  conventions** (flagged by: architecture, standards) — sibling
  `api/<x>.rs` modules export handler functions, not
  `Router<Arc<AppState>>` builders; existing `queryKeys` use
  kebab-case tuple keys (`'doc-content'`, `'template-detail'`); and
  `/api/info` is more naturally a sibling of `/api/healthz` (an
  infrastructure route) than a member of `api::mount` (the stateful
  domain endpoint).
- **`role="status"` for static warnings is the wrong ARIA semantic**
  (flagged by: standards, usability) — `role="status"` is for
  polite live updates, not page-load warnings (the malformed
  banner) or a footer (the version surface). The reconnecting
  indicator is the only correct use.
- **Hard-coded contrast hex tests duplicate the values they're
  meant to lock** (flagged by: code-quality, test-coverage) —
  drift between `:root` tokens and the test's hex literals
  re-introduces the bug class the test exists to prevent.

### Tradeoff Analysis

- **Spec compliance vs. RFC 6648** (security, compatibility,
  standards) — the spec mandates `X-Accelerator-Visualiser`, but
  RFC 6648 has deprecated `X-` prefixes since 2012. Three lenses
  flag this with low-to-medium urgency. The header is brand-new in
  this phase, so the cost of unprefixing it now is zero; the cost
  rises after first release. Recommend renaming both the spec and
  the plan together, but acceptable to defer with an explicit
  rationale.
- **Sidebar accretion vs. component splitting** (architecture,
  code-quality, usability) — placing the version footer + the
  reconnecting indicator inside `<Sidebar>` keeps related UI in one
  component, but turns Sidebar into a status-bar over time. Usability
  separately notes that on a collapsed/mobile sidebar the
  reconnecting signal disappears entirely. Splitting `<SidebarFooter>`
  (or `<StatusBar>`) early addresses both concerns; keeping it
  monolithic is faster to ship.
- **`invalidate-all` on reconnect vs. predicate-scoped invalidation**
  (correctness, usability) — bare `queryClient.invalidateQueries()`
  matches the spec's "invalidate-all on reconnect" but also evicts
  `serverInfo` (which has `staleTime: Infinity`), causing the
  sidebar version footer to flicker. Either accept the flicker
  (tiny cost) or pass a predicate that excludes session-stable
  keys.

### Findings

#### Critical

- 🔴 **Test Coverage**: Sentinel check breaks every existing
  launch-server test; `make_stub_binary` does not exist
  **Location**: Phase 10.1 Step 10.1.a
  The new `test_initialised_project_proceeds` references
  `make_stub_binary` (only `make_fake_visualiser` exists in
  `test-helpers.sh`), and the existing `make_project()` helper
  doesn't seed `meta/tmp/.gitignore`. Adding the sentinel before
  any `mkdir -p` will cause every one of the 12 existing
  happy-path tests to fail with the new init-not-run error. The
  plan acknowledges this only in a parenthetical aside.

- 🔴 **Correctness**: Dual writers to `server.log` will interleave
  bytes and corrupt JSON lines
  **Location**: Phase 10.2 Migration Notes; interaction with
  `launch-server.sh:179`
  The launcher's `nohup … >>"$LOG_FILE" 2>&1 &` continues to
  capture stdout/stderr by appending to the same path the new
  in-process `file-rotate` writer opens. Two independent writers
  on the same path: kernel `O_APPEND` is per-`write(2)` atomic but
  not coordinated across processes, so stray `eprintln!`,
  `panic!`, or runtime warnings will land mid-line inside JSON
  records. After in-process rotation, the nohup'd shell still
  holds the original fd and writes into the *rotated* file,
  defeating the size cap.

- 🔴 **Correctness**: `ReconnectingEventSource` may miss the
  invalidate-all sweep on the most common boot-time recovery path
  **Location**: Phase 10.5 Step 10.5.b — `connect()` /
  `wasErrored` handling
  The gate `if (this.wasErrored && this.hasEverConnected)` skips
  `onReconnect` on the first successful open after a from-boot
  failure (server unreachable, then reachable). That's the case
  most users hit on the first launch of the day, and the spec's
  "invalidate-all on reconnect" rule is silently violated. There
  is no test for `error → backoff → first-ever-open`.

#### Major

- 🟡 **Architecture / Compatibility / Correctness**:
  `version_header` middleware is wired innermost, not outermost
  **Location**: Phase 10.3 Step 10.3.b
  The snippet places `.layer(middleware::from_fn(version_header))`
  as the *first* `.layer(...)` call, which in tower is innermost.
  `host_header_guard` / `origin_guard` are added later → outermost
  → short-circuit before `version_header` runs. The plan's
  manual-verification bullet ("Forbidden requests … also carry the
  header") will not pass. Tests don't cover guard-rejection
  paths.

- 🟡 **Compatibility**: `FileRotate::new` call uses an invalid
  conditional argument
  **Location**: Phase 10.2 Step 10.2.b
  `#[cfg(unix)] Some(0o600)` cannot be applied to a single
  argument — it removes the arg on non-unix, producing an arity
  mismatch. file-rotate 0.7 has a fixed-arity constructor whose
  5th `Option<u32>` mode parameter exists on all targets and is
  ignored on Windows. Verify exact argument order
  (`(path, suffix_scheme, content_limit, compression, mode)`).

- 🟡 **Code Quality**: `ReconnectingEventSource` carries three
  overlapping state flags
  **Location**: Phase 10.5 Step 10.5.b
  `wasErrored`, `hasEverConnected`, and `state: ConnectionState`
  encode one piece of information three ways. The condition in
  `onopen` is doing the work of `state === 'reconnecting'`. Future
  modifications will almost certainly miss one of the three.
  Collapse to a single `state` field; derive everything from it.

- 🟡 **Code Quality / Correctness**: `onerror` and `close()` race;
  closed instances can resurrect themselves
  **Location**: Phase 10.5 Step 10.5.b
  `scheduleReconnect()` doesn't check `state === 'closed'` before
  setting a new timer; `close()` doesn't set a closed-flag and
  doesn't detach `source.onerror` before calling `source.close()`.
  Browsers (notably Firefox) can fire `error` after `close()`,
  resurrecting the wrapper. Also, repeated browser `error` events
  on a flapping connection reset `attempts` to whatever the
  current value is and replace the in-flight timer, defeating
  backoff.

- 🟡 **Code Quality**: `useServerInfo` test is a `.test.ts` but
  contains JSX
  **Location**: Phase 10.4 Step 10.4.c
  TypeScript rejects JSX in `.ts` files regardless of `tsconfig`
  settings. The test will fail to compile when first added,
  causing the TDD step to fail for the wrong reason.

- 🟡 **Code Quality / Test Coverage**: Test fixture mutates a
  single fake across reconnect attempts
  **Location**: Phase 10.5 Step 10.5.a
  `makeFakeSource` returns the same `fake` object on every
  factory call, so the test cannot distinguish "wrapper rebuilt
  the EventSource and re-bound handlers" from "wrapper retained
  stale handlers." Push fresh fakes into a `fakes[]` array per
  call.

- 🟡 **Code Quality / Test Coverage / Correctness /
  Compatibility**: `init_for_test` collides with the global
  tracing subscriber
  **Location**: Phase 10.2 Step 10.2.a
  `try_init` succeeds only the first time per process; subsequent
  calls silently no-op, so subsequent tests' `WorkerGuard`/writer
  is dropped immediately and the global subscriber emits errors.
  Tests pass or fail depending on cargo-test ordering. Use
  `tracing::subscriber::with_default(...)` for scoped subscribers,
  or move file-write tests to `tests/` (one process per test).

- 🟡 **Test Coverage**: Jitter makes the
  `advanceTimersByTime(1000)` assertion flake-prone
  **Location**: Phase 10.5 Step 10.5.a — "opens, errors, then
  reconnects after a backoff"
  `computeBackoff(0, Math.random())` returns any value in
  [800ms, 1200ms]. The test relies on the timer firing within
  1000ms; ~50% of CI runs will see a delay >1000ms. Either inject
  `random` via `ReconnectOpts` (deterministic seed in tests) or
  advance by 1500ms.

- 🟡 **Test Coverage**: `request_emits_trace_summary` test sketch
  is hand-wavy
  **Location**: Phase 10.2 Step 10.2.d
  The test body is a comment. `tower_http::trace::TraceLayer`
  emits events with field names that depend on which
  `DefaultMakeSpan`/`DefaultOnResponse` config is wired. Without
  crisp assertions, mutation-test sensitivity is zero.

- 🟡 **Test Coverage / Code Quality**: Hard-coded hex contrast
  values duplicate `:root` tokens
  **Location**: Phase 10.7 Sub-step C
  The test's `contrastRatio('#4b5563', '#ffffff')` literal is the
  same value the test exists to protect. CSS-only token
  regressions won't be caught. Either parse `global.css` for the
  custom-property values, or move tokens into a `tokens.ts` and
  import them into both CSS and tests.

- 🟡 **Test Coverage**: `rotates_at_five_megabytes` may not
  exercise rotation as the test assumes
  **Location**: Phase 10.2 Step 10.2.a
  The test relies on a synchronous 5.1 MB write triggering
  rotation, but the file-rotate 0.7 API contract on flush /
  byte-count semantics has not been verified by the plan. Do a
  10-minute smoke test against the actual API before committing
  to the assertion.

- 🟡 **Test Coverage**: Missing tests for mid-session state
  transitions
  **Location**: Phase 10.5 + 10.6
  No test renders `LibraryDocView` with `parsed`, dispatches a
  `doc-invalid` SSE event, and asserts the banner appears. No
  test for `EventSource` constructor throwing synchronously, for
  reconnect during `close()`, or for `useDocEvents` actually
  invalidating queries on `onReconnect`.

- 🟡 **Architecture / Standards**: `/api/info` mounting diverges
  from sibling api modules
  **Location**: Phase 10.4 Step 10.4.b
  Existing `api/<x>.rs` modules export handler functions; only
  `mount` calls `.route(...)`. The plan's `pub fn router()` in
  `api/info.rs` is structurally different. Also, `/api/info` is
  closer to `/api/healthz` (infrastructure, no `AppState` graph)
  than to the domain endpoints. Either match the existing handler
  pattern in `api::mount`, or place `/api/info` next to
  `/api/healthz` directly in `server.rs::build_router_with_spa`.

- 🟡 **Correctness**: Sentinel check before reuse short-circuit
  locks users out of already-running servers
  **Location**: Phase 10.1 Step 10.1.b
  The plan inserts the sentinel check immediately after `cd
  $PROJECT_ROOT`, which is *before* `launch-server.sh:46-58`'s
  reuse short-circuit. A user who deletes `meta/tmp/.gitignore`
  while the server is running cannot reconnect to it. Place the
  check after the reuse short-circuit and before the first
  `mkdir -p`.

- 🟡 **Correctness**: `invalidateQueries()` evicts `serverInfo`
  despite `staleTime: Infinity`
  **Location**: Phase 10.5 Step 10.5.c
  Bare `invalidateQueries()` ignores `staleTime` and refetches.
  After every reconnect the version footer briefly flickers to
  empty. Pass a predicate excluding the `['server-info']` key.

- 🟡 **Security**: Rotated log segments may not inherit `0o600`
  **Location**: Phase 10.2 Step 10.2.b
  `file-rotate`'s `mode` parameter governs the active file's
  open-time permissions; rotated siblings created post-rotation
  are subject to the process umask (typically 022 → 0o644).
  Combined with TraceLayer logging full request paths (which
  encode the user's project tree), rotated segments could become
  world-readable on multi-user hosts. Add a chmod after every
  rotation, or assert on segment permissions in a test.

- 🟡 **Usability**: Banner copy uses developer jargon
  ("Frontmatter")
  **Location**: Phase 10.6
  "Frontmatter" is a Markdown/Hugo term-of-art unfamiliar to
  many users. "We couldn't read this document's metadata header;
  showing the file as-is" reads more accessibly.

- 🟡 **Usability**: Kanban announcements omit ticket numbers
  **Location**: Phase 10.7 Sub-step B
  Tickets are disambiguated by `NNNN-` prefix, but the
  announcement uses only `entry.title`. Two tickets sharing a
  title are indistinguishable to a screen-reader user. Also,
  column ids (`'in-progress'`) are read kebab-case rather than
  human English; map to display labels.

- 🟡 **Usability**: No "Reconnected" confirmation when SSE
  recovers
  **Location**: Phase 10.5 Sub-step D
  The "Reconnecting…" indicator silently disappears. Both
  visual and screen-reader users get no explicit confirmation
  that data is fresh. Render a transient "Reconnected"
  message before clearing.

- 🟡 **Usability**: Init-not-run hint is ambiguous about which
  project to initialise
  **Location**: Phase 10.1
  "this project" doesn't disambiguate cwd / repo root /
  workspace. Include the resolved `PROJECT_ROOT` path in the
  hint or the JSON.

#### Minor

- 🔵 **Architecture**: Sidebar prop accretion (docTypes →
  versionLabel → connectionState) trends toward a status-bar
  **Location**: Phase 10.4 / 10.5
  Extract a `<SidebarFooter>` consuming `useServerInfo` and
  `useDocEvents` directly via hooks.

- 🔵 **Architecture**: `ReconnectingEventSource` exposes both
  `onopen` and `onReconnect`
  **Location**: Phase 10.5
  The wiring shows `onopen` set to a no-op placeholder; drop it
  from the public surface or document the distinction.

- 🔵 **Code Quality / Correctness**: State-observable test asserts
  initial `'closed'` state but subscriber registered in
  constructor never sees it
  **Location**: Phase 10.5 Step 10.5.a
  Either emit the initial state synchronously on first
  `setState` (drop the equal-state early return for the very
  first call) or document that subscribers must read initial
  state via a public getter.

- 🔵 **Code Quality**: `log::init` panics on duplicate global
  subscriber; `main` has no fallback
  **Location**: Phase 10.2 Step 10.2.b/c
  Bootstrap failures get a panic stack trace instead of the
  structured `eprintln!` exit-2 path. Use `try_init` and surface
  `LoggingError::AlreadyInitialised`.

- 🔵 **Code Quality**: Bash assertions invert argument order from
  the existing harness
  **Location**: Phase 10.1 Step 10.1.a
  Existing tests use `assert_eq "<message>" "<expected>"
  "<actual>"`; the plan's snippet uses `assert_eq "$rc" 1
  "<message>"`. Match existing convention.

- 🔵 **Code Quality**: `useDocEvents` snippet leaves dead
  branches and silently drops eager-invalidate-on-error
  **Location**: Phase 10.5 Step 10.5.c
  The wiring elides handler bodies into comments and
  no-longer-eagerly invalidates `['docs']` on error. Document
  the behaviour change in Migration Notes if intentional.

- 🔵 **Code Quality**: `Announcements` thunk closes over a stale
  value via `useMemo` deps
  **Location**: Phase 10.7 Sub-step B
  Thunk implies deferred reads, but rebuilding on every
  `entriesByRelPath` change defeats that. Either pass the Map
  directly or back the thunk with a `useRef`.

- 🔵 **Test Coverage**: No tests for `/api/info` error or
  malformed-body responses; Sidebar tests in 10.4 won't have
  `connectionState` after 10.5
  **Location**: Phase 10.4 / 10.5
  Add 500/404/empty-body tests for `useServerInfo`; specify in
  10.5 that 10.4's Sidebar tests be updated.

- 🔵 **Test Coverage**: `LoggingError::CreateDir` partial-creation
  path has no test
  **Location**: Phase 10.2
  Trivial regression-catcher missing.

- 🔵 **Correctness / Test Coverage**: Computed-backoff overflow at
  large attempts is correct today but brittle to refactor
  **Location**: Phase 10.5
  Cap `attempts` at 32; add a `computeBackoff(1000, 0.5)` sanity
  test.

- 🔵 **Correctness**: `_log_guard` lifetime is partial protection;
  panics / aborts can drop pending lines
  **Location**: Phase 10.2 Performance Considerations
  Document the trade-off explicitly, or install a panic hook
  that flushes synchronously to stderr.

- 🔵 **Correctness**: Connection state stored in a ref does not
  re-render the Sidebar
  **Location**: Phase 10.5 Step 10.5.c
  The snippet uses `connectionStateRef.current = s`; refs don't
  trigger renders. Use `useState` or `useSyncExternalStore`.

- 🔵 **Correctness**: `attempts` resets on every successful open
  — flapping connection never escalates backoff
  **Location**: Phase 10.5
  Either reset only after a sustained open window (~30 s), or
  document the behaviour explicitly.

- 🔵 **Compatibility**: `with_ansi(false)` is a no-op on the JSON
  formatter
  **Location**: Phase 10.2 Step 10.2.b
  Delete the call and its misleading comment.

- 🔵 **Compatibility**: TraceLayer + CompressionLayer interaction
  with SSE
  **Location**: Phase 10.2 Step 10.2.d
  Confirm `text/event-stream` responses arrive uncompressed;
  add an explicit `compress_when` predicate if not.

- 🔵 **Compatibility**: dnd-kit `Announcements` callback return-
  type strictness
  **Location**: Phase 10.7 Sub-step B
  Add explicit `: string | undefined` return annotations.

- 🔵 **Compatibility**: WorkerGuard drop on non-graceful shutdown
  may lose buffered log lines
  **Location**: Phase 10.2
  Confirm shutdown returns from `main` rather than
  `process::exit`.

- 🔵 **Standards**: `X-Accelerator-Visualiser` violates RFC 6648
  **Location**: Phase 10.3 / spec mandate
  Rename to `Accelerator-Visualiser-Version` (or similar) in
  spec and plan together; v1 is the cheap moment.

- 🔵 **Standards / Usability**: `role="status"` is the wrong ARIA
  role for the malformed-frontmatter banner
  **Location**: Phase 10.6
  Use a plain `<div>` with a styled "Warning:" prefix, or
  `role="note"`. Reserve `role="status"` for the dynamic
  reconnecting indicator.

- 🔵 **Standards**: `role="contentinfo"` on a footer nested in
  `<nav>` is a landmark misuse
  **Location**: Phase 10.4 Step 10.4.f
  Drop the role and use `aria-label`, or move the footer out
  of the `<nav>` into `RootLayout` as a sibling.

- 🔵 **Standards**: `queryKeys.serverInfo()` casing diverges from
  flat-tuple naming
  **Location**: Phase 10.4 Step 10.4.d
  Existing keys are kebab-case (`'doc-content'`,
  `'lifecycle-cluster'`). Use `['server-info']`.

- 🔵 **Standards**: WCAG version not specified (2.1 vs 2.2)
  **Location**: Phase 10.7 / Desired End State
  Pin the version; 2.2 adds 2.4.11 and 2.5.7 which intersect
  the plan's work.

- 🔵 **Usability**: Bumping `--color-muted` may cascade visually;
  no review pass scheduled
  **Location**: Phase 10.7 Sub-step C
  Add a manual-verification bullet to scan all `--color-muted`
  consumers, or split into `--color-muted-text` and
  `--color-muted-decorative`.

- 🔵 **Usability**: No accommodation for forced-colors /
  high-contrast mode
  **Location**: Phase 10.7 Sub-step A
  Add `@media (forced-colors: active) { :focus-visible {
  outline-color: Highlight; } }`.

- 🔵 **Usability**: 2px outline-offset may clip on dense kanban
  cards
  **Location**: Phase 10.7 Sub-step A
  Add a manual scan; consider `box-shadow` fallback for
  overflow:hidden ancestors.

- 🔵 **Usability**: Onboarding flow not in the manual smoke list
  **Location**: Phase 10 Manual smoke
  Add an explicit "first-contact in uninitialised project →
  init → re-invoke" walkthrough.

- 🔵 **Usability**: Sidebar-only placement of reconnecting
  indicator risks invisibility on collapsed/mobile sidebars
  **Location**: Phase 10.5 Sub-step D
  Either document "sidebar always visible at v1" in
  What-We're-NOT-Doing or render a fixed-position toast.

- 🔵 **Security**: Init sentinel can be trivially satisfied by an
  empty `.gitignore`
  **Location**: Phase 10.1
  Either match expected content or note the foot-gun in the
  threat-model commentary.

- 🔵 **Security**: Request paths in logs may include user document
  tree structure
  **Location**: Phase 10.2
  Document the disclosure surface in Performance/Security; the
  `0o600` posture is non-optional.

#### Suggestions

- 🔵 **Architecture**: Three independent `env!("CARGO_PKG_VERSION")`
  call sites after Phase 10
  **Location**: Phase 10.3 / 10.4
  Define `pub const VERSION: &str = env!("CARGO_PKG_VERSION");`
  at crate root.

- 🔵 **Architecture**: TraceLayer position rationale not
  documented
  **Location**: Phase 10.2 Step 10.2.d
  Add a comment on the `.layer(TraceLayer::...)` line
  documenting the timing-semantic intent.

- 🔵 **Test Coverage**: User-visible reconnecting indicator has
  no end-to-end coverage
  **Location**: Phase 10.5
  Consider a single Playwright test now or stub it for Phase 11.

- 🔵 **Security**: Run `cargo audit` after the dependency bump
  **Location**: Phase 10.2
  Both new crates are mature but the dependency-graph delta is
  worth a one-time check.

- 🔵 **Compatibility**: Add `Cache-Control` to `/api/info` for
  forward proxy compatibility
  **Location**: Phase 10.4
  `Cache-Control: no-cache` is robust against future tunnel /
  proxy exposure.

- 🔵 **Standards**: Consider richer `/api/info` payload (`name`,
  `version`)
  **Location**: Phase 10.4
  Industry convention; trivial extension point.

- 🔵 **Usability**: Version footer could link to GitHub release
  notes for support flows
  **Location**: Phase 10.4
  Out of v1 scope; flag in What-We're-NOT-Doing.

### Strengths

- ✅ Each inner phase pairs a clearly-failing test with its
  implementation, keeping the build green between steps and
  giving every new behaviour a regression anchor.
- ✅ Strong separation between `ReconnectingEventSource`
  (transport) and `useDocEvents` (dispatch) — the wrapper
  exposes the same `EventSource`-like surface so the existing
  `EventSourceFactory` injection point keeps working.
- ✅ Logging is extracted into a dedicated `log.rs` module with
  explicit `LoggingError`, `make_writer`, and `init` —
  single-responsibility public surface.
- ✅ Init sentinel runs before any directory creation, binary
  download, or PID-file write — leaves zero state on rejection.
  Reuses `die_json` and the same `<TMP_PATH>/.gitignore`
  sentinel that `config-summary.sh` uses.
- ✅ Phase ordering is genuinely shippable in isolation: bash,
  then server logging, then middleware, then endpoint, then
  frontend transport, then doc-page, then CSS/a11y.
- ✅ Resilience strategy is sound: bounded backoff (1s→30s,
  ±20% jitter), bounded log retention (3 segments × 5 MB),
  bounded request body and timeout, non-blocking writer.
- ✅ Plan correctly identifies that `tracing-appender` lacks
  size rotation and reaches for `file-rotate` rather than
  open-coding it.
- ✅ Observability is treated as first-class: structured JSON
  lines + request-summary trace layer + version-stamp header +
  JSON `/api/info` give three independent diagnosis vectors.
- ✅ Pure-function extraction is good in spots (`computeBackoff`
  is split out for tabular tests; `buildKanbanAnnouncements`
  takes a `Deps` struct rather than reading from React context).
- ✅ Endpoint URL `/api/info` and JSON error envelope
  `{error, hint}` match the project's existing conventions.
- ✅ WCAG AA contrast ratio targets correctly stated; audit
  table uses correct WebAIM-formula ratios.
- ✅ dnd-kit `accessibility.announcements` callbacks
  (`onDragStart`, `onDragOver`, `onDragEnd`, `onDragCancel`)
  match the canonical four-callback shape.
- ✅ TanStack Query sidebar version footer is conditional
  (omitted when serverInfo is absent) so it degrades gracefully
  if `/api/info` ever fails.

### Recommended Changes

Ordered by impact. Critical and high-confidence majors first.

1. **Fix the launcher dual-writer hazard** (addresses: dual writers
   to `server.log` corruption). After `log::init` succeeds inside
   the Rust process, redirect or close stdout/stderr so only the
   in-process writer touches `server.log`. Alternatively, redirect
   the launcher's nohup capture to a separate
   `server.bootstrap.log` used only for pre-init panics. Update
   Migration Notes to reflect the chosen split.

2. **Reorder `version_header` to be the outermost layer**
   (addresses: layer ordering theme — architecture, correctness,
   compatibility). Move
   `.layer(middleware::from_fn(version_header))` to be the *last*
   `.layer(...)` call in `build_router_with_spa`. Add an
   integration test that issues a request with a non-loopback
   Host header, asserts 403, and asserts the
   `x-accelerator-visualiser` header is present.

3. **Fix the `ReconnectingEventSource` state machine** (addresses:
   state-machine theme + critical "missed first-recovery"
   finding). Collapse `wasErrored` + `hasEverConnected` + `state`
   into a single `state: ConnectionState` field; have `onopen`
   fire `onReconnect` iff the previous state was `'reconnecting'`
   (this also fixes the "first error → first open" recovery
   case). Add a `closed` guard checked at the top of every state-
   mutating handler. Detach `source.onerror = null` inside
   `close()` before calling `source.close()`. Make
   `scheduleReconnect()` idempotent. Add tests for:
   error-then-first-open, repeated browser errors during
   reconnect, `close()` during `onerror`, and
   `EventSource`-constructor-throws.

4. **Fix the bash test harness breakage** (addresses: critical
   sentinel-test theme). Update `make_project()` to also create
   `meta/tmp/.gitignore`; replace the non-existent
   `make_stub_binary` with the existing `make_fake_visualiser`
   pattern; correct `assert_eq` argument order to match the
   harness convention. Place the sentinel check *after* the reuse
   short-circuit (so already-running servers aren't killed by a
   transient sentinel deletion) but before `mkdir -p $TMP_DIR`.

5. **Verify the `file-rotate` 0.7 API and harden the rotation
   test** (addresses: file-rotate plumbing theme). 10-minute
   smoke against the real crate: confirm constructor signature,
   flush semantics, byte-count semantics. Drop the `#[cfg(unix)]
   Some(0o600)` syntax — pass `Some(0o600)` unconditionally.
   Tighten `rotates_at_five_megabytes` to assert byte counts on
   each segment. Add a chmod-after-rotation helper or a test
   asserting `0o600` on rotated segments.

6. **Replace the `init_for_test` global subscriber with scoped
   subscribers** (addresses: tracing-init theme). Use
   `tracing::subscriber::with_default(...)` per test, or move
   subscriber-installing tests into `tests/` so each runs in its
   own process. The `make_writer` test should write directly via
   `Write` and not install a subscriber at all.

7. **Pass an explicit predicate to `invalidateQueries` on
   reconnect** (addresses: serverInfo flicker). Exclude
   `['server-info']` (and any future session-stable keys) from
   the invalidate-all sweep.

8. **Realign `/api/info` with sibling api modules** (addresses:
   architecture/standards convention drift). Drop `router()` from
   `api/info.rs`; export only `pub(crate) async fn get_info(...) ->
   Json<ServerInfoBody>`; register via `.route("/api/info",
   get(info::get_info))` either in `api::mount` (matches existing
   pattern) or directly alongside `/api/healthz` in
   `build_router_with_spa` (better expresses
   infrastructure-vs-domain). Use `['server-info']` for the
   queryKey.

9. **Soften banner copy and enrich kanban announcements**
   (addresses: usability themes). Banner: "We couldn't read this
   document's metadata header; showing the file as-is." Kanban
   announcements: include the ticket number prefix (`"Picked up
   ticket 0001 — Foo"`) and map column ids to display labels
   (`"Moved … to In progress"`). Add a transient "Reconnected"
   confirmation when the indicator clears. Include
   `PROJECT_ROOT` in the init-not-run hint.

10. **Fix ARIA roles** (addresses: standards/usability themes).
    Drop `role="status"` from the malformed-frontmatter banner;
    use a plain `<div>` styled as a warning or `role="note"`.
    Drop `role="contentinfo"` from the sidebar version footer
    (use `aria-label` instead, or move the footer to
    `RootLayout`).

11. **Hex-vs-token drift in contrast tests** (addresses: token
    drift). Either parse `:root` from `global.css` at test time
    or move tokens into a shared `tokens.ts` module imported by
    both CSS and the test.

12. **Rename `Sidebar` prop accretion** (addresses: prop
    accretion theme). Extract `<SidebarFooter>` consuming hooks
    directly. Sidebar keeps `docTypes` only.

13. **Connection state must be reactive** (addresses: ref-vs-
    state correctness). Use `useState` (or
    `useSyncExternalStore`) inside `useDocEvents` for
    `connectionState` so consumers re-render on transition.

14. **Decide on the X- prefix** (addresses: RFC 6648 theme).
    Either rename to `Accelerator-Visualiser-Version` in spec +
    plan, or document the deferral with rationale. v1 is the
    cheap moment.

15. **Clarify accessibility scope** (addresses: WCAG version,
    forced-colors, focus-ring clipping). Pin "WCAG 2.1 AA" or
    "WCAG 2.2 AA" explicitly; add a `forced-colors` rule; add
    a manual scan for outline clipping on dense surfaces.

16. **Cover mid-session frontmatter transitions and
    EventSource-throws** (addresses: missing edge cases). Add
    targeted tests in 10.5 and 10.6.

17. **Address minor and suggestion items** as bandwidth permits;
    most are one-line edits or documentation.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Mostly well-structured: preserves the transport/dispatch
boundary in the SSE rewrite, isolates logging into a new `log` module,
respects existing TDD discipline. Two architectural concerns stand
out: version-header middleware ordering, and `/api/info` integration
inconsistency with sibling api modules.

**Strengths**: clean separation between `ReconnectingEventSource` and
`useDocEvents`; logging extracted into a dedicated module; init
sentinel reuses `die_json`; phase ordering shippable in isolation;
resilience strategy sound (bounded backoff, bounded log retention).

**Findings**: 2 major (version-header ordering, `/api/info` mounting),
3 minor (subscriber non-reentrancy, sidebar prop accretion,
onopen/onReconnect overlap), 2 suggestions (centralise
`CARGO_PKG_VERSION`, document TraceLayer position).

### Code Quality

**Summary**: Well-structured, TDD-disciplined, decomposed. The
`ReconnectingEventSource` snippet has multiple state-tracking smells.
Logging init has subtle lifetime/global-subscriber concerns. Sidebar
prop API clumping. Several test snippets won't compile or run as
written.

**Strengths**: shippable phases with clear failing-test → impl →
verify cycles; typed errors via `thiserror`; correct identification
of `tracing-appender`/`file-rotate` split; observability as first-
class; pure-function extraction (`computeBackoff`,
`buildKanbanAnnouncements`); explicit test seams.

**Findings**: 5 major (overlapping state flags, onerror/close race,
state-observable test asserts impossible sequence, fake-source
mutation hides bugs, JSX in `.test.ts`), 8 minor (init_for_test global
subscriber, init panics on duplicate, file-rotate mode claim
unverified, sidebar prop clump, hard-coded contrast hex, useMemo
thunk staleness, bash assertion order, useDocEvents wiring elisions).

### Test Coverage

**Summary**: Comprehensively TDD-shaped. Several individual tests
have specification gaps risking flakiness, semantic emptiness, or
broken compilation. Notable coverage gaps around mid-session
transitions, EventSource constructor failure, `/api/info` error
responses, and contrast-token drift.

**Strengths**: failing-test → impl → verify cycles; axum oneshot
patterns including 404 negative path; `computeBackoff` table
coverage; testing-strategy section enumerates all new tests;
contrast-audit table is rigorous.

**Findings**: 1 critical (sentinel breaks harness +
`make_stub_binary` doesn't exist), 6 major (`init_for_test` flakes,
jitter-vs-1000ms-advance, hand-wavy trace test, hard-coded contrast
hex, file-rotate API unverified, missing mid-session/throw tests),
3 minor (`/api/info` error paths, Sidebar prop drift after 10.5,
LoggingError::CreateDir untested), 1 suggestion (Playwright SSE
reconnect).

### Correctness

**Summary**: Core mechanisms mostly sound but the
`ReconnectingEventSource` state machine has multiple edge-case bugs
(first-connect failure, re-entrant close, wasErrored flag); the file
logging design has a serious dual-writer hazard with the launcher's
existing `>>"$LOG_FILE"` redirection; init-sentinel placement breaks
the reuse short-circuit.

**Strengths**: TDD-first ordering; explicit WorkerGuard lifetime;
sentinel rejection before any state writes; reconnect wrapper uses
DI for the factory.

**Findings**: 2 critical (dual-writer corruption, missed onReconnect
on boot-recovery), 5 major (close/onerror race, sentinel-vs-reuse
ordering, invalidate-all evicts serverInfo, computeBackoff overflow,
version_header guard test missing), 5 minor (handler binding race,
panic-time log loss, ref-not-state for connectionState,
init_for_test global subscriber, attempts-resets-on-flap).

### Security

**Summary**: Operates within the established localhost-only trust
boundary; additions largely benign. The most material concrete
concern is rotated log segment permissions. Smaller issues around
log content hygiene, sentinel bypass, supply-chain footprint.

**Strengths**: sentinel runs before state writes; JSON error is
injection-safe; 0o600 posture carried forward; TraceLayer
`include_headers(false)`; version header trivial to disclose
(localhost-only); /api/info inherits guards.

**Findings**: 1 major (rotated segment permissions), 2 minor
(request-path log disclosure on multi-user hosts, empty-`.gitignore`
sentinel bypass), 2 suggestions (`cargo audit`, RFC 6648 X- prefix).

### Usability

**Summary**: A thoughtful polish phase addressing most of the v1
quality bar (error UX, accessibility, focus rings, contrast). Biggest
concerns are micro-copy and end-user clarity: jargon in the banner,
missing ticket numbers in announcements, ambiguous init hint, no
positive Reconnected confirmation. Forced-colors mode silent. Muted-
colour bump may cascade visually. Onboarding flow not exercised.

**Strengths**: structured JSON error mirrors die_json; reconnecting
indicator with `role="status"` and aria-label; full four-callback
dnd-kit announcements; tokens factored into `:root` and locked by
test; FrontmatterChips kept as secondary signal; sidebar footer
degrades gracefully.

**Findings**: 4 major (banner jargon, no ticket-numbers in
announcements, no Reconnected confirmation, ambiguous init hint),
5 minor (muted cascade, no forced-colors, outline clipping,
onboarding flow not in smoke list, sidebar-only placement),
1 suggestion (version footer link to release notes).

### Compatibility

**Summary**: Largely compatible with the existing axum 0.7 /
tower-http 0.5 / dnd-kit 6.3 stack; new deps appropriate and respect
1.85 MSRV. Concrete API issues: file-rotate constructor uses invalid
conditional argument; version-header middleware is wired innermost
not outermost; test helper relies on global subscriber init.

**Strengths**: `/api/info` purely additive with camelCase; X-header
purely additive; `ReconnectingEventSource` preserves test seam;
correct identification of size-rotation gap in tracing-appender;
MSRV preserved; tower-http API matches.

**Findings**: 3 major (`FileRotate::new` arity, version_header
ordering, init_for_test global subscriber), 5 minor (RFC 6648 X-,
TraceLayer + SSE compression, Announcements typing, focus-ring
clipping, WorkerGuard non-graceful shutdown), 1 suggestion
(`Cache-Control` on `/api/info`).

### Standards

**Summary**: Broadly aligned with project conventions and industry
standards. Inherits the spec's RFC-6648-violating X- prefix, proposes
`role="status"` for a static page-load warning where the role doesn't
match live-region semantics, mis-specifies `role="contentinfo"` for
a footer nested inside `<nav>`, and diverges from the project's flat
`queryKeys` and `api::mount` patterns in small but visible ways.

**Strengths**: `/api/info` URL convention; `{error, hint}` shape;
WCAG AA targets correct; dnd-kit four-callback shape; bash test
naming; `HeaderValue::from_static` ASCII-safety; `#[serde(rename_all
= "camelCase")]` consistency.

**Findings**: 0 major, 6 minor (X- prefix RFC 6648, role="status"
on banner, role="contentinfo" in nav, queryKey casing, api::mount
pattern divergence, WCAG version unspecified), 3 suggestions
(forced-colors, with_ansi(false) no-op, richer `/api/info` payload).

---

## Re-Review (Pass 2) — 2026-04-28T17:17:00Z

**Verdict:** REVISE

The Pass 1 findings are largely resolved (most majors and all
critical-tagged items fixed cleanly), but the rewrite of Phase 10.5's
`ReconnectingEventSource` introduced a new **critical** regression:
the constructor's first `connect()` is short-circuited by the
closed-state guard, so the wrapper never opens its initial
connection. A second new **critical** regression appears in the new
launcher regression test (`test_sentinel_does_not_kill_already_running_server`),
where the seeded `start_time` from `date +%s` cannot match what
`start_time_of $$` reads back from `/proc` or `ps` — the test cannot
verify the invariant it was added to lock in. Plus a new **major**
file-mode regression on `server.bootstrap.log` (inherits launcher
umask) and a new **major** test-compilation hazard on the inline
`MutexWriter` newtype's lifetime. These four blockers prevent
APPROVE; the rest of the changes hold up under re-review and the
plan is structurally close to landable.

### Previously Identified Issues — Resolution Status

#### Critical (Pass 1)

- 🔴 **Test Coverage**: `make_stub_binary` doesn't exist; harness breakage — **Resolved**. New Prerequisites step updates `make_project()`; tests use the existing `make_fake_visualiser`; `assert_eq` argument order corrected.
- 🔴 **Correctness**: Dual writers to `server.log` — **Resolved**. Launcher's nohup retargets to `server.bootstrap.log`; Rust process closes stdout/stderr after `log::init` via `redirect_std_streams_to_devnull`.
- 🔴 **Correctness**: `ReconnectingEventSource` may miss `onReconnect` on boot-time recovery — **Partially resolved**. The single-state-field redesign correctly addresses the gate logic, but a NEW critical regression supersedes it (see "New Issues Introduced").

#### Major (Pass 1)

- 🟡 Layer ordering for `version_header` — **Resolved**. Now last `.layer(...)` call (outermost); locked in by `version_header_present_on_host_header_guard_rejection`.
- 🟡 `FileRotate::new` invalid `#[cfg(unix)]` — **Resolved**. Pass `Some(0o600)` unconditionally with explicit comment.
- 🟡 Overlapping reconnect state flags — **Resolved**. Single `state` field; `wasErrored` / `hasEverConnected` removed.
- 🟡 `onerror` / `close()` race — **Mostly resolved**. `closed` guard at top of every handler; `detachAndCloseSource` nulls handlers before close. One remaining minor edge case (re-entrant `close()` during constructor binding — see new findings).
- 🟡 State-observable test asserts impossible sequence — **Resolved**. `setState` no longer short-circuits on first call; subscriber sees initial transition.
- 🟡 Test fixture mutates a single fake — **Resolved**. `makeFakeFactory` produces fresh fakes per call.
- 🟡 `useServerInfo` test extension `.test.ts` contains JSX — **Resolved**. Renamed to `.test.tsx`.
- 🟡 `init_for_test` global subscriber — **Resolved**. `tracing::subscriber::with_default` per test; `make_writer` tests don't install a subscriber at all.
- 🟡 Jitter flake on `advanceTimersByTime(1000)` — **Resolved**. Deterministic `random` injection; tests assert exact 800 ms boundaries.
- 🟡 Trace-summary test hand-wavy — **Mostly resolved**. Sketch is now concrete but field-shape assertions remain potentially brittle across tower-http patch versions (see new minor).
- 🟡 Hard-coded contrast hex tests — **Resolved**. `tokens.ts` is single source of truth; `tokens-vs-css` drift test enforces equality.
- 🟡 `rotates_at_five_megabytes` test sensitivity — **Resolved**. Test now writes `MAX_BYTES + 64 KiB`; asserts byte counts on segments.
- 🟡 Missing mid-session frontmatter / EventSource-throw tests — **Mostly resolved**. Tests added; `dispatchSse` helper signature still loosely specified (see new finding).
- 🟡 `/api/info` mounting inconsistency — **Resolved**. Mounted directly alongside `/api/healthz` in `server.rs`.
- 🟡 Sentinel placement before reuse short-circuit — **Resolved**. Now placed after reuse short-circuit; new test `test_sentinel_does_not_kill_already_running_server` added (but see new critical — the test itself is broken).
- 🟡 `invalidateQueries()` evicts `serverInfo` — **Resolved**. Predicate excludes `'server-info'`.
- 🟡 `computeBackoff` overflow — **Resolved**. `Math.pow` + `Math.min` cap test added.
- 🟡 `version_header` guard test missing — **Resolved**. Test added.
- 🟡 Rotated log segments may not inherit 0o600 — **Resolved**. Dedicated `active_and_rotated_segments_are_owner_only` test asserts mode 0o600 on every segment in the directory.
- 🟡 Banner copy uses developer jargon — **Resolved**. "Frontmatter" replaced with "metadata header"; banner queries via `aria-label`.
- 🟡 Kanban announcements omit ticket numbers — **Resolved**. `ticketNumberFromRelPath` extracts the `NNNN-` prefix; `STATUS_COLUMNS` maps column ids to display labels.
- 🟡 No "Reconnected" confirmation — **Resolved**. `justReconnected` state surfaces "Reconnected — refreshing" for ~3 s.
- 🟡 Init hint ambiguous — **Resolved**. JSON now embeds `project_root`; hint string interpolates the resolved path.

#### Minor (Pass 1)

- 🔵 `log::init` non-reentrant — **Partially resolved**. `try_init` + `LoggingError::AlreadyInitialised`; underlying global-subscriber constraint remains.
- 🔵 Sidebar prop accretion — **Resolved**. `<SidebarFooter>` extracted; `<Sidebar>` keeps `{ docTypes }`.
- 🔵 `onopen` / `onReconnect` overlap — **Resolved**. `onopen` removed from public surface.
- 🔵 file-rotate API claims unverified — **Resolved**. Prerequisites step locks the API; tests assert behaviour.
- 🔵 `useMemo` thunk in dnd-kit announcements — **Resolved**. `useRef`-backed thunk makes the deferred-read intent meaningful.
- 🔵 Bash assertion order inverted — **Resolved**. New tests use `assert_eq "<label>" "<expected>" "<actual>"`.
- 🔵 `useDocEvents` snippet leaves dead branches; eager-invalidate-on-error silently dropped — **Resolved**. Full body spelled out; Migration Notes calls out the behaviour change.
- 🔵 RFC 6648 X- prefix — **Resolved**. Renamed to `Accelerator-Visualiser-Version` in plan + spec amendment.
- 🔵 `role="status"` on static banner — **Resolved**. Banner uses no role; `aria-label` distinguishes it.
- 🔵 `role="contentinfo"` in `<nav>` — **Resolved**. Footer dropped that role; uses `aria-label` instead.
- 🔵 `queryKey.serverInfo()` casing — **Resolved**. Tuple is `['server-info']` (kebab-case).
- 🔵 WCAG version unspecified — **Resolved**. Pinned at WCAG 2.2 AA.
- 🔵 Forced-colors mode unaddressed — **Resolved**. `@media (forced-colors: active)` rule with `outline-color: Highlight`.
- 🔵 `with_ansi(false)` no-op — **Resolved**. Removed.
- 🔵 Outline-clipping risk — **Partially resolved**. Manual sweep step + documented `box-shadow` fallback; no automated coverage.
- 🔵 `--color-muted` cascade — **Resolved**. Token split into `-text` (AA) and `-decorative`.
- 🔵 Missing `/api/info` error / Sidebar-prop tests — **Resolved**. 500 + missing-version cases added; `<SidebarFooter>` test covers all three states.
- 🔵 `LoggingError::CreateDir` untested — **Resolved**. `make_writer_errors_when_parent_path_is_a_file` added.
- 🔵 `connectionStateRef.current = s` non-reactive — **Resolved**. Replaced with `useState`.
- 🔵 `attempts` resets on every successful open — **Accepted**. Documented as v1 behaviour in design choices.
- 🔵 Init sentinel satisfied by empty `.gitignore` — **Still present** (acceptable per security lens; not a security boundary).
- 🔵 Request paths in logs — **Still present** (acceptable; bounded by 0o600 + bootstrap-log split, which the new finding partially erodes).

#### Suggestions (Pass 1)

- 🔵 Centralise `CARGO_PKG_VERSION` — **Resolved**. `crate::VERSION` constant defined at lib.rs root.
- 🔵 TraceLayer position rationale — **Resolved**. Inline comment explains "INSIDE compression and version_header".
- 🔵 `cargo audit` gate — **Resolved**. Added as automated success criterion.
- 🔵 Cache-Control on `/api/info` — **Resolved**. `no-cache` set.
- 🔵 Richer `/api/info` payload — **Resolved**. `{name, version}`.
- 🔵 Logs-on-panic loss — **Documented**. Performance Considerations and Migration Notes acknowledge crash-time partiality.
- 🔵 E2E for SSE reconnect — **Deferred to Phase 11** (explicit).
- 🔵 Version footer link to release notes — **Documented as out-of-scope** in What-We're-NOT-Doing.

### New Issues Introduced

#### Critical

- 🔴 **Correctness / Compatibility**: `ReconnectingEventSource.connect()` is short-circuited by the closed-state guard on first construction
  **Location**: Phase 10.5 Step 10.5.b — implementation snippet
  `state` is initialised to `'closed'`. The constructor calls
  `this.connect()`. `connect()` begins with
  `if (this.state === 'closed') return`. Result: no `EventSource`
  is ever created, no `factory(url)` call ever happens, the
  wrapper sits in `'closed'` state forever. Every Phase 10.5.a
  test that drives `fakes[0].onopen?.({})` will fail because
  `fakes` is empty. Production: SSE never connects on app boot
  — all live updates, the reconnecting indicator, and the
  invalidate-on-reconnect sweep are non-functional.
  **Fix**: Decouple the post-`close()` resurrection guard from
  the initial state. Either (a) introduce a third initial state
  like `'connecting'` (best — also gives the SidebarFooter a
  cleaner UX signal for bootstrap vs. recovery; flagged
  separately by the compatibility lens), or (b) keep `state =
  'closed'` initial but use a separate `private closed = false`
  flag toggled only by `close()` and checked in `connect()` /
  handlers / `scheduleReconnect`. The state machine and the
  resurrection guard cannot share the same sentinel.

- 🔴 **Correctness / Test Coverage**: `test_sentinel_does_not_kill_already_running_server` cannot verify what it claims
  **Location**: Phase 10.1 Step 10.1.a
  Test seeds `server-info.json` with
  `start_time: $(date +%s)` (current wall-clock second) and
  `server.pid` with `$$` (test runner pid). The launcher's
  reuse short-circuit reads `start_time_of $EXISTING_PID` from
  `/proc/$pid/stat` (Linux) or `ps -o lstart=` (macOS) — the
  test runner's *actual* process start time, which has zero
  chance of equalling `date +%s` evaluated mid-test. The
  identity check fails, the launcher deletes the seeded files
  and falls through to the sentinel check (which fires because
  the sentinel was deleted), exits 1 with init-not-run, and
  `assert_eq "exit code" "0" "$rc"` fails — or worse, passes
  for the wrong reason on a future refactor.
  **Fix**: Source `launcher-helpers.sh` in the test (or
  replicate the lookup inline) and seed `start_time` from
  `start_time_of $$` itself, so the seeded value matches what
  the launcher reads back. Alternatively, spawn a controlled
  child process (`sleep 30 &`) and use its pid + computed
  start_time as the fake live server.

#### Major

- 🟡 **Security**: `server.bootstrap.log` inherits launcher umask (typically 0o644), regressing from `server.log`'s 0o600
  **Location**: Phase 10.2 Step 10.2.d
  The launcher's `nohup … >> "$LOG_FILE.bootstrap" 2>&1` runs
  under the user's default shell umask (usually 022), so the
  bootstrap log lands at mode 0o644 — world-readable. The
  bootstrap log captures pre-`log::init` panics, which is
  exactly the window where backtraces, environment variables,
  and configuration error detail surface in cleartext. On
  multi-user hosts, any local user can read the bootstrap log
  while the rest of `<TMP>/visualiser/` enforces 0o600.
  **Fix**: Set `umask 077` immediately before the nohup line
  (or `: > "$LOG_FILE.bootstrap" && chmod 600` then redirect
  with `>>`). Add a launcher test asserting
  `stat -c '%a' server.bootstrap.log` returns `600`.

- 🟡 **Compatibility / Code Quality**: `MutexWriter` / `Locked<'a>` newtype inside the `MakeWriter` closure has a self-referential lifetime
  **Location**: Phase 10.2 Step 10.2.a-bis
  The inline `Locked<'a>(MutexGuard<'a, FileRotate<AppendCount>>)`
  newtype borrows from the outer `Mutex` — but it's returned
  from a `move ||` closure passed to `with_writer`, which
  requires `W: Write + 'static`. The test won't compile.
  Separately, the same plan claims in Phase 10.2.e that
  `MutexWriter` and `build_test_json_subscriber` should be
  extracted to `log.rs`'s `pub(crate) test_support` module —
  but in 10.2.a-bis the extraction is left optional with a
  parenthetical "if it proves awkward".
  **Fix**: Promote the extraction from optional to required.
  Define `MutexWriter<W>(Arc<Mutex<W>>)` once in
  `log::test_support` with a proper `MakeWriter` impl that
  returns a `'static`-bounded writer (typical pattern: hold an
  `Arc`, lock per `make_writer` call). Verify against the
  pinned `tracing-subscriber 0.3` API surface before locking
  the test code in the plan.

- 🟡 **Test Coverage**: `redirect_std_streams_to_devnull` has no automated test
  **Location**: Phase 10.2 Step 10.2.c
  The helper uses `unsafe { libc::dup2 }` to overwrite fds 1
  and 2. This is critical to the dual-writer hazard the plan
  is trying to eliminate, but no test verifies post-call
  writes to stdout/stderr go to `/dev/null`, the function is
  idempotent, or it doesn't accidentally clobber fd 0. A
  silent regression here re-introduces byte-interleaving into
  `server.log`.
  **Fix**: Add a `#[cfg(unix)]` integration test that forks
  (via `nix::unistd::fork` or a child process), runs the
  redirect, writes a known sentinel string to fd 2, then
  asserts a captured pre-redirect log file does not contain
  it. At minimum, add the manual-verification step to the
  Phase 10.2 manual checklist.

- 🟡 **Test Coverage**: `dispatchSse` test helper required by 10.6 is admitted-not-yet-existing
  **Location**: Phase 10.6 Step 10.6.a
  The mid-session SSE banner test depends on a `dispatchSse`
  helper returned by `renderLibraryDocView`. The plan
  parenthetical concedes "If the existing helper doesn't yet
  expose `dispatchSse`, extend it once at the top of the test
  file" — without specifying what the helper should look
  like, what code path it routes through (the real
  `useDocEvents`? a direct `dispatchSseEvent` call?), or how
  it interacts with the test's mocked `EventSource` factory.
  **Fix**: Specify the helper signature concretely. A direct
  `dispatchSseEvent(event, queryClient); act(() => {})`
  bypasses `useDocEvents` machinery and is sufficient for the
  cache-invalidation → re-render path the test cares about;
  alternatively, thread through a fake `EventSourceFactory`
  whose mock instance receives `dispatchSse(data)` calls.

- 🟡 **Compatibility**: `?raw` CSS imports require `vite/client` types not mentioned
  **Location**: Phase 10.7 Sub-step A and Sub-step C
  `import globalCss from './global.css?raw'` requires
  `vite/client` ambient types (typically referenced via
  `vite-env.d.ts`). The plan adds two `?raw` test imports
  but doesn't mention adding/verifying the declaration.
  Without it, `npm run typecheck` fails.
  **Fix**: Add a one-line prerequisite in Phase 10.7 stating
  that `frontend/src/vite-env.d.ts` references
  `vite/client`, or include adding the reference as a
  sub-step. Also confirm `global.css` is imported as a side
  effect from `main.tsx` and only via `?raw` in tests —
  never both in production code.

#### Minor

- 🔵 **Architecture**: `<SidebarFooter>` couples server identity + transport state
  **Location**: Phase 10.4 / 10.5
  Resolves the previous Sidebar prop-clump but migrates the
  coupling into `<SidebarFooter>` (consumes `useServerInfo` +
  `useDocEvents`). The "no collapsed/mobile sidebar" carve-
  out's future extraction will be harder than necessary
  because the version footer's render condition is entangled
  with the reconnecting indicator.
  **Suggestion**: Split into `<SidebarVersion>` and
  `<ConnectionStatus>`. Not blocking.

- 🔵 **Architecture**: `redirect_std_streams_to_devnull` introduces unsafe FFI in main
  **Location**: Phase 10.2 Step 10.2.c
  Direct `libc::dup2` in main.rs adds a new unsafe surface at
  the application entry point. The cross-platform stub on
  non-unix means the dual-writer hazard is silently different
  on platforms outside the supported set.
  **Suggestion**: Encapsulate as
  `accelerator_visualiser::log::detach_inherited_streams() ->
  io::Result<()>` so the unsafe + cfg branching lives next to
  logging concerns.

- 🔵 **Architecture / Security**: bootstrap log lifecycle is under-specified; helper has unbounded growth and no operator surfacing
  **Location**: Phase 10.2 Step 10.2.d
  The launcher uses `>>` (append) but the prose claims the
  bootstrap log captures only the boot window. Across many
  restarts, the file grows unboundedly. Also no `/api/info` or
  UI affordance points operators at it.
  **Suggestion**: Either truncate (`>` not `>>`) on each
  launch, or rename-to-.1 on launch to give a single-boot
  window. Document the choice in code.

- 🔵 **Architecture**: Two parallel `/api/*` mount conventions coexist with no enforced boundary
  **Location**: Phase 10.4 Step 10.4.b
  `/api/healthz` and `/api/info` live in `server.rs` directly;
  domain endpoints live in `api::mount`. The convention is
  encoded only in prose.
  **Suggestion**: Add a doc comment at the top of `api/mod.rs`
  listing the criterion, or extract a small `infra_routes() ->
  Router<()>` helper in `server.rs` that aggregates healthz +
  info.

- 🔵 **Code Quality**: `main.rs` ordering — trailing `eprintln!("server error: {e}")` after `drop(_log_guard)` writes to /dev/null
  **Location**: Phase 10.2 Step 10.2.c
  The plan documents the line as "advisory only" — a code
  smell. Future maintainers may spend time grep-failing for
  it; deleting the redirect during debugging would silently
  re-corrupt server.log.
  **Suggestion**: Log the server error via `tracing::error!`
  *before* `drop(_log_guard)` and remove the trailing
  `eprintln!`.

- 🔵 **Code Quality**: TraceLayer position comment is technically correct but easy to read backwards
  **Location**: Phase 10.2 Step 10.2.e
  "INSIDE compression and version_header so recorded latency
  excludes compression cost" reads naturally backwards because
  in tower terms inner layers measure handler-only time
  (compression hasn't happened yet on the response path).
  **Suggestion**: Replace with a one-liner naming surrounding
  layers explicitly: "TraceLayer is inner-to compression and
  version_header (which are outer): recorded `latency` is
  handler-only and does not include compressing the body or
  stamping the version header."

- 🔵 **Code Quality**: `<SidebarFooter>` calling `useDocEvents()` directly may double-subscribe
  **Location**: Phase 10.4 / 10.5
  If both `<SidebarFooter>` and `<RootLayout>` (or any other
  top-level consumer) call `useDocEvents()`, two
  `ReconnectingEventSource` instances open parallel SSE
  streams and run independent invalidate-all sweeps. Tests
  mock the hook per-component, sidestepping the question.
  **Suggestion**: Either (a) introduce a
  `useConnectionStatus()` selector backed by a context
  provider populated once at app root, or (b) document
  explicitly that `useDocEvents` must be called at most once
  per render tree.

- 🔵 **Code Quality / Correctness**: Factory-throws path emits `onerror` before consumer can attach handler
  **Location**: Phase 10.5 Step 10.5.b
  In `connect()`, the `try { factory(url) } catch { onerror(...);
  scheduleReconnect() }` path runs synchronously inside the
  constructor. At that point `this.onerror` is still `null`
  because the consumer hasn't had a chance to assign it yet.
  **Suggestion**: Either defer the initial `connect()` call
  out of the constructor (e.g. via a `start()` method or
  `queueMicrotask`), or accept `onerror` as a constructor
  option alongside `onReconnect` / `onStateChange`.

- 🔵 **Correctness**: Re-entrant `close()` during constructor handler binding leaves dangling references
  **Location**: Phase 10.5 Step 10.5.b
  If a synchronous fake/factory triggers `onerror` from
  inside the constructor's `connect()` and the consumer's
  error callback calls `close()`, `detachAndCloseSource`
  nulls handlers, but the constructor's `src.onerror = ...`
  reassignment then overwrites the null. Closures all
  closed-state-guard so behaviour is benign.
  **Suggestion**: Bind handlers to a local `src` variable
  inside `connect()` and only assign `this.source = src`
  once binding completes. Add a re-entrant-close test.

- 🔵 **Correctness**: `connect()` early-return on `'closed'` makes the resurrection guard the same flag as the initial state
  **Location**: Phase 10.5 Step 10.5.b
  Same root cause as the critical first-construction bug.
  Decoupling state from closed flag (or introducing a
  `'connecting'` state) resolves both at once.

- 🔵 **Code Quality / Correctness**: Predicate `q.queryKey[0] !== 'server-info'` is a string-equality check on an `unknown` key
  **Location**: Phase 10.5 Step 10.5.c
  Returns true for any non-string root, including future
  numeric / object keys. For v1 fine; brittle to renames.
  **Suggestion**: Maintain `SESSION_STABLE_QUERY_ROOTS:
  ReadonlySet<unknown>` alongside `queryKeys`; predicate
  becomes `!SESSION_STABLE_QUERY_ROOTS.has(q.queryKey[0])`.

- 🔵 **Test Coverage**: `useDocEvents` reconnect-sweep predicate test is in success criteria but not sketched in TDD steps
  **Location**: Phase 10.5 Step 10.5.c
  Mirrors a Pass 1 gap. A test asserting `invalidateSpy.toHaveBeenCalled()`
  passes without verifying the predicate; the version-footer-
  doesn't-flicker carve-out's purpose is defeated.
  **Suggestion**: Inline a 20-line test sketch under Step
  10.5.c that seeds the cache with `['server-info']` and
  `['docs']`, simulates a reconnect via the wrapper, then
  asserts `'server-info'` cached value is unchanged while
  `'docs'` is invalidated.

- 🔵 **Test Coverage**: Bootstrap-log test asserts presence not exclusivity
  **Location**: Phase 10.2 Step 10.2.d
  `assert_file_present "server.bootstrap.log"` would pass
  even if the launcher still appended to `server.log`.
  **Suggestion**: Have the fake binary print a known
  sentinel; assert `grep BOOTSTRAP_SENTINEL server.bootstrap.log`
  succeeds AND `grep BOOTSTRAP_SENTINEL server.log` returns
  empty.

- 🔵 **Test Coverage**: `?raw` import behaviour in vitest unverified
  **Location**: Phase 10.7 Sub-step A / C
  `import css from './*.css?raw'` returning unmodified CSS
  source depends on vitest's loader chain not intercepting
  the `?raw` query first.
  **Suggestion**: Add a one-line prerequisite verifying `?raw`
  returns raw text under the project's vitest config; fall
  back to `fs.readFileSync(new URL(...))` if not.

- 🔵 **Test Coverage / Code Quality**: 3-second `justReconnected` timer has no cleanup / interleaving tests
  **Location**: Phase 10.5
  No test for: (a) component unmount during the 3 s window
  (React strict-mode warning); (b) two reconnects within 3 s
  (timer interleaving); (c) `useEffect` cleanup resetting
  `justReconnected`.
  **Suggestion**: Add three small vitest cases.

- 🔵 **Test Coverage**: `contrast.ts` helper itself has no tests
  **Location**: Phase 10.7 Sub-step C
  If the WebAIM-luminance helper has a bug, every contrast
  assertion is meaningless.
  **Suggestion**: Add 3-4 fixture cases against WebAIM-
  published reference ratios (`#777`/`#fff` = 4.48:1;
  `#000`/`#fff` = 21:1).

- 🔵 **Code Quality**: Inline `MakeWriter` boilerplate is unmaintainable; the suggested `MutexWriter` extraction is left optional
  **Location**: Phase 10.2 Step 10.2.a-bis
  Two halves of the plan disagree about whether the helper
  exists. (Same root cause as the new major above.)

- 🔵 **Standards**: tokens.ts → global.css adapter is hand-mirrored
  **Location**: Phase 10.7 Sub-step C
  Vite has well-established patterns (CSS Modules `:export`,
  PostCSS custom-properties, virtual-module plugin) that
  make drift unrepresentable rather than test-detected.
  **Suggestion**: Commit explicitly to one of those, or
  embrace the hand-mirror pattern with a header comment
  naming the test as the enforcement mechanism.

- 🔵 **Standards**: `aria-label` on inert `<span>` elements diverges from idiomatic ARIA
  **Location**: Phase 10.4 / 10.5 / 10.6
  `aria-label` on a bare `<span>` (implicit role `generic`)
  may be ignored by some AT and replaces the visible text
  where honoured. The reconnect / reconnected spans already
  carry `role="status"`; the malformed banner's `<div>` has
  no role, so its `aria-label` may not surface at all.
  **Suggestion**: For the version span, drop `aria-label` and
  query tests by visible text or `data-testid`. For the
  banner, use `role="note"` and let visible text be the name.

- 🔵 **Compatibility**: `libc` dependency must be confirmed in server `Cargo.toml`
  **Location**: Phase 10.2 Step 10.2.c
  Phase 10.2 dependency block lists only `file-rotate` and
  `tracing-appender`. If `libc` is currently transitive, the
  new code fails to build.
  **Suggestion**: Add an explicit `libc = "0.2"` confirmation,
  or use `nix::unistd::dup2` if `nix` is already a direct
  dep. Also check `dup2`'s return value with an
  `assert!`/`tracing::warn!`.

- 🔵 **Compatibility**: TraceLayer trace-summary field shapes unverified
  **Location**: Phase 10.2 Step 10.2.e
  The test asserts `"method":"GET"`, `"uri":"/api/healthz"`,
  `"status":200`, and `latency` substrings. Exact field
  placement varies across tower-http patch versions
  (`status` may serialise as `"200 OK"`; `latency` as
  `"latency":"42 ms"` vs `"latency_ms":42`).
  **Suggestion**: Run a quick smoke against tower-http 0.5
  with the JSON subscriber and pin the assertion to the
  emitted format. Alternatively, parse the JSON line and
  assert keys exist.

- 🔵 **Compatibility**: Plan acknowledges but doesn't resolve mode-on-rotation enforcement
  **Location**: Phase 10.2 Step 10.2.b
  If file-rotate 0.7 doesn't propagate `mode` to rotated
  siblings, the `active_and_rotated_segments_are_owner_only`
  test will fail under typical umask. The plan mentions a
  `Notify` adapter as a contingency but doesn't wire it.
  **Suggestion**: The Prerequisites step should commit
  explicitly: either "verified: file-rotate enforces mode on
  rotation" (and skip the adapter), or "implement the chmod
  adapter unconditionally" (so the test always passes).

- 🔵 **Usability**: "Showing the file as-is" doesn't name the consequence
  **Location**: Phase 10.6 banner copy
  An idiomatic English casualism that doesn't translate
  cleanly and doesn't tell the user what to expect or how to
  recover.
  **Suggestion**: Replace with a concrete consequence +
  remediation, e.g. "The body is shown without titles, tags,
  or status. To restore them, fix the YAML block at the top
  of the file."

- 🔵 **Usability**: Em-dash separator in kanban announcements pronounced inconsistently across screen readers
  **Location**: Phase 10.7 Sub-step B
  "Picked up ticket 0001 — Foo" — VoiceOver reads "em
  dash"; NVDA depends on punctuation level; JAWS often
  skips. The intended-improvement audience is the most
  affected.
  **Suggestion**: Use a comma-and-word phrasing
  ("Picked up ticket 0001, titled Foo") or a colon
  ("Picked up ticket 0001: Foo"). The colon is generally a
  brief pause across all major screen readers.

- 🔵 **Usability**: Sidebar-only placement of reconnecting / reconnected indicators risks invisibility
  **Location**: Phase 10.5
  A user focused on the kanban or a doc page won't see the
  3 s "Reconnected — refreshing" toast and may miss that
  data has been swapped under them.
  **Suggestion**: Either co-locate a small fixed-position
  toast at the top-right, or stretch `justReconnected` to
  ~6 s with a subtle animation so peripheral vision picks
  it up.

- 🔵 **Usability**: Long absolute paths in init-not-run hint may render awkwardly
  **Location**: Phase 10.1
  On macOS the resolved `project_root` is typically a long
  path; the hint sentence becomes 100+ characters and may
  wrap mid-path or be truncated by the slash-command
  renderer.
  **Suggestion**: Either commit to a shorter hint that
  doesn't interpolate the path and let the renderer surface
  `project_root` as a separate styled line, or add an
  explicit assertion to the manual onboarding smoke step
  that the rendered hint is readable end-to-end on a
  representative deep workspace path.

- 🔵 **Usability**: Two `role="status"` spans in `<SidebarFooter>` may render together
  **Location**: Phase 10.5 Sub-step D
  The conditions are mutually exclusive at a single point in
  time but a future state-machine regression could expose
  both, and screen readers concatenate / queue polite live
  regions in vendor-specific orders.
  **Suggestion**: Add a defensive test pinning the
  mutual-exclusion invariant. Alternatively, fold both into
  a single `<span role="status">` whose text content is
  computed from both flags.

- 🔵 **Usability**: `<SidebarFooter>` empty-state causes layout jank on first load
  **Location**: Phase 10.4
  Component returns `null` while `/api/info` is loading;
  layout shifts when version label resolves.
  **Suggestion**: Reserve the footer's vertical space with
  `min-height` so the version label fades in without
  shifting other content.

- 🔵 **Standards**: `Cache-Control: no-cache` semantics may not match intent
  **Location**: Phase 10.4
  `no-cache` allows storage but mandates revalidation; if
  the intent is "never cache including in disk caches",
  `no-store` is the stricter directive.
  **Suggestion**: Keep `no-cache` (conventional for
  freshness-on-every-use) but note explicitly that
  `no-store` would be appropriate if the body ever embeds
  session-specific information.

- 🔵 **Standards / Compatibility**: `ConnectionState` lacks `'connecting'` value
  **Location**: Phase 10.5 Step 10.5.b
  Collapses bootstrap and reconnect into one
  `'reconnecting'` state. The `<SidebarFooter>` will show
  "Reconnecting…" on the very first page load before the
  server has ever responded — confusing for fresh launches.
  Adopting `'connecting'` would also resolve the critical
  first-construction bug naturally.
  **Suggestion**: Adopt `'connecting' | 'open' |
  'reconnecting' | 'closed'` and use `'connecting'` as the
  initial state (`connect()` permits it; `setState` can
  transition through it on the first connect).

- 🔵 **Architecture**: `log::init` non-reentrancy still constrains future logging-config evolution
  **Location**: Phase 10.2 Step 10.2.b
  Partially resolved (typed error replaces panic), but the
  global-subscriber-once constraint remains and the
  production init() has no test coverage of subscriber
  installation.
  **Suggestion**: Consider splitting `log::init` into
  `make_layer(path) -> (Layer, WorkerGuard)` and
  `install_global(layer) -> Result<(), AlreadyInitialised>`.
  Not blocking.

#### Suggestions

- 🔵 **Security**: Silent failure if `/dev/null` cannot be opened leaves inherited fds intact
  **Location**: Phase 10.2 Step 10.2.c
  `if let Ok(devnull)` swallows the error. In sandboxed/
  chrooted environments where /dev is missing, the redirect
  silently no-ops and the dual-writer hazard re-emerges.
  **Suggestion**: Treat `/dev/null` open failure as a hard
  error: log via `tracing::error!` and `return ExitCode::from(2)`.

- 🔵 **Security**: `project_root` field disclosure acceptable for v1 but worth documenting
  **Location**: Phase 10.1
  The field reveals the user's project layout. Acceptable
  in the local-only invocation model but worth noting if
  the launcher's stdout ever becomes a non-local artifact.
  **Suggestion**: Add a one-line note in What-We're-NOT-
  Doing or threat-model commentary.

- 🔵 **Standards**: `?raw` CSS imports require ambient declaration
  **Location**: Phase 10.7 Sub-step A / C
  Same root cause as the new compatibility major.

- 🔵 **Compatibility**: `ServerInfoBody` `rename_all = camelCase` is a no-op today but locks future field naming
  **Location**: Phase 10.4 Step 10.4.a
  Single-word fields unchanged; future multi-word fields
  silently switch to camelCase.
  **Suggestion**: Drop the attribute or document the
  convention.

### Assessment

The plan has come a long way: the previous round's three critical
findings and most of the majors are resolved cleanly with concrete
test coverage and architectural rationale. The Pass 2 verdict is
**REVISE** because two new criticals were introduced by the rewrite
itself — both blockers, both fixable by relatively small edits:

1. **`ReconnectingEventSource` first-construction bug.** Single-
   field state machine collapsed two semantics (initial state vs.
   resurrection guard) into one sentinel. Either decouple via a
   separate `closed: boolean` flag, or introduce a `'connecting'`
   state. The latter also resolves the standards/usability minor
   about the bootstrap-vs-recovery indicator.

2. **`test_sentinel_does_not_kill_already_running_server` cannot
   verify what it claims.** Source `start_time_of $$` from the
   helper rather than seeding `date +%s`.

Plus two new majors that are also small fixes:

3. **`server.bootstrap.log` mode regression.** `umask 077` before
   the nohup line.

4. **`MutexWriter` lifetime + extraction commitment.** Promote
   the extraction from optional to required; pick a `'static`-
   bounded `MakeWriter` shape that compiles.

Most of the new minors are documentation or marginal hardening
items that can land alongside or after these four fixes. After
those four fixes plus, optionally, the `'connecting'` state
adoption (which doubles as a correctness-and-UX win), the plan is
APPROVE-ready.

---

## Re-Review (Pass 3) — 2026-04-28

**Verdict:** APPROVE

All four Pass 2 blockers have been addressed:

1. **`ReconnectingEventSource` first-construction bug** — resolved by
   introducing `'connecting'` as the initial state (four-value enum:
   `'connecting' | 'open' | 'reconnecting' | 'closed'`). The
   `'closed'` guard in `connect()` no longer blocks the first
   connection. This also cleanly differentiates bootstrap from
   recovery in the SidebarFooter indicator.

2. **`test_sentinel_does_not_kill_already_running_server` start_time
   mismatch** — resolved by sourcing `launcher-helpers.sh` and
   seeding `start_time` from `start_time_of $$`, matching what the
   launcher's reuse short-circuit actually reads back.

3. **`server.bootstrap.log` mode regression** — resolved by
   pre-creating the file with `: > "$BOOTSTRAP_LOG"` followed by
   `chmod 0600` before the nohup append, with a dedicated test
   (`test_bootstrap_log_is_owner_only`) that forces a permissive
   umask to prove the chmod overrides it.

4. **`MutexWriter` lifetime + extraction** — resolved by promoting
   the extraction to required. `MutexWriter<W>(Arc<Mutex<W>>)` lives
   in `log::test_support` with a proper `for<'a> MakeWriter<'a>`
   impl via `MutexGuardWriter<'a, W>`.

### Per-Lens Results (Pass 3)

| Lens | Verdict | Blocking Issues |
|------|---------|-----------------|
| Architecture | APPROVE | None |
| Code Quality | APPROVE | None |
| Test Coverage | APPROVE | None |
| Correctness | APPROVE | None |
| Security | APPROVE | None |
| Usability | APPROVE | None |
| Compatibility | APPROVE | None |
| Standards | APPROVE | None |

### Remaining Minor Notes (non-blocking)

- **Code Quality**: The `useDocEvents` reconnect integration test
  uses JSX but lives in a `.test.ts` file — implementer should use
  `.test.tsx` or extract to a separate file.
- **Architecture**: `onmessage` is assigned post-construction while
  `onerror`/`onReconnect` are constructor options — minor API
  asymmetry matching the native EventSource shape.

### Assessment

The plan is structurally sound, test-complete, and ready for
implementation. All critical and major findings from Passes 1 and 2
are resolved with locked-in design decisions, concrete code snippets,
and regression tests. The remaining notes are minor ergonomic items
the implementer can address inline without plan revision.

---
*Pass 3 review generated by /review-plan*

