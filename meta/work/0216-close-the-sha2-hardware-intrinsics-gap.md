---
type: "work-item"
id: "0216"
title: "Close the sha2 hardware-intrinsics gap"
date: "2026-08-17T20:36:49+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "done"
kind: "task"
priority: "medium"
parent: "work-item:0136"
derived_from: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement"]
relates_to: ["work-item:0189", "work-item:0205", "work-item:0215", "work-item:0191", "work-item:0217"]
tags: ["cli", "launcher", "performance"]
last_updated: "2026-09-11T10:06:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-745"
---

# 0216: Close the sha2 hardware-intrinsics gap

**Kind**: Task
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Close the ~3.1x sha256 shortfall by enabling the ARMv8 SHA-2 hardware backend the
`sha2` crate is currently not compiling in. The switch is crate-global — a
workspace `sha2` 0.11 bump reaches every first-party consumer (the launcher
verifier, the visualiser server, and four other crates), not only the launcher's
cache-hit check. Static inspection already localises the cause; the work is to
switch the backend on, verify it across all four shipped targets, and measure the
darwin-arm64 gain. The switch also carries a workspace-wide cargo-deny
no-duplicates cleanup — flipping `[bans] multiple-versions` to `deny` and giving
each duplicate in the enumerated set (see Requirements) a resolve-or-justify
disposition — for which the `sha2` collapse is the first step.

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

- Enable the ARMv8 SHA-2 hardware backend for `verifier::sha256_hex` by bumping
  the workspace `sha2` dependency to 0.11, whose runtime-detected `aarch64-sha2`
  backend needs no features or RUSTFLAGS and is musl-safe without global target
  flags. Move **both** the `[workspace.dependencies]` pin (`cli/Cargo.toml`) and
  the server's independent `sha2 = "0.10"` literal
  (`cli/visualiser/server/Cargo.toml`), converting the latter to
  `sha2 = { workspace = true }` so backend selection has a single source of truth.
  The `asm` feature on the pinned 0.10 line is the rejected alternative (see Open
  Questions).
- The bump is crate-global: the workspace pin is inherited by `launcher`,
  `work-adapters`, `jira-client`, `design-adapters` and `remote-projection` via
  `sha2 = { workspace = true }`, and by the server once its literal is converted.
  All are re-verified by the workspace build; name them so the shared-artefact
  blast radius is captured, not implied.
- The 0.11 bump is not manifest-only. It is a major RustCrypto version change
  that moves the digest output type from `generic-array` to `hybrid-array`,
  which drops the `LowerHex` impl, so three test-crate
  `format!("{:x}", …)` sites (`work-adapters`, `jira-client`) must migrate to
  `hex::encode`, and `hex` must become a `[workspace.dependencies]` entry
  referenced by the server and those two crates rather than the server's current
  independent literal. Audit all six consumers (`cargo check --all-targets -p
  <crate>` for launcher, work-adapters, jira-client, design-adapters,
  remote-projection, server) to confirm these are the only source changes; the
  other digest sites (`.into()`, `hex::encode`, byte-iteration) are drop-in.
- Verify the change across all four shipped targets, including the musl statics.
  There is no `.cargo/config.toml` or RUSTFLAGS in the tree, so confirm by
  inspection that nothing forces `-C target-feature=+sha2` — which would fault on
  aarch64 CPUs without the extension — and that the runtime-detected backend is
  what compiles.
- Hold `cargo-deny` clean and, as part of this work, switch `[bans]
  multiple-versions` to `deny` in `cli/deny.toml`, then give every duplicate in
  the enumerated set below a disposition — collapse it, or record a justified
  `skip`/`skip-tree` — and land the policy clean. The set is the current
  `cargo tree -d` inventory (18 straddled crates): `sha2` (collapsed by this
  bump), `digest`, `crypto-common`, `block-buffer`, `cpufeatures`, `getrandom`
  (three-way), `hashbrown` (three-way), `rand`, `rand_chacha`, `rand_core`,
  `itertools`, `syn`, `serde_spanned`, `toml_datetime`, `toml_edit`, `winnow`,
  `tower-http` and `untrusted`. Expect most to resolve to justified skips — they
  are transitive major-version straddles (the rand/getrandom ecosystem, ring's
  `untrusted`, the toml/winnow and syn 2/3 splits) that no first-party change can
  collapse — while `sha2` is the one the bump actually removes. Record that the
  0.11 bump collapses first-party `sha2` onto the single 0.11.0 the lockfile
  already carries via transitive `rust-embed-utils` (`rust-embed`'s own
  dependency); a future `rust-embed` move off 0.11.0 would reintroduce the
  duplicate under the stricter policy. That policy is itself an ongoing downstream
  coupling: any later work adding a version-straddling crate must add a justified
  skip or it fails the cargo-deny gate.
- Keep the reproducibility-relevant artefacts intact. The cross-compiled binaries
  are not byte-reproducible by design (`RELEASING.md`), so verify instead that
  `lint:vendor-shims:check` passes unchanged and that the assembled-archive
  `pins.toml` digests are unaffected — not by a byte-identical binary rebuild. The
  vendor shims are the committed cross-compiled `accelerator-verify` binaries in
  `bin/`; their drift guard compares `vendor_shim_marker_digest` — a hash over
  `cli/verify`'s build inputs and lockfile closure, recorded in
  `bin/accelerator-verify.vendored.sha256`. `cli/verify` depends on
  `minisign-verify`, not `sha2`, so the marker should not move; if the lockfile
  change trips it, regenerate the shims (`mise run build:vendor-verify-shims`) and
  commit them.
- Record measured throughput before and after on darwin-arm64 using the committed
  term harness (`cli/launcher/tests/warm_terms.rs`), which reports
  `verifier::sha256_hex` separately. Teach `tasks/measure.py` to persist the
  `asset_bytes` line the harness already emits into the measurement JSON, and
  document the `asset_bytes ÷ median_ms` derivation so the throughput figure is a
  recorded quantity, not a hand-division.
- Order the work so the "before" state stays recoverable: (1) teach
  `tasks/measure.py` to persist `asset_bytes`, (2) capture the pre-bump warning and
  throughput baselines on the soft 0.10 backend, then (3) apply the 0.11 bump and
  capture the "after" figures. The `measure.py` persistence change is also a
  prerequisite 0217 inherits for its aarch64-musl throughput figure.

## Acceptance Criteria

- [ ] Given the launcher verifier on darwin-arm64, when `verifier::sha256_hex`
      runs over the 2,493,792-byte `vcs` sub-binary that set the baseline, then its
      median is **≤ 1.79 ms** (≥1,390 MB/s = 2.5× the ~555 MB/s portable
      soft-backend baseline). This fixed threshold is the single binding gate; the
      freshly-captured pre-bump baseline is recorded as context, not as a re-scaling
      of the floor. The ~2,000+ MB/s (~1.25 ms) the hardware path is expected to
      reach (Technical Notes) is the expectation, not the gate, and a result still
      near ~4.3 ms (~575 MB/s) means the soft backend is still selected.
- [ ] The chosen remedy builds cleanly across all four shipped targets
      (`aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-musl`,
      `x86_64-unknown-linux-musl`) via the project's cross-build flow
      (`cargo zigbuild --release --target <triple>`, as run by the
      `mise run build:cli-cross-compile` task): each target exits 0, and its
      per-target warning set matches a pre-change baseline captured before the bump
      and recorded as evidence — no new warnings beyond that baseline.
      `mise run cli:check` also exits 0, covering the non-shipped consumer crates
      (`work-adapters`, `jira-client`, `design-adapters`, `remote-projection`) the
      shipped-target builds need not exercise. The clean build presupposes the
      0.11 `hybrid-array` output-type migration (Requirements) — the three `{:x}`
      test-crate sites moved to `hex::encode` with `hex` hoisted to a workspace
      dependency — since without it `cli:check`/`test` fail to compile.
- [ ] On `aarch64-unknown-linux-musl` the build succeeds (via the AC 2 zigbuild
      flow) and the runtime-detected backend is selected. Evidence, in two parts:
      (a) that the hardware backend is *available* — by inspection (no
      `-C target-feature=+sha2` forced; the workspace on `sha2` 0.11), corroborated
      by `nm`/`strings` for the `aarch64-sha2` backend symbols over an unstripped
      (`strip = false`) build or the `sha2` `.rlib`, not the stripped release
      binary; and (b) that it is *selected at runtime* — on darwin, a one-off
      `cpufeatures`-based print, corroborating only: darwin resolves the feature
      via `sysctlbyname`, not the Linux getauxval/HWCAP path the `crt-static` musl
      static uses, and `cpufeatures` is the selector `sha2` 0.11 ships, unlike
      `std::arch`. Conclusive aarch64-musl runtime selection is deferred to 0217
      and bounded here by AC 7's soft-backend-selection trigger. Execution on an
      aarch64 CPU lacking the sha2 extension is not directly tested here — it
      relies on the upstream-tested 0.11 runtime-detection path, an accepted
      limitation. The
      aarch64-musl throughput is measured under 0217; confirm 0217 carries the
      reciprocal `work-item:0216` edge and its SHA-extension-keyed
      `verifier::sha256_hex` criterion before closing 0216 on inspection-only musl
      evidence.
- [ ] The reproducibility-relevant artefacts are intact: the cross-compiled
      binaries are not byte-reproducible by design, so this is evidenced by
      `lint:vendor-shims:check` passing unchanged (or the shims regenerated and
      committed) and the assembled-archive `pins.toml` digests being unaffected —
      not by a byte-identical binary rebuild. The 0.11 bump's effect on the `sha2`
      duplication (the current 0.10 pin resolves to 0.10.9, alongside the transitive
      0.11.0) is recorded.
- [ ] The single cargo-deny end state: `cli/deny.toml` sets `[bans]
      multiple-versions = "deny"`, every duplicate in the enumerated set
      (Requirements) is resolved or carries a `skip`/`skip-tree` entry, and the
      cargo-deny check exits 0 under that stricter config. The exit-0 check is the
      binding gate; a skip's justification must record that the duplicate is a
      transitive major-version straddle no first-party change can collapse (the
      reasoning already given in Requirements). This is the only cargo-deny gate —
      there is no separate check under the current lenient config. Independently of
      the fallback-waivable AC 3, `cli/visualiser/server/Cargo.toml` declares
      `sha2 = { workspace = true }` (not an independent literal), so the
      single-source-of-truth conversion is bound by a criterion that always applies.
      The new 0.11 RustCrypto substack's advisories/licenses/sources surface is
      also re-vetted: a pre-bump `cargo deny check advisories licenses sources`
      baseline is committed and diffed after the bump, with no new advisory and no
      license outside the pruned `[licenses].allow`, and `deny:check` exits 0 for
      advisories/licenses/sources (which run regardless of the bans posture, so
      the vet gates the phase that introduces the substack). Narrow `skip-tree`
      masks — depth-bounded and version-pinned rather than unbounded — preserve
      the gate against future first-party duplicates.
- [ ] Before/after `verifier::sha256_hex` throughput on darwin-arm64 is recorded
      on the work item or its implementing change, derived from a persisted
      `asset_bytes` — `tasks/measure.py` captures the `asset_bytes` line into the
      measurement JSON, so the figure is `asset_bytes ÷ median_ms`, not a hand
      measurement.
- [ ] Hardware enablement is the committed outcome. The accepted-fallback state
      applies only on a named, reproducible build or link failure on
      `aarch64-unknown-linux-musl` that resists the 0.11 runtime-detection path —
      "genuinely cannot be made clean" means the default 0.11 runtime detection
      under `crt-static`, with no forced `-C target-feature=+sha2`, is shown to fail
      reproducibly; a recorded failure of that specific approach, not mere
      difficulty. The fallback also fires on the more likely musl failure mode:
      runtime detection selecting the soft backend (`cpufeatures` returns `false`)
      under the static musl binary, which builds and links clean and so does not
      surface in a build/link check. That trigger cannot be closed within 0216's
      inspection-only musl scope and stays open pending 0217's aarch64-musl
      throughput — a result in the ~555 MB/s soft band there fires this fallback.
      In that case the blocker is recorded with evidence, 0215 is named
      as the chosen route, and the decision to accept the gap is captured here.
      Binding status of the other criteria in the fallback: the darwin-arm64 path
      still lands (0.11 is adopted; only the musl static resists), so AC 1, AC 2
      (its darwin targets), AC 4 and AC 6 remain binding; AC 3's musl backend and
      throughput confirmation is waived for the failing target and recorded as the
      blocker; AC 5's cargo-deny cleanup remains binding, since it does not depend
      on the hardware path landing.

## Open Questions

- **Resolved**: bump the workspace `sha2` to 0.11 rather than enabling the `asm`
  feature on the pinned 0.10 line. 0.11 is already resolved in the lockfile, is
  runtime-detected, and is musl-safe without global target flags; the `asm` route
  would pull `sha2-asm` and needs care on the musl statics. Both the workspace pin
  and the server's independent literal move to 0.11.
- If enabling the hardware path can't be made clean across the musl targets, the
  fallback is to accept the gap and pursue 0215 instead — removing the cache-hit
  sha256 and relying on minisign's BLAKE2b for corruption detection, with the
  asset's name/version binding preserved by the cheaper means 0215 defines (a
  post-exec version check), since minisign signs bytes only and does not bind the
  name or version. **Resolved**: hardware enablement is the committed
  deliverable; this fallback fires only if the musl statics genuinely cannot be
  made clean, not as an optional alternative.

## Dependencies

- **Gates** 0215 (removes the cache-hit sha256 from warm dispatch). The full
  conditional in one place: if 0216 succeeds, 0215's result decides whether it
  still proceeds — pursued on architectural grounds to cut a hash off the warm
  path — or is dropped as unnecessary now the digest is cheap; if 0216 hits its
  AC 7 build-failure fallback, 0215 becomes the chosen remedy instead. This task
  ships and is measured first. Both touch the same
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
  measurement is scoped to darwin-arm64. This hand-off is an obligation, not an
  assumption: confirm 0217 carries the reciprocal `work-item:0216` edge, its
  SHA-extension-keyed `verifier::sha256_hex` criterion, and the inherited
  `measure.py` `asset_bytes` prerequisite before closing 0216 on inspection-only
  musl evidence. All were added to 0217 alongside this task; the confirmation
  guards against their being reverted.
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
- Diagnostic (not a gate): benchmark `verifier::sha256_hex` on the soft (0.10)
  backend versus the 0.11 runtime-detected `aarch64-sha2` backend. Expect the
  hardware path around 2,000+ MB/s (RustCrypto's M2 datapoint is 365 → 2,386
  MB/s), overshooting openssl's 1,708 MB/s — the expected result, not the pass
  gate, which is AC 1's ≥2.5×-baseline floor; a result still in the ~500 MB/s band
  means the soft backend is still selected. Do not benchmark a `features = ["asm"]`
  build: `asm` is removed in `sha2` 0.11, the chosen version.
- The committed remedy is the `sha2` 0.11 bump — its MSRV (minimum supported Rust
  version) of 1.85 is under the pinned 1.90, and 0.11.0 is already in the lockfile.
  The `asm` feature (removed in 0.11), a crate swap, and a vendored assembly path
  are rejected alternatives (see Open Questions), not fallbacks: the 0.11 hardware
  path trailing openssl by ~10–30% — intrinsics scheduling versus openssl's
  interleaved multi-block assembly — is an accepted residual, not a trigger, since
  AC 1 gates on the ≥2.5×-baseline floor, not openssl parity. The one and only
  fallback is the AC 7 build-failure exit (musl statics genuinely cannot be made
  clean → accept the gap via 0215).
- Do not force `-C target-feature=+sha2` for the `*-unknown-linux-musl` static
  targets; it faults on aarch64 CPUs lacking the extension. 0.11 runtime detection
  selects the backend by querying CPU features at startup via getauxval/HWCAP (the
  libc auxiliary-vector feature query) and works under `crt-static`.

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
- Priority raised to `medium` to match 0215, the item it gates: leaving it `low`
  let a scheduler sorting by priority pull the medium-priority 0215 ahead of its own
  blocker, defeating the no-parallel gate ordering.
- Scope: the workspace-wide cargo-deny no-duplicates cleanup is deliberately kept in
  this item rather than extracted, bounded to the enumerated 18-crate set. This is a
  recorded decision — the cleanup is orthogonal to the backend switch (only `sha2` is
  collapsed by the bump) and could stand alone, but it is retained here as a
  delivery-sequencing convenience, with AC 5 and AC 7 carving it out as separable.
- Reconciled with the plan review (2026-09-11): the 0.11 bump is a major
  RustCrypto version change, not manifest-only — the `hybrid-array` output type
  drops `LowerHex`, so three `{:x}` test-crate sites migrate to `hex::encode` and
  `hex` is hoisted to a workspace dependency (Requirements, AC 2). AC 5 gains the
  advisories/licenses/sources re-vet of the new substack; AC 7 gains the silent
  soft-backend-selection trigger; AC 3(b)'s darwin print is corrected to
  corroborating-only, with conclusive musl selection deferred to 0217.

## References

- `meta/research/codebase/2026-09-10-0216-close-the-sha2-hardware-intrinsics-gap.md`
  — the codebase research backing these requirements and the open-question
  resolutions
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
