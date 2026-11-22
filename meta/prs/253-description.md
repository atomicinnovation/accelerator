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
revision: "ce6c14c3d2f9f283ca32dc2dfa7c60409158c3bf"
repository: "accelerator"
last_updated: "2026-11-22T10:39:34+00:00"
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
| `test:unit:visualiser`, `test:integration:visualiser` | libtest | positional substring | `test result:` lines summed across every run |
| `test:integration:tracker-contract` | nextest | `-E '<expr>'` within `--profile contract` | exit 4 |
| `test:unit:frontend` | Vitest | `-- -t '<expr>'` | `Tests` summary line, ANSI stripped |
| both design-automation lanes | `node --test` | `--test-name-pattern` before the first file | top-level TAP points not named after a suite file |
| `test:e2e:visualiser`, `:docker`, `:docker:update` | Playwright | `-- --grep '<expr>'`, quoted for Docker's inner `bash -c` | "No tests found" in output |

- `Selection` (`tasks/shared/selection.py`) renders one filter in each
  runner's syntax; `tasks/test/filters.py` holds each runner's lane runner and
  its no-match detection.
- Four bare `uv run pytest` mise tasks became invoke tasks so they declare
  `--filter` like the rest.
- Every lane's `mise.toml` `usage` names its syntax with an example that
  selects a real test in that lane, and forwards the flag only when given.
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
  retires `makeAuthHeaderHandler`.
- **`snapshot`** returns `locator('body').ariaSnapshot()` (YAML text);
  `page.accessibility` no longer exists. `PROTOCOL.md` is updated.
- **`file://` navigation** is now refused by the daemon (`malformed`) as well
  as by the Rust location parser; previously `page.route` never saw it.
- **A refused continuation fails the hop.** If Chrome refuses
  `Fetch.continueRequest`, the interceptor now sends `Fetch.failRequest`
  rather than leaving the hop paused until the 300 s wall clock.
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
  asserts nothing writes into.
- The lane joins `test:integration`, so it runs in the default task and on
  both CI legs.
- `bin/trees/` is gitignored; it was the one launcher cache entry missing.
- Floors raised to today's totals: runtime 37 → 38 cases; unit 10 → 11
  suites, 105 → 114 cases.

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
- Earlier increments: #247, #249
- The runtime-lane repair completes PP-805, PP-738 and PP-778, marked done
  in this PR

## Testing

- [x] `mise run` (the full local CI mirror) passes at `20ffd986`, with
      both interception fixes and the runtime lane inside
      `test:integration`: design-automation 38 passed, 0 failed, 0 skipped;
      `test:unit:cli` 8,113 passed with each shared fixture family built once
- [x] After the forwarding fix: `test_mise.py` (133 passed) and
      `mise run build-system:check`; `--filter ''` through mise now stops with
      `--filter needs an expression`, and `--filter "forward and not absent"`
      ran 25 tests. The full `mise run` was not repeated after this change
- [x] `test_the_forwarded_filter_reaches_invoke_as_given` runs the
      forwarding through `sh` and failed on the spaced form under macOS
- [x] The two interception fixes are pinned by "a hop whose continuation is
      refused is failed rather than left paused" and "a browser whose request
      interception fails to start is closed and launched afresh", the second
      driving a forked daemon against a fake `playwright-core`
- [x] `mise run build-system:check` passes
- [x] Interceptor unit tests (`lib/request-interception.test.js`, 10 cases)
      cover per-hop refusal, sub-frame refusals not masking the main frame,
      fail-closed classification, and header attach/strip per hop
- [x] The opaque-origin `links` test fails when the `u.origin !== 'null'`
      guard is removed
- [x] Each lane run by hand with a subset filter (fewer tests, all
      passing), a no-match filter (`selected no tests`) and its malformed
      example (runner error), including both Docker lanes; `:update` filtered
      to one snapshot regenerated it byte for byte
- [x] `tracker-contract` subset run against the in-memory `RecordingTracker`
      binary; the live Jira, Linear and Shortcut tests need credentials and
      were not run
- [x] CI's integration job ran the lane on both legs at `d5c8cfe8`: 38
      passed, 0 failed, 0 skipped on `ubuntu-latest`, the first run of
      `chrome-headless-shell` there. The first CI run failed the
      `entrypoint` suite's untouched-`bin/` guard, because the lane wrote
      `bin/trees`; the trees now go under `cli/target/`
- [ ] CI on the current head `ce6c14c3`, which adds Phase 9, both
      interception fixes and the forwarding fix

## Notes for Reviewers

- **Security fix in `lib/request-interception.js`.** Worth the closest read:
  it replaces both route handlers. A refused continuation now fails the hop,
  but a hop the interceptor never answers still stalls until the
  per-operation wall clock. Not exercised against service workers or deep
  frame trees.
- **Known gaps, for follow-up rather than this PR.** Interception covers only
  the first page's own target, so popups, out-of-process iframes, workers and
  websockets skip the navigation policy (the bearer fails closed there). The
  bearer is attached by request URL whatever the initiator, so a cross-origin
  frame that fetches the location origin borrows it; `page.route` behaved the
  same, and it needs a security decision.
- **`snapshot` changed shape**, from a JSON tree to a YAML string. The
  in-repo consumers are the `browser-analyser` and `browser-locator` agents,
  which read it as text; nothing outside the repo was checked.
- **The `links` tests changed fixture**, from a `file://` page to loopback
  HTTP, and the opaque-origin case moved to `about:blank`. A
  `Content-Security-Policy: sandbox` page was rejected as the substitute:
  `location.origin` reports the URL's origin there, and making it opaque
  would change crawl semantics for sandboxed pages.
- **CI time.** The integration job downloads the driver and browser trees
  whenever `cli/target/design-automation-cache/` is cold (about 45 s locally);
  whether the cargo cache step keeps that directory was not checked.
- The executor repair could stand as its own PR if you would rather review
  the security fix apart from the filters work.
