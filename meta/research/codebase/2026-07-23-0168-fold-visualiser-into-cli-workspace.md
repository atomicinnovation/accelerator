---
type: "codebase-research"
id: "2026-07-23-0168-fold-visualiser-into-cli-workspace"
title: "Research: Folding the Visualiser into the cli/ Workspace (0168)"
date: "2026-07-23T01:40:15+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0168"
parent: "work-item:0168"
relates_to: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
topic: "Folding the Visualiser into the cli/ Workspace (0168)"
tags: ["research", "codebase", "rust", "visualiser", "cli", "launcher", "corpus", "distribution", "workspace"]
revision: "393761cd0220577eb4a8470263ce4cb4039d1cd0"
repository: "build-system"
last_updated: "2026-07-23T01:40:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Folding the Visualiser into the cli/ Workspace (0168)

**Date**: 2026-07-23T01:40:15+00:00
**Author**: Toby Clemson
**Git Commit**: 393761cd0220577eb4a8470263ce4cb4039d1cd0
**Branch**: HEAD (jj working copy, `build-system` workspace)
**Repository**: build-system

## Research Question

For work item [0168 — Fold the Visualiser into the cli/ Workspace](../../work/0168-fold-visualiser-into-cli-workspace.md):
map the live codebase reality behind every requirement and acceptance criterion —
the visualiser crate as it stands, the shared `corpus`/`config`/`document` crates
it must retire onto, the `cli/` workspace and unified launcher it joins, the shell
orchestration and security lifecycle it must preserve, and the release/distribution
machinery it becomes the first live entry in — so a plan can be built on verified
facts rather than the story's (mostly-accurate but occasionally stale) prose.

## Summary

Every retire target named in the story's Technical Notes **exists** in the shared
crates and is public, and every prerequisite (0178/0179/0164) is landed. The fold-in
is genuinely buildable today. The research surfaced a small number of places where
the story's framing is stale or under-specifies a real decision the plan must make:

1. **The frontmatter engine is `serde_yml`, not `gray_matter`.** `gray_matter` is
   declared in `server/Cargo.toml` but the actual parser (`frontmatter.rs`) uses
   `serde_yml` directly (wrapped in `catch_unwind` to survive libyml panics). Both
   deps should be dropped; the story only names them jointly.

2. **`corpus::DocTypeKey` is *not* serde-free-with-no-wire-mapping.** The story says
   the wire mapping "must be re-homed in a thin server view type over
   `wire_str`/`from_wire_str`" — but `corpus::doc_type` already ships `wire_str()` and
   `from_wire_str()` (hand-rolled kebab-case). The server can call them directly; only
   the API view types that need serde derives wrap them. This *reduces* the work the
   story anticipated.

3. **The launcher forwards *no* config to sub-binaries** (Open Question 3). Dispatch
   is name-agnostic `exec`-replace; the child inherits the environment and must read
   `.accelerator/*.md` itself. The `config`/`config-adapters` reader entry point
   (`config_adapters::compose` → `ConfigService::effective`) exists and already carries
   the `visualiser.idle_timeout = "8h"` catalogue default, so the server reading config
   directly is the low-friction path and matches ADR-0054's "Model 1".

4. **Naming reconciliation is required and currently unresolved.** The launcher derives
   the manifest key **and** asset filename from the *bare subcommand token*:
   `accelerator visualiser start` → looks up `manifest.binaries["visualiser"]` and fetches
   asset `visualiser-{platform}`, override var `ACCELERATOR_VISUALISER_BIN`. But the crate/bin
   is `accelerator-visualiser`, the release producer's `DISPATCHED_SUBBINARIES` comment says
   append `"accelerator-visualiser"`, and the golden manifest fixture keys on
   `accelerator-visualiser`. These do not match. The plan must pick one (rename the
   subcommand token, or key the manifest/asset on `visualiser`, or add a launcher alias).
   ADR-0054 asserts the *intent* (`accelerator visualiser …` → the `accelerator-visualiser`
   binary) but the dispatch code does not bridge the name gap. The 0168 review's Pass-3/6
   residual already flagged this "bare-noun referent drift."

5. **No routing-table change is needed** to add `visualiser` — clap's
   `#[command(external_subcommand)]` catch-all already routes any unknown token. The work
   is producing the sub-binary, publishing the manifest entry, and homing the
   `start|stop|status` orchestration inside the sub-binary.

6. **The `store/` crate exists** as an 11th workspace member (0180 landed), even though
   0168 is "not blocked by 0180" — the fold-in reuses `corpus-adapters`' existing
   `FileCorpusStore`/`AtomicWrite` write path, which is present.

## Detailed Findings

### 1. The visualiser crate as it stands (`skills/visualisation/visualise/`)

The crate is `accelerator-visualiser`, a `[lib]` + `[[bin]]` split
(`server/Cargo.toml:17-21`; bin → `src/main.rs`, integration tests consume the lib
at `src/lib.rs:3-5`). Version is a **hand-copied literal** `version = "1.24.0-pre.15"`
(`Cargo.toml:3`, `publish = false`), surfaced at runtime via `env!("CARGO_PKG_VERSION")`
(`lib.rs:7`). AC7 requires this become `version.workspace = true`.

**Modules that retire** (domain/parsing logic, moves out):

- `docs.rs:6-21` — `DocTypeKey` (14 variants) deriving `Serialize/Deserialize`
  `#[serde(rename_all="kebab-case")]` (`docs.rs:4-5`), **plus** a hand-written
  `wire_str()` (`:153-170`) / `from_wire_str()` (`:175-177`) with a test pinning agreement
  (`:278-283`). Also `config_path_key()` (`:43-65`, distinct from wire tokens — e.g.
  `WorkItems→"work"`, `Research→"research_codebase"`, `PrDescriptions→"prs"`), `label()`,
  `in_lifecycle()`, `carries_target_frontmatter()`, `describe_types(cfg)` (`:180-222`).
- `slug.rs` — `derive(kind, filename, cfg)` (`:22-72`), `derive_work_item_with_regex`
  (`:8-20`), `humanise_slug` (`:196-204`), and a **private** `title_case_segment` (`:232-240`,
  comment notes deferred unification with `api::library::humanise_status`).
- `frontmatter.rs` — engine is **`serde_yml` 0.0.12** (`:144`, wrapped in
  `std::panic::catch_unwind` `:143-154` because libyml panics on adversarial trailing
  whitespace). `fence_offsets(raw) -> Result<Option<(usize,usize)>, FenceError>` (`:21-70`,
  1 MiB cap, CRLF-tolerant), `parse()` (`:95-194`), `FrontmatterState::{Parsed(BTreeMap),
  Absent, Malformed}` (`:72-77`), plus `title_from` (`:284-310`) and `read_ref_keys` (`:320-370`).
- `patcher.rs` — `patch_status(raw, new_value) -> Result<Vec<u8>, PatchError>` (`:34-94`),
  line-preserving top-level `status:` replacement over `fence_offsets`.
- `typed_ref.rs` — `parse_typed_ref(raw) -> Option<TypedRef>` (`:32-72`), prefixes
  `work-item:`/`plan:`/`adr:`/`pr:`, path-shaped payloads → `TypedRef::Path`.
- `config.rs` — `Config::from_path` reads the launcher-emitted **config.json** via
  `serde_json` with `#[serde(deny_unknown_fields)]` (`:14`, `:281-291`). The ID logic
  (`WorkItemConfig`, the `WorkItemIdScheme`-analogue) with `extract_id`/`normalise_id`
  (`:178-229`). **`resolve_idle_limit_ms`** (`:384-410`): `DEFAULT_IDLE_TIMEOUT = "8h"`
  when absent (`:352`, `:385`), `"never"`/bare `"0"`/zero-length duration → disabled
  sentinel `i64::MAX` (`:364`, `:389-402`), otherwise `humantime::parse_duration`. A drift
  test ties the `"8h"` default to `lifecycle.rs`'s constant (`:791-794`).
- `file_driver.rs` — the async `FileDriver` **port trait** (`:53-94`, hand-rolled
  `Pin<Box<dyn Future>>`, no `async-trait`). `LocalFileDriver::kind_for_canonical_path`
  (`:527-532`) keys doc-type off *which configured root a path sits under* — **not** the
  `corpus::doc_type::infer` longest-segment matcher (a decision the story flags). Also
  `atomic_write_preserving_perms` (`:187-262`, `spawn_blocking` + NamedTempFile + fsync).
- `related.rs`, `clusters.rs`, `cluster_key.rs` — linkage/cluster logic delegating to
  `typed_ref`/`indexer` and computing lifecycle clusters (`clusters.rs:151-311`).
  **Naming caveat:** `src/lifecycle.rs` is the idle/owner-death shutdown loop (stays),
  **not** linkage; the lifecycle HTTP handler is `src/api/lifecycle.rs`.

**Parts that stay** (axum/tokio/notify — the crate is retained, not deleted):

- `server.rs` — router assembly (`:212-270`), **loopback bind** (`bind_host_is_allowed`
  `:619-621`, binds `SocketAddr::new(host, 0)` random port `:310-311`), **Host guard →
  403** (`host_header_guard` `:633-650`), **Origin guard → 403** for `PATCH/POST/PUT/DELETE`
  (`origin_guard` `:652-678`, missing Origin allowed). E2E insecure bypass compiled only
  under `dev-frontend` + env var (`:607-615`).
- `lifecycle.rs` — the idle-shutdown tokio task (`:29-52`); disabled = `i64::MAX` sentinel.
- `shutdown.rs`, `sse_hub.rs`, `indexer.rs`, `watcher.rs`, `write_coordinator.rs`, `assets.rs`.

**Embed wiring** (stays; AC8): the literal `../frontend/dist` appears in **three** places
that cannot share a const — `build.rs:5` (`FRONTEND_DIST_REL`, duplicated because build.rs
can't import the crate), `assets.rs:9` (the const), and `assets.rs:71` (the `rust_embed`
`#[folder = "../frontend/dist"]` proc-macro attribute, which can't reference the const).
`build.rs` only acts under `CARGO_FEATURE_EMBED_DIST` and asserts `../frontend/dist/index.html`
exists, failing with a "run npm build" message otherwise (`build.rs:8-19`). Features:
`default=["embed-dist"]`, `embed-dist`, `dev-frontend` (`Cargo.toml:12-15`). Because the
`server`+`frontend` pair moves as a *unit* to `cli/visualiser/{server,frontend}`, the three
`../frontend/dist` literals stay valid unchanged (the story's central assumption — confirmed
by the relative layout).

### 2. The shared crates (retire targets all exist and are public)

The `cli/` workspace has **11 members** (`cli/Cargo.toml`): `kernel`, `document`, `config`,
`config-adapters`, `corpus`, `corpus-adapters`, `vcs`, `vcs-adapters`, `store`, `launcher`,
`verify`. There is **no `cli/visualiser/` yet** (confirmed). Workspace lint config lives at
`cli/{deny.toml, pup.ron, rustfmt.toml, clippy.toml}`.

Retire-target map (each verified with signature):

| Story's target | Actual location | Signature / note |
|---|---|---|
| `corpus::doc_type::DocTypeKey` | `cli/corpus/src/doc_type.rs:8-24` | 14 variants, **no serde**, `DocTypeKey::all()` (`:28`). |
| doc-type wire mapping | `cli/corpus/src/doc_type.rs:168-192` | `wire_str()` + `from_wire_str()` **already present** (kebab). Also `config_path_key` (`:48`), `linkage_type_name` (`:71`) — three distinct surfaces. |
| `doc_type::infer` | `cli/corpus/src/doc_type.rs:198-217` | `infer(path, table) -> Option<DocTypeKey>`, longest whole-segment match. |
| `corpus::slug` | `cli/corpus/src/slug.rs` | `derive(kind, filename, &scheme, &dyn IdScanner)` (`:10`), `humanise_slug` (`:160`), **`title_case_segment` is `pub`** (`:193`, doc-comment names it the retire target). |
| `document::fence_offsets` | `cli/document/src/fence.rs:30-82` | `fence_offsets(&[u8]) -> Result<Option<(usize,usize)>, DocumentError>`, 1 MiB cap, CRLF-aware, `Err(Unterminated)`. |
| frontmatter parse | `cli/corpus-adapters/src/document.rs:30-33` | `corpus_adapters::parse(&[u8]) -> ParsedDocument` (classifying wrapper) over `document::parse` (`cli/document/src/parse.rs:16`, serde-saphyr, **confined to `document`**). |
| `patch_status` | `cli/corpus-adapters/src/patcher.rs:48-94` | `patch_status(&[u8], &str) -> Result<Vec<u8>, PatchError>`. |
| `corpus::typed_ref` | `cli/corpus/src/typed_ref.rs:30-70` | `parse_typed_ref(&str) -> Option<TypedRef>`; `TypedRef::{WorkItem,Plan,Adr,Pr,Path}`. |
| `WorkItemIdScheme` + `IdScanner` | `cli/corpus/src/work_item_id.rs` | `trait IdScanner { fn scan(&self,&str)->Option<IdScan> }` (`:15`); `WorkItemIdScheme` (`:20`, `extract_id(&self,filename,&dyn IdScanner)` `:115`). Concrete impl **`corpus_adapters::RegexScanner`** (`cli/corpus-adapters/src/scanner.rs:10-40`) keeps `regex` out of the kernel. |
| `AtomicWrite` + `FileCorpusStore` | port at `cli/corpus/src/store.rs:59-64`; impl at `cli/corpus-adapters/src/store.rs` | `AtomicWrite` is a **`corpus`** trait; `FileCorpusStore` (its only impl) is in `corpus-adapters`, root-bounded, preserves perms, refuses symlink escapes (`StoreError::UnsafePath`). |
| `corpus::linkage` | `cli/corpus/src/linkage.rs` | `parse_document(source_type, content, table) -> Vec<LinkageRecord>` (`:565`), `TYPE_PAIRS` (**16 rows**, `:60-77`), `enum Band {Resolved, Ambiguous}` (`:21`). |
| config reader | `cli/config-adapters/src/compose.rs:33-44` | `compose(cwd, policy) -> Result<Composed, ConfigError>`; scalar reads via `config::ConfigAccess::{get, effective, effective_nonempty}` (`cli/config/src/service.rs:328-417`). `Key::parse("visualiser.idle_timeout")`. |
| `visualiser.idle_timeout` default | `cli/config/src/catalogue.rs:220` | `("visualiser.idle_timeout", Default::Scalar("8h"))` — **the `8h` default already lives here**. Doc-comment (`:199-206`) explicitly names the server's own `config.rs` fallback as the retire target. Other keys: `visualiser.kanban_columns` (default), `visualiser.editor/editor_project/binary` (absent-means-off, no default, `:128-130`). |

**Sync vs async (Open Question 2):** `corpus` and `corpus-adapters` are **fully
synchronous** — zero `async`/`tokio`/`spawn_blocking` in either crate; `document` is sync
serde-saphyr. There are no async variants. So the async I/O boundary stays in the visualiser
crate: a `spawn_blocking` wrapper (or thin async façade) over the sync store primitives is the
appropriate integration, and nothing in the shared crates conflicts with that. This directly
answers OQ2 in favour of "server keeps an async façade over the sync store."

**Naming hazards for the refactor** (call out in the plan):
- Two `parse`s: `document::parse` (raw → `Yaml`, fallible) vs `corpus_adapters::parse`
  (bytes → classified `ParsedDocument`, infallible).
- `AtomicWrite` lives in `corpus`; its only implementor `FileCorpusStore` in `corpus-adapters`.

### 3. The unified launcher & dispatch (`cli/launcher`, 0164)

The `accelerator` binary is the composition root. `main()` (`src/main.rs:213`) parses a
3-arm clap tree (`launch/inbound/cli.rs:16-29`): `Version`, `Config`, and
`External(Vec<OsString>)` marked `#[command(external_subcommand)]` (`:27-28`). `dispatch`
(`launch/mod.rs:176-198`) runs `version`/`config` **in-process** and routes any unknown
first token to `run_external` → `resolver.resolve()` → **`exec`-replace** via `UnixExec`
(`outbound/exec.rs:12-23`, `CommandExt::exec` — exit codes and signals propagate).

**Adding `visualiser` needs no routing change.** `accelerator visualiser start` gives
`ExternalCommand{name:"visualiser", args:["start"]}` (`core.rs:25-32`), forwarded verbatim.
The sub-binary contract is minimal: standalone executable, args as `argv[1..]`, owns its exit
codes, inherits the environment; no IPC/stdin/required-flags. `foo --help` also routes to
External, so the child renders its own help.

**Fetch/verify/cache** (`outbound/resolve/mod.rs`): `override_path(name)` first (offline
escape hatch), else fetch `manifest.json` + `manifest.minisig`, verify the detached
signature over **raw bytes before parsing** (`load_manifest` `:116-135`), derive
`asset_name = "{name}-{platform}"` (`:143`), look up `manifest.platform_entry(name, platform)`
(`:145`), fetch, `verify_binary` (sha256 then minisign), cache keyed `{name}-{version}-{sha256}`.
`HOST_PLATFORM ∈ {darwin-arm64, darwin-x64, linux-arm64, linux-x64}` (`:20-28`). Embedded key
`keys/accelerator-release.pub` (id `683A47B0B7AC4AD0`) via `include_str!` (`keys.rs:11-12`,
`build.rs:28-45`). Cache root `${CLAUDE_PLUGIN_ROOT}/bin` then `ACCELERATOR_CACHE_DIR`, no XDG.

**Test seams for the AC12 dispatch assertion** (both directly reusable):
- **Unit spy:** `RecordingExec` (`core.rs:287-301`) records `(program, args)` reaching `exec`;
  canonical test `run_external_execs_the_resolved_path_with_forwarded_args` (`:327-344`).
- **Black-box:** `accelerator-fixture` bin + `tests/dispatch.rs` runs the *real* launcher,
  pointing the resolver at the fixture via `ACCELERATOR_<SUB>_BIN` (`launcher_for` `:17-23`);
  distinguishing side effects = exit-code / signal / help-sentinel propagation. A `visualiser`
  test mirrors `launcher_for("visualiser", "ACCELERATOR_VISUALISER_BIN")`.
- **E2E resolver harness:** `tests/resolution.rs` spins `common::MockServer` with a
  runtime-generated minisign keypair and inline `manifest_json()` — the exact pattern AC11's
  "local/test manifest fixture" wants; skips cleanly if the `minisign` CLI is absent.

**Config at dispatch (Open Question 3 — answered):** the `External` arm **never** composes
config; `UnixExec` injects nothing beyond the inherited environment. A sub-binary needing
config reads it itself. No launcher config-passing seam exists for externals today (it would
be ADR-0054's reserved "Model 2"). Recommendation for the plan: server reads `.accelerator/*.md`
directly via `config`/`config-adapters` (Model 1), which also lets `config.json` and
`write-visualiser-config.sh` be retired.

### 4. Shell orchestration lifecycle to preserve (`.../visualise/scripts/`)

Five scripts (+ `launcher-helpers.sh` sourced lib, + 7 co-located `test-*.sh` suites).
`visualiser.sh` is a 25-line dispatcher (`start→launch-server.sh`, `stop→stop-server.sh`,
`status→status-server.sh`, `:16-24`), invoked from `SKILL.md:30` via the `!` preprocessor.

State/config/lock files all under `$PROJECT_ROOT/$(accelerator config path tmp)/visualiser/`
(`launch-server.sh:17-25`): `server-info.json` (url+pid+start_time), `server.pid`, `server.log`,
`server.bootstrap.log`, `config.json`, `server-stopped.json`, `launcher.lock`.

- **`launch-server.sh` — two roles.** (a) *Daemon:* `nohup "$BIN" --config "$CFG"` + `disown`
  (`:202-204`); the shell does **not** write PID/start_time — the **Rust server** writes
  `server.pid` + `server-info.json` via `process_start_time()` (`server/src/server.rs:324-344`,
  `:528`), and the shell polls up to 5 s for them (`:206-213`). A reuse short-circuit checks
  live PID **and** matching `start_time` before relaunch (`:30-43`). Owner-PID/start_time
  handshake computed and passed into config for owner-death shutdown (`:174-181`). Serialised by
  flock / `mkdir` fallback (`:59-77`). (b) *Fetch/distribution:* tri-precedence bin resolution
  (`ACCELERATOR_VISUALISER_BIN` > `visualiser.binary` config > download), reading expected SHA
  from `bin/checksums.json`, rejecting the all-zeros "no release" sentinel and version drift,
  downloading TLS-pinned `curl` from GitHub Releases, `sha256` verify, `install -m 0755` to cache
  (`:105-170`). Role (a) re-homes into `accelerator visualiser start`; role (b) is what the
  launcher's fetch/verify replaces.
- **`stop-server.sh` — recycle guard** (via `launcher-helpers.sh:139-201`): requires **both**
  `kill -0` liveness **and** exact `start_time` string match; on mismatch it **refuses**
  (prints `{"status":"refused",...}`, removes stale files, returns 1 — AC2). Then SIGTERM,
  2 s wait, SIGKILL escalation.
- **`status-server.sh`** (`launcher-helpers.sh:112-135`): emits a **JSON object**, not a bare
  token — `{"status":"not_running"|"running"|"stale",...}`. **AC3 asks for a `running`/`stopped`
  stdout token** — so the Rust re-home changes the status output shape (`stopped` where shell
  said `not_running`/`stale`). The plan must decide the exact tokens.
- **`write-visualiser-config.sh`** emits `config.json` (`jq -n`, `:305-354`) from the
  `accelerator config` CLI: `plugin_root/version`, `project_root`, `tmp_path`, hardcoded
  `host:"127.0.0.1"`, `owner_pid/start_time`, 14 `doc_paths`, `templates`, `work_item`
  (compiled regex + id_pattern), `kanban_columns`, and `idle_timeout`/`editor` spliced only when
  non-empty. If the server reads config directly, this whole script and the `config.json`
  contract can be retired (OQ3).
- **Idle timeout** flows *only* through `config.json` today; the shell shape-guard is a UX
  backstop, Rust `resolve_idle_limit_ms` is authoritative (`8h` default, `never`/`0`/zero-length
  disable). **Security model is purely Rust** — shell only writes `host:"127.0.0.1"` and
  regex-validates the returned URL.

### 5. Release/distribution — becoming the first live manifest entry (0165)

New contract: signed `manifest.json` (schema v1) `{schema_version, version, binaries:
BTreeMap<String, {description, platforms: BTreeMap<String,{sha256, signature}>}>}`
(`cli/launcher/src/launch/outbound/resolve/manifest.rs:26-46`). No `url` field — synthesised.
`version` checked for **exact** equality vs `CARGO_PKG_VERSION` (anti-rollback, `:103-108`);
all-zeros `sha256` = "no binary" sentinel; higher `schema_version` fails closed. Producer
mirrors this in `tasks/manifest.py` (`PlatformAsset/ManifestBinary/Manifest` TypedDicts,
`:24-37`), signs `manifest.minisig` over shipped bytes (`:111-130`), and enforces **version
coherence** across `plugin.json`, the visualiser server `Cargo.toml`, `checksums.json`,
`cli/Cargo.toml` workspace version, any pinned member, and `manifest.json`
(`tasks/build.py:184-210`).

**The manifest carries no visualiser entry yet** —
`DISPATCHED_SUBBINARIES: tuple[str,...] = ()` (`tasks/shared/paths.py:21`, comment: "empty at
HEAD; 0168 appends the visualiser"). The golden fixture
(`cli/launcher/tests/fixtures/manifest.example.json`) shows the target shape keyed on
**`accelerator-visualiser`**.

**Old bespoke path being retired** (AC10): `skills/visualisation/visualise/bin/checksums.json`
— a *flat* `{version, note, binaries: platform→"sha256:..."}` (no signatures), consumed by
`launch-server.sh` with SHA-256-only verification (no minisign, no SLSA on the download path).
Produced by `tasks/build.py:create_checksums` (`:418-429`). AC10 removes this + the five scripts
once `accelerator visualiser` is in place.

**Distribution cut-over ordering (from the story + review):** removing `launch-server.sh` +
`checksums.json` is only safe once 0165's manifest actually carries the visualiser entry.
Within-story, AC11 verifies fetch/verify/dispatch against a **local/test manifest fixture** (the
`resolution.rs` MockServer pattern); the live-manifest assertion defers to 0165's entry landing.

### 6. The naming reconciliation (concrete, unresolved)

The single most important open decision the plan must settle:

| Surface | Value the code uses | Source |
|---|---|---|
| Subcommand token | `visualiser` | `accelerator visualiser start` |
| Launcher manifest lookup key | `visualiser` | `resolve/mod.rs:145` (`platform_entry("visualiser", …)`) |
| Launcher asset filename | `visualiser-{platform}` | `resolve/mod.rs:143` |
| Launcher override var | `ACCELERATOR_VISUALISER_BIN` | `core.rs:215-240` (also what the shell uses today) |
| Crate / bin name | `accelerator-visualiser` | `server/Cargo.toml:2` |
| Producer `DISPATCHED_SUBBINARIES` | `"accelerator-visualiser"` | `tasks/shared/paths.py:21` comment |
| Golden manifest fixture key | `accelerator-visualiser` | `manifest.example.json:5` |
| Old GitHub asset name | `accelerator-visualiser-{OS}-{ARCH}` | `launch-server.sh:151` |

ADR-0054 states the *intent* — "`accelerator visualiser …` resolves to the
`accelerator-visualiser` binary" (`ADR-0054:44-49, 126-127`) — but the dispatch derives its
lookup key from the bare token and has no alias layer, so the intent and the code diverge. The
plan must choose: (a) publish the manifest/asset under key `visualiser` (change the producer +
fixture + old asset name), (b) rename the subcommand token to `accelerator-visualiser` (ugly,
against ADR-0054's UX), or (c) add an alias/mapping in the launcher's resolver. Option (a) is
the smallest change and aligns with the bare-token dispatch already shipped.

## Code References

- `skills/visualisation/visualise/server/src/frontmatter.rs:144` — actual engine is `serde_yml` (not `gray_matter`).
- `skills/visualisation/visualise/server/src/config.rs:384-410` — `resolve_idle_limit_ms`, `8h` default, disable tokens.
- `skills/visualisation/visualise/server/src/server.rs:633-678` — Host guard + Origin guard (403).
- `skills/visualisation/visualise/server/src/file_driver.rs:527-532` — `kind_for_canonical_path` (root-based doc-type keying).
- `skills/visualisation/visualise/server/build.rs:5` + `src/assets.rs:9,71` — the three `../frontend/dist` literals.
- `skills/visualisation/visualise/server/Cargo.toml:3` — hand-copied version literal (→ `version.workspace`).
- `skills/visualisation/visualise/scripts/launcher-helpers.sh:157-167` — recycle-guard refusal on start_time mismatch.
- `skills/visualisation/visualise/scripts/status-server.sh` / `launcher-helpers.sh:112-135` — JSON status output (`not_running`/`running`/`stale`).
- `skills/visualisation/visualise/bin/checksums.json` — old flat checksum manifest (retire).
- `cli/corpus/src/doc_type.rs:168-192` — `wire_str`/`from_wire_str` already present.
- `cli/corpus/src/store.rs:59-64` + `cli/corpus-adapters/src/store.rs` — `AtomicWrite` port + `FileCorpusStore`.
- `cli/config/src/catalogue.rs:220` — `visualiser.idle_timeout = "8h"` default.
- `cli/launcher/src/launch/mod.rs:176-198` — dispatch (External arm never composes config).
- `cli/launcher/src/launch/outbound/resolve/mod.rs:143-145` — asset/manifest key from bare token.
- `cli/launcher/src/launch/core.rs:287-344` — `RecordingExec` spy + canonical dispatch test.
- `cli/launcher/tests/resolution.rs` + `tests/common/mod.rs` — MockServer e2e fetch/verify harness.
- `tasks/shared/paths.py:21` — `DISPATCHED_SUBBINARIES = ()` ("0168 appends the visualiser").
- `tasks/build.py:184-210` — version coherence gate.

## Architecture Insights

- **Ports-and-adapters is already the shape on both sides.** The visualiser has an async
  `FileDriver` port; `corpus` exposes `AtomicWrite`/`RecordStore` ports with `FileCorpusStore`
  and `RegexScanner` as adapters, and `IdScanner` injected as `&dyn`. The fold-in is mostly
  deleting the visualiser's private domain modules and wiring its async adapter layer to the
  shared sync core across a `spawn_blocking` seam.
- **serde-saphyr is deliberately confined to `document`.** The visualiser must not re-derive
  serde on `corpus::DocTypeKey`; it uses `wire_str`/`from_wire_str` for API tokens and keeps
  serde only on its own wire view types. This is the crate-isolation discipline ADR-0053/0054
  established, and it *shrinks* the work the story anticipated for the wire mapping.
- **The launcher's name-agnostic external dispatch means the "first sub-binary" milestone is
  mostly a naming + producer-wiring exercise, not a dispatch-code exercise.** The load-bearing
  fetch/verify/cache/exec pipeline already exists and is tested; 0168 exercises it end-to-end
  rather than building it.
- **Two behaviour-preservation surfaces the review flagged and the plan must test explicitly:**
  the frontmatter/slug/doc-type engine swap (AC7 parity golden fixtures, frozen *before*
  deletion) and the Host/Origin 403 guards (AC5, exercised independently).

## Historical Context

- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md` — the settled
  dispatch model; explicitly names the visualiser as the first on-demand sub-binary and the
  `accelerator visualiser …` → `accelerator-visualiser` intent, and holds config Model 2 in
  reserve (relevant to OQ3).
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md` — the hexagon +
  inward-dependency rule the crate split obeys. ADR-0045 — skills-vs-CLI division.
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md` — the
  source research this story derived from (0136 epic scope/architecture).
- `meta/research/codebase/2026-07-03-0164-...`, `2026-07-06-0165-...`, `2026-07-07-0178-...`,
  `2026-07-11-0179-...`, `2026-07-19-0180-...` — dependency research; plans and validations for
  each landed under `meta/plans/` and `meta/validations/`.
- `meta/reviews/work/0168-fold-visualiser-into-cli-workspace-review-1.md` — the 6-pass work-item
  review (final **APPROVE**). Confirms: 0165 ordering handled via the within-story stand-in;
  parity + Host/Origin ACs added; the residual "visualiser bare-noun referent drift" (crate vs
  binary vs product) explicitly logged but left for planning — i.e. the naming reconciliation in
  §6 is a known, deliberately-deferred decision, not an oversight.

## Related Research

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md` (parent epic scope)
- `meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md` (the crates being retired onto)
- `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md` (the dispatch host)
- `meta/research/codebase/2026-07-06-0165-multi-binary-distribution-release-pipeline.md` (the manifest joined)

## Open Questions

1. **Naming reconciliation (§6)** — publish the manifest/asset under `visualiser`, rename the
   subcommand, or add a launcher alias? (Recommend: key on `visualiser`.) Everything else is
   downstream of this.
2. **OQ2 (async/sync):** the shared store is sync; recommend an async façade / `spawn_blocking`
   in the visualiser crate rather than adding async variants to `corpus-adapters`. Confirm this
   satisfies the visualiser's `write_frontmatter` etag-verify-then-write flow under load.
3. **OQ3 (config source):** recommend the server reads `.accelerator/*.md` directly via
   `config`/`config-adapters` (Model 1), retiring `write-visualiser-config.sh` + the `config.json`
   contract. Confirm every field `config.json` supplies today (14 doc_paths, templates roster,
   work_item regex, kanban columns, owner_pid/start_time) has a `config`-crate reader path — the
   owner_pid/start_time handshake in particular is process-runtime data, not config, and needs a
   non-config channel once the shell wrapper is gone.
4. **OQ1 (frontend toolchain path):** does `cli/visualiser/frontend` get its own Biome/vitest/
   Playwright task path under the mise/invoke tree, or stay as-is? (Story: decided at
   implementation.)
5. **Status output shape:** AC3 wants `running`/`stopped` stdout tokens; the shell emits a JSON
   object with `not_running`/`running`/`stale`. Pin the exact Rust token vocabulary.
6. **`gray_matter` is dead code today** — confirm nothing else in the crate uses it before
   dropping it alongside `serde_yml` (AC6 names both).
