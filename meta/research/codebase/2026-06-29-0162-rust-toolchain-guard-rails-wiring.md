---
type: codebase-research
id: "2026-06-29-0162-rust-toolchain-guard-rails-wiring"
title: "Research: Rust Toolchain Guard Rails in mise + CI (work item 0162)"
date: "2026-06-29T00:20:02+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0162"
parent: "work-item:0162"
relates_to: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
topic: "How to extend Rust toolchain guard rails (format/lint/test/coverage/supply-chain/architecture-enforcement) into mise tasks and CI for the incoming cli/ workspace"
tags: [research, codebase, rust, tooling, ci, mise, cargo-deny, cargo-pup, architecture-enforcement]
revision: "09e89cf0215120877a7562cae508dd4b08b10471"
repository: "accelerator"
last_updated: "2026-06-29T00:20:02+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Recorded the three implementation decisions (clippy lint set, cargo-pup nightly/version pins, workspace-level version) that resolve the open questions"
schema_version: 1
---

# Research: Rust Toolchain Guard Rails in mise + CI (work item 0162)

**Date**: 2026-06-29T00:20:02+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 09e89cf0215120877a7562cae508dd4b08b10471
**Branch**: HEAD (detached / jj-colocated)
**Repository**: accelerator

## Research Question

For work item `meta/work/0162-rust-toolchain-guard-rails.md`: how is the repo's
Rust toolchain currently wired into mise and CI, and what exactly must change to
extend format / lint / test / coverage / supply-chain / architecture-enforcement
tooling into `mise run` tasks and CI for the incoming `cli/` Rust workspace —
holding it to the same automated bar as the other components and enforcing the
inward-dependency rule (ADR-0053) and supply-chain constraints (ADR-0046)
mechanically rather than by review?

## Summary

The repo today provisions **one stable Rust toolchain** (`rust = 1.90.0` +
rustfmt + clippy, `mise.toml:8`) for a **single crate** — the visualiser server
at `skills/visualisation/visualise/server/`. There is **no** cargo-nextest,
cargo-llvm-cov, cargo-deny, or cargo-pup anywhere in committed code; **no**
`clippy.toml` or `deny.toml`; **no** nightly lane or second toolchain; and **no**
`[workspace].members` parsing in the Python build system. 0162 is therefore an
*extension* of a working single-crate pattern, not a green-field stand-up.

The existing Rust pattern is clean and copyable:

- **Roll-ups live entirely in `mise.toml` as `depends`-only aggregates.** Python
  `invoke` tasks under `tasks/` are *leaf executors only* — they do work and
  raise `Exit(code=1)` on failure. There is no aggregation logic in Python.
- The server's leaves (`tasks/format/server.py`, `tasks/lint/server.py`) invoke
  cargo via `--manifest-path {CARGO_TOML}` (no `cd`), where `CARGO_TOML` is a
  single hard-coded constant in `tasks/shared/paths.py:8`.
- `check` and `test` are **fully disjoint trees**; `cargo test` appears only in
  the `test:` namespace (`tasks/test/unit.py`, `integration.py`), never in any
  `*:check` roll-up. This is exactly where 0162 must put Rust tests + coverage.
- CI (`.github/workflows/main.yml`, the only workflow) does **not** run the
  aggregate `mise run check`; it fans the four `*:check` leaf tasks into four
  single-OS (ubuntu) jobs, and runs `test:unit`/`test:integration`/`test:e2e`
  across a `[ubuntu-latest, macos-latest]` matrix. The Rust check job
  (`check-visualiser-server`) carries a **load-bearing `RUSTUP_HOME` routing
  workaround** + a dedicated `cache_key_prefix` to avoid a parallel
  toolchain-auto-install race — any new cargo-running job needs the same.

The architecture-enforcement design comes from **ADR-0053** (inward dependency
is "the load-bearing rule, enforced mechanically"): **cargo-deny** ban-lists
enforce direction *between* crates (keep infra crates out of the light crates'
dependency closures) and the workspace-wide native-tls/OpenSSL ban (ADR-0046,
a musl-static prerequisite); **cargo-pup** enforces direction *inside* a crate
at module granularity on a **pinned-nightly lane**, isolated so the product
build + all stable checks stay green if nightly breaks. Critically, in the
single-crate starting state cargo-pup is the *sole* enforcer (cargo-deny's
cross-crate bans are inert until crates split — they first bite at the
`config`/`config-adapters` split in **0166**). The grep tripwire proposed in
the source research and 0158 was **deliberately rejected** by ADR-0053 and 0162;
do not add it.

The manifest contract — **`cli/Cargo.toml`** at the **repo top level** (not
under `skills/`), with a standard `[workspace].members` array — is authored by
the paired scaffold story **0163**; both 0162's per-crate `<crate>:check`
generation and the `paths.py` member resolution read it. The launcher crate is
named **`launcher`**, not `cli` (so per-crate tasks target `-p launcher`).

## Detailed Findings

### Current Rust wiring in mise.toml

- **Single toolchain pin**: `mise.toml:8` — `rust = { version = "1.90.0",
  components = "rustfmt,clippy" }`. No cargo-* subcommand plugins pinned; cross-
  compile uses `cargo zigbuild` installed separately. No nightly anywhere.
- **Task declaration shape** — two kinds of `[tasks."..."]` entries:
  - *Executor*: has `run = "invoke <module>.<task>"`, e.g. `format:server:check`
    (`mise.toml:283-285` → `tasks/format/server.py::check`). Dotted invoke path
    maps to the module tree; `build_system` → `build-system` via invoke's
    `auto_dash_names`.
  - *Aggregate*: no `run`, only `depends`, e.g. `server:check`
    (`mise.toml:305-307`).
- **`default` (bare `mise run`)** — `mise.toml:358-360`: depends on
  `build:frontend`, `build:server:dev`, `format:fix`, `lint:check`,
  `types:check`, `test`. (Note: runs `format:fix` mutating, lint/types read-only,
  plus full `test`.)
- **`check`** — `mise.toml:354-356`: depends on the four per-component roll-ups
  `frontend:check`, `server:check`, `build-system:check`, `scripts:check`.
- **`fix`** — `mise.toml:350-352`: `format:fix` + `lint:fix`. `scripts` is
  deliberately absent from `lint:fix` (shell has no autofixer).
- **`server:check`** — `mise.toml:305-307`: `format:server:check` +
  `lint:server:check` (no `types` — clippy covers it).

### How a new Rust component slots into the task tree

The naming contract is documented in `tasks/README.md:8-45` (entity-first for
roll-ups: `server:check`; family-first inside families: `format:server:check`;
no `<component>:fix` roll-ups; only `frontend`/`build-system` have `types:*`).

To add a multi-crate Rust component, the implementer must touch **five places**
(confirmed by the analyser):

1. **Leaf Python modules** mirroring `tasks/format/server.py` +
   `tasks/lint/server.py` (+ optional types). For per-crate generation, iterate
   the member list running either `-p <crate>` passes or per-crate
   `--manifest-path`.
2. **Register** them in `tasks/format/__init__.py:1-3`,
   `tasks/lint/__init__.py:1-3`, and add `Collection.from_module(...)` calls in
   `tasks/__init__.py:54-94`.
3. **mise executor wrappers** (`format:<comp>:check`, `lint:<comp>:check`, and
   per-crate `lint:<comp>:<crate>:check` if desired) mirroring
   `mise.toml:283-303`.
4. **`<comp>:check` aggregate** (`depends`-only) mirroring `mise.toml:305-307`.
5. **Wire into top-level** `check.depends` (`mise.toml:356`) and the family
   aggregates `format:check`/`fix` and `lint:check`/`fix`.

The closest existing template for "one component, several sub-checks rolled into
the component check" is the nested shell-lint example:
`lint:scripts:check` depends on three `lint:scripts:<sub>:check` tasks
(`mise.toml:226-244`). A per-crate `lint:<comp>:<crate>:check` →
`lint:<comp>:check` depends list follows the same grain.

### Current cargo invocations (the leaf pattern to copy)

All cargo calls run from repo-root cwd and locate the crate via
`--manifest-path` (no `cd`):

- **fmt check** — `tasks/format/server.py:11`: `cargo fmt --manifest-path
  {CARGO_TOML} --all -- --check`; runs with `warn=True` then raises
  `Exit(code=1)` (lines 14-18).
- **fmt fix** — `tasks/format/server.py:25`: `cargo fmt --manifest-path
  {CARGO_TOML} --all`.
- **clippy check** — `tasks/lint/server.py:6-30`: a collect-all-failures
  **two-pass** clippy: base `cargo clippy --manifest-path {CARGO_TOML}
  --all-targets`, run once `--all-features` and once default, each `-- -D
  warnings`; both with `warn=True`, then a single `Exit` naming failed configs.
  Gated on `build:frontend:stub` (`mise.toml:295-298`) because clippy on the
  default `embed-dist` feature needs a `dist/index.html` to exist.
- **clippy fix** — `tasks/lint/server.py:33-46`: `cargo clippy --fix
  --allow-dirty --manifest-path {CARGO_TOML} --all-targets` (default features).
- **No `-p <crate>` is used anywhere today** — the per-crate pattern is net-new.

### check vs test separation (where Rust tests + coverage go)

`check` and `test` are disjoint; no `test:*` task is reachable from `check`. The
`test` tree:

- `test` aggregate — `mise.toml:206-208`: `test:unit` + `test:integration` +
  `test:e2e`.
- Rust **unit** — `mise.toml:100-103` → `tasks/test/unit.py:6-24`: two
  `cargo test --lib` runs (one `--no-default-features --features dev-frontend`,
  one default). **Re-derives the manifest literal inline** at `unit.py:19`
  (`repo_root() / "skills/visualisation/visualise/server/Cargo.toml"`) rather
  than importing `CARGO_TOML`.
- Rust **integration** — `mise.toml:128-131` → `tasks/test/integration.py:40-52`:
  `cargo test --tests --no-default-features --features dev-frontend`, also
  re-derives the literal at `integration.py:47`.
- **There is no coverage task anywhere** — `cargo-llvm-cov`/`nextest`/tarpaulin
  appear nowhere; the coverage task is brand-new with no template.

0162's Rust tests + `cargo llvm-cov nextest` coverage task must go under the
`test:` namespace / `tasks/test/` package (e.g. `test:unit:<comp>` wired into
`test:unit`'s depends) and **must not** appear in any `*:check` roll-up or in
`check.depends` — matching the server pattern. Coverage is *collected, not
gated* at this Phase 0 stage (0162 AC, no threshold fails the run).

### tasks/shared/paths.py and version coherence (the multi-crate generalisation)

`paths.py` (`/Users/.../tasks/shared/paths.py`) defines the single manifest
constant once:

- `paths.py:7-8` — `SERVER = VISUALISER / "server"` / `CARGO_TOML = SERVER /
  "Cargo.toml"`. This is the **only** manifest reference; nothing knows about
  workspace membership. The actual server `Cargo.toml` is a plain `[package]`
  with no `[workspace]` table.

Consumers that must generalise (every one assumes a single `[package]` and reads
`data["package"]["version"]` — **no `[workspace].members` parsing exists
anywhere**, confirmed by grep):

1. **Version-coherence read** — `tasks/build.py`:
   - `build.py:30` — `_CARGO_TOML_RELATIVE = CARGO_TOML.relative_to(REPO_ROOT)`.
   - `build.py:49-52` — `_read_cargo_toml_version(root)` reads one manifest's
     `[package].version`. Must iterate members (handling
     `version.workspace = true` inheritance).
   - `build.py:87-103` — `validate_version_coherence` builds a `found` dict with
     a single `"Cargo.toml"` key; must gain one entry per member so a mismatch
     names the offending crate.
2. **Version write** — `tasks/version.py:32-35,58,62`: `_render_cargo_toml`
   parses with `tomlkit` (preserves comments/`[lints.clippy]`) and atomic-writes
   one manifest. **Decided: the version lives at workspace level
   (`[workspace.package].version`), with members inheriting via
   `version.workspace = true`** — so there is a single write site (the root
   `cli/Cargo.toml`), not a per-member fan-out. The read/coherence side
   (`build.py`) must still **enumerate members** so a member that hardcodes its
   own `[package].version` instead of inheriting is caught as a mismatch.
3. **Build invocations** — `build.py:138,150,161-167`: `cargo build` /
   `zigbuild` via `--manifest-path {CARGO_TOML}`; the cross-compile binary path
   derives from `SERVER / "target" / triple / ...` — for a virtual workspace the
   `target/` dir moves to the workspace root, so this depends on the layout.
4. **Test literals** — `tasks/test/unit.py:19`, `integration.py:47`
   independently re-derive the same string (separate hard-codings to keep in
   sync).

`tasks/shared/targets.py` (4 cross-compile triples) and `tasks/deps.py`
(`rustup target add`) reference **no** manifest and need **no** change for the
version-coherence concern — they are toolchain/target concerns, crate-count-
agnostic.

**Test scaffolding to extend**: `tests/conftest.py:11-20` (`fake_repo_tree`
writes exactly one manifest), `tests/unit/tasks/test_build.py:159-204`
(coherence mismatch test), `tests/unit/tasks/test_version.py:26-30,191-224`
(tomlkit round-trip + `test_cargo_and_rustfmt_editions_match` reading
`SERVER_DIR / "Cargo.toml"`).

### CI structure (.github/workflows/main.yml — the only workflow)

- **No aggregate-check job.** CI fans the four leaf `*:check` tasks into four
  single-OS (`ubuntu-latest`) jobs: `check-scripts` (`main.yml:99-115`),
  `check-build-system` (117-133), `check-visualiser-server` (135-164),
  `check-visualiser-frontend` (166-182). These are exactly `check`'s deps in
  `mise.toml:354-356`.
- **Test-OS matrix** (defined inline, duplicated per job, not centralised):
  `os: [ubuntu-latest, macos-latest]`, `fail-fast: false`, `runs-on:
  ${{ matrix.os }}` on `test-unit` (`main.yml:15-35`), `test-integration`
  (37-57), `test-e2e` (59-79). `test-visual-regression` (81-97) is ubuntu-only.
- **Toolchain provisioning** — every job uses `jdx/mise-action@v4.1.0` with
  `install: true`, `cache: true`, `experimental: true`. Versions come from
  `mise.toml`.
- **The Rust toolchain caching workaround** (load-bearing, only on
  `check-visualiser-server`):
  - `main.yml:149-150` — `echo "RUSTUP_HOME=$HOME/.local/share/mise/rustup" >>
    "$GITHUB_ENV"` *before* `mise install`. The comment (143-148) documents the
    race: rust defaults to `~/.rustup` (outside the cache); a cache hit makes
    `mise install` a no-op while the toolchain is physically absent, so parallel
    cargo passes race to auto-install it ("detected conflict: bin/cargo").
  - `main.yml:157-160` — dedicated `cache_key_prefix: mise-server-v1` isolating
    this job's heavier (toolchain-carrying) cache.
  - **The test jobs that also run cargo do NOT have this routing** — they share
    the default cache namespace. Any *new* cargo-running job (Rust check, Rust
    test+coverage, the cargo-pup nightly lane) needs the same `RUSTUP_HOME`
    treatment + its own `cache_key_prefix`.
- **No nightly / second toolchain exists.** The cargo-pup pinned-nightly lane is
  wholly greenfield: a new `mise.toml` rust entry / per-task tool override (or a
  `rust-toolchain.toml`), plus a dedicated single-OS job with its own
  `cache_key_prefix` + `RUSTUP_HOME` routing.
- **Merge gating is out-of-band.** No branch-protection / required-checks config
  is committed (refs appear only in `meta/` planning docs). New required checks
  are configured in GitHub settings, not in-repo. If a check should also gate
  releases, add it to `prerelease.needs` (`main.yml:187-195`).
- **Workflow guard caveat**: `tests/unit/tasks/test_workflows.py:87` asserts
  exactly **two** `accelerator-release` concurrency-lock members — new Rust jobs
  must not carry that concurrency group or that test fails.

### Architecture-enforcement design (ADR-0053 + ADR-0046)

Two non-redundant mechanisms at two granularities:

- **cargo-deny — between crates** (ADR-0053:102-104): ban-lists keep
  infrastructure crates out of the dependency closures of the **light /
  infra-free** crates — the domain subdomain crates plus shared pure crates
  `kernel` and the pure `config` (0162 Requirements; ADR-0053). Infrastructure
  to exclude: `config-adapters` and anything pulling fs / http / serde /
  external-tool wrappers. **Inert in the single-crate state** — first bites at
  the `config`/`config-adapters` split in **0166**.
- **cargo-deny — workspace-wide native-tls/OpenSSL ban** (ADR-0046:105-119):
  rustls-only is a **musl-static-linking prerequisite, not a preference** —
  native-tls breaks the static build. Protects the `default-features = false`
  posture (ADR-0054:144-145). **Independently demonstrable inside 0162**: adding
  a native-tls/OpenSSL dependency must make `cargo deny check` exit non-zero.
- **cargo-pup — inside a crate, module granularity** (ADR-0053:106-111): blocking
  check on a **pinned-nightly lane** (hooks rustc internals); product build +
  all other checks stay on **stable**. In the single-crate start it is the
  **sole** inward-rule enforcer. The lane must be **isolated**: a nightly outage
  fails *only* the architecture check (0162 AC lines 112-115).
- **The grep tripwire is deliberately omitted.** The source research (lines 118,
  379, 465) and 0158 (lines 177-178) proposed a zero-dependency CI grep tripwire
  (forbid `use crate::{adapters,inbound,outbound}` from `domain`) as a floor
  beneath cargo-pup. **ADR-0053:161-163 and 0162 Assumptions reject it** — do
  not add it. The accepted trade-off: a nightly/cargo-pup outage leaves the
  inward rule unenforced until restored.

Neither the quality stack (nextest, llvm-cov, cargo-deny) nor the nightly pin
has a ratifying ADR — they derive from the 0158 spike; ADR ratification is a
possible follow-up. Only the architecture enforcement traces to ADR-0053.

### The shared manifest contract (0158 / 0163 / 0166)

- **Location + format**: `cli/Cargo.toml` at the **repo top level** (single
  top-level `cli/` directory, resolved in 0163 Context), standard
  `[workspace].members` array. Both 0162 mechanisms (per-crate `<crate>:check`
  generation + `paths.py` member resolution) read this single list. Agree it
  with 0163 before either lands.
- **Launcher crate is `launcher`, not `cli`** (0163 deviation, to avoid
  `cli/cli/`) — still produces the `accelerator` binary. Per-crate tasks target
  `-p launcher`.
- **Starting crate set at the 0162+0163 pair's close**: likely just `launcher`
  (+ `kernel`) implementing `accelerator version`. The workspace may be a single
  crate; per-crate criteria are satisfied for whatever members exist, with the
  generation mechanism demonstrated to extend.
- **0162 is not independently demonstrable**: it pairs with 0163 for all
  stable-lane green-build criteria, and its infra-out-of-domain ban only *fires*
  once 0166's config split exists. 0162 ships the rule "armed but inert".

### Co-verification boundary map

| Concern | Encoded/wired by | Demonstrated/co-verified by | Verifiable at |
|---|---|---|---|
| Per-crate `<crate>:check` (rustfmt + clippy) | 0162 | 0163 supplies `launcher`/`kernel` | 0162+0163 pair |
| nextest + llvm-cov coverage (collected, not gated) | 0162 | 0163 test-first `version` test | 0162+0163 pair |
| cargo-deny native-tls/OpenSSL ban | 0162 | 0162 itself (OpenSSL dep fails deny) | 0162 close |
| cargo-deny infra-out-of-domain ban | 0162 (encodes) | 0166 (config split fires it) | 0166 close |
| cargo-pup intra-crate import rule (nightly lane) | 0162 | 0162 (failing domain→adapter fixture) | 0162 close |
| Nightly-lane isolation | 0162 | 0162 itself | 0162 close |
| paths.py multi-member resolution | 0162 | reads 0163's `[workspace].members` | 0162+0163 pair |
| `cli/Cargo.toml` members manifest | 0163 (authors) | consumed by 0162 | authored at 0163 |

## Code References

- `mise.toml:8` — single stable Rust pin (1.90.0 + rustfmt + clippy); no nightly
- `mise.toml:283-307` — server format/lint executor + `server:check` aggregate
- `mise.toml:350-360` — `fix` / `check` / `default` top-level composition
- `tasks/__init__.py:54-94` — invoke Collection wiring (register new modules here)
- `tasks/format/server.py:6-26` — fmt leaf template (`--manifest-path`, `Exit`)
- `tasks/lint/server.py:6-46` — clippy two-pass leaf template, `-D warnings`
- `tasks/shared/paths.py:7-8` — the single hard-coded `CARGO_TOML` constant
- `tasks/build.py:30,49-52,87-103,138,150,161-167` — version-coherence read +
  build invocations (single `"Cargo.toml"` key to multiplex)
- `tasks/version.py:32-35,58,62` — tomlkit version write (single manifest)
- `tasks/test/unit.py:19`, `tasks/test/integration.py:47` — re-derived manifest
  literals; the `test:` tree where Rust tests + coverage belong
- `tasks/README.md:8-45` — task-naming contract
- `.github/workflows/main.yml:135-164` — `check-visualiser-server` + the
  `RUSTUP_HOME` routing (149-150) and `cache_key_prefix` (160) workaround
- `.github/workflows/main.yml:15-79` — test jobs + inline OS matrix
- `tests/conftest.py:11-20`, `tests/unit/tasks/test_build.py:159-204`,
  `tests/unit/tasks/test_version.py:191-224` — test scaffolding to extend
- `skills/visualisation/visualise/server/Cargo.toml` — current single `[package]`,
  no `[workspace]`
- `skills/visualisation/visualise/server/rustfmt.toml` — only existing Rust
  config; 80-col duplicated by hand (no `clippy.toml`/`deny.toml` exist)

## Architecture Insights

- **Roll-ups are pure mise `depends`; Python tasks are leaves.** Adding a
  component's check means new `[tasks."..."]` aggregates in `mise.toml`, not
  Python aggregation code. This keeps the new Rust per-crate generation cleanly
  expressible (loop members → leaf tasks; aggregate in mise).
- **`--manifest-path` over `cd`** is the uniform cargo convention here; per-crate
  scoping is `-p <crate>` (new) against the workspace manifest, or per-crate
  `--manifest-path`.
- **check / test disjointness is a hard invariant** — coverage being slow and
  test-shaped is exactly why it goes in `test:`, never `check`.
- **CI mirrors `mise run check` by fan-out, not by calling the aggregate** — so
  new Rust gates become new jobs (or extensions of `check-visualiser-server`),
  each needing the `RUSTUP_HOME` + `cache_key_prefix` toolchain-race workaround.
- **Two enforcement mechanisms, not one** — cargo-deny (between crates,
  inert until split) vs cargo-pup (inside a crate, the sole enforcer at single-
  crate start, on an isolated nightly lane). Conflating them is the main risk.
- **80-col duplicated by hand** into `rustfmt.toml` (and `clippy.toml`/`deny.toml`
  as created) per ADR-0048 + CLAUDE.md — no automated sync check.

## Historical Context

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  — primary source; the lum 0006 → 0162 mapping ("Extend, not stand-up";
  reference ADR-0048/0053/0054, not lum 0004/0009/0010); proposed the grep
  tripwire that ADR-0053 later rejected; flags the `paths.py` single-Cargo.toml
  revisit. Workspace layout: top-level `cli/`, crates `kernel`/`config`/
  `config-adapters`/`corpus`/`tracker`/subdomains, ~10 binaries.
- `meta/decisions/ADR-0046-zero-setup-static-binary-distribution.md` — rustls-only
  / native-tls breaks musl-static (the cargo-deny ban rationale); version
  coherence obligation.
- `meta/decisions/ADR-0048-four-toolchain-split.md` — Rust as a first-class CI
  lane held to the same bar; 80-col duplicated by hand; pinned tool versions.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
  — the inward-dependency rule; cargo-deny (between) + cargo-pup (inside, nightly);
  deliberate omission of the grep tripwire (lines 161-163).
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
  — multi-binary workspace by bounded context; drives per-crate checks and
  multi-`Cargo.toml` version coherence; `reqwest`+rustls `default-features = false`.
- `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`
  — the spike that chose the tool stack (no ratifying ADR); three-tier
  enforcement; pinned-nightly isolation mandate.
- `meta/work/0163-scaffold-cli-workspace-version-subcommand.md` — paired scaffold;
  authors `cli/Cargo.toml` manifest contract; `launcher` crate rename; supplies
  the code 0162's gates run against.
- `meta/work/0166-shared-config-corpus-store-crates.md` — config/config-adapters
  split; where the infra-out-of-domain ban first bites (co-verifies that AC).
  (Caveat: 0166 frontmatter lacks a machine-visible `relates_to: work-item:0162`
  edge — only prose cross-references exist.)

## Related Research

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  (parent epic 0136 scope/architecture — the seeding research for this story)
- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
  (earlier 0136 migration-surface research)
- `meta/research/codebase/2026-06-27-0157-porting-luminosity-adrs-and-feeding-spikes.md`
  (how the ADRs/work items were ported from luminosity)
- `meta/plans/2026-06-27-0157-port-luminosity-adrs-and-feeding-spikes.md`

## Resolved Decisions

The work item's open questions (lint set, nightly pin) and the version-coherence
shape are now decided (2026-06-29):

- **Clippy lint configuration** — set at the workspace via
  `[workspace.lints.clippy]`, with members opting in through `[lints]
  workspace = true`:

  ```toml
  [workspace.lints.clippy]
  pedantic = { level = "warn", priority = -1 }
  nursery  = { level = "warn", priority = -1 }
  # restriction is allow-by-default; these are the cherry-picked opt-ins.
  unwrap_used    = "warn"
  expect_used    = "warn"
  panic          = "warn"
  dbg_macro      = "warn"
  todo           = "warn"
  unimplemented  = "warn"
  module_name_repetitions = "allow"
  must_use_candidate      = "allow"
  ```

  Note: `pedantic`/`nursery` carry `priority = -1` so the individual
  restriction/allow entries override them. The clippy task keeps the `-D
  warnings` gate (`tasks/lint/server.py` pattern), promoting all these `warn`
  levels to hard failures in CI. Because lints live in the manifest, the
  existing `tasks/test/test_version.py` round-trip guard that already preserves
  a `[lints.clippy]` table stays relevant.

- **cargo-pup pins** — `PUP_NIGHTLY = "nightly-2026-01-22"` and `PUP_VERSION =
  "0.1.8"`, both pinned in `mise.toml`. The nightly drives only the isolated
  cargo-pup lane; the product build + all other checks stay on stable 1.90.0.

- **Version coherence** — the version is kept at **workspace level**
  (`[workspace.package].version` in the root `cli/Cargo.toml`); members inherit
  via `version.workspace = true`. `version.py` therefore writes one site; the
  `paths.py`/`build.py` read side still enumerates members so a member that
  hardcodes its own `[package].version` instead of inheriting is flagged.

## Open Questions

- **Where `target/` lands** for the cross-compile binary-path derivation in
  `build.py:165` once the visualiser folds into the workspace (0168) — a virtual
  workspace moves `target/` to the workspace root. Out of scope for 0162 but
  worth flagging for the 0168 fold.
