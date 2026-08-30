---
type: "pr-description"
id: "29"
title: "Stop tracking the built visualiser SPA"
date: "2026-07-28T23:40:17+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0182", "work-item:0168"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/29"
pr_number: 29
tags: ["build", "visualiser", "frontend", "jj", "gitignore", "release", "test-flakes"]
revision: "bded7c9098f1def9780ad8445619ed503197d4cc"
repository: "accelerator"
last_updated: "2026-07-28T23:40:17+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Stop tracking the built visualiser SPA

## Summary

`cli/visualiser/frontend/dist/` was both **tracked** and matched by the nested `cli/visualiser/frontend/.gitignore`. Under jj that combination destroys the tracked copy: Vite empties the directory before rewriting it, a snapshot taken while it is empty records the deletions, and the recreated files are ignored so nothing re-tracks them. The next commit — on any branch, for any reason — silently drops the SPA that `embed-dist` bakes into the release binary.

It is not hypothetical. It happened on the 0182 branch during the previous PR and was caught by eye, not by any check. This removes the contradiction by untracking the generated output, wires the four tasks that silently depended on the committed copy, and adds guards for all three properties that make it safe.

## Changes

**Untrack the generated output** — all 14 files under `cli/visualiser/frontend/dist/`. They were never added deliberately: they were swept in by `e93758de1` ("Move the visualiser server and frontend into cli/visualiser/", 0168) when the tree was relocated with build output present.

**Wire the four tasks that silently relied on the committed copy.** `lint:cli:check`, `lint:cli:fix`, `pup:check` and `test:integration:pup` compile the server with the default `embed-dist` feature, whose `build.rs` asserts `dist/index.html` exists — and unlike `lint:server:check`/`lint:server:fix` they did not gate on `build:frontend:stub`. Each was measured failing from a dist-less tree with `frontend/dist/index.html not found`, and measured green after the edge was added. `test:unit:cli`, `deny:check` and `test:integration:deny` need nothing — also measured, not assumed.

**New guards** (`tests/unit/tasks/test_frontend_dist.py`), pinning the three properties that keep this safe:

- the nested ignore rule still matches `dist/`;
- `build:server:release`, `build:server:cross-compile`, `test:unit:visualiser` and `test:e2e:visualiser` still depend on `build:frontend`, so nothing embeds or serves a stale SPA;
- the six compile-only tasks still gate on `build:frontend:stub`.

Plus a check that every font in `dist/fonts/` is reproducible from the tracked `public/fonts/` — eleven of the fourteen untracked entries were fonts, and dropping them would have lost the only copy had Vite not been sourcing them from `public/`. It is, so `public/fonts/` is the original and `dist/fonts/` pure output.

The walk is deliberately VCS-agnostic. `git ls-files` is blind inside a jj workspace, which would make a tracked-file assertion pass vacuously rather than fail — the hazard `tasks/shared/sources.py` already records.

**A fourth timing flake, on its own commit.** `forced_kill_synthesises_stopped_sentinel` spawned `sh -c "trap '' TERM; sleep 30"` and signalled it without waiting for the trap to be installed. Lose that race — which needs a loaded machine — and SIGTERM is fatal after all, the stop path reports a graceful stop, and the assertion on `forced` sees `Null`. The fake now announces itself once the trap is set and the test waits for that. This is the same family as the three fixed in #28, but a genuine race in the test rather than a budget too close to the noise floor.

## Context

- Surfaced while completing work item **0182** (PR #28), where the deletion landed in a commit and had to be reverted before review. The reviewer notes on that PR flagged it as needing its own item; this is that work.
- Root cause dates to **0168** (`e93758de1`), the visualiser fold into `cli/`.
- No work item — a repo-hygiene fix whose rationale lives in the commit messages and here.

## Testing

- [x] `mise run` (bare default) exits 0 **from a tree with no `dist/` at all** — the fresh-clone case, which is what this change creates.
- [x] After that full run rebuilt the SPA, `jj status` reports **zero** `dist` entries and nothing under `dist/` is tracked. That is the original bug's exact trigger, now producing nothing.
- [x] `mise run check` exits 0 from a dist-less tree.
- [x] `mise run build:server:release` from a dist-less tree regenerates the SPA with **identical asset hashes** (`index-BDjK11f4.css`, `index-gqMEiSsF.js`), confirming releases are self-sufficient.
- [x] Each of the four newly-wired tasks measured failing before the edge and passing after, from a dist-less tree.
- [x] The three mise/gitignore guards are falsifiable — mutation-tested by dropping the pup stub edge, the `dist/` ignore rule, and the release `build:frontend` edge in turn; each turns the suite red.
- [x] `orchestration_lifecycle` 9/9, and the previously-flaky suites green in the same full run.

## Notes for Reviewers

- **The ergonomic cost is real and worth a decision.** A bare `cargo build` in `cli/` on a fresh clone now fails until the frontend is built, where before the committed `dist/` made it work. `build.rs` already prints the remedy (`run npm run build … or use --features dev-frontend`), and every `mise` path is wired, but anyone bypassing mise sees a new failure. The alternative — keeping `dist/` tracked and removing it from `.gitignore` — was rejected because it makes every rebuild dirty the tree with an 880KB bundle diff.
- **`build:frontend:stub` is not a build.** It writes a placeholder `index.html` purely to satisfy the `build.rs` existence check for lint-only compiles. The tasks given that edge only ever *compile* the server; anything that runs or embeds it gets the real `build:frontend`. The guard encodes that split so the two do not drift.
- The orchestration-test fix is a separate commit and can be dropped independently if you would rather it went elsewhere.
