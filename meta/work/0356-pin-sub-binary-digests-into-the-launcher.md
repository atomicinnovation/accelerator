---
type: "work-item"
id: "0356"
title: "Resolve Every Artifact from the Launcher's Pins"
date: "2026-10-27T18:01:29+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0355"
blocks: ["work-item:0358", "work-item:0360"]
tags: ["distribution", "runtime", "security", "release-pipeline"]
last_updated: "2026-10-27T19:12:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
relates_to: ["adr:ADR-0075"]
external_id: "PP-948"
---
# 0356: Resolve Every Artifact from the Launcher's Pins

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As an operator relying on a shared, relocated or pre-seeded cache root, I want
the launcher to accept a sub-binary or tree archive only at the digest and
size compiled into it, so that an older or different artifact the release key
once signed cannot be executed in place of the current one, and no download
can grow beyond the artifact's pinned size. This brings sub-binaries under the
rule trees already follow and takes the release manifest out of resolution.

## Context

`FetchVerifyCacheResolver` verifies a sub-binary with `verify_binary`: a SHA-256
against an expected digest, then a minisign signature against the compiled-in
`TrustedKeys`. On a cache hit the expected digest is parsed from the cache
filename (`<name>-<version>-<sha256>`), which is unauthenticated, and the
signature check ignores the trusted comment, so nothing binds a signature to a
name, version or platform. An older signed `accelerator-research`, or a signed
`accelerator-visualiser` renamed to `accelerator-research-…`, passes
`reverify` and is executed. Its download is also unbounded:
`Fetcher::try_get` reads the whole body into memory.

Trees avoid the substitution gap because `acquire` resolves only the
compiled-in `archive_sha256` (`pins.rs`), but their fetch path still loads the
signed manifest: it compares the manifest's digest to the pin, checks the
archive's signature from the manifest, and takes `archive_size` from it. That
size is the tree download cap (`StreamLimits::for_archive`) and feeds the
free-space precheck, so the manifest cannot leave resolution until the size is
pinned.

Sub-binaries cannot use the committed-pin mechanism because they are built
from the same commit as the launcher: today `cli_cross_compile` builds the
whole `cli/` workspace in one `cargo zigbuild` per target, so no sub-binary
digest exists when the launcher compiles. Measured locally, a second release
pass that changes only a launcher `build.rs` input recompiles only the
launcher crate: ~25 s wall, ~200 s CPU, against a 4 m 55 s cold workspace
build.

## Requirements

- The release build produces every sub-binary (including the visualiser server)
  for every target before any launcher, records each one's SHA-256 and size,
  and builds each target's launcher with the full
  `(sub-binary, platform) → (sha256, size)` map for all four platforms
  embedded.
- The second pass reruns the same full-workspace invocation so only the
  launcher crate recompiles.
- `pins.toml` carries each tree archive's `archive_size` beside its
  `archive_sha256`, and `build.rs` embeds both; a pin with a missing or zero
  size fails the build.
- The resolver accepts a sub-binary, on fetch and on every cache hit, only if
  its SHA-256 equals the compiled-in digest for `(name, host platform)`; the
  cache filename is never the source of the expected digest.
- Every sub-binary and tree archive download is capped at its pinned size.
- Neither sub-binary nor tree resolution reads the release manifest or checks
  a sub-binary or archive `.minisig`; the tree attestation and its
  `.sealed.sig` are still fetched and verified, and still checked on every
  tree cache hit.
- The tree free-space precheck uses the pinned `archive_size` and the verified
  attestation's `uncompressed_size`.
- A sub-binary with no pin for the host platform is an unsupported-platform
  miss, not a reason to fetch.
- Unpinned is a compile-time mode with no runtime switch. An unpinned build
  (local `cargo build`, PR CI) refuses release-host resolution of sub-binaries
  and resolves them only through `ACCELERATOR_<SUB>_BIN`.
- An unpinned launcher carries a marker the release scans for, failing before
  signing, as `_assert_no_test_loopback` does for the test feature.
- The registration checklist in `tasks/README.md` covers a new sub-binary's
  pin.

## Acceptance Criteria

- [ ] Given a cache root holding an older release's validly signed sub-binary
      and `.minisig` under the current version's filename, when that
      sub-binary is dispatched, then it is refused and re-fetched rather than
      executed.
- [ ] Given a validly signed sub-binary renamed to another sub-binary's cache
      filename, when the other sub-binary is dispatched, then it is refused.
- [ ] Given a release build, then each target's launcher embeds a digest and
      size for every dispatched sub-binary on every platform, each equal to
      the published asset's.
- [ ] Given a release host serving a sub-binary or tree archive larger than
      its pinned size, then the download stops at the pinned size and the
      artifact is refused without being cached.
- [ ] Given a `pins.toml` tree entry with no `archive_size`, or a size of zero,
      then the launcher build fails.
- [ ] Given a pinned launcher and a fetcher that records requests, when a
      sub-binary and a tree are fetched, then no request is made for
      `manifest.json`, `manifest.minisig`, a sub-binary's `.minisig` or an
      archive's `.minisig`.
- [ ] Given a tree generation whose attestation signature no longer verifies,
      when the tree is resolved from the cache, then it is a miss.
- [ ] Given a release build in which any launcher was built unpinned, then the
      release task fails before signing.
- [ ] Given a release build, then the second pass recompiles only the launcher
      crate.
- [ ] Given an unpinned development launcher and no `ACCELERATOR_<SUB>_BIN`,
      when a sub-binary is dispatched, then it refuses without a network call
      and names the override; with the override set, it executes that path.
- [ ] Given a pinned launcher, then no environment variable or flag makes it
      resolve sub-binaries as an unpinned build would.

## Dependencies

- Blocked by: ADR-0075 reaching accepted.
- Blocks: 0358 (the `cache` verbs over every pinned artifact); 0360
  (`cache stage`).

## Assumptions

- Artefacts need not be reproducible: digests are taken from the bytes the
  release actually publishes, not recomputed elsewhere.

## Technical Notes

- `cli/launcher/build.rs` already generates `EXPECTED_TREE_DIGESTS` from
  `pins.toml` with `rerun-if-changed`; the sub-binary map can follow the same
  generated-source pattern from a file the first pass writes.
- `server_cross_compile` (`tasks/build.py`) builds the visualiser separately
  and must move ahead of the launcher pass.
- Under resolver 3, a `-p accelerator` second pass may unify features
  differently and recompile shared crates; rerun the full-workspace command.
- The manifest's own documentation (`ArtifactPlatformEntry`,
  `resolve/manifest.rs`) records why a defaulted zero size is dangerous: it
  disables both the download cap and the decompression ceiling. The pinned
  size inherits that rule.
- `TreeResolver::materialise_fresh` currently streams the archive before
  fetching the attestation; with the cap pinned, either order is safe.

## Drafting Notes

- Read "uniform with trees" as every platform's digests in every launcher, not
  only the host's, because `cache stage --platform` prepares directories for
  other platforms.
- Read ADR-0075's "version coherence is asserted at release time only" as
  removing the launcher's runtime `expected_version` check against the
  manifest; the pinned digest already implies the release.
- Folded removing the manifest from the tree fetch path into this story
  because it shares the resolver modules and is blocked on the same pinned
  sizes; the title changed accordingly while the filename kept its slug.

## References

- `cli/launcher/src/launch/outbound/resolve/mod.rs`, `verifier.rs`, `cache.rs`,
  `keys.rs`, `fetcher.rs`, `manifest.rs`;
  `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs`, `download.rs`;
  `cli/launcher/build.rs`; `pins.toml`; `tasks/build.py`.
- ADR-0046, ADR-0064, ADR-0075.
