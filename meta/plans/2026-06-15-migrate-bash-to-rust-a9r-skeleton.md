---
type: plan
id: "2026-06-15-migrate-bash-to-rust-a9r-skeleton"
title: "a9r Migration: Walking Skeleton + config-read Family Implementation Plan"
date: "2026-06-15T15:16:31+00:00"
author: "Phil Helm"
producer: create-plan
status: draft
tags: [bash, rust, a9r, cli, migration, visualiser, build-system, workspace]
revision: "21b96adac354e0285d1c76420a92681cb1938697"
repository: "accelerator"
last_updated: "2026-06-15T15:16:31+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# a9r Migration: Walking Skeleton + config-read Family Implementation Plan

## Overview

Stand up a Cargo workspace producing a single `a9r` binary, fold the existing
visualiser into it as the `a9r visualise` subcommand, then migrate the
high-fan-in `config-read-*` bash scripts to Rust subcommands — each proven
byte-for-byte equivalent against the *existing* bash test suites before its
`.sh` is replaced with a thin a9r-or-bash shim. This is the walking skeleton
(Decision 4, step 1 of the research) plus the first bulk slice (step 2),
realised end-to-end through every integration point: workspace, `a9r-core` lib,
clap subcommands, test-parity harness, provisioning hook, distribution, and CI.

## Current State Analysis

- **No Cargo workspace exists.** `accelerator-visualiser` is a single package at
  [`skills/visualisation/visualise/server/Cargo.toml`](../../skills/visualisation/visualise/server/Cargo.toml#L1)
  with one `[lib]` and one `[[bin]]` named `accelerator-visualiser`
  ([`Cargo.toml:17-21`](../../skills/visualisation/visualise/server/Cargo.toml#L17-L21)).
  `[profile.release]` and `[lints.clippy]` are package-level
  ([`Cargo.toml:65-95`](../../skills/visualisation/visualise/server/Cargo.toml#L65-L95))
  and become workspace-root-only after a split.
- **The CLI is flat.** [`server/src/main.rs:7-13`](../../skills/visualisation/visualise/server/src/main.rs#L7-L13)
  is a single `Cli { --config PATH }` parser; the boot block
  ([`main.rs:18-50`](../../skills/visualisation/visualise/server/src/main.rs#L18-L50))
  calls `Config::from_path` then `server::run(cfg, &info_path)`
  ([`server.rs:297`](../../skills/visualisation/visualise/server/src/server.rs#L297)).
  The crate is already a thin binary over a fat lib
  ([`lib.rs:9-31`](../../skills/visualisation/visualise/server/src/lib.rs#L9-L31)),
  so subcommands become a clap `#[derive(Subcommand)]` enum over lib calls.
- **`config-read-path.sh` → `config-read-value.sh` is the simplest hot-path
  vertical.** `config-read-path.sh` sources only `config-defaults.sh` +
  `vcs-common.sh`, applies a defaults table, then `exec`s `config-read-value.sh`
  ([`config-read-path.sh:75`](../../scripts/config-read-path.sh#L75)).
  `config-read-value.sh` sources `config-common.sh` (which sources
  `vcs-common.sh` + `config-defaults.sh`) and does the actual YAML lookup.
  `atomic-common.sh`/`jsonl-common.sh` are **not** in this dependency graph.
- **The test seam is uniform and swappable.** Every call site in
  [`scripts/test-config.sh`](../../scripts/test-config.sh) is `bash "$VAR" args`
  via `READ_PATH` ([`test-config.sh:2939`](../../scripts/test-config.sh#L2939),
  25 sites) and `READ_VALUE`
  ([`test-config.sh:12`](../../scripts/test-config.sh#L12), 33 sites). Helpers in
  [`scripts/test-helpers.sh`](../../scripts/test-helpers.sh) assert stdout
  (`assert_eq`/`assert_contains`), stderr in isolation (`assert_stderr_empty`,
  L242), and exit code (`assert_exit_code`, L197) independently — exactly the CLI
  contract a port must preserve.
- **Integration points are numerous but templated.** Adding a Rust component
  touches ~7 `mise.toml` aggregates, `tasks/shared/paths.py` constants, the
  `tasks/{format,lint,test}` invoke tasks, and needs a CI job mirroring the
  `check-visualiser-server` `RUSTUP_HOME`/`cache_key_prefix` workaround
  ([`.github/workflows/main.yml:143-160`](../../.github/workflows/main.yml#L143-L160)).
- **Distribution already exists and already ships the binary we want.**
  `tasks/build.py` cross-compiles 4 targets and copies `bin/accelerator-visualiser-<platform>`
  ([`build.py:153-167`](../../tasks/build.py#L153-L167)); `launch-server.sh`
  resolves env → config → download+SHA-verify+cache
  ([`launch-server.sh:104-168`](../../skills/visualisation/visualise/scripts/launch-server.sh#L104-L168))
  and invokes `"$BIN" --config "$CFG"`
  ([`launch-server.sh:200`](../../skills/visualisation/visualise/scripts/launch-server.sh#L200)).
  Once the binary is renamed `a9r`, this *same* pipeline provisions `a9r`.

### Key Discoveries:

- **Byte-for-byte traps in config-read** (must be reproduced exactly in Rust):
  - `config-read-path.sh` treats an explicitly-empty `$2` the same as omitted —
    the guard is `[ -n "${2:-}" ]`
    ([`config-read-path.sh:31`](../../scripts/config-read-path.sh#L31)) — so an
    empty default still falls through to the defaults table.
  - "Not found" is signalled by `echo "$DEFAULT"` (often an empty line) with
    **exit 0** ([`config-read-value.sh:117-130`](../../scripts/config-read-value.sh#L117-L130));
    there is no distinct not-found exit code.
  - Last config file wins across files (local overrides team — the loop does not
    `break`), but **within** a file the **first** matching subkey in a section
    wins (awk `exit`, [`config-read-value.sh:86-88`](../../scripts/config-read-value.sh#L86-L88))
    — two orthogonal precedence axes.
  - Frontmatter has **three** states: absent and empty-but-closed (`---\n---`,
    [`config-read-value.sh:54`](../../scripts/config-read-value.sh#L54)) are both
    silent not-found; only **unclosed** frontmatter emits a stderr warning, and the
    warning gate's `grep -q '^---'` ([line 49](../../scripts/config-read-value.sh#L49))
    is unanchored (looser than the parser's strict `^---\s*$`).
  - An unknown `config-read-path` key with no default still **looks the key up in
    config** with an empty default (after a stderr warning), so a user-set
    `paths.<unknown>` is honoured, not forced empty
    ([`config-read-path.sh:75`](../../scripts/config-read-path.sh#L75)); the
    legacy-override warning is gated on a non-empty *resolved* value, not a bare
    grep match ([`config-read-path.sh:59-73`](../../scripts/config-read-path.sh#L59-L73)).
  - One layer of matched surrounding `"`/`'` is stripped from values
    ([`config-read-value.sh:79-84`](../../scripts/config-read-value.sh#L79-L84)).
  - Section/top-level matching is *string-prefix* (not regex) to avoid metachar
    injection ([`config-read-value.sh:42-43`](../../scripts/config-read-value.sh#L42-L43)).
  - The legacy-layout guard `config_assert_no_legacy_layout` can `exit 1` before
    any lookup ([`config-common.sh:54-66`](../../scripts/config-common.sh#L54-L66)).
  - Project root = nearest ancestor with `.git`/`.jj`, else `$PWD`, **no
    realpath normalisation** ([`config-common.sh:16-18`](../../scripts/config-common.sh#L16-L18),
    [`vcs-common.sh:8-18`](../../scripts/vcs-common.sh#L8-L18)).
  - The 17-row defaults table lives in
    [`config-defaults.sh:26-64`](../../scripts/config-defaults.sh#L26-L64).
- **`config-read-path.sh` migration warnings are stderr-only and side-effect-free**
  for stdout ([`config-read-path.sh:44-73`](../../scripts/config-read-path.sh#L44-L73)) —
  the legacy-override probe even shells out to `config-read-value.sh` but only to
  decide whether to print a stderr warning.
- **Tests that cannot cross the boundary** must be excluded under `A9R_BIN`:
  the sourced-function tests (`config_extract_frontmatter`/`config_extract_body`,
  [`test-config.sh:24,55`](../../scripts/test-config.sh#L24)) and the SKILL.md
  grep-assertions ([`test-config.sh:4398-4452`](../../scripts/test-config.sh#L4398-L4452)).
- **`../frontend/dist` is triplicated** (`build.rs:5`,
  [`assets.rs:9`](../../skills/visualisation/visualise/server/src/assets.rs#L9),
  [`assets.rs:71`](../../skills/visualisation/visualise/server/src/assets.rs#L71))
  and is relative to the server crate manifest — the server crate dir must **not
  move** during the workspace split or all three literals break.
- **`types:check` has no `server` entry** — Rust type-checking folds into clippy,
  so `a9r` needs no `types:*` task.

## Desired End State

- A Cargo workspace at `skills/visualisation/visualise/` with members
  `a9r-core` (pure logic lib), `visualiser` (the former server, now lib-only),
  and `a9r` (the single binary).
- **`a9r-core` is the single owner of shared parsing logic** (frontmatter, config
  resolution) and has **zero dependency on the `visualiser` lib** — the dependency
  arrows point `a9r → {a9r-core, visualiser}` and, where the visualiser needs
  frontmatter/config, `visualiser → a9r-core`, never the reverse. This keeps the
  *logic* boundary clean even though the single binary couples the hot-path config
  commands to the heavy axum/SPA closure (an accepted Decision-1 consequence), and
  keeps the door open to a later two-binary split. The visualiser lib's existing
  `frontmatter`/`config` modules are either reduced to consumers of `a9r-core` or,
  if its richer parser is intentionally kept separate, that separation is
  documented — there must not be two undocumented frontmatter implementations in
  one binary.
- `a9r` exposes `config-read-value`, `config-read-path`, and `visualise`
  subcommands. `a9r visualise --config <path>` is what `launch-server.sh`
  launches; a transitional alias keeps the bare `--config` form working.
- All `config-read-*` scripts (and `artifact-derive-metadata.sh`) are thin shims:
  use `a9r` when resolvable, else run the original bash inline. Both paths pass
  the *same* existing suites, run twice in CI (`A9R_BIN` set / unset).
- A `SessionStart` hook eagerly downloads + SHA-verifies `bin/a9r-<platform>`;
  failure degrades silently to the bash fallback.
- `mise run check` and `mise run` (default) exit 0 end-to-end at every phase
  boundary.

**Verification of end state:** `mise run` is green; `bash scripts/test-config.sh`
is green with and without `A9R_BIN` set to the built binary; `a9r visualise`
boots the visualiser; a fresh session with no cached binary still loads every
skill (fallback); deleting/forcing the cache then starting a session downloads
and verifies `a9r`.

## What We're NOT Doing

- **Not flipping config-read to `a9r`-only / deleting the bash fallback.** The
  shim keeps the bash implementation for the whole of this plan (Decision 3's
  "once proven across enough sessions" is a later step).
- **Not migrating** the Jira cluster, `work-item-*`, `decisions/adr-*`, `github`
  scripts, lifecycle/guard hooks (`vcs-guard.sh`, `config-detect.sh`,
  `vcs-detect.sh`, `migrate-discoverability.sh`), or dev tooling
  (`lint-bashisms.sh`). Out of scope per the research scope clarification.
- **Not removing** `config-common.sh`/`config-defaults.sh` — they remain for the
  bash fallback path; logic is *duplicated* into `a9r-core`, not deleted.
- **Not writing an ADR or work-item epic** here (separate artifacts).
- **Not backfilling coverage** for scripts outside this slice (JIT per Decision 6).
  `config-read` is well-covered by `test-config.sh`, but "well-covered" is a
  measured claim, not an assumption: before Phase 3 lands, **audit `test-config.sh`
  against the Key Discoveries trap list and backfill bash-green assertions for any
  trap not already pinned** (especially empty-vs-omitted default, one-layer
  quote-strip, within-file first-match, last-file-wins-from-second-file-only, the
  three frontmatter states, and unknown-key-still-reads-config). The parity gate
  only catches divergence where an assertion exists, so an untested bash behaviour
  would otherwise become whatever `a9r` does, silently.

## Implementation Approach

Prove the cheap, well-tested vertical first, then do the risky rename. Phases 1–5
build and prove the config-read port behind a bash fallback without touching the
visualiser's user-facing distribution; Phase 6 performs the binary rename / fold;
Phase 7 repeats the proven pattern across the rest of the family. TDD is applied
where it bites: Rust `#[test]`s precede `a9r-core` logic (Phase 2), and the
cross-language parity gate (Phase 3) is the contract every ported command must
satisfy before its shim lands.

**Merge gate and ordering.** Each phase ends green on the **full `mise run`
default** (or, for a faster loop, the specific test aggregate that *runs* the
relevant suite — e.g. `test:integration:config`, `test:integration:a9r-acquisition`),
**not** `mise run check`. `mise run check` is format + lint only — it does **not**
run tests, so it does not exercise the parity gate or the acquisition tests, which
are the actual correctness guarantees. CI must block merges on the twice-run parity
suite, not just on format/lint.

The phases are **sequentially mergeable, forward-only**, not independently
mergeable in arbitrary order: Phase 4's shims behave correctly only because Phase 2
built the binary and Phase 3 proved byte-for-byte parity on the *same* artifact
Phase 5 ships; the shim's fallback triggers on a missing *or* unverified binary
(per Phase 4 §1), so a present-but-buggy binary must never reach the hot path. The
invariant: **a shim must never route to a binary whose SHA is not in the verified
checksums manifest**, and Phase 4 must not merge until Phase 3's gate is enforced
on the artifact Phase 5 provisions.

---

## Phase 1: Cargo workspace (pure restructure, no rename)

### Overview

Convert the single `accelerator-visualiser` package into a one-member workspace
with no behaviour change. This isolates the structural risk (profile/lints
hoisting, version inheritance, lockfile relocation) from all later logic changes.

### Changes Required:

#### 1. Workspace root manifest

**File**: `skills/visualisation/visualise/Cargo.toml` (new)
**Changes**: New `[workspace]` with `members = ["server"]`, `resolver = "2"`.
Hoist `[profile.release]` (lto/codegen-units/strip/opt-level) and
`[lints.clippy]` from the server manifest to `[workspace.lints.clippy]`. Add
`[workspace.package]` with `version`, `edition = "2021"`, `rust-version = "1.85"`.

```toml
[workspace]
resolver = "2"
members = ["server"]

[workspace.package]
version = "1.22.0-pre.21"
edition = "2021"
rust-version = "1.85"

[profile.release]
lto = "thin"
codegen-units = 1
strip = true
opt-level = 3

[workspace.lints.clippy]
# moved verbatim from server/Cargo.toml:71-95
```

#### 2. Server manifest adopts workspace inheritance

**File**: `skills/visualisation/visualise/server/Cargo.toml`
**Changes**: Replace `version`/`edition`/`rust-version` literals with
`.workspace = true`; delete the moved `[profile.release]` and `[lints.clippy]`
tables; add `[lints] workspace = true`. Crate name and `[[bin]]` unchanged.

#### 3. Lockfile relocation

**File**: move `server/Cargo.lock` → `skills/visualisation/visualise/Cargo.lock`
**Changes**: Cargo regenerates at the workspace root on first build; delete the
stale package-level lock.

#### 4. Task manifest paths

**File**: `tasks/shared/paths.py`
**Changes**: `CARGO_TOML` stays pointing at `server/Cargo.toml` (member manifest
is still valid for `--manifest-path`). No change required for `cargo fmt
--manifest-path <member>` / clippy / test; confirm version reads (build.py) now
target the workspace `version` (see below).

#### 5. Version-coherence source

**File**: `tasks/build.py`
**Changes**: `validate_version_coherence` reads the Cargo version
([`build.py:44-57,87-103`](../../tasks/build.py#L87-L103)); point it at the
workspace `[workspace.package] version` so the single inherited version is the
source of truth. `tasks/version.py` writer updated to the same key.

Because this changes *which* key is authoritative for the runtime version-drift
guard (`launch-server.sh:134-140`) and the coherence check, verify the **fail-closed**
path, not just the happy path: add a Phase 1 check that `server/Cargo.toml` no
longer carries a literal `version` (it inherits `.workspace = true`) and that
`validate_version_coherence` **fails loudly** if the workspace key is missing — a
silent mismatch here would defeat the guard that prevents shipping a binary
mismatched to its checksums manifest.

### Success Criteria:

#### Automated Verification:

- [x] Workspace builds dev: `mise run build:server:dev`
- [x] Release build still embeds frontend: `mise run build:server:release`
- [x] Rust lint clean (both passes): `mise run lint:server:check`
- [x] Rust format clean: `mise run format:server:check`
- [x] Server unit tests pass: `mise run test:unit:visualiser` (424 passed)
- [x] Version coherence holds: `uv run pytest tests/unit/tasks -k coherence -v`
- [x] `server/Cargo.toml` carries no literal `version` (inherits `.workspace = true`),
      and `validate_version_coherence` fails loudly if the workspace key is missing.
- [x] Full CI green: `mise run check` (format + lint, all four components) + the
      per-component test criteria above. (Bare `mise run` default not separately
      re-run; Phase 1 touches no frontend/shell/template suites.)

#### Manual Verification:

- [x] `git diff` shows no source-logic changes — only manifest/lockfile moves.
- [x] Built binary name is still `accelerator-visualiser` (no rename yet).

---

## Phase 2: `a9r-core` library + the `a9r` binary

### Overview

Add the `a9r-core` pure-logic lib (config resolution) test-first, and the `a9r`
binary crate exposing `config-read-value`, `config-read-path`, and `visualise`
subcommands. The old `accelerator-visualiser` bin remains; no skill calls `a9r`
yet. Wire `a9r` into format/lint/test/CI as a new component.

### Changes Required:

#### 1. `a9r-core` lib crate (TDD)

**File**: `skills/visualisation/visualise/a9r-core/{Cargo.toml,src/lib.rs}` (new)
**Changes**: Workspace member with **no dependency on the `visualiser` lib** (see
Desired End State). To avoid a `config.rs` god-module as Phase 7 absorbs
`config-common.sh` (template resolution, array parsing, display-path shortening),
state the module split up front: `repo` (root discovery), `files` (config-file
resolution/precedence), `frontmatter` (extraction + warning gate), `lookup` (value
resolution), `defaults` (`PATH_DEFAULTS`), and later `template`. Each module
mirrors the bash logic with Rust `#[test]`s written **before** the implementation:

- `repo_root()` — walk ancestors for `.git`/`.jj`, else CWD, no realpath. **Match
  the bash loop exactly: the marker check stops *before* `/`** (`while [ "$dir" !=
  "/" ]`, [`vcs-common.sh:10-17`](../../scripts/vcs-common.sh#L10-L17)), so a repo
  located at the filesystem root (`/.git`) is **not** matched and falls back to
  `$PWD`. A Rust `Path::ancestors()` walk that includes `/` would diverge — pin
  the chosen behaviour with a `#[test]`.
- `config_files(root)` — `.accelerator/config.md` then `config.local.md`;
  legacy fallback only when `ACCELERATOR_MIGRATION_MODE=1`.
- `assert_no_legacy_layout()` — the `exit 1` guard equivalent (returns an error).
- `extract_frontmatter()` — distinguishes **three** states, not two: (a) **absent**
  (line 1 is not frontmatter) → silent not-found; (b) **empty-but-closed** (`---\n
  ---`, `[ -z "$fm" ]`, [`config-read-value.sh:54`](../../scripts/config-read-value.sh#L54))
  → silent not-found, **no warning**; (c) **unclosed** (open `^---\s*$`, no close)
  → not-found **with** a stderr warning. The parser opens only on strict
  `^---[[:space:]]*$`, but the unclosed-warning *gate* re-reads line 1 with an
  **unanchored** `grep -q '^---'` ([`config-read-value.sh:49`](../../scripts/config-read-value.sh#L49))
  that also fires on `---foo`/`----` — model the loose warning gate separately from
  the strict parser regex so stderr matches bash.
- `lookup(key, default)` — 2-level split on first dot, string-prefix match,
  whitespace trim, one quote-layer strip. **Two distinct precedence axes**:
  *within* a file, the **first** matching subkey in a section wins (awk `exit`,
  [`config-read-value.sh:86-88`](../../scripts/config-read-value.sh#L86-L88)) — a
  `HashMap`/last-write-wins would invert same-file duplicate keys; *across* files,
  the **last** file wins (the loop does not `break`). Both must be tested.
  **Found-empty is distinct from not-found.** A key that is *present but set to an
  empty value* (`key:`) returns success in `_read_from_file` (awk matched, prints
  empty) → `FOUND=true`, `RESULT=""`, and the **default is suppressed**
  ([`config-read-value.sh:85-87,118-130`](../../scripts/config-read-value.sh#L118-L130));
  only an *absent* key (`FOUND=false`) echoes the default. So `lookup` must return a
  found/not-found flag *separate from* the value — e.g. `Option<FoundValue>` where
  `Some("")` ≠ `None`, not a bare `String` — and the default is applied only when
  **no** file produced a match. This compounds across files: a local config setting
  `key:` (empty) must override a non-empty team value, because the empty match still
  sets `FOUND=true`. A naive `Option<String>` port collapsing found-empty into
  "use default" silently injects the wrong value into the prompt. Fixtures: (a) key
  set empty suppresses the default; (b) local empty value overrides team non-empty.
- `PATH_DEFAULTS` — the 17-row table from `config-defaults.sh:26-64`.
- `read_path(key, default_opt)` — prepend `paths.`, defaults-table fallback only
  when the override is `None`-or-empty. **For an unknown key with no default, bash
  warns to stderr AND still looks up `paths.<key>` with an empty default**
  ([`config-read-path.sh:75`](../../scripts/config-read-path.sh#L75)) — so a user
  who set `paths.unknownkey` gets that configured value, not empty. `read_path`
  must delegate to the value-lookup for unknown keys (the warning is independent of
  the lookup), never return empty immediately. The legacy-override migration
  warning is **two-stage**: substring presence in the config files AND a non-empty
  *resolved* `paths.<legacy>` value ([`config-read-path.sh:59-73`](../../scripts/config-read-path.sh#L59-L73))
  — reusing the in-process value lookup, not the grep gate alone.

```rust
// a9r-core/src/config.rs (signatures)
pub fn repo_root() -> PathBuf;
pub fn read_value(key: &str, default: &str) -> Result<String, ConfigError>;
pub fn read_path(key: &str, default: Option<&str>) -> Result<ReadOutcome, ConfigError>;
// ReadOutcome carries stdout line + stderr warnings, so the bin controls streams.
```

#### 2. `a9r` binary crate

**File**: `skills/visualisation/visualise/a9r/{Cargo.toml,src/main.rs}` (new)
**Changes**: Workspace member. Binary name `a9r`. Depends on `a9r-core` and the
visualiser lib **with `default-features = false`** — the visualiser crate's default
feature is `embed-dist`, whose `build.rs` hard-asserts `frontend/dist/index.html`
exists ([`server/build.rs:5`](../../skills/visualisation/visualise/server/build.rs#L5),
[`Cargo.toml:12-14`](../../skills/visualisation/visualise/server/Cargo.toml#L12-L14)).
Because Cargo unifies features across a workspace build, a plain dependency would
activate `embed-dist` transitively and force the frontend stub onto every
`build:a9r:dev` / `lint:a9r` run — contradicting the lightweight-config-read story.
Gate the SPA embedding behind an `a9r` feature (e.g. `visualise = ["visualiser/embed-dist"]`)
enabled only for the release/visualise build; the default `a9r` build compiles the
visualiser lib **without** `embed-dist` and needs no frontend artifact. clap
`#[derive(Subcommand)]`:

```rust
#[derive(Subcommand)]
enum Command {
    ConfigReadValue { key: String, default: Option<String> },
    ConfigReadPath  { key: String, default: Option<String> },
    Visualise { #[arg(long = "config")] config: PathBuf },
}
```

`Visualise` reuses the boot block lifted from `server/src/main.rs:18-50`
(`Config::from_path` → `log::init` → `redirect_std_streams` → `server::run`).
`ConfigRead*` map to `a9r-core`, writing the value to stdout (single trailing
newline) and warnings to stderr.

- **Exit-code mapping centralised.** Map `ConfigError` → exit code in one function
  (e.g. `impl ConfigError { fn exit_code(&self) }`) with a comment that **not-found
  deliberately exits 0** (echoing the default) to match bash — there is no distinct
  not-found code. Scattering this across `main.rs` arms invites a future "fix" that
  silently breaks parity. Codes: 0 = normal / not-found; 1 = usage error +
  legacy-layout guard.
- **Per-subcommand check ordering differs and must be preserved.**
  `config-read-value` runs `config_assert_no_legacy_layout` *before* argument
  validation ([`config-read-value.sh:22-30`](../../scripts/config-read-value.sh#L22-L30)),
  whereas `config-read-path` validates the key first, then execs value (which
  re-runs the legacy assert). So an invocation with both a legacy layout and a
  missing key emits the *legacy* message (not the usage message) for `value`. Test
  the legacy-layout + empty-key combination per subcommand.
- **Clean-stderr-on-success is an enforced contract, not an assumption.** The
  binary must route *all* clap usage/error output and any panic backtrace to
  stderr (never stdout), and emit nothing on stderr on a success path — this stdout
  is injected verbatim into prompts via the `!` preprocessor. The Phase 3 gate
  asserts any stderr on a success path fails (extending the dropped-newline check).
- **Preserve the empty-vs-omitted default distinction at the arg layer.** Bash
  treats an explicitly-empty `$2` as omitted via `[ -n "${2:-}" ]`
  ([`config-read-path.sh:31`](../../scripts/config-read-path.sh#L31)); clap's
  `default: Option<String>` must keep `config-read-path <key> ''` distinct from
  `config-read-path <key>` (both reach the defaults-table fallthrough, but the
  distinction must survive parsing). Add a parity test for both forms.

#### 3. New `a9r` component tasks

**Files**: `tasks/format/a9r.py`, `tasks/lint/a9r.py`, additions to
`tasks/test/unit.py`, `tasks/shared/paths.py`
**Changes**:
- `paths.py`: `A9R_CARGO_TOML = VISUALISER / "a9r" / "Cargo.toml"` (and a core
  manifest constant). `cargo fmt`/`clippy`/`test` target these manifests.
- `format/a9r.py`: mirror `format/server.py` (`cargo fmt --manifest-path … --
  --check` / fix).
- `lint/a9r.py`: **single** clippy pass (`--all-targets -- -D warnings`) — no
  `build:frontend:stub` dependency (no embedded assets).
- `test/unit.py`: add an `a9r` function running `cargo test --manifest-path
  <a9r-core> --lib` (+ the `a9r` bin crate tests).

#### 4. `mise.toml` wiring

**File**: `mise.toml`
**Changes**: Add `format:a9r:check/fix`, `lint:a9r:check/fix`, `a9r:check`
(`depends = ["format:a9r:check", "lint:a9r:check"]`), `build:a9r:dev`,
`test:unit:a9r`. Add to aggregates: `check` (L342), `format:check` (L210),
`format:fix` (L214), `lint:check` (L326), `lint:fix` (L330), `test:unit` (L126).

#### 5. CI job

**File**: `.github/workflows/main.yml`
**Changes**: Add `check-a9r` mirroring `check-visualiser-server` (L135-164),
including the `RUSTUP_HOME` routing step and a **distinct**
`cache_key_prefix: mise-a9r-v1`. Add `check-a9r` to the `prerelease` (and stable
release) `needs:` lists.

### Success Criteria:

#### Automated Verification:

- [x] `a9r-core` unit tests pass (55 tests): `cargo test -p a9r-core`
- [x] `a9r` builds and lints with **no frontend artifact present** (default
      uses the visualiser `dev-frontend` feature — `embed-dist` not activated):
      `rm -rf …/frontend/dist && mise run build:a9r:dev && mise run lint:a9r:check`
- [x] `a9r` release build with the `visualise` feature embeds the SPA
      (`--no-default-features --features visualise`).
- [x] `a9r config-read-path plans` prints `meta/plans` (byte-for-byte vs bash)
- [x] `a9r config-read-value agents.reviewer reviewer` matches the bash script
- [x] `a9r visualise --config <fixture>` boots (smoke: process started, wrote
      server-info.json with a bound port)
- [x] Lint/format/test for the new component: `mise run a9r:check && mise run test:unit:a9r`
- [x] Full CI green: each `check-*` component passes in isolation (`frontend:check`,
      `server:check`, `a9r:check`, `build-system:check`, `scripts:check`) — which is
      how CI runs them (one component per isolated job). NOTE: the *local* `mise run
      check` aggregate is intermittently flaky due to a **pre-existing** race —
      pyrefly's tree glob (no `node_modules` exclude) hits `frontend/node_modules`
      while the concurrent `frontend:check` runs `npm`. Does not affect CI.

#### Manual Verification:

- [x] `a9r --help` lists the three subcommands with sensible help text.
- [x] stderr is empty on a successful `config-read-*` (clean-stderr contract).
- [x] Two binaries coexist (`a9r`, `accelerator-visualiser`); skills unaffected.

---

## Phase 3: Cross-language parity gate

### Overview

Make the *existing* `config-read-path`/`config-read-value` suites runnable against
either bash or `a9r`, and run both in CI. This is the contract proving the Phase 2
port is byte-for-byte correct — and the gate every later command must pass.

### Changes Required:

#### 1. `run_sut` launcher + env switch

**File**: `scripts/test-helpers.sh`
**Changes**: Add `run_sut <subcommand-key> args…`. When `${A9R_BIN:-}` is unset,
run `bash "$SCRIPT_DIR/<mapped-script>.sh" "$@"` (current behaviour); when set,
run `"$A9R_BIN" <subcommand> "$@"`. Map `read-path`→`config-read-path` /
`config read-path` subcommand, `read-value`→`config-read-value`.

**`run_sut` must NOT `exec`.** Several call sites pass the SUT as the trailing
`"$@"` of an assertion helper (e.g. `assert_exit_code … bash "$READ_REVIEW"`,
[`test-config.sh:1235`](../../scripts/test-config.sh#L1235)); an `exec` would
replace the shell and discard the assertion wrapper. `run_sut` is a plain
function that returns the SUT's exit code so it composes both as a bare command
(under `$(…)` capture) and as the trailing argument of `assert_exit_code` /
`assert_stderr_*`.

**Fail loud, never degrade.** When `A9R_BIN` is set it must be a non-empty,
executable path; if it is set but unresolvable/non-executable, `run_sut` aborts
the suite with a clear error rather than silently falling through to bash. This
closes the "a9r-mode run silently tested bash twice" failure mode (a typo, a
build-ordering bug, or an empty path).

**Mode banner + executed-assertion floor (concrete mechanism).** `run_sut`
increments a mode-scoped counter (`A9R_RUN_SUT_COUNT`) **only on the branch that
actually invokes `a9r`**; `test_summary` prints a banner naming the mode (`SUT
MODE: bash` / `SUT MODE: a9r=<path>`) and the observed count. In a9r mode,
`test_summary` itself fails (non-zero) if that count is below an explicit floor
(e.g. `>= 50`, sized to the ~58 rerouted sites minus guarded ones), so guarding
everything out of the a9r path fails the suite directly — not only via the task
wiring. This per-mode executed floor is **distinct from** the existing
suite-discovery floor (`_EXPECTED_CONFIG_SUITES`, which counts files); add a new
`pytest` assertion for it rather than relying on the discovery test
([`tasks/test/integration.py:14`](../../tasks/test/integration.py#L14)).

#### 2. Reroute config-read call sites

**File**: `scripts/test-config.sh`
**Changes**: Replace `bash "$READ_PATH"` (25 sites) and `bash "$READ_VALUE"`
(33 sites) with `run_sut read-path` / `run_sut read-value`, preserving the
trailing args, redirections, and `cd "$REPO"` exactly. The call sites are **not
uniform** — the reroute must account for every form, not just the bare
command-substitution one:

- bare command-substitution: `OUTPUT=$(cd "$REPO" && bash "$READ_PATH" …)`
  ([L2943](../../scripts/test-config.sh#L2943));
- inline redirections inside the subshell: `2>/dev/null`, `2>&1 1>/dev/null`,
  `2>&1 >/dev/null` (the stderr-isolation tests, ~L448-460 — the most
  error-prone, and exactly where the clean-stderr prompt-injection contract is
  verified);
- helper-form trailing `"$@"`: `assert_exit_code … bash "$READ_REVIEW"`
  ([L1235](../../scripts/test-config.sh#L1235)) — relies on `run_sut` not
  `exec`-ing;
- nested `bash -c "cd '$REPO' && bash '$READ_REVIEW' …"`
  ([L2001](../../scripts/test-config.sh#L2001)) — the embedded literal a naive
  replace will miss (mostly Phase 7 scripts, but enumerate now).

After the reroute, **grep for residual `bash "$READ_`** to prove no site was
left un-migrated (an un-migrated site silently tests bash even in a9r mode).

**Exclude only the genuinely sourced-function tests** — `config_extract_frontmatter`/
`config_extract_body` (L24/L55), which `source` the library and test internal
bash functions with no binary analogue. Guard them via a single
`skip_unless_bash_mode "<reason>"` helper (increments the SKIP counter and logs
the reason in `test_summary`) rather than raw `if [ -z "${A9R_BIN:-}" ]` blocks,
so excluded tests are visibly accounted for, not silently absent.

**Do NOT exclude the SKILL.md grep-assertions** (L4398-4452): they assert literal
strings (e.g. `config-read-path.sh plans`) appear in SKILL.md files and do not
invoke the SUT at all. Because skills keep calling the `.sh` shim (Phase 4), they
remain exactly as valid in a9r mode and must run in both — gating them would
needlessly shrink the a9r-mode net and miss a stale-reference regression if a
later phase ever flips a skill to invoke `a9r` directly.

#### 3. Raw-byte differential assertions (true byte-for-byte gate)

**File**: `scripts/test-config.sh` (or a small sibling `test-config-parity.sh`)
**Changes**: The existing assertions capture output via command substitution
(`OUTPUT=$(…)`), which **strips all trailing newlines** — so a port emitting
zero, one, or three trailing newlines passes `assert_eq` identically. Since this
stdout is injected verbatim into prompts via the `!` preprocessor, trailing-newline
fidelity is contract-critical and currently invisible to the gate.

Add an explicit raw-byte differential check for a representative set of cases per
command: write bash output and `a9r` output to files (no command substitution)
and `cmp` them, or compare `… | xxd`. This is the only assertion form that
actually proves "byte-for-byte". Cover at minimum: a found value, a not-found
default (empty line, exit 0), a sectioned value, a quote-stripped value, and the
tab-delimited `config-read-template` output (Phase 7). The differential runs only
when `A9R_BIN` is set (it compares the two backends directly).

#### 4. CI runs the suite twice (concrete mechanism)

**Files**: `tasks/test/integration.py`, `tasks/test/helpers.py`, `mise.toml`,
`.github/workflows/main.yml`
**Changes**: The existing `run_shell_suites` runs each discovered suite **once**
with no environment control ([`tasks/test/helpers.py:13-40`](../../tasks/test/helpers.py#L13-L40)),
so the twice-run must be wired explicitly — it is not free. Add a dedicated
`test:integration:config-parity` task (and `run_shell_suites`-style helper variant
accepting an `env` overlay) that runs the config suite a **second** time with
`A9R_BIN` exported at the built binary; the existing `config` task continues the
bash (unset) run. Build `a9r` in the job before the parity run. Make the a9r-mode
invocation a **structural requirement**, not a hand-wired CI step: add a unit test
(mirroring `test_integration.py`'s existing run-recording `_FakeContext`) asserting
both the unset and the `A9R_BIN`-set invocations are issued, so a dropped second
run fails CI. The merge gate is the test aggregate that *runs* both
(the bare `mise run` default or `test:integration:config` + `:config-parity`), not
`mise run check` (format + lint only — see Implementation Approach).

### Success Criteria:

#### Automated Verification:

- [x] Bash mode green: `bash scripts/test-config.sh` (555 passed)
- [x] a9r mode green: `A9R_BIN=<built> bash scripts/test-config.sh` (543 passed,
      1 skipped — the sourced-function block, 0 failed)
- [x] Both modes wired into the test task and green in CI: `mise run
      test:integration:config` (bash) + `test:integration:config-parity` (a9r,
      depends on `build:a9r:dev`), both in the `test:integration` aggregate.
- [x] Suite count floors still satisfied: `uv run pytest
      tests/unit/tasks/test_integration.py -v` (7 passed; floor bumped to 17 for
      the new parity suite).
- [x] Raw-byte differential passes: bash vs `a9r` output `cmp`s equal (including
      the single trailing newline) for the representative case set (8/8 cases).
- [x] `A9R_BIN` set to a non-empty non-executable path **fails the suite loudly**
      (does not silently fall back to bash) — exits 1 with a clear message.
- [x] No residual `bash "$READ_` call sites remain after the reroute (grep clean;
      64 `run_sut` sites; dead `$READ_VALUE`/`$READ_PATH`/`$CONFIG_READ_PATH`
      definitions removed).
- [x] Executed-assertion floor met in both modes (a9r-mode executed 64 ≥ floor 50).

#### Manual Verification:

- [x] A deliberately-broken `a9r` output makes the gate bite. Two axes,
      complementary by design: a **trailing-newline** corruption fails the
      raw-byte differential (8/8) while `test-config.sh` is structurally blind to
      it (`$(…)` strips trailing newlines — exactly why the differential exists);
      a **value-level** corruption fails the a9r-mode `test-config.sh` run (55
      failures). Together they cover both stdout-fidelity axes.
- [x] The mode banner (`SUT MODE: …`) prints the expected backend per run
      (`SUT MODE: bash` / `SUT MODE: a9r=<path> (executed N a9r assertions)`).
- [x] Only the sourced-function tests are skipped under `A9R_BIN` (1 logged SKIP
      in the summary); the SKILL.md grep-assertions still run in both modes.

---

## Phase 4: Shims (config-read-path / config-read-value)

### Overview

Replace the two scripts' bodies with thin shims: use `a9r` when resolvable, else
run the original bash inline. Behaviour is identical — both paths are already
green under Phase 3.

### Changes Required:

#### 1. Shared a9r-resolution helper (single source of resolution precedence)

**File**: `scripts/a9r-resolve.sh` (new, sourced library)
**Changes**: `a9r_bin()` returns a usable binary path or non-zero. This helper is
the **single source of truth for resolution precedence** — both the config-read
shims and the refactored launcher's `acquire_binary` (Phase 5 §1) build on it
rather than re-encoding the order, so a future env-var/config-key change lives in
one place. No download (pure resolution); empty/unfound → caller falls back.
Honour an `A9R_FORCE_BASH` escape hatch (forces fallback).

Precedence and trust gates on the **load-time hot path** (invoked on nearly every
skill load), which is a far larger blast radius than today's use-time-only
visualiser launch:

1. `A9R_BIN` (explicit test/dev override) — trusted as a developer signal.
2. `ACCELERATOR_VISUALISER_BIN` — **probe for subcommand support before trusting**
   (e.g. treat a non-zero `<bin> config-read-path --help` as "not a usable a9r"
   and fall back). A user who set this to a pre-rename, visualiser-only binary
   (no `config-read-*` subcommand) must degrade to bash, not error.
3. Config `visualiser.binary` — **restricted on the hot path to gitignored local
   config (`config.local.md`) only**, never the team-committed
   `.accelerator/config.md`. A team-committed `visualiser.binary: ./evil` would
   otherwise auto-execute an attacker-chosen binary on every skill load of a
   merely-cloned/PR'd repo. (The explicit `a9r visualise` launch may still honour
   the team key — that path is user-triggered, not load-time-automatic.)
4. Cached `bin/a9r-<platform>` — **must be a regular, non-symlink, non-world-writable
   file whose SHA-256 is in the verified `bin/checksums.json` manifest** before it
   is returned. Carry the launcher's existing `! -L` symlink rejection and
   executable check ([`launch-server.sh:141-144`](../../skills/visualisation/visualise/scripts/launch-server.sh#L141-L144)).
   A cheap mtime/size guard against the eager-hook-verified cache (Phase 5 §3)
   avoids re-hashing on every call; full re-hash only when that guard trips. This
   means a **present-but-tampered or present-but-buggy** binary is rejected and the
   caller falls back to bash — fallback must not be limited to the binary-*absent*
   case.

#### 2. Rewrite the two scripts as shims

**Files**: `scripts/config-read-path.sh`, `scripts/config-read-value.sh`, and new
siblings `scripts/config-read-path-impl.sh`, `scripts/config-read-value-impl.sh`
**Changes**: Move the existing bash implementation **verbatim** into a sibling
`*-impl.sh` (chosen over an in-file `_fallback()` so the original is preserved
unchanged, the entry script stays a uniform ~5-line shim, and a botched in-place
edit cannot leave a script that neither execs nor falls back cleanly). Apply this
identical shape across every script ported in Phase 7. Top of each shim:

```sh
if [ -z "${A9R_FORCE_BASH:-}" ] && bin="$(a9r_bin 2>/dev/null)"; then
  exec "$bin" config-read-path "$@"   # or config-read-value
fi
exec "$SCRIPT_DIR/config-read-path-impl.sh" "$@"   # or -value-impl
```

Confirm the `config-read-path` → `config-read-value` `exec` chain
([`config-read-path.sh:75`](../../scripts/config-read-path.sh#L75)) still produces
identical output in both shim-active and forced-bash modes — in particular that
the defaults table is applied **exactly once** (the path impl must not re-enter the
value shim's a9r branch in a way that double-applies defaults).

**bash 3.2 floor.** `a9r-resolve.sh`, the rewritten shims, and the
`a9r-provision.sh` hook (Phase 5 §3) run on macOS bash 3.2 and are scanned by
`lint-bashisms.sh`. No bash-4 constructs (associative arrays, `${var,,}`/`${var^^}`):
lower-case `uname` output via `tr '[:upper:]' '[:lower:]'` as `launch-server.sh`
already does. State this in the success criteria.

`allowed-tools` globs already cover `${CLAUDE_PLUGIN_ROOT}/scripts/*`; the binary
is invoked via the shim (never a bare `a9r` path in a SKILL body), so no SKILL.md
frontmatter changes are needed — and the new `SessionStart` hook registration must
fit the v2.1.144 `hooks.json` schema.

### Success Criteria:

#### Automated Verification:

- [x] Shim → a9r path green: `A9R_BIN=<built> bash scripts/test-config.sh`
      (543 passed, 1 skipped, 0 failed; 64 a9r assertions ≥ floor 50).
- [x] Shim → bash fallback green: `A9R_FORCE_BASH=1 bash scripts/test-config.sh`
      (555 passed, 0 failed).
- [x] shellcheck/shfmt/bashisms clean (bash 3.2 floor): `mise run scripts:check`
      (green; one justified `# shellcheck disable=SC2016` in the test's inner
      `bash -c` where `$0` must expand in the inner shell).
- [x] Path→value `exec` chain applies the defaults table exactly once in both
      modes (bash, forced-bash, and a9r agree byte-for-byte for the defaults-table,
      explicit-default, and configured-override cases).
- [x] Full CI green: ran the relevant merge-gate aggregates rather than the heavy
      bare `mise run` (Phase 4 touches only shell + the integration wiring):
      `mise run scripts:check`, `mise run test:integration:config` (bash) +
      `test:integration:config-parity` (a9r), and `pytest test_integration.py` —
      all green. (`mise run check` is format + lint only and does not run the gate.)
- [x] `a9r-resolve.sh` has a black-box suite (`scripts/test-a9r-resolve.sh`, 20/20)
      covering: symlinked cache rejected; world-writable cache rejected; SHA not in
      manifest → fall back (plus no-entry, flat-schema, and all-zeros sentinel
      variants); team-committed `visualiser.binary` ignored on hot path; gitignored
      `visualiser.binary` honoured; legacy visualiser-only binary in
      `ACCELERATOR_VISUALISER_BIN` → fall back; A9R_FORCE_BASH forces fallback.

#### Manual Verification:

- [x] With no binary present, `config-read-path.sh plans` still prints `meta/plans`
      (fallback) with clean stderr (verified in all three modes — empty stderr on
      success).
- [x] With `A9R_BIN` set, the same call routes through `a9r` (the shim `exec`s
      `"$bin" config-read-value/-path`) and output is byte-for-byte identical
      (differential 8/8).
- [x] A tampered cached binary (SHA no longer matches the manifest) is rejected and
      the call falls back to bash — not executed (`test-a9r-resolve.sh` SHA-mismatch
      and sentinel cases).
- [x] A `visualiser.binary` set in team-committed `.accelerator/config.md` does
      **not** alter the hot-path resolution (only `config.local.md` does) — enforced
      by the `ACCELERATOR_CONFIG_LOCAL_ONLY` read mode and pinned by two suite cases.

---

## Phase 5: Distribution + provisioning hook

### Overview

Cross-compile and checksum `a9r`, and add a `SessionStart` hook that eagerly
downloads + SHA-verifies `bin/a9r-<platform>` before skills load. Until a release
ships `a9r`, the hook hits the existing all-zeros "no released binary" sentinel
and degrades to fallback — consistent and mergeable.

### Changes Required:

#### 1. Factor out acquisition helpers

**File**: `skills/visualisation/visualise/scripts/launcher-helpers.sh` (or a new
`bin-acquire.sh`)
**Changes**: Extract the download → SHA-verify → version-drift → cache logic from
`launch-server.sh:124-168` into a reusable function `acquire_binary <name>` so
both the launcher and the new hook call it. `launch-server.sh` refactored to use
it (no behaviour change). The refactor **must preserve the existing portability
and atomicity guarantees** as explicit success criteria, since the hook calls this
on a far wider host range than the launcher ever did:

- `sha256_of` keeps its `sha256sum` → `shasum -a 256` fallback (Linux/macOS).
- `download_to` keeps its `curl` → `wget` fallback and the 127 "no downloader"
  return; the hook treats 127 as a clean degrade-to-fallback, not an error.
- Download writes to a `mktemp` path and is published to the final cache path by
  **atomic rename/`install`** — never a partial file at the cache path — so a
  shim resolving the cache concurrently with the hook's download cannot `exec` a
  truncated binary (the TOCTOU between hook-write and shim-read).
- Add a **hard wall-clock timeout** to the network calls using the downloader's
  **own** flags — `curl --connect-timeout … --max-time …`, and `wget --timeout=…
  --dns-timeout=…` on the fallback branch. **Do not wrap in `timeout(1)`**: it is
  GNU coreutils and is **absent on stock macOS** (only `gtimeout` via Homebrew), so
  a `timeout`-based bound would silently not apply on a primary target. State the
  chosen flag values in the success criteria.

#### 2. Cross-compile + checksums for `a9r`

**Files**: `tasks/build.py`, `tasks/shared/paths.py`
**Changes**: `a9r_binary_path(platform)` → `bin/a9r-<platform>`. Cross-compile
loop (reusing `TARGETS`) builds the `a9r` bin **with the `visualise`/`embed-dist`
feature** (so the released binary embeds the SPA — the default-feature-off applies
to dev/lint, not release), magic-byte-checks, copies to `bin/`.

**`checksums.json` schema must gain an asset-name dimension.** The manifest's
`binaries` map is currently keyed by `<platform>` alone (`darwin-arm64`, …), one
SHA slot per platform ([`bin/checksums.json`](../../bin/checksums.json)), and the
launcher verifies whatever asset it fetched against that single hash
([`launch-server.sh:125`](../../skills/visualisation/visualise/scripts/launch-server.sh#L125)).
That cannot represent the dual-asset transition (Phase 6 §1): an old launcher
falling back to `accelerator-visualiser-<platform>` would verify it against the
`a9r` SHA and fail. Nest hashes by asset name (`binaries[platform][asset-name]`),
update `create_checksums`/`update_checksums_json`
([`tasks/build.py`](../../tasks/build.py)) and **both** the new and old launcher's
lookup in lockstep, and state which schema each launcher resolves against. Extend
`validate_version_coherence` to include the `a9r` artifact.

**Sequence the checksum key with the published asset.** The launcher `die`s on the
all-zeros sentinel and on `MANIFEST_VERSION != PLUGIN_VERSION`, and the visualiser
launch path has **no bash fallback** (only config-read does). So a `checksums.json`
that lists an `a9r` key *before* a release actually publishes the `a9r` asset would
make `a9r visualise` un-launchable. During Phase 5 (binary not yet renamed) the
`a9r` artifact is built and checksummed **alongside** the still-shipping
`accelerator-visualiser` artifact — both names present — rather than replacing it.
The all-zeros sentinel for the not-yet-released `a9r` key is acceptable here only
because the config-read shim degrades to bash; the visualiser still launches via
the existing `accelerator-visualiser` asset until the transition release lands.

#### 3. SessionStart provisioning hook (eager, blocking)

**Files**: `hooks/a9r-provision.sh` (new), `hooks/hooks.json`
**Changes**: Hook modelled on `config-detect.sh` shape (`jq` guard, `SCRIPT_DIR`,
JSON `hookSpecificOutput`). Register as a 4th `SessionStart` object in
`hooks.json:3-31`. The hook's resilience contract — it gates skill loading, so it
must never stall or fail a session:

- **Fast cache-valid fast-path first.** If `bin/a9r-<platform>` is already present
  and SHA-valid (the `a9r-resolve.sh` cheap guard), the hook does **no network
  work** and returns immediately. The network is touched only on a cold/invalid
  cache.
- **Bounded and fail-open.** The acquisition runs under the Phase 5 §1 hard
  timeout; on *any* non-success — offline, timeout, missing curl/wget (127),
  all-zeros sentinel, unsupported platform/arch, SHA mismatch — the hook emits
  nothing and **returns 0**. It must never surface the launcher's `die_json`; the
  bash fallback covers every degrade path.
- **Eager-blocking vs background.** Default to a *bounded blocking* download
  (simplest; the timeout caps worst-case session-start latency). If the bounded
  worst case is still judged too costly, the hook may kick the download into the
  background and return 0 immediately, since the shim already degrades to bash for
  any call that races ahead of the download completing. Record the chosen mode and
  its timeout budget in the hook.
- **SLSA provenance.** Where provenance is available for the released `a9r`
  artifact, verify it in `acquire_binary` (the binary is auto-executed at session
  start, so its integrity root should not be the co-located `checksums.json`
  alone); treat any `ACCELERATOR_VISUALISER_RELEASES_URL` mirror override as
  untrusted and HTTPS-pinned on this eager path.

#### 4. Acquisition integration test

**File**: `mise.toml` + a new `test:integration:a9r-acquisition` suite modelled on
`binary-acquisition`
**Changes**: Cover sentinel-rejection, SHA-mismatch, 404/offline → fallback, and
version-drift rejection for the `a9r` artifact. Add the hook-resilience cases:
**timeout** (slow server) → exit 0 fallback; **missing downloader** (no curl/wget,
127) → exit 0 fallback; **unsupported platform/arch** → exit 0 fallback (no
`die_json`); **fast-path** (valid cache → no network call); and a
**concurrent-download** check that a shim resolving the cache mid-download never
sees a partial file (atomic publish).

### Success Criteria:

#### Automated Verification:

- [x] Cross-compile produces `bin/a9r-<platform>` for all 4 targets:
      `mise run build:server:cross-compile`. **Code-complete; the 4-target run is
      CI-gated, not run locally** (needs `zig` + 3 cross rust targets the sandbox
      lacks). `server_cross_compile` now builds *both* assets per target —
      accelerator-visualiser (default/embed-dist) and a9r
      (`--no-default-features --features visualise`, embedding the SPA) — and
      stages each to `bin/`. Fixed the stale `server/target` path (workspace
      members share `visualise/target`). The native `a9r --features visualise`
      release compile was verified to build.
- [x] `checksums.json` nests hashes by asset name; both the new and old launcher
      resolve their respective asset against the correct SHA. Schema is now
      `binaries[platform][asset-name]`; `launch-server.sh` resolves
      `accelerator-visualiser-<platform>`, `a9r-resolve.sh`+the hook resolve
      `a9r-<platform>`, both tolerating the legacy flat schema for a skewed
      manifest. Verified via the launcher suite's sentinel/SHA-mismatch/404 cases
      (nested fixtures) and test-a9r-resolve (20/20).
- [x] Network timeouts use `curl --max-time`/`--connect-timeout` + `wget
      --timeout` (no dependency on `timeout(1)`). Defaults: connect 10s, total
      60s (overridable via `ACCELERATOR_DOWNLOAD_*`). Added `--retry-max-time` so
      `--retry 3` back-off cannot blow the cap — verified by the timeout case
      (returned in 1s against an 8s-stalled server).
- [x] Checksums + coherence pass: `mise run build:checksums`. **Code-complete;
      CI-gated** (depends on the cross-compile above). `create_checksums` /
      `update_checksums_json` / `validate_version_coherence` for the nested
      dual-asset schema are covered by the build unit tests (17 passed); the a9r
      artifact now participates in the same checksum/coherence pipeline.
- [x] Acquisition tests pass: `mise run test:integration:a9r-acquisition`
      (22/22 — sentinel, version drift, valid download, fast-path, SHA mismatch,
      404, timeout-in-budget, no-downloader, atomic-publish/no-partial).
- [x] Hook is valid and registered: `jq . hooks/hooks.json` (4 SessionStart
      objects) and the existing `bash hooks/test-*.sh` suites stay green
      (126 + 11). The hook's own black-box suite is `test-a9r-acquisition.sh`
      (in visualise/scripts so it reuses the launcher's HTTP-fixture +
      `acquire_binary`), run via `test:integration:a9r-acquisition`.
- [x] Full CI green: ran the relevant merge-gate aggregates rather than the heavy
      bare `mise run` (Phase 5 touches shell + Python build/wiring + the manifest;
      no frontend/server source) — `scripts:check`, `build-system:check`,
      `test:integration:config` (bash) + `:config-parity` (a9r) + `:a9r-acquisition`,
      and the full `tests/unit/tasks` suite (194) — all green. (`mise run check`
      is format + lint only and does not run the gate.)

#### Manual Verification:

- [x] Offline session start: hook fails quietly, all skills still load (fallback).
      Covered by the 404 and no-downloader (127) cases — exit 0, no cache, so the
      config-read shims fall back to bash.
- [x] With a populated cache, hook takes the fast-path (no network) and exits
      fast. The fast-path case points the mirror at a server serving *different*
      bytes and asserts the cache is untouched — i.e. the network was not hit.
- [x] Tampered cached binary is rejected and re-acquired (or fallback). The
      fast-path SHA check rejects a non-matching cache and falls through to
      `acquire_binary` (which re-downloads); the SHA-mismatch case proves a
      tampered/served binary is never published (no partial at the cache path).
- [x] Slow/hung network: hook returns within the timeout budget and degrades to
      fallback — session start is not stalled. Timeout case: returned in 1s
      against an 8s-stalled server, no cache written.

---

## Phase 6: Complete the fold (rename + single binary)

### Overview

Make `a9r` the one shipped binary. Switch the launcher to `a9r visualise`, rename
the server lib package to `visualiser`, remove the old `accelerator-visualiser`
`[[bin]]`, and keep a transitional alias (Decision 2).

### Changes Required:

#### 1. Launcher invokes `a9r visualise`

**File**: `skills/visualisation/visualise/scripts/launch-server.sh`
**Changes**: Resolve the binary via the shared acquire helper, trying the new
`bin/a9r-<platform>` asset name **then falling back to the old
`accelerator-visualiser-<platform>`** so a version-skewed plugin/release pair
(installed launcher requesting one name, the pinned release carrying the other)
does not hard-404. Launch `nohup "$BIN" visualise --config "$CFG"` (was `"$BIN"
--config "$CFG"`, L200).

**Close the `visualise`-path team-key RCE.** The hot-path restriction (Phase 4 §1)
keeps the team-committed `visualiser.binary` out of config-read, but
`launch-server.sh` still reads `visualiser.binary` from team-committed
`.accelerator/config.md` and execs it after only an `-x` check
([`launch-server.sh:110-121`](../../skills/visualisation/visualise/scripts/launch-server.sh#L110-L121)),
with the path rooted at the project ([L114](../../skills/visualisation/visualise/scripts/launch-server.sh#L114)).
A checked-in `visualiser.binary: ./tools/x` plus a checked-in `./tools/x` is then a
one-step RCE the first time a user opens the visualiser on a hostile clone. Apply
the **same gitignored-`config.local.md`-only restriction** to the `visualiser.binary`
key on the launch path (or, at minimum, require interactive confirmation when a
team-committed value points inside the repo) — the rename must not carry the
unmitigated team-config execution vector forward.

**Transition release publishes both asset names.** The renamed plugin's launcher
requests `a9r-<platform>`; an older installed launcher still requests
`accelerator-visualiser-<platform>`. Define an explicit transition release that
uploads **both** assets (and lists both in `checksums.json`) for at least one
version, so neither direction of skew 404s. Document the minimum release version
that first ships `a9r`.

**Transitional invocation alias.** Prefer an `accelerator-visualiser` symlink /
wrapper that deterministically prepends `visualise` over teaching the clap parser
two grammars. clap's derive `Subcommand` has no native default subcommand, and a
bare top-level `--config` mixed with the `ConfigRead*` positional grammar is
ambiguous — so if the bare `a9r --config` form is supported at all, implement it
with explicit argv pre-processing (detect "no subcommand + leading `--config`" and
inject `visualise`), guarded by `a9r --version`/`--help` handling, with a test
asserting `a9r --config X` ≡ `a9r visualise --config X`. Record a concrete removal
trigger (tied to the bash-fallback deletion / alias-removal milestone) so the
shim is bounded.

#### 2. Rename server lib package → `visualiser`

**Files**: `server/Cargo.toml`, the workspace manifest `members`, `a9r/Cargo.toml`
dep, all `use accelerator_visualiser::…` in `server/tests/*` and `a9r`.
**Changes**: Package `name = "visualiser"`; drop the `[[bin]]` (lib only).
Mechanical `use` updates. **Keep the directory name `server/` and rename only the
package** — the preferred path, since the `../frontend/dist` literal is triplicated
(`build.rs:5`, `assets.rs:9,71`), relative to the crate manifest, and a missed copy
breaks asset embedding only in the `embed-dist` release build (dev builds still
pass, so the regression is easy to miss). While the crate is being touched, **hoist
`../frontend/dist` into a single shared item** (e.g. a `const`/`concat!(env!(
"CARGO_MANIFEST_DIR"), …)` referenced by all three sites) to remove the hand-sync
requirement at low cost. A directory rename `server/` → `visualiser/` is permitted
only if all three literals move in lockstep and a success criterion asserts the
release build still embeds and serves the SPA.

#### 3. Distribution/paths cleanup

**Files**: `tasks/shared/paths.py`, `tasks/shared/targets.py`, `tasks/build.py`,
`tasks/version.py`, `.github/workflows/main.yml`,
`skills/visualisation/visualise/scripts/test-launch-server.sh`
**Changes**: Drop `accelerator-visualiser-<platform>` naming in favour of
`a9r-<platform>` (keep an alias copy/symlink during transition if downstream
expects the old name — see the dual-asset transition release in Phase 6 §1).
Update release `needs:` lists and any `accelerator-visualiser` string references.

**Update the launcher test suite.** `test-launch-server.sh` drives a
`make_fake_visualiser` fake invoked as `"$BIN" --config` against a fixture named
`accelerator-visualiser-${OS}-${ARCH}`. Teach the fake the `visualise` subcommand
(it must accept `visualise --config`), rename the cached/fixture filename to
`a9r-<platform>`, and add an assertion that the `visualiser.binary` /
`ACCELERATOR_VISUALISER_BIN` overrides still route to `a9r visualise` unchanged —
this is the exact user-facing override contract that must survive the rename.

### Success Criteria:

#### Automated Verification:

- [ ] Single binary builds and boots: `mise run build:a9r:dev && a9r visualise --config <fixture>`
- [ ] Launcher integration green (fakes updated for `visualise`): `mise run test:integration:binary-acquisition`
- [ ] Launcher tries `a9r-<platform>` then falls back to `accelerator-visualiser-<platform>`.
- [ ] Transition release publishes **both** asset names and both appear in `checksums.json`.
- [ ] Visualiser unit tests pass under the renamed lib: `mise run test:unit:visualiser`
- [ ] Coherence holds with the renamed/single artifact: `mise run build:checksums`
- [ ] Full CI green: `mise run` (default — includes tests; `mise run check` is
      format + lint only and does not run the parity gate)

#### Manual Verification:

- [ ] The visualiser skill launches and serves the SPA via `a9r visualise`.
- [ ] Transitional alias (`--config` shorthand / symlink) still works, and
      `a9r --config X` ≡ `a9r visualise --config X`.
- [ ] `visualiser.binary` / `ACCELERATOR_VISUALISER_BIN` overrides still launch the
      visualiser through `a9r visualise`.

---

## Phase 7: Port the rest of the `config-read-*` family

### Overview

Apply the proven pattern (a9r-core logic → subcommand → parity gate → shim) to the
remaining family members, absorbing `config-common.sh` into `a9r-core`
incrementally. Each command is its own independently-mergeable sub-slice.

### Changes Required (repeated per command):

Commands, in order of increasing complexity:
`config-read-context`, `config-read-skill-context`,
`config-read-skill-instructions`, `config-read-agents`, `config-read-template`
(3-tier template resolution, tab-delimited output —
[`config-common.sh:189-229`](../../scripts/config-common.sh#L189-L229)), and
`artifact-derive-metadata`.

For each: (1) JIT-audit its dependency closure and backfill any missing black-box
tests **green on bash first** (Decision 6); (2) add the `a9r-core` logic with Rust
`#[test]`s; (3) add the `a9r` subcommand; (4) run its existing suite under
`A9R_BIN` (extend `run_sut` mapping); (5) convert the script to a shim.

**Files** (per command): `a9r-core/src/*.rs`, `a9r/src/main.rs`,
the relevant `scripts/<name>.sh`, `scripts/test-*.sh` (mapping + any backfill),
`.github/workflows/main.yml` (twice-run coverage already established in Phase 3).

### Success Criteria:

#### Automated Verification:

- [ ] Each command's suite green in both modes: `bash scripts/test-config.sh` and
      `A9R_BIN=<built> bash scripts/test-config.sh` (plus
      `test-metadata-helpers.sh` for `artifact-derive-metadata`)
- [ ] `config-read-template` tab-delimited `<source>\t<path>` output matches bash
      byte-for-byte
- [ ] Coverage backfill suites (if any) green on bash before the port lands
- [ ] Full CI green: `mise run` (default — includes tests; `mise run check` is
      format + lint only and does not run the parity gate)

#### Manual Verification:

- [ ] Spot-check a real skill load (e.g. a `!`-preprocessor `config-read-context`
      call) renders identically with `a9r` active vs forced-bash.
- [ ] stderr remains clean on success for every ported command (prompt-injection
      safety).

---

## Testing Strategy

### Unit Tests (Rust, `a9r-core`):

- Repo-root discovery (`.git`/`.jj`/none, no realpath, **marker at `/` not
  matched**), config-file precedence (team/local/legacy + migration-mode),
  frontmatter extraction (absent / empty-but-closed-silent / unclosed-warns /
  open-closed, **strict parser vs loose warning-gate regex**), value lookup
  (sectioned, top-level, quote-strip, 2-level split, **within-file first-match-wins**,
  **across-file last-file-wins-from-second-file-only**), defaults table,
  **empty-vs-omitted default**, **unknown key still reads `paths.<key>`**,
  **two-stage legacy-override warning** (match-but-empty vs match-and-set), per-
  subcommand check ordering (legacy-layout + empty-key), exact stderr warning
  substrings (`.claude/accelerator.md`, `/accelerator:migrate`, "Warning") and the
  exit code per branch. Codify these as a shared fixture table that **both** the
  bash parity suite and the Rust `#[test]`s iterate, so the spec lives in one place
  rather than as prose.

### Integration / Parity Tests (the regression net):

- The existing `test-config.sh` (and `test-metadata-helpers.sh`) run twice per CI
  — bash and `a9r` — via the `A9R_BIN` switch. This is the primary correctness
  guarantee. Acquisition behaviour covered by `test:integration:a9r-acquisition`.
- **The command-substitution assertions do not prove byte-for-byte parity on
  their own** (they strip trailing newlines). A raw-byte differential (`cmp` of
  bash vs `a9r` output files) for a representative case set is what enforces the
  exact-bytes contract, including the single trailing newline that reaches the
  `!`-preprocessor prompt-injection channel.
- The gate is guarded against silent erosion: `A9R_BIN` must be executable (fail
  loud otherwise), no `bash "$READ_` site may survive the reroute, and an
  executed-assertion floor ensures the a9r path actually ran.

### Manual Testing Steps:

1. Build `a9r`; run `bash scripts/test-config.sh` with and without `A9R_BIN`.
2. Boot the visualiser via the skill (`a9r visualise`) and confirm the SPA serves.
3. Start a session offline (no cache) — confirm all skills still load (fallback).
4. Force a tampered/empty cache — confirm the hook re-acquires and SHA-verifies.

## Performance Considerations

Load-time `!` config reads run synchronously before a skill renders. A Rust spawn
is single-digit ms and touches none of the axum/SPA weight (only `a9r visualise`
starts tokio), so the port is expected to be **faster** than the bash path
(sourcing `*-common.sh` + spawning `jq`). The eager SessionStart download adds
one-time session-start latency; it is bounded by the existing
download/verify path and degrades to fallback on slowness/offline.

## Migration Notes

- The bash fallback is retained for the entire plan; nothing is deleted that the
  fallback depends on (`config-common.sh`, `config-defaults.sh`, `vcs-common.sh`).
- Binary-name transition uses an alias/symlink (Decision 2); removal of the alias
  and of the bash fallback are explicitly later-release work.
- The single inherited workspace version keeps `plugin.json` / Cargo / checksums
  coherence to one source of truth.

## References

- Research: [`meta/research/codebase/2026-06-15-migrate-bash-scripts-to-rust-a9r.md`](../research/codebase/2026-06-15-migrate-bash-scripts-to-rust-a9r.md)
  (incl. the six follow-up Decisions)
- Bare-path invocation call sites: [`meta/research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md`](../research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md)
- Bash-prefix permission pain point: [`meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`](../research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md)
- Current CLI: [`skills/visualisation/visualise/server/src/main.rs:7-13`](../../skills/visualisation/visualise/server/src/main.rs#L7-L13)
- Config contract: [`scripts/config-read-value.sh`](../../scripts/config-read-value.sh), [`scripts/config-read-path.sh`](../../scripts/config-read-path.sh), [`scripts/config-common.sh`](../../scripts/config-common.sh)
- Test harness: [`scripts/test-helpers.sh`](../../scripts/test-helpers.sh), [`scripts/test-config.sh`](../../scripts/test-config.sh)
- Build/dist: [`tasks/build.py:87-103,153-167`](../../tasks/build.py#L87-L103), [`tasks/shared/paths.py`](../../tasks/shared/paths.py), [`skills/visualisation/visualise/scripts/launch-server.sh:104-200`](../../skills/visualisation/visualise/scripts/launch-server.sh#L104-L200)
- CI template: [`.github/workflows/main.yml:135-164`](../../.github/workflows/main.yml#L135-L164)
