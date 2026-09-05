---
type: "plan-review"
id: "2026-08-31-0206-classify-navigation-urls-review-1"
title: "Plan Review: Classify Navigation URLs, Not Only The Initial Location"
date: "2026-08-31T21:53:24+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-08-31-0206-classify-navigation-urls"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["security", "correctness", "architecture", "test-coverage", "code-quality", "compatibility", "standards", "documentation"]
review_number: 1
review_pass: 3
tags: ["security", "ssrf", "playwright", "executor", "access-policy"]
last_updated: "2026-08-31T22:45:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Classify Navigation URLs, Not Only The Initial Location

**Verdict:** REVISE

The plan is architecturally disciplined and correctly locates redirect
enforcement in the only place it can run — a JS `page.route` handler — with a
clean functional-core/imperative-shell split and inert-plumbing-before-enforcement
phasing that keeps every phase green and independently mergeable. Two structural
gaps recur across nearly every lens and must be closed before implementation: the
per-request allowance scope leaks because `currentAllowances` is set only by
`navigate`/`links` (not by other commands or page-driven navigations on a reused
daemon), and the branchable `details.classification` contract omits the
`unspecified` token the classifier actually emits. A cluster of test-seam and
verification gaps means the plan's headline guarantee — redirect-hop enforcement —
is proven only by a mocked route plus a manual step, against a `daemon.js` that
exposes no injection seam the specified tests require.

### Cross-Cutting Themes

- **Allowance scope leak on a warm daemon** (flagged by: Security, Architecture,
  Code Quality, Correctness) — `currentAllowances` is assigned only in the
  `navigate` and `links` handlers, but the page-global route handler fires for
  every navigation the page issues: `click`/`type`/`wait_for`-triggered
  navigations, meta-refresh, and script-driven `location=` all run under the
  *previous* invocation's allowances (or an undefined value that throws inside the
  handler before any `navigate`). On a reused daemon this defeats the core
  acceptance criterion — that each request is judged under the allowances *its*
  invocation carried.
- **`unspecified` classification token outside the documented set** (flagged by:
  Compatibility, Documentation, Correctness, Standards) — `classifyUrl` returns
  `classification: 'unspecified'` for `0.0.0.0`/`::`, and the corpus vocabulary
  lists it, but Phase 4's success criterion, the work-item AC, and the Phase 6
  PROTOCOL.md value set enumerate only `{private, link-local, reserved,
  insecure-scheme}`. The whole point of carving `details.classification` out of
  the "SHOULD NOT branch on details" rule is that its enumeration is reliable; an
  undocumented fifth value defeats that.
- **Anti-drift corpus binds only sampled inputs, and JS fails open** (flagged by:
  Architecture, Test Coverage, Security, Code Quality) — the corpus is the plan's
  sole defence against the second source of truth, but its only guard is a
  non-zero count, so a branch dropped during migration silently loses coverage on
  both sides; the sketched `classifyUrl` ends in an unconditional `return {ok:
  true}`, failing *open* on any reach class it does not explicitly handle; and a
  new Rust encoding branch forces no corpus case.
- **The specified mock/injection tests have no seam to attach to** (flagged by:
  Test Coverage, Code Quality) — the route handler and `links` folding are inline
  anonymous closures over daemon-scoped state; `daemon.js` exports only
  `startDaemon`, `daemon.test.js` forks a real subprocess and never reaches
  browser ops, and the "started with mock Playwright" comment is stale. The Phase
  4/5 "drive the handler with a fake route" criteria are unimplementable — and
  TDD red-first impossible — without an unspecified extraction/injection refactor.

### Tradeoff Analysis

- **Fail-closed strictness vs crawl continuity**: the plan states fail-closed
  intent (malformed/refused URLs are refusals) but the `classifyUrl` sketch and
  the uninitialised `currentAllowances` both fail *open* or *throw*. Resolve in
  favour of fail-closed — a deny-all default and an explicit final refusal branch
  — accepting that a mis-forwarded allowance refuses a legitimate internal crawl
  rather than silently permitting an internal one.
- **Corpus as sufficient vs necessary-only**: a static fixture is cheap and
  legible but binds only enumerated inputs. Given the security stakes, add a
  completeness assertion (every `HostReach` variant, every `embedded_v4`
  transition form, every `HostError` kind) and consider a differential/property
  generator — accepting more test machinery for a real exhaustiveness guarantee.
- **Details sub-field vs top-level envelope field** (Architecture): promoting
  `details.classification` to branchable erodes the "details are informational"
  invariant into a field-by-field allowlist. A top-level field on
  `navigation-refused` (covered by the existing additive-fields guarantee) keeps
  the invariant intact; weigh contract cleanliness against the extra envelope key.

### Findings

#### Major

- 🟡 **Security / Architecture / Code Quality / Correctness**: Per-request
  allowance scope leaks — `currentAllowances` set only by `navigate`/`links`
  **Location**: Phase 4 §2 (navigate handler); Phase 5 §1 (links)
  Navigations triggered by `click`/`type`/`wait_for` or by page JavaScript fire
  the page-global route handler under the *previous* navigate's allowances; on a
  reused daemon a later, less-privileged invocation's click reaches an internal
  host under an earlier `--allow-internal`. Before any `navigate`,
  `currentAllowances` is undefined and destructuring it throws inside the handler
  (an ambiguous hang, neither continue nor abort). Set allowances once per request
  centrally (every request body already carries them from Phase 3), defaulting to
  deny-all.

- 🟡 **Compatibility / Documentation / Correctness / Standards**: Branchable
  `details.classification` omits the emittable `unspecified` token
  **Location**: Desired End State; Phase 4 Success Criteria; Phase 6 §3
  `classifyUrl` emits `unspecified` for `0.0.0.0`/`::` (Chromium does not
  normalise `0.0.0.0` away), but the documented/asserted set is four tokens. A
  consumer switching exhaustively over the documented set hits an undocumented
  fifth value. Pin one canonical set across the fixture, Phase 4 criterion, and
  PROTOCOL.md — either admit `unspecified` or fold it into an existing token so it
  never reaches the wire. Also pin the token for `error`/malformed refusals
  (numeric-encoding, userinfo, control-character), currently undefined.

- 🟡 **Architecture**: Proposed route handler does not compose with the 0209
  handler it claims to seat
  **Location**: Phase 4 §1 (route handler); What We're NOT Doing (0209)
  Phase 4 installs `page.route('**/*', …)` resolving with `continue`/`abort`;
  0209's dormant `makeAuthHeaderHandler` installs its *own* `page.route('**/*', …)`
  resolving with `continue({headers})`. Playwright runs overlapping handlers
  last-registered-first and requires `route.fallback()` — not `continue()` — to
  chain; two `continue`/`abort` handlers cannot coexist, so one security control
  is silently bypassed when 0209 lands. Specify a single composed handler or
  mandate `fallback()` chaining, and state that 0209 extends this handler in place.

- 🟡 **Architecture / Test Coverage / Security / Code Quality**: Corpus binds only
  sampled inputs; no completeness guard; JS classifier fails open
  **Location**: Phase 1 §1–2; Phase 2 §1
  The count-only guard lets a branch dropped in migration lose coverage on both
  sides silently; the subtle branches most at risk (Teredo inversion, 6to4/NAT64
  unwraps in `embedded_v4`) are not enum variants, so the Rust `token()` match
  does not force a case for them; and `classifyUrl`'s trailing unconditional
  `return {ok: true}` permits any unhandled reach class. Make the JS classifier
  fail-closed and assert every `HostReach` variant, every `embedded_v4` form, and
  every `HostError` kind appears in the corpus.

- 🟡 **Test Coverage / Correctness**: Redirect-hop enforcement has no automated
  test that drives a real redirect
  **Location**: Phase 4 §1; Testing Strategy (Integration Tests)
  The mocked `daemon.test.js` route feeds an already-decomposed request and cannot
  verify that Playwright re-invokes `page.route` per 3xx hop, that
  `isNavigationRequest()` is true for a redirected navigation, or that `abort`
  prevents the fetch — the plan's headline guarantee rests on one manual step. The
  opt-in `daemon-runtime.test.js` (real forked daemon over HTTP) is the natural
  home for a 302-to-internal test asserting the internal request is never issued.

- 🟡 **Test Coverage / Code Quality**: The specified mock/injection tests have no
  seam in `daemon.js`
  **Location**: Phase 4 §1–2; Phase 5 §1; Testing Strategy (Unit Tests)
  The route decision and `links` folding are anonymous closures over module-scoped
  state; `daemon.js` exports only `startDaemon` and has no fake-chromium/page
  injection point. Extract pure functions — `classifyNavigationRequest(request,
  allowances)` and a `links` host-classify/strip — tested directly against fake
  `route`/anchor inputs, so the Phase 4/5 criteria are implementable and TDD
  red-first is possible.

- 🟡 **Correctness / Code Quality**: `lastRefusal` attribution by truthiness
  misattributes sub-frame or unrelated failures
  **Location**: Phase 4 §2 (navigate handler)
  `isNavigationRequest()` is true for iframe navigations, and the catch block
  returns `navigation-refused` whenever `lastRefusal` is merely truthy — the prose
  says "matches this navigation" but the code performs no match. A top-level
  `goto` failing for an unrelated reason (timeout, DNS) after an internal iframe
  was aborted reports the iframe's URL/classification and masks the real failure.
  Record `lastRefusal` only for `page.mainFrame()` requests, or match its URL
  against the failed navigation before returning the envelope.

- 🟡 **Correctness**: Daemon-global mutable state is not race-safe across
  concurrent connections
  **Location**: Desired End State; Key Discoveries (daemon-scoped state)
  "One request per connection sequentially" guarantees ordering *within* a
  connection, not a single in-flight request *across* connections. Two concurrent
  invocations on the reused daemon let request B overwrite `currentAllowances` at
  an `await` point while request A's redirect hop is still being classified by the
  shared handler — a security-relevant misclassification on a shared single page.
  Serialise `handleRequest` (per-daemon mutex/queue) or bind allowances to the
  in-flight navigation locally. Architecture reinforces this as an implicit
  invariant that should be named at the declaration site.

- 🟡 **Documentation**: Promised changelog entry is unwired and mislocated
  **Location**: Migration Notes; Phase 6 Changes Required
  The Migration Notes want a changelog entry "under the design skill", but no
  per-skill changelog exists (only root `CHANGELOG.md` `[Unreleased]`), no phase
  lists a changelog file, and no criterion verifies one. A behaviour-breaking
  change risks shipping unrecorded. Add `CHANGELOG.md` to Phase 6 with the exact
  `[Unreleased]` subsection (Changed/Security) and a verifying criterion, and
  correct the location wording.

- 🟡 **Documentation**: Phase 6 verification greps for deletion only and never
  inspects PROTOCOL.md
  **Location**: Phase 6 Success Criteria (Automated Verification)
  The sole automated check proves two phrases were removed and excludes
  `PROTOCOL.md` from its path list — so the most consumer-critical change (the
  `navigation-refused` envelope and the branchable-details carve-out) has no
  automated verification, and nothing asserts the required positive statements
  were added. Add presence assertions (grep for `navigation-refused` in
  PROTOCOL.md; the per-request classification sentence in each doc).

- 🟡 **Security**: De-scoped residuals are materially weaker under this adversarial
  threat model than for `validate-source`
  **Location**: What We're NOT Doing (DNS rebinding; Subresource SSRF)
  `validate-source` classifies operator-supplied URLs; this plan classifies
  attacker-influenced navigation and redirect targets. An attacker controlling a
  hostname's DNS registers a public name resolving to `169.254.169.254` and
  bypasses pre-resolution classification entirely; a hostile page `fetch()`/`<img>`
  reads cloud-metadata as a subresource whose response flows into the DOM
  `browser-analyser` snapshots. The cost argument for the subresource exclusion is
  weak given the plan calls classification a cheap in-process check. At minimum,
  Phase 6 docs must state these residuals plainly rather than framing navigation
  coverage as closing SSRF; reconsider classifying same-document XHR/fetch to
  internal reaches.

- 🟡 **Documentation**: New `navigation-refused` error has no agent-facing handling
  guidance
  **Location**: Phase 3 §3; Phase 4 (agent docs)
  The work item's design decision is that the agent decides whether to stop or
  continue on a mid-crawl refusal, but only Phase 3 §3 touches the agent files
  (to forward allowances) — no phase tells `browser-analyser.md`/`browser-locator.md`
  what `navigation-refused` means or how to react. The agents meet an unknown
  non-retryable code and may abort or loop. Add a short instruction: non-retryable,
  a policy refusal not a transient failure, and the expected reaction.

#### Minor

- 🔵 **Code Quality**: `links` folding uses double-negation and a URL round-trip
  **Location**: Phase 5 §1
  `refusable = a.host && !classifyUrl(...).ok` (truthy, not boolean) then
  `same_origin && !refusable` reads as "same-origin and not not-refused", and
  reconstitutes `${scheme}://${host}` to re-parse a URL the browser already
  parsed. Introduce a `classifyLocation({scheme, host})` core and write
  `policyAllows = !a.host || classifyLocation(a).ok`.

- 🔵 **Security**: `navigation-refused` message embeds the full refused URL
  **Location**: Phase 4 §2
  `Navigation to ${lastRefusal.url}` uses `request.url()`, including query and
  fragment — the same token-bearing components PROTOCOL.md:269 strips from `links`
  and `href`. Limit the message and any details to host plus pathname.

- 🔵 **Compatibility**: `same_origin` semantic overload changes a documented field
  **Location**: Phase 5; What We're NOT Doing
  A genuinely same-origin internal destination now reports `same_origin: false`.
  Safe for the sole current consumer, but Phase 6 must *restate* the field
  definition in PROTOCOL.md (not footnote it) to "same-origin and not
  policy-refused", so no future consumer relies on the old meaning.

- 🔵 **Compatibility**: New request-body fields not documented in the request schema
  **Location**: Phase 6 §3
  `allow_internal`/`allow_insecure_scheme` are added to the per-request body but
  Phase 6 documents only the error code and carve-out, not the fields. PROTOCOL.md
  is the canonical reference; document them as optional, default-false, injected by
  the executor. (Completeness gap, not a break — no v2 bump.)

- 🔵 **Standards**: Fixture directory `testdata/` deviates from `tests/fixtures/`
  **Location**: Phase 1 §1
  The repo convention is `cli/<crate>/tests/fixtures/`; `testdata/` is a Go idiom
  not otherwise used here. Both the `CARGO_MANIFEST_DIR` loader and the JS
  repo-relative path work equally from `cli/design/tests/fixtures/`.

- 🔵 **Standards**: `host-reach.js` under-describes a module that also embodies
  canonicalisation and access-policy
  **Location**: Phase 2 §1
  The module reproduces `host.rs`, `host_reach.rs` *and* `access_policy.rs` and
  fronts the scheme/allowance verdict, but is named for its reachability sub-step.
  Name it for its responsibility (e.g. `access-policy.js` / `url-classification.js`).

- 🔵 **Architecture**: Branchable carve-out erodes the details-are-informational
  invariant
  **Location**: Phase 6 §3
  Consider promoting the classification to a top-level field on `navigation-refused`
  (covered by additive-fields guarantee) rather than a branchable sub-field of
  `details`, keeping the general invariant intact.

- 🔵 **Documentation**: Removing the "not a boundary" hedge risks overclaiming
  **Location**: Phase 6 §1–2
  Deleting "front door, not a boundary" while adding a per-request-coverage
  sentence can read as "fully bounded" when subresources and DNS-rebinding hosts
  remain unclassified. State the scope precisely: per-request navigation and
  followed links are classified, still pre-resolution, not covering subresources.

- 🔵 **Documentation**: `access_policy.rs` has no "front door" wording to remove
  **Location**: Phase 6 §1
  The module doc is a single line with no such wording; Phase 6 correctly only
  *adds* to it. Note the addition-only intent so the AC-wording discrepancy
  ("no longer appears in … access_policy") is not read as a missed removal.

- 🔵 **Documentation**: Carve-out should state whether the value set is closed
  **Location**: Phase 6 §3
  A branching consumer needs to know whether to code a default for future values.
  State explicitly whether the classification set is exhaustive/stable or
  extensible.

#### Suggestions

- 🔵 **Standards**: Executor flag-ordering rationale is imprecise — the constraint
  is the CLI-side position of flags relative to the `trailing_var_arg` positional,
  not struct field declaration order. Restate in invocation terms and note it as a
  deliberate departure from the `ValidateSource` convention (Phase 3 §1).
- 🔵 **Compatibility**: Also assert a flag placed *after* `command` is captured as
  a trailing argument, pinning the flag-then-command-then-trailing contract
  (Phase 3 §1).
- 🔵 **Test Coverage**: Raise `_EXPECTED_DESIGN_AUTOMATION_SUITES` to 10 when adding
  the new test, and note the `_bare_returns_in_tests` constraint so the
  fixture-driven test iterates-and-asserts rather than early-returns (Phase 2 §2).
- 🔵 **Test Coverage / Correctness**: Export and test the lower-level
  `canonicalise`/`classifyHost` against the `reach` and `error` corpus layers —
  through `classifyUrl` alone, `loopback` and `public`-https both collapse to
  `{ok: true}` and error tokens are indistinguishable, weakening the anti-drift
  binding at the highest-risk branches (Phase 2 §1–2).
- 🔵 **Code Quality**: Port the subtle branches as named helpers (`embeddedV4`,
  `unwrapTeredoMappedAddress`, `isNumericEncoding`) so the JS reads as domain steps
  and needs no explanatory comment against the repo's low comment tolerance
  (Phase 2 §1).
- 🔵 **Test Coverage**: State the TDD ordering explicitly for the fixture rewrite
  and the route handler — failing fake-route and envelope tests observed red before
  the handler is added (Phase 1; Phase 4/5).

### Strengths

- ✅ The single language-neutral vector corpus is the right anti-drift design for
  security-critical logic split across two languages, and the test-only Rust
  reach→token mapping guards the shared vocabulary from a token Rust could never
  emit.
- ✅ Redirect enforcement is correctly located in the only place it can run (a JS
  route handler; Rust never sees the hops after `execve`), and `route.abort` firing
  before the fetch satisfies "the refused URL is never loaded" by construction.
- ✅ Per-request allowance scoping via the JSON body (not daemon-lifetime env vars)
  correctly avoids leaking an allowance into a later reused-daemon invocation — the
  right mechanism, undermined only by the incomplete assignment flagged above.
- ✅ Inert-plumbing-first sequencing (Phase 3 before enforcement) keeps every
  phase's `mise run` green, makes each phase independently mergeable and strictly
  more secure than the last, and prevents a window where a legitimate
  `--allow-internal` crawl is refused mid-flight.
- ✅ The `navigation-refused` envelope reuses the `makeError`/`wall-clock-exceeded`
  precedent (category `browser`, `retryable: false`, structured `details`), and the
  Phase 3 payload-pre-set refusal mirrors the existing `client.js` command/protocol
  guard — both keep established idioms.
- ✅ `links` keeps the full host inside Node and deletes it before returning, so
  only `pathname`/`same_origin`/`scheme`/`text`/`role` cross the wire, consistent
  with the existing `href`-stripping rationale.
- ✅ Command-set equality (`FORWARDABLE_COMMANDS`, `daemon.test.js`) is correctly
  preserved because the change adds request fields, not commands — called out
  explicitly.

### Recommended Changes

1. **Set `currentAllowances` once per request, deny-all by default** (addresses:
   allowance scope leak; fail-closed default; install-once stale state) — assign
   from the request body at the top of request handling for every command, not
   only `navigate`/`links`, and initialise to `{allowInternal: false,
   allowInsecureScheme: false}`. Add a `daemon.test.js` case where a non-navigate
   command under stricter allowances refuses a page-triggered internal navigation,
   and one asserting a request with no allowance fields is fully restricted.

2. **Pin one canonical `classification` value set end-to-end** (addresses:
   `unspecified` token gap; malformed/error token undefined) — decide whether
   `unspecified` and the canonicalisation-error tokens are emitted on the wire;
   make the fixture vocabulary, Phase 4 success criterion, and Phase 6 PROTOCOL.md
   table enumerate exactly that set; assert the token for every `error`/malformed
   refusal on both sides.

3. **Make the JS classifier fail-closed and add a corpus completeness guard**
   (addresses: fail-open default; no completeness guard) — replace the trailing
   `return {ok: true}` with an explicit deny for any unhandled reach; add a Rust
   assertion that every `HostReach` variant, every `embedded_v4` transition form,
   and every `HostError` kind has at least one corpus case.

4. **Specify a single composed route handler and the 0209 seam** (addresses: 0209
   composition hazard) — one `page.route` that runs classification then header
   logic in sequence (or mandate `route.fallback()` chaining), with a Phase 4 note
   that 0209 extends this handler in place rather than registering a second `**/*`
   route.

5. **Extract pure, testable functions and add a real redirect test** (addresses:
   missing test seam; redirect unverified; per-request reuse untested) — extract
   `classifyNavigationRequest(request, allowances)` and the `links`
   classify/strip; add a real 302-to-internal test to `daemon-runtime.test.js`
   asserting the internal request is never issued; add an automated two-navigation
   reuse-scope case once the seam exists.

6. **Fix refusal attribution to the main frame** (addresses: `lastRefusal`
   misattribution) — record `lastRefusal` only for `page.mainFrame()` requests, or
   match its URL against the failed navigation before returning
   `navigation-refused`; otherwise re-throw.

7. **Serialise request processing or bind allowances locally** (addresses:
   concurrency race) — a per-daemon mutex/queue around `handleRequest`, or bind
   allowances to the in-flight navigation rather than daemon-global state; name the
   sequential-processing dependency as an invariant at the declaration site.

8. **Complete the Phase 6 documentation and verification** (addresses: changelog
   unwired; grep-only verification; agent-facing docs; residual honesty; request
   fields; `same_origin` restatement) — add the `CHANGELOG.md` entry with a
   verifying criterion; add presence assertions covering PROTOCOL.md; document the
   allowance request fields and the redefined `same_origin`; add agent-facing
   `navigation-refused` handling guidance; and state the subresource/DNS-rebinding
   residuals plainly.

9. **Minor consistency fixes** (addresses: fixture location; module name; URL leak
   in message; double-negation) — move the fixture under `tests/fixtures/`; rename
   the JS module for its access-policy responsibility; limit the refusal message to
   host plus pathname; and clean the `links` boolean logic.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Security

**Summary**: A well-conceived SSRF-mitigation plan — classification moves from the
front door to per-request navigation, aborts before the network fetch, keeps host
authorities inside Node, and defends the dual implementations with a shared corpus.
But per-request allowance scoping is wired only into `navigate`/`links`, leaving
navigations triggered by other commands or page JS to run under stale allowances
on a reused daemon. The two de-scoped residuals (subresource SSRF, DNS rebinding)
are materially more dangerous under this adversarial threat model than under
`validate-source`'s operator model, because the attacker now controls the
navigation target and its DNS.

**Strengths**:
- Shared language-neutral corpus binds Rust and JS in CI, targeting the highest
  risk of reimplementing security logic in a second language.
- `route.abort()` fires before the network fetch and covers the initial navigation
  plus every redirect hop through one mechanism.
- `links` keeps the full host inside Node and deletes it before returning; only
  pathname/same_origin/scheme/text/role cross the wire.
- The Phase 3 merge refuses a payload that pre-sets an allowance key, mirroring the
  `client.js:46` guard.
- Fail-closed intent stated explicitly; enforcement staged after inert plumbing.

**Findings**:
- 🟡 major, high — *Per-request allowance scope leaks* (Phase 4 route handler +
  `currentAllowances`; Phase 5 links): the handler reads a daemon-scoped
  `currentAllowances` set only by `navigate`/`links`; `click`/`type`/`evaluate`
  and page JS (meta-refresh, `setTimeout`) fire under the previous invocation's
  allowances. Across a reused daemon, invocation A's `--allow-internal` decides B's
  page-triggered navigation to `169.254.169.254`. Set `currentAllowances` at the
  top of `handleRequest` for every command, deny-all default.
- 🟡 major, medium — *Pre-resolution classification far weaker under this threat
  model* (What We're NOT Doing: DNS rebinding): attacker controls both redirect
  hostname and DNS; a public name resolving to `169.254.169.254` bypasses the
  control. Enforce post-resolution or state the residual plainly.
- 🟡 major, medium — *Navigation-only scope leaves subresource exfiltration open*
  (What We're NOT Doing: Subresource SSRF): a hostile page can `fetch()`/`<img>`
  cloud-metadata as a subresource and render it into the DOM the analyser
  snapshots; the cost argument is weak given classification is a cheap in-process
  check. Classify same-document XHR/fetch to internal reaches or call out the
  residual.
- 🟡 major, medium — *Fixed corpus binds only enumerated cases* (Phase 2): the JS
  host parser reimplements `IpAddr::from_str` + `Host::canonicalise` from scratch;
  a novel unlisted encoding can classify `public` in JS but `internal` in Rust.
  Enumerate a case per branch and add a differential/property generator.
- 🔵 minor, medium — *Refusal message embeds full URL* (Phase 4): `request.url()`
  includes query/fragment that may carry tokens. Limit to host + pathname.
- 🔵 minor, medium — *Confirm `allowancesOf` defaults closed and `classifyUrl` never
  throws on missing allowances* (Phase 3/4): default to `{false, false}` and test a
  no-allowance request is fully restricted.

### Correctness

**Summary**: The classification logic (reach-before-scheme ordering, loopback/
unspecified carve-outs, the shared corpus) is sound and faithfully mirrors the Rust
domain. The principal risks are in the new daemon-scoped mutable state: navigation-
triggering commands other than `navigate` never refresh `currentAllowances`;
`lastRefusal` attribution checks only truthiness; and the anti-drift corpus is
under-specified against a single `classifyUrl` entry point that cannot express the
reach/error tokens it asserts.

**Strengths**:
- Phasing enforces allowances flow inert before enforcement turns on.
- `classifyUrl` reproduces `evaluate`'s exact decision order.
- `route.abort` before the fetch guarantees a refused URL is never loaded by
  construction.
- The single corpus with a Rust reach→token mapping fails CI on either side
  diverging.

**Findings**:
- 🟡 major, medium — *Non-navigate commands never refresh `currentAllowances`*
  (Phase 4 §2): stale allowances bleed across invocations; undefined before first
  navigate throws inside the handler. Set once per request centrally, deny-all
  default.
- 🟡 major, medium — *`lastRefusal` checked only for truthiness* (Phase 4 §2):
  `isNavigationRequest()` is true for iframes; an aborted iframe plus an unrelated
  top-level failure reports the iframe's URL/classification and masks the real
  failure. Restrict to `page.mainFrame()` or match the URL.
- 🟡 major, medium — *Daemon-global state not race-safe across connections*
  (Desired End State / Key Discoveries): sequential-per-connection ≠ single
  in-flight across connections; concurrent B overwrites allowances while A's hop is
  classified. Serialise `handleRequest` or bind locally.
- 🔵 minor, high — *`classifyUrl` emits values outside the documented set* (Phase 4
  vs Phase 2 / Phase 6): `unspecified` for `0.0.0.0`/`::`; canonicalisation error
  tokens. Fold into the documented set or map to a catch-all.
- 🔵 minor, medium — *Single `classifyUrl` cannot express reach/error tokens* (Phase
  2): `loopback` and `public`-https both collapse to `{ok: true}`; export/test
  `canonicalise`/`classifyHost` against the reach/error layers.
- 🔵 minor, low — *Redirect enforcement rests on an unverified Playwright
  assumption* (Phase 4 §1): mocked route verifies only branch logic, not that
  Playwright re-invokes the handler per hop. Add an integration check.

### Architecture

**Summary**: Architecturally disciplined — redirect enforcement is correctly
located in the only place it can run, the functional-core/imperative-shell split is
clean, and inert plumbing precedes enforcement. The load-bearing risk is the
deliberately-accepted second source of truth: the corpus binds only sampled inputs
and the JS classifier defaults fail-open, so evolutionary fitness under a new reach
class is weaker than the corpus framing implies. Two seam concerns stand out — the
page-global handler fed by state only `navigate` maintains, and a handler mechanism
that does not compose with the 0209 handler it claims to seat.

**Strengths**:
- Explicitly acknowledges the second-source-of-truth tradeoff and pairs it with a
  CI-enforced anti-drift corpus.
- Clean functional-core/imperative-shell boundary; both classifiers pure.
- Correctly reasons redirect enforcement can only live in JS and per-request scope
  falls out of the per-call `execve` model.
- Fail-closed allowance default; JSON merge refuses pre-set allowance keys.
- Sound phase sequencing; each phase independently mergeable and strictly more
  secure.

**Findings**:
- 🔴 major, high — *Route handler does not compose with the 0209 handler* (Phase 4 /
  0209): two `page.route('**/*')` handlers both calling `continue`/`abort` cannot
  coexist; Playwright requires `route.fallback()` to chain. One control silently
  bypassed when 0209 lands. Specify a single composed handler.
- 🟡 major, medium — *Page-global handler fed by state only `navigate` maintains*
  (Phase 4): click/form/`wait_for` navigations fire under stale or uninitialised
  allowances; the latter throws. Initialise fail-closed and decide how non-navigate
  paths are classified.
- 🟡 major, medium — *Corpus binds only sampled inputs; JS fails open* (Phase 1 &
  2): subtle encoding branches are not enum variants so nothing forces a case; the
  `classifyUrl` sketch's trailing `return {ok: true}` allows an unhandled reach.
  Fail-closed + per-variant/per-branch corpus assertion.
- 🔵 minor, medium — *Sequential-protocol invariant implicit at the coupling site*
  (Key Discoveries / Phase 4): name it at the mutable-state declaration.
- 🔵 minor, medium — *Branchable `details.classification` erodes the informational
  contract* (Phase 6): consider a top-level envelope field instead.

### Test Coverage

**Summary**: The shared corpus is a strong, well-motivated coverage design, and the
executor-plumbing negative paths are properly automated. But the redirect-hop AC is
met only by a mocked simulation plus a manual step; the corpus has no completeness
guard so migrating inline Rust cases risks silent branch-coverage regression; and
the mocked page/route tests require an injection/extraction seam in `daemon.js` that
neither exists nor is planned.

**Strengths**:
- The single JSON corpus iterated by both sides turns a JS drift red in CI
  automatically.
- Test-only Rust reach→token mapping validates the shared vocabulary.
- Fixture-count (non-zero) assertion on both sides guards a mis-resolved path.
- Payload-pre-set refusal and clap flag parsing are automated negative tests.
- Error-envelope assertions are specific; positive paths included.

**Findings**:
- 🔴 major, high — *Redirect-hop AC has no automated test that drives a redirect*
  (Phase 4 / Integration Tests): a mocked route cannot verify per-hop re-invocation
  or that abort prevents the fetch; `daemon-runtime.test.js` is the natural home.
- 🟡 major, high — *No completeness guard; count-only fixture risks silent branch
  loss* (Phase 1 §1–2): assert a case per `HostReach` variant, `embedded_v4` form,
  and `HostError` kind.
- 🟡 major, medium — *Mocked page/route tests need a seam `daemon.js` lacks* (Phase
  4/5): `daemon.js` exports only `startDaemon`; the "mock Playwright" comment is
  stale. Extract pure functions or add a chromium-injection parameter.
- 🟡 major, medium — *Reach/error corpus layers may not be exercised by a single
  `classifyUrl`* (Phase 2): export a reach-level entry or re-express cases as policy
  cases forcing the branch.
- 🔵 minor, medium — *Fail-closed refusal token undefined for malformed/numeric/
  userinfo/control-char* (Phase 2/4): pin and assert each.
- 🔵 minor, medium — *Per-request reuse-scope proven only manually* (Phase 4): add
  an automated two-navigation differing-allowances case.
- 🔵 suggestion, high — *Suite floor not raised; bare-return guard applies* (Phase
  2): raise `_EXPECTED_DESIGN_AUTOMATION_SUITES` to 10; write iterate-and-assert.
- 🔵 suggestion, medium — *TDD ordering asserted but not evidenced* (Phase 1, 4/5):
  state the failing tests are observed red first.

### Code Quality

**Summary**: Well-structured, phased to stay green, and reuses the `makeError`
envelope cleanly. The main risks concentrate in the daemon: module-scoped mutable
state driving control flow by side-effect, an anonymous route-handler closure with
no extraction seam despite claiming unit tests against it, and a `links` snippet
with a double-negation and a URL round-trip. The Rust↔JS duplication is a real
maintainability cost, explicitly acknowledged and mitigated by the corpus — the
right call.

**Strengths**:
- The single corpus is a strong DRY/anti-drift design; the Rust reach→token mapping
  guards the vocabulary.
- The `navigation-refused` envelope mirrors `wall-clock-exceeded`.
- `classifyUrl` reads top-to-bottom in reachability-before-scheme order.
- Inert-plumbing-first sequencing shrinks each change's blast radius.
- The payload-pre-set refusal mirrors the existing `client.js` guard.

**Findings**:
- 🔴 major, high — *Route decision buried in an anonymous closure with no test seam*
  (Phase 4 §1): extract `classifyNavigationRequest(request, allowances)` and keep
  the closure a thin adapter.
- 🟡 major, medium — *Refusal inferred from a mutable side-channel, not a typed
  signal* (Phase 4 §2): a reorder or coincident `goto` failure mislabels an
  unrelated error. Carry the verdict structurally and match on the abort error's
  identity.
- 🟡 major, medium — *Install-once handler reading per-request state relies on
  unenforced set-before-use* (Phase 4): give `currentAllowances` a fail-closed
  default; state how click/type-initiated navigations are classified.
- 🔵 minor, high — *`links` folding: double-negation, non-boolean flag, URL
  round-trip* (Phase 5 §1): introduce `classifyLocation`; positive naming.
- 🔵 minor, medium — *Corpus binds drift only at enumerated cases* (Phase 1–2):
  assert every `HostReach`/`embedded_v4`/`HostError` case.
- 🔵 suggestion, low — *Port subtle branches as named helpers, not commented
  arithmetic* (Phase 2 §1): `embeddedV4`, `unwrapTeredoMappedAddress`,
  `isNumericEncoding`.

### Compatibility

**Summary**: Largely additive at the wire level — PROTOCOL.md's Stability
Commitment permits new error codes and optional request fields, and both agents
branch on `error.category` not the code, so `navigation-refused` renders gracefully.
The sharper concerns are three deliberate contract changes: the branchable
`details.classification` value set omits a producible token, the `same_origin` field
is semantically overloaded, and a previously-successful navigate/redirect is now
refused (intended but breaking). Command-set equality is correctly preserved.

**Strengths**:
- `navigation-refused` is additive and explicitly permitted; agents dispatch on
  `error.category`.
- Command-set equality preserved — request fields, not commands.
- Per-request scoping avoids an allowance leaking into a reused-daemon invocation.
- Phased sequencing prevents a mid-crawl refusal window.

**Findings**:
- 🔴 major, high — *Branchable `details.classification` omits the producible
  `unspecified` token* (Desired End State / Phase 4 / Phase 6): a `navigate` to
  `http://0.0.0.0` emits an undocumented fifth value. Reconcile the producible
  tokens with the documented contract.
- 🟡 minor, medium — *Semantic overload of `same_origin`* (Phase 5): a same-origin
  internal destination now reports `false`; Phase 6 must restate the field
  definition, not footnote it.
- 🔵 minor, medium — *New request fields not documented in the request schema*
  (Phase 6 §3): document `allow_internal`/`allow_insecure_scheme` as optional,
  default-false.
- 🔵 minor, medium — *Behavioural breaking change* (Migration Notes / Phase 4): a
  previously-successful navigate/redirect may now be refused; keep the changelog
  entry and verify allowance forwarding covers every call site.
- 🔵 suggestion, low — *Confirm clap binds `--allow-*` ahead of the trailing
  positional* (Phase 3 §1): also assert a flag after `command` is a trailing
  argument.

### Standards

**Summary**: Largely faithful to project conventions — the `navigation-refused`
code, category, and details shape follow the `makeError`/`wall-clock-exceeded`
precedent; the JS module/test pair slot into the `lib/*.test.js` glob with no task
wiring; snippets honour the low comment tolerance. Three genuine inconsistencies
remain: a divergent fixture directory name, a module name that under-describes what
it embodies, and a `details.classification` vocabulary not held consistent across
the plan's own phases, the work item, and the protocol.

**Strengths**:
- `navigation-refused` matches the kebab-case/category/retryability/details
  precedents precisely.
- The `lib/host-reach.js`/`.test.js` pair auto-joins `test:unit:design-automation`.
- The Phase 3 payload-guard mirrors the `client.js:46` `payload-rejected` guard.
- Phase 6 reconciles the "SHOULD NOT branch on details" caveat rather than silently
  contradicting it.
- Snippets are comment-free; success criteria are mostly machine-verifiable.

**Findings**:
- 🔵 minor, medium — *`testdata/` deviates from `tests/fixtures/`* (Phase 1 §1).
- 🔵 minor, medium — *`host-reach.js` under-describes canonicalisation + access-policy*
  (Phase 2 §1).
- 🔵 minor, medium — *`details.classification` value set inconsistent across plan,
  AC, protocol* (Phase 4 vs Phase 2 vs work item): `unspecified` is emittable but
  absent from the criteria. Pin one canonical set.
- 🔵 suggestion, low — *Executor flag ordering diverges from `ValidateSource` and the
  rationale is imprecise* (Phase 3 §1): restate in CLI-invocation terms.

### Documentation

**Summary**: Phase 6 is thoughtfully scoped and correctly anticipates the biggest
hazard — the "SHOULD NOT branch on details" contradiction — planning an explicit
carve-out. But the documented `details.classification` set omits the emittable
`unspecified` token, the promised changelog entry is unwired and mislocated, and the
automated criterion is a phrase-removal grep that never touches PROTOCOL.md and
cannot prove the positive statements were added. There is also no agent-facing
documentation for the new non-retryable `navigation-refused` error.

**Strengths**:
- Phase 6 surfaces and plans to reconcile the details-branching tension.
- Retains the still-true residual caveats and the pinned prose tests.
- Slots `navigation-refused` into the existing error table consistently with
  `wall-clock-exceeded`.
- Manual verification checks each doc for the positive per-request statement.

**Findings**:
- 🔴 major, high — *Documented value set omits emittable `unspecified`* (Phase 4 /
  Phase 6 §3).
- 🔴 major, high — *Promised changelog entry is unwired and points at a non-existent
  location* (Migration Notes / Phase 6): add `CHANGELOG.md` with a verifying
  criterion; correct the location.
- 🔴 major, high — *Grep criterion proves deletion only and never inspects
  PROTOCOL.md* (Phase 6 Success Criteria): assert presence of the new statements.
- 🟡 major, medium — *New `navigation-refused` error has no agent-facing handling
  doc* (Phase 3 §3 / Phase 4): add a short instruction on meaning and reaction.
- 🔵 minor, medium — *Removing the "not a boundary" hedge risks overclaiming a
  complete boundary* (Phase 6 §1–2): state the scope precisely.
- 🔵 minor, medium — *Carve-out should state whether the value set is closed* (Phase
  6 §3).
- 🔵 minor, medium — *`access_policy.rs` has no "front door" wording to remove*
  (Phase 6 §1): note the addition-only intent.

## Re-Review (Pass 2) — 2026-08-31

**Verdict:** REVISE (focused — one round from done)

The revision resolves essentially every prior finding across all eight lenses:
the module-global `currentAllowances`/`lastRefusal` are gone, replaced by a
per-navigation `guardedNavigation` closure with a local refusal and `fallback()`
composition; `classifyUrl` fails closed; the corpus gains a per-variant
completeness assertion and exported reach/error layers; the classification set is
pinned (with `unspecified` + `malformed`) identically across code, criteria and
PROTOCOL.md; the changelog, agent docs, `same_origin` restatement and residual
honesty all land. What remains is a **new, convergent tradeoff** the per-navigation
design introduced — flagged independently by Security, Correctness, Architecture
and Code Quality — plus a `links` canonicalisation asymmetry and a handful of
polish gaps.

### Previously Identified Issues

- ✅ **Security** allowance scope leak — Resolved (per-navigation closure, no
  module state).
- ✅ **Security** navigation-refused URL leak — Resolved (`hostAndPath`).
- ✅ **Security** fail-closed defaults / missing-allowances — Resolved
  (`allowancesOf` deny-all, `malformed`, unhandled-reach refusal).
- ✅ **Security / Documentation** DNS-rebinding & subresource residuals — Resolved
  as honestly documented (not fixed — the accepted scope decision).
- 🟡 **Security / Architecture / Correctness / Code Quality** corpus binds only
  sampled inputs — Partially resolved (completeness assertion added; a
  differential/property check is still absent).
- ✅ **Correctness** `lastRefusal` misattribution — Resolved (main-frame gate +
  local variable).
- ✅ **Correctness / Architecture** daemon-global race — Resolved for the defect
  (state removed); shared-`page` single-flight assumption remains implicit.
- ✅ **Correctness / Compatibility / Documentation / Standards** `unspecified`
  token gap — Resolved (set pinned incl. `malformed`).
- ✅ **Test Coverage** no real redirect test / no completeness guard / missing
  seam / reach-error layers — Resolved (forked-daemon redirect + reuse tests,
  completeness assertion, extracted pure functions, exported layers).
- ✅ **Code Quality** anonymous closure / side-channel / install-once /
  double-negation — Resolved (pure `classifyNavigationRequest`, structured
  outcome, `classifyLocation`).
- ✅ **Architecture** 0209 composition — Resolved (`route.fallback()`; the
  in-place-extension claim softened to a decoupled note in the recommendation).
- ✅ **Compatibility / Standards / Documentation** `same_origin` restatement,
  request-field docs, changelog wiring, fixture location, module rename, clap
  rationale, grep→presence, agent docs, closed-set statement — Resolved.

### New Issues Introduced

- 🟡 **Security / Correctness / Architecture / Code Quality** (major, medium):
  the per-navigation guard **narrows the enforcement envelope**. The handler is
  installed only for the wrapped `page.goto` and torn down in `finally`, so a
  page-initiated navigation firing outside a guard window — `<meta refresh>`, a
  `setTimeout(() => location = 'http://169.254.169.254/')`, or a redirect after
  `domcontentloaded` — runs with no handler installed and is never classified. The
  Phase 4 overview's "deny-all default before any navigate" is inaccurate: outside
  a window there is no handler at all. The prior persistent-handler design would
  have caught these (under the wrong allowances — the bug the revision fixed).
- 🟡 **Correctness** (major, medium): `click`/`type` wrapping is **under-specified
  and subtly wrong** — `page.click` resolves when the click dispatches, not when
  the triggered navigation completes, so a guarded click to a refused internal
  host can return `{ ok: true }` before `refusal` is set: the navigation is
  blocked but silently, with no `navigation-refused` signal.
- 🟡 **Security** (major, medium): `links` classifies a **browser-normalised host**
  (`new URL()` already lowercased/punycoded/numeric-resolved), bypassing the raw
  `canonicalise`, so `navigate` and `links` can disagree on the same target and the
  userinfo/numeric/control-char rejections never fire for links.
- 🟡 **Test Coverage** (major, medium ×2): sub-frame isolation is listed under the
  pure `classifyNavigationRequest` unit test where the frame check is not reachable;
  and the redirect/reuse tests live only in the opt-in
  `test:integration:design-automation` lane that `mise run` never executes — the
  Phase 4 checklist presents them ambiguously as CI-enforced.
- 🔵 **Security / Correctness** (minor): a classifier **exception in the route
  handler** calls neither `fallback()` nor `abort()`, hanging the request rather
  than failing closed — needs a try/catch → `abort` wrapper.
- 🔵 **Code Quality** (minor): the `{ ok }`/`{ refused }` discrimination depends on
  the implicit "goto rejects on main-frame abort" invariant (derive from `refusal`
  instead); `hostAndPath` re-parses the raw URL exactly where it is `malformed`.
- 🔵 **Compatibility / Test Coverage / Standards** (minor/suggestion): `malformed`
  and `unspecified` are not asserted end-to-end through `classifyUrl` in the
  corpus; the case-count floor and error-token vocabulary are unpinned; the
  changelog omits the `links`/`same_origin` change; the Phase 6 "agreed phrase"
  grep target is unpinned; the unhandled-reach branch emits a raw reach token.

### Assessment

The plan moved from genuinely blocking (a scope leak defeating its core acceptance
criterion) to near-final. The one substantive remaining decision is the
enforcement-envelope tradeoff the per-navigation-local binding introduced: it
trades the original cross-request-bleed bug for a coverage gap on out-of-window and
page-initiated navigations. A persistent page-lifetime handler sourcing allowances
from a per-command binding set at handler entry (deny-all default) closes both —
the synthesis the reviewers converge on. That choice, the `click`/`type`
navigation-completion semantics, and the `links` canonicalisation asymmetry are the
only items that need a decision; the rest is mechanical polish.

## Approval (Pass 3) — 2026-08-31

**Verdict:** APPROVE

The pass-2 findings were addressed in a follow-up revision. Phase 4 moved to a
persistent page-lifetime `page.route` handler reading a per-command
`currentAllowances` (deny-all default) with a per-command `lastRefusal` reset,
closing the enforcement-envelope gap for out-of-window and page-initiated
navigations while retaining per-request allowance scoping; the handler fails closed
on a classifier throw; `click`/`type` surface refusals via a post-action
`lastRefusal` check rather than silently; the `links` canonicalisation asymmetry is
documented; every wire token (`malformed`, `unspecified`) is bound end-to-end
through `classifyUrl` in the corpus; the case-count floor, error-token vocabulary,
opt-in-lane CI boundary, changelog scope, and grep phrase are pinned; and the
unhandled-reach branch maps to a documented token.

Two residuals remain accepted by decision: subresource SSRF and DNS rebinding stay
out of scope (documented, not closed). One reviewer suggestion is deferred, not
applied: a differential/property fuzz check across the Rust and JS classifiers — the
enumerated corpus plus the per-variant completeness assertion is the agreed
anti-drift mechanism. Approved for implementation on that basis.
