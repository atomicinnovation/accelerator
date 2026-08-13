---
type: work-item
id: "0189"
title: "At-Most-Once Guarantee for the Launcher's Cache-Root Probe"
date: "2026-08-03T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: in-progress
kind: task
priority: low
parent: "work-item:0136"
blocked_by: ["work-item:0169"]
relates_to: ["work-item:0186", "work-item:0164", "work-item:0191"]
tags: [cli, launcher, performance, bootstrap]
last_updated: "2026-08-11T13:21:34+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0189: At-Most-Once Guarantee for the Launcher's Cache-Root Probe

**Kind**: Task
**Status**: In Progress
**Priority**: Low
**Author**: Toby Clemson

## Summary

The launcher's write-chmod-exec cache-root probe has already been moved off the
warm dispatch path — 0169 landed the split this item originally asked for. Three
things remain. Pin the probe's invocation count with a test across the warm-hit,
cold-miss and refetch paths; delete `cache_root::resolve`, the now-unused
wrapper that keeps a second probe path alive in the module; and take the
warm-dispatch latency measurement 0169 deferred, which no other open item owns.

Throughout this item, **dispatch** means one launcher process. One
`verify_writable` call performs exactly one probe and increments the
`SEQUENCE` counter exactly once; "probe count" always means that counter.

**Retracted 2026-08-12.** `SEQUENCE` cannot serve as the probe count.
`SEQUENCE.fetch_add` sits *after* the `create_dir_all` early return inside
`probe_writable_and_executable`, so a `verify_writable` call whose directory
creation fails increments it zero times — demonstrated by running
`a_probe_against_an_uncreatable_directory_still_counts` against a hoisted
`SEQUENCE` accessor, which read a delta of 0. "Probe count" therefore means the
invocation count held in `PROBE_ATTEMPTS`, a thread-local incremented as the
first statement of `verify_writable`. `SEQUENCE` keeps its filename-uniqueness
meaning unchanged.

## Context

`cache_root::candidate`
(`cli/launcher/src/launch/outbound/resolve/cache_root.rs`) selects the cache
root — the `ACCELERATOR_CACHE_DIR` override when set, otherwise
`${ACCELERATOR_PLUGIN_ROOT}/bin` — and does nothing else: no filesystem write,
no process spawn. That derivation is why assertions about a read-only *plugin*
root and a read-only *cache* root both appear below; they are the same probe
reached by different selections.

The probe itself is `verify_writable` (renamed by 0169 from
`probe_writable_and_executable`, which survives as the private function it
calls). It runs from exactly one production call site:
`FetchVerifyCacheResolver::fetch_verify_store`
(`cli/launcher/src/launch/outbound/resolve/mod.rs:141`), reached on a cache miss
or a failed re-verification, never on a warm hit. `main.rs` calls
`cache_root::candidate`, never `cache_root::resolve`.

Two of this item's original acceptance criteria are therefore already satisfied,
by tests 0169 landed:

- A warm dispatch writes no probe file —
  `resolve_succeeds_from_a_read_only_cache_root_on_a_hit`
  (`cli/launcher/tests/resolution.rs:549`).
- A cold dispatch still probes and an unwritable root still fails with
  `CacheRootUnavailable` —
  `an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss`
  (`cli/launcher/tests/resolution.rs:568`), which additionally asserts the
  failure precedes any network round trip.

An amendment to this item dated 2026-08-06 claimed the
`CorruptCacheAndRefetchFailed` retry path could invoke `verify_writable` twice
within one process. Reading `FetchVerifyCacheResolver::resolve` shows it cannot:
every branch returns, so a single resolution reaches `fetch_verify_store` at
most once. That amendment is superseded by this paragraph and was removed rather
than left in place — which makes 0169's Phase 10 hand-off record correspondingly
stale, since it records as complete that dated 2026-08-06 amendments were
grep-verified onto 0125, 0172, 0183 and 0189. The gap the amendment was reaching
for is real, though: no test pins the invariant, so a future refactor could
reintroduce a second probe silently.

`cache_root::resolve` (`candidate` followed by `verify_writable`) survives with
no production caller — only its own four unit tests, which do probe, and which
share a test process with the counting tests below. Removing it leaves
`fetch_verify_store` as the module's single probe entry point, making the
at-most-once property structural rather than incidental.

The pre-fix cost this item was raised to remove — measured on darwin-arm64
(macOS 26.3, Apple M4 Max, 2026-08-03) at **131.97 ms** for the
write-chmod-exec-rm cycle against a **3.72 ms** re-exec of a file left in place
— describes a cost the warm path no longer pays. The probe itself survives on
the miss path, and these figures are retained to explain why the split was made
and as the starting point for the outstanding measurement.

## Requirements

- Pin the probe's invocation count with a test: the count observed across a
  single `FetchVerifyCacheResolver::resolve` call is exactly 1 on a cold miss,
  exactly 1 on each refetch-after-failed-re-verification path, and exactly 0 on
  a warm hit. The assertions take the form of a **delta** captured either side
  of that call, permanently and not as an interim measure — the counter is
  process-wide, so an absolute read is never the right observation.

  **Retracted 2026-08-12.** The counter is thread-local, not process-wide. The
  delta requirement stands, but for a different reason: a single test process
  performs several resolutions on one thread, so an absolute read stops meaning
  anything the moment a second resolution enters it.
- Delete `cache_root::resolve` and re-home its unit tests onto `candidate` and
  `verify_writable`, so that every assertion they make is discharged by a named
  test after the change.
- Do not memoise the probe result. The invariant is to be established by
  structure and asserted by test, not by caching across calls — a process-wide
  cache would also change behaviour for the launcher's concurrent-first-use
  tests, which deliberately resolve from more than one thread.
- Take the warm-dispatch latency measurement 0169's Phase 10 deferred,
  inheriting its definition rather than inventing a new one: warm-call latency
  G against the shell baseline B on one darwin host in one session, both figures
  recorded, gated on `G ≤ 1.1 × B`. This is release-gated (see Dependencies) and
  is the only part of this item that cannot be started immediately.
- Land the work in this order: settle the counting seam, then the invariant
  test, then the deletion of `cache_root::resolve`. The deletion goes last
  because its four unit tests are themselves probe call sites; until they are
  gone or re-homed, they can run concurrently with the counting tests in the
  same process, which is what the delta convention and the isolation
  precondition below exist to survive.

  **Retracted 2026-08-12.** The stated reason is wrong twice. The `cli/`
  workspace runs under `cargo nextest`, which gives each test function its own
  process, so those unit tests never share a process with the counting tests;
  and the counter is thread-local, so it would not matter if they did. The
  ordering is kept — it is the right TDD sequence — but nothing depends on it
  for soundness.
- Before starting, re-confirm the two premises this scope rests on:
  `cache_root::resolve` still has no production caller, and no branch or loop in
  `FetchVerifyCacheResolver::resolve` reaches `fetch_verify_store` more than once
  per call. If either has changed, re-scope rather than proceed.

Every count criterion below is captured with **no other probe in flight in the
same process** — each counting test runs in its own test process, or is
serialised against the concurrent-first-use tests and the `cache_root` unit
tests. Without that isolation a delta of 2 is ambiguous between a regression and
cross-test interference.

**Retracted 2026-08-12.** This precondition is withdrawn rather than satisfied.
A thread-local counter makes cross-test interference impossible — every
assertion reads the count from the same thread that drove the calls — so no
runner precondition, `nextest.toml` test-group or serialisation is needed, and
none was built. The delivered assertions are sound under `cargo test` and
`cargo nextest` alike.

## Acceptance Criteria

- [ ] Given an empty cache directory and a stubbed fetcher serving a valid
      asset, when a sub-binary is resolved, then the probe count delta across
      that single `FetchVerifyCacheResolver::resolve` call is exactly 1.
- [ ] Given a cache pre-populated by the fixture writing a verified binary
      directly to disk (not by a prior resolution), when a sub-binary is
      resolved and re-verification succeeds, then the probe count delta across
      that single `FetchVerifyCacheResolver::resolve` call is exactly 0.
- [ ] Given a cached binary whose re-verification fails by a test-only failing
      verifier (never by filesystem permissions on the cache root), when the
      stubbed fetcher serves a valid asset and the refetch **succeeds**, then
      the probe count delta across that single call is exactly 1.
- [ ] Given a cached binary whose re-verification fails by the same test-only
      failing verifier, when the stubbed fetcher fails and resolution ends in
      `CorruptCacheAndRefetchFailed`, then the probe count delta across that
      single call is exactly 1.
- [ ] Given two successive cold-miss resolutions within one process, with the
      cache directory emptied between them, when both complete, then the probe
      count increments once per resolution — total 2. This is the criterion that
      fails under memoisation.
- [ ] With a second `verify_writable` call deliberately introduced into
      `fetch_verify_store`, the cold-miss, both refetch and the two-resolution
      criteria go red with the cold-miss delta observed as exactly 2, while the
      warm-hit criterion stays green; the mutation is then reverted. Without
      this the guard cannot be shown to guard anything, since the invariant
      already holds.
- [ ] `verify_writable` has exactly one production call site,
      `fetch_verify_store`, confirmed by a recorded search of the crate.
- [ ] `cache_root::resolve` is absent from the crate, and each of the four
      assertions its unit tests made — unset plugin root, override honoured,
      writable plugin root, read-only root — is discharged by a named test
      against `candidate` or `verify_writable`. The read-only case may be
      discharged by the existing `verify_writable_rejects_a_read_only_directory`
      rather than a re-homed copy.
- [ ] The two pick-up premises were re-confirmed before work began, and the
      confirmation recorded.
- [ ] Warm-call latency G and shell baseline B are both recorded from one darwin
      host in one session, with `G ≤ 1.1 × B`. Blocked until a signed
      `accelerator-vcs` release asset exists; see Dependencies.
- [ ] The mutation command and its output, the crate search, the old-test →
      discharging-test mapping, the pick-up confirmation and the latency figures
      are all recorded in the implementation plan's Validation Results.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Open Questions

- Where should the probe counter live? **Default if unresolved**: expose the
  per-process `SEQUENCE` atomic already inside `probe_writable_and_executable`
  as a single test-only accessor, read as a delta around the call under test —
  which is what makes a process-wide counter sufficient for a per-resolution
  assertion. The accessor is the whole of the permitted public-surface growth;
  anything wider, including injecting the probe behind a port the resolver
  holds, is **out of scope** and becomes its own work item.

  **Resolved 2026-08-12, against the default.** `SEQUENCE` cannot count
  invocations (see the retraction in the summary above). The counter is a new
  thread-local `PROBE_ATTEMPTS` incremented as the first statement of
  `verify_writable`, read through a single `pub fn probe_attempts()`. Public
  surface growth is still one function, as the default permitted.
- Does a seam for injecting a re-verification failure already exist in
  `cli/launcher/tests/resolution.rs`? The stubbed fetcher and the read-only-root
  fixtures do; a failing *verifier* is assumed by two criteria above and has not
  been confirmed. If it does not exist, building it is a prerequisite inside
  this item.

  **Resolved 2026-08-12.** A seam exists in a different shape, and no verifier
  port is built. `resolution.rs` already produces both refetch outcomes by
  poisoning the cached *bytes*, which works because the expected sha256 is
  parsed out of the cache filename rather than recomputed. Byte poisoning is
  not "filesystem permissions on the cache root", so it discharges what
  acceptance criteria 3 and 4 ask for; a validator reading those criteria
  literally should look for byte poisoning, not for a failing verifier.

## Dependencies

- **Delivered by 0169.** 0169's Phase 5 implemented this item's original
  Requirements in full, pulled forward because its own Phase 10 warm-call
  latency gate could not be measured while every warm `vcs guard` dispatch paid
  the probe cost.
- **This item now owns the deferred latency measurement.** 0169 is closed
  (`status: done`) with its Phase 10 criterion unchecked and B, G, ratio,
  payload, fixture and host all recorded as pending, so no other open item
  carries it. Taking it requires the launcher resolving against a real,
  minisign-signed `accelerator-vcs` release asset, which does not exist
  pre-release; that release cut and its signing key are owned by whoever
  performs epic-0136 releases. **This item cannot close before that release.**
- **The latency gate has co-requisites beyond this item.** 0169's hand-off notes
  identify 0191 (batching the two verify-shim hashes into one invocation, ~2.5
  ms — "essentially this story's whole shortfall") as the cheapest remaining
  lever, alongside the backend-dependent `sha256_file` residual 0186
  deliberately retained. `G ≤ 1.1 × B` may not be reachable without 0191.
- **Relates to 0186**, which established the pattern, the diagnostic shape and
  the measurement method on the shell side.
- **Relates to 0164**, which established the fetch-verify-cache resolver and
  the probe itself. The at-most-once property is a refinement of the
  cache-resolution contract 0164 defined.
- **Blocks: none.** 0169 and 0186 both still carry prose naming the launcher
  probe as the dominant unaddressed cost gating 0169's latency threshold. That
  framing is stale — 0169's own Phase 5 absorbed the fix — and nothing now waits
  on this item.
- **Parent**: epic 0136.

## Assumptions

- A launcher process serves a single dispatch and performs exactly one
  resolution, so per-process and per-resolution probe counts coincide in
  production. The criteria assert a per-resolution delta regardless, which holds
  in a multi-resolution test process too.
- 0169's Phase 10 definition of the gate (`G ≤ 1.1 × B`, one darwin host, one
  session) is still the right shape for the measurement inherited here. If the
  epic has since revised the threshold, this item follows the epic.

## Technical Notes

- Production call site to protect:
  `cli/launcher/src/launch/outbound/resolve/mod.rs:141`.
- The four unit tests bound to the doomed `cache_root::resolve`:
  `unset_plugin_root_with_no_override_is_a_named_error`,
  `a_writable_plugin_root_is_used`,
  `a_read_only_plugin_root_with_no_override_is_a_named_error`, and
  `an_override_is_honoured`. The read-only case is substantially a
  `verify_writable` test and already has a direct equivalent in
  `verify_writable_rejects_a_read_only_directory`.
- `verify_writable` delegates to `probe_writable_and_executable`, one call to
  one increment, which carries the per-process `SEQUENCE` atomic alongside the
  PID in the probe filename — the atomic exists because the concurrent-first-use
  tests resolve from multiple threads and would otherwise collide on one path.
  That atomic is the counter the default seam exposes.

  **Retracted 2026-08-12.** The last sentence is false: `SEQUENCE` is not the
  counter exposed, because its increment sits after the `create_dir_all` early
  return. It keeps its filename-uniqueness role, unchanged and untouched, and a
  separate thread-local `PROBE_ATTEMPTS` carries the invocation count.
- The integration tests in `cli/launcher/tests/resolution.rs` are a separate
  crate, so the counter accessor must be public — the cost to weigh when
  settling the Open Question.

## Drafting Notes

- The original before/after measurement criterion was dropped, then restored on
  the author's instruction after review pass 2 established that 0169 is closed
  with its Phase 10 gate unmeasured, leaving the obligation with no open owner.
  It returns in 0169's own form (`G ≤ 1.1 × B`) rather than the original
  `after ≤ 0.5 × before`, because the pre-fix "before" no longer exists in the
  tree. The consequence, accepted deliberately: this item cannot close until the
  epic-0136 release cut produces a signed `accelerator-vcs` asset.
- The title still names only the probe guarantee, not the inherited latency
  measurement. It was already changed twice — once because the original asserted
  something no longer true of the code, once from "Once-Per-Dispatch" to
  "At-Most-Once" because the dominant production case is zero probes — and was
  left alone this time rather than churned again. The filename slug still reads
  `once-per-dispatch`.
- "At most once per process" was reinterpreted as a per-resolution delta. The
  two coincide in production, and a delta is the only form observable through a
  process-wide counter in a multi-resolution test process.
- Priority stays low. The guard work is small, and the measurement half is
  release-gated rather than urgent — but the item now carries an epic-level
  obligation, so raising it is a reasonable challenge.
- Memoisation was ruled out in favour of a test on the author's instruction, and
  carries its own criterion because every *other* per-path count criterion
  passes under a memoising implementation.
- The stale "dominant unaddressed cost" framing inside 0169 and 0186 was
  retracted in place on 2026-08-11. Both retractions are dated notes appended
  beside the original text rather than edits to it, so each document still
  records what was believed when it was written.

## References

- `meta/reviews/work/0189-once-per-dispatch-cache-root-probe-guarantee-review-1.md`
  — five-lens review, verdict REVISE across two passes, which drove this revision
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — delivered the split;
  closed with the Phase 10 latency gate unmeasured
- `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` — Phase 5,
  and Phase 10's deferred latency gate and hand-off record
- `meta/work/0191-batch-the-two-shim-hashes-into-one-invocation.md` —
  co-requisite for the latency gate
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — the
  shell-side change, its measurement method and its diagnostic shape
- `meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md`
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
- `cli/launcher/src/launch/outbound/resolve/mod.rs` —
  `FetchVerifyCacheResolver::fetch_verify_store`
- `cli/launcher/tests/resolution.rs` — the two already-satisfied criteria
- `docs/internals.md` — "Offline, mirrored and read-only installs"
