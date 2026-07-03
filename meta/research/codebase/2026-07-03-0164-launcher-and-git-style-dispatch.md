---
type: codebase-research
id: "2026-07-03-0164-launcher-and-git-style-dispatch"
title: "Research: Launcher and Git-Style Dispatch (0164)"
date: "2026-07-03T16:58:28+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0164"
parent: "work-item:0164"
relates_to: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
topic: "Launcher and Git-Style Dispatch"
tags: [research, codebase, rust, launcher, dispatch, cli, fetch-verify-cache-exec, minisign]
revision: "9b12f6dec3eb8a38831637b01d2966198e9ddcd8"
repository: "accelerator"
last_updated: "2026-07-03T16:58:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Launcher and Git-Style Dispatch (0164)

**Date**: 2026-07-03 16:58 UTC
**Author**: Toby Clemson
**Git Commit**: 9b12f6dec3eb8a38831637b01d2966198e9ddcd8
**Branch**: (jj-colocated; detached HEAD)
**Repository**: accelerator

## Research Question

How do I implement work item 0164 "Launcher and Git-Style Dispatch" — the
Rust `accelerator` launcher's clap `external_subcommand` dispatch and the
on-demand fetch→verify→cache→exec pipeline, fronted by a thin bash bootstrap?
What already exists in the codebase, what are the governing ADR/spike/research
decisions, what contracts must the implementation honour, and what is the
concrete delta from the 0163 scaffold?

## Summary

0164 is **Phase 1** of the 0136 Rust-CLI-migration epic and sits directly on
top of the 0163 scaffold. The scaffold delivered a working two-crate workspace
(`cli/launcher` + `cli/kernel`) producing the `accelerator` binary with a
single built-in `version` subcommand in a strict hexagonal (ports-and-adapters)
layout. **Everything the story needs — the `external_subcommand` arm, Unix
`exec`, `reqwest`/rustls fetch, sha256 + minisign verification, the cache
layer, manifest-driven help, and the bash bootstrap — is absent and is exactly
0164's work surface.**

Three things shape the implementation decisively:

1. **The design is fully specified but nowhere implemented.** ADR-0054 (dispatch
   model) and ADR-0046 (distribution model) settle the decisions; the 0136
   architecture research resolves the open questions; and — critically — the
   **luminosity project, which this mirrors, has NOT implemented its launcher
   either.** Luminosity work item 0008 is fully *planned* (plan
   `2026-07-03-0008` + ADR-0010) at near-implementation fidelity but ships only
   the `version` hexagon. **You are implementing the reference, not copying it.**
   The luminosity plan is therefore the single most valuable specification
   document available.

2. **The cache MUST live under `${CLAUDE_PLUGIN_ROOT}`** (resolved constraint) —
   because the bare-path invocation contract matches `allowed-tools` globs
   against `${CLAUDE_PLUGIN_ROOT}` paths, and a user-level (XDG) cache would
   break the permission match. `${CLAUDE_PLUGIN_ROOT}` is version-scoped, so a
   new plugin version gets a fresh cache and redownloads — deemed correct
   because binaries are version-coherent with the plugin. The *exact* path/layout
   under the root is deferred to 0164 to decide.

3. **The existing shell launcher (`launch-server.sh`) is a strong behavioural
   reference for fetch→verify→cache but NOT for dispatch or exec.** It is a
   daemon launcher (backgrounds a server, PID handshake), not a git-style
   sub-binary dispatcher. Its manifest schema, platform-key derivation,
   download hardening, tri-precedence override, and verify-then-install ordering
   are the compatibility contract to mirror; its daemon lifecycle is discarded.

The single highest-risk coordination item is the **manifest schema + asset
naming contract with 0165** (distribution/signing). 0164 tests against
*fixtures* whose shape must match what 0165's production pipeline eventually
emits, and neither story has pinned the full JSON schema or the per-binary×target
asset filename template. Minisign does not exist anywhere in the repo yet.

## Detailed Findings

### 1. The 0163 scaffold — what exists today (the starting point)

The `cli/` workspace (`cli/Cargo.toml`) is a two-member Cargo workspace,
`resolver = "2"`, members `["launcher", "kernel"]`, shared
`version = "1.24.0-pre.2"`, `edition = "2021"`, `license = "MIT"`,
`publish = false`.

**Shared dependency pool** (`cli/Cargo.toml:11-21`) — the *complete* set today:
`clap = "4.6"` (`features = ["derive"]`), `vergen = "=9.0.6"` (exact pin,
`default-features = false`), `vergen-gitcl = "=1.0.8"` (`features =
["build","cargo"]`), `thiserror = "2"` (⚠️ upgraded from `1` post-0163-validation
in commit `a332c7982` — use 2.x), `tracing = "0.1"`, `tracing-subscriber =
"0.3"` (`env-filter`), `time = "0.3"` (`parsing`, dev-dep only).

**Absent from the pool** (0164 must add to `[workspace.dependencies]` then
reference via `{ workspace = true }`): `reqwest`, `tokio`, `rustls`,
`minisign-verify`, `sha2`/`hex`, `serde`/`serde_json`, any HTTP/TLS/signature/async
crate.

**The launcher crate** (`cli/launcher/Cargo.toml`): binary is
`[[bin]] name = "accelerator"`, `path = "src/main.rs"` — so the crate is
`launcher` but the binary is `accelerator` (deliberate rename to avoid
`cli/cli/`; a **recorded deviation** from ADR-0053/0054's `cli` crate name).
Tests reference it via `env!("CARGO_BIN_EXE_accelerator")`. Deps today: only
`kernel` (path), `clap`, `tracing`. No `[features]`.

**Entry point** (`cli/launcher/src/main.rs:13-28`): `main() -> ExitCode` runs
`Cli::parse()` then `run(&cli)`, which calls `kernel::logging::init()?`,
constructs `VersionReporter::new(VergenBuildMetadata)` (concrete adapter injected
at the composition root), and calls `dispatch(cli, &reporter)`. `Ok(())` →
`ExitCode::SUCCESS`; `Err(error)` → `eprintln!("{error}")` + `ExitCode::FAILURE`.
No `anyhow`, no panics — the whole app funnels through `kernel::Error` and renders
via the `thiserror`-derived `Display`.

**clap surface** (`cli/launcher/src/version/inbound/cli.rs:9-21`):
```rust
#[derive(Parser)]
#[command(name = "accelerator", disable_version_flag = true)]
pub struct Cli { #[command(subcommand)] pub command: Command }

#[derive(Subcommand)]
pub enum Command { Version }
```
There is **no `#[command(external_subcommand)]` variant** — the enum has exactly
one arm. Unknown subcommands are hard-rejected by clap (asserted in
`tests/version.rs`). `disable_version_flag = true` frees `--version` (version is
a subcommand by design — do not re-add the flag).

**The hexagonal template** (`cli/launcher/src/version/`) — the pattern every new
subcommand must mirror. `mod.rs` declares exactly three children: `core`,
`inbound`, `outbound`.
- **core** (`core.rs`): outbound port trait `BuildMetadata`; value object
  `VersionReport` (pure data); inbound port trait `ReportVersion`; application
  service `VersionReporter<M: BuildMetadata>` with `const fn new`. Unit-tested
  with a hand-written `FakeBuildMetadata` — **no mocking framework.**
- **inbound** (`inbound/cli.rs`): the clap driving adapter — holds `Cli`/`Command`,
  a pure `render(&VersionReport) -> String`, and `dispatch(cli, reporter: &impl
  ReportVersion) -> Result<(), kernel::Error>`. Doc comment states dispatch
  returns `kernel::Error` "to share one fallible contract across subcommands"
  even though the version arm can't fail.
- **outbound** (`outbound/build_metadata.rs`): `VergenBuildMetadata` unit struct
  reading `env!("CARGO_PKG_VERSION")` + `option_env!("VERGEN_*")` via an
  `or_unknown()` helper.

**kernel** (`cli/kernel/src/lib.rs`): `pub mod logging;` + the app-wide error
enum — `#[derive(Debug, thiserror::Error)] pub enum Error { LogFilter(#[from]
tracing_subscriber::filter::ParseError) }`, one variant so far. **This is the
single error taxonomy all subcommands funnel through; 0164's
network/verify/cache/exec failures must be added here (or wrapped into it).**
`logging.rs` reads env var **`ACCELERATOR_LOG`** (namespaced, not `RUST_LOG`) and
installs a stderr `tracing` subscriber via idempotent `try_init()`.

**Enforcement 0164 must satisfy:**
- **cargo-pup** (`cli/pup.ron`): an inward-import rule scoped to
  `^launcher::version::core($|::)` allowing imports only from `std/core/alloc`,
  `kernel::Error`, and `crate::version::core`. **Each new subcommand's core needs
  its own analogous rule** — it does not auto-extend. Runs on a pinned nightly
  lane (`nightly-2026-01-22`).
- **cargo-deny** (`cli/deny.toml`): targets are the four release triples +
  linux-gnu; **hard-bans `native-tls`/`openssl`/`openssl-sys`** (protects the
  musl-static build → 0164's HTTP client must be rustls). License allow-list is
  `MIT`/`Apache-2.0`/`Unicode-3.0` and *warns on unused allowances* — adding the
  HTTP/TLS/signature stack requires adding ISC/BSD/Zlib (anticipated in comments).
  `wildcards = "deny"`.
- **rustfmt** (`cli/rustfmt.toml`): `max_width = 80`, `edition = "2021"`.

**Integration-test pattern** (`cli/launcher/tests/version.rs`): black-box —
`Command::new(env!("CARGO_BIN_EXE_accelerator"))`, always `env_remove("ACCELERATOR_LOG")`
first, `Result<(), Box<dyn Error>>` throughout, string assertions on
stdout/stderr. The test `an_unknown_subcommand_exits_non_zero` asserts non-zero
exit + `"unrecognized subcommand"` — **this documents behaviour that
`external_subcommand` will change** (unknown becomes a fetch/exec passthrough)
and will need updating.

### 2. Governing decisions (ADR-0054, ADR-0046, 0136 research, spike 0158)

**ADR-0054 (git-style modular CLI of on-demand static binaries)** — the dispatch
ADR that directly governs 0164:
- Dispatch = clap 4.x derive `#[command(external_subcommand)] External(Vec<OsString>)`.
  First element = subcommand name, rest forwarded verbatim. `Vec<OsString>` (not
  `String`) preserves non-UTF-8 args.
- `version` and `config` are **built-in** (compiled into `accelerator`); external
  dispatch is *purely the growth mechanism* for on-demand subdomains. "A
  deliberate, standing distinction."
- **Unix `exec` only** — `CommandExt::exec`, process-replacing (exit codes +
  signals propagate). Windows out of scope. No spawn-and-wait shim.
- Resolution (fetch→verify→cache→exec) lives **in the Rust binary**, not bash. A
  **thin bash bootstrap** fetches `accelerator` itself on first use; thereafter
  Rust owns everything. **No launcher self-update.**
- **uv-style resolve-once-and-cache**, keyed by **name+version+checksum**,
  fetch-on-miss.
- **`reqwest` + rustls workspace-wide, `default-features = false`** — chosen over
  `ureq` for sync+async uniformity from one dependency; accepted as pulling
  `tokio` into the launcher.
- Discoverable help: clap cannot list external subcommands, so synthesise an
  "external subcommands" section from the **release manifest's `description`
  field**; delegate per-command `--help` by re-exec'ing the child with `--help`.
- Rejected: fat multicall binary (busybox), rustup-style PATH shims, one shared
  `core` crate with subcommands as inbound adapters.
- **Explicitly deferred to 0164/downstream**: the exact managed cache/bin
  directory path (NOT pinned to `${CLAUDE_PLUGIN_ROOT}` in the ADR itself); final
  clap-derive confirmation on the pinned version.

**ADR-0046 (zero-setup static-binary distribution)**:
- Four Unix targets, all static: `darwin-arm64`, `darwin-x64`, `linux-arm64`,
  `linux-x64` (Linux via **musl**).
- **Integrity layering**: sha256 verified on fetch **and re-verified before every
  exec**, plus TLS in transit, **plus minisign signing verified in-process** —
  trust rests on "signed by our key," not merely "served over TLS." minisign
  verifies in-process with **no `gh` dependency** on the user's machine. Sigstore/
  SLSA in-process verification **parked** (breaks musl-static; kept as free CI-side
  out-of-band provenance only).
- rustls throughout is a **static-linking prerequisite** (native-tls breaks
  musl-static), not merely an architecture preference.
- Version coherence required across `plugin.json`, the CLI `Cargo.toml`, and the
  release manifest.
- First use requires network access — **no offline/air-gapped fallback** in the
  model itself (but see the `ACCELERATOR_*_BIN` env override, which is the escape
  hatch — AC6).
- musl caveats: **DNS resolution behaviour** relevant to reqwest/rustls+tokio on
  musl (luminosity's answer: `hickory-dns` to bypass getaddrinfo).

**0136 architecture research** (`meta/research/codebase/2026-06-28-0136-...`) —
resolved all eight open questions on 2026-06-28:
- **Cache under `${CLAUDE_PLUGIN_ROOT}` is a RESOLVED requirement** (Open Q3):
  the bare-path invocation contract matches `allowed-tools` globs against
  `${CLAUDE_PLUGIN_ROOT}` paths; a cache outside it breaks the permission match.
  Version-scoped ⇒ new plugin version ⇒ fresh cache ⇒ redownload (correct).
  Writability established by precedent (the current launcher downloads into
  `$SKILL_ROOT/bin/`).
- Workspace root is `cli/` (resolved deviation from the ADRs); launcher crate
  renamed `cli`→`launcher`.
- `kernel` is intended to hold the **dispatch/launcher contract traits** (plus
  error taxonomy + logging) — but 0163 did NOT add them; deferred to 0164.
- **Provenance decision** (Open Q6): runtime integrity = minisign + sha256
  in-process ONLY. Drop runtime `gh attestation verify`. `RELEASING.md` still
  advertises an unimplemented `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE` runtime
  hook that should be removed (0165's job).
- **Hooks** factor in via a `--format=hook` switch on domain subcommands (Phase 6,
  not 0164), but the hot `PreToolUse` guard "should lean built-in to avoid a
  per-Bash-call sub-binary fetch" — flagging that 0164's built-in/external
  boundary may later need a built-in `vcs guard`.

**Spike 0158 §2/§3** is the authoritative design source (ported from luminosity),
consistent with the above; it explicitly leaves the cache/bin dir path ("XDG data
dir vs plugin `bin/`") as a residual open question for the distribution work.

### 3. The luminosity reference — a specification, not working code

**Critical finding**: the fetch→verify→cache→exec launcher does **not** exist as
working Rust in luminosity. Work item 0008 is fully researched + planned but
unimplemented; luminosity ships only the `version` hexagon (from its scaffold),
`kernel::Error` is still the uninhabited `enum Error {}`, and there is no `bin/`
bootstrap and no `cli/verify/` shim crate. **Accelerator is implementing the
reference.** The luminosity plan
(`../luminosity/meta/plans/2026-07-03-0008-static-binary-distribution-and-launcher.md`)
is the near-implementation-fidelity spec. Key designed shapes worth lifting:

- **Dispatch**: `Command { Version, #[command(external_subcommand)]
  External(Vec<std::ffi::OsString>) }`. The plan mandates moving `dispatch` out of
  `version/inbound/cli.rs` into a **dedicated launcher-level dispatch/launch
  module** so no launcher glue sits under `version::core` (cargo-pup would reject
  it). clap pinned 4.6.1.
- **Exec**: `std::os::unix::process::CommandExt::exec`; resolution abstracted
  behind `trait ResolveBinary { fn resolve(&self, name, args) -> Result<PathBuf,
  kernel::Error> }` — fake adapter (fixture path) in an early phase, real fetch
  adapter later. **Note: luminosity has NO per-binary exec-path env override** —
  accelerator's `ACCELERATOR_<SUB>_BIN` escape hatch (AC6) is an
  accelerator-specific addition; luminosity's only override is cache-location
  (`LUMINOSITY_CACHE_DIR`).
- **Cache root resolution** (`CacheRootResolver`): `${CLAUDE_PLUGIN_ROOT}/bin/`
  primary, **probed** (not inferred) for writable + exec-capable; XDG fallback also
  probed for exec-capability (noexec mounts); unset `CLAUDE_PLUGIN_ROOT` is a named
  error; XDG dir bounded by a retained-versions cap (oldest-by-mtime eviction under
  the per-key lock). (Accelerator's resolved constraint is stricter — cache under
  the plugin root for `allowed-tools`; consider whether the XDG fallback even
  applies here.)
- **Atomic cache write** (`CacheStore`): fetch into a unique temp file **inside the
  resolved cache dir** (intra-fs `rename(2)`, avoids `EXDEV`), verify there, then
  atomic-rename into the name+version+checksum name — only fully-verified bytes
  ever appear at the final path. Caps download size + checks free space.
  Concurrency via **per-cache-key `flock`/`fcntl` advisory lock** scoped to
  fetch/verify/rename, lock fd `FD_CLOEXEC`/closed before exec. Cache-hit
  verification failure → **evict + re-fetch once + re-verify** (self-healing).
- **Verification order**: verify `manifest.minisig` FIRST (before trusting any
  field) + a **version-equality anti-rollback check**; then per-binary sha256 +
  minisign. **minisign is the security boundary; sha256 is only a corruption
  check.** Re-verify the signature **before every exec, including cache hits**
  (the cache dir is user-writable, could be poisoned). A valid sha256 with a
  non-release-key signature must be refused.
- **minisign** via `minisign-verify` **0.2.5** (jedisct1, zero-dep) — the one
  firmly-pinned new dep. Pubkey embedding via `include_str!` of the committed
  release public key(s), supporting a **verify-any-of small set of trusted keys**
  for rotation overlap.
- **Crypto provider — `ring`, explicit**: reqwest `-no-provider` feature + direct
  `rustls` dep `features=["ring"]`, install via `CryptoProvider::install_default(...)`
  as a fallible call mapped to `kernel::Error` (never `.unwrap()` — restriction
  lints are deny-level).
- **reqwest features**: `default-features = false` + `["rustls-tls-webpki-roots-no-provider",
  "blocking", "hickory-dns"]`. `default-features = false` is mandatory or the
  default native-tls feature pulls `openssl-sys` and trips deny.toml.
  **No direct `tokio` dep** — blocking reqwest manages its own runtime. (This
  nuances ADR-0054's "pulls tokio in" — it's transitive, not a direct dep.)
  Fetcher: connect timeout + read/idle stall timeout + aggregate deadline (not one
  total timeout); bounded retry-with-backoff on transient/5xx; pins `https`;
  redirects only to a `*.githubusercontent.com` suffix allowlist + the release
  origin.
- **Error taxonomy**: a **launcher-local resolution error** (variants:
  fetch/network, checksum mismatch, signature mismatch, asset-not-found,
  release-unavailable, IO/cache, exec) that **maps into a small `kernel::Error` at
  the boundary** — keeps `version` from compiling against fetch/signature variants.
- **MSRV**: `rust-version = "1.90.0"` on the launcher + bump workspace to
  `resolver = "3"` for MSRV-aware selection. (Accelerator is currently
  `resolver = "2"`.)
- **Help synthesis (lazy path)**: `Cli::parse()` intercepts `--help` too early and
  `after_help` takes only a compile-time string — so `try_parse`, and *only* on
  `ErrorKind::DisplayHelp` read+verify the manifest and re-render via
  `Cli::command().after_help(section)`. Keeps offline built-ins (`version`)
  independent of manifest availability. **Strip control/escape chars** from
  manifest-derived strings before printing.
- **Bootstrap**: entry point `bin/luminosity` (extensionless, named for the
  command). bash-3.2 constructs only (`set -uo pipefail`, `BASH_SOURCE`, `case`;
  no assoc arrays / `${x,,}` / `mapfile`). Because it's extensionless,
  `lint-bashisms.sh`/shfmt/ShellCheck globs (which target `*.sh`) **must be
  extended to include it**. **Root-of-trust subtlety**: the bootstrap cannot trust
  the launcher's own re-verify (a tampered launcher can't verify itself), so it
  verifies the launcher's minisig via a **vendored per-triple `cli/verify/`
  minisign shim** against the plugin-committed key, fail-closed. exec forwards
  `"$@"` as an argv list, not a shell string.
- **Test strategy**: an **arg-driven fixture `[[bin]]` inside the launcher crate**
  (located via `env!("CARGO_BIN_EXE_<name>")` — cargo only sets that var for bins
  in the package under test), with behaviours `exit-42`, `block-on-sigterm`,
  `print-help-sentinel`. Hermetic fetch/verify/cache against a local mock HTTP
  server; fixture manifest + a test keypair **plus a second non-release key**.
  Required assertions: exactly-one-fetch across two invocations (cache reuse);
  mutated-on-disk cached binary refused with no exec; cache-hit-fail →
  evict+re-fetch; checksum mismatch refused; non-release-key sig refused;
  tampered-manifest sig refused; wrong-version manifest refused (anti-rollback);
  5xx-then-200 recovers / persistent-5xx gives up; cross-host redirect allow/deny;
  offline cache-hit still resolves; SIGTERM readiness-handshake → caller `$?`=143;
  non-UTF-8 arg survives verbatim. **cargo-deny scans dev-deps too** — mock-server/
  signing dev-deps must be rustls-only or `exclude-dev` set deliberately.

### 4. The existing shell launcher — behavioural contract to mirror

`launch-server.sh` is a **daemon** launcher (backgrounds a server, PID/start-time
handshake), not a git-style dispatcher, so it "genuinely diverges" for the
dispatch/exec half. But its **fetch→verify→cache** core is the compatibility
contract:

- **Platform key** = `"${OS}-${ARCH}"` from `uname -s` (lowercased; darwin/linux
  only) and `uname -m` (`arm64|aarch64→arm64`, `x86_64→x64`)
  (`launch-server.sh:80-93`). Must be replicated exactly.
- **Tri-precedence resolution** (`:104-168`), MUST PRESERVE: (1) env
  `ACCELERATOR_VISUALISER_BIN` — used verbatim, no checksum, no exec check; (2)
  config `visualiser.binary` via `config-read-value.sh` — relative→PROJECT_ROOT,
  must be `-x`; (3) download/cache. Both overrides bypass the manifest entirely.
  (0164's AC6 generalises this to `ACCELERATOR_<SUB>_BIN`.)
- **Manifest verification** (`:124-146`): `jq` lookup of
  `binaries["<os>-<arch>"]`; strip `sha256:` prefix; **all-zeros digest = "no
  binary for this version" sentinel → hard fail**; version-drift guard (manifest
  `.version` vs `plugin.json` version; absent manifest version tolerated).
- **Verify-then-install** (`:141-166`): reject a symlink cache (force re-download);
  compare cached actual-sha to expected; on miss, download to a `mktemp` sibling in
  `bin/`, re-hash the freshly-downloaded bytes, compare, then `install -m 0755` to
  the cache path. Verification always runs against fresh bytes.
- **Download hardening** (`launcher-helpers.sh:18-32`): curl
  `-fsSL --proto '=https' --tlsv1.2 --retry 3 --max-redirs 3 --max-filesize
  33554432` (32 MiB cap). Insecure escape hatch
  `ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD` drops only proto+tls. wget fallback
  does NOT enforce https-only/filesize (a gap the Rust rewrite closes).
- **Portable sha256** (`scripts/hash-common.sh`): `command -v sha256sum` else
  `shasum -a 256`, `awk '{print $1}'` to strip the filename; bash-3.2-safe. This is
  the idiom the bootstrap needs.
- **Discard** (visualiser-specific): PID+start_time reuse short-circuit, init
  sentinel, flock/mkdir lock, config.json handshake, `nohup … &` daemon launch +
  `server-info.json`/`server.pid` polling + `127.0.0.1` URL validation, and all of
  `launcher-helpers.sh` beyond `sha256_of`/`download_to`. A generic launcher
  `exec`s the binary directly rather than daemonising.

### 5. The distribution pipeline & manifest contract (Python side, 0165 territory)

The current `checksums.json` (`skills/visualisation/visualise/bin/checksums.json`,
written by `tasks/build.py:118-128`):
```json
{ "version": "1.24.0-pre.7",
  "note":  "...sha256:0…0 is a deliberate sentinel; fail any build that sees it.",
  "binaries": { "darwin-arm64": "sha256:<64-hex>", "darwin-x64": "...",
                "linux-arm64": "...", "linux-x64": "..." } }
```
- Value format `"sha256:<64-hex>"`; the `sha256:` prefix is added when writing the
  manifest (`build.py:127`), NOT by the hasher — `tasks/shared/hashing.py`
  returns **bare** lowercase hex. Verification strips the prefix before comparing.
- Platform keys are the four short strings above (not the Rust triples). Targets:
  `tasks/shared/targets.py` maps `aarch64-apple-darwin→darwin-arm64`,
  `x86_64-apple-darwin→darwin-x64`, `aarch64-unknown-linux-musl→linux-arm64`,
  `x86_64-unknown-linux-musl→linux-x64`.
- **Asset filename** = `accelerator-visualiser-<platform>` (no extension);
  `.debug.tar.gz` sibling is debug-only. **Download URL** (from the shell launcher)
  = `{RELEASES_URL_BASE}/v{version}/accelerator-visualiser-<platform>`, base
  overridable via `ACCELERATOR_VISUALISER_RELEASES_URL`, default
  `https://github.com/atomicinnovation/accelerator/releases/download`. Tag segment
  has a leading `v`.
- **Publish flow** (`tasks/github.py:136-178`): draft → upload both assets per
  platform → re-download + re-sha256-verify → un-draft; `AssetVerificationError`
  preserves draft+tag for triage, generic exceptions delete the release+tag. SLSA
  attestation runs as a CI step between prepare and finalise
  (`.github/workflows/main.yml:352-355,437-455`).
- **Version coherence** (`build.py:131-151`): single version across `plugin.json`,
  server `Cargo.toml`, `checksums.json.version`, CLI workspace version, and any
  member pinning its own version.

**Gaps that are 0165's to close but 0164's fixtures must anticipate**:
- **No minisign anywhere yet** — confirmed no `.sig`/minisign/ed25519 refs in
  `tasks/`, `.github/`, `scripts/`, or the launcher. The Rust launcher should not
  expect a signature asset to exist against the *current* pipeline; it will
  test against fixture-signed binaries.
- **No formal manifest with a `description` field** — only `checksums.json`
  (version + binaries). Help synthesis needs either a new manifest schema
  (coordinate with the Python writer) or another description source. Luminosity's
  plan defines a **new `manifest.json`** distinct from `checksums.json`, name-keyed
  with a per-platform inner map carrying `sha256` + `signature` + a top-level
  per-binary `description`, plus a `schema_version` integer (deserialise
  leniently, fail-closed on unrecognised higher major).

### 6. Sibling-story contracts and the built-in/external boundary

- **0165** (distribution/signing) produces the artefacts 0164 consumes: per-binary
  sha256, detached `.minisig` files, the embedded pubkey (whose storage/rotation
  policy is 0165's open question — CI secret vs offline key), the release manifest
  incl. `description`, and the published launcher asset the bootstrap fetches.
  **Undecided between the stories**: the full manifest JSON schema, and the asset
  naming template per binary×target. 0165 is developed in parallel; 0164 lands
  against fixtures.
- **0167** (config): `config` is **built-in, compiled into the launcher, no
  sub-binary fetch** (confirmed). 0167 is `blocked_by` 0164 (it consumes 0164's
  bootstrap). The `allowed-tools` glob shape (single
  `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator *)` vs per-subcommand) is 0167's open
  question — doesn't gate 0164's mechanism but defines the invocation shape.
- **0169** (hooks/VCS): one universal bash wrapper (0164's bootstrap reused);
  hot `PreToolUse` guard "should lean built-in" / warm-cache — the sharpest
  external constraint on 0164's dispatch design (a per-Bash-call fetch is
  unacceptable). `--format=hook` switch produces the hook I/O envelope.
- **The built-in/external split point has no 0164 AC pinning it** (review finding
  🔵) — `version`/`config` built-in with no fetch, all else external. The three
  stories "must agree"; a divergent 0167/0169 decision forces rework.

### 7. The 0164 work-item review — resolved and still-open items

Review verdict was REVISE→now APPROVE-ready; ACs grew 5→10. All Pass-1/Pass-2
majors are resolved in the current story. **Remaining open minors an implementer
should note** (not yet ACs):
- 🔵 **Re-verify-before-every-exec has no dedicated criterion** — the requirement
  exists (story lines 61-62) but no AC gates "tamper a cached binary between
  invocations → reject on next exec." Luminosity treats this as a required test.
- 🔵 Per-target verifier environment implicit (AC9 spans four targets but doesn't
  say `otool -L` for darwin vs `ldd` for linux-musl, in CI).
- 🔵 The built-in/external split point is unpinned by any AC (see §6).
- 🔵 AC9's four-target release-build criterion drifts toward 0165's territory;
  reviewer suggests reframing as a host-target launcher-crate build property.
- 🔵 Cross-story contract agreement (manifest/checksum/minisign schemas) not
  tracked as an actionable coupling; fixture-vs-production pubkey handoff only
  partially traced — **the most load-bearing open item**.

## Code References

- `cli/Cargo.toml:11-37` — workspace dependency pool + lint config (what's present/absent)
- `cli/launcher/Cargo.toml:12-26` — `accelerator` bin target + launcher deps
- `cli/launcher/src/main.rs:13-28` — composition root; `run`/`main`, `kernel::Error` → `ExitCode`
- `cli/launcher/src/version/inbound/cli.rs:9-53` — clap `Cli`/`Command` (single `Version` arm), `render`, `dispatch`
- `cli/launcher/src/version/core.rs:1-78` — the hexagon port+fake pattern to copy
- `cli/launcher/src/version/outbound/build_metadata.rs:11-27` — outbound adapter idiom
- `cli/launcher/tests/version.rs:18-175` — black-box test idiom; `an_unknown_subcommand_exits_non_zero` (will change)
- `cli/kernel/src/lib.rs:6-10` — the single `kernel::Error` taxonomy to extend
- `cli/kernel/src/logging.rs:26-36` — `ACCELERATOR_LOG` subscriber init
- `cli/pup.ron:12` — per-core inward-import rule (needs a new one per hexagon)
- `cli/deny.toml:35-53` — license allow-list + native-tls/openssl ban
- `skills/visualisation/visualise/scripts/launch-server.sh:80-168` — platform key, tri-precedence, verify, download, install
- `skills/visualisation/visualise/scripts/launcher-helpers.sh:18-32` — curl hardening flags
- `skills/visualisation/visualise/bin/checksums.json:1-10` — manifest schema + sentinel note
- `scripts/hash-common.sh:12-27` — portable bash-3.2 sha256 idiom
- `tasks/shared/targets.py:1-6` — the four triple→platform-key pairs
- `tasks/build.py:118-151` — `update_checksums_json` + version coherence
- `tasks/github.py:105-178` — verify + draft/upload/re-verify/un-draft publish flow
- `../luminosity/meta/plans/2026-07-03-0008-static-binary-distribution-and-launcher.md` — the near-impl-fidelity spec
- `../luminosity/meta/decisions/ADR-0010-git-style-modular-cli-of-on-demand-static-binaries.md` — luminosity dispatch ADR

## Architecture Insights

- **Hexagonal, two axes**: a binary axis (one crate per shippable sub-binary) and a
  layering axis (hexagonal layers as modules within each crate, split into crates
  only under pressure). The launcher is its own composition root; `cli` depends on
  `kernel` (+ `config` later), never on a subdomain.
- **`kernel` is the seam**: the dispatch/launcher *contract traits* are meant to
  live in `kernel` (ADR-0053/0054) but were deferred by 0163 — 0164 introduces
  them. A launcher-local rich resolution error mapped down to a small
  `kernel::Error` at the boundary keeps light crates (e.g. `version`) from
  compiling against network/signature variants.
- **Dispatch glue must NOT live under `version::core`** (cargo-pup) — put it in a
  dedicated launcher-level module (`dispatch`/`launch`), mirroring luminosity.
- **minisign is the trust boundary; sha256 is corruption-detection.** Verify
  signature before trusting the manifest and before every exec (including cache
  hits) because the cache is user-writable.
- **Atomic-rename-or-nothing** cache writes (temp file *inside* the cache dir →
  verify → intra-fs rename) satisfy AC3's "no partial/temp entry survives a
  verification failure" cleanly.
- **The one production TLS stack in the whole system is the launcher's rustls** —
  the visualiser server has no production TLS. `default-features = false` on
  reqwest is non-negotiable (deny.toml bans openssl).
- **`${CLAUDE_PLUGIN_ROOT}` cache is what keeps `allowed-tools` matching intact**
  through the later invocation-contract rewrite (Phase 4) — a non-negotiable
  constraint even though the exact sub-path is 0164's to choose.

## Historical Context

- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md` — dispatch model (built-in vs external, exec, help synthesis)
- `meta/decisions/ADR-0046-zero-setup-static-binary-distribution.md` — distribution/integrity model (four targets, minisign+sha256, rustls)
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md` — the hexagonal layout the `version/` tree embodies
- `meta/decisions/ADR-0049-bash-3.2-compatibility-floor.md` — constrains the bootstrap
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md` — resolved the eight open questions; direct architectural parent
- `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md` — the spike (§2 dispatch, §3 launcher)
- `meta/work/0163-scaffold-cli-workspace-version-subcommand.md` + `meta/plans/2026-07-02-0163-...` + `meta/validations/2026-07-02-0163-...` — the scaffold 0164 builds on
- `meta/reviews/work/0164-launcher-and-git-style-dispatch-review-1.md` — scoping/AC feedback

## Related Research

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md` (parent scope/architecture)
- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md` (migration-surface inventory)
- `meta/research/codebase/2026-07-02-0163-cli-workspace-version-subcommand-scaffold.md` (scaffold research)

## Open Questions

1. **Cache path/layout under `${CLAUDE_PLUGIN_ROOT}`** — the exact sub-directory
   and per-key filename scheme (name+version+checksum) is 0164's to decide. Does
   the luminosity XDG fallback apply here at all, given the `allowed-tools`
   constraint forces the plugin-root location? (Likely: plugin-root only, no XDG
   fallback — confirm.)
2. **Manifest schema + asset naming with 0165** — must the fixture manifest match
   a new `manifest.json` (luminosity-style, name-keyed with per-platform sha256 +
   signature + description) or an extended `checksums.json`? What is the
   per-binary×target asset filename template? Agree before finalising fixtures.
3. **reqwest/rustls/sha2 versions** — luminosity leaves these as `<pin>`;
   accelerator must choose. `minisign-verify = 0.2.5` and `clap 4.6.1` are the only
   firm pins. Does accelerator adopt luminosity's `resolver = "3"` + `rust-version
   = 1.90.0` MSRV bump (currently `resolver = "2"`)?
4. **Fixture-signed binaries & test keypair** — 0164 embeds a *fixture* pubkey for
   its tests and must later swap to 0165's production pubkey (verify-any-of for
   rotation). How is that handoff tracked?
5. **Built-in/external boundary AC** — should 0164 add an AC pinning the split
   point (version/config built-in, all else external) to protect against divergent
   0167/0169 decisions?
6. **Re-verify-before-exec AC** — add a dedicated criterion (tamper cached binary →
   rejected on next exec) per the review's open minor and luminosity's required test?
7. **Bootstrap root-of-trust** — does accelerator adopt luminosity's vendored
   `cli/verify/` minisign shim crate for the bootstrap to verify the launcher
   itself (since a tampered launcher can't verify itself, and in-bash Ed25519 isn't
   feasible)? This is a whole extra crate the story doesn't currently mention.
8. **lint globs for an extensionless bootstrap** — if the bootstrap is
   extensionless (`bin/accelerator`), `lint-bashisms.sh`/shfmt/ShellCheck `*.sh`
   globs must be extended (AC10 references `lint-bashisms.sh`).
