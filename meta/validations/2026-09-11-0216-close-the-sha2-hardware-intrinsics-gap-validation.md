---
type: "plan-validation"
id: "2026-09-11-0216-close-the-sha2-hardware-intrinsics-gap-validation"
title: "Validation Report: Close the sha2 hardware-intrinsics gap Implementation Plan"
date: "2026-09-12T19:33:17+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-11-0216-close-the-sha2-hardware-intrinsics-gap"
tags: ["cli", "launcher", "performance"]
last_updated: "2026-09-12T19:33:17+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Close the sha2 hardware-intrinsics gap

All four phases are fully implemented and every automated gate re-run this
session exits 0. The `sha2` 0.11 bump resolves to a single `0.11.0`, the darwin
hardware backend is live (0.847 ms median, 2,944 MB/s — a 4.8× lift over the
soft baseline), the musl static carries the ARMv8 SHA-256 intrinsics, and
cargo-deny holds under `multiple-versions = "deny"`. Two minor traceability nits
and one plan-acknowledged deferred manual check are the only findings; none
blocks the verdict.

### Implementation Status

- ✓ Phase 1: Persist `asset_bytes` — Fully implemented
- ✓ Phase 2: Pre-bump (soft 0.10) baseline — Fully implemented
- ✓ Phase 3: `sha2` 0.11 bump, after-measurement, musl evidence — Fully implemented
- ✓ Phase 4: cargo-deny no-duplicates cleanup — Fully implemented

### Automated Verification Results

Re-run first-hand this session against the post-implementation tree:

- ✓ Workspace check: `mise run cli:check` — exit 0, zero warnings across every crate
- ✓ Cross-build: `mise run build:cli:cross-compile` — exit 0, four clean `Finished release` builds, zero warnings
- ✓ cargo-deny: `mise run deny:check` — `advisories ok, bans ok, licenses ok, sources ok`
- ✓ Deny suite: `mise run test:integration:deny` — 111 passed
- ✓ Vendor-shim guard: `mise run lint:vendor-shims:check` — exit 0, marker unmoved
- ✓ Phase 1 units: `uv run pytest tests/unit/tasks/test_measure.py -k asset_bytes` — 6 passed
- ✓ Single `sha2`: `cargo tree -d` lists no `sha2`; `cargo tree -i sha2` shows one `sha2 v0.11.0`
- ✓ musl intrinsics: `llvm-objdump -d` of the freshly built `aarch64-unknown-linux-musl` binary — `sha256h`×32, `sha256h2`×32, `sha256su0`×24, `sha256su1`×24

Not re-run this session: the full `mise run test` end-to-end. Its two most
change-relevant slices were run directly (the deny integration suite and the
`measure` units), and `cli:check` compiles every workspace crate and target
against 0.11 under `--locked`, so the migrated `hex::encode` sites and the new
`sha256_hex` vectors are exercised; the plan records a full-suite green with only
the three pre-existing `test_vendor_assemble.py` parallel-load flakes.

### Code Review Findings

#### Matches Plan:

- `cli/Cargo.toml:76` pins `sha2 = "0.11"`; `cli/visualiser/server/Cargo.toml:49` collapses the literal to `sha2 = { workspace = true }` — one source of truth.
- `hex` hoisted to `[workspace.dependencies]` (`cli/Cargo.toml:77`) and referenced via `workspace = true` from the server plus `work-adapters` and `jira-client` dev-deps; no new version resolves (`cargo tree -d` lists no `hex` duplicate).
- The three `format!("{:x}", …)` sites migrated to `hex::encode(hasher.finalize())` at `corpus_hashes.rs:27`, `bash_parity_baseline.rs:98`, `adf_oracle_manifest.rs:33`.
- `verifier.rs` gains the empty, `abc`, padding-boundary, and multi-block known-answer vectors — the intrinsic path is now pinned directly, not by distant coverage.
- `measure.py` adds `LauncherTerms`, `parse_asset_bytes`, and an extracted pure `assemble_terms_report`; `tasks/README.md` records the `asset_bytes / (median_ms * 1000)` formula.
- `deny.toml:110` flips to `multiple-versions = "deny"` with 23 exact version-pinned `skip` entries and no `skip-tree`; the TLS/wildcard/`serde-saphyr` bans are intact.
- Before/after measurement JSONs carry a shared `asset_bytes` with MB/s derived; the musl evidence file documents AC 3/AC 4; `meta/work/0217` carries the reciprocal `work-item:0216` edge and the inherited `asset_bytes` prerequisite.

#### Deviations from Plan:

- ⚠️ **Exact CPU model not recorded.** Phase 2 §1 asked for it explicitly ("record the exact CPU model in the committed JSON"). Both records state only `darwin-arm64; battery power; load ~1.4 / ~2–5`. Non-blocking — the 611→2,944 MB/s delta is decisive on any chip — but the plan's "~4.3 ms band meaningful only on an M4-Max-class host" loses its host referent.
- ⚠️ **Asset size is 2,493,392 B, not the plan's cited 2,493,792.** The measured asset is `vcs-1.24.0-pre.41` at 2,493,392 B; the plan repeatedly names 2,493,792 as "canonical". Before and after both use 2,493,392 and the gate is the derived MB/s, so the ms and MB/s thresholds do not diverge — the risk the plan flagged does not materialise. The "canonical" figure was simply never the asset measured.

#### Potential Issues:

- Conclusive aarch64-musl runtime backend *selection* is unproven within 0216 — by design (inspection-only musl scope). The compiled-in opcodes and darwin `cpufeatures = true` corroborate but do not exercise the Linux getauxval/HWCAP path a `crt-static` musl binary uses. Correctly deferred to 0217 and backstopped by the AC 7 fallback; flagged so the hand-off is not dropped.
- The Phase 4 skip set is a standing downstream coupling: any later version-straddling crate must add a justified `skip` or fail `deny:check`. The plan's Migration Notes already call this out.

### Manual Testing Required:

1. Deferred in-plan (accepted, marked `[~]`):
  - [ ] End-to-end `mise run measure:warm-dispatch` producing a live record carrying `asset_bytes` — blocked: it refuses on a cold cache for the unreleased tree version and fetches published binaries, so it cannot observe a local unreleased backend change. Compensated by `TestTermsReport` (persistence path) and the committed `warm_terms` golden (the Rust-emits/Python-parses contract).

2. Deferred to 0217 (out of 0216 scope):
  - [ ] Conclusive aarch64-musl `verifier::sha256_hex` throughput — the only decisive net for musl runtime selection and the AC 7 soft-backend trigger.

### Recommendations:

- Record the exact CPU model in future measurement artefacts, and retro-fill the two 0216 JSONs if cheap, so the fixed ms gate has an unambiguous host referent.
- Reconcile the plan's "canonical 2,493,792" references with the measured 2,493,392 (or note the asset changed), so 0217 does not inherit a stale asset-size assumption.
- Schedule 0217 promptly — it closes the musl AC and the AC 7 trigger that 0216 leaves open by design.
