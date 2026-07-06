---
type: pr-description
id: "14"
title: "Multi-binary static distribution and release pipeline with minisign"
date: "2026-07-06T17:37:11+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
work_item_id: "0165-multi-binary-distribution-and-release-pipeline"
parent: "work-item:0165"
relates_to: ["work-item:0164", "work-item:0168"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/14"
pr_number: 14
tags: [rust, distribution, release, cross-compile, minisign]
revision: "da9276c630caf608c38e4d34e90fb9aed91d749c"
repository: "accelerator"
last_updated: "2026-07-06T17:37:11+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Multi-binary static distribution and release pipeline with minisign

## Summary

Builds the **producer half** of on-demand static-binary distribution (work item
0165). The hand-rolled invoke release pipeline now cross-compiles the
`accelerator` launcher for all four targets, minisign-signs each binary, emits
and signs a `manifest.json`, vendors the per-platform verify shims the bootstrap
needs, and publishes both the visualiser and launcher tracks through a single
draft→published gate. The **consumer half was frozen by 0164** — this PR makes
the pipeline emit artifacts the launcher and bootstrap already accept; the frozen
contract (`resolve/*.rs`, `cli/verify`, `bin/accelerator`, the manifest fixtures)
is untouched.

The change is strictly **additive**: the visualiser keeps releasing at every
commit, and the launcher track wires in live only in the final commit.

## Changes

- **Signing primitives** (`tasks/signing.py`): `sign_file` (explicit signature
  path, `SigningError` carrying minisign's stderr), `resolve_secret_key`
  context manager (materialises the GHA secret to a `0600` temp file or yields
  the local dev key, fails closed if neither exists), a `keys:generate` task for
  `-W` keypairs, and `sign_staged_binaries` (signs an explicit expected set —
  fails closed on a partial cross-compile).
- **Cross-compile + vendored shims** (`tasks/build.py`, `mise.toml`):
  `cli_cross_compile` (four-target `cargo zigbuild` with magic-byte + `file`-based
  musl static-linking assertions), `vendor_verify_shims`, the four committed
  `bin/accelerator-verify-{platform}` shims + a drift-guard marker, and a
  `lint:vendor-shims:check` task that fails CI when the shims' `cli/verify` build
  inputs change.
- **Manifest emitter** (`tasks/manifest.py`): `collect_entries` /
  `build_manifest` / `emit_manifest` — sources each sub-binary's description from
  its crate `Cargo.toml`, computes sha256, embeds the inline `.minisig`, and signs
  the serialised bytes to `manifest.minisig` (with no re-serialisation between
  sign and upload). `validate_version_coherence` now folds in `manifest.version`.
- **Unified single-gate publish** (`tasks/github.py`):
  `upload_and_verify_release` uploads every asset across both tracks, re-verifies
  each (visualiser sha256, launcher/manifest shim-minisig against the committed
  key, sub-binary sha256 + inline signature), then flips `--draft=false` exactly
  once. A verification failure on either track preserves the draft with a
  track-labelled forensic alert; any other error deletes the release from inside
  the pre-publish envelope. Uploads use `--clobber`.
- **Pipeline wiring** (`tasks/release.py`, `.github/workflows/main.yml`): the
  release orchestration is now `prepare → sign → finalise`; the workflow gains a
  secret-scoped `Sign*` step per cut so `ACCELERATOR_RELEASE_SECRET_KEY` is never
  in the environment during compilation, and the attest globs extend to the
  launcher binaries.
- **Docs** (`RELEASING.md`, `README.md`): key lifecycle runbook, shim
  regeneration, the push-to-`main`-is-signing-authority trust boundary,
  preserved-draft recovery; the stale `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE`
  runtime hook claim is removed (no such hook exists).
- **Dependency**: `jsonschema` pinned in the dev group for the manifest
  shape tests.

## Context

- Work item: `meta/work/0165-multi-binary-distribution-and-release-pipeline.md`
- Plan: `meta/plans/2026-07-06-0165-multi-binary-distribution-and-release-pipeline.md`
  (status `done`)
- Validation: `meta/validations/2026-07-06-0165-multi-binary-distribution-and-release-pipeline-validation.md`
  (result `pass`)
- Frozen consumer contract delivered by 0164; 0168 will populate the manifest
  with the visualiser sub-binary and remove `bin/checksums.json`.

## Testing

- [x] `uv run pytest tests/unit/tasks tests/integration/tasks` — full task suite
  green (127 new/changed producer tests across signing, build, manifest,
  workflows, github, release)
- [x] `cargo test --workspace` — 81 cli tests pass; the frozen consumer contract
  is intact
- [x] `mise run build-system:check` (ruff `ALL` + pyrefly strict + actionlint)
- [x] `mise run cli:check` (rustfmt + clippy `-D warnings` + vendored-shim drift
  guard)
- [x] `mise run scripts:check` (shfmt + ShellCheck + bashisms + exec-bits)
- [x] `mise run build:vendor-verify-shims` run for real — the four committed
  shims are genuine static musl ELF / Mach-O and pass their assertions
- [x] `prerelease:sign` fails closed (`SigningError`) with no secret and no dev
  key — verified by direct invocation
- [ ] Full CI prerelease with `ACCELERATOR_RELEASE_SECRET_KEY` provisioned, and a
  launcher built from that commit bootstrapping + loading the manifest
  end-to-end (operational — requires the production secret / a fork)
- [ ] Fork dry-run of the live upload/re-verify/publish envelope

## Notes for Reviewers

- **Merge sequencing is load-bearing.** Signing fails closed, so the final
  commit (`Wire the launcher signing track into the release pipeline`) must not
  take effect until a repo admin has (a) provisioned
  `ACCELERATOR_RELEASE_SECRET_KEY` as a **repository/org** GHA secret, and
  (b) committed a fresh `-W` public key and shipped a launcher built from that
  HEAD. Merging before then would abort the whole prepare/sign flow — including
  the visualiser — a total release outage, not graceful degradation. See
  `RELEASING.md` and the plan's Migration Notes.
- **Two deliberate deviations from the plan** (both documented in the validation
  report): the musl static-linking check uses `file` rather than `llvm-readelf`
  (no lightweight mise `llvm-readelf` exists; the luminosity reference impl uses
  `file`; this dropped a heavy release-runner dependency), and the manifest
  end-to-end test proves the producer→consumer contract through the real
  `accelerator-verify` shim (the same `minisign-verify` the launcher embeds) plus
  jsonschema + sha256 rather than an in-process resolver call that would need a
  fragile cross-language handoff.
- **The committed `keys/accelerator-release.pub` stays a placeholder** of unknown
  secret provenance — the real keypair is an operational rollout step, not code.
- **Residual git state**: `_publish` pushes the version-bump commit + tag before
  upload/re-verify, so a re-verify failure leaves the commit/tag advanced while
  the draft is preserved. Recovery is documented; re-ordering the push is a
  deferred follow-up.
- **Scope note**: the branch also carries the 0165 meta artifacts (research,
  work-item + plan reviews, plan, validation) and one unrelated commit,
  `Add comment-hygiene guidance to implement-plan instructions`
  (`.accelerator/skills/implement-plan/instructions.md`), which predated this
  session on the branch. Foreign-arch launchers/shims are gated by
  magic-byte + static assertions + the drift guard, not an on-target run — a
  conscious coverage deferral called out in the plan.
