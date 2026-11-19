---
type: "work-item"
id: "0360"
title: "Stage a Release Directory for Cache Seeding"
date: "2026-10-27T18:01:29+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0355"
blocked_by: ["work-item:0356", "work-item:0357"]
tags: ["runtime", "cache", "offline"]
last_updated: "2026-10-27T18:01:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-952"
---
# 0360: Stage a Release Directory for Cache Seeding

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an operator preparing a no-egress image, I want `accelerator cache stage
<dir> [--platform <p>]` to write exactly the pinned release assets for a
platform into a directory, so that I can seed an image without reproducing
asset names, platform tokens and sidecars by hand.

## Context

A staged release directory must hold, for one platform, the launcher, every
sub-binary, and each tree archive with its `.sealed` and `.sealed.sig`
sidecars, under their published names. A `gh release download` recipe can
fetch these, but the operator must know which assets the release's pins cover;
that list drifts as artifacts are added or retired. The launcher is the only
component that knows the pinned set, which is what justifies a verb over a
documented recipe.

## Requirements

- `cache stage <dir>` writes, for the host platform by default or the platform
  named by `--platform`, every pinned asset under its published name.
- Every written asset is hash-checked against its pin before it is kept: the
  launcher against the plugin tree's launcher pin, sub-binaries and trees
  against the launcher's compiled-in pins.
- Tree sidecars are written alongside each archive, with the attestation's
  signature verified and the attestation matching the pinned archive digest.
- The verb fetches from the release host; it neither reads nor writes the cache
  root.
- An unknown `--platform` is refused before any network access.
- Re-running against a directory that already holds correct assets fetches
  nothing.

## Acceptance Criteria

- [ ] Given an empty `<dir>`, when `cache stage <dir> --platform linux-x64`
      runs on a darwin host, then `<dir>` holds exactly the linux-x64 launcher,
      sub-binaries and tree archives with sidecars, each matching its pin.
- [ ] Given that directory, when `cache ensure --from <dir>` runs on a
      linux-x64 host with an empty cache and no network, then it succeeds.
- [ ] Given an asset the release host serves with a digest differing from the
      pin, then `cache stage` fails naming the asset and leaves no partial file
      under its published name.
- [ ] Given `--platform` names a platform with no pins, then the verb refuses
      without a network call.
- [ ] Given a directory already fully staged, then a re-run reports every asset
      present and makes no asset download.

## Dependencies

- Blocked by: pinning sub-binary digests into the launcher; pinning the
  launcher digest in the plugin tree.

## Technical Notes

- The launcher reads the launcher pin from the plugin tree (it cannot pin
  itself), located through `ACCELERATOR_PLUGIN_ROOT`.

## Drafting Notes

- Treated cross-platform staging as in scope for this verb even though 0222
  ingests one platform per run, because preparing an image on a different host
  platform is the common case.

## References

- `tasks/signing.py` (published asset names), `cli/launcher/build.rs`.
