---
type: "work-item"
id: "0359"
title: "Bootstrap the Launcher from a Staged Release Directory"
date: "2026-10-27T18:01:29+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0355"
blocked_by: ["work-item:0357", "work-item:0222"]
tags: ["bootstrap", "cache", "offline"]
last_updated: "2026-10-27T18:01:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-951"
---
# 0359: Bootstrap the Launcher from a Staged Release Directory

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an operator building an image with no network egress, I want
`bin/accelerator` to take the launcher from a staged release directory named by
`ACCELERATOR_RELEASE_DIR`, so that `cache ensure --from` can run at all and a
seeded image is usable end to end.

## Context

`bin/accelerator` obtains the launcher before any launcher verb runs, fetching
`accelerator-<platform>` and its `.minisig` from the release host when the
cache misses. With no egress that fetch fails, so a staged route implemented
only inside the launcher is unreachable. Three hooks were weighed: an
environment variable read by the bootstrap, argv-peeking for `--from`
(duplicating clap's grammar in bash), and `file://` through
`ACCELERATOR_RELEASE_BASE_URL`. The environment variable was chosen.

## Requirements

- When `ACCELERATOR_RELEASE_DIR` is set and the launcher cache misses, the
  bootstrap copies `accelerator-<platform>` from that directory instead of
  fetching, accepts it only if it matches the plugin tree's launcher pin, and
  makes no network call.
- A pinned launcher already in the cache is used without reading the directory.
- A missing or mismatched staged launcher fails with the bootstrap's integrity
  error naming the directory; it never falls back to the network.
- The directory is read-only to the bootstrap.
- The bootstrap stays within the bash 3.2 floor and the surviving-shell lint
  set.
- A guide documents warming an image online and with no egress, including a
  Dockerfile for each.

## Acceptance Criteria

- [ ] Given `ACCELERATOR_RELEASE_DIR` naming a directory produced by
      `cache stage`, an empty cache and no network, when `bin/accelerator cache
      ensure` runs, then the launcher, every sub-binary and every tree for the
      platform are in the cache and no network call was made.
- [ ] Given a staged launcher whose digest does not match the pin, then the
      bootstrap fails with its integrity error and caches nothing.
- [ ] Given `ACCELERATOR_RELEASE_DIR` naming a directory without the launcher,
      then the bootstrap fails naming the missing file and does not fetch.
- [ ] Given a successful run, then every file in the directory is unchanged.
- [ ] Given the guide's no-egress Dockerfile, when it is built in CI with
      networking disabled, then the build succeeds.

## Dependencies

- Blocked by: pinning the launcher digest in the plugin tree; 0222.

## Technical Notes

- `bin/accelerator` (`fetch_and_verify`); `ACCELERATOR_RELEASE_BASE_URL` is the
  existing precedent for overriding the release source.

## Drafting Notes

- Placed the image-warming guide here because this is the first story after
  which a no-egress image works end to end; its Dockerfile doubles as the
  end-to-end acceptance test.

## References

- `bin/accelerator`.
