---
type: "pr-description"
id: "56"
title: "Add the in-process section diff work item"
date: "2026-08-08T12:51:42+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0201"
parent: "work-item:0201"
relates_to: ["work-item:0170", "work-item:0174", "work-item:0188", "work-item:0198"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/56"
pr_number: 56
tags: []
revision: "41d384342e46b108f673ac9119b5d9913fa73ca3"
repository: "accelerator"
last_updated: "2026-08-08T12:51:42+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Add the in-process section diff work item

## Summary

Adds work item 0201, capturing a follow-up to 0170: replace `cli/work-adapters/src/diff_shellout.rs`'s subprocess-based `diff -u` invocation — writing two temp files and shelling out to the real `diff` binary — with an in-process Rust implementation. No code changes in this PR; it's a single new work item file.

## Changes

- **`meta/work/0201-in-process-section-diff.md`** — new `task`-kind work item, `status: draft`, `priority: medium`, parented under `work-item:0174` (the shell-tooling-retirement epic), relating to `work-item:0170` (where the subprocess diff was originally built), and `work-item:0188`/`work-item:0198` (the analogous subprocess-to-in-process migration pattern already applied on the VCS side).
- Captures six Given/When/Then acceptance criteria covering: the diff renderer running fully in-process with no `std::process::Command`; the existing `=== name (- LOCAL / + REMOTE) ===` header/framing staying unchanged; empty-diff behaviour on identical sections; retiring or rewriting the `bash-parity` feature and `diff_shellout_parity.rs` suite (which currently assert byte-identical output against the real binary); removing or updating `cli/pup.ron`'s `work_adapters_filesystem_reads_in_process` rule (carved out solely to quarantine this one subprocess-spawning module); and a green `mise run` with no `diff` binary dependency left in the `work`/`work-adapters`/`work-cli` crates.
- Names `similar` in Technical Notes as a researched (not mandated) default crate recommendation — dependency-free, provides `TextDiff::unified_diff()` directly, ~14.8M downloads/month — with `imara-diff` and `diffy` noted as alternatives.

## Context

Surfaced directly out of the 0170 PR (#55) review: `diff_shellout.rs`'s subprocess call was a deliberate choice during the bash-to-Rust port, to guarantee byte-identical output against the legacy script while the `bash-parity` suite verified it. That parity requirement no longer holds now that 0170 is done and merged — the diff body's internal hunk formatting doesn't need to match GNU diffutils byte-for-byte, only the existing header/framing contract does. A `meta/work` search turned up no existing item covering this; the closest candidates were 0170 (where the code lives) and 0174 (the general shell-retirement epic), used here as `relates_to`/`parent` respectively.

## Testing

- [x] Not applicable — this PR adds only a Markdown work item, no code. `mise run` is unaffected.

## Notes for Reviewers

- This is a pure work-item addition, deliberately based on `main` rather than stacked on the 0170 branch (#55), since it's unrelated follow-up work rather than part of that story.
- Two open questions are recorded on the work item rather than resolved here, left for implementation-time judgment: whether `diff_shellout.rs`/the module should be renamed once it no longer shells out, and whether it's genuinely the only subprocess-spawning code in `work-adapters` (which would let the pup.ron isolation rule be deleted outright rather than narrowed).
- The item is currently unsynced (no `external_id`) — Linear is the configured integration but the push was deliberately declined when the item was created; push later via `/sync-work-items` or `/create-linear-issue 0201` if desired.
