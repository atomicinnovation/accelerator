---
type: adr
id: "ADR-0064"
title: "Producer-Signed Tree Attestation over a Compiled-In Digest"
date: "2026-08-20T00:00:00+00:00"
author: Toby Clemson
producer: create-adr
status: accepted
supersedes: ["adr:ADR-0061"]
relates_to: ["adr:ADR-0059", "adr:ADR-0060", "adr:ADR-0063",
  "work-item:0196", "work-item:0214"]
tags: [architecture, distribution, integrity, manifest, launcher, cache,
  design]
last_updated: "2026-08-20T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# ADR-0064: Producer-Signed Tree Attestation over a Compiled-In Digest

**Date**: 2026-08-20
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0061 settled how a launcher binds a cache hit on a tree artifact to the
release key: address by content digest and generation, verify a signed
attestation on every hit, and gate reclamation on a shared `flock` lease. Those
decisions hold and are restated here. Two of the mechanisms it named for the
attestation and the pointer could not be built as written, and this ADR replaces
ADR-0061 rather than annotating it because the first contradicts an assertion in
its Decision section.

**The attestation cannot reuse the manifest's signature.** ADR-0061 says the
attestation "carries the manifest's minisign signature over the archive digest,
and `locate` verifies it under the embedded release key on every hit". The
manifest signature signs the archive *file's bytes*, and the launcher deletes the
archive after extraction — so nothing remains on disk for that signature to
verify against, and a signature the consumer cannot check is not a control.
Worse, a signature over a bare digest binds neither the artifact's identity nor
its platform: any process able to write the trees directory could repoint at
another artifact's or another platform's generation whose signature is entirely
valid, since identity and platform would live only in an unsigned pointer
filename.

**A per-release pointer is the wrong key.** ADR-0061 addresses generations "with
the release version held in a separate pointer file naming the current
generation". Signing the release version was considered in an intermediate
revision and is impossible: the job that assembles the archives runs before the
release version is chosen and its one archive set serves two cuts, so the
producer cannot know the value; and an older generation's document would name the
older version, making cross-version adoption on a shared root impossible. Keying
the pointer on the release version has three costs even unsigned — it accumulates
one entry per plugin version on a shared root (so `prune` reclaims nothing),
forces a manifest load before the reuse scan can find an already-present tree (so
a zero-byte upgrade fails offline), and needs a field the producer cannot supply.

Work-item:0214 settled the tree-artifact mechanisms against prototypes; this ADR
records the attestation and pointer shapes that survived building them.

## Decision Drivers

- **A hit must be bound to the release key on identity and content**, not on a
  signature the consumer cannot check or one that binds neither.
- **Rollback to an older generation must be refused** without a signed release
  version to compare against.
- **Cross-version adoption and offline resolution must fall out of the same
  mechanism** — a populated cache must resolve a present tree with the release
  host unreachable, and a newer plugin version must adopt an unchanged tree.
- **The producer must be able to compute every signed field** in the job that
  assembles, which runs before the release version exists.
- **The table's integrity must follow from a signature**, not from a digest the
  consumer writes locally.

## Considered Options

1. **A producer-signed attestation binding identity and content, a digest-keyed
   pointer, and a compiled-in expected digest** — a standalone signed document
   naming the artifact, platform and archive digest; the pointer keyed on the
   digest, not the release version; rollback refused by comparing the attested
   digest against one baked into the launcher at build time.
2. **ADR-0061's shape unchanged** — the manifest signature reused as the
   attestation and a per-release pointer.
3. **Sign the release version into the attestation** — bind the full tuple
   including the release version, so rollback is refused by comparing versions.

## Decision

We will attest a tree with a **producer-signed document that binds artifact
identity and content only**, key the **pointer on the content digest**, and
refuse rollback by comparing the attested digest against a **digest compiled into
the launcher**. Everything else in ADR-0061 — content-and-generation addressing,
the shared `flock` lease, the age backstop for lease-less generations, the layout
version, the read-only seal as a consistency check, and the per-exec exemption —
carries forward unchanged.

- **The attestation is a standalone producer-signed document**, not the manifest
  signature reused. It is signed as a `.sealed` document with a detached
  `.sealed.sig`, and `verified` checks the signature over the raw bytes under the
  embedded release key *before* the document is parsed, so a malformed document
  cannot be interpreted on the strength of arriving beside a signature.
- **It binds the tuple `{attestation_format_version, artifact, platform,
  archive_sha256, uncompressed_size, entry_count, table_sha256}`** — identity
  (`artifact`, `platform`), content (`archive_sha256`), and the shape and table
  digest the extractor needs. `matches` checks the format version, artifact and
  platform against what is being resolved, so a valid signature over another
  artifact's or platform's document is refused by field, not just by digest.
- **It deliberately binds neither the release version nor the layout version.**
  The release version is unknowable in the assembling job and would make
  cross-version adoption impossible. The layout version is consumer-owned policy:
  a signed copy could never be rewritten by the launcher that owns it, so a
  policy bump would miss, re-materialise, fetch the same producer document still
  carrying the old value, and miss again.
- **The pointer is digest-keyed**, `<name>-<platform>-<digest>`, naming the
  generation currently published for that digest — one pointer per distinct
  artifact version rather than one per release. This gives cross-version adoption
  and offline resolution from the reuse scan alone, and lets `prune` reclaim a
  root that two plugin versions share.
- **Rollback is refused against a compiled-in digest.** The launcher is built
  with the expected `archive_sha256` for each artifact baked in from `pins.toml`
  (via `build.rs`); a validly-signed attestation whose digest is any value other
  than the compiled-in one is an `UnexpectedDigest` refusal. This is what an
  unsigned digest-keyed pointer alone cannot provide — the anchor of trust is the
  binary the user is running, not a file in a cache root any writer can forge.
- **The `.files` table ships inside the archive**, as its first member, so the
  archive signature covers it and its digest is the attested `table_sha256`. The
  table is not a separate locally-written sidecar whose integrity the launcher
  would have to establish on its own.

Option 2 was rejected because it is not implementable: the archive is deleted
after extraction, leaving the manifest signature nothing to verify against, and a
digest-only signature binds neither identity nor platform.

Option 3 was rejected because the producer cannot know the release version when
it signs, and binding it forecloses cross-version adoption — the older
generation's document would name the older version.

## Consequences

### Positive

- A hit is bound to the release key on identity and content through a document
  the consumer can actually verify, closing the fabricated-tree and
  wrong-artifact/wrong-platform repoint gaps in one check.
- Rollback to a superseded generation is refused with no signed release version,
  because the trust anchor is the compiled-in digest in the running binary.
- Cross-version adoption, offline resolution and a `prune`-able shared root all
  fall out of digest keying, needing no field the producer cannot compute.
- The table's integrity follows from the archive signature rather than from a
  locally-written digest.

### Negative

- The producer signs a second document per artifact (the attestation, beside the
  archive `.minisig`), and the launcher carries a compiled-in digest map that a
  pin bump must regenerate — a build-time coupling ADR-0061's manifest-signature
  reuse would not have had.
- A tree modified in place after materialisation is still executed without
  detection, unchanged from ADR-0061: the hit path never re-hashes.

### Neutral

- The attestation is one more piece of per-tree local state with its own format
  version, though it replaces rather than adds to ADR-0061's reused-signature
  scheme.
- Whether two plugin versions ever actually share a root, and who reclaims it,
  remain ADR-0063's questions; digest keying only makes either answer safe.

## References

- ADR-0061 (signed content-addressed tree generations) — superseded by this ADR;
  its content-and-generation addressing, `flock` lease, layout version, seal and
  per-exec exemption are restated here unchanged. This ADR replaces only its
  attestation shape and its per-release pointer.
- ADR-0059 (build-time assembly of vendored browser artifacts), ADR-0060
  (launcher-resolved tree artifacts), ADR-0063 (plugin-version-scoped artifact
  cache)
- `meta/work/0214-settle-the-vendored-runtime-tree-artifact-mechanisms.md` —
  settled the attestation and pointer shapes against prototypes
- `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md` — the tooling
  work these artifacts serve
