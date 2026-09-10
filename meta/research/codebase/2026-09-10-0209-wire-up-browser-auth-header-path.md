---
type: "codebase-research"
id: "2026-09-10-0209-wire-up-browser-auth-header-path"
title: "Research: Wiring up the browser auth-header path in design skills"
date: "2026-09-10T00:17:50+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0209"
parent: "work-item:0209"
topic: "Wiring up the browser auth-header path in design skills"
tags: ["research", "codebase", "design", "auth", "playwright", "browser", "security"]
revision: "41f2e7f60b5432724959087e62f7717d3ec62064"
repository: "accelerator"
last_updated: "2026-09-10T00:17:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Wiring up the browser auth-header path in design skills

**Date**: 2026-09-10T00:17:50+00:00
**Author**: Toby Clemson
**Git Commit**: 41f2e7f60b5432724959087e62f7717d3ec62064
**Branch**: detached HEAD (jj mode)
**Repository**: accelerator

## Research Question

What does the codebase look like today for wiring up the browser auth-header
path documented in work item 0209 — the dead `makeAuthHeaderHandler`, the
navigation-classifier route it must compose with, the Rust credentials surface
and its inert-path warnings, the browser-launch sites across design skills, and
the test scaffolding an authenticated-crawl acceptance test would reuse?

## Summary

The path is dead exactly as 0209 describes, and the sole browser-launch site is
confirmed, so one wire-up covers every design skill. **One instruction in the
work item is wrong, though: the auth-header route must register _before_ the
navigation-classifier route, not after.** Playwright dispatches `page.route`
handlers last-registered-first, and the classifier's allow path terminates with
`route.fallback()` while the auth-header handler terminates with
`route.continue()`. Register auth-header _after_ the classifier and it fires
first, `route.continue()`s, and the classifier never runs — navigation policy is
silently bypassed. Two independent code analyses reached this conclusion; it is
the single most load-bearing correction for planning.

A second, quieter tension: the permitted origin is meant to come from the
resolved `[location]` (the first `navigate` URL's origin), but the route is
installed in `ensureBrowser` at page-creation time — **before any `navigate`
arrives**, so the origin is not yet known at install time. The wire-up must
either defer route installation to the first navigate or read the origin from a
mutable closure variable set on first navigate. This is the substance behind the
work item's Open Question 2.

Everything else lines up with the work item. The shared daemon (`daemon.js:182`)
is the only `chromium.launch` in production; `analyse-design-gaps` has no browser
code. `ACCELERATOR_BROWSER_LOCATION_ORIGIN` is read in exactly one place
(`auth-header.js:7`) and nowhere in Rust, so the AC4 "no longer read anywhere"
grep is straightforward. The three warning surfaces are located to the line. The
test harness (`node --test`, `withDaemon` + `withServer`) already has a template
that maps onto every acceptance criterion.

## Detailed Findings

### The dead handler and its wire-up seam (`auth-header.js`, `daemon.js`)

`makeAuthHeaderHandler` is a two-stage factory: `makeAuthHeaderHandler(page,
{ env })` returns an async _installer_; the `page.route` registration happens
only when that installer is invoked (`auth-header.js:5,26-27`). It reads the
header from `ACCELERATOR_BROWSER_AUTH_HEADER` and the expected origin from
`ACCELERATOR_BROWSER_LOCATION_ORIGIN` (`auth-header.js:6-7`), and returns a
no-op installer when either is unset, the header lacks a `:`, or the origin
fails `new URL()` (`auth-header.js:9-24`).

The per-request handler covers exactly the three branches AC1–AC3 need, and
every branch ends in `route.continue()` — the request always proceeds:

| Branch | Condition | Action | Line |
|---|---|---|---|
| Inject | `requestOrigin === expectedOrigin` | `continue({ headers + bearer })` | `auth-header.js:37-39` |
| Strip | origin differs | `continue({ headers − bearer })`, header key lowercased for `delete` | `auth-header.js:40-44` |
| Passthrough | URL unparseable | `route.continue()`, no change | `auth-header.js:32-35` |

The strip branch already satisfies the AC3 requirement that only the header is
withheld — the request still `continue()`s, so cross-origin CDN assets load.
Origin comparison is exact `URL.origin` equality. A pure helper
`shouldAttachHeader(requestUrl, expectedOrigin)` (`auth-header.js:51-59`) exists
for unit tests.

The import at `daemon.js:13` is never called — confirmed by grep (only the
import, the definition, and the test reference it). `ensureBrowser`
(`daemon.js:173-214`) launches `chromium` (`daemon.js:182`), creates the context
and page (`daemon.js:183-184`), and registers the classifier route
(`daemon.js:194`) as the _only_ route on the page. The daemon holds no
persistent resolved-location state — its closure vars (`daemon.js:81-93`) are
`browser`, `page`, timers, `currentAllowances`, and `lastRefusal`, the last two
reset per request.

### Route ordering — the work item's instruction is inverted ⚠️

The work item (lines 84–85, Technical Notes 207–209), the 0206 plan, and the
0209 review all state the auth-header route registers _after_ the classifier
"so fallen-through requests reach it." Under Playwright's actual semantics this
is backwards.

```text
Playwright page.route dispatch: LAST-registered runs FIRST.
route.fallback()  → defers to the PREVIOUSLY-registered handler.
route.continue()  → terminates the chain; earlier handlers never run.

classifier allow path  → route.fallback()   (daemon.js:208)
auth-header all paths   → route.continue()   (auth-header.js:33,39,43)
```

Register order `[classifier, auth-header]` (auth-header "after") → dispatch order
`[auth-header, classifier]` → auth-header `continue()`s first → **classifier
never runs → navigation policy bypassed.**

Register order `[auth-header, classifier]` (auth-header "before") → dispatch
order `[classifier, auth-header]` → classifier decides first: refusal `abort()`s
(auth-header never runs, correct); allow `fallback()`s down to auth-header, which
injects/strips and `continue()`s. **Correct.**

Two viable resolutions, both defensible:

1. **Register the auth-header route before the classifier.** Minimal, keeps the
   two handlers separate as 0206 intended, honours the `fallback()` seam. The
   daemon comment at `daemon.js:190-191` ("later-registered route handler able
   to act") is loosely worded — `fallback()` reaches the _earlier_-registered
   handler, so the comment should be corrected alongside the wire-up.
2. **Fold inject/strip into the single classifier handler.** On the allow path,
   replace `route.fallback()` with `route.continue({ headers })` after computing
   the header decision. Sidesteps ordering entirely — this is what the 0206
   plan's "extends this handler in place" alternative meant (plan Phase 4 §1) —
   at the cost of collapsing the deliberate 0206 seam.

❓ **Decision for the planner**: option 1 (separate route, register before) or
option 2 (fold into the classifier). Prove whichever with a test that navigates
to a _refused_ origin with the auth-header route installed and asserts the
navigation is still blocked — otherwise a policy bypass ships silently.

### Origin sourcing is unknown at install time ⚠️

The route is installed in `ensureBrowser` (`daemon.js:194`) when the page is
created. The first `navigate` — which carries the resolved `[location]` as
`req.url` (`daemon.js:279-285`) — arrives on a _later_ request. So
`new URL(req.url).origin` is not available when the route is installed. The
handler as written captures `expectedOrigin` at construction (`auth-header.js:7,
21`), which cannot work if the origin only becomes known at first navigate.

Resolutions map onto the work item's Open Question 2:

- **Capture at first navigate**: install the auth-header route lazily on the
  first `navigate`, or install at page creation but read a mutable
  `resolvedLocationOrigin` closure variable that the `navigate` case sets from
  `new URL(req.url).origin`. Requires reworking `makeAuthHeaderHandler` to read
  a live origin rather than one frozen at construction.
- **Thread from the executor**: add the origin as a new field on the `navigate`
  request (or a dedicated request), so the daemon is told the location rather
  than inferring it. The daemon holds no such field today.

### Rust credentials surface and the three warning surfaces

`AuthMode::Header` (`credentials.rs:22`) is emitted as `"header"`
(`credentials.rs:30`) and wins over a complete form config: `resolve()` checks
`auth_header.is_some()` first (`credentials.rs:87`) and returns Header
regardless of the form trio (`credentials.rs:101`), only computing a warning
naming the ignored form vars. Header mode replaces form login — consistent with
the single-origin assumption.

The leak-scrub rule stays as-is under a live header. `NamedSecret::needles()`
(`leaked_credentials.rs:25-34`) always adds the whole value as needle #1, then —
when the value contains a colon — adds the trimmed right half as a second
needle. For `Authorization: Bearer abc123` it yields both the full pair and
`Bearer abc123`; `scan()` reports only the variable _name_, never the value
(`leaked_credentials.rs:44-45`). This is the file 0207 also touches.

The three inert-path warning surfaces AC6 requires rewriting:

| Surface | Location | Current text (gist) |
|---|---|---|
| Module doc | `credentials.rs:1-6` | "`header` mode is inert downstream … handler never called" |
| CLI help | `cli.rs:32-39` (`ResolveAuth`) | "Do not place a live credential … until that is wired up" |
| SKILL prose | `SKILL.md:90-96` (also `:93`, `:99`, `:110`, `:222`) | origin allowlist "is set nowhere" |

Env reads: `environment.rs` reads `ACCELERATOR_BROWSER_AUTH_HEADER` only
(`environment.rs:9,18-20,26`) to build domain values and never reads
`ACCELERATOR_BROWSER_LOCATION_ORIGIN` or passes anything into a browser
environment — the header reaches the browser purely through the JS handler's
`process.env`. `ACCELERATOR_BROWSER_LOCATION_ORIGIN` is read in exactly one
place tree-wide: `auth-header.js:7`. **Zero `cli/` reads** — the AC4 grep is
clean.

### Sole browser-launch site — AC5 is satisfiable as written ✅

There are two design skills. `inventory-design` holds all browser code under
`scripts/playwright/`; `analyse-design-gaps` has only `SKILL.md` + `evals/` and
consumes inventory output without driving a browser. The one production
browser-launch is `chromium.launch` at `daemon.js:182`, reached only via
`run.js` → `startDaemon()` → `ensureBrowser`. No `puppeteer`, `launchServer`,
`browserType.launch`, or browser-level `connect()` exists in production. The
`client.js` `connect()` calls are TCP sockets to the daemon, not browser
attaches. **The shared daemon lib is the sole launch site; one wire-up covers
every consumer.**

### Test scaffolding for the acceptance criteria

JS tests run under **`node --test`** (`node:test` + `node:assert/strict`) — no
vitest/jest. `package.json` has no `scripts` block; tests run through two
mise/invoke lanes:

- **Unit (runtime-free)**: `tasks/test/unit.py` task `design_automation`, globs
  `lib/*.test.js`. Enforces floors — 10 suites, 85 cases, zero skips — and
  rejects bare `return;` in a test body. Adding a `lib/*.test.js` suite means
  bumping these floors.
- **Runtime (real Chromium)**: `tasks/test/integration.py` runs a _named_ tuple
  `_DESIGN_AUTOMATION_RUNTIME_SUITES = ("test-run.js", "daemon-runtime.test.js")`.
  A live auth-header acceptance test must be added to this tuple and live at the
  playwright root (not under `lib/`) so the unit glob never reaches it.

The acceptance-test template already exists — compose the runtime-daemon and
fixture-server helpers:

```js
// withDaemon (daemon-runtime.test.js:94-105) sets auth env via extraEnv;
// withServer (daemon-runtime.test.js:211-234) binds an ephemeral loopback port.
await withDaemon(async info => {
  await withServer(gatedHandler, async serverUrl => {          // 401 without header, 200 with
    const res = await send(info.url, { protocol: 1, command: 'navigate', url: serverUrl });
    assert.equal(res.ok, true, JSON.stringify(res));
  });
});
```

For AC2's mutation-resistant cross-origin strip and AC3's asset-loads check,
reuse the request-header capture pattern (`client.test.js:107-126`): record
`req.headers[...]` inside the handler, assert after. Runtime suites gate on
`requireRuntime()` and deliberately never skip. Rust tests follow the inline
`#[cfg(test)]` + `Result<(), String>` pattern (`credentials.rs:125-219`;
`leaked_credentials.rs:51-133`, esp. the value-half-is-a-needle and
name-alone-no-false-positive cases).

## Code References

- `skills/design/inventory-design/scripts/playwright/lib/auth-header.js:5-59` — the dead handler, its inject/strip/passthrough branches, and `shouldAttachHeader`
- `skills/design/inventory-design/scripts/playwright/lib/daemon.js:13` — unused import; the wire-up site
- `skills/design/inventory-design/scripts/playwright/lib/daemon.js:173-214` — `ensureBrowser`, `chromium.launch:182`, classifier route `:194`, allow-path `route.fallback():208`, refusal `route.abort():212`
- `skills/design/inventory-design/scripts/playwright/lib/daemon.js:279-292` — `navigate` case; `req.url` → resolved `[location]`
- `skills/design/inventory-design/scripts/playwright/lib/access-policy.js:318-324` — `classifyNavigationRequest`
- `cli/design/src/credentials.rs:1-6,19-35,87-104` — module doc warning, `AuthMode`, Header-wins precedence
- `cli/design/src/leaked_credentials.rs:25-34,40-49` — bearer-pair needle split; name-only report
- `cli/design-cli/src/cli.rs:32-39` — `resolve-auth` help warning
- `cli/design-adapters/src/environment.rs:9,18-20,26` — `ACCELERATOR_BROWSER_AUTH_HEADER` read; no location-origin read
- `skills/design/inventory-design/SKILL.md:90-96,222` — SKILL prose warning
- `skills/design/inventory-design/scripts/playwright/daemon-runtime.test.js:94-105,211-234` — `withDaemon`, `withServer` template
- `skills/design/inventory-design/scripts/playwright/lib/client.test.js:107-126` — request-header assertion pattern
- `tasks/test/unit.py`, `tasks/test/integration.py` — test lanes, suite floors, runtime tuple

## Architecture Insights

- **The 0206 seam is real but under-specified in prose.** 0206 deliberately
  chose `route.fallback()` on the allow path to leave room for the 0209 handler.
  The mechanism is sound; the _direction_ described around it ("register after",
  "later-registered handler acts") does not match Playwright's reverse-order
  dispatch. The correct-behaviour ordering is the inverse of the documented one.
- **Lazy origin resolution collides with eager route installation.** The daemon
  installs routes when the page is born but only learns the location on first
  navigate. Any origin-keyed route logic must bridge that gap with mutable state
  or deferred installation — it cannot capture the origin at construction, which
  is how `makeAuthHeaderHandler` is written today.
- **Single browser-launch chokepoint.** All design browser work funnels through
  one `chromium.launch`, so security-relevant route wiring has exactly one place
  to live and one place to test.
- **Fail-closed classifier.** The classifier `abort()`s on both refusal and
  classifier-throw (`daemon.js:201-212`), so a wire-up that preserves classifier
  precedence inherits fail-closed behaviour for free.

## Historical Context

- `meta/plans/2026-08-11-0196-design-cli-migration.md` — Phase 2 §1 and Phase 6
  §5 record the "doubly dead" finding and the deliberate decision to preserve
  the path while correcting the docs (SKILL.md files, `resolve-auth` help,
  `credentials.rs` module doc). Removal sweep §4 raises the wire-up-vs-retire
  fork as the follow-up 0209 descends from.
- `meta/plans/2026-08-31-0206-classify-navigation-urls.md` — Phase 4 §1 defines
  the `route.fallback()` contract and states it "composes with 0209 whether it
  extends this handler in place or registers its own"; "What We're NOT Doing"
  hands cross-origin header stripping to 0209.
- `meta/research/codebase/2026-08-31-0206-classify-navigation-urls.md` —
  "Sequencing 0206 first establishes the route-interception plumbing 0209 then
  extends."
- `meta/work/0243-browser-auth-header-support-in-design-skills.md` — **status
  `abandoned`** (PP-773); the thin, single-AC capability 0209 supersedes. Names
  no dependants beyond its backlog-note source; says nothing about single- vs
  dual-origin.
- `meta/reviews/work/0209-wire-up-or-retire-the-header-auth-path-review-1.md` —
  APPROVED over three passes. Resolved: wire up (not retire); single-origin
  allowlist sourced from the resolved `[location]`, `ACCELERATOR_BROWSER_LOCATION_ORIGIN`
  dropped and grep-verified gone; dual-origin deferred to a follow-up (Open
  Question 1); AC3 must be mutation-resistant. Reviewer treated "register after
  the classifier via `route.fallback()`" as a settled strength — this research
  contradicts that ordering.

## Related Research

- `meta/research/codebase/2026-08-31-0206-classify-navigation-urls.md` — the
  route seam this work extends
- `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md` —
  the CLI surface that ported the credentials logic
- Background ADRs: `ADR-0057`, `ADR-0058`, `ADR-0059`, `ADR-0062` (browser
  automation platform boundary)

## Open Questions

1. **Route ordering / composition** ❓ — register the separate auth-header route
   _before_ the classifier (option 1), or fold inject/strip into the single
   classifier handler (option 2)? The work item's "register after" is incorrect
   under Playwright's dispatch; the planner must pick and the test must prove the
   classifier still blocks refused origins.
2. **Origin sourcing at install time** ❓ — capture `new URL(req.url).origin` on
   first navigate via mutable closure state / lazy installation, or thread the
   origin from the executor as a new request field? (Work item Open Question 2.)
3. **Dual-origin allowlist** — does header mode ever coexist with a distinct
   login origin? Proceeding single-origin per the review; dual-origin is a
   carved-out follow-up. (Work item Open Question 1.)
4. **0243 abandonment** — already `abandoned` in the tree; confirm no live
   dependants before closing out, per the review's Pass 3 requirement.
