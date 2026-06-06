---
type: plan-review
id: "2026-06-06-0100-configurable-visualiser-auto-shutdown-review-1"
title: "Plan Review: Configurable Visualiser Auto-Shutdown"
date: "2026-06-06T17:27:29+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-06-0100-configurable-visualiser-auto-shutdown"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: [architecture, correctness, test-coverage, code-quality, usability, compatibility, standards, documentation]
review_number: 1
review_pass: 3
tags: [visualiser, server, configuration, lifecycle]
last_updated: "2026-06-06T19:06:55+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Configurable Visualiser Auto-Shutdown

**Verdict:** REVISE

This is a well-structured, well-grounded plan — it faithfully mirrors the
`kanban_columns` precedent across all four layers, uses a clean single-parse-path
design for the default, reuses the proven injectable `Settings` test seam, and
sequences phases for independent mergeability. The verdict is REVISE not because
the design is wrong but because the plan accumulates ten major findings (REVISE
threshold: three): a cluster of *verifiable factual errors* about the codebase it
references (a non-existent `mise` task, a doc precedent that doesn't exist, a
mislabeled error wrapper), plus a genuine end-to-end gap in the error experience
and the timeout's test coverage. Most are low-effort, high-value corrections.

### Cross-Cutting Themes

- **Plan cites codebase structures that don't exist** (flagged by: standards,
  documentation) — `mise run typecheck` (Phase 2), a `binary` row +
  `ACCELERATOR_VISUALISER_BIN` note in the configure SKILL.md table to "mirror"
  (Phase 4), and a `binary` key in `.accelerator/config.md` (Phase 4.3) are all
  referenced but verified absent. An implementer following these verbatim stalls
  or improvises.
- **The fail-fast error experience is broken end-to-end** (flagged by: usability,
  code-quality ×2) — the carefully-worded `InvalidIdleTimeout` message is (a)
  wrapped by `AppStateError::Config`'s hard-coded `"invalid work-item config"`
  prefix (mislabeling it), (b) built with `map_err(|_| …)` that discards
  humantime's underlying reason, and (c) written only to a tracing log because
  stderr is `/dev/null`'d after config load — so the user sees a silent launch
  failure with no on-screen reason. Three findings describe one compounding
  failure.
- **The zero / disable / overflow boundary is under-specified and under-tested**
  (flagged by: correctness, compatibility, test-coverage) — `"0"` disables but
  `"0s"`/`"0ms"` parse to a zero window that fires on the first tick;
  `as_millis() as i64` can wrap negative for extreme durations and clamp to 0
  (fire-immediately, the opposite of the clamp's stated intent); and none of these
  edges have planned tests.
- **The env-var override isn't documented where users look** (flagged by:
  usability, documentation) — Phase 4 adds `ACCELERATOR_VISUALISER_IDLE_TIMEOUT`
  only to the configure table, not the README env-var table (README:496) or the
  visualise SKILL.md `### Overrides` section (SKILL.md:83) where its sibling
  `ACCELERATOR_VISUALISER_BIN` lives.
- **Tick-granularity surprise is undocumented for users** (flagged by:
  documentation, usability) — at the 60s production tick a configured `"10s"`
  fires up to ~60s late; the plan notes this for the implementer but no Phase 4
  task documents it, inviting "the setting is ignored" bug reports.

### Tradeoff Analysis

- **Architecture's single-parse-path purity vs Usability's visible errors**:
  Architecture credits keeping all parsing/validation in Rust (one parser,
  fail-fast at the right layer). Usability's strongest finding (invisible error)
  would be most directly solved by *also* validating shell-side in
  `write-visualiser-config.sh`, before stderr is suppressed — which reintroduces a
  second parse location. Recommendation: keep parsing in Rust, but have the
  launcher surface the server's non-zero exit + the tracing-log path/tail (the
  `{"error","hint"}` object the visualise SKILL.md already relays), getting
  visibility without duplicating the parser.
- **Forgiving disable tokens vs work-item scope**: Usability suggests accepting
  `"off"`/`"none"`; the work item explicitly scoped these out (Open Question
  answered "no"). Recommendation: honour the work item — don't broaden — but
  ensure the (now-reachable) error message lists the accepted disable tokens.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Standards**: Referenced mise task `typecheck` does not exist
  **Location**: Phase 2 — Success Criteria / Automated Verification
  `mise run typecheck` is listed, but no such task exists; the closest are
  `check`, `lint:check`, `format:check`. The plan's own parenthetical
  `cargo check --manifest-path …` is the correct fallback.

- 🟡 **Standards + Documentation**: Phase 4 instructs mirroring a configure-table precedent that doesn't exist
  **Location**: Phase 4, Section 1 — Configuration reference
  The plan says to mirror "the existing `kanban_columns` / `binary` row style and
  the `ACCELERATOR_VISUALISER_BIN` env-var note" in `configure` SKILL.md, but that
  table has only a `kanban_columns` row — no `binary` row, no env-var note (those
  live in visualise SKILL.md:83 and README:496). No template to copy.

- 🟡 **Usability + Documentation**: env-var override not documented where users will look
  **Location**: Phase 4 — Documentation (scope)
  `ACCELERATOR_VISUALISER_IDLE_TIMEOUT` is added only to the configure table, not
  the README env-var table (README:496) nor the visualise `### Overrides` section
  (SKILL.md:83) where its sibling `ACCELERATOR_VISUALISER_BIN` is documented.

- 🟡 **Usability**: Fail-fast error is well-worded but unreachable
  **Location**: Phase 2, Section 7 (stderr → /dev/null note)
  The descriptive `InvalidIdleTimeout` message lands only in a tracing log file
  (stderr is suppressed after config load, main.rs:34); the user sees only a launch
  that never produces a URL. Fail-fast that fails silently from the user's
  vantage point.

- 🟡 **Code Quality**: Reused `AppStateError::Config` variant mislabels idle-timeout failures
  **Location**: Phase 2, Section 4
  Routing through `AppStateError::Config` (whose `#[error]` is
  `"invalid work-item config: {0}"`, server.rs:145) logs an idle-timeout failure
  as `…invalid work-item config: invalid visualiser.idle_timeout 'soon'…` — the
  wrapper misattributes the failure.

- 🟡 **Code Quality**: Resolver discards the underlying `humantime` parse error
  **Location**: Phase 2, Section 3 (`map_err(|_| …)`)
  The sibling `ConfigError::InvalidScanRegex` carries a `source` field
  (config.rs:319-321); `InvalidIdleTimeout` as planned drops it, so the boot log
  names the bad value but not *why* humantime rejected it.

- 🟡 **Correctness**: `as_millis() as i64` can wrap negative for extreme durations, defeating the anti-disable clamp
  **Location**: Phase 2, Section 3 (clamp expression)
  `as_millis()` is `u128`; humantime accepts arbitrarily large unit counts. A value
  above `i64::MAX` ms truncates (not saturates) on the `as i64` cast to a negative,
  which `.clamp(0, …)` pins to `0` — turning an absurdly-large timeout into
  fire-immediately, the exact inverse of the clamp's stated purpose. Fix: `min` in
  `u128` space before the cast.

- 🟡 **Test Coverage**: No boundary-survival assertion
  **Location**: Phase 2, Section 6 / Testing Strategy
  Lifecycle tests assert the timeout *fires by* a deadline but never that the
  server is *still alive* just below it. An off-by-tick or fire-unconditionally
  mutation of `idle >= idle_limit_ms` would pass. The work item's "within tolerance
  of D" is two-sided.

- 🟡 **Test Coverage**: Several acceptance criteria exercised only at resolver-math level; the resolve→wire→loop chain is untested
  **Location**: Phase 2, Sections 4-5
  ACs 1/2/3/7 map only to `resolve_idle_limit_ms` unit tests. The wiring (resolve →
  store on `AppState` → build `Settings`) has no test, so passing
  `Settings::DEFAULT.idle_limit_ms` instead of `state.idle_limit_ms` would pass
  every listed test.

- 🟡 **Documentation + Usability**: Tick-granularity caveat undocumented for users
  **Location**: Phase 3 Manual Verification / Phase 4
  A configured `"10s"` fires up to ~60s late at the production tick; noted for the
  implementer but no Phase 4 doc task captures it for users.

#### Minor

- 🔵 **Correctness + Compatibility**: `"0"` disables but `"0s"`/`"0ms"`/`"0m"`
  parse to a zero window that fires on the first tick — an inconsistent boundary.
  (Phase 2, Section 3) Consider treating any `dur.is_zero()` as disabled, or
  test+document the distinction.
- 🔵 **Correctness**: Empty/whitespace env var (`ACCELERATOR_VISUALISER_IDLE_TIMEOUT=""`)
  silently falls through to config via `:-`; defensible but undocumented and
  untested. (Phase 3, Section 1)
- 🔵 **Code Quality**: 8h magic-number `8*60*60*1000` duplicated; after Phase 2,
  `Settings::DEFAULT.idle_limit_ms` becomes shadow state that can drift from
  `DEFAULT_IDLE_TIMEOUT = "8h"`. (Phase 1/2)
- 🔵 **Code Quality**: Clamp mixes a narrowing cast with a sentinel-adjacent bound
  (`DISABLED_IDLE_LIMIT_MS - 1`) whose intent lives only in a comment.
  (Phase 2, Section 3)
- 🔵 **Architecture**: Disable sentinel `i64::MAX` is an implicit cross-module
  contract (config.rs / lifecycle.rs / tests) shared by value, not an imported
  named constant. (Phase 2, Section 3)
- 🔵 **Architecture**: Resolved `idle_limit_ms` on `AppState` duplicates state
  reachable via `cfg.idle_timeout`, inviting a future re-parse. (Phase 2, Section 4)
- 🔵 **Architecture**: `"0" → disabled` inverts the loop's natural
  `idle_limit_ms = 0 → fire immediately` semantics; correct per the work item but
  make the inversion legible at the loop site. (Desired End State)
- 🔵 **Architecture + Correctness**: Wall-clock idle basis is amplified 16× over the
  8h window — a forward NTP/clock step can trip a spurious shutdown mid-review (the
  exact scenario the work item set out to prevent). (Performance Considerations)
- 🔵 **Test Coverage**: `disabled_idle_never_fires` is a sleep-based negative
  assertion (proves "didn't fire in 300ms", not "never"); pair with the direct
  resolver assertion. (Phase 2, Section 6)
- 🔵 **Test Coverage**: Fail-fast test must assert exit code 1 (not merely
  non-zero) and use an otherwise-valid fixture — `config_cli.rs` already uses exit
  code 2 for missing config. (Phase 2, Section 7)
- 🔵 **Test Coverage**: No coverage for sub-tick / near-zero durations or the
  0-vs-disable distinction at the loop level. (Phase 2, Section 5)
- 🔵 **Compatibility**: A user pinning an older server via
  `ACCELERATOR_VISUALISER_BIN`/`visualiser.binary` while a newer launcher emits
  `idle_timeout` hits `deny_unknown_fields` boot failure; note as accepted
  behaviour. (Phase 3, Section 1)
- 🔵 **Compatibility**: `humantime` license/MSRV/transitive-deps claim is asserted
  but not verified; the `"2"` caret admits future `2.x` MSRV bumps above
  `rust-version = 1.85`. (Phase 2, Section 1)
- 🔵 **Standards**: Stale `lifecycle.rs:23-26` line references in Current State
  Analysis / References will drift once Phase 1 edits land; prefer symbol anchors.
- 🔵 **Standards**: New resolver doc-comment diverges from the sibling
  `resolve_kanban_columns` `Semantics:`-heading style. (Phase 2, Section 3)
- 🔵 **Standards**: Fixture `config.bad-idle.json` — consider
  `config.invalid-idle-timeout.json` to match the `missing-required` descriptor
  style. (Phase 2, Section 7)
- 🔵 **Documentation**: Config docs should state precisely that `never`/`0` disable
  idle shutdown (not "immediate"), and that owner-exit + `stop` still terminate.
  (Phase 4, Section 1)
- 🔵 **Documentation**: Record the 30m→8h default raise as its own `### Changed`
  CHANGELOG line, not buried in the feature entry. (Phase 4, Section 2)

#### Suggestions

- 🔵 **Code Quality**: Consider storing resolved `lifecycle::Settings` (or an
  `IdleConfig` newtype) on `AppState` rather than a bare `i64`, matching how kanban
  stores its resolved type. (Phase 2, Section 4)
- 🔵 **Usability + Documentation**: Promote the `.accelerator/config.md` commented
  example from "optional" to recommended — and fix it (the file has a
  `kanban_columns` key, not `binary`, and no comments today). (Phase 4, Section 3)

### Strengths

- ✅ Faithfully reuses the `kanban_columns` template end-to-end (serde
  `Option<String>` + `#[serde(default)]`, a `Config::resolve_*` method,
  `?`-propagation in `AppState::build`, boot-time fail-fast) — verified against the
  actual code.
- ✅ Single canonical parse path: the 8h default is the string `"8h"` run through
  the same resolver as user input, eliminating default-vs-configured drift.
- ✅ Reuses the existing injectable `Settings` seam instead of introducing an
  unnecessary `Clock` trait, and explicitly scopes that abstraction out as YAGNI.
- ✅ Correctly identifies that the three shutdown triggers converge on one channel
  and that disabling idle is a purely local change — owner-exit and SIGTERM remain
  in force.
- ✅ The `i64::MAX` disable sentinel is verified-inert against the
  `idle >= idle_limit_ms` comparison and matches the shipped
  `tests/lifecycle_owner.rs` usage.
- ✅ Phases are sequenced for independent mergeability, TDD-first, with the
  default-raise landable alone.
- ✅ Migration is purely additive — old `config.json` and existing test callers
  omit the key and default cleanly.

### Recommended Changes

1. **Fix the three factual errors** (addresses: typecheck task; non-existent
   configure-table precedent; config.md binary key). Replace `mise run typecheck`
   with `cargo check --manifest-path …` (or `mise run check`); rewrite Phase 4.1 to
   mirror only the real `kanban_columns` row and decide explicitly how the env
   override is surfaced in the configure reference; correct the Phase 4.3 example to
   the real `kanban_columns`-only file.

2. **Repair the error experience end-to-end** (addresses: unreachable error;
   mislabeled wrapper; discarded source). Add `source: humantime::DurationError` to
   `InvalidIdleTimeout` (mirror `InvalidScanRegex`); rename/neutralise the
   `AppStateError::Config` message (e.g. `"invalid configuration: {0}"`); and have
   the launcher surface the server's non-zero exit + tracing-log path/tail so the
   user sees *why* the launch failed.

3. **Close the timeout test gaps** (addresses: no boundary-survival assertion;
   untested resolve→wire→loop chain). Add a finite-limit lifecycle test asserting
   *no* shutdown below the threshold then firing shortly after; add a test that the
   resolved value actually reaches `spawn`; and map each of the 8 acceptance criteria
   to a named test in the Testing Strategy.

4. **Harden the zero/overflow boundary** (addresses: `as_millis() as i64` wrap;
   `"0s"` vs `"0"`). Saturate in `u128` before the `i64` cast; decide and test
   whether any zero-valued duration disables or fires-immediately; add resolver tests
   for a multi-billion-year value, `"0s"`, `"00"`, and a whitespace-only value.

5. **Document the behaviour users will hit** (addresses: env-var discoverability;
   tick granularity; disable semantics; default-raise changelog). Add the env
   override to the README env-var table and visualise `### Overrides` section;
   document the ~60s tick granularity; state `never`/`0` = disable (not immediate)
   with owner-exit/stop still active; and record the 30m→8h raise as a `### Changed`
   line.

6. **Optional polish** (addresses: minor architecture/standards). Import a shared
   `DISABLED_IDLE_LIMIT_MS` in lifecycle.rs + tests; align the resolver doc-comment
   to the `Semantics:` style; prefer symbol-anchored references; note the
   pinned-old-binary `deny_unknown_fields` interaction as accepted; verify the
   `humantime` license/MSRV.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: Structurally sound and faithful to the `kanban_columns` precedent
end-to-end. Respects functional-core/imperative-shell (a pure resolver plus the
pre-existing injectable `Settings` seam), keeps the three shutdown triggers
structurally independent, and explicitly acknowledges its tradeoffs (wall-clock
basis, no Clock abstraction, no ADR). Main concerns are minor: an implicit
cross-module sentinel contract and a duplicated source of the resolved value.

**Strengths**:
- Faithfully reuses the kanban_columns architectural template, minimising
  divergence from established patterns.
- Preserves functional core / imperative shell: a pure string→i64 resolver plus
  the injectable lifecycle::Settings seam.
- Correctly identifies that disabling idle is a purely local change to the idle
  comparison; owner-exit and SIGTERM remain untouched.
- The shell-resolves-precedence / Rust-parses-and-validates split is the right
  boundary given the env var never reaches the server process.
- Default expressed as `"8h"` through the same parser as user input — one code
  path for all values.

**Findings**:
- 🔵 **minor** (confidence: high) — _Phase 2, Section 3_ — Disable sentinel is an
  implicit cross-module contract, not a shared abstraction. `DISABLED_IDLE_LIMIT_MS
  = i64::MAX` is coupled across config.rs, lifecycle.rs and tests by value, not an
  imported constant; a future refactor of the idle comparison could silently break
  disable with no compile-time signal. Have lifecycle.rs and lifecycle_owner.rs
  reference the same exported constant.
- 🔵 **minor** (confidence: medium) — _Phase 2, Section 4_ — Resolved `idle_limit_ms`
  on `AppState` duplicates state already reachable via `cfg.idle_timeout`. Matches
  the kanban precedent and resolves-once is correct; minor risk of a future re-parse.
  Either document that cfg is the raw input and AppState the resolved output, or
  resolve inline at the single spawn site.
- 🔵 **minor** (confidence: high) — _Desired End State / Key Discoveries_ — `"0"`
  mapping to disabled diverges from the loop's natural `idle >= 0` (always true =
  immediate) semantics. Correct per the work item, but make the inversion explicit
  at the loop site.
- 🔵 **minor** (confidence: medium) — _Performance Considerations / What We're NOT
  Doing_ — Wall-clock idle basis is a more significant resilience surface over the
  8h window (16× the old 30m). A forward clock correction could shut the server
  down mid-review — the exact scenario the work item set out to prevent. Deferral is
  acknowledged; consider a follow-up to monotonic time.

### Correctness

**Summary**: Core logic is sound and well-grounded. The i64::MAX disable sentinel
is genuinely inert against `idle >= idle_limit_ms`, the shell-resolves/Rust-parses
precedence split is coherent, and the single-parse-path default is a clean
invariant. Main risks are in the resolver's duration arithmetic: the
`dur.as_millis() as i64` cast can wrap negative for extreme durations and clamp to
0 (fire-immediately), and the disable-token set treats bare `"0"` as disable while
`"0s"`/`"0ms"` parse to a zero window that fires on the first tick.

**Strengths**:
- The i64::MAX disable sentinel is verified-correct against the lifecycle.rs:46
  comparison and matches tests/lifecycle_owner.rs:25.
- The single-parse-path invariant eliminates a divergent-default class of bug.
- Owner-PID and SIGTERM/SIGINT triggers are correctly identified as structurally
  independent of the idle comparison.
- The fail-fast exit-code reasoning is correct (code 1 via
  ConfigError → AppStateError → ServerError::Startup, distinct from code-2 missing).

**Findings**:
- 🟡 **major** (confidence: medium) — _Phase 2, Section 3_ — `as_millis() as i64`
  can wrap negative for extreme durations, defeating the anti-disable clamp.
  `as_millis()` is u128; humantime accepts arbitrarily large unit counts; a value
  above i64::MAX ms truncates to a negative on the cast, which `.clamp(0, …)` pins
  to 0 — immediate shutdown, the inverse of intent. Saturate before the cast:
  `dur.as_millis().min((DISABLED_IDLE_LIMIT_MS - 1) as u128) as i64`; add a
  multi-billion-year resolver test asserting near-MAX, not 0.
- 🔵 **minor** (confidence: high) — _Phase 2, Sections 3 & 5_ — Bare `"0"` disables
  but `"0s"`/`"0ms"`/`"0m"` parse to a zero window (fire on first tick). Either
  normalise `dur.is_zero()` to the disable sentinel, or document/test that only bare
  `0`/`never` disable.
- 🔵 **minor** (confidence: medium) — _Phase 2, Section 1 / Phase 1_ — 8h-in-ms and
  the i64 idle arithmetic are safe; residual edge is the wall-clock basis (a forward
  step can trip the threshold early over the 16× window). Note in the doc comment.
- 🔵 **minor** (confidence: medium) — _Phase 3, Section 1_ — Empty-string env var
  falls through to config via `:-`; defensible but undocumented. Add a shell test for
  `=""` falling through and a resolver test for a whitespace-only value asserting
  InvalidIdleTimeout.
- 🔵 **minor** (confidence: low) — _Phase 2, Section 3_ — Disable matching is
  asymmetric (`never` case-insensitive, `0` exact). Correct per the work item; add
  tests for `"00"`/`"0.0"` asserting InvalidIdleTimeout to lock the boundary.

### Test Coverage

**Summary**: Well-thought-out test split: resolver math by unit tests, the disable
sentinel and idle-firing by lifecycle integration tests, precedence by shell tests,
fail-fast by a fixture-driven CLI test. Reuses the injectable-Settings seam to avoid
real-time waits. Main gaps are the absence of boundary-survival assertions, reliance
on a sleep-based negative assertion for disable, and acceptance criteria mapped only
to resolver math rather than observable lifecycle behaviour.

**Strengths**:
- Uses the injectable `Settings` seam to test the boundary without a real-time wait.
- Splits coverage sensibly across the pyramid (resolver units, lifecycle integration,
  shell precedence).
- Adopts the proven kanban resolver-test and shell-test patterns and the
  EmptyKanbanColumns fail-fast template.
- Includes the very-large-duration clamp edge case.
- Asserts fail-fast on exit code rather than stderr text.

**Findings**:
- 🟡 **major** (confidence: high) — _Phase 2, Section 6 / Testing Strategy_ — No
  boundary-survival assertion: tests confirm the timeout fires but never that the
  server stays alive before the deadline. An off-by-tick or fire-unconditionally
  mutation would pass. Add a test asserting no shutdown below the threshold then a
  fire shortly after.
- 🟡 **major** (confidence: high) — _Phase 2, Sections 4-5_ — Several acceptance
  criteria exercised only at resolver-math level; the resolve→wire→loop chain is
  untested, so passing `Settings::DEFAULT.idle_limit_ms` instead of
  `state.idle_limit_ms` would pass every test. Add an AppState::build → lifecycle
  test and map each AC to a named test.
- 🔵 **minor** (confidence: medium) — _Phase 2, Section 6_ — `disabled_idle_never_fires`
  is sleep-based (proves "didn't fire in 300ms"). Pair with the direct resolver
  assertion that the disable tokens return DISABLED_IDLE_LIMIT_MS.
- 🔵 **minor** (confidence: medium) — _Phase 2, Section 7_ — Fail-fast fixture must
  assert exit code 1 (config_cli.rs uses 2 for missing config) and be an
  otherwise-valid config so the resolver is what fails.
- 🔵 **minor** (confidence: medium) — _Phase 2, Section 5_ — No coverage for sub-tick
  / near-zero durations or the 0-vs-disable distinction at the loop level.

### Code Quality

**Summary**: A careful plumbing job that faithfully mirrors the kanban_columns
precedent with strong TDD discipline and clear doc comments. Concerns are localised
to Phase 2: the resolver discards the underlying humantime error via
`map_err(|_| …)`, the reused `AppStateError::Config` variant mislabels idle-timeout
failures as "invalid work-item config", and several magic-number duration
computations plus an unguarded sentinel/clamp leave brittle arithmetic.

**Strengths**:
- Faithfully mirrors the resolve_kanban_columns precedent.
- Single canonical parse path for the default.
- Strong testability via the existing injectable seam; Clock trait correctly scoped
  out as YAGNI.
- Clear doc comments explaining the disable-sentinel semantics.
- The resolver is a pure function, easy to unit test.

**Findings**:
- 🟡 **major** (confidence: high) — _Phase 2, Section 3_ — Resolver discards the
  underlying humantime parse error. The sibling InvalidScanRegex carries a `source`;
  add `source: humantime::DurationError` to InvalidIdleTimeout and map with
  `|source| …`.
- 🟡 **major** (confidence: high) — _Phase 2, Section 4_ — Reused
  `AppStateError::Config` variant mislabels idle-timeout failures (its `#[error]` is
  `"invalid work-item config: {0}"`, server.rs:145). Rename to a domain-neutral
  message like `"invalid configuration: {0}"`.
- 🔵 **minor** (confidence: high) — _Phase 1 / Phase 2, Section 3/5_ — 8h magic-number
  arithmetic duplicated; after Phase 2, `Settings::DEFAULT.idle_limit_ms` becomes
  shadow state that can drift from `DEFAULT_IDLE_TIMEOUT = "8h"`. Pick one source of
  truth and drop or assert-equal the unused default.
- 🔵 **minor** (confidence: medium) — _Phase 2, Section 3_ — Clamp mixes a narrowing
  cast with a sentinel-adjacent bound; correctness depends on the clamp catching a
  prior overflow. Clamp in u128 first, or extract a named MAX_IDLE_LIMIT_MS.
- 🔵 **suggestion** (confidence: medium) — _Phase 2, Section 4_ — Primitive `i64` on
  AppState plus Settings reassembled inline in run(); differs from kanban storing the
  resolved domain type. Consider storing resolved Settings or an IdleConfig newtype.

### Usability

**Summary**: Mirrors the kanban_columns precedent end-to-end and gets the core
ergonomics right (env > config > default precedence, familiar humantime format,
zero-config 8h default, forgiving trimming/case-insensitive tokens). The principal
concern is the error experience: the descriptive fail-fast error is written only to
a tracing log (stderr is /dev/null'd after config load), so a developer who
fat-fingers a duration sees only a non-zero exit and a missing server-info.json.
Secondary: env-var discoverability, the narrow disable surface, and an
under-surfaced tick-granularity surprise.

**Strengths**:
- Precedence and env-var naming faithfully mirror ACCELERATOR_VISUALISER_BIN.
- Sensible zero-config defaults; the default flows through the same parse path.
- Forgiving input handling (trimming, case-insensitive never, compound durations).
- Fail-fast-at-boot rather than silent fallback is the right call.
- Purely additive, optional migration.

**Findings**:
- 🟡 **major** (confidence: high) — _Phase 2, Section 7_ — Fail-fast error is
  well-worded but unreachable: it lands only in a tracing log the user is never told
  about. Resolve/validate shell-side, OR have the launcher surface the non-zero exit
  + log path/tail in the error object the visualise SKILL.md already relays.
- 🟡 **major** (confidence: high) — _Phase 4, Section 1_ — Env var documented only in
  the config table; the analogous Overrides section (visualise SKILL.md:83) and
  README env-var table (README:496) — where developers actually look — are untouched.
- 🔵 **minor** (confidence: high) — _Phase 3 Manual Verification / Performance_ —
  Tick-granularity surprise (configured 10s but fires up to 60s late) is noted for
  the implementer but not for the end user. Add a one-line note to the configure row.
- 🔵 **minor** (confidence: medium) — _What We're NOT Doing / Phase 2, Section 3_ —
  Narrow disable-token surface (never/0 only) invites near-miss failures
  (off/none/disabled/0s) that fail-fast rather than degrade — compounding the
  invisible-error finding. Honour the work item but ensure the message lists the
  tokens and is reachable.
- 🔵 **suggestion** (confidence: medium) — _Phase 4, Section 3_ —
  Documentation-by-example in config.md is marked optional but is the
  highest-leverage discoverability lever; promote to recommended.

### Compatibility

**Summary**: Fundamentally sound: the new `idle_timeout` field is purely additive,
gated behind `#[serde(default)] Option<String>`, and the launcher omits the key when
unset, so old config.json files and the existing contract test remain valid. The
plan correctly identifies the deny_unknown_fields constraint and keeps the wire
format a plain string. Residual concerns: the pinned-older-binary forward-compat
case and unverified humantime dependency claims.

**Strengths**:
- Additive-only schema change mirroring the proven kanban_columns/work_item pattern.
- Correctly honours deny_unknown_fields and omits the key when unset.
- Keeps the shell/Rust contract a plain unparsed string with a single parse path.
- Explicitly verifies config_contract.rs stays green and the omit-when-unset path is
  exercised.
- Reuses the shipped i64::MAX disable convention rather than a new wire value.

**Findings**:
- 🔵 **minor** (confidence: medium) — _Phase 3, Section 1_ — A user pinning an older
  server via ACCELERATOR_VISUALISER_BIN / visualiser.binary while a newer launcher
  emits idle_timeout would hit a deny_unknown_fields boot failure. Note as accepted
  behaviour (consistent with kanban_columns) or document the same-or-newer-server
  contract.
- 🔵 **minor** (confidence: medium) — _Phase 2, Section 1_ — humantime
  license/transitive/MSRV claim is unverified, and the `"2"` caret admits future
  2.x MSRV bumps above rust-version 1.85. Add a license/MSRV check to success
  criteria.
- 🔵 **minor** (confidence: high) — _Phase 3 / Phase 2, Section 3_ —
  config-read-value.sh strips quotes (quoted/unquoted both arrive bare, validating
  that test case), but `"0s"` would parse to a zero duration rather than the disable
  sentinel. Add a unit test asserting `"0s"` is NOT treated as disabled.

### Standards

**Summary**: Generally faithful to the kanban_columns precedent across all four
layers, and the naming it introduces is consistent with existing conventions. Main
gaps: a non-existent mise task referenced in success criteria, and a
documentation-mirroring instruction pointing at a precedent row/note that does not
exist in the target file. Doc-comment style, thiserror phrasing, and CHANGELOG
placement otherwise track established patterns.

**Strengths**:
- Naming conventions consistent end-to-end (idle_timeout, ACCELERATOR_VISUALISER_IDLE_TIMEOUT,
  InvalidIdleTimeout, DEFAULT_IDLE_TIMEOUT/DISABLED_IDLE_LIMIT_MS).
- serde field convention faithfully copied.
- resolve_*→Result<_, ConfigError> shape and ?-propagation match the precedent.
- thiserror message names the bad value and lists accepted forms.
- Shell-side precedence resolution follows the launcher pattern; parsing stays in Rust.
- CHANGELOG entry correctly placed under [Unreleased].

**Findings**:
- 🟡 **major** (confidence: high) — _Phase 2, Success Criteria_ — Referenced mise task
  `typecheck` does not exist (tasks are check, lint:check, format:check). Replace with
  `cargo check --manifest-path …` or `mise run check`.
- 🟡 **major** (confidence: high) — _Phase 4, Section 1_ — Plan instructs mirroring a
  `binary` row + ACCELERATOR_VISUALISER_BIN note in configure SKILL.md that does not
  exist (only kanban_columns is there; the binary docs live in visualise SKILL.md and
  README). Correct the precedent reference and decide how the env override is shown.
- 🔵 **minor** (confidence: high) — _Phase 1, Section 2 / References_ — Stale
  lifecycle.rs:23-26 line references will drift once Phase 1 edits land; prefer
  symbol-anchored references.
- 🔵 **minor** (confidence: medium) — _Phase 2, Section 3_ — Resolver doc-comment
  diverges from the sibling resolve_kanban_columns `Semantics:`-heading style.
- 🔵 **minor** (confidence: medium) — _Phase 2, Section 7_ — Fixture
  `config.bad-idle.json` — consider `config.invalid-idle-timeout.json` to match the
  `missing-required` descriptor style.

### Documentation

**Summary**: Documents most new behaviour, and the line references for the
default-change wording (SKILL.md:39/63, README.md:474, configure table) are accurate.
However, Phase 4 has a factual error about the doc structure it claims to mirror,
omits the env-var override from the two places users actually discover visualiser env
vars, never documents the tick-granularity caveat or the disable-token semantics in a
user-facing location, and leaves the config.md example as a vague (and factually
wrong) "optional".

**Strengths**:
- Phase 1 correctly identifies every stale "30 minutes" string and pairs the change
  with a grep success criterion; line references verified accurate.
- The configure SKILL.md visualiser table is the correct canonical home, anchored to
  the kanban_columns row style.
- Migration Notes and the additive/optional framing are accurate.

**Findings**:
- 🟡 **major** (confidence: high) — _Phase 4, Section 1_ — Instructs mirroring a
  `binary` row / ACCELERATOR_VISUALISER_BIN note in the configure reference that does
  not exist there. Correct to mirror the real kanban_columns row and decide how the
  env override is surfaced.
- 🟡 **major** (confidence: high) — _Phase 4 (scope)_ — The env override is documented
  only in the configure table, touching neither the README env-var table (README:494+)
  nor the visualise Overrides section (SKILL.md:83+) where the sibling binary override
  lives. Add it to both.
- 🟡 **major** (confidence: high) — _Phase 3 Manual Verification / Phase 4_ — The
  ~60s tick-granularity caveat is acknowledged for the implementer but no Phase 4 doc
  task captures it for users. Add a one-line note that the timeout is honoured to
  within one ~60s tick.
- 🔵 **minor** (confidence: high) — _Phase 4, Section 3_ — The config.md example is
  marked optional and is factually wrong (the file has a kanban_columns key, not
  binary, and no comments). Either drop it or make it concrete and committed.
- 🔵 **minor** (confidence: medium) — _Phase 4, Section 1_ — Disable semantics should
  be stated precisely: `0` disables (not "immediate"); never/0 disable only the idle
  trigger while owner-exit and stop still terminate.
- 🔵 **minor** (confidence: medium) — _Phase 4, Section 2_ — The 30m→8h default raise
  is a user-observable behaviour change; record it as its own `### Changed` CHANGELOG
  line, not buried in the feature entry.

## Re-Review (Pass 2) — 2026-06-06T18:04:23+00:00

**Verdict:** REVISE

All ten Pass-1 major findings were resolved by the edits — the three factual
errors, the error-experience chain, the test gaps, the overflow/zero boundary, and
the documentation surfaces all check out against the codebase. However, the deeper
pass found that one Pass-1 fix — the shell-side pre-flight validation added to make
the error visible — **introduced a critical regression**: the coarse guard regex
rejects the headline compound-duration form `"1h30m"`. With one critical finding
present, the verdict remains REVISE (the configured threshold is any
critical). The regression and the two accompanying majors are small, well-understood
fixes.

### Previously Identified Issues (Pass 1 → Pass 2)

- 🟡 **Standards**: `mise run typecheck` doesn't exist — **Resolved.** Phase 2 now uses `cargo check`; (a stale `typecheck` mention lingered in Phase 4 — re-flagged minor below).
- 🟡 **Standards + Documentation**: Phase 4 mirrored a non-existent configure-table precedent — **Resolved.** Correction note added; verified the table holds only `kanban_columns`.
- 🟡 **Usability + Documentation**: env override not documented where users look — **Resolved.** Added to README env table + visualise Overrides section.
- 🟡 **Usability**: fail-fast error unreachable — **Resolved.** Shell-side pre-flight validation surfaces a terminal error (but see the regression it introduced).
- 🟡 **Code Quality**: `AppStateError::Config` mislabels error — **Resolved.** Renamed to `"invalid configuration"`.
- 🟡 **Code Quality**: resolver discards humantime source — **Resolved.** `source: humantime::DurationError` added, mirroring `InvalidScanRegex`.
- 🟡 **Correctness**: `as_millis() as i64` wrap — **Resolved.** Now saturates in `u128` via `MAX_IDLE_LIMIT_MS`.
- 🟡 **Test Coverage**: no boundary-survival assertion — **Resolved.** `idle_survives_below_threshold_then_fires` added.
- 🟡 **Test Coverage**: resolve→wire→loop untested — **Resolved.** Wiring test + AC coverage map added (test file naming re-flagged minor below).
- 🟡 **Documentation + Usability**: tick-granularity undocumented — **Resolved.** Documented in Phase 4.1.
- 🔵 Pass-1 minors (zero-token boundary, empty-env fall-through, magic numbers, sentinel sharing, fixture name, references, CHANGELOG Changed entry, disable-semantics docs, license/MSRV, pinned-binary compat) — **Resolved/addressed.**

### New Issues Introduced

- 🔴 **Correctness / Usability / Compatibility** (critical): The shell pre-flight guard regex `^[0-9]+[A-Za-zµ ]*$` (Phase 3, Section 1) **rejects the compound form `"1h30m"`** — after the leading digit run, the embedded `30` is not in the `[A-Za-zµ ]` class, so `grep` fails and the launcher exits 1. `"1h30m"` is documented as valid throughout the plan (Overview, Desired End State, the guard's own error message) and is acceptance criterion 3; `humantime` accepts it. The guard is *stricter* than the authoritative Rust parser for the central use case — a self-introduced regression from the Pass-1 shell-validation edit. Fix: anchor on a leading digit but allow digits throughout, e.g. `^[0-9][0-9A-Za-zµ. ]*$`; add `"1h30m"`/`"1h 30m"` shell test cases.
- 🟡 **Correctness** (major): A sub-millisecond non-zero duration (`"1ns"`, `"500us"`) passes `dur.is_zero()` but truncates to `0` at `as_millis()`, yielding `idle_limit_ms == 0` → fires on the first tick. A non-zero window silently collapses to immediate shutdown. Fix: clamp the finite path to a minimum of 1 ms (`.max(1)`), and add a resolver test.
- 🟡 **Test Coverage** (major): AC5/AC7 require that with idle disabled the server *still* exits on owner-exit/stop, but the only disable test (`disabled_idle_never_fires`) uses `owner_pid: 0`, neutralising the owner trigger — so the "disable idle, not all triggers" contract is unverified. Fix: add an integration test combining `idle_limit_ms: DISABLED_IDLE_LIMIT_MS` with a dead owner PID asserting `OwnerPidExited` still fires.
- 🔵 **Correctness / Usability** (minor): the shell guard runs on the untrimmed value and is leading-digit-anchored, so a whitespace-padded `"  8h"` that the Rust resolver trims-and-accepts is rejected by the shell. Trim in the shell so its accept-set is a superset of Rust's.
- 🔵 **Code Quality** (minor): the shell guard comment claims it catches `"5 mins please"`, but that value *passes* the regex (leading digit + letters/spaces). Use a genuinely-rejected example (`"soon"`).
- 🔵 **Code Quality** (minor): the accepted-format guidance string is duplicated verbatim in the Rust `#[error]` and the shell `echo`; add cross-reference "keep in sync" comments at both sites.
- 🔵 **Standards** (minor): a stale `mise run typecheck` reference remains in Phase 4 success criteria, contradicting Phase 2's correction.
- 🔵 **Standards** (minor): the new lifecycle tests use `config::DISABLED_IDLE_LIMIT_MS` and `Duration` without the required `use` additions; note the import changes for `lifecycle_idle.rs` and `lifecycle_owner.rs`.
- 🔵 **Standards** (minor): place `### Added` before `### Changed` under `[Unreleased]` per Keep-a-Changelog ordering.
- 🔵 **Test Coverage** (minor): the resolve→wire→loop test names no target file and assumes `AppState.idle_limit_ms` is inspectable; name the file and confirm the field is `pub`.
- 🔵 **Correctness / Test Coverage** (minor): the drift-guard ties the default to `Settings::DEFAULT` relatively; add one absolute assertion that the absent-field default resolves to `28_800_000` ms.
- 🔵 **Documentation** (minor): the visualise SKILL.md user-relayed line ("auto-exits after 8 hours idle") should note the window is now configurable/disable-able, not assert a flat figure.
- 🔵 **Documentation** (minor): the README customisation table pairs env vars with config keys (binary appears as both); add a `visualiser.idle_timeout` config-key row alongside the env-var row.
- 🔵 **Documentation** (minor): the "first env-var note in the configuration reference" claim is inaccurate (the Jira token env vars are already documented there); point the implementer at that precedent instead.
- 🔵 **Test Coverage** (suggestion): the negative timing assertions (`disabled_idle_never_fires`, boundary-survival) are wall-clock-timed and the repo's visualiser/config suites already flake under CI load; consider `tokio::time::pause`/`advance` for determinism.
- 🔵 **Architecture** (suggestion): the disable sentinel lives in `config.rs` but encodes a `lifecycle.rs` comparison assumption; add a comment at the comparison site naming the sentinel it must stay inert against. A `Settings::production(idle_limit_ms)` constructor would keep Settings assembly cohesive.

### Assessment

The first round of edits fully landed — every Pass-1 finding is resolved and the
plan is materially stronger and factually accurate against the codebase. It does
not reach APPROVE only because the shell-validation fix introduced a critical
false-rejection of the compound `"1h30m"` form (the guard ended up stricter than
the authoritative parser), plus two majors (sub-millisecond collapse-to-immediate,
and the uncovered owner-exit-while-disabled clause). All three are small, localised
fixes — a looser regex, a `.max(1)` clamp, and one integration test — after which
the plan should clear APPROVE.

## Re-Review (Pass 3) — 2026-06-06T19:06:55+00:00

**Verdict:** COMMENT

Re-ran the six lenses that intersected the changed logic (correctness, usability,
compatibility, test-coverage, code-quality, standards). **All Pass-2 issues are
resolved — the critical regression and both majors are fixed — and no new
critical or major findings were raised**, so the verdict drops from REVISE to
COMMENT. The plan is acceptable as-is for implementation; everything remaining is
optional minor/suggestion polish, much of which was already absorbed in this pass.

### Previously Identified Issues (Pass 2 → Pass 3)

- 🔴 **Correctness / Usability / Compatibility**: shell guard rejected valid `"1h30m"` — **Resolved.** The guard is now a `case` accepting disable tokens or any digit-led value after a trim; all three lenses verified it accepts every documented-valid form (compound, spaced, whitespace-padded) and can never reject a value Rust accepts. Regression shell test added.
- 🟡 **Correctness**: sub-millisecond `"1ns"` collapsed to fire-immediately — **Resolved.** `.max(1)` floor confirmed correct; sits after the `is_zero()` disable branch, so zero disables and sub-ms-non-zero floors to 1ms (never 0). Resolver test added.
- 🟡 **Test Coverage**: AC5/AC7 owner-exit-while-disabled uncovered — **Resolved.** Section 6c test added; AC map updated this pass to cite it.
- 🔵 Pass-2 minors (shell trim asymmetry, misleading comment, error-message duplication, stale `typecheck` in Phase 4, CHANGELOG ordering, test-import note, resolve→wire test seam, drift-guard absolute assertion, doc surfaces, "first env-var note" claim, determinism note) — **Resolved/addressed**, verified accurate against the codebase.

### New Issues Surfaced (all minor / suggestion — none blocking)

- 🔵 **Correctness** (minor): whitespace-only value diverges (shell trims to empty → treated as unset/8h; Rust would reject) — **addressed this pass** with an explanatory comment in the trim block (the divergence is benign and now documented).
- 🔵 **Correctness / Standards** (minor): `sed` whitespace class is locale-sensitive — **addressed this pass** by adding `LANG=C` to the trim, consistent with the launcher's locale-hardening.
- 🔵 **Compatibility** (minor): the shell-superset guarantee rests on "all humantime durations are digit-led"; consider a property test pinning the premise against dependency-grammar drift. *(Left as a noted suggestion.)*
- 🔵 **Test Coverage** (minor): AC map omitted 6c for AC5/AC7 — **addressed this pass.** Also: 6c inherits the reaped-PID approach from `lifecycle_owner.rs` (consider recording a start_time for PID-reuse robustness); the wrapper-message rename has no asserting test. *(Latter two left as noted suggestions.)*
- 🔵 **Standards** (minor): the test-import note overstated what's new (`Duration`/`ShutdownReason` already imported; only `config` is new) — **corrected this pass.**
- 🔵 **Code Quality** (minor): error payload reports untrimmed `raw`; example list duplicated across three sites (mitigated by sync comments); doc-comment line ref `:25` — **line ref de-anchored this pass**; the other two left as noted suggestions.
- 🔵 **Usability** (minor/suggestion): the disable-token error message advertises only `never`/0 (not `0s`/`0ms`); the config-reference env override lives in prose; sub-tick granularity. *(All left as noted suggestions; granularity is already a planned doc item.)*

### Assessment

The plan is now in good shape and ready for implementation. The critical
regression introduced in Pass 1's edits was caught in Pass 2 and fixed in Pass 3,
the shell→Rust validation contract is now provably consistent (shell accept-set is
a superset of Rust's), the resolver arithmetic is correct across the zero /
sub-millisecond / overflow boundaries, and the acceptance criteria are fully
mapped to named tests. The remaining findings are minor wording/coverage
refinements an implementer can absorb without re-scoping. COMMENT reflects that
none of the open items block the work; APPROVE is within reach with the handful of
optional suggestions above.
