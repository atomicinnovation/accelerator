---
type: "pr-description"
id: "225"
title: "[0364] Keep the resolved-styles probes off the fixtures the drag specs write"
date: "2026-11-16T14:55:20+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0364"
parent: "work-item:0364"
relates_to: ["plan:2026-11-15-0364-parallelise-the-serial-test-suites"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/225"
pr_number: 225
tags: ["visualiser", "testing", "playwright", "fixtures", "flakiness"]
revision: "2978f894885963908444e6c76f325ce3b82e3dda"
repository: "accelerator"
last_updated: "2026-11-16T14:55:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0364] Keep the resolved-styles probes off the fixtures the drag specs write

## Summary

This PR implements Phase 4 of the 0364 plan, the first phase of the
Playwright track. Phase 6 will run the `resolved-styles` and `chromium`
projects at the same time. The `chromium` drag specs rewrite work items
0001, 0003, 0005 and 0006 in place. This PR makes sure that no
`resolved-styles` probe measures an element that comes from one of those
files.

Four probes did, and each is now pinned to work item 0002, a `done` item
that no spec writes. The suite still runs on one worker, so this change
is safe on its own. It only changes which element each probe measures,
not what it asserts.

## Changes

- **`tests/lib/stable-fixtures.ts`** (new) names the unwritten item once:
  - `UNWRITTEN_WORK_ITEM` (`0002-second-work-item`);
  - `UNWRITTEN_WORK_ITEM_CARD`, its kanban card
    (`li[data-relpath="meta/work/…"]`);
  - `UNWRITTEN_LIFECYCLE_CARD`, its `/lifecycle` cluster card.
- **`tests/lib/detail-route-slugs.ts`**: the `work-items` detail slug moves
  from 0001 to `UNWRITTEN_WORK_ITEM`. `fixture-coverage` and
  `detail-eyebrow-resolved-colours` assert the same things against a
  `done` item.
- **`typography-resolved-sizes.spec.ts`**: the kanban card title and id
  probes are scoped to 0002's card.
- **`radius-resolved-radii.spec.ts`**: the kanban stage-dot probe is
  scoped to 0002's card. The `/lifecycle` card probe is scoped to 0002's
  cluster card.
- **The plan**: brought up to the Phase 3 branch's version, with Phase 4
  ticked and the extra `/lifecycle` conflict recorded.

## Context

- Work item: `meta/work/0364-parallelise-the-serial-test-suites.md`
- Plan: `meta/plans/2026-11-15-0364-parallelise-the-serial-test-suites.md`
  (Phase 4)
- Independent of the dev-suite track (#222), and based on `main`.

## Fixture audit

A probe conflicts when the element it measures, or that element's
computed style, comes from a fixture that a concurrently running spec
writes.

### How each list was traced

- **Specs' own file access.** I grepped `e2e/` and `tests/` for
  `writeFileSync`, `readFileSync`, `request.patch` and `request.post`.
- **Server file opens.** I ran each project on its own under
  `sudo fs_usage -w -f filesys accelerator-visualiser`, starting the trace
  before Playwright started the server, then filtered to
  `tests/fixtures/meta/`. `fs_usage` truncates long paths at the front,
  so the filter matched on the `fixtures/meta/` suffix. Each rename's
  target is the file the server re-reads straight after it.
- **Index-served aggregate routes.** I read the code and queried
  `/api/lifecycle` and `/api/related/…` against the fixture server.

### `chromium`

| Fixture | Written by | How |
|---|---|---|
| `work/0001-first-work-item.md` | `kanban`, `kanban-click-vs-drag`, `default-pattern` (`docs[0]`) | specs' `writeFileSync`; server PATCH (2 temp-file renames in the trace) |
| `work/0005-sse-test-work-item.md` | `kanban`, `kanban-drag-overlay` | specs' `writeFileSync`; server PATCH (3 temp-file renames in the trace) |
| `work/0006-conflict-test-work-item.md` | `kanban-conflict` | specs' `writeFileSync` (the server re-read it 6 times) |
| `work/0003-third-work-item.md` | `legacy-schema` | `writeFileSync` of identical content; in this run its 5 opens match the unwritten items, consistent with the spec's lookup skipping |

`chromium` reads every work item, because the board shows them all.

### `resolved-styles`

- **Writes:** none. The trace shows no write opens, renames or chmods.
- **Reads of written fixtures:** 0001, 0003, 0005 and 0006 are each
  opened once, all at 13:00:19.285. That is the server's start-up index;
  the server first appeared at 13:00:19.236. No request after that opens
  them.
- **Reads after start-up:** 0002 (detail pages, 3 more opens) and 0099
  (related-row pages, 32 opens).

### Conflicting probes found and pinned

| Probe | Why it conflicted | Pin |
|---|---|---|
| Work-item detail (`fixture-coverage`, `detail-eyebrow-resolved-colours`) | opened 0001 | `DETAIL_ROUTE_SLUGS["work-items"] = UNWRITTEN_WORK_ITEM` (0002) |
| `/kanban` card title and id (typography) | the `.first()` card could be a written one | scoped to `UNWRITTEN_WORK_ITEM_CARD` |
| `/kanban` stage dot (radius) | same | scoped to `UNWRITTEN_WORK_ITEM_CARD` |
| `/lifecycle` index card (radius) | cards are sorted by most recent change, so a write puts that item's cluster first | scoped to `UNWRITTEN_LIFECYCLE_CARD` |

### Probes recorded as not conflicting

- **Page chrome:** headings, eyebrows, the sidebar (including its META
  block and the activity heading and badge), the brand, tokens, the sort
  and filter menus, and `/kanban` column headings, which come from the
  column config.
- **`/lifecycle/first-plan`:** its cluster is the plan and its two
  reviews, with no work item.
- **`/library/plans/first-plan`:** its related rows are its reviews and
  validation.
- **`/library/work-items/0099-ac2-coverage` and `ADR-0099`:** their
  related rows are only the `ac2-coverage` siblings, and nothing declares
  a relation to them.
- **Accepted page-level aggregate reads:** the `/library` hub has one card
  per doc type, so the work-items card's count and "latest" line read
  every work item. The `/library/work-items` listing does the same. These
  probes measure computed style, which doesn't depend on fixture content.

### Conflict classes under the per-probe definition

- A probe measuring an element rendered from a written fixture: **none**.
- A probe whose element is chosen by an order that a write can change:
  **none**.

## Mutated-fixture run

I mutated the four written fixtures into states the drag and conflict
specs can leave behind:
- 0001 moved to `done`;
- 0005 moved to `in-progress`;
- 0006 got a new title and body, and `version: 2`;
- 0003 was rewritten.

`npx playwright test --project=resolved-styles` with the task's env:
**313 passed**. I then restored the fixtures with
`jj restore cli/visualiser/server/tests/fixtures/meta/work`, and `jj diff`
of the fixtures is empty.

## Testing

- [x] `mise run test:e2e:visualiser`: 355 passed and 1 skipped
      (`legacy-schema`'s existing lookup, which 0364 leaves alone), on one
      worker.
- [x] `npx playwright test --project=resolved-styles` with the task's
      env: 313 passed.
- [x] The mutated-fixture run above: 313 passed, and the fixtures are
      restored.
- [x] `mise run frontend:check` passes.
- [x] No `resolved-styles` spec names a written fixture:
      `grep -rEn "0001-first|0003-third|0005-sse|0006-conflict" cli/visualiser/frontend/tests`
      returns nothing.
- [x] No pinned probe can pass vacuously. Each one either `waitFor`s its
      element or evaluates it through `.first()`, so a selector that
      matches nothing times out.

## Notes for Reviewers

- **One conflict was missing from the plan.** The plan listed the detail
  slug and the `/kanban` probes. The audit also found the `/lifecycle`
  card probe, because that list is sorted by recency.
- **The 0002 cluster card is located by its link.** The selector is
  `li:has(> a[href="/lifecycle/second-work-item"])`. The cluster slug is
  the work item's slug without its id prefix.
- **The plan file also changes in #222.** This branch carries #222's
  version of the plan, plus Phase 4's edits in Phase 4's own section. The
  two PRs should therefore merge cleanly in either order.
- **Out of scope.** No change is made to the `chromium` specs or the
  Docker lane. Running the two projects concurrently is Phase 6.
