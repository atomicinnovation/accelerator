---
type: "work-item"
id: "0208"
title: "Runtime Test Lane Absent From Every Build"
date: "2026-08-13T13:33:39+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
relates_to: ["work-item:0196"]
tags: ["ci", "testing", "design", "playwright"]
last_updated: "2026-08-20T00:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-738"
---

# 0208: Runtime Test Lane Absent From Every Build

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`test:integration:design-automation` runs in no build — not locally under
`mise run`, not in any CI job. It is the only lane exercising the Playwright
executor daemon against a real browser, and it is reached only by typing the task
by hand. Six defects accumulated behind it undetected.

## Context

The lane is deliberately absent from the `test:integration` aggregate and the
default task, commented at `mise.toml:284-287`: the suites need a bootstrapped
Playwright runtime, which no CI lane provisions, and they fail rather than skip
without one — so wiring them into an aggregate as-is would redden every build.

That reasoning is sound and the conclusion is incomplete. **The repo already
solves this shape twice**, both times with a dedicated job rather than an
aggregate leg:

| Lane | Absent from aggregate because | CI job |
|---|---|---|
| `test:e2e:visualiser:docker` | needs Docker | `test-visual-regression` |
| `test:integration:zero-spawn:strong` | needs `sudo` binary shadowing; fails under macOS SIP | `check-zero-spawn` |
| `test:integration:design-automation` | needs a bootstrapped Playwright runtime | **none** |

So the gap is an omission, not a constraint the repo hasn't met before.

What was hiding there, found on 2026-08-13 by running it by hand during
`plan-validation:2026-08-11-0196-design-cli-migration`: three defects in the
migrated executor (a forwarding allowlist omitting four documented commands, an
unreachable `WriterUnavailable` state respawning the daemon on every invocation
in containers, and a harness never updated for the identity handoff) and three in
the retained daemon wall clock (a backstop pre-empting the graceful path it
guards, an envelope written onto an unterminated HTTP response, and a cold start
charged to the first operation's budget).

The plan that introduced the lane also converted 14 self-skipping tests into hard
failures precisely so an absent runtime would be a visible refusal rather than a
silent pass. That worked. It just has no observer.

## Decision (2026-08-20)

The vendored-runtime plan has landed the deletions that foreclose the
bootstrap-now approach: `ensure-playwright.sh` and the lockhash namespace are
gone, so there is no script for a CI job to run. **The container lane from
`plan:2026-08-11-0196-design-vendored-runtime-distribution` owns the CI job** —
it provisions the runtime once and asserts zero skips across the whole set, so
this item's acceptance is satisfied by pointing at that lane rather than adding a
second job. The lane is not built yet — it is gated on that plan's container
harness — so this item stays open until the harness lands.

## Requirements

- The design-automation runtime suites must be exercised by a scheduled build
  rather than a manual invocation.
- Two candidate approaches were weighed; see the Decision above. Only the second
  survives:
  - **Dedicated CI job that bootstraps** — ran `ensure-playwright.sh` then the
    lane. **Foreclosed**: the vendored-runtime plan deleted that script and the
    lockhash namespace, so there is nothing to bootstrap.
  - **Container lane from the vendored-runtime plan** (chosen) — that plan moves
    these suites into a lane which already provisions a runtime and can then
    assert zero skips across the whole set. No throwaway work; delivers nothing
    until it lands.
- Whichever is chosen, an absent or broken runtime must fail the job visibly. Do
  not restore conditional execution at the task layer — that relocates the
  silent-pass shape rather than removing it.
- The local story must be stated explicitly: either `mise run` continues not to
  run the lane (and the docs say where it does run), or it runs only when a
  namespace is already bootstrapped. The second reintroduces conditional
  execution and is the weaker option.

## Out of scope

- Changing what the suites assert. This is wiring only.
- `test:unit:design-automation`, which already runs via the `test:unit` roll-up.
- Provisioning a runtime for any other lane.

## Acceptance Criteria

- [ ] Given a push to a branch, when CI runs, then the design-automation runtime
      suites execute and their result gates the build.
- [ ] Given a CI environment with no Playwright runtime available, when the lane
      runs, then the job fails with a message naming the missing namespace — it
      does not skip, pass vacuously, or succeed with zero tests executed.
- [ ] Given the lane executes, when it reports, then the count of executed tests
      is asserted against a floor, so a suite that silently stops being
      discovered fails the build.
- [ ] `mise.toml:284-287`'s comment no longer says no CI lane provisions a
      runtime, or is replaced by one naming where the lane does run.
- [ ] The repo's own CI documentation lists the new job alongside
      `test-visual-regression` and `check-zero-spawn`.

## Open Questions

- ~~Which approach — bootstrap-now or wait for the container lane?~~ Resolved:
  the container lane, since the bootstrap script no longer exists (see Decision).
- Does the lane need the OS matrix the other test jobs use, or is one runner
  enough? The executor's adapter layer is platform-specific (`/proc` on Linux,
  `sysctl` on Darwin), but that half is already covered by `test:unit:cli` on
  both runners.
- Is a cached runtime tree acceptable in CI, keyed on the vendored artifact's
  digest? (The lockhash namespace this once referenced no longer exists.)

## Dependencies

- Relates to: 0196 — the migration that created the lane and whose validation
  exposed the gap.

## Assumptions

- CI is permitted to download a Chromium build. If the vendored-runtime work has
  already constrained that, the bootstrap approach is off the table and this
  collapses to the container option.
- ~90s of wall-clock for the lane is acceptable in a build. That is what it takes
  locally.

## Technical Notes

- Task definition: the `test:integration:design-automation` block in
  `mise.toml`; preflight and named-suite discovery in
  `tasks/test/integration.py` (`_resolve_driver_tree` +
  `_DESIGN_AUTOMATION_RUNTIME_SUITES`). The preflight now resolves the vendored
  driver tree, not a lockhash namespace.
- Precedents to copy: `.github/workflows/main.yml:122-145`
  (`test-visual-regression`) and `:332-345` (`check-zero-spawn`) — the latter's
  comment explains why a lane with an environmental prerequisite gets its own job
  instead of an aggregate leg.
- The lane discovers `test-run.js` and `daemon-runtime.test.js` by name, not by
  glob, so a new runtime-dependent suite must be added there explicitly.
- The executed-count floor pattern already exists for the unit lane at
  `tasks/test/unit.py:62,66`.

## Drafting Notes

- Framed as an omission rather than a new capability, on the strength of the two
  existing dedicated-job precedents. If those were themselves reluctant
  compromises, the framing is too confident.
- Titled after the problem, not a solution, because the approach is the open
  question.
- Read "a build that actually runs it" as CI rather than local, since no local
  invocation is scheduled either. The local story is captured as a requirement
  rather than assumed away.
- Kept as a task, not a bug: nothing is broken in the product, the guard is
  missing.
- Standalone rather than a child of 0196 so it survives 0196 closing — CI
  coverage is its own concern that 0196 merely exposed.

## References

- Validation that exposed the gap:
  `meta/validations/2026-08-11-0196-design-cli-migration-validation.md`
- Related: 0196
- Sibling plan owning the container lane:
  `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md`
