---
type: codebase-research
id: "2026-06-23-0136-shell-scripts-rust-cli-migration-surface"
title: "Research: Shell-script feature & test-suite surface for the Rust CLI migration"
date: "2026-06-23T19:27:07+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0136"
parent: "work-item:0136"
topic: "Shell-script feature & test-suite surface for the Rust CLI migration"
tags: [research, codebase, shell, rust-cli, migration, tooling, vcs, config, frontmatter]
revision: "d1f2186bfeb1d753a1b844e9f0d238a538e76e98"
repository: "accelerator"
last_updated: "2026-06-23T19:27:07+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Shell-script feature & test-suite surface for the Rust CLI migration

**Date**: 2026-06-23T19:27:07+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: d1f2186bfeb1d753a1b844e9f0d238a538e76e98
**Branch**: HEAD (detached / jj working copy)
**Repository**: accelerator

## Research Question

To educate epic [0136 — Migrate Shell Scripts into a Rust CLI](../../work/0136-migrate-shell-scripts-to-rust-cli.md):
we want to perform the migration **in stages**, so we need to understand the
**full extent of features and test suites currently implemented in shell** in
order to phase the delivery of the Rust-based CLI replacement.

## Summary

The shell surface is **large but cleanly partitioned**: ~226 tracked `.sh`
files / ~58.9k lines (excluding `workspaces/` jj checkouts and vendored
`node_modules`). The **bulk is test code, not production logic** — the single
biggest file is a 6,188-line test suite. Production logic concentrates in
**~30 sourced-only libraries** (the `SHELL_LIBRARIES` manifest) plus a fan-out
of thin entrypoint scripts that skills invoke by bare path.

The work decomposes into **eight functional clusters** that are largely
independent and can be migrated one at a time, plus a **build-system/CI
tooling layer** (shfmt, ShellCheck, a custom bashisms linter, the exec-bit
invariant, and the shell-suite discovery/floors) that the migration must
progressively retire.

The migration is gated by **one hard contract** that must be preserved at every
step: skills invoke scripts as **bare executable paths under
`${CLAUDE_PLUGIN_ROOT}`**, matched against `allowed-tools` prefix globs — a
`bash`/`sh`/`env` wrapper breaks the permission match. A Rust CLI must be
reachable and directly executable at those same paths (per-name shims/symlinks)
or every call site + `allowed-tools` glob + the 0107 lint guard must change in
lockstep.

**Recommended phasing axis: by functional cluster, leaf-first.** Order clusters
by coupling (leaf utilities → config/VCS reading → parsers → atomic/locking →
integrations → migration engine), keeping the shell test suites as the
behavioural oracle until each cluster's Rust replacement passes them.

## Detailed Findings

### Scale and shape of the shell surface

Repo-wide (verified live, excluding `workspaces/` and `node_modules`):
**226 `.sh` files, ~58,909 lines.** Distribution by area (line counts from the
locator sweep):

| Area | Dir | ~Files | ~Lines | Production vs test |
|---|---|---|---|---|
| Core library + config/frontmatter | `scripts/` | 63 | 16,230 | mixed; biggest test suite here |
| Hooks | `hooks/` | 7 | 1,267 | 4 prod + 2 test + 1 fixture |
| Work-item lifecycle | `skills/work/scripts/` | 28 | 6,572 | 22 prod + 6 test |
| Jira integration | `skills/integrations/jira/scripts/` | 43 | 11,938 | 22 prod + 21 test |
| Linear integration | `skills/integrations/linear/scripts/` | 24 | 5,245 | 12 prod + 12 test |
| Migration engine + migrations | `skills/config/migrate/` | 23 | 11,476 | engine + 7 migrations + 4 large test suites + fixtures |
| Design / inventory tooling | `skills/design/` | 14 | 1,904 | 10 prod + 4 test |
| Visualiser launcher | `skills/visualisation/visualise/scripts/` | 13 | 2,255 | 7 prod + 6 test |
| GitHub PR helpers | `skills/github/` | 6 | 1,445 | 3 prod + 3 test |
| ADR helpers | `skills/decisions/scripts/` | 3 | 362 | 2 prod + 1 test |
| Config init | `skills/config/init/scripts/` | 2 | 215 | 1 prod + 1 test |

**Key structural fact**: `tasks/` has **no `.sh` files** — the build system is
pure Python invoke. Shell is confined to `scripts/`, `hooks/`, and
`skills/*/scripts/`.

**Largest single targets** (effort sinks, mostly tests):
`scripts/test-config.sh` (6,188), `skills/config/migrate/scripts/test-migrate.sh`
(2,511), `test-migrate-0007.sh` (2,193), `test-migrate-interactive.sh` (2,081),
`skills/work/scripts/test-work-item-scripts.sh` (1,822).

### Cluster 1 — Migration engine (HIGHEST complexity, deeply stateful)

Files: `skills/config/migrate/scripts/run-migrations.sh` (677),
`interactive-lib.sh` (985), `scripts/interactive-protocol.sh` (169), plus 7
numbered `migrations/*.sh` (115–806 lines each).

Hard-to-port behaviour:
- **Two named FIFOs + literal fds 7/8/9** for bidirectional runner↔child IPC —
  bash 3.2 has no `coproc` (`interactive-lib.sh:726-754`, fd dance `:421-437`).
- **30s watchdog subshell** escalating SIGTERM→SIGKILL (`:937-954`).
- **JSON parsed twice** with two independent escape implementations that MUST
  agree: shell writer (`jsonl-common.sh`) and an embedded **awk JSON parser**
  reader (`interactive-lib.sh:46-124`).
- **Guarded resume / staleness** keyed on jj `change_id` vs git `HEAD`
  (`run-migrations.sh:180,246`); fail-closed dirty-tree ownership check.
- 19-frame TSV state machine documented at `interactive-protocol.sh:9-59`.

This cluster is the natural **last** migration phase — it depends on the
atomic/JSONL and config/VCS clusters and carries the most subtle concurrency.

### Cluster 2 — Atomic JSONL writes + locking (HIGH, concurrency-critical)

Files: `scripts/atomic-common.sh` (248), `scripts/jsonl-common.sh` (149).

- `atomic_write` (stdin → same-dir `mktemp` → `mv` rename, EXIT-trap cleanup).
- **`mkdir(2)` as the lock primitive** (not `flock`, absent on macOS;
  `atomic-common.sh:82-89`); sidecar `<target>.lockdir`.
- **PID-owner reclaim**: `$BASHPID` in `lockdir/owner`; waiters `kill -0` the
  owner and reclaim a dead holder's lock (`:105-163`) — fixed a CI flake (see
  Historical Context).
- **Jittered exponential back-off**, 300s ceiling (`:118-136`).
- **bash 3.2 fallback**: when `$BASHPID` is unset, degrades to spin-only.
- `jsonl_compose_record` enforces canonical field order; awk remover uses
  `ENVIRON` (not `-v`) to avoid backslash re-interpretation (`:243`).

Depended on by the migration engine and work-item sync. Port early so
downstream clusters can build on the Rust primitive.

### Cluster 3 — VCS detection (HIGH, branchy)

File: `scripts/vcs-common.sh` (281). Consumers: hooks `vcs-detect.sh`/
`vcs-guard.sh`, `vcs-status.sh`/`vcs-log.sh`, config + migration engine.

- `vcs_mode` — **`.jj`-wins** dispatch (jj outranks git even when colocated).
- `classify_checkout` — emits a 6-line `KEY=VALUE` record; `KIND ∈ {main,
  jj-secondary, git-worktree, colocated, nested-jj-in-git, nested-git-in-jj,
  none}`. **Arm-ordering is load-bearing** (`colocated` before `nested-*`,
  first-match-wins, `:232-272`).
- Handles git worktrees, submodules, bare repos, `GIT_DIR` scrubbing; the
  single place the `.jj/repo` file-vs-dir marker is read (`:74`).

External tools: `jj`, `git` (≥2.5), `realpath`, `dirname`, `command -v`.

### Cluster 4 — Config reading + frontmatter/YAML (MEDIUM-HIGH)

Files: `scripts/config-common.sh` (463), `config-read-value.sh` (holds the YAML
reader), `config-read-path.sh`, `config-defaults.sh` (115), plus the
`config-read-*` / `config-*-template` entrypoint family (~17 entrypoints).

- **Hand-rolled awk YAML parser** — no YAML library; only a frontmatter subset.
  2-level dot-nested keys via `substr`/`index` (string compare, not regex, to
  avoid metachar injection) (`config-read-value.sh:43-44`).
- **Team→local override** order (`.accelerator/config.md` then
  `config.local.md`); legacy `.claude/accelerator.md` only under
  `ACCELERATOR_MIGRATION_MODE=1`.
- **Frontmatter writeback** with re-parse + read-back integrity check before
  atomic rename (`config-common.sh:160,244`); value via `ENVIRON` to block
  injection.
- **Parallel arrays** stand in for associative arrays on the 3.2 floor
  (`config-defaults.sh:66-115`).

This cluster is the **most-invoked** at skill-load time (`config-read-context`,
`config-read-path`, `config-read-agents`, `config-read-template`, …) — it is
the highest-value early win for skill responsiveness, but also the contract
hotspot (every skill body calls it).

### Cluster 5 — Path resolution + doc-type inference (MEDIUM)

Files: `config-read-path.sh`, `doc-type-inference.sh` (74),
`config-read-doc-type-paths.sh`, `doc-type-table.sh`.

- Longest-dir-wins path→doc-type classification over injected parallel arrays.
- **Locale-forcing `export LC_ALL=C`** (`config-read-doc-type-paths.sh:39-41`)
  for byte-stable sorting/matching across host locales.
- ⚠️ The doc-type fact is **triplicated** (here, the corpus validator, and
  `0007-frontmatter-rewrite.awk:path_to_typed`) and must stay aligned
  (`doc-type-inference.sh:24-29`) — a Rust port can consolidate this.

### Cluster 6 — Filesystem + leaf utilities (LOW-MEDIUM)

- `scripts/fs-common.sh` (73): `merge_move` — recursive **non-atomic** dir
  merge, source-wins, idempotent, unsafe-destination guards; bash-3.2-safe
  glob triples.
- `scripts/hash-common.sh` (29): SHA-256 via `sha256sum`/`shasum` detection —
  trivial in Rust (crypto crate).
- `scripts/log-common.sh` (18): `log_die`/`log_warn`.
- `scripts/work-common.sh` (26), `artifact-derive-metadata.sh` (25),
  `linkage-parser.sh` (338), `validate-corpus-frontmatter.sh` (440).

These are the **ideal first phase** — low coupling, easy to validate against
existing tests, build out the Rust CLI skeleton + test harness on them.

### Cluster 7 — Integrations (MEDIUM, network/auth-heavy)

- **Jira** (`skills/integrations/jira/scripts/`, 22 prod): libs `jira-common`,
  `jira-auth`, `jira-jql`, `jira-body-input`, `jira-custom-fields`; flows for
  create/update/comment/transition/search/show/attach/init; ADF↔markdown
  converters; uses `jq` + `curl`.
- **Linear** (`skills/integrations/linear/scripts/`, 12 prod): libs
  `linear-common`, `linear-auth`; `linear-graphql.sh` (535) + flows.
- Both have **Python mock HTTP servers** for tests
  (`test-helpers/mock-jira-server.py`, `mock-linear-server.py`).

Self-contained, network-bounded clusters — good mid-stage migration candidates;
the mock servers carry over as integration-test scaffolding for the Rust client.

### Cluster 8 — Work-item lifecycle (MEDIUM)

`skills/work/scripts/` (22 prod): libs `work-item-common` (477),
`work-item-bridge-codes`; ops for create/fetch/update remote, sync
(classify/decide/apply/baseline/label), normalise, next-number, section-diff,
read-field/status. Depends on config + JSONL + the integration clients.
**Coverage gap**: ~14 `work-item-*` scripts have no dedicated test suite (see
Open Questions).

### Hooks (separate I/O contract)

`hooks/hooks.json` registers (bare `${CLAUDE_PLUGIN_ROOT}` paths):
- **SessionStart**: `vcs-detect.sh`, `config-detect.sh`,
  `migrate-discoverability.sh`.
- **PreToolUse** (matcher `Bash`): `vcs-guard.sh` (git-guard).

Hooks follow the **Claude Code hook I/O protocol** (JSON on stdin,
exit-code/JSON semantics), *not* the simpler stdout-injection of the `!`
preprocessor — a Rust replacement must honour that protocol and remain
executable at the `hooks/*.sh` registered paths (or `hooks.json` changes).

### Test suites — the behavioural oracle for the migration

Harness: a **custom plain-bash assertion library** `scripts/test-helpers.sh`
(381) — no bats. Provides `assert_eq`, `assert_contains`, `assert_exit_code`,
`assert_json_eq`, `assert_file_executable`, `test_summary`, etc. Sourced by
every suite. Two area-specific helper libs (GitHub `gh`/`jq` stubs; visualiser).
Two Python mock servers (jira/linear).

**Runner = Python invoke layer**, not shell:
- `tasks/test/helpers.py::run_shell_suites` glob-discovers **executable**
  `**/test-*.sh`, excluding `{test-helpers.sh, test-jira-scripts.sh}`, runs each
  via the exec bit (not `bash …`).
- `tasks/test/integration.py` has one `@task` per area with **minimum suite-count
  floors** (config ≥19, work ≥6, integrations ≥32, migrate ≥4) + by-name gates.
  Rationale: a dropped exec bit would silently shrink the regression net.

~80 dedicated shell suites total. By area: config 20, integrations 32 (jira 20 +
linear 12), work 6, visualiser 6, migrate 4, design 4, github 3, hooks 2,
decisions 1, config-init 1.

**Phasing implication**: the shell suites are the **specification** for each
cluster. The clean migration loop per cluster is: keep the shell suite green
while it still tests shell → port the cluster to Rust → repoint/port the suite
→ delete the shell suite + production scripts together. The Python floors must
be decremented in lockstep as suites retire.

### The invocation contract (the hard constraint, preserve at every step)

1. **Addressing** — every entrypoint is a bare path under
   `${CLAUDE_PLUGIN_ROOT}/scripts/`, `…/skills/.../scripts/`, or `…/hooks/`,
   directly executable (shebang + `0755`).
2. **Invocation shape** — bare path, **never** `bash`/`sh`/`env`-wrapped;
   positional args (`config-read-path.sh work`); `"$ARGUMENTS"` passthrough.
3. **Permission match** — invocations must match `allowed-tools` prefix globs
   (e.g. `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)`). The matcher strips
   only `timeout/time/nice/nohup/stdbuf` before matching; `bash`/`sh`/`env` are
   **not** stripped, so a wrapper escapes the glob and forces a permission
   prompt (work item 0106; RCA `2026-06-10-bash-prefix-defeats-...`).
4. **Output contract** — stdout = the value injected into the prompt (path,
   template body, context block); **exit codes carry meaning** (e.g.
   `config-diff-template.sh` exit 2 = "no customisation").
5. **Hooks** additionally honour the harness hook I/O protocol.

**Migration consequence**: the Rust CLI must be invocable as a bare path
matching the existing globs with no wrapper. Two strategies:
(a) ship one thin executable shim/symlink per current script name under the
globbed prefixes (lowest-risk, keeps call sites + `allowed-tools` unchanged,
enables incremental swap), or (b) change every body invocation + `allowed-tools`
glob + the **0107 lint guard** together (bigger blast radius). Strategy (a) is
what makes staged delivery possible.

### Build-system / CI tooling that the migration retires

All in `tasks/` (Python invoke) + `mise.toml`:
- `format:scripts:check/fix` → `shfmt` (style from `.editorconfig`, no flags).
- `lint:scripts:shellcheck` → ShellCheck (`.shellcheckrc`, `enable=all`).
- `lint:scripts:bashisms` → custom `scripts/lint-bashisms.sh` (8-construct
  bash-4 denylist enforcing the 3.2 floor, ADR-0016).
- `lint:scripts:exec-bits` → pure-Python exec-bit invariant + `SHELL_LIBRARIES`
  frozenset (`tasks/lint/scripts.py:18-51,99-147`).
- Source discovery: `tasks/shared/sources.py::shell_sources()` (`.gitignore`-
  honouring `os.walk`, **not** `git ls-files` — blind in jj workspaces).
- CI: dedicated `check-scripts` job (`main.yml:99-115`) on ubuntu; shell
  integration suites run on **both** ubuntu + macos (`fail-fast: false`).

Each is work the migration **removes** as scripts disappear: retire the bashisms
linter, the exec-bit invariant + `SHELL_LIBRARIES`, the `.shellcheckrc`, the
`[*.sh]` editorconfig block, and shrink the shell-suite floors to zero. The
Rust CLI inherits the visualiser's existing Rust toolchain checks (clippy,
rustfmt, cargo test).

## Code References

- `tasks/lint/scripts.py:18-51` — `SHELL_LIBRARIES` (the authoritative
  sourced-only/entrypoint partition; ~30 libraries).
- `tasks/lint/scripts.py:99-147` — exec-bit invariant guard.
- `tasks/shared/sources.py:60-100` — `shell_sources()` discovery (feeds all
  shell tasks).
- `tasks/test/helpers.py:17-44` — `run_shell_suites` discovery.
- `tasks/test/integration.py:14-26` — per-area floors.
- `scripts/lint-bashisms.sh:40-60` — 8 banned bash-4 constructs.
- `scripts/atomic-common.sh:82-163` — mkdir-lock + PID-owner reclaim.
- `scripts/vcs-common.sh:177-272` — `classify_checkout` (load-bearing arm
  order).
- `scripts/config-common.sh:160-316` — frontmatter writeback + array parse.
- `scripts/config-read-value.sh:43-124` — hand-rolled awk YAML reader.
- `skills/config/migrate/scripts/interactive-lib.sh:421-954` — FIFO IPC +
  watchdog + awk JSON parser.
- `hooks/hooks.json:1-44` — SessionStart + PreToolUse registration.
- `scripts/test-helpers.sh` — custom assertion harness.

## Architecture Insights

- **Filesystem-as-IPC philosophy**: phases communicate through `meta/`, and the
  shell library exists largely to read/write that state predictably. A Rust CLI
  must preserve the on-disk contract (paths, frontmatter format, JSONL records),
  not just function behaviour.
- **bash 3.2 floor as the migration's *motivation***: parallel arrays, literal
  fds, mkdir-locks, `$BASHPID` fallbacks, glob triples — every awkward idiom
  marks where the shell did extra work that Rust makes trivial. The floor is
  enforced by the bashisms linter (ADR-0016) and is the standing macOS-failure
  suspect.
- **No `jq` in the core library** — JSON is composed/escaped by hand
  (`jsonl-common.sh`) and parsed by hand in awk. `jq` is used only by the
  integration skills. Rust gets `serde_json` for free, eliminating the
  writer/reader escape-agreement hazard.
- **Clusters are loosely coupled** with a clear dependency spine: leaf utils →
  (config-reading, VCS detection) → parsers/path-resolution → atomic/JSONL →
  work-item sync + integrations → migration engine. This spine *is* the phasing
  order.
- **The exec-bit + floor machinery is itself a migration liability**: it must be
  decremented carefully in lockstep, or CI floors fail green-to-red as suites
  retire.

## Historical Context

- `meta/work/0136-migrate-shell-scripts-to-rust-cli.md` — the epic (this
  research's parent). Open questions: which scripts migrate first; how the CLI
  is distributed.
- `meta/notes/2026-06-22-ideas-backlog.md` — single-line origin ("Migrate shell
  scripts into Rust CLI").
- **Distribution precedent (the epic's stated model)**:
  - `meta/specs/2026-04-17-meta-visualisation-design.md` (decisions D8) — single
    static binary per arch, fetched from GitHub Releases on first use, SHA-256
    verified against committed `bin/checksums.json`; end users need no toolchain.
  - `meta/plans/2026-04-30-meta-visualiser-phase-12-packaging-docs-and-release.md`
    — release mechanics: `compute_sha256`, `update_checksums_json`,
    `validate_version_coherence`, env→config→cached→download resolution order.
  - `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md`
    — GitHub Releases pipeline + checksum manifest + `launch-server.sh` download
    path.
- **Invocation-contract precedent**:
  - `meta/work/0106-invoke-plugin-scripts-by-bare-path.md` — the bare-path rule.
  - `meta/work/0107-lint-skill-body-script-invocations.md` — the lint guard that
    encodes the contract (must update alongside any path/shape change).
  - `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`
    — RCA proving `bash`-prefix breaks the permission match.
  - `meta/research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md`
    — enumerates the `config-*`/`artifact-*` bare-path call sites.
- **Existing shell-library inventory**:
  - `meta/research/codebase/2026-06-09-0098-repo-wide-linting-formatting-static-analysis.md`
    — prior inventory of ~160 `.sh` scripts + the shellcheck/shfmt/bashisms
    tooling. The baseline this research extends.
  - `meta/work/0098-repo-wide-linting-formatting-static-analysis.md`.
- **Subsystem ADRs / tickets**: `meta/decisions/ADR-0016-userspace-configuration-model.md`,
  `ADR-0017-configuration-extension-points.md`,
  `meta/work/0020-vcs-abstraction-layer.md`, `0024-configuration-system-architecture.md`,
  `0016-plugin-packaging-and-scope.md`.
- **Concurrency-flake history** (relevant to porting the lock):
  the atomic-jsonl PID-owner reclaim fixed CI flakes under parallel load — the
  lock semantics are load-bearing, not incidental.

## Related Research

- `meta/research/codebase/2026-06-09-0098-repo-wide-linting-formatting-static-analysis.md`
- `meta/research/codebase/2026-03-16-jujutsu-integration-and-vcs-autodetection.md`
- `meta/research/codebase/2026-03-14-plugin-extraction.md`
- `meta/research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md`
- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md`

## Open Questions

1. **Distribution shape**: one combined `accelerator` CLI binary with
   subcommands, or per-script shims pointing at one binary? The bare-path
   contract favours per-name executable shims (symlinks/wrappers) over rewriting
   every call site — but symlink behaviour under the plugin install + the
   `allowed-tools` glob match needs validating.
2. **Shim vs rewrite decision** determines whether 0106/0107 and every SKILL.md
   `allowed-tools` glob must change. This is the single biggest scoping fork.
3. **Hook protocol**: do the SessionStart/PreToolUse hooks migrate in the same
   binary, or stay shell longest (they're small, isolated, and already work)?
4. **Migration engine last?** Its FIFO/watchdog/awk-JSON machinery is the
   highest-risk port; confirm it can be deferred to a final phase without
   blocking earlier clusters.
5. **Test-suite strategy**: re-port each shell `test-*.sh` to Rust `cargo test`,
   or keep the shell suites running against the Rust binary as black-box
   integration tests during transition? The latter preserves the oracle while
   internals change.
6. **Coverage gaps to close before/while porting**: ~14 `work-item-*` scripts
   and several `skills/design/` scripts (`scrub-secrets.sh`, `resolve-auth.sh`,
   `gap-metadata.sh`, `inventory-metadata.sh`, `audit-cue-phrases.sh`) have **no
   dedicated test suite** — porting these is riskier without an oracle.
7. **Floor management**: how to decrement the Python suite-count floors
   (`tasks/test/integration.py`) and shrink `SHELL_LIBRARIES` safely as clusters
   retire, without a green-to-red CI gap.
8. **`jq`/`curl` runtime deps**: the integration skills currently shell out to
   `jq`/`curl`; a Rust client removes those `allowed-tools` entries — confirm no
   other skill relies on them.
