---
type: codebase-research
id: "2026-06-15-migrate-bash-scripts-to-rust-a9r"
title: "Research: Migrating skill-invoked bash scripts to a unified Rust `a9r` executable"
date: "2026-06-15T14:01:59+00:00"
author: "Phil Helm"
producer: research-codebase
status: complete
topic: "Migrating skill-invoked bash scripts to Rust under a unified `a9r` executable, test-guarded for behavioural parity"
tags: [research, codebase, bash, rust, a9r, cli, migration, testing, visualiser, build-system]
revision: "21b96adac354e0285d1c76420a92681cb1938697"
repository: "accelerator"
last_updated: "2026-06-15T15:00:24+00:00"
last_updated_by: "Phil Helm"
last_updated_note: "Added follow-up Decisions section resolving the eight open questions"
schema_version: 1
---

# Research: Migrating skill-invoked bash scripts to a unified Rust `a9r` executable

**Date**: 2026-06-15T14:01:59+00:00 (UTC)
**Author**: Phil Helm
**Git Commit**: 21b96adac354e0285d1c76420a92681cb1938697
**Branch**: sacramento
**Repository**: accelerator

## Research Question

We want to migrate the bash scripts in this project to Rust — in particular the
bash scripts called by the skills. We already have some Rust in the project; we
want an `a9r` executable with sub-commands (the existing visualiser becomes one,
plus new ones). To avoid breaking anything, the strategy is: first ensure good
test suites exist for the bash scripts to be migrated and that they all pass
*before* migration, then run the *same* tests against the migrated Rust to
confirm nothing has broken.

## Summary

The migration is feasible and the codebase is unusually well-positioned for it,
but three structural facts shape the whole effort:

1. **The migration surface is large and library-coupled.** There are ~142 `.sh`
   files. Production logic concentrates in a shared root [`scripts/`](../../../scripts)
   library (~40 production files) plus per-skill `scripts/` directories, with
   the Jira integration ([`skills/integrations/jira/scripts/`](../../../skills/integrations/jira/scripts))
   being the single largest cluster (~20 production scripts). Scripts pervasively
   `source` sibling `*-common.sh` libraries and even `exec` each other, and
   resolve paths via `${BASH_SOURCE[0]}`-relative traversal. This internal
   coupling — not the call sites — is the hard part of the port.

2. **The skill→script contract is a CLI contract with two modes.** Skills invoke
   scripts either (a) at *load time* via the `!` preprocessor (``!`command` ``),
   whose **stdout is injected into the prompt** (clean stderr separation is
   mandatory), or (b) at *use time* as plain Bash-tool calls with positional/flag
   args and a meaningful **exit-code taxonomy**. A Rust replacement must preserve
   argv parsing, byte-for-byte stdout, clean-stderr-on-success, and exit codes.
   The `allowed-tools` frontmatter globs (e.g. `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)`)
   also gate execution and must permit the replacement binary's path.

3. **The existing test harness is migration-friendly *for black-box CLI tests*
   but not for sourced-function tests.** The home-grown
   [`scripts/test-helpers.sh`](../../../scripts/test-helpers.sh) harness asserts
   stdout/stderr/exit-code independently — exactly the CLI contract a port must
   preserve. But many suites invoke scripts as `bash "$SCRIPT" args` (needs
   parameterising to point at a binary) and a significant subset `source` the
   library and test internal bash *functions* directly (cannot be reused against
   a binary at all). There are also concrete coverage gaps (`vcs-status.sh`,
   `vcs-log.sh`, `jsonl-common.sh`, `log-common.sh`, `vcs-guard.sh`) that must be
   filled *before* porting those scripts.

The existing Rust visualiser already depends on `clap` 4 (derive) but is a flat
single-command (`--config <path>`) crate named `accelerator-visualiser`. Turning
it into an `a9r` multi-command binary is a clap `#[derive(Subcommand)]` refactor,
but the binary name, the launcher's hard-coded invocation, the build/cross-compile
targets, and version-coherence enforcement all reference `accelerator-visualiser`
and would need coordinated updates. **No prior ADR governs language choice — this
is net-new architectural-decision territory.**

## Detailed Findings

### 1. The migration surface: inventory of bash scripts

Total: ~142 `.sh` files plus one extensionless shell wrapper
([`skills/visualisation/visualise/cli/accelerator-visualiser`](../../../skills/visualisation/visualise/cli/accelerator-visualiser)).
No `.bash` files; no shell under `tasks/`, `bin/`, or the `server/`/`frontend/`
trees.

**Production clusters (port targets):**

- **Root shared library** [`scripts/`](../../../scripts) (~40 production files).
  The hottest dependencies, called by nearly every skill: `config-read-context.sh`,
  `config-read-skill-context.sh`, `config-read-skill-instructions.sh`,
  `config-read-path.sh`, `config-read-template.sh`, `config-read-agents.sh`,
  `artifact-derive-metadata.sh`. Sourced libraries use a `-common.sh` suffix:
  `atomic-common.sh`, `config-common.sh`, `config-defaults.sh`, `fs-common.sh`,
  `jsonl-common.sh`, `log-common.sh`, `vcs-common.sh`, `work-common.sh`.
- **Jira integration** [`skills/integrations/jira/scripts/`](../../../skills/integrations/jira/scripts)
  (~20 production `jira-*-flow.sh` + `jira-common.sh`) — the largest single
  cluster, with a namespaced exit-code taxonomy (e.g. `80 E_SHOW_NO_KEY`).
- **Config init/migrate** — [`skills/config/init/scripts/init.sh`](../../../skills/config/init/scripts/init.sh)
  and the migration framework under
  [`skills/config/migrate/`](../../../skills/config/migrate) (numbered migration
  scripts `0001`–`0007`, a `run-migrations.sh` driver, `interactive-lib.sh`).
- **Per-skill scripts**: decisions (`adr-next-number.sh`, `adr-read-status.sh`),
  work (`work-item-*.sh`), github (`pr-base-repo.sh`, `pr-update-body.sh`),
  design (`inventory-design/scripts/*`, incl. the Playwright executor
  `playwright/run.sh`), visualisation (`launch-server.sh`, `visualiser.sh`, etc.).
- **Hooks** [`hooks/`](../../../hooks): `config-detect.sh`, `vcs-detect.sh`,
  `vcs-guard.sh`, `migrate-discoverability.sh`.

**Naming conventions** (stable across the repo): tests are uniformly `test-*.sh`;
sourced libraries are `*-common.sh` or `*-lib.sh`; `test-helpers.sh` is a sourced
test-support library, not a suite.

### 2. How skills invoke scripts (the contract to preserve)

Three syntactic patterns (see [`skills/vcs/commit/SKILL.md:6-15`](../../../skills/vcs/commit/SKILL.md#L6-L15)
as the canonical example):

- **`allowed-tools` gate** — frontmatter globs restrict which scripts a skill may
  run, e.g. `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)`
  ([`skills/vcs/commit/SKILL.md:6-7`](../../../skills/vcs/commit/SKILL.md#L6-L7)).
  A replacement binary must be covered by these globs.
- **`!` preprocessor (load-time)** — ``!`command` `` lines whose **stdout is
  captured into the rendered prompt**
  ([`skills/vcs/commit/SKILL.md:12-15`](../../../skills/vcs/commit/SKILL.md#L12-L15)).
  Used both as standalone lines and inline within prose/args (e.g. a nested
  agent-name lookup at [`skills/planning/review-plan/SKILL.md:265`](../../../skills/planning/review-plan/SKILL.md#L265)).
- **Plain-path body instructions (use-time)** — script paths in code fences with
  args, executed later via the Bash tool (e.g.
  [`skills/github/review-pr/SKILL.md:125`](../../../skills/github/review-pr/SKILL.md#L125)
  redirects `pr-base-repo.sh {number}` output to a file).

**Output / exit-code conventions** (all `set -euo pipefail`):

- stdout is the prompt-injection channel; many scripts emit ready-to-inject
  Markdown blocks (e.g. `config-read-skill-instructions.sh` prints
  `## Additional Instructions`,
  [`scripts/config-read-skill-instructions.sh:31-37`](../../../scripts/config-read-skill-instructions.sh#L31-L37)).
- **Errors/warnings always go to stderr** so they never pollute injected stdout.
- Exit codes are meaningful: `exit 0` for empty-but-valid, `exit 1` for usage
  errors, and namespaced taxonomies in Jira/work scripts (e.g.
  [`skills/integrations/jira/scripts/jira-show-flow.sh:15-21`](../../../skills/integrations/jira/scripts/jira-show-flow.sh#L15-L21)).

**Agents that invoke scripts**: only the two browser agents
([`agents/browser-analyser.md`](../../../agents/browser-analyser.md),
[`agents/browser-locator.md`](../../../agents/browser-locator.md)), and only
indirectly — they receive the resolved Playwright `run.sh` path from the
preloaded `browser-executor` skill. The other seven agents invoke no scripts.

**Internal coupling (the real migration risk)**: scripts `source` sibling
libraries, resolve `SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"`
then derive `PLUGIN_ROOT` by relative `..` traversal (e.g.
[`skills/work/scripts/work-item-next-number.sh:15-20`](../../../skills/work/scripts/work-item-next-number.sh#L15-L20)),
and even `exec` each other (`config-read-path.sh` execs `config-read-value.sh`,
[`scripts/config-read-path.sh:75`](../../../scripts/config-read-path.sh#L75)).

### 3. Existing bash test suites (the regression net)

**Framework**: no third-party runner (no bats/shunit2). Every suite sources the
home-grown harness [`scripts/test-helpers.sh`](../../../scripts/test-helpers.sh),
which maintains `PASS`/`FAIL`/`SKIP` counters
([lines 16-18](../../../scripts/test-helpers.sh#L16-L18)) and ends via
`test_summary` ([lines 348-358](../../../scripts/test-helpers.sh#L348-L358),
`return 1` on any failure, propagated as exit code under `pipefail`).

**Assertion helpers** cover exactly the three CLI channels a port must preserve:
- stdout: `assert_eq` (line 20), `assert_contains` (33), `assert_matches_regex` (119)
- stderr (isolated, treated as significant): `assert_stderr_empty` (242),
  `assert_stderr_contains` (257)
- exit code: `assert_exit_code` (197)
- JSON: `assert_json_eq` (289, via `jq`)
- filesystem: `assert_file_exists`/`assert_dir_exists`/`assert_file_content_eq`, etc.

**Suite→script mapping** (the giant one is
[`scripts/test-config.sh`](../../../scripts/test-config.sh), 5992 lines, covering
the whole config layer plus `hooks/config-detect.sh`):

- Black-box CLI suites (reusable against a binary once invocation is
  parameterised): `test-config.sh`, `test-metadata-helpers.sh`
  (`artifact-derive-metadata.sh`), `test-validate-corpus-frontmatter.sh`,
  `test-template-frontmatter.sh`, `hooks/test-vcs-detect.sh` (CLI part),
  `hooks/test-migrate-discoverability.sh`, plus the per-skill suites
  (decisions, github, jira ×20+, design, migrate, work via root suites).
- Sourced-function suites (test internal bash functions — **cannot** be reused
  against a binary, must be rewritten as black-box CLI tests):
  `test-atomic-common.sh`, `test-interactive-protocol.sh`
  (`escape_field`/`unescape_field`), `test-merge-move.sh` (`merge_move` only),
  and the sourced-function portions of `test-config.sh`
  (`config_extract_frontmatter`/`config_extract_body`) and `test-vcs-detect.sh`.

**How tests run**:
- Individually: `bash scripts/test-config.sh`, `bash hooks/test-vcs-detect.sh`.
- Via tasks: discovered by `run_shell_suites(context, subtree)`
  ([`tasks/test/helpers.py:13-40`](../../../tasks/test/helpers.py#L13-L40)),
  which globs `<subtree>/**/test-*.sh`, keeps only **executable** files
  (`os.access(p, os.X_OK)`) excluding `test-helpers.sh`, and runs each. Dispatched
  per-subtree under `test:integration:*` (config, decisions, hooks, github,
  migrate) plus three corpus suites under `test:unit:templates`. **There is no
  single `test:scripts` task.** Count floors and by-name requirements guard
  against silently dropped suites (e.g. `_EXPECTED_CONFIG_SUITES = 16`,
  [`tasks/test/integration.py:14`](../../../tasks/test/integration.py#L14)).
- `mise run check` does **not** run tests — only format + lint. Tests live under
  the `test`/`test:integration` aggregates.

**Coverage gaps (must be filled before porting these)**: `scripts/vcs-status.sh`,
`scripts/vcs-log.sh`, `scripts/jsonl-common.sh`, `scripts/log-common.sh`,
`scripts/accelerator-scaffold.sh`, `scripts/interactive-harness.sh`,
`scripts/lint-bashisms.sh` (the linter itself), and `hooks/vcs-guard.sh` have no
dedicated test coverage. `fs-common.sh` is only partially covered (`merge_move`
only).

**Migration caveats from the harness design:**
1. Suites invoke the SUT as `bash "$SCRIPT" args` — to reuse against Rust,
   parameterise this (e.g. an env var pointing at the binary), since
   `bash <rust-binary>` will not work.
2. `assert_eq` against multi-line `printf` expectations means the Rust port must
   reproduce stdout **byte-for-byte** (mind trailing-newline behaviour — command
   substitution strips trailing newlines, so post-strip output must match).
3. `assert_stderr_empty` treats *any* stderr as significant — the Rust binary
   must emit nothing to stderr on success paths.
4. Discovery depends on the executable bit and `test-*.sh` naming plus count
   floors — renaming/splitting suites during migration requires updating
   [`tasks/test/integration.py`](../../../tasks/test/integration.py).

### 4. Existing Rust setup (the foundation for `a9r`)

The crate is [`skills/visualisation/visualise/server/`](../../../skills/visualisation/visualise/server):

- **`Cargo.toml`**: package `accelerator-visualiser`, edition 2021, MSRV 1.85,
  `publish = false`. Features: `default = ["embed-dist"]` (bakes SPA via
  `rust-embed`), `dev-frontend` (serves SPA from disk). A `[lib]` holds the logic;
  one `[[bin]]` named **`accelerator-visualiser`** at `src/main.rs`
  ([`Cargo.toml:17-21`](../../../skills/visualisation/visualise/server/Cargo.toml#L17-L21)).
- **Already depends on `clap` 4 (derive)**
  ([`Cargo.toml:34`](../../../skills/visualisation/visualise/server/Cargo.toml#L34)),
  but the CLI is a **flat single command** — no `Subcommand` enum:
  ```rust
  // src/main.rs:7-13
  #[command(name = "accelerator-visualiser", version, about)]
  struct Cli { #[arg(long = "config", value_name = "PATH")] config: PathBuf }
  ```
  Adding `a9r` subcommands is a `#[derive(Subcommand)]` refactor where the
  current server path becomes one subcommand (e.g. `serve`/`visualise`).
- **`src/` is a thin binary over a fat lib** — `main.rs` only parses CLI + boots;
  `lib.rs` exposes modules (`server`, `config`, `indexer`, `api/`, etc.). This
  structure is favourable: new subcommands become new lib modules behind a clap
  enum.

**Build / distribution** ([`tasks/build.py`](../../../tasks/build.py)):
- `build:server:dev` → `cargo build --no-default-features --features dev-frontend`.
- `build:server:release` → `cargo build --release` (embeds frontend).
- `build:server:cross-compile` → loops 4 targets via `cargo zigbuild`
  (`aarch64`/`x86_64` × `apple-darwin`/`unknown-linux-musl`,
  [`tasks/shared/targets.py`](../../../tasks/shared/targets.py)), verifies
  Mach-O/ELF magic bytes, copies to `bin/accelerator-visualiser-<platform>`.
- Release: cross-compile → SHA-256 → `bin/checksums.json` → GitHub Release
  upload + re-download verify ([`tasks/github.py:136-178`](../../../tasks/github.py#L136-L178)),
  optional SLSA provenance.

**Runtime download/verify/invoke is bash, not Rust** —
[`skills/visualisation/visualise/scripts/launch-server.sh`](../../../skills/visualisation/visualise/scripts/launch-server.sh):
platform detection, tri-precedence binary resolution (env
`ACCELERATOR_VISUALISER_BIN` > config `visualiser.binary` > download-and-cache),
SHA verify against `checksums.json`, then `nohup "$BIN" --config "$CFG" &`
([line 200](../../../skills/visualisation/visualise/scripts/launch-server.sh#L200)).
**This hard-coded `--config` invocation is a call site to update** when the CLI
becomes subcommand-based.

**Version coherence**: `.claude-plugin/plugin.json`, `server/Cargo.toml`, and
`bin/checksums.json` must agree. Written together by
[`tasks/version.py`](../../../tasks/version.py) and validated by
`validate_version_coherence` ([`tasks/build.py:87-103`](../../../tasks/build.py#L87-L103));
the launcher independently rejects version drift at runtime
([`launch-server.sh:134-140`](../../../skills/visualisation/visualise/scripts/launch-server.sh#L134-L140)).

### 5. Build-system task wiring (how `a9r` integrates)

`mise.toml` **is** the dependency graph (`depends = [...]`); invoke tasks are
mostly leaves. Four components — `frontend`, `server`, `build-system`, `scripts`
— each have parallel format/lint(/types) tasks rolled up by `<component>:check`,
then folded into `format:check`, `lint:check`, `types:check`, the top-level
`check` (CI mirror), `fix`, and `default`.

**The Rust `server` component is the template a new binary follows:**
- Format: `cargo fmt --manifest-path {CARGO_TOML} --all -- --check`
  ([`tasks/format/server.py`](../../../tasks/format/server.py)).
- Lint: clippy run **twice** (all-features and default-features) to cover
  feature-gated arms ([`tasks/lint/server.py:6-30`](../../../tasks/lint/server.py#L6-L30)).
  The mise lint tasks `depends = ["build:frontend:stub"]` because clippy needs a
  `frontend/dist/index.html` to satisfy `embed-dist`'s `build.rs` — **a
  standalone `a9r` binary with no embedded frontend would not need this stub**.
- Test: `cargo test --lib` run twice (dev-frontend, default features)
  ([`tasks/test/unit.py:6-24`](../../../tasks/test/unit.py#L6-L24)).

**Integrating a new component requires editing ~5 aggregate `depends` lists**
(`format:check`, `format:fix`, `lint:check`, `lint:fix` if it has an autofixer,
`check`, plus `types:check` if typed) and **adding a CI job** in
[`.github/workflows/main.yml`](../../../.github/workflows/main.yml). Any new Rust
CI job must mirror the `check-visualiser-server` `RUSTUP_HOME` /
`cache_key_prefix` workaround ([lines 144-161](../../../.github/workflows/main.yml#L144-L161))
that prevents parallel format/lint invocations racing to install the toolchain.

**Shell-component plumbing relevant during the transition**: shell sources are
discovered by a `.gitignore`-honouring filesystem walk
([`tasks/shared/sources.py:60-100`](../../../tasks/shared/sources.py#L60-L100)),
with extensionless scripts appended via `_EXTRA_SHELL_SOURCES`
([lines 55-57](../../../tasks/shared/sources.py#L55-L57)) — currently just the
`accelerator-visualiser` CLI wrapper. While ported scripts remain as thin shell
shims, they still flow through shfmt/shellcheck/bashisms.

### 6. The bash 3.2 floor and the bashisms linter

[`scripts/lint-bashisms.sh`](../../../scripts/lint-bashisms.sh) enforces a bash
**3.2 floor** (ADR-0016) because macOS ships bash 3.2 while CI runs bash 5.x. It
is explicitly **KNOWN-INCOMPLETE** — it only catches an enumerated denylist
(associative arrays, `mapfile`/`readarray`, `${var^^}`/`${var,,}` case
modification, `&>>`, `|&`, negative array subscripts). Per-line opt-out via
`# lint-bashisms: ignore`.

**Relevance**: the entire bashisms/shellcheck/shfmt apparatus exists to keep
shell on the 3.2 floor. Migrating a script's logic into `a9r` removes it from
this constraint — but only once the logic moves and the `.sh` is deleted or
shrunk to a thin shim. This is itself a strong *motivation* for the migration.

## Code References

- `scripts/test-helpers.sh:16-358` — the shared assertion harness (counters,
  stdout/stderr/exit-code/JSON asserts, `test_summary`). The contract any Rust
  port must satisfy.
- `scripts/test-config.sh:1-5992` — largest suite; config layer + `config-detect.sh`.
- `scripts/config-read-path.sh:75` — example of inter-script `exec` chaining.
- `skills/work/scripts/work-item-next-number.sh:15-50` — `${BASH_SOURCE[0]}` path
  resolution + GNU-style flag parsing.
- `skills/integrations/jira/scripts/jira-show-flow.sh:15-21` — namespaced
  exit-code taxonomy.
- `skills/vcs/commit/SKILL.md:6-15` — canonical `allowed-tools` gate + `!`
  preprocessor example.
- `skills/visualisation/visualise/server/src/main.rs:7-13` — current flat clap
  CLI (no subcommands).
- `skills/visualisation/visualise/server/Cargo.toml:17-21,34` — binary name,
  features, existing clap dependency.
- `skills/visualisation/visualise/scripts/launch-server.sh:200` — hard-coded
  `--config` invocation to update.
- `tasks/build.py:87-103` — version-coherence validator.
- `tasks/lint/server.py:6-30` — the Rust lint template (double clippy pass).
- `tasks/test/helpers.py:13-40` — shell-suite discovery (exec-bit + naming).
- `tasks/test/integration.py:14` — count-floor guard.
- `tasks/shared/sources.py:55-57` — `_EXTRA_SHELL_SOURCES` for extensionless
  scripts.
- `scripts/lint-bashisms.sh:46-51` — the bash-3.2 denylist.

## Architecture Insights

- **The skill↔script boundary is already a clean CLI/IPC contract**, not in-process
  coupling. Skills shell out and consume stdout/exit codes. This is the single
  most important enabler: a Rust binary that reproduces the contract is a
  drop-in replacement, language-agnostic from the skill's perspective.
- **Two distinct contracts must be honoured separately**: (a) prompt-injecting
  stdout with clean stderr (the `!` preprocessor path) and (b) argv + exit-code
  taxonomy (the use-time Bash path). Byte-for-byte stdout fidelity matters for (a).
- **The test harness already asserts the right things** (stdout/stderr/exit
  independently) — but a subset tests *internal bash functions* via `source`,
  which has no analogue in a compiled binary. Those need rewriting as black-box
  CLI tests, ideally *before* the port so they remain green across the boundary.
- **`a9r` is a clap subcommand refactor of an existing crate**, not a greenfield
  binary. The thin-binary/fat-lib layout means each migrated script becomes a lib
  module behind a `#[derive(Subcommand)]` enum. The visualiser's `embed-dist`
  coupling (frontend stub for clippy, cross-compile magic-byte checks) is specific
  to the SPA-serving subcommand; new pure-logic subcommands avoid that weight.
- **Naming is a cross-cutting rename**: `accelerator-visualiser` is referenced in
  `Cargo.toml`, `tasks/shared/paths.py`, `tasks/shared/targets.py`, the launcher,
  `checksums.json`, and `_EXTRA_SHELL_SOURCES`. Renaming the binary to `a9r` (or
  adding `a9r` as the umbrella with `accelerator-visualiser` as an alias/subcommand)
  is a coordinated change across distribution + version-coherence machinery.
- **Migration sequencing is naturally test-gated**: (1) backfill missing test
  coverage and convert sourced-function tests to black-box CLI tests; (2)
  parameterise suites to run against either `bash <script>` or `<binary> <subcmd>`;
  (3) port a script's logic to an `a9r` subcommand; (4) re-run the same suite
  against the binary; (5) replace the `.sh` with a thin shim or update call sites
  and delete it; (6) drop the script from the bashisms/shellcheck scope.

## Historical Context

No prior document directly proposes the bash-to-Rust / `a9r` migration — it is a
new idea. Closest adjacent context:

- [`meta/work/0106-invoke-plugin-scripts-by-bare-path.md`](../../../meta/work/0106-invoke-plugin-scripts-by-bare-path.md)
  and [`meta/research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md`](2026-06-11-0106-bare-path-script-invocation-call-sites.md)
  — inventory of bare-path script-invocation call sites; directly the set a Rust
  CLI would replace.
- [`meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`](../issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md)
  — a concrete pain point with the current bash-wrapper invocation model;
  motivation for a unified binary.
- [`meta/work/0098-repo-wide-linting-formatting-static-analysis.md`](../../../meta/work/0098-repo-wide-linting-formatting-static-analysis.md)
  — where shellcheck/shfmt/bashisms + shell-testing guardrails were decided
  (the baseline a migration would shrink).
- [`meta/research/codebase/2026-06-06-0101-unified-dev-task-for-visualiser.md`](2026-06-06-0101-unified-dev-task-for-visualiser.md)
  — closest "consolidation" precedent (process orchestration, not binary unification).
- [`meta/plans/2026-04-30-meta-visualiser-phase-12-packaging-docs-and-release.md`](../../../meta/plans/2026-04-30-meta-visualiser-phase-12-packaging-docs-and-release.md)
  — Rust binary build/release/download mechanics.
- No ADR governs language choice / Rust-vs-shell / binary consolidation — net-new
  ADR territory if the migration proceeds.

## Related Research

- [`meta/research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md`](2026-06-11-0106-bare-path-script-invocation-call-sites.md)
- [`meta/research/codebase/2026-06-09-0098-repo-wide-linting-formatting-static-analysis.md`](2026-06-09-0098-repo-wide-linting-formatting-static-analysis.md)
- [`meta/research/codebase/2026-06-06-0101-unified-dev-task-for-visualiser.md`](2026-06-06-0101-unified-dev-task-for-visualiser.md)

## Open Questions

1. **Naming strategy**: rename `accelerator-visualiser` → `a9r` with the visualiser
   as a subcommand, or introduce `a9r` as a new umbrella binary and keep
   `accelerator-visualiser` as an alias during transition? This drives the scope
   of the distribution/version-coherence rename.
2. **Single binary vs. workspace of crates**: one `a9r` binary with many
   subcommands, or a Cargo workspace (shared lib + `a9r` bin)? Affects build times,
   binary size, and whether the SPA-embedding weight leaks into pure-logic commands.
3. **Distribution for non-visualiser commands**: the visualiser binary is
   downloaded on first *visualiser* use. If `a9r` backs the *hot* config-read
   scripts invoked on nearly every skill load, the download/verify/cache path must
   be fast and must exist before the binary is needed — or those commands must ship
   another way (e.g. always-present). How is `a9r` provisioned for load-time `!`
   calls?
4. **Migration ordering**: start with the high-fan-in but simple config-read
   scripts (max leverage, well-tested via `test-config.sh`), or with a
   self-contained vertical like Jira (largest cluster, own exit-code taxonomy,
   own tests)? 
5. **Sourced-function test rewrite**: which suites test internal functions vs CLI,
   and what is the effort to convert them to black-box CLI tests before porting?
6. **Test parameterisation mechanism**: env var selecting `bash <script>` vs
   `<binary> <subcmd>` — confirm byte-for-byte stdout and clean-stderr parity can
   be asserted unchanged for both backends.
7. **Performance/latency**: load-time `!` calls run synchronously before a skill
   renders. Is a Rust binary (process spawn + any first-run download) faster or
   slower than bash here, especially for the many small config reads?
8. **Coverage backfill scope**: `vcs-status.sh`, `vcs-log.sh`, `jsonl-common.sh`,
   `log-common.sh`, `vcs-guard.sh`, `interactive-harness.sh` need tests written
   first — how much of this is on the critical path for the first migration slice?

## Follow-up: Decisions [2026-06-15T15:00:24+00:00]

The eight open questions were talked through and resolved into six decisions
(some questions merged). These are durable choices that shape the migration; they
are recorded here as research follow-up and have **not yet** been promoted to an
ADR, work item, or plan.

### Decision 1 — Crate architecture (resolves Q2)

**Cargo workspace producing a single `a9r` binary.**

- `a9r-core` library crate: shared logic (config parsing, frontmatter, VCS, etc.),
  unit-testable in Rust.
- `visualiser` library crate: the existing axum + `rust-embed` server logic,
  refactored from the current `accelerator-visualiser` crate into a lib.
- `a9r` binary crate: thin `clap` `#[derive(Subcommand)]` umbrella depending on
  both libs; all subcommands (including `visualise`) live here.

Accepted consequence: the shipped binary bundles axum + the embedded SPA even for
tiny hot-path config reads, so it is **heavy**. This makes provisioning (Decision
3) the critical-path concern. Rejected alternatives: single crate with
feature-gated visualiser (build-variant maintenance), and a two-binary split
(would have decoupled hot-path provisioning but conflicts with the single-binary
vision).

### Decision 2 — Naming & transition (resolves Q1)

**Transitional alias.** Ship `a9r`; keep `accelerator-visualiser` working
(symlink, and/or accept the bare `--config` form as shorthand for `a9r visualise
--config`) until the migration settles, then remove the alias in a later release.
The visualiser subcommand is `a9r visualise` (British spelling, matching
`skills/visualisation/visualise`). User-facing knobs are kept compatible during
transition.

### Decision 3 — Provisioning & latency (resolves Q3, Q7)

**SessionStart hook + bash-fallback shims.**

- A new SessionStart hook (alongside `config-detect.sh` / `vcs-detect.sh`) ensures
  `bin/a9r-<platform>` is present and SHA-verified before any skill loads —
  downloading once via the existing release/checksums mechanism, then a fast cache
  check on subsequent sessions.
- Each migrated script becomes a **thin shim**: `if a9r present → exec a9r
  <subcommand>; else → existing bash implementation`. A failed/offline download
  degrades to bash and breaks nothing. The fallback cost is already sunk by the
  test-parity strategy (both implementations coexist during transition). Once a
  subcommand is proven, the shim flips to a9r-only, and the bash is deleted later.
- **Latency (Q7) is resolved as a non-issue, likely a win.** A Rust spawn is
  single-digit ms; the embedded SPA is never touched by a config read, and only
  `a9r visualise` starts the tokio runtime. This is almost certainly faster than
  the current bash path (sourcing `*-common.sh` libs + spawning `jq`). Binary
  *size* affects first-download/disk, not per-call latency.

Rejected: embedding all four platform binaries in the git-distributed plugin
(~50–100MB+, bypasses the checksums/SLSA pipeline) — especially unattractive given
the single heavy binary; and keeping the hot path in bash entirely (would skip the
highest-fan-in scripts).

### Decision 4 — Migration ordering (resolves Q4)

**Walking skeleton, then the `config-read-*` family.**

1. Walking skeleton: drive one trivial, pure-black-box, well-covered script
   (`config-read-path.sh` or `config-read-value.sh`) end-to-end through the entire
   new pipeline (workspace + `a9r-core` + clap subcommand + shim + SessionStart
   hook + parameterised tests + CI job) before any bulk migration.
2. `config-read-*` family — highest fan-in, simplest logic, best existing
   coverage; absorbs `config-common.sh` into `a9r-core` as it goes.
3. Self-contained verticals: `work-item-*`, `decisions/adr-*`, `github`
   (`pr-base-repo`, `pr-update-body`).
4. **Jira (`jira-*-flow`) last** — largest, most complex (HTTP, auth, ADF), its
   own exit-code taxonomy.
5. Shared libraries absorbed into `a9r-core` incrementally as their consumers
   migrate (not an up-front slice).

### Decision 5 — Test mechanics (resolves Q6, Q5-test-facet)

**Shim env-switch for parity; split-by-layer for internals.**

- Parameterisation: the shim is the switch. Give it an `A9R_FORCE_BASH` override
  and run each migrated suite **twice** in CI (bash path forced / `a9r` active) —
  both must be green. Existing `bash "$SCRIPT" args` invocations barely change,
  and the shim itself gets exercised.
- Sourced-function tests: the cross-language **parity gate lives at the CLI
  boundary** (black-box suites, both backends). Internal helpers get idiomatic
  unit tests per language — bash function tests stay while the bash fallback
  exists and retire with it; `a9r-core` equivalents get Rust `#[test]`s. The same
  test is not forced to run both sides for internals (they are not the contract).
  "Promote to a hidden subcommand" only where a helper has no observable
  command-level effect (expected to be rare).

### Decision 6 — Coverage backfill scope (resolves Q8)

**Just-in-time per slice.** At each slice start, audit the dependency closure of
the scripts being ported; for any untested dependency, write its black-box tests
(green on bash) *before* porting. No up-front baseline sweep. The first slice
(`config-read-*`) is already well-covered by `test-config.sh`, so it needs no
backfill.

### Scope clarification

Per the user's framing ("scripts called by the skills"), the following are
**out of scope** for this migration and treated as a possible later phase:
lifecycle/guard hooks (`vcs-guard.sh`, `config-detect.sh`, `vcs-detect.sh`,
`migrate-discoverability.sh`) and dev tooling (`lint-bashisms.sh`). Note the new
SessionStart provisioning hook (Decision 3) is additive, not a migration of an
existing hook.

### Not yet decided / next artifacts

The decisions above have not been promoted to formal artifacts. Candidate next
steps (not started): an ADR for the durable architecture + test-parity strategy
(no prior ADR governs language choice), a work-item epic with per-slice children,
and an implementation plan for the walking skeleton + `config-read` slice.
