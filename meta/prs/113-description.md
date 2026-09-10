---
type: "pr-description"
id: "113"
title: "[0209] Wire up the browser auth-header path in design skills"
date: "2026-09-10T11:50:17+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
parent: "work-item:0209"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/113"
pr_number: 113
tags: ["design", "security", "playwright", "auth"]
revision: "d80ce94408382752d787a5e1438b2cf22f191fbd"
repository: "accelerator"
last_updated: "2026-09-10T11:50:17+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0209] Wire up the browser auth-header path in design skills

## Summary

The browser auth-header path was dead: the daemon imported the handler and
never called it, and the origin input it needed was set nowhere — so an
authenticated design crawl silently produced an *unauthenticated* inventory.
This PR wires it up. In header mode the daemon now injects the bearer on
requests whose origin matches the crawl's declared location and strips it on
every cross-origin request, enforced in code, and the documentation that called
the path inert is corrected to match.

## Changes

- **Handler resolves a live origin** (`auth-header.js`). `makeAuthHeaderHandler`
  takes `getExpectedOrigin`/`getAuthHeader` getters read per request and reads
  no environment; `parseAuthHeader` and `applyHeader` are extracted as pure,
  unit-tested functions; the route body fails closed on error.
- **Client declares the crawl's location and header** (`client.js`,
  `request-fields.js`). Both are read from the client's own environment
  (`ACCELERATOR_BROWSER_LOCATION`, `ACCELERATOR_BROWSER_AUTH_HEADER`) and
  injected into the loopback request body — never onto `argv`, where
  `/proc/<pid>/cmdline` would expose a live token. A shared constant module
  names the two body fields so the client and daemon cannot drift; a
  page-influenced payload pre-setting either field is refused.
- **Daemon keys auth per request** (`daemon.js`). The auth-header route installs
  before the classifier (so navigation policy still gates every request under
  Playwright's last-registered-first dispatch); `currentExpectedOrigin` and
  `currentAuthHeader` are set where the allowances reset and cleared at request
  end, so a warm daemon governs each crawl by its own configuration and the
  inter-command gap is fail-closed. A header without a resolvable location warns.
- **Auth resolver requires both env vars** (`credentials.rs`, `environment.rs`,
  `cli.rs`). `resolve-auth` treats the header and `ACCELERATOR_BROWSER_LOCATION`
  as a pair and refuses a header without a location loudly, rather than
  proceeding into a stripped, unauthenticated crawl.
- **Daemon-spawn hardening** (`process.rs`). The daemon spawn `env_clear`s and
  applies an explicit allowlist, so neither auth variable rests in the
  long-lived daemon's environment; the client remains the sole injector.
- **Documentation truth-up**. `SKILL.md`, `agents/browser-analyser.md`,
  `docs-site/.../design.md`, the generated reference mirror, and `CHANGELOG.md`
  drop the inert-path warnings, reduce the origin prose to single-origin, and
  document the two env vars and the cross-origin-redirect caveat.
- **Tests**. Unit coverage for the handler, client injection, and daemon
  `originOf`; a real-Chromium acceptance suite in `daemon-runtime.test.js`; unit
  and runtime case-count floors; the `cargo-public-api` snapshot regenerated.

## Context

Implements work item `0209`. Planning, research, review, and validation
artifacts are included under `meta/` (`meta/plans/`, `meta/validations/`,
`meta/work/0209-*`).

## Testing

- [x] Full local CI mirror is green: `mise run` exits 0.
- [x] Runtime-free JS suite: `mise run test:unit:design-automation` — 105/105.
- [x] Rust + build-system gates: `mise run cli:check`, `test:unit:cli`,
      `build-system:check`.
- [x] Real-Chromium acceptance (7 cases) under a live browser: gated-page load,
      cross-origin strip, cross-origin asset load, cross-origin navigate strip,
      late same-origin subresource, header-without-location warns, no-bleed,
      ordering safety.
- [ ] Runtime cross-origin-redirect leak-guard: fails in the local dev
      environment only — see Notes.
- [ ] Manual: a live authenticated `inventory-design` crawl against a
      login-gated server.

## Notes for Reviewers

- **Redirect leak-guard is environmentally blocked here.** The local Chromium
  build does not re-intercept server-side 302 redirect hops, so neither the
  classifier nor the auth-header route sees the hop; the pre-existing
  link-local-redirect cases (8/10) fail identically on the unmodified baseline,
  confirming the cause is the browser/version, not this change. Please run
  `mise run test:integration:design-automation` in a runtime where redirect
  interception works and confirm the redirect cases pass.
- **`env_clear` allowlist warrants a look.** The daemon spawn is cleared to
  `PATH`/`HOME`/`TMPDIR` plus the explicit runtime vars. No lane exercises the
  production spawn with real Chromium, so a host needing an additional variable
  for the vendored browser would fail only in a live crawl.
- The runtime lane is outside the enforced `mise run` gate (no CI lane
  provisions a Playwright runtime); it now carries its own case-count floor and
  bare-return guard.
