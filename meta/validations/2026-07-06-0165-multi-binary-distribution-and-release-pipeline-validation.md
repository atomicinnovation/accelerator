---
type: plan-validation
id: "2026-07-06-0165-multi-binary-distribution-and-release-pipeline-validation"
title: "Validation Report: Multi-Binary Static Distribution and Release Pipeline with minisign"
date: "2026-07-06T17:16:59+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
parent: "plan:2026-07-06-0165-multi-binary-distribution-and-release-pipeline"
target: "plan:2026-07-06-0165-multi-binary-distribution-and-release-pipeline"
tags: [rust, distribution, release, cross-compile, minisign]
last_updated: "2026-07-06T17:16:59+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Multi-Binary Static Distribution and Release Pipeline with minisign

### Implementation Status

- ✓ **Phase 1: Signing primitives + key-generation tooling + key runbook** — fully implemented (commit `be6e8765`)
- ✓ **Phase 2: Multi-binary cross-compile + static assertions + vendored shims** — fully implemented (commit `077e84f5`)
- ✓ **Phase 3: manifest.json emitter + version-coherence extension** — fully implemented (commit `7387e561`)
- ✓ **Phase 4: Unified upload + re-verify + single-gate publish** — fully implemented (commit `dc2c4caf`)
- ✓ **Phase 5: Wire the new track into the release workflow + CI secret** — fully implemented (commit `6d690359`)
- ✓ **Phase 6: Drop the runtime provenance hook + correct docs** — fully implemented (commit `2437c1ea`)

Every phase carries a dedicated commit (one per phase, atomic), matching the plan's "six independently mergeable phases" approach. All automated success-criteria checkboxes in the plan are marked `[x]`.

### Automated Verification Results

- ✓ `test_signing.py` / `test_build.py` / `test_manifest.py` / `test_workflows.py` / `test_github.py` / `test_release.py` — **127 passed**
- ✓ `mise run build-system:check` (ruff `ALL` + pyrefly strict + actionlint) — passes
- ✓ `mise run cli:check` (workspace rustfmt + clippy `-D warnings` + `lint:vendor-shims:check` drift guard) — passes
- ✓ `mise run lint:workflows:check` (actionlint) — passes
- ✓ `mise run scripts:check` (shfmt + ShellCheck + bashisms + exec-bits) — passes
- ✓ `cargo test --workspace` (frozen consumer contract: `verify.rs`, `resolution.rs`, `manifest.rs`) — **81 passed**
- ✓ `grep -rn ACCELERATOR_VISUALISER_VERIFY_PROVENANCE README.md RELEASING.md` — returns nothing
- ✓ Four committed `bin/accelerator-verify-{platform}` shims + `bin/accelerator-verify.vendored.sha256` marker present; musl shims are real static ELF, darwin are Mach-O; drift guard green
- ✓ `prerelease:sign` fails closed (`SigningError`) with no secret and no dev key — verified by direct invocation

Components not re-run (out of scope of this change — no frontend, server-Rust, or cli-dependency-graph edits): `frontend:check`, `server:check`, `deny:check`, `pup:check`. These are unaffected by the diff.

### Code Review Findings

#### Matches Plan

- `tasks/signing.py` — `sign_file` (explicit signature path, `SigningError` with captured stderr), `resolve_secret_key` `@contextmanager` (0600 temp materialisation, fail-closed), `keys.generate` (`-W`, no secret echo), `sign_staged_binaries` (explicit expected set, fail-closed on partial cross-compile) — all as specified.
- `tasks/manifest.py` — `PlatformAsset` TypedDict, `BinaryEntry`, `build_manifest` (frozen shape, `schema_version: 1`), `collect_entries` (description from crate `Cargo.toml`, sha256, inline `.minisig`), `emit_manifest` (serialise → coherence → sign the exact bytes to `manifest.minisig`).
- `tasks/build.py` — `cli_cross_compile` (four-target zigbuild, magic-byte + musl static assertions), `vendor_verify_shims` (0755, marker), `assert_staged_launcher_versions` (byte-grep for embedded version), `validate_version_coherence` extended with `manifest.version`.
- `tasks/github.py` — `upload_and_verify_release` (single `--draft=false` gate, both tracks, `--clobber`, track-labelled forensic alert, delete inside pre-publish envelope), `_reverify_via_shim` against the **committed** key; old `upload_and_verify` removed.
- `tasks/release.py` — `prepare → sign → finalise` split, `cli_cross_compile` after the version bump, `_sign` under one `resolve_secret_key`, `_assert_no_leaked_artifacts` guard before commit, `_publish` → `upload_and_verify_release`.
- `.github/workflows/main.yml` — secret-scoped `Sign*` step per cut (prerelease + stable + post-stable), attest globs extended to `dist/release/accelerator-*`.
- `RELEASING.md` / `README.md` — key lifecycle, shim regeneration, signing-authority trust boundary, recovery of a preserved draft; stale runtime-provenance claim removed.

#### Deviations from Plan (both sound; one author-approved)

1. **Static-linking check uses `file`, not `llvm-readelf`** — surfaced during implementation via `AskUserQuestion`; the author chose the `file`-based approach. Rationale: no lightweight mise-installable `llvm-readelf` exists (only the full `asdf:mise-plugins/mise-llvm` toolchain, ~GB), and the reference impl (`../luminosity`, work item 0008) uses `file`. `_is_statically_linked` matches `statically linked` / `static-pie linked` / `not a dynamic executable`; `_assert_static_elf` fails closed if `file` is absent. This *removed* the plan's mise ELF-reader pin + deps task + preflight (net simplification). Plan Phase 2 text and success criteria were updated in place to reflect the decision.
2. **Manifest e2e proves the contract via the real shim, not an in-process `FetchVerifyCacheResolver`** — the plan's literal ask (feed producer bytes to the Rust resolver) needs a fragile cross-language artifact handoff (the resolver embeds/injects keys at build/config time). Instead the test verifies producer-emitted bytes through the built `accelerator-verify` shim (the *same* `minisign-verify` crate the launcher embeds) for both the raw manifest signature and each inline per-binary signature, plus a `jsonschema` shape check and a sha256 cross-check. The existing Rust `resolution.rs` / `manifest.rs` tests parse the identical shape, so serde deserialisation is covered. Net: the security-critical assertions (real signature over raw bytes, real sha256, schema conformance) are all exercised against producer output at HEAD.

Minor, non-behavioural: `collect_entries(subbinaries, ...)` drops the plan-sketch's unused leading `version` parameter (`build_manifest` carries the version); param `dir=` renamed `staging_dir=` to avoid shadowing the `dir` builtin (ruff `A002`). `validate_version_coherence` takes `manifest_path=<Path>` rather than a `require_manifest=bool` reading a fixed path — reads the exact emitted file, which is more testable and matches the stated intent.

#### Potential Issues

- **Residual git state after a preserved-draft failure** (called out in the plan's Migration Notes and now documented in `RELEASING.md`): `_publish` pushes the version-bump commit + tag *before* upload/re-verify, so a re-verify failure leaves the commit/tag advanced while the draft is preserved. Recovery is documented (re-drive upload/verify against the same tag; `--clobber` idempotent). Not a regression — inherited from the pre-existing flow; re-ordering `push` is explicitly deferred.
- **Foreign-arch runtime coverage gap** (a conscious plan deferral): only the host-arch (darwin-arm64) shim executes in CI re-verify; the other three platforms are gated by magic-byte + static assertions + the drift guard, not an on-target run. Documented in "What We're NOT Doing".
- **The committed `keys/accelerator-release.pub` remains a placeholder of unknown secret provenance** — by design; the real `-W` keypair is an operational rollout step, not code (see Manual Testing).

### Manual Testing Required

These are the plan's outstanding manual-verification items — all operational (require the production secret / a fork) and none block the implementation:

1. Key lifecycle:
   - [ ] Read `RELEASING.md` "Release signing key lifecycle" as an executable runbook.
2. Rollout (Phase 5 merge gate — **must precede merging Phase 5**):
   - [ ] `mise run keys:generate`; confirm the `.pub`/`.sec` pair round-trips.
   - [ ] Commit the fresh public key; ship a launcher built from that HEAD.
   - [ ] Provision `ACCELERATOR_RELEASE_SECRET_KEY` as a repository/org GHA secret.
3. End-to-end release (needs the provisioned secret / a fork):
   - [ ] `mise run prerelease` against a fork → launcher + manifest + `.minisig` assets present; draft published only after every re-verify.
   - [ ] A launcher built from that commit bootstraps and loads the manifest end-to-end.
   - [ ] Corrupt a staged binary before re-verify → release stays draft.

### Recommendations

- **Do not merge Phase 5 (`6d690359`) until the secret is provisioned and the fresh public key is committed + shipped in the launcher.** Signing fails closed, so an early merge aborts the whole prepare/sign flow (visualiser included) — a total release outage, not graceful degradation. This is the load-bearing sequencing constraint from Migration Notes.
- Before the first real signed release, run the fork dry-run (Manual Testing 3) end-to-end — it is the only exercise of the live upload/re-verify/publish envelope against a real GitHub release.
- Consider the deferred `push`-after-re-verify tightening (out of scope here) in a follow-up to eliminate the residual advanced-commit-on-draft-failure window.
- 0168 (sibling) will populate `DISPATCHED_SUBBINARIES` with the visualiser and remove `bin/checksums.json`; the sub-binary manifest/upload/re-verify paths are implemented and fixture-covered but only exercised with a real entry once 0168 lands.
