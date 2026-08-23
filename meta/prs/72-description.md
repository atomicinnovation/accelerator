---
type: pr-description
id: "72"
title: "accelerator-design: vendored runtime distribution"
date: "2026-08-21T14:24:50+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
work_item_id: "work-item:0196"
parent: "work-item:0196"
relates_to: ["work-item:0205", "work-item:0208", "work-item:0214"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/72"
pr_number: 72
tags: [rust, design, playwright, launcher, release-pipeline, tree-artifacts, distribution]
revision: "b2f869a39e24c026bf017a3979ef1788052de903"
repository: "accelerator"
last_updated: "2026-08-21T14:24:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# accelerator-design: vendored runtime distribution

## Summary

Vendors the Playwright runtime so the design tooling stops depending on a system Node.js. The launcher gains the ability to resolve directory-tree artifacts alongside the single-file sub-binaries it already fetches; the release pipeline gains a build-time assembly step that constructs the driver bundle and a headless Chromium from verified upstream inputs and publishes them under the project's own signing key; and the executor swaps onto them. This delivers work-item:0196 across three phases plus a removal sweep, and provisions the real trust anchors — the whole verify-and-assemble pipeline now runs green end-to-end against real upstream artifacts.

## Changes

### Phase 1 — launcher resolves tree artifacts (`cli/launcher`)

- The resolver fetches, verifies, extracts and seals directory-tree artifacts: a `trees/` layout that is content-addressed (an unchanged pin is one tree across plugin versions), per-platform, generation-suffixed (so a rename target is always fresh), and sealed read-only. A digest-keyed pointer resolves offline; the launcher embeds its expected `(artifact, platform) → digest` map at build time from `pins.toml`, so rollback is refused and cross-version adoption is free.
- The hit path's only cryptographic anchor is a signed, producer-side attestation binding artifact identity, platform, content and the `.files` table digest — verified under the embedded release key. The `.files` table ships inside the archive as its first member, so extraction verifies each member in one pass and the archive signature covers it.
- A streaming download replaces the buffered fetch (a ~120MB archive is never held in RAM), with a per-read idle bound, a whole-loop wall-clock bound, and cross-process `Range` resume. Release signatures are already prehashed, so verification needs no second pass.
- The `accelerator cache` built-in adds `ensure`, `verify`, `repair`, `prune` and `notices`. `flock` is the only cross-process liveness mechanism — an in-use lease and a single-flight lock, no pid gates — and retention claims (`trees/claims/`) let sibling installs share a cache root without one evicting another's ~294MB.
- Adversarial and concurrency coverage: path-escape/symlink/hardlink/device rejection, decompression-bomb ceilings, crash injection across every publish step, two concurrent cold resolutions issuing exactly one fetch, the ownership/symlink guard on `trees/`, and `cache verify`'s tamper detection (including a `.files` table rewritten to match a substituted member, caught by the signed `table_sha256`).

### Phase 2 — release pipeline assembles and verifies the runtime (`tasks/vendor`, `.github/workflows`)

- A new `assemble-runtime` CI job (`permissions: {}`) verifies the three upstream inputs against their publishers and assembles the driver and browser trees per platform; a matrix `smoke-runtime` job executes the binaries natively; `release`/`prerelease` consume the workflow artifacts and gate them against `pins.toml` before signing.
- Upstream verification: `playwright-core`'s npm/Sigstore SLSA provenance (via `cosign verify-blob-attestation`, pinned to microsoft/playwright's publish workflow) plus the registry-signature-and-integrity binding; Node's GPG-signed `SHASUMS256.txt`; and Chromium pinned by revision (cross-checked against `browsers.json`) and per-platform byte digest.
- The manifest gains an additive `artifacts` map beside `binaries`; signing, upload and pre-publish re-verification each grow a tree-artifact arm; each archive ships with its `.minisig`, its `.sealed` attestation and that document's detached signature. Assembly is byte-deterministic (asserted across `TZ`/`LANG`/`umask`), and every artifact carries redistribution `NOTICES/`.
- A placeholder-detection trust-anchor guard (`vendor:check-trust-anchors`) fails the assembly job legibly if any pin or publisher key is still a placeholder, pointing at the RELEASING.md refresh procedure. The two artifact GitHub Actions are SHA-pinned.

### Phase 3 — the executor swaps onto the vendored runtime (`cli/design`, `cli/design-cli`, `skills/design`)

- The downgrade vocabulary is retargeted to the vendored-runtime reasons (dropping `node-missing`/`node-too-old`/`bootstrap-failed`; adding `unsupported-platform`, `loader-unresolvable`, `glibc-too-old`, `runtime-libraries-missing`, `artifact-unavailable`, `materialisation-in-progress`). Platform classification, spawn-failure classification, the ADR-0062 availability ordering, the sticky-marker policy and a `cache ensure` structured-cause envelope are added as a self-contained domain layer.
- The executor composes the runtime over lazy thunks (platform probe → runtime → browser), retargets both spawn sites at the driver tree's own `node`, threads the resolved browser executable, and reads the `design.browser_path` hatch through a vetted precedence helper. The Playwright loader imports the driver tree's `playwright-core` by absolute path (ESM ignores `NODE_PATH`), and `daemon.js` launches Chromium with the executor-resolved path. The lockhash namespace is removed.
- `PROTOCOL.md`, the skill, evals and benchmarks are retargeted to the live reasons, with a standing conformance guard against stale-script and retired-reason references.

### Removal sweep

- Deletes `ensure-playwright.sh` and its test, `package-lock.json` and the metadata-script residue; drops the system-Node prerequisite from `plugin.json`; updates the config-suite floor, the docs, ADR-0064 (superseding ADR-0061), and raises four follow-up work items (0222, 0223, 0225, 0226).

### Trust anchors provisioned, and the assembly proven against real upstream

Running the pipeline against real inputs for the first time surfaced and fixed five latent defects the fabricated test fixtures had hidden:

- **SLSA** — `gh attestation verify` cannot check playwright-core's npm/Sigstore provenance (subject keyed by the tarball sha512 and a `pkg:npm` PURL, not the file sha256 GitHub's attestation store matches); replaced with `cosign` over the registry bundle, `cosign` pinned in `mise`.
- **Node** — the code fetched the clearsigned `.asc` and verified it as detached (Node ships a detached `.sig`); it rejected `EXPKEYSIG` (a signature made by a key valid at signing time whose key has since expired — now accepted, while `REVKEYSIG` stays rejected); and it passed the armored keyring to `gpg --keyring`, which needs a binary keyring (now imported into an ephemeral homedir).
- **Chromium** — the assembly globbed `chrome-headless-shell`, but Playwright ships `chrome-<platform>/headless_shell` (renamed to the runtime's name on placement); and the archive ships no licence, so Chromium's BSD-3-Clause `LICENSE` is committed and sourced for NOTICES.

The real anchors are committed and verified: the nine active Node release signers, the npm registry signing key, Chromium revision 1193 with per-platform digests, Node 22.22.2, and all eight `assembled_sha256` values from a byte-deterministic assembly.

## Context

Implements work-item:0196, following `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md`. Depends on the warm-dispatch measurement method from work-item:0205 and the tree-artifact mechanisms settled in work-item:0214; the container CI lane is owned by work-item:0208.

## Testing

- [x] Component suites green locally: launcher (153 tests, `cli:check` clean), vendor/tasks (120 tests), and `mise run build-system:check`.
- [x] The full verify-and-assemble pipeline runs end-to-end against real upstream: `vendor:verify-upstream-inputs` and `vendor:assemble-tree-artifacts` both exit 0, producing eight byte-deterministic archives (verified identical across `TZ`/`LANG`/`umask`).
- [x] `vendor:check-trust-anchors` passes with the real anchors, and the launcher's compiled-in digest map agrees with `pins.toml`.
- [x] The darwin-arm64 archives pass the native smoke check — the vendored `node` and `chrome-headless-shell` execute.
- [ ] The full `mise run` CI mirror is re-run by CI on this PR (it was green locally before the trust-anchor/assembly changes; the changed components are covered by the checks above).

## Notes for Reviewers

- This is the whole work-item:0196 epic (~100 commits). Reviewing per phase or per commit is more tractable than the squashed diff; commit messages are scoped to one concern each.
- The five assembly-pipeline fixes in the final section are the highest-value review target: the release/assembly path had never run against real upstream, and each fix corrects a real correctness defect (SLSA verification mechanism, Node signature handling and expiry policy, Chromium layout and licensing) rather than a fixture detail.
- The publisher keys and pins are trust anchors: `keys/nodejs-release.asc`, `keys/npm-registry.pem`, and the `pins.toml`/`licenses/chromium.LICENSE` values warrant an out-of-band fingerprint check as part of review.
- Known human/org-gated remainders, out of scope here: a criterion-3138 review-gate CI job (needs a named reviewer team, branch protection and an org-read token); a real release cut (the sign/upload/reverify path has not yet run against real artifacts); the container runtime harness (AC6/AC11/AC12); `timeout-minutes` from a measured double-pass; and a full third-party-credits (about:credits) legal review for Chromium.
