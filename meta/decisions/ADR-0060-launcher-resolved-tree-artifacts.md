---
type: adr
id: "ADR-0060"
title: "Launcher-Resolved Tree Artifacts"
date: "2026-08-11T10:45:25+00:00"
author: Toby Clemson
producer: create-adr
status: accepted
relates_to: ["adr:ADR-0046", "adr:ADR-0054", "adr:ADR-0057", "adr:ADR-0059",
  "work-item:0164", "work-item:0186", "work-item:0196"]
tags: [architecture, distribution, integrity, manifest, launcher, cache,
  design]
last_updated: "2026-08-11T10:58:31+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# ADR-0060: Launcher-Resolved Tree Artifacts

**Date**: 2026-08-11
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0059 introduces two directory-tree artifacts per platform — the assembled
driver bundle and the browser. The launcher's distribution machinery was built
for single files, and three of its properties do not carry across.

The manifest keys `binaries` by dispatched token, one asset per platform. The
cache names each entry `{name}-{version}-{sha256}`, so the filename itself
asserts the digest and a hit resolves offline. Replacement renames a fresh
inode over a corrupt entry, which is what keeps a process mid-`exec` from
hitting `ETXTBSY`. And on every cache hit the resolver re-reads the file,
recomputes its digest, and re-verifies the minisign signature before exec
(`resolve/mod.rs:196`), self-healing by refetch when either fails. A tree has
no single file to name by hash, rename-by-inode has no directory equivalent,
and re-verification stops being free.

Measured on an Apple Silicon host with hardware-accelerated sha256 at roughly
2.5GB/s, against the revision `playwright-core` 1.55.1 names:

| Artifact | Size | Files | Hash cost |
|---|---|---|---|
| `chromium-1193` | 297MB | 327 | ~120ms |
| `chromium-headless-shell-1193` | 177MB | 14 | ~71ms |
| Driver bundle | ~117MB | ~490 | ~47ms |

A crawl is bounded at 50 routes and 5 minutes, and each route costs several
executor invocations, so a crawl makes on the order of 100–200 of them.
Re-verifying the driver and the browser on each would spend 16–33 seconds per
crawl re-hashing bytes that cannot have changed — 5–11% of the crawl's whole
budget, and roughly six times work-item:0186's ~30ms warm-bootstrap target on
every single invocation.

The threat model bounds what that spending buys. The cache lives under the
user's own home directory; anything able to write there already runs as the
user, and can equally modify the plugin, the shell profile, or anything on
`PATH`. Per-exec re-verification is defence in depth against accidental
corruption, truncated writes, and a narrow window between fetch and exec. It is
not a privilege boundary. Verify-at-materialisation is the established pattern
elsewhere: cargo checks a crate's checksum when it extracts, writes a
`.cargo-ok` marker, and thereafter builds against the extracted source without
re-hashing it.

One constraint shapes the manifest side. `manifest.json` is a single signed
document every deployed launcher consumes, and its schema gate rejects only
`schema_version` greater than the supported value, so a bump is a flag day for
every sub-binary at once. The parser was written for this: "Unknown additive
fields are ignored" is its opening line, and no structure denies unknown
fields.

## Decision Drivers

- **Additive change only** — a schema bump breaks every sub-binary, not just
  this one.
- **One key holder, one verify path** — ADR-0059 turns on the CLI having a
  single artifact trust primitive.
- **Per-invocation cost must fit the warm path** — work-item:0186 spent real
  effort getting there; a browser command should not undo it.
- **Claim only what the check delivers** — an integrity guarantee that a
  same-uid attacker can bypass should be described as what it is.
- **`binaries` should keep meaning one key, one executable.**

## Considered Options

1. **A new top-level `artifacts` map**, resolved by the launcher, verified at
   materialisation and sealed read-only thereafter.
2. **Extend `binaries` with a kind discriminator** and keep per-exec
   re-verification for trees.
3. **Have `accelerator-design` resolve its own trees**, leaving the launcher
   untouched.
4. **Verify per daemon lifetime**, with a cheap metadata seal check on the
   per-command invocations.

## Decision

We will carry tree artifacts in a **new top-level `artifacts` map** in
`manifest.json`, **resolved by the launcher**, **verified once at
materialisation** and **sealed read-only** thereafter.

- **The manifest extension is additive under `schema_version: 1`.** Older
  launchers parse the document, ignore `artifacts`, and keep resolving
  single-file binaries unchanged. `binaries` continues to mean one key, one
  executable. The existing all-zeros sentinel digest carries over for platforms
  where an artifact is deliberately absent.
- **The launcher owns resolution.** It already holds the embedded trusted key,
  the fetcher, the cache root and the error vocabulary; adding a second holder
  of that key in `accelerator-design` would contradict ADR-0059's single-trust-
  primitive driver.
- **Verification happens on the fetched archive, before extraction.** The
  digest and signature are checked over the archive bytes, so nothing is
  written to the tree until both pass and no unverified byte is ever extracted.
- **Materialisation is atomic.** The archive is extracted into a temp directory
  and the *directory* is renamed into place in one syscall, mirroring the
  temp-then-rename discipline single-file entries already use. A concurrent
  invocation therefore sees either no tree or a complete one, never a partial
  extraction — which matters more here than for single files, since with no
  per-exec check nothing would detect a partial tree afterwards.
- **After the rename the tree is sealed**: read-only permissions, plus a
  sentinel recording the verified digest and the release version. The sentinel
  lives beside the tree rather than inside it, so it never becomes part of what
  it attests. Subsequent runs check for the sentinel, which is what makes a
  warm invocation cost microseconds rather than milliseconds.
- **Tree entries are addressed by release version and digest**, as single-file
  entries are by `{name}-{version}-{sha256}`. A new release materialises a new
  tree rather than mutating one in place, which is what makes the rename safe
  and what leaves prior versions on disk for the pruning question ADR-0059
  records.
- **Tree artifacts are exempt from per-exec re-verification.** Single-file
  sub-binaries keep theirs; the resolver's re-verify-and-self-heal path is
  unchanged for them. The two models coexist deliberately, and the exemption is
  a documented difference rather than an oversight.
- **An explicit repair path replaces automatic self-healing.** Because nothing
  re-checks the tree per exec, corruption cannot be detected and refetched on
  the fly. A user-invocable re-verify-and-refetch restores that recovery
  without paying for it on every command.

Option 2 was rejected on the measurements above: it spends 5–11% of a crawl's
budget re-hashing immutable bytes, and puts every invocation six times over the
warm-path target, to close a window against an attacker who already has the
privileges needed to bypass it elsewhere.

Option 3 was rejected because it would put the embedded signing key and a
second copy of fetch-verify-cache in another crate.

Option 4 is strictly more protective than what we chose — a metadata seal
catches a careless tamperer that we will not catch at all. It was rejected
because that increment is small against the same-uid threat model, while the
cost is permanent: a second verification path, a per-exec check to keep within
budget, and seal semantics to define and test. We would rather have one clearly
stated boundary than two partial ones.

## Consequences

### Positive

- A warm browser command pays a sentinel check rather than hundreds of
  milliseconds of hashing, so the crawl budget and 0186's warm path both
  survive the arrival of a 400MB artifact set.
- No schema bump, so no flag day for the other sub-binaries.
- The signing key keeps exactly one holder, and `binaries` keeps one meaning.
- Verifying the archive before extraction means a failed check leaves nothing
  on disk to clean up.

### Negative

- A tree tampered with after materialisation will be executed without
  detection. This is a real reduction against what single-file sub-binaries
  get, and it is permanent rather than transitional.
- Corruption no longer self-heals. A truncated or partially-extracted tree
  surfaces as a confusing runtime failure until the repair path is run, where
  today the resolver silently refetches.
- Read-only sealing is undone by the same uid that set it, so it deters
  accident and stray writes rather than an attacker.
- Two integrity models now live in one codebase, and every future reader of
  `resolve/` has to learn which applies where.

### Neutral

- The `chromium-headless-shell` build is 14 files against full Chromium's 327,
  so the sealing and extraction work differs by more than an order of magnitude
  between them. The decision holds either way; ADR-0059 leaves that choice to
  fidelity measurement.
- The sentinel replaces what the `{name}-{version}-{sha256}` filename does for
  single files: it is where a tree's verified digest is recorded, since a
  directory cannot carry it in its name.
- `ETXTBSY` does not arise for trees the way it does for single-file
  replacement: because a repair materialises a fresh directory and renames it
  into place, a process already executing from the old tree keeps its inodes.

## References

- ADR-0046 (zero-setup static binary distribution), ADR-0054 (git-style modular
  CLI), ADR-0057 (browser automation as a glibc-only capability), ADR-0059
  (build-time assembly of vendored browser artifacts)
- `cli/launcher/src/launch/outbound/resolve/` — the resolver this extends;
  `mod.rs:196` is the per-exec re-verify this exempts trees from, and
  `manifest.rs:1` the additive-fields contract it relies on
- `meta/work/0164-launcher-and-git-style-dispatch.md` — froze re-verification
  before every exec, which this narrows to single-file artifacts
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — established
  the warm-path budget
- `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
