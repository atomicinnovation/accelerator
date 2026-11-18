---
type: "work-item"
id: "0355"
title: "Pinned, Seedable Runtime Cache"
date: "2026-10-27T18:01:28+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "epic"
priority: "high"
relates_to: ["work-item:0218", "work-item:0340", "work-item:0339", "adr:ADR-0075"]
tags: ["distribution", "runtime", "cache", "security", "offline"]
last_updated: "2026-10-27T19:31:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0355: Pinned, Seedable Runtime Cache

**Kind**: Epic
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As an operator building a container image or cache volume for Accelerator
sessions, I want to warm the runtime cache — launcher, sub-binaries and tree
artifacts — either from the release host or from a staged release directory,
with every cached artifact bound to its release by a pinned digest, so that
sessions start without fetching large artifacts at request time and a shared or
seeded cache cannot be rolled back to an older signed artifact.

## Context

The runtime cache resolves three kinds of artifact, anchored inconsistently:

- Tree artifacts (`browser`, `driver`) are anchored on an `(artifact, platform)
  → archive_sha256` map compiled into the launcher from `pins.toml`, so a
  superseded tree is never looked for (ADR-0064).
- Sub-binaries are anchored on a minisign signature over their bytes, with the
  version bound only by the unauthenticated cache filename
  `<name>-<version>-<sha256>`.
- The launcher is anchored the same way by `bin/accelerator`:
  `accelerator-launcher-<version>-<platform>` plus a shim-verified `.minisig`.

Signature verification never reads the minisign trusted comment, so any binary
the release key ever signed passes for any name and version. Anyone able to
write the cache root can plant an older signed launcher or sub-binary under the
current filename, and the older launcher brings its own older tree pins with
it. This matters exactly where the cache root has a different owner or lifetime
from the plugin tree — a relocated `ACCELERATOR_CACHE_DIR` (0218), a shared
multi-user root (0340), or a pre-seeded image volume.

Warming a cache today is possible only online and only for trees
(`cache ensure <tree>...`); with no egress, the bootstrap cannot even obtain the
launcher. 0222 originally scoped an offline route for trees alone, which would
not deliver a usable seeded image.

Refinement settled the shape:

- One trust rule across every layer, recorded in ADR-0075: each layer pins the
  digests of what it executes, rooted in the installed plugin tree. The plugin
  tree pins the launcher; the launcher pins sub-binaries and trees. Signatures
  leave resolution except for the tree attestation, resolution never reads the
  release manifest, and unpinned builds resolve only through the overrides.
- `cache ensure` is the single warming verb across every pinned artifact, with
  a new per-artifact output contract (unreleased, so breaking it is acceptable).
- `ACCELERATOR_RELEASE_DIR` lets the bootstrap take the launcher from a staged
  directory; `cache ensure --from` defaults to it.
- `cache stage` produces a staged directory for any platform.

## Requirements

- ADR-0075 (digest-pinned resolution chain) is accepted before the pinning
  stories start.
- Child stories:
  - 0356: Resolve every artifact from the launcher's pins.
  - 0357: Pin the launcher digest in the plugin tree.
  - 0358: Extend the `cache` verbs over every pinned artifact.
  - 0222: Seed the runtime cache from a staged release directory.
  - 0359: Bootstrap the launcher from a staged release directory.
  - 0360: Stage a release directory for cache seeding (`cache stage`).
- A guide documents warming an image online and with no egress.

## Acceptance Criteria

- [ ] Given a release directory produced by `cache stage --platform <p>` on an
      online machine, when an image for `<p>` is built with no network egress
      using the guide's Dockerfile, then the build completes and a session in
      that image dispatches a tree-consuming sub-binary with zero network calls.
- [ ] Given an older release's validly signed launcher, sub-binary or tree
      planted in a cache root under the current release's name, when it is
      resolved, then it is refused at whichever layer it sits.
- [ ] Given ADR-0075, then its status is accepted before 0356 or 0357 starts.
- [ ] Given every child story done, then no resolution path reads the release
      manifest or checks a minisign signature other than a tree attestation's.

## Dependencies

- Relates to: 0218 (relocated cache roots), 0340 (shared cache roots), 0339
  (tree lock across a materialise).

## Assumptions

- The installed plugin tree is the root of trust; an attacker able to rewrite
  `bin/accelerator` is out of scope.

## Drafting Notes

- Raised as an epic because one offline-route task grew, through refinement,
  into a trust-model change plus four user-facing capabilities.
- Priority is high because the pinning stories close a rollback and
  substitution gap that exists on today's cache-hit path, independent of
  seeding.

## References

- ADR-0046 (zero-setup static binary distribution), ADR-0060, ADR-0061,
  ADR-0064 (producer-signed tree attestation over a compiled-in digest),
  ADR-0075 (digest-pinned resolution chain).
- Plan: `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md`.
- Related: 0136, 0218, 0222, 0339, 0340.
