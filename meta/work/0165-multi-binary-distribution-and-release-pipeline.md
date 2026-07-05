---
type: work-item
id: "0165"
title: "Multi-Binary Static Distribution and Release Pipeline with minisign"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: in-progress
kind: story
priority: high
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0164", "work-item:0168"]
tags: [rust, distribution, release, cross-compile, minisign]
last_updated: "2026-07-05T22:47:31+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-186"
---

# 0165: Multi-Binary Static Distribution and Release Pipeline with minisign

**Kind**: Story
**Status**: In Progress
**Priority**: High
**Author**: Toby Clemson

## Summary

Extend the existing visualiser release pipeline to build, sign, and publish the
`accelerator` launcher and every `accelerator-<sub>` binary as zero-setup, fully
static, per-platform GitHub Release assets that satisfy the `manifest.json` +
minisign contract the 0164 launcher already consumes, with minisign + sha256
integrity and workspace-wide version coherence (ADR-0046).

## Context

This is the **producer half** of on-demand static-binary distribution; the
**consumer half is already frozen by 0164**. The launcher today already:

- depends on `minisign-verify` (pinned) and verifies sha256 + minisign in
  `verifier.rs`;
- embeds a rotation-ready public key — `keys/accelerator-release.pub` is
  committed, `build.rs` embeds it, and `TrustedKeys` does verify-any-of (the
  committed key is a placeholder value; this story replaces it with the real
  public half — see Assumptions);
- fetches and verifies a rich `manifest.json` (`schema_version: 1`, exact-version
  anti-rollback, `binaries → {name} → {description, platforms → {platform} →
  {sha256, signature}}`) plus a detached `manifest.minisig`.

The release pipeline, by contrast, is stale relative to that contract: it
cross-compiles a single `accelerator-visualiser`, emits only a flat
`checksums.json` (`platform → "sha256:hex"`, no descriptions, no signatures, no
`schema_version`), and contains **no minisign code at all**. This story builds
the pipeline up to emit artifacts that satisfy the frozen contract. It reuses
the existing `cargo-zigbuild` cross-compile, `checksums`, version-coherence, and
`gh` upload-and-re-verify machinery (`tasks/build.py`, `tasks/github.py`,
`tasks/version.py`). ADR-0046 rejects cargo-dist in favour of the hand-rolled
invoke pipeline. This is the distribution half of luminosity 0008, materially
bigger here because of multi-binary coherence and minisign key management.

## Requirements

- Cross-compile every workspace binary (the launcher + each `accelerator-<sub>`
  sub-binary present in the cli workspace as of 0163, excluding the visualiser —
  0168 folds it into the workspace and into this release) for the four
  targets (`aarch64/x86_64-apple-darwin`, `aarch64/x86_64-unknown-linux-musl`) via
  `cargo-zigbuild`, retaining the Mach-O/ELF magic-byte sanity checks and adding a
  fully-static assertion for the musl targets. (musl links statically by default
  under `cargo-zigbuild` — do **not** force `+crt-static` with `--target`; verify
  the output instead.)
- Emit a `manifest.json` matching the launcher's frozen `schema_version: 1`
  contract — per-binary `description` and per-platform `{sha256, signature}`. The
  schema is **not** to be designed here; the pipeline conforms to it. Each
  binary's `description` is sourced from that crate's `Cargo.toml`
  `package.description`.
- Retire the flat `checksums.json` this pipeline emits entirely; `manifest.json`
  becomes the single integrity artifact the launcher consumes. 0165 owns
  retirement of the release pipeline's flat `checksums.json`; the visualiser's
  separate standalone `bin/checksums.json` is owned and removed by 0168 (see
  Dependencies) — one file per story, no shared ownership.
- Sign each binary **and** the manifest (`manifest.minisig`) with the minisign
  secret key whose public half is the already-committed
  `keys/accelerator-release.pub`. Sign whole-file (not `-H` prehashed) to match
  the launcher's whole-file `verify`. The launcher-side embed/verify is **out of
  scope — it shipped in 0164.**
- Establish the minisign **key lifecycle** as an operational procedure: generate a
  passwordless (`-W`) keypair (upstream C minisign reads its password from a TTY, so
  `echo | minisign -S` is unreliable in CI); commit its public half to
  `keys/accelerator-release.pub`, replacing the placeholder currently committed
  there — the launcher must be built from that HEAD so it embeds the matching key;
  store the secret half as a GitHub Actions encrypted secret (future: additionally
  encrypted in-repo via fnox — an in-repo secret-encryption tool — for team access);
  rotate
  **on compromise only**, by embedding a new key in the next launcher release. No
  rotation-overlap keyring is needed because each launcher is version-pinned and
  only ever verifies its own release's manifest (the launcher's verify-any-of
  capability leaves headroom if that ever changes).
- Extend version coherence to span `plugin.json`, every crate's effective version,
  and `manifest.version` (the launcher enforces `manifest.version ==
  launcher CARGO_PKG_VERSION` exact-equality); fail the build on any mismatch. The
  workspace-version model already makes members inherit one version — coherence
  additionally flags any member that hardcodes `[package].version`.
- Generalise the `gh`-based upload + re-download-and-re-verify flow to all binaries
  plus `manifest.json` and every `.minisig`, preserving the draft on verification
  failure.
- Drop the documented-but-unimplemented runtime provenance hook and correct
  `RELEASING.md`; keep CI-side SLSA (Supply-chain Levels for Software Artifacts)
  attestations as out-of-band provenance.

## Acceptance Criteria

- [ ] Static binaries build for all four targets for the launcher and each
      sub-binary. Each musl binary passes a `readelf` assertion — no `PT_INTERP`
      program header and no `DT_NEEDED` dynamic entries (do **not** assert ELF type
      `EXEC`; musl static-PIE is `ET_DYN`); each darwin binary passes the Mach-O
      magic check. All published as GitHub Release assets, each with a detached
      `.minisig`; per-binary sha256 values are recorded in `manifest.json` (no
      separate sha256 sidecar assets).
- [ ] Given a pipeline-produced `manifest.json`, when the HEAD launcher parses it,
      then the schema is accepted, `version` equals the release version, and every
      binary carries a `description` — equal to its crate's `Cargo.toml`
      `package.description` — plus per-platform `{sha256, signature}`; the
      `manifest.minisig` verifies against the launcher's embedded key.
- [ ] A pipeline-produced release contains no `checksums.json` asset;
      `manifest.json` is the only integrity artifact published.
- [ ] A binary signed by the pipeline verifies against the launcher's embedded
      `keys/accelerator-release.pub` — i.e. a launcher built from HEAD fetches,
      sha256-verifies, and minisign-verifies a pipeline-produced release
      end-to-end. (Guards against signing with a key that doesn't match the
      committed public key.)
- [ ] Version coherence fails the build when `plugin.json`, any crate's effective
      version, or `manifest.version` disagree.
- [ ] Given an uploaded asset is corrupted (or its `.minisig` swapped) before
      re-download so re-verification fails, the release remains in draft state and
      no assets are published.
- [ ] `RELEASING.md` no longer advertises a runtime `gh attestation verify` hook;
      CI still emits SLSA attestations.
- [ ] The minisign key lifecycle (generate `-W` keypair / commit the public half
      to `keys/accelerator-release.pub` / store the secret half as a GHA secret /
      rotate-on-compromise) is documented in `RELEASING.md` and the signing step
      runs in the release flow.

## Open Questions

- Which minisign implementation/version does the pipeline install, and does it
  accept a passwordless `-W` key non-interactively in CI? Confirm against the
  installed build before relying on it; if an encrypted key is ever required, the
  Go `aead/minisign` reimplementation reads the password from stdin whereas the
  upstream C binary does not.

## Dependencies

- Blocked by: 0163 (workspace exists to build from).
- Relates to: 0164 (froze the consumer contract — `manifest.json` schema,
  checksum + minisign formats, embedded-key mechanism — this story produces
  artifacts that satisfy it; note the currently-committed
  `keys/accelerator-release.pub` is a placeholder this story replaces).
- Coordinate with 0168 (the visualiser joins the multi-binary release): 0165 owns
  retirement of the release pipeline's flat `checksums.json`; 0168 owns removal of
  the visualiser's standalone `bin/checksums.json`. The two share the
  checksums-retirement theme but each owns a distinct file — no ordering
  constraint beyond that separation.
- Operational prerequisite: the release flow's signing step is gated on the
  passwordless `-W` secret key being provisioned as a GitHub Actions encrypted
  secret, which requires a repo administrator. (The key is generated by this
  story — see Requirements — so the blocker is the privileged provisioning
  action, not the key's prior existence.)
- Parent: epic 0136.

## Assumptions

- The existing zigbuild/invoke/`gh` pipeline ports to multi-binary largely
  verbatim; cargo-dist remains rejected (ADR-0046).
- The manifest schema is not to be designed — it is frozen by 0164
  (`schema_version: 1`); the pipeline conforms.
- The currently-committed `keys/accelerator-release.pub` is a placeholder; this
  story generates the real `-W` keypair, commits its public half in place of the
  placeholder, and the launcher must be built from that HEAD so it embeds the
  matching key — otherwise the launcher rejects every release.
- musl targets are static by default under `cargo-zigbuild`; no `+crt-static`
  flag is passed with `--target`.

## Technical Notes

- **Frozen consumer contract (do not modify — reference only):**
  `cli/launcher/src/launch/outbound/resolve/{manifest,verifier,keys,fetcher,mod}.rs`,
  `cli/launcher/src/launch/help.rs`, fixture
  `cli/launcher/tests/fixtures/manifest.example.json`, the committed
  `keys/accelerator-release.pub`, and the `cli/verify` root-of-trust shim.
- **Pipeline extension points:** `tasks/shared/paths.py` / `targets.py` /
  `hashing.py`; `tasks/build.py` (cross-compile loop, magic-byte check
  ~`build.py:104-115`, version coherence ~`build.py:131-151`); `tasks/github.py`
  (draft-preserve-on-failure seam ~`github.py:165-171`); `tasks/version.py`;
  `tasks/release.py`.
- zig + cargo-zigbuild are PyPI deps in `pyproject.toml`, not mise; rust targets
  added via `rustup target add`.
- **Static assertion:** `readelf` (or `llvm-readelf` to avoid needing foreign-arch
  binutils) — fail if `PT_INTERP` present or any `DT_NEEDED` entry exists. `ldd`
  is unsuitable (can't run a cross-arch binary; executes the target).
- **minisign CI gotcha:** the upstream C binary reads its password from a TTY;
  use a `-W` passwordless key, or an action such as `thomasdesr/minisign-action`
  that shims the key/password onto disk.

## Drafting Notes

- Scope reframed during enrichment from "adds minisign, which doesn't exist yet"
  to "build the producer side to satisfy the contract 0164 already froze" — the
  codebase reconciliation found the launcher's minisign verification, embedded
  rotation-ready key, and `manifest.json`/`manifest.minisig` consumption all
  shipped.
- Key lifecycle resolved per author decision: passwordless `-W` key stored as a
  GHA encrypted secret (future fnox-in-repo for team access), compromise-only
  rotation, no overlap window (launchers are version-pinned and verify only their
  own release's manifest); the launcher's latent verify-any-of keyring is
  unused by this policy.
- Descriptions sourced from crate `Cargo.toml` `package.description`, and
  `checksums.json` retired fully in favour of `manifest.json` — both confirmed by
  the author during enrichment.
- Added `relates_to: work-item:0168`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Related: `meta/work/0164-launcher-and-git-style-dispatch.md`,
  `meta/work/0168-fold-visualiser-into-cli-workspace.md`
- ADRs: ADR-0046, ADR-0054
- Spike: `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md` (§4 cross-compile & distribution)
- Frozen contract: `cli/launcher/src/launch/outbound/resolve/manifest.rs`,
  `keys/accelerator-release.pub`, `RELEASING.md`
- minisign / static-binary research: `minisign-verify` crate
  (docs.rs/minisign-verify), minisign CI signing
  (github.com/thomasdesr/minisign-action; jedisct1/minisign#43), fully-static
  verification via `readelf` (no `PT_INTERP` / `DT_NEEDED`)
- Mirrors (luminosity): https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0008-on-demand-static-binary-distribution-and-launcher.md
