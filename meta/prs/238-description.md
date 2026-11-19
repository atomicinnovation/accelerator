---
type: "pr-description"
id: "238"
title: "[0364] Wait for the /dev deep-link landing before the scroll-hash check"
date: "2026-11-18T07:29:25+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0364"
parent: "work-item:PP-956"
relates_to: ["plan:2026-11-15-PP-956-parallelise-the-serial-test-suites"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/238"
pr_number: 238
tags: ["visualiser", "testing", "playwright", "flakiness"]
revision: "f263dda19e696120c196a201ac31fcc88b950b60"
repository: "accelerator"
last_updated: "2026-11-18T07:29:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0364] Wait for the /dev deep-link landing before the scroll-hash check

## Summary

This PR fixes a race in the Playwright spec "scrolling back to the top
clears the hash to bare /dev". The spec now waits for the page to land
its `#code` deep link before it scrolls back to the top. Before, on a
busy runner, the landing could come after the spec's scroll and leave
the hash on `#code`. The race became visible once #232 ran the
Playwright projects concurrently with retries off.

## Changes

- **`e2e/dev-design-system.spec.ts`**:
  - A `scrollTop` helper sits beside the spec's `hash` and `theme`
    helpers.
  - After `goto("/dev#code")`, the test polls until the scroll root has
    moved off the top, which is the deep-link landing. Only then does it
    scroll back to the top and expect the bare hash.

No page code changes, and the spec gains no retry, `skip` or `fixme`.

## Context

- Work item: `meta/work/PP-956-parallelise-the-serial-test-suites.md`
- Plan: `meta/plans/2026-11-15-PP-956-parallelise-the-serial-test-suites.md`
  (Phase 7)
- **How it was found.** Phase 7's paired CI comparison ran 11 `Main`
  runs on each of two draft PRs from `main` at `a646f527`. #235 kept
  both suites' parallel workers, and #236 reverted them.
  - This spec failed on 2 of #235's 11 macOS E2E jobs (run 44189947764,
    attempts 1 and 9), and on none of #236's.
  - It had also failed once on #232's own branch, in run 44062948001 on
    `cc46ef68` with 2 workers. That failure isn't in the plan's Phase 6
    record.
- **Why it raced.** The page lands a deep link from a `requestAnimationFrame`
  after `document.fonts.ready`. `page.goto` returns at `load`, and `load`
  already waits for the fonts. The landing therefore runs in the first
  frame after the page mounts, before the scroll-spy's first
  `IntersectionObserver` callback.
  - Usually that frame has passed by the time the spec scrolls up. On a
    loaded runner it may not have, so the spec's `scrollTo({ top: 0 })`
    finds the page still at the top. It does nothing and fires no
    `scroll` event.
  - The landing then scrolls to `#code`, and the scroll-spy keeps the
    hash there. The poll saw `#code` for its whole 5 s, which matches the
    CI failures' `Received: "#code"`.

## Testing

- [x] **Red.** I added a temporary init script that delays every
      `requestAnimationFrame` callback by 400 ms, plus a check that the
      hash stays bare afterwards. Neither is committed. The unfixed spec
      failed 5 of 5 runs with `Received: "#code"`, CI's failure.
- [x] **Green.** With this fix and the same harness, it passed 5 of 5.
- [x] The whole spec file passes 12 of 12 with `--repeat-each 3`
      (`--project=chromium`).
- [x] `mise run test:e2e:visualiser`: 355 passed and 1 skipped, the same
      as before, in 44 s on 9 workers.
- [x] `mise run frontend:check` passes.

## Notes for Reviewers

- **The count stands.** By decision, this fix does not restart
  Playwright's 10-run validation count. It changes one spec's wait, not
  the suite's config or its other specs. CI here and later `main` runs
  confirm it.
- **Phase 7's paired comparison passes on timing.** Playwright's median
  is 27.1% lower on ubuntu and 12.5% lower on macOS with the workers.
  The full figures will be recorded in #232's description.
- **A page behaviour this shows, left alone.** A user who scrolls before
  a deep link lands is moved back to the linked section. This is what
  the landing is designed to do. The spec now tests the hash behaviour
  from a landed page, which is what it is named for.
- **Not covered here.** The comparison also turned up an
  integration-job failure outside the dev suite, in
  `test:integration:entrypoint`. It is being investigated separately.

https://claude.ai/code/session_01HEhrwWSAMeTH1BGPYLkEZP
