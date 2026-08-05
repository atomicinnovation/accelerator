---
type: codebase-research
id: "2026-08-02-0188-library-backed-vcs-adapter"
title: "Research: Library-Backed VCS Adapter over gix and jj-lib (0188)"
date: "2026-08-02T21:19:48+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0188"
parent: "work-item:0188"
relates_to: ["codebase-research:2026-07-29-0169-vcs-subdomain-and-hooks-migration"]
topic: "Library-backed VCS adapter over gix and jj-lib: enforcement surfaces, test apparatus, fixture matrix, CI capability and sibling hand-offs"
tags: [research, codebase, vcs, vcs-adapters, gix, jj-lib, cargo-deny, cargo-pup, ci, fixtures]
revision: "8f64589b3fb56512f7b0a55af5209ffa1babfaa3"
repository: "accelerator"
last_updated: "2026-08-03T06:52:09+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added the gix 0.85 API spike (2026-08-03) closing the three unevidenced taxonomy queries plus the boundary and scrub riders"
schema_version: 1
---

# Research: Library-Backed VCS Adapter over gix and jj-lib (0188)

**Date**: 2026-08-02 21:19 UTC
**Author**: Toby Clemson
**Git Commit**: `8f64589b3fb56512f7b0a55af5209ffa1babfaa3`
**Branch**: detached (`HEAD`); revision unpushed, so references below are local
paths rather than permalinks
**Repository**: accelerator

## Research Question

Comprehensive codebase research for the story at
`meta/work/0188-library-backed-vcs-adapter.md` — a library-backed
implementation of the `vcs` crate's ports over `gix` and `jj-lib`, plus six
inherent taxonomy queries and the test apparatus proving in-process reads.

## Summary

The dependency work is **entirely unstarted** — `gix`, `jj-lib` and `uluru`
appear nowhere in `cli/Cargo.lock`, `cli/deny.toml`, `cli/pup.ron` or
`cli/Cargo.toml`. The only landed prerequisite is the `mise.toml` `jj` pin at
0.43.0 with its lockstep comment, which is present and correct
(`mise.toml:12-16`).

Eight findings materially change how the story should be planned:

1. **The CI open question is answerable, and the answer changes the
   criterion.** All Linux jobs are GitHub-hosted, non-containerised
   `ubuntu-latest` VMs where the runner user has passwordless sudo — so
   shadowing `/usr/bin/git` is feasible. But **`jj` is not a system binary in
   CI at all**: it is mise-installed under
   `$HOME/.local/share/mise/installs/jj/0.43.0/…` (`mise.lock:89-106`).
   Shadowing `/usr/bin/jj`, `/usr/local/bin/jj`, `/opt/homebrew/bin/jj` is a
   **no-op on the runner**. The shadow list as written in the work item is
   half-vacuous.

2. **The cargo-pup two-clause rule is unexercised in-repo, and the grouped-
   import gotcha bites the new module.** No shipped rule sets both
   `allowed_only` and `denied` (`cli/pup.ron`, six rules, all pure-permit or
   pure-deny). Separately, `allowed_only` matches *literal use-path text*, and
   a braced `use std::path::{Path, PathBuf}` resolves to an empty module name
   that matches no permit pattern — which is exactly what
   `cli/vcs-adapters/src/lib.rs:13-14` writes today.

3. **There is no cross-crate test-fixture precedent anywhere in the `cli/`
   workspace** — zero path-based `[dev-dependencies]`, no `test-support` crate,
   no `#[cfg(feature = "test-fixtures")]` module. Sharing is done today by
   *duplicating* `tests/common/mod.rs`. 0188 establishes a new convention.

4. **The "no `[features]` beyond `bash-parity`" criterion may forbid the
   natural implementation of the shared fixture**, and CI's `--all-features`
   would turn any new feature on workspace-wide regardless.

5. **The musl cross-compile is release-only and runs only on macOS**;
   `_assert_static_elf` is never exercised by `mise run check` or by any test.
   Adding the reference artefact to that path is a release-pipeline change.

6. **The fixture matrix is emptier than the work item assumes.** A git
   submodule fixture exists nowhere in the repo, in any language. The shell
   "colocated" fixture is a hand-graft — not `jj git init --colocate` — and a
   real colocated main repo classifies as `main`, not `colocated`. The shell's
   "main jj workspace" builder almost certainly produces a *colocated* repo, so
   there is no pure-jj precedent either.

7. **`gix` API evidence for three of the six queries is zero.** The 2026-07-29
   probe ran exactly one gix entry point (`gix::discover`); the word
   "submodule" does not appear in that research at all.

8. **No benchmarking machinery exists** — no criterion, no hyperfine, no
   `bench:*` task, no timing helper. `B = 35.1 ms` comes from 0186's table, not
   0169; the "~41 ms warm bootstrap" is derived (`149.1 − 107.9`), not measured.

## Detailed Findings

### 1. The crates as they stand (what 0188 extends)

`cli/vcs` is 193 lines with **zero dependencies** — `VcsKind`
(`cli/vcs/src/lib.rs:14-17`), `RepoFacts { root, name, kind, revision }`
(`:38-43`), `trait RepoRoot { discover, repository_root }` (`:46-57`),
`trait VcsProbe { kind, revision }` (`:60-67`), and the composition function
`facts(start, &dyn RepoRoot, &dyn VcsProbe)` (`:74-91`).

`cli/vcs-adapters` (323 lines) holds `MarkerWalkRoot`
(`cli/vcs-adapters/src/lib.rs:32`), `CommandProbe` (`:73`), the two subprocess
invocations (`:110-125`), the environment scrub (`:139-154`), the capped-stdout
poll loop (`:158-220`), and the hard-wired composition root:

```rust
pub fn facts(start: &Path) -> Option<RepoFacts> {
    vcs::facts(start, &MarkerWalkRoot, &CommandProbe::new())
}
```
(`cli/vcs-adapters/src/lib.rs:224-227`)

The single spawn chokepoint is `command.spawn()` at `:168`. The jj
secondary-workspace rule is already implemented by pure file reads with no
subprocess (`jj_repository_root`, `:57-68`) — so `jj-lib` is not strictly
needed for that one query, only for the taxonomy queries around it.

Two facts give 0188 latitude: `RepoFacts.root` and `.kind` currently have **no
production consumer** (only `.name` and `.revision` are read, at
`cli/corpus-adapters/src/metadata.rs:185-186`); and six of the crate's seven
unit tests construct `std::process::Command` directly inside
`#[cfg(test)] mod tests` (`:231`, `:254`, `:270`, `:279`, `:288`, `:297`,
`:307`), which is relevant to how the new pup deny clause is scoped.

### 2. Dependency policy machinery

**`cli/deny.toml`** (76 lines):

- **Zero `[[licenses.exceptions]]` entries exist today** — anywhere in the
  repo. The policy mandating their use is at `:38-40`; there is no worked
  example to copy a comment style from. The nearest in-file precedent is the
  `CC0-1.0` allow-entry comment at `:51` (crate name, role in parentheses,
  why). The house style of citing a work-item *path* exists in `mise.toml:15`,
  not in `deny.toml`.
- `multiple-versions = "warn"` at `:57` — the work item's claim is **verified**.
  This is why the single-`gix`-version invariant must be asserted directly.
- `unmaintained = "all"` at `:22` — the strictest setting. Two large transitive
  trees entering `advisories` scope under this setting is a materially bigger
  ongoing cost than the work item's "a future RustSec advisory" wording
  suggests: an *unmaintained* crate anywhere in the `gix`/`jj-lib` closure also
  fails the workspace-wide check.
- `[graph].targets` lists five triples (`:4-17`) including both musl targets —
  the new trees must resolve on all five.
- `deny = [{ crate = "serde-saphyr", wrappers = ["document"] }]` (`:69`) is the
  existing precedent for "banned except when reached through crate X".
- Runner: `tasks/deny.py:13-18`, `cargo deny check advisories licenses bans
  sources` with cwd = `cli/`.

**`cli/Cargo.lock`**: committed, `version = 4`, **358** packages. Verified
absent: `gix`, any `gix-*`, `jj-lib`, `openssl`, `openssl-sys`, `native-tls`,
`curl`, `git2`, `libgit2-sys`. Present: `rustls 0.23.41`, `ring 0.17.14`,
`hickory-resolver 0.25.2`, `webpki-roots 1.0.8`. **`zstd 0.13.3` is already in
the graph** via `include-flate-compress` (`:1420-1428`) — `jj-lib` also pulls a
zstd, so this is a live `multiple-versions` warn candidate the plan should
anticipate.

**`cli/Cargo.toml`**: `resolver = "3"` (`:3`), `rust-version = "1.90.0"`
(`:9`), `edition = "2021"` (`:8`). Pinning convention is established and
explicit: exact `=` pins for behaviour-sensitive crates, each carrying a
comment justifying the pin (`clap = "=4.6.1"`, `reqwest = "=0.12.28"`,
`rustls = "=0.23.41"`, `serde-saphyr = "=0.0.29"`). **`vergen` /
`vergen-gitcl` at `:17-21` is the existing precedent for a matched-pair exact
pin with a comment explaining the coupling** — the shape 0188's `gix`/`jj-lib`
pins should copy.

`[workspace.lints]` (`:56-70`): `warnings = "deny"`, clippy `pedantic` +
`nursery` at warn with `-D warnings`, plus `unwrap_used`/`expect_used`/`panic`
opt-ins. Every new line of adapter code is held to this.

**MSRV is hand-duplicated in three places** — `mise.toml:8`,
`cli/Cargo.toml:9`, `cli/clippy.toml:1` — guarded by
`tests/unit/tasks/test_msrv_coherence.py:30-38`. **That file is the exact shape
0188's `mise.toml` jj-pin ↔ `jj-lib`-pin lockstep assertion should take**: a
bare pytest under `tests/unit/tasks/`, no invoke task, 39 lines.

### 3. The cargo-pup rule — two unmodelled problems

`cli/pup.ron` has six `Module` lints, all `RestrictImports(allowed_only,
denied, severity)`. Scoping is by **resolved module path regex**, never file
glob. Single-module scoping precedent exists (three of six):
`^accelerator::version::core($|::)` (`:14`), `^accelerator::launch::core($|::)`
(`:28`), `^accelerator::config_command($|::)` (`:101`). The `vcs` domain rule
is `^vcs($|::)` (`:76`) — the `($|::)` anchor means **`vcs_adapters` is
currently unruled**.

**Problem A — the combined rule is unexercised.** `allowed_only` and `denied`
are independent `Option` fields on the same variant, so structurally they can
coexist, but **no shipped rule sets both**, and the pup integration tests
(`tests/integration/pup/test_import_rule.py`) do not cover the combination.
Which list wins on overlap (`std` permitted, `std::process` denied) is
unestablished in-repo. This is the mechanism 0188 relies on to make zero-spawn
*structural*; it needs an empirical check in planning, exactly like the oracle
mapping.

**Problem B — the grouped-import gotcha.** `cli/pup.ron:90-98` records that
cargo-pup resolves a grouped `use foo::{a, b}` to an **empty module name**,
which an `allowed_only` rule rejects as unpermitted. Consequence: every module
under a permit rule must write one single-item `use` per import — visible at
`cli/config/src/service.rs:4-13` (ten consecutive single-item lines) and
`cli/config/src/level.rs:3-4` (splitting `std::fmt::{Display, Formatter}`).
The current adapter writes the forbidden shape:

```rust
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
```
(`cli/vcs-adapters/src/lib.rs:13-14`)

So the new library-backed module must adopt single-item imports throughout.
Note `#[cfg(test)]` modules appear exempt in practice (`cli/vcs/src/lib.rs:95,
97` use grouped imports under a whole-crate rule).

The narrowed-permit precedent to copy is `"^kernel::Error(::|$)"`
(`cli/pup.ron:18`), proven discriminating by
`tests/integration/pup/test_import_rule.py:196-205`. Failure-message contracts
are `"is not allowed"` (permit violation) vs `"is denied"` (deny violation)
(`:303`, `:486`) — useful for the non-vacuity demonstration's assertion.

Runner: `tasks/pup.py:16-25`, `cargo +nightly-2026-01-22 pup` from `cli/`;
nightly + cargo-pup version are a matched pair pinned at
`tasks/shared/rust.py:6-7`, rustup-managed, deliberately not a mise tool.

### 4. The behavioural oracle and the fixture matrix

`classify_checkout` (`scripts/vcs-common.sh:177-280`) emits the six-line
KEY=VALUE record. The arm cascade is at `:240-272`, first-match-wins, with
`colocated` deliberately preceding the `nested-*` arms.

The only real detection oracle suite is `hooks/test-vcs-detect.sh` (713 lines,
42 cases, ~131 assertions). Its **recorded** oracle mapping:

| Fixture | KIND | BOUNDARY | JJ_PARENT | GIT_PARENT | Source |
| --- | --- | --- | --- | --- | --- |
| main jj workspace | `main` | `""` | `""` | `""` | `:338-345` |
| main git checkout | `main` | `""` | — | — | `:347-352` |
| jj secondary | `jj-secondary` | secondary | jj parent | `""` | `:354-360` |
| git linked worktree | `git-worktree` | worktree | `""` | git parent | `:362-368` |
| colocated (grafted) | `colocated` | target | jj parent | git parent | `:370-376` |
| nested-jj-in-git | `nested-jj-in-git` | target | jj parent | git parent | `:596-603` |
| nested-git-in-jj | `nested-git-in-jj` | target | jj parent | git parent | `:605-611` |
| plain dir | `none` | `""` | — | — | `:378-382` |
| bare git repo | `none` | (unasserted) | — | — | `:384-385` |

Secondary oracles (`:258-314`, `:424-480`): `find_git_main_worktree_root`
returns **exit 1 + empty stdout for a bare repo** (`:308-314`) and the parent
for a linked worktree; `find_jj_main_workspace_root` returns own root for main,
`$FIXTURE_PARENT` for secondary, exit 1 for a plain dir.

**Three fixture findings that change the matrix's cost and meaning:**

- **A git submodule fixture exists nowhere in the repo** — in shell, Rust or
  Python. `grep -rl submodule --include='*.sh'` returns nothing;
  `scripts/vcs-common.sh:124` is the only mention, in a comment. Entirely new
  construction, and the only shape whose oracle
  (`git rev-parse --show-superproject-working-tree`) has never been run here.
- **The shell "colocated" fixture is a hand-graft, not `jj git init
  --colocate`** (`hooks/test-vcs-detect.sh:96-157`). It is a *jj secondary
  workspace whose path is simultaneously a git linked worktree of an unrelated
  git repo*, assembled by moving `.jj/` into a `git worktree add` target and
  rewriting `.jj/repo` with an absolute path and no trailing newline. Because
  `classify_checkout`'s `colocated` arm requires `jj_secondary==1 &&
  git_worktree==1` (`scripts/vcs-common.sh:242-247`), a genuine
  `jj git init --colocate` main repo classifies as **`main`**, not
  `colocated`. 0188's matrix row "colocated | root" is ambiguous between the
  two shapes and must be disambiguated in planning.
- **There is probably no pure-jj fixture anywhere.** The golden snapshot
  `hooks/test-fixtures/vcs-detect/main-jj-workspace.json:4` reads
  `mode: jj-colocated`, and `hooks/vcs-detect.sh:28-33` emits that only when
  `.git` is a *directory* — yet the fixture that produced it is a bare
  `jj git init --quiet` (`regenerate.sh:36`). So `jj git init` appears to be
  colocated-by-default. 0188's pure-jj measurement fixture must explicitly
  assert `.git` is absent.

The one `--colocate` builder in the tree is
`scripts/test-metadata-helpers.sh:41-55`, which is also **the only hermetic-env
suite in the repo** (`:28-40`: unsets `GIT_DIR GIT_WORK_TREE JJ_CONFIG`, sets
`HOME` and `XDG_CONFIG_HOME` to temp dirs). `hooks/test-vcs-detect.sh` by
contrast scrubs almost nothing — only `GIT_CEILING_DIRECTORIES="$TMPDIR_BASE"`
(`:35-40`) and per-repo git identity (`:58`). The developer's `~/.gitconfig`
and `~/.jjconfig.toml` are in effect for every jj fixture it builds.

The PATH-stripping technique 0188's harness can borrow is
`strip_binary_from_path` (`hooks/test-vcs-detect.sh:164-188`), carrying two
recorded gotchas: macOS provides `git` in both `/opt/homebrew/bin` **and**
`/usr/bin` so stripping one dirname is insufficient; and `type -p` is served
from bash's command hash table and caused a **hard hang on macOS bash 3.2**.
The subshell-wrapper scoping idiom (`:387-400`) and the
`assert_eq "PATH not leaked"` guard (`:680`, `:708`) are both worth copying.

### 5. Test apparatus — what exists and what is new

**Injection seam.** `cli/vcs-adapters/tests/detection.rs` is
`#![cfg(feature = "bash-parity")]` (`:9`) and calls `vcs_adapters::facts`
directly at `:95`, `:121`, `:134`, `:155`, `:191`, `:234`, `:272` — no
injection point. Existing shapes: plain git, nested subdir, no-commits,
colocated (`:144-169`), jj secondary (`:171-209`), `.git`-as-file worktree
(`:211-248`), bare (`:250-277`).

**Reference artefact.** Three `[[bin]]`-under-`tests/fixtures/` precedents
exist: `accelerator-fixture` (`cli/launcher/Cargo.toml:17-21`),
`document-fixture` (`cli/document/Cargo.toml:12-14`),
`config-adapters-fixture` (`cli/config-adapters/Cargo.toml:12-17`). The last is
the best template — it is a composition root that calls the real API and
*prints* the result
(`cli/config-adapters/tests/fixtures/config_adapters_fixture.rs:1-49`). Every
one carries the same required prelude:

```rust
#![allow(clippy::exit, clippy::print_stdout, clippy::print_stderr,
         clippy::restriction)]
```

No `required-features`, no `[[example]]`, no `#[cfg(test)]` binary anywhere in
`cli/`.

**Cross-crate sharing — no precedent at all.** Zero path-based
`[dev-dependencies]` in the whole `cli/` workspace; every dev-dep table holds
only external crates. `corpus-adapters` has no `[dev-dependencies]` table at
all and pulls `tempfile` as a *runtime* dependency
(`cli/corpus-adapters/Cargo.toml:28`). Today's sharing mechanism is duplicating
`tests/common/mod.rs` per crate with `#![allow(dead_code)]`.

Two hard constraints on the shared fixture's shape:

- **`CARGO_BIN_EXE_*` does not cross crate boundaries.** Recorded verbatim at
  `cli/launcher/Cargo.toml:17-18` and worked around at
  `cli/corpus-adapters/tests/common/mod.rs:62-86` by deriving the path from
  `current_exe()`. If the reference artefact is a `[[bin]]` in `vcs-adapters`,
  `corpus-adapters` cannot locate it via the env var — the fixture's public API
  must expose a path resolver.
- **A feature gate conflicts with an acceptance criterion.** The natural route
  is `vcs-adapters/[features] fixtures = []` plus
  `vcs-adapters = { path = "…", features = ["fixtures"] }` in a *new*
  `[dev-dependencies]` table on `corpus-adapters` (resolver 3 keeps dev-dep
  features out of the normal build, which matters because
  `cli/visualiser/server` depends on `corpus-adapters`). But 0188's criterion
  says `vcs-adapters` "gains no `[features]` entry beyond `bash-parity`"
  (work item `:320-321`). And CI runs `--all-features`
  (`tasks/test/cli.py:11-14`), so any such feature is on workspace-wide during
  the CI test run regardless. **Resolve in planning.**

**Temp dirs and path comparison** — solid, reusable as-is. The builder shape is
`tempfile::Builder::new().prefix("<crate>-<label>-").tempdir()` returning the
owned guard (`cli/vcs-adapters/tests/detection.rs:32-39`); struct-owned
`_guard` variants at `cli/launcher/tests/config_read.rs:22-40` and `:79-90`.
Canonicalise *immediately* after building, then compare against the
canonicalised value (`detection.rs:92-97`); for a nested path, canonicalise the
parent then join (`:179-180`).

**Parallelism** — nextest process-per-test is the isolation mechanism
(`tasks/test/cli.py:31-35`). There is **no `nextest.toml` anywhere in the
repo**, no `serial_test`, and not a single `std::env::set_var`/`remove_var`
under `cli/`. The convention is per-child `Command::env` overlays. If the
zero-spawn suite needs serialisation, that config file is new.

**Non-vacuity precedent** — `the_scrubbed_environment_drops_the_redirecting_variables`
(`cli/vcs-adapters/src/lib.rs:305-321`) is the closest existing shape to 0188's
"unscrubbed control must diverge" criterion. The spy-port template for "must
never fire" is `cli/launcher/tests/crypto_provider.rs:33-55` (`Cell<bool>` spy
plus a panicking null).

### 6. Build system and enforcement wiring

**`_assert_static_elf`** (`tasks/build.py:132-159`) shells out to `file -b` and
accepts three phrasings via `_is_statically_linked` (`:118-129`). Its **sole
production caller** is `cli_cross_compile` (`:312-331`), guarded by
`if "musl" in triple:` at `:329`, over `_CLI_RELEASE_BINARIES = ("accelerator",
"accelerator-verify")` (`:37`).

**The musl cross-compile is release-only and macOS-only.** `cli_cross_compile`
runs `cargo zigbuild --release --target {triple}` and is invoked from
`tasks/release.py:82-94` / `:109-125`, wired to `mise.toml:512-515` / `:525-528`
and the `prerelease`/`release` jobs, both `runs-on: macos-latest`
(`.github/workflows/main.yml:343`, `:464`). **No `check-*` job cross-compiles,
and no test exercises `_assert_static_elf` against a real build.** Adding the
reference artefact to that path means editing the staging loop at `:326-331`
and the constants at `:37` — a release-pipeline change with its own risk, not a
test addition.

Toolchain: `cargo zigbuild`, with `ziglang` and `cargo-zigbuild` as **PyPI deps
in the `build` group** (`pyproject.toml:20-21`), not mise tools. No
`.cargo/config.toml` linker wiring exists.

**Lockfile-assertion precedent** — three shapes, none a standing invariant
check:

- `_cargo_lock_package_block` / `_strip_lock_dependencies`
  (`tasks/build.py:341-354`, `:397-412`) feeding `vendor_shim_marker_digest`
  (`:415-459`) — a *digest-drift* guard scoped to `accelerator-verify` +
  `minisign-verify`. Adding deps to `vcs-adapters` cannot trip it.
- `tests/integration/deny/test_launcher_feature_graph.py` — `cargo tree -e
  features -p accelerator` with `_PRESENT`/`_ABSENT` crate lists matched by a
  `(?<![\w-]){crate} v\d` regex (`:23-31`, `:48-51`). **This is the nearest
  architectural precedent for 0188's "exactly one gix version / no TLS stack"
  assertion**, and it already lives in `test:integration:deny`.
- `tests/unit/tasks/test_msrv_coherence.py` — hand-duplicated pin coherence as
  a bare pytest. **The exact shape for the `mise.toml` jj-pin ↔ `jj-lib`-pin
  lockstep clause.**

**Adding a leaf task** — module under `tasks/lint/` with a thin `@task` wrapper
over a pure function; export in `tasks/lint/__init__.py:1-27`; register in
`tasks/__init__.py:102-104` (underscores auto-dash); mise leaf in `mise.toml`
(`:` where invoke uses `.`). Roll-up membership is the decision point: a
cli-scoped guard joins **both** `cli:check.depends` (`mise.toml:422-424`) and
`lint:check.depends` (`:464-466`), because the bare `default` task depends on
`lint:check` but not on `check`. A standalone entity gate joins
`check.depends` directly (`:480-482`), as `deny:check` and `pup:check` do.
Pure guards with no autofixer stay out of `fix` entirely.

**`tests/unit/tasks/test_mise.py` pins the topology with exhaustive equality
assertions** — `_CHECK_GATES` (`:18`), `_CLI_CHECK_GATES` (`:25-29`), and for
any new `test:integration:*` task, `_LAUNCHER_DEPENDENTS`/`_NO_LAUNCHER_NEEDED`
(`:36-56`) plus the roll-up or `_NOT_IN_INTEGRATION_ROLLUP` (`:127-137`). These
fail if skipped.

**`tasks/README.md`** — a new gate documents in the entity-gate paragraph
(`:38-42`, which currently *enumerates* "`deny:check` … and `pup:check`" and
will go stale), a new `### <name>` subsection alongside "Executable-bit
invariant" (`:67`) and "Rust nightly lane" (`:101`), and a row in the CI table
(`:153-157`).

**Test invocation**: `cargo nextest run --manifest-path cli/Cargo.toml
--workspace --exclude accelerator-visualiser --all-features`
(`tasks/test/cli.py:11-14`, `:31-38`), with `cargo llvm-cov nextest` when
coverage is on. `bash-parity` is enabled **implicitly by `--all-features`** —
there is no explicit `--features bash-parity` anywhere.

### 7. CI capability — answering the open question

There is exactly one workflow file, `.github/workflows/main.yml` (564 lines, 14
jobs). Every Linux job is a bare `ubuntu-latest` label; **no `container:` key
exists anywhere**; **no `sudo` appears in `.github/`, `tasks/`, `scripts/`,
`hooks/` or `cli/`** (repo-wide grep — the only hits are substrings in
design-prototype HTML).

**Feasibility: yes, on the existing runners.** GitHub-hosted Ubuntu VMs run as
the `runner` user with passwordless sudo, so `sudo mv`, `sudo ln -sf` or
`sudo mount --bind` on `/usr/bin/git` are all available without a privileged
container. Corroborating in-repo evidence: `test-visual-regression`
(`main.yml:122-145`) already runs `docker run` with bind mounts and
`--ipc=host` from a step (`tasks/test/e2e.py:60-81`), which implies effective
root on the host; and every cargo job already writes outside the workspace into
`$HOME` via `$GITHUB_ENV` (`main.yml:31`, `:71`, `:198`, `:237`, `:270`,
`:295`). The residual risk is that this is **unproven in-repo** — the first run
is itself the experiment, and it is cheap to settle with a one-step smoke test.

**But the shadow list is wrong for the actual CI layout.** On `ubuntu-latest`,
`git` is at `/usr/bin/git`, but `jj` is installed by mise from an aqua-backed
GitHub release into `$HOME/.local/share/mise/installs/jj/0.43.0/…`
(`mise.toml:16`, `mise.lock:89-106`). There is no `/usr/bin/jj`,
`/usr/local/bin/jj` or `/opt/homebrew/bin/jj` on the runner at all — shadowing
them proves nothing. The meaningful jj shadow target in CI is the mise install
path (or the shim), and the criterion should say so.

**Where the job goes.** `test-unit` and `test-integration` are OS matrices
(`main.yml:20-21`, `:60-61`), so anything placed there also lands on
`macos-latest` where the strong form is impossible under SIP. Critically,
**`test:unit:cli` runs on both legs**, and `detection.rs` is the natural home
for the query matrix — so a strong-form assertion placed there would fail on
macOS. `check-cli` (`:223`), `check-supply-chain` (`:259`) and
`check-architecture` (`:286`) are the Linux-only single-runner template;
`check-architecture` is the closest precedent for "one Linux-only job that
self-provisions unusual infrastructure and is deliberately isolated".

**Constraints on a new job**, from `tests/unit/tasks/test_workflows.py`:

- The nightly-isolation invariant (`:287-337`) asserts the set of jobs whose
  `run:` text contains `pup:check`, `deps:install:pup` or `+nightly` is
  **exactly `{check-architecture}`**, and that no job outside
  `{check-architecture}` ∪ the three release jobs declares
  `needs: check-architecture`.
- No step may reference `ACCELERATOR_RELEASE_SECRET_KEY` unless its `name`
  starts with `"Sign"` (`:115-124`).
- Renaming `check-cli` breaks the guard's own mutation tests (`:346`, `:351`).
- **actionlint lints only `main.yml`** — `_WORKFLOW = ".github/workflows/main.yml"`
  (`tasks/lint/workflows.py:7`). A separate new workflow file would not be
  linted at all. Put the job in `main.yml`.

**mise in CI**: `jdx/mise-action` SHA-pinned, always `install: true, cache:
true, experimental: true`. There is **no explicit `mise install` step
anywhere** — the jj bump is picked up because mise-action keys its cache on
`mise.toml`/`mise.lock` contents. The `RUSTUP_HOME` routing comment
(`main.yml:191-198`) is now replicated to six jobs and explains why: a cache
hit makes `mise install` a no-op while the toolchain is absent, and parallel
cargo tasks race to auto-install. A new cargo job needs the same routing step
plus its own `cache_key_prefix`.

**Who needs a real `jj`**: `test:integration:hooks` (`hooks/test-vcs-detect.sh:11-16`
hard-requires `jj git realpath jq`, exit 77 if absent — and `run_shell_suites`
treats 77 as a *failure*, not a skip) and `test:unit:cli` (`detection.rs:5`).
Both run on both OS legs.

### 8. Measurement — no machinery, and the provenance of B

Searched the whole repo: **no criterion, no `[[bench]]`, no `benches/`, no
hyperfine, no `bench:*` mise namespace, no timing helper in `tasks/`.**
`tasks/shared/clock.py` is a 12-line injectable clock for poll loops, not a
measurement facility. The only timing code anywhere is an upper-bound
assertion at `cli/vcs-adapters/src/lib.rs:257-265`. 0188's "median of 20,
library init / warm in-process / cold per-process" has zero existing machinery
to hook into — the runner, the timing loop and the results convention are all
new.

**`B ≈ 35 ms` is `35.1 ms`**, from 0186's table
(`meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md:46-63`, also
reproduced at research `2026-07-29-…:622-630`): darwin-arm64, warm cache, 20
iterations, 2026-07-30. The same table gives `bin/accelerator version` at
149.1 ms, launcher-direct at 3.0 ms, minisign verify of the 8 MB launcher at
2.3 ms, and `probe_dir` write+chmod+exec+rm at **107.9 ms**.

**The "~41 ms warm bootstrap" is derived, not measured**: `149.1 − 107.9`,
i.e. the bootstrap with only 0186's `probe_dir` fix landed and the ~23 ms shim
double-hash retained. 0186 states this explicitly at `:61-63`. The plan should
record both provenances rather than citing 41 ms as an observation.

**No gix or jj-lib timing exists at all.** And the platform caveat matters:
research `:688-690` records that the ~97 ms probe delta is macOS-specific
(first-exec check), with no Linux equivalent — so darwin is the worst case and
cross-host comparison is unsound. Review-2 pass 3 (`:652-654`) left exactly
this open: a threshold calibrated on one host against a baseline captured on
another is not well-posed. **0188 measuring on a different host from 0169's `B`
would leave the `G ≤ 1.1 × B` gate ill-defined even though 0188 itself gates
nothing.** Record host and OS, and state whether they match 0186's.

### 9. Consumer side — `cli/corpus-adapters`

Exactly four references to `vcs`/`vcs_adapters` exist in the crate:
`Cargo.toml:24-25` (both **normal `[dependencies]`**),
`src/metadata.rs:14` (`use vcs::RepoFacts;`) and `src/metadata.rs:201`
(`let facts = vcs_adapters::facts(start);` — the only call site, inside
`derive_at`). Only `.name` and `.revision` are read (`:185-186`).

**0188 needs to change nothing here.** Because the crate reaches the free
function rather than naming the adapters, leaving `vcs_adapters::facts`
hard-wired keeps the pair in place automatically.

**The metadata parity suite is `tests/metadata.rs`, not `tests/parity.rs`.**
The specific test is `derive_at_agrees_with_the_live_metadata_helper`
(`:265`, gated at `:263`); it runs `scripts/artifact-derive-metadata.sh`
against **the live accelerator checkout** (`repo_root()`), compares
`Current Revision:` and `Repository Name:`, and asserts the bash-side name is
literally `"accelerator"` as an anti-vacuity guard (`:299-303`).
`tests/parity.rs` is the *linkage* parity suite and never touches VCS. Naming
the wrong file would pin the wrong suite.

**Two traps for the cross-crate zero-spawn test:**

- `SystemClock::try_new()` **spawns `date` unconditionally**
  (`src/metadata.rs:106-110`, rationale at `:96-98`), reached by three ungated
  tests. A blanket "no subprocess" marker would trip on it — the assertion must
  be scoped to `git`/`jj` specifically.
- The dependency edge already exists as a **normal** dependency, so a
  `corpus-adapters` integration test can `use vcs_adapters::…` today with no
  manifest change. Adding `features = ["fixtures"]` to that existing
  `[dependencies]` entry would leak into the production build (and into
  `cli/visualiser/server`, which depends on `corpus-adapters`); a new
  `[dev-dependencies]` table is the safe route under resolver 3.

No test in the crate builds a git or jj repo, manipulates PATH, or asserts
"no spawn". Everything the criterion needs is new here — which is precisely
what makes it meaningful rather than a formality.

### 10. Sibling hand-offs — verified state

Every claim 0188 makes about its siblings checks out, with three additions:

- **0185 has a sixth stale site 0188 does not name.** Its References
  (`:148-153`) cite two anchors that no longer exist in 0169: there is no
  "Adapter-swap boundary" requirement heading, and the Dependencies bullet is
  titled "Unowned debt this story creates" (`0169:436`, no "or inherits"),
  whose item 1 is now about `vcs-common.sh` residue. Both should be repointed.
  Also, 0185's harness criterion (`:82-84`) describes *PATH stubs only* — the
  harness it inherits is 0188's strictly larger three-part strong form.
- **0125 reciprocates no edges.** Its `relates_to` is
  `["work-item:0124", "work-item:0058", "work-item:0020"]` (`:11`), while 0169
  (`:14`), 0185 (`:13`) and 0188 (`:14`) each already list it. Adding only
  `work-item:0188` leaves 0169 and 0185 one-directional — a deliberate
  decision, not an oversight, but worth making consciously. 0125 has a
  `## Dependencies` section at `:141-146` to append to, and no Validation
  Results or Open Questions sections.
- **0125's constraints 3, 4 and 5 are untouched by 0188** and must survive the
  note: caller audit required for changed submodule semantics (`:90-92`),
  `find_repo_root` ≠ `find_git_main_worktree_root` (`:93-94`), and `vcs_mode`
  is a command-set selector that must preserve `.jj`-WINS (`:95`). Only
  constraints 1 (no git/jj on PATH) and 2 (1-3 subprocesses per call) are what
  0188 dissolves.

Ground truth on the call sites: `find_repo_root` has 21 production call sites
outside `workspaces/*/`, of which only four (`hooks/vcs-detect.sh`,
`hooks/vcs-guard.sh`, `scripts/vcs-status.sh`, `scripts/vcs-log.sh`) are
retired by 0169. `vcs_mode` has exactly one
(`skills/work/scripts/work-item-file-dirty.sh`).

**0169's three silences are genuine** — defining a domain port over inherent
methods, widening the pup rule for `status`/`log`, and reusing 0188's named
pure-jj builder are absent from 0169, confirmed by exhaustive grep. They are
silences, not contradictions.

**Contention is minimal.** 0187 has not landed (`tasks/build.py:35` still holds
`_VISUALISE_SKILL_RELATIVE`, `:189` is unchanged) and its region (`:189-208`)
is ~130 lines from 0188's (`:118-159`, `:290-331`) with three unrelated
functions between. The only genuine collision surface is the import block
(`:11-32`) and the module constants (`:34-56`). 0168 is `done` and its
restructuring landed; `mise.toml` contention is already discharged.

**One unabsorbed correction**: `meta/prs/35-description.md:52` records that
0188's affected-suite list is incomplete. Four more sites build or read jj
fixtures: `scripts/test-metadata-helpers.sh`,
`skills/config/migrate/scripts/test-migrate.sh`, `…/test-migrate-interactive.sh`,
and Python task tests under `tests/unit/tasks/`. 0188 `:413-416` still names
only two.

## Code References

- `cli/vcs/src/lib.rs:46-67` — the two ports 0188 implements; untouched by the story
- `cli/vcs-adapters/src/lib.rs:13-14` — grouped imports the new pup rule would reject
- `cli/vcs-adapters/src/lib.rs:57-68` — jj secondary resolution already done by pure file reads
- `cli/vcs-adapters/src/lib.rs:139-154` — the production scrub list (no `HOME`/`XDG_CONFIG_HOME`)
- `cli/vcs-adapters/src/lib.rs:168` — the single spawn chokepoint `CommandProbe` retains
- `cli/vcs-adapters/src/lib.rs:224-227` — the hard-wired composition root that stays hard-wired
- `cli/vcs-adapters/src/lib.rs:305-321` — the non-vacuity template for the scrub control
- `cli/vcs-adapters/tests/detection.rs:9` — the `bash-parity` gate; `:32-39` the tempdir builder
- `cli/corpus-adapters/src/metadata.rs:106-110` — `SystemClock` spawns `date` unconditionally
- `cli/corpus-adapters/tests/metadata.rs:265` — the metadata parity test that must pass unchanged
- `cli/corpus-adapters/tests/common/mod.rs:62-86` — the `current_exe()` cross-crate binary derivation
- `cli/config-adapters/tests/fixtures/config_adapters_fixture.rs:1-49` — reference-artefact template
- `cli/config-adapters/Cargo.toml:12-17` — the `[[bin]]`-under-`tests/fixtures/` convention
- `cli/deny.toml:38-40` — the policy mandating `[[licenses.exceptions]]`; `:57` the warn-level bans
- `cli/pup.ron:90-98` — the grouped-import resolution gotcha; `:14`, `:28`, `:101` single-module scoping
- `cli/Cargo.toml:17-21` — the matched-pair exact-pin precedent (`vergen`/`vergen-gitcl`)
- `mise.toml:12-16` — the landed `jj = "0.43.0"` pin and its lockstep comment
- `scripts/vcs-common.sh:130-135` — the scrubbed branch; `:206-215` the unscrubbed one
- `scripts/vcs-common.sh:240-272` — the first-match-wins arm cascade
- `scripts/test-metadata-helpers.sh:28-55` — the only hermetic-env suite, and the only `--colocate` builder
- `hooks/test-vcs-detect.sh:96-157` — the hand-grafted "colocated" fixture
- `hooks/test-vcs-detect.sh:164-188` — `strip_binary_from_path` and its bash-3.2 hang post-mortem
- `hooks/test-fixtures/vcs-detect/main-jj-workspace.json:4` — evidence `jj git init` is colocated-by-default
- `tasks/build.py:132-159` — `_assert_static_elf`; `:312-331` its only caller
- `tasks/test/cli.py:11-14` — `--all-features` is what enables `bash-parity`
- `tests/unit/tasks/test_msrv_coherence.py:30-38` — the shape for the pin-lockstep assertion
- `tests/integration/deny/test_launcher_feature_graph.py:23-51` — the shape for the version invariants
- `tests/unit/tasks/test_workflows.py:287-337` — the nightly-isolation invariant a new job must satisfy
- `tasks/lint/workflows.py:7` — actionlint lints only `main.yml`
- `.github/workflows/main.yml:122-145` — the docker/bind-mount precedent on `ubuntu-latest`
- `.github/workflows/main.yml:286-321` — `check-architecture`, the Linux-only isolated-job template

## Architecture Insights

- **The hexagon boundary forces the design.** `cli/pup.ron:76` restricts the
  `vcs` domain to `std`/`kernel::Error`/`crate::`, so `gix` and `jj-lib` can
  only ever live in `vcs-adapters`. That is why the six queries must be
  inherent methods and why 0169's port has to be defined over plain domain
  values with no library types leaking in.
- **Enforcement is layered but uneven.** `cargo deny` and `cargo-pup` are
  standalone entity gates wired straight into top-level `check`, *not* into
  `cli:check` — so a `cli:check` inner loop will not catch a licence or import
  violation. The musl guarantee has an even bigger hole: it is only ever
  verified during release, on macOS.
- **The `-e` existence test is a deliberate, load-bearing choice.**
  `find_repo_root` (`scripts/vcs-common.sh:11`), `vcs_mode` (`:28`, `:30`) and
  `MarkerWalkRoot` (`cli/vcs-adapters/src/lib.rs:38`) all test existence so a
  `.git` *file* counts. 0188's boundary rule is written to preserve exactly
  this, which is why `MarkerWalkRoot` stays the reference implementation for
  `RepoRoot` even after the library-backed type lands.
- **Cross-crate test sharing is genuinely unmodelled here.** The workspace's
  answer to "two crates need the same helper" has always been "write it twice".
  0188 introduces the first shared fixture, and the two mechanisms the
  workspace already sanctions — a mirrored feature gate, and a `[[bin]]`
  located by path derivation — pull in different directions.
- **Closed lists written by inference have a poor track record here.** The
  review record shows the oracle mapping wrong twice and each zero-spawn
  mechanism found evadable in the *next* pass. 0188 contains three more closed
  lists — the six-query contract, the four TLS crate names, the three absolute
  shadow paths — and the third is already demonstrably wrong for CI (§7).

## Historical Context

- `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §9 — the feasibility probe. `cargo deny` passes `bans`/`advisories`/`sources`
  with one licence rejection (`uluru 3.1.0`, MPL-2.0, `gix-pack`'s LRU cache);
  `jj-lib` 0.43 no longer depends on `git2`/`libgit2-sys`; `gix` default
  features exclude network transports (compression resolves to `zlib-rs`); a
  binary calling `gix::discover` and jj-lib's loader cross-compiles to
  `aarch64-unknown-linux-musl` and reports `statically linked, stripped`,
  accepted by `_assert_static_elf` unmodified. §12 holds the latency table.
- Same document, §9 — the two traps. `gix::discover` walked up past a jj
  workspace boundary and returned the parent repo's `.git` from inside
  `workspaces/build-system`. `UserSettings` was abandoned after five
  successive panics (`user.name` → `operation.hostname` → `operation.username`
  → `debug.randomness-seed` → `signing.behavior` → …) — **the chain was never
  exhausted**, which is the empirical basis for the crate-wide prohibition.
- `meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md` pass 4
  (`:811-825`) — the split recommendation. 0188 is item 3 of four. Pass 3
  (`:720-725`) recorded its independent value as dissolving 0125's rationale.
  The zero-spawn escalation ladder runs pass 1 (semantic contradiction) → pass
  2 (an in-crate seam cannot see inside `gix`/`jj-lib`) → pass 3 (`PATH` stubs
  evaded by `/usr/bin/git`) — each rung reached only because the previous was
  found insufficient one pass later.
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md:46-63` — the
  measurement table that is the true provenance of `B = 35.1 ms`, and `:61-63`
  where the ~41 ms figure is derived rather than measured.
- `meta/prs/35-description.md:52` — an unabsorbed correction to 0188's
  affected-suite list.
- `meta/reviews/work/0188-library-backed-vcs-adapter-review-1.md` — verdict
  APPROVE; `:1195-1207` moved the 0125 note from 0169 to 0188; `:1053-1071`
  repointed the 0185 harness attribution; `:1860` is the origin of the open
  ordering question. Note `:554-556` suggests naming `tests/parity.rs` as the
  suite to keep green — **that is the wrong file** (see §9).

## Open Questions

**Now answerable from the codebase:**

- **The strong-form CI job.** Feasible on the existing `ubuntu-latest` runners
  with `sudo`, no container needed; put it in `main.yml` (actionlint sees
  nothing else) as a Linux-only job modelled on `check-architecture`, avoiding
  the three nightly markers. **But the shadow list must be rewritten**: there
  is no system `jj` on the runner, so the meaningful target is the mise install
  path. Unproven in-repo — settle with a one-step `sudo` smoke test before
  committing the criterion.

**Newly surfaced, and gating:**

- **Does a combined `allowed_only` + `denied` cargo-pup rule behave as
  intended?** No shipped rule sets both and no test covers the combination.
  The entire structural zero-spawn guarantee rests on `std::process` being
  denied while `std` is permitted. Verify empirically alongside the oracle
  mapping.
- **How is the shared fixture published, given the "no features beyond
  `bash-parity`" criterion?** There is no cross-crate precedent, `CARGO_BIN_EXE_`
  does not cross crates, and `--all-features` makes any new feature global in
  CI. The criterion and the mechanism need reconciling.
- **Which "colocated" does the matrix mean?** The shell fixture is a hand-graft
  that classifies as `colocated`; a real `jj git init --colocate` main repo
  classifies as `main`. Both are legitimate shapes; the matrix currently names
  one row.
- **Does `jj git init` produce a colocated repo by default at 0.43?** The
  golden snapshot says the 0.36-era bare `jj git init` did. If so, the pure-jj
  measurement fixture needs an explicit non-colocating construction and a
  `.git`-absent assertion, and `make_main_jj_workspace` is misnamed.
- ~~**Does `gix` 0.85 expose usable APIs for bare detection, worktree/common-dir
  resolution, and superproject resolution?**~~ **ANSWERED 2026-08-03 by spike**
  — see Follow-up Research below. Queries 1 and 2 are served by library calls
  (`is_bare()`, `kind()`, `git_dir()`, `common_dir()`, `main_repo()`); **query 3
  has no gix API** and needs a ~15-line hand-rolled derivation, validated
  against the oracle at submodule depths 1 and 2. The spike also overturned the
  boundary mechanism (no ceiling can enforce the rule) and showed the git-side
  scrub invariant holds for free.
- **Which host do the cost measurements run on?** `B = 35.1 ms` is
  darwin-arm64 and the probe delta is macOS-specific. Measuring 0188's figures
  on a different host leaves 0169's gate ill-posed — an unresolved review-2
  pass-3 finding now spanning two work items.
- **Who changes `vcs_adapters::facts`** — 0169 or 0185 — remains open, and 0185
  currently *assumes* 0169 will ("0169 will need to alter this anyway",
  `0185:117-121`), which 0188 contradicts by design.

## Follow-up Research: gix 0.85 API spike (2026-08-03T06:52:09+00:00)

Closes the open question "Does `gix` 0.85 expose usable APIs for bare detection,
worktree/common-dir resolution, and superproject resolution?" and two riders
that shared the same fixtures. Method: a throwaway Rust prototype
(`gix` 0.85.0, **default features**) against real git fixtures, with every gix
answer printed beside the `git rev-parse` oracle for the same start directory.
Environment: darwin-arm64, `git` 2.54.0, Rust 1.90.0. Prototype discarded.

**One environment finding first**: the machine had `jj` **0.42.0** on `PATH`
from Homebrew, not the pinned 0.43.0 — `mise.toml` is untrusted there so mise
was not activating. This confirms 0188's outstanding "`mise install` on each
machine" item, and sharpens the CI shadow-list finding in §7: on a developer's
macOS machine `/opt/homebrew/bin/jj` **is** the real binary, whereas on CI no
system `jj` exists at all. The shadow list is wrong in *opposite* directions on
the two platforms.

### Q1 — bare-repository detection: available

`Repository::is_bare()` at `gix-0.85.0/src/repository/worktree.rs:64`:

```rust
pub fn is_bare(&self) -> bool {
    self.config.is_bare.unwrap_or_else(|| self.workdir().is_none())
}
```

Against a `git init --bare` fixture: `is_bare() == true`, `workdir() == None`,
`git_dir() == common_dir() == ./bare.git`, oracle
`--is-bare-repository == true`. Agreement.

Note it consults `core.bare` before falling back to workdir absence — the only
one of the six probed calls that reads configuration, which is why Q5 poisoned
`GIT_CONFIG_GLOBAL` as well as `GIT_DIR`.

### Q2 — worktree detection and common-dir: first-class

`Repository::kind()` (`src/repository/location.rs:127`) returns
`gix::repository::Kind` (`src/repository/mod.rs:6-16`), whose three variants map
directly onto the distinctions 0188 needs:

```rust
pub enum Kind {
    /// An ordinary Git repository.
    Common,
    /// A submodule worktree, whose `git` repository lives in `.git/modules/**/<name>` …
    Submodule,
    /// A worktree, whose `git` repository lives in `.git/worktrees/**/<name>` …
    LinkedWorkTree,
}
```

So "is this a linked worktree" is a single enum read, not the `--git-dir` vs
`--git-common-dir` string comparison the shell performs
(`scripts/vcs-common.sh:217-219`). Observed, linked worktree fixture:

| Query | From the linked worktree | From the main worktree |
| --- | --- | --- |
| `kind()` | `LinkedWorkTree` | `Common` |
| `git_dir()` | `./wt-main/.git/worktrees/wt-linked` | `./wt-main/.git` |
| oracle `--git-dir` | `./wt-main/.git/worktrees/wt-linked` | `./wt-main/.git` |
| `common_dir()` | `./wt-main/.git` | `./wt-main/.git` |
| oracle `--git-common-dir` | `./wt-main/.git` | `./wt-main/.git` |
| `main_repo().workdir()` | `./wt-main` | `./wt-main` |
| `worktrees()` | `1 [wt-linked]` | `1 [wt-linked]` |

Full agreement on both start directories. `main_repo()`
(`src/repository/worktree.rs`, "Return the repository owning the main worktree")
answers "what is the common (main) git directory" directly, returning a full
`Repository` whose `workdir()` is the main worktree root.

**A method-note for the plan**: the oracle emits `--git-dir` /
`--git-common-dir` **relative to the invocation directory** — from a
subdirectory, `--git-common-dir` returned `../../.git`. Resolve against the
start path before comparing, exactly as `vcs-common.sh:215-216` does by hand.
An early version of the prototype canonicalised against the process CWD instead
and produced spurious mismatches; the oracle mapping's cells must record which
directory each relative path is relative to.

### Q3 — superproject resolution: no API, but hand-rollable

**There is no superproject-ward API in `gix` 0.85, `gix-discover` or
`gix-submodule`.** Confirmed two ways: a search for
`pub fn .*superproject|superproject_dir|show_superproject|fn superproject`
across all three crates returns nothing; and behaviourally, `main_repo()` called
on a submodule returns `./super/mod` — the submodule itself. The one submodule
API, `Repository::submodules()` (`src/repository/submodule.rs:93`), resolves
only the **opposite** direction and is gated on the `attributes` feature:

```
submodule (modern) @ the superproject   GIX submodules(): [mod->./super/mod]
submodule (modern) @ the submodule      GIX submodules(): None (no .gitmodules)
```

A hand-rolled derivation does work. `Kind::Submodule` identifies the case, and
the superproject's git dir is the grandparent of the nearest `modules` path
component in `git_dir()`. Validated against the oracle:

| Start | `kind()` | `git_dir()` | oracle superproject | derived | |
| --- | --- | --- | --- | --- | --- |
| `./super/mid` | `Submodule` | `./super/.git/modules/mid` | `./super` | `./super` | MATCH |
| `./super/mid/leaf` | `Submodule` | `./super/.git/modules/mid/modules/leaf` | `./super/mid` | `./super/mid` | MATCH |
| `./super` | `Common` | `./super/.git` | `-` | `-` | MATCH |

The depth-2 row matters: the `modules` segment repeats when submodules nest, and
taking the *nearest* such ancestor (not the first) is what makes the derivation
correct. About 15 lines, plus the bare-superproject branch (a superproject whose
git dir is not named `.git` needs a `gix::open` to recover its workdir).

**This is a sizing correction for the story**: five of the six queries are
library calls; query 3 is bespoke path logic with its own edge cases and
therefore its own tests.

**Old-form submodules agree with git, so they are not a divergence.** A nested
`.git` *directory* inside a superproject reports `Kind::Common` — as gix's own
docs warn — and `git rev-parse --show-superproject-working-tree` also returns
empty for it. Both say "no superproject". The fixture matrix should still carry
the shape, because a `Kind::Submodule`-only implementation would silently miss
it and no oracle disagreement would reveal the omission.

### Q4 — boundary containment: the ceiling route cannot work

Fixture: `git-outer/` is a git repository; `git-outer/inner-jj/` carries only a
`.jj` marker. Under 0188's rule the boundary for any start inside `inner-jj` is
`inner-jj`, so the only acceptable answers are "no repository" or `inner-jj`.

```
start=boundary, ceil=boundary, strict  ->  Err(None of the passed ceiling
                                           directories prefixed the git-dir
                                           candidate, making them ineffective.)
start=boundary, ceil=boundary, lax     ->  ./git-outer   <<< VIOLATES THE RULE
start=boundary, ceil=parent,   strict  ->  ./git-outer   <<< VIOLATES THE RULE
start=deep,     ceil=boundary, strict  ->  Err(… within ceiling height of 3)
start=deep,     ceil=parent,   strict  ->  ./git-outer   <<< VIOLATES THE RULE
```

**No anchor works, and the reason is structural.**
`gix-discover-0.54.0/src/upwards/util.rs`, `find_ceiling_height`, computes
`search_dir.strip_prefix(ceiling_dir).components().count()` and then
`.filter(|height| *height > 0)` — so a ceiling equal to the start directory
yields height 0 and is **discarded**, matching the field's own doc ("we ignore
ceiling directories if the search directory is directly on top of one"). With
the ceiling discarded and `match_ceiling_dir_or_error: false`, `max_height`
becomes `None` and the walk is *unbounded*. And where a ceiling does apply, the
loop in `upwards/mod.rs:96-103` tests `current_height > max_height` before
incrementing `current_height` from 0, permitting one level of ascent past the
ceiling. There is no configuration that confines the walk to the start
directory alone.

`gix::open` is the right instrument — it does not walk:

```
open(boundary)  [.jj only, no .git]  ->  Err(… does not appear to be a git repository)
open(git-outer) [real git repo]      ->  ./git-outer   (no walk)
open(deep)      [plain subdir]       ->  Err(… does not appear to be a git repository)
```

Composing the existing marker walk with a non-walking open gives exactly the
required behaviour on every start:

| Start | Marker-walk boundary | `gix::open(boundary)` |
| --- | --- | --- |
| `./git-outer/inner-jj` | `./git-outer/inner-jj` | None (not a git repo) |
| `./git-outer/inner-jj/x/y` | `./git-outer/inner-jj` | None (not a git repo) |
| `./git-outer` | `./git-outer` | `./git-outer` |

`MarkerWalkRoot` (`cli/vcs-adapters/src/lib.rs:34-44`) already *is* that walk,
so the boundary rule is satisfied by composition rather than by new code. 0188's
Library traps section has been rewritten accordingly.

The **paired negative assertion** the criterion requires is available and
reproducible: unbounded `gix::discover` from `./git-outer/inner-jj` escapes to
`./git-outer`. Note also that plain `gix::discover` never reads
`GIT_CEILING_DIRECTORIES` — only `discover_with_environment_overrides` consults
the environment — so the criterion's concern that the environment might be what
makes the rule hold does not arise on this path.

**A consequence for query 1**: a bare repository has neither marker, so
`marker_walk(bare)` returns `None` (verified in a directory with no ancestor
marker), while `gix::discover(bare)` and `gix::open(bare)` both succeed with
`is_bare = true`. Under a marker-walk-first design bare detection is unreachable
via the boundary path and needs its own entry point. This is consistent with
today's pinned behaviour (`cli/vcs-adapters/tests/detection.rs:250-277`,
`facts(&bare) == None`), but it means query 1's start path is *not* the boundary.

### Q5 — the scrub invariant holds for free on the git side

Poison: `GIT_DIR` and `GIT_COMMON_DIR` pointed at another fixture's real `.git`,
plus `GIT_CONFIG_GLOBAL` at a config asserting `core.bare = true`.

| Query | Linked worktree | Main worktree |
| --- | --- | --- |
| `kind()` | STABLE | STABLE |
| `is_bare()` | STABLE | STABLE |
| `git_dir()` | STABLE | STABLE |
| `common_dir()` | STABLE | STABLE |
| `workdir()` | STABLE | STABLE |
| `main_repo().workdir()` | STABLE | STABLE |
| **oracle `--git-dir`** | **DIVERGED** | **DIVERGED** |

The oracle divergence is the control: under the same poisoning
`git rev-parse --git-dir` returned `./plain/.git` from both start directories,
so the poison was live and effective. Separately,
`discover_with_environment_overrides` diverged (returning `./plain`) while plain
`gix::discover` did not.

Two consequences:

- **No explicit scrub is needed for the git-side queries.** 0188's invariant
  ("`GIT_DIR`/`GIT_COMMON_DIR` are scrubbed for the duration of any detection
  call") is satisfied by construction when the code uses `gix::discover` /
  `gix::open` rather than the `_with_environment_overrides` variants. The
  invariant becomes a property to **verify**, not to implement — which is a
  cheaper and stronger position than the work item assumes.
- **The non-vacuity control is ready-made.** The criterion demands that "an
  unscrubbed control must diverge under the same poisoning".
  `discover_with_environment_overrides` is exactly that control, in-library, no
  fixture surgery required.

The jj-lib side is untested — this spike was scoped to the git-side queries.

### Residual risks and what remains open

- **jj-lib is untested here.** Queries 4 and 5 (jj workspace root, main vs
  secondary) and the jj half of query 6 still rest only on the 2026-07-29
  probe — which, per §Historical Context, was run with a `jj` **0.36** CLI
  writing fixtures for `jj-lib` 0.43. That skew is unretired.
- **Single platform.** darwin-arm64, git 2.54.0. The Linux behaviour of
  `is_bare()`'s config fallback and of `main_repo()` on a bare main repository
  is unverified.
- **Config poisoning was global, not repo-local.** A repository-local
  `core.bare = true` lie in `.git/config` was not tested; `is_bare()` reads
  config, so that is the one plausible route to a query-1 divergence.
- **`submodules()` needs the `attributes` feature.** Confirmed present under
  gix's default features, but if the plan narrows features to shrink the
  binary, the superproject-ward direction is unaffected (it is hand-rolled)
  while `submodules()` would be lost.
