---
type: adr
id: "ADR-0059"
title: "Build-Time Assembly of Vendored Browser Artifacts"
date: "2026-08-10T16:58:36+00:00"
author: Toby Clemson
producer: create-adr
status: accepted
relates_to: ["adr:ADR-0046", "adr:ADR-0054", "adr:ADR-0057", "adr:ADR-0058",
  "work-item:0164", "work-item:0165", "work-item:0196"]
tags: [architecture, distribution, provenance, supply-chain, playwright,
  browser, design]
last_updated: "2026-08-10T17:23:38+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# ADR-0059: Build-Time Assembly of Vendored Browser Artifacts

**Date**: 2026-08-10
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0057 scoped browser automation to Playwright's platforms and deliberately
left two questions to their own decisions: the integrity model for a
directory-tree artifact, and the provenance semantics of putting our signature
on a third-party bundle. This ADR takes up both, together with a third that
turns out to be the same question — how the browser itself is acquired.

Today `ensure-playwright.sh` runs `npm ci` and `npx playwright install
chromium` on the user's machine at first use. That requires system Node and
npm, which ADR-0046 forbids, and it means every user independently fetches
bytes from hosts we do not control. Work-item:0196 replaces it. The CLI has
exactly one artifact-verification mechanism to replace it with: the
`manifest.json` plus minisign contract frozen by work-item:0164, verified
against a public key embedded in the launcher.

The original plan was to vendor Microsoft's prebuilt Playwright driver bundle
and re-sign it with our key. Research on 2026-08-10 found that the premise
does not hold:

- **The prebuilt bundle carries no upstream signature.** Microsoft's
  `scripts/build_driver.py` assembles it from two published artifacts — the
  `playwright-core` npm tarball and a Node binary from `nodejs.org/dist` — and
  verifies neither. `setup.py` then checks only that the downloaded zip exists.
  There is no third-party chain of custody for our signature to inherit or
  launder.
- **Its inputs, fetched directly, have real provenance.** `playwright-core` on
  npm carries sha512 integrity, registry signatures, and SLSA v1 provenance
  attestations. Node publishes `SHASUMS256.txt` with both a GPG `.asc` and a
  `.sig`, covering all four of our platforms.
- **The browser is the weak link, and always was.** Chromium is an unsigned zip
  from `cdn.playwright.dev`; Playwright's registry retries across mirrors
  without verifying content. Its revision, however, is pinned by the
  `playwright-core` we choose — `browsers.json` at 1.55.1 names chromium
  revision 1193.

Three versions are therefore in play, not one: the `playwright-core` we
vendor, the Node runtime paired with it, and the Chromium revision it names.
The automation that must agree with the first of these declares `~1.55.1` in
`package.json` — a range, not a version, which leaves patch drift between what
the automation was written against and what a release actually ships.

So the choice is not whether to re-sign someone else's artifact. It is whether
we assemble our own from inputs we can verify, and where in the lifecycle that
verification happens.

## Decision Drivers

- **One verification mechanism in the CLI** — every additional trust primitive
  shipped to users is code that must be correct on every platform, forever.
- **Verify as close to the publisher as possible** — an upstream signature is
  worth more than a hash we computed after the fact.
- **Trust-on-first-use should happen once, under review** — not once per user,
  against whatever a CDN serves that day.
- **The user's first run should touch only hosts we control** — ADR-0046's
  zero-setup promise degrades badly if it depends on npm and a CDN being
  reachable.
- **Driver and browser must not drift apart** — they share a protocol contract
  and a mismatch is a runtime failure.

## Considered Options

1. **Re-sign the prebuilt bundle**, and keep fetching Chromium on the user's
   machine at first use.
2. **Assemble in CI from upstream inputs**, verify each against its publisher's
   own primitive at build time, and ship the result as our own signed
   artifacts.
3. **Fetch upstream inputs at runtime** on the user's machine, teaching the CLI
   OpenPGP for Node's `SHASUMS256.txt` and npm integrity plus sigstore for the
   tarball.

## Decision

We will **assemble the design capability's third-party artifacts in CI** from
their original distributors, verify each input by its strongest upstream
primitive at build time, and ship the results through the existing
`manifest.json` and minisign contract as artifacts of our own.

- **Our signature attests our own build output**, not third-party provenance.
  The re-signing question does not arise: these are our artifacts, assembled
  from inputs we verified, exactly as our four compiled binaries are.
- **The vendored `playwright-core` version is the one the automation declares,
  pinned exactly.** `package.json`'s range is tightened to a single version, so
  the package we fetch, the API `lib/*.js` was written against, and the Chromium
  revision derived from it are one choice rather than three that can drift. A
  Playwright upgrade becomes an explicit edit to that pin.
- **Inputs are verified by their publisher's own mechanism.**
  `playwright-core` against its npm registry signature and SLSA provenance
  attestation; the Node runtime against the GPG signature on `SHASUMS256.txt`.
  The published sha512 integrity is a fixity check, not provenance — it comes
  from registry metadata fetched over TLS, so it is the signature and the
  attestation that carry the chain of custody. A failure of either fails the
  release, not the user's run.
- **Chromium is pinned, not verified.** Its revision is read from the
  `browsers.json` of the `playwright-core` we vendored, and its bytes are
  pinned by hash in our manifest. We record plainly that its chain of custody
  bottoms out at TLS to Microsoft's CDN: pinning bounds the blast radius and
  makes the bytes reviewable, but it does not establish provenance.
- **Driver and browser ship as separate manifest artifacts per platform**, not
  as one fused bundle. A driver-only version bump then does not force a browser
  re-download, and a user relying on `design.browser_path` can skip the browser
  artifact entirely.
- **The Node version mirrors the pairing upstream ships**, rather than being
  chosen freely, so we stay on a combination Microsoft tests.
- **`ffmpeg` is excluded.** `browsers.json` marks it install-by-default, but it
  serves video recording and this tooling captures screenshots.
- **The CLI gains no new verification mechanism.** All upstream-primitive
  verification lives in the release pipeline.

Option 1 was rejected because it signs bytes whose origin we never checked
while leaving the browser fetched per user, unverified, at first run — the
weakest position available, and the one that made the provenance question look
unanswerable.

Option 3 was rejected because it would put OpenPGP and sigstore verification
into the CLI on every platform, and make a user's first run depend on npm and
`nodejs.org` being reachable. It buys per-user freshness we do not want:
pinning a reviewed artifact is the property we are after.

## Consequences

### Positive

- The CLI keeps exactly one artifact trust primitive, on the one path
  work-item:0164 froze and work-item:0165 produces.
- Two of the three inputs are verified against signatures their publishers
  actually make. The prebuilt bundle offers none, so this is a gain over both
  the original plan and the status quo.
- Trust-on-first-use collapses from once per user to once per release, in CI,
  where the result is hashed, reviewable, and identical for everyone.
- Both version pairings become structural rather than tested. The browser
  revision is derived from the vendored `playwright-core`, and that core is the
  exact version the automation declares — so the compatibility criterion on
  work-item:0196 is satisfied by construction rather than by a check that could
  fail late.
- A user's first run touches only our release host, preserving the air-gapped
  and proxy-mirror stories.

### Negative

- Chromium's bytes are still accepted on TLS trust alone. Our signature over
  them is an assertion about what we fetched, not about who built it.
- Per-release artifact weight grows substantially — a driver bundle already
  around 117–118MB per platform, plus a browser of comparable order, across
  four platforms. This makes the release-hosting capacity coupling already
  recorded on work-item:0196 load-bearing rather than a formality, and
  lengthens the user's first run.
- We take on redistribution obligations for Node, `playwright-core` and
  Chromium, which the licensing assumption open on work-item:0196 must now
  actually resolve.
- The release pipeline gains multi-source assembly with GPG and npm
  verification — new surface that must keep working, and that fails releases
  when upstream changes shape.
- Verifying Node's signature means maintaining a trusted set of Node release
  keys and handling their rotation. A key set that goes stale fails releases;
  one that is refreshed carelessly is the verification's weakest point.
- Every release a user upgrades through adds another cached browser tree of
  this size, so the cache needs a pruning story that the lockfile-hash keying
  does not currently provide.
- Mirroring upstream's Node pairing means our runtime version is chosen for us,
  including when upstream lags a security release we would rather have.

### Neutral

- ADR-0057's consequence about Chromium's shared libraries is untouched.
  Bundling the browser actually removes the `playwright install --with-deps`
  route that would have installed them, so a lean glibc image still needs
  system packages.
- The `design.browser_path` escape hatch gains value: it is now also the way to
  avoid a large browser download.
- Whether we ship `chromium-headless-shell` or full Chromium is left to
  fidelity measurement. The daemon launches headless, so the shell may suffice
  and is smaller; what this ADR fixes is that whichever ships is fetched and
  pinned at build time at the revision the vendored driver names.
- The separate-artifact shape bears on the manifest schema question open on
  work-item:0196: the manifest must address several artifacts of differing
  kinds, not one binary per platform.

## References

- ADR-0046 (zero-setup static binary distribution), ADR-0054 (git-style modular
  CLI), ADR-0057 (browser automation as a glibc-only capability), ADR-0058
  (shell-free CLI-to-Node delegation)
- `meta/work/0164-launcher-and-git-style-dispatch.md` — froze the
  `manifest.json` and minisign contract this decision reuses
- `meta/work/0165-multi-binary-distribution-and-release-pipeline.md` — the
  producer side that would perform the assembly
- `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
- Driver bundle assembly, unverified:
  https://github.com/microsoft/playwright-python/blob/main/scripts/build_driver.py
- `playwright-core` npm provenance: https://registry.npmjs.org/playwright-core
- Node.js release signatures: https://nodejs.org/dist/
- Chromium revision pinning:
  https://github.com/microsoft/playwright/blob/v1.55.1/packages/playwright-core/browsers.json
