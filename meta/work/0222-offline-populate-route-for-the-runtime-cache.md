---
type: "work-item"
id: "0222"
title: "Offline Populate Route for the Runtime Cache"
date: "2026-08-20T00:00:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "low"
blocked_by: ["work-item:0196"]
relates_to: ["work-item:0196"]
tags: ["distribution", "runtime", "cache", "offline", "security"]
last_updated: "2026-08-20T00:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-752"
---

# 0222: Offline Populate Route for the Runtime Cache

**Kind**: Task
**Status**: Draft
**Priority**: Low

## Summary

Design and ship `accelerator cache ensure --from <path-or-url>`, a route that
populates the runtime cache from a locally-staged artifact rather than the
release host, for disconnected and air-gapped environments. It was cut from the
vendored-runtime plan's documentation rather than shipped under-specified,
because it is a second ingestion path into the trust boundary the fetched path
spends a whole step establishing.

## Context

An offline route cannot be "the same checks as a fetched one". Verifying against
the *manifest* costs two HTTPS GETs, which is exactly unavailable in the
disconnected case the flag exists for. So verification has to be anchored on the
attestation alone, and the operator has to stage three files (the archive, its
`.sealed` attestation and the `.sealed.sig` signature) rather than one.

Specifying this in a documentation bullet would have got it implemented without
the extraction rules, size bounds, cause mapping and adversarial coverage the
fetched path receives — hence a properly-scoped work item.

## Requirements

- Accept a local path only, or the same host allowlist the fetched path uses.
- Stage the `.sealed` and `.sealed.sig` alongside the archive so verification is
  anchored on the attestation without the manifest.
- Apply the same entry rules and size bounds, read from the attestation.
- Map any failure into the downgrade vocabulary, distinct from a fetch failure.

## Acceptance Criteria

- [ ] Given a valid staged triple, when `cache ensure --from` runs offline, then
      the tree materialises and verifies.
- [ ] Given a mismatched digest, a wrong-artifact or wrong-platform attestation,
      or an unsigned archive, then each is refused with a distinct, named cause.
- [ ] Given the release host is unreachable, then the route completes without any
      network access.

## Dependencies

- Blocked by: 0196 — needs the tree resolver, the attestation format and the
  extraction rules the vendored-runtime work establishes.

## References

- Surfaced by:
  `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md` (Removal
  sweep, follow-up work items) — cut from the documentation section rather than
  shipped under-specified.
- Related: 0196
