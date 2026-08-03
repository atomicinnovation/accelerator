---
type: plan
id: "2026-08-03-0188-library-backed-vcs-adapter"
title: "Library-Backed VCS Adapter over gix and jj-lib Implementation Plan"
date: "2026-08-03T09:10:56+00:00"
author: "Toby Clemson"
producer: create-plan
status: draft
work_item_id: "work-item:0188"
parent: "work-item:0188"
derived_from: ["codebase-research:2026-08-02-0188-library-backed-vcs-adapter"]
relates_to: ["work-item:0169", "work-item:0185", "work-item:0125"]
tags: [rust, vcs, dependencies, gix, jj-lib]
revision: "2ec1cc10961f3070ff6432cd2ebe54c52886b13e"
repository: "accelerator"
last_updated: "2026-08-03T09:10:56+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Library-Backed VCS Adapter over gix and jj-lib Implementation Plan

## Overview

Add an in-process implementation of `cli/vcs`'s `RepoRoot` and `VcsProbe` ports
over `gix` 0.85 and `jj-lib` 0.43, plus six inherent taxonomy queries and the
test apparatus that proves the whole thing reads git and jj without spawning a
subprocess. `CommandProbe` and `MarkerWalkRoot` are retained; nothing is wired
to the new type. The value delivered is risk isolation: two dependency trees, a
workspace-wide licence exception and a pre-1.0 API bet land where they can be
reviewed and reverted alone.

## Current State Analysis

`cli/vcs` (193 lines, zero dependencies) defines `VcsKind`, `RepoFacts`,
`RepoRoot` and `VcsProbe` (`cli/vcs/src/lib.rs:46-67`) and the composition
function `facts` (`:74-91`). **This crate is not touched by this plan.**

`cli/vcs-adapters` (323 lines) holds `MarkerWalkRoot`
(`cli/vcs-adapters/src/lib.rs:32`), `CommandProbe` (`:73`), the two subprocess
invocations (`:110-125`), the single spawn chokepoint (`:168`), the environment
scrub (`:139-154`) and the hard-wired composition root (`:224-227`). The jj
secondary-workspace rule is already implemented by pure file reads
(`jj_repository_root`, `:57-68`).

The dependency work is entirely unstarted: `gix`, `jj-lib` and `uluru` appear
nowhere in `cli/Cargo.lock` (358 packages), `cli/deny.toml`, `cli/pup.ron` or
`cli/Cargo.toml`. The only landed prerequisite is the `mise.toml` `jj` pin at
0.43.0 with its lockstep comment (`mise.toml:12-16`), and — as of this planning
session — `mise install` on this machine.

`cli/corpus-adapters` reaches the free function `vcs_adapters::facts`
(`src/metadata.rs:201`), not the adapter types, so leaving `facts` hard-wired
keeps the retained pair in place with no consumer change.

## Desired End State

`cli/vcs-adapters` exports a module `library` containing one type,
`InProcessProbe`, that implements both ports over `gix`/`jj-lib` and carries six
inherent taxonomy queries returning plain domain values behind
`Result<Option<_>, _>`, with every returned path canonicalised. A new workspace
crate `cli/vcs-test-support` publishes the fixture matrix, the zero-spawn
harness and the named pure-jj builder, consumed by both `cli/vcs-adapters` and
`cli/corpus-adapters`. Two reference artefact binaries link the trees,
cross-compile to static musl on all four release triples, and are measured
against a committed size floor. A Linux-only CI job proves the strong form of
zero-spawn against prebuilt fixtures and prebuilt binaries. `mise run` is green;
`CommandProbe`, `MarkerWalkRoot`, `cli/vcs/src/**` and every existing consumer
are behaviourally unchanged — though both dependency trees do enter the build
graph of `cli/corpus-adapters` and `cli/visualiser/server`.

### Key Discoveries

Established empirically during this planning session (jj 0.43.0, git 2.54.0,
Rust 1.90.0, darwin-arm64). The full oracle mapping is in
[Oracle Mapping](#oracle-mapping) below.

1. **`jj git init` colocates by default at 0.43, and `--no-colocate` exists.**
   A bare `jj git init` produces both `.jj` and `.git`. The pure-jj fixture is
   therefore a one-flag construction, not a post-hoc `.git` deletion — and the
   shell suite's `make_main_jj_workspace` (`hooks/test-vcs-detect.sh:47-52`) has
   been building *colocated* repositories all along.

2. **Three distinct walks are required, not one.** The work item names only the
   combined `.jj`-or-`.git` boundary walk. Measurement shows queries 4, 5 and
   the jj half of query 6 need a **`.jj`-only** walk: from
   `nested-git-in-jj`'s inner git worktree the boundary walk yields `sub`, and
   `DefaultWorkspaceLoaderFactory::create(sub)` returns
   `Err(There is no Jujutsu repo in …)`, whereas a `.jj`-only walk yields the
   outer jj root — which is exactly what the `jj workspace root` oracle
   returns. Using the boundary walk for query 4 would report absence where the
   oracle reports a root.

3. **`gix::open` for the boundary, `gix::discover` for the queries.** Both
   mechanisms live in the same type. `gix::open(boundary)` performs no walk and
   is the `RepoRoot` mechanism; `gix::discover(start)` legitimately escapes the
   boundary and is required by queries 1, 2, 3 and the git half of 6. Confirmed:
   on `nested-jj-in-git`'s inner workspace, `gix::open(boundary)` errors while
   `gix::discover(start)` returns the outer git root, matching
   `git rev-parse --show-toplevel`.

4. **`common_dir()` is not normalised.** For a linked worktree gix returns
   `…/.git/worktrees/<id>/../..` where the oracle returns `…/.git`. The two are
   equal only after canonicalisation. Recorded nowhere previously.

5. **`main_repo()` on a submodule returns the submodule**, so it must not be
   used for superproject resolution. The hand-rolled derivation (nearest
   `modules` component in `git_dir()`, then `gix::open` on its parent) matched
   the oracle at submodule depths 1 and 2.

6. **The scrub invariant holds for free on both sides.** All 24 (fixture, start
   directory) pairs returned identical values with `GIT_DIR` and
   `GIT_COMMON_DIR` poisoned at another fixture's real `.git`. The research had
   proven only the git side; the jj-lib loader is pure filesystem reads and is
   likewise unaffected. **No scrub is implemented — the invariant is verified.**
   `gix::discover_with_environment_overrides` diverges under the same poison
   (returning the poison target) and is the confirmed non-vacuity control.
   Scope note: this measured two variables. `gix::open`'s default permissions do
   consult the environment for object directories and for system/global config
   discovery, and `is_bare()` reads `core.bare` *through* that config — so
   Phase 2 §3 widens the poisoning matrix to everything `scrub_environment`
   touches plus the object-directory and `GIT_CONFIG_COUNT` families, rather
   than generalising from these two to "uniformly immune".

7. **The two-clause cargo-pup rule works.** With `allowed_only` permitting
   `^(std|core|alloc)(::|$)` and `denied` listing `^std::process(::|$)`, a
   `use std::process::Command` fails with *"Use of module 'std::process::Command'
   is denied"*. The grouped-import gotcha still bites under the combined rule:
   `use std::path::{Path, PathBuf}` fails with *"Use of module '' is not
   allowed"*. Single-item imports are mandatory throughout the new module.

8. **`jj-lib` 0.43 with default features does pull `gix` 0.85**, enabling
   `attributes`, `blob-diff`, `index`, `max-performance-safe`, `sha1` and
   `zlib-rs` on it. The single-graph reasoning behind the pin holds, and
   `submodules()`' `attributes` feature comes from jj-lib itself.

9. **56 gix-family packages, zero duplicate versions**, no TLS stack, no
   `git2`/`libgit2-sys`, and **no zstd** — the `include-flate-compress`
   collision the research anticipated does not materialise. `uluru 3.1.0`
   (MPL-2.0, via `gix-pack`, reached from both `gix` and `gix-odb`) is the only
   licence rejection; `bans`, `sources` and `advisories` pass against the real
   `cli/deny.toml`, and the exception clears `licenses`.

10. **The size-delta criterion is mis-calibrated** — see
    [Work-Item Amendments](#work-item-amendments). Measured deltas: musl-static
    stripped 2,031,448 B; darwin stripped 1,639,872 B. The ratio is 6.19×.

11. **MSRV-aware resolution is load-bearing.** Without `rust-version` +
    `resolver = "3"` the graph selects `kstring 2.0.4`, which requires Rust
    1.96 and refuses to build on the pinned 1.90.0. The `cli/` workspace has
    both, so the lock must be generated in-workspace.

12. **The single-query mode is not load-bearing.** `refart all` (six queries +
    both port methods) measured 3.66 ms against `refart only q4` at 3.65 ms on
    the pure-jj fixture — process startup dominates. The mode is retained
    because 0169 will want it, not because the figure would otherwise be
    unusable.

13. **No cross-crate test-fixture precedent exists** in `cli/` (zero path-based
    `[dev-dependencies]`); the build-system guards scan `cli/**/src` rather than
    an enumerated crate list, so a new member costs the workspace `members`
    array and the lock.

## What We're NOT Doing

- **Not touching `cli/vcs/src/**`.** No port added, widened or changed.
- **Not removing `CommandProbe` or `MarkerWalkRoot`**, and not adding any
  method to either. That is 0185's work.
- **Not wiring anything.** `vcs_adapters::facts` stays hard-wired to
  `MarkerWalkRoot`/`CommandProbe`. No feature flag, config switch or
  composition helper routes a caller to `InProcessProbe`.
- **Not building `classify_checkout`'s arm cascade**, and not implementing
  `vcs status` / `vcs log`. 0169 owns both, and owns widening the pup rule to
  cover wherever it puts that code.
- **Not defining a domain port over the six queries.** 0169 defines whatever
  port its classifier needs.
- **Not gating on cost.** Numbers are measured and recorded; 0169 owns the
  `G ≤ 1.1 × B` gate.
- **Not migrating the ~26 shell call sites.** They keep running in bash.

## Implementation Approach

Five phases, each independently mergeable and each leaving `mise run` green.
Phase 1 lands the dependency trees and every enforcement gate, so a
policy objection surfaces before any query logic exists. Phase 2 builds the six
queries test-first against the recorded oracle mapping, in the shared
test-support crate created there. Phase 3 adds the stub harness and proves it
across a crate boundary. Phase 4 adds the stub artefact, the musl staging
and the strong-form CI job, and takes the measurements. Phase 5 writes the
sibling hand-offs and the work-item amendments.

**Phase 1 starts once work-item amendments 5-8 have landed.** The `revision`
question that previously blocked it was closed by spike on 2026-08-03 (jj
`revision` is descoped to 0185 — see Phase 1 §4).

Test-driven throughout where the oracle is known: every query's expected values
come from the mapping table below, written as assertions before the query
exists.

---

## Oracle Mapping

Established by running each candidate oracle against each fixture on
**2026-08-03**, jj **0.43.0**, git **2.54.0**, Rust **1.90.0**,
`Darwin arm64`. Fixtures were built in a hermetic environment (`HOME`,
`XDG_CONFIG_HOME`, `JJ_CONFIG` at temp dirs; `GIT_CONFIG_NOSYSTEM=1`;
`GIT_CONFIG_GLOBAL=/dev/null`; `GIT_CEILING_DIRECTORIES` at the temp base) and
paths are shown relative to that base as `$BASE`. Every oracle command is run
**with the start directory as cwd**.

Query 2's `PJG-i` common-dir, Query 5's `NGPJ-i` row, and Query 6's `JS-r`,
`JS-s`, `WT-m`, `SM-2`, `SM-s`, `SO` and `PJS` rows were **re-measured on
2026-08-03** after plan review 1 found the first draft's table incomplete in
those cells; the re-measurement used the same environment and tool versions.

`superproject()` deliberately conflates the two absence signals below into
`None` — it cannot distinguish "in a repository with no superproject" from "not
in a repository at all", and nothing in the taxonomy needs the distinction.

**Absence signals differ per oracle** and are part of the contract:

| Oracle | Absence signal |
| --- | --- |
| `git rev-parse --is-bare-repository` / `--git-dir` / `--git-common-dir` / `--show-toplevel` | exit **128**, empty stdout |
| `git rev-parse --show-superproject-working-tree`, inside a repository with no superproject | exit **0**, empty stdout |
| `git rev-parse --show-superproject-working-tree`, outside any repository | exit **128**, empty stdout |
| `jj workspace root` | exit **1**, empty stdout |
| `find_git_main_worktree_root` | exit 1, empty stdout |
| `find_jj_main_workspace_root` | exit 1, empty stdout |

### Fixture keys

| Key | Shape | Start directory |
| --- | --- | --- |
| `CR` | colocated, real (`git init` + `jj git init --colocate`) | root |
| `CG` | colocated, hand-grafted (git linked worktree + grafted `.jj`) | root |
| `JS-r` | jj secondary workspace | workspace root |
| `JS-s` | jj secondary workspace | subdirectory `deep/er` |
| `PG-r` | plain git | root |
| `PG-s` | plain git | subdirectory `deep/er` |
| `NJG-i` | nested-jj-in-git — the inner directory is a jj **secondary** workspace of `$BASE/jjmain-colocated` (which is itself colocated); the inner directory carries `.jj` only | inner jj workspace |
| `NJG-o` | nested-jj-in-git | outer git root |
| `NGJ-i` | nested-git-in-jj (colocated outer) | inner git worktree |
| `NGJ-o` | nested-git-in-jj (colocated outer) | outer jj root |
| `WT-l` | linked git worktree | the linked worktree |
| `WT-m` | linked git worktree | the main worktree |
| `SM-1` | git submodule, modern form | the submodule (`super/mid`) |
| `SM-2` | git submodule nested | depth-2 submodule (`super/mid/leaf`) |
| `SM-s` | git submodule | the superproject root |
| `SO` | old-form submodule (nested `.git` **directory**) | the nested repo |
| `BARE` | bare repository | the bare dir |
| `NONE` | no repository at all | a marker-less dir |
| `PJ` | pure jj (`--no-colocate`), root three dirs below temp root | root |
| `PJS` | secondary workspace of a `--no-colocate` main | workspace root |
| `NGPJ-i` | nested-git-in-**pure**-jj | inner git worktree |
| `NGPJ-o` | nested-git-in-**pure**-jj | outer jj root |
| `PJG-i` | pure-jj-in-git — the inner directory is a jj secondary workspace of a `--no-colocate` main | inner jj workspace |
| `PJG-o` | pure-jj-in-git | outer git root |

Every row is one (fixture, start directory) pair, so the **core matrix is 24
pairs**. Ten further fixtures were added after plan review and measured on
2026-08-03; their keys and oracle values are in
[Extended fixtures](#extended-fixtures-measured-2026-08-03) below, which also
records why they are kept separate from the core matrix.

**Shared parents are named per fixture**, because an earlier draft collapsed them
and produced two impossible identities: a single `$BASE/jjmain` was named as the
main of both `NJG-i` (a colocated main) and `PJG-i` (a `--no-colocate` main), and
a single `$BASE/gitparent` was recorded as hosting worktrees keyed `sub` for both
`NGJ-i` and `NGPJ-i` — which `git worktree add` cannot do, since it derives the id
from the basename and de-duplicates the second to `sub1`. The `worktrees()` count
corroborated the collapse: three worktrees of one parent would have counted 3,
not the recorded 1.

The names, matching the measurement runs:

| Parent | Shape | Fixtures that use it |
| --- | --- | --- |
| `$BASE/jjmain-colocated` | `jj git init` (colocated by default at 0.43) | `JS-r`, `JS-s`, `NJG-i` |
| `$BASE/jjmain-pure` | `jj git init --no-colocate` | `PJG-i` |
| `$BASE/gitparent-cg` | plain git, hosts the `CG` graft worktree | `CG` |
| `$BASE/gitparent-ngj` | plain git, hosts `NGJ`'s inner worktree | `NGJ-i` |
| `$BASE/gitparent-ngpj` | plain git, hosts `NGPJ`'s inner worktree | `NGPJ-i` |

Each `gitparent-*` hosts exactly **one** linked worktree, so `worktrees()` is 1
from both ends, as recorded.

`CR` and `CG` are **both** carried because the shell's `colocated` arm requires
`jj_secondary && git_worktree` (`scripts/vcs-common.sh:242-247`): a genuine
`jj git init --colocate` main repository classifies as `main`, not `colocated`.
The work item's single "colocated | root" row is ambiguous between them.

### Query 1 — bare-repository check

Oracle: `git rev-parse --is-bare-repository`.
Library: `gix::discover(start)?.is_bare()`.

| Fixture | Oracle stdout (exit) | Library | Verdict |
| --- | --- | --- | --- |
| `CR` | `false` (0) | `false` | agree |
| `CG` | `false` (0) | `false` | agree |
| `JS-r` | *empty* (128) | not a git repo → `None` | agree |
| `JS-s` | *empty* (128) | `None` | agree |
| `PG-r` | `false` (0) | `false` | agree |
| `PG-s` | `false` (0) | `false` | agree |
| `NJG-i` | `false` (0) | `false` | agree |
| `NJG-o` | `false` (0) | `false` | agree |
| `NGJ-i` | `false` (0) | `false` | agree |
| `NGJ-o` | `false` (0) | `false` | agree |
| `WT-l` | `false` (0) | `false` | agree |
| `WT-m` | `false` (0) | `false` | agree |
| `SM-1` | `false` (0) | `false` | agree |
| `SM-2` | `false` (0) | `false` | agree |
| `SM-s` | `false` (0) | `false` | agree |
| `SO` | `false` (0) | `false` | agree |
| `BARE` | `true` (0) | `true` | agree |
| `NONE` | *empty* (128) | `None` | agree |
| `PJ` | *empty* (128) | `None` | agree |
| `PJS` | *empty* (128) | `None` | agree |
| `NGPJ-i` | `false` (0) | `false` | agree |
| `NGPJ-o` | *empty* (128) | `None` | agree |
| `PJG-i` | `false` (0) | `false` | agree |
| `PJG-o` | `false` (0) | `false` | agree |

`BARE` is reachable only via `gix::discover`/`gix::open` on the start path — the
boundary walk returns `None` there (a bare repository carries neither marker),
confirming the work item's Library-traps consequence. `kind()` for a bare
repository is `Common`, not a distinct variant.

### Query 2 — worktree detection and common dir

Oracle: `git rev-parse --git-dir` vs `git rev-parse --git-common-dir`
(different ⇒ linked worktree), and the main worktree root as
`realpath $(dirname <absolutised common-dir>)`.
Library: `kind()`, `git_dir()`, `common_dir()`, `main_repo().workdir()`.

**Oracle outputs are relative to the invocation directory** where git chooses to
emit them; they must be absolutised against the start path before comparison, as
`scripts/vcs-common.sh:215-216` does by hand.

| Fixture | `--git-dir` (exit 0) | `--git-common-dir` (exit 0) | `kind()` | `git_dir()` | `common_dir()` (raw) | `main_repo().workdir()` | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `CR` | `.git` | `.git` | `Common` | `$BASE/CR/.git` | `$BASE/CR/.git` | `$BASE/CR` | agree |
| `CG` | `$BASE/gitparent-cg/.git/worktrees/CG` | `$BASE/gitparent-cg/.git` | `LinkedWorkTree` | `$BASE/gitparent-cg/.git/worktrees/CG` | `$BASE/gitparent-cg/.git/worktrees/CG/../..` | `$BASE/gitparent-cg` | agree **after canonicalising `common_dir()`** |
| `PG-r` | `.git` | `.git` | `Common` | `$BASE/PG/.git` | `$BASE/PG/.git` | `$BASE/PG` | agree |
| `PG-s` | `$BASE/PG/.git` | `../../.git` | `Common` | `$BASE/PG/.git` | `$BASE/PG/.git` | `$BASE/PG` | agree after absolutising the oracle |
| `NJG-i` | `$BASE/NJG/.git` | `../.git` | `Common` | `$BASE/NJG/.git` | `$BASE/NJG/.git` | `$BASE/NJG` | agree |
| `NJG-o` | `.git` | `.git` | `Common` | `$BASE/NJG/.git` | `$BASE/NJG/.git` | `$BASE/NJG` | agree |
| `NGJ-i` | `$BASE/gitparent-ngj/.git/worktrees/sub` | `$BASE/gitparent-ngj/.git` | `LinkedWorkTree` | `$BASE/gitparent-ngj/.git/worktrees/sub` | `…/worktrees/sub/../..` | `$BASE/gitparent-ngj` | agree after canonicalising |
| `NGJ-o` | `.git` | `.git` | `Common` | `$BASE/NGJ/.git` | `$BASE/NGJ/.git` | `$BASE/NGJ` | agree |
| `WT-l` | `$BASE/main/.git/worktrees/WT` | `$BASE/main/.git` | `LinkedWorkTree` | `$BASE/main/.git/worktrees/WT` | `…/worktrees/WT/../..` | `$BASE/main` | agree after canonicalising |
| `WT-m` | `.git` | `.git` | `Common` | `$BASE/main/.git` | `$BASE/main/.git` | `$BASE/main` | agree |
| `SM-1` | `$BASE/super/.git/modules/mid` | `$BASE/super/.git/modules/mid` | `Submodule` | same | same | `$BASE/super/mid` | agree — **equal dirs, so not a worktree** |
| `SM-2` | `$BASE/super/.git/modules/mid/modules/leaf` | same | `Submodule` | same | same | `$BASE/super/mid/leaf` | agree |
| `SM-s` | `.git` | `.git` | `Common` | `$BASE/super/.git` | `$BASE/super/.git` | `$BASE/super` | agree |
| `SO` | `.git` | `.git` | `Common` | `$BASE/old/inner/.git` | same | `$BASE/old/inner` | agree |
| `BARE` | `.` | `.` | `Common` | `$BASE/bare` | `$BASE/bare` | `workdir() == None` | agree |
| `NGPJ-i` | `$BASE/gitparent-ngpj/.git/worktrees/sub` | `$BASE/gitparent-ngpj/.git` | `LinkedWorkTree` | as oracle | `…/../..` | `$BASE/gitparent-ngpj` | agree after canonicalising |
| `PJG-o` | `.git` | `.git` | `Common` | `$BASE/PJG/.git` | same | `$BASE/PJG` | agree |
| `JS-r`, `JS-s`, `NONE`, `PJ`, `PJS`, `NGPJ-o` | *empty* (128) | *empty* (128) | n/a | n/a | n/a | n/a | agree (absent) |
| `PJG-i` | `$BASE/PJG/.git` | `../.git` | `Common` | `$BASE/PJG/.git` | same | `$BASE/PJG` | agree after absolutising the oracle |

The `PJG-i` common-dir was recorded as `.git` in the first draft of this table;
re-measurement on 2026-08-03 returned `../.git`. Absolutised against the start
directory `$BASE/PJG/sub` that is `$BASE/PJG/.git`, equal to `--git-dir`, so the
shell oracle sets `git_worktree=0` and `kind() == Common` agrees. The earlier
value would have made the oracle take the `colocated` arm.

The `main_worktree_root` column's oracle is `realpath $(dirname <absolutised
common-dir>)` **only for `Kind::Common` and `Kind::LinkedWorkTree`**. It is
undefined for the bare and submodule shapes: applied to `BARE` the formula gives
`$BASE`, and to `SM-1`/`SM-2` it gives a `.git/modules/…` path — neither of which
is a worktree root. For those shapes the recorded library values
(`workdir() == None`, and the submodule's own workdir) are the contract, and
`WorktreeFacts.main_worktree_root` carries no oracle. The Verdict column covers
the `kind()`/`git_dir()`/`common_dir()` comparison only.

`worktrees()` enumerated `1` linked worktree by id from **both** the linked and
the main worktree (`WT-l`, `WT-m`, `CG`, `NGJ-i`, `NGPJ-i`), and `0` elsewhere.

### Query 3 — superproject / submodule resolution

Oracle: `git rev-parse --show-superproject-working-tree` (**exit 0 + empty**
when there is no superproject).
Library: no gix API exists — derived from `Kind::Submodule` plus the nearest
`modules` component of `git_dir()`, then `gix::open` on its parent to recover
the workdir.

| Fixture | Oracle stdout (exit) | Derived | Verdict |
| --- | --- | --- | --- |
| `SM-1` | `$BASE/super` (0) | `$BASE/super` | agree |
| `SM-2` | `$BASE/super/mid` (0) | `$BASE/super/mid` | agree — **nearest**, not first, `modules` |
| `SM-s` | *empty* (0) | `None` (`kind() == Common`) | agree |
| `SO` | *empty* (0) | `None` (`kind() == Common`) | agree — old-form submodules are not submodules to either |
| every other fixture | *empty* (0) or 128 | `None` | agree |

### Query 4 — jj workspace-root resolution

Oracle: `jj workspace root`.
Library: **`.jj`-only** upward walk, then
`DefaultWorkspaceLoaderFactory::create(root)?.workspace_root()`.

| Fixture | Oracle stdout (exit) | Via boundary walk | Via `.jj`-only walk | Verdict |
| --- | --- | --- | --- | --- |
| `CR` | `$BASE/CR` (0) | `$BASE/CR` | `$BASE/CR` | agree |
| `CG` | `$BASE/CG` (0) | `$BASE/CG` | `$BASE/CG` | agree |
| `JS-r` | `$BASE/JS` (0) | `$BASE/JS` | `$BASE/JS` | agree |
| `JS-s` | `$BASE/JS` (0) | `$BASE/JS` | `$BASE/JS` | agree |
| `PG-r`, `PG-s` | *empty* (1) | `Err(no Jujutsu repo)` | walk found nothing | agree |
| `NJG-i` | `$BASE/NJG/sub` (0) | `$BASE/NJG/sub` | `$BASE/NJG/sub` | agree |
| `NJG-o` | *empty* (1) | `Err(no Jujutsu repo)` | nothing | agree |
| **`NGJ-i`** | **`$BASE/NGJ` (0)** | **`Err(There is no Jujutsu repo in $BASE/NGJ/sub)`** | **`$BASE/NGJ`** | **only the `.jj`-only walk agrees** |
| `NGJ-o` | `$BASE/NGJ` (0) | `$BASE/NGJ` | `$BASE/NGJ` | agree |
| `WT-l`, `WT-m`, `SM-*`, `SO`, `BARE`, `NONE` | *empty* (1) | nothing / `Err` | nothing | agree |
| `PJ` | `$BASE/PJ/a/b/c` (0) | same | same | agree |
| `PJS` | `$BASE/PJS` (0) | same | same | agree |
| **`NGPJ-i`** | **`$BASE/NGPJ` (0)** | **`Err(no Jujutsu repo in $BASE/NGPJ/sub)`** | **`$BASE/NGPJ`** | **only the `.jj`-only walk agrees** |
| `NGPJ-o` | `$BASE/NGPJ` (0) | `$BASE/NGPJ` | `$BASE/NGPJ` | agree |
| `PJG-i` | `$BASE/PJG/sub` (0) | `$BASE/PJG/sub` | `$BASE/PJG/sub` | agree |
| `PJG-o` | *empty* (1) | `Err(no Jujutsu repo)` | nothing | agree |

The two bold rows are the empirical basis for Key Discovery 2.

### Query 5 — jj main-vs-secondary, and where the main repository is

Oracle: `.jj/repo` is a **file** ⇒ secondary (`_jj_workspace_is_secondary`,
`scripts/vcs-common.sh:74-81`); main repository root via
`find_jj_main_workspace_root`.
Library: `repo_path()` canonicalised ≠ `<workspace_root>/.jj/repo`
canonicalised ⇒ secondary; main root = `repo_path().parent().parent()`.

| Fixture | `.jj/repo` | Oracle `find_jj_main_workspace_root` (exit) | `repo_path()` | Library verdict | Main root | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| `CR` | directory | `$BASE/CR` (0) | `$BASE/CR/.jj/repo` | main | `$BASE/CR` | agree |
| `CG` | file → `$BASE/jjparent/.jj/repo` (absolute) | `$BASE/jjparent` (0) | `$BASE/jjparent/.jj/repo` | secondary | `$BASE/jjparent` | agree |
| `JS-r` | file → `../../jjmain/.jj/repo` (relative) | `$BASE/jjmain-colocated` (0) | `$BASE/jjmain-colocated/.jj/repo` | secondary | `$BASE/jjmain-colocated` | agree |
| `JS-s` | as `JS-r` | `$BASE/jjmain-colocated` (0) | as `JS-r` | secondary | `$BASE/jjmain-colocated` | agree |
| `NJG-i` | file → `../../../jjmain/.jj/repo` | `$BASE/jjmain-colocated` (0) | `$BASE/jjmain-colocated/.jj/repo` | secondary | `$BASE/jjmain-colocated` | agree |
| `NGJ-i` | directory (at the outer root) | `$BASE/NGJ` (0) | `$BASE/NGJ/.jj/repo` | main | `$BASE/NGJ` | agree via the `.jj`-only walk |
| `NGJ-o` | directory | `$BASE/NGJ` (0) | `$BASE/NGJ/.jj/repo` | main | `$BASE/NGJ` | agree |
| `PJ` | directory | `$BASE/PJ/a/b/c` (0) | `$BASE/PJ/a/b/c/.jj/repo` | main | same | agree |
| `PJS` | file | `$BASE/PJ/a/b/c` (0) | `$BASE/PJ/a/b/c/.jj/repo` | secondary | `$BASE/PJ/a/b/c` | agree |
| `NGPJ-i` | directory (at the outer root) | `$BASE/NGPJ` (0) | `$BASE/NGPJ/.jj/repo` | main | `$BASE/NGPJ` | agree via the `.jj`-only walk |
| `NGPJ-o` | directory | `$BASE/NGPJ` (0) | `$BASE/NGPJ/.jj/repo` | main | `$BASE/NGPJ` | agree |
| `PJG-i` | file | `$BASE/jjmain-pure` (0) | `$BASE/jjmain-pure/.jj/repo` | secondary | `$BASE/jjmain-pure` | agree |
| non-jj fixtures (`PG-*`, `WT-*`, `SM-*`, `SO`, `BARE`, `NONE`, `NJG-o`, `PJG-o`) | absent | *empty* (1) | n/a | `None` | `None` | agree |

`repo_path()` is already canonicalised by jj-lib (`dunce::canonicalize`,
`jj-lib-0.43.0/src/workspace.rs:576`) while `workspace_root()` is the path
passed in — both sides must be canonicalised before comparison.

### Query 6 — independent dual-root resolution

Oracle: git root = `git rev-parse --show-toplevel`; jj root =
`jj workspace root`.
Library: git root = `gix::discover(start)?.workdir()` (its own walk, permitted
to escape the boundary); jj root = the `.jj`-only walk.

| Fixture | Oracle git root (exit) | Oracle jj root (exit) | Library git | Library jj | Roots | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| `CR` | `$BASE/CR` (0) | `$BASE/CR` (0) | `$BASE/CR` | `$BASE/CR` | equal | agree |
| `CG` | `$BASE/CG` (0) | `$BASE/CG` (0) | `$BASE/CG` | `$BASE/CG` | equal | agree |
| `JS-r` | *empty* (128) | `$BASE/JS` (0) | `None` | `$BASE/JS` | jj only | agree |
| `JS-s` | *empty* (128) | `$BASE/JS` (0) | `None` | `$BASE/JS` | jj only | agree |
| `PG-r`/`PG-s` | `$BASE/PG` (0) | *empty* (1) | `$BASE/PG` | `None` | git only | agree |
| `NJG-i` | `$BASE/NJG` (0) | `$BASE/NJG/sub` (0) | `$BASE/NJG` | `$BASE/NJG/sub` | differ, jj inside git | agree |
| `NJG-o` | `$BASE/NJG` (0) | *empty* (1) | `$BASE/NJG` | `None` | git only | agree |
| `NGJ-i` | `$BASE/NGJ/sub` (0) | `$BASE/NGJ` (0) | `$BASE/NGJ/sub` | `$BASE/NGJ` | differ, git inside jj | agree |
| `NGJ-o` | `$BASE/NGJ` (0) | `$BASE/NGJ` (0) | `$BASE/NGJ` | `$BASE/NGJ` | equal | agree |
| `WT-l` | `$BASE/WT` (0) | *empty* (1) | `$BASE/WT` | `None` | git only | agree |
| `WT-m` | `$BASE/main` (0) | *empty* (1) | `$BASE/main` | `None` | git only | agree |
| `SM-1` | `$BASE/super/mid` (0) | *empty* (1) | `$BASE/super/mid` | `None` | git only | agree |
| `SM-2` | `$BASE/super/mid/leaf` (0) | *empty* (1) | `$BASE/super/mid/leaf` | `None` | git only | agree |
| `SM-s` | `$BASE/super` (0) | *empty* (1) | `$BASE/super` | `None` | git only | agree |
| `SO` | `$BASE/old/inner` (0) | *empty* (1) | `$BASE/old/inner` | `None` | git only | agree |
| `BARE` | *empty* (128) | *empty* (1) | `workdir() == None` | `None` | neither | agree |
| `NONE` | *empty* (128) | *empty* (1) | `None` | `None` | neither | agree |
| `PJ` | *empty* (128) | `$BASE/PJ/a/b/c` (0) | `None` | `$BASE/PJ/a/b/c` | jj only | agree |
| `PJS` | *empty* (128) | `$BASE/PJS` (0) | `None` | `$BASE/PJS` | jj only | agree |
| `NGPJ-i` | `$BASE/NGPJ/sub` (0) | `$BASE/NGPJ` (0) | `$BASE/NGPJ/sub` | `$BASE/NGPJ` | differ, git inside jj | agree |
| `NGPJ-o` | *empty* (128) | `$BASE/NGPJ` (0) | `None` | `$BASE/NGPJ` | jj only | agree |
| `PJG-i` | `$BASE/PJG` (0) | `$BASE/PJG/sub` (0) | `$BASE/PJG` | `$BASE/PJG/sub` | differ, jj inside git | agree |
| `PJG-o` | `$BASE/PJG` (0) | *empty* (1) | `$BASE/PJG` | `None` | git only | agree |

The Roots column records the raw comparison only. **Dual-root equality is
necessary but not sufficient for `colocated`, and inequality is not sufficient
for either `nested-` arm.** `CR` and `NGJ-o` both have equal roots and classify
as `main`, because `classify_checkout`'s `colocated` arm also requires
`jj_secondary && git_worktree` (`scripts/vcs-common.sh:242-247`); a jj *main*
workspace nested in a git repository has differing roots and still classifies as
`main`. A classifier reads this column together with the jj-secondary bit
(Query 5) and the linked-worktree bit (Query 2), never alone — see the
`classify_checkout` records below for the composite answer.

### `classify_checkout` records, for cross-reference

The composite oracle, for the transitional `detection.rs` comparison and for
0169's later use. `BOUNDARY` is empty for `main` and `none`, per contract.

| Fixture | KIND | BOUNDARY | JJ_PARENT | GIT_PARENT |
| --- | --- | --- | --- | --- |
| `CR` | `main` | *empty* | *empty* | *empty* |
| `CG` | `colocated` | `$BASE/CG` | `$BASE/jjparent` | `$BASE/gitparent-cg` |
| `JS-r` / `JS-s` | `jj-secondary` | `$BASE/JS` | `$BASE/jjmain-colocated` | *empty* |
| `PG-r` / `PG-s` | `main` | *empty* | *empty* | *empty* |
| `NJG-i` | `nested-jj-in-git` | `$BASE/NJG/sub` | `$BASE/jjmain-colocated` | `$BASE/NJG` |
| `NJG-o` | `main` | *empty* | *empty* | *empty* |
| `NGJ-i` | `nested-git-in-jj` | `$BASE/NGJ/sub` | `$BASE/NGJ` | `$BASE/gitparent-ngj` |
| `NGJ-o` | `main` | *empty* | *empty* | *empty* |
| `WT-l` | `git-worktree` | `$BASE/WT` | *empty* | `$BASE/main` |
| `WT-m` | `main` | *empty* | *empty* | *empty* |
| `SM-1` / `SM-2` / `SM-s` / `SO` | `main` | *empty* | *empty* | *empty* |
| `BARE` | `none` | *empty* | *empty* | *empty* |
| `NONE` | `none` | *empty* | *empty* | *empty* |
| `PJ` | `main` | *empty* | *empty* | *empty* |
| `PJS` | `jj-secondary` | `$BASE/PJS` | `$BASE/PJ/a/b/c` | *empty* |
| `NGPJ-i` | `nested-git-in-jj` | `$BASE/NGPJ/sub` | `$BASE/NGPJ` | `$BASE/gitparent-ngpj` |
| `NGPJ-o` | `main` | *empty* | *empty* | *empty* |
| `PJG-i` | `nested-jj-in-git` | `$BASE/PJG/sub` | `$BASE/jjmain-pure` | `$BASE/PJG` |
| `PJG-o` | `main` | *empty* | *empty* | *empty* |

The last five rows were measured on **2026-08-03** by running `classify_checkout`
directly against the fixtures (an earlier draft omitted them, so the two nested
arms' pure-jj variants had no composite expected value). This table now covers
all 24 pairs.

`find_git_main_worktree_root` returns an **ordinary root with exit 0** for every
non-submodule, non-bare checkout, and the **superproject** for a submodule
(`$BASE/super` from `SM-1`, `$BASE/super/mid` from `SM-2`) — confirming the work
item's warning that the underlying `git rev-parse`, not the shell wrapper, is
the query-3 oracle. For `BARE` it returns exit 1 and empty stdout.

### Extended fixtures (measured 2026-08-03)

Ten fixtures added after plan review. **Oracle columns only.** `gix`/`jj-lib` are
not in the workspace until Phase 1, so the library columns
(`is_bare()`, `kind()`, raw `common_dir()`, `main_repo().workdir()`) are **not
measurable yet** and are filled in Phase 2 against these oracle values. They are
kept in a separate table rather than folded into Queries 1-6 for exactly that
reason — folding them in would create half-empty rows that read as gaps.

`dirs` is the absolutised `--git-dir` vs `--git-common-dir` comparison, i.e. the
`worktree.linked` oracle.

| Key | Shape | Start directory | `--is-bare` | `--show-toplevel` (exit) | `jj workspace root` (exit) | `--git-dir` | `--git-common-dir` (raw) | `dirs` | `--show-superproject…` (exit) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `JS-in` | jj secondary workspace **inside its own colocated main** | `$B/JSIN/workspaces/inner` | `false` | `$B/JSIN` (0) | `$B/JSIN/workspaces/inner` (0) | `$B/JSIN/.git` | `../../.git` | **equal** | *empty* (0) |
| `SM-m` | submodule whose path contains a `modules` segment | `$B/supm/modules/foo` | `false` | `$B/supm/modules/foo` (0) | *empty* (1) | `$B/supm/.git/modules/modules/foo` | same | equal | `$B/supm` (0) |
| `SM-w` | linked worktree **of a submodule** | `$B/smw` | `false` | `$B/smw` (0) | *empty* (1) | `$B/supw/.git/modules/mid/worktrees/smw` | `$B/supw/.git/modules/mid` | **differ** | *empty* (0) |
| `SM-wt` | submodule initialised **inside a linked worktree** of the superproject | `$B/supwt-wt/sub` | `false` | `$B/supwt-wt/sub` (0) | *empty* (1) | `$B/supwt/.git/worktrees/supwt-wt/modules/sub` | same | equal | `$B/supwt-wt` (0) |
| `RF` | reftable ref backend | `$B/RF` | `false` | `$B/RF` (0) | *empty* (1) | `.git` | `.git` | equal | *empty* (0) |
| `S256` | sha256 object format | `$B/S256` | `false` | `$B/S256` (0) | *empty* (1) | `.git` | `.git` | equal | *empty* (0) |
| `HOSTILE` | plain git + adversarial `.git/config` | `$B/HOSTILE` | `false` | `$B/HOSTILE` (0) | *empty* (1) | `.git` | `.git` | equal | *empty* (0) |
| `D1` | `.jj/repo` pointer → **deleted** directory | `$B/D1` | *empty* (128) | *empty* (128) | *empty* (1) | n/a | n/a | n/a | *empty* (128) |
| `D2` | `.git`-file worktree, gitdir target **removed** | `$B/D2` | *empty* (128) | *empty* (128) | *empty* (1) | n/a | n/a | n/a | *empty* (128) |
| `D3` | `.jj/repo` pointer → **existing non-store** directory | `$B/D3` | *empty* (128) | *empty* (128) | **`$B/D3` (0)** | n/a | n/a | n/a | *empty* (128) |

Composite oracle and helper results:

| Key | KIND | BOUNDARY | JJ_PARENT | GIT_PARENT | `find_jj_main…` (exit) | `find_git_main…` (exit) |
| --- | --- | --- | --- | --- | --- | --- |
| `JS-in` | `jj-secondary` | `$B/JSIN/workspaces/inner` | `$B/JSIN` | *empty* | `$B/JSIN` (0) | `$B/JSIN` (0) |
| `SM-m` | `main` | *empty* | *empty* | *empty* | *empty* (1) | `$B/supm` (0) |
| `SM-w` | `git-worktree` | `$B/smw` | *empty* | **`$B/supw/.git/modules`** | *empty* (1) | **`$B/supw/.git/modules`** (0) |
| `SM-wt` | `main` | *empty* | *empty* | *empty* | *empty* (1) | `$B/supwt-wt` (0) |
| `RF` / `S256` / `HOSTILE` | `main` | *empty* | *empty* | *empty* | *empty* (1) | (root) (0) |
| `D1` / `D2` | `none` | *empty* | *empty* | *empty* | *empty* (1) | *empty* (1) |
| `D3` | **`jj-secondary`** | `$B/D3` | ***empty*** | *empty* | ***empty* (1)** | *empty* (1) |

What these settle:

1. **`JS-in` confirms the plan's claim exactly.** The workspace carries `.jj` only
   (no `.git`), `--git-dir` escapes upward to the colocated main's `.git`, and
   `--git-common-dir` absolutises equal so it is **not** a worktree. Dual roots
   *differ* (`$B/JSIN` vs `$B/JSIN/workspaces/inner`) yet the classification is
   `jj-secondary`, **not** `nested-jj-in-git`, because `jj_main_root` and
   `git_main_root` are both `$B/JSIN` and that arm's `!=` guard fails. This is the
   shape this repo runs in daily and the one where differing dual roots point at
   the wrong arm.

2. **`SM-m` confirms the revised `superproject` rule.** `git_dir()` is
   `$B/supm/.git/modules/modules/foo`; the innermost `modules` has parent
   `$B/supm/.git/modules`, not a repository, so the scan continues outward to the
   next `modules`, whose parent `$B/supm/.git` opens — yielding `$B/supm`, the
   oracle value. A bare `rposition` would stop at the first candidate and return
   absence.

3. **`SM-wt` downgrades the superproject-accuracy concern.** Git does **not** put
   the submodule's git dir under the common dir's `modules/` as feared; it puts it
   under `worktrees/<id>/modules/sub`, and `--show-superproject-working-tree`
   returns `$B/supwt-wt` — the **worktree**, matching `find_git_main_worktree_root`.
   The plan's scan anchors on the innermost `modules` whose parent is
   `$B/supwt/.git/worktrees/supwt-wt`; if `gix::open` accepts a linked-worktree
   gitdir, its `workdir()` is `$B/supwt-wt` and the rule is correct.
   **Phase 2 must confirm `gix::open` on a worktree gitdir**; that is the one
   remaining unknown, not the layout.

4. **`SM-w` exposes a shell-oracle quirk worth inheriting knowingly.**
   `find_git_main_worktree_root` returns `$B/supw/.git/modules` — a `.git/modules`
   path, not a worktree root — and `classify_checkout` puts it in `GIT_PARENT`.
   Also `--show-superproject-working-tree` is *empty* for a linked worktree of a
   submodule. So if the library's `kind()` reports `Submodule` here, the
   superproject derivation would return a path where git returns nothing.
   `dirs differ`, so `worktree.linked = true` is correct either way.

5. **`S256` breaks the revision-shape assertion.** `git rev-parse HEAD` is **64
   hex characters** in a sha256 repository, while
   `cli/vcs-adapters/tests/detection.rs:84-86`'s `is_full_revision_id` asserts 40.
   Any `RepoFacts.revision` validation must either accept both widths or record
   sha256 as unsupported. `RF` needs no such accommodation — the CLI reports
   `--show-ref-format=reftable` and behaves normally.

6. **`HOSTILE` runs nothing, as suspected.** With `core.pager`, `core.fsmonitor`,
   `diff.external`, `filter.*.clean`/`smudge`, an `alias.*` and an `include.path`
   chain all pointed at marker-writing commands, **none of the seven ran** under
   `--show-toplevel`, `--git-dir` or `--is-bare-repository`. The oracle does not
   reach that machinery either, so `HOSTILE`'s value is as a **regression guard
   for the APIs 0169 adds** (`status`/`log` reach blob-diff, attributes and
   pagers), not as evidence for this story's call set. State the claim that
   narrowly in `tasks/README.md`.

7. **`D3` is the load-bearing degenerate, and the defensive invariant works.**
   `jj workspace root` **succeeds** (exit 0, `$B/D3`) on a pointer targeting an
   existing non-store directory, while `find_jj_main_workspace_root` returns exit
   1 — its `[ -d "$candidate/.jj/repo" ]` post-condition firing — so
   `classify_checkout` degrades gracefully to `jj-secondary` with an **empty**
   `JJ_PARENT` rather than emitting a wrong-but-non-empty root. The library must
   reproduce this: `jj_workspace_root` → `Ok(Some($B/D3))`, `jj_repository` →
   `Err`. `D1` and `D2` report plain absence from both CLIs (`none`), so their
   expectation is `Ok(None)`, **not** `Err` — correcting the blanket
   "expected outcome is `Err`" in the construction table.

8. **`jj workspace add` does not create intermediate directories** at 0.43. A
   bare `jj workspace add --name inner <root>/workspaces/inner` fails with
   `Error: Cannot access … / No such file or directory` unless `workspaces/`
   exists. The `JS-in` builder must `mkdir -p` the parent first.

### Recorded divergences

- **`GIT_DIR` scrub asymmetry** — `classify_checkout` reads `GIT_DIR`
  unscrubbed at `scripts/vcs-common.sh:206-215` while
  `find_git_main_worktree_root` scrubs at `:130-135`. `InProcessProbe` is
  immune across the poisoning matrix in Phase 2 §3; this is the one
  pre-authorised divergence.
- **Absence-signal conflation** — `superproject()` maps both of
  `--show-superproject-working-tree`'s absence signals (exit 0 + empty, and
  exit 128) to `Ok(None)`. Deliberate; nothing in the taxonomy needs the
  distinction.
- **The library's absence cells are unfenceable by the environment.** The
  oracle side's exit-128s come from `GIT_CEILING_DIRECTORIES`; `gix::discover`
  reads no environment and walks to the filesystem root, so the library side's
  `None` holds only under the no-`.git`-ancestor precondition the harness
  asserts (Phase 2 §2). This is a property of the mapping's *provenance*, not a
  behavioural divergence.
- **Library error conditions mapped to `Ok(None)` rather than `Err`.** The
  **rule** is fixed here so 0169 inherits a contract rather than
  reverse-engineering it, and so Phase 2's assertions can be written before the
  queries exist: **only the not-found-shaped variant of each library error maps
  to `Ok(None)`; every other variant is `Err`.** Concretely —
  `WorkspaceLoadError::NoWorkspaceHere` and `RepoDoesNotExist` → `Ok(None)`
  (this is what Query 4 already folds into absence); `DecodeRepoPath`, any
  `PathError`/IO failure, a canonicalisation failure, and a failed
  `<main_root>/.jj/repo` post-condition → `Err`. On the gix side, "no repository
  found" → `Ok(None)`; an open/config/odb failure on a repository that *was*
  found → `Err`. Phase 2 confirms the exact variant names against the crates and
  records them here; the **partition rule does not change**, only its spelling.
  The measured extended fixtures pin the observable half already: `D1`/`D2` →
  `Ok(None)`, `D3` → `Ok(Some)` from `jj_workspace_root` and `Err` from
  `jj_repository`.
- **`InProcessProbe::revision` returns `None` for `VcsKind::Jj`** by design —
  spike-established that jj-lib 0.43 exposes no read-only settings-free route to
  the working-copy commit id. Warn-logged, and handed to 0185.
- **sha256 repositories carry 64-hex revision ids.** Measured on `S256`:
  `git rev-parse HEAD` is 64 hex characters, while
  `cli/vcs-adapters/tests/detection.rs:84-86`'s `is_full_revision_id` asserts 40.
  Any `RepoFacts.revision` validation must accept both widths or record sha256 as
  unsupported — a decision 0185 inherits, since after its switch a user on a
  sha256 repository is affected. `RF` (reftable) needs no accommodation at the
  CLI level; whether gix 0.85 serves either format is the Phase 2 question.
- **`find_git_main_worktree_root` returns a `.git/modules` path** for a linked
  worktree of a submodule (`SM-w`, measured), which `classify_checkout` then
  reports as `GIT_PARENT`. That is a pre-existing shell-oracle quirk, not
  something `InProcessProbe` should reproduce; `WorktreeFacts.main_worktree_root`
  carries the library value and does not claim parity there.
- **No other divergence was observed** across the 24 (fixture, start
  directory) pairs and six queries. Every cell above reads *agree*, after the
  `PJG-i` common-dir correction noted under Query 2.

#### Confirmed during implementation (2026-08-03, all 34 pairs)

The delivered queries were measured against the live CLIs across the whole
matrix. Every cell agrees except the three below, each deliberate.

- **`superproject` gates on the worktree comparison, not on `kind()`.** The plan
  proposed gating the derivation on `kind() == Submodule`. Measured, that is
  wrong in *both* directions and the two extended fixtures added after plan
  review are exactly what exposed it. `SM-w` (a linked worktree of a submodule)
  reports `kind() == Submodule`, so the gate admits it and the scan returns
  `$B/supw` where git returns **empty**. `SM-wt` (a submodule inside a linked
  worktree) does **not** report `Submodule`, so the gate rejects it and the scan
  never runs, where git returns `$B/supwt-wt`. The delivered gate is the
  oracle's own discriminator — canonicalised `git_dir() != common_dir()` means
  "linked worktree, no superproject" — after which the `modules` scan runs
  unconditionally. All seven submodule shapes then agree.
- **`gix::open` accepts a linked-worktree gitdir — confirmed.** This was the one
  open question Phase 2 was asked to settle for query 3. `SM-wt` resolves
  through `$B/supwt/.git/worktrees/supwt-wt`, whose `workdir()` is
  `$B/supwt-wt`, matching the oracle.
- **gix 0.85 cannot read a sha256 repository**, answering the open question.
  `S256` returns `Err` from every gix-backed query —
  `open::Error::Config(ConfigTypedString { key: "extensions.objectFormat" })` —
  rather than misreading it. `Err` is the correct outcome under the partition
  rule (a repository *is* here and the pinned library cannot answer) and is
  precisely why the queries carry an error channel; collapsing it to `Ok(None)`
  would report "no repository" for a valid checkout. **`RF` (reftable) reads
  normally.** 0185 inherits the sha256 consequence, since its switch is what
  exposes a user on such a repository.
- **`D1` returns `Err`, not `Ok(None)` — a deliberate departure from this plan's
  own extended-fixture cell.** A `.jj/repo` pointing at a deleted directory makes
  jj-lib's loader fail `dunce::canonicalize` and return `WorkspaceLoadError::Path`,
  which the partition rule maps to `Err`. The `Ok(None)` recorded above was read
  off the *CLI's* exit code, and the CLI conflates. There **is** a jj workspace
  at `D1` — `.jj` is a directory carrying a repo pointer — and it is broken, so
  reporting absence is exactly the "a corrupt store must not read as no VCS
  here" failure the error channel exists to prevent. The partition rule wins;
  the cell was wrong. `D2` (no `.jj` at all) remains `Ok(None)`, and `D3`
  remains `Ok(Some)` from `jj_workspace_root` with `Err` from `jj_repository`,
  both as recorded.
- **`superproject_of` does not canonicalise.** The plan put every returned path
  through the canonicalisation choke point. Applied inside the scan that makes
  the injected-probe unit tests impossible — they drive it over paths that do
  not exist, and canonicalisation fails. Canonicalisation moved to the probe
  closure the method supplies, which is the only place a real filesystem path
  appears. The injected probe also returns `Result<Option<PathBuf>, Error>`
  rather than the planned `Result<bool, Error>`, so the anchor is opened once
  rather than twice; the three-state distinction the plan required is preserved.

---

## Phase 1: Dependencies, Policy Gates and the Boundary-Safe Ports

### Overview

Land both dependency trees, the licence exception, the two-clause import rule
and every committed invariant check, plus `InProcessProbe` implementing
`RepoRoot` and `VcsProbe`. After this phase a dependency-policy objection is
reviewable in isolation and the boundary rule is proven.

### Changes Required

#### 1. Workspace dependency pins

**File**: `cli/Cargo.toml`
**Changes**: two exact pins in `[workspace.dependencies]`, following the
`vergen`/`vergen-gitcl` matched-pair precedent (`:17-21`).

```toml
# jj-lib and gix are a matched pair, pinned asymmetrically on purpose.
#
# jj-lib is EXACT: it declares its API unstable, this design leans on its
# workspace-loader internals, and a patch release can shift its MSRV or its own
# gix requirement. Adopting one is a deliberate act, not a resolution outcome.
#
# gix is TILDE: ~0.85.0 permits 0.85.x only, which is the range jj-lib already
# requires (^0.85.0), so patch agility costs nothing here. gix 0.86 exists and a
# caret on a 0.x crate will not cross it, so requesting 0.86 would put two gix
# graphs in the lock.
#
# A jj-lib bump is a coordinated four-pin change: jj-lib, gix, the Rust
# toolchain (jj-lib's MSRV moved 1.85 -> 1.88 -> 1.89 across eight releases) and
# the mise jj CLI pin, which writes the format jj-lib reads.
jj-lib = "=0.43.0"
gix = "~0.85.0"
```

The asymmetry is the point. `~0.85.0` lets a `gix` RustSec fix be adopted with a
lock update rather than a pin edit, which matters under
`cli/deny.toml:19-31`'s `unmaintained = "all"` + `yanked = "deny"` — otherwise an
`advisories.ignore` entry becomes the cheapest response to an advisory. The
single-graph property is unaffected: it comes from jj-lib's own `^0.85.0` plus
cargo's range unification, not from exactness, and `Cargo.lock` supplies the
exactness either way. `jj-lib` keeps its `=` pin because the agility argument
does not apply to the crate whose declared-unstable internals the design
depends on, and because a transitive advisory anywhere in *its* closure is
adoptable via `cargo update -p <crate>` without touching the pin at all.

The graph test asserts `gix` matches `0.85.\d+` (consistent with the tilde) and
`jj-lib` at exactly `0.43.0`. The break-glass procedure for a yank or advisory
in the closure is documented in `tasks/README.md` (below).

**File**: `cli/vcs-adapters/Cargo.toml`
**Changes**: consume both. `tempfile` stays in `[dev-dependencies]` and
`bash-parity` keeps its current empty definition — the fixture builders live in
`cli/vcs-test-support` from Phase 2, so nothing needs them outside test targets,
and widening `bash-parity` to enable a dependency would change its meaning for
no gain.

`cli/vcs-test-support` and `cli/vcs-adapters`' `[dev-dependencies]` edge to it
are created in **Phase 2**, together with the `members` entry, the lock change
that entry forces, and the `[[bin]]` declaration for the reference artefact
Phase 2 §3 needs. Key Discovery 13 records that this workspace has zero
path-based `[dev-dependencies]` today, so that edge is novel and is stated
explicitly rather than implied. Phase 1 therefore has no dependency on the
new member, and the "passes with the new member" gate criterion belongs to
Phase 2.

Phase 1 also extracts `marker_kind(root)` in `cli/vcs-adapters/src/lib.rs` (a
crate-private free function that `CommandProbe::kind` and
`InProcessProbe::kind` both call) and `walk_up(start, predicate)`, so Phase 2
does not have to dedup code Phase 1 just landed. Both live in a crate-private
module that **survives 0185's deletion of `CommandProbe`/`MarkerWalkRoot`**:
`MarkerWalkRoot::discover` and `CommandProbe::kind` delegate *to* them, not the
other way round, so 0185's deletion stays mechanical and needs no re-homing step.

```toml
[dependencies]
vcs = { path = "../vcs" }
gix = { workspace = true }
jj-lib = { workspace = true }
tracing = { workspace = true }
```

`gix` takes **default features** — network transports are excluded by default,
so no TLS stack enters the graph, and `attributes` (which `submodules()` needs)
is present. `jj-lib` takes default features, which is what pulls `gix` 0.85.

**File**: `cli/Cargo.lock` — regenerated and committed in the same change,
because clippy runs `--locked`.

#### 2. Licence exception

**File**: `cli/deny.toml`
**Changes**: the workspace's first `[[licenses.exceptions]]` entry, placed after
`confidence-threshold`.

```toml
# uluru is gix-pack's LRU object cache (reached from both gix and gix-odb) and
# is not feature-gatable out of the graph. MPL-2.0 is file-level weak copyleft:
# we ship no modifications, so §3.1 is satisfied trivially. The obligation that
# binds is §3.2 — distributing the Executable Form requires telling recipients
# how to obtain the Source Form — and this crate reaches the published
# accelerator-visualiser binary, so it is discharged by the third-party licence
# file staged into the release payload, not by the absence of modifications.
# Adopted for the library-backed VCS adapter (work item 0188).
[[licenses.exceptions]]
crate = "uluru"
allow = ["MPL-2.0"]
```

The work-item reference is a deliberate exception to the repo's
no-references-in-comments convention: the acceptance criterion requires the
comment to cite this work item.

This is the workspace's first MPL crate, and it enters the closure of a
*publicly distributed, signed, download-on-first-use* binary rather than a
dev-only artefact. Nothing in the release pipeline carries third-party licences
today (`_release_uploads()` enumerates binaries, signatures and the manifest
only), so the reasoning must not stop at "we ship no modifications" — that
answers §3.1 and sets the template every future exception will copy. Resolve by
either generating a third-party attribution artefact (cargo-about or
cargo-bundle-licenses) and adding it to the release upload set, or recording a
verified finding that dead-code elimination leaves no `uluru` code in any
shipped binary, with the exception comment pointing at whichever holds.

**File**: `cli/deny.toml`
**Changes**: extend `[bans].deny` (`:65-70`) with **`gix-credentials` and
`curl-sys` only**. Both were confirmed absent from a real resolved lock (spike,
2026-08-03).

**`gix-transport` and `gix-protocol` must NOT be banned — they are present.** A
spike crate depending on `jj-lib = "=0.43.0"` resolves both into the graph
(`gix-transport 0.51.0`, `gix-protocol 0.63.0`, and both compile), because
jj-lib's default features reach them even with no network client enabled.
Banning either would fail `deny:check` on the first run, exactly as a bare
`rustls` ban would. An earlier draft of this section named them as "confirmed
absent" without checking — they are not.

What this means for the property: the absence of a *TLS stack* (Key Discovery 9)
is real and worth asserting, but "no gix transport crates in the graph" is
**false** and cannot be a gate. The enforceable statements are:

- `[bans].deny` on `gix-credentials`, `curl-sys`, plus the existing
  `native-tls`/`openssl`/`openssl-sys` — all verified absent.
- The subtree pytest asserts no `rustls`/`openssl`/`native-tls`/`curl-sys` under
  `-p vcs-adapters`, looped over all five configured targets.
- The **feature-set** assertion (below) carries the real weight: `gix`'s
  `blocking-network-client`, `async-network-client` and
  `blocking-http-transport-*` features must be **off**. That is what keeps the
  transport crates inert, and it is checkable where their presence is not.

**`rustls` must NOT be added.** It is a first-party workspace dependency:
`cli/Cargo.toml:35` pins it `=0.23.41` and `cli/launcher/Cargo.toml:31` consumes
it directly, and the section's own comment reads "rustls only" — meaning rustls
is the *chosen* TLS stack, not a banned one. A bare `{ crate = "rustls" }` entry
would fail `mise run deny:check` on the first run, and `deny:check` is in
`check-supply-chain`, which is in `prerelease.needs`. The same trap applies to
any gix package that is in the closure with its client features off, which is
why each name above is verified absent first rather than assumed.

If reachability of `rustls` *from the VCS subtree* is ever the concern, express
it wrapper-scoped (`{ crate = "rustls", wrappers = ["reqwest", "launcher"] }`)
so a new direct edge fails while the sanctioned path passes — not as a bare ban.

`[bans]` is evaluated against all five configured targets (`:11-17`, enumerated
deliberately "so the ubuntu graph a banned edge could hide in is always
evaluated"). The pytest below additionally loops its
`cargo tree -e features -p vcs-adapters` assertion over `--target` for each of
the four release triples plus `x86_64-unknown-linux-gnu`, because the work item's
criterion is that the **`vcs-adapters` subtree** carries no TLS stack — a
strictly different and narrower proposition than `[bans]`' whole-graph
absence, which is false here for `rustls`. Confirm `deny:check` is green with
these entries before ticking the Phase 1 criterion.

#### 3. The two-clause import rule

**File**: `cli/pup.ron`
**Changes**: a rule scoped to the new module only — `CommandProbe` legitimately
spawns, so `vcs_adapters` as a whole must stay unruled.

`tracing` is on the permit list because `VcsProbe::revision`'s contract requires
it: "A caller cannot distinguish the two; an adapter is expected to log the
failure" (`cli/vcs/src/lib.rs:63-66`). Omitting it would make `library` the
crate's only silently-failing adapter, against a sibling that warn-logs all six
of its failure paths. `kernel::Error` is **not** on the list — `kernel` is not a
dependency of `vcs-adapters`, so permitting it would be inert.

Two properties this rule does **not** deliver, stated so neither the rule
comment nor `tasks/README.md` overclaims:

- `RestrictImports` resolves `use` paths in first-party source. An inline
  fully-qualified `std::process::Command::new(…)` with no `use` is unaffected,
  and the permitted `^crate(::|$)` reaches `crate::CommandProbe`, which spawns
  by design. The rule raises the cost of spawning; it does not make it
  impossible. The zero-spawn *property* is established by the Phase 3 harness.
- The rule says nothing about `gix` or `jj-lib`. Key Discovery 8 records that
  jj-lib's defaults enable `attributes` and `blob-diff` on gix — the subsystem
  that runs clean/smudge filters, external diff drivers and pagers from
  **repository-controlled** configuration. Phase 3's harness therefore carries a
  hostile-configuration fixture (below) rather than relying on this rule.

**Alternative considered**: expressing the rule as `denied`-only, which
`cli/pup.ron:93-98` already chose for `config_command` precisely to avoid the
grouped-import false positive. That would drop the single-item-import tax at the
cost of the closed-world property. Kept as `allowed_only` because the module's
whole point is a small, enumerable import surface, and the constraint is
recorded in a `//!` note at the top of `library.rs` so it is discoverable where
it binds rather than only in `pup.ron`.

```
        // The library-backed adapter reads git and jj in-process. A permit list
        // alone cannot express this because std::process sits inside the
        // permitted std, so the rule pairs a permit list with an explicit deny:
        // verified 2026-08-03 that the deny wins on overlap. Note the permit
        // list rejects grouped imports (cargo-pup resolves `use a::{b, c}` to
        // an empty module name), so this module writes one single-item `use`
        // per import.
        Module((
            name: "vcs_adapters_library_reads_in_process",
            matches: Module("^vcs_adapters::library($|::)"),
            rules: [
                RestrictImports(
                    allowed_only: Some([
                        "^(std|core|alloc)(::|$)",
                        "^gix(::|$)",
                        "^jj_lib(::|$)",
                        "^tracing(::|$)",
                        "^vcs(::|$)",
                        "^crate(::|$)",
                    ]),
                    denied: Some([
                        "^std::process(::|$)",
                    ]),
                    severity: Error,
                ),
            ],
        )),
```

#### 4. `InProcessProbe`

**File**: `cli/vcs-adapters/src/library.rs` (new)
**Changes**: the type and the two port implementations. Single-item imports
throughout.

```rust
use std::path::Path;
use std::path::PathBuf;

use vcs::RepoRoot;
use vcs::VcsKind;
use vcs::VcsProbe;

#[derive(Debug, Clone, Copy, Default)]
pub struct InProcessProbe;

impl InProcessProbe {
    fn boundary(start: &Path) -> Option<PathBuf> { /* .jj|.git existence walk */ }
    fn jj_workspace(start: &Path) -> Option<PathBuf> { /* .jj-only walk */ }
}

impl RepoRoot for InProcessProbe {
    fn discover(&self, start: &Path) -> Option<PathBuf> {
        Self::boundary(start)
    }

    fn repository_root(&self, working_copy_root: &Path) -> PathBuf { /* loader */ }
}

impl VcsProbe for InProcessProbe {
    fn kind(&self, root: &Path) -> VcsKind { /* marker existence, jj wins */ }

    fn revision(&self, root: &Path, kind: VcsKind) -> Option<String> { /* gix / jj-lib */ }
}
```

##### `revision`: jj is descoped to 0185 — spike-resolved 2026-08-03

The git half is `gix::discover(root)?.head_commit()?.id()`, in scope here.

**The jj half is descoped to 0185.** A spike (throwaway crate, `jj-lib =
"=0.43.0"`, Rust 1.90.0, resolver 3) established that jj-lib 0.43 offers **no
read-only, settings-free route to the working-copy commit id**. The chain needs
four links and two are blocked:

| Link | Settings-free? | Read-only? |
| --- | --- | --- |
| `DefaultWorkspaceLoaderFactory::create(root)` → `repo_path()` | ✅ | ✅ |
| workspace **name** (to index `View::get_wc_commit_id`) | ❌ / ⚠️ | **❌** |
| `SimpleOpHeadsStore::load(dir)` → `get_op_heads()` | ✅ | ✅ |
| `SimpleOpStore::load(path, RootOperationData)` → `read_operation` → `read_view` | ✅ | ✅ |

The op stores are genuinely settings-free (both take a path only; the trait
methods are `async` and jj-lib itself drives them with `pollster`). The
**workspace name** is the blocker, and both routes to it fail:

- `LocalWorkingCopy::load(...)` requires `&UserSettings` — it builds
  `TreeStateSettings::try_from_user_settings` (`local_working_copy.rs:2708-2716`).
  `CheckoutState`, which holds exactly the `operation_id` + `workspace_name` pair
  needed, is a **private** struct with a private `load`.
- `SimpleWorkspaceStore::load(repo_path)` is settings-free but **mutates the
  repository**. Verified empirically: with `.jj/repo/workspace_store` removed, the
  call recreated the directory and wrote an `index` file
  (`workspace_store existed before = false` → `exists after = true`). It is
  therefore unusable in a detection probe, which must be read-only and must work
  on a read-only filesystem. Its trait also exposes only
  `get_workspace_path(name)`, with no listing API, so a root→name inversion is not
  available even setting the write aside.

Reading `.jj/working_copy/checkout`'s protobuf by hand is the only remaining
path, and that is precisely the "leaning on private internals" this design set
out to avoid — a private wire format with no compatibility promise.

**Consequences, applied throughout this plan:**

- `InProcessProbe::revision` returns `None` for `VcsKind::Jj`, and warn-logs that
  the mechanism is unavailable rather than implying an empty repository.
- The crate-wide `UserSettings` / `Workspace::load` guard **stays crate-wide** —
  the spike is the evidence that nothing in scope needs it.
- Phase 3's parity criterion narrows **per `VcsKind`**: full `RepoFacts` parity
  for `VcsKind::Git`, and `root`/`name`/`kind` only for `VcsKind::Jj`. It is not
  dropped wholesale — the git path is achievable and
  `detection.rs:84-86` already pins a 40-hex id there.
- 0185 inherits the jj revision mechanism as named work, alongside the
  composition-root switch it already owns. Its options are a jj-lib version that
  exposes the checkout state publicly, an upstream request for one, or keeping
  `CommandProbe` for `revision` alone.

For the record, the discarded alternative: narrowing the `UserSettings` ban to
the detection paths would have put a settings chain abandoned after five
successive panics into code with no crash isolation, running inside
`cli/visualiser/server` and on the hook path after 0185's switch. The spike
removed the need to weigh that at all.

<details>
<summary>Original framing, kept because it records what was unknown</summary>

The **jj half has no established mechanism**, and two of this plan's own
constraints may make it unreachable:

- Reading jj's `@` commit id needs the repo loaded (`RepoLoader` /
  `Workspace::load`), which needs a `UserSettings` — the construct Phase 2's
  source guard forbids anywhere in `cli/vcs-adapters`.
- Git `HEAD` is not a substitute: a `--no-colocate` repository has no git HEAD
  at all, and in a colocated repository jj exports HEAD as `@-`, not `@`, so the
  ids differ from `CommandProbe`'s `jj log -r @ -T commit_id`.

Phase 3 requires `detection.rs` to produce **identical `RepoFacts`** from both
implementations, and `RepoFacts.revision` is asserted as a 40-hex id
(`cli/vcs-adapters/tests/detection.rs:84-86`), so the two constraints are
mutually unsatisfiable as drafted. Resolve by picking one:

Ranked, most preferred first:

1. **Find a settings-free jj-lib path** (an op-store / view read reached from
   `WorkspaceLoader::repo_path()`) and prove it in a short spike before Phase 1.
   Preferred if it exists — it keeps the guard crate-wide and Phase 3's
   criterion unchanged.
2. **Descope jj `revision` from 0188** — the **fail-safe** resolution, and the
   one to take if the spike cannot prove option 1. Return `None` for
   `VcsKind::Jj` and hand the mechanism to 0185 with the composition-root switch
   it already owns. Narrow Phase 3's parity criterion **per `VcsKind`** — full
   `RepoFacts` parity for `VcsKind::Git`, `root`/`name`/`kind` only for
   `VcsKind::Jj` — rather than dropping `revision` wholesale, which would also
   discard achievable protection on the git path where
   `gix::discover(root)?.head_commit()?.id()` is expected to match exactly and
   `detection.rs:84-86` already pins a 40-hex id.
3. **Narrow the `UserSettings` ban** from crate-wide to the detection paths.
   **Least preferred — this carries a real safety cost.** It puts `UserSettings`
   construction inside `revision`, and the ban exists because those defaults are
   private to jj-lib and were "discovered one panic at a time" — abandoned after
   five successive panics with the chain never exhausted. This code has no crash
   isolation (that is the containment delta this plan documents), and after 0185
   flips `facts` it runs inside `cli/visualiser/server` and on the hook path.
   Take it only with a committed `catch_unwind` or error boundary around the
   construction, and record why the wider statement was abandoned.

</details>

`kind` and `revision` gain oracle rows (`git rev-parse HEAD` for git shapes,
`jj log -r @ -T commit_id` recorded as the value 0185 must eventually produce,
plus the commitless-repository and colocated cases `detection.rs` already pins)
and are tested **in Phase 1** against the plain-git, colocated, commitless and
secondary-workspace fixtures — not first exercised two phases later.

Every arm of `revision`, `kind` and `repository_root` that cannot answer
warn-logs before returning, mirroring `CommandProbe`'s six labelled `warn!`
sites; legitimate absence (a repository with no commits) does not log.

`discover` is the marker walk; the boundary rule is satisfied by composition
rather than by new code, because `gix::open` at exactly the boundary performs no
upward walk. **`gix_discover::upwards::Options::ceiling_dirs` must not be used**
— it computes the ceiling height as
`start.strip_prefix(ceiling).components().count()` and discards height 0, then
tests `current_height > max_height` before incrementing, so no anchor confines
the walk.

`repository_root` resolves a jj secondary workspace through the loader rather
than by reading `.jj/repo` by hand, so the file-vs-directory rule has one
implementation.

**File**: `cli/vcs-adapters/src/lib.rs`
**Changes**: one line — `pub mod library;`. The module is unconditional so the
pup rule always applies; `facts` (`:224-227`) is untouched.

#### 5. Committed invariant checks

**File**: `tests/integration/deny/test_vcs_library_graph.py` (new)
**Changes**: modelled on `test_launcher_feature_graph.py:23-51`, over
`cargo tree -e features -p vcs-adapters`.

Asserts: `gix` resolves to a version matching `0.85.\d+` (a bare
single-version assertion would hold vacuously if jj-lib's `gix` feature were
off); no `gix` or `gix-*` package at more than one version; `jj-lib` at `0.43`;
and no package in the resolved graph declares a `rust-version` above the pinned
1.90.0 — catching the `kstring` class of trap (Key Discovery 11) directly rather
than depending on resolver 3's `incompatible-rust-versions = "fallback"`, which
is a *preference*, not a hard constraint. **The MSRV comes from
`cargo metadata --locked --format-version 1`, whose per-package `rust_version`
field carries it — `Cargo.lock` does not record MSRV at all** (zero occurrences
of `rust-version` in the committed lock; the format holds only `name`, `version`,
`source`, `checksum` and `dependencies`), so a lock-parsing implementation would
find nothing and pass vacuously on precisely the mechanism this check exists to
backstop. The comparison is semver-ordered against the workspace `rust-version`
(`cli/Cargo.toml:9`), and the test carries a non-vacuity case so a graph with no
declared `rust_version` anywhere cannot pass silently. Duplicate detection reads
`cli/Cargo.lock` directly, because the repo's `multiple-versions` policy is
`warn` (`cli/deny.toml:57`) and would not fail on its own.

The `gix` **enabled feature set** is asserted too, following the
`_PRESENT`/`_ABSENT` shape of `test_launcher_feature_graph.py` — present:
`attributes`, `blob-diff`, `index`, `max-performance-safe`, `sha1`, `zlib-rs`;
absent: `blocking-network-client`, `async-network-client`, the
`blocking-http-transport-*` family, `credentials`. Version assertions alone are
insufficient now the pins are tilde ranges taking **default** features: gix
republishes its `[features] default` with every patch and jj-lib's own patches
can change what it enables, so the effective surface is upstream-controlled
within the permitted range. This is the same reasoning that made
`test_launcher_feature_graph.py` exist alongside `deny.toml`'s name-based bans,
and that pinned `reqwest` exactly "so a patch cannot re-scope the DNS feature".

It also **snapshots the build-script- and proc-macro-carrying crates** in the
closure, so an addition is a visible diff. The release workflow already
documents build-script trust as a live concern — "the Prepare step above runs
cargo zigbuild over untrusted transitive build scripts, so the secret is never
in the environment during compilation" (`.github/workflows/main.yml:412-414`) —
and this change grows that set by ~56 packages, executing new `build.rs` and
proc-macro code on the release runner and on every developer machine. A future
lock regeneration could otherwise add one silently.

The transport prohibition lives in `cli/deny.toml`'s `[bans].deny` rather than
here, because this test is host-target-scoped; see above.

**File**: `tests/unit/tasks/test_vcs_pin_lockstep.py` (new)
**Changes**: modelled on `test_msrv_coherence.py:30-38` — a bare pytest, no
invoke task. Asserts `mise.toml`'s `jj` pin and `cli/Cargo.toml`'s `jj-lib` pin
share a major.minor; that the `mise.toml` pin retains its lockstep comment; and
that the `gix` pin comment in `cli/Cargo.toml` and the `uluru` exception comment
in `cli/deny.toml` are both present and non-empty. The work item requires those
comments, and comments are the first thing lost when a contributor regenerates
either file — which the Migration Notes anticipate happening on sibling
contention.

This checks *declarations in files*. The runtime half — that the `jj` binary
actually building the fixtures matches the pin — is asserted by the fixture
harness (Phase 3), not here.

Neither adds a mise leaf, so `tests/unit/tasks/test_mise.py`'s topology
assertions are untouched.

#### 6. Documentation

**File**: `tasks/README.md`
**Changes**: extend the entity-gate paragraph (`:38-42`, which enumerates
`deny:check` and `pup:check` and would otherwise go stale) to name the new
`vcs-adapters` import rule, and add a `### Library-backed VCS dependency pins`
subsection alongside "Executable-bit invariant" (`:67`) describing the four-pin
coupling and both committed checks.

That subsection also documents the **break-glass procedure**, because both
transitive trees enter `cargo deny`'s advisories scope under
`unmaintained = "all"` with `yanked = "deny"` (`cli/deny.toml:19-31`), and the
advisory DB is fetched fresh every run. One upstream `unmaintained` or `yanked`
advisory anywhere in a ~60-crate closure that *no code in the repo calls* turns
`check-supply-chain` red for every unrelated PR — and that job is in
`prerelease.needs`, so it also stops releases. Recovery is a scoped, dated
`[advisories].ignore` entry following the existing `RUSTSEC-2026-0118/0119`
precedent (`:26-31`), with a review-by date. Writing it down makes recovery a
known five-minute action rather than an investigation under release pressure.

**The break-glass is scoped to `unmaintained`, `yanked` and `notice` classes
only.** A `vulnerability`-class advisory requires the escalation path — upgrade,
patch, or vendor — never an ignore, regardless of release pressure, because this
closure reaches the publicly distributed signed `accelerator-visualiser` binary.
`cli/deny.toml`'s `ignore` list is flat with no class distinction, so the scoping
has to be written down or the pre-authorised action silently covers every class:
a documented five-minute suppression is exactly what gets applied reflexively to
a real vulnerability. cargo-deny does not enforce review-by dates either — the
two existing entries say "Remove when reqwest adopts 0.26" with nothing checking
it — so `tests/integration/deny/` gains an assertion that every `ignore` entry
carries a machine-readable review-by date and that none is in the past.

The same subsection documents the **licence-side** failure, which has no `ignore`
mechanism at all: `[licenses].allow` is deliberately "pruned to exactly the
licenses the current dependency closure carries" (`:35-40`), so a transitive
crate acquiring or replacing a licence is a hard failure needing either an
`allow` addition (permissive) or a justified `[[licenses.exceptions]]` (copyleft),
with the uluru entry as the template.

It further records the division of labour between the two enforcement
mechanisms this plan adds, so a contributor knows which is authoritative:
**cargo-pup owns import prohibitions**; **the `tasks/` source guard owns usage
prohibitions imports cannot express** — `RestrictImports` resolves `use` paths,
so a fully-qualified `jj_lib::settings::UserSettings::from_config(…)` or a
`Workspace::load` method call is invisible to it. That is the whole
justification for the extra Python machinery over a one-line `denied` clause.

### Success Criteria

#### Automated Verification

- [x] `mise run deny:check` passes with the `uluru` exception
- [x] `mise run pup:check` passes
- [x] `mise run cli:check` passes (clippy `--locked`, pedantic + nursery)
- [x] `mise run test:integration:deny` passes, including the new graph test
- [x] `mise run test:unit:tasks` passes, including the new lockstep test
- [x] Unit test: `InProcessProbe::discover` returns the boundary, never an
      ancestor, on the colocated and `.git`-file shapes `detection.rs` already
      builds — plus the paired negative assertion that an unbounded
      `gix::discover` on the same fixture *does* escape to the parent repository.
      The **both-nesting-directions** form of this criterion moves to Phase 2,
      which owns the nesting fixtures; Phase 1 must not duplicate them
- [x] `kind` and `repository_root` agree with `CommandProbe` on the plain-git,
      colocated, commitless and jj-secondary fixtures; `revision` agrees for
      `VcsKind::Git` and returns `None` (warn-logged) for `VcsKind::Jj`, per the
      spike-resolved descope above
- [x] Non-vacuity is **committed**, not demonstrated by hand: probe cases in
      `tests/integration/pup/test_import_rule.py` for the new rule — a
      `std::process::Command` import rejected with `"is denied"` and the rule
      name present, a compliant single-item-import module as the positive
      control (catching a rule whose scope silently matched nothing, which
      0169's module rename would otherwise cause), and a grouped-import case
      pinning the `use a::{b, c}` behaviour
- [x] The lockstep test also asserts the `gix` pin comment in `cli/Cargo.toml`
      and the `uluru` exception comment in `cli/deny.toml` are present and
      non-empty
- [x] `mise run` green end to end

#### Manual Verification

- [x] `cli/vcs/src/**` is unmodified (`jj diff --stat`)
- [ ] The `accelerator-visualiser` musl-static size is recorded before and after
      this phase, so a later regression has a baseline (the trees enter its
      closure via `vcs-adapters` → `corpus-adapters` → `visualiser/server`)
- [ ] Cold-cache `build:server:dev` is timed against
      `test-visual-regression`'s `timeout-minutes: 20`
      (`.github/workflows/main.yml:125`), and the budget raised in this same
      change if the margin is thin — this plan guarantees a cold cache by
      changing the lock
- [x] `CommandProbe` and `MarkerWalkRoot` have no new methods
- [x] `vcs_adapters::facts` still names `MarkerWalkRoot`/`CommandProbe`
- [x] `cli/vcs-adapters/Cargo.toml` gains no `[features]` entry beyond
      `bash-parity`

---

## Phase 2: The Six Taxonomy Queries

### Overview

Add the six queries as inherent methods on `InProcessProbe`, test-first against
the recorded oracle mapping, over the full fixture matrix. Add the source guard
forbidding `UserSettings` and `Workspace::load`.

### Changes Required

#### 1. The query surface

**File**: `cli/vcs-adapters/src/library.rs`
**Changes**: six inherent methods returning plain domain values, so no library
type leaks into what 0169 will build its port over.

```rust
pub struct WorktreeFacts {
    pub linked: bool,
    pub git_dir: PathBuf,
    pub common_dir: PathBuf,
    pub main_worktree_root: Option<PathBuf>,
}

pub enum JjWorkspaceRole {
    Main,
    Secondary,
}

pub struct JjRepositoryFacts {
    pub role: JjWorkspaceRole,
    pub main_root: PathBuf,
}

pub struct DualRoots {
    pub git: Result<Option<PathBuf>, Error>,
    pub jj: Result<Option<PathBuf>, Error>,
}

impl InProcessProbe {
    pub fn is_bare(&self, start: &Path) -> Result<Option<bool>, Error>;
    pub fn worktree(&self, start: &Path) -> Result<Option<WorktreeFacts>, Error>;
    pub fn superproject(&self, start: &Path) -> Result<Option<PathBuf>, Error>;
    pub fn jj_workspace_root(&self, start: &Path) -> Result<Option<PathBuf>, Error>;
    pub fn jj_repository(&self, start: &Path) -> Result<Option<JjRepositoryFacts>, Error>;
    pub fn dual_roots(&self, start: &Path) -> DualRoots;
}
```

`dual_roots` is **infallible and carries a per-side `Result`**, not
`Result<DualRoots, Error>`. A whole-struct `Result` cannot say "the git side
failed but the jj side answered": a one-sided failure would either propagate as
`Err` and discard a valid answer, or be flattened to `None` — which silently
reinstates the absence/failure conflation on the *single field* 0169
discriminates `colocated` from `nested-*` on. A repository whose git side the
pinned library cannot parse must never be observable as "jj only". Callers
comparing the two sides for equality must treat any `Err` as "not comparable",
never as inequality.

**Precondition: `start` is absolutised before any walk.** The three walks
disagree otherwise. `gix::discover(start)` absolutises against the process cwd
internally, whereas `walk_up` — extracted from `MarkerWalkRoot`
(`cli/vcs-adapters/src/lib.rs:35-44`) — is purely lexical: for a relative
`"sub"`, `Path::new("sub").parent()` is `Some("")` and `Path::new("").parent()`
is `None`, so it tests `sub` once and exits. Given a relative `start` a colocated
checkout would report `dual_roots.git = Ok(Some(root))` and
`dual_roots.jj = Ok(None)` — the wrong arm. The 24-pair matrix cannot catch this
because every fixture path is absolute, so the choke point absolutises (via
`std::path::absolute` or a cwd join) and asserts it, and one test per walk uses a
relative `start`.

`JjRepositoryFacts` carries `role: JjWorkspaceRole` (a two-variant `Main` /
`Secondary`) rather than `secondary: bool` — 0169 builds its classifier
vocabulary directly on these names, so a bare boolean whose other state is
unnamed is the wrong thing to freeze.

`Error` lives in this module. It is deliberately **not** `kernel::Error`:
`kernel` is not a dependency of `vcs-adapters`, and adding one to carry an error
type would couple the adapter to the launcher's error vocabulary for no gain.
0169 maps it into whatever its domain port needs.

**Every `PathBuf` these six queries and both port methods return is
canonicalised**, at one choke point each return value passes through, and the
rule is stated as a `//!` doc comment on the module rather than per query.
Without it the surface is inconsistent — `repo_path()` arrives already
canonicalised by jj-lib (`dunce::canonicalize`,
`jj-lib-0.43.0/src/workspace.rs:576`) while `workspace_root()` is whatever path
was passed in, and a linked worktree's `workdir()` is reconstructed from the
absolute path git recorded at `git worktree add` time rather than derived from
`start`. `dual_roots` equality is the colocated-vs-nested discriminator, so
comparing one canonicalised side against one uncanonicalised side is exactly how
a macOS `/var` → `/private/var` split produces a wrong classification. The
recorded `CG` equality survives today only because Phase 2's fixtures
canonicalise at construction; production callers have arbitrary cwds.

**Failure is distinguishable from absence.** Every query returns
`Result<Option<T>, Error>` (`dual_roots` is infallible and carries a per-side
`Result`, per the signatures above):
`Ok(None)` is "no repository of this kind here", `Err` is "a repository is here
and the pinned library could not answer". Collapsing both into `None` would be a
real regression against the adapter being replaced — `CommandProbe` runs its
parse in a child process with a 10-second cap, a kill-on-timeout and a scrubbed
environment, and warn-logs every distinct failure (`cli/vcs-adapters/src/lib.rs:171-217`).
`InProcessProbe` parses repository-controlled data **in the caller's address
space**, with no time bound, no memory bound and no crash isolation, and once
0185 flips `vcs_adapters::facts` that parsing runs inside a long-lived HTTP
server and on the hook path. A corrupt object store or an unreadable `.jj/repo`
must not read as "no VCS here". The two port methods keep their `Option`
signatures — the ports are not being changed — and warn-log instead. Which
library error conditions are deliberately mapped to `Ok(None)` rather than `Err`
is recorded in [Recorded divergences](#recorded-divergences).

Implementation notes drawn from measurement:

- `is_bare`, `worktree`, `superproject` and `dual_roots.git` resolve through
  `gix::discover(start)`, which is permitted to escape the boundary. Only
  `RepoRoot::discover` uses `gix::open` at the boundary.
- `worktree` canonicalises `common_dir()` before exposing it — the raw value
  carries `../..` for a linked worktree.
- `worktree.linked` is the canonicalised `git_dir() != common_dir()`
  comparison, matching the oracle exactly; `kind()` is used only as the
  submodule signal for `superproject`. `Kind` is a single mutually-exclusive
  enum, so it cannot represent a checkout that is *both* a submodule and a
  linked worktree — `git worktree add` from inside `super/mid` has `--git-dir`
  `…/modules/mid/worktrees/x` against `--git-common-dir` `…/modules/mid`, unequal,
  so the oracle says worktree while `kind()` can report only one of the two facts.
- `superproject` is bespoke path logic. The rule is: **scan the `modules`
  components of `git_dir()` from the innermost outward and take the first whose
  parent opens as a repository.** The two discriminating cases:
  - `SM-2` — `git_dir() == $super/.git/modules/mid/modules/leaf`. The innermost
    `modules` has parent `$super/.git/modules/mid`, which opens (it is the `mid`
    submodule's git dir), so the answer is `$super/mid` — matching the measured
    oracle. Taking the *outermost* would give `$super`, which is wrong.
  - `SM-m` — a submodule added at `modules/foo`, so
    `git_dir() == $super/.git/modules/modules/foo`. The innermost `modules` has
    parent `$super/.git/modules`, which is **not** a repository, so the scan
    continues outward to the next `modules`, whose parent `$super/.git` opens,
    giving `$super` — matching git. A bare `rposition` stops at the first
    candidate and returns `Ok(None)` here.

  It is extracted as a function taking a **fallible** probe alongside the path —
  `impl Fn(&Path) -> Result<bool, Error>`, not `-> bool` — so the method passes a
  `gix::open`-backed closure while the unit tests pass a total closure over known
  paths. A `bool` return makes "this candidate is not a repository"
  indistinguishable from "this candidate is a repository the pinned library could
  not open", so on a corrupt or hostile superproject the scan would silently
  continue outward and return a plausible wrong path — exactly what the
  degenerate and `HOSTILE` fixtures exist to forbid ("`Err`, not a panic, and not
  a plausible wrong path"). An unopenable candidate short-circuits to `Err`.

  Without the injected probe the derivation is not pure — it must consult the
  filesystem to decide which candidate anchors — and the stated testability win
  (edge cases exercised without the matrix's most expensive fixtures) would be
  unreachable.

  Measured (`SM-wt`, 2026-08-03): a submodule initialised inside a linked
  worktree does **not** put its git dir under the common dir's `modules/` — it
  goes to `$super/.git/worktrees/<id>/modules/sub`, and the oracle returns the
  *worktree*. The scan therefore anchors on `$super/.git/worktrees/<id>`, whose
  `workdir()` is the worktree — the right answer. **The one thing Phase 2 must
  confirm is that `gix::open` accepts a linked-worktree gitdir**; the directory
  layout is settled.

  `SM-w` (a linked worktree *of* a submodule) is the shape to be careful with:
  `--show-superproject-working-tree` is **empty** there, so if `kind()` reports
  `Submodule` the scan must not run at all. Gate on the measured oracle, not on
  `kind()` alone.

  Old-form submodules report `Kind::Common` and yield `Ok(None)`, agreeing with
  git.
- `jj_workspace_root`, `jj_repository` and `dual_roots.jj` use the **`.jj`-only**
  walk, then `DefaultWorkspaceLoaderFactory::create`. `jj_repository.role`
  compares canonicalised `repo_path()` against canonicalised
  `<root>/.jj/repo`; `main_root` is `repo_path().parent().parent()`, which
  returns `Err` rather than panicking when the path is too short — **and then
  asserts `<main_root>/.jj/repo` is a directory, else `Err`**. That
  post-condition mirrors the shell oracle's, which carries it explicitly:
  `[ -d "$candidate/.jj/repo" ] || return 1`, commented "so a future jj layout
  change cannot silently produce a wrong-but-non-empty answer"
  (`scripts/vcs-common.sh:106-112`). Without it, a `.jj/repo` pointer resolving
  to any *existing* directory that is not a jj store yields a real-looking
  `main_root` two levels up and `role` compares unequal, so the query reports
  `Secondary` with a bogus root — which becomes `RepoFacts.name` once 0185 flips
  `facts`, stamping artefacts with the wrong repository. The degenerate fixture
  set covers a *deleted* pointer target; it needs one whose target is an existing
  non-store directory. Hand-written `.jj/repo` pointers are in scope: the `CG`
  builder writes one.
- `dual_roots` resolves each side by its own walk so neither is truncated by the
  other's marker, and `dual_roots.jj` calls `jj_workspace_root` rather than
  re-walking.
- `discover` and `kind` call the crate-private `walk_up(start, predicate)` and
  `marker_kind(root)` extracted in Phase 1, so the "never test the filesystem
  root" rule has one implementation and the `.jj`-only walk shares it. The
  delegation direction matters: `MarkerWalkRoot` and `CommandProbe` delegate *to*
  those helpers, so 0185's deletion of the retained pair is mechanical and
  requires no re-homing. Pointing the surviving code at the code 0185 deletes
  would have made `^crate(::|$)` — the one pup clause that reaches the spawning
  `CommandProbe` — permanently load-bearing.

#### 2. The fixture matrix

**File**: `cli/vcs-test-support/{Cargo.toml,src/lib.rs}` (new; see Phase 3 §1
for the manifest), carrying the `fixtures` and `hermetic` modules
**Changes**: builders for every shape, written in their final home. Phase 3 adds
only `stubs`, the `detection.rs` seam and the cross-crate proof.

The crate is created here rather than in Phase 3 for two reasons. The builders
are the most intricate code in the change — the depth-1 and depth-2 submodules
"exist nowhere in the repo, in any language" — and writing, reviewing and
merging them once only to relocate them next phase means the move's diff noise
obscures any behavioural change smuggled in with it. And Phase 2's own
hermeticity criterion depends on the `hermetic` module, so a reviewer of Phase 2
could not otherwise verify it. Phase 3's rationale for a dedicated crate (it
satisfies the no-extra-`[features]` criterion with no interpretation, and
sidesteps CI's `--all-features` turning a fixture feature on workspace-wide)
applies just as well here, and Phase 1's dependency-policy review is unaffected
by a test-only workspace member.

**File**: `cli/vcs-adapters/tests/queries.rs` (new), `#![cfg(feature =
"bash-parity")]`
**Changes**: one table-driven test **per (fixture, start directory) pair**,
asserting all six query values at once, including explicit not-applicable
expectations. Values come from the mapping above; the tests are written before
the query methods.

Per-pair rather than per-query: nextest is process-per-test with no sharing, so
one test per query would rebuild the whole ~19-fixture matrix six times — and
with the poisoned duplicates, on both legs of the OS matrix, that is on the
order of a hundred-plus fixture constructions per CI run, each driving several
`git`/`jj` subprocesses. This repo already has a fixture-flake history under
parallel CI load. Per-pair keeps the per-cell traceability the acceptance
criterion demands, localises a failure to a shape rather than a query, and costs
roughly a sixth of the fixture builds.

Existing shapes reusable from `cli/vcs-adapters/tests/detection.rs`: plain git
(`:61-80`), nested subdir (`:114-125`), colocated-real (`:144-169`), jj
secondary (`:171-209`), `.git`-file worktree (`:211-248`), bare (`:250-277`).

New construction required:

| Fixture | Cost | Note |
| --- | --- | --- |
| git submodule (depth 1 and 2) | substantial | **Exists nowhere in the repo, in any language.** Needs `git -c protocol.file.allow=always submodule add` and a recursive `submodule update --init` |
| old-form submodule | small | a nested `git init` directory |
| colocated, hand-grafted | substantial | port the graft from `hooks/test-vcs-detect.sh:96-157`; `.jj/repo` must be written with `%s` and no trailing newline |
| nested-jj-in-git / nested-git-in-jj | moderate | port from `hooks/test-vcs-detect.sh:516-536` |
| pure jj | small | `jj git init --no-colocate`, root three dirs below the temp root, one commit, plus a `.git`-absent assertion |
| linked worktree, main-worktree start | trivial | second start directory on the existing fixture |
| **`JS-in`** — jj secondary workspace *inside* its own colocated main | small | `mkdir -p workspaces` **then** `jj workspace add` — 0.43 does not create intermediate directories. **This repo's own `workspaces/<name>` shape.** Measured: dual roots differ while `classify_checkout` reports `jj-secondary`, because `jj_main_root == git_main_root` |
| **`SM-m`** — submodule whose path contains a `modules` segment | small | `git submodule add … modules/foo`; produces `git_dir() == $super/.git/modules/modules/foo`, the case a bare `rposition` misresolves |
| **`SM-w`** — linked worktree of a submodule | moderate | `git worktree add` from within `super/mid`. Measured: dirs differ, superproject **empty**, and `find_git_main_worktree_root` returns a `.git/modules` path |
| **`SM-wt`** — submodule inside a linked worktree of the superproject | moderate | settles whether `superproject()` is worktree-accurate; measured git dir is under `worktrees/<id>/modules/`, oracle returns the worktree |
| **`RF`** / **`S256`** — reftable ref backend, sha256 object format | small | `git init --ref-format=reftable`, `git init --object-format=sha256`. **`S256`'s HEAD is 64 hex, not 40** — see the revision-shape note below |
| **degenerate shapes** (`D1`/`D2`/`D3`) | small | `.jj/repo` → a deleted directory; a `.git`-file worktree whose gitdir target is removed; `.jj/repo` → an **existing non-store** directory |
| **`HOSTILE`** — adversarial configuration | small | a plain git repository whose `.git/config` sets `core.pager`, `core.fsmonitor`, `diff.external`, `filter.*.clean`/`smudge`, an `alias.*` and an `include.path` chain |

All ten are measured; oracle values and per-fixture expectations are in
[Extended fixtures](#extended-fixtures-measured-2026-08-03).

The degenerate shapes exist because every query consumes repository-supplied
data: Query 5 reads a `.jj/repo` pointer and takes `parent().parent()`, Query 3
string-searches a repository-supplied `git_dir()`, and Query 2 exposes a
`common_dir()` that a `.git` file can point anywhere. **Their expectations are
not uniform**, contrary to an earlier draft that said all of them yield `Err`:

- `D1`, `D2` — both CLIs report plain **absence**, so `Ok(None)`.
- `D3` — `jj workspace root` **succeeds**, so `jj_workspace_root` is
  `Ok(Some(root))` while `jj_repository` must be **`Err`**, because the
  `<main_root>/.jj/repo`-is-a-directory post-condition fails. This is the fixture
  that proves the invariant.

The **truncated pack index** shape from the earlier draft is dropped: none of the
six queries reads object data (only `revision` does, and it is a port method
returning `Option`), so it would have proved nothing.

`HOSTILE` runs through the strong-form harness asserting no stub marker is
written. Measured caveat: **none of its seven configured commands ran** under the
oracle's own calls, and none of the eight delivered library calls enters gix's
filter/pager/external-diff machinery either. So it is a **regression guard for
the APIs 0169 adds**, not evidence for this story's call set — state it that
narrowly wherever the zero-spawn property is described.

**Precondition, asserted once by the harness with a named diagnostic**: no
ancestor of the temp base carries `.git` or `.jj`. Roughly a quarter of the
mapping's cells assert absence for the gix-backed queries, and `gix::discover`
reads **no environment** — so `GIT_CEILING_DIRECTORIES`, which produced the
oracle side's exit-128s, cannot fence the library side. On a host whose `TMPDIR`
resolves inside a repository those cells silently resolve the enclosing
repository instead. `hooks/test-vcs-detect.sh:35-40` guards the same hazard for
the shell suite. The harness walks from the temp base to the filesystem root and
fails with one legible message rather than a mass of confusing per-cell
failures.

Temp dirs follow `tempfile::Builder::new().prefix(…).tempdir()` returning the
owned guard (`detection.rs:32-39`); every path is canonicalised immediately
after construction, and nested paths canonicalise the parent then join
(`:179-180`), because on macOS `$TMPDIR` resolves `/var` → `/private/var`.

#### 3. Scrub-invariant verification

**File**: `cli/vcs-adapters/tests/queries.rs`
**Changes**: every query, over every pair, asserted equal with and without the
environment poisoned — where "poisoned" means pointed at **another fixture's
real `.git`**, not an empty or non-existent path. Plus the control:
`gix::discover_with_environment_overrides` on the same fixture under the same
poison must return the poison target, proving the poisoning live.

**The poisoned variable set is everything `scrub_environment` scrubs or forces**
(`cli/vcs-adapters/src/lib.rs:139-154`) — `GIT_DIR`, `GIT_WORK_TREE`,
`GIT_INDEX_FILE`, `GIT_COMMON_DIR`, `GIT_CONFIG`, `GIT_CONFIG_COUNT`,
`JJ_CONFIG`, `GIT_CONFIG_NOSYSTEM`, `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_SYSTEM` —
**plus** `GIT_OBJECT_DIRECTORY`, `GIT_ALTERNATE_OBJECT_DIRECTORIES` and a
`GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0`/`GIT_CONFIG_VALUE_0` triple. The first
draft verified two variables and generalised to "uniformly immune", which is
broader than the evidence: `gix::open`'s default `Permissions` do consult the
environment for object directories and for system/global config discovery, and
`is_bare()` reads `core.bare` *through* that config. At least one case runs with
a **populated** global config asserting a divergent `core.bare`, because the
Phase 2 empty-config criterion otherwise masks exactly the config influence this
invariant is meant to exclude.

The `gix::open`/`gix::discover` `Options` and `Permissions` the module passes
are recorded in its doc comment. Where an isolated permission set can be
requested explicitly, it is — **constructing** immunity beats relying on an
observed default of a pre-1.0 crate, and the difference is invisible at the call
site otherwise.

Coverage is the six queries and `RepoRoot`. `VcsProbe` parity against
`CommandProbe` is out of scope for this criterion — `CommandProbe` shells out
and does honour an ambient `GIT_DIR`.

**Both arms run through the same child binary**, poisoned and clean invocations
of the reference artefact compared on captured stdout, and the live-poison
control is asserted inside the poisoned child in the same run. Comparing an
in-process value against a child's rendered output would compare two
serialisation routes — producing false failures across the whole matrix, and the
more dangerous false pass where both arms render absence identically for
different reasons. The reason a child is needed at all is that libtest runs each
test on a spawned thread, so in-process `set_var` is racy by construction;
nextest's process-per-test model (`tasks/test/cli.py:31-35`) mitigates
cross-*test* interference but not this. The child is the Phase 4 reference
artefact, built in this phase so there is exactly one linked binary and one
stubbed one across the whole plan rather than a third interim target.

#### 4. The `UserSettings` source guard

**File**: `tasks/lint/vcs_settings.py` (new), exported in
`tasks/lint/__init__.py`, registered in `tasks/__init__.py`
**Changes**: a pure function scanning `cli/vcs-adapters/**/*.rs` for
`UserSettings` and `Workspace::load`, with a thin `@task` wrapper.

It **strips comments before matching**. The model it copies
(`tasks/lint/store_duplication.py:48-50`) matches a regex against every raw
line, which would make it impossible to document *why* `UserSettings` is avoided
in the very crate it guards — and that reason (private defaults abandoned after
five successive panics with the chain never exhausted) is exactly the
extremely-non-obvious kind of fact this repo's comment bar admits. Without the
strip, an implementer hits the self-flagging problem immediately and works
around it by mangling the word.

Stated crate-wide, deliberately wider than the detection paths require. The
scope narrows to the detection paths only if the `revision` question above is
resolved by option 2.

A `denied` clause on the cargo-pup rule this plan is already adding would be
cheaper, and is not sufficient: `RestrictImports` resolves `use` paths, so it
cannot see a fully-qualified
`jj_lib::settings::UserSettings::from_config(…)` or the `Workspace::load`
method call. That is what buys the extra Python module, its two `__init__.py`
registrations, the mise leaf and the `test_mise.py` constants.

**File**: `mise.toml`
**Changes**: a `lint:vcs-settings:check` leaf joining **both**
`cli:check.depends` (`:422-424`) and `lint:check.depends` (`:464-466`) — the
bare `default` task depends on `lint:check` but not on `check`. Pure guard, no
autofixer, so it stays out of `fix`.

**File**: `tests/unit/tasks/test_vcs_settings.py` (new)
**Changes**: synthetic `tmp_path` trees for each branch plus a real-tree
assertion, mirroring `test_store_duplication.py`.

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: add the new leaf to `_CHECK_GATES` and `_CLI_CHECK_GATES` — the
exhaustive equality assertions fail if skipped.

### Success Criteria

#### Automated Verification

- [x] All six queries pass against every (fixture, start directory) pair:
      `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-adapters --all-features`
- [x] The scrub invariant holds across the whole matrix, and the unscrubbed
      control diverges under the same poisoning
- [x] `mise run lint:vcs-settings:check` passes
- [x] `mise run test:unit:tasks` passes, including the new guard tests
- [x] Non-vacuity: a deliberately added `UserSettings::from_config(…)` in
      `cli/vcs-adapters/src/library.rs` fails the guard, and is reverted
- [x] Every jj query succeeds under the hermetic environment exactly as
      enumerated in the `hermetic` module (`GIT_CONFIG_NOSYSTEM=1` as a
      boolean, not a path)
- [x] The extended fixtures match their measured per-fixture expectations —
      **with one amendment**: `D1` is `Err`, not `Ok(None)`, because a `.jj/repo`
      pointing at a deleted directory is a broken workspace rather than an absent
      one and the partition rule maps it so; the `Ok(None)` here was read off the
      CLI's exit code, and the CLI conflates. Otherwise as recorded: `D2` →
      `Ok(None)`; `D3` → `jj_workspace_root` `Ok(Some)` but `jj_repository`
      `Err`; `JS-in` → dual roots differ; `SM-m` → superproject `$B/supm`;
      `SM-wt` → superproject the worktree; `SM-w` → `linked` true and
      superproject `Ok(None)`; and none of them panics
- [x] `gix::open` on a linked-worktree gitdir is confirmed (the one open question
      `SM-wt` leaves), and `RF`/`S256` outcomes are recorded in Recorded
      divergences
- [x] The fixture harness fails with a named diagnostic when an ancestor of the
      temp base carries `.git` or `.jj`
- [x] `mise run` green end to end

#### Manual Verification

- [x] Every expected value in `queries.rs` is traceable to a cell in the mapping
      table above, and every (fixture, start directory) pair has a cell in every
      query table — no query table is partial
- [x] The pure-jj fixture asserts `.git` absent, not merely `.jj` present
- [x] Whatever gix 0.85 does with the `RF`/`S256` fixtures is recorded in
      Recorded divergences rather than assumed — `RF` reads normally, `S256`
      returns `Err` from every gix-backed query

---

## Phase 3: The Shared Test-Support Crate and the Zero-Spawn Harness

### Overview

Complete the harness crate created in Phase 2 with its `stubs` module, add the
`detection.rs` injection seam, and prove the harness across a crate boundary
from `cli/corpus-adapters` — the crate 0185 will extend.

A dedicated crate rather than a feature on `vcs-adapters`: it satisfies the
"no `[features]` beyond `bash-parity`" criterion with no interpretation, and
sidesteps CI's `--all-features` turning a fixture feature on workspace-wide.

### Changes Required

#### 1. The crate

Created in Phase 2 with `fixtures` and `hermetic`; this phase adds `stubs`. The
manifest and the three modules are specified together here for readability.

**File**: `cli/vcs-test-support/Cargo.toml` — created in **Phase 2** together
with its `cli/Cargo.toml` `members` entry and the lock change that forces;
restated here because all three modules are specified together
**Changes**: the full inheriting manifest every other member declares —
`version.workspace`, `edition.workspace`, `rust-version.workspace`,
`license.workspace`, `publish.workspace`, `[lints] workspace = true` — depending
on `vcs` and `tempfile` only.

Spelling the inheritance out matters: a member omitting `edition` silently
defaults to the 2015 edition, and one omitting `rust-version` drops out of the
MSRV-aware fallback that Key Discovery 11 identifies as load-bearing, which
`test_msrv_coherence.py` would not notice (it compares `mise.toml`,
`cli/Cargo.toml` and `cli/clippy.toml` only).

It depends on `vcs` (the ports), **not** on `vcs-adapters`. The fixtures, stubs
and hermetic environment need only `tempfile` and the real `git`/`jj` binaries,
so the `vcs-adapters` edge is unnecessary — and taking it would create an
undeclared dev-dependency cycle (`vcs-adapters` tests → `vcs-test-support` →
`vcs-adapters`) that also forces `vcs-adapters` to be compiled with
`bash-parity` on via a *normal* edge, weakening the very rationale for a
separate crate.

**File**: `cli/vcs-test-support/src/lib.rs` (new)
**Changes**: three public modules.

- `fixtures` — every builder, written here in Phase 2, including the **named
  reusable pure-jj builder** 0169 must be able to reconstruct identically.
- `stubs` — marker-writing `git`/`jj` stubs on a synthetic `PATH`, plus a
  **platform-aware report of the absolute paths it cannot control**. On Linux CI
  the meaningful jj target is the mise install path
  (`$HOME/.local/share/mise/installs/jj/<version>/jj`) and its shim, not
  `/usr/bin/jj` — there is no system `jj` on the runner at all
  (`mise.lock:89-106`). On a developer's macOS machine `/opt/homebrew/bin/jj`
  **is** the real binary. git keeps `/usr/bin/git`, `/usr/local/bin/git`,
  `/opt/homebrew/bin/git`. The list is resolved at runtime by asking the
  environment where each binary actually lives.

  **This crate never writes outside its own temp directories.** It resolves and
  *reports* absolute paths; it does not move, replace or chmod them, and it
  never invokes `sudo`. All privileged mutation lives in the CI workflow step,
  which is the only thing that can move a root-owned binary anyway — but the
  boundary is stated because `/opt/homebrew/bin` is user-writable, so an
  "attempt and record the failure" design would succeed there and could leave a
  developer's machine without `git` or `jj`. `test:integration:zero-spawn` is
  reachable from the bare `mise run` every contributor is told to run before
  pushing, so this is the difference between a test and a hazard. The harness
  reads `ACCELERATOR_ZERO_SPAWN_MODE`/`ACCELERATOR_ZERO_SPAWN_SHADOWED` to learn
  what the workflow shadowed, and hard-fails if `strong` is claimed but a listed
  path is still executable.

  The in-repo precedent, `strip_binary_from_path`
  (`hooks/test-vcs-detect.sh:164-188`), is likewise purely non-destructive — it
  edits a `PATH` string and touches no file.
- `hermetic` — the empty-config environment, and a `Command` decorator applying
  it plus the stub `PATH`. It **enumerates the exact variables and value shapes
  the oracle mapping was measured with**, in one place, so the recorded mapping
  and the shipped harness cannot drift apart per platform: `HOME`,
  `XDG_CONFIG_HOME` and `JJ_CONFIG` at temp dirs, `GIT_CONFIG_GLOBAL=/dev/null`,
  `GIT_CEILING_DIRECTORIES` at the temp base, and `GIT_CONFIG_NOSYSTEM=1` — a
  **boolean, not a path**, and the only thing suppressing the *system*
  gitconfig, which lives at `/etc/gitconfig` on ubuntu and inside the Command
  Line Tools on macOS. Pointing it at a temp dir, as a loose reading of "GIT_CONFIG_*
  at empty temp dirs" would, leaves host config leaking into fixture
  construction on one platform and not the other.

  It also asserts, at fixture-build time under `bash-parity`, that
  `jj --version` matches the jj-lib version **at major.minor only**, and that
  `git --version` meets a documented minimum with a named diagnostic. The
  `mise.toml` ↔ `cli/Cargo.toml` lockstep test compares two *declarations*;
  nothing otherwise checks the binaries that write the repository formats the
  libraries read. This planning session hit exactly that skew — the local `jj`
  was 0.42.0 from Homebrew because `mise.toml` was untrusted — and the failure
  would otherwise surface as an apparently wrong answer in a 24×6
  expected-value table rather than as a version mismatch. The same argument
  applies to `git`: the whole mapping is calibrated to 2.54.0 and `git` is
  pinned nowhere, so a floor with a diagnostic beats recording a version nobody
  reads.

  Two mechanics this needs: `vcs-test-support` depends on `vcs` and `tempfile`
  only, so it has **no jj-lib edge** and cannot observe a compiled-in version —
  the linking crate (`vcs-adapters`' test target) injects it as a parameter,
  sourced from a `build.rs`-emitted constant, since `env!("CARGO_PKG_VERSION")`
  yields the *calling* crate's version and jj-lib publishes no version constant.
  And the comparison is major.minor because the tilde ranges deliberately permit
  `mise.toml`'s exact `0.43.0` CLI to sit alongside a resolved `jj-lib 0.43.x`;
  an exact-equality assertion would fail the entire fixture matrix, for every
  contributor and both CI legs, on a skew this plan pre-authorises.

  Fixture builders pin the format-relevant knobs per invocation via the
  **documented mechanisms** — `git init --object-format=<fmt>` /
  `--ref-format=<fmt>` (or `GIT_DEFAULT_HASH` / `GIT_DEFAULT_REF_FORMAT`), plus
  `-c init.defaultBranch=main` — except in the `RF`/`S256` fixtures that exist to
  vary them. Note that `extensions.objectFormat` is a *repository-format
  extension* read from the repo's own config, not an `init` knob: injecting it
  via `-c` into a `core.repositoryformatversion = 0` repository drives git's
  "repo version is 0, but v1-only extension found" failure path, and
  `init.defaultRefFormat` is not a documented `init.*` key across the supported
  range. Each constructed fixture asserts it actually has the intended object
  format and ref backend, so a silently-ignored knob fails loudly rather than
  restoring the cross-runner drift the pinning exists to remove.

  Fixture builders also supply an identity explicitly —
  `-c user.name=… -c user.email=… -c commit.gpgsign=false`, and a written
  `user.name`/`user.email` in `JJ_CONFIG` rather than an empty temp dir. With
  `HOME` at a temp dir, `GIT_CONFIG_GLOBAL=/dev/null` and
  `GIT_CONFIG_NOSYSTEM=1`, `git commit` otherwise falls back to an auto-detected
  `user@hostname` identity and **refuses** the commit when the hostname carries
  no domain — the normal case on CI runners and in containers. Several fixtures
  need commits, and `hooks/test-vcs-detect.sh:58` already sets both keys on every
  git fixture for exactly this reason.

  No fixture builder writes outside its own `TempDir`, and
  `protocol.file.allow=always` (needed by the submodule fixtures) is passed
  per-invocation via `git -c`, never written into a config file — not even one
  under a temp `HOME`.

The `PATH`-stripping technique is ported from `strip_binary_from_path`
(`hooks/test-vcs-detect.sh:164-188`) with both recorded gotchas preserved:
macOS provides `git` in two directories, so a single dirname is insufficient;
and `type -p` must not be used.

Fixture builders use `tempfile::TempDir`, **not** `NamedTempFile`/`.persist`/
`fs::rename`, which the store-duplication guard flags anywhere under
`cli/**/src`.

#### 2. The injection seam

**File**: `cli/vcs-adapters/tests/detection.rs`
**Changes**: replace the seven direct `vcs_adapters::facts` calls (`:95`,
`:121`, `:134`, `:155`, `:191`, `:234`, `:272`) with a helper parameterised by
`(&dyn RepoRoot, &dyn VcsProbe)`, and run every existing case against **both**
the retained `MarkerWalkRoot`/`CommandProbe` pair and `InProcessProbe`,
asserting the suite's existing fixed expected values — agreement between two
implementations is not on its own an oracle.

Parity is asserted **per `VcsKind`**, per the spike-resolved `revision` descope:
full `RepoFacts` equality for `VcsKind::Git`, and `root`/`name`/`kind` only for
`VcsKind::Jj`, where `InProcessProbe::revision` is `None` by design while
`CommandProbe` returns a 40-hex id. The jj cases therefore assert the
`CommandProbe` arm against the suite's fixed values as today, and the
`InProcessProbe` arm against the three fields it owns — so neither adapter's
coverage is silently dropped.

The `.git`-as-file worktree case keeps **today's** `RepoFacts` value, and the
identical-facts assertion applies to it unrelaxed. The `colocated` discussion
concerns `classify_checkout`'s taxonomy, not `RepoFacts` — the relevant
`RepoFacts` field is `VcsKind`, so no divergence arises there. `classify_checkout`
reports `main` for that shape with the git side unseen; 0169 owns correcting it
to `colocated`, and that correction is out of scope here.

This dual comparison is transitional: 0185 deletes `CommandProbe` and collapses
the suite to `InProcessProbe` alone.

#### 3. The cross-crate proof

**File**: `cli/corpus-adapters/Cargo.toml`
**Changes**: a **new `[dev-dependencies]` table** carrying
`vcs-test-support = { path = "../vcs-test-support" }`. Under resolver 3 a
dev-dependency's features stay out of the normal build, which matters because
`cli/visualiser/server` depends on `corpus-adapters`. The existing normal
`vcs-adapters` dependency (`:25`) is untouched.

**File**: `cli/corpus-adapters/tests/zero_spawn.rs` (new)
**Changes**: one test running a **full strong-form assertion end to end** —
stubs *and* shadow list *and* empty-config environment — through
`vcs-test-support`'s public API only, with no fixture-private helpers. It runs
the query × fixture table and asserts no stub marker was written **and** that
every value matches an unrestricted run, because an adapter degrading to `None`
also writes no marker.

The assertion is scoped to `git`/`jj` specifically, not "no subprocess at all":
`SystemClock::try_new` spawns `date` unconditionally
(`cli/corpus-adapters/src/metadata.rs:106-110`) and a blanket marker would trip
on it.

### Success Criteria

#### Automated Verification

- [x] `mise run test:unit:cli` passes; `detection.rs` runs every existing case
      through the seam against both implementations, producing identical
      `RepoFacts`
- [x] `cli/corpus-adapters`' `zero_spawn.rs` passes: no marker written and every
      value matches the unrestricted run
- [x] `mise run cli:check`, `deny:check`, `pup:check` pass (the new workspace
      member landed in Phase 2 and was gated there)
- [x] `cli/corpus-adapters`' metadata parity suite passes unchanged:
      `derive_at_agrees_with_the_live_metadata_helper`
      (`cli/corpus-adapters/tests/metadata.rs:265`) — note this is
      `tests/metadata.rs`, **not** `tests/parity.rs`, which is the linkage suite
      and never touches VCS
- [x] `mise run` green end to end

#### Manual Verification

- [x] `zero_spawn.rs` imports only `vcs_test_support`'s public API — all three
      parts, not one
- [x] The shadow list records which absolute paths it could not shadow on the
      host it ran on, and `vcs-test-support` modified nothing outside `TMPDIR`
      (checksum the resolved absolute paths before and after)
- [x] `cli/vcs-test-support` does not depend on `cli/vcs-adapters`, so no
      dev-dependency cycle exists

---

## Phase 4: Reference Artefact, Strong-Form CI, and Measurements

### Overview

Add the stub artefact (the linked one landed in Phase 2), wire both into the musl cross-compile, add the
Linux-only strong-form zero-spawn job, and take the cost and size measurements.

### Changes Required

#### 1. The reference artefact

**File**: `cli/vcs-adapters/tests/fixtures/vcs_adapters_fixture_stub.rs` (new) —
the **linked** artefact `vcs_adapters_fixture.rs` and its `[[bin]]` declaration
landed in **Phase 2**, which needs it as the child process for the poisoned
scrub-invariant runs. Both follow the `config-adapters-fixture` convention
(`cli/config-adapters/Cargo.toml:12-17`)
**Changes**: a composition root calling every query and both port methods and
**printing each result**, so the calls are not eliminable. Modes: `all`, and
`only <query>` for the cold per-process figure. Carries the required prelude:

```rust
#![allow(clippy::exit, clippy::print_stdout, clippy::print_stderr,
         clippy::restriction)]
```

A `stub` feature replaces every query call with a constant, for the size delta.
This is a feature on `vcs-adapters`, which the "no `[features]` beyond
`bash-parity`" criterion forbids — so the stubbed build is produced by a
**second `[[bin]]`** (`vcs_adapters_fixture_stub.rs`) that calls stubs directly,
adding no feature. Both binaries are staged and measured.

Because `CARGO_BIN_EXE_*` does not cross crate boundaries
(`cli/launcher/Cargo.toml:17-18`), `vcs-test-support` exposes a path resolver
derived from `current_exe()`, as
`cli/corpus-adapters/tests/common/mod.rs:62-86` does.

#### 2. musl staging

**File**: `tasks/build.py`
**Changes**: a **new `_CLI_FIXTURE_BINARIES` constant and its own staging loop**,
reusing `_assert_magic_bytes` and `_assert_static_elf` (`:132-159`) but asserting
in place under `cli/target/<triple>/release/` rather than copying into
`dist/release/`.

`_CLI_RELEASE_BINARIES` (`:37`) is **not** extended. That constant means
"binaries we ship": it drives `cli_binary_path()` staging into `dist/release/`,
the tree the signed manifest and release assets are assembled from, and whose
`accelerator-*` members are provenance-attested (`.github/workflows/main.yml:420-425`).
Nothing would be published today — `_release_uploads()` enumerates assets
explicitly rather than globbing — but two unshipped diagnostic binaries that
print absolute repository paths would sit in the release staging directory on
every release run, one glob change away from being published as product. The
sibling convention says the same: `cli/config-adapters/Cargo.toml:12-14` keeps
fixture binaries "off the crate's normal build surface" as "not a shipped
artifact". A unit assertion checks `_release_uploads()` never contains a fixture
name, and the fixture bin names avoid the `accelerator-` prefix so they cannot
be swept into the attestation glob.

**The size floor is a committed check, not a number written once.** This is the
guard against the story's headline false-pass — dead-code elimination letting
the musl and size checks succeed while linking almost none of `gix`/`jj-lib` —
and leaving it as a Validation Results figure would mean nothing catches a later
edit that stops printing a query result and lets the linker drop the trees.

Two scoping rules, because a heuristic threshold that first executes in the
release pipeline can abort a whole product release:

- The **ratio** floor (`linked ≥ 3× stubbed`) is asserted on every triple. It has
  wide margin — the measured darwin figure is 5.42× and musl 6.19×.
- The **absolute-byte** floor (`delta ≥ 1,500,000`) is asserted for **musl
  triples only**, matching how `_assert_static_elf` is already guarded
  (`if "musl" in triple`, `tasks/build.py:329-330`). The darwin stripped delta is
  1,639,872 B — only **9.3%** above the floor, and `[profile.release] strip =
  true` (`cli/Cargo.toml:76`) means every triple is stripped. Applying the
  absolute floor to darwin would put a 9%-margin heuristic on the critical path
  of `prerelease:prepare`, which builds the launcher, the verify shims and the
  visualiser too. Darwin figures are recorded, not gated.

A **host-native ratio assertion** also runs in `check-zero-spawn` (linked vs stub
under `cli/target/release/`), so the invariant has PR-level feedback rather than
first executing in a release. Without it, a contributor who deletes a result
print sees green everywhere they look. A unit test covers the comparison
function against synthetic sizes — threshold logic only, not a real build.

Recovery when the floor fires is documented in `tasks/README.md` alongside the
advisories break-glass: re-measure, and raise or lower the constant in
`tasks/build.py`.

This is a **release-pipeline change**: `cli_cross_compile` (`:312-331`) is
invoked only from `tasks/release.py:82-94` / `:109-125`, wired to the
`prerelease`/`release` jobs, both `runs-on: macos-latest`
(`.github/workflows/main.yml:343`, `:464`). No `check-*` job cross-compiles and
no test exercises `_assert_static_elf` against a real build today.

Verified this session: both binaries cross-compile to
`aarch64-unknown-linux-musl` via `cargo zigbuild --release` and `file -b`
reports `statically linked, stripped`, which `_is_statically_linked` (`:118-129`)
accepts unmodified.

**All four triples must be verified in this phase**, not just that one.
`cli_cross_compile` iterates the whole of `TARGETS` (`tasks/shared/targets.py`):
`aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-musl`,
`x86_64-unknown-linux-musl`. And this is not confined to the fixtures —
`gix`/`jj-lib` are **normal** dependencies of `vcs-adapters`, which
`cli/corpus-adapters` depends on (`Cargo.toml:25`), which `cli/visualiser/server`
depends on (`Cargo.toml:23`), so both trees must compile for all four triples in
the shipped visualiser cross-compile too. Since no `check-*` job cross-compiles,
an `x86_64-musl` or darwin failure inside the enlarged closure would land on
`main` and first surface in the `prerelease` job on `macos-latest`. Each
triple's result is recorded in Validation Results, alongside the
`accelerator-visualiser` binary size before and after, so any dead-code
elimination shortfall in the artefact that actually ships is visible rather than
inferred from the fixture's 391 KB stub figure.

Contention: 0187 rewrites `validate_dispatch_coherence` in the same file, at
`:189-208` — roughly 130 lines from this plan's regions (`:118-159`, `:290-331`)
with three unrelated functions between. The genuine collision surface is the
import block (`:11-32`) and the module constants (`:34-56`). Whichever lands
second regenerates rather than merges.

#### 3. The strong-form CI job

**File**: `.github/workflows/main.yml`
**Changes**: a new Linux-only job modelled on `check-architecture`
(`:286-321`) — the closest precedent for a single Linux runner that
self-provisions unusual infrastructure and is deliberately isolated.

```yaml
  check-zero-spawn:
    name: Check zero-spawn
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - Checkout
      - Route the rust toolchain into the cached mise data dir
      - Install dependencies (mise-action, cache_key_prefix: mise-zero-spawn-v1)
      - Cache cargo build (workspaces: cli)
      - Compile the test binaries          # cargo nextest run --no-run
      - Build the fixture matrix           # needs real git/jj; writes $MATRIX_DIR
      - Shadow the absolute VCS paths      # sudo mv -> $RUNNER_TEMP/shadowed
      - Run the strong-form zero-spawn suite  # prebuilt binaries, prebuilt fixtures
      - Restore the absolute VCS paths     # if: always()
      - Assert the restore worked          # if: always(); git --version && jj --version
```

Constraints satisfied: the job's `run:` text contains none of `pup:check`,
`deps:install:pup` or `+nightly`, so the nightly-isolation invariant
(`tests/unit/tasks/test_workflows.py:294-337`) still sees exactly
`{check-architecture}`; it declares no `needs: check-architecture`; it lives in
`main.yml` because actionlint lints only that file
(`tasks/lint/workflows.py:7`); and it carries its own `RUSTUP_HOME` routing step
plus `cache_key_prefix`, per the comment replicated to six jobs
(`main.yml:191-198`).

It is added to `prerelease.needs` (`:344-355`) so a red zero-spawn check blocks
a prerelease, matching every other check job.

The job must **not** be placed in `test-unit` or `test-integration`: both are OS
matrices (`:20-21`, `:60-61`) and `test:unit:cli` runs on both legs, so a
strong-form assertion there would fail on macOS under SIP. macOS degrades to
`PATH`-only with unshadowable paths recorded.

The shadow step uses `sudo mv` on GitHub-hosted Ubuntu VMs, where the `runner`
user has passwordless sudo and no container is needed. Corroborating in-repo
evidence: `test-visual-regression` (`:122-145`) already runs `docker run` with
bind mounts from a step. The jj target is the mise install path, not
`/usr/bin/jj`.

**The shadow window is deliberately narrow, and must stay that way.** Everything
that needs a real `git` or `jj` happens *before* it:

- **Compilation.** `cli/launcher` carries a `vergen-gitcl` build dependency that
  shells out to `git`, and cargo may need `git` on a cold registry cache. The
  cargo cache makes this a no-op on warm runs and a failure on cold ones — and
  this plan guarantees a cold run by changing `cli/Cargo.lock`. The suite is
  therefore compiled with `--no-run` first and only executed inside the window.
- **Fixture construction.** Every fixture in the matrix is built by invoking the
  real `git`/`jj` CLIs — that is what `bash-parity` means. Building them inside
  the window would leave the suite with an empty matrix, which would pass
  vacuously. The matrix is built beforehand into `$MATRIX_DIR` and the suite
  consumes prebuilt fixtures, asserting the expected fixture count so an empty
  or truncated matrix fails loudly rather than passing.

  **The handoff needs an API the plan must specify, because `TempDir` cannot do
  it.** Phase 2/3 mandate owned `tempfile::TempDir` guards, which delete on drop,
  so a builder that owns a guard leaves the consuming step an empty directory.
  The workflow creates the root (`MATRIX_DIR=$(mktemp -d -p "$RUNNER_TEMP")`) and
  the builder writes beneath a caller-supplied root **without owning a guard**;
  `TempDir` stays for the in-process test paths. The builder never deletes
  `$MATRIX_DIR` — no `rm -rf "$MATRIX_DIR"` idempotency step, which becomes a
  wildcard delete if the variable is unset. Name the binary or test target that
  drives construction, and compile it in the `--no-run` step. Do **not** reach
  for `TempDir::keep()`/`into_path()` to work around this: the store-duplication
  guard does not catch it, and the same builder would then leak a ~19-fixture
  tree of git/jj repositories into `TMPDIR` on every developer `mise run`.

  `$MATRIX_DIR` must also satisfy the no-`.git`-ancestor precondition, asserted
  at the start of the consuming step — `$RUNNER_TEMP` satisfies it on
  GitHub-hosted runners, but it is a provider-specific variable and self-hosted
  or containerised runners commonly point `TMPDIR` inside the workspace, where
  every absence cell would silently resolve the enclosing accelerator repository.
- **`mise` tool resolution.** If `mise run` is the entry point inside the
  window, mise may observe `jj` as missing and reinstall it, silently restoring
  the binary and making the assertion vacuous. The post-shadow step invokes the
  prebuilt test binary directly, not through `mise run`.

Binaries are moved to `$RUNNER_TEMP/shadowed`, **not renamed in place**, because
the meaningful jj target lives inside `$HOME/.local/share/mise/installs/` — the
tree `mise-action` saves to the cache on its post step, which runs *after* the
restore. An in-place shadow that failed to restore would persist a `jj`-less
mise install into the `mise-zero-spawn-v1` namespace, making every subsequent
run of this job fail or pass vacuously until someone bumped the prefix.

Shadow and restore are both **idempotent per path** — test-then-move each entry,
tolerate a missing source, and exit non-zero at the end only if a path is still
shadowed — so a partial shadow does not leave a straight-line `set -e` restore
aborting on the first missing source. The final `if: always()` assertion that
`git --version` and `jj --version` both succeed turns a failed restore into a
red job rather than a poisoned cache; note that `actions/checkout`'s own post
step runs `git` to strip the auth token, so an unrestored `git` breaks it.

This containment relies on GitHub-hosted runners being **ephemeral VMs**, and on
`git` at `/usr/bin/git` and `jj` under the mise install tree. Both are recorded
as inline comments on the job and in `tasks/README.md`, because a move to
self-hosted, containerised or reusable runners turns a contained hazard into a
persistently broken runner. Targets are resolved at run time (`mise which jj`,
and an enumeration of *every* `PATH` hit rather than the first, per the recorded
macOS two-directory gotcha) and the step fails closed when a resolved target
cannot be shadowed.

The workflow step and the harness have an explicit contract rather than each
assuming the other did the work: the step exports
`ACCELERATOR_ZERO_SPAWN_MODE=strong` and `ACCELERATOR_ZERO_SPAWN_SHADOWED=<paths>`,
and the harness hard-fails when the mode is `strong` and any listed path is
still executable, or any expected target went unshadowed. Without that, a runner
image that relocates `git` turns the `sudo mv` into a no-op and the job goes
green with the property unproven.

The contract is **fail-closed on malformed input**, not just on an unshadowed
path. The harness accepts exactly `strong` or `path-only`; any other non-empty
value is a hard error, and a non-empty `ACCELERATOR_ZERO_SPAWN_SHADOWED` with a
mode other than `strong` is a hard error. The harness also *reports* the mode it
ran in and the paths it verified, and the CI step asserts `strong` was observed
after the suite exits. Otherwise a typo, a renamed variable or a step-ordering
change that drops the export silently downgrades the one job that proves the
property — the fail-open case for a gate the work item calls non-degradable.

**The restore is a shell guarantee, not a scheduler guarantee.** Shadow → run →
restore live in a single step whose script installs a `trap restore EXIT`, with
`timeout-minutes` on **that step** (shorter than the job's), so the step rather
than the job is what expires. A job-level `timeout-minutes` cancels the job, and
whether trailing `if: always()` steps run then is not a contract a system-binary
restore should rest on — a hard second cancel will not run them at all, and a
hanging suite is the most likely unattended failure. The separate `if: always()`
liveness assertion stays as the backstop.

Consider `cache: false` on this job's `mise-action`. The `jj` shadow target sits
inside the tree the action saves on its post step, so a failed restore persists a
`jj`-less tool tree into `mise-zero-spawn-v1` that every later run restores —
self-perpetuating until someone bumps the prefix. Moving the binary to
`$RUNNER_TEMP` does not avoid that (both shapes leave the tree without a working
`jj`); it only makes the loss unrecoverable within the run. The job installs one
toolchain plus `jj`, so disabling its cache is cheap and removes the hazard
outright. If the cache is kept, "bump `cache_key_prefix`" is the documented
recovery in `tasks/README.md`.

**File**: `mise.toml`
**Changes**: a `test:integration:zero-spawn` leaf running the strong-form suite,
classified `_NO_LAUNCHER_NEEDED` and placed in **`_NOT_IN_INTEGRATION_ROLLUP`**
(`tests/unit/tasks/test_mise.py:127-137`), following the precedent
`test:integration:pup` already sets for a suite owned by one dedicated job.

It stays out of the roll-up because membership would construct the ~19-fixture
matrix a *second* time per run — on both legs of `test-integration`
(`.github/workflows/main.yml:55-61`) and on every bare `mise run` via
`default` → `test` → `test:integration` — on top of `queries.rs`. Each fixture
drives several `git`/`jj` subprocesses including recursive submodule adds, and
this is the code path with a documented flake history under parallel CI load; a
flake there reddens `test-integration`, which is in `prerelease.needs`. It also
keeps the harness that reads the shadow contract off the local path. The leaf
stays runnable on demand.

**File**: `tasks/README.md`
**Changes**: a `### Zero-spawn strong form` subsection and a row in the CI table
(`:153-157`).

#### 4. Measurements

Recorded in the work item's Validation Results. Host and OS recorded; darwin-arm64
is chosen deliberately to match 0186's `B = 35.1 ms`
(`meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md:46-63`), because
the ~97 ms probe delta is macOS-specific and cross-host comparison would leave
0169's gate ill-posed.

Baseline figures already taken this session (median of 20, darwin-arm64,
warm cache, jj 0.43.0, git 2.54.0, Rust 1.90.0):

| Measurement | Median | Min | Max |
| --- | --- | --- | --- |
| cold per-process, single query (`only q4`, pure-jj) | **3.65 ms** | 3.27 | 4.18 |
| cold per-process, single query (`only q1`, plain git) | 3.65 ms | 3.46 | 3.86 |
| cold per-process, all six queries + both ports (pure-jj) | 3.66 ms | 3.20 | 4.35 |
| cold per-process, all six queries + both ports (plain git) | 4.49 ms | 3.96 | 5.21 |
| `CommandProbe` jj subprocess (`jj log -r @ -T commit_id`) | **7.05 ms** | 6.34 | 7.79 |
| `CommandProbe` git subprocess (`git rev-parse HEAD`) | 4.40 ms | 4.13 | 5.00 |

Warm in-process, same host, median of 20: first-call jj-lib 13.2 µs, first-call
gix 41.2 µs; per-query 2.6 µs (`jj_workspace_root`) to 31.0 µs (`dual_roots`).

The library-backed cold per-process cost is roughly **half** a single `jj`
subprocess, which is favourable for 0169's `G ≤ 1.1 × B` gate. Two provenance
notes carried forward: `B = 35.1 ms` is from 0186's table, not 0169's; and the
"~41 ms warm bootstrap" 0169 cites is **derived** (`149.1 − 107.9`), not
measured.

Size, `aarch64-unknown-linux-musl`, `--release`, stripped by the toolchain:

| Build | Size | Ratio |
| --- | --- | --- |
| linked | 2,422,864 B | — |
| stubbed | 391,416 B | — |
| delta | **2,031,448 B** | **6.19×** |

Darwin comparison: unstripped delta 2,058,656 B (5.62×); stripped delta
1,639,872 B (5.42×).

**These figures were taken with a feature-gated prototype** (one binary, built
twice, with the query calls stubbed by `--features stub`), whereas the delivered
shape is two sibling `[[bin]]` targets in a crate that declares both
dependencies either way. Dead-code elimination is what produces the delta in
both shapes — the prototype's stubbed build was 391 KB despite declaring `gix`
and `jj-lib` — so the figures are expected to transfer, but **Phase 4 must
re-measure against the delivered two-binary shape** and record those numbers,
not these.

### Success Criteria

#### Automated Verification

- [x] Both fixture binaries build and print every query result
- [x] `mise run test:integration:zero-spawn` passes locally in `PATH`-only mode
- [ ] The strong-form run passes in the named Linux CI job, with absolute paths
      shadowed — **not verifiable locally**; the job is written and lands with
      this branch, and macOS degrades to `PATH`-only under SIP
- [x] Both binaries cross-compile to musl and pass `_assert_static_elf`
- [x] Size: linked ≥ 3× stubbed **and** delta ≥ 1,500,000 bytes, on the
      musl-static stripped artefact (see
      [Work-Item Amendments](#work-item-amendments))
- [x] `mise run test:unit:tasks` passes, including the updated
      `test_mise.py` and `test_workflows.py`
- [x] `mise run` green end to end

#### Manual Verification

- [ ] The strong-form job's shadow step actually replaced the binaries — **CI
      only**. The harness half *is* verified: it hard-fails when
      `ACCELERATOR_ZERO_SPAWN_MODE=strong` and a listed path is still
      executable, and fails closed on a malformed mode
- [ ] Both size figures and all six cost figures are written into Validation
      Results with host and OS (**done in Phase 5**), plus one
      `x86_64-unknown-linux-musl` cold per-process figure — the gate-comparable figure stays darwin-arm64, but
      the shipped artefact is static musl and 0169 otherwise sets a threshold
      with no Linux datapoint
- [x] All four release triples cross-compile, each recorded individually
- [ ] The restore step is a `trap restore EXIT` inside the single shadow-run
      step (stronger than `if: always()`, which a job-level cancel need not
      honour), is idempotent per path, and is followed by an `if: always()`
      assertion that `git --version` and `jj --version` both succeed —
      **observable only on a CI run** — `if: always()` alone does not establish the
      guarantee, and the containment additionally assumes ephemeral runners
- [x] Nothing was staged into `dist/release/` that `_release_uploads()` does not
      enumerate

---

## Phase 5: Sibling Hand-Offs and Closeout

### Overview

Dated notes on three siblings, then Validation Results and the Open Questions on
0188 itself. The four work-item amendments already landed before implementation,
so this phase does not carry them.

**Every sibling edit is an append-only dated amendment block**, the same pattern
0188 used for its own four amendments — not an in-place rewrite of the host's
Summary, Context, Assumptions or acceptance criteria. These are live items in
the same epic that other work may be editing concurrently, and the only
automated gate (`test:integration:work`) validates frontmatter and linkage, not
content. An in-place rewrite that loses a criterion to a botched conflict
resolution is invisible; an append-only block surfaces the same conflict
visibly and leaves the prior text in the file. Where a statement is now wrong,
the amendment block says so and quotes it rather than deleting it.

### Changes Required

#### 1. `meta/work/0125-*.md`

**Changes**: a dated note in `## Dependencies` (`:141-146`) recording that 0188
dissolves the lexical-fallback rationale **for consumers that reach the Rust
adapter** — the ~26 shell call sites keep running in bash until later epic-0136
phases migrate them. Constraints 3, 4 and 5 (`:90-95`) are untouched and must
survive the note; only constraints 1 and 2 are dissolved. Add `work-item:0188`
to `relates_to` (`:11`), which currently reciprocates no edges — leaving 0169
and 0185 one-directional is a conscious choice, not an oversight.

#### 2. `meta/work/0185-*.md`

**Changes**: a dated amendment block repointing the adapter and the zero-spawn
harness from 0169 to **0188** wherever its Summary, Context, Assumptions,
Technical Notes and acceptance criteria name the wrong owner; recording that its
harness criterion (`:82-84`) describes `PATH` stubs only whereas it inherits
0188's strictly larger three-part strong form; recording that **0185 owns the
`vcs_adapters::facts` switch** and that the switch and the `CommandProbe`
deletion are one atomic change (closing Open Question 2); and that the
transitional dual-adapter `detection.rs` comparison must be collapsed when
`CommandProbe` goes. Note the two stale References anchors (`:148-153`): 0169
has no "Adapter-swap boundary" heading, and its Dependencies bullet is titled
"Unowned debt this story creates" (`0169:436`). Note that the
"0169 will need to alter this anyway" assumption (`:117-121`) is now stale.

Three further inheritances, new from the review of this plan:

- **0185 must re-home the boundary walk and `marker_kind` before deleting**
  `MarkerWalkRoot`/`CommandProbe` — `InProcessProbe` delegates to both rather
  than duplicating them, so the deletion is not mechanical.
- **The containment delta.** `CommandProbe` parses in a child process with a
  10-second cap, kill-on-timeout and a scrubbed environment.
  `InProcessProbe` parses repository-controlled data in the caller's address
  space with no time or memory bound and no crash isolation, and after the
  switch that runs inside `cli/visualiser/server` and on the hook path. 0185
  decides whether an equivalent bound is needed before flipping `facts`.
- **jj `revision` — descoped to 0185, spike-established.** jj-lib 0.43 exposes no
  read-only, settings-free route to the working-copy commit id: the op stores are
  settings-free but the workspace name is reachable only through
  `LocalWorkingCopy::load` (needs `UserSettings`) or
  `SimpleWorkspaceStore::load` (**writes** to `.jj/repo/workspace_store`,
  verified empirically), and `CheckoutState` is private. 0185's options are a
  jj-lib version exposing the checkout state publicly, an upstream request for
  one, or retaining `CommandProbe` for `revision` alone — which would make the
  "atomic switch plus deletion" sizing wrong, so it needs deciding early.
  `InProcessProbe` therefore implements `VcsProbe` **partially**: `kind` fully,
  `revision` for `VcsKind::Git` only.

#### 3. `meta/work/0169-*.md`

**Changes**: a dated note recording that it inherits the closed six-query
contract; must define its own domain port over the inherent methods; must widen
the pup rule to cover wherever it puts `status`/`log`; must reuse 0188's named
pure-jj builder; that its 0125 hand-off sub-clause is now redundant; and — new
from this planning session — that **queries 4, 5 and the jj half of 6 resolve
through a `.jj`-only walk**, so a classifier built on the combined boundary walk
would report absence where `jj workspace root` reports a root.

Four further inheritances, new from the review of this plan:

- **`WorktreeFacts`, `JjRepositoryFacts`, `JjWorkspaceRole` and `DualRoots` must
  move into `cli/vcs`** when 0169 defines its port. They are domain-shaped value
  types currently declared in the adapter crate, and
  `vcs_domain_imports_only_permitted` (`cli/pup.ron:75-89`) restricts `vcs` to
  `std`/`kernel::Error`/`crate` — so a domain port structurally cannot reference
  them where they sit. 0188 cannot pre-empt the move because its own criteria
  forbid touching `cli/vcs/src/**`. This is a planned move, not a defect; expect
  it to churn the `queries.rs` expected-value tables.
- **Dual-root equality is necessary but not sufficient** for `colocated`, and
  inequality is not sufficient for either `nested-` arm — see the note under
  Query 6. The arms also need the jj-secondary bit (Query 5) and the
  linked-worktree bit (Query 2).
- **The `JS-in` fixture** (jj secondary workspace inside its own colocated main —
  this repo's `workspaces/<name>` shape) is where differing dual roots point at
  the wrong arm: `classify_checkout` reports `jj-secondary` because its
  `nested-jj-in-git` arm additionally requires
  `jj_main_root != git_main_root`, and there they are equal.
- **The queries return `Result<Option<T>, _>`**, so 0169's classifier must decide
  what a repository the pinned library cannot parse classifies as. It is not
  `none`.

#### 4. Work-item closeout

**File**: `meta/work/0188-library-backed-vcs-adapter.md`
**Changes**: Validation Results filled in from Phases 1–4, and both Open
Questions marked closed with the answers recorded in
[Open Questions Closed](#open-questions-closed).

The four amendments are **already applied** (2026-08-03, before implementation)
— see [Work-Item Amendments](#work-item-amendments). Nothing to re-apply here.

Note that the size and cost figures quoted in this plan came from a prototype;
Phase 4's measurements against the delivered artefact are what get written into
Validation Results.

### Success Criteria

#### Automated Verification

- [ ] `mise run test:integration:work` passes (frontmatter and linkage
      validation on all four edited work items)
- [ ] `mise run` green end to end

#### Manual Verification

- [ ] Each of the three sibling notes is dated and raises information without
      re-scoping its host
- [ ] 0125's `relates_to` edge is present and 0188's reciprocal edge already
      exists
- [ ] Every *pending* line in 0188's Validation Results is resolved or
      explicitly carried forward with a reason

---

## Work-Item Amendments

Measurement during planning forced four amendments; plan review 1 forced two
more; the pin decision and the `revision` spike force a further two. Each is a
change to *this* work item, recorded here rather than absorbed silently.

**Amendments 1-4 were applied to `meta/work/0188-library-backed-vcs-adapter.md`
on 2026-08-03, before implementation started**, together with the two
informational corrections below. The work item carries a summary at the head of
its Requirements and the detail inline at each affected criterion.

**Amendments 5-8 are not yet applied** — 5 and 6 arose from plan review 1, 7 from
the pin decision and 8 from the `revision` spike. All four must land on the work
item before Phase 1 starts, by the same route.

Phase 5 does *not* apply any of them — it only fills in Validation Results and
writes the sibling hand-offs.

1. **The size-delta floor is mis-calibrated.** The criterion demands the linked
   artefact be "at least **2 MB** larger" than the stubbed one. Measured:
   musl-static stripped delta **2,031,448 B**, darwin stripped **1,639,872 B**,
   darwin unstripped 2,058,656 B. The criterion passes only on a decimal-MB
   reading of the musl build, by 1.6%, and fails outright on the stripped darwin
   build and on every MiB reading — while the trees are unambiguously linked
   (ratio **6.19×**). **Amended to**: linked ≥ 3× stubbed **and** delta
   ≥ 1,500,000 bytes (decimal, unit stated), measured on the musl-static
   stripped artefact, with darwin figures also recorded.

2. **Three walks, not one.** The Requirements describe a single boundary walk.
   Queries 4, 5 and the jj half of 6 need a **`.jj`-only** walk; using the
   boundary walk makes `DefaultWorkspaceLoaderFactory::create` return
   `Err(There is no Jujutsu repo …)` on both nested-git-in-jj shapes, where the
   oracle reports a root. Add the distinction to the Requirements.

3. **The "colocated" matrix row is two shapes.** A real
   `jj git init --colocate` main repository classifies as `main`
   (`Kind::Common`, jj main); the shell suite's hand-grafted shape classifies as
   `colocated` (`Kind::LinkedWorkTree`, jj secondary). Both are carried; the
   matrix names one row. Related: **`jj git init` colocates by default at 0.43**
   and `--no-colocate` exists, so the pure-jj fixture is a one-flag build and
   the shell's `make_main_jj_workspace` is misnamed.

4. **The single-query-mode rationale is measurably false.** The criterion states
   that timing a binary running all six queries plus both port methods "would
   inflate it by an unknown factor". Measured inflation: **0%** on pure-jj
   (3.66 vs 3.65 ms) and 23% on plain git — process startup dominates. The mode
   is retained because 0169 will want it, not because the figure would otherwise
   be unusable. Reword the rationale.

5. **The size-delta criterion needs a mechanism, not just a number.** Amendment 1
   recalibrated the threshold but left it as a figure recorded in Validation
   Results. It is now a committed assertion in `tasks/build.py`'s fixture
   staging with a unit test over the comparison, because it guards the story's
   headline false-pass and a one-off number catches no later regression.

6. **The six queries return `Result<Option<T>, _>`, not `Option<T>`.** The
   Requirements describe `Option`-returning queries. That conflates "no
   repository of this kind here" with "a repository is here and the pinned
   pre-1.0 library could not answer", and silently drops `CommandProbe`'s time
   cap, crash isolation and warn-logging when 0185 switches the composition
   root. Add the error channel to the Requirements.

7. **The pins are asymmetric, and the coupling is four-way not two-way.** The
   work item states "`jj-lib` pinned exactly at `=0.43`" (`:264`, `:538`) and
   frames upgrades as a "coordinated two-crate bump". Delivered: `jj-lib =
   "=0.43.0"` (unchanged in spirit) but `gix = "~0.85.0"`, permitting `0.85.x`
   patches so a gix RustSec fix is a lock update rather than a pin edit — the
   range is anyway identical to jj-lib's own `^0.85.0`, so the single-graph
   property is untouched. The coupling is jj-lib + gix + the Rust toolchain + the
   `mise.toml` jj CLI pin. **Amend to**: state the asymmetry and the four-way
   coupling.

8. **jj `revision` is out of scope; `InProcessProbe` implements `VcsProbe`
   partially.** The work item assumes both port methods are fully implemented.
   A spike established that jj-lib 0.43 has no read-only, settings-free route to
   the working-copy commit id (details in Phase 1 §4). **Amend to**: `revision`
   covers `VcsKind::Git`; the jj mechanism transfers to 0185, and the crate-wide
   `UserSettings` guard stays crate-wide because nothing in scope needs it.

Two further corrections, informational:

- **The `gix` feature-gating assumption resolves in favour of the pin.**
  `jj-lib` 0.43 with default features *does* pull `gix` 0.85 and enables
  `attributes`, `blob-diff`, `index`, `max-performance-safe`, `sha1`, `zlib-rs`.
  The single-graph reasoning holds as written, and `submodules()`' `attributes`
  feature comes from jj-lib rather than from gix's defaults.
- **The scrub invariant is a property to verify, not to implement**, on both
  sides — not just the git side the research had proven.

## Open Questions Opened by Plan Review 1

- ~~**How does `InProcessProbe::revision` read a jj working-copy commit id
  without a `UserSettings`?**~~ **Closed 2026-08-03 by spike**: it cannot, in
  jj-lib 0.43. Descoped to 0185; see Phase 1 §4 for the evidence and the
  consequences applied throughout.
- **Does the MPL-2.0 §3.2 notice obligation need a third-party licence artefact
  in the release payload, or does dead-code elimination remove `uluru` from every
  shipped binary?** Either answer is acceptable; the exception comment must name
  the one that holds.
- **Can gix 0.85 serve reftable and sha256 repositories?** The `RF`/`S256`
  fixtures answer it; whatever they find is recorded as a known boundary rather
  than assumed.

## Open Questions Closed

- **Who owns the CI job for the strong-form run, and does the runner permit
  it?** 0188 owns it. Feasible on the existing GitHub-hosted `ubuntu-latest`
  runners with passwordless `sudo` and no container; placed in `main.yml`
  because actionlint lints nothing else; modelled on `check-architecture` to
  stay clear of the nightly-isolation invariant. **The shadow list is rewritten**
  to be platform-aware: on CI the meaningful jj target is the mise install path
  (there is no system `jj` at all), on developer macOS it is
  `/opt/homebrew/bin/jj`. Built directly rather than behind a smoke test, per
  decision this session.
- **Who changes `vcs_adapters::facts`, 0169 or 0185?** **0185**, atomically with
  the `CommandProbe` deletion — the composition root cannot switch until nothing
  else needs the subprocess pair. 0169 wires its own classifier port without
  touching `facts`. Recorded on 0185.
- **Does a combined `allowed_only` + `denied` cargo-pup rule behave as
  intended?** Yes — verified 2026-08-03; the deny wins on overlap.
- **How is the shared fixture published?** A new `cli/vcs-test-support` crate.
- **Which "colocated" does the matrix mean?** Both; see amendment 3.
- **Does `jj git init` produce a colocated repo by default at 0.43?** Yes, and
  `--no-colocate` exists.
- **Which host do the cost measurements run on?** darwin-arm64, matching
  0186's `B`.

## Testing Strategy

### Unit Tests

- The boundary walk against both nesting directions, with the paired negative
  assertion that unbounded `gix::discover` escapes
- Each of the six queries against every (fixture, start directory) pair, with
  expected values traceable to the oracle mapping
- Absence: `None` for every query on `NONE`, and the per-query absence signals
- The superproject derivation at submodule depths 1 and 2, plus the old-form
  shape that must yield `None` while `Kind::Submodule`-only logic would miss it
- `jj_repository.role` for both the relative (`jj workspace add`) and
  absolute (hand-grafted) `.jj/repo` pointer forms

### Integration Tests

- `detection.rs` running every existing case through the injection seam against
  both implementations, with fixed expected values retained
- The scrub invariant across the whole matrix, with the live-poison control
- `cli/corpus-adapters`' cross-crate strong-form zero-spawn test
- The lockfile version invariants and the `mise.toml` ↔ `jj-lib` pin lockstep
- Non-vacuity: `std::process` failing cargo-pup, `UserSettings` failing the
  source guard, and the unscrubbed control diverging

### Manual Testing Steps

1. Confirm the strong-form CI job's shadow step took effect by asserting
   `git --version` fails inside the step before restoring.
2. Re-run the shell suites that build jj fixtures, last green against the
   pre-bump `jj` 0.36 pin: `hooks/test-vcs-detect.sh`, the work-item script
   suite under `skills/work/scripts/`, and — per the unabsorbed correction at
   `meta/prs/35-description.md:52`, which 0188 `:413-416` still omits —
   `scripts/test-metadata-helpers.sh`,
   `skills/config/migrate/scripts/test-migrate.sh`,
   `skills/config/migrate/scripts/test-migrate-interactive.sh`, and the Python
   task tests under `tests/unit/tasks/`.
3. Verify `mise install` has been run on each development machine — this session
   found the local `jj` was 0.42.0 from Homebrew because `mise.toml` was
   untrusted.
4. Read the four edited work items end to end to confirm no note re-scoped its
   host.

## Performance Considerations

Cost is measured, not gated. The figures above show the library-backed cold
per-process path at ~3.65 ms against ~7.05 ms for a single `jj` subprocess, and
warm in-process calls in the low tens of microseconds. Two ongoing costs are
structural rather than measurable here: both transitive trees enter `cargo
deny`'s `advisories` scope under `unmaintained = "all"` (`cli/deny.toml:22`), so
an unmaintained crate anywhere in the `gix`/`jj-lib` closure fails the
workspace-wide check for every unrelated change (break-glass documented in
`tasks/README.md`); and any future `jj-lib` minor bump is a coordinated
four-pin change.

Two costs the cold per-process figures do not capture: every downstream crate —
including the shipped `accelerator-visualiser` — now compiles both trees, so
cold-cache CI compiles lengthen (measured against
`test-visual-regression`'s 20-minute budget in Phase 1); and `InProcessProbe`
parses repository-controlled data in-process, trading `CommandProbe`'s
subprocess containment for speed. That trade is priced in Phase 2 §1 and handed
to 0185.

## Migration Notes

Nothing migrates at runtime. `InProcessProbe` ships unwired;
`vcs_adapters::facts` keeps naming `MarkerWalkRoot`/`CommandProbe`, so
`cli/corpus-adapters` and `cli/visualiser/server` are behaviourally unaffected by
construction. They are **not** unaffected at the build-graph level: both new
trees enter their dependency closure, and thence the shipped visualiser binary.

### Revert order

Rollback is a multi-file ordered operation, not a single-module `jj restore`,
and one ordering is actively dangerous. In order:

1. **`needs: check-zero-spawn` in `.github/workflows/main.yml`, before the job
   itself.** Removing the job while leaving the `needs` edge is a workflow-level
   validation error that stops the *whole* Main workflow from running, not just
   that job.
2. `_CLI_FIXTURE_BINARIES` and its staging loop in `tasks/build.py`, before the
   `[[bin]]` targets they name.
3. The `mise.toml` leaves: **the `depends` references first** — `lint:check`
   (`mise.toml:466`) and `cli:check` (`:424`) name `lint:vcs-settings:check`, and
   removing the task definition while a `depends` entry still names it makes
   `mise run`, `mise run check` and every lint job error out — then the task
   definitions, then the `test_mise.py` assertions (`_CHECK_GATES`,
   `_CLI_CHECK_GATES`, `_LAUNCHER_DEPENDENTS`/`_NO_LAUNCHER_NEEDED`, the
   integration roll-up), which are exhaustive equality assertions and fail if
   either side moves alone.
4. **The two `[dev-dependencies]` path edges to `cli/vcs-test-support` before the
   crate directory** — `cli/vcs-adapters` (Phase 2) and `cli/corpus-adapters`
   (Phase 3 §3), together with the test files that import it (`queries.rs`,
   `zero_spawn.rs`). Deleting the directory while either
   `path = "../vcs-test-support"` remains makes cargo fail to load the workspace
   manifest graph, which breaks *every* cargo invocation: `cli:check`,
   `server:check` (the visualiser server is a `cli/` member), `test:unit:cli`, and
   `deny:check`/`test:integration:deny`, which shell `cargo metadata`. A
   wrong-order revert fails the whole Rust and lint surface, not the module being
   reverted.
5. Then `cli/vcs-adapters/src/library.rs`, `cli/vcs-test-support` and its
   `members` entry, the two fixture binaries, the pins, the licence exception, the
   `[bans]` additions and the pup rule.
6. `cli/Cargo.lock` is **regenerated, not reverted** — once a sibling has landed
   on top, the prior lock is not a valid state to return to.

### Lock contention

`cli/Cargo.lock` is shared-artefact contention with any epic-0136 item adding
crates under `cli/`. No ordering is imposed. On conflict, drop the conflicted hunk, re-resolve `cli/Cargo.toml` first, then
run `cargo metadata --manifest-path cli/Cargo.toml`, which performs a **minimal**
lock update adding only the missing entries. Never `cargo generate-lockfile`.

Note what does and does not float. `reqwest = "=0.12.28"` and
`rustls = "=0.23.41"` are **exact** requirements (`cli/Cargo.toml:30`, `:35`), so
resolution cannot move them whatever happens to the lock — an earlier draft of
this section cited them, wrongly. What a wholesale regeneration actually floats
is the caret/tilde-bounded workspace set (`thiserror`, `tracing`, `time`,
`sha2`, `serde`, `serde_json`, `regex`, `tempfile`, `rand`, `rustix`) and the
whole ~360-package transitive closure — including the MSRV trap of Key
Discovery 11, where an unconstrained re-resolution selects a crate requiring a
Rust newer than the pinned 1.90.0. That lands an unaudited whole-graph
dependency change disguised as merge cleanup, on a repo whose supply-chain
failures block releases.

Do **not** reach for `cargo update -p gix -p jj-lib` in this scenario: after
dropping the conflicted hunk in favour of a sibling's lock, those packages are
absent from it and the command fails with "package ID specification did not
match any packages" — which is exactly the moment someone reaches for
`generate-lockfile`. `cargo add` is also wrong here: it rewrites the manifest,
replacing `{ workspace = true }` inheritance or the tilde requirement and
dropping the adjacent lockstep comment that the new test asserts is present.

The lock diff must contain only the new closure plus crates cargo was forced to
move, and the full `mise run deny:check` plus `test_vcs_library_graph.py` must
pass against the regenerated lock — not merely the single-`gix`-version
assertion.

## References

- Original work item: `meta/work/0188-library-backed-vcs-adapter.md`
- Related research:
  `meta/research/codebase/2026-08-02-0188-library-backed-vcs-adapter.md`
- Feasibility probe and the two traps:
  `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md` §9
- Split rationale:
  `meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md` pass 4
- Behavioural oracle: `scripts/vcs-common.sh`
- Ports implemented: `cli/vcs/src/lib.rs:46-67`
- Retained pair and composition root: `cli/vcs-adapters/src/lib.rs:32`, `:73`,
  `:224-227`
- Reference-artefact template:
  `cli/config-adapters/tests/fixtures/config_adapters_fixture.rs:1-49`
- Invariant-check shapes: `tests/integration/deny/test_launcher_feature_graph.py:23-51`,
  `tests/unit/tasks/test_msrv_coherence.py:30-38`
- CI job template: `.github/workflows/main.yml:286-321`
- jj loader internals: `jj-lib-0.43.0/src/workspace.rs:499-600`
- ADRs: ADR-0053 (thin CLI over a hexagonal ports-and-adapters core)
