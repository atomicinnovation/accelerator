---
type: "pr-description"
id: "106"
title: "Mint a fresh releaser token for each release push"
date: "2026-09-08T12:23:25+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/106"
pr_number: 106
tags: []
revision: "1bea682df848105853df95003d7dde2cceb9ed39"
repository: "accelerator"
last_updated: "2026-09-08T12:23:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Mint a fresh releaser token for each release push

## Summary

The Main pipeline has been failing on every push to `main`: the `prerelease`
job bumps the version, commits it, then fails at `git push` with
`fatal: could not read Username for 'https://github.com': Device not
configured`. The push credential — a GitHub App token minted at checkout —
expires after 60 minutes, and the four-target cross-compile in `*:prepare`
routinely runs longer, so the token is dead by the time `finalise` pushes.
This PR mints a fresh token immediately before each push instead.

## Changes

- **Re-mint the releaser App token right before each finalise step**
  (`.github/workflows/main.yml`): one in the `prerelease` job, two in the
  `release` job (stable and post-stable), each seconds before its push.
- **Push over a token-authenticated remote URL** (`tasks/git.py`): `git.push`
  injects `RELEASER_TOKEN` into the origin URL, left unexpanded so the shell
  substitutes it at push time and the secret never enters this process's
  arguments. Local dev has no `RELEASER_TOKEN` and pushes through the
  developer's own `origin` credential unchanged.
- **Drop `persist-credentials` on the release checkouts**: the stale checkout
  credential leaves a host-matched `extraheader` that would collide with the
  fresh token on the push. The repository is public, so clone and pull work
  anonymously without it.
- **Regression guards** (`tests/unit/tasks/`): a new `test_git.py` pins the
  token-URL push and the `origin` fallback; new `test_workflows.py` cases
  assert every finalise step is preceded by a fresh App-token step wired
  through `RELEASER_TOKEN`, and that neither publishing checkout persists
  credentials.

## Context

No linked work item — a direct fix for the red Main pipeline on `main`. Root
cause confirmed across the recent failing runs: token minted at checkout
(e.g. `00:18:29`), push attempted 69 minutes later (`01:27:30`), well past the
GitHub App token's hard 60-minute TTL. The one recent run that succeeded had a
warm cargo cache and finished the compile under the hour. Direct pushes to
`main` are required because the branch ruleset mandates pull requests; only
the releaser App (a bypass actor) can push the version bump, so the fix keeps
the App token and refreshes it rather than switching to `GITHUB_TOKEN`.

## Testing

- [x] `mise run build-system:check` — format, lint, types, and actionlint all
  pass.
- [x] `uv run pytest tests/unit/tasks` — 2786 passed, including the new
  `git.push` and workflow-wiring guards.
- [ ] End-to-end release push — cannot be reproduced locally; the fix rests on
  the 60-minute TTL, the public-repo anonymous-fetch guarantee (confirmed via
  anonymous `ls-remote`), and the unit + actionlint coverage. Verified in CI on
  merge.

## Notes for Reviewers

- The token reaches `git` as `https://x-access-token:${RELEASER_TOKEN}@...`
  built in Python with the variable **left literal**; the child shell expands
  it. Confirm you are comfortable that the token stays out of `argv` and logs
  (invoke does not echo the command; the App-token output is auto-masked).
- The top-of-job App-token step and App-token checkout are retained (public
  repo, so only the clone uses them) to satisfy the existing app-token wiring
  guards; the push no longer depends on that credential.
- The `release` job now mints the token three times per run in total (once at
  checkout, once per finalise). Each installation-token mint is cheap and
  self-revoking at post-job.
