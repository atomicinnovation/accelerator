---
type: pr-description
id: "82"
title: "Add linux-arm64 entries to mise.lock"
date: "2026-08-26T23:26:43+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
pr_url: "https://github.com/atomicinnovation/accelerator/pull/82"
pr_number: 82
tags: []
revision: "fc1eeee37c614452e4bc4b1ed4aaaac3527a22e7"
repository: "accelerator"
last_updated: "2026-08-26T23:26:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Add linux-arm64 entries to mise.lock

## Summary

The Main pipeline went red on `main` immediately after #72 merged: the new `Smoke vendored runtime (linux-arm64, ubuntu-24.04-arm)` job fails at "Install dependencies" because `mise.lock` carries no `linux-arm64` platform entries, so mise's locked mode cannot resolve `python@3.14.4` on that runner. This PR regenerates the lockfile for `linux-arm64`, restoring parity with the other three platforms and unblocking the pipeline.

## Changes

- Add `linux-arm64` platform URLs and checksums for every locked tool in `mise.lock` (python, uv, node, jj, gh, jq, shellcheck, shfmt, cargo-deny, cargo-nextest, cargo-llvm-cov, actionlint), generated with `mise lock --platform linux-arm64`.
- The change is purely additive — 63 insertions, 0 deletions — leaving the existing `linux-x64`, `macos-arm64`, and `macos-x64` entries untouched.

## Context

Root cause of the post-#72 Main failure. #72 (vendored-runtime distribution) introduced the `smoke-runtime` matrix whose `linux-arm64 / ubuntu-24.04-arm` lane is the first CI job ever to run mise on Linux/arm64; the lockfile had only ever been generated on the three previously-used platforms, so the arm64 lane is the first to hit the gap. No application code is involved — this is a tooling-lockfile completeness fix. There is no linked work item.

## Testing

- [x] Ran `mise lock --platform linux-arm64` locally: it resolves every arm64 asset — including `cpython-3.14.4+20260414-aarch64-unknown-linux-gnu` and `uv` — and writes 63 additive lines, reaching 12-tool parity across all four platforms.
- [ ] The `linux-arm64` smoke lane runs green — not exercisable by this PR's own CI, because the `smoke-runtime` job is gated `if: github.event_name == 'push'`; it validates on the first push to `main` after merge.

## Notes for Reviewers

This is a hotfix to unblock `main`; the diff is one file and touches only tooling metadata. The one caveat to be aware of: because `smoke-runtime` is push-only, this PR's own checks will not run the arm64 lane that was failing — the real confirmation is the first post-merge Main run. If you want pre-merge assurance beyond the local `mise lock` resolution, the alternative is to temporarily allow `smoke-runtime` on `pull_request`, which is out of scope for this fix.
