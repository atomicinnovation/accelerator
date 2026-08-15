---
id: "ADR-0057"
date: "2026-08-10T09:36:08+00:00"
author: Toby Clemson
status: accepted
tags: [architecture, distribution, playwright, browser, platform-support, glibc, musl, design]
type: adr
title: "ADR-0057: Browser Automation as a Glibc-Only Capability"
schema_version: 1
last_updated: "2026-08-10T16:29:22+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0045", "adr:ADR-0046", "adr:ADR-0048", "adr:ADR-0054",
  "adr:ADR-0058", "work-item:0196"]
---

# ADR-0057: Browser Automation as a Glibc-Only Capability

**Date**: 2026-08-10
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0046 decided that the CLI reaches users as **zero-setup, fully static,
dependency-free binaries** the plugin fetches, verifies, and executes on
demand. Its decision drivers include "the binary must execute without a local
build toolchain and **without depending on a specific host libc or system
libraries**", and it explicitly rejected dynamic linking because "dynamic
linking reintroduces a dependency on the host's libc and system libraries —
fragile across Linux distributions". Linux targets are built against musl for
full static linking, so today's four binaries run on glibc and musl hosts
alike.

The `inventory-design` skill drives a real browser to capture design
inventories. Work-item:0196 migrates that tooling into an
`accelerator-design` sub-binary and, to remove the current system Node.js ≥20
and `npm`/`npx` prerequisites, vendors Microsoft's official per-platform
Playwright driver bundle (a prebuilt Node.js binary plus `playwright-core`)
as a distributed artifact fetched through the CLI's own manifest/minisign
mechanism.

That bundle is glibc-linked, which reads at first glance as a direct
contradiction of ADR-0046. Research on 2026-08-10 established that the
framing is wrong in an important way:

- **Playwright does not support musl at all.** Its officially supported Linux
  platforms are Debian 12/13 and Ubuntu 22.04/24.04/26.04. Alpine and other
  musl-based distributions are documented as unsupported, with no
  officially-supported workaround.
- **The runtime and the browser bind independently.** Both the driver's Node
  binary and Playwright's Chromium builds are glibc-linked, so replacing
  either one alone leaves the other unable to run.
- **The driver bundle has no musl variant.** Microsoft publishes it for `mac`,
  `mac-arm64`, `linux`, `linux-arm64`, `win32_x64` and `win32_arm64` only; the
  Linux builds are manylinux (glibc). All four of this project's platforms —
  macOS x64 and arm64, Linux x64 and arm64 — are covered.
- **The constraint already binds us today.** `ensure-playwright.sh` runs
  `npx playwright install chromium`, which downloads the same glibc Chromium.
  The design tooling cannot run its Playwright path on a musl host now, before
  any bundling.

So work-item:0196 does not introduce a musl gap — it inherits one that is
inherent to depending on Playwright at all. What it does change is that a
glibc-linked artifact begins flowing through the distribution mechanism whose
governing ADR says "fully static, dependency-free". That narrower tension is
what this ADR resolves.

Two facts bound the decision. First, the tooling **already degrades gracefully
in its default and hybrid modes**: `notify-downgrade.sh` emits a documented
downgrade notice and the skill falls back to a code-only crawler when the
Playwright runtime is unavailable (`node-missing`, `node-too-old`,
`bootstrap-failed`, and siblings). An explicit `--crawler runtime` request
still hard-fails, by design. A host that cannot run Playwright is an
already-designed case, not a new failure. Second, the *status quo* is itself in
tension with ADR-0046's "the end user installs nothing beyond the plugin
itself; no toolchain, **runtime**, package manager, or `PATH` configuration" —
requiring system Node and npm is precisely the setup burden that ADR forbids.
Vendoring the runtime moves toward ADR-0046's intent, not away from it.

This ADR records only the **platform scope of the browser-automation
capability** and the escape hatch for hosts outside it. The integrity model
for a directory-tree artifact and the provenance semantics of re-signing a
third-party bundle are separate concerns, deliberately left to their own
decisions.

## Decision Drivers

- **Honesty about reach** — the supported-platform claim should match what
  upstream actually supports, not what our distribution mechanism aspires to.
- **Preserve the CLI's universal guarantee** — whatever is decided for browser
  automation must not weaken ADR-0046's promise for the CLI binaries
  themselves.
- **Zero user setup** — removing the system Node.js/npm prerequisite is the
  point of the exercise; a decision that reintroduces host prerequisites
  defeats it.
- **Graceful degradation over hard failure** — hosts outside the supported
  matrix should lose a capability, not the tool.
- **A host that cannot obtain our browser should still have a route** — where
  the platform can run a browser but the download cannot happen, a user who
  has already solved the browser problem should be able to use their solution.

## Considered Options

1. **Scope the capability to upstream's matrix**, keep the CLI's static
   guarantee intact, rely on the existing code-only downgrade, and add a
   system-browser escape hatch.
2. **Build a musl driver bundle in-house** by pairing a musl Node.js from
   `nodejs/unofficial-builds` with `playwright-core`.
3. **Drop Node entirely** and drive Chromium over the DevTools Protocol from
   Rust (e.g. `chromiumoxide`).
4. **Keep the system-Node prerequisite** and do not vendor anything.
5. **Retire the runtime crawler** and ship only the code-only crawler,
   removing browser automation from the product.

## Decision

We will treat **browser automation as a glibc-only capability of the
`accelerator-design` sub-binary**, scoped to the platforms Playwright itself
supports, while the CLI's own binaries remain fully static and universal.

Concretely:

- **The CLI's guarantee is unchanged.** Every `accelerator` binary, including
  `accelerator-design` itself, remains a fully static musl/macOS build that
  runs without regard to host libc. ADR-0046 continues to hold, without
  qualification, for everything we build.
- **The vendored Playwright driver bundle is an explicitly-scoped exception**
  to "dependency-free", covering only artifacts we vendor rather than build.
  It inherits Playwright's support matrix; we neither extend nor narrow it.
- **`accelerator design` degrades in its default and hybrid modes.** On a host
  where the bundled driver or its browser cannot run, the Playwright path
  reports the existing structured downgrade reason and the skill falls back to
  the code-only crawler. Losing the runtime crawler is a reduced-fidelity
  inventory, not an error. An explicit `--crawler runtime` request continues
  to hard-fail: a caller who asked for the runtime crawler should be told it
  is unavailable, not handed a lower-fidelity inventory silently.
- **A system-browser escape hatch is supported.** A `design.browser_path`
  configuration key, with a one-shot environment override, names an existing
  Chromium-family executable to drive instead of the bundled browser
  download. This serves glibc hosts that cannot or should not download a
  browser — air-gapped installs, sandboxed networks, and anyone with a
  distro-packaged Chromium they would rather use. The key is a path to an
  executable and is trusted on par with the rest of the project's
  configuration.
- **The escape hatch does not reach musl hosts, and we accept that.** It
  substitutes the browser, not the runtime; the vendored driver's Node binary
  is still glibc-linked, so a musl host fails before the browser path is
  consulted. Reaching musl would need a second substitution — driving a
  user-supplied Node and `playwright-core` — which reintroduces exactly the
  host-runtime prerequisite this work removes. Musl hosts therefore keep the
  code-only crawler.
- **We will not vendor a musl runtime.** Option 2 is rejected on the record:
  `nodejs/unofficial-builds` musl binaries are experimental, minimally tested,
  excluded from Node's release gating, and require `libstdc++` to be installed
  on Alpine — and none of that helps, because Playwright's Chromium remains
  glibc-linked. It would add an unsupported dependency and deliver no working
  capability.

Option 3 was rejected **for this work item** rather than on the merits: it
would remove the Node dependency and the bundle altogether, but it discards
the existing Playwright automation (`run.js` and `lib/*.js`, roughly 1,700
lines with their tests) in favour of reimplementing those behaviours against
raw CDP, and browser acquisition regresses on arm64 — Google's Chrome for
Testing publishes no linux-arm64 build, whereas Playwright does. It remains a
legitimate future direction and should be reconsidered if the vendored-bundle
mechanism proves costly to maintain.

Option 4 was rejected because it preserves exactly the host prerequisite
ADR-0046 set out to eliminate.

Option 5 was rejected because the runtime crawler is what makes an inventory
faithful to the rendered product. A code-only inventory reports what the
source declares; only a driven browser reports computed values, actual
contrast, and states that exist solely at runtime. Narrowing that capability's
platform reach is a smaller loss than deleting it.

## Consequences

### Positive

- The end user installs no toolchain, runtime, or package manager to use the
  design tooling on a supported platform — the ADR-0046 intent the status quo
  violates is satisfied.
- The supported-platform claim becomes accurate and checkable: it is
  Playwright's, by reference, rather than an aspiration of our own.
- The CLI's universal static guarantee is preserved and stated explicitly,
  rather than being quietly eroded by a counter-example.
- Musl hosts keep a working tool with a reduced-fidelity crawler, and glibc
  hosts that cannot download a browser gain a supported route to the full
  crawler via the escape hatch.
- Rejecting the musl-runtime option on the record stops it being re-proposed
  as an obvious fix.

### Negative

- A glibc-linked artifact now flows through a distribution mechanism whose
  governing ADR says "fully static, dependency-free"; the exception must be
  read alongside ADR-0046 rather than from it alone.
- The runtime crawler's reach is narrower than the CLI's, so "which platforms
  are supported" now has two answers depending on the capability.
- Musl hosts lose the runtime crawler outright, with no configuration that
  recovers it. The escape hatch narrows the gap for glibc hosts only.
- "Zero setup" holds for the runtime and package manager, not for Chromium's
  shared libraries. A lean glibc image can still lack `libnss3`, `libatk` and
  their siblings — the reason upstream ships `playwright install --with-deps`
  — so a minimal container may need system packages before the browser starts.
- The escape hatch admits a user-supplied executable path, which is a trusted
  input and a surface worth reviewing as such.
- The escape hatch drives a browser build we neither pin nor verify. Playwright
  pins browser revisions because the driver and the browser share a protocol
  contract, so an inventory captured through a host Chromium is not reproducible
  against one captured through the bundled browser, and a sufficiently divergent
  build can fail on protocol mismatch rather than degrade cleanly. Choosing the
  hatch trades fidelity guarantees for reach.
- Vendoring a third-party runtime creates an ongoing obligation to track
  upstream's platform matrix; if Playwright's support set changes, ours moves
  with it.

### Neutral

- Windows is absent from our platform matrix, so the bundle's two Windows
  variants are simply unused.
- This decision extends ADR-0048's account of Node.js: Node moves from
  dev-time tooling provisioned via `mise` to a vendored artifact inside the
  product's distribution closure. ADR-0048 records "only the split and the
  role each toolchain plays" and never contemplated distribution, so this is
  new territory rather than a contradiction.
- The vendored runtime is spawned directly by the Rust binary, with no shell in
  between; that delegation chain is decided by ADR-0058, not here.

## References

- ADR-0045 (skills vs CLI division of labour), ADR-0046 (zero-setup static
  binary distribution), ADR-0048 (four-toolchain split), ADR-0054 (git-style
  modular CLI), ADR-0058 (shell-free CLI-to-Node delegation)
- `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
- `meta/research/codebase/2026-08-10-0196-accelerator-design-inventory-gap-tooling-cli.md`
- Playwright system requirements (Debian/Ubuntu only):
  https://playwright.dev/docs/intro
- Playwright Docker/Alpine guidance: https://playwright.dev/docs/docker
- Driver bundle platform list and download logic:
  https://github.com/microsoft/playwright-python/blob/main/setup.py
- Node.js unofficial (musl) builds and their caveats:
  https://github.com/nodejs/unofficial-builds
- Chrome for Testing availability (no linux-arm64):
  https://googlechromelabs.github.io/chrome-for-testing/
