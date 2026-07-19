---
type: pr-description
id: "23"
title: "[0180] Atomic store primitives"
date: "2026-07-20T15:13:35+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
work_item_id: "0180"
parent: "work-item:0166"
relates_to: ["work-item:0172"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/23"
pr_number: 23
tags: [rust, corpus, store, crates]
revision: "b4b127ac9570b7c244380b2da115c29e21a6163a"
repository: "accelerator"
last_updated: "2026-07-20T15:13:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0180] Atomic store primitives

## Summary

Ports the load-bearing atomic-store concurrency primitives from the bash implementation into the Rust `corpus`/`corpus-adapters` crate pair: atomic whole-file writes, a canonical-order JSONL record composer with an anchored remove-by-key, and a `mkdir`-based advisory lock with single-winner dead-holder reclaim. The domain contracts (error taxonomy + two driven ports) live in `corpus`; the filesystem implementations live behind those ports in `corpus-adapters`.

> Stacked on `0179-mark-work-item-done` — this PR's base branch is `0179`, not `main`. The diff shown is the delta over 0179's corpus crate scaffolding.

## Changes

**Domain contracts (`corpus`)**

- `StoreError` — a `#[non_exhaustive]` taxonomy (`NotWritable`, `LockTimeout`, `CrossFilesystem`, `Validation`, `Io`) with `Display` copy and a `From<StoreError> for kernel::Error` mapping onto `kernel::Error::Failed` for the boundary.
- `Record` / `Outcome` model — the three-value outcome (`accepted`/`edited`/`skipped`), `proposed_value` required, `user_value` presence-based, and `extras` carrying author-declared fields in declaration order.
- Two driven ports: `AtomicWrite` (whole-file atomic replacement) and `RecordStore` (canonical-order JSONL append + anchored remove-by-key).

**Adapters (`corpus-adapters`)**

- `store.rs` — `FileCorpusStore` implements both ports. Atomic writes use a same-directory temp plus atomic rename, split into `stage`/`persist` so the interruption seam is deterministically testable. A cross-filesystem rename (`EXDEV`) is classified fail-closed as `CrossFilesystem`.
- `jsonl.rs` — the canonical-order record composer and the anchored remove prefix, both routed through one `record_opener` so the load-bearing `{"transformation_key":"<escaped>",` prefix cannot drift between writer and remover. Validates required fields, rejects reserved and malformed extras keys, and pins JSON escaping via `serde_json`.
- `lock.rs` — a `mkdir`-based advisory lock (POSIX exclusive-acquisition mutex) with an `owner` PID sentinel, single-winner reclaim of dead holders, and jittered exponential back-off to an injectable ceiling. A missing, empty, or unparseable owner is treated as *live* so PID reuse and the acquisition window can never break a genuinely held lock. Liveness uses `rustix::process::test_kill_process` (safe `kill(pid, 0)` wrapper) — the crate carries no `unsafe`.

**Dependencies** — adds `tempfile`, `libc`, `rand`, `rustix` (`process` feature), and `serde_json` as workspace deps; `corpus` carries `kernel` for the boundary error mapping.

## Context

- Implements work item **0180** — *Atomic-Store Primitives in corpus-adapters* (`meta/work/0180-atomic-store-primitives-corpus-adapters.md`), external `PP-704`, under epic `0166`.
- Blocked by **0179** (corpus crate scaffolding — this PR's base branch).
- Relates to **0172** (migration engine subdomain) — carries a reciprocal clean-cutover obligation on the JSONL escaper.
- Plan: `meta/plans/2026-07-19-0180-atomic-store-primitives-corpus-adapters.md`; research, work/plan reviews, and the execution validation are all included in the diff under `meta/`.

## Testing

- [x] `cargo test -p corpus -p corpus-adapters` — 130 passed (8 suites)
- [x] Adversarial transformation keys (backslash, quote, tab, `\x7f`) round-trip append→remove to an empty file
- [x] Anchored remove prefix does not over-match (`foo` vs `foobar`)
- [x] Interruption before rename leaves existing content intact / a fresh path absent, with no stray temp
- [x] Lock reclaim covered across dead / live / missing / empty / unparseable owner, plus the timeout ceiling and non-`AlreadyExists` fail-fast
- [ ] Full local CI mirror (`mise run check`) not run in this pass — Rust workspace check + clippy recommended before merge

## Notes for Reviewers

- **Base branch is `0179`, not `main`** — review the stack in order.
- The `record_opener` sharing between composer and remover is the key invariant: the "opener drift" and "anchored prefix does not over-match" tests guard it — worth confirming they cover the escaping edge cases you care about.
- Lock semantics are POSIX-only by design (matches the bash source and the darwin + musl target set). The dead-owner reclaim is deliberately conservative — every ambiguous owner state resolves to "live".
- `atomic_write` is `pub(crate)`; the public surface is `FileCorpusStore` via the `AtomicWrite`/`RecordStore` ports.
