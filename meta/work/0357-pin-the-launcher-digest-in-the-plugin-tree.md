---
type: "work-item"
id: "0357"
title: "Pin the Launcher Digest in the Plugin Tree"
date: "2026-10-27T18:01:29+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0355"
blocks: ["work-item:0359", "work-item:0360"]
tags: ["distribution", "bootstrap", "security", "release-pipeline"]
last_updated: "2026-10-27T19:12:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
relates_to: ["adr:ADR-0075"]
---
# 0357: Pin the Launcher Digest in the Plugin Tree

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As an operator relying on a shared, relocated or pre-seeded cache root, I want
`bin/accelerator` to accept a cached or fetched launcher only if its digest
matches one shipped in the installed plugin tree, so that an older signed
launcher — and the older pins it carries — cannot be substituted. This is the
root of the digest-pinned resolution chain, and it retires the vendored verify
shim whose only job was the launcher's signature.

## Context

`bin/accelerator` caches the launcher as
`accelerator-launcher-<version>-<platform>` and accepts it if the vendored
`accelerator-verify` shim verifies its `.minisig` against the committed public
key. The version is bound only by the filename. An older signed launcher placed
at that path passes, and because the launcher carries the compiled-in pins for
trees (and, once pinned, sub-binaries), it rolls back everything below it.

The shim exists because bash cannot verify an Ed25519 signature and a binary
cannot verify itself. Under ADR-0075 the launcher is accepted on its SHA-256,
which the bootstrap already computes with `sha256_files` when it stages the
shim, so the shim has no remaining job. Hashing the 16 MB launcher with
`/sbin/sha256sum` takes ~10 ms on macOS 26 against ~10–20 ms for today's shim
check; the `shasum` fallback takes ~40–70 ms. The cache directory's
exec-capability probe does not depend on the shim: the cold path runs a
separate probe script, and a warm start proves the capability by executing the
launcher.

The digest file changes in every release, so a malicious edit would be less
conspicuous than a change to the public key, which never changes. Release
commits are pushed by the release workflow rather than merged through pull
requests, which is what makes a pull-request guard on the file meaningful.

## Requirements

- The release writes a launcher digest file into the plugin tree, giving each
  platform's launcher SHA-256 and size, and includes it in the release commit,
  so an installed plugin carries the pins of exactly the launchers its release
  published.
- `bin/accelerator` accepts a cached, freshly fetched or staged launcher only
  if its SHA-256 and size equal the host platform's entry; this is the only
  acceptance rule, and no `.minisig` is fetched or checked.
- The launcher download is capped at its pinned size.
- A plugin checkout whose digest file is missing, malformed or lacks the host
  platform refuses to fetch a launcher and runs one only through the existing
  development-launcher override; it never falls back to signature checking.
- Pull-request CI fails any change to the digest file; only the release
  workflow changes it.
- After publishing, the release re-downloads every launcher asset and fails
  unless each matches the committed digest file, its published `.minisig`
  verifies under the release key, and it carries build provenance from the
  release workflow at the release commit.
- The vendored verify shim is retired: the `bin/accelerator-verify-*`
  binaries, their vendor marker and lint, the shim build task, the `cli/verify`
  crate, the bootstrap's shim staging, and the shim branches in retention and
  prune. Staged shim copies left in existing cache roots are reclaimed by
  prune.
- The warm-dispatch measurement replaces its shim-verification term with the
  launcher hash and stays within the harness's existing thresholds.
- The bootstrap stays within the bash 3.2 floor and the surviving-shell lint
  set.

## Acceptance Criteria

- [ ] Given an older release's validly signed launcher and `.minisig` placed at
      the current version's cache path, when `bin/accelerator` runs, then it
      refuses that launcher and fetches the pinned one.
- [ ] Given a fetched launcher whose digest differs from the pin, then the
      bootstrap fails with its integrity error and caches nothing.
- [ ] Given a release host serving a launcher larger than its pinned size, then
      the download is refused and nothing is cached.
- [ ] Given a release, then the digest file in the release commit lists every
      platform's launcher, each equal in SHA-256 and size to the published
      `accelerator-<platform>` asset.
- [ ] Given a pull request that changes the digest file, then CI fails.
- [ ] Given a published launcher that differs from the committed digest, whose
      `.minisig` does not verify, or that lacks build provenance from the
      release workflow, then the post-release check fails.
- [ ] Given a plugin checkout with no digest file and no development-launcher
      override, when `bin/accelerator` runs, then it fails without a network
      call and names the override.
- [ ] Given the plugin tree after this story, then it contains no verify shim
      binary and the bootstrap runs no shim step.
- [ ] Given a cache root holding staged shim copies from an earlier version,
      when `cache prune` runs, then they are reclaimed.
- [ ] Given `measure:warm-dispatch`, then the warm path stays within the
      harness's existing ratio threshold and median ceilings.
- [ ] Given macOS's bash 3.2, then the bootstrap passes its existing lint set
      (shfmt, ShellCheck, bashisms) and the pinned-launcher tests.

## Open Questions

- `wget` has no per-file size cap equivalent to curl's `--max-filesize`. Does
  the bootstrap drop `wget`, or bound it another way?

## Dependencies

- Blocked by: ADR-0075 reaching accepted.
- Blocks: 0359 (bootstrapping the launcher from a staged release directory);
  0360 (`cache stage`).

## Technical Notes

- Relevant code: `bin/accelerator` (`verify_launcher`, `fetch_and_verify`,
  `sha256_files`, shim staging), `tasks/release.py` (`_publish`,
  `_assert_no_leaked_artifacts`), `tasks/signing.py`, `tasks/github.py`
  (re-download and re-verify), `tasks/lint/vendor_shims.py`, and
  `retention/root_entries.rs` and `retention/single_file.rs` in the launcher.
- The release bumps the version, builds, signs, then commits in `_publish`, so
  the digest file can be written after the build and committed with the
  version bump; `_assert_no_leaked_artifacts` and the version commit must both
  admit it.
- The digest file joins the plugin's shipped files, so its path must be in the
  plugin packaging and coherence checks.

## Drafting Notes

- Kept separate from pinning sub-binaries because it changes the bash
  bootstrap and the release commit, not the Rust build, and can ship in
  parallel.
- Folded the shim's retirement into this story rather than a follow-up, so the
  shim never sits unused between the two changes.
- Placed the post-release provenance comparison here because it restores the
  tripwire a never-changing public key provided.

## References

- `bin/accelerator`, `tasks/release.py`, `tasks/signing.py`,
  `tasks/github.py`, `tasks/README.md` (surviving thin shell).
- ADR-0046, ADR-0075.
