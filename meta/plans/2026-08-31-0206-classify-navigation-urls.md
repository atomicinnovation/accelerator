---
type: "plan"
id: "2026-08-31-0206-classify-navigation-urls"
title: "Classify Navigation URLs, Not Only The Initial Location Implementation Plan"
date: "2026-08-31T20:43:02+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0206"
parent: "work-item:0206"
derived_from: ["codebase-research:2026-08-31-0206-classify-navigation-urls"]
tags: ["design", "security", "ssrf", "playwright", "executor", "access-policy"]
revision: "9c17233ee9f3c56d191c9323ceef4daa0658137e"
repository: "accelerator"
last_updated: "2026-08-31T22:45:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Classify Navigation URLs, Not Only The Initial Location Implementation Plan

## Overview

Carry the `AccessPolicy` verdict — reachability and scheme together — from the
`validate-source` front door down to per-request `navigate` and `links`
classification in the design crawl. Today the verdict is enforced once, at the
location a crawl starts from; every navigation, redirect hop and followed link
after that is unclassified, so an attacker-influenced page or a redirect can
steer a crawl at an internal endpoint the front door would refuse.

The classifier that must run per request lives in the Node daemon, where the
Rust domain does not exist. Per the design decision on this plan, the classifier
is **reimplemented in JavaScript** rather than bound or shelled out to. That
introduces a second source of truth for security-critical logic, so the plan's
spine is a single **language-neutral vector corpus** that both the Rust
classifier and the new JS classifier are tested against — either side drifting
fails CI.

## Current State Analysis

The verdict itself is solved and well-tested. `access_policy::evaluate`
(`cli/design/src/access_policy.rs:26`) combines reachability and scheme over an
injected `Host` and `Allowances`; `host_reach::classify`
(`cli/design/src/host_reach.rs:71`) dispatches on a canonicalised `Host`
(`cli/design/src/host.rs:72`) that already closes decimal/hex/octal numeric
encodings, IPv6 transition forms (6to4, Teredo, NAT64) and IPv4-mapped
addresses. All of it is pure Rust with no JS binding.

The carrier is the whole job, and three structural facts shape it.

- **The executor is a per-call `execve`, not a request loop.** `ExecClient::run`
  (`cli/design-adapters/src/process.rs:283`) ends in `command.exec()`, replacing
  the Rust process with `node run.js <cmd> <args>`. `Command::Executor`
  (`cli/design-cli/src/cli.rs:66`) carries no allowance flags. The per-request
  channel is the JSON body merged at `lib/client.js:52`; env vars are
  daemon-lifetime and would leak an allowance into a reused daemon.
- **Redirect enforcement can only run in JS.** `page.goto`
  (`lib/daemon.js:205`) follows redirects inside Chromium; Rust never sees the
  hops. The only abort-before-fetch point is a Playwright `page.route` handler.
  ⚠️ No route handler is active today — `makeAuthHeaderHandler` is imported
  (`lib/daemon.js:13`) but never installed, and no `route.abort()` exists
  anywhere.
- **The acceptance-criteria `details` vocabulary does not exist.**
  `evaluate` returns `Verdict<String>` — a human sentence — and
  `HostReach::description()` (`host_reach.rs:47`) emits `RFC1918`, not `private`.
  The machine tokens the AC names (`private`, `link-local`, `reserved`,
  `insecure-scheme`) are greenfield.

The consumer call sites are the two browser agents, not only the skill:
`agents/browser-analyser.md:43` (navigate) and `agents/browser-locator.md:64`
and `:76` (navigate, links) issue executor calls with no allowances;
`skills/design/inventory-design/SKILL.md:56` forwards allowances only to
`validate-source`.

## Desired End State

Every `navigate` request (its initial URL and every redirect hop) and every
`links` destination is judged by the same reachability + scheme verdict that
guards the initial location, under the allowances the invocation carried. A
refused `navigate` returns a `navigation-refused` error envelope
(`retryable: false`) whose `details.classification` names the reach or scheme
class, and the refused URL is never loaded. `links` reports `same_origin: false`
for a destination the policy would refuse. The Rust and JS classifiers are held
identical by a shared vector corpus.

Verify by running `mise run` to green, then the manual redirect and reuse
checks in the Testing Strategy.

### Key Discoveries

- The JS unit lane already globs `lib/*.test.js` via `node --test`
  (`tasks/test/unit.py:119`), run by `mise run test:unit:design-automation` — a
  new `lib/access-policy.test.js` joins it, and the `_EXPECTED_DESIGN_AUTOMATION_SUITES`
  floor rises from 9 to 10 so a later deletion of the suite trips the guard.
- `makeError` (`lib/errors.js:6`) takes an optional object-shaped `details`;
  `wall-clock-exceeded` (`daemon.js:109`) is a live `retryable: false` + details
  precedent on the wire.
- The daemon handles one request per connection sequentially
  (`PROTOCOL.md:16`), so a daemon-scoped "current allowances" and "last refusal"
  variable set per request is safe against interleaving.
- `looks_numeric` / `is_numeric_label` (`host.rs:131`) and the Teredo
  bitwise-inversion unwrap (`host_reach.rs:152`) are the subtle branches most at
  risk of JS drift; the corpus must exercise each.
- `FORWARDABLE_COMMANDS` (`cli/design/src/executor/forwardable.rs:20`) and
  `daemon.test.js` hold the Rust and JS command sets equal; adding request
  fields (not commands) needs no change there.

## What We're NOT Doing

- **Subresource SSRF.** The route handler intercepts only navigation/document
  requests (`route.request().isNavigationRequest()`), not `<img>`, `<script>`,
  XHR or other subresources. The story scopes to "every navigation and every
  followed link"; a subresource is neither, and classifying every subresource
  would abort legitimate page assets and balloon per-request cost. ⚠️ Under this
  story's adversarial model a hostile page can still read internal/metadata
  responses via a subresource into the agent-visible DOM; Phase 6 documents this
  residual plainly rather than framing navigation coverage as closing SSRF.
- **DNS rebinding.** Classification stays pre-resolution; a public hostname that
  resolves to an internal address is out of scope, the same residual limit
  `validate-source` already carries. ⚠️ The residual bites harder here than for
  `validate-source`: the navigation target is attacker-influenced, not
  operator-supplied, so an attacker controlling a hostname's DNS bypasses
  pre-resolution classification. Phase 6 states this plainly.
- **0209's auth-header stripping.** This plan installs the route seam 0209
  extends; it does not strip cross-origin auth headers.
- **A new `links` protocol concept.** A policy-refused destination folds into the
  existing `same_origin: false` skip rather than adding a wire signal.
- **A Rust runtime classification token.** `validate-source` keeps its existing
  `RFC1918` stderr sentence; the machine token is a JS-side and test-fixture
  concern only.

## Implementation Approach

Build the shared authority first (Phase 1), port the JS classifier against it
(Phase 2), then plumb allowances end-to-end but inert (Phase 3) so that when
enforcement turns on (Phases 4–5) the allowances already flow and no
intermediate merge refuses a legitimate `--allow-internal` crawl. Finish with
docs (Phase 6). Every phase is red-green-refactor and leaves `mise run` green.

---

## Phase 1: Shared classification vector corpus

### Overview

Extract the reach and access-policy test vectors currently inlined in the Rust
tests into one language-neutral JSON fixture, rewrite the Rust tests to consume
it, and pin the machine-token vocabulary the JS side will emit. Behaviour is
unchanged; this is the anti-drift keystone every later phase leans on.

### Changes Required

#### 1. The corpus fixture

**File**: `cli/design/tests/fixtures/host-classification-vectors.json` (new)
**Changes**: One authoritative list of cases. Each case carries the raw
authority or URL, the allowances, and the expected outcome expressed in the
shared token vocabulary. Home it in the domain crate; the JS test resolves it by
repo-relative path.

```json
{
  "reach": [
    { "authority": "127.0.0.1", "reach": "loopback" },
    { "authority": "169.254.169.254", "reach": "link-local" },
    { "authority": "[2001:0:0:0:0:0:5601:5601]", "reach": "link-local" },
    { "authority": "0x7f000001", "error": "numeric-encoding" }
  ],
  "policy": [
    { "url": "https://example.com", "allow_internal": false,
      "allow_insecure_scheme": false, "verdict": "accepted" },
    { "url": "http://example.com", "allow_internal": false,
      "allow_insecure_scheme": false, "verdict": "rejected",
      "classification": "insecure-scheme" },
    { "url": "http://10.0.0.1", "allow_internal": false,
      "allow_insecure_scheme": false, "verdict": "rejected",
      "classification": "private" },
    { "url": "http://0.0.0.0", "allow_internal": true,
      "allow_insecure_scheme": true, "verdict": "rejected",
      "classification": "unspecified" },
    { "url": "http://user@example.com", "allow_internal": false,
      "allow_insecure_scheme": false, "verdict": "rejected",
      "classification": "malformed" }
  ]
}
```

The `reach` cases mirror `host_reach.rs` tests
(`each_headline_address_classifies_as_its_own_reach`,
`every_indirect_encoding_classifies_internally`, the loopback and boundary
sets); the `error` cases mirror `host.rs`
(`every_numeric_encoding_that_is_not_an_address_is_rejected`, userinfo, control
character); the `policy` cases mirror `access_policy.rs`. Three token sets are
closed vocabularies. Reach: `loopback`, `private`, `link-local`, `reserved`,
`unspecified`, `public`. Error (asserted against `canonicalise` /
`Host::canonicalise`): `numeric-encoding`, `userinfo`, `control-character`. Policy
`classification`: `private`, `link-local`, `reserved`, `unspecified`,
`insecure-scheme`, `malformed`. Every error-layer token folds to the single
`malformed` policy token on the wire — `malformed` is a JS-side label with no Rust
`evaluate` producer, so a `policy` case asserts `classifyUrl` emits `malformed`
(a userinfo URL) and `unspecified` (`http://0.0.0.0`) directly, binding every wire
token at the entry point the daemon consumes.

#### 2. Rust tests consume the fixture

**File**: `cli/design/src/host_reach.rs`, `cli/design/src/host.rs`,
`cli/design/src/access_policy.rs`
**Changes**: Replace the inlined vector loops with a loader that reads the
fixture (via `CARGO_MANIFEST_DIR`) and asserts `classify` / `canonicalise` /
`evaluate` agree with each case. Keep the pinned prose tests
(`the_rfc1918_rejection_names_the_reach_and_the_recovering_flag`) as they are —
they guard the human sentence, which does not change.

A test-only reach→token mapping validates the shared vocabulary on the Rust
side, so a fixture token the Rust classifier could not produce fails here rather
than silently diverging from JS.

```rust
fn token(reach: HostReach) -> &'static str {
    match reach {
        HostReach::Loopback => "loopback",
        HostReach::Private => "private",
        HostReach::LinkLocal => "link-local",
        HostReach::Reserved => "reserved",
        HostReach::Unspecified => "unspecified",
        HostReach::Public => "public",
    }
}
```

### Success Criteria

#### Automated Verification

- [x] Rust suite passes against the fixture: `mise run cli:check` and
      `cargo test --manifest-path cli/Cargo.toml -p design`
- [x] The fixture parses and every case is exercised (no silently-skipped
      cases): a fixture-count assertion in the loader
- [x] The fixture is complete: a Rust assertion that every `HostReach` variant,
      every `embedded_v4` transition form (6to4, Teredo, NAT64, IPv4-mapped,
      IPv4-compatible), and every `HostError` kind appears in at least one case,
      so a branch dropped during migration fails the loader rather than silently
      losing coverage on both sides
- [ ] Full local mirror green: `mise run` (deferred to the consolidated
      end-to-end run)

#### Manual Verification

- [ ] The fixture reads as the single authority — a reviewer can see every
      encoding class from the story (decimal, hex, octal, IPv6 transition,
      IPv4-mapped) present as a named case

---

## Phase 2: JavaScript classifier ported from the corpus

### Overview

Port host canonicalisation, reachability and scheme classification into the
daemon's lib as a pure, unwired module, driven test-first against the **same**
fixture. No daemon behaviour changes yet.

### Changes Required

#### 1. The JS classifier module

**File**: `skills/design/inventory-design/scripts/playwright/lib/access-policy.js`
(new)
**Changes**: A pure module exporting a top-level `classifyUrl` (URL string +
`{ allowInternal, allowInsecureScheme }` → `{ ok: true }` or
`{ ok: false, classification }`) plus the lower-level `canonicalise` and
`classifyHost` it composes, so the corpus asserts the reach and error layers
directly rather than only through the policy verdict. `classification` is one
of the shared tokens. It reproduces `Host::canonicalise` (percent-decode,
userinfo rejection, control-character rejection, port/bracket/zone stripping,
numeric-encoding rejection), `host_reach::classify` (including the IPv6
transition unwraps and Teredo bitwise inversion), and `access_policy::evaluate`
(loopback accept, unspecified reject, internal gate, public-http scheme gate,
reachability-before-scheme ordering).

```javascript
export function classifyUrl(rawUrl, allowances) {
  const { allowInternal = false, allowInsecureScheme = false } = allowances ?? {};
  const parsed = canonicalise(rawUrl);
  if (!parsed.ok) return { ok: false, classification: 'malformed' };
  const reach = classifyHost(parsed.host);
  if (reach === 'loopback') return { ok: true };
  if (reach === 'unspecified') return { ok: false, classification: 'unspecified' };
  if (reach === 'private' || reach === 'link-local' || reach === 'reserved') {
    return allowInternal ? { ok: true } : { ok: false, classification: reach };
  }
  if (reach !== 'public') return { ok: false, classification: 'malformed' };
  if (parsed.scheme === 'http' && !allowInsecureScheme) {
    return { ok: false, classification: 'insecure-scheme' };
  }
  return { ok: true };
}
```

The classifier fails closed on every ambiguous input: missing allowances default
to deny, a canonicalisation failure returns `malformed`, and any reach class the
policy does not explicitly handle (a future reach beyond the known set) is refused
as the documented `malformed` default rather than emitting an off-contract token or
falling through to accept. The daemon never dials a URL the classifier could not
judge.

#### 2. The JS classifier test

**File**:
`skills/design/inventory-design/scripts/playwright/lib/access-policy.test.js` (new)
**Changes**: Load the Phase 1 fixture by repo-relative path and assert the
`reach` and `error` cases against the exported `canonicalise`/`classifyHost` (so
the subtle reach and parse-error branches bind at their own layer, not collapsed
into a policy verdict) and the `policy` cases against `classifyUrl`. Fail if the
fixture case count is zero, so a mis-resolved path cannot pass vacuously. Iterate
and assert without early `return`s — `_bare_returns_in_tests` (`tasks/test/unit.py`)
forbids guard-clause returns in test bodies.

### Success Criteria

#### Automated Verification

- [x] JS unit lane passes: `mise run test:unit:design-automation`
- [x] The fixture is found and non-empty (path-resolution guard asserts a
      positive case count)
- [x] The suite floor is raised: `_EXPECTED_DESIGN_AUTOMATION_SUITES` in
      `tasks/test/unit.py` is 10, not 9, and `_EXPECTED_DESIGN_AUTOMATION_CASES` is
      raised by the new suite's case count so the corpus cases enjoy the same
      at-least floor
- [x] Fail-closed paths hold: a missing-allowances call and an unhandled-reach
      input each return a refusal, not an allow
- [x] Every wire token is bound end-to-end through `classifyUrl`: policy cases
      assert `malformed` (a userinfo URL) and `unspecified` (`http://0.0.0.0`), not
      only the reach/error layers
- [x] Lint and format clean: `mise run check`

#### Manual Verification

- [x] A deliberate one-character edit to the JS Teredo unwrap turns a corpus
      case red, confirming the fixture actually binds the two implementations

---

## Phase 3: Allowances plumbed end-to-end, inert

### Overview

Add the two allowance flags to the executor command, inject them into the
forwarded per-request JSON body, and have both browser agents forward them on
every `navigate` and `links` call, with the skill passing the operator's
allowances into the agent prompts. The daemon receives the fields and ignores
them; no behaviour changes.

### Changes Required

#### 1. Executor command flags

**File**: `cli/design-cli/src/cli.rs`
**Changes**: Add `allow_internal` and `allow_insecure_scheme` bool flags to
`Command::Executor`. On the command line the flags must precede `command`,
because `allow_hyphen_values` on the trailing `arguments` makes any
hyphen-leading token after `command` a literal argument — so clap parses
`executor --allow-internal navigate '{...}'` as flag-then-command-then-trailing.
This is a deliberate departure from `ValidateSource` (which declares its
positional first); the trailing-var-arg constraint, not struct field order,
forces the flip.

```rust
Executor {
    #[arg(long)]
    allow_internal: bool,
    #[arg(long)]
    allow_insecure_scheme: bool,
    command: String,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    arguments: Vec<String>,
},
```

#### 2. Injection into the forwarded request

**File**: `cli/design-cli/src/main.rs`, `cli/design-cli/src/executor.rs`
**Changes**: `main` passes the two flags into `executor::run`. Before the
verbatim forward (`executor.rs:427`), merge the allowances into the request's
JSON argument: parse `arguments[0]` as an object (synthesising `{}` when
absent), set `allow_internal` and `allow_insecure_scheme`, re-serialise, and
forward that. The flags become fields on the per-request body the daemon reads.

```rust
pub fn run(
    command: &str,
    arguments: &[String],
    allowances: Allowances,
) -> ExitCode {
```

The merge refuses a payload that already carries either allowance key, matching
the `command`/`protocol` guard in `client.js:46`, so a page-influenced payload
cannot pre-set its own allowance.

#### 3. Agents forward the allowances

**File**: `agents/browser-analyser.md`, `agents/browser-locator.md`
**Changes**: Show every `navigate` (and, for the locator, `links`) invocation
carrying the operator's allowance flags, and add a one-line instruction that the
agent forwards exactly the allowances it was given on each such call, never
inventing them.

```text
accelerator design executor {allow-flags} navigate '{"url":"<url>"}'
accelerator design executor {allow-flags} links
```

#### 4. Skill passes allowances to the agents

**File**: `skills/design/inventory-design/SKILL.md`
**Changes**: Where the skill spawns the browser agents, inject the same
`--allow-internal` / `--allow-insecure-scheme` values it forwarded to
`validate-source` (`SKILL.md:56`) into the agent prompt, so the agents forward
consistent allowances.

### Success Criteria

#### Automated Verification

- [x] Clap accepts `executor --allow-internal --allow-insecure-scheme navigate
      '{...}'` and rejects an unknown flag: new tests in `cli.rs`
- [x] A flag placed after the JSON body is captured as a trailing argument, not
      the flag, pinning the flags-then-command-then-trailing contract: new test
      in `cli.rs`. (Correction to the plan: clap only begins trailing collection
      once the first `arguments` value appears, so a hyphen token immediately
      after a bare `command` is still parsed as the flag; the flags must precede
      `command`, which is how the agents emit them. A page influences only the
      body, never argv, so this quirk grants no allowance.)
- [x] The injected body carries both allowance keys, and a payload pre-setting
      one is refused: new tests in `executor.rs`
- [ ] Rust and JS suites green: `mise run` (deferred to the consolidated
      end-to-end run)

#### Manual Verification

- [ ] `accelerator design executor --allow-internal navigate
      '{"url":"http://localhost:3000"}'` still succeeds (loopback), confirming
      the inert plumbing changed no outcome (needs a live runtime + app)
- [x] The agent templates read naturally — an operator can see allowances are
      forwarded, not fixed

---

## Phase 4: `navigate` enforcement via a route handler

### Overview

Install a single `page.route` handler on the page for its whole lifetime, reading
a request-scoped `currentAllowances` that every command sets at entry (deny-all
default) and a `lastRefusal` every command clears. The handler classifies each
main-frame navigation/document request (initial and every redirect hop) under the
current request's allowances, aborts a refused one before its fetch via
`route.abort`, records the refusal, and passes an allowed request on with
`route.fallback()` so a later handler (0209's header strip) can still act. Because
the handler is always installed, a page-initiated navigation — a `<meta refresh>`,
a `setTimeout(() => location = ...)`, or a redirect after `domcontentloaded` — is
classified under the current command's allowances too, not left unguarded. The
`navigate` handler (and any command whose action can trigger a navigation) maps a
recorded refusal to a `navigation-refused` envelope. Setting `currentAllowances`
per request with a deny-all default means a warm daemon never judges one
invocation's navigation under another's allowances; the page-scoped
`currentAllowances`/`lastRefusal` are safe against interleaving only because the
daemon processes one request at a time (`PROTOCOL.md:16`) — a single-flight
invariant the handler depends on, called out at its declaration.

### Changes Required

#### 1. The persistent route handler and its pure decision

**Files**: `skills/design/inventory-design/scripts/playwright/lib/daemon.js`,
`skills/design/inventory-design/scripts/playwright/lib/access-policy.js`
**Changes**: Extract a pure `classifyNavigationRequest(request, allowances)` (in
`access-policy.js`, over `classifyUrl`) that returns `{ continue: true }` for a
non-navigation request and for an allowed navigation, or
`{ abort: true, classification, url }` for a refused one — unit-tested directly
against a fake `route.request()`, no browser needed.

In `ensureBrowser` (`daemon.js:132`), after the page exists, install one
`page.route('**/*', handler)` for the page's lifetime. The handler classifies
under the page-scoped `currentAllowances`, records a refusal only for
`request.frame() === page.mainFrame()` (so a sub-frame abort never masks the
top-level result), and uses `route.fallback()` (not `route.continue()`) on the
allow path so a separately registered 0209 header handler still fires — the
`fallback()` choice composes with 0209 whether it extends this handler in place or
registers its own. The body is wrapped so any thrown classifier error fails closed
to `route.abort` with a `malformed` refusal rather than leaving the request
unhandled (which would hang it until the wall-clock backstop).

```javascript
await page.route('**/*', async route => {
  let decision;
  try {
    decision = classifyNavigationRequest(route.request(), currentAllowances);
  } catch {
    decision = { abort: true, classification: 'malformed', url: route.request().url() };
  }
  if (decision.continue) return route.fallback();
  if (route.request().frame() === page.mainFrame()) lastRefusal = decision;
  return route.abort('blockedbyclient');
});
```

#### 2. Per-request allowances and the refusal envelope

**File**: `skills/design/inventory-design/scripts/playwright/lib/daemon.js`
**Changes**: At the top of request dispatch, for **every** command, set
`currentAllowances = allowancesOf(req)` (deny-all default) and clear
`lastRefusal = null`, so each request is judged under its own allowances and no
prior refusal bleeds through. The `navigate` case runs `page.goto` and, whether it
throws or resolves, maps a recorded `lastRefusal` to the envelope via a shared
`navigationRefusedEnvelope(refusal)` helper — deriving the outcome from the
recorded refusal, not from whether `goto` threw. Any other command whose action can
trigger a navigation (`click`, `type`, `wait_for`) checks `lastRefusal` after its
action the same way. The message names host plus pathname only via a defensive
`hostAndPath` (falling back to a redacted placeholder on an unparseable `malformed`
URL), so a token-bearing query or fragment on the refused hop never leaks —
consistent with the `href`/`links` stripping (`PROTOCOL.md:269`).

```javascript
// top of dispatch, every command:
currentAllowances = allowancesOf(req);
lastRefusal = null;

case 'navigate': {
  if (!req.url) return makeError({ error: 'missing-url', message: 'navigate requires a "url" field', category: 'usage', retryable: false });
  try {
    await page.goto(req.url, { waitUntil: 'domcontentloaded', timeout: WALL_CLOCK_MS });
  } catch (error) {
    if (lastRefusal) return navigationRefusedEnvelope(lastRefusal);
    throw error;
  }
  if (lastRefusal) return navigationRefusedEnvelope(lastRefusal);
  return { protocol: PROTOCOL, ok: true, url: page.url() };
}
```

`route.abort` fires before the network fetch, so the refused URL is never loaded.
Because the handler is always installed and reads the current request's
allowances, a redirect after `domcontentloaded` and a page-initiated navigation are
classified too; a `click`/`type`-triggered navigation is aborted at the network
layer and surfaced as `navigation-refused` by the post-action `lastRefusal` check,
not silently — `page.click` resolves before the navigation settles, so the check,
not the action's return, is authoritative.

### Success Criteria

#### Automated Verification

- [ ] `classifyNavigationRequest` unit tests: for each refusal class in the corpus
      a fake navigation request returns `abort` with the documented classification;
      an allowed host and every non-navigation request return `continue`: new
      `access-policy.test.js` cases
- [ ] The handler fails closed on a thrown classifier error — a stubbed
      `classifyNavigationRequest` that throws yields `route.abort`, never an
      unhandled (hung) request: new handler-level case
- [ ] Sub-frame isolation: an iframe navigation to an internal host aborts but does
      not become the `navigate` result — a top-level goto failing for an unrelated
      reason after an iframe abort surfaces the real failure, not
      `navigation-refused` (the main-frame gate lives in the handler, not the pure
      decision, so this is a `daemon-runtime.test.js` case)
- [ ] A refused `navigate` returns `navigation-refused` with
      `details.classification` in `{private, link-local, reserved, unspecified,
      insecure-scheme, malformed}`, `retryable: false`, and a message carrying host
      plus pathname only (no query/fragment; `malformed` falls back to a redacted
      placeholder)
- [ ] Out-of-window coverage: a page that issues a `<meta refresh>` /
      `setTimeout(location=...)` redirect to an internal host after
      `domcontentloaded` is still aborted, and a `click`-triggered navigation to an
      internal host returns `navigation-refused` (not a silent `ok: true`): new
      `daemon-runtime.test.js` cases
- [ ] Per-request scope: two sequential navigations on one daemon with differing
      allowances are each judged under their own — the second, without
      `--allow-internal`, refuses an internal host the first allowed: new
      `daemon-runtime.test.js` case
- [ ] Redirect hop: a real 302 from a public host to `http://169.254.169.254/`
      never issues the internal request: new `daemon-runtime.test.js` case over a
      forked daemon and a local redirecting server (proves Playwright re-invokes the
      handler per hop — a mocked route cannot)
- [ ] Positive path: with `allow_internal`, an internal host continues and
      `navigate` succeeds
- [ ] `mise run` green covers the unit lane (`test:unit:design-automation`). ⚠️ The
      `daemon-runtime.test.js` cases above run only in the opt-in
      `test:integration:design-automation` lane, which `mise run` does **not**
      execute — run them explicitly, and gate a CI job on the provisioned Playwright
      runtime so at least the redirect and out-of-window cases run on merge

#### Manual Verification

- [ ] Against a local server that 302-redirects to `http://169.254.169.254/`,
      `navigate` returns `navigation-refused` and the metadata endpoint is never
      contacted (observed via the server/network, not the returned URL)
- [ ] A daemon reused by a later invocation that omits `--allow-internal`
      refuses an internal `navigate`, confirming per-request scope

---

## Phase 5: `links` enforcement folded into `same_origin`

### Overview

Classify each `links` destination under the current allowances and report
`same_origin: false` for any the policy would refuse, computing the verdict in
Node from the host while never returning the host to the agent.

### Changes Required

#### 1. Return the host to Node, classify, strip

**Files**: `skills/design/inventory-design/scripts/playwright/lib/daemon.js`,
`skills/design/inventory-design/scripts/playwright/lib/access-policy.js`
**Changes**: In the `links` case (`daemon.js:214`), have `page.evaluate` include
the resolved host (and the raw same-origin result) per anchor. In Node, read the
request's own allowances (`allowancesOf(req)`, deny-all default) and classify each
anchor with `classifyLocation({ scheme, host }, allowances)` — the core
`classifyUrl` also wraps, applying the same reach (`classifyHost`) and
scheme/allowance gate. Compute `same_origin` as same-origin **and** policy-allowed,
then delete the host before returning.

`links` judges the **browser-resolved** host, whereas `navigate` canonicalises the
raw URL pre-resolution — a deliberate, documented difference (Phase 6 §3). The
browser has already produced a concrete host for an anchor, so the raw-URL
canonicalisation rejections (numeric-encoding, userinfo, control-character) that
yield `malformed` on the `navigate` path cannot arise for a `links` destination;
both paths refuse the same internal reach classes. The full host stays in Node;
only `pathname`, `same_origin`, `scheme`, `text`, `role` reach the caller,
unchanged from the current wire shape.

```javascript
const allowances = allowancesOf(req);
const links = raw.map(a => {
  const policyAllows = !a.host || classifyLocation(a, allowances).ok;
  const { host, sameOriginRaw, ...rest } = a;
  return { ...rest, same_origin: sameOriginRaw && policyAllows };
});
```

Only host authorities (never query strings or fragments) cross into Node, so no
token-bearing URL component leaves the browser — consistent with why `href` is
stripped (`PROTOCOL.md:269`).

### Success Criteria

#### Automated Verification

- [ ] A policy-refused internal destination reports `same_origin: false`; a
      genuinely same-origin public destination still reports `true`: new
      `daemon.test.js` links cases
- [ ] The response carries no `host` field for any anchor (host never leaks):
      assertion over the returned links
- [ ] `mise run` green

#### Manual Verification

- [ ] On a page whose same-origin anchors point at an internal host without
      `--allow-internal`, the locator receives zero followable candidates for
      those anchors

---

## Phase 6: Documentation and protocol reconciliation

### Overview

Remove the "front door, not a boundary" / "only the initial location" wording
from the module docs and the design page, state that classification now applies
to every `navigate` and `links` request, and document the `navigation-refused`
envelope and its branchable `details.classification`.

### Changes Required

#### 1. Module docs

**File**: `cli/design/src/host_reach.rs`, `cli/design/src/access_policy.rs`
**Changes**: In the `host_reach.rs` module doc (`:9`–`:13`), drop the "covers
only the initial location" and "front door, not a boundary" sentences; keep the
pre-resolution and path-location limits. Replace them with a precise scope
statement — the same verdict is now applied to every `navigate` (initial and
redirect hops) and every `links` destination in the daemon, but classification
stays pre-resolution and does not cover subresources — so removing the "not a
boundary" hedge does not read as "the navigation surface is now fully bounded".
`access_policy.rs`'s module doc carries no "front door" wording to remove, so it
gets the equivalent statement as an addition only.

#### 2. Design page

**File**: `docs-site/src/content/docs/design.md`
**Changes**: Rewrite the "What this check does not cover" passage (`:96`–`:102`):
remove "It is the front door, not a boundary" and "It covers only the initial
location", keep the pre-resolution/DNS caveat and the path-location caveat, and
add that `navigate` and `links` are now classified per request. State plainly
that two residuals remain open — a hostile hostname that resolves to an internal
address (DNS rebinding) still reaches it, and page subresources (`<img>`,
`<script>`, XHR/`fetch`) to internal hosts are not classified, so a hostile page
can still read internal/metadata responses into the DOM — so the reader does not
mistake per-request navigation coverage for end-to-end SSRF safety.

#### 3. Protocol reference

**File**: `skills/design/inventory-design/PROTOCOL.md`
**Changes**:
- Add `navigation-refused` (category `browser`, non-retryable) to the `navigate`
  error table, documenting `details.classification` and enumerating its full value
  set: `private`, `link-local`, `reserved`, `unspecified`, `insecure-scheme`,
  `malformed`. State whether the set is closed or may gain values in a future
  additive change, and instruct a branching consumer to treat an unknown value as
  a refusal (default-deny), so the carve-out does not re-create the fragility the
  general "don't branch" rule prevents.
- Reconcile the blanket "callers SHOULD NOT branch on `details`" note (`:661`) by
  carving out `details.classification` on `navigation-refused` as a documented,
  branchable field.
- Restate the `same_origin` field definition (`:265`) — not a footnote — to mean
  "the resolved origin matches the page origin **and** the destination is not
  policy-refused", so no future consumer relies on the old meaning; note the
  overload for policy-refused `links` destinations in the `links` section.
- Document `allow_internal` and `allow_insecure_scheme` as optional per-request
  fields on the `navigate` and `links` request envelopes (default false, injected
  by the executor), so a future caller can reproduce a correctly-classified
  request from the reference alone.
- Note that `links` classifies the browser-resolved host while `navigate`
  canonicalises the raw URL pre-resolution, so the numeric-encoding / userinfo /
  control-character rejections (which yield `malformed` on `navigate`) do not
  apply to `links`; both refuse the same internal reach classes.

#### 4. Agent-facing refusal handling

**File**: `agents/browser-analyser.md`, `agents/browser-locator.md`
**Changes**: Add a short instruction that `navigation-refused` is a non-retryable
policy refusal, not a transient failure: the agent records the refused
destination as an inspected-with-gap result and continues the crawl rather than
retrying or aborting, matching the work item's "stop or continue with a reported
gap" decision.

#### 5. Changelog

**File**: `CHANGELOG.md`
**Changes**: Add an entry under `[Unreleased]`. Under `Changed`: a `navigate` or
redirect hop to an internal/insecure destination may now be refused where it
previously succeeded, and `links` now reports `same_origin: false` for a
policy-refused destination under the restated field semantics. Under `Security`:
per-request classification of navigations and followed links, scoped explicitly to
navigations and redirects — naming subresource SSRF and DNS rebinding as known
residuals, not implying end-to-end SSRF closure. This replaces the Migration Notes'
imprecise "changelog entry under the design skill"; there is no per-skill
changelog, only the root `CHANGELOG.md`.

### Success Criteria

#### Automated Verification

- [ ] No "front door" / "only the initial location" wording remains: `grep -rn
      "front door\|only the initial location" cli/design/src docs-site/src
      skills/design/inventory-design/PROTOCOL.md` returns nothing
- [ ] The positive statements are present: `navigation-refused` appears in
      `PROTOCOL.md`, and each of `host_reach.rs`, `access_policy.rs`, and
      `design.md` contains the pinned substring `every navigate` (the shared phrase
      asserting per-request `navigate`/`links` classification), so the three files
      carry the same statement rather than divergent wordings
- [ ] The `CHANGELOG.md` `[Unreleased]` section carries the behaviour-change entry
- [ ] Docs build and the full mirror pass: `mise run` (and `mise run docs:check`
      when touching `docs-site/`)

#### Manual Verification

- [ ] Each of `host_reach.rs`, `access_policy.rs` and `design.md` states the scope
      precisely — per-request `navigate`/`links` classification, still
      pre-resolution, not covering subresources — without reading as "fully
      bounded"
- [ ] `PROTOCOL.md` documents `navigation-refused`, the full classification value
      set, the restated `same_origin` meaning, and the allowance request fields
      without contradicting the general details caveat
- [ ] Both browser agent docs tell the agent how to react to `navigation-refused`

---

## Testing Strategy

### Unit Tests

- The shared corpus is the primary unit surface: Rust (`host.rs`, `host_reach.rs`,
  `access_policy.rs`) and JS (`access-policy.test.js`) both assert against it — the
  reach and error cases against the exported `canonicalise`/`classifyHost`, the
  policy cases against `classifyUrl` — so every encoding class is covered on both
  sides by construction. A Rust completeness assertion forces a corpus case per
  `HostReach` variant, `embedded_v4` form and `HostError` kind. The test iterates
  and asserts without early `return`s (the `_bare_returns_in_tests` guard).
- `classifyNavigationRequest` in `access-policy.test.js` with a fake
  `route.request()`: abort on each refusal class (incl. `malformed` and
  `unspecified` bound end-to-end through `classifyUrl`), continue on allow and on
  every non-navigation request. The `navigation-refused` envelope shape, sub-frame
  isolation, and the handler failing closed on a thrown classifier error are
  asserted at the daemon-runtime level, since the main-frame gate and the
  persistent handler live in `daemon.js`, not the pure decision.
- `links` folding (`classifyLocation`) in `access-policy.test.js`: refusal →
  `same_origin: false`, allowed same-origin unchanged, host never present in the
  response.
- Executor plumbing in `cli.rs` / `executor.rs`: flag parsing (including a flag
  after `command` captured as a trailing argument), JSON injection,
  payload-pre-set refusal.

### Integration Tests

- Several cases added to `daemon-runtime.test.js` (the opt-in
  `test:integration:design-automation` lane, `tasks/test/integration.py`), which
  forks a real daemon over HTTP: a 302 from a public host to an internal address
  that must never issue the internal request (proving Playwright re-invokes the
  handler per redirect hop — a mocked route cannot); a `<meta refresh>` /
  `setTimeout(location=...)` redirect after `domcontentloaded` and a
  `click`-triggered internal navigation, both refused; sub-frame isolation; the
  handler failing closed on a thrown classifier error; and two sequential
  navigations with differing allowances proving per-request scope. ⚠️ This lane is
  outside `mise run` — gate a CI job on the provisioned Playwright runtime so at
  least the redirect and out-of-window cases run on merge. The pure
  `classifyNavigationRequest`/`classifyLocation` decisions are unit-tested against
  the corpus in `access-policy.test.js` without a browser.

### Manual Testing Steps

1. Start a local server that returns `302` to `http://169.254.169.254/`;
   `navigate` to it without flags and confirm `navigation-refused` and that the
   metadata endpoint is never contacted.
2. Repeat with `--allow-internal` against a private host and confirm the
   positive path loads.
3. Navigate a page whose same-origin anchors point at an internal host; confirm
   `links` reports those as `same_origin: false` without the flag and `true`
   with it.
4. Reuse one daemon across two invocations with differing flags; confirm the
   second invocation's allowances, not the first's, decide each request.

## Performance Considerations

Classification is in-process string work per navigation request and per anchor —
negligible against browser navigation and `page.evaluate` costs. The route
handler runs on every request but exits immediately for non-navigation ones
(`isNavigationRequest()` guard), so subresource-heavy pages pay only a boolean
check per asset.

## Migration Notes

This is a behaviour change, not a data migration: a redirect that previously
succeeded may now be refused. It wants a `CHANGELOG.md` `[Unreleased]` entry
(Security + Changed), added in Phase 6 §5. No stored state changes; the daemon's
on-disk contract is untouched.

## References

- Original work item:
  `meta/work/0206-classify-navigation-urls-not-only-the-initial-location.md`
- Research:
  `meta/research/codebase/2026-08-31-0206-classify-navigation-urls.md`
- Verdict domain: `cli/design/src/access_policy.rs:26`,
  `cli/design/src/host_reach.rs:71`, `cli/design/src/host.rs:72`
- Executor seam: `cli/design-cli/src/executor.rs:427`,
  `cli/design-adapters/src/process.rs:283`, `cli/design-cli/src/cli.rs:66`
- Daemon: `skills/design/inventory-design/scripts/playwright/lib/daemon.js:132`
  (`ensureBrowser`), `:203` (`navigate`), `:214` (`links`)
- Consumers: `agents/browser-analyser.md:43`, `agents/browser-locator.md:64`,
  `skills/design/inventory-design/SKILL.md:56`
- Coordinating story: `meta/work/0209-wire-up-or-retire-the-header-auth-path.md`
