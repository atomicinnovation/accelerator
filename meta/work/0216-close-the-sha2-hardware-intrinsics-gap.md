---
type: work-item
id: "0216"
title: "Close the sha2 hardware-intrinsics gap"
date: "2026-08-17T20:36:49+00:00"
author: Toby Clemson
producer: implement-plan
status: draft
kind: spike
priority: low
parent: "work-item:0136"
derived_from: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement"]
relates_to: ["work-item:0189", "work-item:0205"]
tags: [cli, launcher, performance]
last_updated: "2026-08-17T20:36:49+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-745
---

# 0216: Close the sha2 hardware-intrinsics gap

**Kind**: Spike
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Answer one question: **why does our sha256 run at roughly a third of the
hardware's rate, and which remedy is worth taking?** Recommend, with measured
before/after per candidate. Do not fix.

## Context

Work item 0205 measured `verifier::sha256_hex` at **555 MB/s** against `openssl
sha256` at **1,708 MB/s** over the identical file on the same chip — a 3.1x
shortfall affecting **every** sha256 the Rust binaries compute, not only the
launcher's cache-hit check.

0189's closing session corroborates the rate: 4.5 ms over 2.49 MB is ~550 MB/s.

⚠️ **BLAKE2b, which has no hardware path at all, outruns it 2.6x** — 1.7184 ms
against 4.4895 ms over 2,493,792 bytes. That inverts the assumption the
optimisation discussion was conducted under, and it means minisign's own digest
is already the faster of the two.

## Requirements

- Establish why the gap exists: whether the `sha2` crate is taking a portable
  fallback rather than the ARMv8 SHA-2 extensions, and if so what selects it.
- Evaluate and cost each candidate remedy with a measured before/after:
  enabling the `sha2` crate's `asm`/intrinsics feature; a crate swap; a vendored
  assembly path; or accepting the gap and moving the corruption check to
  BLAKE2b, which minisign already computes.
- State the effect on the build: any feature or crate change must hold across
  all four shipped targets including the musl statics, and must not disturb
  `cargo-deny` or the reproducibility of the release artefacts.

## Acceptance Criteria

- [ ] The cause of the 3.1x shortfall is identified, or the attempt to identify
      it is recorded with what was ruled out.
- [ ] Each candidate carries a measured throughput before and after on
      darwin-arm64, and a stated build cost across all four targets.
- [ ] A recommendation is recorded, including the option of accepting the gap.
- [ ] Every throwaway artefact is positively asserted absent.

## Dependencies

- **Relates to** 0189 (measured the rate), 0205 (first measured it and recorded
  the BLAKE2b inversion), and the cache-hit-sha256 removal item, which this may
  make unnecessary by making the digest cheap instead of removing it.
- **Parent**: epic 0136.

## Assumptions

- The shortfall is a build/feature-selection property rather than a measurement
  artefact. Two independent sessions agree on the rate, so this is well
  supported, but the spike should confirm it before pursuing remedies.

## References

- `meta/work/0205-close-the-warm-dispatch-measurement-method.md` — the
  555 MB/s against 1,708 MB/s comparison and the BLAKE2b figures
- `cli/launcher/src/launch/outbound/resolve/verifier.rs` — `sha256_hex`
- `cli/launcher/tests/warm_terms.rs` — the committed term harness, which already
  reports `verifier::sha256_hex` and `TrustedKeys::verifies` separately
