---
type: "plan-validation"
id: "2026-09-10-0209-wire-up-browser-auth-header-path-validation"
title: "Validation Report: Wire Up The Browser Auth-Header Path In Design Skills Implementation Plan"
date: "2026-09-10T11:22:27+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-10-0209-wire-up-browser-auth-header-path"
tags: ["design", "security", "playwright", "auth"]
last_updated: "2026-09-10T11:22:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Wire Up The Browser Auth-Header Path In Design Skills

All three phases are implemented and committed, and the enforced `mise run`
gate exits 0. Result is **pass**, with one caveat carried forward: the
real-Chromium runtime lane — which is outside the enforced gate — has two
redirect cases that fail in this local environment for a browser-version
reason, and should be confirmed in a provisioned runtime before merge.

### Implementation Status

- ✅ Phase 1: Handler Resolves A Live Origin — fully implemented (`796b0f2f`).
  `makeAuthHeaderHandler` takes `getExpectedOrigin`/`getAuthHeader` getters and
  reads no environment; `parseAuthHeader`/`applyHeader` are extracted and
  unit-tested; the route body fails closed.
- ✅ Phase 2: The Client Declares The Crawl's Location And Header — fully
  implemented (`c2d9d49c`). `client.js` injects `location_url`/`auth_header`
  from its own environment into the loopback body via the shared
  `request-fields.js` constants; a page-influenced payload pre-setting either
  is refused.
- ✅ Phase 3: Daemon Wires The Handler, Keys Auth Per Request, And The Docs Tell
  The Truth — fully implemented (`62198748`). Route installed before the
  classifier; per-request `currentExpectedOrigin`/`currentAuthHeader` set where
  the allowances reset and cleared at request end; `resolve-auth` requires both
  env vars; daemon spawn `env_clear`s + allowlists; six doc surfaces corrected;
  reference mirror and public-API snapshot regenerated.
- ✅ Follow-up (`e9c7693e`): auth-header comments tightened to the strict comment
  policy (not a plan item — a cleanup requested separately).

Every plan checkbox is now checked. The one Phase 1 box left open at the time
(the `ACCELERATOR_BROWSER_LOCATION_ORIGIN` grep, blocked only by `SKILL.md`
prose) is satisfied by the Phase 3 truth-up, where the identical grep is the
enforced gate and passes.

### Automated Verification Results

- ✅ Runtime-free JS suite: `mise run test:unit:design-automation` — 105 passed,
  0 failed, 0 skipped; floor raised to 105 in `tasks/test/unit.py`.
- ✅ Rust checks: `mise run cli:check` (rustfmt + clippy) and
  `mise run test:unit:cli` — 0 failed.
- ✅ Build-system check: `mise run build-system:check` — passed (validates the
  `integration.py`/`unit.py` edits).
- ✅ Full local CI mirror: `mise run` — exit 0, 0 failures. Reaching a green run
  took retries: three earlier runs each failed on a *different unrelated* lane
  (`spawn_properties` fork timing, an `e2e:visualiser` stale `.e2e-port`, a
  `hooks` vcs smoke), all of which pass in isolation and are load-induced, none
  touching this change.
- ✅ Reference mirror: `mise run docs:generate` leaves `git status` clean under
  `docs-site/`.
- ⚠️ Real-Chromium lane: `mise run test:integration:design-automation` — the
  seven non-redirect auth cases pass (gated-page load, cross-origin strip,
  cross-origin asset load, cross-origin navigate strip, late same-origin
  subresource, header-without-location warns, no-bleed, ordering safety); the
  cross-origin-redirect leak-guard fails in this environment. See Potential
  Issues. This lane is not part of the enforced gate.

Grep gates, all clean:

- ✅ No inert-path warning on any of the six surfaces (AC6).
- ✅ No `ACCELERATOR_BROWSER_LOGIN_URL` in `SKILL.md` or the reference mirror.
- ✅ No `ACCELERATOR_BROWSER_LOCATION_ORIGIN` anywhere under `cli/`, `skills/`,
  `docs-site/` (AC4).
- ✅ Sole browser-launch site is `daemon.js:210` (AC5).
- ✅ Neither auth value reaches a forwarded `argv`: both are read only by
  `client.js` (into the loopback body) and by the Rust `resolve-auth`/scrub
  readers.

### Code Review Findings

#### Matches Plan:

- The handler keys the header in code to the crawl's declared location origin
  and strips it cross-origin, exactly as the Implementation Approach specifies.
- Per-request auth state is set beside `currentAllowances` and cleared at
  request end, so a warm daemon governs each crawl by its own configuration and
  the inter-command gap is fail-closed.
- The client injects off `argv`, and `merge_allowances` (Rust) is left
  untouched — auth stays in the JS layer, sharing one field-name constant.
- `resolve-auth` treats the header and location as a required pair and refuses a
  header without a location loudly (`AuthConfigurationError::HeaderWithoutLocation`).
- The `env_clear` + allowlist on the daemon spawn is implemented as written,
  with `ExecClient` left inheriting the full environment.

#### Deviations from Plan:

- **Late-subresource test fires during `evaluate`, not `snapshot`.**
  `page.accessibility.snapshot()` cannot deterministically trigger a network
  fetch, so the test dispatches the A-origin fetch inside a following `evaluate`
  that carries the auth fields. The property proven is identical — auth rides a
  *following* command and a mutant authenticating `navigate` only would strip
  the fetch — and the test is framed honestly as such.
- **Runtime-lane floor + bare-return guard added to `integration.py`.** The plan
  called for this as a parenthetical mitigation; it is implemented with a
  case-count floor of 37, reusing the unit lane's `_bare_returns_in_tests` and
  `_tap_counts` helpers.

#### Potential Issues:

- ⚠️ **Cross-origin-redirect leak-guard fails locally (environmental).** This
  Chromium build does not re-intercept server-side 302 redirect hops, so neither
  the classifier nor the auth-header route sees the hop; the browser follows the
  loopback A→B redirect internally and carries the header. The pre-existing
  link-local-redirect cases (8 and 10) fail identically on the unmodified
  baseline, confirming the cause is the browser/version, not this change. It
  passes where redirect interception works — but that has not been observed
  here, so it is an outstanding verification (see Manual Testing).
- ⚠️ **The `env_clear` hardening is unverifiable in CI.** No lane exercises the
  production `DaemonSpawner` with real Chromium (the runtime lane forks the
  daemon via `node` with the full inherited env), so a deployment host needing an
  environment variable beyond `PATH`/`HOME`/`TMPDIR` for the vendored Chromium
  would fail only in a live crawl. A Rust unit test proves the two auth vars are
  absent from the constructed environment, but not that Chromium still launches
  under it.
- **The runtime lane needs a materialised `package-lock.json` to run.**
  `requireRuntime()` hashes `skills/design/inventory-design/scripts/playwright/package-lock.json`
  to locate the namespace, and that file is neither tracked nor present in a
  fresh checkout. Running the lane locally required copying the canonical
  lockfile from the cache. This is pre-existing runtime-provisioning behaviour,
  not introduced here, but it blocks the lane in a clean tree.

### Manual Testing Required:

1. Runtime acceptance in a provisioned environment:
  - [ ] Run `mise run test:integration:design-automation` against the vendored
    browser (or a CI runtime) and confirm all 37 cases pass, specifically the
    cross-origin-redirect leak-guard and the pre-existing link-local-redirect
    cases 8/10.

2. Live authenticated crawl:
  - [ ] Export `ACCELERATOR_BROWSER_AUTH_HEADER="Authorization: Bearer <token>"`
    and `ACCELERATOR_BROWSER_LOCATION` to a login-gated local server's
    `[location]`, run `inventory-design`, and confirm the gated pages appear.
  - [ ] Confirm the daemon bootstrap log shows no bearer on any cross-origin
    request and no bearer value in any warning line, and that no `node run.js`
    `argv` (via `ps`) carries the token.

### Recommendations:

- Before merge, run the runtime lane in an environment whose browser
  re-intercepts redirect hops, to convert the redirect leak-guard from
  environmentally-blocked to confirmed. If the vendored CI browser shares this
  local build's behaviour, reconsider whether the redirect strip needs a
  defence that does not rely on per-hop route interception.
- Consider a smoke check that the daemon still launches Chromium under the
  cleared-and-allowlisted environment on each supported platform, since no
  automated lane covers that path today.
- Consider tracking or documenting the runtime lane's `package-lock.json`
  provisioning so the lane is runnable from a clean checkout without copying a
  cached lockfile.
