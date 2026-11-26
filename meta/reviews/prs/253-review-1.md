---
type: "pr-review"
id: "253-review-1"
title: "[PP-957] Give every remaining test lane a --filter, and repair the design-automation runtime lane"
date: "2026-11-22T12:57:26+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:253"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "security", "test-coverage", "code-quality", "correctness", "standards", "usability", "documentation"]
review_number: 1
pr_number: 253
tags: []
last_updated: "2026-11-22T12:57:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #253 - [PP-957] Give every remaining test lane a --filter, and repair the design-automation runtime lane

**Verdict:** COMMENT

Both halves of this PR are well built. Every multi-test lane now takes `--filter`, rendered once per runner through `Selection`, and a guard test keeps future lanes from dropping it. The executor now judges and authenticates each request hop through CDP `Fetch`, and every failure path fails closed. No lens found a critical issue. Three major findings stand out. The pytest `--filter` example selects no test in any of the 11 lanes. The single-target `Fetch` session probably covers fewer targets than `page.route` did, which contradicts what the follow-up draft says. And no test would have failed against the original bearer leak on cross-origin redirects. The rest are minor: two "nothing selected" counters that count tests which never ran, stale comments and remediation hints, and design-automation helpers that `integration.py` imports from `unit.py` by their private names.

### Cross-Cutting Themes

- **Pytest `--filter` example selects nothing** (flagged by: usability, documentation, standards). `'resume and not slow'` appears in `PYTEST_FILTER_HELP`, in all 11 `mise.toml` `usage` blocks and in the `tasks/README.md` table, yet no test under `tests/` matches it. The guard checks only that an `e.g.` is present.
- **The security fix's guarantees are under-pinned** (flagged by: security, test-coverage, architecture, documentation). The rewrite lacks four things:
  - a regression test that fails against the old `page.route` leak;
  - a check of whether OOPIFs and workers were covered before;
  - a guard against a future `page.route` adding a second `Fetch` client;
  - a module header that states its single-target scope.
- **Design-automation helpers live in the wrong module** (flagged by: architecture, standards, code-quality). `integration.py` imports `_tap_counts`, `_bare_returns_in_tests` and `report_filtered_design_automation_run` from `unit.py`. Every other runner helper sits in `filters.py`.
- **Text left stale by the runtime lane's move** (flagged by: standards, documentation, usability):
  - the floor comment still says the lane "sits outside the enforced gate";
  - the remediation hints (`accelerator cache ensure`, `accelerator cache repair`) target the default cache root in `bin/`, which this lane deliberately keeps clear;
  - `executor_preflight.rs` still calls the lane opt-in.
- **Default build now needs the network** (flagged by: architecture, usability). A cold `cli/target/design-automation-cache/` turns `test:integration` and the bare `mise run` into a ~45 s download that fails if the network is down, and nothing tells the developer.

### Tradeoff Analysis

- **Coverage in the default build vs. needing the network**: moving the runtime lane into `test:integration` ends the silent rot that let all 37 of its tests fail unnoticed. The cost is that local and CI builds now depend on the artefact host being reachable. The move is worth that cost. A retry on `cache ensure`, a message that names a failed download as the cause, and an explicit CI cache step would make the tradeoff cheaper.
- **One PR vs. splitting it**: the description offers to split out the executor repair. Architecture favours a split, so the security change can be reviewed and reverted on its own. Splitting now mainly helps revertability, since the review is already done. Keep the PR whole unless the OOPIF question leads to further interception changes.

### Strengths

- ✅ `Selection._option` turns each runner rendering into a one-liner, and `run_pytest_lane`/`run_nextest_lane` collapse 11 pytest tasks and the contract lane into the same short call.
- ✅ `test_every_multi_test_leaf_declares_a_filter` and `test_every_task_that_takes_no_filter_exists` make the convention enforced and keep the exempt list from going stale.
- ✅ The "nothing selected" error is the same across runners and tells the developer what to do. An empty `--filter ''` is refused rather than running everything, and the `--filter="$usage_filter"` form is pinned by a test that runs it through `sh`.
- ✅ `makeRequestInterceptor` takes an injected `send`, so per-hop decisions are unit-tested without a browser. A forked-daemon test against a fake `playwright-core` covers the case where interception fails to start.
- ✅ The interceptor fails closed: a throwing classifier is treated as `malformed`, a refused continuation sends `Fetch.failRequest`, and `page` is set only after interception is installed. The handler is registered before `Fetch.enable`, so no hop can pause before it exists.
- ✅ `file://` is now refused in the daemon as well as in the Rust parser. A cross-origin bearer is stripped even when page script sets it.
- ✅ `run_libtest_lane` sums counts across both visualiser `cargo test` invocations, and a filtered design-automation run skips its floors only after its failures have been checked.
- ✅ `CLAUDE.md` and `tasks/README.md` link to each other rather than duplicating, and the README records which runner output each lane depends on.
- ✅ The `url` 2.5.9 bump clears RUSTSEC-2026-0592, and the notices are regenerated to match.

### General Findings

- 🔵 **Security**: the bearer is still attached regardless of initiator, so a cross-origin frame can borrow it by fetching the location origin. This predates the PR and is tracked in `PP-992`. As an interim tightening, attach it only when `event.frameId === mainFrameId`.
- 🔵 **Test Coverage**: the comment on the runtime test "a cross-origin redirect does not leak the header to the target" (`daemon-runtime.test.js:542`) still says the browser never forwards `Authorization` across origins. That contradicts the premise of this PR. Confirm the test fails with the old handler restored, then rewrite the comment.
- 🔵 **Test Coverage**: `node_tests_run` is tested only against flat TAP. There are no cases for indented subtests or a `not ok` suite-file point.
- 🔵 **Documentation**: `cli/design-cli/tests/executor_preflight.rs:6-8` still calls the runtime lane "opt-in".
- 🔵 **Usability**: the default `mise run` now has a cold-cache download and a network dependency that nothing mentions.
- 🔵 **Code Quality**: `filters.py` mixes help constants, runners, context managers and parsers with no per-runner grouping.
- 🔵 **Architecture**: each runner is split across `Selection`, `filters.py`, `mise.toml` and the README table. That is acceptable at today's six runners.
- 🔵 **Architecture**: the PR bundles two unrelated changes. See the tradeoff above.

### Additional Findings

- 🔵 `mise.toml:574` — `test:integration` and the default build now need the network (architecture)
- 🔵 `tasks/test/e2e.py:81` — `docker_visual_command` rebuilds a `Selection` from the raw filter (code-quality, standards)
- 🔵 `skills/design/inventory-design/scripts/playwright/lib/request-interception.js:92` — the interception error log does not say which hop failed (code-quality)
- 🔵 `tasks/test/filters.py:110` — "nothing selected" detection takes four different shapes (code-quality)
- 🔵 `tasks/test/integration.py:289` — a partial export and a failing `cache ensure` are not tested (test-coverage)
- 🔵 `skills/design/inventory-design/scripts/playwright/lib/request-interception.test.js:80` — the redirect-hop unit test does the same as the plain navigation test (test-coverage)
- 🔵 `skills/design/inventory-design/scripts/playwright/lib/daemon.js:335` — `snapshot` waits out the full wall clock on a document without `<body>` (correctness)
- 🔵 `skills/design/inventory-design/scripts/playwright/lib/request-interception.js:14` — the interceptor fakes a Playwright request for the classifier (architecture, code-quality)
- 🔵 `skills/design/inventory-design/scripts/playwright/lib/daemon.js:216` — the comment repeats what the interceptor's option names already say (standards, code-quality)
- 🔵 `skills/design/inventory-design/scripts/playwright/lib/daemon.js:202` — `ensureBrowser` now checks `page`, so its name no longer fits (code-quality)
- 🔵 `skills/design/inventory-design/scripts/playwright/lib/request-interception.js:1` — the module header claims "every request hop" without stating that it covers one target (documentation)
- 🔵 `skills/design/inventory-design/scripts/playwright/runtime-preflight.js:29` — the `accelerator cache repair` hint does not name the lane's cache root (documentation)
- 🔵 `skills/design/inventory-design/scripts/playwright/lib/request-interception.test.js:194` — the path where `failQuietly` swallows a refused failure is untested (test-coverage)
- 🔵 `tasks/test/filters.py:175` — `vitest_tests_run`, an output parser, raises a task-level `Exit` (code-quality)
- 🔵 `tasks/test/integration.py:278` — `_resolve_runtime_trees` does three jobs (code-quality)


## Inline Comments

### `tasks/test/filters.py:10-13` — The pytest `--filter` example selects no test in any of the 11 lanes
**Severity**: major | **Confidence**: high | **Lens**: usability, documentation, standards

🟡 **Usability / Documentation / Standards**

All 11 pytest lanes give `e.g. 'resume and not slow'` as their `--filter` example. It appears in `PYTEST_FILTER_HELP`, in every pytest lane's `mise.toml` `usage` block and in the `tasks/README.md` table. No test function, class or module under `tests/` contains `resume`. The word appears only in two pin-guard JSON fixtures. So a developer who copies the example from `mise run test:unit:tasks --help` gets `--filter 'resume and not slow' selected no tests`, and the help it then points to shows the same example again.

The PR description says each lane's example "selects a real test in that lane". That holds for the libtest, nextest, Vitest, node and Playwright lanes, but not for pytest. The guard checks only that the help contains `e.g.`.

**Suggestion**: give each pytest lane, or each group of lanes, an expression that selects real tests in it, as the libtest and node lanes do with separate unit and integration constants. Update `mise.toml` and the README table to match. A cheap guard would catch it from now on: run `pytest --collect-only -k <example>` per lane, or assert that the example's first bare word appears in that lane's test sources.

---

### `skills/design/inventory-design/scripts/playwright/lib/request-interception.js:84-102` — A single page-target `Fetch` session probably covers fewer targets than `page.route` did
**Severity**: major | **Confidence**: medium | **Lens**: security

🟡 **Security**

`interceptRequests` turns on `Fetch` only on the first page's own CDP target. In Chromium, Playwright's `page.route` turned interception on for every frame session it auto-attaches to. That includes out-of-process (cross-site) iframes and, in recent versions, dedicated workers. With no Playwright route registered, Playwright no longer intercepts anything. Once an OOPIF commits, its later document navigations and redirect hops, and worker requests, go out with no classifier check.

The PR's "Known gaps" note and `PP-992` both describe this target coverage as unchanged. For OOPIFs it is more likely a regression. The bearer still fails closed on those targets. What fails open is the navigation policy: the gate on internal, link-local and reserved hosts.

**Impact**: a cross-site iframe in the crawled app could navigate to a refused internal host, such as a link-local metadata address, without the classifier seeing it.

**Suggestion**: check coverage against the old `page.route` on Playwright 1.64.1. An OOPIF fixture that navigates to `INTERNAL_URL` after it loads would show it. If it is a regression, either close it here or narrow it cheaply now:
- call `Target.setAutoAttach({ autoAttach: true, flatten: true, waitForDebuggerOnStart: true })` and enable `Fetch` on each attached target;
- or launch with `--disable-site-isolation-trials`;
- and set `serviceWorkers: 'block'` on `newContext()`.

Then correct the draft's wording and priority.

---

### `skills/design/inventory-design/scripts/playwright/lib/request-interception.test.js:165` — No test fails against the original bearer leak on cross-origin redirects
**Severity**: major | **Confidence**: medium | **Lens**: test-coverage

🟡 **Test Coverage**

The leak came from Playwright carrying a `route.continue({ headers })` override onto later hops. This unit test checks only the per-hop *decision* (`shouldAttachHeader` against the hop URL). That decision was already made per URL before the fix, so this test would have passed against the vulnerable `makeAuthHeaderHandler` too.

The only end-to-end check is "a cross-origin redirect does not leak the header to the target" in `daemon-runtime.test.js`. Its unchanged comment (line 542) still says it is "not mutation-resistant: the browser does not forward Authorization across origins on a redirect". That contradicts the bug this PR fixes.

**Impact**: a change that brings back an override spanning hops, such as returning to `page.route`, could reintroduce the leak without any test failing.

**Suggestion**: confirm the runtime redirect test goes red with the old `page.route` handler restored, and rewrite its comment so it says it is the regression test for this leak. If it cannot be made red, drive `interceptRequests` with a fake CDP session that emits two `Fetch.requestPaused` events for one chain. Then assert that the second `continueRequest` carries no `authorization`.

---

### `tasks/test/filters.py:171` — `libtest_tests_run` counts `#[ignore]`d tests as run
**Severity**: minor | **Confidence**: medium | **Lens**: correctness

🔵 **Correctness**

`libtest_tests_run` adds `ignored` to `passed + failed`. libtest reports a test that matches the filter but is marked `#[ignore]` as `ignored`, not as run. So a `--filter` that selects only ignored tests produces a non-zero count, and the lane exits 0 without running anything. `vitest_tests_run` counts only passed and failed tests, so the two lanes disagree.

**Suggestion**: count `passed + failed`, as `vitest_tests_run` does. If counting ignored tests is intended, say so in the nothing-selected message.

---

### `tasks/test/filters.py:193-196` — `node_tests_run` counts `# SKIP` and `# TODO` points as tests run
**Severity**: minor | **Confidence**: medium | **Lens**: correctness

🔵 **Correctness**

`node_tests_run` counts every top-level `ok N - name` point except the suite-file wrappers. A point carrying a `# SKIP` directive, from `test.skip` or from `t.skip()` inside a test, is therefore counted as run. A filtered run also bypasses the skip floor. So a filter that selects only skipped tests passes with nothing executed, the outcome this lane's comments call indistinguishable from a pass.

**Suggestion**: leave out points with a `# SKIP` or `# TODO` directive. Alternatively, take the count from the TAP `# pass` and `# fail` summary lines, which `_tap_counts` already parses.

---

### `tasks/test/integration.py:303-321` — The runtime-tree error hides why `cache ensure` failed and suggests writing into `bin/`
**Severity**: minor | **Confidence**: medium | **Lens**: usability, documentation

🔵 **Usability / Documentation**

The lane runs `cache ensure driver browser` with `hide=True` and `ACCELERATOR_CACHE_DIR=cli/target/design-automation-cache`. On failure it prints only "Run `accelerator cache ensure driver browser`, then retry". The launcher's stderr is discarded, so a network, signature or disk failure never shows. The suggested command, run by hand, uses the default cache root, the shipped `bin/`. This lane moved away from `bin/` precisely because the `entrypoint` suite asserts that nothing writes there. Following the advice does not fill the lane's own cache, and it can make the entrypoint lane fail next.

**Suggestion**: include the launcher's stderr when `ensured.exited != 0`. Make the remedy match what the lane does, either `ACCELERATOR_CACHE_DIR=cli/target/design-automation-cache … cache ensure driver browser`, or "fix the cause shown, then re-run `mise run test:integration:design-automation`". The `accelerator cache repair` hint in `runtime-preflight.js` has the same problem.

---

### `tasks/test/integration.py:266-270` — Stale comment: the runtime lane is no longer outside the enforced gate
**Severity**: minor | **Confidence**: high | **Lens**: standards, documentation

🔵 **Standards / Documentation**

The comment on `_EXPECTED_DESIGN_AUTOMATION_RUNTIME_CASES` still says "this lane sits outside the enforced gate". This PR adds the lane to `test:integration`, so it now runs in the default task and on both CI legs.

**Suggestion**: drop the clause, or reword the rationale so it does not depend on the lane's place in the gate. For example: `node --test` reports a vanished suite or an early-returning test as passing, so the floor and the bare-return guard are what catch them.

---

### `tasks/test/integration.py:24-28` — `integration.py` imports private and shared design-automation helpers from `unit.py`
**Severity**: minor | **Confidence**: high | **Lens**: architecture, standards, code-quality

🔵 **Architecture / Standards / Code Quality**

`integration.py` imports `_bare_returns_in_tests` and `_tap_counts`, both private by convention, from its sibling `unit.py`. This PR adds a third import, `report_filtered_design_automation_run`. Every other runner helper this PR adds (`node_tests_run`, `libtest_tests_run`, `vitest_tests_run`, the `nothing_selected_*` context managers) lives in `filters.py`. Changing the unit lane's internals can now silently break the runtime lane. The reporter's name also undersells it, since it can fail the task with `Exit`.

**Suggestion**: move the TAP helpers and the filtered-run reporter into `filters.py`, next to `node_tests_run`, or into a small `tasks/test/design_automation.py`. Give them public names, and consider a name for the reporter that says it can refuse the run.

---

### `tasks/test/filters.py:16-52` — Most lanes' filter help is copied by hand into `mise.toml` with no equality check
**Severity**: minor | **Confidence**: high | **Lens**: architecture

🔵 **Architecture**

Each runner's `--filter` help is defined twice: as a constant here, and again by hand in each lane's `mise.toml` `usage`. `test_mise.py` checks that the two copies match only for the pytest lanes and `test:unit:cli`. For the libtest, nextest-contract, Vitest, node and Playwright lanes, it checks only that the help contains a phrase.

**Impact**: an example meant to select a real test (`'spa_route'`, `'chip-showcase'`) can drift between `mise run <task> --help` and `invoke --help <task>` with nothing failing.

**Suggestion**: extend the existing check that the mise help equals the invoke help to every lane that takes a filter.

---

### `skills/design/inventory-design/scripts/playwright/lib/request-interception.js:84-102` — Nothing enforces that Playwright routing never runs alongside this `Fetch` client
**Severity**: minor | **Confidence**: medium | **Lens**: architecture

🔵 **Architecture**

Playwright's own routing also runs on the `Fetch` domain. `interceptRequests` is correct only because nothing now calls `page.route` or `context.route`. The old design wrote down its handler-ordering rule in a comment. The new rule, that Playwright routing must never be used on this page, is neither written down nor checked anywhere.

**Impact**: a later `page.route`, added for request mocking for example, would put a second `Fetch` client on the same target. Hops could then be paused twice or released by the wrong handler. That is a quiet failure with security consequences.

**Suggestion**: add a source-scanning test like the existing "reads neither retired auth env var" one, failing if `daemon.js` or `lib/` calls `.route(` or `.unroute(`. At minimum, state the rule in this module's header.

---

## Per-Lens Results

### Architecture

**Summary**: Structurally sound. Runner syntax is rendered once in `Selection`, a guard test enforces the `--filter` convention, and CDP is confined to `request-interception.js`, which fails closed. The weak spots are small: `integration.py` imports `unit.py`'s private helpers, a rule the interceptor depends on goes unguarded, the help text is duplicated by hand, and the default build now needs the network.

**Comments**:
- 🔵 minor/high `tasks/test/integration.py:24-28`: `integration.py` imports another task module's private helpers
- 🔵 minor/high `tasks/test/filters.py:16-52`: most lanes' filter help is copied into `mise.toml` with no equality check
- 🔵 minor/medium `request-interception.js:84-102`: nothing enforces that Playwright routing is never used alongside the `Fetch` interceptor
- 🔵 suggestion/medium `request-interception.js:14-19`: the navigation policy still takes a Playwright-shaped request
- 🔵 minor/medium `mise.toml:574`: `test:integration` and the default build now need the network

**General Findings**: each runner is spread across two modules (suggestion); one PR bundles two unrelated changes (minor).

### Security

**Summary**: Per-hop `Fetch` interception fixes the cross-origin redirect bearer leak and the redirect hops that went unjudged, and it fails closed throughout. The swap probably narrows which targets the navigation policy covers (OOPIFs, workers), which the PR and its draft describe as unchanged. The filter plumbing introduces no injection paths.

**Comments**:
- 🟡 major/medium `request-interception.js:84-102`: a single page-target `Fetch` session probably covers fewer targets than `page.route` did

**General Findings**: the bearer is still attached regardless of initiator, tracked in `PP-992`. Interim fix: attach it only for `frameId === mainFrameId` (suggestion).

### Test Coverage

**Summary**: Strong, mostly test-first coverage. Every lane is tested against every behaviour, the interceptor has a precise unit suite, and the daemon's recovery from a failed interception setup is tested by forking it. The main gap is that no test would fail against the original cross-origin bearer leak.

**Comments**:
- 🟡 major/medium `request-interception.test.js:165`: no test fails against the pre-fix bearer leak
- 🔵 minor/high `request-interception.test.js:80`: the redirect-hop unit test is identical in effect to the plain navigation test
- 🔵 suggestion/medium `request-interception.test.js:194`: the path where `failQuietly` swallows a refused failure is untested
- 🔵 minor/high `tasks/test/integration.py:289`: a partial export and a failing `cache ensure` are not covered

**General Findings**: the comment on the runtime cross-origin redirect test contradicts the fix (minor); `node_tests_run` is tested only against flat TAP (suggestion).

### Code Quality

**Summary**: Well factored. `_option` and the `run_*_lane` helpers keep the tasks uniform, and replacing the two `page.route` handlers removes a fragile dependency on their order. The remaining issues are about consistency and where things live.

**Comments**:
- 🔵 minor/high `tasks/test/unit.py:176-185`: the filtered-run reporter lives in `unit.py` but is shared
- 🔵 minor/medium `tasks/test/filters.py:110-133`: "nothing selected" detection takes four different shapes
- 🔵 minor/high `tasks/test/e2e.py:81-83`: `docker_visual_command` rebuilds a `Selection`
- 🔵 suggestion/medium `tasks/test/filters.py:175-178`: an output parser raises a task-level `Exit`
- 🔵 minor/high `request-interception.js:92-98`: the interception error log omits which hop failed
- 🔵 suggestion/medium `request-interception.js:14-19`: CDP events are dressed up as Playwright requests
- 🔵 suggestion/medium `daemon.js:202-203`: `ensureBrowser` now checks `page`, so its name and state no longer match
- 🔵 suggestion/medium `daemon.js:216-221`: a narrative comment restates what the option names already say
- 🔵 suggestion/low `tasks/test/integration.py:278-328`: `_resolve_runtime_trees` mixes three responsibilities

**General Findings**: `filters.py` has become a grab-bag module (suggestion).

### Correctness

**Summary**: The interception rewrite is logically sound: every hop is judged, the bearer is decided per hop, refused continuations are failed, and `ensureBrowser` is keyed on `page`. Every runner's "nothing selected" signal is detected correctly. Two counters count tests that never ran, and `snapshot` can wait out the whole wall clock.

**Comments**:
- 🔵 minor/medium `tasks/test/filters.py:171`: the libtest count treats `#[ignore]`d tests as run
- 🔵 minor/medium `tasks/test/filters.py:193-196`: the TAP counter treats `# SKIP` and `# TODO` points as run
- 🔵 minor/low `daemon.js:335-337`: `snapshot` waits out the full wall clock on a document without `<body>`

### Standards

**Summary**: The `--filter` convention is applied evenly: `usage`, a `filter_` option with matching help, and `=`-form forwarding. The main departure is the pytest example. There is also a stale comment, and some helper placement and comments drift from convention.

**Comments**:
- 🔵 minor/high `mise.toml:335`: the shared pytest help example selects no test (merged into the inline comment at `filters.py:10`)
- 🔵 minor/high `tasks/test/integration.py:266-269`: stale comment about the enforced gate
- 🔵 suggestion/medium `tasks/test/integration.py:24-28`: a shared lane helper lives in `unit.py`
- 🔵 suggestion/medium `daemon.js:216-221`: the comment restates what the code already says
- 🔵 suggestion/low `tasks/test/e2e.py:70`: `docker_visual_command` takes a raw `grep` string

### Usability

**Summary**: A clear improvement: one flag, per-runner help, a consistent error message, and a guard test. Two things undercut it: the pytest example does not work, and the runtime-tree error hides its cause and suggests writing into `bin/`.

**Comments**:
- 🟡 major/high `tasks/test/filters.py:10-13`: the pytest `--filter` example selects no test
- 🔵 minor/high `mise.toml:335`: the same placeholder example is copied into 11 lanes' help
- 🔵 minor/medium `tasks/test/integration.py:303-319`: the runtime-tree error hides its cause and suggests writing into `bin/`

**General Findings**: the default `mise run` now has a cold-cache download and a network dependency that nothing mentions (suggestion).

### Documentation

**Summary**: The docs stay closely in step with the code. The new "Running a subset of tests" sections in `CLAUDE.md` and `tasks/README.md` link to each other, and `PROTOCOL.md` documents the new `snapshot` shape. One example is wrong, and a few comments and hints still describe the lane's old status or point at the cache root the lane avoids.

**Comments**:
- 🟡 major/high `tasks/test/filters.py:10-13`: the pytest example selects no test
- 🔵 minor/high `tasks/README.md:129`: the README table repeats that example
- 🔵 minor/high `tasks/test/integration.py:268-270`: the floor comment says the lane sits outside the gate
- 🔵 minor/medium `tasks/test/integration.py:315-321`: the remediation hint points at the default cache root
- 🔵 suggestion/medium `runtime-preflight.js:29-30`: the `cache repair` hint does not name the lane's cache root
- 🔵 suggestion/medium `request-interception.js:1-7`: the header claims "every request hop" without stating that it covers one target

**General Findings**: `executor_preflight.rs` still calls the runtime lane opt-in (minor).

---
*Review generated by /accelerator:review-pr*
