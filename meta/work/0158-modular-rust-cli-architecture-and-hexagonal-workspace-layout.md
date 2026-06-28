---
type: work-item
id: "0158"
title: "Modular Rust CLI Architecture & Hexagonal Workspace Layout"
date: "2026-06-27T12:23:42+00:00"
author: Toby Clemson
status: done
kind: spike
priority: high
external_id: "PP-181"
tags: [spike, rust, cli, architecture, hexagonal]
last_updated: "2026-06-27T12:23:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0158: Modular Rust CLI Architecture & Hexagonal Workspace Layout

**Kind**: Spike
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

> **Ported from luminosity.** This spike was conducted in the downstream
> luminosity project and is imported here as a faithful historical record of the
> research that feeds Accelerator's Rust-CLI direction. Its `## Spike Outcome`
> and `## Recommendation` are preserved as captured; only luminosity's own
> command/crate names and paths have been rewritten to Accelerator's. The
> luminosity original is linked in `## References`.

## Summary

Resolve how to structure the Accelerator Rust CLI as a modular, git-style command
over a hexagonal core, evaluating current options on merit, so the dependent
scaffold, distribution, and spike-dependent ADRs proceed on settled
foundations. Time-boxed to 3 days.

## Context

The Rust-CLI direction commits to a thin CLI over a ports-and-adapters core,
presented as a single `accelerator` command that dispatches to on-demand
sub-binaries. The author is not deeply familiar with structuring a modularised
Rust CLI or splitting crates across CLI / domain / persistence, so this spike
de-risks those choices before the scaffold work begins. Accelerator's existing
binary-distribution infrastructure (the visualiser's build and release pipeline,
its bash launcher) is one input among several — the spike evaluates alternatives
rather than adopting it by default.

## Requirements

Research questions to resolve, time-boxed to **3 days**:

- **Crate split & dependency direction.** Confirm the lean `core` (domain +
  ports as traits + application) / `adapters` (port impls) / `cli` (composition
  root) workspace layout, and how the inward dependency direction is enforced.
  (Research finding: the Cargo dependency graph is itself the primary enforcement
  — `core` simply does not depend on `adapters`/I/O crates and a violation won't
  compile — reinforced by a `cargo-deny` ban-list keeping infra crates out of
  `core`. No ArchUnit-equivalent exists for Rust. Reference: howtocodeit/hexarch,
  which models the trait/`Service` core within a single crate — evaluate whether
  the multi-crate split earns its keep now or later.)
- **Git-style dispatch.** Validate clap 4.x `allow_external_subcommands` /
  `#[command(external_subcommand)]` collecting `Vec<OsString>` (not `String`, to
  pass non-UTF-8 args); first positional as subcommand, everything after handed
  off verbatim. Decide how to synthesise `--help`/a plugin listing (clap won't
  generate help for external subcommands) and `exec` on Unix vs spawn+wait on
  Windows. Compare rustup's shim/proxy model and uv's resolve-once-and-cache
  model.
- **On-demand launcher.** Decide the fetch → verify → cache → exec pipeline:
  `reqwest`(rustls) or `self_update`(rustls feature) for download, `sha2` for
  checksum, signature verification (zipsign/minisign), a managed bin dir for
  caching, `self-replace` for the launcher's own self-update. Confirm
  `default-features = false` + rustls throughout (native-tls pulls OpenSSL on
  Linux and breaks musl-static).
- **Cross-compile & distribution (scan, defer detail to the distribution
  work).** Note current best practice — `cargo-zigbuild` for musl-static/macOS
  cross, `cross` as fallback for awkward native deps, `dist` (formerly
  cargo-dist, v0.32+) for signed/checksummed per-platform artifacts — and
  confirm which of these we already use before re-selecting.

Output: a written recommendation with a decision, evaluating the alternatives
above, recorded directly in this spike work item (a `Recommendation` section
added when the spike concludes). The spike-dependent CLI ADRs (the hexagonal-core
and modular-CLI decisions) cite this work item, and the scaffold and distribution
work reference it when implemented. A throwaway proof-of-concept of the
dispatch/launcher exec path is optional, not required.

## Acceptance Criteria

- [x] Given the 3-day time-box, when the spike concludes, then this work item
      contains a Recommendation section naming a chosen crate split, dispatch
      mechanism, launcher pipeline, and cross-compile/distribution toolchain,
      each with the alternatives considered and the rationale for the choice.
- [x] The Recommendation explicitly states whether the multi-crate
      core/adapters/cli split is adopted now or deferred, and how the inward
      dependency direction is enforced.
- [x] The spike-dependent ADR(s) for the hexagonal-core and modular-CLI
      decisions cite this work item, and the scaffold and distribution work
      reference it when implemented.

## Spike Outcome

- **Date**: 2026-06-23
- **Time spent vs box**: Concluded well within the 3-day time-box (agent-driven
  research + collaborative discussion; no prototype required).
- **Verdict**: All four research areas resolved. The CLI is a Cargo workspace
  decomposed **by bounded context** (not the brief's single `core`/`adapters`
  split), dispatching git-style via clap external subcommands to on-demand
  static sub-binaries fetched and verified (sha256 + minisign) by a Rust
  launcher, cross-compiled with `cargo-zigbuild` and released via our proven
  hand-rolled `gh` pipeline.

This recommendation supersedes the brief's working assumption of a
`core`/`adapters`/`cli` crate split: investigation showed sub-commands are
distinct **subdomains** (bounded contexts) with divergent dependency profiles,
not different adapters onto one shared core — so the decomposition axis is
subdomain-first, hexagonal-within. See Recommendation §1.

## Recommendation

### 1. Crate split & dependency direction — workspace now, hexagon per subdomain, split later

**Decision.** Structure the CLI as a **Cargo workspace decomposed by bounded
context (subdomain)**, not as one shared hexagonal core with sub-commands as
adapters. A workspace is required regardless, because the git-style model ships
**independently-fetched sub-binaries** — and multiple binaries from a single
package would force one shared dependency set and one version, defeating the
lean on-demand goal. The decomposition therefore runs on two orthogonal axes:

- **Binary axis** — one crate per independently-shippable sub-binary
  (`accelerator-<sub>`), each its own composition root.
- **Layering axis** — within each subdomain, hexagonal layers as *modules*.

Layout:

- **`kernel`** — a thin, deliberately dependency-light crate for genuinely
  cross-cutting concerns (error taxonomy, the config-access contract, the
  dispatch/launcher contract, logging). Admitting anything with a dependency
  tail is resisted, because everything links it.
- **Each subdomain** (e.g. a media or content subdomain) — its own hexagon,
  starting as a **single crate** with `domain` / `application` (ports as traits
  — *both* inbound/driving and outbound/driven ports live in
  application/domain) and separate `inbound/` + `outbound/` adapter modules.
  Split internally into crates only under pressure (reuse, or divergent heavy
  adapter deps).
- **`config`** — a deliberately **shared** crate that other subdomains may
  depend on, split into **`config`** (domain + application + ports) and
  **`config-adapters`** (outbound readers; pulls serde/toml/fs). Consumers
  depend only on the light `config` crate; the outbound adapters are wired at
  the composition root. (Composition-root **Model 1**: each sub-binary wires its
  own `config-adapters` — their deps are light. Model 2, where the launcher
  resolves config once and injects it, is held in reserve if resolution becomes
  expensive.)
- **`cli`** — the `accelerator` launcher binary; depends on `kernel` (+ `config`
  for the built-in `config` command), never on a subdomain.

**Multi-crate now or later?** The *workspace* and the *per-subdomain* split are
adopted **now** (forced by independent sub-binaries). Splitting the hexagonal
*layers* of a single subdomain into separate crates is **deferred** until a
second consumer or a heavy divergent adapter set appears — matching the canonical
reference (howtocodeit/hexarch is a single crate with `domain`/`inbound`/
`outbound` modules) and the broader Rust consensus (Tokio's crate *merge* is the
cautionary tale; splitting later is cheap, over-splitting early is not).

**Enforcing the inward dependency direction (layered, mostly on stable):**

- **Crate boundaries are the primary enforcement** — once layers/contexts are
  separate crates, the Cargo dependency graph makes a violation fail to compile
  (`core`/`config`/`kernel` simply do not depend on outer crates). `cargo-deny`
  ban-lists keep infrastructure crates out of the light crates' closures.
- **`cargo-pup`** (DataDog; the closest ArchUnit-equivalent for *intra-crate*
  module-import rules) runs in **local (`mise run check`) and standard CI as a
  blocking check**. This accepts a **pinned-nightly check lane** (cargo-pup hooks
  compiler internals); the **product build and all other required checks stay on
  stable** — the shipped binary never touches nightly. The nightly is pinned in
  `mise.toml` and bumped deliberately.
- A **CI grep tripwire** (forbidding `use crate::{adapters,inbound,outbound}`
  from `domain` modules) is the zero-dependency floor inside leaf crates.

**Alternatives considered.** (a) Single shared `core` crate with sub-commands as
adapters — *rejected*: a subdomain's heavy deps (e.g. video codecs) would land in
`core`'s closure and bleed into every binary. (b) Multi-crate layer split per
subdomain from day one — *deferred*: pays the interface/coordination cost the
community warns against before a second consumer exists. (c) Feature-gating one
monolithic `core` to contain dependency bleed — *rejected*: fragile, recouples
everything into one versioned crate, fights Cargo. (d) Relying on module
visibility alone for direction — *rejected*: visibility controls exposure, not
who imports.

**Evidence.** howtocodeit/hexarch (single crate, `domain`/`inbound`/`outbound`
modules, ports as traits); Tokio crate-merge cautionary tale (corrode.dev);
flosse/clean-architecture-with-rust and the Barrage "banker" tutorial
(multi-crate examples, Cargo graph enforces inward deps); DataDog/cargo-pup
(intra-crate import lints, v0.1.x, nightly-pinned); cargo-deny bans; Rust Project
Primer (workspace trade-offs).

### 2. Git-style dispatch — clap external subcommands, Unix-only

**Decision.** Use clap 4.x derive **`#[command(external_subcommand)]
External(Vec<OsString>)`**: the first element is the subcommand name, the rest
are forwarded verbatim. `Vec<OsString>` (not `String`) preserves non-UTF-8 args.
The derive attribute alone enables external dispatch (no manual
`allow_external_subcommands`) — *confirm on the pinned clap version at scaffold
time*.

- **Built-in vs external.** `version` and `config` are **built-in** subcommands
  compiled into `accelerator`; external dispatch is purely the *growth*
  mechanism for on-demand subdomains.
- **Platform/exec model.** **Unix `exec` only** (`CommandExt::exec`,
  process-replacing — exit codes and signals propagate naturally). **Windows is
  out of scope**, since the four target triples are all Unix
  (`darwin-arm64/x64`, `linux-arm64/x64` musl); this drops the entire Windows
  spawn+wait/Ctrl-C path the brief framed.
- **Help & listing synthesis.** clap cannot list external subcommands, so render
  clap's built-in help plus a synthesised "external subcommands" section built
  by scanning the managed cache dir **and the release manifest's `description`
  field** (the manifest the launcher already needs for fetch+verify — one extra
  field, no executing untrusted binaries to introspect them). Per-command
  `--help` is **delegated** by re-exec'ing the child with `--help` (cargo's
  convention) — breadth from the manifest, depth from the binary.
- **Caching model.** uv-style **resolve-once-and-cache** (fits a single entry
  point far better than rustup's per-tool shims). Managed cache dir scanned
  first, fetch-on-miss, keyed by name+version+checksum; exact path deferred to
  the distribution work.

**Alternatives considered.** rustup shim/proxy model — *rejected* for a
single-entry-point CLI (its per-tool-on-PATH benefit doesn't apply); multicall /
busybox single-fat-binary — *rejected*: grows monolithically and ships every
subdomain's deps to everyone, the opposite of on-demand. Stale clap 2/3
`AppSettings` advice ignored.

**Evidence.** clap `git-derive.rs` example & derive cookbook; `git`/`cargo`/
`kubectl plugin`/`gh extension` listing patterns (`cargo --list`'s missing
third-party descriptions — cargo#10662 — is the gap the manifest closes);
`std::os::unix::process::CommandExt::exec`; rustup vs uv caching models.

### 3. On-demand launcher — Rust in-binary, thin bash bootstrap

**Decision.** Fetch → verify → cache → exec for sub-binaries lives **inside the
Rust `accelerator` binary** (the `cli` crate), not in bash. A **thin bash
bootstrap** (simplified from our existing `launch-server.sh`) fetches the
`accelerator` binary itself on first use; thereafter the Rust launcher owns
everything.

- **HTTP/TLS stack.** **`reqwest` + rustls, workspace-wide** (`default-features =
  false`, `rustls-tls`): blocking in the launcher, async in subdomains that need
  it. Chosen over `ureq` because reqwest covers **both sync and async** from one
  dependency (ureq is sync-only), giving workspace uniformity; the cost is that
  blocking reqwest pulls `tokio` into the launcher (acceptable on musl-static).
  **rustls throughout is mandatory** — native-tls/OpenSSL breaks musl-static.
- **Verification.** `sha2` checksum verified on fetch **and re-verified on every
  launch** before exec (our cheap belt-and-suspenders), against a
  version-coherent manifest shipped with the plugin — **plus minisign** (see §4).
- **Self-update.** **None.** The plugin-update mechanism drives the launcher
  version (new plugin version → bootstrap fetches new launcher; sub-binaries
  re-fetch on manifest hash change). `self-replace` is dropped from the brief.

**Alternatives considered.** Bash launcher wholesale (extend our existing shell
pipeline to sub-binaries) — *rejected*: pushes dispatch/launch logic into
untestable bash on a 3.2 floor. `ureq` — *rejected* on the sync+async uniformity
criterion. `self_update`/`self-replace` — *rejected*: the plugin upgrade already
replaces the launcher.

**Evidence (our existing launcher, analysed directly).** Accelerator's launcher
is bash + curl + sha256 caching into the plugin's `bin/` and `nohup`-ing a
*daemon* — it does **not** model a git-style sub-binary CLI, so the new CLI
genuinely diverges here; our *distribution* half (§4) ports cleanly, our
*launcher* half does not.

### 4. Cross-compile & distribution — reuse our existing pipeline + minisign

**Decision (scan; detail deferred to the distribution work).**

- **Cross-compile** with **`cargo-zigbuild`** for the four targets
  (`aarch64/x86_64-apple-darwin`, `aarch64/x86_64-unknown-linux-musl`) — proven
  in our existing visualiser build, handles musl-static + macOS cross from one
  host. `cross` not needed (the launcher is pure Rust + rustls, no awkward native
  deps).
- **Release orchestration**: **reuse our existing hand-rolled approach** —
  Python invoke tasks + `gh` release with re-download-and-re-verify + **version
  coherence** across `plugin.json` / `Cargo.toml` / the manifest. Chosen over
  **`dist` (cargo-dist)**, which is oriented at a fixed set of workspace binaries
  released together and would fight the individually-fetched-sub-binary +
  per-command-manifest model; it also adds tooling we deliberately avoid, and the
  invoke infrastructure already exists in this repo.
- **Integrity & signing**: sha256 + TLS + in-repo manifest **plus minisign**.
  Sign each released artifact with minisign, embed the public key in the
  launcher, and verify the detached `.minisig` **in-process via `minisign-verify`**
  (mature, zero-dependency, pure-Rust, sync — ideal for a lean static launcher),
  with **no `gh` dependency on the user's machine**.

**Alternatives considered for signing.** (A) In-process verification of the SLSA
provenance already attested by `actions/attest-build-provenance`, via
`sigstore-verify` — *parked*: technically possible without `gh`, but the crates
are pre-1.0, pull a heavy native tree (`aws-lc-rs`, awkward for musl-static),
require periodic trust-root re-embedding, and the one production in-process
attempt (`jdx/sigstore-verification`, used by **mise**) was **archived in May
2026** after repeated breakage. Revisit when `sigstore-verify` reaches a stable
1.0 — it is the strictly stronger "who built it" claim. (C) sha256 + TLS only
(our current floor) — *rejected as the ceiling*: TLS authenticates the CDN, not
the build; retained only as the integrity layer *under* minisign.

**Evidence.** Our `tasks/build.py` (cargo-zigbuild, four triples),
`tasks/github.py` (gh release + re-verify), version coherence; `minisign-verify`
0.2.5 (zero-dep, jedisct1) and `minisign`/`rsign2` for the release side;
sigstore-rs 0.14 (experimental) / prefix-dev `sigstore-verify` (0.1.x);
jdx/sigstore-verification archival; GitHub offline-attestation docs.

## Residual Risks & Open Questions

- **cargo-pup nightly lane (blocking).** A nightly bump that breaks cargo-pup
  blocks all merges, local and CI. *Mitigation:* pin the exact nightly in
  `mise.toml` and bump deliberately. *Trigger to revisit:* if the lane proves
  fragile in practice, downgrade cargo-pup from blocking to advisory while
  keeping crate-boundary + grep enforcement.
- **minisign key management.** A new Ed25519 keypair must be generated,
  protected (CI release secret), and rotated; a leaked private key forges
  releases, and minisign proves "signed by our key", not "built by our
  workflow". *Owner:* the distribution work defines key storage, rotation
  policy, and `.minisig` publishing.
- **Provenance verification deferred (option A).** *Trigger to revisit:* prefix-
  dev's `sigstore-verify` reaching a stable 1.0 with a ring/rustls backend and an
  embedded-trust-root offline verify path — at which point in-process SLSA
  provenance verification becomes the preferred strengthening over minisign.
- **clap derive external-subcommand behaviour.** Low risk; *confirm* the derive
  attribute enables external dispatch without a manual builder call on the
  pinned clap version when the scaffold is built.
- **Config composition-root Model 1 → Model 2.** *Trigger to revisit:* if config
  resolution becomes expensive or a single per-invocation source of truth is
  wanted, move resolution into the launcher and inject it (sub-binaries then
  depend on `config` core only).
- **Exact cache/bin directory path** for fetched sub-binaries (XDG data dir vs
  plugin `bin/`) is deferred to the distribution work.

## Open Questions

- Whether the hexagonal split starts multi-crate or begins single-crate
  (hexarch-style) and splits later — to be decided by the recommendation.

## Assumptions

- The hexagonal split is crate-level, not module-level — though the spike may
  recommend a single-crate start, given the leading reference (hexarch) does
  hexagonal well within one crate.

## Technical Notes

- clap 4.x external-subcommand support is stable and ergonomic; ignore older
  clap 2/3 `AppSettings` workarounds in stale articles.
- rustls + webpki-roots keeps the launcher independent of the host cert store and
  statically linkable; audit the dep tree so no transitive dep re-enables
  `default-tls`.
- `self_update` is steady but low-velocity and oriented at self-replacement; for
  a multi-sub-binary launcher, compose download/verify/cache primitives directly
  and use `self-replace` only for the launcher's own update.

## Drafting Notes

- Kind set to `spike` because the work is framed as resolving an unknown via
  research with a written recommendation, not a deliverable.
- Latitude set to genuine evaluation of alternatives; our existing infrastructure
  is one input, not the default.
- Output framed so the recommendation is recorded in this spike work item itself;
  dependent work and ADRs reference this work item rather than the decisions
  being copied into them.
- Enriched with web research (clap 4.x dispatch, rustls launcher, hexarch,
  cargo-zigbuild/dist) current as of mid-2026.
- In luminosity (where this spike ran) it blocked the scaffold and distribution
  stories; those downstream work items are not ported into Accelerator.

## References

- **Ported from luminosity** — original spike:
  https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0002-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md
- Related research: `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
  (Accelerator's Rust-CLI migration surface).
- Research: clap external subcommands (docs.rs/clap 4.6.x); rustup proxy/shim
  model; uv tool caching; howtocodeit/hexarch (hexagonal in Rust); cargo-zigbuild
  v0.23; dist (formerly cargo-dist) v0.32; self_update / self-replace; rustls vs
  native-tls.
</content>
</invoke>
