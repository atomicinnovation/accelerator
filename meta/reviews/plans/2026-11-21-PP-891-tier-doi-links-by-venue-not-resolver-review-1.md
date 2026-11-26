---
type: "plan-review"
id: "2026-11-21-PP-891-tier-doi-links-by-venue-not-resolver-review-1"
title: "Plan Review: Tier DOI Links by Venue, Not by the doi.org Resolver Implementation Plan"
date: "2026-11-21T17:16:01+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-21-PP-891-tier-doi-links-by-venue-not-resolver"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "usability", "performance", "documentation"]
review_number: 1
review_pass: 5
tags: ["research", "research-cli", "queue", "crossref", "pacing"]
last_updated: "2026-11-22T01:23:52+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Tier DOI Links by Venue, Not by the doi.org Resolver Implementation Plan

**Verdict:** REVISE

The plan is test-led and structurally sound. It characterises arXiv
before the refactor, makes pacing a pure domain value, decides ticket
provenance by directory, keeps arXiv's contract and adds the `doi` family
additively. No lens raised a critical finding. The weak point is the new
multi-slot, start-measured pacing gate: four lenses independently found
that it ignores a `Retry-After` written after a caller has reserved its
send time, spaces reservations rather than actual sends, has no ceiling
on a future `last_sent`, and is checked only by real-time tests whose
oracle is the gate's own log and one of whose assertions can never fail.
The research.md and research-topic docs work is also under-scoped for
how much arXiv-only and single-lock wording becomes false.

### Cross-Cutting Themes

- **Reserve-then-sleep pacing breaks shared deferral and real spacing**
  (flagged by: architecture, correctness, performance, test-coverage) —
  `paced` writes a reservation, releases `pacing.lock`, sleeps and sends
  without re-reading state. In the 3-slot polite pool a 429's
  `Retry-After` is ignored by callers already asleep; gaps are guaranteed
  between reserved times, not actual stamps, so the 20 ms / 50 ms real
  slack is consumed by jitter; the deadline check's order relative to
  the reservation write is unstated.
- **Real-time timing tests are weak and slower than stated**
  (flagged by: test-coverage, performance, correctness) — the oracle is
  the gate's own `requests.log`, upper-bound margins are 82–110 ms, the
  polite "no 11 in 1 s" assertion over 10 calls is vacuous, and the
  suite cost is ~40 s, not 10–15 s.
- **Single source for pacing values is ambiguous** (flagged by:
  code-quality, architecture) — `Service::pacing()` in the domain and the
  phase 4/5 adapter bullets both appear to define intervals, slots and
  directories.
- **Two outcome shapes for one engine** (flagged by: code-quality,
  architecture) — arXiv flattens `Queued<FetchOutcome>` back into
  `FetchOutcome` with its own `Waiting`/`Rejected`, while DOI keeps
  `Queued<DoiAnswer>` with a second `Unavailable`.
- **`crossref.mailto` feedback and resolution** (flagged by: usability,
  code-quality, architecture) — the warning does not name the source of
  the value; the unreadable-level warning has no text; the tuple return
  type admits impossible states; the env → local → team ladder is built
  in research-cli rather than the config crate.
- **Docs describe a design the plan replaces** (flagged by:
  documentation, compatibility) — research.md's Pacing/Limits text
  promises shared `arxiv.lock` pacing with older versions, which the
  plan removes.
- **Foreign and legacy directory listing** (flagged by: architecture,
  compatibility) — no failure handling for an unlistable directory and
  no retirement point for the `arxiv-queue/` lookup.

### Tradeoff Analysis

- **Test realism vs determinism**: the plan's multi-process real-time
  tests prove the cross-process property end to end, but test-coverage
  and performance want exact reservation arithmetic in fake-clock
  adapter tests and only hard limits (≤ N per window) in real time.
  Recommend both: deterministic adapter tests for the arithmetic, real
  time for the invariants, measured by the mock's `hit_instants`.
- **Pacing safety vs throughput**: re-validating under `pacing.lock`
  before each send (correctness, performance) adds a lock round-trip per
  call, and a coarser margin costs throughput against the "10 within
  1 s" liveness criterion. Recommend re-validation; it is cheap at these
  rates and is the only way to make the limit a guarantee.
- **Generality vs scope**: architecture suggests a config-crate resolver
  and pluggable foreign-ticket providers; both widen the change. The
  config resolver is worth it (the env var name otherwise lives in two
  places); the provider list can be a one-sentence seam.

### Findings

#### Major

- 🟡 **Architecture + Correctness + Performance**: Reserved callers ignore a `Retry-After` written while they sleep, and the deadline check's order is unspecified
  **Location**: Phase 3, §4 Adapters: state per service (`FilePacingGate::paced`)
  With 3 polite slots, B and C send inside A's deferral because `paced` never re-reads `not_before` after sleeping; the 1-slot public test hides this. A reservation written before a `WaitTooLong` check burns an interval for everyone behind it.
- 🟡 **Correctness + Performance**: Spacing is guaranteed between reservations, not actual sends
  **Location**: Implementation Approach: Pacing is a domain value; Phase 3, §4
  Actual gap = interval + o(k+1) − o(k), so a late earlier send compresses the next gap; real slack is 50 ms (public) and 20 ms (polite), not the overshoot absorption the plan claims.
- 🟡 **Correctness**: `PreviousStart` `send_at` has no ceiling
  **Location**: Phase 3, §1 Domain: services and pacing
  A backward clock step or corrupt state leaves `last_sent` far in the future; every caller returns `WaitTooLong` and writes nothing, wedging the pool.
- 🟡 **Test Coverage**: Polite-pool rate assertion can never fail
  **Location**: Phase 5, §5 Tests (rate, polite pool; mixed pools)
  Ten calls cannot produce eleven stamps, so `interval: 0` survives every polite test.
- 🟡 **Test Coverage**: Real-time rate tests use the gate's own log as oracle, with flaky upper bounds
  **Location**: Phase 4, §5; Phase 5, §5; Implementation Approach
  `requests.log` is written by the code under test; margins of 50–110 ms on post-sleep stamps will flake on loaded CI; spawn latency inside the window is unaccounted for.
- 🟡 **Test Coverage**: Multi-slot serving locks and the split `pacing.lock` have no deterministic adapter tests
  **Location**: Phase 3, §4
  The most concurrency-sensitive new code is first exercised by real-time CLI tests in phase 5; existing single-lock `A/pacing.rs` tests have no stated successor.
- 🟡 **Test Coverage**: Cross-process `Retry-After` test can pass vacuously
  **Location**: Phase 4, §5 (429 with `Retry-After: 5`)
  Nothing proves B was waiting before the deferral ended; no upper bound catches a hung B.
- 🟡 **Test Coverage**: Several settled decisions and error paths are untested
  **Location**: Phase 4 §2, §5; Phase 5 §3, §5; Phase 3 §4
  Undecodable 2xx body, a ticket re-presented after its pool changed, a `StoredBinding` with another service, an unreadable config level and `Response::TimedOut` have no test.
- 🟡 **Test Coverage**: "Phase 2 tests pass unchanged" cannot hold for the domain and adapter tests it cites
  **Location**: Phase 2 mapping table; Phase 3 Success Criteria
  `R/queue.rs` (15 `is_front_given` references), `R/arxiv_fetch.rs` and `A/pacing.rs` must change; the `jj diff --stat` check omits `cli/research/tests`.
- 🟡 **Performance + Test Coverage**: Real-time test cost underestimated by ~3x
  **Location**: Performance Considerations; Phase 4 §5; Phase 5 §5
  Public concurrency alone needs ≥ 20 s (10 × 2 s, 1 slot); total is ~40 s, not 10–15 s, and runs under nextest parallelism that widens jitter.
- 🟡 **Performance**: Fixed 100 ms `ADMISSION_POLL` is coarse against 102/210 ms intervals
  **Location**: Phase 3, §3 (`admitted`); Performance Considerations
  Hand-off lag up to 100 ms resets the reservation chain to `now`, eating the liveness slack; 4.7/s overstates real public-pool throughput.
- 🟡 **Code Quality + Architecture**: Pacing values have no single source
  **Location**: Phase 3 §1 vs Phase 4 §3 and Phase 5 §4
  `Service::pacing()`, the adapter bullets, `PacingGate::pacing()` and `ServiceQueue::join(…, pacing)` all carry the value; slot count and lock-file count could disagree.
- 🟡 **Code Quality + Architecture**: Two outcome shapes, and `unavailable` at two levels
  **Location**: Phase 3, §3; Phase 4, §2 (`DoiAnswer`)
  arXiv flattens `Queued<FetchOutcome>` into a type with its own queue variants; DOI nests `Queued<DoiAnswer>` with a second `Unavailable`.
- 🟡 **Architecture**: `crossref.mailto` resolution is built in research-cli, not config
  **Location**: Phase 5, §3 (`crossref_contact.rs`)
  The env var name is declared on the catalogue `ExtraKey` and hard-coded again in research-cli; `pup.ron` names config as the home for resolution.
- 🟡 **Usability + Code Quality**: The implausible-mailto warning does not name its source; the unreadable-level warning has no text
  **Location**: Phase 5, §3; `main.rs` warning
  A junk env var silently overrides a valid `config.md` and the warning names only the key; `(Option<ContactAddress>, Option<Warning>)` admits impossible states.
- 🟡 **Usability**: A misconfigured mailto is reported only to the researcher subagent
  **Location**: Phase 5, §3; Phase 7
  The per-call `warning:` lands in a subagent's stderr; the profile does not say what to do with it, and no human-facing check exists.
- 🟡 **Documentation**: research.md's arXiv-only fetch wording reaches well beyond the four listed bullets
  **Location**: Phase 4, §6 Docs
  Intro, family table, Output, `rate_limited` row, waiting text, exit-code and E_* rows, `E_RESEARCH_CLIENT_ERROR`, and Retries become false; the "line 127" reference is wrong (the `--ticket` paragraph is at 51–53).
- 🟡 **Documentation + Compatibility**: Pacing/queue docs describe the single-lock design the plan replaces
  **Location**: Phase 3, §7; Migration Notes
  research.md:298–360 describes one exclusive lock and promises older versions still pace through `arxiv.lock`; only paths are slated to change.
- 🟡 **Documentation**: research-topic recovery rows still say only arXiv fetches can queue or contend
  **Location**: Phase 7
  `research-topic/SKILL.md:619–620` will misdirect a user diagnosing a Crossref `lock_contention` or abandoned `waiting`.

#### Minor

- 🔵 **Code Quality**: File gate branches on `SpacedFrom` — **Location**: Phase 3, §4. Move the reservation rule into a domain `PacingState::reserve` so the adapter runs one unbranched sequence.
- 🔵 **Code Quality**: Undecodable body loses its `DecodeFailure` — **Location**: Phase 4, §2. Keep `upstream_error` output but emit a diagnostic so schema drift is distinguishable from an outage.
- 🔵 **Code Quality**: `TicketRejection` mirrors `Provenance` — **Location**: Phase 3, §2. Use `TicketRejection::Foreign { ticket, provenance }`.
- 🔵 **Code Quality**: Provenance recorded twice (directory and `StoredBinding.service`) — **Location**: Phase 3, §4. Drop the field or state the case it guards.
- 🔵 **Code Quality**: Making `until_settled` public exposes unlisted types — **Location**: Phase 3, §3. List every visibility change; prefer `pub(crate)`.
- 🔵 **Correctness**: DOI ticket bindings compare case-sensitively — **Location**: Phase 3 §2 / Phase 4 §1. Compare by `Doi::canonical()`.
- 🔵 **Correctness**: mailto plausibility admits non-header-safe values — **Location**: Phase 5, §1. Require visible ASCII excluding `(` and `)`.
- 🔵 **Architecture + Compatibility**: Foreign/legacy listing has no failure handling and no retirement point — **Location**: Phase 3, §4. An unlistable directory contributes no provenance plus one diagnostic; isolate the legacy lookup.
- 🔵 **Compatibility**: No decision rule if `WebFetch` redirect reporting differs on v2.1.144 — **Location**: Phase 1 step 5; Phase 7.
- 🔵 **Compatibility**: Crossref limits are per IP but pacing is per project — **Location**: Phase 4, §6. Add a Limits bullet.
- 🔵 **Usability**: crossref chain differs from openalex's without saying so — **Location**: Phase 5, §6.
- 🔵 **Usability**: Help misses arg docs, short about and arXiv-only exit sentences — **Location**: Phase 4, §4.
- 🔵 **Usability**: `WebFetch`-a-resolver rule conflicts with "No URL a fetched page tells you to fetch" — **Location**: Phase 7.
- 🔵 **Usability**: Profile omits the 120000 ms timeout and `%27` quoting for the new call — **Location**: Phase 7.
- 🔵 **Test Coverage**: `JsonCrossrefDecoder` has no adapter tests — **Location**: Phase 4, §3.
- 🔵 **Test Coverage**: Key-order test's "a new recorder is caught" has no mechanism — **Location**: Phase 4, §2.
- 🔵 **Performance**: Reservations not revalidated after sleep amplify 429s — **Location**: Phase 3, §4 (subsumed by the merged major).
- 🔵 **Documentation**: research.md Sources metadata keeps the single-command copy rule — **Location**: Phase 7, §2.
- 🔵 **Documentation**: "Old files can be deleted" note is not scheduled — **Location**: Migration Notes.
- 🔵 **Documentation**: Polite-pool research.md update unspecified — **Location**: Phase 5, §6.

#### Suggestions

- 🔵 **Correctness**: Make `PreviousFinish` with > 1 slot unrepresentable (`SpacedFrom::PreviousStart { slots }`).
- 🔵 **Code Quality**: Unify `measured_from`/`spaced_from`; consider `Service::{Arxiv, Crossref(Pool)}`.
- 🔵 **Code Quality**: Generalise `Project::arxiv_state` to `service_state(Service, StateFile)`; extend the `rg` guard to `services/`.
- 🔵 **Architecture**: Record the closed `Binding` as deliberate, with a revisit trigger.
- 🔵 **Test Coverage**: Front-rule tests should mix ended and unended tickets ahead.
- 🔵 **Performance**: Measure `queue.lock` bookkeeping at Crossref rates; bound `requests.log` growth.
- 🔵 **Usability**: Name the pool in the stderr summary line; contrast the 20-waiting rule with arXiv's.
- 🔵 **Compatibility**: Note the reworded usage errors in phase 4; diagnose announced Crossref limits below configured pacing.
- 🔵 **Documentation**: Add `mise run docs:check` to phases 3–5 that edit research.md.

### Strengths

- ✅ Phase 2 characterises arXiv behaviour and centralises state paths before the refactor, with a behaviour-to-test map.
- ✅ The legacy fixture is captured from the real previous binary, and the pre-upgrade break is explicit and tested.
- ✅ Pure pacing arithmetic moves into the domain crate as `Pacing { interval, spaced_from, slots }`, reaching admission and retries without changing their arithmetic.
- ✅ `is_served_given` with one slot reduces exactly to today's rule; own tickets always beat foreign ones; provenance listing is read-only.
- ✅ `classify::crossref` maps every status class explicitly, making 429 the only throttling verdict so `Attempts::deferral` needs no change.
- ✅ The `doi` family reuses arXiv's status, exit-code and `--ticket` contract; mismatch messages are actionable.
- ✅ Key ordering (doi keys after web keys) makes the combined block canonical by construction.
- ✅ `ACCELERATOR_CROSSREF_API_URL` joins the release guard; the address goes only in the `User-Agent`.
- ✅ The JSON decoder stays in `research-adapters`, respecting the pup boundary.
- ✅ Every SKILL.md edit is followed by `docs:generate`, and the plan adds no code comments.

### Recommended Changes

1. **Re-validate pacing under `pacing.lock` immediately before each send** (addresses: reserved callers ignore `Retry-After`; spacing between reservations; reservations not revalidated)
   In Phase 3 §4, after the sleep take `pacing.lock` again, re-check `not_before` and `last_actual_send + interval`, re-reserve or return `WaitTooLong`, then stamp the actual send. State that the deadline check runs before any reservation is written. Put the rule in a domain `PacingState` method so the adapter does not branch on `SpacedFrom`, and restate the margin rationale.
2. **Bound `PreviousStart` against a future `last_sent`** (addresses: no ceiling)
   Extend `trusted_at` to drop a `last_sent` more than `slots × interval` ahead of now; add a domain test.
3. **Add deterministic adapter tests for the multi-slot gate in Phase 3** (addresses: no adapter tests; oracle)
   3 slots admit 3 and refuse a 4th; drop frees one; concurrent `paced` under a fake clock reserves distinct, spaced `send_at`; out-of-lock sleep is charged to the deadline; a polite deferral arriving mid-sleep is honoured. Map each old single-lock test to its successor.
4. **Strengthen and re-cost the real-time tests** (addresses: vacuous polite assertion; oracle; vacuous `Retry-After` test; cost)
   Measure from `hit_instants`; wait until all invocations have joined before measuring; use ≥ 21 polite calls or a minimum-gap assertion; add a polite-pool `Retry-After` variant and a B-joined precondition with an upper bound; cut concurrency call counts (3–4 public, 4+ polite, shorter delay); put them in a nextest `test-group` with `max-threads = 1`; restate the time budget.
5. **Name a single source for pacing values and directories** (addresses: pacing values have no single source)
   `Service::pacing()` and `Service::code()` only; rewrite phase 4/5 adapter bullets as wiring.
6. **Unify the queued outcome shape** (addresses: two outcome shapes)
   Choose either `Queued<ArxivAnswer>` for arXiv or a single engine-folded `Unavailable`, or record the divergence as a deliberate PP-888 follow-up.
7. **Derive the admission poll from pacing** (addresses: coarse `ADMISSION_POLL`)
   e.g. `min(100 ms, interval / 4)`; restate the 4.7/s figure as an upper bound.
8. **Rework `crossref.mailto` resolution and feedback** (addresses: config-crate placement; warning source; subagent-only reporting; impossible states; plausibility)
   Add an open-trust resolver to `config`; return an enum (`Polite`, `Anonymous`, `AnonymousBecause(ContactWarning)`); name the source in the warning and give the unreadable case its own text; reject non-visible-ASCII and parentheses; have the profile repeat a `warning:` once in its summary.
9. **Fill the test gaps for settled decisions** (addresses: untested decisions; decoder; key-order; front-rule)
   Undecodable 2xx, pool-change re-presentation, foreign-service record, unreadable config, `TimedOut`, `A/crossref_json.rs`, key-order derived from real producers, mixed ended/unended front-rule cases.
10. **Make the Phase 2 → 3 invariance claim accurate** (addresses: phase 2 unchanged claim)
    Scope "unchanged bodies" to black-box CLI tests; add `cli/research/tests` to the diff check with an allowed-renames list; move wait-rule cases verbatim.
11. **Widen the docs scope** (addresses: research.md arXiv-only wording; single-lock docs; research-topic rows; Sources metadata; deletion note; polite-pool docs; per-IP limits; docs:check)
    Enumerate research.md passages per phase, rewrite Pacing/Limits for the new design and the accepted overlap risk, generalise research-topic rows 619–620, schedule the deletion note and polite-pool subsection, add `docs:check` to phases 3–5.
12. **Tidy the smaller design points** (addresses: DOI case; foreign listing failures; `TicketRejection`; `StoredBinding.service`; visibility; WebFetch gap; help text; profile timeout/quoting/fetch boundary)
    Apply each minor finding's suggestion in place.

## Per-Lens Results

### Architecture

**Summary**: Structurally sound: a generic engine, pacing as a pure domain value, file locks in `research-adapters`, provenance by directory, all within the functional-core / imperative-shell split and pup boundaries. Weaknesses: reservation-based pacing loses cross-process `Retry-After` with multiple slots; the mailto ladder is built outside config; a few seams make the next service or cleanup harder.

**Strengths**:
- Pure pacing arithmetic moves to the domain crate, testable without I/O.
- `queued<T>` takes a `serve` closure, separating queue/admission/retry from request and decoding.
- Pacing reaches admission and retries via `PacingGate::spacing()` without arithmetic changes.
- N-slot front rule reduces to today's with one slot; phase 2 tests pin arXiv.
- Read-only provenance keeps service state isolated; own tickets win.
- Crossref decoder respects the serde-out-of-`research` pup rule.
- Accepted risks are named openly.

**Findings**:
- 🟡 major / medium — Phase 3 §4 — *Reserving a send slot then sleeping outside the lock skips a later Retry-After, and an aborted reservation pushes back other callers.* B and C reserve after A; A's 429 writes `not_before`; B and C still send. Deadline check order unstated. Suggest re-reading `not_before` before sending, checking the deadline before writing, and a polite multi-process deferral test.
- 🟡 major / high — Phase 5 §3 — *The crossref.mailto lookup is built in research-cli instead of the config crate.* The env var is declared on the catalogue `ExtraKey` and hard-coded again; config already has `Environment`, `environment_candidate` (consent.rs:1400) and per-level reads; `pup.ron:132-133` says resolution lives in config. Suggest `open_value(environment, config, &ExtraKey)` in config.
- 🔵 minor / high — Phase 3 §1 vs Phase 4 §3 / Phase 5 §4 — *Unclear whether pacing values and directory names belong to the domain or the adapters.* Three possible sources; slot count and lock-file count could disagree. Suggest `Service::pacing()` and `Service::code()` as sole sources.
- 🔵 minor / medium — Phase 3 §4; Migration Notes — *Legacy `arxiv-queue/` lookup is built into every service queue with no way to remove it.* Suggest provider list (`OtherFamilyQueues`, `PreviousReleaseQueue`) assembled in `main.rs`.
- 🔵 suggestion / medium — Phase 3 §2–3 — *Two places still change for every new service, and the arXiv outcome type is duplicated.* Record closed `Binding` as deliberate; note `Queued<ArxivAnswer>` as a PP-888 follow-up.

### Code Quality

**Summary**: Strong on maintainability: characterisation first, pacing as a domain value, pure arithmetic in the domain, rejection wording as a domain `Display`. Risks: two parallel outcome shapes, ambiguous ownership of pacing values, policy leaking into the file adapter, and types admitting nonsensical states or duplicating information.

**Strengths**:
- Phase 2 characterisation and path centralisation give a strong safety net.
- `Pacing { interval, spaced_from, slots }` concentrates per-service policy.
- `PacingState` arithmetic becomes pure and testable.
- `RejectedTicket` Display removes per-family string duplication.
- `is_served_given` keeps one home for the front rule.
- `CrossrefDecoder` port follows the hexagonal pattern pup enforces.

**Findings**:
- 🟡 major / high — Phase 3 §3; Phase 4 §2 — *Two parallel outcome shapes, and 'unavailable' represented at two levels.* Suggest one shape for both families.
- 🟡 major / medium — Phase 3 §1 vs Phase 4 §3 / Phase 5 §4 — *Unclear whether pacing values live in the domain or the adapters.* Suggest `Service::pacing()` as sole source.
- 🔵 minor / medium — Phase 3 §4 — *File gate branches on the spacing origin.* Suggest `PacingState::reserve(&Pacing, now) -> (SystemTime, PacingState)`.
- 🔵 minor / high — Phase 5 §3 — *Contact resolution returns a tuple that allows impossible states, and has a missing warning text.* Suggest `CrossrefContact { Polite, Anonymous, AnonymousBecause(ContactWarning) }`.
- 🔵 minor / medium — Phase 4 §2 — *An undecodable Crossref body is reported as upstream_error and the decode failure is lost.* Emit a diagnostic.
- 🔵 minor / medium — Phase 3 §2 — *TicketRejection copies the shape of Provenance.* Use `Foreign { ticket, provenance }`.
- 🔵 minor / medium — Phase 3 §4 — *Provenance recorded twice.* Drop `StoredBinding.service` or justify it.
- 🔵 minor / medium — Phase 3 §3 — *Making `until_settled` public also exposes types the plan does not mention.* List visibility changes; prefer `pub(crate)`.
- 🔵 suggestion / medium — Implementation Approach vs Phase 3 §1 — *Inconsistent names: measured_from vs spaced_from, and Service for what is a pool.*
- 🔵 suggestion / medium — Phase 2 §1; Phase 3 §5; Phase 4 §5 — *Test path helper is named for arXiv but must serve every service.* Generalise to `service_state(Service, StateFile)`.

### Test Coverage

**Summary**: Unusually test-led, with a characterisation phase, a captured legacy fixture and black-box Crossref tests. Weak where risk is highest: multi-slot start-spaced pacing is checked only by real-time tests with a self-written oracle, tight margins and one vacuous assertion; the "phase 2 unchanged" claim does not hold for cited domain/adapter tests; several settled decisions are untested.

**Strengths**:
- Behaviour-to-test mapping and characterisation before generalisation.
- Legacy fixture captured from the real binary.
- Each Crossref criterion gets its own test under `MockHTTPServer` and `LoggingClock`.
- Sensible pyramid of pure-domain unit tests.
- Parity test pins concrete values, not just agreement.
- Mixed-pool, cross-service and cross-family tests check independence directly.

**Findings**:
- 🟡 major / high — Phase 2 mapping; Phase 3 Success Criteria — *The 'phase 2 tests pass unchanged' check cannot hold for the domain and adapter tests it cites.*
- 🟡 major / high — Phase 5 §5 — *The polite-pool rate-limit assertion can never fail with only 10 calls.*
- 🟡 major / medium — Phase 4 §5; Phase 5 §5 — *Real-time rate tests use the gate's own log as the oracle and have flaky upper-bound margins.*
- 🟡 major / high — Phase 3 §4 — *The new multi-slot serving locks and split pacing.lock reservation have no deterministic adapter tests.*
- 🟡 major / medium — Phase 4 §5 — *The cross-process Retry-After test can pass without checking anything.*
- 🟡 major / high — Phase 4 §2, §5; Phase 5 §3, §5; Phase 3 §4 — *Several settled design decisions and error paths have no test.*
- 🔵 minor / high — Phase 4 §5; Performance Considerations — *Concurrency tests take far longer than the stated time budget.*
- 🔵 minor / medium — Phase 4 §3 — *The Crossref JSON decoder has no adapter-level tests.*
- 🔵 minor / medium — Phase 4 §2 — *The key-order test's claim to catch a new recorder has no mechanism behind it.*
- 🔵 suggestion / medium — Phase 3 §2 — *The slot front-rule tests miss the case with ended tickets mixed in.*

### Correctness

**Summary**: Existing invariants carry over well. The new start-measured multi-slot pacing has three flaws: reserved callers never re-read the shared deferral, `PreviousStart` drops today's per-wait ceilings, and spacing holds between reserved times, not actual sends. Smaller: case-sensitive DOI bindings and a plausibility check that admits non-header-safe values.

**Strengths**:
- `is_served_given` with one slot reduces exactly to today's rule, keeping the early exit.
- Foreign tickets are checked only when absent from own standings.
- Chained reservations cannot overlap; a killed process errs conservative.
- `classify::crossref` maps every status class explicitly.
- The undecodable-2xx case is mapped to `unavailable`, closing an exit-1 path.
- Arithmetic checks out: 9 × 210 = 1.89 s, 9 × 102 = 0.918 s; 5 × 210 = 1.05 s, 10 × 102 = 1.02 s.

**Findings**:
- 🟡 major / high — Phase 3 §4 — *Polite-pool callers that already hold a reservation ignore a Retry-After written while they sleep.*
- 🟡 major / high — Phase 3 §1 — *PreviousStart send_at has no ceiling, so a wall-clock step or a stale future reservation can wedge the pool.*
- 🟡 major / medium — Implementation Approach; Phase 3 §4 — *Reservations space the reserved times, not the actual sends, so an earlier late send compresses the next gap.*
- 🔵 minor / medium — Phase 3 §4 — *Order of the deadline check and the reservation write is unspecified, so phantom reservations are possible.*
- 🔵 minor / high — Phase 3 §2 / Phase 4 §1 — *DOI ticket bindings compare case-sensitively.*
- 🔵 minor / medium — Phase 5 §1 — *The mailto plausibility check accepts values that cannot go in a User-Agent header.*
- 🔵 suggestion / medium — Phase 3 §1 — *Pacing can represent PreviousFinish with more than one slot.*

### Compatibility

**Summary**: In good shape: CLI changes are additive, the new key follows `ExtraKey::overridden`, arXiv's output and `E_ARXIV_TICKET_*` text stay the same, and the one deliberate break is pinned by a legacy-fixture test. Gaps: docs promise cross-version pacing the plan removes; no decision path for a `WebFetch` gap on v2.1.144; per-project pacing against per-IP limits is unstated.

**Strengths**:
- Binary and skill text ship together per plugin version.
- arXiv contract held stable by construction.
- Pre-upgrade break explicit and tested.
- Old tickets only ever read.
- `public-api` checks in every phase that changes the surface.
- `ACCELERATOR_CROSSREF_API_URL` added to the release guard.
- Contact in `User-Agent`, with an `x-api-pool: polite` manual check.

**Findings**:
- 🔵 minor / high — Phase 3 §7; Migration Notes — *Docs promise shared arXiv pacing with older versions, which this plan removes.* Optional mitigation: 1-slot arXiv gate also takes the legacy `arxiv.lock` (lock only).
- 🔵 minor / medium — Phase 1 step 5; Phase 7 — *No decision path if WebFetch's redirect reporting differs on the minimum supported Claude Code.*
- 🔵 minor / medium — Phase 4 §6 — *Crossref limits are per IP, but pacing is coordinated only within one project.*
- 🔵 minor / medium — Phase 3 §4 — *Failure handling for the foreign/legacy directory listing is unspecified, and the legacy read has no retirement point.*
- 🔵 suggestion / medium — Phase 4 §1 — *Existing usage-error messages change for the openalex, arxiv and web families.*
- 🔵 suggestion / low — Implementation Approach — *Hard-coded Crossref pool limits won't follow Crossref's announced changes.*

### Usability

**Summary**: Works out of the box: anonymous by default, one opt-in key, and the arXiv contract reused. Friction is in feedback: the implausible-mailto warning omits its source and reaches only a subagent, the chain differs from openalex's without saying so, and help/profile text leave surfaces unaligned.

**Strengths**:
- Sensible default; `crossref.mailto` never makes a call fail.
- `doi` reuses arXiv's statuses, exit codes and `--ticket`.
- Mismatch messages say why and what to do.
- `E_DOI_MALFORMED` shows accepted forms and echoes input.
- Key ordering makes the combined block correct by construction.
- `/configure` section mirrors `openalex`.

**Findings**:
- 🟡 major / high — Phase 5 §3 — *The implausible-mailto warning does not name the source of the value, though resolution never falls through.*
- 🟡 major / medium — Phase 5 §3 / Phase 7 — *A misconfigured mailto is only reported to the researcher subagent.*
- 🔵 minor / high — Phase 5 §6 — *The crossref resolution chain differs from openalex's in ways the docs should spell out.*
- 🔵 minor / high — Phase 4 §4 — *Help changes miss the arg docs, the short about and the outdated exit-code sentences.*
- 🔵 minor / medium — Phase 7 — *The WebFetch-a-resolver rule conflicts with the profile's 'No URL a fetched page tells you to fetch' boundary.*
- 🔵 minor / high — Phase 7 — *The profile text omits the Bash timeout and argument-quoting guidance for the new call.*
- 🔵 suggestion / medium — Phase 4 render / Phase 5 — *No way to see from the tool which Crossref pool a call used.*
- 🔵 suggestion / low — Phase 7 Waiting — *The 20-waiting handling differs from the arXiv profile's for no stated reason.*

### Performance

**Summary**: Efficient in shape: short-held `pacing.lock`, per-pool state, early-exit front rule, provenance off the hot path. Weak in timing: margins do not absorb overshoot because reservations chain from reserved starts; the 100 ms admission poll eats liveness slack; real-time test cost is ~3x the estimate.

**Strengths**:
- Per-slot serving locks plus a short `pacing.lock` remove the coarse whole-call lock.
- Front rule stops after `slots` unended contenders.
- Foreign listing only on an unknown ticket, names only.
- Separate per-pool state avoids cross-pool contention.
- 100 s / 900 s criteria run under `LoggingClock`, adding no wall time.

**Findings**:
- 🟡 major / high — Performance Considerations; Phase 4 §5; Phase 5 §5 — *Real-time test cost is underestimated by roughly 3x.* Suggest a nextest `test-group` with `max-threads = 1` and shorter delays / fewer calls.
- 🟡 major / high — Implementation Approach; Phase 3 §4 — *Margins over 100/200 ms do not absorb sleep overshoot as claimed.* Anchor to actual sends; consider ~105 ms polite.
- 🟡 major / medium — Phase 3 §3; Performance Considerations — *Fixed 100 ms ADMISSION_POLL is coarse against 102/210 ms intervals.*
- 🔵 minor / medium — Phase 3 §4 — *Reservations are not revalidated after sleep, which amplifies 429s and wastes intervals.*
- 🔵 suggestion / medium — Phase 3 §4 — *Per-call file bookkeeping and queue.lock hold time are unmeasured at Crossref rates.*
- 🔵 suggestion / medium — Phase 3 §4 — *Production request log grows without bound.*

### Documentation

**Summary**: Takes documentation seriously: each phase names its docs, `docs:generate` follows every SKILL.md edit, the fixture gets a provenance README, no code comments. Weak in that research.md work is a few bullet labels while much arXiv-only and single-lock wording becomes false, and the research-topic recovery rows are missed.

**Strengths**:
- `docs:generate` after every SKILL.md edit.
- `FETCH_HELP` updated and tested.
- Concrete `/configure` crossref outline.
- Legacy fixture README.
- Phase 1 corrects work-item inaccuracies first.
- No explanatory code comments.

**Findings**:
- 🟡 major / high — Phase 4 §6 — *research.md's arXiv-only fetch wording goes beyond the four listed bullets.*
- 🟡 major / high — Phase 3 §7 — *Pacing/queue docs describe a single-lock design that Phase 3 replaces, but only the paths are slated to change.*
- 🟡 major / high — Phase 7 — *research-topic recovery rows still say only arXiv fetches can queue or hit lock contention.*
- 🔵 minor / high — Phase 7 §2 — *research.md's Sources metadata section keeps the single-command copy rule.*
- 🔵 minor / high — Migration Notes — *The promised 'old files can be deleted' note is not scheduled in any phase.*
- 🔵 minor / medium — Phase 5 §6 — *The polite-pool research.md update is unspecified, and nothing cross-links it to the new setting.*
- 🔵 suggestion / medium — Phases 3–5 Success Criteria — *research.md is edited in phases 3-5 but the docs build first runs in phase 7.*

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-22T00:11:42+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Architecture + Correctness + Performance**: Reserved callers ignore a `Retry-After` written while they sleep — Resolved
- 🟡 **Correctness + Performance**: Spacing is guaranteed between reservations, not actual sends — Still present (the re-check design is right, but `PacingState` cannot tell a reservation from an actual send; see new issues)
- 🟡 **Correctness**: `PreviousStart` `send_at` has no ceiling — Partially resolved (ceiling anchored at `now`, not the deferral; `last_finish` uncapped)
- 🟡 **Test Coverage**: Polite-pool rate assertion can never fail — Resolved
- 🟡 **Test Coverage**: Real-time rate tests use the gate's own log as oracle — Partially resolved (oracle fixed; upper-bound slack of ~9–12 ms per gap remains, now minor)
- 🟡 **Test Coverage**: Multi-slot gate has no deterministic adapter tests — Resolved (fake-clock mechanism unspecified; see new issues)
- 🟡 **Test Coverage**: Cross-process `Retry-After` test can pass vacuously — Resolved for the public pool; the polite variant reintroduces it (minor)
- 🟡 **Test Coverage**: Settled decisions and error paths untested — Resolved
- 🟡 **Test Coverage**: "Phase 2 tests pass unchanged" cannot hold — Partially resolved (allowed-diff list omits the `Queued<ArxivAnswer>` re-wrapping in `R/arxiv_fetch.rs` and the fake's `spacing` → `pacing`)
- 🟡 **Performance + Test Coverage**: Real-time test cost underestimated — Resolved
- 🟡 **Performance**: Fixed 100 ms `ADMISSION_POLL` — Resolved
- 🟡 **Code Quality + Architecture**: Pacing values have no single source — Resolved
- 🟡 **Code Quality + Architecture**: Two outcome shapes — Partially resolved (`Unavailable` still exists in `Queued` and in each answer; now minor)
- 🟡 **Architecture**: `crossref.mailto` resolution built in research-cli — Resolved
- 🟡 **Usability + Code Quality**: Warning does not name its source — Resolved
- 🟡 **Usability**: Misconfigured mailto reported only to the subagent — Partially resolved (conduct never surfaces a successful researcher's summary)
- 🟡 **Documentation**: research.md arXiv-only wording — Partially resolved (four passages still missed, including the redirect rule; now minor)
- 🟡 **Documentation + Compatibility**: Pacing docs describe the single-lock design — Resolved
- 🟡 **Documentation**: research-topic recovery rows arXiv-only — Resolved
- 🔵 All other minor findings and suggestions — Resolved, except: pool name in stderr (Still present, suggestion), hard-coded Crossref limits (Still present, suggestion), `requests.log` growth (accepted).

### New Issues Introduced

- 🟡 **Correctness + Performance + Code Quality**: Reservations and actual sends share `last_sent` — read literally, a lone caller re-reserves behind its own reservation until its deadline, and concurrent wakers leapfrog each other; `reserve` has no input carrying the caller's own reservation. Split the state into `last_sent` (actual) and a `next_free` reservation cursor, give the re-check an explicit `Reservation` input, and test that one caller who wakes on time sends.
- 🟡 **Correctness**: `trusted_at`'s `last_sent` ceiling drops legitimate post-deferral reservations, and the re-check loop makes a backward clock step block arXiv for the step's length via `last_finish`.
- 🟡 **Architecture + Code Quality**: `CrossrefContact`/`ContactWarning` in the `research` domain carry `config`'s `SettingSource` and `Level`, which pup forbids research to import.
- 🔵 **Correctness**: A re-reservation refused by the deadline after waking leaves the first reservation behind.
- 🔵 **Correctness**: Admission poll values are 52.5 ms and 25.5 ms under `Duration` division, not 52 and 25.
- 🔵 **Correctness + Test Coverage**: Outcomes of the legacy `arxiv.lock` in `try_serve` (busy, unopenable) are unspecified and untested.
- 🔵 **Code Quality + Architecture**: The legacy `arxiv.lock` is an arXiv-only branch inside the service-neutral gate; inject it from `main.rs`.
- 🔵 **Architecture**: OpenAlex's `NoPacing` has no `Pacing` value, and a zero interval gives a zero admission poll.
- 🔵 **Architecture**: `admitted` in `fetch.rs` and `queued` in `serving.rs` depend on each other.
- 🔵 **Test Coverage**: Nothing tests that `admitted` uses `admission_poll()`.
- 🔵 **Test Coverage**: The fake clock for the concurrent and mid-sleep adapter tests is unspecified; the existing `RecordingClock` is single-threaded.
- 🔵 **Test Coverage**: The `trusted_at` ceiling is tested only far from its boundary.
- 🔵 **Performance**: `max-threads = 1` does not isolate the real-time tests from CPU load; use `threads-required`.
- 🔵 **Compatibility**: Byte-identical arXiv stderr is claimed, but phase 2 tests match only with `contains`/`starts_with`.
- 🔵 **Compatibility**: The work item still says the releases "never read each other's state".
- 🔵 **Compatibility**: The plausibility rule admits `\`, a quoted-pair escape in a `User-Agent` comment.
- 🔵 **Usability + Documentation**: The new research-topic warning row overlaps the existing generic "a warning" row.
- 🔵 **Usability**: The resolver-fetch boundary sentence leaves unclear whether the landing page should be fetched.
- 🔵 **Documentation**: Some research.md line ranges are off (`:58` vs `:95`, `:294-360` vs `:294-365`, `:142` vs `:148-150`).
- 🔵 Suggestions: `open_value` accepts non-open keys; build gate, queue and contention log from one factory; warning text gains a remedy clause; public-API criterion should list removals and renames.

### Assessment

The revision resolved 13 of the 19 major findings outright, and the design decisions all landed. Five majors remain. Three come from the new reserve-and-recheck pacing, whose state model cannot yet express what the plan asks of it. One is a crate-boundary slip in the contact types, and one is the allowed-diff list in phase 3. None is a critical, but the pacing state model must be fixed before implementation, because as written the polite pool either livelocks or loses its spacing guarantee.

## Re-Review (Pass 3) — 2026-11-22T00:43:24+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness + Performance + Code Quality**: Reservations and actual sends share `last_sent` — Resolved (`last_sent`/`next_free` split, explicit `Reservation`; correctness found no livelock, lost deferral or `PreviousFinish` regression)
- 🟡 **Correctness**: `trusted_at` ceiling anchoring and uncapped `last_finish` — Resolved (but see the new arXiv characterisation conflict)
- 🟡 **Architecture + Code Quality**: `ContactWarning` carries config types into research — Resolved
- 🟡 **Usability**: Misconfigured mailto reported only to the subagent — Resolved
- 🟡 **Test Coverage**: "Phase 2 tests pass unchanged" cannot hold — Partially resolved (more test-body changes outside the allowed list)
- 🔵 **Correctness**: phantom reservation on re-check refusal — Resolved in design (precision flaw below)
- 🔵 **Correctness**: admission poll arithmetic — Resolved
- 🔵 **Correctness + Test Coverage**: legacy `arxiv.lock` outcomes — Partially resolved (degraded path and `main.rs` wiring untested; open-or-create unspecified)
- 🔵 **Code Quality + Architecture**: arXiv-only branch in the gate — Resolved (companion locks)
- 🔵 **Architecture**: `NoPacing` has no `Pacing` — Resolved (`UNPACED`)
- 🔵 **Architecture**: `fetch`/`serving` cycle — Partially resolved (type-level cycle remains)
- 🔵 **Test Coverage**: admission-poll wiring untested — Partially resolved (named test cannot reach `pub(crate)` `Attempts`)
- 🔵 **Test Coverage**: fake clock unspecified — Resolved (`HookedClock`)
- 🔵 **Test Coverage**: `trusted_at` boundary — Resolved
- 🔵 **Test Coverage**: polite `Retry-After` vacuous — Partially resolved (now self-contradictory)
- 🔵 **Performance**: `max-threads = 1` isolation — Resolved
- 🔵 **Compatibility**: byte-identical stderr, work-item sentence, `\` exclusion — Resolved
- 🔵 **Compatibility**: public-API list — Partially resolved (list disagrees with the code)
- 🔵 **Usability + Documentation**: warning row overlap; fetch-boundary sentence; line references — Resolved
- 🔵 **Documentation**: research.md arXiv-only wording — Partially resolved (The fetch queue and `queue_unusable` remedy)
- 🔵 Suggestions: `open_value` trust — Resolved; adapters factory — Resolved; remedy clause — Partially resolved; pool name in stderr and Crossref limit diagnostics — Still present (deliberately skipped)

### New Issues Introduced

- 🟡 **Architecture**: The public `Settled<T>` serve contract carries `Empty` and `Failed`, which `queued<T>` cannot fold into `Queued<T>`. Make `serve` return `Result<Result<T, Unavailable>, OutOfBudget>` and keep `Settled` `pub(crate)`.
- 🟡 **Test Coverage**: Phase 3's start-spaced and three-slot tests have no `Service` or constructor to build them from; only `Service::Arxiv` exists until phase 4.
- 🟡 **Test Coverage + Compatibility**: Making `Verdict` and `Attempts` `pub(crate)` breaks `R/classify.rs` and blocks the `R/serving.rs` wiring test.
- 🟡 **Test Coverage**: Dropping future stamps in `trusted_at` contradicts the arXiv characterisation assertions that pin 3 s and 3.1 s waits after a clock step.
- 🔵 **Correctness**: `released` compares a nanosecond `send_at` with a millisecond `next_free`, so the hand-back rarely fires.
- 🔵 **Correctness**: `confirm`'s three branches overlap and leave a gap; precedence is unspecified.
- 🔵 **Correctness**: "No window can be compressed" holds for stamps, not arrivals; connection setup jitter is an accepted risk to state.
- 🔵 **Code Quality**: `Deferred` carries no released state, so hand-back policy and a jump-back loop sit in the adapter.
- 🔵 **Code Quality + Architecture**: `SettingSource`, `SettingValue` and `UnreadableLevel` are undefined and overlap `Source`, `TokenSource` and `ConfigError`.
- 🔵 **Architecture + Code Quality**: The factory leaves derivable `OtherFamilyQueues` and split previous-release inputs to `main.rs`.
- 🔵 **Compatibility**: Open semantics of the companion `arxiv.lock` are unspecified and conflict with the phase 3 manual check and the deletion note.
- 🔵 **Performance**: Exclusive running for every real-time test adds idle wall time; overshoot is carried forward; process start-up falls inside measured windows.
- 🔵 **Usability**: Where collected warnings print in conduct's summary; "every time" in Metadata step 2 conflicts with fetch reuse.
- 🔵 **Documentation**: Crossref pacing docs omit start-to-start spacing.
- 🔵 Suggestions: `DoiAnswer` has one variant; work-item plausibility sentence should be rewritten in place; `trusted_at` bound can be exceeded by a non-latest hole; warnings as a structured channel.

### Assessment

The pacing model is now sound: correctness found no livelock, lost deferral, double send or deadlock, and every pass-2 major is resolved. Five majors remain, all at the interface and test-scaffolding level: the `serve` contract, phase 3 test constructibility, two visibility choices, and the arXiv clock-step behaviour. Each has a mechanical fix, and none requires revisiting the design.

## Re-Review (Pass 4) — 2026-11-22T01:08:05+00:00

**Verdict:** REVISE

Lenses re-run: correctness, architecture, test coverage. The other five lenses had only minor findings in pass 3, and the pass-4 edits did not touch their concerns.

### Previously Identified Issues

- 🟡 **Architecture**: Public `Settled<T>` serve contract — Resolved
- 🟡 **Test Coverage**: Phase 3 tests had nothing to build from — Resolved (all three `Service` variants in phase 3)
- 🟡 **Test Coverage + Compatibility**: `Verdict`/`Attempts` visibility — Resolved
- 🟡 **Test Coverage**: `trusted_at` contradicting arXiv's 3 s / 3.1 s assertions — Resolved (clamp to `now`)
- 🟡 **Test Coverage**: "Phase 2 tests pass unchanged" cannot hold — Partially resolved (`join(…, SPACING)` call sites and the failed-write diagnostic count remain)
- 🔵 **Correctness**: `release` precision, `confirm` precedence, stamps vs arrivals, non-latest hole — Resolved
- 🔵 **Correctness + Test Coverage**: legacy `arxiv.lock` outcomes — Resolved
- 🔵 **Architecture**: `fetch`/`serving` cycle; undefined setting types; factory inputs — Resolved
- 🔵 **Test Coverage**: admission-poll wiring; polite `Retry-After` vacuity — Resolved (a launch race replaces the vacuity; see below)

### New Issues Introduced

- 🟡 **Architecture**: Phase 3's `Service::Crossref` needs `Family::Doi`, which phase 4 introduces.
- 🟡 **Test Coverage**: A single barrier ticket cannot hold back the three-slot polite pool; the barrier must be one held ticket per slot.
- 🔵 **Correctness**: Clamping both future stamps turns the 3.1 s unfinished-send wait into 3 s when the finish precedes the send.
- 🔵 **Correctness**: `now` must be read after taking `pacing.lock`, or a stale `now` can let two sends fall inside one interval.
- 🔵 **Architecture**: `queued` and `QueuePorts` are public but take the crate-private `Attempts`.
- 🔵 **Architecture**: `OpenKey`'s credential refusal relies on a pairing the config catalogue does not record.
- 🔵 **Architecture**: `fetch_doi` writes through a diagnostics port the research domain does not have.
- 🔵 **Test Coverage**: The polite `Retry-After` test races A's deferral write against B's and C's launch.
- 🔵 **Test Coverage**: The new legacy-path tests trip the "no state path in test bodies" `rg` check.
- 🔵 **Test Coverage**: No test pins how the gate reads a pacing file of another schema version or missing `next_free_ms`.
- 🔵 Suggestions: arXiv's `serve` mapping should name the in-turn withdrawal confirmations; enforce the one-way module rule; restrict `PreviousRelease` to arXiv; drive the poll wiring test through the public path.

### Assessment

The pacing model and the engine contract are settled: correctness confirms the reserve/confirm model holds through millisecond truncation, the `trust` clamps and the new `serve` contract. Three majors remain, exactly at the revise threshold, and all three are local: a phase-ordering slip for `Family::Doi`, the barrier's ticket count, and two missing entries in the allowed-diff list. The findings are converging on test scaffolding and phase sequencing rather than design.

## Re-Review (Pass 5) — 2026-11-22T01:22:39+00:00

**Verdict:** COMMENT

Lens re-run: test coverage only, the lens that raised most pass-4 findings.

### Previously Identified Issues

- 🟡 **Test Coverage**: "Phase 2 tests pass unchanged" cannot hold — Resolved (every §1a call site verified against the code; the failed-write count stays at 2)
- 🟡 **Test Coverage**: A single barrier ticket cannot hold back the polite pool — Resolved (`hold_barrier` holds one ticket per slot)
- 🟡 **Architecture**: `Family::Doi` needed in phase 3 — Resolved (no CLI test touched; `Family::ALL` and `parse` unchanged)
- 🔵 **Test Coverage**: polite `Retry-After` launch race; legacy-path `rg` trips; schema-version test; poll wiring via public path — Resolved
- 🔵 Correctness and architecture minors from pass 4 — applied in the plan; not re-reviewed in this pass

### New Issues Introduced

- 🟡 **Test Coverage**: Skipping unchanged reserve writes, with `trust` re-clamping on every read, makes the kept clock-step gate tests livelock.
- 🔵 **Test Coverage**: The `arxiv-queue-stress` ignore attribute and scratch-only names trip both `rg` checks.
- 🔵 **Test Coverage**: A's 429 route is not scripted to answer once, so A's retry can re-defer B and C.
- 🔵 **Test Coverage**: The schema-version test claims a diagnostic "as a corrupt file does today", but none is written today.
- 🔵 **Test Coverage**: The stale-`now` test has no stated way to know the caller is blocked on `pacing.lock`.
- 🔵 **Test Coverage**: No failing test drives `TokenKeys::declared` refusing an unmarked plaintext key.
- 🔵 Suggestion: copy, rather than move, the `A/pacing.rs` wait-rule cases.

### Fixes Applied After This Pass

All seven were applied to the plan without a further review pass:
- `reserve_send` writes whenever the result differs from the state read from disk, so a `trust` clamp is persisted at once and `confirm_send` never re-clamps.
- Both `rg` checks match only quoted path literals; scratch-only names are renamed in phase 2.
- Both `Retry-After` tests script A's route as `Route::Sequence([429, 200])` and bound A's retry.
- The schema-version diagnostic is stated as new behaviour, one per `paced` call, on `Crossref(Public)`; an unreadable file still reports nothing.
- The stale-`now` test holds `pacing.lock` itself and uses a `HookedClock` read hook with a 200 ms signal window.
- A red test in `cli/config/tests/credentials.rs` drives the `TokenKeys::declared` refusal; the catalogue test checks `*_cmd` siblings.
- The `A/pacing.rs` wait-rule cases stay as gate characterisation, with pure equivalents added as new tests.

### Assessment

One major remained, below the revise threshold, so the verdict is COMMENT. It and every minor are now fixed in the plan. Across five passes the design (generic queue engine, reserve/confirm pacing, provenance by directory, the open-setting ladder) has held, and the remaining risk sits in test scaffolding, which the implementer will meet under test-first development. The plan is ready to implement.

## Verdict Change — 2026-11-22T01:23:52+00:00

**Verdict:** APPROVE

The reviewer changed the verdict from COMMENT to APPROVE on the strength of the post-pass-5 fixes, without a further review pass, and marked the plan `ready`.
