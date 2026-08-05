---
type: plan
id: "2026-08-05-0169-vcs-subdomain-and-hooks-migration"
title: "VCS Subdomain and Hooks Migration Implementation Plan"
date: "2026-08-05T15:35:17+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0169"
parent: "work-item:0169"
derived_from: ["codebase-research:2026-08-05-0169-vcs-subdomain-and-hooks-migration"]
tags: [rust, vcs, hooks, migration]
revision: "bdfcdea501958c41e2ffac0bf3f491d2d63ac53b"
repository: "accelerator"
last_updated: "2026-08-05T18:56:50+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# VCS Subdomain and Hooks Migration Implementation Plan

## Overview

Build the `vcs detect|status|log|guard` subdomain as a new dispatched
sub-binary (`accelerator-vcs`) over the 0188 library-backed adapters, migrate
the two VCS hooks and the `config-detect.sh` registration into the CLI, and
repoint `skills/vcs/commit` at the new subcommands so the shell VCS surface
(`scripts/vcs-common.sh`'s hooks and helpers) can retire.

## Current State Analysis

The shell source is small and its contract is exact: `scripts/vcs-common.sh`'s
`classify_checkout` (`scripts/vcs-common.sh:157-280`) produces a seven-arm,
first-match-wins taxonomy; `hooks/vcs-detect.sh` and `hooks/vcs-guard.sh` each
duplicate a *simpler*, buggier `-d "$REPO_ROOT/.git"` mode check
(`hooks/vcs-detect.sh:22-37`, `hooks/vcs-guard.sh:76-81`) that misreads a
colocated checkout whose `.git` is a *file* (worktree/submodule) as pure-jj;
`scripts/vcs-status.sh`/`scripts/vcs-log.sh` check only `.jj` and are
unaffected. `hooks/vcs-guard.sh:44-108` implements compound-command splitting,
a 13-subcommand blocklist, and the deprecated `decision`/`allow` PreToolUse
shapes that must not be reproduced.

On the Rust side, `cli/vcs` (pure domain, `RepoRoot`/`VcsProbe` ports) and
`cli/vcs-adapters` (`library::InProcessProbe`'s six taxonomy queries —
`is_bare`, `worktree`, `superproject`, `jj_workspace_root`, `jj_repository`,
`dual_roots`, `cli/vcs-adapters/src/library.rs:204-343`) exist and compile, but
no `vcs` dispatch token, no `accelerator-vcs` package, and no classifier
composing the six queries exist anywhere. `WorktreeFacts`, `JjWorkspaceRole`,
`JjRepositoryFacts` and `DualRoots` are declared inside
`vcs-adapters::library` (`:157-189`), not in the domain crate, and must move
before a domain-crate classifier can reference them
(`vcs_domain_imports_only_permitted`, `cli/pup.ron:75-89`, restricts `vcs` to
`std`/`kernel::Error`/`crate` imports only).

0187 (sub-binary registration surface) and 0188 (library-backed adapters) —
the two blockers 0169's own `blocked_by` list still names — are both
functionally landed: 0187's frontmatter still reads `ready`, but its code is
on disk (`SKILL_EXEMPT_SUBBINARIES` at `tasks/shared/paths.py:31`,
`BARE_LAUNCHER` renamed, `validate_dispatch_coherence` generalised at
`tasks/shared/dispatch_coherence.py`, the thirteen-point checklist at
`tasks/README.md:304-456`). 0167 and 0182 are both `status: done` on disk,
resolving the two stale caveats in 0169's own prose.

### Key Discoveries

- **The hook-schema floor check (Sequencing Constraint 1) is resolved,
  confirmed ahead of any envelope code.** A synthetic PreToolUse hook was run
  against both the then-current Claude Code client and the declared floor
  (v2.1.144, via `npx @anthropic-ai/claude-code@2.1.144`) on 2026-08-05:
  `permissionDecision:"deny"` blocked a matching Bash call, and a bare
  top-level `{"systemMessage":...}` (no `permissionDecision` key) let the call
  through while still surfacing the message, at both versions. No exit-2
  fallback is needed; the `permissionDecision`/bare-`systemMessage` envelope
  design in Phase 4 stands as specified. See the work item's Sequencing
  Constraint 1 and Validation Results.
- **`cli/launcher` has no `[lib]` target** (`cli/launcher/Cargo.toml:1-16` —
  only two `[[bin]]` entries). Since `accelerator-vcs` is dispatched as an
  `External` subcommand (`cli/launcher/src/launch/inbound/cli.rs:26-28`), not
  a launcher built-in, it **cannot** depend on `cli/launcher` as a library.
  This settles the work item's own open question about where the shared
  hook-envelope module lives: it must be `kernel` (`cli/kernel`), the only
  crate both `cli/launcher` and the new `accelerator-vcs` binary can share.
  `kernel::Error` is already the documented "lowest crate, cannot name a
  subdomain's type" boundary (`cli/kernel/src/lib.rs:7-19`).
- **A launcher-level fail-safe gap exists and blocks two of the guard's
  fail-open acceptance criteria.** Grepping `cli/launcher/src/launch/` for
  `fail_safe` outside `config_command` returns nothing: `--fail-safe` is
  consumed two ways today, neither of which covers external-dispatch
  resolution failure. The bootstrap's own scan (`bin/accelerator:28-39`) only
  wraps the bootstrap's *own* fetch/verify of the `accelerator` launcher
  binary and the verify shim — by the time the Rust launcher's own
  `LazyProductionResolver::resolve` (`cli/launcher/src/main.rs:56-69`) tries
  to fetch `accelerator-vcs` and fails (unreachable host, missing manifest
  entry), the bootstrap shell process has already been replaced by `exec` and
  cannot intervene. That failure returns as `kernel::Error::Failed`, mapped to
  `ExitCode::FAILURE` (exit 1) by `report()` (`cli/launcher/src/main.rs:203-212`),
  with zero awareness of the `--fail-safe` token sitting in the forwarded
  argv. The work item states plainly that a PreToolUse hook exiting non-zero
  *is* a blocking error — so this gap must close, or criteria (a) and (b) of
  "the guard fails open, three ways" cannot pass. This is new work the
  research pass did not surface.
- **`vcs status`/`vcs log` cannot be produced from the six taxonomy
  queries** — none of them render `jj status`/`git diff --stat`-shaped text,
  and reimplementing that formatting against `gix`/`jj-lib` would be a
  disproportionate undertaking with no byte-parity guarantee. The existing
  `vcs_adapters::subprocess` module (`cli/vcs-adapters/src/subprocess.rs`)
  already establishes that shelling `jj`/`git` for VCS interaction is a
  legitimate, first-class adapter pattern in this codebase (used today for
  `VcsProbe`). `vcs status`/`vcs log` follow the same pattern: an in-process
  check decides jj-vs-git (matching `vcs-status.sh:9`/`vcs-log.sh:9`'s
  `.jj`-only branch, confirmed unaffected by the `.git`-as-file correction),
  then a subprocess adapter execs `jj status`/`git diff --cached --stat` (or
  `jj log --limit 5`/`git log --oneline -5`) and passes stdout through
  verbatim, matching the shell's `2>/dev/null || echo "(... unavailable)"`
  fallback. `vcs_adapters_library_reads_in_process`
  (`cli/pup.ron:100-121`) stays scoped to `vcs_adapters::library` and is not
  widened to forbid this — the Amendment's "widen wherever `vcs status`/`vcs
  log` land" note is read here as: wherever those subcommands need an
  in-process facts read (deciding jj-vs-git), that read must go through the
  existing `library`-backed probe rather than a fresh ad-hoc filesystem check
  — not as a ban on subprocess execution for the terminal status/log text,
  which has no in-process substitute.
- **The `.git`-as-file correction falls out of switching from a shell `-d`
  test to the library-backed queries, not from new logic.** `vcs-detect.sh`'s
  and `vcs-guard.sh`'s mode checks are literal `[ -d "$REPO_ROOT/.git" ]`
  tests, which are blind to a `.git` *file* (worktree/submodule marker).
  `gix::discover` (which every `InProcessProbe` git query goes through) walks
  and opens the repository correctly regardless of whether `.git` is a file
  or directory. So the classifier's `dual_roots()`/`worktree()` queries
  already answer "is there a git side here" correctly for both cases — the
  work item's departure is a natural consequence of building `vcs detect`
  and `vcs guard` on the library queries rather than a bespoke fix.
- **`classify_checkout`'s six-query composition is now traceable end to
  end**: `is_bare()` gates git eligibility exactly as the shell's
  `is-bare-repository` check does; `dual_roots()` supplies both single-VCS
  boundary values (`git` = the checkout's own toplevel via `gix`'s discover
  walk, `jj` = the jj workspace root); `worktree()`'s `linked`/
  `main_worktree_root` fields supply `git_worktree`/`git_main_root`;
  `jj_repository()`'s `role`/`main_root` supply `jj_secondary`/`jj_main_root`.
  `jj_workspace_root()` and `superproject()` are not independently needed by
  the cascade (the first duplicates `dual_roots().jj`'s walk; the second's
  submodule-of-worktree handling is exercised only through the existing
  `vcs-test-support` fixture matrix, not hand-derived here) — Phase 3 verifies
  this composition against that matrix rather than re-deriving it from shell
  prose.
- **The 42-case parity-gate partition is now exact** (verified by reading the
  full 713-line file): 26 in-process `vcs-common.sh` cases, 8 subprocess
  `HOOK`-invoking cases, 5 missing-binary cases (deleted — no external binary
  is consulted by the Rust port), 2 named singletons (the `hooks.json`
  literal assertion and the top-of-file comment-block grep), **plus one case
  the work item's own arithmetic misses**: a static golden-snapshot
  host-artefact check (`hooks/test-vcs-detect.sh:196-202`) that calls neither
  a `vcs-common.sh` function nor `$HOOK`. 26+8+5+2+1 = 42. The AC8
  `hooks.json` assertion (`:620-634`) does today hardcode
  `.hooks.SessionStart[0]`, confirming it must be rewritten order-independent
  as the work item requires.
- **`test_github.py`'s upload-count assertion is already derived, not a
  hardcoded 22** (`tests/integration/tasks/test_github.py:456`: `assert
  len(uploads) == len(DISPATCHED_SUBBINARIES) * len(_PLATFORMS) * 2`) — the
  `tasks/README.md` checklist's point 1 describes updating a hardcoded count
  as "expected of the first sibling to land," but that conversion is already
  done, so adding `"vcs"` to `DISPATCHED_SUBBINARIES` needs no test-count
  edit there.
- **`accelerator_env()` (`tasks/test/helpers.py:17-40`) already exists** and
  documents its own `build:cli:dev` dependency; wiring the repointed parity
  gate onto it is additive, not new infrastructure. `tests/unit/tasks/test_mise.py:61`
  is the exact pin naming `test:integration:hooks` as currently running with
  no `accelerator_env`, confirmed as the one line this story must update.

## Desired End State

`accelerator-vcs` is a registered dispatched sub-binary implementing `vcs
detect|status|log|guard`, reproducing the shell's decisions byte-for-byte
except the four declared departures (the PreToolUse envelope shape; the
`.git`-as-file colocated correction; `vcs detect`'s default output narrowing
to structured-only, with the shell's always-on reference text moved behind a
new `--descriptive` flag — see Phase 5; `vcs guard`'s compound-command
splitter becoming quote-aware instead of porting the shell's quote-blind
split — see Phase 1 and Phase 7). `hooks.json` registers three verbatim
command strings against the CLI instead of the five shell scripts, all five
shell files are deleted, `skills/vcs/commit` invokes the new subcommands, and
`mise run` is green end to end.

**Verification**: every phase below states its own automated and manual
success criteria; the story-level acceptance criteria in
`meta/work/0169-vcs-subdomain-and-hooks-migration.md` are the composite
target and are cross-referenced per phase.

## What We're NOT Doing

- Not removing `log`/`diff` from the guard's blocked set — reproduced
  verbatim; a follow-up work item owns that decision (Phase 10).
- Not touching `scripts/vcs-common.sh`'s `find_repo_root`/`vcs_mode`, which
  keep their other ~20 callers; not touching
  `hooks/migrate-discoverability.sh` or `hooks/launcher-link-refresh.sh`.
  A follow-up work item owns the `vcs-common.sh` residue and
  `launcher-link-refresh.sh` (Phase 10).
- Not adding a `config detect` subcommand — the SessionStart config summary
  behaviour already ships via `config summary --format=hook`.
- Not performing the release cut Sequencing Constraint 4 requires before
  `hooks.json`'s rewrite reaches an installed-plugin path — that is a
  release-process action outside this plan's code changes, flagged as a
  deployment gate on Phase 9, owned by whoever runs epic-0136 releases.
- Not collapsing sub-binary registration to a single allowlist entry — 0187
  explicitly scoped that out; this plan follows the existing multi-step
  checklist.
- Not reimplementing `jj status`/`git diff --stat` text rendering against
  `gix`/`jj-lib` — `vcs status`/`vcs log` shell the real binaries (see Key
  Discoveries).
- Not re-scoping 0172, 0183, or 0125 beyond the dated hand-off notes and
  follow-up items the work item's acceptance criteria require.

## Implementation Approach

Ten phases, each independently mergeable and each leaving `mise run` green.
Fixture capture comes first (Sequencing Constraint 2: shell behaviour must be
committed as goldens in a commit preceding any deletion). The domain-crate
type move is isolated from the classifier that depends on it. The four `vcs`
subcommands are built as separate vertical slices (detect; status+log; guard)
so each phase's diff is a complete, working increment rather than a stub.
Registration and the skill repoint land together, per the checklist's own
"points 1 and 7 in the same change" rule. The `hooks.json` rewrite and shell
deletions land last, gated on the release-process note above rather than on
any further code dependency.

---

## Phase 1: Capture Shell Behaviour as Fixtures

### Overview

Commits the shell's current behaviour as goldens before any of it is deleted
(Sequencing Constraint 2), so every later phase has a fixed, testable target
instead of the shell source itself.

### Changes Required

#### 1. Volatile-field mask set

**File**: `hooks/test-fixtures/masks.toml` (new)

A machine-readable table of named regex patterns (not prose), each with a
comment naming what it masks: hex object ids (7-40 chars), jj change ids
(32-char non-hex), ISO-8601 timestamps, jj's space-separated timestamp
format, relative age strings, the fixture tempdir path, and author identity.
Machine-readable so the Python golden-generation script (item 2) and the
Rust comparison harness (Phase 6) load and apply the *same* patterns rather
than each reimplementing them — the two-implementation drift that would
otherwise let a looser comparison-side mask silently swallow a real
regression. Closed once committed — no mask added later to make a failing
golden pass (this is itself an acceptance criterion of the work item).

#### 2. `vcs status`/`vcs log` goldens

**File**: `hooks/test-fixtures/vcs-status-log/` (new directory)

A Python fixture-generation script builds each of the nine states (clean git,
dirty git — one untracked + one modified tracked + one staged; git
ahead/behind — a local clone two commits ahead, one behind upstream;
detached-HEAD git; clean jj; dirty jj; colocated; jj secondary workspace; no
repository at all), runs the real `scripts/vcs-status.sh` and
`scripts/vcs-log.sh` against each, and commits the masked output as
`<state>-status.txt` / `<state>-log.txt` golden pairs.

#### 3. `vcs guard` decision table

**File**: `hooks/test-fixtures/vcs-guard/decision-table.json` (new)

Generated by running `hooks/vcs-guard.sh` with each of the 34 command cases
(13 blocked git subcommands, 7 allowed, `gh`, `rtk`, 12 compound — 4
separators × {match-first, match-later, no-match}) against each of the 4 repo
modes (pure-jj, colocated, git, non-repo): 136 rows, one JSON object per row
recording `{repo_mode, command, decision, reason_pattern}`. A 137th row
captures the `.git`-as-file colocated case as a hand-authored **deliberate
divergence** (today's shell blocks it as pure-jj; the row records the
corrected expectation — warn, not block — per the departure the work item
declares).

**Fourth declared departure**: the shell's compound-splitter is quote-blind (a
plain `sed`-then-split, `hooks/vcs-guard.sh:44-70`), so `git commit -m "build
&& test"` is wrongly split inside the quoted string, treating the embedded
`&&` as a real separator. This is not reproduced — the Rust port's splitter is
quote-aware (tracking single- and double-quote state, splitting on `&&`/`||`/
`;`/`|` only when unquoted), fixing the bug rather than porting it, since
nothing about behaviour-preservation parity is served by carrying forward a
parsing defect discovered during the rewrite. A 138th row, **hand-authored**
(not captured — the shell is not the oracle for a declared departure, per the
same rule as the `.git`-as-file case), records the corrected expectation for
`git commit -m "build && test"` against a pure-jj repo: the quoted `&&` is
not treated as a separator, so the whole string is evaluated as a single
`commit` invocation and blocked accordingly — whereas today's shell
over-splits and evaluates `test"` as a spurious second segment.

#### 4. Third `vcs detect` fixture (`--descriptive`)

**File**: `hooks/test-fixtures/vcs-detect/colocated-git-as-file.json` (new)

Hand-authored (not captured from the shell, which gets this case wrong) —
marked in the fixture as the new expectation: mode `jj-colocated` for a
colocated checkout whose `.git` is a file. Captured under `--descriptive`,
matching the two pre-existing detect fixtures (`main-jj-workspace.json`,
`main-git-checkout.json`) — the shell has no non-descriptive mode, so all
three of today's byte-parity fixtures represent `--descriptive` output.

#### 5. Structured (non-`--descriptive`) `vcs detect` fixture

**File**: `hooks/test-fixtures/vcs-detect/jj-secondary-structured.json` (new)

Hand-authored — the shell has no structured-only mode, so this fixture
defines the new default-invocation contract rather than deriving it: for a
jj-secondary checkout (a boundary-carrying case), `vcs detect` without
`--descriptive` emits only the boundary block (mode, boundary path,
`jj_parent`), with no VCS command reference text. Paired with the existing
"success with nothing to report" criterion (a main checkout, no boundary,
default invocation → zero bytes), this pins both ends of the default output's
range.

**Path placeholders**: since a jj-secondary checkout is boundary-carrying, its
`additionalContext` necessarily contains real filesystem paths that vary per
test run — unlike the three pre-existing/departure detect fixtures, which are
all `Main`-classified (no boundary) and so contain no paths at all. The
committed fixture uses literal `<BOUNDARY_PATH>` / `<JJ_PARENT_PATH>` tokens
(the same `<PATTERN_NAME>` convention as `masks.toml`); the Phase 5 comparison
test must mask its own dynamically-built fixture's real paths to these exact
tokens before the byte-comparison, rather than the fixture embedding a
specific host path. Also: the structured contract deliberately drops the
shell's leading double-blank-line quirk in `build_boundary_block` (present
only because that text was designed to be appended after the cheat-sheet
CONTEXT prose) — with no CONTEXT prefix in structured mode, the boundary block
starts directly with `WORKSPACE BOUNDARY DETECTED` and carries no trailing
blank line.

### Success Criteria

#### Automated Verification

- [x] The fixture-generation script runs cleanly and produces every listed
      golden file: `uv run python hooks/test-fixtures/generate_vcs_goldens.py`
      — produces 10 golden pairs (9 prose-enumerated states, "git ahead" and
      "git behind" split into two pairs from one bullet)
- [x] `mise run test:integration:hooks` still passes unchanged (nothing in
      this phase touches the existing suites yet) — 131 shell-suite
      assertions unchanged, plus the new `tests/integration/hooks/test_masks.py`
      auto-discovered by the existing `pytest tests/integration/hooks`
      invocation (24 items total)
- [x] `hooks/test-fixtures/masks.toml` exists and covers every field category
      named above (checked by a new unit test asserting the file contains the
      six named patterns), and is loadable by both the Python
      fixture-generation script and the Phase 6 Rust comparison harness —
      note: the prose names 7 categories (hex object ids, jj change ids,
      ISO-8601 timestamps, jj space-separated timestamps, relative age,
      fixture tempdir path, author identity); the "six" in this bullet
      appears to be an off-by-one in the plan text, so all 7 are implemented
      and asserted
- [x] Each named pattern is pinned by a positive/negative sample pair — held
      in `masks.toml` itself (`sample_match`/`sample_no_match` fields per
      pattern) rather than duplicated in each test file, so both engines pin
      against the identical source of truth
- [x] A cross-engine differential test: `cli/vcs-test-support/tests/masks.rs`
      (Rust `regex`) and `tests/integration/hooks/test_masks.py` (Python
      `re`) independently load the same `masks.toml` and assert the same
      match/no-match outcomes — a pattern the two engines interpret
      differently fails exactly one suite

#### Manual Verification

- [x] Spot-checked golden files by eye (clean-jj, dirty-jj, colocated,
      jj-secondary, dirty-git, git-ahead, detached-head-git, no-repo) —
      masking is correct, no leaked temp paths/hex ids/change ids/timestamps/
      emails; confirmed the mise-pinned jj (0.43.0) defaults `git.colocate`
      to `true`, so the golden generator explicitly overrides
      `--config git.colocate=false` to build genuinely pure-jj states
      distinct from the colocated state

---

## Phase 2: Move Checkout Types into the `vcs` Domain Crate

### Overview

Relocates `WorktreeFacts`, `JjWorkspaceRole`, and `JjRepositoryFacts` from
`vcs-adapters::library` into `vcs`, purely mechanical except for `DualRoots`,
whose two `Result<Option<PathBuf>, library::Error>` fields must be retyped to
`Result<Option<PathBuf>, kernel::Error>` — the domain crate cannot import
`vcs-adapters`'s adapter-specific `Error` type
(`vcs_domain_imports_only_permitted`, `cli/pup.ron:75-89`), so
`InProcessProbe::dual_roots` maps its internal `library::Error` into
`kernel::Error` at the crate boundary, mirroring the existing
`ResolutionError`-into-`kernel::Error` pattern
(`cli/launcher/src/launch/core.rs:167-171`).

### Changes Required

#### 1. Domain crate gains the checkout types

**File**: `cli/vcs/Cargo.toml`
**Changes**: add `kernel = { path = "../kernel" }` to `[dependencies]`
(currently empty — its own comment states "nothing does yet" need
`kernel::Error`, but `DualRoots`' retyping below is the first thing that
does). Without this, `cli/vcs` fails to compile the moment `checkout.rs`
references `kernel::Error`, which would also block Phase 3's `classify()`
and `CheckoutProbe` (both of which use `kernel::Error` in their
signatures) and every test added in this phase and Phase 3. Mirrors the
precedent `cli/corpus/Cargo.toml` already set when `corpus` first needed
`kernel::Error` ("the kernel dependency is added here").

**File**: `cli/vcs/src/checkout.rs` (new)
**Changes**: `WorktreeFacts`, `JjWorkspaceRole`, `JjRepositoryFacts`,
`DualRoots` (moved from `cli/vcs-adapters/src/library.rs:157-189`), `DualRoots`
retyped to use `kernel::Error`.

```rust
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeFacts {
    pub linked: bool,
    pub git_dir: PathBuf,
    pub common_dir: PathBuf,
    pub main_worktree_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JjWorkspaceRole {
    Main,
    Secondary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JjRepositoryFacts {
    pub role: JjWorkspaceRole,
    pub main_root: PathBuf,
}

#[derive(Debug)]
pub struct DualRoots {
    pub git: Result<Option<PathBuf>, kernel::Error>,
    pub jj: Result<Option<PathBuf>, kernel::Error>,
}
```

#### 2. Adapter re-exports and boundary mapping

**File**: `cli/vcs-adapters/src/library.rs`
**Changes**: remove the four moved definitions; import them from `vcs`;
`dual_roots()` maps `Error` into `kernel::Error` via `.map_err(Into::into)` (or
an explicit `From<Error> for kernel::Error` impl, matching the
`ResolutionError` precedent).

#### 3. Expected-value tables

**File**: `cli/vcs-adapters/tests/queries.rs`
**Changes**: update imports from `vcs_adapters::library::{WorktreeFacts, ...}`
to `vcs::checkout::{WorktreeFacts, ...}`; update any `DualRoots` field
construction in the oracle-mapping table to the new `kernel::Error` type.

### Success Criteria

#### Automated Verification

- [x] `mise run cli:check` passes (rustfmt, clippy, cargo-pup)
- [x] `cargo test -p vcs -p vcs-adapters --locked` passes (via
      `mise run test:unit:cli` or the crate-scoped equivalent) — 46 passed
      (10 suites) with `--all-features`
- [x] `mise run pup:check` passes — confirms `vcs` still imports only
      `std`/`kernel::Error`/`crate`
- [x] `mise run deny:check` passes — note: this phase *does* add a dependency
      edge (`kernel` to both `vcs` and `vcs-adapters`, both already in the
      workspace graph), so "no dependency change" undersells it slightly, but
      no new external crate enters the graph and deny passes with the same
      pre-existing duplicate-version warnings as before this phase

#### Manual Verification

- [x] None — this phase is a pure mechanical relocation verifiable entirely
      by the existing test suite

---

## Phase 3: The Checkout Classifier

### Overview

Adds the seven-arm `classify_checkout` taxonomy to `cli/vcs`, composed over
the six `InProcessProbe` queries via a new domain port, test-driven against
`vcs-test-support`'s existing fixture matrix.

### Changes Required

#### 1. Classification type and port

**File**: `cli/vcs/src/classify.rs` (new)
**Changes**: a `Classification` enum with the seven authoritative arms
(`Main`, `JjSecondary`, `GitWorktree`, `Colocated`, `NestedJjInGit`,
`NestedGitInJj`, `None`) carrying optional `boundary`/`jj_parent`/`git_parent`
paths, and a `CheckoutProbe` port trait narrowing `InProcessProbe`'s six
methods to exactly what the cascade needs (`is_bare`, `worktree`,
`jj_repository`, `dual_roots`).

```rust
pub enum Classification {
    Main,
    JjSecondary { boundary: PathBuf, jj_parent: PathBuf },
    GitWorktree { boundary: PathBuf, git_parent: PathBuf },
    Colocated { boundary: PathBuf, jj_parent: PathBuf, git_parent: PathBuf },
    NestedJjInGit { boundary: PathBuf, jj_parent: PathBuf, git_parent: PathBuf },
    NestedGitInJj { boundary: PathBuf, jj_parent: PathBuf, git_parent: PathBuf },
    None,
}

pub trait CheckoutProbe {
    fn is_bare(&self, start: &Path) -> Result<Option<bool>, kernel::Error>;
    fn worktree(&self, start: &Path) -> Result<Option<WorktreeFacts>, kernel::Error>;
    fn jj_repository(&self, start: &Path) -> Result<Option<JjRepositoryFacts>, kernel::Error>;
    fn dual_roots(&self, start: &Path) -> DualRoots;
}

pub fn classify(
    start: &Path,
    probe: &dyn CheckoutProbe,
) -> Result<Classification, kernel::Error> { .. }
```

Cascade order matches `scripts/vcs-common.sh:229-272` exactly: `colocated`
before the two `nested-*` arms (load-bearing — a true colocated checkout
satisfies both nested predicates too).

**Two different kinds of `Err` are handled two different ways**, matching the
trait's own fallibility shape (three hard-fallible methods, one
comparison-shaped type): `is_bare`, `worktree`, and `jj_repository` each
return `Result<Option<T>, kernel::Error>` — an `Err` from any of these means
the cascade cannot safely determine an arm at all (there is no sensible
degraded value; propagating a *wrong* arm is worse than reporting failure),
so `classify()` propagates it as `Err(kernel::Error)`, giving `vcs-cli` the
signal it needs for the adapter-failure output contract. `dual_roots`,
however, returns the `DualRoots` type directly — its two sides
(`Result<Option<PathBuf>, kernel::Error>` each) are a *comparison*, not a
fact lookup, and per the Amendment's finding 4 an `Err` on one side must be
treated as "not comparable," never as a false inequality; this degrades to a
single-VCS arm or `None` within a still-`Ok` classification, exactly as
described in the original draft. The distinction is deliberate: a failure to
determine "is this bare" is a genuine adapter failure; a failure to compare
two roots that *were* otherwise determined degrades gracefully because the
cascade can still produce a defensible single-VCS answer from what it does
know.

#### 2. Adapter impl of the narrowed port

**File**: `cli/vcs-adapters/src/library.rs`
**Changes**: `impl vcs::classify::CheckoutProbe for InProcessProbe` delegating
to the existing inherent methods.

#### 3. Test placement: pure cascade tests in `vcs`, real-fixture matrix in `vcs-adapters`

**File**: `cli/vcs/src/classify.rs` (test module)
**Changes**: `classify()`'s cascade-ordering and closed-set-variant tests use a
hand-rolled test-double `CheckoutProbe` (no fixtures, no external toolchain) —
these need no new dev-dependency and run unconditionally under
`cargo test -p vcs --locked`.

**File**: `cli/vcs-adapters/tests/classify.rs` (new)
**Changes**: the fixture-matrix test iterating `vcs_test_support::fixtures::Matrix`'s
existing ~34-fixture set against `vcs::classify::classify(start,
&InProcessProbe)` lives here, not in `vcs` — `vcs-adapters` already has both
`vcs` and `vcs-test-support` as dependencies (the latter as a dev-dependency),
so this needs no new Cargo wiring. It also sidesteps a real risk: adding
`vcs-test-support`/`vcs-adapters` to `vcs` (even as a dev-dependency) risks
tripping `vcs_domain_imports_only_permitted`, whose `Module("^vcs($|::)")`
match likely covers `vcs`'s own `#[cfg(test)]` modules too. Gated behind the
existing `bash-parity` feature, matching `vcs-adapters`' established
convention exactly (`cli/vcs-adapters/Cargo.toml`) — no new feature is
introduced.

### Success Criteria

#### Automated Verification

- [x] `cargo test -p vcs --locked` passes unconditionally (no feature flag
      needed) — the pure cascade-ordering and closed-set tests (21 tests)
- [x] A test-double `CheckoutProbe` returning `Err` from `is_bare`,
      `worktree`, or `jj_repository` (independently, one case per method)
      asserts `classify()` propagates `Err(kernel::Error)` rather than
      degrading to a `Classification` arm — this is the mechanism Phase 5 and
      Phase 7's adapter-failure success criteria depend on
- [x] A test-double `CheckoutProbe` returning `Err` on exactly one side of
      `dual_roots` (git or jj, independently) while the other three methods
      succeed asserts `classify()` still returns `Ok`, degrading to the
      correct single-VCS arm rather than propagating `Err` or misreading the
      failed side as absent. Two sub-cases per side implemented: the *other*
      side reporting `Ok(Some(_))` (degrades to the other side's single-VCS
      arm) and the *other* side reporting `Ok(None)` (degrades to `Main`,
      not `None` — since `jj_repository`, hard-fallible, still confirms a
      repository is present even though its secondary boundary is
      unparseable)
- [x] A test-double `CheckoutProbe` returning `Err` on **both** sides of
      `dual_roots` simultaneously, with the other three methods succeeding,
      asserts the specific resulting arm explicitly (`Main`) rather than
      leaving it implicit
- [x] `cargo test -p vcs-adapters --locked --features bash-parity` iterates
      the full 34-fixture matrix and asserts `classify()` returns `Ok` with
      the expected arm for every one, including the named ambiguous case (CG
      — a colocated checkout nested inside another repository classifies
      `Colocated`, not a `Nested*` arm) and 4 error/degenerate cases
      (S256/D1/D3 hard-fail, D2 degrades to `None`). Two expectations in the
      original derivation were corrected empirically against real git/jj
      rather than assumed: `SM-w` (a linked worktree of a submodule) has no
      resolvable `main_worktree_root` (the oracle table already recorded
      `main=absent`), so it degrades to `Main` rather than `GitWorktree`
      with a fabricated parent
- [x] A closed-set test asserts `Classification` has exactly seven variants
- [x] `mise run cli:check` passes (rustfmt, clippy, cargo-pup); full
      workspace `cargo test --workspace --all-features`: 1114 passed

#### Manual Verification

- [x] None

---

## Phase 4: Shared Hook Envelope Module

### Overview

Adds a `kernel::hooks::envelope` module carrying every JSON shape both
`config summary --format=hook` and the new `vcs` subcommands need — the
existing SessionStart envelope, the new PreToolUse `permissionDecision` deny
shape, the bare `systemMessage`-only warn shape, and the shared
adapter-failure shape — then refactors `config_command`'s existing
`hook_envelope` to delegate to it rather than duplicating the JSON
construction.

### Changes Required

#### 1. The envelope module

**File**: `cli/kernel/src/hooks.rs` (new)
**Changes**:

```rust
pub fn session_start(context: &str, system_message: Option<&str>) -> String { .. }
pub fn pre_tool_use_deny(reason: &str) -> String { .. }
pub fn pre_tool_use_warn(system_message: &str) -> String { .. }
pub fn adapter_failure(system_message: &str) -> String { .. }
```

`pre_tool_use_warn` emits a **bare** top-level `{"systemMessage":...}` with no
`hookSpecificOutput` and no `permissionDecision` key at all — confirmed safe
by this story's own hook-schema check: an absent `permissionDecision` falls
through to the normal permission flow rather than being read as a decision.
`json_escape` moves here too (RFC 8259 escaper, unchanged from
`config_command/render/summary.rs:76-101`).

#### 2. `config_command` delegates instead of duplicating

**File**: `cli/launcher/src/config_command/render/summary.rs`
**Changes**: `hook_envelope` becomes a thin wrapper calling
`kernel::hooks::session_start`; local `json_escape` removed.

### Success Criteria

#### Automated Verification

- [ ] New `kernel::hooks` unit tests pin each shape's literal JSON output
      (mirroring the existing `hook_envelope` test's style)
- [ ] Existing `config_command` summary tests still pass unchanged — no
      behavioural regression to the already-shipped SessionStart contract:
      `cargo test -p accelerator --locked`
- [ ] `mise run cli:check` passes

#### Manual Verification

- [ ] None

---

## Phase 5: Launcher Fail-Safe for External Dispatch, and `vcs detect`

### Overview

Closes the launcher-level fail-safe gap identified in Key Discoveries, closes
the cache-root probe cost that would otherwise leave Phase 10's warm-call
latency gate unmeetable on every external dispatch (not just `vcs`), then
scaffolds the `accelerator-vcs` crate and implements `vcs detect`.

### Changes Required

#### 1. Fail-safe-aware external dispatch

**File**: `cli/launcher/src/launch/core.rs`
**Changes**:
- a `forwarded_fail_safe(args: &[OsString]) -> bool` helper scanning up to the
  first `--` for a literal `--fail-safe` token, mirroring
  `bin/accelerator:28-39`'s own semantics exactly;
- a pure `swallow_under_fail_safe(error: &kernel::Error, args: &[OsString]) ->
  bool` combining it with an explicit **allowlist**, not a `Refusal`
  exclusion: `forwarded_fail_safe(args) && matches!(error,
  kernel::Error::Failed(_))`. Only `Failed` (the `ResolutionError` catch-all)
  is swallowable. This is deliberately narrower than "anything except
  `Refusal`" — `kernel::Error` has a third variant, `LogFilter` (from
  `kernel::logging::init()`, called before dispatch even runs), unrelated to
  external-dispatch resolution; an exclusion-based predicate would swallow it
  too under `--fail-safe`, and in that specific case the `tracing::warn!`
  diagnostic below would have no subscriber to write to, since logging never
  finished initialising. The allowlist form falls through to `report()` for
  both `Refusal` and `LogFilter` uniformly, unit-testable directly against
  literal `kernel::Error` values, no test doubles required for the pure-logic
  case;
- `From<ResolutionError> for kernel::Error` changes from an unconditional
  `Self::Failed(error.to_string())` to mapping four variants unconditionally
  to `Self::Refusal(error.to_string())` — `ChecksumMismatch`,
  `SignatureMismatch`, and `ManifestSignature` (the three trust-chain
  integrity checks on a first-fetch), plus `ManifestVersionMismatch`. The
  last is independently justified by the actual release pipeline, not
  assumed: `tasks/release.py`'s `_assert_staged_manifest_is_current` and
  `tasks/build.py`'s `validate_version_coherence` force `manifest.json` and
  every published binary to the same version before publish;
  `github.create_release` stages as `--draft` and flips visible only after
  every asset re-verifies together (`tasks/github.py`); and the launcher
  fetches its manifest from `.../releases/download/v{CARGO_PKG_VERSION}`
  (`cli/launcher/src/main.rs`) — its own exact compiled version, not
  "latest". A running launcher can only ever observe a
  `ManifestVersionMismatch` if the manifest it fetched for its own release
  tag was altered after publication — there is no benign-skew window the
  pipeline can produce. Every other variant (network/availability failures:
  `Fetch`, `AssetNotFound`, `ReleaseUnavailable`, `CacheRootUnavailable`,
  etc.) stays `Failed`, including `UnsupportedSchema` — despite being raised
  from the same `parse_and_validate` call, immediately beside
  `ManifestVersionMismatch` (`cli/launcher/src/launch/outbound/resolve/manifest.rs`),
  it signals a client-too-old-for-the-manifest condition expected during an
  ordinary schema rollout, not a content-integrity violation; the two checks
  sit next to each other in the code but answer different questions
  ("is this the right binary" vs. "can this client understand the manifest
  shape at all"), so this is a deliberate exclusion, not an oversight. This
  is the load-bearing change: it makes the existing `Refusal` exclusion apply
  to real cases instead of being unreachable dead code, so a corrupted or
  improperly-signed sub-binary is never silently swallowed alongside an
  ordinary network hiccup — it now surfaces as a genuine, non-zero-exit
  failure.

  `CorruptCacheAndRefetchFailed` needs a fifth, *conditional* case, not an
  unconditional one: reading `FetchVerifyCacheResolver::reverify`
  (`cli/launcher/src/launch/outbound/resolve/mod.rs`), it can fail two
  structurally different ways — a genuine `ChecksumMismatch`/
  `SignatureMismatch` (confirmed tampering), **or** a plain
  `ResolutionError::Cache` I/O error from `std::fs::read`/`read_to_string`
  (a permissions change, a concurrent process touching the cache file, a
  transient disk error — nothing to do with tampering). `resolve()`'s current
  `Err(_) => { .. }` arm discards which one occurred before constructing
  `CorruptCacheAndRefetchFailed`, so an I/O hiccup combined with a failed
  refetch would be misclassified as confirmed tampering under an
  unconditional mapping — hard-failing the hook instead of failing open,
  which directly contradicts the work item's "any failure... must let the
  Bash call through" requirement. The fix changes `resolve()`'s match itself,
  not just the `From` impl: split the `reverify()` failure arm on its error
  type. On `ChecksumMismatch`/`SignatureMismatch` (confirmed integrity
  failure), attempt the self-heal refetch and wrap *any* refetch outcome —
  success or failure — in `CorruptCacheAndRefetchFailed` on failure, since
  the local detection alone is sufficient evidence regardless of why the
  retry failed; this variant is now unconditionally safe to map to `Refusal`
  because it can only be constructed from a confirmed-integrity path. On a
  plain `Cache` I/O error, attempt the self-heal refetch too, but on failure
  propagate the refetch's *own* `ResolutionError` verbatim (not wrapped) —
  already correctly classified by this same mapping, since a fresh
  integrity failure during the retry maps to `Refusal` on its own merits and
  a fresh network failure maps to `Failed`, without needing
  `CorruptCacheAndRefetchFailed` to carry that distinction.

**Compatibility note**: `From<ResolutionError> for kernel::Error` is shared
launcher code (`cli/launcher/src/launch/core.rs`), used by every
`Command::External` dispatch — not scoped to `vcs`. `DISPATCHED_SUBBINARIES`
today also contains `visualiser`, so this change deliberately changes that
existing dispatch's exit code from 1 (`Failed`) to 2 (`Refusal`) on a
checksum/signature/version-mismatch failure too. This is an intentional,
desirable correction applied uniformly (integrity failures were never
correctly distinguished from availability failures for any dispatched
subcommand), not a `vcs`-only change with an accidental side effect on
`visualiser` — see Migration Notes.

**File**: `cli/launcher/src/main.rs`
**Changes**: in `main()`, when `run(&cli)` returns `Err` and `cli.command` is
`Command::External(raw)`, call `swallow_under_fail_safe(&error, raw)`; if
true, emit `tracing::warn!(%error, "external dispatch failed under \
--fail-safe; exiting 0")` (visible via `ACCELERATOR_LOG`, matching
`bin/accelerator`'s own precedent of never silently discarding this class of
failure) and return `ExitCode::SUCCESS` instead of calling `report()`;
otherwise fall through to `report()` as today, which now also handles the
integrity-failure case correctly since it already maps `Refusal` to exit 2.

#### 2. Skip the cache-root write-probe on a warm cache hit

**File**: `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
**Changes**: split `resolve` into `candidate(config) -> Result<PathBuf,
ResolutionError>` (the override-or-`plugin_root.join("bin")` selection only,
no I/O beyond `ACCELERATOR_PLUGIN_ROOT`/`ACCELERATOR_CACHE_DIR` env reads) and
`verify_writable(dir: &Path) -> Result<(), ResolutionError>` (today's
write-chmod-exec probe, unchanged, renamed from
`probe_writable_and_executable`).

**File**: `cli/launcher/src/main.rs`
**Changes**: `LazyProductionResolver::resolve` calls `cache_root::candidate`
instead of `cache_root::resolve`, so the probe no longer runs ahead of the
cache-hit check.

**File**: `cli/launcher/src/launch/outbound/resolve/mod.rs`
**Changes**: `FetchVerifyCacheResolver::fetch_verify_store` — reached only on
a cache miss or a failed re-verify — calls
`cache_root::verify_writable(&self.config.cache_root)?` as its **first**
statement, before `load_manifest()`'s two HTTP round-trips, the asset fetch,
and `verifier::verify_binary`'s sha256/minisign work, not immediately before
`cache::store`. This preserves today's fail-fast behaviour on the miss path
(an unwritable cache root is a purely local, cheap-to-check condition — it
shouldn't cost a full network round-trip and cryptographic verification to
discover) while still only reaching `verify_writable` via this function,
never via the hit path. A warm cache hit (`cache::find` + `reverify`, both
read-only) never reaches this path and so never pays the probe.

**Rationale**: `cache_root::resolve` currently runs the write-chmod-exec probe
unconditionally, on every external-subcommand dispatch, before
`FetchVerifyCacheResolver` even checks whether a valid cached binary already
exists (Key Discoveries: measured at ~132ms in the repo's `bin/`, against a
3.72ms warm re-exec). The probe only matters when the resolver is about to
*write* a binary; a cache hit only reads. This closes the direct conflict
between that measured cost and Phase 10's `G ≤ 1.1 × B` gate without pulling
in 0189's full scope — 0189 still owns whatever cost remains after this
narrow fix. As a side effect, this also fixes a latent bug: today, a
read-only-but-already-populated cache root fails to resolve at all, even
though the cached binary itself is perfectly usable.

#### 3. `accelerator-vcs` crate scaffold

**File**: `cli/vcs-cli/Cargo.toml` (new)
**Changes**: package `accelerator-vcs`, `[[bin]] name = "accelerator-vcs"`,
`version.workspace = true` plus the other inherited fields, mandatory
`package.description`, depends on `vcs`, `vcs-adapters`, `kernel`, `clap`.

**File**: `cli/vcs-cli/src/main.rs`, `cli/vcs-cli/src/cli.rs` (new)
**Changes**: a clap `Cli`/`Command` tree with a single `Detect { format:
Option<Format>, fail_safe: bool, descriptive: bool }` variant for this phase
(mirrors `config_command`'s inbound/render split at a smaller scale —
`cli.rs` for parsing, `render.rs` for output).

**`ACCELERATOR_VCS_BIN` trust note**: `vcs` is registered via the existing
`_SUBBINARY_MANIFESTS`/`override_path` convention, so `ACCELERATOR_VCS_BIN`
is gated by nothing but the env var's presence — no signature check, no
marker file, no path containment, unlike the launcher's own local-build
override for the launcher binary itself (which requires an opt-in flag, a
marker, and path containment). This is a deliberate, accepted difference, not
an oversight, for two independent reasons: (1) blast radius — the launcher
override compromises the trust boundary for every dispatched subcommand
across every skill if abused, whereas `ACCELERATOR_VCS_BIN` can only redirect
the `vcs` dispatch specifically; (2) **`vcs guard` itself is not a security
boundary** — per its own threat-model note (Phase 7, item 1), it exists to
steer Claude Code toward jj-native commands in a jj repo, not to enforce
access control. Redirecting it via this override changes what suggestion a
user sees, not what a determined actor can do, since the guard was never a
hardened control to begin with. Document this as a local-dev-only convenience
in the crate's module docs: `ACCELERATOR_VCS_BIN` must never be set in a
production/installed-plugin environment.

#### 4. `vcs detect`

**File**: `cli/vcs-cli/src/detect.rs` (new)
**Changes**: mode determination via `dual_roots()`/`jj_workspace_root()`
presence (jj-wins-if-present, robust to `.git`-as-file — the correction falls
out of using the library queries rather than a bespoke fix, see Key
Discoveries); the boundary block reproduces `build_boundary_block`/
`_emit_parent_block`'s exact text (`hooks/vcs-detect.sh:94-130`) for the four
boundary-carrying `Classification` arms from Phase 3.

**Adapter-failure wiring**: two independent sources can fail here — the
direct mode-determination queries, and `vcs::classify::classify()` (called
for the boundary block) — and either's `Err` is the disjoint-output-contract
signal: `Err(kernel::Error)` from *either* renders
`kernel::hooks::adapter_failure` (exactly one `systemMessage` object, exit 0
under `--fail-safe`); this is what makes the "test-only failing probe"
success criterion below concrete rather than an unspecified mechanism. Only
when both succeed does rendering proceed: `Ok(classification)` renders the
boundary block (or nothing, for `Main`/`None` under the default,
non-`--descriptive` mode) as already described.

**Third declared departure**: the shell's `CONTEXT` variable
(`hooks/vcs-detect.sh:40-82`, the "VCS Command Reference" cheat-sheet) is
unconditional in the shell but conditional on the new `--descriptive` flag
here. Without the flag, output is structured-only — the boundary block if a
boundary exists, nothing at all otherwise (satisfying the "success with
nothing to report → zero bytes" contract cleanly for the common case). With
the flag, output additionally carries the reference text, reproducing the
shell verbatim for that mode; the pre-existing `main-jj-workspace.json`/
`main-git-checkout.json` goldens and the new `colocated-git-as-file.json`
fixture (Phase 1, item 4) are `--descriptive` targets. `hooks.json`'s
SessionStart registration (Phase 9) passes `--descriptive` so the
user-visible transcript output is unchanged; no other caller of `vcs detect`
is known to exist. Output wrapped via `kernel::hooks::session_start`.

### Success Criteria

#### Automated Verification

- [ ] `forwarded_fail_safe` unit tests: token present at any position before
      `--` returns true; token after `--` or absent returns false
- [ ] `swallow_under_fail_safe` unit tests, against literal `kernel::Error`
      values, no test doubles needed: `Failed` + forwarded → `true`; `Failed`
      + not forwarded → `false`; `Refusal` + forwarded → `false` (never
      swallowed, regardless of `--fail-safe`); `LogFilter` + forwarded →
      `false` (the allowlist covers only `Failed`, so an unrelated logging
      error is never swallowed either)
- [ ] `From<ResolutionError> for kernel::Error` unit tests: `ChecksumMismatch`,
      `SignatureMismatch`, `ManifestSignature`, `ManifestVersionMismatch`, and
      `CorruptCacheAndRefetchFailed` each map to `Refusal`; every other
      variant maps to `Failed`. Written as an exhaustive match with no
      wildcard arm (not `_ => Self::Failed(..)`), so a future
      `ResolutionError` variant forces a compile-time classification decision
      rather than silently defaulting to `Failed`
- [ ] `FetchVerifyCacheResolver::resolve`'s reverify-failure branch, exercised
      with a stubbed fetcher: a `ChecksumMismatch`/`SignatureMismatch`
      reverify failure followed by a failed refetch produces
      `CorruptCacheAndRefetchFailed` (→ `Refusal`, never swallowed); a plain
      `Cache`-I/O reverify failure followed by a failed refetch propagates
      the refetch's own error verbatim, not wrapped — confirming a benign
      double-failure (I/O hiccup + blocked retry) stays swallowable under
      `--fail-safe` rather than being misclassified as confirmed tampering
- [ ] This launcher-wide change is not `vcs`-scoped: a success criterion
      exercises the existing `accelerator visualiser` external-dispatch path
      (the only other `DISPATCHED_SUBBINARIES` entry today) through the same
      integrity-class mapping, confirming its exit code deliberately changes
      from 1 (`Failed`) to 2 (`Refusal`) on a checksum/signature failure —
      documented as an intentional fix (integrity failures should never have
      been conflated with availability failures for any dispatched
      subcommand), not an accidental side effect
- [ ] A `ResolveBinary`/`ExecBinary` test-double pair (matching the existing
      style in `cli/launcher/src/launch/core.rs`'s test module) confirms a
      failing resolve exits 0 when forwarded and the failure is availability
      class, and exits non-zero (2) when the failure is an integrity-class
      `ResolutionError`, regardless of `--fail-safe`
- [ ] `cache_root::candidate` performs no filesystem write or process spawn —
      asserted by a test pointing it at a directory that would fail the write
      probe (e.g. a non-existent parent with no create permission) and
      confirming it still returns the candidate path rather than an error
- [ ] A read-only cache root directory containing an already-cached,
      correctly-signed binary still resolves successfully end to end (proves
      the probe is skipped on a hit); an empty/unwritable cache root still
      fails with `CacheRootUnavailable` when a fetch is actually attempted
      (proves the probe still guards the write path)
- [ ] The unwritable-cache-root-on-miss case above fails **fast**, not just
      correctly: pointed at a local mock HTTP server (e.g.
      `ACCELERATOR_RELEASE_BASE_URL` set to a `127.0.0.1` listener) whose
      request log is asserted empty, proving `verify_writable` runs before
      `load_manifest`'s network round-trips rather than after them — the
      terminal `CacheRootUnavailable` error alone doesn't distinguish this
      from the pre-reordering behaviour, since both orderings reach the same
      final error
- [ ] `vcs detect --descriptive` output matches all three `--descriptive`
      fixtures from Phase 1/existing `hooks/test-fixtures/vcs-detect/*.json`
      after `jq -S .` canonicalisation, including the new
      colocated-`.git`-as-file case
- [ ] `vcs detect` (no `--descriptive`) output matches the new
      `jj-secondary-structured.json` fixture (Phase 1, item 5) — boundary
      block only, no reference text
- [ ] Success-with-nothing-to-report: in a main checkout with no boundary,
      `accelerator-vcs vcs detect --format=hook --fail-safe` (no
      `--descriptive`) exits 0 and writes zero bytes to stdout
- [ ] Adapter-failure: with a test-only failing probe on either source (the
      direct mode-determination queries, or the narrower `CheckoutProbe` port
      `classify()` uses for the boundary block), the same
      command exits 0 and writes exactly one JSON object containing
      `systemMessage`, independent of `--descriptive`
- [ ] `mise run cli:check` passes; `cargo test -p accelerator-vcs --locked`
      passes

#### Manual Verification

- [ ] `ACCELERATOR_VCS_BIN=$(pwd)/cli/target/debug/accelerator-vcs
      ${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs detect` runs correctly in a
      scratch colocated-worktree checkout

---

## Phase 6: `vcs status` and `vcs log`

### Overview

Adds the two remaining read-only subcommands, each a thin mode check plus a
subprocess exec of the real `jj`/`git` binary (see Key Discoveries for why
this is not library-backed).

### Changes Required

#### 1. Subprocess status/log adapter

**File**: `cli/vcs-adapters/src/subprocess.rs`
**Changes**: `capped_stdout` is refactored into a shared `run_capped(command,
cap, vcs) -> Option<String>` primitive — same spawn/timeout-and-kill/read
machinery and the same `warn!` call at every failure point, but *without*
`revision`'s "empty output is itself a failure" check, since a clean `git
diff --cached --stat` or an empty `jj status` are legitimate, common results
for `status`/`log`, not failures. `revision()` becomes a thin wrapper: call
`run_capped`, then apply its own empty-is-a-failure `warn!` check on top
(preserving its exact existing behaviour and tests).

`status(root: &Path, kind: VcsKind) -> String` and `log(root: &Path, kind:
VcsKind) -> String` are added, each running the four exact shell commands
(`jj status`, `git diff --cached --stat`, `jj log --limit 5`, `git log
--oneline -5`), reusing `run_capped` (the same 10-second cap-and-kill) and
`scrub_environment` (the same `GIT_DIR`/`GIT_CONFIG`/`JJ_CONFIG` scrubbing)
already established for `revision`. On `run_capped` returning `None` (already
`warn!`-logged internally), fall back to the shell's literal `(...
unavailable)` text — matching `2>/dev/null || echo "(... unavailable)"`'s
behaviour, but now diagnosable via `ACCELERATOR_LOG` rather than silently
indistinguishable from a clean, empty repository.

#### 2. CLI wiring

**File**: `cli/vcs-cli/src/cli.rs`
**Changes**: add `Status` and `Log` variants: no `--format` (these are
plain-text-only, matching the shell scripts, which have no hook envelope),
but they **do** take `--fail-safe`. This is not for a hook envelope — it's
for the launcher-level dispatch swallow from Phase 5, item 1, which is
generic to any `Command::External` forwarding the token, not specific to
hook-registered subcommands. Without it, a resolution failure (the binary
not yet cached, an unreachable release host on first use) surfaces as a
non-zero exit through `skills/vcs/commit`'s `!` shell-preprocessor injection
— contradicting the work item's own Dependencies claim that "the skill
degrades its injected context rather than failing when the sub-binary cannot
be fetched." With `--fail-safe`, that specific failure class exits 0 with no
output, so the skill's injected context is silently missing that section
rather than the whole skill invocation erroring. This is a separate concern
from Phase 6, item 1's in-process command failure handling (lock contention,
missing `jj`/`git` binary), which is already covered by `run_capped`'s
fallback to `(... unavailable)` once `accelerator-vcs` itself is running.

**`--fail-safe` layering, stated once**: the flag is a launcher-level
dispatch-resolution signal available to *any* external subcommand
(Phase 5, item 1), optionally also consumed internally by a subcommand's own
handler where an in-process adapter failure needs the same fail-open
treatment (`vcs detect`/`vcs guard` do this, per Phase 5 item 4 and Phase 7
item 2). `vcs status`/`vcs log` use only the first layer — their parsed
`fail_safe: bool` field has no referent inside their own handler; the flag's
only effect is that the token must be present in argv for the *parent*
launcher process to detect and swallow a resolution failure before
`accelerator-vcs` ever runs. This asymmetry (full protocol control on two
subcommands, pass-through-only on the other two) is deliberate, not an
oversight — call it out explicitly rather than leaving a reader to infer it
by analogy with `Detect`/`Guard`.

**File**: `cli/vcs-cli/src/status.rs`, `cli/vcs-cli/src/log.rs` (new)

#### 3. Golden-comparison harness

**File**: `cli/vcs-cli/tests/status_log_goldens.rs` (new)
**Changes**: a Rust integration test that runs the compiled
`accelerator-vcs` binary's `status`/`log` subcommands against each of the
nine captured fixture states, loads `hooks/test-fixtures/masks.toml` (Phase
1, item 1), applies the same named patterns to the live output that the
Python generator applied when producing the committed goldens, and asserts
byte-equality. Living in `cli/vcs-cli/tests/` (not the Python-based hooks
suite) keeps this phase independently mergeable ahead of Phase 9's parity-gate
repoint, which this test does not depend on.

### Success Criteria

#### Automated Verification

- [ ] `vcs status`/`vcs log` match the Phase 1 goldens (masked) across all
      nine captured states
- [ ] A slow/blocking stand-in command (mirroring `capped_stdout`'s own
      existing timeout test style) proves `status`/`log` are bounded by the
      same cap-and-kill as `revision`, falling back to `(... unavailable)`
      rather than hanging
- [ ] Ambient `GIT_CONFIG`/`JJ_CONFIG` pointed at a scratch file with
      attacker-controlled content does not affect `status`/`log` output,
      proving `scrub_environment` reuse
- [ ] `accelerator-vcs vcs status --fail-safe` and `vcs log --fail-safe`
      parse and execute successfully (exit 0, non-empty output against a
      fixture repo) — proves the flag's clap wiring is correct and doesn't
      conflict with anything, since this is the exact form
      `skills/vcs/commit`'s repointed invocation uses (Phase 8) and nothing
      else exercises that flag combination end to end
- [ ] `vcs status --fail-safe` and plain `vcs status` (no flag) produce
      byte-identical output against the same fixture when the command itself
      succeeds — pinning that the flag has no internal effect on a
      successful run, only on the launcher-level dispatch-resolution path
      (per the layering note above), so a future change can't accidentally
      wire it into the handler without a test noticing the behaviour split
- [ ] `mise run cli:check` passes; `cargo test -p accelerator-vcs --locked`
      passes

#### Manual Verification

- [ ] `accelerator-vcs vcs status`/`vcs log` run correctly against a live
      dirty checkout and a live jj secondary workspace

---

## Phase 7: `vcs guard`

### Overview

Adds the PreToolUse guard: compound-command splitting, the 13-subcommand
blocklist, jj-equivalent suggestions, and the two-shape envelope (deny for
pure-jj, bare warn for colocated).

### Changes Required

#### 1. Command-decision domain module

**File**: `cli/vcs/src/guard.rs` (new)
**Changes**: a `GuardDecision` enum (`Allow`, `Block { subcommand: String,
suggestion: String }`) and a pure `decide(command: &str) -> GuardDecision`
function, mirroring Phase 3's `vcs::classify` placement — this logic is pure
data-in/data-out with no I/O dependency, so unlike `classify()` it needs no
port trait, just a plain function. `decide` splits on `&&`/`||`/`;`/`|`, but
**quote-aware** — tracking single- and double-quote state and only treating a
separator token as a real split point when unquoted — rather than
`hooks/vcs-guard.sh:44-70`'s plain `sed`-then-`while read` shape, which is
quote-blind (the fourth declared departure; see Phase 1, item 3). It matches
the 13 blocked git subcommands, allows `gh`/`rtk` unconditionally, and carries
the exact jj-equivalent suggestion table from `hooks/vcs-guard.sh:82-91`
(`status`→`jj status`, `add`→"not needed — jj has no staging area...", etc.).
Placing this in `vcs` rather than `vcs-cli` keeps it under the same
`vcs_domain_imports_only_permitted` protection against creeping I/O that
`classify()` already has, and makes it independently unit-testable and
reusable outside the binary crate.

**Threat model note**: the guard is a steering aid — nudging Claude Code
toward jj-native commands in a jj repo — not a hardened access-control or
sandboxing boundary. Reproduced/inherited from the shell verbatim (parity),
it remains bypassable by shell wrappers, absolute binary paths, aliasing, and
command substitution (`$(...)`/backticks inside a quoted argument, which the
new quote-aware splitter deliberately does not evaluate — it tracks quoting,
not shell expansion). This is an accepted limitation of the heuristic, not a
gap this story is scoped to close.

#### 2. Mode, envelope, and I/O composition

**File**: `cli/vcs-cli/src/guard.rs` (new)
**Changes**: mode is determined **first**, via the same direct
`dual_roots()`/`jj_workspace_root()` presence check `vcs detect` uses
(Phase 5, item 4) — **not** via `vcs::classify::classify()`'s 7-arm
`Classification`. This corrects an inconsistency from an earlier draft of
this phase: `Classification::Main` carries no fields at all (per Phase 3's
definition), so it cannot by itself distinguish a plain git-only main
checkout from a plain jj-only one — exactly the distinction the guard most
needs, since a git-only or non-repo checkout must always allow (matching
`hooks/vcs-guard.sh:22-24`'s early exit on `.jj` absent, before the shell
ever parses the command), while a jj-present checkout needs the
pure-jj-vs-colocated split to choose deny-vs-warn. Reusing the same
mode-determination mechanism as `vcs detect` also avoids duplicating that
logic through two different, potentially-diverging compositions of the same
underlying queries.

Composition: determine mode first. No jj present at this location → `Allow`,
exit 0, no output — `vcs::guard::decide` is never called, matching the
shell's early exit exactly (no command parsing needed for a non-jj
checkout). A query failure here (`Err`) renders `kernel::hooks::adapter_failure`
directly — the concrete mechanism behind the fail-open fault-injection
criterion below (a corrupt repository degrades to "warn, don't block" rather
than either blocking on unreliable information or crashing). Otherwise
(pure-jj or colocated, corrected for `.git`-as-file), calls
`vcs::guard::decide(command)`: `Allow` exits 0 with no output; `Block`
chooses the envelope shape by mode — pure-jj emits
`kernel::hooks::pre_tool_use_deny`; colocated emits
`kernel::hooks::pre_tool_use_warn`.

### Success Criteria

#### Automated Verification

- [ ] `vcs::guard::decide` unit tests run via `cargo test -p vcs --locked`,
      independent of any repo fixture or probe — the compound-splitting and
      blocklist-matching cases from the Phase 1 decision table's command axis
      (34 cases plus the quote-aware-split departure case), decoupled from
      the 4 repo-mode axis that only `vcs-cli`'s composition needs
- [ ] Targeted state-boundary tests for the quote-aware splitter specifically,
      beyond the one decision-table row: an unterminated/mismatched quote
      (a trailing unclosed `"`); an escaped quote character within a quoted
      segment; single-quote-containing-unescaped-double-quote (and the
      reverse); and a separator token immediately adjacent to a quote
      boundary with no surrounding whitespace (`"a"&&b`, not `"a" && b`).
      This is new parsing logic (a declared departure, not a port), and
      quote-state-tracking splitters are exactly the class of code where
      state-transition edge cases hide real bugs that a single happy-path
      row wouldn't catch
- [ ] All 138 rows of the Phase 1 decision table pass end to end through
      `accelerator-vcs` (136 shell-parity rows + the 1 `.git`-as-file
      departure row + the 1 quote-aware-split departure row)
- [ ] `{hookSpecificOutput:{hookEventName:"PreToolUse",
      permissionDecision:"deny", permissionDecisionReason:...}}` for a
      pure-jj block; a bare top-level `{systemMessage:...}` with no
      `permissionDecision` key for colocated warn
- [ ] Fail-open fault injection: a test-only failing mode-determination probe
      simulating a corrupt repository (mirroring the AC's `.git/HEAD`
      truncation) exits 0 and emits exactly one `systemMessage` object, no
      `permissionDecision`
- [ ] Release-host-unreachable and manifest-missing-entry fail-open, exercised
      through the Phase 5 launcher mechanism with a stubbed fetcher/manifest,
      exit 0 with no blocking envelope
- [ ] `mise run cli:check` passes; `cargo test -p accelerator-vcs --locked`
      passes

#### Manual Verification

- [ ] A real `git status` Bash call in a scratch pure-jj repo, dispatched
      through `${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs guard`, denies
      correctly end to end

---

## Phase 8: Sub-Binary Registration and Skill Repoint

### Overview

Registers `accelerator-vcs` per the 0187 thirteen-point checklist and
repoints `skills/vcs/commit`, landed together per the checklist's "points 1
and 7 in the same change" rule.

### Changes Required

#### 1. Registration

**File**: `tasks/shared/paths.py`
**Changes**: `DISPATCHED_SUBBINARIES` gains `"vcs"`.

**File**: `tasks/manifest.py`
**Changes**: `_SUBBINARY_MANIFESTS["vcs"] = CLI_DIR / "vcs-cli/Cargo.toml"`.

**File**: `cli/Cargo.toml`
**Changes**: `[workspace].members` gains `"vcs-cli"`; regenerate
`cli/Cargo.lock`.

**File**: `.gitignore`
**Changes**: add `bin/vcs-*`.

**File**: `tasks/build.py`
**Changes**: `_CLI_RELEASE_BINARIES` gains `"accelerator-vcs"`.

#### 2. Skill repoint

**File**: `skills/vcs/commit/SKILL.md`
**Changes**: drop `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)`; add
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs *)`; repoint body lines 13-14
from `!`${CLAUDE_PLUGIN_ROOT}/scripts/vcs-status.sh`` /
`!`${CLAUDE_PLUGIN_ROOT}/scripts/vcs-log.sh`` to
`!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs status --fail-safe`` /
`!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs log --fail-safe``, per Phase 6,
item 2's fail-safe rationale.

### Success Criteria

#### Automated Verification

- [ ] `mise run lint:dispatch-coherence:check` passes — `vcs` is bound by
      `skills/vcs/commit`'s narrowed rule, not by an ancestor glob
- [ ] `tests/unit/tasks/test_build.py`'s dispatch-coherence pass/fail suite
      still passes with `vcs` added to the real registry
- [ ] `EXPECTED_INJECTION_SKILLS` in `tasks/lint/skill_permissions.py` stays
      at 42 (this replaces two sites within one already-counted skill)
- [ ] `mise run cli:check`, `mise run build-system:check` pass
- [ ] `mise run deny:check` passes (no new dependency)

#### Manual Verification

- [ ] `accelerator vcs status`/`vcs log` exit 0 with non-empty output in a
      fixture repo, run through the repointed skill's invocation form

---

## Phase 9: `hooks.json` Rewrite and Shell Deletion

### Overview

Replaces the shell hook registrations with the three verbatim command
strings, repoints the 42-case parity gate per the confirmed partition, and
deletes the five retired shell files.

**Deployment note**: per Sequencing Constraint 4, this phase's `hooks.json`
change should not reach an installed-plugin path ahead of a release whose
published manifest lists `accelerator-vcs`. **Revised in light of Phase 5's
fail-safe fix**: a missing `accelerator-vcs` manifest entry maps to
`ResolutionError::AssetNotFound`, a `Failed`-class error, not one of the
integrity-class variants Phase 5 routes to `Refusal` — so this case is
already covered by the fail-safe swallow. A premature merge therefore
degrades gracefully (SessionStart shows nothing, the guard silently doesn't
block) rather than hard-failing every installed plugin, which is a narrower
risk than a bare reading of the anti-rollback rule suggests. The ordering
constraint still stands — a premature merge leaves the guard silently
non-protective for anyone on that release until the real release catches up,
which is worth avoiding — but given the fail-open mitigation, this plan
relies on the existing prose note plus Phase 10's manual scheduling item
rather than adding new release-pipeline automation. The code in this phase
can be written, tested, and merged in dev mode
(`ACCELERATOR_VCS_BIN`-overridden) without that release; only the actual
rollout is gated, and that gate is a Phase 10 acceptance item, not a
Phase 9 code dependency.

### Changes Required

#### 1. `hooks.json`

**File**: `hooks/hooks.json`
**Changes**: SessionStart loses the `vcs-detect.sh` and `config-detect.sh`
entries, replaced by the two verbatim strings:
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs detect --format=hook --fail-safe
--descriptive` and `${CLAUDE_PLUGIN_ROOT}/bin/accelerator config summary
--format=hook --fail-safe`. The `--descriptive` flag preserves today's
user-visible transcript content (the reference cheat-sheet), matching the
Phase 5 departure. `migrate-discoverability.sh` and `launcher-link-refresh.sh`
entries are untouched. PreToolUse's `vcs-guard.sh` entry becomes
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs guard --format=hook --fail-safe`.

#### 2. Parity gate repoint

**File**: `scripts/test-vcs-common.sh` (new)
**Changes**: the 26 in-process cases from `hooks/test-vcs-detect.sh`, moved
verbatim (they exercise the surviving `scripts/vcs-common.sh`, unaffected by
this story).

**File**: `hooks/test-vcs-detect.sh`
**Changes**: the 8 subprocess cases repoint their `HOOK` constant to invoke
the built `accelerator-vcs` binary via an `accelerator_env()`-style overlay
instead of `bash "$HOOK"`; the 5 missing-binary cases are deleted; the
`hooks.json` literal assertion (singleton 1) is updated to the new command
strings and rewritten order-independent (no `SessionStart[0]` indexing); the
top-of-file comment-block grep (singleton 2) is deleted (no equivalent
comment exists in the Rust source); the golden-snapshot host-artefact check
(the previously-uncounted case) is preserved, now asserting the Rust-produced
goldens are equally free of host-specific path leakage.

#### 3. Task wiring

**File**: `tasks/test/integration.py`
**Changes**: `test:integration:hooks` gains a `build:cli:dev` dependency and
uses `accelerator_env()` (extended with an `ACCELERATOR_VCS_BIN` entry) for
the repointed suite.

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: line 61's `"test:integration:hooks": "shell suites run with no
accelerator_env"` pin is removed.

#### 4. Deletion

**Files removed**: `hooks/vcs-detect.sh`, `hooks/vcs-guard.sh`,
`hooks/config-detect.sh`, `scripts/vcs-status.sh`, `scripts/vcs-log.sh`.

### Success Criteria

#### Automated Verification

- [ ] `mise run test:integration:hooks` passes, floor unchanged at 2 suites
      (`tasks/test/integration.py:72`, `_EXPECTED_HOOKS_SUITES`)
- [ ] The `hooks.json` golden set covers every emitted shape: SessionStart
      with `systemMessage`, SessionStart without, plain `vcs detect`
      (structured and `--descriptive`), plain `vcs guard`, PreToolUse deny,
      PreToolUse warn-only
- [ ] `git grep -n "vcs-detect.sh\|vcs-guard.sh\|config-detect.sh\|vcs-status.sh\|vcs-log.sh"`
      returns nothing outside this diff's own history
- [ ] `mise run check` passes end to end

#### Manual Verification

- [ ] A real Claude Code session against this branch shows the SessionStart
      VCS context and, on a blocked git call in a pure-jj scratch repo, the
      PreToolUse denial — both via the new subcommands

---

## Phase 10: Hand-offs, Documentation, and Validation

### Overview

Closes out the story's remaining acceptance criteria: dated hand-off notes,
two follow-up work items, and the two manual validation measurements.

### Changes Required

#### 1. Hand-off notes

**Files**: `meta/work/0172-*.md`, `meta/work/0183-*.md`, `meta/work/0125-*.md`,
`meta/work/0189-*.md`
**Changes**: append a dated note to each Dependencies section per the work
item's own wording (0172: proposed `blocked_by: work-item:0169` plus
`hooks/migrate-discoverability.sh` in its source list; 0183: `accelerator vcs
detect` as a new SessionStart audit site; 0125: the in-process adapter
dissolves the lexical-fallback rationale; 0189: its cache-root write-probe
finding is now partly resolved by this story's Phase 5, item 2 — 0189's scope
should be re-measured against the post-fix dispatch cost rather than the
pre-fix ~132ms figure).

#### 2. Follow-up work items

**Changes**: create two new work items via `create-work-item` — one owning
`scripts/vcs-common.sh`'s residue and `hooks/launcher-link-refresh.sh`; one
owning the decision on removing `log`/`diff` from the guard's blocked set.

#### 3. Documentation sweep

**Changes**: `git grep` for the five removed filenames across `README.md`,
`docs-site/`, `tasks/README.md` (a prior sweep found no hits — re-verify
directly rather than trusting that as final) and update any prose reference
found.

### Success Criteria

#### Automated Verification

- [ ] `mise run` passes end to end
- [ ] The dated hand-off notes and both new work items exist and are
      cross-linked (grep-verified)

#### Manual Verification

- [ ] **Claude Code floor check**: on the actual client version in use, a
      real Bash call to a blocked git subcommand in a pure-jj repo is denied
      and the colocated warning appears in the session transcript; observed
      version recorded in the work item's Validation Results
- [ ] **Warm-call latency**: median of 20 `hooks/vcs-guard.sh` invocations
      (B) vs 20 warm `accelerator vcs guard` invocations (G) against the same
      stdin payload and pure-jj fixture on one host; `G ≤ 1.1 × B` recorded
      with the payload, fixture, host, and ratio in Validation Results. **G is
      measured through the real bootstrap-to-launcher-to-sub-binary dispatch
      path** (no `ACCELERATOR_VCS_BIN` override), so the recorded figure
      reflects what an installed plugin actually pays, including the Phase 5
      cache-root fix's effect
- [ ] The release-cut deployment gate (Phase 9's note) is scheduled with
      whoever performs epic-0136 releases before `hooks.json`'s rewrite
      reaches an installed-plugin path

---

## Testing Strategy

### Unit Tests

- `forwarded_fail_safe` token-scanning edge cases (Phase 5)
- `cache_root::candidate` probe-skipping and `verify_writable`
  write-path-only-guard tests (Phase 5)
- `kernel::hooks::envelope` shape-pinning tests (Phase 4)
- `classify()` against the full fixture matrix, including the ambiguous
  nested-vs-colocated case (Phase 3)
- Guard command-splitting and subcommand-matching, independent of any repo
  fixture (Phase 7)

### Integration Tests

- The Phase 1 goldens for `vcs status`/`vcs log`/`vcs detect`, exercised
  against the compiled `accelerator-vcs` binary (Phases 5-6)
- The 138-row guard decision table (Phase 7)
- The repointed 42-case parity gate, now dispatching through the compiled
  binary (Phase 9)
- Fail-open fault injection at both the adapter level (corrupt repository)
  and the launcher-dispatch level (unreachable host, missing manifest entry)

### Manual Testing Steps

1. Run a live session against a scratch pure-jj repo and confirm a blocked
   `git status` call is denied with the jj-equivalent suggestion.
2. Run a live session against a scratch colocated repo (including one with
   `.git` as a worktree file) and confirm the warn-not-block behaviour.
3. Measure warm-call latency per Phase 10's criterion on the actual
   development host.

## Performance Considerations

The `G ≤ 1.1 × B` warm-call latency gate (Phase 10) is host-relative by
design; no code in this plan targets a fixed millisecond figure. The
launcher-side cache-root probe (Key Discoveries: ~132ms per external
dispatch, unconditionally, ahead of the cache-hit check) is closed narrowly
within this story (Phase 5, item 2) rather than deferred wholesale to 0189 —
this story's own gate cannot be meaningfully measured while every warm `vcs
guard` dispatch pays that cost. 0189 retains whatever launcher-dispatch cost
remains after this fix (if any); this section does not restate 0189's
figures, since they predate the fix and would no longer be representative
once Phase 5 lands.

## Migration Notes

Existing installed plugins keep working against the old shell hooks until a
release publishing `accelerator-vcs` lands and `hooks.json`'s rewrite ships
(Phase 9's deployment note). No data migration is involved — this is a
behaviour-preserving port with four declared, tested departures.

**Cross-cutting exit-code change**: Phase 5's `ResolutionError`→`kernel::Error`
integrity/availability split is shared launcher code, so it also changes
`accelerator visualiser`'s exit code from 1 to 2 on a checksum/signature/
version-mismatch dispatch failure — the only other consumer of external
dispatch today. This is intentional (see Phase 5's Compatibility note), not a
`vcs`-scoped side effect; anything that happened to depend on `visualiser`'s
specific exit code on that failure class should be re-checked.

## References

- Work item: `meta/work/0169-vcs-subdomain-and-hooks-migration.md`
- Research: `meta/research/codebase/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md`
- Prior research: `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
- 0187 checklist: `tasks/README.md:304-456`
- 0188 plan: `meta/plans/` (library-backed VCS adapter)
- ADR-0048 (hook logic in the CLI), ADR-0053 (thin CLI over a hexagonal core)
