---
type: "adr"
id: "ADR-0075"
title: "Digest-Pinned Resolution Chain"
date: "2026-10-27T18:20:57+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "proposed"
parent: "work-item:PP-947"
relates_to: ["adr:ADR-0046", "adr:ADR-0054", "adr:ADR-0063", "adr:ADR-0064", "adr:ADR-0072", "work-item:PP-944"]
tags: ["architecture", "distribution", "integrity", "launcher", "cache", "release-pipeline", "security"]
last_updated: "2026-10-27T19:12:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0075: Digest-Pinned Resolution Chain

**Date**: 2026-10-27
**Status**: Proposed
**Author**: Toby Clemson

## Context

The runtime cache holds three kinds of artifact, and each is accepted on a
different basis:

- **Tree artifacts** are accepted only at the `archive_sha256` compiled into
  the launcher from `pins.toml` (ADR-0064), so a superseded tree is never
  looked for.
- **Sub-binaries** are accepted if a minisign signature over their bytes
  verifies under the embedded release key (ADR-0046). On a cache hit the
  expected SHA-256 and the version come from the cache filename
  `<name>-<version>-<sha256>`, which nothing authenticates.
- **The launcher** is accepted by `bin/accelerator` in the same way:
  `accelerator-launcher-<version>-<platform>` plus a `.minisig` checked by the
  vendored `accelerator-verify` shim.

Verification never reads the minisign trusted comment, so a signature binds no
name, version or platform. Anyone who can write a cache root can put any binary
the release key ever signed under the current release's filename. That
includes an older launcher, which brings its own older tree pins. The plugin
tree and the cache root share an owner by default, which limits the exposure,
but not for a relocated `ACCELERATOR_CACHE_DIR` (ADR-0072), a shared
multi-user root, or a pre-seeded image volume. Those are the cases that seeding
a cache from a staged release directory makes routine.

The release public key lives in the plugin tree, so today's signatures were
never stronger than the plugin tree's own integrity. Their essential job is to
authenticate a list of trusted hashes, the manifest, that arrives from the
release host. Every artifact the launcher resolves is built in the same release
as the launcher and plugin tree, or before it in the case of trees, and the
version is fixed by `plugin.json`; nothing in our stack asks for the latest
release.

Trees cannot carry a release version: the assembling job runs before the
version is chosen, and one archive set serves two cuts (ADR-0064). Sub-binaries
and the launcher are built from the same commit as each other. Today one
`cargo zigbuild` per target builds the whole `cli/` workspace, so no sub-binary
digest exists when the launcher compiles.

## Decision Drivers

- Refuse rollback and substitution at every layer, including on cache hits.
- Bound every download before its hash can be checked.
- Use one acceptance rule a reviewer can check, the same for the fetched and
  staged routes.
- Keep trees version-free, as ADR-0064 requires.
- Add no new root of trust beyond the installed plugin tree, and minimise
  what a leaked secret can achieve.
- Bound the release-time cost and leave development builds no weaker path to
  downgrade into.

## Considered Options

1. **Digest pins at every layer.** The plugin tree pins the launcher's digest;
   the launcher pins every sub-binary's and every tree archive's digest and
   size. This is the lockfile model (Homebrew bottles, `Cargo.lock`, `go.sum`,
   `image@sha256:` references).
2. **Signed artifacts with bound trusted comments.** Signing writes `artifact`,
   `platform`, `version` and `size` into each minisign trusted comment; the
   shim and the launcher require the comment to match the artifact being
   resolved and their own version. Tree pins move to a committed file read at
   run time, so the launcher embeds no artifact digests.
3. **Signed manifest on every path.** Keep the manifest as the signed list of
   trusted hashes, bound to `expected_version`, and cache it so cache hits are
   checked against it too.
4. **Status quo.** Signatures for sub-binaries and the launcher, pins for
   trees.

## Decision

We will resolve every executable artifact through a **digest-pinned chain
rooted in the installed plugin tree**: an artifact is accepted if, and only
if, its SHA-256 equals the digest pinned by the layer above it.

- **The plugin tree pins the launcher.** The release writes a per-platform
  launcher digest file into the release commit. `bin/accelerator` accepts a
  cached, fetched or staged launcher only at that digest. Only release commits
  may change the file.
- **The launcher pins sub-binaries and trees.** The release builds every
  sub-binary, including the visualiser server, for every target, then builds
  each launcher with the full `(artifact, platform) → (sha256, size)` map for
  all four platforms embedded. Tree archive pins, now including
  `archive_size`, continue to come from `pins.toml`.
- **Every pin carries a size, and the size bounds the download.** A pin with
  no size, or a size of zero, fails the build.
- **The pin is the only acceptance rule** for the launcher and sub-binaries,
  on fetch, on every cache hit, and for staged copies. The cache filename is
  never the source of an expected digest.
- **The release key remains a trust root for tree attestations only.** The
  attestation binds `table_sha256`, the extracted tree's content, to the
  pinned archive digest, and its `.sealed.sig` is verified on every tree cache
  hit.
- **Signatures leave launcher and sub-binary resolution but stay published.**
  The release keeps signing every asset and verifies the published signatures
  and build provenance after publishing, so artifacts remain verifiable on
  their own. The vendored verify shim, whose only job was the launcher's
  signature, is retired.
- **Resolution never reads the release manifest.** It becomes publication
  metadata, and version coherence is asserted at release time only.
- **Unpinned builds refuse release-host resolution.** Unpinned is a
  compile-time mode with no runtime switch. A launcher built without
  sub-binary pins resolves only through the `ACCELERATOR_<SUB>_BIN` and
  `ACCELERATOR_TREE_*` overrides; a checkout whose launcher digest file is
  missing or malformed refuses rather than falls back. A release containing an
  unpinned launcher fails before signing.

We chose option 1 because all our artifacts are known when the verifier is
built, which is the condition under which pins are sufficient and signatures
add only a second check beneath the same root. It closes rollback,
substitution and mixing builds within one version by construction, and it
leaves no secret whose theft yields code execution.

Option 2 was rejected because the CI signing key would remain the single
secret that lets an attacker produce an acceptable artifact for any version,
with no revocation for installs that never update. Because the pipeline bumps,
builds and signs before it pushes, a run that fails after signing and is rerun
leaves two validly signed builds for one version, so mixing builds within a
version stays possible unless the pipeline reserves versions before signing.
It also grows the root-of-trust shim into a comment parser and still needs a
pin for trees. Option 3 was rejected because it keeps the manifest, and
therefore two network requests or two extra staged files, on the resolution
path, and anchors sub-binaries differently from trees. Option 4 leaves the
substitution gap open.

## Consequences

### Positive

- An older or differently named signed artifact planted in a cache root is
  refused at every layer, on cache hits as well as fetches.
- A leaked release key no longer allows code execution through the launcher
  or sub-binaries; it can forge only tree attestations, which still needs
  cache write access.
- One rule, "hash and size match the pin", covers the launcher, sub-binaries
  and tree archives on the fetched and staged routes alike.
- Every download is bounded, closing the unbounded sub-binary download that
  exists today.
- A cold dispatch drops the two manifest requests, and a staged directory
  needs no signatures except the tree attestations'.

### Negative

- Every artifact must exist when the launcher is built. Sub-binaries released
  on their own cadence, hotfixes without a launcher rebuild, and third-party
  sub-binaries would need signed versioned metadata (TUF or similar) and a new
  ADR.
- The release build becomes two-stage: all sub-binaries for all targets, then
  all launchers. Measured locally, the second pass recompiles only the
  launcher crate, at about 25 s wall per target. It must rerun the same
  full-workspace invocation, or feature unification may rebuild shared crates.
- Every launcher embeds digests for all four platforms, so its bytes change
  whenever any sub-binary on any platform changes.
- The launcher digest file changes in every release, so a malicious edit is
  less conspicuous than a public-key change would be. The release-commit-only
  guard and the post-release comparison against published assets and their
  provenance restore that signal.
- Development from a checkout loses release-host resolution, so contributors
  must use the overrides.
- Pinning protects cache content at rest, not against a concurrent writer
  replacing a file between its check and its exec. A seeded read-only volume
  or a root owned by one user is safe; a root other users can write while
  sessions run needs the further measures 0340 owns.

### Neutral

- This replaces ADR-0046's minisign integrity layer for the launcher and
  sub-binaries. ADR-0046's static-linking, cross-compilation and release
  orchestration decisions stand.
- ADR-0064's tree attestation, pointer and compiled-in tree digest are
  unchanged; this decision extends its anchor to the rest of the chain and
  adds the archive size.
- Retiring the verify shim does not affect the cache directory's
  exec-capability probe: the cold-path probe is a separate script, and a warm
  start proves the capability by executing the launcher.
- Neither model protects the plugin's own update channel against freeze or
  rollback; that belongs to Claude Code's plugin marketplace.

## References

- `meta/decisions/ADR-0046-zero-setup-static-binary-distribution.md`
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
- `meta/decisions/ADR-0063-plugin-version-scoped-artifact-cache.md`
- `meta/decisions/ADR-0064-producer-signed-tree-attestation-over-a-compiled-in-digest.md`
- `meta/decisions/ADR-0072-claim-gated-automatic-prune-for-relocated-cache-roots.md`
- `meta/work/PP-947-pinned-seedable-runtime-cache.md` and its children 0356–0360,
  0222; `meta/work/PP-944-shared-cache-roots-on-multi-user-systems.md`
- The Update Framework specification (https://theupdateframework.github.io/specification/latest/)
  for the threat list the options were compared against
