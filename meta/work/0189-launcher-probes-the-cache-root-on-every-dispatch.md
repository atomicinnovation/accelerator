---
type: work-item
id: "0189"
title: "The launcher probes the cache root on every external-subcommand dispatch"
date: "2026-08-03T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: draft
kind: task
priority: high
parent: "work-item:0136"
relates_to: ["work-item:0186", "work-item:0169", "work-item:0164"]
tags: [cli, launcher, performance, bootstrap]
last_updated: "2026-08-06T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0189: The launcher probes the cache root on every external-subcommand dispatch

**Kind**: Task
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

0186 removed the write-chmod-exec probe from the shell bootstrap's warm path.
The launcher still runs the identical probe, in Rust, on **every** dispatch to
an external sub-binary — so the saving does not reach `accelerator vcs guard`
or any other dispatched subcommand. Apply the same `ensure_dir` / lazy-probe
split on the Rust side.

## Context

`cache_root::resolve`
(`cli/launcher/src/launch/outbound/resolve/cache_root.rs`)
calls `probe_writable_and_executable`, which `create_dir_all`s, writes
`.accelerator-probe-<pid>`, `chmod`s it to `0o755`, runs it and removes it.
`LazyProductionResolver::resolve` (`cli/launcher/src/main.rs`) calls it
*before* constructing `FetchVerifyCacheResolver`, so it runs ahead of the
sub-binary cache-hit test rather than behind it.

Built-ins (`version`, `config`) never reach the resolver — which is why today's
SessionStart hook escapes the cost, and why 0186's `version`-based measurement
cannot see it.

Measured on darwin-arm64 (macOS 26.3, Apple M4 Max, 2026-08-03) as part of
0186's closeout: the write-chmod-exec-rm cycle costs **131.97 ms** in the repo's
own `bin/` and **107.15 ms** in `/tmp`, against a **3.72 ms** re-exec of the
same file left in place and a **1.41 ms** bare fork+exec floor. Nearly all of it
is macOS's first-exec assessment of a freshly written file. No third-party
EndpointSecurity or anti-malware agent is installed on the measuring host — only
Apple's own `xprotectd`, with SIP and Gatekeeper assessments enabled — so the
penalty is attributable to stock macOS and should be expected on other macOS
hosts rather than being an artefact of this machine.

## Requirements

- Split `probe_writable_and_executable` the way `bin/accelerator` was split: an
  always-run directory-creation half, and a probe reached only when the
  sub-binary is not already cached and verified.
- Gate the probe on the cache miss inside `FetchVerifyCacheResolver::resolve`,
  which is the first place that must write into the cache root, rather than in
  `cache_root::resolve` which runs unconditionally.
- Keep `ResolutionError::CacheRootUnavailable` firing on every path that cannot
  use its cache root, with no new hang and no loss of the `noexec` diagnostic.
  0186's shell side reports which of three causes it detected; the Rust side
  should be at least as specific.

## Acceptance Criteria

- [ ] A warm dispatch to an already-cached, already-verified sub-binary writes
      no probe file — asserted behaviourally against a cache root made
      non-writable after warming, mirroring
      `test_warm_path_survives_a_non_writable_cache_dir`.
- [ ] A cold dispatch still probes, and a `noexec`-shaped cache root still
      fails with `CacheRootUnavailable` naming the directory.
- [ ] The probe runs at most once per process.
- [ ] Warm dispatch latency is measured before and after on one darwin host in
      one session, both figures recorded, with `after ≤ 0.5 × before` as the
      gate — the same shape 0186 used.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- **Relates to**: 0186, which established the pattern, the diagnostic shape and
  the measurement method.
- **Necessary but not sufficient for 0169.** 0169's own hand-off note records
  the bootstrap alone landing near 30 ms against a ≈38.6 ms gate, so the
  threshold decision is 0169's regardless and **must not be deferred pending
  this item**.
- **Parent**: epic 0136.

**Amendment 2026-08-06 — this item's own Requirements are now landed, as a
side effect of unblocking 0169's Phase 10 latency gate; only the acceptance
criteria specific to this item remain unverified.** 0169's Phase 5 (item 2,
"Skip the cache-root write-probe on a warm cache hit") implemented exactly
this item's split: `cache_root::candidate`
(`cli/launcher/src/launch/outbound/resolve/cache_root.rs`) is now selection
only (no I/O beyond env reads), and the write-chmod-exec probe
(`verify_writable`, renamed from `probe_writable_and_executable`) runs only
from `FetchVerifyCacheResolver::fetch_verify_store` — reached on a cache
miss or a failed reverify, never on a warm hit. This is Requirements items 1
and 2 above, verbatim, plus the same `CacheRootUnavailable` diagnostic
Requirements item 3 asks be preserved (unchanged). It was pulled forward
because 0169's own gate (`G ≤ 1.1 × B` warm-call latency, Phase 10) could not
be meaningfully measured while every warm `vcs guard` dispatch paid the
~132 ms probe cost this item names.

**Not yet closed by that landing**: two of this item's acceptance criteria
are still this item's own to satisfy, not 0169's — "the probe runs at most
once per process" (not specifically asserted; the `CorruptCacheAndRefetchFailed`
retry path can invoke `verify_writable` a second time within one process,
which 0169 did not audit against this exact invariant) and the isolated
before/after warm-dispatch latency measurement with the `after ≤ 0.5 ×
before` gate (0169's Phase 10 measures the whole guard dispatch path
end-to-end against the shell baseline, not this fix in isolation). Re-scope
this item to those two remaining items rather than the full original
Requirements list, and re-measure against the post-fix dispatch cost — the
pre-fix ~132 ms figure in Context above no longer describes the code as it
exists.

## Assumptions

- `FetchVerifyCacheResolver::resolve` re-verifies the cached sub-binary's
  signature on every dispatch, so a warm dispatch already proves the cache root
  is exec-capable for real — the same argument that made the shell probe
  redundant on the warm path.

## References

- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — the
  shell-side change, its measurement method and its diagnostic shape
- `meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md`
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
- `cli/launcher/src/main.rs` — `LazyProductionResolver::resolve`
- `docs/internals.md` — "Offline, mirrored and read-only installs", which
  documents this as the limit on a read-only cache directory
