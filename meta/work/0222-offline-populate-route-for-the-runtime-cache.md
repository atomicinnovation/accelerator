---
type: "work-item"
id: "0222"
title: "Seed the Runtime Cache from a Staged Release Directory"
date: "2026-08-20T00:00:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0355"
blocks: ["work-item:0359"]
blocked_by: ["work-item:0358"]
relates_to: ["work-item:0196", "work-item:0218", "work-item:0339", "work-item:0340"]
tags: ["distribution", "runtime", "cache", "offline", "security"]
last_updated: "2026-10-27T17:58:04+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reshaped into a staged source for every pinned artifact under the pinned, seedable runtime cache epic."
schema_version: 1
external_id: "PP-752"
---

# 0222: Seed the Runtime Cache from a Staged Release Directory

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an operator building a container image or cache volume, I want `accelerator
cache ensure --from <dir>` to populate the runtime cache from a staged release
directory instead of the release host, so that an image can be warmed without
network egress and sessions do not fetch large artifacts at request time. It is
a second way into the cache's trust boundary, so it receives the same
extraction rules, size bounds and adversarial coverage as the fetched path.

## Context

The primary use is pre-seeding an image or volume; disconnected and proxied
hosts benefit but are not the target. One platform per run is the goal, not a
cross-platform cache.

Under the digest-pinned resolution chain every artifact the launcher resolves —
sub-binaries and tree archives — is accepted only if its SHA-256 equals a digest
compiled into the launcher. That pin, not the manifest, anchors a staged asset,
so the route needs no manifest and no network. Trees still need their `.sealed`
attestation and `.sealed.sig`: a sealed generation stores the release's own
attestation bytes as sidecars, and `cache verify` and the reuse scan check the
extracted tree against its `table_sha256`, `uncompressed_size` and
`entry_count`. The attestation is not covered by the pin, so its signature
check stays, and `Attestation::matches(artifact, platform, digest)` binds it to
the pinned archive.

Specifying this in a documentation bullet would have got it implemented without
the extraction rules, size bounds, cause mapping and adversarial coverage the
fetched path receives — hence a properly-scoped work item.

## Requirements

- `--from` takes a local directory path only; no URL, fetcher or host
  allowlist. When omitted and `ACCELERATOR_RELEASE_DIR` is set, `--from`
  defaults to it.
- The directory holds release assets under their published names: each
  sub-binary, and each tree archive with its `.sealed` and `.sealed.sig`.
- Each staged sub-binary and tree archive is accepted only if its SHA-256
  equals the launcher's pin for `(artifact, host platform)`.
- A tree attestation is accepted only if its signature verifies under a
  trusted key and it matches the artifact, host platform and pinned digest.
- The fetched path's entry rules, path-escape checks, size bounds (from the
  attestation) and free-space precheck apply unchanged.
- The route verifies and installs a cache-owned copy of each asset, never the
  operator's file in place, and never modifies or deletes anything under
  `--from`.
- An artifact already present at its pinned digest succeeds without reading
  `--from`.
- Failures map onto the existing `EnsureCause` tokens; any generalisation
  renames a token rather than adding one.
- The route holds a tree artifact's lock from staging until `reap_orphans`
  returns, as the fetched path does (0339).
- The route refreshes tree claims, holds the generation lease, and either takes
  part in the cold-path root prune or explicitly skips it (0218).

## Acceptance Criteria

- [ ] Given a directory produced by `cache stage` for the host platform, an
      empty cache and a fetcher that fails every call, when `cache ensure
      --from <dir>` runs, then every pinned sub-binary and tree is present,
      `cache verify` passes, and the fetcher was called zero times.
- [ ] Given a staged sub-binary or tree archive whose digest differs from the
      pin (corrupt, another release, or another platform), then the route
      refuses with cause `digest-mismatch` and installs nothing for that
      artifact.
- [ ] Given a tree attestation whose signature does not verify, or which names
      another artifact or platform, then the route refuses with the attestation
      cause and creates no generation, and the message names the failed check.
- [ ] Given a pinned asset or sidecar missing from `<dir>`, then the route
      refuses with an existing cause and the message names the missing file.
- [ ] Given a tree archive matching its pin whose entries break the
      attestation's bounds or escape the root, then extraction refuses exactly
      as on the fetched path, through the shared extraction path.
- [ ] Given every artifact already present at its pinned digest, when the route
      runs with an empty `<dir>`, then it reports each as present.
- [ ] Given a successful run, then every file under `<dir>` is byte-identical
      and still present.
- [ ] Given the route is mid-extraction, when a reap for another tree artifact
      runs, then the route's `.tmp-` entries survive and the route completes.
- [ ] Given a successful run, then each tree's claim is refreshed and its
      generation's lease is held at the same points as on the fetched path.
- [ ] Given `ACCELERATOR_RELEASE_DIR` set and no `--from`, then the route reads
      from that directory.

## Open Questions

- `signature-mismatch` is the token for an attestation that is well signed but
  names the wrong artifact or platform. Should it generalise to an
  attestation-level token, as a rename moving the executor's mapping with it?
- Does the executor need to tell a staged failure from a fetched one? If so,
  the proposal is a `source` field on the `ensure-failed` envelope rather than
  a new cause.

## Dependencies

- Blocked by: 0358 — the `cache` verbs over every pinned artifact, which in
  turn needs sub-binary pins (0356).
- Blocks: 0359 — the bootstrap reading the launcher from the same staged
  directory.
- Relates to: 0339 — a reap spares another artifact's `.tmp-` entries only
  while that artifact's tree lock is held; 0218 — a second materialise route
  must refresh claims and handle the cold-path prune; 0340 — shared cache
  roots.

## Assumptions

- The pins compiled into the launcher are a sufficient anchor because the
  plugin tree pins the launcher (0357); if that root weakens, staged assets
  would need their signatures back.
- Operators stage with `cache stage` or from the release matching the launcher
  in use; any other release fails `digest-mismatch` by design.

## Technical Notes

- `TreeResolver::materialise_fresh` hard-codes the network: `load_manifest`,
  `stream_archive`, `fetch_and_check_attestation`, `reread` and
  `write_sidecars` all go through `self.fetcher` and `self.base_url`. Extract a
  release-asset source port with network and staged-directory sources so lock,
  reap, lease, pointer, claim and extraction logic is shared rather than
  forked; that sharing is what meets the 0339 and 0218 obligations.
  `FetchVerifyCacheResolver::fetch_verify_store` needs the same port for
  sub-binaries.
- The manifest's `archive_size` and `uncompressed_size` feed the free-space
  precheck; staged, they come from the file's length and the attestation's
  `uncompressed_size`.
- Check a tree archive's digest before its attestation, so a wrong-platform
  archive reports `digest-mismatch`; the fetched path checks the attestation
  first.

## Drafting Notes

- Reframed the primary use from air-gapped hosts to pre-seeding images and
  volumes, per the author.
- Took `--from` to name a directory rather than an archive, so assets keep
  their published names and files cannot be mispaired.
- Replaced "unsigned archive" with "untrusted attestation": no staged asset
  carries a signature this route relies on except the tree attestation.
- Moved verb extension, the output contract, the bootstrap's launcher hook and
  staging into sibling stories; this item covers the staged source only.

## References

- Surfaced by:
  `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md` (Removal
  sweep, follow-up work items) — cut from the documentation section rather than
  shipped under-specified.
- Code: `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs`, `pins.rs`,
  `attestation.rs`; `cli/launcher/src/launch/core/tree.rs` (`EnsureCause`);
  `cli/launcher/src/launch/outbound/resolve/mod.rs`.
- ADR-0064.
- Related: 0196, 0218, 0339, 0340, 0355, 0356, 0357, 0358, 0359, 0360.
