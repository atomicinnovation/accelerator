---
type: "plan"
id: "2026-09-10-0209-wire-up-browser-auth-header-path"
title: "Wire Up The Browser Auth-Header Path In Design Skills Implementation Plan"
date: "2026-09-10T00:43:40+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0209"
parent: "work-item:0209"
derived_from: ["codebase-research:2026-09-10-0209-wire-up-browser-auth-header-path"]
tags: ["design", "security", "playwright", "auth"]
revision: "cd20b7799b9af0f0c9dc3e607fcc42803a696c29"
repository: "accelerator"
last_updated: "2026-09-10T08:45:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Wire Up The Browser Auth-Header Path In Design Skills Implementation Plan

## Overview

Wire the dead browser auth-header injection path so an authenticated design
crawl produces a complete, login-gated inventory. The crawl declares its
`[location]` through `ACCELERATOR_BROWSER_LOCATION` and the bearer through
`ACCELERATOR_BROWSER_AUTH_HEADER`; the JS client threads both into the loopback
request body it POSTs to the daemon — off `argv`, from its own inherited
environment — on every command. The daemon attaches the header only to requests
whose origin matches the declared location origin, stripping it on every
cross-origin one — a code-enforced boundary that a followed cross-origin link or
redirect cannot cross. Both the header and the expected origin are per-request
daemon state, reset each request exactly as `currentAllowances` is, so nothing
persists across a warm daemon's reuse. Header mode requires **both** env vars: the
auth resolver fails loudly when the header is set without the location, so a
missing `[location]` can never silently strip the header into an unauthenticated
inventory. The documentation surfaces that call the path inert are corrected
to match.

## Current State Analysis

The path is doubly dead, exactly as the work item describes. `makeAuthHeaderHandler`
is imported at `daemon.js:13` and never called; its origin input
`ACCELERATOR_BROWSER_LOCATION_ORIGIN` is set nowhere and read only at
`auth-header.js:7`. An authenticated crawl therefore yields a silently
unauthenticated inventory.

Reading the code against the research resolved three points that shape the
implementation:

- **Route ordering in the work item is inverted.** The classifier's allow path
  is `route.fallback()` (`daemon.js:208`); the auth-header handler ends every
  branch in `route.continue()` (`auth-header.js:33,39,43`). Playwright dispatches
  `page.route` handlers last-registered-first, so the auth-header route must
  register **before** the classifier — the opposite of the work item's "register
  after". Registered after, it would `continue()` first and the classifier would
  never run, silently bypassing navigation policy.
- **The origin and header are unknown when the route is installed.** The route
  installs in `ensureBrowser` at page creation (`daemon.js:194`); the first
  `navigate` carrying `[location]` arrives later (`daemon.js:279-285`). The factory
  freezes `expectedOrigin` at construction (`auth-header.js:21`), which cannot
  work; the handler must read both the expected origin and the header live,
  per request.
- **The daemon has no crawl boundary and no invocation identity.** The Rust
  executor `run` is invoked once per command (`executor.rs:101-164`), stateless;
  the crawl loop lives in the browser-locator agent's prose (`browser-locator.md:88-97`),
  which issues each `navigate`/`links`/`snapshot` as a separate executor process
  against the one warm daemon. Nothing in the request body carries a crawl or
  invocation id (`client.js:52`), and the daemon serves an undifferentiated
  command stream, resetting only `currentAllowances`/`lastRefusal` per request
  (`daemon.js:226`). A warm daemon (10-minute reuse) therefore cannot tell one
  crawl from the next — so any auth state pinned across requests would bleed
  between crawls. The location and header are consequently re-declared on every
  forwarded command from the crawl's own configuration and reset per request,
  never held as a cross-request pin.
- **Same-origin scoping is an agent rule, not code.** The daemon's `links`
  command returns every anchor, cross-origin included, tagged `same_origin`
  against the current page's origin (`daemon.js:305,329-332`); the browser-locator
  agent is merely instructed to follow only `same_origin: true`
  (`browser-locator.md:94,133-134`). The daemon obeys any `navigate` it is given,
  so the crawler's link discipline cannot be the boundary: the header is keyed in
  code to the crawl's declared location origin, threaded from
  `ACCELERATOR_BROWSER_LOCATION`, so a cross-origin navigate is stripped
  regardless of what the agent follows.

### Key Discoveries:

- The shared daemon is the sole browser-launch site: `chromium.launch`
  (`daemon.js:182`) is the only production launch; `analyse-design-gaps` has no
  browser code. One wire-up covers every design skill (AC5).
- The classifier passes subresources straight through:
  `classifyNavigationRequest` returns `{continue:true}` for non-navigation
  requests (`access-policy.js:319`), so a cross-origin CDN asset reaches the
  auth-header route via `fallback()` and is stripped without being aborted (AC3).
- The executor's `merge_allowances` (`executor.rs:176-207`) parses the body,
  injects the allowance keys, and refuses a body that pre-sets them — the guard
  pattern the client mirrors for its own `location_url`/`auth_header` fields.
  `merge_allowances` itself is left unchanged: those two fields are env-derived
  and injected client-side, off `argv`.
- Rust cannot cheaply reproduce WHATWG `URL.origin`: `Host::canonicalise` drops
  the port (`host.rs:186`). The origin string is therefore derived in the daemon
  via `new URL().origin`, the same parser used for per-request origins, so the
  two sides compare byte-identically.
- Test lanes: runtime-free JS is `test:unit:design-automation` (floors in
  `tasks/test/unit.py`: ≥10 suites, ≥85 cases, zero skips); the real-Chromium
  lane is `test:integration:design-automation` over the tuple at
  `integration.py:110`, which already lists `daemon-runtime.test.js`.
  `auth-header.test.js` already exists, so no new unit suite is added.
- The leak-scrub rule (`leaked_credentials.rs:25-34`) already splits the bearer
  pair and keeps the value half as a needle; it stays as-is under a live header.

## Desired End State

With `ACCELERATOR_BROWSER_AUTH_HEADER` and `ACCELERATOR_BROWSER_LOCATION` set, the
JS client threads both the location URL and the header into the loopback request
body (never `argv`) on every command; the daemon attaches the header only on
requests whose origin matches the crawl's declared location origin and strips it
on every other origin, so gated pages load while cross-origin assets — and any
cross-origin navigate — load without the header. The
location and header are per-request daemon state re-declared on every command and
reset each request, so a warm daemon reused across crawls governs each crawl by
its own configuration, never by a prior one, and a same-origin subresource that
loads lazily during a following `snapshot`/`links` still carries the header. A
`[location]` that redirects cross-origin keeps the header on the location origin
only, so the header is stripped on the redirect hop. `ACCELERATOR_BROWSER_LOCATION_ORIGIN`
is read nowhere. The inert-path warnings are gone on every surface that carried
them. `mise run` exits 0.

Verification: the runtime acceptance suite proves gated-page load; the
mutation-resistant cross-origin strip (the header present to be stripped, the
recording origin answering the CORS preflight); cross-origin asset load; that a
cross-origin navigate is stripped; the cross-origin redirect leak-guard; a late
same-origin subresource during a following command still authenticated; that a
second crawl of the same origin with no header configured attaches none (no
cross-request bleed); and — the load-bearing safety check — that a refused origin
is still blocked with the auth-header route installed.

## What We're NOT Doing

- **Dual-origin allowlists.** Header mode replaces form login, so a single
  origin keyed to the declared `[location]` is sufficient. Admitting a distinct
  login-URL origin (work item Open Question 1) is a carved-out follow-up, not part
  of this item. The SKILL prose promising "location origin or login URL" is
  reduced to single-origin here.
- **Following the header across a redirect or a cross-origin hop.** The header is
  keyed in code to the declared `[location]` origin, so it never reaches any other
  origin: a `[location]` that redirects cross-origin (http→https, apex→www, an SSO
  bounce) loads the redirected page without the header, and a followed cross-origin
  link is navigated without it. Where a login target redirects, the caller supplies
  the canonical post-redirect origin as `[location]`; the SKILL prose documents the
  behaviour, its consequence, and a worked example, and a leak-guard test verifies
  the strip on the hop. Admitting a redirect target's origin is a follow-up, not
  part of this item.
- **Encoded-credential scrubbing.** `leaked_credentials.rs` is shared with 0207
  but the two are behaviourally independent; this plan leaves the scrub rule
  untouched beyond confirming it still holds and runs over daemon-captured
  content once the header is live.
- **A cross-request origin pin.** Rejected: the daemon has no crawl boundary to
  reset such a pin against (see Current State Analysis), so a pin would bleed
  between crawls on a warm daemon. Auth is per-request state instead.
- **Retiring the path.** The direction is settled as wire-up.
- **A `location_origin` computed in Rust or JS.** Rejected for the `URL.origin`
  parity hazard; the client declares the location *URL* and the daemon derives the
  origin, so one parser normalises both comparands.

## Implementation Approach

Auth is per-request state keyed to the **crawl's declared location origin**, not
to each navigate's own origin. The crawl declares its `[location]` once through
`ACCELERATOR_BROWSER_LOCATION` (the header-mode companion to
`ACCELERATOR_BROWSER_AUTH_HEADER`), resolved in the executor's `run` shell and
threaded — as the *URL*, so the daemon derives the origin and parser parity holds
— into **every** forwarded browser command as `location_url`. The daemon sets,
per request, `currentExpectedOrigin` = `new URL(req.location_url).origin` and
`currentAuthHeader` = the parsed header pair, both reset each request, exactly as
`currentAllowances` is (`daemon.js:226`). The reworked handler reads both live
through getters and attaches the header only when the request origin equals
`currentExpectedOrigin`, failing closed — strip, never inject — when either is
absent.

Keying to the declared origin is the security boundary, enforced in code rather
than by the crawler's link discipline. Because the location origin is constant
across the crawl and independent of what is navigated:

- **Subresources** of the loaded page are judged against the location origin —
  same-origin assets keep the header, a cross-origin CDN asset is stripped.
- **A cross-origin navigate** (a followed external link, an open-redirect the
  agent resolves) targets an origin other than the location origin, so its
  requests are stripped — the header **cannot** follow the crawl off-site, even
  though same-origin scoping is only an agent rule (see Current State Analysis).
- **A cross-origin redirect** of a navigate resolves to a different origin than
  the location, so the redirected request is stripped — the header is held to the
  location origin (see What We're NOT Doing).

The location and the header ride **every** forwarded command, not just
`navigate`, so the daemon's per-request auth state is set for `snapshot`/`links`
too: a gated page's same-origin subresource that loads lazily after
`domcontentloaded`, during a following command, still carries the header rather
than being stripped.

Both fields reach the daemon off the process argument vector. The command body
the executor forwards becomes `argv` to the short-lived `node run.js` client, and
`argv` is world-readable via `/proc/<pid>/cmdline` and `ps`; a live bearer must
never sit there. So the **JS client** — not the Rust executor — reads
`ACCELERATOR_BROWSER_AUTH_HEADER` and `ACCELERATOR_BROWSER_LOCATION` from its own
inherited environment and injects `auth_header` and `location_url` into the
loopback request body it POSTs to the daemon, never into `argv`. Both are
env-derived and belong to the same layer, so keeping both in the client leaves the
Rust `merge_allowances` untouched (it still injects only the invocation-derived
allowances) and makes the field-name contract same-language: `client.js` and
`daemon.js` import the field names from one shared constant module, so a rename
cannot drift across a language boundary. The client guards both fields against a
page-influenced body pre-setting them. As defence-in-depth the daemon spawn
`env_clear`s and passes an explicit allowlist that excludes both env vars, so
neither the token nor the location rests in the long-lived daemon's environment;
the daemon reads no environment for auth at all and receives both per request.

There is no cross-request pin to bleed between crawls on a warm daemon: every
command re-declares the location and header from the current crawl's
configuration, so a later crawl with a different location, or none, governs its
own requests. `ACCELERATOR_BROWSER_LOCATION` is a new companion env, distinct
from the retired `ACCELERATOR_BROWSER_LOCATION_ORIGIN` (a URL the daemon parses,
not a pre-computed origin), so AC4 still holds. Because the header attaches only
against a resolved location origin, header mode is meaningless without the
location: `design resolve-auth` treats both env vars as required and fails loudly
when the header is set without the location, and the daemon warns if a request
ever carries a header with no resolvable expected origin — so the fail-closed
strip can never be reached silently by misconfiguration.

Each phase follows the red-green loop: the failing test named first in its change
list is written and observed red before the production change that satisfies it.
Three phases, each leaving `mise run` green. Phases 1 and 2 are independent of
each other; Phase 3 depends on both and carries the documentation truth-up so no
merge point ever documents behaviour that does not yet exist.

## Phase 1: Handler Resolves A Live Origin

### Overview

Rework `makeAuthHeaderHandler` to source both the expected origin and the header
from getters supplied by the daemon, read no environment at all, fail closed when
either is absent, and never throw an unhandled rejection into a live route.
Extract the header parse and the header-map mutation as pure, unit-testable
functions. Runtime-free; the handler is still not wired into the daemon, so the
path stays inert and the warnings stay true.

### Changes Required:

#### 1. Unit cases (red first)

**File**: `skills/design/inventory-design/scripts/playwright/lib/auth-header.test.js`
**Changes**: Written and observed red before the factory rework. Cases:
- `parseAuthHeader` returns null for an unset value, a value with no colon, an
  empty trimmed name (`":Bearer x"`), and an empty trimmed value
  (`"Authorization:"`); returns `{ name, value }` for a valid pair, trimmed.
- `shouldAttachHeader` returns false for a null/undefined origin (fail-closed),
  false for a differing origin, true for an exact origin match, and false for an
  unparseable request URL.
- Preserve the existing security-relevant origin edges (the current suite covers
  them, and the rework must not lose them): default-port equivalence
  (`https://h:443/` matches `https://h`), case-insensitive host, subdomain
  confusion (`https://app.example.com.evil.com/` does **not** match
  `https://app.example.com`), an IDN homograph host, and an opaque/`data:` origin.
  The request-URL side is normalised by `new URL().origin`; the port-normalisation
  assertions on the expected side move to the daemon's `originOf` unit cases
  (Phase 3), keeping the equivalence covered end-to-end.
- `applyHeader` adds the pair under the lowercased name when attaching, and
  deletes the header by its lowercased name (matching Playwright's lowercase
  `allHeaders()` keys) when not.
- The installed route no-ops the header (attaches nothing, strips nothing beyond
  a configured name) when `getAuthHeader` returns null.
- Assert no `ACCELERATOR_BROWSER_LOCATION_ORIGIN` and no
  `ACCELERATOR_BROWSER_AUTH_HEADER` read remains in the module.

#### 2. Auth-header factory

**File**: `skills/design/inventory-design/scripts/playwright/lib/auth-header.js`
**Changes**: Take `getExpectedOrigin` and `getAuthHeader` getters, called per
request; read no environment. Wrap the route body fail-closed, mirroring the
classifier (`daemon.js:194-213`): on any thrown error, log a warning bound to the
error message (never the header or request body, so the token cannot reach the
bootstrap log) and fall back to an unmodified `route.continue()` rather than
leaving the request to hang or swallowing the failure silently. Delegate the
origin decision to `shouldAttachHeader` and the header-map mutation to
`applyHeader`.

```javascript
export function makeAuthHeaderHandler(
  page,
  { getExpectedOrigin, getAuthHeader } = {}
) {
  return async () => {
    await page.route('**/*', async (route) => {
      try {
        const header = getAuthHeader?.() ?? null;
        const attach =
          !!header &&
          shouldAttachHeader(route.request().url(), getExpectedOrigin?.());
        const headers = applyHeader(
          await route.request().allHeaders(),
          attach,
          header
        );
        await route.continue({ headers });
      } catch (error) {
        const detail = error?.message ?? String(error);
        console.error(`auth-header route error: ${detail}`);
        try {
          await route.continue();
        } catch {}
      }
    });
  };
}

export function parseAuthHeader(raw) {
  if (!raw) return null;
  const colonIdx = raw.indexOf(':');
  if (colonIdx === -1) return null;
  const name = raw.slice(0, colonIdx).trim();
  const value = raw.slice(colonIdx + 1).trim();
  if (!name || !value) return null;
  return { name, value };
}

export function applyHeader(current, attach, header) {
  const headers = { ...current };
  if (!header) return headers;
  const key = header.name.toLowerCase();
  if (attach) {
    headers[key] = header.value;
  } else {
    delete headers[key];
  }
  return headers;
}

export function shouldAttachHeader(requestUrl, expectedOrigin) {
  if (!expectedOrigin) return false;
  try {
    return new URL(requestUrl).origin === expectedOrigin;
  } catch {
    return false;
  }
}
```

`expectedOrigin` is an origin string (the daemon derives it via `new URL().origin`),
so `shouldAttachHeader` compares against it directly rather than re-parsing it.

#### 3. Unit case floor

**File**: `tasks/test/unit.py`
**Changes**: Raise `_EXPECTED_DESIGN_AUTOMATION_CASES` to the new executed-case
count to lock in the added cases. Suite count is unchanged.

### Success Criteria:

#### Automated Verification:

- [ ] Runtime-free JS suite passes at the raised floor: `mise run test:unit:design-automation`
- [ ] `ACCELERATOR_BROWSER_LOCATION_ORIGIN` reads nowhere in the shipped code
      (AC4, scoped off `meta/`): `grep -rn ACCELERATOR_BROWSER_LOCATION_ORIGIN cli/ skills/ docs-site/`
      returns nothing
- [ ] Full read-only gate passes: `mise run check`

#### Manual Verification:

- [ ] The handler is confirmed still uncalled by the daemon in this phase (grep
      shows the import unused), so the path remains inert and the warnings remain
      accurate until Phase 3.

---

## Phase 2: The Client Declares The Crawl's Location And Header

### Overview

The JS client injects both `location_url` (from `ACCELERATOR_BROWSER_LOCATION`)
and `auth_header` (from `ACCELERATOR_BROWSER_AUTH_HEADER`), read from its own
inherited environment, into the loopback request body it POSTs to the daemon — on
every command, never into `argv`. Both are env-derived and belong to the client
layer, so the Rust executor's `merge_allowances` is **left untouched** (it keeps
injecting only the invocation-derived allowances) and the field-name contract
stays same-language: a shared constant module names `location_url`/`auth_header`
for both `client.js` and `daemon.js`. The daemon ignores both fields until Phase
3, so this is inert and green on its own, and independent of Phase 1.

### Changes Required:

#### 1. Client unit cases (red first)

**File**: `skills/design/inventory-design/scripts/playwright/lib/client.test.js`
(the existing client suite, or a new one)
**Changes**: Written and observed red before the client change. The client adds
`location_url` to the request body when `ACCELERATOR_BROWSER_LOCATION` is set and
`auth_header` when `ACCELERATOR_BROWSER_AUTH_HEADER` is set; omits each when its
env is unset; refuses (throws) a body pre-setting either field; and neither value
appears in the arguments the client received or forwarded (the loopback body is
the only carrier). Raise `_EXPECTED_DESIGN_AUTOMATION_CASES` in `tasks/test/unit.py`
to lock these in (and the suite count if a new `*.test.js` file is added).

#### 2. Client injects both fields off argv

**File**: `skills/design/inventory-design/scripts/playwright/lib/client.js`
**Changes**: Where the client assembles the loopback request body it POSTs to the
daemon (`client.js:52`), add `location_url` from `process.env.ACCELERATOR_BROWSER_LOCATION`
and `auth_header` from `process.env.ACCELERATOR_BROWSER_AUTH_HEADER` when each is
set — into the body, not `argv`. Throw if the incoming body already carries either
field (a page-influenced payload must not pre-set them, mirroring the allowance
guard). The field names come from a shared constant module (below), so the client
and daemon cannot drift.

#### 3. Shared field-name constants

**File**: `skills/design/inventory-design/scripts/playwright/lib/` (new small
module, e.g. `request-fields.js`)
**Changes**: Export `LOCATION_URL_FIELD = 'location_url'` and
`AUTH_HEADER_FIELD = 'auth_header'`; import them in `client.js` (writing) here in
Phase 2, and in `daemon.js` (reading) in Phase 3 — keeping Phase 2 inert. Because
both consumers are JS, one real shared constant replaces a cross-language contract
test entirely: a rename changes both sides at once and cannot silently
un-authenticate the crawl.

The Rust executor is unchanged: `merge_allowances` keeps its name, its two-key
guard, and its allowances-only injection. Nothing auth-related enters `argv` on
either the Rust or the JS side.

### Success Criteria:

#### Automated Verification:

- [ ] Runtime-free JS suite passes at the raised floor (client cases):
      `mise run test:unit:design-automation`
- [ ] Full read-only gate passes: `mise run check`

#### Manual Verification:

- [ ] The forwarded fields are confirmed inert without Phase 3: a daemon on this
      revision ignores `location_url` and `auth_header`, so behaviour is unchanged.
- [ ] Neither the header nor the location value appears in any spawned process's
      `argv` (checked in the client test), only in the loopback request body.

---

## Phase 3: Daemon Wires The Handler, Keys Auth Per Request, And The Docs Tell The Truth

### Overview

Install the auth-header route before the classifier, set the expected origin and
header as per-request state from `req.location_url`/`req.auth_header` where
`currentAllowances` is set, prove the behaviour under a real Chromium, and rewrite
the inert-path warnings on every surface that carries them so no merge point
documents behaviour that does not exist. This phase turns the feature on. The
runtime cases are the red step for the wiring, so they are listed first.

### Changes Required:

#### 1. Runtime acceptance and safety cases (red first)

**File**: `skills/design/inventory-design/scripts/playwright/daemon-runtime.test.js`
**Changes**: Written and observed red before the daemon wiring. Runtime cases call
the daemon directly, so they set `location_url` and `auth_header` in the request
body themselves, playing the client's role; no `extraEnv` is
needed for auth, since the daemon reads no environment for it. The existing
`withDaemon` harness is reused. Use two distinct loopback origins (two `withServer`
binds) for the cross-origin cases; the existing `redirectTo` fixture supplies the
redirect case.

- **AC1 gated page loads**: fixture returns 401 without the bearer header, 200
  with it, on a gated path. With `auth_header` and `location_url` set in the body
  to the fixture origin, `navigate` succeeds and an `evaluate` of the body confirms
  the gated content.
- **AC2 cross-origin strip, mutation-resistant**: the page on origin A issues a
  cross-origin request to origin B **with the bearer header explicitly set on the
  fetch**, and fixture B **answers the CORS preflight** (`Access-Control-Allow-Origin`
  plus `Access-Control-Allow-Headers: authorization`) so the actual GET reaches B.
  Because `Authorization` is not a CORS-safelisted header, without B satisfying the
  preflight the GET would never fire and the strip-deleted mutant would survive; B
  answering it is what makes strip-*removal* observable. B records request headers;
  assert B saw no bearer. The case must be confirmed to fail with the strip branch
  deleted. Paired with AC1's inject assertion, it fails if either branch is mutated.
- **AC3 cross-origin asset loads**: the same B request completes with HTTP 200 and
  carries no bearer header — the strip withholds only the header, not the request.
- **Cross-origin navigate strip**: with the location declared as origin A,
  `navigate` directly to origin B (playing a followed external link); B records
  request headers; assert no bearer reaches B. This proves the code-enforced bound
  — the header keys to the declared location, not the navigate target.
- **Cross-origin redirect leak-guard**: declare location A, `navigate` to an A path
  that 302-redirects to origin B (fixture B records headers); assert no bearer
  reaches B. This is a **boundary/regression assertion**, not mutation-resistant:
  the browser does not forward `Authorization` across origins on a redirect, so no
  bearer is present on the B hop to strip; it still catches an origin-blind
  injection mutant. Framed honestly as such.
- **Late same-origin subresource stays authenticated**: `navigate` to gated origin
  A whose page defers an A-origin fetch until a signal the following `snapshot`
  triggers (so the fetch fires deterministically *during* `snapshot`, not in the
  inter-command gap); `snapshot` carries `location_url`/`auth_header` for A; assert
  the late A-origin request carried the bearer. Because the daemon clears the auth
  state at request end (Section 3), the fetch cannot ride lingering `navigate`
  state, so a mutant that injected the fields on `navigate` only — leaving
  `snapshot` unauthenticated — makes this case fail. This proves the location and
  header riding *every* command keep late subresources authenticated.
- **Header without location warns and strips**: a request carrying `auth_header`
  but no resolvable `location_url` attaches no header (fail closed) **and** emits a
  daemon warning (asserted in the bootstrap log, header value absent). This proves
  the LOCATION-missing misconfiguration degrades loudly, not silently.
- **No cross-request bleed (same origin)**: `navigate` to origin A with
  `auth_header` set (assert the header attaches on A), then a second `navigate` to
  origin A with **no** `auth_header`/`location_url` (a second crawl of the same
  origin, unconfigured); assert A's second request carries no bearer. Using the
  **same** origin is what kills the full-persistence mutant — a design that kept
  origin and header across requests would still attach on the re-crawl.
- **Ordering safety**: with the auth-header route installed, a navigate to a
  refused origin (link-local, no `allow_internal`) still returns
  `navigation-refused`. This fails if the auth-header route bypasses the classifier.

#### 2. Daemon unit cases (red first)

**File**: `skills/design/inventory-design/scripts/playwright/lib/daemon.test.js`
**Changes**: Written and observed red before the daemon wiring. Export `originOf`
from `daemon.js` (or a small sibling module) so it is unit-testable, and cover:
`new URL(value).origin` for a valid URL; **default-port equivalence**
(`https://h:443/p` → `https://h`); case-folded host; a malformed value → null with
a warning emitted (message only). These are the port/normalisation edges shed from
`auth-header.test.js` in Phase 1, given a real, CI-enforced home on the daemon
side. Raise `_EXPECTED_DESIGN_AUTOMATION_CASES` in `tasks/test/unit.py` for the
added cases (net against the assertions removed from `auth-header.test.js`).

#### 3. Route registration and per-request auth state

**File**: `skills/design/inventory-design/scripts/playwright/lib/daemon.js`
**Changes**: Add `currentExpectedOrigin` and `currentAuthHeader` closure
variables. Set both where `currentAllowances`/`lastRefusal` are reset
(`daemon.js:226`), so they reset every request and are live before any request the
command triggers: `currentExpectedOrigin` from `req.location_url` via `originOf`
(its origin, or null if absent/unparseable, logging a warning bound to the parse
error only — never the body or `auth_header` — since daemon stderr is persisted to
the bootstrap log); `currentAuthHeader` from `parseAuthHeader(req.auth_header)`.
**Warn when a header is present but no origin resolves**: if `currentAuthHeader` is
non-null and `currentExpectedOrigin` is null, log a warning (header value absent)
so the LOCATION-missing misconfiguration degrades loudly, not silently. **Clear
both to null at request end** (after the command completes), so the inter-command
gap is fail-closed and per-command re-declaration is what authenticates, not
lingering prior-command state. Install the auth-header route **before** the
classifier route in `ensureBrowser`, passing getters over the two closure
variables and importing the shared field-name constants (Phase 2). Correct the
misleading comment at `daemon.js:190-191` (the seam reaches the earlier-registered
handler via `fallback()` under Playwright's last-registered-first dispatch, not a
later-registered one), and co-locate the two `page.route` installs so the ordering
invariant is visible at the one point it matters.

```javascript
let currentExpectedOrigin = null;
let currentAuthHeader = null;

async function ensureBrowser() {
  if (browser) return;
  // ... launch, context, page ...

  const installAuthHeader = makeAuthHeaderHandler(page, {
    getExpectedOrigin: () => currentExpectedOrigin,
    getAuthHeader: () => currentAuthHeader,
  });
  await installAuthHeader();

  await page.route('**/*', async route => {
    // ... unchanged classifier body, fallback() on the allow path ...
  });
}
```

```javascript
// where currentAllowances / lastRefusal are reset, at request start:
currentAllowances = allowancesOf(req);
lastRefusal = null;
currentExpectedOrigin = originOf(req.location_url);
currentAuthHeader = parseAuthHeader(req.auth_header);
if (currentAuthHeader && !currentExpectedOrigin) {
  console.error('auth-header configured but no location origin resolved');
}
```

```javascript
// at request end (finally), so the inter-command gap is fail-closed:
currentExpectedOrigin = null;
currentAuthHeader = null;
```

`originOf` returns `new URL(value).origin` or null, warning on a malformed value
rather than swallowing it silently. Because the location and header ride every
forwarded command (Phase 2), both are set for `navigate`, `snapshot`, and `links`
alike, so a late same-origin subresource during a following command keeps the
header; because both are cleared at request end, a late fetch in the gap between
commands is stripped rather than riding stale state; and because both are
re-declared each request from the request's own body, no state persists across
crawls on a warm daemon. The keying origin is the crawl's declared location, held
constant across the crawl, so a followed cross-origin link or a cross-origin
redirect targets a different origin and is stripped — the header cannot leave the
location origin. Per-request auth keying relies on the daemon's existing
one-request-at-a-time serialisation (the same invariant `currentAllowances`
depends on); a future concurrency change must revisit both together.

**File**: `cli/design-adapters/src/process.rs`
**Changes**: The daemon reads no environment for auth, so remove the token and the
location from its inherited environment as defence-in-depth. `DaemonSpawner`
currently sets vars additively with no `env_clear` (`process.rs:115-123`); change
the daemon `Command` to `env_clear()` then apply an explicit allowlist — the
existing `STATE_DIR`/`NODE_PATH`/`NS_ROOT`/`BROWSER_EXECUTABLE` vars plus the base
runtime vars Node and Chromium need (`PATH`, `HOME`, `TMPDIR`) and the identity FD
— so neither `ACCELERATOR_BROWSER_AUTH_HEADER` nor `ACCELERATOR_BROWSER_LOCATION`
reaches the long-lived daemon's `/proc/<pid>/environ`. Leave `ExecClient`
inheriting the full environment (the client must read both to inject them per
request); document this daemon-excludes / client-includes asymmetry at the seam. A
test asserts the spawned daemon's environment contains neither auth env var.

Also confirm the daemon's outer request catch (`daemon.js:532`) returns only a
static message on the auth path, never an exception string that could embed a
request header or body — the auth-header route's own catch is the sole place
header values are handled.

#### 4. Auth resolver requires both env vars (fail loud, red first)

**File**: `cli/design/src/credentials.rs` (and its tests)
**Changes**: Header mode is meaningless without a location to key against, so make
`resolve-auth` treat `ACCELERATOR_BROWSER_AUTH_HEADER` and
`ACCELERATOR_BROWSER_LOCATION` as a pair: header mode resolves only when both are
set, and a header set **without** a location produces a loud diagnostic (a
downgrade/warning the caller renders) rather than silently proceeding into a
stripped, unauthenticated crawl. Drive this with unit cases first: both set →
header mode; header without location → diagnostic; neither → the existing
non-header resolution. This is the upfront half of the LOCATION-missing guard; the
daemon warning (Section 3) is the defence-in-depth half.

#### 5. Documentation truth-up

**File**: `cli/design/src/credentials.rs`
**Changes**: Rewrite the module doc (`:1-6`): the header is injected on
same-origin requests and stripped cross-origin; drop the "inert" and "never
calls it" statements.

**File**: `cli/design-cli/src/cli.rs`
**Changes**: Rewrite the `ResolveAuth` help (`:32-39`): state same-origin inject /
cross-origin strip; state that header mode requires **both**
`ACCELERATOR_BROWSER_AUTH_HEADER` and `ACCELERATOR_BROWSER_LOCATION` (the latter
keys the allowlist, and a header without it is refused loudly); remove "Do not
place a live credential".

**File**: `skills/design/inventory-design/SKILL.md`
**Changes**: Remove the `[!WARNING]` block (`:90-96`). Reduce the origin-allowlist
prose (`:98-104`, `:221-226`) to single-origin: the header is injected only on
requests whose origin matches the declared `[location]` origin and stripped on any
cross-origin request. Drop the `ACCELERATOR_BROWSER_LOCATION_ORIGIN` and
login-URL-origin references, and the "once wired up" framing. Document the two
header-mode env vars: `ACCELERATOR_BROWSER_AUTH_HEADER` (the bearer pair) and
`ACCELERATOR_BROWSER_LOCATION` (the crawl's `[location]`, which keys the
allowlist). Update the imperative auth-walled skip message (`:109-113`) — the
call-to-action that today tells the reader to set only `ACCELERATOR_BROWSER_AUTH_HEADER`
— to state that **both** env vars are required and that a header without a location
is refused. Add the redirect caveat with its consequence and a worked example:
"If `[location]` redirects to a different origin (e.g. `http://app.example.com` →
`https://app.example.com`, or an SSO bounce), the header is stripped on the
redirected page and its gated content is captured **unauthenticated** and silently
missing from the inventory — set `ACCELERATOR_BROWSER_LOCATION` to the canonical
post-redirect URL (`https://app.example.com`)." Reconcile `:221-226`: the
cross-origin strip is now code-enforced in the daemon, so drop the instruction to
have the browser-analyser agent enforce the allowlist for manual header injection.

**File**: `agents/browser-analyser.md`
**Changes**: The agent doc (`:123`) states navigation goes through the executor
"so the origin allowlist for the auth header applies." Reframe to say the daemon
enforces the strip in code (keyed to the declared `[location]` origin), so the
agent relies on that boundary rather than performing or enforcing manual header
injection. This sixth surface was outside the original truth-up.

**File**: `docs-site/src/content/docs/design.md`
**Changes**: Rewrite the `:::caution` block (`:125-131`) that repeats "inert
downstream… never calls it… Do not put a live credential": state same-origin
inject / cross-origin strip, the two env vars, and the redirect caveat. This is
the highest-visibility surface and was missing from the original truth-up.

**File**: `docs-site/src/content/docs/reference/skills/design/inventory-design.md`
**Changes**: This page mirrors `SKILL.md` (`Source:` frontmatter) and still
carries the WARNING (`:91-97`) and the "resolved `[location]` origin or the
`ACCELERATOR_BROWSER_LOGIN_URL` origin" prose (to `~:213`). It is a generated
artefact: `docs:check` regenerates it in place via its `docs:generate` dependency
(`mise.toml`: `docs:check` → `docs:generate:check` → `docs:generate`), but
`generate_check` only verifies the page exists and has no orphan — it never fails
on stale committed content. So regenerate it with `mise run docs:generate` and
commit the output; do **not** hand-edit it. The `git status` clean check after
`docs:generate` catches a stale committed mirror; the AC6 grep is the content net.

**File**: `CHANGELOG.md`
**Changes**: Add an `[Unreleased]` entry announcing the browser auth-header path is
now live (same-origin inject / cross-origin strip), requiring both
`ACCELERATOR_BROWSER_AUTH_HEADER` and `ACCELERATOR_BROWSER_LOCATION`, and that a
live credential may now be set — so the newly-working capability and its two-var
requirement are discoverable.

### Success Criteria:

#### Automated Verification:

- [ ] Runtime-free JS suite passes: `mise run test:unit:design-automation`
- [ ] Real-Chromium suite passes, including the acceptance, cross-origin
      navigate/redirect, late-subresource, header-without-location, no-bleed, and
      ordering-safety cases: `mise run test:integration:design-automation`.
      ⚠️ This lane is **not** part of the enforced `mise run` gate (no CI lane
      provisions a Playwright runtime), so the acceptance/mutation assertions run
      only on manual invocation — a recorded reliance, mitigated by the runtime
      lane's own case-count floor and bare-return guard (added alongside these
      cases, mirroring the unit lane).
- [ ] Rust checks pass: `mise run cli:check` and `mise run test:unit:cli`
- [ ] Reference mirror regenerated and committed: `mise run docs:generate` leaves
      `git status` clean under `docs-site/`
- [ ] No inert-path warning remains on any surface (AC6), portable ERE:
      `grep -rniE "inert|never call|is set nowhere|not enforced|wired up|do not (put|place) a live credential|origin allowlist for the auth header" cli/design/src/credentials.rs cli/design-cli/src/cli.rs skills/design/inventory-design/SKILL.md agents/browser-analyser.md docs-site/src/content/docs/design.md docs-site/src/content/docs/reference/skills/design/inventory-design.md`
      returns nothing
- [ ] No stale header-path login-URL prose remains (literal token, the real
      phrasing):
      `grep -rn ACCELERATOR_BROWSER_LOGIN_URL skills/design/inventory-design/SKILL.md docs-site/src/content/docs/reference/skills/design/inventory-design.md`
      returns nothing (the legitimate form-mode references live in `credentials.rs`
      and `design.md`, outside this scope)
- [ ] `ACCELERATOR_BROWSER_LOCATION_ORIGIN` reads nowhere in the shipped code
      (AC4, scoped off `meta/`): `grep -rn ACCELERATOR_BROWSER_LOCATION_ORIGIN cli/ skills/ docs-site/`
      returns nothing
- [ ] The shared daemon is the sole browser-launch site (AC5), production JS only:
      `grep -rnE --include=*.js --exclude=*.test.js "chromium\.launch|browserType\.launch|puppeteer|launchServer" skills/design`
      returns only `skills/design/inventory-design/scripts/playwright/lib/daemon.js:182`
- [ ] Neither auth value appears in any process `argv`:
      `grep -rnE "ACCELERATOR_BROWSER_(AUTH_HEADER|LOCATION)" cli/ skills/` shows
      both read only by the client (into the loopback body) and never inserted into
      a forwarded argument vector
- [ ] Full local CI mirror exits 0: `mise run`

#### Manual Verification:

- [ ] A live authenticated crawl against a local login-gated server (with
      `ACCELERATOR_BROWSER_LOCATION` set to its `[location]`) produces an inventory
      that includes the gated pages, and the daemon bootstrap log shows no bearer
      header on cross-origin requests and no bearer value in any warning line.
- [ ] The rewritten SKILL prose reads coherently as single-origin, documents both
      env vars and the redirect caveat with its consequence, and leaves no dangling
      reference to a login-URL origin or the removed env var.

---

## Testing Strategy

### Unit Tests:

- Handler logic (`auth-header.test.js`): `parseAuthHeader` null on
  unset/no-colon/empty-name/empty-value and a trimmed pair on a valid value;
  `shouldAttachHeader` fail-closed on a null origin, exact-origin match, strip on
  mismatch, false on an unparseable URL, and the preserved origin edges (subdomain
  confusion, IDN, opaque); `applyHeader` add/delete by lowercased name; the
  installed route no-ops when the header getter returns null.
- Client injection (`client.js`): `location_url` and `auth_header` added to the
  loopback body when their env vars are set, omitted when unset, each refused when
  pre-set, and neither value present in the spawned process's `argv`.
- Daemon origin (`daemon.test.js`): `originOf` valid-URL origin, default-port
  equivalence, case-fold, malformed→null with warning.
- Auth resolver (`credentials.rs`): both env vars → header mode; header without
  location → loud diagnostic; neither → non-header resolution.
- Field-name contract: `client.js` and `daemon.js` import one shared constant
  module, so a rename changes both at once — no cross-language drift to test.

### Integration Tests:

- Real-Chromium acceptance (`daemon-runtime.test.js`): gated-page load,
  cross-origin strip (mutation-resistant, header present, fixture answers the CORS
  preflight), cross-origin asset load, cross-origin navigate strip, cross-origin
  redirect boundary assertion, late same-origin subresource stays authenticated,
  header-without-location warns and strips, no cross-request bleed across a
  same-origin re-crawl, and the refused-origin safety check. This lane is outside
  the enforced `mise run` gate; it carries its own case-count floor and
  bare-return guard.

### Manual Testing Steps:

1. Export `ACCELERATOR_BROWSER_AUTH_HEADER="Authorization: Bearer <test-token>"`
   and `ACCELERATOR_BROWSER_LOCATION` to the crawl's `[location]`.
2. Run `inventory-design` against a local server that gates a path on that header.
3. Confirm the gated pages appear in the inventory.
4. Confirm the bootstrap log carries no bearer header on any cross-origin request
   and no bearer value in any warning, and that no `node run.js` `argv` (via `ps`)
   carries the token.

## Performance Considerations

The auth-header route adds one `page.route` handler over the page's life, running
per request alongside the existing classifier. Both already run on every request;
the added work is an origin-equality check and a header map copy — negligible
against navigation and network cost.

## Migration Notes

No state format changes. Header mode now needs two env vars: the existing
`ACCELERATOR_BROWSER_AUTH_HEADER` (the bearer pair) and the new
`ACCELERATOR_BROWSER_LOCATION` (the crawl's `[location]`, which keys the
allowlist); `resolve-auth` refuses a header without a location loudly, so the
requirement cannot be missed silently. Neither reaches the daemon through its
inherited environment: the short-lived client reads both and injects them per
request into the loopback body, while the daemon spawn `env_clear`s and
allowlists its environment to exclude both, so a warm daemon governs each crawl by
that crawl's own requests rather than by whatever environment it was spawned with.
A warm daemon running pre-Phase-3 code ignores the new fields until it idles out
(10 minutes), the existing reuse contract. A `CHANGELOG.md` `[Unreleased]` entry
records the capability going live.

## References

- Original work item: `meta/work/0209-wire-up-or-retire-the-header-auth-path.md`
- Related research: `meta/research/codebase/2026-09-10-0209-wire-up-browser-auth-header-path.md`
- Dead handler and wire-up seam: `skills/design/inventory-design/scripts/playwright/lib/auth-header.js:5-59`, `daemon.js:13,173-214,279-292`
- Classifier seam: `skills/design/inventory-design/scripts/playwright/lib/access-policy.js:318-324`
- Executor merge (`merge_allowances`, left unchanged — allowances only): `cli/design-cli/src/executor.rs:176-207`
- Executor is per-command/stateless; body becomes client `argv` (why the token stays off it): `cli/design-cli/src/executor.rs:101-164,483-485`, `run.js:45`
- Client loopback POST (location + header injection seam, off argv): `lib/client.js:28,52-88`
- Shared field-name constants (new module, imported by client and daemon): `skills/design/inventory-design/scripts/playwright/lib/`
- Crawl loop and same-origin rule live in the agent: `agents/browser-locator.md:88-97,94,133-134`; daemon returns all links tagged `same_origin`: `daemon.js:305,329-332`
- Daemon spawn env (add `env_clear` + allowlist; client keeps inheriting): `cli/design-adapters/src/process.rs:115-123`, spawn env vec at `cli/design-cli/src/executor.rs:464,469`
- Auth resolver (require both env vars, fail loud): `cli/design/src/credentials.rs`
- Warning surfaces: `cli/design/src/credentials.rs:1-6`, `cli/design-cli/src/cli.rs:32-39`, `skills/design/inventory-design/SKILL.md:90-96,109-113`, `agents/browser-analyser.md:123`, `docs-site/src/content/docs/design.md:125-131`, `docs-site/src/content/docs/reference/skills/design/inventory-design.md:91-97`
- Docs generation vs check: `tasks/docs.py` (`generate` regenerates; `generate_check` verifies existence/orphans only) and `mise.toml` (`docs:check` → `docs:generate:check` → `docs:generate`)
- Test lanes and floors: `tasks/test/unit.py`, `tasks/test/integration.py:110`
