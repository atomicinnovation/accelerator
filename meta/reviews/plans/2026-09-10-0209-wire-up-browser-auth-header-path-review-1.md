---
type: "plan-review"
id: "2026-09-10-0209-wire-up-browser-auth-header-path-review-1"
title: "Plan Review: Wire Up The Browser Auth-Header Path In Design Skills"
date: "2026-09-10T00:56:37+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-10-0209-wire-up-browser-auth-header-path"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["security", "correctness", "architecture", "test-coverage", "code-quality", "documentation", "standards"]
review_number: 1
review_pass: 4
tags: ["design", "security", "playwright", "auth"]
last_updated: "2026-09-10T08:45:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Wire Up The Browser Auth-Header Path In Design Skills

**Verdict:** REVISE

The plan is unusually security-literate and well-decomposed: it inverts the work
item's route-ordering error correctly (auth-header registers before the
classifier so `route.fallback()` reaches it under Playwright's
last-registered-first dispatch), keeps origin derivation daemon-side to avoid a
Rust/WHATWG parser mismatch, fails closed when the origin is unpinned, and pairs
a mutation-resistant strip test with a load-bearing ordering-safety check. Two
critical issues block it: the origin pin and env-frozen token are never reset
per invocation, so a warm daemon (10-minute reuse) governs later crawls by the
first — a credential-bleed and silently-unauthenticated-inventory vector; and
the first-write-wins pin immutability the plan itself calls "the security
mechanism, not tidiness" has no test. Redirect handling — the common auth flow —
is asserted safe but never exercised, and the documentation truth-up misses two
published surfaces.

### Cross-Cutting Themes

- **Origin-pin lifecycle** (flagged by: architecture, security, test-coverage,
  correctness) — the pin's whole correctness rests on its lifecycle, and the
  plan gets both ends wrong. It lives too long (never reset between crawls on a
  warm daemon) and its within-crawl immutability — the guarantee a followed link
  cannot move it off-site — is untested. Every other per-request daemon state
  (`currentAllowances`, `lastRefusal`) is explicitly reset at `daemon.js:226`;
  the pin diverges from that established pattern.
- **Redirect from the pinned origin** (flagged by: correctness, security,
  test-coverage, documentation) — pinning the pre-redirect origin has two
  opposite failure modes on a redirecting `[location]` (http→https, apex→www,
  SSO bounce): the gated page after the redirect is stripped of the header (401,
  incomplete inventory), and the bearer may leak onto the redirect target if
  Playwright reuses `continue`-supplied headers across the hop. Neither is
  tested; the redirect caveat is undocumented. AC1's single non-redirecting
  fixture never exercises this.
- **Test-vs-mechanism gap** (flagged by: test-coverage, security) — the plan's
  named security guarantees (pin immutability, redirect stability, the
  executor→daemon `location_url` contract) are each argued in prose but not
  bound by a test, so a mutation to any of them survives the suite.
- **Incomplete documentation truth-up** (flagged by: documentation) — two
  published surfaces beyond the three named (`docs-site/.../design.md` and the
  generated mirror) still warn users off the capability, and AC6's grep is too
  narrow to catch the phrasings that remain.

### Tradeoff Analysis

- **Threaded `location_url` (executor seam) vs pinning directly from
  `req.url`**: Architecture argues Phase 2's field duplicates `req.url` with no
  distinct invariant and could be dropped (Open Question 2's first option).
  Security values the executor as the authoritative, tamper-guarded declarer of
  "the location", which the guard against a page-derived pre-set enforces. The
  duplication is defensible only if the executor is deliberately positioned as
  the location authority for anticipated dual-origin work — otherwise pinning
  from `req.url` is simpler. Recommendation: keep the executor seam but state the
  rationale explicitly, or drop Phase 2 and pin from `req.url`.

### Findings

#### Critical

- 🔴 **Architecture + Security**: Origin pin and env-frozen token persist across
  invocations on a warm daemon
  **Location**: Implementation Approach / Phase 3 Section 1 / Migration Notes
  `resolvedLocationOrigin` is a closure `let` set first-write-wins and never
  reset, and the bearer is read once from the daemon's frozen `process.env` at
  launch. A warm daemon is reused for ~10 minutes, so the first navigate that
  sets the pin may belong to a prior invocation, and the token is that prior
  invocation's value. A later crawl to the pinned origin silently transmits the
  earlier bearer (credential bleed); a later crawl to a different origin is
  stripped to an unauthenticated inventory — the exact defect this item fixes.
  The "first navigate is never page-derived" argument holds only for a
  freshly-spawned daemon. Reset the pin where `currentAllowances` is reset
  (`daemon.js:226`) and thread the header per request, or scope pin+token to the
  invocation.

- 🔴 **Test Coverage**: First-write-wins pin immutability — the named security
  mechanism — has no test
  **Location**: Implementation Approach / Phase 3 Section 3
  None of the four runtime cases issue a *second top-level navigate* to a
  different origin and assert the header stays keyed to the first. AC2/AC3 test a
  cross-origin subresource within one page (pin unchanged); the ordering-safety
  case tests the classifier. Dropping the `resolvedLocationOrigin === null`
  guard so every navigate re-pins would let every listed test still pass. Add a
  case: navigate A (pin), navigate B, request A- and B-origin resources, assert
  the header attaches only on A.

#### Major

- 🟡 **Correctness + Security + Test Coverage + Documentation**: Redirect from
  the pinned origin is unhandled, untested, and undocumented
  **Location**: Phase 3 Section 1 (redirect note) and Section 3
  Pinning the pre-redirect origin strips the header on the redirected gated page
  (401, incomplete inventory) and may leak the bearer onto a cross-origin
  redirect target if `continue`-supplied headers survive the hop — a known
  Playwright footgun. The suite covers a cross-origin subresource, not a
  cross-origin *navigation redirect*, which is a different interception path. The
  SKILL prose never warns that a redirecting `[location]` disables the header.
  Add a 302→distinct-loopback-origin fixture asserting no bearer reaches the
  target, decide re-pin-vs-hold behaviour, and document the redirect caveat.

- 🟡 **Correctness**: Reworked handler drops the fail-closed `try/catch` its
  sibling classifier relies on
  **Location**: Phase 1 Section 1 / Phase 3 Section 1
  The classifier route wraps its body so a throw aborts as `malformed` rather
  than "leaving the request unhandled to hang until the wall-clock"
  (`daemon.js:194-213`). The reworked auth-header handler has no such wrapper,
  yet `allHeaders()`/`continue()` can reject (request already handled, frame
  detached, mid-request navigation) once it runs on every request via
  `fallback()`. An unhandled rejection hangs the navigation to `WALL_CLOCK_MS`.
  Mirror the classifier's fail-closed pattern.

- 🟡 **Test Coverage**: Executor→daemon `location_url` contract tested only in
  disconnected halves
  **Location**: Phase 2 / Phase 3 Section 2
  Phase 2 unit-tests the executor injects `location_url`; Phase 3 runtime cases
  set it themselves, "playing the executor's role". No test drives the real
  executor into the real daemon, so a field-name or shape drift between what the
  executor writes and what the daemon reads (`req.location_url`) is caught by
  nothing — silently reproducing the unauthenticated-inventory defect. Assert
  the field name from both sides (shared constant) or exercise the real merge
  output against the daemon.

- 🟡 **Test Coverage**: AC2 cross-origin strip may pass vacuously
  **Location**: Phase 3 Section 3 (AC2)
  A browser does not propagate an `Authorization` header onto cross-origin
  subresource requests on its own, so origin B sees no bearer even if the strip
  branch is deleted entirely — only the branch-*swap* mutant is caught (via
  injection), not strip-*removal*. Have the origin-A fixture page issue its
  request to B with the bearer explicitly set, so the strip has a real header to
  remove.

- 🟡 **Documentation**: Truth-up misses two published documentation surfaces
  **Location**: Phase 3 Section 4
  Verified against the tree: `docs-site/src/content/docs/design.md:126-129`
  carries the same inert-path caution ("never calls it… Do not put a live
  credential"), and the generated mirror
  `docs-site/src/content/docs/reference/skills/design/inventory-design.md`
  (lines 92-99) repeats the WARNING and "once wired up" prose. Neither is in the
  plan's changes or AC6's grep scope. After wire-up the public docs site would
  actively warn users off a working, security-relevant capability. Add
  `design.md` to the truth-up and confirm the mirror is regenerated.

- 🟡 **Documentation**: AC6 grep too narrow to prove the warnings are gone
  **Location**: Phase 3 Success Criteria (AC6)
  Verified: the pattern misses "never calls it" (surfaces say this, not "never
  called"), "once wired up" (SKILL.md:98), "not enforced by anything"
  (SKILL.md:95), and is case-sensitive against "Do not put"/"Do not place". A
  partial rewrite that strips "inert" but leaves those would pass AC6 falsely.
  Add `-i` and the terms `wired up`, `not enforced`, `never call`.

- 🟡 **Architecture**: Threaded `location_url` duplicates `req.url` with no
  distinct invariant
  **Location**: Phase 2 Section 1 / Implementation Approach
  For `navigate` the executor copies `object.get("url")` verbatim into
  `location_url`, so the field is always byte-identical to `req.url` and the
  daemon's pin is indistinguishable from pinning `new URL(req.url).origin`. The
  security guarantee rests on first-write-wins, not on the field being separate.
  Either pin from `req.url` and drop Phase 2, or state the executor-as-location-
  authority rationale explicitly (see Tradeoff Analysis).

- 🟡 **Architecture**: Command-specific branching overloads a command-agnostic
  body merge
  **Location**: Phase 2 Section 1 (`merge_allowances` → `merge_request_context`)
  `merge_allowances` is uniform across commands today; threading `command` in and
  adding `if command == "navigate"` gives it a second reason to change, and the
  vaguer new name will attract every future per-command body tweak. Keep the
  allowance merge command-agnostic and place navigate-specific injection in a
  small navigate-scoped step reusing the pre-set-key guard, or note the cohesion
  tradeoff and constrain what the function may own.

#### Minor

- 🔵 **Code Quality**: Empty `catch {}` on the origin pin silently swallows a
  malformed `location_url`
  **Location**: Phase 3 Section 1 (navigate case)
  A malformed `location_url` leaves the origin unpinned; fail-closed stripping
  then silently yields an unauthenticated inventory with no log — the original
  defect, undiagnosable. Emit a structured warning rather than an empty catch.

- 🔵 **Correctness + Security**: Colon-split parse admits an empty header name or
  value
  **Location**: Phase 1 Section 1
  `":Bearer x"` yields an empty name and `"Authorization:"` an empty value after
  trim, slipping past the `colonIdx === -1` guard despite the "no-op on
  malformed" intent; an empty-named header may throw in `continue`. Return the
  no-op installer when the trimmed name is empty.

- 🔵 **Security + Correctness**: Pin poisoning on a refused or unparseable first
  navigate
  **Location**: Phase 3 Section 1
  The pin is set before the classifier runs and before `goto` settles, so a
  first navigate the classifier refuses (link-local, no `allow_internal`) still
  fixes the pin to an origin that never passed policy; first-write-wins then bars
  a later legitimate navigate from re-pinning. Pin only after the first navigate
  is admitted, or document the intent explicitly.

- 🔵 **Standards + Code Quality**: `merge_allowances` → `merge_request_context`
  rename ripple is unstated
  **Location**: Phase 2 Section 1
  The plan introduces the new name without noting the `run` call site
  (`executor.rs:113`), the five `merge_allowances` references in the
  `#[cfg(test)]` module, or the function's `///` doc comment (`:166-175`) that
  still describes only two-key allowance behaviour. A literal reading leaves a
  dangling name and a stale contract. Enumerate the call sites and update the
  doc comment in Phase 2.

- 🔵 **Security**: Inject branch uses original-case header name and no empty-name
  guard
  **Location**: Phase 1 Section 1
  The strip deletes `headerName.toLowerCase()` (correct against Playwright's
  lowercase `allHeaders()` keys), but the inject spreads the lowercase map and
  adds `[headerName]` in original case, risking a duplicate key rather than an
  override. Lowercase the injected key too.

- 🔵 **Test Coverage**: Header add/delete and casing covered only by the heavy
  integration lane
  **Location**: Phase 1 Section 1
  The header-map mutation lives in a closure over `route`, exercisable only in
  the real-Chromium lane; a casing bug in the strip takes minutes to surface.
  Extract a pure `applyHeader(headers, shouldAttach, name, value)` and unit-test
  it in the runtime-free lane alongside `shouldAttachHeader`.

- 🔵 **Security**: Live-header activation raises token-in-artefact exposure while
  the scrub stays literal-substring
  **Location**: Current State Analysis / What We're NOT Doing
  Wiring the path live sends the token to gated pages, where it can surface in
  captured snapshots/links/evaluate output in encoded forms the scrub does not
  derive (0207, deferred). Confirm and test that `scan` actually runs over
  daemon-captured content, and note the increased leakage surface.

- 🔵 **Code Quality**: `shouldAttachHeader`'s `expectedOrigin` is an origin but
  re-parsed as a URL
  **Location**: Phase 1 Section 1
  The daemon pins `new URL(...).origin`, yet the function re-wraps it as
  `new URL(expectedOrigin).origin`. Idempotent for http/https, but the parameter
  name and body disagree on contract. Compare origins directly or rename.

- 🔵 **Code Quality + Standards**: Sample `route.continue` line exceeds the
  80-column floor
  **Location**: Phase 1 Section 1
  `await route.continue({ headers: { ...headers, [headerName]: headerValue } });`
  at 8-space indent runs past 80. Extract the merged-headers object to a local so
  the sample matches what `mise run fix` will impose.

- 🔵 **Documentation**: No CHANGELOG entry for a newly-live capability
  **Location**: Migration Notes
  `CHANGELOG.md [Unreleased]` tracks the design command family; turning a
  documented-as-inert path into a working feature warrants an entry so the
  capability is discoverable. Add an `[Unreleased]` line.

- 🔵 **Standards + Test Coverage**: Red-green sequencing implied, not stated
  **Location**: Phases 1-3 "Changes Required" ordering
  Each phase lists the production change before its tests with no explicit
  failing-test-first instruction, against the repo's non-negotiable
  red-green-refactor. State per phase that the failing test is written and
  observed red first; note Phase 3's red step runs via
  `test:integration:design-automation`.

#### Suggestions

- 🔵 **Test Coverage**: Runtime missing-url guard on `navigate` is uncovered
  **Location**: Phase 3 Section 1
  The new `if (!req.url) return makeError({...missing-url})` has no runtime case
  asserting the daemon's error envelope. Add one if the guard is new here.

- 🔵 **Security**: Live token sits in the inherited daemon env and may reach logs
  **Location**: Manual Verification (bootstrap log)
  No `env_clear` (`process.rs:115-123`), so the bearer resides in the daemon's
  environ for its life, and the manual step inspects a header log. Add a
  defence-in-depth redaction check for the configured header in request logging.

- 🔵 **Code Quality**: Magic-string command comparison
  **Location**: Phase 2 Section 1
  `if command == "navigate"` compares to a literal; use an enum or shared
  constant if the codebase models commands as one — otherwise acceptable given
  commands are stringly-typed throughout.

- 🔵 **Code Quality**: Verify retained/corrected comments meet the low-tolerance
  policy
  **Location**: Phase 3 Section 1 (daemon.js comment) / Phase 1 factory
  Keep the corrected `fallback()` ordering comment only for the non-obvious
  last-registered-first semantics, and ensure the reworked factory drops the
  code-restating comments (`// No-op when env vars not set`) rather than carrying
  them forward.

### Strengths

- ✅ Correctly inverts the work item's route-ordering error: the classifier ends
  its allow path with `route.fallback()` (`daemon.js:208`) and Playwright
  dispatches last-registered-first, so the auth-header route must register
  *before* the classifier — registering after would `continue()` first and
  silently bypass navigation policy.
- ✅ Pin-before-`goto` timing is correct: `resolvedLocationOrigin` is set before
  `page.goto` in the navigate case, so the getter already returns the pinned
  origin when the first navigation's own request hits the route.
- ✅ Origin derivation is kept daemon-side via `new URL().origin` — the same
  parser used for per-request origins — deliberately avoiding a Rust/WHATWG
  port-handling mismatch; the parity hazard is explicitly acknowledged.
- ✅ Fail-closed default (strip, never inject, when unpinned or the URL is
  unparseable) is a real improvement over the current handler, which continued
  without stripping on an unparseable URL.
- ✅ Phase decomposition is clean: Phases 1 and 2 are mutually independent and
  each stays inert-and-green, with wiring risk concentrated honestly in Phase 3,
  which also carries the documentation truth-up.
- ✅ The AC1 (same-origin inject) + AC2 (cross-origin strip) pairing is
  genuinely mutation-resistant against a branch swap, and AC1 asserts gated
  *content* via `evaluate` rather than mere navigation success.
- ✅ The ordering-safety case (refused link-local origin still returns
  `navigation-refused` with the auth route installed) directly guards the
  classifier-bypass regression.
- ✅ Verification commands match the documented mise task names exactly, and the
  case-floor bump reads executed counts from the runner's own TAP summary with
  zero-skip and bare-return backstops.
- ✅ All three named documentation surfaces' cited line ranges hold against the
  revision-hash-anchored tree; both SKILL prose blocks are reduced to
  single-origin so no login-URL promise survives within them.

### Recommended Changes

1. **Reset the origin pin per invocation and scope the token to the request**
   (addresses: warm-daemon persistence critical) — Reset `resolvedLocationOrigin`
   to `null` where `currentAllowances`/`lastRefusal` are reset (`daemon.js:226`),
   so first-write-wins holds *within* a crawl but not across crawls on a warm
   daemon. Thread the auth header from the executor per request (mirroring
   `location_url`) rather than relying on the daemon's frozen env, or key
   pin+token to the invocation. State in the plan how immutability-within-crawl
   is preserved while resetting between crawls.

2. **Add a pin-immutability runtime case** (addresses: pin immutability critical)
   — Navigate A (pin), navigate B, request an A-origin and a B-origin resource;
   assert the header attaches only on A. This kills the "drop the null guard so
   every navigate re-pins" mutant.

3. **Handle, test, and document the redirect case** (addresses: redirect major)
   — Add a 302→distinct-loopback-origin fixture that records request headers and
   asserts no bearer reaches the target; decide and implement re-pin-vs-hold on a
   main-frame redirect; add the redirect caveat to the SKILL/help prose.

4. **Wrap the handler body fail-closed** (addresses: try/catch major) — Mirror
   the classifier: wrap `allHeaders()`/`continue()` in try/catch and fall back to
   an unmodified `continue()`/`fallback()` on error.

5. **Bind the executor→daemon contract with a test** (addresses: disconnected-
   halves major) — Assert the injected field name from both sides via a shared
   constant, or run the real merge output against the daemon.

6. **Make AC2 exercise a real strip** (addresses: vacuous-AC2 major) — Have the
   origin-A fixture page set the bearer explicitly on its cross-origin request to
   B, so strip-removal becomes observable.

7. **Extend the documentation truth-up and AC6 grep** (addresses: both
   documentation majors) — Add `docs-site/src/content/docs/design.md` to Phase 3,
   confirm the generated mirror regenerates, and broaden AC6 to `-i` plus
   `wired up`, `not enforced`, `never call`. Add a CHANGELOG `[Unreleased]` entry.

8. **Resolve the `location_url` seam question and the merge cohesion**
   (addresses: both architecture majors) — Either drop Phase 2 and pin from
   `req.url`, or document the executor-as-location-authority rationale; keep the
   allowance merge command-agnostic or constrain `merge_request_context`.

9. **Tighten the handler parse and inject branch** (addresses: colon-split,
   inject-casing, empty-catch minors) — No-op on an empty trimmed header name;
   lowercase the injected key; log rather than swallow a malformed
   `location_url`.

10. **Document the rename ripple and reorder for red-green** (addresses: rename-
    ripple, red-green minors) — Enumerate the `merge_allowances` call sites and
    doc comment; state the failing-test-first sequence per phase.

## Per-Lens Results

### Security

**Summary**: The plan is unusually security-aware for a wire-up: first-write-wins
pinning, fail-closed stripping, the executor guard against a page-derived
`location_url`, daemon-side `URL.origin` parity, and a mutation-resistant strip
test with a dedicated ordering-safety check are all sound. The dominant residual
risk is that the "first navigate is never page-derived" argument only holds for a
freshly-spawned daemon — the pin and env-frozen token persist across invocations
on a warm (10-minute) daemon, creating a credential-bleed / stale-token vector.
Secondary gaps: the redirect leak path is asserted safe but never tested, and
pin-poisoning on a refused first navigate is unaddressed.

**Strengths**:
- First-write-wins pinning plus the executor guard is a genuine tamper control:
  a followed link cannot move the pin off-site.
- Fail-closed on unpinned/unparseable is a real improvement over the current
  handler, which continued without stripping on an unparseable URL.
- Route ordering correctly analysed; adds a load-bearing safety test.
- Daemon-side `new URL().origin` avoids a Rust/WHATWG parser-mismatch bypass.
- The cross-origin strip test is designed to be mutation-resistant.

**Findings**:
- 🟡 major (high): Origin pin and env-frozen token persist across invocations on
  a warm daemon — a later invocation crawling the pinned origin transmits the
  first invocation's bearer; a crawl to a different origin is silently stripped.
  Reset per invocation and thread the header per request.
- 🟡 major (medium): Redirect from the pinned origin is asserted safe but never
  tested — the suite covers a cross-origin subresource, not a cross-origin
  navigation redirect, a different interception path where `continue`-supplied
  headers may leak. Add a 302→distinct-origin recording fixture.
- 🔵 minor (medium): Pin set before classification/success allows pin poisoning
  by a refused first navigate. Pin only after a non-refused load.
- 🔵 minor (medium): Live activation raises token-in-artefact exposure while the
  scrub stays literal-substring; confirm `scan` runs over captured content.
- 🔵 minor (low): Inject branch uses original-case header name and no empty-name
  guard — risks a duplicate key and an empty-named header. Lowercase and guard.
- 🔵 suggestion (low): Live token sits in the inherited daemon env and may reach
  request logs; add header redaction to logging.

### Correctness

**Summary**: The core logic is sound — the last-registered-first dispatch
reasoning is correct, pin-before-`goto` makes the getter return the origin for
the first navigate's own requests, the fail-closed getter is correct, and
re-parsing an origin string via `new URL().origin` is idempotent for http/https.
Phases 1 and 2 are genuinely inert and independent. Two gaps stand out: the
reworked handler drops the classifier's fail-closed try/catch, and first-write-
wins pinning to the pre-redirect origin silently strips the header on cross-
origin login redirects — untested by the non-redirecting AC1 fixture.

**Strengths**:
- Correctly identifies and inverts the work item's route-ordering error.
- Pin-before-`goto` sequencing is correct.
- Fail-closed design is sound (`shouldAttachHeader` returns false for a null
  origin before any parse).
- The `URL.origin` parity argument holds and is well-reasoned.
- Phases 1 and 2 are genuinely inert and mutually independent.

**Findings**:
- 🟡 major (medium): Handler drops the fail-closed try/catch its sibling
  classifier relies on (`daemon.js:194-213`); an unhandled rejection hangs the
  navigation to `WALL_CLOCK_MS`. Wrap and fall back to unmodified `continue()`.
- 🟡 major (medium): Pinning the pre-redirect origin strips the header on cross-
  origin login redirects (401, incomplete inventory); AC1's single origin never
  exercises it. Re-pin the post-redirect main-frame origin or test/document the
  no-redirect constraint.
- 🔵 minor (medium): Colon-split parse admits an empty header name or value
  despite the no-op-on-malformed intent. Guard the empty trimmed name.
- 🔵 suggestion (low): A refused/unparseable first navigate leaves the pin wrong
  for the crawl. Pin once a first navigation is admitted, or clarify the intent.

### Architecture

**Summary**: Well-decomposed into independent, individually-green phases, with
one genuinely strong call: keeping origin computation on a single side of the
trust boundary. The dominant risk is the pin's lifecycle — first-write-wins over
the daemon's whole life rather than per crawl, contradicting the daemon's per-
invocation state-reset pattern and reintroducing the silent-incomplete-inventory
bug for a second crawl on a warm daemon. Secondary concerns: the threaded
`location_url` duplicates `req.url`, and overloading the generic body-merge with
navigate-only branching erodes cohesion.

**Strengths**:
- Origin derivation kept on one side of the trust boundary; parity hazard
  explicitly avoided.
- Getter-injection decouples the handler from the daemon closure and resolves
  the "origin unknown at install time" problem.
- Clean phase decomposition; wiring risk concentrated in Phase 3.
- Handler fails closed when unpinned.

**Findings**:
- 🔴 critical (high): Origin pin is first-write-wins over daemon lifetime, not
  per crawl, on a reused warm daemon — a second crawl hits the stale pin and
  silently produces an unauthenticated inventory. Reset at the crawl boundary as
  `currentAllowances` is (`daemon.js:224-227`).
- 🟡 major (medium): Threaded `location_url` duplicates `req.url`; the executor
  seam adds ceremony without a distinct guarantee. Pin from `req.url` and drop
  Phase 2, or state the executor-as-authority rationale.
- 🟡 major (medium): Command-specific branching overloads a previously command-
  agnostic body merge; the vaguer `merge_request_context` name will attract
  future per-command tweaks. Keep the merge command-agnostic.
- 🔵 minor (medium): Correctness depends on hidden ordering coupling between two
  independently-registered route handlers; frame the safety test as protecting
  the ordering invariant and keep the corrected comment stating why.

### Test Coverage

**Summary**: The plan is unusually test-literate: pure logic in the runtime-free
lanes, real-header manipulation in the Chromium lane, an AC1+AC2 pairing that
kills the branch-swap mutant, and AC1 asserting gated content. However, the load-
bearing security invariant the plan names — first-write-wins immutability — has
no runtime test, the executor→daemon contract is exercised only in disconnected
halves, and the cross-origin strip case risks passing vacuously.

**Strengths**:
- Correct lane placement (pure logic unit, real `page.route` integration).
- AC1+AC2 is mutation-resistant against a branch swap; AC1 asserts content.
- The ordering-safety case guards the classifier-bypass regression.
- Executor merge cases mirror the proven allowance-guard test shape.
- The design-automation floor reads executed counts with zero-skip and bare-
  return backstops.

**Findings**:
- 🔴 critical (high): First-write-wins pin immutability has no test — no case
  issues a second top-level navigate to a different origin. Add one.
- 🟡 major (high): Executor→daemon contract tested only in disconnected halves; a
  `location_url` field-name drift is caught by nothing. Assert from both sides.
- 🟡 major (medium): AC2 cross-origin strip may pass vacuously — a browser does
  not add `Authorization` cross-origin on its own, so strip-removal survives.
  Set the header explicitly on the A→B fixture request.
- 🔵 minor (medium): Redirect-following pin behaviour claimed but untested;
  existing `redirectTo` fixtures make a case cheap.
- 🔵 minor (medium): Header add/delete and casing covered only by the heavy
  integration lane; extract a pure `applyHeader` and unit-test it.
- 🔵 minor (low): Runtime missing-url guard on `navigate` is not covered.
- 🔵 suggestion (low): Red-green sequencing implied, not stated, across phases.

### Code Quality

**Summary**: The reworked handler is largely an improvement: it collapses the
origin-comparison duplication by delegating to `shouldAttachHeader`, introduces a
clean getter seam, and renames `merge_allowances` to reflect broadened
responsibility. The main concern is observability: the empty `catch {}` around
origin pinning silently swallows a malformed `location_url` and, with fail-closed
stripping, can silently recreate the exact defect this item fixes. Secondary: a
stale Rust doc comment, an origin-vs-URL ambiguity in `shouldAttachHeader`, and a
sample line over 80 columns.

**Strengths**:
- Delegating the route decision to `shouldAttachHeader` removes duplicated
  origin-comparison logic.
- Renaming `merge_allowances` → `merge_request_context` signals broadened intent.
- The `getExpectedOrigin` getter is a clean dependency-injection seam.
- The factory keeps a clear guard-clause shape and a simple fail-closed default.

**Findings**:
- 🟡 major (medium): Empty `catch {}` on the origin pin silently swallows a
  malformed `location_url`; combined with fail-closed stripping the original
  silent-failure recurs undiagnosed. Emit a structured warning.
- 🔵 minor (medium): The renamed merge's `///` doc comment will go stale (still
  describes two-key allowance behaviour). Update it in Phase 2.
- 🔵 minor (medium): `shouldAttachHeader`'s `expectedOrigin` is an origin but
  re-parsed as a URL; the signature and body disagree on contract.
- 🔵 minor (low): Magic-string command comparison (`command == "navigate"`);
  compare against an enum/constant if one exists.
- 🔵 suggestion (low): Sample `route.continue` line exceeds 80 columns; extract
  the merged-headers object to a local.
- 🔵 suggestion (low): Verify retained/corrected comments meet the low-tolerance
  comment policy; drop the code-restating ones.

### Documentation

**Summary**: The cited line ranges for all three named surfaces are accurate, and
both SKILL.md allowlist blocks are reduced to single-origin so no dangling login-
URL promise survives within those files. However, the truth-up scope is
incomplete: a fourth hand-authored published surface
(`docs-site/src/content/docs/design.md`) carries the same inert-path caution and
is neither rewritten nor covered by AC6's grep. The AC6 pattern is also too
narrow to prove the warnings are gone, and the new first-write-wins/redirect
behaviour is left undocumented for users.

**Strengths**:
- All cited line ranges for the three named surfaces still match the tree; the
  revision-hash anchors hold.
- Both SKILL.md prose blocks (`:98-104`, `:221-226`) are reduced to single-origin
  with no login-URL reference left dangling within them.
- AC4's tree-wide grep and the manual coherence re-read give the env-var removal
  and single-origin rewrite genuine verification.

**Findings**:
- 🟡 major (high): The truth-up rewrites only three surfaces;
  `docs-site/src/content/docs/design.md:125-131` carries the same stale caution
  and is outside AC6's scope — the highest-visibility surface left saying the
  opposite of the truth. Add it and extend AC6.
- 🟡 major (high): AC6's grep does not match "never calls it", "Do not
  put/place" (case-sensitive + verb mismatch), "once wired up", or "not enforced
  by anything"; a partial rewrite could pass it falsely. Add `-i` and more terms.
- 🟡 major (medium): First-write-wins/redirect behaviour is undocumented for
  users — a redirecting `[location]` yields a silently-incomplete inventory with
  nothing in the docs. Document the first-navigate-pins behaviour and caveat.
- 🔵 minor (medium): No CHANGELOG `[Unreleased]` entry for the newly-live
  capability, so it ships undiscoverable.
- 🔵 minor (low): The generated mirror
  `docs-site/.../reference/skills/design/inventory-design.md` still carries the
  WARNING and "once wired up" prose; confirm it is regenerated.

### Standards

**Summary**: Strongly aligned with the repo's conventions: verification commands
use the correct mise task names, it mirrors the existing allowance-key guard,
keeps each phase independently mergeable and green, and follows WHATWG same-origin
semantics. A few gaps: the executor function is silently renamed without
documenting the ripple to callers and tests, one JS sample line breaches the
80-column floor, and the per-phase change lists put production code ahead of
their tests rather than making red-green explicit.

**Strengths**:
- Every automated verification command matches the documented mise task names.
- Phase 2 extends the existing `merge_allowances` guard consistently, reusing the
  identical rejection message.
- The reworked factory drops the current descriptive comments while keeping the
  one justified "why" comment (Playwright ordering).
- Origin comparison uses `URL.origin`; the strip deletes the lowercased name.
- Phase independence and "green at every merge point" are explicit; the case
  floor is updated in lockstep.

**Findings**:
- 🔵 minor (high): `merge_allowances` → `merge_request_context` renamed without
  documenting the `run` call site and five `#[cfg(test)]` references that must
  move with it. Enumerate them.
- 🔵 minor (high): Sample JS `route.continue` line exceeds the 80-column floor.
  Wrap the header-spread in the sample.
- 🔵 minor (medium): Per-phase change lists put production code before tests, not
  the mandated red-green order. Reorder or add a red-green instruction per phase.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-10

**Verdict:** REVISE

The revision resolves both Pass 1 criticals and most majors, but a codebase
trace (executor stateless per command; daemon has no crawl boundary; same-origin
scoping is agent prose, not code) reshaped the design to per-request auth keying,
and that reshape introduced a fresh crop of majors — two of them genuine
regressions. The plan is closer but not yet approvable: the token now travels in
a subprocess argv (a security regression), a gated page's late same-origin
requests are stripped (a completeness regression flagged by three lenses), and
several of the new tests do not kill the mutants they claim.

### Previously Identified Issues

- 🔴 **Architecture + Security**: Warm-daemon state persistence — **Resolved.**
  Per-request keying (reset like `currentAllowances` at `daemon.js:226`) removes
  the cross-request pin; no state bleeds across a warm daemon.
- 🔴 **Test Coverage**: Pin immutability untested — **Partially resolved.** The
  pin is gone and a no-bleed case added, but the case navigates A-then-B, which a
  full-persistence mutant survives; it needs a same-origin second navigate (see
  new issues).
- 🟡 **Correctness/Security/Test/Doc**: Redirect unhandled/untested/undocumented
  — **Mostly resolved.** Pre-redirect origin held, caveat and leak-guard added;
  the leak-guard is a boundary assertion, not mutation-resistant, and the caveat
  omits the failure consequence (see new issues).
- 🟡 **Correctness**: Handler drops fail-closed try/catch — **Resolved, but
  introduced a new issue.** try/catch added; its catch is empty and silent (see
  new issues).
- 🟡 **Test Coverage**: Executor→daemon contract in halves — **Partially
  resolved.** A contract test was added but is tautological across the
  language boundary (see new issues).
- 🟡 **Test Coverage**: AC2 vacuous — **Partially resolved.** The fixture now
  sets the bearer explicitly, but a CORS preflight can still make it vacuous (see
  new issues).
- 🟡 **Documentation**: Two doc surfaces missed — **Resolved.** `design.md` and
  the generated mirror are added; the mirror's regeneration mechanism is
  misdescribed (see new issues).
- 🟡 **Documentation**: AC6 grep too narrow — **Mostly resolved.** Broadened with
  `-iE` and more terms; still misses "until … wired up" and login-URL-origin.
- 🟡 **Architecture**: `location_url` duplicates `req.url` — **Downgraded to
  minor.** The seam now also carries `auth_header`; `location_url` alone still
  duplicates `req.url`.
- 🟡 **Architecture**: merge overload — **Accepted.** Single merge kept with an
  updated doc comment; explicitly acknowledged tradeoff.
- 🔵 Empty catch on origin pin — **Resolved** (`originOf` warns). Colon-split
  empty name/value — **Resolved** (`parseAuthHeader`). Pin poisoning —
  **Obviated** (no pin). Rename ripple — **Resolved** (enumerated). Inject casing
  — **Resolved** (`applyHeader` lowercases both branches). shouldAttachHeader
  origin-vs-URL — **Resolved.** Header casing integration-only — **Resolved**
  (pure `applyHeader`). CHANGELOG — **Resolved.** Red-green — **Resolved for
  Phases 1–2, not Phase 3** (see new issues). 80-col — **Mostly** (one Rust array
  line still >80).

### New Issues Introduced

- 🟡 **Security** (high): 🔒 Live token routed through subprocess argv — the
  navigate body carries `auth_header` and is passed as a command-line argument to
  `node run.js`, visible via `/proc/<pid>/cmdline` and `ps`; strictly worse than
  the prior environment placement. Inject the header in the JS client from its
  inherited env into the loopback request body, keeping it out of argv.
- 🟡 **Security** (high): 🔒 Header keyed to any navigate target, bounded only by
  agent prose — the daemon attaches the token to whatever origin the executor is
  told to navigate to; only the browser-locator's same-origin prose rule stops a
  cross-origin hop. Combined with removing the "do not place a live credential"
  warning, a mis-followed or injected cross-origin link exfiltrates a live token.
  This is the residual accepted when choosing stateless keying; the lens
  recommends a code-enforced origin bound (the hard-boundary option) or keeping
  the credential caution.
- 🟡 **Correctness + Architecture + Test Coverage**: Late same-origin
  subresource stripped — resetting auth to null every request means a gated
  page's post-`domcontentloaded` XHRs firing during a following `snapshot`/`links`
  command are stripped and 401, silently re-opening the incomplete-inventory
  failure. Keep the auth state sticky (set on navigate, persist across the page's
  non-navigate commands, re-key on the next navigate), and add a case.
- 🟡 **Architecture + Test Coverage**: Token read via `std::env::var` inside
  `merge_request_context` — breaks the allowances-as-parameter pattern and forces
  racy process-global env mutation in Rust tests (`set_var` is unsafe in the 2024
  edition). Resolve the header in `run` and pass it as a parameter. (Moving
  injection to the JS client per the argv finding removes the Rust env read
  entirely.)
- 🟡 **Code Quality + Correctness**: Empty catch in the route body — swallows any
  throw with no signal, silently never attaching the header; bind the error and
  emit a structured warning before falling back to `route.continue()`.
- 🟡 **Test Coverage** (high): No-bleed case uses the wrong second origin — a
  same-origin second navigate with no token is the mutant-killing case.
- 🟡 **Test Coverage** (high): Contract test is tautological — two per-language
  constants cannot be single-sourced; bind with a shared checked-in wire sample
  both sides assert against, or a real end-to-end merge→daemon test.
- 🟡 **Test Coverage**: AC2 CORS preflight — `Authorization` is not
  CORS-safelisted, so fixture B must answer the preflight or the strip-deleted
  mutant survives; confirm the case fails with the strip removed.
- 🟡 **Standards**: Phase 3 breaks the "red first" section ordering of Phases 1–2
  — list the runtime cases first.
- 🟡 **Documentation** (high): Generated-mirror regeneration misdescribed —
  `docs:check` verifies existence only, not content, and default `mise run` runs
  `docs:check`, not `docs:generate`; state that `mise run docs:generate` must be
  run and its output committed, and that the file must not be hand-edited.
- 🔵 **Test Coverage**: The `auth-header.test.js` rewrite drops existing
  security-relevant origin edge cases (port equivalence, subdomain confusion, IDN
  homograph, opaque origins) — preserve them.
- 🔵 **Standards**: AC5 grep uses GNU-only `\|` (false pass on BSD grep); AC4
  grep is unscoped and self-matches `meta/`/docs — scope both and use `-E`.
- 🔵 **Standards**: Phase 2 doc-comment instruction says "three-key guard" for a
  four-key guard.
- 🔵 **Security**: Token still inherited into daemon/client env despite the
  plan's "no longer reaches the daemon through its inherited environment" claim
  (no `env_clear`) — soften the wording or `env_clear` the spawn; ensure the
  malformed-value warning never logs the body/`auth_header` to the bootstrap log.
- 🔵 **Documentation**: Agent-enforcement prose (`agents/browser-analyser.md:123`,
  `SKILL.md:224-226`) not reconciled with the now code-enforced strip; redirect
  caveat omits the failure consequence and a worked example; mirror anchors drift
  (WARNING at 91-97, login-URL to ~213).
- 🔵 **Code Quality**: `applyHeader` boolean flag argument; `merge_request_context`
  generic name; "single-sourced" overstates a two-constant contract.

### Assessment

Not yet approvable — REVISE. The core mechanism is now right (per-request keying
is the correct model for a boundary-less warm daemon), and the Pass 1 criticals
are closed. But the reshape introduced two regressions that must be fixed before
implementation: the token in argv (a security regression) and the late-subresource
strip (a completeness regression, flagged by three lenses). Two decisions are the
user's: whether to adopt a code-enforced cross-origin bound (hard boundary) or
accept the prose-bounded residual and retain a credential caution; and whether to
make auth state sticky across a page's command window. Once those land — plus the
argv fix, the sticky-state fix, the test corrections (same-origin no-bleed, real
contract binding, CORS-aware AC2, preserved origin edges), and the Phase 3
red-first reorder — a third pass should approve.

## Re-Review (Pass 3) — 2026-09-10

**Verdict:** REVISE

The core mechanism has converged and is validated: no lens now challenges the
hard-boundary keying (header keyed to the declared `[location]` origin, held
constant across the crawl), per-request auth state, token-off-argv, or the
late-subresource fix — security, correctness, and architecture each confirm these
are sound, and every Pass-2 major is resolved. The verdict stays REVISE only on
narrower, concrete follow-through: one genuine design gap (a two-env-var silent
failure), one control the plan claims but does not implement (`env_clear`), an
optional simplification, and a batch of test-enforcement, verification-command,
and doc-accuracy fixes. Zero criticals; the architecture is settled.

### Previously Identified Issues (Pass 2 → Pass 3)

- 🔴→🟢 Token in argv — **Resolved.** Executor forwards only `location_url`; the
  JS client injects `auth_header` into the loopback body.
- 🔴→🟢 Cross-origin bounded only by prose — **Resolved.** Code-enforced boundary
  keyed to the declared location origin; a followed link or redirect is stripped.
- 🟡→🟢 Late same-origin subresource — **Resolved.** Location and header ride
  every command, so `snapshot`/`links` keep the state; confirmed correct.
- 🟡→🟢 Env-read inside the merge — **Resolved.** Location threaded as a `run`
  parameter; the merge is pure and needs no env-mutating tests.
- 🟡→🟢 Empty catch — **Resolved.** Binds the error, logs a token-safe message.
- 🟡→🟢 No-bleed wrong origin / contract tautology / AC2 CORS / origin edges —
  **Resolved.** Same-origin re-crawl; checked-in wire fixture; preflight-answering
  fixture; edges preserved.
- 🟡 env-inheritance (token in daemon environ) — **Not resolved; now overclaimed**
  (see new issues): the plan asserts an `env_clear` allowlist that the code and
  Phase 3 do not implement.
- 🟡 Phase 3 red-first / greps — **Partially:** Phase 3 leads with tests, but the
  client and `originOf` cases are still out of order, and AC5/AC4-Phase-1 greps
  remain non-runnable-as-written.
- 🟡 Mirror regeneration mechanism — **Still inaccurate**, now in the other
  direction (see new issues): `docs:check` *does* transitively regenerate.

### New Issues Introduced (Pass 3)

- 🟡 **Correctness + Architecture + Documentation** (the headline, 3 lenses): the
  two-env-var design fails **silently** when `ACCELERATOR_BROWSER_AUTH_HEADER` is
  set but `ACCELERATOR_BROWSER_LOCATION` is absent, unparseable, or diverges from
  the navigated origin — the header is stripped on its own gated origin and the
  inventory is silently unauthenticated, the exact 0209 defect via a plausible
  misconfiguration. Add a loud failure: the daemon warns when a header is present
  but no origin resolves/matches, and the executor or skill fails the crawl (or
  warns) when `AUTH_HEADER` is set without `LOCATION`; surface `LOCATION` in the
  imperative CTA docs (the SKILL auth-walled skip message and the `resolve-auth`
  help), not only the prose.
- 🟡 **Security** (high): the daemon-environment token exclusion is **claimed as
  delivered but not implemented** — `DaemonSpawner` adds env additively with no
  `env_clear` (`process.rs:121-123`), so the token still rests in the daemon's
  `/proc/<pid>/environ`. Add a concrete `process.rs` change (`env_clear` + an
  enumerated allowlist — `STATE_DIR`/`NODE_PATH`/`NS_ROOT`/`BROWSER_EXECUTABLE`
  plus `HOME`/`TMPDIR`/`PATH` as node/Chromium need — and a test asserting the
  header is absent) or temper the claim.
- 🟡 **Architecture**: `location_url` could be injected by the JS client too (both
  it and the token are env-derived and off argv), leaving `merge_allowances`
  untouched — no rename, no three-key guard — and making the field-name contract
  **same-language** (client writes, daemon reads, both JS), dissolving the
  cross-language contract problem. An optional but real simplification.
- 🟡 **Test Coverage** (high): the load-bearing acceptance/mutation cases live in
  `test:integration:design-automation`, which CI and the bare `mise run` do **not**
  run, and the lane has no case-floor or bare-return guard — so the whole
  mutation-resistance argument rests on tests the enforced gate never executes.
  Add a floor/guard to the runtime lane, and state the reliance as a recorded risk.
- 🟡 **Test Coverage + Standards**: the moved `originOf` port-normalization cases
  have no scheduled home (module-internal, `daemon.test.js` is black-box) and no
  `_EXPECTED_DESIGN_AUTOMATION_CASES` bump — export `originOf`, add its unit cases,
  attribute the floor bump.
- 🟡 **Standards** (high): the AC5 sole-launch-site grep returns two matches
  (`daemon.js:182` **and** `playwright-loader.test.js:16`) — scope it to
  production JS / anchor the pattern, and print the full path.
- 🟡 **Documentation** (high): the AC6 login-URL grep does not match the actual
  stale prose ("the resolved `[location]` origin or the `ACCELERATOR_BROWSER_LOGIN_URL`
  origin") — the brackets/backticks/underscore break both alternatives, so it can
  never catch the surviving prose. Grep the literal `ACCELERATOR_BROWSER_LOGIN_URL`
  in the header-path sections instead.
- 🟡 **Standards**: red-first ordering is inconsistent within Phase 2 (client
  production before its test) and Phase 3 (`originOf` cases have no red-first
  entry) — reorder.
- 🔵 Minors: the executor should also refuse a pre-set `auth_header` (not only the
  client) [security]; the daemon's outer catch (`daemon.js:532`) could echo an
  exception to stdout [security]; the error-path `route.continue()` can throw again
  [correctness]; the "no token in argv" behavioural assertion belongs on the
  executor merge (assert no `auth_header` emitted), not the client [test]; the
  Phase 1 AC4 grep is unscoped [standards]; `docs:check` **does** regenerate the
  mirror via its `docs:generate` dependency — the plan's prose rationale is
  inaccurate though the git-clean AC is sound [standards, verified]; AC6 lists
  `browser-analyser.md` but no pattern matches its phrasing [documentation];
  `applyHeader` flag argument, `merge_request_context` generic name, non-Error
  catch logging `undefined` [code-quality]; commit the contract test to the
  fixture approach and bind both emitting sides [test].

### Assessment

Not yet approvable — REVISE — but the plan is close and the architecture is
settled. Every remaining major is a concrete, bounded follow-through rather than a
rethink: fail-loud validation for the `LOCATION`/`AUTH_HEADER` pairing (the one
real design gap, flagged by three lenses), a real `env_clear` implementation, the
optional location-to-client simplification, and a batch of test-enforcement,
grep, red-first, and doc-accuracy corrections. A further full seven-lens pass is
diminishing returns; a focused final revision plus a spot-check is the
proportionate path to approval.

## Approval (Pass 4) — 2026-09-10

**Verdict:** APPROVE

Approved on the round-4 revision, which addressed every Pass-3 finding:

- **location→client simplification** — both `location_url` and `auth_header` are
  injected by the JS client; the Rust `merge_allowances` is untouched; the
  field-name contract is a same-language shared constant (dissolves the
  cross-language contract issue).
- **Two-env-var silent failure** (3-lens headline) — `resolve-auth` requires both
  env vars and fails loudly without a location; the daemon warns on a header with
  no resolvable origin; the CTA docs state both vars.
- **`env_clear` made real** — a concrete `process.rs` change (`env_clear` +
  enumerated allowlist + test); client keeps inheriting.
- **Test/verification fixes** — deterministic late-subresource case with
  auth-state cleared at request end; `originOf` exported with its own unit cases +
  floor bump; AC5 grep scoped to production JS; AC6 login-URL literal; runtime
  lane flagged outside the CI gate with its own floor/guard; `docs:check`
  regeneration prose corrected; red-first ordering consistent across all phases.

No fourth full seven-lens pass was run: the architecture converged at Pass 3 and
round 4 addressed concrete findings without reshaping the design, so approval is
granted on the revision with two items carried to implementation as spot-checks
rather than review blockers — (1) verify the daemon-spawn `env_clear` allowlist
empirically (the daemon/Chromium must still launch with only the allowlisted
vars), and (2) `ACCELERATOR_BROWSER_LOCATION` is a new user-facing requirement
whose fail-loud `resolve-auth` guard is what prevents a one-var misconfiguration
from silently regressing 0209.
