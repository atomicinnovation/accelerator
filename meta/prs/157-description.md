---
type: "pr-description"
id: "157"
title: "[0225] Guard the vendored runtime pins with scheduled advisory and age checks"
date: "2026-10-10T23:36:18+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0225"
parent: "work-item:0225"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/157"
pr_number: 157
tags: []
revision: "80a3c9974341bd1f7a213c6508173762e7471bfe"
repository: "accelerator"
last_updated: "2026-10-10T23:36:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0225] Guard the vendored runtime pins with scheduled advisory and age checks

## Summary

`cargo-deny` covers only Rust crates, so nothing watched the vendored runtime
(`playwright-core`, Node and the Chromium build Playwright pins) or the Node
release keyring used to verify it. This PR adds `vendor:guard-pins` and a
daily `runtime-pin-guard` workflow. The workflow opens one deduplicated GitHub
issue, assigned to the owner, for each of these:

- a pin matching a disclosed advisory (for Chromium, a known-exploited CVE);
- a pin or the keyring older than its maximum age;
- a Node release key near or past expiry;
- a failing advisory feed.

It also bumps the vendored Node to 22.23.2 and fixes two flaky tests found
along the way.

## Changes

- **Guard package, `tasks/shared/vendor/pin_guard/`.** One run reads and
  validates the local inputs, evaluates every check, then reconciles against
  existing issues. Any abort happens before the first issue is written.
  - **Age checks.** A 90-day limit applies to each pin and a 365-day limit to
    the keyring. Age is measured from new `bumped` dates in `pins.toml`, never
    from VCS history.
  - **Key-expiry check.** It lists `keys/nodejs-release.asc` with
    `gpg --with-colons`, without importing it. Primary keys and signing
    subkeys inside a 60-day window, or already expired, are findings.
  - **Advisory checks.**

    | Check | Feeds | Matches |
    | --- | --- | --- |
    | `playwright-core-advisories` | OSV `querybatch` | npm `playwright` and `playwright-core` GHSAs |
    | `node-advisories` | Node security WG `vuln/core` | entries for `all`/`darwin`/`linux` whose ranges cover the pin |
    | `chromium-advisories` | npm registry, CISA KEV, OSV | exploited Chromium CVEs whose fix is above the declared `browserVersion` |

  - **Feed failures.** A failed feed skips only the checks that depend on it
    and opens a feed-outage issue; every other check still runs. Each request
    is retried at most twice, with a 20 s per-attempt limit and a 10-minute
    run deadline. Unknown range syntax, renamed environments or an empty KEV
    Chromium set count as feed failures, not as "clear".
  - **Reconciliation.** Each issue body ends with a hidden marker that names
    the finding. Any existing issue with that marker suppresses the finding,
    whether open or closed. An outage is suppressed only while its issue is
    still open. Only issues authored by `github-actions[bot]` count.
    - **Issue cap.** One run opens at most 10 new finding issues. If there are
      more, it opens a single guard-tripped issue that lists them and fails.
- **Task and workflow.**
  - `mise run vendor:guard-pins` prints the issues it would open, and writes
    them only with `--open-issues`. Combining `--today` with `--open-issues`
    is refused, so a simulated date cannot file real issues early.
  - `.github/workflows/runtime-pin-guard.yml` runs daily at 06:17 UTC with
    only `contents: read` and `issues: write`. A manual dispatch can raise the
    cap with `maximum-new-issues`.
  - `test_mise.py` asserts the guard is unreachable from any aggregate `mise`
    task.
- **Assembly cross-check.** `BrowsersManifest` (`tasks/shared/vendor/browsers.py`)
  replaces `browser_revision`, `assert_version_pairing` and `verify_chromium`,
  none of which were called. `assemble_tree_artifacts` now refuses a pinned
  Chromium revision that the extracted `playwright-core`'s `browsers.json`
  does not declare. The `chromium.py` docstring already claimed this check
  existed.
- **Workflow linting.** `lint:workflows` now runs `actionlint` over every
  workflow, not only `main.yml`. A new test requires every `uses:` in every
  workflow to be pinned to a 40-hex SHA.
- **Node 22.23.2.** 22.22.2 matched 21 `vuln/core` entries. 22.23.2 is signed
  by a key already in the keyring, so only the driver archives' assembled
  digests change, in `pins.toml` and `manifest.example.json`.
- **`RELEASING.md`.** A new "Vendored-runtime pin guard" section covers the
  owner line the task parses, the limits, the bump-date rule, the feeds and
  their blind spots, the owner's action for each kind of issue, the rules for
  closing issues, and schedule health checks. The trust-anchor docs no longer
  claim a fresh checkout ships placeholders.
- **Flaky-test fixes found along the way.**
  - **`command_runner.rs`.** The grace period ended as soon as the process
    group was empty, which could drop output still unread in the pipes. It now
    also waits for both pipes to reach end of file. This fixes the
    combined-cap test failing under load.
  - **`kanban-drag-overlay.spec.ts`.** The fixture is now rewritten until the
    card moves, within the same 300 s budget that `sse_e2e.rs` uses. The
    visualiser can answer HTTP before its FSEvents watches register.

## Context

- Work item: `meta/work/0225-scheduled-advisory-and-age-guards-for-the-vendored-runtime-pins.md`,
  plus its review under `meta/reviews/work/`
- Research: `meta/research/codebase/2026-10-09-0225-scheduled-advisory-and-age-guards.md`
- Plan, seven phases: `meta/plans/2026-10-09-0225-scheduled-advisory-and-age-guards.md`,
  plus its review under `meta/reviews/plans/`
- The work item was renamed from `0225-advisory-feed-monitoring-…` during its
  review passes.

## Testing

- [x] `mise run check` exits 0.
- [x] `pytest tests/unit/tasks` passes (3497 tests). Every check, the
  reconciliation and the workflow shape are tested against recorded feed
  fixtures under `tests/unit/tasks/fixtures/pin-guard/` and in-memory fakes.
- [x] `mise run vendor:guard-pins` dry run against the live feeds: no feed
  failures, 26 findings, cap tripped (details below).
- [ ] Full `mise run` default not re-run after the final comment-only commit.
- [ ] First scheduled or dispatched run on GitHub Actions, to confirm the
  `GITHUB_TOKEN` can create the label, assign the owner and open issues.

## Notes for Reviewers

- ⚠️ **The first run will trip the cap and fail.** Today's dry run finds 26
  new findings:

  | Kind | Count | Detail |
  | --- | --- | --- |
  | Pin age | 2 | `playwright-core` 1.55.1 and Chromium 1193, both 145 days |
  | Key expiry | 3 | `4DAA80D1E737BC9F` expires 2026-12-09; `8BEAB4DFCF555EF4` and `04CD3F2FDE079578` already expired |
  | Chromium advisory | 5 | CVE-2025-13223, CVE-2025-14174, CVE-2026-2441, CVE-2026-3909, CVE-2026-3910 |
  | Chromium CVE to assess | 16 | OSV has no record, or no four-part fix |

  The genuine findings call for a Playwright bump; this PR does not attempt
  one.
- ❓ **Old CVEs to assess by hand.** 11 of the 16 assess-by-hand CVEs date
  from 2016 to 2023 and are almost certainly fixed in Chromium 140. They fall
  through because OSV gives no four-part fix, a documented blind spot. Options:
  - **Dispatch with a higher cap** and close those issues with a comment.
  - **Add a cutoff** by KEV `dateAdded` or by CVE year, before merging.
- **Closing an issue suppresses its finding for good.** For advisories, that
  covers every later version of the pin, so the rules in `RELEASING.md` under
  "Closing issues" matter.
- **Out of scope** (see the plan's "What We're NOT Doing"):
  - auto-closing issues;
  - verifying registry signatures or provenance in the guard;
  - non-KEV Chromium CVEs;
  - an automated heartbeat for the schedule;
  - pinning `gpg` through `mise`.

https://claude.ai/code/session_01YaFGsEvG11kQBXJ7PoTxLb
