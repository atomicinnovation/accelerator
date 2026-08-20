---
type: work-item
id: "0220"
title: "Advisory-Feed Monitoring for the Vendored Runtime Pins"
date: "2026-08-20T00:00:00+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: medium
relates_to: ["work-item:0196"]
tags: [security, distribution, runtime, playwright, ci]
last_updated: "2026-08-20T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0220: Advisory-Feed Monitoring for the Vendored Runtime Pins

**Kind**: Task
**Status**: Draft
**Priority**: Medium

## Summary

Cross-reference the pinned vendored-runtime revisions — `playwright-core`, Node
and `CHROMIUM_REVISION` — against a security advisory feed, so a disclosed
vulnerability in a shipped runtime is noticed rather than left to a manual bump.
`cargo-deny` covers Rust crates only, and the vendored browser engine is shipped
to every user exempt from per-exec re-verification.

## Context

The vendored-runtime work
(`plan:2026-08-11-0196-design-vendored-runtime-distribution`) surfaced this and
did not fix the whole of it. The **stale-pin half is already handled**: the
release process records these three as security-relevant dependencies in
`RELEASING.md` with an owner and a maximum age, and a scheduled guard opens an
issue when the age is exceeded. That is cheap and bounds neglect.

What remains is the harder half: age is a proxy, not a signal. A revision can be
fresh and vulnerable, or old and fine. The exposure — a full browser engine
shipped to every user, exempt from per-exec re-verification, with the reuse path
skipping fetch-and-verify while pins hold — is large enough to warrant matching
the actual revisions against disclosed advisories rather than only their age.

## Requirements

- A scheduled check cross-references the pinned `playwright-core`, Node and
  Chromium revisions against a CVE/advisory source.
- On a match, it opens an issue naming the advisory and the affected pin, in the
  same shape as the existing max-age guard.
- The advisory source and its coverage limits are documented, since no single
  feed covers all three ecosystems.

## Acceptance Criteria

- [ ] Given a pinned revision with a known advisory, when the check runs, then it
      opens an issue naming the advisory and the pin.
- [ ] Given all pins are clear, when the check runs, then it opens nothing and
      exits cleanly.
- [ ] The advisory source's coverage and blind spots are documented.

## Dependencies

- Relates to: 0196 — the vendored-runtime work that shipped the pins and the
  age-based half of the guard.

## References

- Surfaced by:
  `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md` (Removal
  sweep, follow-up work items)
- Related: 0196
