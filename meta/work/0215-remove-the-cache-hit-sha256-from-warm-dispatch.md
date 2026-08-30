---
type: "work-item"
id: "0215"
title: "Remove the cache-hit sha256 from warm dispatch"
date: "2026-08-17T20:36:49+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0136"
derived_from: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement"]
relates_to: ["work-item:0189", "work-item:0191"]
tags: ["cli", "launcher", "performance", "bootstrap"]
last_updated: "2026-08-17T20:36:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-744"
---

# 0215: Remove the cache-hit sha256 from warm dispatch

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Drop the cache-hit `sha256` recomputation from the launcher's warm dispatch. It
costs a measured **6.05 ms** of a 35.53 ms warm dispatch — the single largest
launcher-side term — and buys a name/content consistency check that the minisign
signature over the same bytes largely subsumes. Scope the removal to that one
call site and replace what it genuinely protects.

## Context

Measured on darwin-arm64 (Apple M4 Max, macOS 26.3) in work item 0189's closing
session, `meta/measurements/warm-dispatch-3.json`: `reverify` 6.049 ms, of which
`verifier::sha256_hex` over the 2.49 MB sub-binary is the bulk and
`TrustedKeys::verifies` — the Ed25519-over-BLAKE2b check that actually binds the
bytes — is 1.71 ms. So the corruption check costs roughly 2.5x the security
check it precedes.

`resolve/verifier.rs:1-2` names minisign "the security boundary" and sha256 the
"corruption check" outright. `verify_binary` compares the digest and **then**
calls `keys.verifies(bytes, signature)`, so provenance does not depend on the
sha256 at all.

⚠️ **Removal costs more than a `ChecksumMismatch` diagnostic.** Minisign signs
only **bytes**; nothing in the signature binds the asset's **name or version**,
while `cache::find` selects the entry by its `{name}-{version}-` prefix and
takes the expected digest from the filename. Today that comparison is what
rejects an entry whose filename disagrees with its content. Without a
replacement, a stale copy, a botched manual cache edit or a version rename
becomes a silent wrong-version — potentially downgraded — execution rather than a
clean error.

⚠️ An `mmap` is **not** behaviour-preserving here: two passes over a mapping of a
user-writable file can observe different bytes, and truncation raises `SIGBUS`
rather than a clean `Cache` error.

## Requirements

- Remove the cache-hit digest recomputation from the warm path only. Leave
  `verify_binary` intact for `fetch_verify_store`, where the digest arrives from
  the signature-verified manifest and does bind the bytes.
- Preserve the name/version binding by a cheaper means — verifying the
  sub-binary's reported version after exec, or keeping one digest comparison and
  dropping only the redundant second hash.
- Re-measure the warm path before and after in one session.

## Acceptance Criteria

- [ ] The warm dispatch computes the sub-binary's sha256 at most once, evidenced
      by a before/after decomposition from `mise run measure:warm-dispatch`.
- [ ] **A cache entry whose filename disagrees with its content is still
      rejected**, by a named test. This is the criterion the whole item turns on:
      without it the change trades a measured 6 ms for a silent wrong-version
      execution.
- [ ] `fetch_verify_store`'s verification is unchanged, evidenced by a recorded
      search of its call sites.
- [ ] No `mmap` is introduced on this path, or its `SIGBUS` and
      torn-read semantics are handled explicitly.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- **Relates to** 0189, which measured the term and declined this route while its
  criterion was being settled, on the ground that verification posture should not
  be set by an arithmetic target. That objection was to the sequencing; this item
  stands on its own merits.
- **Relates to** 0191, the other warm-path lever, and the `sha2`
  hardware-intrinsics spike, which may reduce the cost instead of removing it.
- **Parent**: epic 0136.

## Assumptions

- The name/version binding is the only property the cache-hit digest provides
  that the signature does not. If a review finds another, this item re-scopes
  rather than proceeding.

## References

- `cli/launcher/src/launch/outbound/resolve/verifier.rs:1-2`, `:29-49`
- `cli/launcher/src/launch/outbound/resolve/keys.rs:62-69`
- `cli/launcher/src/launch/outbound/resolve/cache.rs:51-73`
- `cli/launcher/src/launch/outbound/resolve/mod.rs:90-109` — `reverify`
- `meta/measurements/warm-dispatch-3.json` — the measured term set
