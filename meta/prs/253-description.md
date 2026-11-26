---
type: "pr-description"
id: "253"
title: "[PP-957] Give every remaining test lane a --filter, and repair the design-automation runtime lane"
date: "2026-11-22T00:00:09+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-957"
parent: "work-item:PP-957"
relates_to: ["work-item:PP-943", "work-item:PP-805", "work-item:PP-738", "work-item:PP-778"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/253"
pr_number: 253
tags: ["build-system", "testing", "design", "playwright", "security", "ci"]
revision: "b3fa6b7040b0a35108504e6edc82e1409b1c96ad"
repository: "accelerator"
last_updated: "2026-11-22T16:27:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [PP-957] Give every remaining test lane a --filter, and repair the design-automation runtime lane

## Summary

PP-957's filters increment, which closes the work item: every multi-test
`test:*` lane now takes `--filter` in its own runner's syntax, fails with
`--filter '<expression>' selected no tests` when a filter runs nothing, and a
guard keeps it that way for new lanes. Running the design-automation
runtime lane for that work showed it had been failing all 37 of its tests,
outside every build, so this PR also repairs the Playwright executor against
the vendored Playwright 1.64.1 and moves the lane into `test:integration`.
That repair includes a security fix: a bearer scoped to the crawl's location
origin was reaching the target of a cross-origin redirect.

## Changes

### `--filter` on every remaining lane (Phases 7–8)

| Lanes | Runner | Rendering | "Nothing selected" detected from |
|---|---|---|---|
| 11 pytest lanes | pytest | `-k '<expr>'` | exit 5 |
| `test:unit:visualiser`, `test:integration:visualiser` | libtest | positional substring | passed + failed in `test result:` lines, summed across every run; `#[ignore]`d tests do not count |
| `test:integration:tracker-contract` | nextest | `-E '<expr>'` within `--profile contract` | exit 4 |
| `test:unit:frontend` | Vitest | `-- -t '<expr>'` | `Tests` summary line, ANSI stripped |
| both design-automation lanes | `node --test` | `--test-name-pattern` before the first file | top-level TAP points, less suite-file points and `# SKIP`/`# TODO` points |
| `test:e2e:visualiser`, `:docker`, `:docker:update` | Playwright | `-- --grep '<expr>'`, quoted for Docker's inner `bash -c` | "No tests found" in output |

- `Selection` (`tasks/shared/selection.py`) renders one filter in each
  runner's syntax; `tasks/test/filters.py` holds each runner's lane runner and
  its no-match detection. The design-automation lanes share their TAP and
  bare-return helpers through `tasks/test/design_automation.py`.
- Four bare `uv run pytest` mise tasks became invoke tasks so they declare
  `--filter` like the rest.
- Every lane's `mise.toml` `usage` names its syntax with an example that
  selects real tests in that lane, and forwards the flag only when given. Each
  pytest lane has its own example, such as `'filter and not empty'` for
  `test:unit:tasks` and `'orphan'` for `test:integration:dev`.
- A filtered design-automation run skips its pass-count and skip floors and
  says so; a failing test still fails it.
- Every lane forwards the flag as `${usage_filter+--filter="$usage_filter"}`.
  The `=` matters: mise runs `sh`, and macOS's bash 3.2 `sh` drops the quoted
  empty word in the spaced form, so `--filter ''` died in invoke's parser
  instead of reaching the task, which refuses it.

### Guidance (Phase 9)

- `test_every_multi_test_leaf_declares_a_filter` fails any `test:*` leaf
  that is not an exempt guard, a single-test lane or a roll-up and lacks a
  `--filter` flag with an `e.g.` example and the forwarding;
  `test_every_task_that_takes_no_filter_exists` stops the exempt list going
  stale.
- `test_every_filtered_leaf_has_its_help_compared_with_invoke_s` fails a
  filtered lane whose `mise.toml` help is not checked against its invoke
  task's.
- `test_a_pytest_lane_s_filter_example_selects_a_test_in_it` runs
  `pytest --collect-only -k <example>` against each pytest lane's own test
  path, adding about 4 s to `test:unit:tasks`.
- The four roll-ups' descriptions point at their leaves' `--filter` rather
  than only `test:unit:cli`'s.
- `CLAUDE.md` and `tasks/README.md` gain "Running a subset of tests": each
  runner's syntax, the exempt lanes, that roll-ups ignore `--filter`, and that
  plain `cargo test` is not a supported way to run the `cli/` tests.

### Design executor repair

- **Per-hop interception.** Playwright 1.64's `page.route` fires only for a
  redirect chain's first hop, and `route.continue({ headers })` carries the
  override onto every later hop. The daemon now intercepts every hop through
  CDP's `Fetch` domain (`lib/request-interception.js`): the navigation
  classifier and the auth-header decision run per hop, and a header override
  stays on the hop it is set on. This replaces both `page.route` handlers and
  retires `makeAuthHeaderHandler`. The classifier now takes
  `{ url, isNavigation }` rather than a stand-in Playwright request.
- **Every request stays on the intercepted target.** `Fetch` sees only the
  page's own target. The daemon launches Chromium with
  `--disable-site-isolation-trials`, so a cross-site frame's later navigations
  stay on that target rather than moving to an out-of-process iframe, and
  creates its context with `serviceWorkers: 'block'`, since service-worker
  requests escaped both the old and the new interception.
- **No Playwright routing beside it.** Playwright's own routing also runs on
  `Fetch`, so a source sweep fails if the executor calls `.route(`,
  `.unroute(` or `.routeFromHAR(`.
- **`snapshot`** returns `locator(':root').ariaSnapshot()` (YAML text);
  `page.accessibility` no longer exists. `PROTOCOL.md` is updated. The document
  root rather than `body`, because a document without a `<body>` left
  `snapshot` waiting out the wall clock.
- **`file://` navigation** is now refused by the daemon (`malformed`) as well
  as by the Rust location parser; previously `page.route` never saw it.
- **A refused continuation fails the hop.** If Chrome refuses
  `Fetch.continueRequest`, the interceptor sends `Fetch.failRequest` rather
  than leaving the hop paused until the 300 s wall clock. An interception
  error names the hop by origin and path, without its query.
- **A failed interception setup discards the browser.** If `Fetch.enable`
  fails, the daemon closes that browser and the next command launches a fresh
  one, rather than reusing a page that would navigate without its policy.

### Design-automation runtime lane

- `runtime-preflight.js` read the `package-lock.json` the bootstrap removal
  deleted. The suites now take the driver tree as the playwright-core root and
  `<browser tree>/chrome-headless-shell` as the browser, as the executor gives
  them to the daemon.
- The lane resolves both trees, through the `build:cli:dev` launcher's
  `cache ensure driver browser` when the launcher has not exported them,
  into `cli/target/design-automation-cache/` rather than the launcher's
  default cache root, the shipped `bin/`, which the `entrypoint` suite
  asserts nothing writes into. A failed `cache ensure` now shows the
  launcher's own error and tells you to re-run the lane.
- The lane joins `test:integration`, so it runs in the default task and on
  both CI legs.
- `bin/trees/` is gitignored; it was the one launcher cache entry missing.
- Floors raised to today's totals: runtime 37 → 38 cases; unit 10 → 11
  suites, 105 → 114 cases. Both are at-least floors, and this PR's later
  tests sit above them.

### CI and the network

- The integration job caches `cli/target/design-automation-cache` in
  `actions/cache`, keyed on `pins.toml` and the runner's OS and architecture,
  so the ~325 MB of runtime trees are downloaded once per pin bump rather than
  on every run.
- `CLAUDE.md` and `tasks/README.md` ("System prerequisites") say that the
  first `mise run` after a fresh clone, a `cargo clean` or a pin bump downloads
  those trees and needs the network.

### Supply chain

- `url` 2.5.8 → 2.5.9 (and `form_urlencoded` 1.2.2 → 1.2.3) in
  `cli/Cargo.lock`, for RUSTSEC-2026-0592: `Url::origin()` overflows the
  stack on nested `blob:` URLs. The advisory landed after `main`'s last
  supply-chain run, so `main` fails `cargo deny` until this merges.
- `licenses/accelerator-third-party-notices.txt` regenerated to match.

### Phase 6 record

The fixtures-closure measurement is recorded in the plan and work item, and
PP-943's blocking edge released. Nextest phase median 189.4 s → 143.0 s and
task duration 198.3 s → 160.9 s against a fresh before-record, both beyond
noise; the two run records are under `meta/measurements/`.

## Context

- Work item: `meta/work/PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`
  (Requirement 5 carries the confirmed per-runner table)
- Plan: `meta/plans/2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`,
  Phases 6–9 with their "As built" notes; marked `done`
- Validation: `meta/validations/2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run-validation.md`
  (result `pass`); PP-957 is marked done in this PR
- Review: `meta/reviews/prs/253-review-1.md`; every inline thread and every
  general finding has been answered on the PR
- Earlier increments: #247, #249
- The runtime-lane repair completes PP-805, PP-738 and PP-778, marked done
  in this PR

## Testing

- [x] `mise run` (the full local CI mirror) passes at `f6ff503a`, after all
      the review fixes; the head adds only a tracked test fixture
- [x] CI passes at `2e42551d`. The ubuntu integration job needed one re-run:
      its first attempt stalled for 900 s fetching `manifest.json` from the
      `v1.24.0-pre.110` release (see Notes). Both legs' runtime-tree caches
      are now saved
- [x] The runtime test "a cross-origin redirect does not leak the header to
      the target" fails with the old `page.route` auth handler restored: the
      target receives the bearer. It passes on the `Fetch` interceptor
- [x] Target coverage measured with a standalone script against the vendored
      Playwright and the shipped `chrome-headless-shell`, and against desktop
      Chrome, which site-isolates by default. With the new launch options
      every request that reached a fixture server was intercepted, including
      a cross-site frame's redirected navigation and a dedicated worker's
      fetch; no service worker registered
- [x] Interceptor unit tests (`lib/request-interception.test.js`, 13 cases)
      cover per-hop refusal, sub-frame refusals not masking the main frame,
      fail-closed classification, header attach/strip per hop, a refused and
      an already-gone hop, and the error log. The redirect-hop and
      already-gone tests fail against mutants that reuse a chain's first
      verdict and that rethrow from `failQuietly`
- [x] Forked-daemon tests against fake `playwright-core` builds cover a
      failed interception start and the launch options
- [x] `test_the_forwarded_filter_reaches_invoke_as_given` runs the
      forwarding through `sh` and failed on the spaced form under macOS
- [x] Each new "nothing selected" rule is tested: libtest selecting only
      `#[ignore]`d tests, node selecting only `# SKIP`/`# TODO` points,
      nested subtests and a suite file that failed to load
- [x] Each pytest lane's example collects at least one test; the guard
      failed in all 11 lanes against the old `'resume and not slow'`
- [x] Each lane run by hand with a subset filter (fewer tests, all
      passing), a no-match filter (`selected no tests`) and its malformed
      example (runner error), including both Docker lanes; `:update` filtered
      to one snapshot regenerated it byte for byte
- [x] `tracker-contract` subset run against the in-memory `RecordingTracker`
      binary; the live Jira, Linear and Shortcut tests need credentials and
      were not run
- [x] `mise run deny:check`, `mise run cli:check` and `mise run
      notices:check` pass after the `url` upgrade

## Notes for Reviewers

- **Security fix in `lib/request-interception.js`.** Worth the closest read:
  it replaces both route handlers. A refused continuation fails the hop, but
  a hop the interceptor never answers still stalls until the per-operation
  wall clock.
- **Site isolation is off in the crawl browser.** That keeps every frame on
  the one intercepted target, at the cost of site isolation's defence against
  cross-site memory leaks. The browser is short-lived, crawls one app, and
  the shipped `chrome-headless-shell` did not site-isolate anyway; the flag
  matters for desktop Chrome through `design.browser_path`.
- **Known gaps, drafted as a follow-up rather than fixed here.**
  Popups, shared workers and websockets still skip the navigation policy
  (the bearer fails closed there). The bearer is attached by request URL
  whatever the initiator, so a cross-origin frame that fetches the location
  origin borrows it; `page.route` behaved the same, and it needs a security
  decision. Both are captured in
  `meta/work/PP-992-apply-the-design-crawl-s-request-policy-to-every-target-and.md`,
  not yet pushed to Linear.
- **The runtime-tree download can stall.** The launcher's download has no
  connection timeout and no retry, so one stalled request uses the whole
  900 s budget and fails the lane. It happened on macOS and then ubuntu in
  CI, while the same URLs answered in under 3 s locally. The CI cache now
  hides it until `pins.toml` changes; a timeout and retry in the launcher is
  not yet tracked.
- **`snapshot` changed shape**, from a JSON tree to a YAML string of the
  document root. The in-repo consumers are the `browser-analyser` and
  `browser-locator` agents, which read it as text; nothing outside the repo
  was checked.
- **The `links` tests changed fixture**, from a `file://` page to loopback
  HTTP, and the opaque-origin case moved to `about:blank`. A
  `Content-Security-Policy: sandbox` page was rejected as the substitute:
  `location.origin` reports the URL's origin there, and making it opaque
  would change crawl semantics for sandboxed pages.
