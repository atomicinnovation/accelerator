---
type: "pr-description"
id: "184"
title: "Give the Jira page-cap test the default request timeout"
date: "2026-11-09T13:43:50+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/184"
pr_number: 184
tags: []
revision: "7dc837e39c89e545b8c257dd356c3644cb77ecba"
repository: "accelerator"
last_updated: "2026-11-09T13:43:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Give the Jira page-cap test the default request timeout

## Summary

`a_paginated_fixture_stops_at_the_page_cap_and_reports_indeterminate` proves
that the Jira client stops paging at its 50-page cap, but it ran each of its 50
requests under a 400 ms timeout. On a loaded macOS runner one request stalled
past that, paging stopped early, and the cap assertion failed. With the default
request timeout, only the cap can stop the loop, which is what the test means
to show.

## Changes

- `cli/jira-client/tests/timeouts.rs`: the page-cap test builds its client with
  `TransportConfig::default()` (a 30 s request timeout) instead of
  `at(Duration::from_millis(400))`. Nothing else changes.

## Context

The failure appeared once, in "Run unit tests (macos-latest)" on #179
([run 42621206783](https://github.com/atomicinnovation/accelerator/actions/runs/42621206783/job/129173505330)):
paging stopped at 34 of 50 pages. The identical code passed #180's macOS job,
and #179's job passed on a re-run. This fix was pushed to #179 and #180 moments
after they merged, so it missed `main`; this PR lands it on its own.

It is runner jitter, not a slower client. Locally the 50 requests take
0.31–0.32 s on `main`'s reqwest 0.13, and up to 0.40 s on the reqwest 0.12 it
replaced. On the macOS runner the whole test took 1.5 s before that upgrade and
2.1 s in the failing run, with a single request past 400 ms ending the loop.

## Testing

- [x] `cargo nextest run -p jira-client -E 'binary(timeouts)'`, run 20 times on
  `main` with this change: all 6 tests passed every time
- [x] `cargo clippy -p jira-client --all-targets --locked -- -D warnings` and
  `cargo fmt -p jira-client --check` are clean
- [ ] The failure is intermittent and was not reproduced on a loaded macOS
  runner; CI here shows only that the test still passes

## Notes for Reviewers

- The other tests in `timeouts.rs` keep their 400 ms and 1 s timeouts on
  purpose: they are about timeouts, and bound elapsed time with a generous
  upper limit. Only the page-cap test, which never meant to test a timeout,
  changes.

https://claude.ai/code/session_01HEhrwWctXLBXgcgijec6N8
