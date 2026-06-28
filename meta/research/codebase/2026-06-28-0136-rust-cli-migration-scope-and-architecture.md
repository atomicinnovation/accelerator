---
type: codebase-research
id: "2026-06-28-0136-rust-cli-migration-scope-and-architecture"
title: "Research: Full migration scope and CLI architecture for the shell-to-Rust migration (epic 0136)"
date: "2026-06-28T13:21:27+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0136"
parent: "work-item:0136"
relates_to: ["codebase-research:2026-06-23-0136-shell-scripts-rust-cli-migration-surface"]
topic: "Full migration scope and CLI architecture required to migrate the shell library into the accelerator Rust CLI under the accepted ADRs"
tags: [research, codebase, rust-cli, migration, hexagonal, workspace, distribution, launcher, config, visualiser, adr]
revision: "9ae5baf60c658edc2d2dd6ad3bd442079004be0c"
repository: "accelerator"
last_updated: "2026-06-28T13:21:27+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added Luminosity backlog mapping; resolved all eight open questions (workspace=cli/ + launcher crate rename; in-process tracker core; cache under CLAUDE_PLUGIN_ROOT; domain-modelled hook subcommands via --format; accelerator-corpus grouping + github→collaboration; minisign-only provenance; hybrid test strategy + interface redesign; hooks count=7)"
schema_version: 1
---

# Research: Full migration scope and CLI architecture for the shell-to-Rust migration (epic 0136)

**Date**: 2026-06-28T13:21:27+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 9ae5baf60c658edc2d2dd6ad3bd442079004be0c
**Branch**: HEAD (detached / jj working copy)
**Repository**: accelerator

## Research Question

The accepted foundation ADRs (ADR-0045, 0046, 0047, 0051, 0052, 0053, 0054) now
fix the *target architecture* for migrating the ~226-file bash library backing the
skills into the `accelerator` Rust CLI. Epic 0136 owns this work. Given those
decisions, the prior surface research, and the requirement to **fold the existing
`accelerator-visualiser` server into the CLI**:

1. What is the full scope of work required to migrate to the target architecture
   while adhering fully to the ADRs?
2. What does the Cargo workspace / crate split look like across all domains?
3. What does the `accelerator` subcommand surface look like across all domains and
   behaviours?
4. Where are the gaps?

The output seeds the child work items under epic 0136.

## Summary

The seven ADRs leave **little architectural freedom and a large amount of
construction**. They settle: a git-style `accelerator` launcher dispatching to
on-demand, independently-shipped `accelerator-<sub>` static binaries (ADR-0054);
each sub-binary a hexagonal ports-and-adapters crate with compiler/CI-enforced
inward dependency (ADR-0053); zero-setup distribution as fully static musl/darwin
binaries fetched, sha256+minisign-verified, and exec'd on demand (ADR-0046); a
CLI-native YAML config reader replacing the bash/awk parser with arbitrary
nesting (ADR-0047); skills remaining the product and communicating through the
filesystem (ADR-0051/0052); and the visualiser folding in as the **first concrete
sub-binary**, `accelerator visualiser …` → `accelerator-visualiser` (ADR-0054).

The **architecture spike (work item 0158) is done** and resolved every open
design question — crate split, dispatch, launcher, cross-compile — but **builds
nothing**. There are **no formal children of 0136 yet**; 0157 only ported the
ADRs, and 0159/0160/0161 are an orthogonal skill-evaluation stream. So the entire
construction is open scope.

Three findings shape the decomposition beyond the prior surface research:

- **The visualiser is already 90% workspace-ready.** It is literally the crate
  `accelerator-visualiser` with a `[lib]`/`[[bin]]` split, a `FileDriver` **port
  trait** already in place, and ~15.4k lines of Rust — of which the corpus core
  (frontmatter parsing, doc-type inference, slug/path conventions, typed-linkage,
  work-item-ID logic) **duplicates the bash library exactly**, which is the
  ADR-0045 duplication the shared core exists to collapse. Folding it in is
  mostly relocation + a 3-literal frontend-path fix + consuming shared crates.
- **The distribution half ports cleanly; the launcher half genuinely diverges.**
  The existing `cargo-zigbuild` four-target pipeline, `checksums.json`, version
  coherence, and `gh`-based release tasks reuse almost verbatim. But the existing
  `launch-server.sh` is a *daemon launcher*, not a git-style sub-binary
  dispatcher; minisign does **not exist yet** anywhere; and provenance
  verification is documented but **unimplemented**. The launcher is net-new Rust.
- **The biggest single risk is the invocation-contract rewrite.** Today every
  skill calls bare script paths (`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-*`)
  matched against `allowed-tools` prefix globs. Moving to `accelerator config …`
  requires rewriting **every SKILL.md call site, every `allowed-tools` glob, and
  the 0107 lint guard** in lockstep, behind a stable bash-bootstrap path. This is
  the "single biggest scoping fork" the prior research flagged, now decided in
  favour of one command (ADR-0054) rather than per-script shims.

A recommended **12-phase decomposition**, **crate split**, and **subcommand
surface** follow in the Architecture Insights section.

## Detailed Findings

### What the ADRs fix (the non-negotiable target)

| ADR | Decision that constrains 0136 |
|-----|-------------------------------|
| 0045 | Skills own probabilistic work only; **all** deterministic procedural logic moves to the compiled CLI. The `configure` skill is named as the first proof. |
| 0046 | Zero-setup, fully static binaries; Linux via **musl**, four targets `darwin/linux × arm64/x64`; `cargo-zigbuild`; **rustls throughout** (native-tls breaks musl-static); sha256 **+ minisign** verified in-process; version coherence across `plugin.json` + `Cargo.toml` + manifest; hand-rolled invoke+`gh` release (not cargo-dist). |
| 0047 | `.accelerator/config.md` + `config.local.md`, last-writer-wins; **CLI-native YAML reader** in the hexagonal core; **arbitrary nesting** (drops the 2-level cap); `accelerator config get/set`; carries forward ADR-0020 (per-skill dirs) and ADR-0021 (template subcommands). Supersedes ADR-0016/0017. |
| 0048 | Rust owns the domain core **and the logic behind hooks**; shell is driven to **thin wrappers only**; Python stays as build/test tooling. |
| 0049 | Bash 3.2 floor is **reinforced, not retired** — any surviving wrapper/bootstrap must still honour it. Migrating logic into Rust removes the floor's reach over that logic. |
| 0051 | Skills remain the product on the Claude Code runtime; the CLI is a complement, invoked via the `!` preprocessor. |
| 0052 | Filesystem stays the message bus; the CLI must preserve the on-disk contract (paths, frontmatter, JSONL) exactly. Ephemeral data under `.accelerator/tmp`. |
| 0053 | Each subdomain is a hexagon: domain+application core, ports as traits (inbound/driving + outbound/driven), thin adapters, composition root. Inward dependency enforced by crate boundaries + `cargo-deny` + **`cargo-pup`** on a pinned-nightly lane. |
| 0054 | `accelerator` launcher + on-demand `accelerator-<sub>` binaries; clap `external_subcommand`; **Unix `exec` only**; `version` and `config` **built-in**; crates `kernel`/`config`/`config-adapters`/`cli`; `reqwest`+rustls workspace-wide; uv-style resolve-once-and-cache; thin bash bootstrap fetches the launcher; no self-update; visualiser is the first sub-binary. |

### The architecture spike (0158) already decided the foundations

0158 is **done** and is research-only — it consumes no implementation scope but
constrains it. Its recommendations are the source of ADR-0053/0054:

- **§1 crate split**: subdomain-first, hexagon-within; workspace mandatory;
  `kernel` (dependency-light cross-cutting), per-subdomain crates (start single,
  split later), shared `config`/`config-adapters` (Model 1: each sub-binary wires
  its own adapters; Model 2 reserved), `cli` launcher. Enforcement = crate graph +
  `cargo-deny` + `cargo-pup` (blocking, **pinned-nightly lane** in `mise`) + a CI
  grep tripwire.
- **§2 dispatch**: clap derive `#[command(external_subcommand)] External(Vec<OsString>)`;
  Unix `exec`; manifest-`description`-driven help; `version`+`config` built-in.
- **§3 launcher**: fetch→verify→cache→exec **in Rust**; thin bash bootstrap for
  the launcher itself; `reqwest`+rustls; sha256 re-verify-every-launch + minisign;
  no self-update.
- **§4 cross-compile/distribution**: `cargo-zigbuild`, reuse invoke+`gh`, minisign
  via `minisign-verify` (pure-Rust, embed pubkey in launcher), Sigstore parked.
  **Deferred to construction**: exact cache/bin dir path, minisign key
  lifecycle, final clap-derive confirmation.

**No work item is parented to 0136.** 0157 (port ADRs, done) and 0159/0160/0161
(Inspect skill-eval, orthogonal) do not cover any migration construction.

### Visualiser server — ready to fold in (`skills/visualisation/visualise/server/`)

- **Crate**: already named `accelerator-visualiser`, version `1.24.0-pre.1`,
  edition 2021, MSRV 1.85; `[lib]` + `[[bin]]` split with a thin `main.rs`
  (`Cargo.toml:2,17-21`). Binary literally `accelerator-visualiser`.
- **Features**: `default = ["embed-dist"]`; `embed-dist` pulls `rust-embed` +
  `mime_guess`; `dev-frontend` serves from disk (`Cargo.toml:12-15`). Embed/disk
  is a **compile-time** switch (`assets.rs:20-29`), not a flag.
- **Entry/args**: single `--config <PATH>` flag pointing at the launcher-written
  `config.json` (`main.rs:7-13`); binds an **ephemeral loopback port**
  (`server.rs:310`); redirects stdout/stderr to `/dev/null` and self-daemonises
  (writes its own `server.pid`/`server-info.json`).
- **Async/TLS**: tokio multi-thread; axum 0.7 (http1 only); **no TLS in the
  shipped server** — loopback binding + Host/Origin guards instead; `reqwest`
  (rustls) is a **dev-dependency only** (`Cargo.toml:58`). So the visualiser adds
  no production TLS stack; the launcher's rustls is the only production TLS.
- **~15,434 lines** across 39 files. Largest: `indexer.rs` (3,706),
  `watcher.rs` (1,160), `clusters.rs` (1,067), `file_driver.rs` (1,025),
  `server.rs` (979), `config.rs` (835), `frontmatter.rs` (738), `slug.rs` (613).
- **Hexagonal seam already present**: `file_driver.rs:53` defines a `FileDriver`
  **trait** (port) with a `LocalFileDriver` adapter.
- **Duplicates the shell library** (the ADR-0045 dedup target):
  - `frontmatter.rs` — YAML fence detection + `serde_yml` parse (with
    `catch_unwind` against libyml panics); twin of the bash frontmatter parsers.
  - `docs.rs` — `DocTypeKey::config_path_key` doc-type→config-key map; twin of
    `scripts/doc-type-inference.sh` (the bash header itself flags the
    triplication, `doc-type-inference.sh:24-29`); `prs`→`pr-descriptions` rename
    hardcoded both sides.
  - `slug.rs` — ADR/work-item/dated/review filename conventions; its tests
    **shell out to `work-item-pattern.sh --compile-scan`** to stay in lock-step
    (`slug.rs:572-588`).
  - `config.rs` `WorkItemConfig` — `scan_regex`/`extract_id`/`normalise_id`; twin
    of `skills/work/scripts/work-item-pattern.sh`.
  - `indexer.rs:903-955` path normalisation/escape; `typed_ref.rs` /
    `cluster_key.rs` typed-linkage (ADR-0034) twin of `linkage-parser.sh`.
- **Frontend coupling**: `build.rs` hard-requires `../frontend/dist/index.html`
  under `embed-dist`; the `../frontend/dist` literal is **duplicated in 3 places**
  (`build.rs:5`, `assets.rs:9`, `assets.rs:71`) — a relocation hazard.

### Distribution / launcher / release (current state)

- **Dispatcher**: `visualiser.sh:16-25` routes `start|stop|status` to
  `launch-server.sh` / `stop-server.sh` / `status-server.sh`.
- **Launch pipeline** (`launch-server.sh`): reuse short-circuit via PID +
  `start_time` recycle guard (`:29-42`); init sentinel (`:44-52`);
  `flock`-or-`mkdir` lock (`:57-76`); platform detection → `{darwin,linux} ×
  {arm64,x64}` (`:80-93`); **tri-precedence binary resolution** env
  (`ACCELERATOR_VISUALISER_BIN`) → config (`visualiser.binary`) → cached/download
  (`:104-168`); owner-PID + `start_time` handshake for idle shutdown
  (`:170-179`); `write-visualiser-config.sh` emits `config.json`; `nohup … &
  disown` then poll ≤5s for the server's info file.
- **Cache + manifest share `bin/`**: `BIN_CACHE="$SKILL_ROOT/bin/accelerator-visualiser-${OS}-${ARCH}"`,
  `MANIFEST="$SKILL_ROOT/bin/checksums.json"` (`launch-server.sh:100-101`). The
  cache staying **under `${CLAUDE_PLUGIN_ROOT}`** is now a resolved requirement (see
  Open Q3) — it is what keeps `allowed-tools` permission matches working; a
  user-level (XDG) cache outside the plugin root would break them.
- **Verification**: reads `sha256:` from `binaries["<os>-<arch>"]`, **all-zeros
  digest = "no binary for this version" sentinel** (dies), manifest-vs-plugin
  version drift guard, downloads to a temp part-file, **re-hashes and compares**,
  `install -m 0755`. `download_to` pins TLS (`curl --proto '=https' --tlsv1.2`,
  32 MB cap).
- **`checksums.json`**: `{version, note, binaries{<os>-<arch>: "sha256:<hex>"}}`.
  **No SLSA/provenance fields.**
- **Release tasks (Python invoke)**: `build:server:cross-compile` →
  `cargo zigbuild --release --target <triple>` with Mach-O/ELF magic-byte checks;
  `build:checksums` → `compute_sha256` (`tasks/shared/hashing.py:5-10`) +
  `update_checksums_json` (`build.py:74-84`); `validate_version_coherence`
  (`build.py:87-103`) compares `plugin.json` / `Cargo.toml` / `checksums.json`,
  called twice; `tasks/github.py` `upload_and_verify` re-downloads and re-verifies
  every asset before un-drafting, preserving the draft on verification failure.
- **Targets**: `tasks/shared/targets.py:1-6` — `aarch64/x86_64-apple-darwin`,
  `aarch64/x86_64-unknown-linux-musl`.
- **Toolchain**: `mise.toml:8` rust `1.90.0`; **zig/cargo-zigbuild are PyPI deps**
  in `pyproject.toml:20-21` (not mise); `rustup target add` for the four triples.
  **No minisign tooling anywhere** (confirmed). **Provenance verification is
  documented in `RELEASING.md` but not implemented** in any launcher script —
  SLSA exists only as CI attestations.

### Config / extension surface (the built-in `config` command's spec)

The CLI's built-in `config` command must reach feature parity with this surface
before the bash reader retires (ADR-0047 negative). All build on
`config-common.sh` + `config-defaults.sh`:

- **Readers**: `config-read-value.sh <key> [default]` (the primitive; one line
  out), `config-read-path.sh <key> [default]`, `config-read-context.sh`,
  `config-read-agents.sh` (bulk `## Agent Names` table),
  `config-read-agent-name.sh <role>` (single resolved `subagent_type`),
  `config-read-template.sh <key>` (fenced), `config-read-doc-type-paths.sh [root]`
  (13 `type<TAB>dir` lines under `LC_ALL=C`), `config-read-work.sh`,
  `config-read-review.sh <pr|plan|work-item>`, `config-read-all-paths.sh`,
  `config-dump.sh` (with `local`/`team`/`default` source attribution),
  `config-summary.sh` (SessionStart), `config-read-browser-executor.sh`.
- **Per-skill (ADR-0020)**: `config-read-skill-context.sh <skill>` →
  `## Skill-Specific Context`; `config-read-skill-instructions.sh <skill>` →
  `## Additional Instructions`; both silent-exit when absent; injection at two
  fixed sites; `KNOWN_SKILLS` scanned dynamically (excluding `configure`);
  unknown dirs are advisory, not fatal.
- **Template management (ADR-0021)**: five actions `list|show|eject|diff|reset`
  with **exit-code contract 0=ok / 1=error / 2=destructive-needs-confirm**;
  three-tier resolution (config path → user override `.accelerator/templates/` →
  plugin default `templates/`); 13 plugin-default templates; raw (`show`) vs
  fenced (`read`) output split.
- **Precedence**: team `config.md` then local `config.local.md`, last-writer-wins;
  legacy `.claude/accelerator.md` only under `ACCELERATOR_MIGRATION_MODE=1`;
  `config_assert_no_legacy_layout` otherwise hard-stops.
- **Parser limits removed by ADR-0047**: the hand-rolled awk reader is frontmatter
  only, **2-level `section.key` cap**, string-compare (anti-injection), no list/
  block parsing. The native reader drops the cap and gains real YAML.
- **Recognised keys**: `paths.*` (17 keys), `templates.*` (6), `work.*`
  (`integration` ∈ {jira,linear,trello,github-issues}, `id_pattern`,
  `default_project_code`), `review.*` (~11 incl. `core_lenses`, severities,
  counts), `agents.*` (9 roles, default `accelerator:<role>`). Plus body =
  project context, per-skill files, and `.accelerator/lenses/*/SKILL.md`.
- **No `config set` exists today** — config is hand-edited; the
  `config_set/upsert_frontmatter_field` primitives serve only Jira/Linear
  create-issue writeback. ADR-0047 names `config set`, so this is **net-new**.

### Shell surface (refreshed) and the tooling to retire

- Inventory is **stable vs 2026-06-23** (scripts 63, work 28, jira 43, linear 24,
  migrate 23, design 14, visualiser 13, github 6, decisions 3, config-init 2),
  with **one delta: `hooks/` now shows 6 tracked `.sh`, not 7** — confirm whether
  a hook was removed/renamed.
- **Domain→script coupling**: every context-injecting skill calls the shared
  `config-read-*` cluster + `artifact-derive-metadata.sh`; domain clusters layer
  on top (`work-item-*`, `jira-*`, `linear-*`, `adr-*`, design `scripts/*`,
  `vcs-*`, github `pr-*`, migrate `run-migrations.sh`+`interactive-*`,
  visualiser `visualiser.sh`).
- **`allowed-tools` glob taxonomy** to preserve a match for: (a) prefix globs on
  `scripts/` (`config-*`, `artifact-*`), (b) whole-directory globs (`<dir>/*`,
  incl. nested `playwright/*`), (c) one catch-all (`scripts/*` in vcs/commit).
- **CI/build tooling the migration retires** (anchors): bashisms linter
  (`scripts/lint-bashisms.sh`, task `tasks/lint/scripts.py:86`); exec-bit
  invariant + `SHELL_LIBRARIES` (`tasks/lint/scripts.py:18,100`); shell-suite
  discovery + floors (`tasks/test/helpers.py:17`, `tasks/test/integration.py:8-36`
  — migrate≥4, config≥19, work≥6, integrations≥32 + by-name gates); shfmt
  (`tasks/format/scripts.py:9`); shellcheck (`tasks/lint/scripts.py:70`);
  `shell_sources()` (`tasks/shared/sources.py:60`); `.shellcheckrc`; `[*.sh]`
  editorconfig block (`.editorconfig:36-39`); `check-scripts` CI job
  (`.github/workflows/main.yml:99`, a release gate at `:192`).

## Code References

- `skills/visualisation/visualise/server/Cargo.toml:2,12-21,57-69` — crate/binary
  name, features, dev-deps, release profile.
- `skills/visualisation/visualise/server/src/file_driver.rs:53` — existing
  `FileDriver` port trait (hexagonal seam).
- `skills/visualisation/visualise/server/src/{frontmatter,docs,slug,config,typed_ref,cluster_key}.rs`
  — domain core duplicating the shell library.
- `skills/visualisation/visualise/server/build.rs:5`,
  `.../src/assets.rs:9,71` — the triplicated `../frontend/dist` literal.
- `skills/visualisation/visualise/scripts/launch-server.sh:29-218` — the daemon
  launcher (tri-precedence resolution, sha256 verify, nohup+poll).
- `skills/visualisation/visualise/bin/checksums.json` — manifest shape + all-zeros
  sentinel.
- `tasks/build.py:60-191` — cross-compile, checksums, version coherence.
- `tasks/shared/targets.py:1-6` — four target triples (musl Linux).
- `tasks/github.py:136-178` — upload + re-download-and-re-verify release flow.
- `mise.toml:8`; `pyproject.toml:20-21` — rust pin; zig/zigbuild as PyPI deps.
- `scripts/config-common.sh:27-67,399-439` — precedence, legacy guard, template
  resolution.
- `scripts/config-defaults.sh:26-115` — the recognised-key registry.
- `scripts/config-read-value.sh:33-130` — the 2-level awk reader + read loop.
- `tasks/lint/scripts.py:18,86,100`; `tasks/test/integration.py:8-36` — tooling to
  retire (SHELL_LIBRARIES, bashisms, exec-bits, suite floors).
- `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`
  — the done architecture spike (source of ADR-0053/0054).

## Architecture Insights

### Proposed Cargo workspace / crate split

Two orthogonal axes (per ADR-0053/0054 and spike §1): a **binary axis**
(one crate per independently-shippable sub-binary) and a **layering axis**
(hexagonal layers as modules within each subdomain crate, split into crates only
under pressure).

**Workspace location (resolved, 2026-06-28).** A single top-level **`cli/`**
directory holds the entire Rust workspace (`cli/Cargo.toml` is the workspace root)
**plus the visualiser frontend**, so skill directories hold only skills, not their
compiled/built implementations. The visualiser `server`+`frontend` pair moves as a
unit to `cli/visualiser/{server,frontend}`, preserving their relative layout so the
three `../frontend/dist` embed literals are unchanged. **Deviation from ADR-0053/
0054**: the launcher crate is renamed `cli` → **`launcher`** to free the `cli/`
directory name and avoid a `cli/cli/` path (the crate produces the `accelerator`
binary regardless; record the deviation in the scaffold work item).

```
cli/
  Cargo.toml            # [workspace] members = ["*", "visualiser/server"]
  launcher/             # the `accelerator` launcher binary (was crate `cli`)
  kernel/  config/  config-adapters/
  corpus/  corpus-adapters/  tracker/      # shared lib crates
  corpus-cli/           # accelerator-corpus binary over the corpus lib
  vcs/  work/  jira-client/  linear-client/  migrate/  design/  collaboration/
  visualiser/
    server/             # accelerator-visualiser crate
    frontend/           # Vite SPA; embed path "../frontend/dist" UNCHANGED
```

**Cross-cutting + shared library crates (not binaries):**

- **`kernel`** — error taxonomy, the config-access + dispatch/launcher contract
  traits, logging. Deliberately dependency-light; everything links it.
- **`config`** — configuration domain + application + ports (precedence
  resolution, key catalogue, review/work/agent resolution). Light deps.
- **`config-adapters`** — outbound config readers (native YAML/`serde`, filesystem).
  Wired at each composition root (Model 1).
- **`corpus`** — the **big ADR-0045 dedup**: frontmatter parse, doc-type
  inference, typed-linkage (ADR-0034), slug/path conventions, work-item-ID logic,
  artifact-metadata derivation. Shared by the visualiser, `accelerator-work`, and
  the `accelerator-corpus` binary, collapsing the bash↔Rust duplication into one
  implementation.
- **`corpus-adapters`** (or `store`) — atomic JSONL writes, the `mkdir`-lock +
  PID-owner reclaim, filesystem I/O. Used by migrate + work sync.
- **`tracker`** — `RemoteTracker` port + the work-item sync state machine (pure
  domain). **`jira-client`** / **`linear-client`** — `reqwest`/serde adapter crates
  implementing `RemoteTracker` (Jira REST + ADF; Linear GraphQL). See the resolved
  coupling decision below.

**Binary crates (`accelerator-<sub>`), each its own composition root:**

- **`launcher`** (was `cli` in ADR-0053/0054) → the `accelerator` launcher binary.
  Built-in `version` + `config` (depends on `kernel` + `config`/`config-adapters`);
  external dispatch for all else. Owns fetch→verify→cache→exec (`reqwest`+rustls,
  sha256, `minisign-verify`).
- **`accelerator-visualiser`** — the existing server, relocated, consuming
  `corpus`/`config`; heavy deps (axum, tokio, notify) stay isolated here.
- **`accelerator-vcs`** — VCS detection/status/log/guard; also backs the hooks.
- **`accelerator-work`** — work-item lifecycle + sync engine; links `tracker` +
  both client crates in-process (see the resolved coupling decision below).
- **`accelerator-jira`**, **`accelerator-linear`** — thin CLI binaries over their
  `jira-client`/`linear-client` crate; `reqwest`+serde replace `jq`/`curl`.
- **`accelerator-migrate`** — the migration engine (highest-risk port).
- **`accelerator-design`** — design inventory/gap tooling (Playwright-adjacent).
- **`accelerator-collaboration`** — PR helpers (`pr-base-repo`, `pr-update-body`);
  shells to `gh`. Domain named `collaboration` per the open github→collaboration
  rename, not `github`.
- **`accelerator-corpus`** — the light corpus-ops bounded context grouped into one
  binary (resolved Q5): ADR numbering/status, artifact-metadata derivation,
  corpus-frontmatter validation, typed-linkage queries. A thin inbound CLI over the
  shared `corpus`/`corpus-adapters` crates; invoked as `accelerator corpus <sub>`.
  No dependency-bleed rationale applies (all light), and these are facets of one
  context — operations over `meta/`. Split out only if a dependency profile
  diverges.

**Enforcement**: crate graph + `cargo-deny` ban-lists keep infra out of light
crates; `cargo-pup` on a pinned-nightly `mise` lane enforces module-level inward
direction; a CI grep tripwire forbids `use crate::{adapters,inbound,outbound}`
from `domain`. Product build + all other checks stay on stable.

**work ↔ integrations coupling (resolved, 2026-06-28).** Jira and Linear each play
two roles — standalone user-facing skills *and* the remote backing for work-item
sync — so each gets its own binary, and the HTTP/auth/serialisation logic lives in
a **shared library crate per provider** reused by both:

- **`tracker`** — domain crate holding the **`RemoteTracker` port (trait)** and the
  sync state machine (classify → decide → apply → baseline → label) in pure domain
  terms. The port lives **here, in its own crate** — not in `kernel`, which stays
  minimal (the tracker port carries domain vocabulary: issue, transition, sync
  verdict).
- **`jira-client`** / **`linear-client`** — adapter crates: `reqwest`/serde Jira
  REST (+ ADF↔markdown) and Linear GraphQL (+ auth); each `impl RemoteTracker`.
- **`accelerator-jira`** / **`accelerator-linear`** — thin CLI binaries over their
  client crate (the standalone skills).
- **`accelerator-work`** — links `tracker` + both client crates, wires the active
  one per `work.integration` at its composition root, and **fakes `RemoteTracker`
  in tests**.

Sync runs **in-process** (not subprocess dispatch): the state machine is
transactional and wants in-process orchestration + port-faking for tests, and
`reqwest`+rustls+tokio is already workspace-wide via the launcher, so this is not
the heavy-dependency bleed ADR-0054 guards against. The cost — the `work` binary
carries `reqwest` + both clients — is accepted as light.

### Proposed `accelerator` subcommand surface

The `# was <script>.sh` annotations below map **behaviour**, not literal CLI shape:
per the resolved Q7 interface-redesign principle, each entrypoint's args/output
should be reconsidered (clap named args, `--format`, structured output for
machine-like consumers) rather than transliterated, while preserving
prose-for-injection outputs and the meaningful 0/1/2 exit codes.

**Built-in (compiled into the launcher; no sub-binary fetch — config is the
hottest path at skill-load time):**

```
accelerator version
accelerator config get <key> [default]          # was config-read-value.sh
accelerator config set <key> <value>            # NET-NEW (ADR-0047)
accelerator config path <key> [default]         # was config-read-path.sh
accelerator config paths                         # was config-read-all-paths.sh
accelerator config context                       # was config-read-context.sh
accelerator config agents                        # was config-read-agents.sh (table)
accelerator config agent <role>                  # was config-read-agent-name.sh
accelerator config template <key>                # was config-read-template.sh (fenced)
accelerator config templates list|show|eject|diff|reset [key]   # ADR-0021 (0/1/2 exits)
accelerator config doc-type-paths [root]         # was config-read-doc-type-paths.sh
accelerator config work <key>                    # was config-read-work.sh
accelerator config review <pr|plan|work-item>    # was config-read-review.sh
accelerator config dump                          # was config-dump.sh
accelerator config summary                       # was config-summary.sh (SessionStart)
accelerator config skill-context <skill>         # ADR-0020
accelerator config skill-instructions <skill>    # ADR-0020
accelerator config browser-executor              # was config-read-browser-executor.sh
accelerator config init                          # was init.sh scaffold
```

**External on-demand subcommands (each an `accelerator-<sub>` binary):**

```
accelerator visualiser start|stop|status         # folds in visualiser.sh + server
accelerator vcs detect|status|log|guard           # vcs-common cluster + hooks backing
accelerator work create|fetch|update|sync|normalise|next-number|read-field|section-diff|...
accelerator jira create|update|comment|transition|search|show|attach|init
accelerator linear create|update|comment|transition|search|show|attach|init
accelerator migrate run|status|...
accelerator design inventory|gaps|...
accelerator collaboration pr-base-repo|pr-update-body|...   # github→collaboration rename
accelerator corpus adr next-number|adr read-status|metadata derive|validate|linkage
```

clap cannot enumerate external subcommands, so the synthesised help reads the
release manifest's `description` field; per-command `--help` is delegated by
re-exec.

### Recommended phased decomposition (seed for 0136 children)

Ordered by the dependency spine (leaf/shared → readers → subdomains → engine →
retirement). Each phase keeps the plugin functional (0136's AC).

0. **Workspace scaffold + enforcement** — create the workspace, `kernel`, wire
   `cargo-pup` (pinned-nightly mise lane), `cargo-deny`, grep tripwire, clippy/
   rustfmt/test into `mise run check` + CI.
1. **Launcher + dispatch (`cli`)** — clap `external_subcommand`, Unix `exec`,
   built-in `version`; fetch→verify→cache→exec (reqwest+rustls, sha256 re-verify,
   `minisign-verify`); thin bash bootstrap (bash-3.2-safe) that fetches the
   launcher; cache-dir decision; manifest-driven help.
2. **Distribution + release pipeline** — extend invoke tasks for **multiple**
   binaries (per-sub cross-compile, per-binary checksums + minisign signing,
   manifest `description` field); version coherence across `plugin.json` + **every
   crate `Cargo.toml`** + manifest; minisign key lifecycle (generate/store/rotate,
   embed pubkey); implement (or formally drop) provenance verification.
3. **Shared `config`/`config-adapters` + `corpus`/`corpus-adapters`/`store`** —
   native YAML reader (arbitrary nesting), full reader logic, frontmatter/doc-type/
   typed-linkage/slug/work-item-ID, atomic JSONL + lock. These dedup with the
   visualiser.
4. **Built-in `config` command + invocation-contract migration** — wire the full
   `config` surface (incl. ADR-0020/0021, agents dual strategy, `config set`);
   **rewrite every SKILL.md call site** + `allowed-tools` glob; update the
   **0106/0107** contract + lint guard; move the SessionStart summary into the
   CLI. `configure` skill is the first proof (ADR-0045). *Highest blast radius.*
5. **Refactor visualiser onto shared crates** — relocate into the workspace, fix
   the 3 frontend-path literals, consume `corpus`/`config`, move start/stop/status
   orchestration into `accelerator visualiser`, retire `launch-server.sh` in
   favour of the unified launcher (preserve the owner-PID/`start_time`/idle
   lifecycle and the loopback+Host/Origin security model).
6. **VCS subdomain + hooks** — `accelerator-vcs`; migrate hook *logic* into the
   CLI (ADR-0048). `hooks.json` registers the universal thin bash **wrapper**
   (which fetches `accelerator` on first use, then execs it) invoking **domain**
   subcommands — SessionStart → `accelerator vcs detect` / `accelerator config
   detect` / `accelerator migrate discoverability`; PreToolUse(`Bash`) →
   `accelerator vcs guard` — with the hook I/O envelope produced by a
   **`--format=hook`** switch on the CLI (so one operation serves both its skill
   caller and its hook caller). No hook-specific subcommand. Wrapper stays
   bash-3.2-safe.
7. **Work-item subdomain** — `accelerator-work` lifecycle + sync; resolve the
   work↔integration coupling decision. Close the ~14-script coverage gap.
8. **Integrations** — `accelerator-jira`, `accelerator-linear`; carry the Python
   mock servers over as integration-test scaffolding; remove `jq`/`curl`
   `allowed-tools`.
9. **Migration engine** — `accelerator-migrate`; `serde_json` removes the dual
   escape hazard; replace FIFO/watchdog/awk-JSON with Rust. Last per the spine.
10. **Remaining subdomains** — `accelerator-corpus` (the grouped light corpus ops:
    ADR numbering/status, artifact metadata, validation, linkage), `accelerator-design`,
    and `accelerator-collaboration` (the renamed github PR helpers).
11. **Tooling retirement** — as each cluster's last script disappears: shrink
    `SHELL_LIBRARIES`, decrement suite floors **in lockstep** (avoid green→red),
    remove bashisms/exec-bit/shfmt/shellcheck/`.shellcheckrc`/`[*.sh]` editorconfig,
    finally `shell_sources()` + the `check-scripts` CI job.

Cross-cutting: a **test strategy** (resolved Q7) — develop each cluster test-first
in `cargo test` (the destination), and where a shell suite exists **repoint it at
the binary as a black-box parity gate** during cutover, retiring the suite +
decrementing the Python floor in the **same change** that deletes the scripts;
characterize-then-port the untested clusters.

Cross-cutting: **interface redesign, not transliteration** (resolved Q7). Moving
off bash frees us from positional-only args, exit-code-as-signal hacks (e.g.
`config-diff-template.sh` exit 2 = "no customisation"), and fragile TSV/markdown
stdout. Because the Phase 4 call-site rewrite touches every invocation anyway, each
entrypoint's interface should be **reconsidered** — clap named args + subcommands,
a `--format` switch, and **structured output where the consumer is machine-like**
(e.g. `next-number`, path resolution) — rather than copied 1:1. Two guardrails:
(a) preserve outputs that are deliberately **prose-for-injection** (the `## Agent
Names`, `## Project Context`, `## Review Configuration` blocks the `!` preprocessor
injects for the model to read); (b) keep the **semantically meaningful exit codes**
(ADR-0021's 0/1/2) but express them through typed errors. Where an interface
*changes*, the parity gate moves from literal CLI shape to **behavioural/semantic**
parity, and the shell suite is updated to the new interface (or re-ported)
accordingly.

### Luminosity backlog mapping (foundational children to mirror)

Luminosity has already decomposed the same foundations under its baseline epic
(lum work-item 0001). Its children map directly onto Phases 0–4 here and should be
**ported as structure, then rewritten where Accelerator diverges** — Accelerator is
a brownfield migration with an existing distribution pipeline, and two of its ADRs
contradict the Luminosity slice text.

| Luminosity item | Accelerator phase | Port as | Required divergence |
|-----------------|-------------------|---------|---------------------|
| lum 0006 — Rust toolchain guard rails in mise + CI | Phase 0 | Foundational child of 0136 | **Extend**, not stand-up: rust/rustfmt/clippy already in `mise`; add cargo-nextest, cargo-llvm-cov, cargo-deny, cargo-pup (pinned-nightly lane), per-crate `<crate>:check`. Reference ADR-0048/0053/0054, not lum 0004/0009/0010. `tasks/shared/paths.py` hard-codes a single `Cargo.toml` — same revisit lum 0006 flags. |
| lum 0007 — Scaffold hexagonal workspace + `version` | Phase 0→1 | Foundational child of 0136 | Ports cleanly: `accelerator version` built-in, subdomain-first workspace, thin `kernel` + `cli`, test-first, in-process dispatch only (external dispatch deferred to the launcher child). Workspace-location decision (Open Q1) lands here. |
| lum 0008 — On-demand static-binary distribution & launcher | Phases 1 + 2 | Foundational child of 0136 (likely split in two) | **Materially bigger here.** lum 0008 says "via `dist`" and "**no minisign**, checksums + optional Attestations"; Accelerator ADR-0046/0054 **reject cargo-dist** (reuse the hand-rolled invoke pipeline) and **mandate minisign** (`minisign-verify` in-process, pubkey embedded). Adds a minisign key-lifecycle thread lum omits, and must **fold the visualiser in as the first sub-binary** and retire `launch-server.sh`. |
| lum 0009 — Multi-level configuration system | Phase 3–4 | Later child | Shared `config`/`config-adapters` crate + native YAML reader (ADR-0047). |
| lum 0011 — Configuration feature parity with Accelerator | Phase 4 | Later child | The built-in `config` command reaching parity with the current bash reader surface; here it is parity with *our own* shell library + the call-site/`allowed-tools`/0107 rewrite. |
| lum 0012 — Cross-crate architecture enforcement as the workspace grows | Cross-cutting (Phase 3+) | Later child | Where the cargo-deny ban-lists stop being inert — first bites at the `config`/`config-adapters` split. |

Net: mirror lum 0006/0007/0008 as the first three (likely four — 0008 splits into
launcher + distribution) foundational children of 0136, rewritten per the
divergences above; the per-cluster shell-migration items (Phases 5–11) layer on
top. Luminosity's 0008 text is itself stale relative to its own ADR-0010 on the
minisign point — follow Accelerator's ADRs, not lum 0008's slice text.

## Historical Context

- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
  — the prior surface research (8 functional clusters, the invocation contract,
  test-suite oracle, tooling-retirement list). This document extends it with the
  ADR-fixed target, the visualiser-folding analysis, and the construction WBS.
- `meta/work/0158-…-hexagonal-workspace-layout.md` — done architecture spike.
- `meta/work/0157-port-luminosity-adrs-and-feeding-spikes.md` — ported the ADRs
  (done); explicitly out of scope for building anything.
- ADR-0045/0046/0047/0051/0052/0053/0054 (accepted foundations); ADR-0048
  (four-toolchain split), ADR-0049 (bash 3.2 floor), ADR-0020 (per-skill dirs),
  ADR-0021 (template subcommands) — in force.
- `meta/work/0106-invoke-plugin-scripts-by-bare-path.md`,
  `0107-lint-skill-body-script-invocations.md` — the invocation-contract precedent
  that Phase 4 must update.

## Related Research

- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md`
- `meta/research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md`

## Open Questions

**All eight resolved interactively on 2026-06-28** (kept here with their rationale
as a decision log; the resolutions are reflected throughout the sections above).

1. **Workspace location** — *Resolved (2026-06-28):* a single top-level `cli/`
   directory holds the whole Rust workspace **and** the visualiser frontend; the
   visualiser `server`+`frontend` pair moves as a unit to
   `cli/visualiser/{server,frontend}` (embed literals unchanged); the launcher
   crate is renamed `cli` → `launcher` (deviation from ADR-0053/0054, noted in the
   scaffold work item). Skill directories hold only skills, not their compiled
   implementations.
2. **work ↔ integrations coupling** — *Resolved (2026-06-28):* shared `tracker`
   core crate (holding the `RemoteTracker` port + sync state machine) with
   per-provider `jira-client`/`linear-client` adapter crates; `accelerator-work`
   links them **in-process** and fakes the port in tests; the standalone
   `accelerator-jira`/`-linear` binaries are thin CLIs over the same client crates.
   The port lives in its own `tracker` crate, not `kernel`.
3. **Cache/bin directory** — *Resolved (2026-06-28):* fetched binaries cache
   **under `${CLAUDE_PLUGIN_ROOT}`** (the plugin install dir), **not** a user-level
   XDG cache — because the bare-path invocation contract matches `allowed-tools`
   globs against `${CLAUDE_PLUGIN_ROOT}` paths, and a cache outside it would break
   the permission match. `${CLAUDE_PLUGIN_ROOT}` is version-scoped, so a new plugin
   version gets a fresh cache and **redownloads** — correct, since binaries are
   version-coherent with the plugin. (Plugin cache dirs are writable in practice —
   the current launcher already downloads into `$SKILL_ROOT/bin/`.)
4. **Hooks: shim vs in-binary** — *Resolved (2026-06-28):* `hooks.json` registers
   the universal thin bash **wrapper** (which fetches `accelerator` on first use,
   then execs it), invoking **domain** subcommands rather than any hook-specific
   subcommand — SessionStart → `accelerator vcs detect` / `config detect` /
   `migrate discoverability`; PreToolUse(`Bash`) → `accelerator vcs guard`. The
   Claude Code hook I/O envelope is produced by a CLI **`--format=hook`** switch
   (preferred over wrapper-side shaping), so one domain operation serves both its
   skill-injection caller and its hook caller. Built-in vs `accelerator-vcs`
   sub-binary for the hot PreToolUse guard is a tuning detail deferred to the hooks
   work item (lean built-in to avoid a per-Bash-call sub-binary fetch).
5. **Binary granularity for small contexts** — *Resolved (2026-06-28):* group the
   light corpus ops (adr numbering/status, artifact metadata, validation, linkage)
   into a single **`accelerator-corpus`** binary over the shared `corpus`/
   `corpus-adapters` crates (no dependency-bleed rationale applies; they are one
   bounded context over `meta/`). Keep the PR helpers separate as
   **`accelerator-collaboration`** (the open github→collaboration domain rename).
   Split any further only if a dependency profile diverges. ~10 binaries total.
6. **Provenance** — *Resolved (2026-06-28):* runtime integrity is **minisign +
   sha256 verified in-process** (per ADR-0046). **Drop the runtime provenance
   check** — a `gh attestation verify` hook would contradict ADR-0046 (needs `gh`
   on the user's machine; in-process Sigstore breaks musl-static and is parked).
   Cleanup: correct `RELEASING.md` to stop advertising the unimplemented
   `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE` runtime hook. **Keep emitting CI-side
   SLSA attestations** as free out-of-band provenance (no user-machine cost).
   Revisit in-process provenance only if Sigstore reaches a stable musl-friendly
   1.0.
7. **Test-port timing & interface redesign** — *Resolved (2026-06-28):* hybrid —
   develop test-first in `cargo test` (the destination); where a shell suite
   exists, repoint it at the binary as a black-box parity gate during cutover;
   retire the suite + decrement the Python floor in the same change that deletes
   the scripts; characterize-then-port the untested clusters. **And** redesign
   entrypoint interfaces rather than transliterating bash (clap args, `--format`,
   structured output for machine-like consumers), preserving prose-for-injection
   outputs and the meaningful 0/1/2 exit codes; where an interface changes, the
   parity gate becomes behavioural/semantic and the suite is updated accordingly.
8. **`hooks/` count delta** — *Resolved (2026-06-28):* no change — `hooks/` still
   has **7** tracked `.sh` (4 prod: `config-detect`, `migrate-discoverability`,
   `vcs-detect`, `vcs-guard`; 2 test: `test-migrate-discoverability`,
   `test-vcs-detect`; 1 fixture: `test-fixtures/vcs-detect/regenerate.sh`). The
   apparent 6-vs-7 delta was a non-recursive `hooks/*.sh` glob in the locator sweep
   missing the nested fixture. Hooks-phase scope unchanged.
