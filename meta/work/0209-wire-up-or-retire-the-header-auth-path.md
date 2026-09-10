---
type: "work-item"
id: "0209"
title: "Wire Up The Browser Auth-Header Path In Design Skills"
date: "2026-08-12T23:21:12+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "done"
kind: "story"
priority: "high"
parent: "work-item:0196"
derived_from: ["plan:2026-08-11-0196-design-cli-migration"]
relates_to: ["work-item:0196", "work-item:0206", "work-item:0207", "work-item:0243"]
tags: ["design", "security", "playwright", "auth"]
last_updated: "2026-09-09T23:07:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-739"
---

# 0209: Wire Up The Browser Auth-Header Path In Design Skills

**Kind**: Story
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

The browser auth-header injection path that `inventory-design` documents as
security-critical is entirely dead: `makeAuthHeaderHandler` is imported but
never called, and the permitted origin it needs is read from an environment
variable (`ACCELERATOR_BROWSER_LOCATION_ORIGIN`) that is set nowhere. An
authenticated design crawl therefore silently produces an *unauthenticated*
inventory — login-gated pages are simply absent, with nothing saying so. This
item wires the path up (rather than retiring it) across every design skill that
drives a browser, so that a designer running an authenticated crawl gets a
complete, login-gated inventory. It therefore both fixes the dead path and
delivers the browser auth-header capability outright — superseding 0243 — which
is why it is modelled as a story rather than a narrow defect fix.

## Context

`inventory-design` documents an auth-header origin allowlist as
security-critical: a bearer token placed in `ACCELERATOR_BROWSER_AUTH_HEADER`
is supposed to be injected only on navigations whose origin matches the
resolved location, and stripped on any cross-origin navigation.

None of that happens. `makeAuthHeaderHandler` is imported at `daemon.js:13` and
never called, and its origin input, `ACCELERATOR_BROWSER_LOCATION_ORIGIN`, is
set nowhere in the repository. The header path is doubly dead.

The consequence has two halves. A designer is told to place a bearer token into
the environment of a browser-driving daemon for a feature that never applies it;
and an authenticated crawl silently produces an unauthenticated inventory — the
pages behind the login are missing, with nothing flagging the gap. Intended
usage is against local servers with non-sensitive test tokens, so the
silently-incomplete inventory is the primary risk, not credential leakage.

The CLI migration (0196, done) deliberately preserved this rather than changing
behaviour inside a port, but corrected the documentation: the SKILL.md, the
`design resolve-auth` help text, and the `credentials.rs` module doc now all
state the path is inert and that a live credential should not be placed there
yet.

## Requirements

### Reproduction
1. Set `ACCELERATOR_BROWSER_AUTH_HEADER` to a `Name: value` bearer pair.
2. Run an authenticated `inventory-design` crawl against a source whose content
   sits behind that header.
3. Observe: the header is never sent (the handler is never installed) and the
   gated pages are absent from the inventory, with no error or warning.

### Expected vs actual
- Expected: the bearer header is injected on same-origin navigations, stripped
  on cross-origin ones, and the gated pages appear in the inventory.
- Actual: the header is never injected; the crawl yields a silently-incomplete,
  unauthenticated inventory.

### Wire it up
- Install the handler: call `makeAuthHeaderHandler(page, { env })` in
  `ensureBrowser` and await the returned installer, registered *after* the
  navigation-classifier route so fallen-through requests reach it.
- Source the permitted origin from the resolved location — the origin of the
  first `navigate` request the executor sends to the daemon, written `[location]`
  throughout — rather than the unset `ACCELERATOR_BROWSER_LOCATION_ORIGIN`. Under
  the current assumption the handler admits this single origin; whether it must
  also admit a login-URL origin is Open Question 1.
- Prove the cross-origin strip with a test that navigates off-origin and
  asserts the header is absent, and fails if the strip is removed — not merely
  a happy-path assertion that the header arrives.
- Cover every design skill that drives a browser by wiring the single shared
  Playwright daemon lib. Confirm it is the sole browser-launch site by
  enumerating the browser-driving design skills and searching `skills/design`
  for browser-launch entry points, so that one wire-up demonstrably covers every
  consumer.
- Preserve legitimate cross-origin asset loading: stripping the bearer header on
  a cross-origin request must withhold only the header, leaving the request
  itself to proceed so CDN and other cross-origin assets still load.
- Remove or rewrite the 'inert path' warning on all three surfaces that carry
  it: `inventory-design/SKILL.md`, `design resolve-auth` help (`cli.rs`), and
  the `credentials.rs` module doc. Any replacement must state that the header is
  injected same-origin and stripped cross-origin, and must no longer warn
  against placing a live credential.

## Acceptance Criteria

- [ ] Given `ACCELERATOR_BROWSER_AUTH_HEADER` is set and the source is
      auth-gated by a fixture — a local test server returning 401 without the
      bearer header and 200 with it on a specific gated path — when a design
      crawl runs, then the gated pages appear in the inventory.
- [ ] Given a navigation to an origin other than the resolved location's
      origin, when the crawl proceeds, then the bearer header is not sent —
      asserted by a test that fails if the strip is removed.
- [ ] Given a page that loads a legitimate cross-origin asset served from a
      fixture endpoint (e.g. a CDN-style resource on a distinct origin), when the
      crawl proceeds, then that asset request completes with HTTP 200 and carries
      no bearer/`Authorization` header.
- [ ] `ACCELERATOR_BROWSER_LOCATION_ORIGIN` is no longer read anywhere in the
      tree (verified by search); the permitted origin is instead the resolved
      `[location]` — the origin of the first `navigate` request. The
      header-on-correct-origin behaviour itself is covered by the criteria above.
- [ ] The browser-driving design skills are enumerated, and each is shown to
      launch a browser only through the shared Playwright daemon lib; a search of
      `skills/design` for the Playwright launch call and the daemon-lib import
      path returns that lib as the sole launch site.
- [ ] The 'inert path' warning is removed on all three surfaces
      (`inventory-design/SKILL.md`, `design resolve-auth` help, and the
      `credentials.rs` module doc); where the text is replaced rather than
      removed, it states that the header is injected same-origin and stripped
      cross-origin, and no longer warns against placing a live credential.
- [ ] `mise run` exits 0.

## Open Questions

1. Does the allowlist need to admit a login-URL origin as well as the location
   origin? The SKILL.md prose promises "location origin or login URL", but the
   handler compares a single origin only. In header-auth mode (bearer token, no
   form login) the login URL may be irrelevant — confirm whether header mode
   ever coexists with a distinct login origin. This item proceeds under the
   single-origin assumption; a dual-origin outcome is a follow-up, not a change
   to this item's scope.
2. Should the origin be captured from the first `navigate` URL, or threaded from
   the executor (the design skill's Playwright-driving caller) as a new request
   field? The daemon holds no persistent notion of the resolved location today.

## Dependencies

- Blocked by: none — parent epic 0196 is done.
- Relates to: 0196 (parent, done); 0206 (navigation-URL classification, done);
  0207 (encoded-credential scrubbing, ready — same `leaked_credentials`
  surface); 0243 (browser auth-header support — folded in here, see Drafting
  Notes).
- File coupling with 0207: both items modify `leaked_credentials.rs` but are
  behaviourally independent — neither gates the other. Merge order is
  first-come-first-served: whichever lands first goes in as-is, and the second
  rebases onto it. No functional ordering is implied.
- Integration dependency on 0206: the auth-header route's correctness depends on
  0206's navigation-classifier registering first and using `route.fallback()`,
  so fallen-through requests reach the auth-header route. 0206 is done, so this
  is an ordering constraint to honour, not a live blocker.
- Consumer coupling: the wire-up targets the shared Playwright daemon lib as the
  sole browser-driving route for design skills. Implementation must confirm no
  design skill bypasses it; if a bypassing launch site is found, its wire-up is
  carved into a follow-up so this item's scope stays fixed at the shared lib.
- Runtime prerequisite: verifying the acceptance criteria runs a live crawl
  through the Playwright daemon, which needs the existing browser runtime
  (system Node ≥20 plus the bootstrapped lockhash namespace, per the 0196 plan).
- Supersedes 0243 (browser auth-header support, draft): this item folds in and
  delivers that capability. Before abandoning 0243 via /update-work-item on
  completion, confirm it has no dependants, or re-point any onto 0209.

## Assumptions

- The direction is settled: wire the path up, not retire it. Intended usage is
  local servers with non-sensitive test tokens, so the risk driver is the
  silently-incomplete inventory rather than credential leakage, and the
  capability is independently wanted (folded in from 0243). This rationale is
  the recorded basis for the decision.
- Header-auth mode replaces form login (the bearer token stands in for the
  login flow), so a single permitted origin keyed to the resolved location is
  sufficient. Dual-origin support (admitting a distinct login origin) is out of
  scope for this item pending Open Question 1; if that question resolves to
  dual-origin, the added handling is carved into a follow-up rather than widening
  this unit of work.
- The shared Playwright daemon lib is the sole browser-driving route for design
  skills, so wiring it once covers every consumer. Implementation confirms this
  rather than assuming it.

## Technical Notes

- Dead import at `daemon.js:13`; the only route installed in `ensureBrowser` is
  the navigation classifier (`daemon.js:194-213`). No call site exists.
- `makeAuthHeaderHandler(page, { env = process.env })` (`auth-header.js:5`)
  returns an async installer that registers a `page.route`
  (`auth-header.js:26-45`): it injects when `requestOrigin === expectedOrigin`
  and deletes the header otherwise. The expected origin is read from
  `env.ACCELERATOR_BROWSER_LOCATION_ORIGIN` (`auth-header.js:7`, normalised at
  `:21`), which is set nowhere.
- Single-origin, not "location or login URL": the handler has no login-URL
  branch. The dual-origin allowlist exists only in SKILL.md prose
  (`:100-102`, `:221-224`).
- Origin sourcing: `[location]` reaches the daemon only as the first `navigate`
  request's `req.url` (`daemon.js:280,285`). Derive `new URL(req.url).origin`
  at first navigate, or thread the origin from the executor.
- Route ordering: the classifier route uses `route.fallback()`
  (`daemon.js:208`), so the auth-header route must register after it to receive
  fallen-through requests.
- `resolve-auth` header mode: `AuthMode::Header` (`credentials.rs:22`, emitted
  as `"header"` at `:30`) wins over a complete form config
  (`credentials.rs:87-104,159`).
- Scrub rule: `leaked_credentials.rs:25-34` splits the `Name: value` pair and
  adds the value half as its own needle. Keep this once the header is live.
- Warning surfaces: `SKILL.md:90-96`, `cli.rs:35-38`, `credentials.rs:3-6`.

## Drafting Notes

- The direction is settled as wire-up; the original "wire up OR retire" fork is
  collapsed throughout Requirements and Acceptance Criteria and is not reopened.
- Reclassified from `bug` to `story`: the item both fixes the dead path and
  delivers the browser auth-header capability across every browser-driving
  design skill, a delivery footprint wider than a single defect fix. The
  reproduction and expected-vs-actual content is retained as evidence of the
  originating defect.
- Reframed the primary risk as the silently-incomplete inventory rather than
  token leakage, because intended usage is local servers with non-sensitive
  test tokens.
- Folded in 0243 "Browser Auth Header Support In Design Skills" (draft,
  PP-773): broadened scope from the `inventory-design` daemon to all
  browser-driving design skills. This item supersedes 0243 — recommend
  abandoning 0243 as a duplicate via /update-work-item.
- Corrected the dead-import reference from `daemon.js:11` (stale, repeated
  repo-wide) to `daemon.js:13`.

## References

- `skills/design/inventory-design/scripts/playwright/lib/daemon.js`
- `skills/design/inventory-design/scripts/playwright/lib/auth-header.js`
- `skills/design/inventory-design/SKILL.md`
- `cli/design/src/credentials.rs`, `cli/design/src/leaked_credentials.rs`
- `cli/design-cli/src/cli.rs`
- `cli/design-adapters/src/environment.rs`
- Related: 0196, 0206, 0207, 0243
- Plan: `meta/plans/2026-08-11-0196-design-cli-migration.md`
