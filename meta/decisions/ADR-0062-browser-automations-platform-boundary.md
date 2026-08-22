---
type: adr
id: "ADR-0062"
title: "Browser Automation's Platform Boundary"
date: "2026-08-17T11:00:25+00:00"
author: Toby Clemson
producer: create-adr
status: accepted
supersedes: ["adr:ADR-0057"]
relates_to: ["adr:ADR-0045", "adr:ADR-0046", "adr:ADR-0048", "adr:ADR-0054",
  "adr:ADR-0058", "adr:ADR-0061", "work-item:0196", "work-item:0214"]
tags: [architecture, distribution, playwright, browser, platform-support,
  glibc, musl, design]
last_updated: "2026-08-17T11:00:25+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# ADR-0062: Browser Automation's Platform Boundary

**Date**: 2026-08-17
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0057 scoped browser automation to glibc hosts, by reference to Playwright's
own support matrix, and recorded the code-only downgrade and the
`design.browser_path` escape hatch. Its decisions hold and are restated here.
What does not hold is the boundary itself: the capability is not glibc-only, and
work-item:0214 established that against prototypes on six hosts.

**Glibc is necessary but not sufficient.** A real glibc-linked aarch64 binary
whose program interpreter is `/lib/ld-linux-aarch64.so.1` exits 0 on Debian 12
and fails on NixOS with `cannot execute: required file not found` (exit 127).
NixOS is a glibc distribution; it keeps its loader in the Nix store rather than
at the path the psABI fixes, so a stock glibc artifact cannot be executed there
at all. A reader implementing ADR-0057 as written would gate on libc and wrongly
enable the capability on such a host, then discover the problem after fetching
~294MB.

**Loader presence alone is not sufficient either.** Alpine with `gcompat`
installed has a glibc loader at the psABI path, and remains unable to run
Playwright's Chromium. Debian with `musl-tools` installed carries *both* loaders
while being a perfectly good glibc host. Either condition read alone is wrong on
a real host, in opposite directions.

The consequence matters because of cost and because of remediation. ADR-0057's
driver "hosts outside the supported matrix should lose a capability, not the
tool" assumes the exclusion is detected; detecting it after a ~294MB fetch is
the failure the vendoring work set out to remove. And the two ways a host can
fall outside the boundary are not equivalent: a musl host has no route, whereas
a relocated-loader host does — `nix-ld`, or a distribution-packaged browser
through the existing escape hatch.

This ADR records only the **platform boundary of the browser-automation
capability**, the shape of the exclusions, and the escape hatch. The probe's
implementation, the tree-artifact integrity model (ADR-0061) and the delegation
chain (ADR-0058) are separate concerns.

## Decision Drivers

- **The boundary must be stated as what actually gates execution**, not as a
  proxy that is wrong on real hosts in both directions.
- **It must be decidable before any artifact is fetched** — a ~294MB download
  followed by `execve` failing is the outcome this replaces.
- **Remediable and irremediable exclusions should be distinguishable**, because a
  user who can fix their host deserves to be told which fix.
- **Graceful degradation over hard failure** — carried forward from ADR-0057.
- **Honesty about reach** — carried forward from ADR-0057.

## Considered Options

1. **A conjunctive boundary** — the host's system libc must be glibc *and* the
   interpreter the artifact demands must resolve and be executable; probe both
   before any resolution.
2. **ADR-0057's glibc-only boundary unchanged** — accept that a relocated-loader
   glibc host fetches and then fails at `execve`.
3. **Interpreter resolution alone** — ignore the libc question and test only
   whether the demanded loader is present.
4. **No boundary check** — fetch on every Linux host and let `execve` be the
   arbiter, which is the behaviour before this work.

## Decision

We will define the browser-automation capability's boundary as a **conjunction**:
the host's system C library must be glibc, **and** the program interpreter the
vendored artifacts demand must resolve and be executable. A host failing either
condition loses the capability and keeps the tool.

- **"The host's system C library" means the one the base system links against**,
  not merely one that is installed. A host with a cross-compilation libc present
  alongside its own — Debian with `musl-tools`, for instance — is whatever its own
  userland links against, and remains a glibc host.
- **Both conditions are required, and the libc condition is evaluated first.**
  `gcompat` places a glibc loader on a musl host, so interpreter resolution
  alone would admit it; NixOS places glibc's loader outside the psABI path, so
  the libc condition alone would admit it. Evaluating libc first is what makes
  the `gcompat` case refuse.
- **The two exclusions are distinct outcomes, not one.** A musl host is excluded
  on its libc, irremediably, and keeps the code-only crawler. A glibc host whose
  loader is relocated is excluded on loader resolution, and its exclusion is
  remediable — by `nix-ld` or by `design.browser_path`. They therefore report
  different downgrade reasons, because a single reason would send a NixOS user
  looking for a different distribution.
- **The boundary is evaluated before any artifact resolution**, so an excluded
  host downgrades at zero network cost.
- **Ambiguous evidence resolves in favour of attempting.** Where **either**
  condition cannot be established — a host with no dynamically-linked shell to
  inspect leaves the libc question open even when the demanded interpreter plainly
  resolves — the capability is attempted and `execve` decides. Only a *positive*
  finding refuses: observed musl, or an interpreter observed absent. Refusing on
  absent evidence would silently remove a working capability, and every
  installer surveyed for prior art defaults the same way.
- **ADR-0057's remaining decisions carry forward unchanged**: the CLI's own
  binaries stay fully static and universal; the vendored Playwright bundle is an
  explicitly-scoped exception to "dependency-free" inheriting upstream's matrix;
  `accelerator design` degrades in default and hybrid modes while
  `--crawler runtime` hard-fails; `design.browser_path` substitutes the browser
  but not the runtime, so it does not rescue a host excluded on either
  condition; and we will not vendor a musl runtime.

Option 2 was rejected because it is wrong on a real, named host in a way that
costs a full artifact fetch to discover.

Option 3 was rejected because `gcompat` makes it wrong on Alpine — the loader
resolves and Chromium still will not run.

Option 4 was rejected on the same grounds as option 2, more so: it makes the
bare `ENOENT` from an absent loader the user-facing diagnosis for every excluded
host.

## Consequences

### Positive

- The stated boundary is the one that actually gates execution, so implementing
  from this ADR cannot produce the NixOS false positive that implementing from
  ADR-0057 would.
- An excluded host is identified before any bytes are fetched.
- A relocated-loader host is told something actionable, rather than sharing a
  reason whose remediation does not apply to it.
- The `gcompat` trap is recorded, so installing a glibc compatibility layer is
  not mistaken for a fix.

### Negative

- "Which platforms are supported" now has three answers rather than two, and the
  boundary is a conjunction rather than a name, so it cannot be stated as a
  distribution list.
- A user who installs `gcompat` specifically to run this capability will find it
  still refused, and the refusal is deliberate — which will read as a bug until
  the reason is read.
- Failing open on absent evidence means one residual host shape — musl, with a
  statically linked shell, with `gcompat` — still fetches before failing.
- The boundary is now a property of the vendored artifacts rather than of a
  distribution list, so it moves silently if upstream relinks them. Nothing
  detects that but the probe itself.
- NixOS users gain a diagnosis but still no working capability out of the box.

### Neutral

- Windows remains absent from the platform matrix.
- How the two conditions are observed — which file is inspected, how the
  interpreter is read — is implementation, deliberately not decided here.
- ADR-0057's account of Node.js moving from `mise`-provisioned tooling into the
  distribution closure is unaffected and carries forward.

## References

- ADR-0057 (browser automation as a glibc-only capability) — superseded by this
  ADR; its CLI-static-guarantee, scoped-exception, degradation, escape-hatch and
  no-musl-runtime decisions are restated here unchanged
- ADR-0045 (skills vs CLI division of labour), ADR-0046 (zero-setup static
  binary distribution), ADR-0048 (four-toolchain split), ADR-0054 (git-style
  modular CLI), ADR-0058 (shell-free CLI-to-Node delegation), ADR-0061 (signed
  content-addressed tree generations)
- `meta/work/0214-settle-the-vendored-runtime-tree-artifact-mechanisms.md` —
  established the boundary against prototypes on six hosts; carries the
  per-host observations
- `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
- Playwright system requirements (Debian/Ubuntu only):
  https://playwright.dev/docs/intro
- Playwright Docker/Alpine guidance: https://playwright.dev/docs/docker
- `nix-ld`, on why a stock glibc binary does not run on NixOS:
  https://github.com/nix-community/nix-ld
- Prior art for defaulting to glibc on absent evidence — `rustup-init.sh` sets
  `_clibtype="gnu"` and flips only on a positive musl match:
  https://github.com/rust-lang/rustup/blob/main/rustup-init.sh
- `nodejs/unofficial-builds` `install-node.sh`, whose `is_musl()` returns false on
  any `ldd` failure:
  https://github.com/nodejs/unofficial-builds/blob/main/www/install-node.sh
- `cargo-binstall`'s `detect-targets`, a musl-static binary detecting host libc by
  executing candidate loaders, returning glibc-first as a retry ladder, and
  special-casing the `gcompat` stub:
  https://github.com/cargo-bins/cargo-binstall/blob/main/crates/detect-targets/src/detect/linux.rs
