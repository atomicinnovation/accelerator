---
type: "pr-review"
id: "157-review-1"
title: "[0225] Guard the vendored runtime pins with scheduled advisory and age checks"
date: "2026-10-12T17:49:35+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:157"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "security", "safety", "documentation"]
review_number: 1
pr_number: 157
tags: []
last_updated: "2026-10-12T17:49:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #157 - [0225] Guard the vendored runtime pins with scheduled advisory and age checks

**Verdict:** COMMENT

The pin guard is a well-built ports-and-adapters design. Every check and abort runs before the first issue write, and feed failures are first-class values that darken only the checks that depend on them. Boundary-heavy tests and recorded fixtures cover most rules, and no lens found a logic error in the range, OSV, Chromium-version or reconciliation matching. The main concerns are about how suppression and failure behave over time, not about today's matching: a closed advisory issue silences that CVE for the pin at every version, one OSV record without an `affected` field can blind the whole Chromium check, and a change made for the guard silently tightens the release lane's packument fetch timeout.

### Cross-Cutting Themes

- **Permanent, version-blind suppression** (flagged by: safety, security) — The advisory marker is `(advisory, id, pin.name)` and any bot-authored issue in any state suppresses it. A revert or downgrade to a vulnerable pin is never reported again. Anyone with triage access can close or edit an issue and silence a finding for good. The planned bulk close of 16 "to assess" CVEs on the first run turns each closure into an irreversible risk acceptance.
- **Failures that silence more than they should** (flagged by: correctness, safety, code-quality) — One OSV record without `affected` fails the whole Chromium check. A gpg or `Owner:` problem aborts every advisory check, leaving only a red run. An owner who cannot be assigned blocks every issue, outage issues included. Each case contradicts the design principle that "a failed feed darkens only its own check".
- **Feed text rendered into issue bodies** (flagged by: security, code-quality) — `code_span` strips `` ` ``, `<` and `>` but not newlines, so a feed value containing a blank line escapes the span into live Markdown. Outage reasons echo rejected values word for word, and the broad `ValueError` catch drops the useful detail while keeping the risky part.
- **`FeedOutage` drafting lives in the transport module** (flagged by: architecture, code-quality) — `feeds.py` imports `findings` and `Owner` only to render outage bodies. Issue drafting is spread across `findings.py`, `feeds.py` and `issues.py`.

### Tradeoff Analysis

- **Alert fatigue vs re-alerting**: Making the marker version-blind deliberately stops a fresh issue on every bump while a CVE remains unfixed. Adding the version (or "suppressed only at or above the version it was closed against") would re-raise reverts and downgrades at the cost of one issue per bump. Recommendation: record the version the issue was closed against, which keeps the low noise and catches regressions.
- **Avoiding dropped output vs latency**: The `command_runner.rs` grace period now waits for pipe EOF. That fixes the flaky test, but a `setsid` descendant holding a pipe open now costs the full `GRACE`. Near the deadline the function can still return truncated `Ok`. Accept the latency; consider reporting undrained pipes as an error.

### Strengths

- ✅ Clean hexagonal split: `GuardPorts` bundles the tracker, key lister, feed client and clock as protocols, and `wiring.py` is the only place adapters are built. The dry-run tracker decorates the real one.
- ✅ Structure enforces "abort before the first write" (`read_local_inputs` → `evaluate` → `reconcile`), and tests check that the tracker records no calls on each abort path.
- ✅ `CheckOutcome` cannot carry both findings and failures. `FeedBudget` adds a run deadline and a three-strike rule that skips a dead feed. Drifted or empty feeds count as failures, never as clear.
- ✅ No lens found a logic error in the matching. npm range parsing, OSV event ordering, the `comparable_fix` major selection, gpg colon-field indices and `type(value) is dt.date` bump validation all check out.
- ✅ Least-privilege workflow: `permissions: {}` at workflow level, actions pinned to SHAs and enforced by a new test, `persist-credentials: false`, dispatch input passed through an env var, and a concurrency group.
- ✅ Feed identifiers are regex-validated (ASCII-only CVE/GHSA ids, an OSV id that must equal the queried id), and markers refuse characters outside `[A-Za-z0-9._-]`. Only the marker on the last line of a bot-authored issue counts.
- ✅ Tests cover boundaries on both sides (90/91, 365/366, 60/61, cap 10/11). Tests also tie RELEASING.md claims to code constants and keep the networked guard out of `check`/`default`.
- ✅ `BrowsersManifest` replaces three dead functions and finally performs the `browsers.json` cross-check that the `chromium.py` docstring claimed.
- ✅ The `command_runner.rs` fix comes with deterministic regression tests that fail without it.

### General Findings

- 🟡 **Architecture**: `fetch.get_json` is shared with the release lane (`upstream.py:90`). It used to call `httpx.get(timeout=60)`, where 60 s limits each socket read. It now also stops the whole body after 60 s in total. Verified in the code. See the inline comment.
- 🔵 **Safety**: The first live run trips the cap with 16 "to assess" CVEs. Closing them in bulk makes each one a permanent, version-blind suppression. Prefer the KEV `dateAdded` or CVE-year cutoff before merging.
- 🔵 **Safety**: A failure to list keys with gpg, or a malformed `Owner:` line, aborts every advisory check and files no issue. Consider treating a key-listing failure like a feed failure.
- 🔵 **Safety**: Every create passes `--assignee`. An owner who cannot be assigned therefore blocks all issues, outages included, and assigning with `GITHUB_TOKEN` has not yet been confirmed on Actions. Consider retrying without the assignee.
- 🔵 **Security**: Permanent suppression rests on issue state and body, which any triager can change, and on the `github-actions[bot]` identity, which every workflow shares. At minimum, document that no other workflow may hold `issues: write`.
- 🔵 **Correctness**: In `command_runner.rs`, a grace period that times out with undrained pipes (for example, the leader exiting within one TICK of the deadline) still returns `Ok` with truncated output.
- 🔵 **Documentation**: "Refreshing the anchors" still frames the Node pin as fixed by the Playwright pairing, while this PR bumps Node alone.
- 🔵 **Documentation**: The keyring-age issue names `npm-registry.pem`, but RELEASING.md calls the subject "Node keyring" only. Say once what `keyring.bumped` covers.
- 🔵 **Code Quality**: The CVE regex is duplicated, and the two advisory finding types each build the advisory marker separately, although that shared identity is a deliberate invariant. Extract `advisory_marker(...)`.
- 🔵 **Architecture**: The owner login is parsed from the prose of RELEASING.md. Consider an `owner` key in `pins.toml`.
- 🔵 **Test Coverage**: Pin-guard test helpers (`BOT`, `OWNER`, `_filed`, reconcile wrappers) are duplicated across four test files.
- 🔵 **Architecture**: Adding a fourth pin touches `PinName`, `LocalInputs`, `read_local_inputs` and `evaluate`. Consider a test asserting that every `PinName` has both an age check and an advisory check.

### Additional Findings

- 🔵 `tasks/shared/vendor/pin_guard/keyring.py:62` — gpg failure takes every advisory check offline (safety)
- 🔵 `tasks/shared/vendor/pin_guard/github_issues.py:91` — Owner who cannot be assigned blocks every issue (safety)
- 🔵 `tasks/shared/vendor/pin_guard/issues.py:150` — Guard-tripped body unbounded; a new tripped issue opens whenever the held-back set changes (safety)
- 🔵 `tasks/shared/vendor/pin_guard/issues.py:79` — Suppression rests on mutable issue state (security)
- 🔵 `tasks/shared/vendor/pin_guard/feeds.py:89` — Outage drafts render unvalidated feed text (security)
- 🔵 `tasks/shared/vendor/pin_guard/chromium_advisories.py:94` — "No KEV entry is a Chromium component" is reported as a missing field (code-quality)
- 🔵 `tasks/shared/vendor/pin_guard/feeds.py:53` — Feed transport module also drafts issue bodies (architecture, code-quality)
- 🔵 `tasks/shared/vendor/pin_guard/chromium_advisories.py:41` — Chromium check imports generic OSV helpers from the playwright module (architecture)
- 🔵 `tasks/shared/vendor/pin_guard/findings.py:13` — RELEASING URLs hard-code the upstream repo while `GH_REPO` is configurable (architecture)
- 🔵 `.github/workflows/runtime-pin-guard.yml:36` — Guard installs the whole mise toolchain (architecture)
- 🔵 `cli/config-adapters/src/command_runner.rs:352` — Grace period waits on a pipe that an escaped descendant can hold open; may return truncated `Ok` (architecture, correctness)
- 🔵 `tasks/shared/vendor/pin_guard/local_inputs.py:177` — `AttributeError` catch works around a weakly typed parser; `TypeError` escapes (code-quality)
- 🔵 `tasks/shared/vendor/pin_guard/node_advisories.py:106` — Node issues titled by a bare vuln/core key (documentation)
- 🔵 `tasks/shared/vendor/pin_guard/findings.py:93` — Keyring-age body vs RELEASING.md subject mismatch (documentation)
- 🔵 `cli/config-adapters/src/command_runner.rs:495` — Combined-cap test depends on pipe capacity above 16 KiB (test-coverage)
- 🔵 `RELEASING.md:269` — "more than 10" hard-codes the configurable cap (documentation)
- 🔵 `tasks/README.md:389` — Throwaway-repo recipe still assigns to the RELEASING.md owner (documentation)
- 🔵 `tasks/shared/vendor/pin_guard/feeds.py:149` — Several docstrings restate their implementations (documentation)
- 🔵 `tasks/shared/vendor/pin_guard/findings.py:69` — Age/key issues link to the section top, not "Acting on an issue" (documentation)
- 🔵 `.github/workflows/runtime-pin-guard.yml:8` — `maximum-new-issues` has no upper bound (safety)
- 🔵 `tasks/vendor/commands.py:144` — Malformed `--today` untested (test-coverage)
- 🔵 `tests/unit/tasks/shared/test_clock.py:19` — `today_or_now(None)` test races UTC midnight (test-coverage)
- 🔵 `tasks/shared/vendor/pin_guard/guard.py:69` — The generator variable `failed` shadows the tuple it builds (code-quality)
- 🔵 `tasks/shared/vendor/pin_guard/wiring.py:20` — Two `Clock` instances in one run (code-quality)
- 🔵 `tasks/shared/vendor/gpg.py:33` — `_KEY_RECORDS` values silently mean `is_primary` (code-quality)
- 🔵 `cli/config-adapters/src/command_runner.rs:442` — Test helper selects the stream with a string flag (code-quality)

---
*Review generated by /review-pr*

## Inline Comments

### `tasks/shared/vendor/pin_guard/findings.py:154-158` — Advisory suppression ignores pin version
**Severity**: major | **Confidence**: high | **Lens**: safety (+ security)

🟡 **Safety**

The advisory marker is `("advisory", advisory_id, pin.name)`, without the pin version, and reconciliation treats an issue in any state, open or closed, as permanent suppression. Suppose the owner bumps past the fix and closes the issue as RELEASING.md instructs, and the bump is later reverted, for example because a new Playwright broke something. The vulnerable version is then live again and the guard stays silent. `UnassessableAdvisoryFinding` shares the same marker, so a CVE closed by hand as "not affected" also stays silent if OSV later publishes a fix showing the pin is affected. Reopening a closed issue does not undo suppression either.

**Impact**: A confirmed vulnerable pin can come back without an alert. Recovering means someone notices by hand and edits a closed issue.

**Suggestion**: Keep the version-blind marker to avoid one issue per bump, but record the pin version the issue was closed against (for example, in a closing comment or a second marker line) and suppress only versions at or above it. Also document how to undo a suppression.

### `tasks/shared/vendor/fetch.py:54-64` — Shared `get_json` change silently tightens the release lane's timeout
**Severity**: major | **Confidence**: high | **Lens**: architecture

🟡 **Architecture**

`get_json` is still the default `fetch_json` in `upstream.verify_upstream_inputs` (`upstream.py:90`), which fetches the full `playwright-core` packument and the SLSA attestations. It used to call `httpx.get(url, timeout=60)`, where the 60 s limit applied to each socket operation. It now goes through `_body`, which also treats `timeout` as a wall-clock total for the whole body. The `playwright-core` packument is large (thousands of nightly versions), so a slow CI network could now hit a `ReadTimeout` that never fired before. The PR description presents the fetch changes as guard infrastructure only.

**Impact**: A change made for a monitoring job can fail a release on a slow download.

**Suggestion**: Have `verify_upstream_inputs` pass an explicit, generous total, or keep a per-read-only path for the release lane. Add a test that pins the release lane's timeout budget.

### `tasks/shared/vendor/pin_guard/feeds.py:81-86` — Feed-identity check in outage suppression is untested
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

Every `is_covered_by` test puts the standing issue and the new failure on the same feed (`osv`). No test has an open outage issue for one feed alongside a failure on another feed that skips the same check. Deleting `marker.parts[0] == self.feed` leaves every test green.

**Impact**: Under that regression, an open KEV outage issue (`kev chromium-advisories`) would silently suppress a later OSV outage that also darkens `chromium-advisories`. That is the "dark feed hidden behind a standing issue" failure this module exists to prevent.

**Suggestion**: Add `test_an_open_issue_for_another_feed_does_not_suppress`. File a KEV outage marker for the Chromium check as open, reconcile an OSV failure for the same check, and assert that the OSV outage opens.

### `tasks/shared/vendor/pin_guard/findings.py:36-39` — `code_span` does not neutralise line breaks
**Severity**: minor | **Confidence**: high | **Lens**: security

🔵 **Security**

`code_span` promises one inert code span "whatever it contains", but strips only `` ` ``, `<` and `>`. A blank line (`\n\n`) ends the paragraph, so the code span is never closed and the rest renders as live Markdown. Outage reasons echo rejected feed values verbatim: `unexpected advisory id {id}`, `unparseable version {text}`, `unexpected entry key {key}`, `unparseable range {text}`, and others. A poisoned value such as `x\n\n@org/everyone [advisory](https://evil.example)` would therefore produce a `github-actions[bot]` issue containing live mentions and links. Echoed values are also unbounded, so one huge value can push the body past GitHub's 65,536-character limit. The existing test covers only single-line input.

**Impact**: A compromised or poisoned feed can post phishing or mention-spam content under the repository's bot identity, or stop an outage issue from being filed at all.

**Suggestion**: Collapse whitespace and control characters, and cap the length, e.g. `re.sub(r"[\s\x00-\x1f\x7f]+", " ", ...)[:200]`. Add a test with `\n\n@mallory`.

### `tasks/shared/vendor/pin_guard/findings.py:172` — "Bump the pin past the fix" even when no fix exists
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

When OSV gives no fixed version, `_remedy` renders "No fixed version is available." The unconditional paragraph after it still says "Bump the pin past the fix and close this issue once the bump merges". RELEASING.md's Advisory bullet covers only the case where a fix exists.

**Impact**: For an advisory with no fix, the owner is told to do something impossible, with no guidance on the real choices: wait, mitigate, or record an explicit risk acceptance.

**Suggestion**: Make the owner paragraph depend on `fix`, and add a matching sentence under "Advisory" in RELEASING.md.

### `tasks/shared/vendor/pin_guard/http_feeds.py:80-83` — Non-transient HTTP errors have no test
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

The retry tests use only 503, 429 and 404. No test shows that a 403 or 400 fails immediately without sleeping, and the `>= 500` boundary is never exercised with a 500. If `_is_transient` returned `True` for every status, or `>=` became `>`, no test would fail.

**Impact**: A regression could retry authentication failures for ~10 s per request, eating into the 10-minute deadline, or stop retrying a plain 500.

**Suggestion**: Add `test_a_403_is_unreachable_without_a_retry` (asserting `clock.sleeps == []`), and parametrise the 503 → 200 test over 500 and 503.

### `tasks/shared/vendor/pin_guard/npm_ranges.py:141-142` — Strict `>` comparator branch has no test
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

`test_each_form_bounds_the_versions_it_holds` covers bare versions, `^`, `>=`, `<=` and `<`, but never `>`, and the recorded `vuln/core` feed contains no `>` comparator. Under TDD, the `case ">"` branch is production code with no failing test behind it. If it changed to `>= lowest`, the pin at exactly X would wrongly fall inside the range.

**Suggestion**: Add `(">10.9.0", ["10.9.1"], ["10.9.0"])` and `(">10", ["11.0.0"], ["10.99.99"])`, or refuse `>` the way `~` and `=` are refused.

### `tasks/shared/vendor/pin_guard/http_feeds.py:72-73` — Broad `ValueError` catch discards the parse error detail
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

`_retrying` turns every `ValueError` into `FeedDocumentError("unparseable")`. The original message (the JSON decode position, or a Unicode error) never reaches the outage issue, and a `ValueError` caused by a bug is absorbed as feed drift.

**Suggestion**: Catch `json.JSONDecodeError` and `UnicodeDecodeError` specifically. Carry a bounded, sanitised `str(error)` into `FeedFailure.detail`, as the code already does for unreachable feeds.

### `tasks/shared/vendor/pin_guard/chromium_advisories.py:144-157` — The fetch-then-gather loop is duplicated across checks
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

This block repeats the one in `osv.playwright_core_advisories`: collect records and failures, return `CheckOutcome.failed` if any request failed, otherwise map the records to findings. Only the record type and the mapping differ.

**Impact**: The partial-failure policy now lives in two places that can drift, and a third feed would copy it again.

**Suggestion**: Extract a single helper next to `CheckOutcome`, e.g. `CheckOutcome.gather(ids, fetch, to_findings)`.

### `tasks/shared/vendor/pin_guard/chromium_advisories.py:109` — OSV record without `affected` fails the whole Chromium check
**Severity**: minor | **Confidence**: medium | **Lens**: correctness

🔵 **Correctness**

`of_record` reads `affected` with `field(..., "affected", list)`, which raises `MissingFieldError` when the key is absent. The OSV schema makes `affected` optional, and NVD-derived records can omit it. Lines 144–152 then turn any single per-CVE failure into `CheckOutcome.failed`, discarding every other KEV CVE's findings for that run.

**Impact**: One sparse record for a newly added KEV CVE blinds the Chromium check every day. Once the first outage issue is open it suppresses the repeats, so exploited CVEs with known fixes go unreported until someone investigates.

**Suggestion**: Treat a missing or empty `affected` like a record with no four-part fix (`UnassessableAdvisoryFinding`).

---

## Per-Lens Results

### Architecture

**Summary**: The new `pin_guard` package has a clean ports-and-adapters design. Every network call, the gh CLI and gpg sit behind small protocols that are wired in one place, so the evaluate-then-reconcile core can be tested without I/O. Its main safety rule holds by structure: every check runs, and every abort fires, before the first write. The biggest risk is outside the package: the shared `fetch.get_json` was rewritten for the guard, and that silently tightens the release lane's packument fetch timeout. The other concerns are smaller issues of cohesion and coupling.

**Strengths**:
- `GuardPorts` bundles the tracker, key lister, feed client and clock as protocols; `wiring.py` is the only place production adapters are built; the dry-run tracker is a decorator.
- "Abort before the first issue is written" is enforced by structure in `run_guard`.
- Feed failures are first-class values (`FeedFailure`, `FeedOutage`, `CheckOutcome`). `FeedBudget` adds a deadline and a three-strike rule that skips a dead feed.
- Reconciliation keyed on markers is idempotent, and the concurrency group prevents races between runs.
- `BrowsersManifest` is a single shared parser that replaces three dead functions.
- `test_mise.py` keeps the networked guard out of aggregate tasks.

**Comments**:
- 🟡 major/medium `tasks/shared/vendor/fetch.py:54-64` — Shared `get_json` change silently tightens the release lane's timeout (see Inline Comments).
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/feeds.py:54-108` — The feed transport module also drafts GitHub issue bodies, importing `findings` and `Owner`; issue drafting is split across three modules. Move `FeedOutage`/`draft` to `outages.py` or `issues.py`.
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/chromium_advisories.py:41` — The Chromium check imports `record_of`/`record_url` from the playwright-specific `osv.py`. Extract `osv_records.py`.
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/local_inputs.py:55-64` — The owner is parsed from RELEASING.md prose, so an ordinary docs edit can abort the guard. Consider a `[guard] owner` key in `pins.toml`.
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/findings.py:13-19` — RELEASING URLs hard-code `atomicinnovation/accelerator` on `main`, while `GH_REPO` is configurable. Pass a rendering context into `draft`.
- 🔵 minor/medium `.github/workflows/runtime-pin-guard.yml:36-40` — `mise-action` installs the whole toolchain, so an unrelated tool failure blocks the monitor. Use `install_args: "python uv gh"`.
- 🔵 minor/low `cli/config-adapters/src/command_runner.rs:352-355` — The grace period now waits for pipe EOF, which an escaped `setsid` descendant can hold open, costing the full `GRACE`. Confirm no caller is affected, and optionally pin the behaviour with a test.

**General Findings**:
- 🔵 suggestion — Adding a fourth vendored pin touches `PinName`, `LocalInputs`, `read_local_inputs` and `evaluate`. Consider a registry, or a test asserting that every `PinName` has both an age check and an advisory check.

### Code Quality

**Summary**: The design is good, with Protocol seams, small frozen domain types and mostly single-responsibility modules. The findings concern maintainability: a duplicated fetch-then-gather loop, error paths that lose detail or report a misleading cause, and broad exception catches. None of them block the PR.

**Strengths**:
- Ports and adapters are applied consistently; `wiring.py` is the single composition root.
- Clear domain language (`PinAgeFinding`, `KeyExpiryFinding`, `UnassessableAdvisoryFinding`, `FeedOutage`, `FeedBudget`, `IssueMarker`). The invariant in `CheckOutcome.__post_init__` guards the rule that failed checks carry no findings.
- Errors are typed by category and translated in one place (`FeedSession.request`).
- Dead code is removed and duplication consolidated (`BrowsersManifest`, `today_or_now`, `PLAYWRIGHT_PACKAGE_JSON`, `_assert_sha512_matches`).
- Comments are rare and the ones kept explain non-obvious reasons.

**Comments**:
- 🔵 minor/high `tasks/shared/vendor/pin_guard/http_feeds.py:72-73` — Broad `ValueError` catch discards the parse error detail (see Inline Comments).
- 🔵 minor/high `tasks/shared/vendor/pin_guard/chromium_advisories.py:144-157` — The fetch-then-gather loop is duplicated with `osv.py` (see Inline Comments).
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/chromium_advisories.py:94` — "No KEV entry is a Chromium component" is raised as `MissingFieldError("vulnerabilities[].product")`; `vuln_core_entries` does the same for renamed environments. Use a `FeedDocumentError` that names the real condition.
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/feeds.py:53-108` — `feeds.py` mixes transport errors, request budgeting and issue drafting. Move `FeedOutage` out.
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/local_inputs.py:177-183` — The `AttributeError` catch works around a weakly typed `pinned_playwright_version`, and a non-string `playwright` value still escapes as a `TypeError`. Validate the shape explicitly.
- 🔵 suggestion/high `tasks/shared/vendor/pin_guard/guard.py:69-72` — The generator variable `failed` shadows the tuple it builds.
- 🔵 suggestion/medium `tasks/shared/vendor/pin_guard/wiring.py:20-21` — Two separate `Clock()` instances in one run; build one and share it.
- 🔵 suggestion/medium `tasks/shared/vendor/gpg.py:33` — The values in `_KEY_RECORDS` silently mean `is_primary`; rename it or use a frozenset.
- 🔵 suggestion/low `cli/config-adapters/src/command_runner.rs:442-443` — `burst(bytes, stream: &str)` silently falls back to stdout for any other string; use an enum or two helpers.

**General Findings**:
- 🔵 minor — The CVE regex is duplicated (`CVE_ID`, `_CVE`). Both advisory finding types build the advisory marker by hand, although that shared identity is a deliberate invariant. The keyring path is spelled twice. Extract `advisory_marker(...)` and shared constants.

### Test Coverage

**Summary**: Coverage is strong and matches the risk. Boundaries, the cap, the deadline, the three-failure skip, author trust, open vs closed suppression and abort-before-write ordering are all tested against recorded fixtures and fakes. The Rust fix has deterministic regression tests. A mutation pass still finds rules no test protects: the feed-identity check in outage suppression, non-transient HTTP errors, and the strict `>` range branch.

**Strengths**:
- Paired boundary tests at 90/91, 365/366, 60/61 (including 23:59 on day 61), cap 10/11 and the deadline.
- Tests verify abort-before-write ordering as behaviour.
- `test_a_failing_request_darkens_only_its_own_check` pins the feed isolation contract end to end.
- Careful doubles: `FakeIssueTracker` enforces `ensure_label` first; the fake world defaults to populated and clear, guarded by its own test.
- The `command_runner.rs` regression tests fail without the fix.
- Tests keep docs and code in step.
- Hostile inputs are covered: echoed markers, mentions, non-ASCII digits, symlinks in tarballs, and tarballs on non-registry hosts.

**Comments**:
- 🟡 major/high `tasks/shared/vendor/pin_guard/feeds.py:81-86` — The feed-identity check in outage suppression is untested (see Inline Comments).
- 🔵 minor/high `tasks/shared/vendor/pin_guard/http_feeds.py:80-83` — Non-transient HTTP errors have no test (see Inline Comments).
- 🔵 minor/high `tasks/shared/vendor/pin_guard/npm_ranges.py:141-142` — The strict `>` comparator branch has no test (see Inline Comments).
- 🔵 minor/low `cli/config-adapters/src/command_runner.rs:495-504` — The combined-cap test relies on the kernel giving each pipe at least ~40 KB, which is not guaranteed under pipe-buffer pressure. Document why 40 000/30 000 were chosen, or restructure the test.
- 🔵 suggestion/medium `tasks/vendor/commands.py:144` — A malformed `--today` is untested and probably raises an uncaught `ValueError`.
- 🔵 suggestion/medium `tests/unit/tasks/shared/test_clock.py:19-20` — The `today_or_now(None)` test races UTC midnight; bracket the call or inject `now`.

**General Findings**:
- 🔵 suggestion — `BOT`, `OWNER`, `_filed` and the reconcile wrappers are duplicated across four test files, and `test_vendor_pin_guard_feeds.py` redefines its constants partway down. Move them into `doubles.py`.

### Correctness

**Summary**: There are no logic errors in the core matching: npm ranges, OSV event ordering, four-part Chromium comparison, marker reconciliation, the cap, age arithmetic and gpg field indices all check out. Two minor concerns remain. The Chromium check fails as a whole on an OSV record without `affected`, and the `command_runner` grace period can still return truncated `Ok`.

**Strengths**:
- `npm_ranges` produces correct half-open bounds and refuses unsupported syntax.
- OSV `_range_affects` matches OSV semantics (`last_affected` exclusive above, `introduced`/`fixed` inclusive).
- `comparable_fix` selects the right major, so a browser between two fixed majors is not treated as covered by the older fix.
- Reconciliation is sound: outage coverage by superset, trusted authors only, and a stable digest for the tripped marker.
- gpg colon parsing uses the documented indices and both expiry formats, in UTC.
- `type(value) is dt.date` rejects TOML datetimes.
- The `command_runner` change fixes the real race.

**Comments**:
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/chromium_advisories.py:109` — An OSV record without `affected` fails the whole Chromium check (see Inline Comments).
- 🔵 minor/medium `cli/config-adapters/src/command_runner.rs:352-358` — A grace period that times out with undrained pipes still returns `Ok` with truncated output. Give the drain a minimum window, or report the timeout as an error.

**General Findings**: none.

### Security

**Summary**: The guard is well built from a security standpoint: least-privilege job permissions, SHA-pinned actions, `persist-credentials: false`, dispatch input passed through an env var, regex-validated feed identifiers, and a registry-bound tarball with integrity and size caps. The one real gap is that `code_span` does not neutralise line breaks. A smaller concern is that suppression rests on mutable issue state.

**Strengths**:
- `permissions: {}` at workflow level; `contents: read` and `issues: write` on the single job.
- The dispatch input reaches the task through a quoted env var, and the task rejects values below 1.
- Every `uses:` is SHA-pinned and enforced by a test; actionlint covers every workflow.
- Strict ASCII patterns for GHSA, CVE and vuln/core keys; the OSV id must equal the queried id; `IssueMarker` refuses characters outside `[A-Za-z0-9._-]`.
- Only the last-line marker of a bot-authored issue counts.
- The tarball must be https on registry.npmjs.org, sha512-verified and size-capped (32 MiB tarball, 1 MiB `browsers.json`).
- `gpg --show-keys` runs with a throwaway homedir and argv (no shell).
- `--today` is refused together with `--open-issues`.
- The assembly-time cross-check against `browsers.json` is restored.

**Comments**:
- 🔵 minor/high `tasks/shared/vendor/pin_guard/findings.py:36-39` — `code_span` does not neutralise line breaks (see Inline Comments).
- 🔵 suggestion/medium `tasks/shared/vendor/pin_guard/feeds.py:89-94` — Outage drafts render unvalidated feed text. Put fixed reason codes in the issue body and keep the verbatim values in the job log.
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/issues.py:79-92` — Permanent suppression rests on issue state that any triager can change, and on a bot identity every workflow shares. Consider checking `state_reason`/`closed_by`, and document that no other workflow may hold `issues: write`.

**General Findings**:
- 🔵 suggestion — `GH_TOKEN` is visible to the whole mise task tree. Optionally read it from a differently named variable and pass it only to the `gh` subprocess.

### Safety

**Summary**: The guard fails safe in most places. Aborts happen before any write, dry run is the default, `--today` is refused with `--open-issues`, outages bypass the cap, the issue listing is paginated and fails closed, and a concurrency group serialises runs. The main gap is permanent, version-blind suppression. Smaller gaps: a local-input failure takes all advisory checks offline, and an owner who cannot be assigned or an oversized tripped body blocks all issue filing.

**Strengths**:
- Every write comes at the end of the run; an abort or a failed listing leaves the tracker unchanged.
- Fail-closed defaults: dry run, `--today` refused with `--open-issues`, a cap below 1 rejected, and a scheduled-run fallback of `${MAXIMUM_NEW_ISSUES:-10}`.
- The cap with a guard-tripped issue limits runaway creation, and outages bypass the cap.
- The paginated REST listing with an author filter fails closed.
- `concurrency: runtime-pin-guard` with `cancel-in-progress: false`.
- Requests are bounded (two retries, 20 s per attempt, a 10-minute deadline, a 30-minute job timeout).
- The assembly cross-check runs before any archive is written.
- The test of the transitive closure of aggregate tasks keeps the guard out of `check`/`default`.

**Comments**:
- 🟡 major/high `tasks/shared/vendor/pin_guard/findings.py:154-158` — Advisory suppression ignores the pin version (see Inline Comments).
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/keyring.py:62-69` — A gpg or local-input failure takes every advisory check offline and files no issue. Treat a key-listing failure like a feed failure.
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/github_issues.py:91-106` — An owner who cannot be assigned blocks every issue, outages included. Retry without `--assignee`, or validate the owner first.
- 🔵 minor/low `tasks/shared/vendor/pin_guard/issues.py:150-173` — The guard-tripped body is unbounded, and a changed held-back set opens a new tripped issue each day. Truncate the list, and suppress a new tripped issue while one is open.
- 🔵 suggestion/low `.github/workflows/runtime-pin-guard.yml:8-11` — `maximum-new-issues` has no upper bound. Add a hard ceiling, e.g. 50.

**General Findings**:
- 🔵 minor — The first live run trips the cap, and the planned bulk close of 16 "to assess" CVEs makes each one a permanent suppression. Prefer a KEV `dateAdded` or CVE-year cutoff before merging.

### Documentation

**Summary**: The documentation is strong and accurate. Nearly every claim in RELEASING.md matches the code, and tests pin several of them. The gaps: issue-body guidance contradicts itself for advisories with no fix, Node advisories are identified by a bare vuln/core key, the "keyring" subject is named inconsistently, and a few docstrings restate their code.

**Strengths**:
- The RELEASING.md limits, retry and timeout behaviour, marker rules, outage scoping and throwaway-repo workflow all match the code.
- `test_vendor_pin_guard_releasing.py` ties the docs to the constants.
- An honest "Blind spots" list.
- Stale docstrings are fixed (`chromium.py`, `trust_anchors.py`).
- Absolute RELEASING.md URLs, with a justified comment; the anchors resolve.
- The `tasks/README.md` subsection explains naming, exclusion from aggregates, dry-run behaviour and a throwaway-repo recipe.

**Comments**:
- 🔵 minor/high `tasks/shared/vendor/pin_guard/findings.py:172` — "Bump the pin past the fix" appears even when no fix exists (see Inline Comments).
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/node_advisories.py:106` — Node advisory issues are titled by a bare vuln/core key. Give them a display label or a link.
- 🔵 minor/medium `tasks/shared/vendor/pin_guard/findings.py:93-94` — The keyring-age body names `npm-registry.pem`, while RELEASING.md says "Node keyring".
- 🔵 suggestion/high `RELEASING.md:269` — "more than 10" hard-codes the configurable cap.
- 🔵 suggestion/medium `tasks/README.md:389` — The throwaway-repo recipe still assigns to the RELEASING.md owner.
- 🔵 suggestion/medium `tasks/shared/vendor/pin_guard/feeds.py:149` — Docstrings that restate their implementations (`field`, `today_or_now`, `run_gh`, `IssueMarker.of_body`, `ChromiumAdvisory.of_record`, `checked_keys`).
- 🔵 suggestion/medium `tasks/shared/vendor/pin_guard/findings.py:69-70` — Age and key issues link to the top of the section rather than "Acting on an issue".

**General Findings**:
- 🔵 minor — "Refreshing the anchors" still frames Node as fixed by the Playwright pairing, while this PR bumps Node independently.

---
*Review generated by /accelerator:review-pr*
