---
type: work-item
id: "0180"
title: "Atomic-Store Primitives in corpus-adapters"
date: "2026-07-06T22:27:35+00:00"
author: Toby Clemson
producer: refine-work-item
status: draft
kind: task
priority: high
parent: "work-item:0166"
external_id: PP-704
tags: [rust, config, corpus, store, crates, dedup]
last_updated: "2026-07-06T23:08:57+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0180: Atomic-Store Primitives in corpus-adapters

**Kind**: Task
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Port the load-bearing atomic-store concurrency primitives — atomic file writes,
the mkdir-based lock with PID-owner reclaim and jittered back-off, and
canonical-order JSONL compose/remove — from the bash library into
`corpus-adapters`.

## Context

Child of 0166 — Shared config, corpus, and store Crates. 0166 folds the
atomic-store capability into `corpus-adapters` rather than a standalone `store`
crate (a later split stays open if a second consumer needs it independently).
These primitives are subtle: the mkdir lock exists because `flock` is not POSIX
and is absent on macOS, and the JSONL remover relies on the writer emitting a
well-defined canonical field order. They must be ported with the concurrency
semantics reasoned through for a Rust context.

## Requirements

- `atomic_write`: same-directory temp file + atomic rename, cleaning up the temp
  on interruption before the rename.
- The mkdir-based lock: `mkdir` as the exclusive-acquisition mutex, PID-owner
  reclaim of a dead holder, jittered exponential back-off, and the 300 s
  acquisition ceiling. Reason through the bash `$BASHPID`-absent fallback for
  Rust (moot there, but PID-reuse safety and the ceiling still apply).
- Canonical-order JSONL compose/remove: `transformation_key` first (so the
  remover's anchored-prefix match is well-defined), `schema_version` second,
  then the remaining fields in canonical order; the writer and remover must
  agree byte-for-byte on the escape rules.

## Acceptance Criteria

- [ ] `corpus-adapters` provides atomic writes (temp + rename) and the
      mkdir-based lock with PID-owner reclaim and jittered back-off, plus
      canonical-order JSONL compose/remove — at parity with `atomic-common.sh` /
      `jsonl-common.sh`.
- [ ] A test covers contended-lock reclaim of a dead holder (the lock is
      reclaimed when the owner PID is gone, and never broken while a live owner
      holds it).
- [ ] JSONL compose and remove agree on canonical field order and escape rules,
      verified by a round-trip test (compose then remove-by-key).

## Dependencies

- Blocked by: 0179 (the `corpus-adapters` crate these primitives live in).
- Parent: 0166.

## Assumptions

- Folding the atomic-store into `corpus-adapters` (vs. a standalone `store`
  crate) is acceptable for now; the capability is still fully delivered.

## Technical Notes

**Size**: M — a faithful port of ~230 lines of subtle concurrency bash (three
primitives: atomic write, mkdir-lock with PID reclaim, canonical JSONL) behind a
driven-port trait with faked-port tests. No twin extraction and no greenfield
UI, so smaller than 0179; the concurrency subtlety (same-dir-temp atomicity, the
serde_json escaper parity trap, PID-reclaim) keeps it at M rather than S.

- Source bash: `scripts/atomic-common.sh:105-247` — `atomic_write` (`:16-32`),
  `_atomic_lock_acquire` (`:105-143`, RANDOM re-seed + jittered back-off +
  300 s ceiling), `_atomic_lock_reclaim_if_stale` (`:157-163`, `kill -0` owner
  check), the subshell-owned EXIT trap for release (`:200-209`, `:233-246`),
  and the anchored-prefix `index($0,p)!=1` remove (`:243-244`, prefix passed via
  `ENVIRON`).
- Source bash: `scripts/jsonl-common.sh:66-149` — `jsonl_json_escape`
  (backslash-first escape ordering) and `jsonl_compose_record` (canonical field
  order, `transformation_key` first, `schema_version` second).
- Same-directory temp invariant (the easiest thing to break): `mktemp` creates
  the temp *in the target's dir* (`atomic-common.sh:26`) because `rename(2)` is
  atomic only within one filesystem. In Rust use `NamedTempFile::new_in(parent)`
  — the default `new()` uses `$TMPDIR` and silently degrades `mv` to a
  non-atomic cross-mount copy.
- `serde_json` escaper mismatch risk: `serde_json`'s string serialiser is not
  guaranteed byte-identical to the hand-rolled `jsonl_json_escape`
  (`jsonl-common.sh:21-50`, backslash-first ordering, control chars as
  `\u00XX`). Records written one way and prefix-matched another will silently
  fail to match — port one escaper and drive **both** compose and remove-by-key
  through it. This is the single most important parity requirement.
- Anchored-prefix, not substring: `atomic_jsonl_remove_by_key` drops lines where
  the `{"transformation_key":"…",` prefix starts at byte 0
  (`atomic-common.sh:243-244`, awk `index($0,p)!=1`). Rust: retain where
  `!line.starts_with(&prefix)`.
- Back-off shape to preserve exactly: writability pre-check early-error
  (`:110-115`); base 4 ms doubling to a ~200–256 ms cap; jitter uniform
  `1..=base`; 300 s acquisition ceiling (`:129`). `rm -rf` (not `rmdir`) on
  release/reclaim because the lockdir carries the `owner` sentinel (`:147,:162`).
- PID-reclaim window: `_atomic_lock_reclaim_if_stale` (`:157-163`) treats a
  missing/empty `owner` file as **live** (holder mid-acquisition, between the
  `mkdir` and the owner write) — preserve that window. `kill -0` →
  `libc::kill(pid,0)`; PID-reuse only degrades to spin-wait, never breaks a live
  lock.
- Port-trait design (Model 1): define an `AtomicStore`/`RecordStore` driven-port
  trait in the `corpus` domain (mirroring `launch/core.rs:173-189`
  `ResolveBinary`/`ExecBinary`), implement it in `corpus-adapters`, and map a
  rich `StoreError` (lock-timeout, not-writable, io) into `kernel::Error::Failed`
  via `From` (precedent `launch/core.rs:167-171`; kernel taxonomy
  `kernel/src/lib.rs:9-15`). Faked-port isolation tests mirror
  `FixedResolver`/`RecordingExec` (ADR-0053).
- Dependency decision: no `tempfile`/`fs2`/`fd-lock` in the workspace today
  (`rand` 0.9 + `libc` present only transitively via rustls/ring). Prefer
  `std::fs::create_dir` for the mkdir-lock + `libc::kill` for liveness — closest
  parity, minimal dep surface. `fd-lock`/`fs2` would be a semantic deviation
  (they reintroduce the `flock` the bash deliberately avoids and drop
  PID-reclaim). New direct deps are subject to the cargo-deny ban-lists
  0162/0166 activate.
- Parity surface: `atomic_write` has many callers (config-common, the migration
  engine, `work-item-sync-*`, and the `jira_atomic_write_json` /
  `linear_atomic_write_json` validate-then-write wrappers). The JSONL primitives
  have one production caller (the migrate session log, `interactive-lib.sh`), but
  `jsonl_compose_record`'s canonical field order is the JSONL contract the
  visualiser reads. Existing Rust atomic-write prior art:
  `server/src/file_driver.rs`.
- Bash-isms that fall away (do not port literally): the `$BASHPID`-absent
  spin-only fallback (`std::process::id()` always exists), the RANDOM lockstep
  re-seed, the `ENVIRON`-vs-`awk -v` backslash hazard, and the EXIT-trap
  string-building (→ RAII drop guard, which also gives the subshell-trap
  isolation for free). Validation nuance: `proposed_value` is documented required
  (`jsonl-common.sh:60`) but omitted from the emptiness check (`:115-116`) — port
  to spec intent.

## References

- Parent: `meta/work/0166-shared-config-corpus-store-crates.md`
- ADRs: ADR-0045, ADR-0053
