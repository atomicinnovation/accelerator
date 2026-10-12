---
type: "plan-validation"
id: "2026-10-09-0225-scheduled-advisory-and-age-guards-validation"
title: "Validation Report: Scheduled Advisory and Age Guards for the Vendored Runtime Pins Implementation Plan"
date: "2026-10-12T10:14:12+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-09-0225-scheduled-advisory-and-age-guards"
tags: ["security", "distribution", "runtime", "playwright", "ci", "advisories", "runtime-pin-guard"]
last_updated: "2026-10-12T10:14:12+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Scheduled Advisory and Age Guards for the Vendored Runtime Pins

### Implementation Status

✓ Phase 1: One Browsers Manifest, Checked at Assembly - Fully implemented
✓ Phase 2: Local Inputs, Age Checks, Issue Reconciliation and the Task - Fully implemented
✓ Phase 3: Key Expiry - Fully implemented
✓ Phase 4: Workflow and Non-Advisory Documentation - Fully implemented
✓ Phase 5: Feed Failures and `playwright-core` Advisories - Fully implemented
✓ Phase 6: Node Advisories - Fully implemented
✓ Phase 7: Chromium Advisories - Fully implemented

Every "Changes Required" item is present. Every behavioural departure is
recorded in the plan's Implementation Notes or the work item; the unrecorded
ones are trivial (listed under Deviations).

### Automated Verification Results

✓ Full local CI mirror: `mise run` exited 0 (1066 s); the working copy was
  unchanged afterwards, so the formatters found nothing to rewrite
✓ Targeted suite: `uv run pytest tests/unit/tasks -k "pin_guard or browsers
  or npm or gpg or nodejs or fetch or clock or workflows or trust_anchors or
  pins or assemble or chromium or mise or docs_audit"` — 644 passed
✓ No references remain: `rg 'verify_chromium|assert_version_pairing|browser_revision' tasks tests`
  returns nothing
✓ Dry run: `mise run vendor:guard-pins` on 2026-10-12 reached every feed,
  drafted 26 findings (2 pin age, 3 key expiry, 21 Chromium), tripped the
  cap of 10, printed one `guard-tripped` draft and exited 1 without writing —
  the behaviour the Migration Notes predict

### Code Review Findings

#### Matches Plan:

- `BrowsersManifest` replaces the two uncalled cross-checks, and
  `assemble_tree_artifacts` refuses a revision mismatch before writing
  anything
- `run_guard` reads local inputs and evaluates every check before touching
  the tracker, so every abort precedes any write
- Reconciliation: trusted closed issues suppress findings, untrusted authors
  are ignored, pull requests and `null` bodies are handled, the cap opens 10
  and trips at 11, and outage drafts sit outside the cap
- Feed-failure identity is the feed plus its sorted skipped checks; an open
  trusted issue covers a failure only when its checks are a superset
- `FeedBudget` starts nothing at or after 600 s and skips a feed after three
  consecutive unreachable failures; a document error resets the streak
- Key expiry uses the UTC date, opens at ≤ 60 days, and selects primaries
  plus signing-capable subkeys; the 60/61 boundary is tested
- `npm_ranges` handles every form found in `vuln/core` and refuses the rest
- The Chromium check binds the tarball to the registry's sha512 integrity,
  requires an `https` tarball on `registry.npmjs.org`, caps the download at
  32 MiB and the manifest member at 1 MiB, and reads it in memory
- The workflow has `permissions: {}` at the top level, `contents: read` plus
  `issues: write` on the job, `persist-credentials: false`, actions pinned to
  the same SHAs as `main.yml`, a non-cancelling `concurrency` group,
  `timeout-minutes: 30`, and the dispatch input routed through an env var (no
  `${{` in `run`, asserted by a test)
- `actionlint` and the SHA-pinning test now cover every workflow file

#### Deviations from Plan:

All recorded in the Implementation Notes except the last three:

- `GuardAbort` is `GuardAbortError` (ruff N818)
- The workflow installs the whole `mise` toolset rather than
  `install_args: python uv gh`
- A 404 raises `FeedRecordNotFoundError` and does not count towards the
  unreachable streak; a CVE with no usable fix opens an
  `UnassessableAdvisoryFinding` instead of failing the feed
- The guard reads the npm version document instead of the 17.7 MB packument
- Non-four-part fixed versions are ignored as another product's, not refused
- Node bumped to 22.23.2 to keep the first dispatched run under the cap
- The step 1 KEV×OSV scan was recorded in the work item after the Chromium
  tests were written (settled with the owner beforehand)
- Unrecorded: `local_inputs._read` also maps `OSError` and `TypeError` to
  `LocalInputError` (`tasks/shared/vendor/pin_guard/local_inputs.py:161-166`)
- Unrecorded: `get_json`'s new wall-clock total also applies to
  `upstream.py`'s callers, not only the guard
- Unrecorded: the Phase 5 feed-failure body assertion lives in the feeds test
  rather than the per-kind body test in `test_vendor_pin_guard_issues.py`

#### Potential Issues:

Confirmed by reproduction or reading:

- `test_the_repository_own_inputs_parse`
  (`tests/unit/tasks/test_vendor_pin_guard_local_inputs.py:36-39`) reads the
  real `pins.toml` against a fixed `TODAY = 2026-10-10`. `node.bumped` already
  equals that date, so the next bump dated later fails the test with a
  future-date error, and the `chromium.version == "1193"` assertion fails on
  any Chromium bump. This is the coupling Phase 1 removed from the assembly
  tests.
- `_PinsFile._read` (`local_inputs.py:156`) indexes
  `error.doc.splitlines()[error.lineno - 1]`. For an unterminated string or
  array at end of file, tomllib reports `lineno` one past the last line
  (reproduced: `'a = """x\n'` gives `lineno` 2 over 1 line), so an
  `IndexError` traceback escapes instead of a `LocalInputError`.
- `gpg.list_keys` maps `OSError` and `subprocess.TimeoutExpired` to
  `KeyListingError` (`tasks/shared/vendor/gpg.py:262-267`), but no test
  drives either branch. This contradicts the repository's test-first rule.
- `_browsers_member` (`tasks/shared/vendor/pin_guard/npm_registry.py:131-146`)
  checks `member.size` but not `member.isfile()`. A link member reports size
  0, `extractfile` follows it, and `handle.read()` is unbounded. Integrity
  binding to the registry makes this low risk; `handle.read(MAXIMUM_MANIFEST_BYTES + 1)`
  would close it.
- `CVE_ID` (`chromium_advisories.py:50`) and `GHSA_ID` (`osv.py:28`) use
  `.match` with `$` and no `re.ASCII`, so a trailing newline or non-ASCII
  digits pass; the ID is interpolated into a feed URL and issue title.
  `chromium_versions.py` already uses `fullmatch` with `re.ASCII`.
- The per-attempt bound is looser than documented. `fetch._body` passes a
  scalar `timeout`, giving connect, write, header read and pool 20 s each,
  and the wall-clock total is checked only after a chunk arrives. One attempt
  can take about 80 s rather than 40 s, so the "evaluation ends within 13
  minutes" estimate and `RELEASING.md`'s "Each attempt is limited to 20 s"
  overstate the guarantee. The 30-minute job timeout remains the hard stop.
- `test_vendor_pin_guard_feeds.py:352` still models a 404 as
  `FeedUnreachableError("HTTP 404")`; production now raises
  `FeedRecordNotFoundError`, so the test passes while modelling the wrong
  case.
- `RELEASING.md:364` is 129 columns, an unreflowed edit, against the 80-column
  rule.
- `RELEASING.md`'s guard-tripped advice says to "run the task locally to print
  their drafts", but a local dry run hits the same cap and prints only the
  `guard-tripped` draft; it needs `--maximum-new-issues` raised.

Lower-impact observations:

- `MAXIMUM_NEW_ISSUES: ${{ inputs.maximum-new-issues || 10 }}` turns a
  dispatched `0` into 10 silently, bypassing the CLI's below-1 rejection.
- The workflow now installs Rust but, unlike `main.yml`'s full-toolset jobs,
  sets no `RUSTUP_HOME` inside the cached directory and no `cache_key_prefix`,
  so Rust is reinstalled every run and the cache name is shared.
- `evaluate` lists the keyring after the three advisory checks, so a missing
  gpg aborts only after up to the full feed budget. It still aborts before
  any write.
- The expired `esa` subkey `A6023530…DE079578` is a standing finding whose
  documented remedy ("fetch the extended key") cannot clear it; the owner can
  only close it. `RELEASING.md` gives no action for this case.
- A dry run needs `gh` authentication, because `DryRunIssueTracker.issues()`
  delegates to `gh api`; `tasks/README.md` does not say so.
- Dedup depends on the `runtime-pin-guard` label; stripping it from a guard
  issue re-opens its finding.
- `ChromiumAdvisory.of_record` does not check the record's `id` against the
  requested CVE, unlike the OSV `unexpected advisory id` check.
- Restating docstrings remain against the comment policy, among them
  `clock.today_or_now`, `run_gh`, `issue_body`, `Owner.from_releasing`,
  `IssueMarker.of_body`, `KeyListingError`, `list_keys`, `checked_keys`,
  `get_json`, `get_bytes`, `FeedSession.request`, `advisory_ids`,
  `node_advisories`, `pinned_browser_build`'s first line, and
  `assert_integrity_binds_bytes`. `gpg.py`'s module docstring still describes
  only signature verification.

### Manual Testing Required:

1. Pull request:
  - [ ] The `assemble-runtime` job assembles the real `playwright-core`
        1.55.1 tarball (revision 1193 matches)
  - [ ] CI's `test_vendor_nodejs.py` fingerprint-consistency test ran rather
        than skipped
  - [ ] Optionally, `GH_REPO=<throwaway> mise run vendor:guard-pins --
        --open-issues --issue-author <login> --maximum-new-issues 30` opens
        labelled, assigned issues, and a second run opens nothing

2. After merge, on `main`:
  - [ ] The first dispatched run trips the cap (26 findings), opens one
        `guard-tripped` issue, and fails, as the Migration Notes predict
  - [ ] A dispatch with `maximum-new-issues` at 30 opens the 26 issues,
        labelled `runtime-pin-guard` and assigned to `@tobyclemson`, with no
        feed-failure issue; then close the `guard-tripped` issue
  - [ ] The Chromium advisory issues for `140.0.7339.186` agree with a hand
        reading of their KEV entries and OSV records; the 16 unassessable
        ones are assessed by hand
  - [ ] Any Node advisory issue matches a manual reading of `vuln/core` for
        22.23.2
  - [ ] A further dispatched run opens nothing new
  - [ ] A `schedule`-triggered run appears within 48 hours and meets the same
        conditions

### Recommendations:

- Decouple `test_the_repository_own_inputs_parse` from the run date and the
  pinned revision before the next pin bump; derive `today` from the
  repository's latest bump date or assert only that parsing succeeds
- Clamp the TOML error line lookup in `local_inputs.py:156`, driven by a
  failing test for an unterminated value at end of file
- Add failing-first tests for `list_keys`'s `OSError` and `TimeoutExpired`
  branches
- Bound the manifest read and refuse non-file members in `_browsers_member`
- Switch `CVE_ID` and `GHSA_ID` to `fullmatch` with `re.ASCII`
- Correct the attempt-time bound in `RELEASING.md` and the plan, or pass an
  `httpx.Timeout` whose phases sum to the intended 20 s
- Update the stale 404 test double, reflow `RELEASING.md:364`, and fix the
  guard-tripped advice to mention `--maximum-new-issues`
- Consider the `RUSTUP_HOME`/`cache_key_prefix` pattern from `main.yml`, or
  `MISE_TASK_RUN_AUTO_INSTALL=false` with `install_args`, once the first
  runs show the setup time
- Run `clean-comments` over the branch to remove the restating docstrings
