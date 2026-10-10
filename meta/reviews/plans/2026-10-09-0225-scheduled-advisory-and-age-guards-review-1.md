---
type: "plan-review"
id: "2026-10-09-0225-scheduled-advisory-and-age-guards-review-1"
title: "Plan Review: Scheduled Advisory and Age Guards for the Vendored Runtime Pins Implementation Plan"
date: "2026-10-10T08:37:41+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-09-0225-scheduled-advisory-and-age-guards"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "security", "safety", "standards", "documentation"]
review_number: 1
review_pass: 4
tags: ["security", "distribution", "runtime", "playwright", "ci", "advisories", "runtime-pin-guard"]
last_updated: "2026-10-10T14:33:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Scheduled Advisory and Age Guards for the Vendored Runtime Pins Implementation Plan

**Verdict:** REVISE

The plan is structurally strong: a pure functional core behind narrow
`IssueTracker`/`FeedClient`/`KeyLister` ports, domain vocabulary mapped
one-to-one onto modules, precise boundary arithmetic, abort-before-write
ordering, a least-privilege workflow, and test-first coverage of nearly every
acceptance criterion. Its weaknesses cluster around ways the guard can fail
open or fail noisily: a documented owner action that permanently silences the
age guard, an error taxonomy that routes domain errors and parse crashes to
the wrong outcome, unvalidated assumptions about the KEV×OSV join, no bounded
behaviour under slow feeds or matcher bugs, and integration tests that can
silently reach real gpg/HTTP once later phases land. No critical findings;
16 major findings drive the REVISE verdict.

### Cross-Cutting Themes

- **Pin-age identity vs the documented re-confirm action** (flagged by: correctness, safety, documentation) — the `pin-age <pin> <version>` marker plus closed-issue suppression means "re-confirm and update the bump date" silences age alerts for that version for ever. Either key the marker on the bump date too (as keyring-age does) or drop the action.
- **Error taxonomy and routing** (flagged by: code-quality, correctness, architecture, test-coverage, safety) — `request` maps every `ValueError` to `unparseable`, which can swallow `UnpinnedRevisionError`'s abort and mislabel `MissingFieldError`; non-`ValueError` parse errors (`TarError`, `KeyError`, `OSError`) escape and kill every check; `gh` failures escape as raw `CalledProcessError` mid-reconcile; `KeyringError`/`PinInconsistencyError` subclass `LocalInputError` only to share the abort path.
- **`GuardPorts` defaults to real collaborators** (flagged by: code-quality, test-coverage) — `key_lister = gpg_key_listing` (and probably `feeds`) lets Phase 2 integration tests silently shell out to gpg or the network from Phase 3 on.
- **Fail-open advisory matching** (flagged by: security, safety, correctness, architecture) — `comparable_fix` misses a pin whose major lies between fixed majors; KEV/`vuln/core`/OSV schema drift yields zero matches with no failure; one unresolvable KEV CVE blinds the whole Chromium check; the KEV×OSV join was verified for only two CVEs.
- **Closing an advisory issue is permanent** (flagged by: documentation, security, safety) — RELEASING.md content for the advisory action is unspecified, yet closing suppresses that advisory on that pin for every later version; mass-closing spurious issues has the same effect.
- **Second readers beside existing owners** (flagged by: architecture, code-quality, standards) — `HttpFeedClient` duplicates `fetch.py` ("the one HTTP client"); `read_local_inputs` parses `pins.toml` beside `pins.py`.
- **`tasks/README.md` placement** (flagged by: documentation, standards) — a row in the `main.yml` check-job table does not fit an issue-writing scheduled job; a subsection beside "The measure namespace" does.
- **Issue-listing trust** (flagged by: security, correctness) — `/issues` returns PRs, bodies can be `null`, and markers are trusted from any labelled body regardless of author.

### Tradeoff Analysis

- **Safety vs simplicity (circuit breaker, dry-run, retries)**: safety wants a per-run issue cap, a dry-run default and bounded retries; each adds surface the work item does not ask for. Recommend the dry-run (cheap, and closes the "spurious issue permanently suppresses" hole) and the time budget; treat the cap and retries as optional.
- **Fail-loud vs availability (gpg missing)**: aborting the whole run on a missing gpg is loud but blinds the advisory and age checks. The work item lists the keyring as a local input, so the abort is defensible; record it as a deliberate choice.
- **Whole-check skip vs coverage (Chromium)**: the work item mandates skipping the Chromium check on any OSV record failure; architecture notes this hides every other KEV match during an OSV lag. Keep the rule, document the breadth of the blind spot.
- **Integrity verification vs scope (tarball)**: security wants `dist.integrity` checked on the `playwright-core` tarball; the plan explicitly scopes it out. Reusing the existing sha512 binding is cheap; recommend it, but it is a judgement call.

### Findings

#### Major

- 🟡 **Correctness + Safety + Documentation**: Pin-age identity omits the bump date, so "re-confirm and update the bump date" permanently silences the age guard for that version
  **Location**: Phase 2 §4 `PinAgeFinding.marker`; Phase 4 §4 RELEASING.md owner actions
  A closed pin-age issue keeps suppressing the same `pin-age <pin> <version>` marker after the bump date is refreshed, so the pin can go unbumped indefinitely.
- 🟡 **Code Quality + Correctness**: `request` maps every `ValueError` to an `unparseable` feed failure; non-`ValueError` parse errors escape
  **Location**: Phase 5 §1 `request`; Phase 7 §1 `pinned_browser_build`
  `UnpinnedRevisionError` inside a `parse` callable becomes a feed failure instead of an abort; `TarError`/`KeyError`/`OSError` escape and crash `evaluate`, losing every check.
- 🟡 **Security**: Fixed-version rule fails open when the pinned major lies between fixed majors
  **Location**: Phase 7 §1 `comparable_fix`
  With fixes at 136.x and 138.x and a 137.x browser, the lowest overall (136.x) is chosen and the pin reads as unaffected. The rule mirrors the work item, so fixing it means amending the work item.
- 🟡 **Correctness + Safety**: No check that every KEV Chromium CVE's OSV record yields a Chrome `fixed` version
  **Location**: Phase 7 §1 fixes from `unresolved_ranges`
  Only two CVEs were verified; one record with no `fixed` event skips the Chromium check every day, and non-Chrome versions in the same record can produce false negatives.
- 🟡 **Safety**: Feed schema drift fails open as "no findings"
  **Location**: Phases 5–7 `is_chromium_component`, `SUPPORTED_ENVIRONMENTS`, `NpmAdvisory.parse`
  A renamed KEV vendor, new `affectedEnvironments` strings or a changed OSV package shape all yield zero matches with every field present.
- 🟡 **Safety**: No per-run cap on opened issues
  **Location**: Phase 2 §5 `reconcile`
  A matcher bug can open dozens to hundreds of issues; mass-closing them permanently suppresses those markers.
- 🟡 **Safety**: No dry-run mode; `--today` can create permanently-suppressing spurious issues
  **Location**: Phase 2 §7 `guard_pins`; Manual Verification
  Any local run writes to whatever repository `gh` resolves; closing a premature issue later suppresses the real one.
- 🟡 **Architecture**: `CheckOutcome` in `guard.py` creates a circular import with every advisory check module
  **Location**: Phase 5 §2; Phases 5–7 check modules
  `osv`, `node_advisories` and `chromium_advisories` return `CheckOutcome` while `guard` imports them.
- 🟡 **Architecture**: Worst-case feed latency exceeds the job timeout, and a timed-out run reports nothing
  **Location**: Phase 5 `HttpFeedClient`; Phase 4 `timeout-minutes: 20`; Performance Considerations
  ~100 sequential 60 s requests can exceed 20 minutes; reconciliation runs last, so no age, expiry or feed-failure issue opens.
- 🟡 **Code Quality + Safety + Correctness**: `gh` failures escape as raw `CalledProcessError`, mid-reconcile
  **Location**: Phase 2 §5 `run_gh`; Phase 5 §2
  stderr is hidden, remaining drafts and feed-failure issues go unopened, and the feed-failure `Exit` is never reached.
- 🟡 **Code Quality + Test Coverage**: `GuardPorts` fields default to real gpg (and possibly HTTP), and earlier guard tests are not updated
  **Location**: Phase 3 §1; Phase 5 §2
  Phase 2's `guard_pins_with` tests would run real gpg from Phase 3 on, making them environment-dependent.
- 🟡 **Test Coverage**: Check-isolation criteria involving the Chromium check are never re-asserted once it exists
  **Location**: Phase 7 §2 `test_pin_guard_feeds.py`
  No test shows an OSV batch or `vuln/core` failure still lets a Chromium advisory open, or a KEV failure leaves the others running.
- 🟡 **Test Coverage**: Phase 1 test edits miss `_realistic_inputs`, couple assembly tests to the real `pins.toml`, and may drop wrong-bytes coverage
  **Location**: Phase 1 §3
  `test_default_spec_builder_maps_the_real_layout` would fail; the `verify_chromium` tests are the only direct wrong-bytes tests.
- 🟡 **Test Coverage**: "Evaluate every check before opening any issue" tests could pass vacuously
  **Location**: Phase 3 §2; Phase 7 §2
  Mid-evaluate abort fixtures need stale bump dates so age findings exist and would otherwise be opened.
- 🟡 **Documentation + Security**: Advisory owner action unspecified, though closing an advisory issue suppresses it permanently
  **Location**: Phase 5 §5 RELEASING.md
  The doc must warn that closing is a per-pin risk-acceptance decision, and list "no auto-close" and unenforced bump dates among blind spots.
- 🟡 **Documentation + Standards**: A row in `tasks/README.md`'s `main.yml` check-job table does not fit; out-of-mirror status undocumented
  **Location**: Phase 4 §4 `tasks/README.md`
  A contributor "reproducing a red job locally" would open real issues.

#### Minor

- 🔵 **Security + Correctness**: Suppression markers trusted from any labelled issue or PR; `body` can be `null`
  **Location**: Phase 2 §5 `GhIssueTracker.issues`
  Filter `pull_request`, coalesce `body`, and consider restricting to the bot author.
- 🔵 **Security**: Third-party feed text goes into issue bodies without validation
  **Location**: Phases 5–7 finding and feed-failure bodies
  A feed value containing `-->` or `@user` could plant a marker or ping users; validate IDs against strict patterns.
- 🔵 **Security**: Unverified tarball from a packument-supplied URL decides the Chromium `browserVersion`
  **Location**: Phase 7 §1; What We're NOT Doing
  Reuse the existing sha512 integrity binding, pin the host and cap the size.
- 🔵 **Correctness**: OSV `affects` wording drops the `introduced` lower bound for `last_affected` and ignores event ordering
  **Location**: Phase 5 §3
- 🔵 **Correctness**: ISO 8601 basic expiry parsed naive would shift by local timezone
  **Location**: Phase 3 §1
- 🔵 **Correctness**: Same-marker findings within one run are not deduplicated
  **Location**: Phase 2 §5 `reconcile`
- 🔵 **Correctness**: `^0.x` is tested but undefined in the range table
  **Location**: Phase 6 §1, §3
- 🔵 **Architecture + Code Quality**: A second HTTP client beside `fetch.py`
  **Location**: Phase 5 §1
- 🔵 **Architecture + Standards**: A second `pins.toml` reader beside `pins.py`
  **Location**: Phase 2 §3
- 🔵 **Architecture + Code Quality**: "Abort" modelled as `LocalInputError`, including non-local failures
  **Location**: Phase 3 §1; Phase 7 §1
- 🔵 **Architecture + Code Quality**: Closed `Finding` union and inconsistent owner placement
  **Location**: Phase 2 §4
- 🔵 **Architecture**: No retry/backoff for transient feed errors
  **Location**: Phase 5 §1
- 🔵 **Architecture**: Release-lane fingerprint test depends on the `pin_guard` subpackage
  **Location**: Phase 3 §2
- 🔵 **Code Quality**: Marker format duplicated across producers and the parser
  **Location**: Phase 2 §5; Phase 5 §2
- 🔵 **Code Quality**: `CheckOutcome.of` hides its drop-findings-on-failure rule
  **Location**: Phase 5 §2
- 🔵 **Code Quality + Correctness**: Positional stdin in `GhRunner`; `_today` unspecified (must default to the UTC date)
  **Location**: Phase 2 §5, §7
- 🔵 **Safety**: One chronically unresolvable KEV CVE blinds the Chromium check behind one stale issue
  **Location**: Phase 7; Phase 5 §2
- 🔵 **Safety**: gpg missing on the runner aborts every check, not just key expiry
  **Location**: Phase 3 §1
- 🔵 **Test Coverage**: `gpg_key_listing` error paths and argv untested; no runner seam
  **Location**: Phase 3 §1
- 🔵 **Test Coverage**: Real-gpg parser compatibility left to a manual check
  **Location**: Phase 3 §2
- 🔵 **Test Coverage**: Feed-failure tests should assert the failure kind; `HttpFeedClient` untested
  **Location**: Phase 5 §1
- 🔵 **Test Coverage**: npm-registry per-failure-kind tests not enumerated
  **Location**: Phase 7 §2
- 🔵 **Test Coverage**: Per-kind issue-body and assignee assertions missing
  **Location**: Phases 2/3/5/7
- 🔵 **Test Coverage**: `next_page_token`, `last_affected` and open-range logic untested
  **Location**: Phase 5 §3
- 🔵 **Documentation**: Refresh step 4 does not say which bump dates to update
  **Location**: Phase 4 §4
- 🔵 **Documentation**: RELEASING.md does not warn that the `Owner:` line is machine-read
  **Location**: Phase 2 §2; Phase 4 §4
- 🔵 **Documentation**: `[playwright-core]` table holds only a date with no pointer to the version
  **Location**: Phase 2 §1
- 🔵 **Documentation**: Adjacent stale comments left in `trust_anchors.py` and `nodejs.py`
  **Location**: Phase 4 §4; Phase 3
- 🔵 **Documentation**: Task docstring and `mise` description promise checks that land in later phases
  **Location**: Phase 2 §7
- 🔵 **Standards**: Guard test files use several unrelated name prefixes
  **Location**: Phases 2–7 tests
- 🔵 **Standards**: Fixture directory `pin_guard/` breaks the kebab-case convention
  **Location**: Phases 1, 3, 5–7 fixtures

#### Suggestions

- 🔵 **Code Quality**: Move npm-registry tarball handling out of `chromium_advisories.py` — Phase 7 §1
- 🔵 **Code Quality**: `reconcile` should take the `GuardReport` — Phase 5 §2
- 🔵 **Architecture**: Split the `gh` adapter out of `issues.py` — Phase 2 §5
- 🔵 **Architecture**: Document that one lagging OSV record suppresses all Chromium advisories — Phase 7
- 🔵 **Security**: Document that closing an advisory issue permanently silences it — Phase 4 §4
- 🔵 **Safety**: Automated liveness heartbeat for the schedule — Phase 4 §4
- 🔵 **Correctness**: Pass `--with-subkey-fingerprint`; record the 2.4 fixture on `ubuntu-latest` — Phase 3 §1
- 🔵 **Test Coverage**: Assert both sides of each derived range bound — Phase 6 §3
- 🔵 **Test Coverage**: Positive assertion of the actionlint command — Phase 4 §3
- 🔵 **Standards**: SHA-pin assertion over every workflow — Phase 4 §2
- 🔵 **Standards**: Record the task → package → workflow/label name mapping — Implementation Approach
- 🔵 **Documentation**: Issue bodies link to the RELEASING.md section — Phase 2 §4
- 🔵 **Documentation**: Drift-test the KEV filter constants in RELEASING.md; record the `gh api --paginate` decision in the work item — Phase 7 §3

### Strengths

- ✅ Functional core / imperative shell: `IssueTracker`, `FeedClient` and `KeyLister` ports gathered in `GuardPorts`, `today` injected everywhere, so every rule is testable against fakes and recorded fixtures.
- ✅ Marker-keyed reconciliation is idempotent; markers end in ` -->` so none matches as a prefix of another.
- ✅ Read → evaluate → reconcile ordering puts every abort before any tracker write; feed failures exit non-zero only after reconciliation.
- ✅ Boundary arithmetic is exact (90/91, 365/366, 60/61, 23:59 UTC on day 61) and `type(v) is dt.date` rejects datetime subclasses.
- ✅ Phase 1 replaces three dead cross-checks with one `BrowsersManifest`, makes the `chromium.py` docstring true, and fails assembly before any archive is written.
- ✅ Feed failures never read as "no findings": `CheckOutcome.of`, `next_page_token` as failure, OSV 404 as failure.
- ✅ Least-privilege workflow: `permissions: {}`, job-scoped `contents: read` + `issues: write`, SHA-pinned actions, `persist-credentials: false`, timeout and concurrency group; actionlint extended to every workflow.
- ✅ `gh` receives bodies on stdin and a strictly validated login as argv; gpg runs in a throwaway homedir.
- ✅ CI-mirror guard with an anti-vacuity test keeps the issue-writing task out of `check` and `default`.
- ✅ Recorded live fixtures (OSV `{}`, 404 body, `unresolved_ranges`, gpg colons) and a range parser scoped to exactly the feed's syntax.
- ✅ Documentation drift fixed and drift-tested: constants asserted by value in RELEASING.md, missing fingerprint test added, stale heading reference corrected, work item updated alongside.

### Recommended Changes

1. **Make pin-age identity include the bump date, or drop the re-confirm action** (addresses: pin-age identity finding)
   Change the marker to `pin-age <pin> <version> <bumped>`, add a lifecycle test (closed issue for V/D1 does not suppress V/D2), and update the work item's Finding-identity table — or remove "re-confirm and update the bump date" from RELEASING.md.
2. **Specify the error taxonomy end to end** (addresses: `request` catch-all, `gh` failures, abort modelling, `CheckOutcome` invariant)
   Introduce a `FeedDocumentError` base for `MissingFieldError`/`RangeSyntaxError`/`BrowsersManifestError`; catch `json.JSONDecodeError`, `httpx.HTTPError` and tarball errors explicitly; convert `UnpinnedRevisionError` outside `request`. Add a `GuardAbort` base in a leaf module. Add `IssueTrackerError` carrying argv and stderr, attempt every draft (feed-failure issues first), and translate at the task boundary. Assert `FeedFailure.reason` in tests.
3. **Remove production defaults from `GuardPorts` and update earlier tests in each phase** (addresses: `GuardPorts` defaults)
   `real()` is the only composition point; add a `fake_ports()` builder in `doubles.py`; Phases 3 and 5 explicitly update `test_vendor_guard_pins.py`.
4. **Move `CheckOutcome` to a leaf module** (addresses: circular import)
   `feeds.py` or a new `checks.py`.
5. **Validate the KEV×OSV join before Phase 7, and fix the fail-open paths** (addresses: KEV×OSV join, `comparable_fix`, schema drift, unresolvable KEV CVE)
   Run a one-off scan of every current KEV Chromium CVE against OSV and record gaps; decide `last_affected`-only policy. Decide with the owner whether to amend the work item's comparable-fix rule to "lowest fix with major ≥ browser major". Add anti-vacuity invariants (KEV filter matches ≥ 1, `vuln/core` yields ≥ 1 supported entry, a returned OSV record keeps ≥ 1 range) reported as feed failures.
6. **Bound the run** (addresses: timeout, retries)
   Per-feed short-circuit after N failures or a deadline, per-request timeouts sized to fit 20 minutes, optional bounded retry for transport/5xx/429 inside `HttpFeedClient`; document the bound.
7. **Add a dry-run default** (addresses: no dry-run; partially the issue cap)
   The task prints drafts unless `--open-issues` is passed; only the workflow passes it. Optionally add a per-run cap that opens one `guard-tripped` issue instead.
8. **Close the test gaps** (addresses: Phase 1 test edits, check isolation, vacuous ordering, npm failure kinds, `gpg_key_listing`, per-kind bodies, OSV edge cases)
   Update `_realistic_inputs` and pass a `tmp_path` pins file to every assembly test; keep direct wrong-bytes tests. Add Phase 7 parametrised isolation tests over each failing feed. Seed stale bump dates in mid-evaluate abort fixtures. Enumerate npm failure kinds. Give `gpg_key_listing` a runner seam and run the real listing through `checked_keys` in CI. Add per-kind body/assignee tests and `next_page_token`/`last_affected`/open-range tests.
9. **Tighten correctness details** (addresses: OSV `affects`, naive ISO, in-run dedup, `^0.x`, PR filtering, `_today`)
   Spell out the sorted-event OSV algorithm; require aware UTC datetimes; dedup drafts by marker; define or reject `^0.x`; filter PRs and coalesce `null` bodies in the jq; `_today` defaults to the UTC date.
10. **Complete the documentation** (addresses: advisory owner action, `tasks/README.md`, refresh step 4, Owner line, `[playwright-core]` pointer, stale comments, task description)
    Specify the advisory action and the permanence of closing; replace the README row with a subsection beside "The measure namespace"; make refresh step 4 conditional per subject; note the `Owner:` line is machine-read; point the `pins.toml` header at `package.json`; remove the stale `PUBLISHER_KEYS` comment and the second-person-review clause; use phase-neutral task wording.
11. **Align with existing owners and conventions** (addresses: second HTTP client, second `pins.toml` reader, naming)
    Extend `fetch.py` and `pins.py` rather than duplicating them; name tests `test_vendor_pin_guard_<module>.py`; rename the fixture directory `pin-guard/`.

## Per-Lens Results

### Architecture

**Summary**: Sound structure overall: a pure functional core under `pin_guard/` with the issue tracker, feeds and gpg behind narrow injected ports, and module names matching the domain vocabulary. Marker-keyed reconciliation makes runs idempotent; Phase 1 merges two uncalled cross-checks into one `BrowsersManifest`. The main structural flaw is a circular import via `CheckOutcome`; the main resilience gap is worst-case request time exceeding the job timeout with reconciliation only at the end. Smaller issues: a second `pins.toml` reader and HTTP client, and `LocalInputError` used to mean "abort".

**Strengths**:
- Functional core / imperative shell with `GuardPorts.real()` as the one composition point.
- Marker-based reconciliation needs no compensation logic.
- Read → evaluate → reconcile guarantees abort before any write.
- `BrowsersManifest` removes the `chromium.py` → `assemble.py` dependency.
- Module table maps concepts one-to-one; `test_mise.py` closure guard.
- actionlint over every workflow fixes a structural blind spot.

**Findings**:
- 🟡 major (high) — **CheckOutcome in guard.py creates a circular import with every advisory check module** — *Phase 5 §2 / Phases 5–7*. `osv`, `node_advisories` and `chromium_advisories` return `CheckOutcome`; `guard.evaluate` imports them. Move `CheckOutcome` to a leaf module (`feeds.py` or `checks.py`).
- 🟡 major (medium) — **Worst-case feed latency exceeds the job timeout, and a timed-out run reports nothing** — *Phase 5 / Phase 7 / Performance / Phase 4*. ~100 sequential OSV GETs at 60 s each can far exceed `timeout-minutes: 20`; reconciliation runs last so nothing opens. Add an overall budget, per-feed short-circuit, and document the bound.
- 🔵 minor (medium) — **No retry/backoff** — *Phase 5 §1*. Transient 5xx/429/resets become feed-failure issues and discard the check's findings. Add bounded retry inside `HttpFeedClient` (never for 404 or parse failures).
- 🔵 minor (high) — **A second HTTP client alongside fetch.py** — *Phase 5 §1*. `fetch.py` documents itself as the one HTTP client. Extend it or state why the guard's is separate and update the docstring.
- 🔵 minor (medium) — **A second reader of pins.toml** — *Phase 2 §3*. Keep `pins.py` as schema owner; add bump-date readers and validation there.
- 🔵 minor (medium) — **"Abort" modelled as LocalInputError** — *Phase 3 / Phase 7*. Introduce `GuardAbort` in a leaf module with named subclasses.
- 🔵 minor (medium) — **Release-lane fingerprint test depends on pin_guard** — *Phase 3 §2*. Move `gpg_key_listing` and `checked_keys` to `gpg.py`.
- 🔵 minor (medium) — **Closed Finding union and inconsistent owner placement** — *Phase 2 §4*. Make `Finding` a Protocol with `draft(owner)`.
- 🔵 suggestion (medium) — **issues.py mixes reconciliation policy with the gh adapter** — *Phase 2 §5*. Split into `github_issues.py` or move `run_gh` to `tasks/shared/`.
- 🔵 suggestion (medium) — **One lagging OSV record suppresses all Chromium advisories** — *Phase 7 / Phase 5*. Keep the mandated rule; document the breadth and list the suppressed check in the feed-failure body.

### Code Quality

**Summary**: Clean domain design: ports and adapters separate pure logic from gh, gpg and HTTP; the module table reads as ubiquitous language; `BrowsersManifest` removes dead duplicated checks. Main risks are in error handling (`ValueError` catch-all, untranslated gh failures) and structures that cost more to change each phase (closed union, duplicated markers, inconsistent owner, `GuardPorts` defaults).

**Strengths**:
- Ports-and-adapters split with `today` passed explicitly.
- One module per domain concept.
- Dead cross-checks replaced by `BrowsersManifest`.
- Frozen slotted dataclasses and StrEnums; errors translated only at the task boundary.
- Range parser scoped to the feed's syntax (YAGNI).
- `checked_keys` is a pure parser separate from running gpg.

**Findings**:
- 🟡 major (high) — **`request` maps every ValueError to "unparseable"** — *Phase 5 §1*. `UnpinnedRevisionError` inside `parse` becomes a feed failure, not the Phase 7 abort. Catch `JSONDecodeError`/`HTTPError` specifically; add a `FeedDocumentError` base; keep `UnpinnedRevisionError` outside it.
- 🟡 major (high) — **gh failures escape as raw CalledProcessError with stderr hidden** — *Phase 2 §5*. Add `IssueTrackerError` with argv and stderr (and timeouts), translate in `guard_pins_with`, test via the recording `GhRunner`.
- 🟡 major (high) — **GuardPorts fields default to real gpg** — *Phase 3 §1 / Phase 5 §2*. No defaults; `real()` is the only wiring point; add a `fake_ports()` builder.
- 🔵 minor (high) — **Closed Finding union with inconsistent owner data** — *findings.py*. `Finding` as a Protocol with `draft(owner)`; drop `owner` from dataclasses.
- 🔵 minor (medium) — **Marker format duplicated across five producers and one parser** — *Phase 2 §5 / Phase 5 §2*. Add an `IssueMarker` value object with `render()`/`parse_all()`.
- 🔵 minor (medium) — **Abort semantics modelled by subclassing LocalInputError** — *Phase 3 / Phase 7*. Introduce `GuardAbort`.
- 🔵 minor (medium) — **`CheckOutcome.of` hides its rule** — *Phase 5 §2*. Enforce in `__post_init__` or use `matched()`/`failed()` constructors; type the parameters.
- 🔵 minor (medium) — **A second HTTP client next to fetch.py** — *Phase 5 §1*. Build on `fetch.py` or move the responsibility and update its docstring.
- 🔵 minor (medium) — **Positional stdin in GhRunner and unspecified `_today`** — *Phase 2 §5, §7*. `GhRunner` as a Protocol with keyword `stdin`; extract a shared `today_or_now`.
- 🔵 suggestion (medium) — **npm tarball handling lives in the Chromium module** — *Phase 7 §1*. Move `pinned_browser_build` to `npm_registry.py`.
- 🔵 suggestion (low) — **`reconcile` gains loose parameters** — *Phase 5 §2*. Take `GuardReport`; state the final signature.

### Test Coverage

**Summary**: Strongly test-first; most acceptance criteria have a named test, including every boundary, lifecycle suppression cases, numeric Chromium comparison and the CI-mirror exclusion. Risks are cross-phase: production defaults in `GuardPorts` without updating earlier tests, isolation criteria never re-asserted once Chromium exists, and several adapters and error paths specified but untested.

**Strengths**:
- Every age/expiry boundary tested on both sides with `today` injected.
- `FakeIssueTracker` seeded with open and closed issues for each dedup rule.
- Fixtures recorded from live responses.
- Anti-vacuity test on the CI-mirror guard.
- Range parser test parses every string in the recorded feed.
- Missing fingerprint-consistency test finally planned.

**Findings**:
- 🟡 major (high) — **New GuardPorts members default to real gpg/HTTP; earlier tests not updated** — *Phase 3 / Phase 5*. Update `guard_pins_with` tests to inject every port; add a `quiet_ports()` builder; make `feeds` required.
- 🟡 major (high) — **Check-isolation criteria involving Chromium never re-asserted** — *Phase 7 §2*. Parametrise over each failing feed and assert every other check's issue still opens.
- 🟡 major (high) — **Phase 1 test edits miss `_realistic_inputs`, couple to real pins.toml, may drop wrong-bytes coverage** — *Phase 1 §3*. Update `_realistic_inputs`; pass `tmp_path` pins to every assembly test; keep matching/mismatched digest tests on `assert_chromium_bytes`.
- 🟡 major (medium) — **"Evaluate every check before opening any issue" could pass vacuously** — *Phase 3 / Phase 7*. Seed stale bump dates in mid-evaluate abort fixtures; assert no tracker calls.
- 🔵 minor (high) — **`gpg_key_listing` untested and no runner seam** — *Phase 3 §1*. Inject runner and `which`; test argv, absent gpg, non-zero exit.
- 🔵 minor (medium) — **Real-gpg parser compatibility left manual** — *Phase 3 §2*. Run the real listing through `checked_keys` in the consistency test.
- 🔵 minor (medium) — **Assert the failure kind; HttpFeedClient untested** — *Phase 5 §1*. Assert exact reasons; add `httpx.MockTransport` tests.
- 🔵 minor (high) — **npm-registry failure kinds not enumerated** — *Phase 7 §2*. Parametrise missing tarball URL, missing member, missing `browserVersion`, unparseable tarball/JSON.
- 🔵 minor (medium) — **Per-kind body and assignee assertions missing** — *Phases 2/3/5/7*. Parametrise one finding of each kind plus a feed failure through `reconcile`.
- 🔵 minor (high) — **`next_page_token`, `last_affected`, open-range untested** — *Phase 5 §3*. Add those four cases.
- 🔵 suggestion (medium) — **Assert both sides of each derived range bound** — *Phase 6 §3*.
- 🔵 suggestion (medium) — **actionlint assertion is negative and weak** — *Phase 4 §3*. Assert the exact command via a recording `Context`.

### Correctness

**Summary**: Boundary arithmetic is sound and worked dates check out; markers end in a terminator; aborts precede any tracker call. Risks: pin-age identity vs the documented re-confirm action; the `ValueError` catch-all and escaping non-`ValueError` errors; the unverified KEV×OSV join; and smaller gaps in OSV range semantics, issue listing and naive ISO timezones.

**Strengths**:
- Precise `> MAX` and `<= 60` boundaries matching every acceptance case.
- `type(v) is dt.date` and `TOMLDecodeError` line reporting.
- Markers end in ` -->`, preventing prefix matches.
- Networked abort path still precedes reconcile.
- `CheckOutcome.of` and per-feed grouping.
- `gh api --paginate` and `next_page_token` as failure.
- npm range table gets `<=N` and caret-zero cases right.

**Findings**:
- 🟡 major (high) — **Pin-age identity omits the bump date** — *Phase 2 §4 / Phase 4*. Add `bumped.isoformat()` to the marker and update the work item, or drop the re-confirm action.
- 🟡 major (medium) — **ValueError catch-all swallows the pin-inconsistency abort; non-ValueError errors escape** — *Phase 5 §1 / Phase 7 §1*. Catch `MissingFieldError` first; convert `UnpinnedRevisionError` outside `request`; map `TarError`/`KeyError`/`OSError` to npm failures.
- 🟡 major (medium) — **KEV Chromium CVEs' OSV `fixed` versions unverified** — *Phase 7 §1*. Scan every current KEV Chromium CVE; decide `last_affected` policy; filter to `google:chrome` ranges if possible.
- 🔵 minor (medium) — **OSV `affects` drops the `introduced` bound for `last_affected`, ignores ordering** — *Phase 5 §3*. Specify the sorted-event algorithm; test below-`introduced`.
- 🔵 minor (medium) — **`/issues` returns PRs; `body` can be null** — *Phase 2 §5*. `select(.pull_request == null)` and `.body // ""`.
- 🔵 minor (medium) — **Naive ISO expiry shifted by local timezone** — *Phase 3 §1*. Require aware UTC datetimes; test under non-UTC `TZ`.
- 🔵 minor (medium) — **Same-marker findings in one run not deduplicated** — *Phase 2 §5*. Dedup drafts by marker.
- 🔵 minor (high) — **`^0.x` tested but undefined** — *Phase 6*. Add a row or require it to raise.
- 🔵 suggestion (low) — **gh failure mid-reconcile skips drafts and feed-failure issues** — *Phase 5 §2*. Specify ordering; translate to `Exit`.
- 🔵 suggestion (low) — **Default `today` should be the UTC date** — *Phase 2 §7*. Follow `tasks/docs.py:48-52`.
- 🔵 suggestion (low) — **Pass `--with-subkey-fingerprint`** — *Phase 3 §1*. Record the 2.4 fixture on `ubuntu-latest`.

### Security

**Summary**: A defensive control with sound CI posture: `permissions: {}`, job-scoped `contents: read` + `issues: write`, SHA-pinned actions, no credential persistence, argv/stdin to `gh`. Main risks are fail-quiet paths: the Chromium fixed-version rule between majors, markers trusted from any labelled body, unvalidated feed text in bodies, and an unverified tarball deciding `browserVersion`.

**Strengths**:
- Least-privilege workflow enforced by a test; actionlint on every workflow.
- Body on stdin, strictly validated login as argv.
- Feed failures, truncation and 404s never read as "no findings".
- Throwaway gpg homedir with strict colon parsing.
- Local-input validation before any write.
- Fingerprint-consistency test enforced on CI.

**Findings**:
- 🟡 major (medium) — **Fixed-version rule fails open between fixed majors** — *Phase 7 `comparable_fix`*. Choose the lowest fix with major ≥ browser major; add 136/138 vs 137 and 139 cases.
- 🔵 minor (medium) — **Suppression markers trusted from any labelled issue or PR** — *Phase 2 §5*. Filter PRs, restrict to the bot author, read the marker from the final line; test non-bot and PR cases.
- 🔵 minor (medium) — **Feed text into issue bodies without validation** — *Phases 5–7*. Validate IDs against strict patterns; render free text in code spans; test `-->` and `@user`.
- 🔵 minor (medium) — **Unverified tarball decides browserVersion** — *Phase 7 / What We're NOT Doing*. Reuse the sha512 binding, require `registry.npmjs.org`, `extractfile` only, size caps.
- 🔵 suggestion (low) — **Document that closing an advisory issue permanently silences it** — *Phase 4 §4*.

### Safety

**Summary**: Safety-conscious overall: local-input aborts before writes, feed failures never read as clean, minimal token scope with timeout and concurrency, assembly fails before writing archives. Risks are fail-open and flood paths: the re-confirm action, feed-schema drift, no per-run cap, no dry-run, and permanent suppression by closed issues making mistakes costly.

**Strengths**:
- Evaluate-then-reconcile with aborts escaping first.
- `CheckOutcome.of` and `next_page_token` as failure.
- Future-dated and datetime bump dates abort.
- Minimal permissions, timeout, non-cancelling concurrency.
- Phase 1 cross-check before any archive or `.sealed` attestation.
- 60 s timeouts on `gh` and HTTP.
- CI-mirror guard keeps local `mise run` from writing issues.

**Findings**:
- 🟡 major (high) — **Re-confirm action permanently silences the pin-age guard** — *Phase 4 / Phase 2*. Include the bump date in the identity or remove the option; add a lifecycle test.
- 🟡 major (medium) — **Feed schema drift fails open** — *Phases 5–7*. Add anti-vacuity invariants reported as feed failures; test each against a drifted fixture.
- 🟡 major (medium) — **No per-run cap on opened issues** — *Phase 2 §5*. Circuit breaker: above ~10 drafts, open one `guard-tripped` issue and exit non-zero.
- 🟡 major (medium) — **No dry-run mode** — *Phase 2 §7*. Print drafts unless `--open-issues`; or refuse a non-real `--today` when writing.
- 🔵 minor (medium) — **One unresolvable KEV CVE blinds the Chromium check** — *Phase 7 / Phase 5*. Run the live join once before Phase 7; comment on the open feed-failure issue when the failed set changes.
- 🔵 minor (medium) — **A gh failure aborts the rest of reconciliation** — *Phase 2 §5*. Open drafts independently; feed-failure issues first.
- 🔵 minor (low) — **gpg missing aborts every check** — *Phase 3 §1*. Confirm intent, or skip only key expiry and fail the run.
- 🔵 suggestion (low) — **Guard liveness depends on a manual check** — *Phase 4 §4*. A "last run" issue or a calendar reminder.

### Standards

**Summary**: Mostly follows repo conventions: three-place task registration, the existing transitive-closure guard pattern, SHA pins matching `main.yml`, doubles in `doubles.py`, no explanatory comments. Gaps are naming and placement: test prefixes, a second `pins.toml` reader, fixture directory casing, and the `tasks/README.md` table.

**Strengths**:
- Task registration matches existing `vendor:*` tasks.
- CI-mirror exclusion reuses the established pattern with anti-vacuity.
- Workflow pins match `main.yml`, permissions tightened.
- Errors follow the `docs.py`/`npm_audit.py` template.
- No explanatory comments; stale prose fixed.
- Path constants centralised in `tasks/shared/paths.py`.

**Findings**:
- 🔵 minor (high) — **Guard test files use several unrelated prefixes** — *Phases 2–7*. Use `test_vendor_pin_guard_<module>.py`.
- 🔵 minor (medium) — **pins.toml read outside its owning module** — *Phase 2 §3*. Add readers to `pins.py`, or state why not.
- 🔵 minor (medium) — **Fixture directory breaks kebab-case** — *Fixtures*. Rename to `pin-guard/`.
- 🔵 minor (high) — **README row in a table scoped to main.yml check jobs** — *Phase 4 §4*. Widen the header or add a subsection.
- 🔵 suggestion (medium) — **SHA-pin assertion covers only the new workflow** — *Phase 4 §2*. Loop over every workflow.
- 🔵 suggestion (low) — **Guard named three ways** — *Implementation Approach*. Record the mapping.

### Documentation

**Summary**: Treats documentation as a deliverable, drift-tests the constants, fixes stale claims and phases RELEASING.md content with shipped behaviour. The main accuracy risk is the "re-confirm and update the bump date" action; smaller gaps are the unspecified advisory action and blind spots, the `tasks/README.md` placement, and adjacent stale comments.

**Strengths**:
- Constants asserted by value in RELEASING.md; `Owner:` line parsed in a test.
- Phase 1 makes existing documentation true.
- Missing fingerprint test added, so the `nodejs.py` comment becomes accurate.
- Placeholder claim and non-existent heading fixed and tested.
- RELEASING.md content never describes unshipped behaviour.
- Work item updated alongside decisions.

**Findings**:
- 🟡 major (high) — **Pin-age action contradicts the bump-date definition and disables future alerts** — *Phase 4 §4*. Drop it, or record deferral with its own marker semantics.
- 🟡 major (medium) — **Advisory owner action unspecified; closing suppresses permanently** — *Phase 5 §5*. Specify the action, warn on closing, list "no auto-close" and unenforced bump dates among blind spots.
- 🟡 major (medium) — **tasks/README.md row does not fit; out-of-mirror status undocumented** — *Phase 4 §4*. Add a subsection beside "The measure namespace".
- 🔵 minor (high) — **Refresh step 4 does not say which dates to update** — *Phase 4 §4*. Make it conditional per subject.
- 🔵 minor (medium) — **Owner line not flagged as machine-read** — *Phase 2 §2 / Phase 4*.
- 🔵 minor (medium) — **`[playwright-core]` table has no pointer to the version** — *Phase 2 §1*.
- 🔵 minor (high) — **Adjacent stale comments** — *Phase 4 / Phase 3*. Remove the `PUBLISHER_KEYS` comment and the second-person-review clause.
- 🔵 minor (medium) — **Task docstring and mise description over-promise in Phase 2** — *Phase 2 §7*. Phase-neutral wording.
- 🔵 suggestion (medium) — **Issue bodies could link to RELEASING.md** — *Phase 2 §4*.
- 🔵 suggestion (low) — **KEV filter not drift-tested; `gh api --paginate` decision unrecorded in the work item** — *Phase 7 §3*.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-10T12:33:58+00:00

**Verdict:** REVISE

### Previously Identified Issues

All 16 prior major findings are resolved, apart from two partials noted below.

- 🟡 **Correctness + Safety + Documentation**: Pin-age identity / re-confirm action — Resolved (but see the new Chromium marker collision)
- 🟡 **Code Quality + Correctness**: `request` maps every `ValueError` to unparseable — Resolved
- 🟡 **Security**: Fixed-version rule fails open between fixed majors — Resolved
- 🟡 **Correctness + Safety**: KEV×OSV join unverified — Resolved (scan step added)
- 🟡 **Safety**: Feed schema drift fails open — Resolved
- 🟡 **Safety**: No per-run cap on opened issues — Resolved (the cap introduces new issues below)
- 🟡 **Safety**: No dry-run mode — Resolved
- 🟡 **Architecture**: `CheckOutcome` circular import — Partially resolved (now a `checks.py` ↔ `feeds.py` cycle)
- 🟡 **Architecture**: Worst-case latency exceeds the job timeout — Partially resolved (bound ignores setup, `Retry-After`, per-phase httpx timeouts and reconciliation)
- 🟡 **Code Quality + Safety + Correctness**: Raw `CalledProcessError` mid-reconcile — Resolved
- 🟡 **Code Quality + Test Coverage**: `GuardPorts` defaults to real collaborators — Resolved
- 🟡 **Test Coverage**: Chromium check-isolation never re-asserted — Resolved
- 🟡 **Test Coverage**: Phase 1 test edits — Resolved
- 🟡 **Test Coverage**: Vacuous abort-ordering tests — Resolved
- 🟡 **Documentation + Security**: Advisory owner action and permanence — Resolved
- 🟡 **Documentation + Standards**: `tasks/README.md` placement — Resolved
- 🔵 **Security**: Feed text into issue bodies — Partially resolved (`code_span` does not stop `parse_all`)
- 🔵 **Test Coverage**: Failure-kind assertions / `HttpFeedClient` tests — Partially resolved (no transport seam)
- 🔵 **Documentation**: Issue bodies link to RELEASING.md — Partially resolved (relative link 404s on GitHub)
- 🔵 **Safety**: One unresolvable KEV CVE blinds Chromium — Partially resolved (a standing feed-failure issue absorbs later failures)
- 🔵 **Safety**: gpg missing aborts every check; schedule heartbeat — Accepted
- 🔵 All other prior minor findings and suggestions — Resolved

### New Issues Introduced

#### Major

- 🟡 **Security + Correctness + Safety**: The guard-tripped issue listing would-be markers can permanently suppress them. `parse_all` reads every marker in a trusted body of any state, so once the tripped issue is closed, every finding it listed stays silenced. The tripped marker's own parts are also unspecified.
- 🟡 **Correctness**: The Chromium pin-age marker collides when Playwright moves but the revision does not. Refresh step 4 updates `chromium.bumped` on every Playwright bump, and the identical `pin-age chromium 1193` marker in the closed issue then suppresses the next breach.
- 🟡 **Safety + Documentation**: The issue cap has no release path when the volume is genuine. Closing the tripped issue re-trips it, and a standing tripped issue hides a changed would-be set.
- 🟡 **Test Coverage**: `fake_ports()` defaults are not "all clear": the recorded key listing has expired keys, and empty feed responses now trip the anti-vacuity rules. Phases 3, 6 and 7 would break earlier tests.
- 🟡 **Test Coverage**: The dry-run default wiring (`GuardPorts.real(open_issues=…)`, `--issue-author` → `trusted_author`) has no automated test.

#### Minor

- 🔵 **Code Quality + Architecture**: `checks.py` and `feeds.py` import each other.
- 🔵 **Code Quality + Architecture**: `IssueTrackerError` lives in the gh adapter, not beside the `IssueTracker` port.
- 🔵 **Code Quality**: No signature carries feed failures from `run_guard` to `guard_pins_with`; how multiple failure reasons combine is unspecified.
- 🔵 **Architecture**: The `FeedClient` port reports failures as `httpx` exceptions.
- 🔵 **Architecture + Correctness + Safety**: The time-budget arithmetic is wrong (60 s of `Retry-After`, per-phase timeouts, slow-drip bodies, unbudgeted reconciliation and setup).
- 🔵 **Safety + Correctness**: The tripped path does not ensure the label, open feed-failure issues or collect its own failure.
- 🔵 **Safety**: A standing feed-failure issue absorbs later, unrelated failures on the same feed.
- 🔵 **Security + Correctness**: `code_span` does not neutralise a marker for `parse_all`.
- 🔵 **Correctness**: The `browserVersion` check sits inside `parse`, but the pinned build is chosen outside the session.
- 🔵 **Correctness**: The lowest same-major fix can pick a lower platform-split patch (`.92` vs `.96`).
- 🔵 **Test Coverage**: No `httpx` transport seam; retry cases are incomplete.
- 🔵 **Test Coverage**: Feed-budget tests let a cumulative or cross-feed counter pass.
- 🔵 **Test Coverage**: Issue-cap tests miss open-only suppression and the feed-failure exemption.
- 🔵 **Standards**: Duplicate `FakeClock`; feed timing bypasses `tasks/shared/clock.Clock`.
- 🔵 **Documentation**: The issue cap and dry-run default are not recorded in the work item.
- 🔵 **Documentation**: The trusted-author rule is undocumented; the safe-trial instructions omit `--issue-author`.
- 🔵 **Documentation**: The work item's Technical Notes keep the superseded `gh issue list` text.
- 🔵 **Documentation**: The pin-age action wording implies auto-close.

#### Suggestions

- 🔵 Composition root in `guard.py`; `trusted_author` inside `GuardPorts`; bare-tuple `failed`; inconsistent parser-to-feed-error rule; trust boundary of `github-actions[bot]`; `today_or_now` belongs in `clock.py`; `tasks/README.md` subsection could land in Phase 2; small untested invariants (`CheckOutcome` rejection, manifest-size cap, npm short-circuit, issue-number parsing, `today_or_now`).

### Assessment

The revision fixed the first review's structural problems: error routing, ports, fail-open matching, the dry-run default and test isolation. The five new majors are all narrow and come from the edits themselves. Two concern the marker and cap design: suppression via the tripped body, and the cap's missing release path. One is a Chromium bump-date rule, and two concern test defaults and wiring. All are fixable with targeted edits and no restructuring. A third pass should mostly confirm those fixes.

## Re-Review (Pass 3) — 2026-10-10T13:51:05+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Security + Correctness + Safety**: Guard-tripped issue suppresses its listed findings — Resolved
- 🟡 **Correctness**: Chromium pin-age marker collision — Resolved
- 🟡 **Safety + Documentation**: Issue cap has no release path — Resolved
- 🟡 **Test Coverage**: `fake_ports()` defaults not all clear — Resolved (residual coupling to fixture pin values, below)
- 🟡 **Test Coverage**: Dry-run wiring untested — Resolved
- 🔵 **Code Quality + Architecture**: `checks.py` ↔ `feeds.py` cycle; `IssueTrackerError` placement; feed failures to the task boundary — Resolved
- 🔵 **Architecture**: `FeedClient` reports failures as httpx exceptions — Partially resolved (`HttpFeedClient` still sits in `feeds.py`; `JSONDecodeError` still crosses the port)
- 🔵 **Architecture + Correctness + Safety**: Time-budget arithmetic — Partially resolved (blocked reads outlast the wall-clock check; raised caps unbounded)
- 🔵 **Safety + Correctness**: Tripped path label/feed-failure/failure handling — Resolved
- 🔵 **Safety**: Standing feed-failure issue absorbs later failures — Resolved (check-set keying adds an inconsistency, below)
- 🔵 **Security + Correctness**: `code_span` vs marker parsing — Resolved
- 🔵 **Correctness**: `browserVersion` check placement; platform-split fix — Resolved
- 🔵 **Test Coverage**: Transport seam and retry cases — Partially resolved; feed-budget counters and cap tests — Resolved
- 🔵 **Standards**: Duplicate `FakeClock`; `Clock` seam; `today_or_now` placement — Resolved
- 🔵 **Documentation**: Cap and dry-run in the work item; trusted author; safe trial; auto-close wording; absolute links; README in Phase 2 — Resolved

### New Issues Introduced

#### Major

- 🟡 **Standards + Safety**: `install_args: python uv` omits the mise-pinned `gh` the guard shells out to — either the runner's unpinned `gh` is used or `mise run` auto-installs everything, invalidating the setup budget.
- 🟡 **Test Coverage**: No test proves `HttpFeedClient` turns every adapter failure (retries spent on transport errors, the wall-clock timeout, `get_bytes` over `max_bytes`) into the domain errors `FakeFeeds` simulates; an escaping httpx error would crash `evaluate`.

#### Minor

- 🔵 **Architecture**: `ChromiumVersion` in `chromium_advisories.py` creates an `npm_registry` ↔ `chromium_advisories` cycle against the declared import order.
- 🔵 **Architecture + Code Quality**: `HttpFeedClient` belongs in its own `http_feeds.py`; `JSONDecodeError` should become a domain error at the adapter.
- 🔵 **Correctness**: Check-set feed-failure identity allows several open issues per feed, contradicting the Desired End State.
- 🔵 **Correctness**: Phase 6 never wires the Node check into `evaluate`.
- 🔵 **Correctness**: Blocked reads can outlast the 20 s per-attempt bound; packument plus tarball may be two client calls in one request.
- 🔵 **Correctness**: Editing a guard issue body breaks the last-line marker rule; CRLF handling unpinned.
- 🔵 **Correctness**: Missing `pins.toml` keys escape as `KeyError`, not `LocalInputError`.
- 🔵 **Correctness**: Clear-world feed defaults are coupled to the fixture repository's pin values.
- 🔵 **Security**: `github-actions[bot]` is shared by every workflow token; `main.yml` has no workflow-level `permissions`.
- 🔵 **Safety**: Cap override is a bare number, not bound to the reviewed set's digest.
- 🔵 **Safety**: A permanently unresolvable OSV record leaves Chromium dark with no clearing action.
- 🔵 **Code Quality**: `AdvisoryFinding` covers three shapes through optional fields.
- 🔵 **Standards**: `wiring.py` departs from the `tasks/dev.py::_dev_deps` composition-root precedent; partial-toolset job shares the default cache key; test file names no longer map one-to-one to modules.
- 🔵 **Documentation**: `pins.toml` comment "a Playwright pin bump refreshes all of them" contradicts the `chromium.bumped` rule; bump-date rule lands in Phase 4 though dates ship in Phase 2; work item Technical Notes keep old marker forms; issue-body anchors not drift-tested.
- 🔵 **Test Coverage + Documentation + Standards**: Testing Strategy still says `FakeFeeds` raises an httpx error.
- 🔵 **Test Coverage**: Wall-clock fetch test needs a clock seam; `issues()`/`ensure_label()` failure paths untested; `of_body` whitespace/CRLF cases; 429 and sub-cap `Retry-After`; multi-feed `failure_reasons`; narrowing feed-failure identity.

#### Suggestions

- 🔵 `code_span` should collapse newlines; `reconcile` could split into a pure plan and an apply step; `FeedOutage`/`TrippedGuard` as draftable values; marker named constructors and drop the repeated `guard-tripped` token; `IssuePolicy` validates itself; dry-run `open_issue` return value; typed `bump_date` subject; `KeyLister` failure as `KeyringError`; cap default asserted against the constant; `commands.py` docstring; `|| 10` turns a dispatched 0 into 10; list superseded tripped/feed-failure issues in new bodies; gpg listing timeout in the budget; tripped digest test seeded from a first run.

### Assessment

The plan is now acceptable for implementation. Every critical-path design problem from passes 1 and 2 is resolved. The two remaining majors are each a one-line fix: add `gh` to `install_args` (or drop the restriction), and add three `HttpFeedClient` adapter-mapping tests. The minors are worth folding in before implementation starts, especially the Phase 6 `evaluate` wiring, the `ChromiumVersion` cycle, the contradictory `pins.toml` comment and the stale Testing Strategy line, but none of them blocks.

## Re-Review (Pass 4) — 2026-10-10T14:18:07+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Standards + Safety**: `install_args` omits the mise-pinned `gh` — Resolved
- 🟡 **Test Coverage**: `HttpFeedClient` adapter-failure mapping untested — Resolved
- 🔵 **Architecture**: `ChromiumVersion` import cycle — Resolved
- 🔵 **Architecture + Code Quality**: `HttpFeedClient` in its own module; JSON errors at the adapter — Resolved (leftover contract wording, below)
- 🔵 **Correctness**: Phase 6 Node wiring — Resolved
- 🔵 **Correctness**: Missing `pins.toml` keys — Resolved
- 🔵 **Correctness**: Clear-world defaults coupled to fixture pins — Resolved
- 🔵 **Correctness**: Feed-failure identity vs Desired End State — Resolved
- 🔵 **Correctness**: Edited bodies / CRLF — Resolved
- 🔵 **Correctness + Architecture**: Blocked reads and two client calls per npm request — Partially resolved
- 🔵 **Documentation**: Contradictory `pins.toml` comment; bump-date rule phasing; Testing Strategy line — Resolved
- 🔵 **Documentation**: Work item marker forms — Partially resolved (guard-tripped form lands in Phase 5, not Phase 2)
- 🔵 **Test Coverage**: Clock seam for the fetch timeout; `of_body` edge cases; 429/`Retry-After`; narrowing feed-failure identity — Resolved
- 🔵 **Test Coverage**: `issues()`/`ensure_label()` failure paths; multi-feed `failure_reasons` — Still present / Partially resolved
- 🔵 Pass-3 suggestions not folded in by choice — not re-raised

### New Issues Introduced

No critical or major findings.

#### Minor

- 🔵 **Architecture + Code Quality + Standards + Correctness**: `FeedSession.request` still says "exactly three error families" but lists two, and its propagation test still names JSON — an implementer could reintroduce a `JSONDecodeError` catch.
- 🔵 **Code Quality + Correctness**: Phase 7's npm `parse` translation list omits `json.JSONDecodeError` and `UnicodeDecodeError` from decoding `browsers.json`, so a corrupt manifest would crash the run.
- 🔵 **Architecture + Correctness**: Whether packument and tarball are one `session.request` or two is still unstated; one request would double the per-request bound and blur labels and skip counting.
- 🔵 **Correctness**: The per-read timeout derived from the remaining budget cannot be set per chunk with httpx streaming; the achievable bound is about twice the total per attempt.
- 🔵 **Correctness**: `of_body` is not specified to return `None` for a malformed last-line marker (unknown kind, empty or invalid part); a naive implementation would crash `reconcile`.
- 🔵 **Correctness**: Superset suppression requires a single covering issue; jointly covering open issues still open a redundant one.
- 🔵 **Code Quality + Standards**: The fetch `now` seam appears in prose but not in the signatures.
- 🔵 **Standards**: `GuardAbort` breaks ruff N818 (`ALL` selected); rename to `GuardAbortError` following `_UpAbortError`.
- 🔵 **Standards**: The naming map calls the job `runtime-pin-guard`, but its id is `guard-pins`.
- 🔵 **Documentation**: The guard-tripped marker form reaches the work item only in Phase 5; the no-edit rule says the finding "reopens" (the guard opens a duplicate); the `[playwright-core]` comment names the wrong dependency (`package.json` declares `playwright`), and `[keyring]` has no comment.
- 🔵 **Security**: Edits to trusted guard-issue bodies are not detected, so a closed issue's marker could be rewritten to pre-suppress a different finding; either treat `lastEditedAt` as untrusted or record the residual under What We're NOT Doing.
- 🔵 **Safety**: A new cause of feed failure can hide behind a stale covering issue; `failure_reasons` should name failed requests. The documented schedule health check passes when every run fails during setup.
- 🔵 **Test Coverage**: No tests for `issues()`/`ensure_label()` failures, two distinct feeds failing in one run, a deadline anchored to a non-zero clock origin, the rewritten `get_json` and per-read timeout, `ChromiumVersion` parsing and an unparseable OSV `fixed`, two failures within one check, or exact npm-registry failure reasons.

#### Suggestions

- 🔵 `CVE_ID` defined twice; `KeyLister` port speaks gpg's error and key types; raised-cap ceiling against the job timeout and a gpg listing timeout; pin mise's task auto-install off rather than checking it once; wall-clock overrun retry case should pin attempts and sleeps; Migration Notes' "no longer writes issues" for a new task; bump-date blind spot filed under OSV; `code_span` newline collapse and `|| 10` (previously declined).

### Assessment

Pass 4 found no critical or major issues, so the plan is ready for implementation. What remains is wording and specification precision that an implementer would otherwise trip over. The top items are the contradictory error-family sentence, the npm JSON-decode gap, the one-or-two npm request question, `of_body` on malformed markers, the `GuardAbort` lint failure, and the job-name mapping. Each is a one- or two-line plan edit and needs no further design work. The test-coverage gaps are additive and can be closed during the red-green-refactor loop.

## Approval — 2026-10-10T14:33:53+00:00

**Verdict:** APPROVE

Approved by Toby Clemson after pass 4. The pass-4 minors stand as implementation-time notes and do not block.
