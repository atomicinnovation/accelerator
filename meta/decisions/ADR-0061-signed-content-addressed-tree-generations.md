---
type: adr
id: "ADR-0061"
title: "Signed Content-Addressed Tree Generations"
date: "2026-08-17T10:53:53+00:00"
author: Toby Clemson
producer: create-adr
status: superseded
superseded_by: ADR-0064
supersedes: ["adr:ADR-0060"]
relates_to: ["adr:ADR-0046", "adr:ADR-0054", "adr:ADR-0057", "adr:ADR-0059",
  "work-item:0164", "work-item:0186", "work-item:0196", "work-item:0214"]
tags: [architecture, distribution, integrity, manifest, launcher, cache,
  design]
last_updated: "2026-08-20T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# ADR-0061: Signed Content-Addressed Tree Generations

**Date**: 2026-08-17
**Status**: Superseded (by ADR-0064, on the attestation shape and the pointer key)
**Author**: Toby Clemson

## Context

ADR-0059 introduces two directory-tree artifacts per platform — the assembled
driver bundle and the browser. ADR-0060 decided how the launcher resolves them:
a new additive `artifacts` map, verification of the fetched archive before
extraction, atomic materialisation by directory rename, a read-only seal, and
exemption from the per-exec re-verification single-file sub-binaries keep. Those
decisions hold and are restated here.

Three of its mechanisms did not survive contact with evidence. Work-item:0214
settled each against prototypes, and this ADR replaces ADR-0060 rather than
annotating it, because one of the three contradicts an assertion in its Decision
section.

**Addressing.** ADR-0060 addresses trees "by release version and digest", by
analogy with `{name}-{version}-{sha256}` for single files. The analogy breaks in
two places. A directory has no atomic replace, so a name that can collide with an
existing target forces a rename-time branch distinguishing a concurrent winner
from a crash leftover, and leaves a repair nowhere to build a complete replacement
beside a tree a daemon is still reading. Separately, keying on the release version
rather than the content digest means two plugin versions sharing a cache root
cannot reuse an unchanged artifact — the driver and browser change only when the
pinned `playwright-core` does. On the default per-version root the release version
is constant, so the two keyings are equivalent there and it is the generation that
does the work; digest keying is what makes the relocated root ADR-0063 keeps
worth having.

**What a cache hit proves.** ADR-0060's sentinel records the verified digest and
the release version. Nothing in it is bound to the release key, so an attestation
whose digest matches the digest in its own directory name is self-referential:
any process able to write the cache root can fabricate a tree, a matching
sentinel and a pointer. ADR-0060's threat model rests on the cache living "under
the user's own home directory", and `ACCELERATOR_CACHE_DIR` — which the
distribution work actively recommends relocating — breaks that premise.
Work-item:0205 recorded the same distinction for single files: a cache-hit
digest check is a corruption check, not a trust check.

**Who holds a tree.** ADR-0060 argues repair is safe against a live daemon
because "a process already executing from the old tree keeps its inodes". That
holds for files already open, but not for those Chromium and the driver open
later — locale packs, `.pak` resources, `icudtl.dat`, lazily-`require`d modules.
Reclaiming a superseded generation under a running daemon can therefore remove a
file it has not opened yet. No in-use signal existed to gate on.

Where the cache lives and who empties it bears on all three of these, and is
decided separately by ADR-0063; this ADR assumes only that some root exists and
that trees within it are reclaimed by someone.

The cost envelope is unchanged from ADR-0060: a crawl is bounded at 50 routes and
five minutes and makes 100–200 executor invocations, each a fresh launcher
process, against work-item:0186's ~30ms warm-bootstrap target. Any per-hit work
is multiplied by that count, which is why re-hashing 294MB per exec was rejected
and remains rejected.

## Decision Drivers

- **A hit must be bound to the release key**, not to local state that the same
  writer who fabricated the tree could fabricate too.
- **A hit must stay local and offline** — 100–200 per crawl, and a populated
  cache must work with the release host unreachable.
- **Reclamation must be safe against a live reader**, since that is what the
  repair path rests on.
- **A root shared by two plugin versions should reuse an unchanged artifact**, or
  the relocated-root escape ADR-0063 keeps buys nothing.
- **Claim only what the check delivers** — carried forward from ADR-0060.
- **Additive manifest change only**, and `binaries` keeps one meaning.

## Considered Options

1. **Content-addressed generations, a signed attestation, and an `flock`
   lease** — address by digest, platform and generation with the release version
   in a separate pointer; carry the release signature over the archive digest
   inside the attestation and verify it on every hit; gate reclamation on a
   kernel-held lease.
2. **ADR-0060's shape unchanged** — version-and-digest addressing, an unsigned
   sentinel, and inode retention as the whole repair-safety story.
3. **Manifest-authoritative hits** — load and verify `manifest.json` on each
   hit so the digest comes from a signed document rather than local state.
4. **Per-exec re-hash of the tree** — ADR-0060's Option 2, restated because the
   trust gap in option 2 is what it was originally proposed to close.

## Decision

We will address tree artifacts by **content digest, platform and generation**,
bind each cache hit to the release key through a **signed attestation**, and gate
reclamation on a **shared `flock` lease**. Everything else in ADR-0060 carries
forward unchanged.

- **Addressing is `<name>-<platform>-<sha256>-<generation>`**, with the release
  version held in a separate pointer file naming the current generation. Within
  any one root an unchanged artifact is one tree rather than one per release;
  whether two plugin versions ever share a root is ADR-0063's question. Because
  every materialisation picks a fresh generation, `rename(2)` never lands on an
  existing target, so there is no already-present case to disambiguate and a
  repair can build a complete replacement beside a tree a daemon is still
  reading.
- **The attestation is signed.** It carries the manifest's minisign signature
  over the archive digest, and `locate` verifies it under the embedded release
  key on every hit. Measured at 51.7µs median cold-process — 0.17% of the
  ~30ms warm budget, 0.35% for both trees — so the hit path stays two small
  reads, two stats and one Ed25519 verify. No manifest load, no network.
- **The signature anchors provenance, not tree contents.** It prevents a
  fabricated tree from being accepted as release-provenanced. It does **not**
  detect modification of a file inside an already-materialised tree, because the
  hit path never re-hashes — that remains true and is stated as a negative
  below.
- **The read-only seal is a consistency check, not a discriminator.** `tar` and
  `unzip` both preserve `0444`/`0555`, and these artifacts are `tar.gz`, so
  extraction into the cache root reproduces the seal exactly. It is retained
  because the `stat` already happens and it therefore costs nothing, but it
  deters accident rather than establishing origin.
- **In-use is a shared `flock` lease**, a sidecar beside each generation rather
  than a file within it — the seal would otherwise make it read-only for the
  dispatches that must take it, and `verify` would report it as an entry absent
  from the `.files` table. It is opened by the
  launcher with `FD_CLOEXEC` cleared so the open file description is inherited
  through the `exec` into the detached daemon. The reaper and `prune` probe for
  an exclusive lock; contention means a live holder. The kernel is the liveness
  oracle, so a crashed holder releases with no cleanup code and no stale state
  is reachable. A shared lease admits concurrent crawls and stays held until the
  last holder exits.
- **An age backstop applies only to generations carrying no lease**, i.e. those
  left by a launcher predating this decision.
- **The generation directory carries a layout version.** Content addressing means
  a newer launcher can adopt an older launcher's tree wherever the two share a
  root, and extraction and sealing policy is launcher-version-specific yet not
  covered by the archive digest. The layout version makes a newer launcher refuse
  an older layout and re-materialise rather than silently inherit a superseded
  policy, under the same "unknown additive fields ignored, higher version refused"
  discipline `manifest.rs:1-3` documents. Whether two launchers ever share a root
  is ADR-0063's question, not this one's; the layout version is what makes either
  answer safe.

Option 2 was rejected on two counts, neither of them refetch cost — on a
per-version root its naming and ours re-materialise equally often. It has no
generation, so a rename can land on an existing target and a repair has nowhere to
build a complete replacement beside a tree in use; and its sentinel makes a hit
provable only against state co-located with what it attests.

Option 3 was rejected on cost and availability. `load_manifest` is two HTTPS GETs
plus a signature verification; at 100–200 dispatches per crawl that is a network
round trip on the warm path, and it makes a populated cache useless offline.
Storing the signature locally buys the same binding for 51.7µs.

Option 4 was rejected on ADR-0060's own measurements — 16–33 seconds per crawl
re-hashing immutable bytes, roughly six times the warm-path target on every
invocation — and the signed attestation closes the provenance gap it was meant
to address at a fraction of the cost.

A pid-and-start-time liveness gate was considered for the lease and rejected on
recorded experience: `meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md`
documents this daemon shutting down seconds after every bootstrap because its
owner pid was an ephemeral shell.

## Consequences

### Positive

- A hit is bound to the release key for the first time, so a fabricated tree in
  a relocated cache root is refused rather than executed.
- Content addressing makes an unchanged artifact one tree per root rather than one
  per release, which is what lets ADR-0063 choose a root freely rather than being
  forced into one by refetch cost.
- **Our** reclamation is safe against a live daemon on a signal the kernel
  maintains, rather than on an assumption about which files it has already opened.
  Reclamation by anything outside this codebase is ADR-0063's concern.
- Generations remove the concurrent-winner-versus-crash-leftover distinction at
  rename time entirely.

### Negative

- A tree modified in place after materialisation is still executed without
  detection. The signature covers the archive digest, not the extracted bytes,
  and the hit path deliberately never re-hashes. This is unchanged from ADR-0060
  and remains permanent rather than transitional.
- Generations accumulate within whichever root is in use, so this decision creates
  a reclamation obligation it does not itself discharge. ADR-0063 assigns it.
- The lease is a second lock discipline in the codebase with semantics
  deliberately inverted from the launcher lock, whose descriptor must *not* leak
  into the daemon. A reader of either has to know which is which.
- Three pieces of local state per tree — attestation, pointer, lease — each with
  its own failure mode, where ADR-0060 had one sentinel.

### Neutral

- The signed attestation narrows but does not remove ADR-0060's "two integrity
  models" cost: single files re-verify per exec, trees verify provenance per hit
  and contents never.
- Every piece of local state now sits beside the generation rather than within it —
  attestation, pointer and lease alike — so the seal covers only archive content
  and `verify` can treat any unexpected entry inside a generation as a discrepancy
  without carving out exceptions.
- Whether `prune` reports the abandoned legacy Playwright cache is an
  independent question this decision does not settle.

## References

- ADR-0060 (launcher-resolved tree artifacts) — superseded by this ADR; its
  additive-manifest, launcher-ownership, verify-before-extraction, atomic-rename
  and per-exec-exemption decisions are restated here unchanged
- ADR-0046 (zero-setup static binary distribution), ADR-0054 (git-style modular
  CLI), ADR-0057 (browser automation as a glibc-only capability), ADR-0059
  (build-time assembly of vendored browser artifacts)
- `meta/work/0214-settle-the-vendored-runtime-tree-artifact-mechanisms.md` —
  settled all three mechanisms against prototypes; carries the measurements
- `meta/work/0205-close-the-warm-dispatch-measurement-method.md` — the
  corruption-check-versus-trust-check distinction
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — the warm-path
  budget the 0.17% figure is measured against
- `meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md` — why
  the liveness gate is not a pid
- ADR-0063 (cache-root placement and tree reclamation) — decides which root trees
  live in and who reclaims them, the obligation this ADR creates but does not
  discharge
