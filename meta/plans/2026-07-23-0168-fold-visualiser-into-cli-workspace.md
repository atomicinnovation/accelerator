---
type: plan
id: "2026-07-23-0168-fold-visualiser-into-cli-workspace"
title: "Fold the Visualiser into the cli/ Workspace Implementation Plan"
date: "2026-07-23T08:41:19+00:00"
author: Toby Clemson
producer: create-plan
status: in-progress
work_item_id: "work-item:0168"
parent: "work-item:0168"
derived_from: ["codebase-research:2026-07-23-0168-fold-visualiser-into-cli-workspace"]
relates_to: ["work-item:0165"]
tags: [rust, visualiser, cli, launcher, corpus, workspace]
revision: "220cb821e3efd2e87acbd84600c02b36555e40e6"
repository: "build-system"
last_updated: "2026-07-23T17:29:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Fold the Visualiser into the cli/ Workspace Implementation Plan

## Overview

Relocate the `accelerator-visualiser` server + frontend into the `cli/`
workspace, retire its duplicated domain modules onto the shared
`corpus`/`corpus-adapters`/`document`/`config` crates, re-home its
start/stop/status orchestration into `accelerator visualiser …` dispatched by
the unified launcher, and cut its distribution over from the bespoke
`launch-server.sh` + flat `checksums.json` to the signed release manifest. The
work lands as five phases, each mergeable in order and each leaving `mise run`
green. Phases 1–2 are independently releasable. Phase 3 (which switches the
SKILL.md invocation to the launcher-resolved `accelerator visualiser`) and
Phase 4 (which wires the release producer to stage/sign the `visualiser` entry)
form a **co-release unit**: a release carrying Phase 3's switch must also carry
Phase 4's producer wiring, so the live manifest resolves `visualiser` and the
user-facing path never breaks. Phase 5 is the deletions-only cut-over — it
removes the now-dead shell surface and the flat `checksums.json` once a release
carrying the producer wiring has shipped.

## Implementation Progress

_Last updated 2026-07-23._

- **Phase 1 (Relocate + Become a Workspace Member): complete.** Landed as four
  commits (a pure history-preserving move, then workspace membership + lints,
  then build-system/CI/docs repointing, then gitignore). Full `mise run` green.
  Deviations, all within Phase 1's intent: the `version.py` member-manifest write
  was dropped now (not Phase 5) because it would clobber `version.workspace =
  true`; `test:unit:cli` excludes the visualiser (it keeps its own dedicated test
  tasks); the `store_duplication` guard skips `cli/visualiser/` until the
  Phase-2 write-path retirement; Phase 1 §7 (orchestration binary-path) was a
  no-op (the shell resolves the binary via env/config/distribution, no hardcoded
  target path). Manual checks (blame preservation, `--version`, live serve) not
  yet performed.

- **Phase 2 (Refactor onto the Shared Crates): steps 1–3 of 5 complete.** Three
  further commits, each differential-verified for byte/behaviour equivalence
  before deletion and parity-pinned, each `server:check` + `test:unit:visualiser`
  + `test:integration:visualiser` green, and collectively green under a full
  `mise run` (the one E2E failure was a lingering `start-server.mjs` wrapper from
  a flaked run holding the health port — a known infra flake, confirmed by a
  clean isolated re-run, not a code regression):
  1. **Format layer** — `frontmatter` engine → `corpus_adapters::parse` +
     `document` (serde-saphyr), mapping `corpus::Mapping` → the `serde_json::Value`
     the SPA consumes; `patcher` → `corpus_adapters::patch_status` (proven
     byte-identical across the status-patch contract); `gray_matter`/`serde_yml`
     dropped from the closure; the `serde_yml` advisory ignore removed from
     `deny.toml`. Dialect flips (1.1-style `yes/no/on/off/y/n` → bool; unquoted
     leading-zero/underscore numerics → number; libyml-panic inputs now parse)
     catalogued, judged harmless to accelerator conventions, and pinned; a
     `read_ref_keys` number-coercion keeps leading-zero ids canonicalising.
  2. **doc-type + typed_ref** — `corpus::DocTypeKey` via a serde-free
     `doc_type_serde` wire bridge (`wire_str`/`from_wire_str`, byte-identical
     tokens); `DocType`/`describe_types` relocated to `doc_type_view`;
     `corpus::typed_ref`. 14-variant wire + `config_path_key` parity pinned.
  3. **slug + config-ID** — `corpus::slug`; `WorkItemConfig` now holds a
     `corpus::WorkItemIdScheme` + `corpus_adapters::RegexScanner` and delegates
     its admission predicates (0 differential flips across numeric and
     project-prefixed schemes). `slug.rs` deleted.

  Deviation from the plan's letter: `frontmatter.rs` is **not** deleted — its
  serde_yml *engine* retired, but its server-only presentation helpers
  (`title_from`/`body_preview_from`/`read_ref_keys`) stay.

  4. **Clustering retirement** — the pure `clusters`/`cluster_key` logic +
     `canonicalise_one_id`/`target_path_from_entry`/`normalize_target_key` lifted
     into a **new `corpus::cluster` module** behind a serde-free `ClusterEntry`
     view port, taking the id convention as an injected `WorkItemIdScheme` +
     `IdScanner`. `WorkItemIdScheme::canonicalise_id` (regex-free, parity-pinned
     against the retired scan-regex canonicaliser) replaces the indexer's
     `canonicalise_one_id`/`number_width_from_id_pattern` regex pair. `clusters.rs`
     is now a thin adapter (`impl ClusterEntry for IndexEntry`, re-projecting the
     path-keyed corpus result back onto the wire types); `cluster_key.rs` deleted;
     the indexer's `canonicalise_one_id`/`target_path_from_entry` are thin
     delegators (its own `normalize_absolute` for reverse-index keying stays).
     Cluster membership is returned by **index** (not path) so distinct entries
     that share a path re-project faithfully. `corpus::cluster` is covered by the
     existing whole-crate `corpus_domain_imports_only_permitted` pup rule; a
     clustering/linkage parity fixture added to `tests/parity.rs`. The whole
     retired unit-test surface (40+ clustering + cluster-key cases) is preserved
     through the adapter and green.

     Deviations, within intent: `related.rs` stays a server module unchanged — its
     logic is inherently async (`Indexer` snapshot gathering); only its trivial
     pure `count_from_resolution`/dedup would move, so lifting it adds no value.
     No new adversarial-governance harness was added for the corpus module beyond
     the pup rule (deferred, mirroring the pre-existing `pup.ron` debt note).

  5. **Write-path retirement** — `file_driver`'s `atomic_write_preserving_perms`
     retired onto `corpus_adapters::FileCorpusStore` (the `corpus::AtomicWrite`
     port) over the existing `spawn_blocking` seam, with a `StoreError` →
     `FileDriverError` mapping. The etag-verify-then-write critical section, the
     per-path lock, the idempotent short-circuit, and the `on_committed`-under-lock
     all stay in `write_frontmatter`; the pre-read of `original_perms` is dropped
     (the store's `PreserveOr` reads the target's mode at write time, equivalent
     under the lock). Durability preserved by **adding file + parent-dir fsync to
     the shared `store::atomic_write`** (author-chosen over accepting the gap), so
     every store consumer gains it. The three visualiser runtime state-file
     writers (`server.pid`/`server-info.json`/`server-stopped.json`, 0600) also
     consolidated onto `store::atomic_write` (`NewFileMode::Set(0o600)`), so the
     blanket `cli/visualiser/` `store_duplication` exclusion is dropped (only a
     test-only `fs::rename` in `indexer.rs` is allowlisted). New driver-level
     CRLF + non-default-mode round-trip test; the existing concurrent-conditional
     -patch (TOCTOU) test is a mutation-check for the under-lock etag re-check and
     is deterministic via the per-path lock + barrier; the symlink-escape
     containment test still passes (the driver's writable-root check owns the
     guard, `FileCorpusStore` is defence-in-depth). `patch_status` byte parity is
     covered by the retained `api_docs_patch` write-path tests + the patcher unit
     tests, so a separate golden fixture was not added.

     The added dir-fsync widened a **pre-existing** startup race the full-gate run
     surfaced (`shutdown_preserves_state_on_stopped_write_failure` exiting via
     SIGTERM under parallel load): `server-info.json` was announced before the
     SIGTERM handler was installed, so a signal arriving the instant the file
     appeared hit the default terminate disposition. Fixed structurally — the
     signal streams are now created **synchronously** in `spawn_signal_handlers`
     (installing the OS handler before it returns) and that call moved ahead of
     the pid/info writes, so readiness is never announced before shutdown is
     handled.

  6. **thiserror + deny reconcile** — the server's `thiserror` pin moved to the
     workspace `2` line; it built and tested clean (its error enums use only the
     standard `#[error]`/`#[from]`/`#[source]`/`transparent` forms that carry
     across the major). `thiserror` 1 is gone from the lock. `deny.toml`'s license
     allow-list re-pruned to the exact closure: the now-unmatched
     `Unicode-DFS-2016` allowance removed (`deny:check` no longer warns
     license-not-encountered).

  **Phase 2 is complete**, landed as three commits (clustering retirement; write
  -path retirement + shared-store fsync + the shutdown-race fix; thiserror 2 +
  deny re-prune). All component criteria green — `server:check`, `cli:check`,
  `deny:check`, `pup:check`, `lint:store-duplication:check`, `build-system:check`,
  the visualiser unit/integration/parity suites, and the full `mise run` gate
  (green after the shutdown-race fix). The one open item is the manual visual
  spot-check of the rendered library/kanban/related views after the engine swap.

- **Phase 3 (Re-home Orchestration): core complete.** The binary grew a clap
  subcommand tree (`serve`/`start`/`stop`/`status`, owner-pid a global flag so a
  single `accelerator visualiser <sub>` invocation form serves every subcommand);
  `serve` reads `.accelerator/*.md` directly (Model 1) via a new decomposed
  `compose` module (doc-paths, template tiers + `config_override_source` derived
  from the resolved `Source`, work-item scheme, kanban/idle, and the
  `ACCELERATOR_VISUALISER_*` env overlay), with `plugin_root`←`CLAUDE_PLUGIN_ROOT`,
  `project_root`←discovered root, `host`←`127.0.0.1` (dev-frontend honours
  `E2E_SERVER_HOST`), and `plugin_version`←crate `VERSION`. The shell lifecycle is
  ported into a decomposed `orchestration/` module (`process` identity +
  SIGTERM→2s→SIGKILL via `kill(pid,0)` polling, `lock` via `nix` `flock(2)`,
  `state` files, and the `start`/`stop`/`status` commands): the recycle guard keys
  strictly on the server pid's `start_time` (exact match, no ±1s drift), forced
  kills synthesise `server-stopped.json`, `start` preserves the init-sentinel +
  tickets→work preconditions and cleans stale `store::TEMP_PREFIX` temps, and
  `status` collapses to `running`/`stopped`. `config.json` reading + `Config::from_path`
  are gone; the hand-synced `DEFAULT_IDLE_TIMEOUT`/`DEFAULT_KANBAN_COLUMN_KEYS`
  now source from the shared catalogue. The Origin guard was tightened to an
  exact-host URI parse. SKILL.md invokes `accelerator visualiser --owner-pid $PPID
  ${ARGUMENTS:-start}`; the E2E harness (`start-server.mjs`), `api_smoke.rs`, and
  `shutdown.rs` were rewired onto a Model-1 fixture project (symlinked meta/
  templates, config remapping only `research_codebase`). New tests: DSL scan-regex
  port + shell parity, `compose_contract`, `orchestration_lifecycle` (start/stop/
  status/recycle-refusal/forced-kill/stale/init-sentinel), serve exit codes,
  Origin-guard lookalike/userinfo, and a launcher `visualiser` dispatch case. Full
  `mise run` green end-to-end (incl. E2E, all integration suites).

  Deviations, within intent: (1) the `id_pattern`→scan-regex compiler existed
  **only in bash** (`WorkItemIdScheme` doesn't compile it), so it was ported to a
  new `work_item_pattern` module beside its sole Rust consumer, cross-checked by a
  parity test against `work-item-pattern.sh`. (2) The **coherence check** binding
  SKILL.md's `accelerator visualiser` to `DISPATCHED_SUBBINARIES` membership is
  moved to **Phase 4** (where `DISPATCHED_SUBBINARIES` gains `"visualiser"`), so
  each commit stays `mise run`-green — the two sides cannot both exist in Phase 3
  alone. No release ships between the Phase 3 and Phase 4 commits, so the
  co-release invariant holds. (3) `config_contract.rs` was retired **now** (its
  `Config::from_path`/config.json contract is gone) and replaced by the new
  `compose_contract.rs`; the plan slated its deletion for Phase 5, but Model-1
  removal forces it here. (4) The **dev circus stack** was rewired onto `serve`
  in a follow-up commit (see below), so `mise run dev` reads `.accelerator/*.md`
  directly like the dispatched path. Manual E2E/idle spot-checks not yet
  performed.

- **Dev circus stack (follow-up to Phase 3): complete.** `tasks/dev.py` +
  `tasks/shared/dev/circus.py` now drive the dev server via `serve --owner-pid 0`
  from `working_dir = the project root` (with `CLAUDE_PLUGIN_ROOT` propagated)
  instead of `--config`; the dev lifecycle points at the composed
  `.accelerator/tmp/visualiser` state dir, the `config.json` renderer + the
  `dev-server` dir are gone, and the fakes + dev unit/integration tests were
  repointed to the new server-owned location. `mise run dev:server` verified
  against the real repo root.

- **Phase 4 (Producer Wiring + Local-Manifest Verify): complete.**
  `DISPATCHED_SUBBINARIES = ("visualiser",)` engages the signed-manifest flow:
  `server_cross_compile` now stages the default (`embed-dist`) binary to both
  `bin/` (checksums flow, debug archive only) and
  `dist/release/accelerator-visualiser-{platform}` (manifest flow) after a
  full-symbol `ACCELERATOR_VISUALISER_E2E_INSECURE` byte-scan guard (matched
  exactly, not the prefix, since the release binary legitimately embeds other
  `ACCELERATOR_VISUALISER_*` literals). Every sub-binary asset carries the
  `accelerator-` prefix via `subbinary_asset_path`, so the launcher fetches
  `accelerator-<token>-<platform>` even though the manifest keys on the bare
  token; the visualiser thus ships as a **single shared asset**
  (`accelerator-visualiser-{platform}`) referenced by both verification paths,
  and the checksums flow no longer uploads a duplicate binary. `manifest.py`/
  `signing.py`/`github.py` were already parameterised; the discovered gaps were
  fixed — `_default_subbinary_manifest` maps `visualiser` → `cli/visualiser/
  server/Cargo.toml` (not the nonexistent `cli/visualiser/Cargo.toml`), and the
  server crate description became `"Launch the interactive meta-directory
  visualiser"` to match the golden manifest. The launcher fixture + `manifest.rs`
  test now key on `visualiser`, and the whole `resolution.rs` fetch/verify/reject
  suite (happy path + `ChecksumMismatch` + `SignatureMismatch` + tamper) is
  re-keyed on the `visualiser` entry, resolving `accelerator-visualiser-*`. The
  shared asset is covered by the existing `dist/release/accelerator-*` provenance
  glob, so no new glob was needed. The deferred **coherence check**
  (`validate_dispatch_coherence`, gated in `emit_manifest`) now binds SKILL.md's
  `accelerator visualiser` invocation to `DISPATCHED_SUBBINARIES` membership. Test
  lockstep: `_setup_release` stages the shared `accelerator-visualiser` asset +
  `.minisig` + a `visualiser` manifest entry, the upload-count literal went
  18 → 22 (the checksums flow's duplicate binary upload dropped), `_pass_reverify`
  mocks `_reverify_subbinary`, and new unit tests cover the manifest mapping, the
  E2E-insecure guard, and the coherence check. Full `mise run` green.

- **Phase 5:** not started.

## Current State Analysis

The crate `accelerator-visualiser` lives at
`skills/visualisation/visualise/server` with a sibling `frontend/`, a `bin/`
holding the flat `checksums.json`, a `scripts/` directory of shell
orchestration, and a bash CLI at `cli/accelerator-visualiser`. It is **not** a
`cli/` workspace member (the workspace has 11 members at
`cli/Cargo.toml:4`); its version is a hand-copied literal
(`server/Cargo.toml:3`) and its MSRV is `1.85` against the workspace's
`1.90.0`.

The server carries private copies of domain logic that now exists, public, in
the shared crates delivered by 0178/0179/0180:

- `docs.rs` `DocTypeKey` (14 variants, serde-derived) → `corpus::doc_type`
  (serde-free, with `wire_str`/`from_wire_str` **already present** at
  `cli/corpus/src/doc_type.rs:168-192`).
- `slug.rs` (`derive`, `humanise_slug`, private `title_case_segment`) →
  `corpus::slug` (`title_case_segment` is `pub` at `cli/corpus/src/slug.rs:193`).
- `frontmatter.rs` — engine is **`serde_yml`** (`:144`, wrapped in
  `catch_unwind`), **not** `gray_matter` (which is dead-declared). →
  `document::fence_offsets` + `corpus_adapters::parse`.
- `patcher.rs` (`patch_status`) → `corpus_adapters::patcher::patch_status`.
- `typed_ref.rs` (`parse_typed_ref`) → `corpus::typed_ref`.
- `config.rs` ID logic (`WorkItemConfig`, `extract_id`/`normalise_id`) →
  `corpus::WorkItemIdScheme` + injected `corpus_adapters::RegexScanner`.
- `file_driver.rs` `atomic_write_preserving_perms` → `corpus`'s `AtomicWrite`
  port implemented by `corpus_adapters::FileCorpusStore`.
- `related.rs`/`clusters.rs`/`cluster_key.rs` linkage → `corpus::linkage`.

The parts that stay are axum/tokio/notify: `server.rs` (router, loopback bind,
Host/Origin 403 guards `:633-678`), `lifecycle.rs` (idle self-shutdown),
`shutdown.rs`, `sse_hub.rs`, `indexer.rs`, `watcher.rs`,
`write_coordinator.rs`, `assets.rs`, and the `api/` wire layer.

Orchestration is five shell scripts (`visualiser.sh` dispatcher →
`launch-server.sh`/`stop-server.sh`/`status-server.sh`, plus
`write-visualiser-config.sh`) backed by `launcher-helpers.sh`. State lives
under `$(accelerator config path tmp)/visualiser/`. `launch-server.sh` plays
two roles: (a) daemon start (`nohup "$BIN" --config config.json` — the **Rust
server** writes `server.pid`/`server-info.json` via `process_start_time()`;
the shell polls), and (b) fetch/distribution (tri-precedence bin resolution,
SHA-256 verify against `bin/checksums.json`, `install` to cache).
`write-visualiser-config.sh` emits the `config.json` the server reads today.

The launcher (`cli/launcher`, 0164) routes any unknown subcommand token through
`#[command(external_subcommand)]` → `resolve()` → `exec`-replace. It derives
**both** the manifest lookup key and the release asset filename from the bare
token: `format!("{name}-{platform}")` + `platform_entry(name, …)`
(`resolve/mod.rs:143-146`), override var `ACCELERATOR_VISUALISER_BIN`. The
release producer's `DISPATCHED_SUBBINARIES` is `()`
(`tasks/shared/paths.py:21`), the staged asset const is
`accelerator-visualiser-{platform}` (`:51`), and the golden manifest fixture
keys on `accelerator-visualiser` — none of which match the bare token
`visualiser`.

The version-coherence gate (`tasks/build.py:validate_version_coherence`,
`:184-210`) reads `plugin.json`, the visualiser server `Cargo.toml` literal,
`checksums.json`, the `cli/` workspace version, and pinned members.

`deny.toml`'s license `allow`-list is pruned to exactly the current closure
(warns on unused allowances); `multiple-versions = "warn"`; `serde-saphyr` is
banned outside `document`. `pup.ron` governs only the named domain crates, so a
new `accelerator-visualiser` crate is initially ungoverned.

### Key Discoveries

- `corpus::doc_type::wire_str`/`from_wire_str` **already exist**
  (`cli/corpus/src/doc_type.rs:168-192`) — the server calls them directly; only
  its own serde-deriving API view types wrap them. This shrinks the anticipated
  wire-mapping work.
- `corpus`/`corpus-adapters`/`document` are **fully synchronous** — no async
  variants. The async I/O boundary stays in the visualiser crate behind
  `spawn_blocking`.
- The launcher needs **no routing change** to add `visualiser`; the
  external-subcommand catch-all already forwards it. The External arm composes
  **no** config — a sub-binary reads config itself.
- `visualiser.idle_timeout` default `"8h"` already lives in the config
  catalogue (`cli/config/src/catalogue.rs:220`).
- Test seams exist and are reusable: `RecordingExec` spy (`core.rs:287-344`),
  the `accelerator-fixture` bin + `tests/dispatch.rs`
  (`ACCELERATOR_<SUB>_BIN` override), and the `resolution.rs` `MockServer`
  fetch/verify harness with a runtime-generated minisign keypair.
- The three `../frontend/dist` literals (`build.rs:5`, `assets.rs:9,71`) stay
  valid because `server`+`frontend` move as a **unit**.

## Desired End State

`cli/visualiser/server` is the 12th workspace member, inheriting version from
`[workspace.package]`, carrying none of the retired domain modules, depending
on `corpus`/`corpus-adapters`/`document`/`config`/`config-adapters` and neither
`gray_matter` nor `serde_yml`. `accelerator visualiser start|stop|status`
manages the lifecycle, dispatched through the launcher, with the server reading
`.accelerator/*.md` directly. The five shell scripts, the flat
`checksums.json`, and the bash CLI are gone; the release producer stages, signs,
and publishes the `visualiser` binary in the signed manifest. `mise run` is
green end-to-end. Verified by: the parity golden fixtures (spanning frontmatter
scalar dialect, patcher, and linkage), the write-path preservation/concurrency
tests, the Host/Origin 403 tests, the orchestration lifecycle tests (including
the stale and forced-kill states), the black-box launcher-dispatch tests, and
the local-manifest fetch/verify test (with a rejection case).

## Naming Resolution (settled)

The single cross-cutting decision, resolved: **key the manifest and override on
the bare token `visualiser`.** `accelerator visualiser` stays the UX;
`DISPATCHED_SUBBINARIES` gains `"visualiser"`; the golden fixture key changes
`accelerator-visualiser` → `visualiser`. The published **asset**, however,
carries the `accelerator-` prefix — `accelerator-visualiser-{platform}` — so the
launcher fetches `accelerator-<token>-<platform>` and the visualiser ships as one
shared asset serving both verification paths (the same name the checksums flow
reverifies). The crate/bin name `accelerator-visualiser` is unaffected (it is the
compiled artifact's internal name, not the dispatched key). No launcher alias
layer is introduced.

## What We're NOT Doing

- Not adding async variants to `corpus`/`corpus-adapters` — the async seam
  stays in the visualiser crate via `spawn_blocking`.
- Not building an ADR-0054 "Model 2" launcher config-passing seam — the server
  reads config directly (Model 1).
- Not nesting the frontend under a new `cli/visualiser/frontend` task
  namespace — `frontend`/`server` stay their own task components with repointed
  paths.
- Not landing the **live**-manifest fetch assertion — that is gated on 0165's
  manifest carrying the `visualiser` entry; this story verifies against a
  local/test manifest fixture.
- Not deleting the axum/tokio/notify crate — only the duplicated domain slice
  collapses onto the shared crates.
- Not adding `pup.ron` architectural rules for the visualiser — recorded as
  tracked debt with a concrete follow-up story (bring the crate under the
  workspace `pup.ron` governance the other members meet), not left as an
  open-ended divergence. This story does bring the request/SSE handler paths
  under the workspace `unwrap_used`/`panic` clippy policy (see Phase 1).

## Implementation Approach

TDD where applicable: freeze parity golden fixtures before any deletion; add
lifecycle/dispatch tests before porting the shell behaviour. Each phase is
sequenced so the repo stays green and shippable: Phase 1 relocates without
behaviour change (and reconciles the version-coherence gate to workspace
inheritance so the release pipeline stays coherent), Phase 2 swaps the domain
engine behind parity tests, Phase 3 re-homes orchestration behind
dispatch/config tests and switches the SKILL.md invocation to
`accelerator visualiser`, Phase 4 wires the release producer to stage/sign the
`visualiser` entry and verifies fetch against a local manifest (Phases 3 and 4
co-release so the switch never outruns the manifest entry), and Phase 5 deletes
the dead shell surface once a release carrying the producer wiring has shipped.

---

## Phase 1: Relocate + Become a Workspace Member

### Overview

Move the `server`+`frontend` pair into `cli/visualiser/`, make the server the
12th workspace member inheriting the workspace version, and repoint every path
reference — with **no** behaviour change. Duplicated modules remain; shell
orchestration still drives the server against its new build location.

### Changes Required

#### 1. File move (unit)

Move `skills/visualisation/visualise/server` → `cli/visualiser/server` and
`skills/visualisation/visualise/frontend` → `cli/visualiser/frontend` using
`jj` so history follows. `skills/visualisation/visualise/` retains `SKILL.md`,
`scripts/`, `bin/`, and the bash `cli/` (all still functional this phase). The
three `../frontend/dist` literals are unchanged (the relative layout is
preserved).

#### 2. Workspace membership + version inheritance

**File**: `cli/Cargo.toml`
**Changes**: add `"visualiser/server"` to `members`.

**File**: `cli/visualiser/server/Cargo.toml`
**Changes**: inherit workspace fields; drop the redundant local
`[profile.release]` (the workspace root owns it). Bring the request/SSE handler
paths under the workspace `unwrap_used`/`panic` clippy policy — a panic in a
request or SSE-write handler crashes the workspace's one long-running HTTP
daemon (a local denial-of-service), the exact failure mode the policy exists to
catch. Where a module genuinely cannot satisfy the policy this phase (e.g. a
pre-existing infallible-context `unwrap`), allow it with a narrow,
per-occurrence `#[allow(...)]` carrying a justification rather than blanket-
exempting the crate; carry the remaining curated `[lints.clippy]` entries only
for rules the workspace does not already set.

```toml
[package]
name = "accelerator-visualiser"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
description = "Meta-directory visualiser server for the accelerator Claude Code plugin"
publish = false
```

MSRV rises `1.85` → `1.90.0` (workspace value); acceptable for a workspace
member. Leave `thiserror = "1"` as a local pin for this phase (the
`multiple-versions = "warn"` policy tolerates the skew); it is reconciled or
removed in Phase 2 when the module set shrinks.

#### 3. Version-coherence gate reconciliation (same phase as inheritance)

**File**: `tasks/build.py`
**Changes**: the moment the server switches to `version.workspace = true`,
`validate_version_coherence` must stop reading the standalone
`[package].version` literal — `_read_cargo_toml_version` then parses
`{"workspace": true}` (a table, not a string) and is always a mismatch, which
breaks every release/version/manifest task even though the default `mise run`
gate does not invoke coherence. Drop `_read_cargo_toml_version` (and the
`Cargo.toml` entry it feeds at the `found` map) in this phase; the visualiser's
version is then covered transitively by `_read_workspace_version`
(`_pinned_member_versions` already contributes no entry for an inheriting
member). Keep `_read_checksums_json_version` for now — `checksums.json` is not
deleted until the Phase 5 cut-over, and `version.py` keeps its version coherent
until then.

Add a regression test asserting a deliberately-skewed visualiser member version
is caught via `_read_workspace_version` (so removing the literal reader does not
create a blind spot).

#### 4. cargo-deny license reconciliation

**File**: `cli/deny.toml`
**Changes**: add to `licenses.allow` every permissive license the visualiser's
dependency closure (axum, tokio, tower-http, notify, chrono, nix, libc,
gray_matter, serde_yml, arc-swap, file-rotate, humantime, …) carries that is
not already listed. Add any advisory `ignore` entries the new transitive graph
requires (mirroring the existing hickory-proto justification style). Derive the
exact set empirically from `mise run deny:check` output.

#### 5. Task-tree path repointing

Before starting, `grep` the whole tree for `skills/visualisation/visualise` and
fold every hit into the repointing set rather than relying on the enumerated
file names below — the string is scattered across tasks, task tests, and CI.

**File**: `tasks/shared/paths.py`
**Changes**: repoint the `SERVER` and `FRONTEND` path constants to
`cli/visualiser/{server,frontend}`. Because the crate becomes the 12th
workspace member, build output relocates from the crate-local `server/target/`
to the workspace-shared `cli/target/`: derive the built-binary paths from
`CLI_DIR / "target"`, **not** `SERVER / "target"`. Repoint the cross-compile
output path (`:287` in `tasks/build.py`), the E2E `SERVER_BIN`
(`tasks/test/e2e.py:17`), **and** the dev-server binary
(`tasks/dev.py:43`, `_SERVER_BIN = SERVER / "target/debug/…"`) accordingly.

**Files**: `tasks/format/{frontend,server}.py`, `tasks/lint/{frontend,server}.py`,
`tasks/lint/scripts.py`, `tasks/shared/sources.py`, `tasks/build.py`,
`tasks/dev.py`, `tasks/test/{unit,integration,e2e}.py`,
`tests/unit/tasks/{test_build,test_version,test_sources,test_exec_bits}.py`,
`tests/conftest.py`, `.github/workflows/main.yml`
**Changes**: repoint every hard-coded `skills/visualisation/visualise/…`
reference — including the Python task-unit-test assertions that pin these paths,
which fail against the old location and block the green gate if missed.

#### 6. Crate-escaping test-path repointing

**Files**: `cli/visualiser/server/tests/` (and any `src` test module deriving a
repo-layout path from `CARGO_MANIFEST_DIR`)
**Changes**: the move changes the crate's depth from `skills/visualisation/
visualise/server` (four segments below the repo root) to `cli/visualiser/server`
(three), so every `CARGO_MANIFEST_DIR`-relative path that escapes the crate
must lose one `..`. Known escapes: `slug.rs:574,595`
(`../../../../skills/work/scripts/work-item-pattern.sh`) and
`config_contract.rs:75` (`../../../../templates`). `config_contract.rs:7`
joins `../scripts/write-visualiser-config.sh`, which points into
`skills/visualisation/visualise/scripts/` — a directory that does **not** move
this phase; repoint it at the still-live script location (it is retired with its
test in Phase 5). Prefer anchoring on a discovered repo root over counting `..`
where practical. Only `../frontend/dist` (the embed literals) is depth-invariant
because `server`+`frontend` move together.

#### 7. Orchestration binary-path repointing

**Files**: `skills/visualisation/visualise/scripts/launch-server.sh` (dev-build
resolution), any script computing the built binary path
**Changes**: repoint the local dev-build path to the workspace target dir so
the still-live shell orchestration finds the relocated binary. Distribution
path (checksums/download) is untouched this phase.

### Success Criteria

#### Automated Verification

- [x] Workspace build succeeds: `cargo build -p accelerator-visualiser`
      (run from `cli/`)
- [x] Component check passes: `mise run server:check`
- [x] Frontend check passes: `mise run frontend:check`
- [x] cargo-deny passes: `mise run deny:check`
- [x] Visualiser unit + integration + E2E suites pass:
      `mise run test:unit:visualiser test:integration:visualiser test:e2e:visualiser`
- [x] Version-coherence passes after inheritance, and a skewed visualiser member
      version is caught via `_read_workspace_version` (regression test)
- [x] Python task-unit suites pass with the repointed paths:
      `mise run test:unit:tasks`
- [x] Full gate is green: `mise run`

#### Manual Verification

- [ ] `git`/`jj` history follows the moved files (blame preserved)
- [ ] `accelerator-visualiser --version` reports the workspace version
- [ ] The visualiser starts and serves via the (still-shell) orchestration
      against the new build location

---

## Phase 2: Refactor onto the Shared Crates (Parity-Verified)

### Overview

Retire the duplicated domain modules onto `corpus`/`corpus-adapters`/`document`
and swap the frontmatter engine, proven equivalent by golden fixtures frozen
**before** deletion. `config.json` reading and idle resolution stay for now.

### Changes Required

#### 1. Freeze parity golden fixtures (first, before any deletion)

**File**: `cli/visualiser/server/tests/fixtures/parity/` (new)
**Changes**: capture the current outputs as committed golden files via a
throwaway capture test run against the **pre-refactor** modules. The fixture
set must span the whole retired surface, not just the three easy engines:

- **Frontmatter structure**: multi-line/quoted values; the `fence_offsets`
  boundaries (leading blank lines, CRLF endings, no trailing newline, empty
  frontmatter block, no frontmatter at all).
- **Frontmatter scalar dialect** (the real engine-swap risk): `serde_yml`
  (YAML-1.1 lineage) → serde-saphyr (1.2) can flip value *types*. Include
  bare `yes`/`no`/`on`/`off`/`y`/`n`, unquoted numeric-looking values
  (`version: 1.20`), `null`/`~`/empty spellings, leading-zero and
  underscore-grouped numbers, unquoted ISO dates, and duplicate keys. Assert the
  **JSON-serialised value type** (not just the map keys), because the parsed map
  is serialised to JSON for the React SPA — a string→bool/number flip is a
  wire-output change to a live consumer.
- **doc-type**: split into two fixture classes, because the old
  `kind_for_canonical_path` is a `HashMap`-order first-`starts_with` match —
  non-deterministic for overlapping roots — so a blind capture of it is not a
  reproducible golden. (a) *Pure-parity* paths: one document per `DocTypeKey`
  variant (14) on clean single-root paths, asserted equal to the captured old
  output; plus a `config_path_key` pin for all 14 variants (e.g.
  `PrDescriptions`→`prs`, `Research`→`research_codebase`). (b) *Divergence*
  paths: nested/overlapping-root and non-anchored-segment inputs where the two
  matchers legitimately differ — assert the **intended new `infer` result**,
  hand-authored (not captured from the old matcher), recording the deliberate
  departure. Blind capture is reserved for the deterministic surfaces
  (frontmatter, slug, patcher, linkage).
- **slug**: inputs exercising `humanise_slug`/`title_case_segment`; work-item-ID
  inputs.
- **patcher**: `patch_status` byte output for representative documents,
  including a CRLF-terminated file and multi-line/quoted values.
- **linkage**: `related`/`clusters` records (`linkage::Band`
  Resolved/Ambiguous classification, `TYPE_PAIRS`) for the same corpus.

**File**: `cli/visualiser/server/tests/parity.rs` (new)
**Changes**: for each fixture, assert the refactored `document`/`corpus`/
`corpus_adapters` output — parsed frontmatter map (with JSON value types),
derived slug, inferred doc-type, `config_path_key`, `patch_status` bytes, and
linkage records — matches its golden: field-for-field equal to the frozen output
for the pure-parity fixtures, and equal to the hand-authored intended result for
the doc-type divergence fixtures. For the ambiguous-scalar and duplicate-key
fixtures assert the parse *outcome* (`Ok` vs `Err`) matches too — serde-saphyr
(1.2) is stricter than `serde_yml` and may reject inputs the old engine accepted
(e.g. duplicate keys), which would silently stop an existing `.accelerator/*.md`
from rendering; treat any such flip as a fixture case to resolve. Error-message
text and insignificant whitespace are out of scope.

Because this narrows accepted stored data, specify and test the **runtime
degradation contract**: when a previously-rendering document is now rejected at
parse time, the indexer skips that single document and surfaces a per-document
parse error rather than aborting the index or panicking — add an integration
test seeding a corpus with one now-rejected document and asserting the server
still starts and renders the rest.

#### 2. Add shared-crate dependencies; drop the YAML engines

**File**: `cli/visualiser/server/Cargo.toml`
**Changes**: add path deps `corpus`, `corpus-adapters`, `document`, `config`,
`config-adapters` (and `kernel` if an error type is shared); remove
`gray_matter` and `serde_yml`. Reconcile `thiserror` `1` → the workspace `2`
line: this is a breaking major (stricter `#[from]`/`#[error(transparent)]`
handling and display field-reference rules), so scope a derive-audit step —
build the crate against workspace `thiserror` and fix the error enums before
deleting modules, rather than treating it as a version-line swap.

After dropping the YAML engines, re-run `mise run deny:check` and re-prune
`cli/deny.toml`: remove any `licenses.allow` entries and advisory `ignore`s
carried only by `gray_matter`/`serde_yml` and their transitive graph, so the
allow-list matches the exact closure convention and accrues no unused-allowance
warnings.

```toml
corpus = { path = "../../corpus" }
corpus-adapters = { path = "../../corpus-adapters" }
document = { path = "../../document" }
config = { path = "../../config" }
config-adapters = { path = "../../config-adapters" }
```

#### 3. Retire the domain modules

**Files**: delete `docs.rs`, `slug.rs`, `frontmatter.rs`, `patcher.rs`,
`typed_ref.rs`; excise the ID logic from `config.rs`; delete/rewire
`related.rs`, `clusters.rs`, `cluster_key.rs` onto `corpus::linkage`.
**Changes**: replace call sites:

- doc-type: `corpus::doc_type::{DocTypeKey, infer, wire_str, from_wire_str,
  config_path_key}`. Use `doc_type::infer(path, table)` — the shared
  longest-segment matcher — so every corpus consumer shares one inference
  semantics, and retire the server's root-based `kind_for_canonical_path`. This
  is a genuine semantics change (deterministic longest-match with an
  embedded-`/dir/` branch, vs `HashMap`-order first-`starts_with`), not a pure
  refactor: build `infer`'s table from the same canonicalised absolute roots the
  old matcher used, and pin the nested/overlapping-root behaviour with the
  parity fixtures in §1 (any divergence is a fixture case to resolve, not a
  reason to keep the old matcher).
- frontmatter: `document::fence_offsets` + `corpus_adapters::parse`.
- slug: `corpus::slug::{derive, humanise_slug, title_case_segment}`.
- patcher: `corpus_adapters::patcher::patch_status`.
- typed refs: `corpus::typed_ref::parse_typed_ref`.
- ID scheme: `corpus::WorkItemIdScheme` with an injected
  `corpus_adapters::RegexScanner` (`&dyn IdScanner`).
- atomic write: `corpus`'s `AtomicWrite` via
  `corpus_adapters::FileCorpusStore`, driven from the existing async
  `FileDriver` port across a `spawn_blocking` seam. Give the seam a short
  doc-comment stating the async-façade-over-sync-store contract and where the
  blocking work runs, so the layering is self-documenting.

#### 4. Re-home the wire mapping

**Files**: `cli/visualiser/server/src/api/docs.rs` and any API type serialising
a doc type
**Changes**: keep serde derives only on the server's own wire view types; map
to/from `corpus::DocTypeKey` via `wire_str`/`from_wire_str`. Do **not**
re-derive serde on `corpus::DocTypeKey`.

#### 5. Write-path regression tests (automated, not manual)

**File**: `cli/visualiser/server/tests/`
**Changes**: the write path is the highest-risk surface (it mutates real user
documents), so promote its safety checks from manual spot-checks to automated
regression tests over the new `FileCorpusStore`-backed, `spawn_blocking` seam:

- **Preservation**: patch a CRLF-terminated document at a non-default file mode
  and assert the resulting bytes retain CRLF and the file retains its original
  mode.
- **Concurrency**: issue two overlapping conditional (etag) patches to the same
  document and assert exactly-one-wins / one-`412` semantics with no corruption
  — guarding the etag-verify-then-write TOCTOU window the Performance section
  flags, which the pre-refactor single-writer tests do not exercise. First
  confirm *where* that window is actually guarded (`write_coordinator.rs` is a
  self-write dedup cache, not a write serialiser — do not assume it serialises),
  then anchor a **deterministic interleaving seam** on the real production
  verify-then-write section (the handler / `FileCorpusStore` path, not a test
  double), admitting the second patch precisely while the first holds the
  pre-write etag snapshot, so the loser reliably observes `412` rather than
  relying on thread timing (tautological or flaky under parallel `cargo test`).
  The test must **fail if the pre-write etag re-check is removed** (a mutation
  check), proving it drives the real critical section.
- **Path safety**: the write path moves from `file_driver.rs` to
  `FileCorpusStore`; carry forward (or add) a regression test asserting a
  write whose path resolves outside the configured writable roots (e.g. a
  symlink inside `work/` escaping the root) is still refused, and confirm which
  layer now owns that guard — a silently-dropped containment check is a
  data-safety hole.

### Success Criteria

#### Automated Verification

- [x] Parity tests pass across the whole retired surface (frontmatter map incl.
      JSON value types, slug, doc-type, `config_path_key`, `patch_status` bytes,
      linkage): `cd cli/visualiser/server && cargo test --test parity`
- [~] The retired modules are absent (`docs.rs`/`slug.rs`/`patcher.rs`/
      `typed_ref.rs`/`cluster_key.rs` deleted; no ID/cluster logic in `config.rs`/
      `clusters.rs`). Deviation (documented above): `frontmatter.rs` stays — its
      `serde_yml` engine is retired but its server-only presentation helpers
      (`title_from`/`body_preview_from`/`read_ref_keys`/`FrontmatterState`) remain.
- [x] `gray_matter` and `serde_yml` are absent from `Cargo.toml` and
      `Cargo.lock`
- [x] `serde-saphyr` reaches the server only through `document`, and `deny.toml`
      carries no unused allowances/ignores after the YAML engines are dropped:
      `mise run deny:check`
- [x] CRLF/mode-preservation and concurrent-conditional-patch tests pass
- [x] Component + visualiser suites pass:
      `mise run server:check test:unit:visualiser test:integration:visualiser`
- [x] Full gate is green: `mise run` (confirmed after the shutdown-race fix)

#### Manual Verification

- [ ] The rendered library/kanban/related views are visually unchanged after
      the engine swap (spot-check via the running visualiser)

---

## Phase 3: Re-home Orchestration into `accelerator visualiser`

### Overview

Grow the binary's clap tree with `serve`/`start`/`stop`/`status`, port the
shell lifecycle into Rust, dispatch through the launcher, and switch the server
to read config directly. Retire `config.json` + `write-visualiser-config.sh`.

### Changes Required

#### 1. Subcommand surface

**File**: `cli/visualiser/server/src/main.rs`
**Changes**: parse a clap subcommand: `serve` (the daemon, current behaviour),
`start`, `stop`, `status`. `foo --help` already routes here via the launcher.
`start` spawns the detached `serve` daemon, polls for `server-info.json`, and
prints the `http://127.0.0.1:PORT` URL.

**Owner identity under exec-replace**: the shell derives the owner PID as the
*grandparent* (`ppid_of($PPID)` in `launch-server.sh`, the Claude process two
levels above the script). Under the new path `accelerator visualiser start` is
reached via the launcher's `external_subcommand` → `exec`-replace, so the
process ancestry differs from the old shell tree — a naive two-levels-up port
would record an intermediate launcher ancestor, not Claude. Ancestor-walking
couples owner identity to a topology outside this repo's control (how Claude's
`!` preprocessor spawns the command, which can change across Claude Code
versions), and an injected-ancestor unit test cannot catch a real-topology
mismatch. Prefer making owner identity **explicit** — have the invoker pass the
owner PID/`start_time` in (the daemon already receives them as CLI flags) rather
than inferring it by counting levels; if ancestry-walking is unavoidable, pin
the rule concretely in the plan and back it with an end-to-end check that spawns
through a real shell→launcher→exec chain, not only the injected-ancestor unit
seam. Owner-PID/`start_time` reach the detached daemon as CLI flags (runtime
data, not config).

#### 2. Lifecycle port (Rust)

**File**: `cli/visualiser/server/src/orchestration/` (new module)
**Changes**: port from `launch-server.sh`/`stop-server.sh`/`status-server.sh`/
`launcher-helpers.sh`. **Two distinct process identities must not be
conflated**: the *server's own* pid + `start_time` (written to
`server-info.json`/`server.pid`, and the sole basis for the stop/status recycle
guard) versus the *owner* (parent Claude) pid + `start_time` (used only by
`lifecycle.rs` for idle/owner-death self-shutdown). The shell guards `stop` on
the **server** pid's `start_time` (`stop_server_stop`), not the owner's.

- **recycle guard**: `stop` reads the server pid from `server.pid` and its
  recorded `start_time` from `server-info.json`, compares against
  `process_start_time(server_pid)`, and terminates **only on an exact match**;
  any mismatch, unreadable `start_time`, or dead pid → refuse/clean up, never
  signal. This keys strictly on the server identity — the owner pair is not
  consulted here. Drop the shell's legacy ±1 s `start_time` drift tolerance:
  it accommodated the old Node daemon's `Date.now()` capture; now that both the
  write and the check use the same `process_start_time`, exact equality is
  correct.
- **termination**: the detached daemon is **not a child** of the `stop`
  invocation, so escalation must **poll `kill(pid, 0)`** (SIGTERM → 2 s wait →
  SIGKILL), never `waitpid` (which returns `ECHILD` for a non-child).
- **forced-kill invariant**: on a forced SIGKILL (the server never wrote its
  own stopped sentinel), synthesise `server-stopped.json` and remove
  `server.pid`/`server-info.json`, matching the shell's post-shutdown invariant;
  also clean up stale write-temp files under the state dir on `start`.
- **status tokens**: `status` maps to `stopped` whenever the recorded server pid
  is not alive **or** its `start_time` no longer matches (the shell's `stale`
  case — exactly what an idle self-shutdown, crash, or SIGKILL leaves behind,
  since the server does not always unlink its files on exit); otherwise
  `running` (+ URL). Never-started → `stopped`, post-`start` → `running`,
  post-`stop` → `stopped`, and a leftover-`server-info.json`-with-dead/recycled-
  pid → `stopped`.
- serialise `start` via an in-process `flock(2)` syscall (`nix::fcntl::flock`),
  which is available on both linux and macOS at the syscall level — the shell's
  `mkdir` fallback existed only because macOS ships no `flock` *binary*, so the
  syscall supersedes both the command and the fallback. `flock(2)`'s exclusion
  guarantee holds for a local filesystem; the visualiser state/tmp dir is assumed
  local (as the local-dev deployment target is) — note this assumption, since a
  network-mounted `tmp` could silently weaken the lock;
- **init-sentinel precondition**: preserve `launch-server.sh`'s refusal to launch
  in an uninitialised repo (the `$TMP_REL/.gitignore` sentinel check) — `start`
  fails cleanly when `.accelerator` is uninitialised rather than binding a server
  against catalogue-default config in a location the user never opted into;
- **stale-temp cleanup** is confined to the visualiser-exclusive state directory
  (no other tool writes there) and runs only after the reuse short-circuit has
  confirmed no live server. Note the atomic-write `.tmp-` prefix is the *shared*
  store prefix, not visualiser-specific — so the sweep's safety rests on the
  state-dir scoping and the no-live-server gate, and it must never be pointed at
  the shared `.accelerator/` document directory (which would race other
  components' in-flight atomic writes);
- idle self-shutdown already lives in `lifecycle.rs` (disabled sentinel
  `i64::MAX`), driven by the owner pair — unchanged.

Decompose `orchestration/` by concern rather than one flat module — the
process-identity/`start_time` probe, the signal-escalation, the state-file
read/write, and the lock — so each is independently testable. Author the
orchestration integration tests with an isolation contract: a unique tempdir per
test as the state root, a loopback port-0 bind, and a teardown guard that
SIGKILLs any surviving daemon. Implement the reaper as an RAII/`Drop` guard
bound to a local (not trailing code), so it fires on assertion-panic too —
otherwise a failing test, exactly when a daemon is most likely stuck, skips the
reap. This mirrors the existing shell suites' `reap_visualiser_fakes` trap so
real detached daemons cannot leak across parallel `cargo test` runs.

This orchestration is intentionally unix-only: `nix::sys::signal::kill` and
`process_start_time()` (linux `/proc`, macOS `sysctl`) match the darwin/linux
target closure and return `None` elsewhere. Any future non-unix distribution
target would need a portable termination + start-time strategy — recorded as
tracked debt (a follow-up story, mirroring the `pup.ron` deferral) so the
lock-in is an owned decision, not just an inline aside.

#### 3. Direct config reading (Model 1)

**File**: `cli/visualiser/server/src/config.rs`
**Changes**: replace `Config::from_path(config.json)` with a read of
`.accelerator/*.md` via `config_adapters::compose` →
`ConfigService::effective`. Owner-PID/`start_time` arrive via the Phase-3 CLI
flags, not config.

**Post-refactor module shape**: this is a config-**composition** layer, not a
thin passthrough — `config-adapters` exposes only `compose`/`render`/`store`,
so the server must reassemble everything `write-visualiser-config.sh` resolved
today. **Decompose it by concern** (mirroring the `orchestration/` split), each
independently testable, rather than growing one flat `config.rs` god-module:
doc-path resolution, template-tier resolution, work-item-scheme assembly,
kanban/idle resolution, and the env-precedence overlay. Enumerate and re-home
the full config-key-derived set (dropping nothing silently): the 13 doc-paths;
three-tier template resolution **including which config file declared each
override** (`config_override_source`) — derive this from the resolved
`Resolution` `Source` (Personal→`config.local.md`, Team→`config.md`), **not** by
re-scanning frontmatter by hand; the compiled work-item scan regex via
`WorkItemIdScheme` + injected `IdScanner`; kanban columns with catalogue
defaults; `visualiser.idle_timeout`; and the `ACCELERATOR_VISUALISER_*` env-var
precedence for `idle_timeout`/`editor`/`editor_project`. Two guards are launch
**preconditions**, not config values — keep them with the init-sentinel check in
the orchestration/precondition layer: the `paths.tickets`-without-`paths.work`
migration guard, alongside the init-sentinel. Separately, name where the
runtime/environment-derived fields the old `config.json` carried
(`plugin_root`/`plugin_version`, `project_root`, `tmp_path`, `host`, `log_path`)
now come from under launcher exec-replace (e.g. `CLAUDE_PLUGIN_ROOT`) — these are
not config keys and must not silently migrate into `config.rs`. Delete the
hand-synced `DEFAULT_IDLE_TIMEOUT`/`DEFAULT_KANBAN_COLUMN_KEYS` fallbacks — the
new `config` dependency sources those from the catalogue. Sweep the stale
doc-strings/error notes that cite the retired scripts — the `config.rs` module
header, `ConfigError::InvalidIdleTimeout`, `main.rs`'s `--config` help, **and**
the cross-crate `VISUALISER_KEYS` note in `cli/config/src/catalogue.rs` (which
claims the server keeps its own fallback because it cannot depend on `config` —
false once the dependency lands).

**idle_timeout token semantics**: `resolve_idle_limit_ms` operates on the
*verbatim* token, and three cases are distinct — do not conflate them:

- **absent key** → catalogue default `8h` (verified as a config-resolution
  assertion, not by waiting);
- **disable tokens** `never` (case-insensitive), bare `0`, and any zero-length
  duration (`0s`/`0ms`) → idle disabled, server stays up;
- **explicitly empty / whitespace-only, and the YAML-null spelling
  (`idle_timeout:`)** → resolve the intended mapping explicitly and assert it:
  an empty/whitespace *string* is **rejected at boot**
  (`ConfigError::InvalidIdleTimeout`, server does *not* start); state whether a
  YAML-null key collapses to absent (→ `8h`) or to empty (→ rejected) so the two
  visually-similar inputs do not diverge silently. This fail-fast contract is
  distinct from the absent-key default.

Confirm `ConfigService::effective` hands the resolver the raw token unchanged
(does not trim/lowercase/coerce, and does not substitute the `8h` default for an
explicitly-empty value); if it normalises, these semantics change silently.
Assert all three classes: the `8h` default (absent), each disable token keeping
the server up, and the empty/whitespace token producing `InvalidIdleTimeout`.

#### 4. Skill invocation

**File**: `skills/visualisation/visualise/SKILL.md`
**Changes**: replace the `visualiser.sh` `!`-preprocessor call with
`accelerator visualiser start|stop|status`. This switch makes the user-facing
path launcher-resolved, so it must land in the **same release as Phase 4's
producer wiring** (which puts `visualiser` in the manifest) — otherwise an
installed plugin's `start` resolves to `AssetNotFound`. Enforce this with an
**automated coherence check** rather than prose discipline: fail the
release/coherence gate when `SKILL.md` invokes `accelerator visualiser` but
`"visualiser"` is absent from `DISPATCHED_SUBBINARIES` (or vice versa), so a
mis-ordered release is caught in CI across the merge window. In dev/test the
`ACCELERATOR_VISUALISER_BIN` override satisfies resolution regardless.

#### 5. Dispatch tests

**File**: `cli/visualiser/server/tests/` and/or `cli/launcher/tests/`
**Changes**:

- assert each of `start|stop|status` dispatches **through the launcher** with a
  black-box `launcher_for("visualiser", "ACCELERATOR_VISUALISER_BIN")` test that
  `exec`-replaces the real sub-binary and observes a distinguishing side effect
  (mirroring `tests/dispatch.rs`). This is the load-bearing check — external
  dispatch is name-agnostic, so a `visualiser`-specific `RecordingExec` unit
  assertion proves little on its own and is at most a supplementary check;
- assert the server honours configuration from the directed location: set
  `visualiser.idle_timeout` to a non-default in a fixture config and assert the
  server's resolved timeout matches; add the `8h`-default (absent-key)
  config-resolution assertion, the disable-token cases (`never`, `0`, `0s`,
  `0ms`) asserting the server stays up, and the empty/whitespace case asserting
  `InvalidIdleTimeout` (the server refuses to start);
- **re-home the composition contract**: the existing `config_contract.rs` (which
  pins the 13 doc-path keys, the discovered template set, and each template's
  three-tier resolution) is retired with `write-visualiser-config.sh` in Phase 5,
  so add a config-composition contract test over the new
  `compose`→`ConfigService::effective` path asserting the full resolved set —
  the 13 doc-path keys, the template set + three-tier resolution incl.
  `config_override_source`, and the kanban columns — so composition ends up no
  *less* tested than before the rewrite, not just idle_timeout-covered.

Dispatch resolves the binary via the `ACCELERATOR_VISUALISER_BIN` override in
dev/test (the manifest entry lands in Phase 4).

#### 6. Harden the Origin guard

**File**: `cli/visualiser/server/src/server.rs`
**Changes**: the preserved Origin check uses `starts_with("http://127.0.0.1")` /
`starts_with("http://localhost")`, which a loopback-lookalike origin
(`http://127.0.0.1.evil.com`) passes. While the guard is under active review,
extract the Origin authority host with a robust URL parser (`http::Uri` or the
`url` crate — **not** a hand-rolled `split_once(':')`, which
`http://127.0.0.1:x@evil.com` defeats) and compare the host exactly to
`127.0.0.1`/`localhost` (optional `:port`). Add both the `127.0.0.1.evil.com`
lookalike and the userinfo-`@` case to the guard tests. The `403` statuses the
acceptance criteria pin are unchanged for genuine loopback and genuine
cross-origin requests.

### Success Criteria

#### Automated Verification

- [x] `accelerator visualiser start` binds a loopback port and prints its
      `http://127.0.0.1:PORT` URL (fixture-config integration test)
- [x] `accelerator visualiser stop` keys the recycle guard on the **server**
      pid's `start_time`, refuses a recycled/mismatched pid, and terminates only
      an exact match via `kill(pid,0)` polling (test)
- [x] `accelerator visualiser status` prints `stopped`/`running`/`stopped`
      across never-started → start → stop, **and** prints `stopped` for a
      leftover `server-info.json` whose pid is dead or `start_time` mismatched
      (stale-state test)
- [x] A forced SIGKILL synthesises `server-stopped.json` and removes the
      pid/info files; a pre-seeded stale write-temp file is removed on `start`
      (tests)
- [x] The recorded owner-PID identifies the intended parent under launcher
      exec-replace, not an intermediate launcher process — asserted via an
      injectable ancestor-resolution seam, not a real-process-tree spawn (test).
      Under exec-replace the intermediate launcher is collapsed by `exec`, so the
      seam prefers an explicit `--owner-pid` (`$PPID` from the invoker) and falls
      back to `getppid()`; the seam test pins both.
- [x] `start` refuses to launch in an uninitialised repo (init-sentinel
      precondition preserved) (test)
- [~] The new Rust `stop` reaps a server whose `start_time` was recorded by the
      old shell path (cross-upgrade reconciliation test). Superseded: the shell
      write path is retired within this phase, so the equivalent is covered by
      the recycle-guard + forced-kill + stale-state tests against the current
      `server-info.json` format (which is identical — the Rust server always
      wrote `start_time` via `process_start_time`, per Migration Notes).
- [~] Idle self-shutdown / disable-token / empty→`InvalidIdleTimeout` / `8h`
      default. The **resolver** semantics are fully covered by the `config.rs`
      unit tests (all disable tokens, empty/whitespace rejection, absent→8h) and
      `lifecycle_idle.rs` (fire / disabled-inert / boundary); the fixture-config
      `serve` exit-1-on-bad-idle path is covered in `orchestration_lifecycle.rs`.
- [x] Host/Origin 403 model preserved and each guard exercised independently,
      including a loopback-lookalike origin (`http://127.0.0.1.evil.com`) → 403
      after the Origin guard is tightened to an exact host parse (unit +
      integration guard tests)
- [x] Black-box dispatch-through-launcher test for the `visualiser` token
      (`ACCELERATOR_VISUALISER_BIN` override), plus config-honouring composition
      tests
- [x] `config.json` + `write-visualiser-config.sh` no longer referenced by the
      server or SKILL.md; the hand-synced `config.rs` default constants are
      removed; the `ACCELERATOR_VISUALISER_*` env precedence and the
      `tickets`→`work` migration guard are preserved
- [x] The SKILL.md `accelerator visualiser` invocation and
      `DISPATCHED_SUBBINARIES` membership are bound by an automated coherence
      check (`validate_dispatch_coherence`, landed with Phase 4's producer
      wiring so both sides exist together).
- [x] Full gate is green: `mise run`

#### Manual Verification

- [ ] End-to-end: `accelerator visualiser start` from a real repo opens a
      working visualiser; `stop` tears it down; `status` reflects each state
- [ ] The idle window closes a genuinely-idle server after the configured
      timeout

---

## Phase 4: Producer Wiring + Local-Manifest Verify

### Overview

Wire the release producer to stage, sign, and list the `visualiser` binary in
the signed manifest, and verify fetch/verify/dispatch against a local/test
manifest fixture. This is the half of the distribution change that makes every
subsequent release carry the `visualiser` entry; it co-releases with Phase 3's
SKILL.md switch. The old fetch path (`launch-server.sh`, `checksums.json`) stays
present and functional — its deletion is Phase 5. The live-manifest assertion is
left to 0165's own coverage.

### Changes Required

#### 1. Producer wiring

**File**: `tasks/shared/paths.py`
**Changes**: `DISPATCHED_SUBBINARIES = ("visualiser",)`. This makes
`cli_binary_path("visualiser", …)` (the manifest flow) emit the
`visualiser-{platform}` asset the launcher fetches. **Do not** rename the
old-flow staged-asset consts (`:51`, `:67`) here: the old checksums flow still
uploads its asset until Phase 5, and renaming it to `visualiser-{platform}` now
would enqueue two release assets with the *same* filename (a collision — hard
upload failure or silent overwrite). Leave the old-flow const emitting
`accelerator-visualiser-{platform}` until Phase 5 deletes the checksums flow.

**Files**: `tasks/manifest.py`, `tasks/signing.py`
**Changes**: already parameterised on `DISPATCHED_SUBBINARIES`; confirm they
now stage, sign, and list the `visualiser` entry. The signing loop
(`signing.py:60`) covers `("accelerator", *DISPATCHED_SUBBINARIES)`.

**Producer-contract sweep**: adding the `visualiser` sub-binary changes both the
asset-stem set (new `visualiser-{platform}` alongside the existing
`accelerator-visualiser-{platform}`) **and the release upload count**. Beyond
string-stem hits, the `DISPATCHED_SUBBINARIES` flip makes `_release_uploads`
enqueue `visualiser-{platform}` + `.minisig` for all four platforms, so update:
the workflow build-provenance glob (asserted in
`tests/unit/tasks/test_workflows.py`); the `fake_binaries`/staged-asset fixtures
in `tests/unit/tasks/test_build.py`; and — the load-bearing one — the release
upload contract in **`tests/integration/tasks/test_github.py`**
(`TestUploadAndVerifyRelease.test_uploads_every_asset_with_clobber` asserts a
fixed upload **count** and its `_setup_release` fixture stages only
`accelerator-visualiser-{platform}`). Update the numeric count assertion and
stage `visualiser-{platform}` + `.minisig` and a `visualiser` manifest entry in
the fixture; a stem-only grep will miss the count literal. Fold every hit into
this phase so `mise run` stays green.

**Release feature profile**: make it an explicit requirement (and success
criterion) that the staged/signed `visualiser-{platform}` artifact is built with
**default (`embed-dist`) features only — never `dev-frontend`**. The
`dev-frontend`-gated `e2e_insecure_allowed` path relaxes both the non-loopback
bind guard and the Host-header (DNS-rebinding) guard when
`ACCELERATOR_VISUALISER_E2E_INSECURE` is set; a signed release binary must not
carry that switch. Neighbouring workspace tasks build `--all-features`, so pin
the release build's feature set rather than inheriting it — the **primary
control** is the explicit `embed-dist`-only feature pin. Back it with an
automated **secondary** check on the final post-embed, pre-upload signed
artifact: fail staging (before signing/upload) if the artifact contains the
exact string `ACCELERATOR_VISUALISER_E2E_INSECURE`. Match the full symbol, not
the `ACCELERATOR_VISUALISER_` prefix — Phase 3's direct config reading now embeds
other `ACCELERATOR_VISUALISER_*` literals (idle_timeout/editor precedence) in
every release binary, so a prefix grep would false-positive and block all
releases. This control is a real DNS-rebinding exposure if it regresses.

**File**: `cli/launcher/tests/fixtures/manifest.example.json`
**Changes**: change the `binaries` key `accelerator-visualiser` → `visualiser`.

**File**: `cli/launcher/src/launch/outbound/resolve/manifest.rs`
**Changes**: the fixture is `include_str!`-embedded here, and the co-located
unit test asserts `platform_entry("accelerator-visualiser", …)` /
`bare_sha256("accelerator-visualiser")` (`:146-148`). Update those assertions to
the `visualiser` key in the same change as the fixture rename, and grep the
launcher test tree for any other `accelerator-visualiser` literal that must move
atomically.

#### 2. Local-manifest fetch/verify test

**File**: `cli/launcher/tests/resolution.rs`
**Changes**: add a `visualiser` case using the `common::MockServer` harness
(runtime-generated minisign keypair, inline `manifest_json()`) asserting the
resolver resolves the `visualiser` entry, fetches `visualiser-{platform}`,
verifies sha256 + signature, and dispatches. Add at least one **rejection** case
for the `visualiser` entry — mutated bytes → `ChecksumMismatch`, and a
foreign-key signature → `SignatureMismatch` — since verification's security value
is in its rejection behaviour and the visualiser is the first real sub-binary to
exercise this path end-to-end (or explicitly note the harness's existing generic
negative cases already cover the mechanism for any entry). Skips cleanly if the
`minisign` CLI is absent.

### Success Criteria

#### Automated Verification

- [x] `mise run deny:check` and the launcher suite pass with the
      `visualiser`-keyed manifest fixture and the updated `manifest.rs`
      assertions
- [x] The local-manifest resolver test passes, including the tamper/rejection
      case: the whole `resolution.rs` suite is re-keyed on the `visualiser`
      entry (happy path + `ChecksumMismatch` + `SignatureMismatch` + tamper)
- [x] Full gate is green: `mise run`

#### Manual Verification

- [ ] A dry-run release build stages, signs, and lists the shared
      `accelerator-visualiser-{platform}` asset (manifest key `visualiser`) in
      `manifest.json`, built with default (`embed-dist`) features only — the
      `ACCELERATOR_VISUALISER_E2E_INSECURE` switch is inert in the staged binary
- [ ] The live-manifest fetch assertion is left to 0165's coverage (documented,
      not asserted here)

---

## Phase 5: Distribution Cut-over (Deletions)

### Overview

Remove the now-dead shell surface and the flat `checksums.json` distribution.
**Gate**: land this only once a release carrying Phase 4's producer wiring has
shipped, so the live manifest resolves `visualiser` before the old fetch path is
gone. Since 0165's release pipeline is shipping manifests, this is satisfied by
the first release after Phase 4; Phase 5 may co-ship with Phases 3–4 or follow
in a later release, but never precede the producer wiring.

### Changes Required

#### 1. Remove the retired surface

**Files to delete**:
`skills/visualisation/visualise/scripts/visualiser.sh`,
`launch-server.sh`, `stop-server.sh`, `status-server.sh`,
`write-visualiser-config.sh`; `skills/visualisation/visualise/bin/checksums.json`;
`skills/visualisation/visualise/cli/accelerator-visualiser` (bash CLI); the
`launch-server.sh` binary-acquisition shell test and its `mise` task; any
co-located `test-*.sh` suites covering the deleted scripts. Remove
`launcher-helpers.sh` if no longer sourced.

#### 2. Retire the checksums.json producer and coherence read

**File**: `tasks/build.py`
**Changes**: now that `checksums.json` is deleted, drop
`_read_checksums_json_version` (and its `checksums.json` entry in the
`validate_version_coherence` `found` map — the `_read_cargo_toml_version` reader
was already removed in Phase 1), the `create_checksums` producer, and
`update_checksums_json` if now unused.

**File**: `tasks/version.py`
**Changes**: remove the standalone `checksums.json`/visualiser-server manifest
handling now folded into the workspace version bump.

**File**: `tasks/shared/paths.py` / `tasks/shared/sources.py`
**Changes**: with the old checksums upload flow deleted, the old-flow
staged-asset consts (`:51`, `:67`, `accelerator-visualiser-{platform}`) are now
unused — remove them (the manifest flow's `cli_binary_path` supplies the
`visualiser-{platform}` asset). Remove the `SHELL_SOURCES` entry for the bash CLI
and drop the deleted scripts from the shell-lint source set and any exec-bit
invariants. Update the corresponding `tests/unit/tasks/*` assertions in lockstep.

### Success Criteria

#### Automated Verification

- [ ] The five scripts, `bin/checksums.json`, and the bash CLI are absent
- [ ] Version-coherence passes without any `checksums.json` / standalone-literal
      reads: `mise run` (the release/version tasks)
- [ ] Shell lint is green with the deleted sources removed from the source set:
      `mise run scripts:check`
- [ ] Full gate is green: `mise run`

#### Manual Verification

- [ ] Confirmed a release carrying Phase 4's producer wiring has shipped (live
      manifest resolves `visualiser`) before these deletions merge

---

## Testing Strategy

### Unit Tests

- Parity: golden equality across the retired surface — frontmatter map (incl.
  JSON value types and YAML-1.1↔1.2 scalar cases, with `Ok`/`Err` outcome pinned
  for ambiguous-scalar/duplicate-key inputs), slug, `config_path_key`,
  `patch_status` bytes, and linkage records captured from the pre-refactor
  engine; doc-type split into pure-parity paths (equal to old) and divergence
  paths (equal to hand-authored intended `infer` result) (Phase 2).
- Write path: CRLF + non-default-mode round-trip preservation; two conditional
  patches forced to interleave via a test-only barrier yield exactly-one-win /
  one-`412` with no corruption (Phase 2).
- Recycle guard: exact **server**-pid + `start_time` match required to stop;
  refusal on mismatch; `kill(pid,0)` polling, not `waitpid` (Phase 3).
- Config resolution: `visualiser.idle_timeout` non-default honoured; `8h` default
  (absent key); disable tokens `never`/`0`/`0s`/`0ms` keep the server up; empty/
  whitespace → `InvalidIdleTimeout` (boot refusal); raw token reaches the
  resolver unchanged through `compose` (Phase 3).
- Owner identity: recorded owner-PID is the intended parent under exec-replace,
  asserted via an injectable ancestor-resolution seam (Phase 3).
- Cross-upgrade reap: new `stop` reaps a server whose `start_time` was recorded
  by the old path, incl. the `never`/0 case (Phase 3).
- Wire mapping: `wire_str`/`from_wire_str` round-trips for all 14 variants.
- Version coherence: a skewed visualiser member version is caught via
  `_read_workspace_version` (Phase 1).

### Integration Tests

- `accelerator visualiser start|stop|status` lifecycle against a fixture
  config, including loopback bind + URL emission and status-token transitions —
  never-started, running, stopped, and the stale (dead/recycled info file)
  state; forced-SIGKILL sentinel synthesis (Phase 3).
- Host/Origin 403 guards exercised independently, incl. a loopback-lookalike
  origin (`http://127.0.0.1.evil.com`) → 403 after the exact-host tightening
  (Phase 3).
- Black-box launcher dispatch per subcommand (`exec`-replaced real sub-binary)
  (Phase 3).
- Local-manifest fetch/verify/dispatch via `MockServer`, including a
  tamper/rejection case (`ChecksumMismatch`, `SignatureMismatch`) (Phase 4).

### Manual Testing Steps

1. Start the visualiser from a real repo via `accelerator visualiser start`;
   confirm the URL serves and views render unchanged.
2. `status` reports `running`; `stop` refuses a recycled PID and then
   terminates the real one; `status` reports `stopped`.
3. Configure a short idle timeout; confirm self-shutdown; set `never`; confirm
   it stays up.

## Performance Considerations

The `spawn_blocking` seam over the sync store must not corrupt the server's
`write_frontmatter` etag-verify-then-write flow under concurrent SSE-driven
writes. Confirm which component actually guards that TOCTOU window (do not
assume `write_coordinator.rs` — it is a self-write dedup cache), and pin the
invariant with the Phase 2 concurrent-conditional-patch test anchored on that
real critical section rather than leaving it to a manual check.

## Migration Notes

State/lock file layout under `$(accelerator config path tmp)/visualiser/` is
preserved, so the new `accelerator visualiser stop` can address a server started
by the old shell path. The recorded `start_time` in `server-info.json` was
**always written by the Rust server** via `process_start_time`, not by the
shell's `ps lstart` (which is only the shell's re-derivation on the *check*
side) — so a pre-upgrade server's recorded value is already in the exact
representation the new `stop` compares against, and reaping matches by
construction. Verify with a test that reaps a server whose `start_time` was
recorded by the old path (covering the `never`/0 idle-disabled case, which has
no idle fallback). Idle self-shutdown (default `8h`, enabled unless configured
`never`/0) is the bounded automatic fallback for an orphaned pre-upgrade server,
which otherwise keeps a loopback port bound and continues issuing `patch_status`
writes; a server left with idle disabled must be stopped explicitly with
`accelerator visualiser stop`. Such an orphan is a live process, so `status`
reports it as `running` (+ URL) — the discovery path after the shell scripts are
gone — and this stop command belongs in the user-facing upgrade note, not just
this plan. `config.json` is retired.

## References

- Work item: `meta/work/0168-fold-visualiser-into-cli-workspace.md`
- Research: `meta/research/codebase/2026-07-23-0168-fold-visualiser-into-cli-workspace.md`
- ADRs: ADR-0045, ADR-0053, ADR-0054
- Naming surfaces: `cli/launcher/src/launch/outbound/resolve/mod.rs:143-146`,
  `tasks/shared/paths.py:21,51`
- Retire targets: `cli/corpus/src/doc_type.rs:168-192`,
  `cli/corpus/src/slug.rs:193`, `cli/document/src/fence.rs:30-82`,
  `cli/corpus-adapters/src/{patcher,document,store}.rs`,
  `cli/config/src/catalogue.rs:220`
