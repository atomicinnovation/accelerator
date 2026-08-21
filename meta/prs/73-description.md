---
type: pr-description
id: "73"
title: "Migrate the work-item subsystem from bash scripts to the Rust CLI"
date: "2026-08-21T15:38:50+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
work_item_id: "0212"
parent: "work-item:0212"
relates_to: ["work-item:0211", "work-item:0213", "work-item:0171"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/73"
pr_number: 73
tags: [rust, cutover, work-items, cli, tracker, sync]
revision: "45bb063038ec39ea7abb2d44bc015730a7e22fdf"
repository: "accelerator"
last_updated: "2026-08-21T15:38:50+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Migrate the work-item subsystem from bash scripts to the Rust CLI

## Summary

Retires the eighteen `work-item-*.sh` / `test-work-item-*.sh` scripts and the build-system machinery that propped them up, replacing every capability they carried with Rust behind `accelerator work …`. Three acceptance criteria could not pass without new `RemoteTracker` port operations, so this PR builds that feature work first (unkeyed discovery, create-preview field resolution, update-preview validation), repoints the work skills onto the CLI, then performs the irreversible deletions last.

## Changes

- Extend the `RemoteTracker` port with `search`, `preview_create`, and `validate_update`, implemented for both the Jira and Linear clients, so the sync engine can discover untracked remotes, resolve create-preview fields, and validate update payloads against the live tracker.
- Grow the sync engine with a create-from-remote path: a new `Action` variant, remote-only facts, untracked-pull discovery, unsynced-create, and preview validation that now issues a real port call instead of early-returning.
- Add a `work list` command and a create-push preview (with retry folded into the CLI) to `work-cli`, and repoint the `create-work-item`, `extract-work-items`, `list-work-items`, `review-work-item`, and `sync-work-items` skills onto `accelerator work`.
- Convert the bash↔Rust parity suites into pure-Rust tests over relocated per-crate fixtures, preserving the committed goldens byte-for-byte, then delete the nineteen shell scripts, their exit-code tables, and the build-system scaffolding that referenced them.
- Add a live corpus-seed harness over the tracker port with offline-testable guards, and scrub stale bash-script references from comments across the tree.

## Context

Implements work item [0212](../work/0212-work-item-script-cutover.md) per the [script-cutover plan](../plans/2026-08-19-0212-work-item-script-cutover.md) and its [review](../reviews/plans/), grounded in the [cutover codebase research](../research/codebase/). Sits alongside 0211 (integration-binary decomposition) and 0213 (conflict-resolution flow); completes the work-item half of the Rust CLI migration begun in 0171.

## Testing

- [x] `mise run cli:check` — workspace-wide rustfmt + clippy across the touched crates.
- [x] Rust test suites for `work`, `work-adapters`, `work-cli`, `tracker`, `jira-client`, `linear-client`, and `remote-projection`, including the converted parity and section-diff corpora.
- [ ] `mise run check` — full read-only CI mirror across all four toolchains (run before merge).
- [ ] Live contract runs (`jira-client` / `linear-client` `contract.rs`) require tracker credentials and are not exercised in offline CI.

## Notes for Reviewers

- Review order mirrors the plan: port additions and sync-engine growth first, skill repoints second, deletions last — the deletions are irreversible and depend on the parity conversions landing intact.
- The converted parity tests pin bash↔Rust goldens by construction; the shell baselines they replaced are gone, so the Rust goldens are now the sole oracle — check they match the frozen 0210 fixtures.
- `work-adapters` is exempt from the public-api gate; the tracker port surface (`cli/tracker/tests/fixtures/public-api.txt`) is the reviewed contract boundary.
- Large diff (163 files, +10.5k/−5.7k) but dominated by relocated fixtures and generated goldens; the load-bearing logic is in `cli/tracker`, `cli/*-client/src/client.rs`, and `cli/work-adapters/src/sync/`.
