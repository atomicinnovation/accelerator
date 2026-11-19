---
type: "pr-description"
id: "244"
title: "Keep explore-pr's smoke check from hanging on tall pages"
date: "2026-11-19T08:24:28+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/244"
pr_number: 244
tags: ["explore-pr", "skills"]
revision: "717eee729a461fbc44bda5efe597aa40bbc68ec1"
repository: "accelerator"
last_updated: "2026-11-19T08:24:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Keep explore-pr's smoke check from hanging on tall pages

## Summary

`smoke.js --shots` could hang indefinitely. It captured each view with
figures in one full-height screenshot. Headless Chrome never answers that
call for a page around 15,000 px tall, and the check waited on every
DevTools call with no deadline. This PR captures tall views in tiles and
puts a deadline on every DevTools call, so the check finishes, or fails
naming the call that stalled.

## Changes

### Capture limits (`scripts/capture.js`, new)

- `tilesOf(size)` splits a page into clips at most `TILE_HEIGHT` (5,000 px)
  tall that cover it exactly. A fractional height is rounded up, so the
  last pixels are kept. A page that fits in one tile is one clip with no
  suffix.
- `withinDeadline(reply, ms, call)` rejects with
  `<call> gave no reply within <ms> ms` when the reply is late. It clears
  its timer once the reply arrives, so it never holds the process open.
- `CALL_DEADLINE_MS` is two minutes. The slowest call in a passing run is
  the commits probe, at about 5.5 s.

### Smoke check (`scripts/smoke.js`)

- Every `send` goes through `withinDeadline` and drops its pending entry
  either way, so a late reply is ignored. A timeout now reaches `main`'s
  `catch`, which closes Chrome and exits 2, instead of waiting forever.
- A view with figures is captured as `<route>.png` when it fits one tile,
  and otherwise as `<route>-1.png`, `<route>-2.png` and so on.
  `wholePage` is replaced by `captureWholePage` and `captureViewport`.

### Guidance

- `SKILL.md` step 9 names the tiled screenshot files. The bullet is
  reflowed to keep the file within its 360-line test cap.
- `reference/page.md` describes the tiles and the deadline, and lists
  `scripts/capture.js`.

## Context

PR #218 recorded under Notes for Reviewers that `smoke.js --shots` "can
stall" on the longest chapter of the PR 188 review page, and left it
unfixed. Building the PR 234 review page hit the same stall: the
6,431 px overview captured, then the 15,570 px Design view never
returned and the process sat at 0% CPU. An instrumented run with per-call
timing confirmed it was `Page.captureScreenshot` with a 15,570 px clip.

## Testing

- [x] `tests/capture.test.js` was written first and failed before
  `capture.js` existed. Its 7 cases now pass: one tile, several tiles,
  no tile over the limit, fractional heights, a reply in time, a reply
  that never comes, and no timer left behind.
- [x] The skill's suite passes: `python3 -m pytest tests` in
  `.claude/skills/explore-pr`, 273 tests, including the new
  `tests/test_capture.py` wrapper for the node suite.
- [x] `ruff check` and `ruff format --check` are clean on the skill.
- [x] Against the PR 234 review page, which hung the check before this
  change, `smoke.js --shots` finished in 76 s with `"ok": true`. It wrote
  the Design view as four tiles and each chapter as three or four.
- [ ] The deadline firing inside a real `smoke.js` run was seen only in
  an instrumented scratch copy, with a 90 s limit. The shipped path is
  covered by the unit test of `withinDeadline`.
- [ ] The bare `mise run` was not run. Nothing in the repository's tasks
  lints the skill's JavaScript.

## Notes for Reviewers

- **Screenshot names change for tall views.** Anything that expects
  `design.png` or `tour-<id>.png` for a long page now finds `-1`, `-2` and
  so on. Only `SKILL.md`'s own instructions read them, and they are
  updated.
- **5,000 px is an observed safe size, not a documented Chrome limit.**
  A 6,431 px capture worked and a 15,570 px one hung. Tiles of 5,000 px
  worked across every view of the PR 234 page.
- **Tiles are not stitched together**, because that would need an image
  library and the check has no dependencies. Reading a view in order
  means opening its tiles in turn.

https://claude.ai/code/session_01HEhrwVYf1RNzYgV1oBkWYT
