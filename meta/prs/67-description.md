---
type: pr-description
id: "67"
title: "Mark the 0185, 0194, and 0197 work items as done"
date: "2026-08-17T08:56:44+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
parent: "work-item:0136"
relates_to: ["work-item:0185", "work-item:0194", "work-item:0197"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/67"
pr_number: 67
tags: []
revision: "52368add9019ca348982ab1f641e3c08721b296d"
repository: accelerator
last_updated: "2026-08-17T08:56:44+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Mark the 0185, 0194, and 0197 work items as done

## Summary

Closes out three completed work items in the `work-item:0136` Rust CLI migration epic — 0185, 0194, and 0197 — by flipping their status from `ready` to `done` now each one's implementation has landed and been verified. This is two commits: one closing 0185 and 0197 together, and a second closing 0194.

## Changes

- `meta/work/0185-converge-corpus-adapters-on-library-backed-vcs.md`: status `ready` → `done` (converging `corpus-adapters` on the library-backed VCS adapter is complete), plus normalisation of the frontmatter's scalar/list quoting style (unquoted strings, bracket lists without per-element quotes) to match the rest of the corpus.
- `meta/work/0197-accelerator-collaboration-pr-helper-cli.md`: status `ready` → `done` (the `accelerator-collaboration` PR helper CLI is shipped), with the same frontmatter quoting normalisation applied.
- `meta/work/0194-tracker-crate-and-remote-sync-engine.md`: status `ready` → `done` (the tracker crate and remote sync engine are implemented).

## Context

All three work items sit under epic `work-item:0136` (Migrate Shell Scripts into a Rust CLI) and are closed out here as their implementations have now landed and been verified independently.

## Testing

- [ ] No code changes — work-item metadata only, nothing to run.

## Notes for Reviewers

Pure status/metadata update, no source changes. The frontmatter quoting diffs on 0185 and 0197 are a deliberate style normalisation bundled with their status flip, not a functional change.
