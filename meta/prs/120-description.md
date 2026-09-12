---
type: "pr-description"
id: "120"
title: "[0216] Close the sha2 hardware-intrinsics gap"
date: "2026-09-12T23:45:44+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0216"
parent: "work-item:0216"
relates_to: ["work-item:0215", "work-item:0217"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/120"
pr_number: 120
tags: ["cli", "launcher", "performance"]
revision: "04cb0679d433b48ef764eccce7821af19441d7b4"
repository: "accelerator"
last_updated: "2026-09-12T23:45:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0216] Close the sha2 hardware-intrinsics gap

## Summary

Enables the ARMv8 SHA-2 hardware backend by bumping the workspace `sha2`
dependency to 0.11, whose runtime-detected `aarch64-sha2` backend needs no
features, no RUSTFLAGS, and is musl-safe under `crt-static`. On darwin-arm64 the
launcher's `verifier::sha256_hex` rises from 611 MB/s (soft) to 2,944 MB/s
(hardware) — a 4.8× lift, median 4.08 ms to 0.847 ms over the 2.49 MB `vcs`
sub-binary — and the crate-global switch reaches all seven first-party SHA-256
sites at once. Closes work item 0216.

## Changes

**sha2 0.11 backend (Phase 3).** Bump `sha2` to 0.11 in `cli/Cargo.toml`,
collapse the server's literal onto the workspace pin, and refresh the lockfile so
a single `sha2 0.11.0` resolves with no duplicate. 0.11's `hybrid-array` output
drops the `LowerHex` impl, so three `format!("{:x}", …)` digest sites move to
`hex::encode(…)` and `hex` is hoisted to `[workspace.dependencies]`. New
known-answer vectors (empty, `abc`, padding-boundary, multi-block) pin the
intrinsic path in `verifier.rs` directly.

**Measurement plumbing (Phase 1).** Thread the harness's trailing `asset_bytes`
line into the warm-dispatch record (`tasks/measure.py`, with a unit-tested pure
assembler and a committed `warm_terms` golden), and document the
`asset_bytes / (median_ms * 1000)` throughput derivation (`tasks/README.md`).

**cargo-deny cleanup (Phase 4).** Flip `[bans] multiple-versions` from `warn` to
`deny` and add 23 exact version-pinned `skip` entries for the transitive
major-version straddles (no `skip-tree`). Update the deny-graph test's rationale
to the `deny` posture.

**Evidence (Phases 2–3).** Commit the before/after throughput JSONs, the
per-target cross-build warning baseline, the pre-bump deny surface, and the musl
backend disassembly evidence (`sha256h`×32, `sha256h2`×32, `sha256su0`×24,
`sha256su1`×24) under `meta/measurements/`.

**Process docs.** Plan, codebase research, plan and work-item reviews, and the
validation report (result: pass); status and note updates to work items 0215,
0216 (now done), and 0217.

**Unrelated, included here.** `.claude/skills/clean-comments/SKILL.md` — the
clean-comments skill, not part of 0216 (see Notes for Reviewers).

## Context

- Work item: `meta/work/0216-close-the-sha2-hardware-intrinsics-gap.md`
- Plan: `meta/plans/2026-09-11-0216-close-the-sha2-hardware-intrinsics-gap.md`
- Validation: `meta/validations/2026-09-11-0216-close-the-sha2-hardware-intrinsics-gap-validation.md` (result: pass)
- Follow-on: 0217 (aarch64-musl throughput), 0215 (cache-hit sha256 fallback)

## Testing

Re-run first-hand against the branch tip:

- [x] `mise run cli:check` — exit 0, zero warnings across every crate
- [x] `mise run build:cli:cross-compile` — four clean release builds, zero warnings
- [x] `mise run deny:check` — advisories ok, bans ok, licenses ok, sources ok
- [x] `mise run test:integration:deny` — 111 passed
- [x] `mise run lint:vendor-shims:check` — marker unmoved
- [x] `uv run pytest tests/unit/tasks/test_measure.py -k asset_bytes` — 6 passed
- [x] `cargo tree -d` shows no `sha2` duplicate; `cargo tree -i sha2` shows one `sha2 v0.11.0`
- [x] `llvm-objdump -d` of the musl binary shows the ARMv8 SHA-256 intrinsics
- [ ] Full `mise run test` end-to-end — not re-run this session; recorded green in the plan (only the three pre-existing `test_vendor_assemble.py` parallel-load flakes)
- [ ] Conclusive aarch64-musl runtime throughput — out of 0216 scope, deferred to 0217

## Notes for Reviewers

- The source change is small and surgical: manifests, lockfile, three `hex::encode` sites, and test vectors. The bulk of the diff is committed evidence and process docs.
- Conclusive aarch64-musl runtime backend *selection* is deferred to 0217 by design — 0216's musl scope is inspection-only (intrinsics compiled in, no forced `-C target-feature=+sha2`, darwin `cpufeatures` reports `true`). The AC 7 fallback (0215) backstops a soft-band musl result.
- The `.claude/skills/clean-comments/SKILL.md` commit is unrelated to 0216 — interleaved in the branch and included here at the author's request. Say the word and it can be split into its own PR.
- The Phase 4 `deny` flip is a standing coupling: any future version-straddling crate must add a justified `skip` or fail `deny:check`.
