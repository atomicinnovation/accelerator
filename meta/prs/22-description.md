---
type: pr-description
id: "22"
title: "[0179] Mark the corpus crates work item done"
date: "2026-07-20T15:10:13+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
work_item_id: "0179"
parent: "work-item:0179"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/22"
pr_number: 22
tags: []
revision: "9565afcd102f90b9467851b633ce450812f40f1f"
repository: "accelerator"
last_updated: "2026-07-20T15:10:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0179] Mark the corpus crates work item done

## Summary

Transitions work item 0179 ("corpus and corpus-adapters Crates for Parsing and Conventions") from `draft` to `done`. The implementation itself landed separately via PR #21; this change closes out the work item's status and records that every acceptance criterion is now satisfied.

## Changes

- Flip 0179's status `draft` → `done` (frontmatter `status:` and the `**Status**:` body line).
- Check off all 11 acceptance criteria (`[ ]` → `[x]`), covering the serde-free `corpus` domain closure, byte-preserving frontmatter round-trip, single-sourced serde-saphyr via the shared document-format crate, the bounded-time adversarial fixture guard, bash-parity for doc-type/linkage/slug inference and work-item-ID handling, the quote/CRLF-preserving `status:` write convention, faked-port artifact-metadata parity, VCS-technique recording, and the collapse of the duplicated width parsers and title-casers.

No source, test, or build changes — this is a work-item metadata transition only.

## Context

- Work item: `meta/work/0179-corpus-crates-parsing-conventions.md` (parent epic 0136 — Migrate Shell Scripts into a Rust CLI; parent task 0166).
- Implementation delivered in PR #21 (`0179-corpus-crates`); this PR is the follow-up bookkeeping that marks the item complete.

## Testing

- [x] Verified via `gh pr diff` that the change is confined to the 0179 work item file — only the `status:` field, the `**Status**:` line, and the 11 acceptance-criteria checkboxes changed; the body is otherwise untouched.
- [ ] No automated suite applies — the change touches only a `meta/work/` markdown work item, not any of the four toolchains, so `mise run check` is not exercised by this diff.

## Notes for Reviewers

Bookkeeping-only PR: confirm the acceptance criteria are genuinely satisfied by the merged 0179 crates (PR #21) before approving, since the diff itself only asserts completion rather than demonstrating it.
