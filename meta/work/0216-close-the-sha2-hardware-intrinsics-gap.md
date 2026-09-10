---
type: "work-item"
id: "0216"
title: "Close the sha2 hardware-intrinsics gap"
date: "2026-08-17T20:36:49+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "ready"
kind: "task"
priority: "low"
parent: "work-item:0136"
derived_from: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement"]
relates_to: ["work-item:0189", "work-item:0205", "work-item:0215", "work-item:0191", "work-item:0217"]
tags: ["cli", "launcher", "performance"]
last_updated: "2026-09-10T17:59:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-745"
---

# 0216: Close the sha2 hardware-intrinsics gap

**Kind**: Task
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

Close the ~3.1x sha256 shortfall in the launcher's verifier by enabling the
ARMv8 SHA-2 hardware backend that the `sha2` crate is currently not compiling
in. Static inspection already localises the cause; the work is to switch the
backend on, verify it across all four shipped targets, and measure the gain.

## Context

Work item 0205 measured `verifier::sha256_hex` at **555 MB/s** against `openssl
sha256` at **1,708 MB/s** over the identical file on the same chip — a 3.1x
shortfall affecting **every** sha256 the Rust binaries compute, not only the
launcher's cache-hit check.

0189's closing session corroborates the rate: 4.5 ms over 2.49 MB is ~550 MB/s.

⚠️ **BLAKE2b, which has no hardware path at all, outruns it 2.6x** — 1.7184 ms
against 4.4895 ms over 2,493,792 bytes. That inverts the assumption the
optimisation discussion was conducted under, and it means minisign's own digest
(BLAKE2b) is already the faster of the two.

## Requirements

- Enable the ARMv8 SHA-2 hardware backend for `verifier::sha256_hex`, choosing
  between the `sha2` crate's `asm` feature on the pinned 0.10 line and a bump of
  the workspace `sha2` dependency to 0.11 (whose `aarch64-sha2` backend is
  runtime-detected and needs no features or RUSTFLAGS).
- Verify the change across all four shipped targets, including the musl statics.
  On `aarch64-unknown-linux-musl` the generic baseline lacks `+sha2`, so confirm
  the chosen path does not depend on forcing `-C target-feature=+sha2`, which
  would fault on CPUs without the extension.
- Hold `cargo-deny` clean and keep the release artefacts reproducible. The
  lockfile already carries both `sha2` 0.10.9 and 0.11.0; record what the chosen
  remedy does to that duplication.
- Record measured throughput before and after on darwin-arm64 using the
  committed term harness (`cli/launcher/tests/warm_terms.rs`), which already
  reports `verifier::sha256_hex` separately.

## Acceptance Criteria

- [ ] Given the launcher verifier on darwin-arm64, when `verifier::sha256_hex`
      runs over the 2.49 MB `vcs` sub-binary that set the baseline, then measured
      throughput is at least 2.5× the ~555 MB/s soft-backend baseline
      (~1,390 MB/s). This floor is the single binding gate; the ~2,000+ MB/s the
      hardware path is expected to reach (Technical Notes) is the expectation,
      not the gate, and a result still in the ~500 MB/s band means the soft
      backend is still selected.
- [ ] The chosen remedy builds cleanly — `cargo build --release` exits 0 with no
      warnings beyond those an unchanged build of the same target already emits —
      across all four shipped targets: `aarch64-apple-darwin`,
      `x86_64-apple-darwin`, `aarch64-unknown-linux-musl`, and
      `x86_64-unknown-linux-musl`.
- [ ] On `aarch64-unknown-linux-musl` the build succeeds and the config selects
      the runtime-detected backend, evidenced by inspection: no
      `-C target-feature=+sha2` is set for the musl targets and the workspace
      depends on `sha2` 0.11 with its runtime-detected `aarch64-sha2` backend.
      Execution on an aarch64 CPU lacking the sha2 extension is not directly
      tested here — it relies on the upstream-tested 0.11 runtime-detection path
      — which is recorded as an accepted limitation. Linux/musl throughput is
      measured under 0217.
- [ ] `cargo-deny` passes and the release artefacts remain reproducible — two
      clean rebuilds of each artefact produce byte-identical outputs — with the
      effect on the 0.10.9/0.11.0 lockfile duplication recorded.
- [ ] Before/after throughput figures are recorded on the work item or its
      implementing change.
- [ ] Hardware enablement is the committed outcome. Only if the hardware path
      genuinely cannot be made clean on the musl statics does the
      accepted-fallback completion state apply instead: the blocker is recorded
      with evidence, 0215 is named as the chosen route, and the decision to
      accept the gap is captured on this item.

## Open Questions

- `asm` on the pinned 0.10 line versus bumping the workspace `sha2` to 0.11 —
  which does the team prefer? 0.11 is favoured here: it is already resolved in
  the lockfile, is runtime-detected, and is musl-safe without global target
  flags.
- If enabling the hardware path can't be made clean across the musl targets, the
  fallback is to accept the gap and pursue 0215 instead — removing the cache-hit
  sha256 and relying on minisign's BLAKE2b for corruption detection, with the
  asset's name/version binding preserved by the cheaper means 0215 defines (a
  post-exec version check), since minisign signs bytes only and does not bind the
  name or version. **Resolved**: hardware enablement is the committed
  deliverable; this fallback fires only if the musl statics genuinely cannot be
  made clean, not as an optional alternative.

## Dependencies

- **Gates** 0215 (removes the cache-hit sha256 from warm dispatch). This task
  ships and is measured first; its result decides whether 0215 still proceeds —
  pursued on architectural grounds to cut a hash off the warm path — or is
  dropped as unnecessary now the digest is cheap. Both touch the same
  `reverify`/`verifier::sha256_hex` call site, so they must not be scheduled in
  parallel.
- **Prerequisite**: the committed term harness
  `cli/launcher/tests/warm_terms.rs` must be present and report
  `verifier::sha256_hex` separately. It is the committed successor to 0205's
  throwaway `spike_0205_warm_terms.rs` and one of 0189's named hand-offs; confirm
  it exists before relying on it for the before/after measurement.
- **Relates to** 0189 (measured the rate), 0205 (first measured it and recorded
  the BLAKE2b inversion), and 0191 (batches the two shim hashes; names this item
  as a sibling lever on warm-dispatch cost).
- **Hand-off to** 0217, which measures warm dispatch on linux, where the musl
  intrinsics question is sharpest. The linux/musl throughput evidence for this
  backend change lands in 0217, sequenced after this task ships; 0216's own
  measurement is scoped to darwin-arm64.
- **Parent**: epic 0136.

## Assumptions

- The shortfall is a build/feature-selection property, not a measurement
  artefact — now corroborated by static inspection: the workspace pins
  `sha2 = "0.10"` with no `asm` feature and `sha2-asm` is absent from the
  lockfile, so the portable `soft` backend is what compiles. The before/after
  benchmark is verification, not discovery.

## Technical Notes

- On `aarch64-apple-darwin`, `+sha2` is baseline (default `target-cpu=apple-m1`
  since Rust 1.71), so a compile-time-gated intrinsics path would already be
  active. The gap is therefore a missing crate backend, not a missing target
  feature.
- First step: `cargo tree -p sha2` to confirm the version, then benchmark
  `verifier::sha256_hex` with and without `features = ["asm"]`. Expect the
  hardware path around 2,000+ MB/s (RustCrypto's M2 datapoint is 365 → 2,386
  MB/s), overshooting openssl's 1,708 MB/s — this is the expected result, not the
  pass gate, which is AC 1's ≥2.5×-baseline floor; a result still in the
  ~500 MB/s band means the soft backend is still selected.
- Remedy (i): `sha2 = { version = "0.10", features = ["asm"] }`. Remedy (ii):
  bump the workspace to `sha2` 0.11 — its MSRV (minimum supported Rust version)
  of 1.85 is under the pinned 1.90, and 0.11.0 is already in the lockfile.
- Do not force `-C target-feature=+sha2` for the `*-unknown-linux-musl` static
  targets; it faults on aarch64 CPUs lacking the extension. Prefer 0.11 runtime
  detection, which selects the backend by querying CPU features at startup via
  getauxval/HWCAP (the libc auxiliary-vector feature query) and works under
  `crt-static`.
- A crate swap or a vendored assembly path remain only as fallbacks if the
  hardware backend still trails openssl after enabling — the known residual is
  intrinsics scheduling versus openssl's interleaved multi-block assembly
  (~10–30%), not the 3x.

## Drafting Notes

- Converted from spike to task per the review: the deliverable is now enabling
  the hardware backend, not investigating and recommending. "Accept the gap +
  BLAKE2b" survives only as a fallback exit condition, not a co-equal option.
- Narrowed the candidate remedies to the `asm` feature, a 0.11 upgrade, and the
  accept-gap fallback, from research plus lockfile inspection; the crate swap
  and vendored assembly are demoted to fallbacks.
- Success bar set as a floor at 2.5× the ~555 MB/s soft baseline (~1,390 MB/s),
  the single binding gate (AC 1). The earlier openssl-parity (~1,700 MB/s)
  framing was lowered to a floor that stays valid even if intrinsics scheduling
  leaves the crate 10–30% behind openssl; the ~2,000+ MB/s hardware figure is the
  expected result, not the gate.
- Added a musl verification criterion because the intrinsics baseline differs
  there.

## References

- `meta/work/0205-close-the-warm-dispatch-measurement-method.md` — the
  555 MB/s against 1,708 MB/s comparison and the BLAKE2b figures
- `meta/work/0215-remove-the-cache-hit-sha256-from-warm-dispatch.md` — the
  cache-hit sha256 removal this task may make unnecessary
- `meta/measurements/warm-dispatch-3.json` — 0189's closing-session term set,
  the source of the corroborating ~550 MB/s figure cited in Context
- `cli/launcher/src/launch/outbound/resolve/verifier.rs` — `sha256_hex`
- `cli/launcher/tests/warm_terms.rs` — the committed term harness, which already
  reports `verifier::sha256_hex` and `TrustedKeys::verifies` separately
- RustCrypto `hashes` PR #490 (aarch64 backend, soft vs asm MB/s) and the
  `sha2` 0.11.0 CHANGELOG (runtime-detected `aarch64-sha2`, `asm` feature
  removed)
