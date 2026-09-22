---
type: "plan"
id: "2026-09-20-0286-eradicate-direct-git-calls-from-skills"
title: "Eradicate Direct Git Calls From Skills Implementation Plan"
date: "2026-09-20T22:38:07+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0286"
parent: "work-item:0286"
derived_from: ["codebase-research:2026-09-20-0286-eradicate-direct-git-calls-from-skills"]
tags: ["vcs", "skills", "cli"]
revision: "df17c8e694a640b57159346e09ffdaeb48b2cd00"
repository: "accelerator"
last_updated: "2026-09-22T08:37:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Eradicate Direct Git Calls From Skills Implementation Plan

## Overview

Make every Accelerator skill VCS-agnostic so it behaves identically under git,
colocated jj, and pure-jj. One skill (`validate-plan`) breaks in a pure-jj
checkout because it issues raw `git log`, `git diff`, and `git rev-parse`; two
others (`config/migrate`, `refine-work-item`) name a backend where they should
defer to the session VCS. The remedy is to stop issuing raw `git` from the
skills, expose the existing backend-neutral working-copy-root capability as an
`accelerator vcs root` command, and extend the SessionStart VCS Command
Reference with a diff-range idiom and a user-identity idiom so the skills share
one documented vocabulary. The skills reference these idioms by name and defer to
the reference — they do not restate backend commands in their bodies — so the
reference is the single source of truth (the pattern `research-issue` already
follows). `refine-work-item`'s partial-write recovery prose is neutralised to
plain file deletion ("delete the newly-written child file(s)"), not a VCS command,
since all three of its abort sites recover newly-written child files. A
`tasks/lint` git-token sweep and a
SKILL↔CLI reference check then make the "no direct git token in `skills/`"
invariant and the skill→CLI dependency durable rather than manually checked.

## Current State Analysis

Every VCS token in the affected skills is prose or an illustrative fenced block
— none is `!`-executed (only inline `` !`accelerator …` `` forms run at
invocation), and no skill's `allowed-tools` grants `git` or `jj`. So the skill
rewrites are wording changes; the only compiled changes are one new CLI
subcommand and two reference bullets.

The current `git <subcommand>` footprint in `SKILL.md` bodies (verified by
sweep at revision `df17c8e6`):

| Skill | Line | Current text | Nature |
|---|---|---|---|
| `validate-plan` | 58 | `git log --oneline -n 20` | fenced bash (model-facing) |
| `validate-plan` | 59 | `git diff HEAD~N..HEAD` | fenced bash (model-facing) |
| `validate-plan` | 62 | `cd $(git rev-parse --show-toplevel) && make check test` | fenced bash (model-facing) |
| `validate-plan` | 48, 247 | "through git", "analyze the git history" | soft prose |
| `config/migrate` | 272-273 | `` `jj status`/`git status` `` (wrapped across two lines) | soft prose |
| `refine-work-item` | 171, 187, 214 | `jj restore <path>` (discard prose) | soft prose |
| `refine-work-item` | 247-249 | `jj config get user.name` → `git config user.name` chain | soft prose |

All line numbers are as-of revision `df17c8e6`; earlier phases' within-file
insertions shift later references, so each phase below anchors its edits on the
target text (a quoted phrase or a whole-sentence reword), not a bare line
number.

Only `validate-plan` truly breaks: in a pure-jj checkout there is no `.git`, so
`git rev-parse`, `git log`, and `git diff` fail with `fatal: not a git
repository`. `config/migrate` and `refine-work-item` name `git` but never break
(migrate's line is user-facing prose; refine-work-item's `git config` reads
global config, which works without a `.git`).

### Key Discoveries:

- `accelerator vcs` is already a dispatched token
  (`tasks/shared/paths.py:31`), so `accelerator vcs root` is a plain
  second-level clap subcommand inside `cli/vcs-cli/`, **not** the thirteen-point
  dispatched-sub-binary checklist. This corrects the work item's Technical Note.
- The command surface is clap derive at `cli/vcs-cli/src/cli.rs:16` (a `Command`
  enum: `Detect`, `Status`, `Log`, `Guard`), dispatched at
  `cli/vcs-cli/src/main.rs:98`. `Status`/`Log` carry a launcher-consumed,
  handler-ignored `--fail-safe` — the CLI shape `Root` mirrors.
- A path-printing handler mirrors `detect.rs` (direct probe ports), **not**
  `status.rs`/`log.rs` (which route through the never-fail `report::run`
  boundary only because they need a `VcsReporter`).
- `RepoRoot::discover` (`cli/vcs-adapters/src/library.rs:512`) returns the
  working-copy root containing `start` — the faithful analog of `git rev-parse
  --show-toplevel` and `jj workspace root`. It is already imported and used by
  `report.rs:56`. `repository_root` (line 516) instead maps a jj secondary
  workspace up to its shared main repo (pinned by
  `cli/vcs-adapters/tests/library.rs:264`), which is the wrong tree for
  `validate-plan` to run checks in. The command surfaces `discover`.
- The SessionStart reference lives in two `concat!` consts in
  `cli/vcs-cli/src/detect.rs:11-59` (`JJ_COMMAND_REFERENCE`, `GIT_REFERENCE`),
  selected by `reference_text(mode)` (line 61). The `--descriptive` hook flag
  (`hooks/hooks.json:9`) prepends it. No `hooks.json` change is needed — the
  extended reference flows through the existing wiring automatically.
- The user-identity probe already exists: `InProcessProbe::user_name`
  (`library.rs:545`) over `vcs::UserIdentityProbe` (`vcs/src/lib.rs:111`). The
  identity idiom is a text edit; the diff-range idiom is net-new model-facing
  text with no probe.
- `validate-plan`'s `allowed-tools` (lines 7-11) lists three scoped entries
  (`Bash(accelerator config *)`, `Bash(accelerator corpus metadata derive)`,
  `Bash(accelerator corpus frontmatter validate *)`), **not** a blanket
  `Bash(accelerator ...)`. It does not cover `accelerator vcs root`, so the
  rewrite must add `Bash(accelerator vcs root)`. This corrects a second work
  item Technical Note.
- `refine-work-item`'s eval fixtures pin the `jj restore` wording:
  `evals/evals.json` asserts `"value": "jj restore"` (lines 146-148, 843-845)
  and `evals/benchmark.json`/`.md` record it as passing evidence (lines 76,
  386, 411). Neutralising the prose forces matching fixture updates. Evals are
  **not** wired into any `mise` task, so this does not gate `mise run` green.
- Adding reference bullets regenerates three end-to-end goldens in
  `cli/vcs-test-support/fixtures/vcs-detect/` (they embed the full reference
  text). The `render`-level unit tests (`detect.rs:217-413`) assert
  `.contains(...)` and tolerate additions.

## Desired End State

- `accelerator vcs root` prints the working-copy root of the checkout it runs in
  and exits 0 under git-only, colocated jj, pure-jj, **and a jj secondary
  workspace** (returning the workspace, not the shared main repo). Its `--help`
  names the working-copy root and the secondary-workspace behaviour, never a
  "repository-root lookup". Out of any repository it errors non-zero with a
  message naming the directory it searched from.
- The SessionStart VCS Command Reference carries two idioms in both the git and
  jj reference blocks:
  - a **diff-range idiom** for the cumulative change since the trunk divergence
    point — git `git diff <trunk>...HEAD` where `<trunk>` is the repository's
    default branch (resolved via `git symbolic-ref --short refs/remotes/origin/HEAD`);
    jj `jj diff --from 'fork_point(trunk() | @)' --to @`. Both forms compute the
    change from the working-copy / trunk merge base; the reference notes that the
    jj `@` endpoint includes tracked working-copy edits that the git committed-tree
    form omits, and that both anchors need an origin trunk (jj's `trunk()` falls
    back to `root()`, the whole history, without one; the git symref fails if
    `origin/HEAD` is unset).
  - a **user-identity idiom** — git `git config user.name`; jj `jj config get
    user.name`, with an explicit model-driven fallback to `git config user.name`
    when the jj identity is unset, so colocated repositories with only a git
    identity keep resolving.
- `validate-plan` gathers evidence via the session's VCS log/diff idioms (named,
  deferring to the reference — no backend commands inlined in the skill) and
  `accelerator vcs root`, with no raw `git log`/`git diff`/`git rev-parse` and
  no soft "git" prose; its `allowed-tools` permits `accelerator vcs root`.
- `config/migrate` names the session's VCS status command with no literal
  `git status`.
- `refine-work-item` resolves child author via the reference's user-identity
  idiom (preserving git-only **and** colocated resolution through the reference's
  fallback clause) and states partial-write recovery as plain file deletion
  ("delete the newly-written child file(s)"), with no `jj config`/`git config`/
  `jj restore`/`git restore` token in the skill body; its eval spec matches.
- `research-issue` is unchanged and verified to gather evidence in pure-jj.
- A registered `tasks/lint` git-token sweep over `skills/**/SKILL.md` and a
  SKILL↔CLI reference check (every `accelerator vcs <sub>` referenced in a skill
  resolves to a real subcommand, pinned against the clap `Command` enum; the idiom
  anchors render-locked in `detect.rs`) both pass and are wired into the aggregate
  `check`, so the invariant and the skill→CLI dependency are enforced by CI.
- Work item 0286's Requirement, Acceptance Criterion 2, and the two corrected
  Technical Notes are updated to match the `discover` decision and the scoped
  allowed-tools reality.
- `mise run` (bare default) exits 0.

## What We're NOT Doing

- Not changing the guard or its blocklist (`cli/vcs/src/guard.rs`,
  `cli/vcs-cli/src/guard.rs`) — spike 0200 decided keep-both; the guard is a
  backstop, not the fix.
- Not adding a broad `accelerator vcs diff` subcommand — 0200 rejected it; the
  diff stays model-driven, backed by the reference idiom, with a detection note
  (Phase 3) so the acknowledged pivot to a deterministic diff command is
  triggerable if steering proves unreliable. Adding the git-token lint and the
  SKILL↔CLI reference check (Phase 6) *is* newly in scope — they enforce the
  invariant this item exists to establish, and neither is the rejected diff
  command.
- Not surfacing `repository_root` — `vcs root` surfaces `discover` (per the
  design decision below). `repository_root` keeps its sole caller,
  `vcs::facts`.
- Not changing `validate-plan`'s `make check test` command itself — it is a
  generic placeholder for a consumer repo's checks; only the root discovery is
  swapped.
- Not touching `research-issue`, agents, templates, or `hooks/hooks.json` —
  all clean; `research-issue` is verify-only.
- Not neutralising `skills/work/update-work-item/SKILL.md:269`'s `jj restore` —
  it is not one of the three affected skills.
- Not altering the migration framework, only the one dirty-path prose line.

## Implementation Approach

The two load-bearing capabilities (the `vcs root` command and the reference
extension) land first, so the skill rewrites that depend on them have a
foundation. Each phase is a single-concern diff, but they are **not all freely
reorderable**: `validate-plan` (Phase 3) references `accelerator vcs root`
(Phase 1) and the reference idioms (Phase 2), and `refine-work-item` (Phase 5)
references Phase 2's identity idiom. These are runtime
dependencies — a skill that names a command or idiom that does not yet exist
breaks at invocation, and neither `mise run check` nor the git-token sweep
executes the skill, so an out-of-order merge would pass CI while breaking the
skill. Two things guard this:

- **Hard prerequisite**: Phases 1 and 2 must merge before, or with, Phase 3;
  Phase 2 before, or with, Phase 5. `config/migrate` (Phase 4) leans only on the
  pre-existing status idiom and is mergeable at any time.
- **Enforcement**: the SKILL↔CLI reference check (Phase 6) fails when a skill
  names an `accelerator vcs <sub>` with no matching subcommand (a genuine two-way
  binding, so a skill can never reference a command that does not yet exist); a
  companion render-lock fails if an idiom anchor is deleted from `detect.rs`
  (guarding the reference content, not a dynamic skill→idiom binding — skills name
  idioms in free prose). The subcommand binding is what makes the prerequisite a
  CI failure rather than a discipline. Until Phase 6 lands it is a documented
  ordering constraint. (Phase 6 depends on Phases 1-5 being present, so it lands
  last — treat "Phase 6 merged" as an explicit acceptance-gate line item, since it
  is the only thing converting the ordering from discipline to CI.)

Work follows red-green-refactor. Each compiled phase (1, 2, 6) writes its failing
test first — the step order below is written test-first for exactly this reason.
For the skill rewrites the git-token sweep, the SKILL↔CLI check, and the eval
spec are the executable checks.

**Reference-pointer convention** (applied uniformly across the rewritten skills):
name the idiom and append "(see the SessionStart VCS Command Reference)" wherever
a *newer or less-obvious* idiom is used — the diff-range and user-identity idioms.
`research-issue`'s bare "the session's VCS log/diff command" phrasing is
deliberately left as-is (log/diff are self-evident and need no pointer), so the
two shapes coexist by design, not by accident.

### Design decision — `vcs root` surfaces `discover`, not `repository_root`

`discover` and `repository_root` agree for every plain checkout and git
worktree; they diverge only for a jj secondary workspace, where
`repository_root` returns the shared main repo. `validate-plan` runs `make check
test` from this root, so surfacing `repository_root` would make it `cd` out of a
secondary workspace into the main repo — building a different tree and crossing
the workspace boundary the tooling otherwise enforces. `discover` returns the
checkout the command runs in, faithfully porting `git rev-parse --show-toplevel`
/ `jj workspace root`. A dedicated secondary-workspace test pins this.

This narrows the work item's Requirement and Acceptance Criterion 2, which name
`repository_root`; the intent ("the checkout's repository root") is preserved,
the capability chosen matches `validate-plan`'s actual use. Because the work item
is the source of truth for the acceptance gate, the narrowing is written back to
it — see "Work Item Reconciliation" below — rather than living only in this plan.

The command keeps the name `vcs root` (renaming would ripple through the plan,
the work item, and `allowed-tools` for little gain), but because it surfaces
`discover` — the *working-copy* root — and deliberately differs from the trait's
`repository_root` in a jj secondary workspace, its clap help and module doc must
describe the working-copy root and that secondary-workspace behaviour explicitly.
They must not call it a "repository-root lookup" or claim it "resolves
identically" across topologies, which would invite a future maintainer to
re-point it at `repository_root` and silently reintroduce the divergence this
decision exists to prevent.

---

## Phase 1: `accelerator vcs root` command

### Overview

Add a `vcs root` subcommand that prints the working-copy root of the checkout it
runs in, over `RepoRoot::discover`. Confined to `cli/vcs-cli/`; no launcher,
manifest, `tasks/`, or public-API change (`accelerator-vcs` is
cargo-public-api-exempt).

### Changes Required (test-first):

#### 1. Failing handler unit test

**File**: `cli/vcs-cli/src/root.rs` (new, test module)
**Changes**: Write the unit test first (red — `root::run` does not yet exist).
Over a stub `RepoRoot` (mirroring the crate's existing `FixedRoot`-style stub),
assert `Some(path)` yields that path and `None` errors with a message naming the
searched-from directory.

Note the limit of this test: `vcs::RepoRoot::repository_root` has a default
identity implementation, so a stub that only overrides `discover` returns the
same value for both methods — this test cannot tell whether the handler called
`discover` or `repository_root`. That discrimination is the job of the
secondary-workspace parity test (Step 5), not this one.

#### 2. The handler

**File**: `cli/vcs-cli/src/root.rs`
**Changes**: Resolve the working-copy root via `RepoRoot::discover` (green);
error when `start` is outside any repository, matching `git rev-parse
--show-toplevel`'s own out-of-repo failure, and name the searched-from path so
the message orients a reader run from an unexpected directory.

```rust
//! `vcs root`: the working-copy root of the checkout, resolved in-process
//! for git and jj via `RepoRoot::discover`. In a jj secondary workspace
//! this is the workspace root, not the shared main repository.

use std::path::Path;

use vcs::RepoRoot;

/// # Errors
///
/// When `start` is not inside a repository.
pub fn run<P>(start: &Path, probe: &P) -> Result<String, kernel::Error>
where
    P: RepoRoot,
{
    probe
        .discover(start)
        .map(|root| root.display().to_string())
        .ok_or_else(|| {
            kernel::Error::Failed(format!(
                "not inside a repository (searched from {})",
                start.display()
            ))
        })
}
```

#### 3. CLI variant

**File**: `cli/vcs-cli/src/cli.rs`
**Changes**: Add a `Root` variant. The doc-comment is the `--help` text, so it
must describe the working-copy root and the secondary-workspace behaviour —
never a "repository-root lookup that resolves identically", which contradicts
the `discover` decision. Omit `--fail-safe`: unlike `Status`/`Log` it has no
caller (validate-plan runs `accelerator vcs root` bare, and no hook wires it),
and forwarding it would be actively wrong — the launcher's
`swallow_under_fail_safe` (`cli/launcher/src/launch/core.rs:236`) would degrade a
dispatch-resolution failure to exit-0-empty-output, defeating validate-plan's
`root="$(…)" && cd` abort chain. A unit variant is the honest surface.

```rust
    /// The working-copy root of the checkout containing the current
    /// directory — the root `git rev-parse --show-toplevel` and
    /// `jj workspace root` return. In a jj secondary workspace this is the
    /// workspace root, not the shared main repository.
    Root,
```

#### 4. Dispatch and handler wiring

**File**: `cli/vcs-cli/src/main.rs`
**Changes**: Register `mod root;`, add a `run_root()` thunk, add the dispatch
arm, and extend the crate doc-comment (line 1) enumerating the subcommands.

```rust
fn run_root() -> Result<(), kernel::Error> {
    let probe = InProcessProbe;
    println!("{}", root::run(&current_dir()?, &probe)?);
    Ok(())
}
```

```rust
        Command::Root => run_root(),
```

#### 5. Secondary-workspace parity test

**File**: `cli/vcs-cli/tests/root_goldens.rs` (new)
**Changes**: Open the file with `#![cfg(feature = "bash-parity")]` — every file
in `cli/vcs-cli/tests/` carries this gate, so without it the test's real jj/git
topology setup (including `jj workspace add`) would run in the default
`cli:check`/test lane instead of the gated bash-parity lane. This is a computed-
expectation parity test, not a committed golden: it builds each checkout and
asserts the printed root equals that checkout's own canonical root.

Cover all four topologies (git-only, colocated, pure-jj, jj secondary
workspace), mirroring the `jj workspace add` setup in `detect_goldens.rs:129-150`
and its `/private`-symlink canonicalisation at `detect_goldens.rs:141-142`. The
secondary-workspace case is the discriminating assertion for the
`discover`-not-`repository_root` decision and must assert `printed ==
secondary_canonical` **and not** the main-repo path — a mere non-empty or
plain-checkout assertion (where `discover == repository_root`) would let a
regression to `repository_root` pass unseen.

### Success Criteria:

#### Automated Verification:

- [ ] Handler unit tests pass (stub `RepoRoot`: `Some(path)` prints the path,
      `None` errors naming the searched-from path): `cd cli && cargo test -p
      accelerator-vcs root::`
- [ ] The parity test passes under all four topologies, the secondary-workspace
      case asserting the printed root equals the workspace's own canonical root
      and differs from the main repo: `cd cli && cargo test -p accelerator-vcs
      --features bash-parity --test root_goldens`
- [ ] Single-component check passes: `mise run cli:check`

#### Manual Verification:

- [ ] `accelerator vcs root` run from this build-system secondary workspace
      prints the workspace path, not `.../accelerator`.
- [ ] `accelerator vcs root` run from a plain git clone prints the clone root.
- [ ] `accelerator vcs root` run outside any repository exits non-zero and names
      the searched-from directory.

---

## Phase 2: SessionStart reference — diff-range and identity idioms

### Overview

Add two `- Use …` bullets to each of `JJ_COMMAND_REFERENCE` and `GIT_REFERENCE`
in `detect.rs` — a diff-range idiom and a user-identity idiom — extend the
render-level unit tests first (red), then hand-edit the three descriptive goldens
that embed the reference text. (No discard idiom: `refine-work-item`'s recovery
sites are file deletion, not a VCS revert, so no skill consumes a discard idiom —
see Phase 5.)

Two semantic points the bullets must carry, because the model acts on them
verbatim:

- **The two diff-range forms are not byte-identical.** `git diff <trunk>...HEAD`
  compares committed trees (the merge-base of trunk and HEAD, to HEAD), whereas
  `jj diff --to @` ends at the working-copy commit, which includes tracked-but-
  uncommitted edits. Both express "the cumulative change since the trunk
  divergence point"; the jj form additionally includes any uncommitted working-
  copy changes. The bullet states this so the difference is expected, not a bug.
- **The git anchor is trunk-name-agnostic.** `<trunk>` is a placeholder for the
  repository's default branch, not a literal `main` — Accelerator runs in
  arbitrary consumer repos, and jj's `trunk()` already resolves the trunk
  dynamically, so the git side must too.

### Changes Required (test-first):

#### 1. Render-level unit tests

**File**: `cli/vcs-cli/src/detect.rs` (test module)
**Changes**: Extend the existing `.contains(...)` assertions first (red, before
the const change). Assert both the commands and the load-bearing semantic
clauses the model acts on verbatim, so a reword that keeps the command but drops
the meaning is caught in the fast unit loop, not only the goldens:

- jj reference carries `fork_point(trunk() | @)`, the `root()`-fallback caveat,
  and the `git config user.name` fallback clause.
- git reference carries `<trunk>...HEAD`, the default-branch resolution hint, and
  `git config user.name`.

Because each render is mode-specific, these also catch a bullet placed in the
wrong block.

#### 2. The two reference consts

**File**: `cli/vcs-cli/src/detect.rs`
**Changes**: Append the two bullets to each command list, keeping the `concat!`
`\n` terminators, backslash-continuation style, and 80-column floor. (These
bullets carry backend commands deliberately — the reference *is* the documented
vocabulary, and the git-token sweep in Phase 6 is scoped to `skills/`, not
`detect.rs`. The new bullets read "Use `jj X` for/to …" rather than the block's
"instead of `git Y`" pattern, which is deliberate: these idioms replace no single
git command.)

```diff
     "- Use `jj bookmark list` or `jj status` instead of `git branch \
      --show-current`\n",
+    "- Use `jj diff --from 'fork_point(trunk() | @)' --to @` for the \
+     cumulative change since the trunk divergence point; `--to @` includes \
+     uncommitted working-copy edits. Needs an origin trunk bookmark; \
+     `trunk()` falls back to `root()` (whole history) without one\n",
+    "- Use `jj config get user.name` to read the configured user identity, \
+     falling back to `git config user.name` when the jj identity is unset\n",
     "\n",
     "Key conceptual differences from git:\n",
```

```diff
     "- Use `git push` to push to remote\n",
+    "- Use `git diff <trunk>...HEAD` for the cumulative change since the \
+     trunk divergence point (three-dot = merge-base to HEAD; committed \
+     trees only), where <trunk> is the default branch, resolved via \
+     `git symbolic-ref --short refs/remotes/origin/HEAD`\n",
+    "- Use `git config user.name` to read the configured user identity\n",
     "\n",
     "Key conventions:\n",
```

The jj diff-range bullet resolves the merge base of both `@` and `trunk`
(`fork_point(trunk() | @)`, not the single-revision `fork_point(trunk())`, which
resolves to trunk's tip). In the rare criss-cross history where more than one
merge base exists, `jj diff --from` rejects a multi-commit revset; use
`latest(fork_point(trunk() | @))` to pick a single base if that topology must be
supported (`heads(...)` would return the incomparable bases, not collapse them —
verify against a criss-cross fixture). The `git symbolic-ref` resolution anchors
the git side on the same origin-tracked trunk jj's `trunk()` uses, so the two
backends agree on the divergence point; both need an origin trunk (the symref
fails if `origin/HEAD` is unset, the parallel of jj's `root()` fallback), which
the AC5 fixtures establish.

The `detect.rs` module doc should note (tersely, not as a mutable enumeration
that drifts) that the reference is a shared contract several skills depend on, and
the ADR-promotion trigger (Work Item Reconciliation) tracks the dependent count.

#### 3. Golden hand-edit

**Files**: `cli/vcs-test-support/fixtures/vcs-detect/main-jj-workspace.json`,
`main-git-checkout.json`, `colocated-git-as-file.json`
**Changes**: `detect_goldens.rs` does an exact `assert_eq!` against these
committed fixtures and has **no `REGENERATE_GOLDENS` mechanism** (unlike
`status_log_goldens.rs`), so update them by hand: edit only the added bullet lines
in each embedded `additionalContext` (the jj-mode goldens gain the two jj bullets;
the git-mode golden gains the two git bullets), then review the fixture diff to
confirm exactly the added lines changed and no pre-existing bullet was reformatted
or dropped.

### Success Criteria:

#### Automated Verification:

- [ ] Render unit tests pass, asserting both idioms per backend plus the jj
      identity fallback clause and the load-bearing semantic phrases: `cd cli &&
      cargo test -p accelerator-vcs detect::tests`
- [ ] The three descriptive goldens match after the hand-edit: `cd cli && cargo
      test -p accelerator-vcs --features bash-parity --test detect_goldens`
- [ ] Single-component check passes: `mise run cli:check`

#### Manual Verification:

- [ ] `accelerator vcs detect --descriptive` in a jj checkout shows both new jj
      bullets (with the identity fallback clause); in a git checkout shows both
      new git bullets.

---

## Phase 3: `validate-plan` rewrite

### Overview

Replace the raw-git evidence block with session-VCS phrasing plus `accelerator
vcs root`, neutralise the soft "git" prose, and add the `allowed-tools` entry
the new command needs. Depends on Phases 1 and 2.

### Changes Required:

#### 1. Permit the new command

**File**: `skills/planning/validate-plan/SKILL.md` (allowed-tools)
**Changes**: Add `- Bash(accelerator vcs root)` and `- Bash(accelerator vcs
detect --descriptive)` (the latter for the reference re-derivation below),
matching the existing scoped, exact-match, bare-`accelerator` entry format. The
`vcs root` entry scopes only the bare invocation; the compound checks command
below (`cd … && make …`) is not covered by it — as before, that step relies on
the caller's general Bash permission, not this entry.

#### 2. Evidence-gathering block

**File**: `skills/planning/validate-plan/SKILL.md` (the fenced raw-git evidence
block — anchor on the `git log --oneline` / `git diff HEAD~N..HEAD` /
`git rev-parse --show-toplevel` lines)
**Changes**: Rewrite the fenced raw-git block to session-VCS guidance and the
`accelerator vcs root` checks step. The diff bullet defers to the reference by
name and inlines no backend command, so the mode-selected reference is the single
source — the model runs the correct form for its session and cannot pick a
wrong-backend command the guard would then deny.

```markdown
3. **Gather implementation evidence**:

- Recent commits: list the checkout's recent revisions using the session's
  VCS log command (see the SessionStart VCS Command Reference).
- Implementation diff: take the cumulative change since the trunk divergence
  point using the session's VCS diff-range idiom (see the SessionStart VCS
  Command Reference).
- Run the repository's checks from its root, aborting if the root cannot be
  resolved (out of a repository, `accelerator vcs root` exits non-zero, so the
  assignment fails and the `&&` chain stops rather than running `make` in the
  wrong tree):
  ```bash
  root="$(accelerator vcs root)" && cd "$root" && make check test
  ```
```

#### 3. Soft prose

**File**: `skills/planning/validate-plan/SKILL.md` (anchor on the phrases
"through git" and "analyze the git history")
**Changes**: "through git and codebase analysis" → "through the session's VCS
and codebase analysis"; "analyze the git history" → "analyze the VCS history".

### Success Criteria:

#### Automated Verification:

- [ ] No git subcommand remains in the skill: `grep -EnC0 '\bgit
      (status|diff|add|commit|log|branch|checkout|switch|merge|rebase|reset|stash|show|rev-parse|config)\b'
      skills/planning/validate-plan/SKILL.md` returns nothing.
- [ ] `allowed-tools` includes `Bash(accelerator vcs root)`.
- [ ] Bare-invocation lint and skill checks pass: `mise run check`.

#### Manual Verification (documented release gate):

Because `validate-plan` is model-driven prose, these behavioural checks cannot be
unit-tested. Record them as a **mandatory release-time gate** for this work item,
run against fixtures carrying three implementation commits with the jj fixtures
holding an `origin` trunk bookmark (so `trunk()` resolves, per Phase 2):

- [ ] In a pure-jj fixture with ≥1 implementation commit ahead of trunk,
      `validate-plan` emits a non-empty recent-commit list and a non-empty
      implementation diff with no `fatal: not a git repository`.
- [ ] The implementation diff equals the committed cumulative change since the
      trunk divergence point — `git diff <trunk>...HEAD` (git-only, `<trunk>` the
      default branch) and `jj diff --from 'fork_point(trunk() | @)' --to @`
      (colocated and pure-jj) — verified with trunk advanced past the divergence
      point in at least one fixture, so a single-endpoint revset regression would
      be caught. Any difference reduces to uncommitted working-copy edits included
      by the jj `@` endpoint (per Phase 2), not to a wrong anchor.
- [ ] The checks step runs from the checkout `validate-plan` was invoked in.

**Detection of steering drift**: the diff is model-driven with no CLI backstop
(per the design decision and "What We're NOT Doing"). This release gate *is* the
detection mechanism — a diff that does not equal the reference form for its
backend, or that is empty in pure-jj, signals the reference idiom is no longer
steering reliably and triggers the acknowledged pivot to a deterministic diff
anchor command. The Phase 6 render-lock on the idiom strings catches the narrower
case of the reference wording drifting. Its coverage is bounded to the fixture
topologies: drift in a real consumer repo whose topology differs surfaces only at
the next gated run, not continuously — an accepted consequence of the deliberate
no-diff-CLI decision, stated so it is not misread as continuous protection.

**Reference availability**: `validate-plan` typically runs late in a long
session, when the SessionStart reference may have been compacted out of context.
Because the skill defers to the reference with no inline command, it should
re-derive the idioms at invocation by running `accelerator vcs detect
--descriptive` if the reference is not in context, guaranteeing the diff-range
and log idioms are present when evidence is gathered. Note the command emits the
SessionStart JSON envelope (`{"hookSpecificOutput":{…,"additionalContext":"…"}}`,
`\n`-escaped), not plain prose — `format` is ignored and output always routes
through `session_start` — so the model reads the idioms out of the
`additionalContext` string.

---

## Phase 4: `config/migrate` status-prose reword

### Overview

Reword the dirty-path confirmation guidance to name the session's VCS status
command. Leans only on the pre-existing reference; mergeable independently.

### Changes Required:

#### 1. Dirty-path confirmation

**File**: `skills/config/migrate/SKILL.md` (the dirty-path confirmation
sentence — anchor on the `` `jj status`/`git status` `` phrase, which wraps
across two source lines)
**Changes**: Because `jj` and ` status` sit on different source lines, reword the
whole sentence rather than matching a verbatim single-line substring. The
sentence should read to the effect of "confirm via the session's VCS status
command (see the SessionStart VCS Command Reference) that the dirty paths …",
naming no backend and leaving no literal `git status`. Use "the session's VCS …"
(the research-issue template's phrasing) uniformly across the rewritten skills,
not "your session's VCS".

### Success Criteria:

#### Automated Verification:

- [ ] No literal `git status` remains: `grep -n 'git status'
      skills/config/migrate/SKILL.md` returns nothing.
- [ ] No git subcommand remains: the Phase 3 sweep pattern over
      `skills/config/migrate/SKILL.md` returns nothing.
- [ ] Skill checks pass: `mise run check`.

#### Manual Verification:

- [ ] The resume guidance reads correctly in both a jj and a git session.

---

## Phase 5: `refine-work-item` — identity chain and discard prose

### Overview

Route the child-author fallback through the reference's user-identity idiom
(preserving git-only **and** colocated resolution) and reword the partial-write
recovery prose to plain file deletion, then update the eval spec that pins the old
wording. Depends on Phase 2 (which carries the identity fallback clause). No
backend token (`jj config`/`git config`/`jj restore`/`git restore`) remains in the
skill body — the author step names the identity idiom and defers to the reference
(matching `research-issue`), and the recovery prose instructs plain file deletion.

### Changes Required:

#### 1. Author resolution chain

**File**: `skills/work/refine-work-item/SKILL.md` (the child-author resolution
chain — anchor on the `jj config get user.name` → `git config user.name` legs)
**Changes**: Collapse the two backend legs into a single session-VCS identity
step that defers to the reference by name and inlines no backend command. The
reference's user-identity idiom (Phase 2) itself carries the fallback to the git
identity when the jj identity is unset, so this preserves git-only resolution
(git session → git identity) *and* colocated resolution (jj session → jj
identity, falling back to git) without spelling either command — which is what
lets the Phase 6 git-token sweep (whose pattern includes `config`) pass over this
skill.

```markdown
- `author:` ← first match in chain: parent work item's `author` field →
  configured `author` value (from context config) → the session's VCS user
  identity (the user-identity idiom in the SessionStart VCS Command Reference,
  which falls back to the git identity when the jj identity is unset, so git-only
  and colocated repositories both resolve) → ask the user once and apply to all
  children
```

#### 2. Discard prose

**File**: `skills/work/refine-work-item/SKILL.md` (the three partial-write abort
diagnostics — anchor on the `jj restore <file>` / `jj restore <parent-path>`
phrases at the "partial state", "Collision", and "Parent not updated" messages)
**Changes**: All three sites recover the same leftover state — **newly-written
child files on disk** (the "Parent not updated" site included: its message says
the parent was *not* updated and the children *remain on disk*, so there is no
tracked parent modification to discard). The correct, backend-neutral recovery is
to **delete the newly-written child file(s)**: right for git (the new files are
untracked, so `git restore` would not remove them) and for jj (deleting the file,
which the next snapshot records). Word each site "delete the newly-written child
file(s)" (the "Parent not updated" site keeps its existing "or add their links
manually" alternative).

These are messages the model prints to the *user*, and "delete the … file(s)" is
already concrete and backend-neutral — no reference lookup and no per-backend
command are needed. No `jj restore` token remains in the skill body, and none is
introduced.

#### 3. Eval spec

**File**: `skills/work/refine-work-item/evals/evals.json`
**Changes**: Update the `expected_output` prose and the assertion
`description`/`value` pairs that hardcode `jj restore` (the `"type": "contains"`
assertions asserting `"value": "jj restore"`). Replace each `contains` value with
the **stable literal the rewritten skill emits** — "delete the newly-written
child file(s)" — not a loose fragment that could match incidental prose and lose
its discriminating power. No eval task exists in `mise.toml`, so this spec is not
CI-gated — a manual release-time check, called out as such so the coverage
boundary is not misread.

#### 4. Recorded benchmark

**Files**: `skills/work/refine-work-item/evals/benchmark.json`,
`evals/benchmark.md`
**Changes**: Regenerate the recorded evidence via the eval harness rather than
hand-editing it — `benchmark.md` is human-readable evidence of an actual run, so
hand-"updating" its rows would record outcomes no run produced. Regeneration is
harness-driven, not CI-gated.

### Success Criteria:

#### Automated Verification:

- [ ] No git subcommand remains: the Phase 3 sweep pattern over
      `skills/work/refine-work-item/SKILL.md` returns nothing.
- [ ] No restore command remains in the skill body: `grep -nE '(jj|git) restore'
      skills/work/refine-work-item/SKILL.md` returns nothing (the git-token sweep
      does not cover `restore`, so this is a separate check; recovery is plain
      deletion).
- [ ] The eval spec no longer hardcodes `jj restore`: `grep -n 'jj restore'
      skills/work/refine-work-item/evals/evals.json` returns nothing.
- [ ] Frontmatter validates and skill checks pass: `mise run check`.

#### Manual Verification:

- [ ] In a jj fixture (`jj config user.name` set, no git identity), a decomposed
      child resolves `author` to that value.
- [ ] In a git-only fixture (`git config user.name` set, no jj), a decomposed
      child resolves `author` to that value, via the identity idiom with no
      hard-coded `git config` call in the skill.
- [ ] In a colocated fixture (jj identity unset, `git config user.name` set), a
      decomposed child resolves `author` to the git value via the reference's
      fallback clause — the previously-regressing case.
- [ ] The refine-work-item eval suite passes when run via the eval harness (a
      manual release-time check — the suite is not CI-gated).

---

## Phase 6: Enforcement — git-token lint and SKILL↔CLI reference check

### Overview

Give the two headline invariants CI teeth. Today the "no direct git token in
`skills/`" sweep is a one-shot manual `grep` (no registered lint), and the
skill→CLI dependency is unenforced. Add two `tasks/lint` checks wired into the
aggregate `check`. This phase depends on Phases 1-5: the git-token lint only
passes once the skills are clean, and the reference check only passes once
`vcs root` and the idioms exist and the skills reference them — so it lands last.

### Wiring surface (both guards)

Each guard is its own `tasks/lint/<name>.py` module (mirror `bare_invocation.py`,
**not** `scripts.py` — that module is the shell-only guard over
`SURVIVING_SHELL_SOURCES`, wired into `scripts:check`, the wrong domain for a
SKILL.md scan). The established skills-tree-guard convention
(`lint:bare-invocation`, `lint:dispatch-coherence`, `lint:integration-skills`)
requires four coordinated surfaces per guard — the plan under-specified this:

1. `tasks/lint/__init__.py` — module import + `__all__` entry.
2. `tasks/__init__.py` — `ns_lint.add_collection(Collection.from_module(...))`
   so `invoke lint.<name>.check` resolves.
3. `mise.toml` — a `[tasks."lint:<name>:check"]` entry wired into **both**
   `build-system:check` (the roll-up CI runs) **and** `lint:check` (the only
   aggregate the bare `default` task reaches — `default` depends on `lint:check`,
   not `check`).
4. `tests/unit/tasks/test_mise.py` — add the task name to
   `_BUILD_SYSTEM_CHECK_GATES`, which pins the dual placement.

Both guards build on the shared `tasks/shared/skill_parsing.py` helpers (as
`dispatch_coherence.py` does) so they cannot drift to a different notion of a
SKILL.md body, and follow the testable `violations(root) -> list[str]` + `@task
check` shape. Each guard's test suite covers three cases (mirroring
`test_dispatch_coherence.py`/`test_lint.py`): `violations()` flags a bad fixture,
`violations()` is empty on a clean fixture, and the `@task check` wrapper **raises
`Exit` when `violations()` is non-empty** (an inverted condition or a `print` in
place of the raise otherwise leaves the gate green forever). All lint tests build
**synthetic `tmp_path` skill trees** (as `tests/unit/tasks/test_bare_invocation.py`
does); the only real-tree assertion is "the current tree passes", so a fixture is
never committed under `skills/` where it would self-trip the plugin-wide sweep.

### Changes Required (test-first):

#### 1. Git-token sweep lint

**File**: `tasks/lint/git_tokens.py` (new)
**Changes**: Write the failing test first, **parametrised** over representative
subcommands — at minimum `log`, `rev-parse`, and `config` (the two absent from
the guard blocklist are the load-bearing ones: `git rev-parse` is
`validate-plan`'s actual pure-jj breakage, `git config` the token
`refine-work-item` sheds) — mirroring the parametrised forms in
`test_bare_invocation.py`; a clean fixture passes. Then scan `skills/**/SKILL.md`
bodies for the blocklisted git subcommands
(`status|diff|add|commit|log|branch|checkout|switch|merge|rebase|reset|stash|show|rev-parse|config`)
and fail with the offending file:line. Scope to `SKILL.md` only — **not**
`evals/*.json`, where `create-work-item`'s fixtures narrate `git config user.name`
as historical prose (not a command invocation), matching AC1's reviewer clause.

The sweep is deliberately **git-token-only** (that is AC1's exact invariant). It
does not guard the jj-token neutralisations (`jj restore`/`jj config`) this plan
makes in `refine-work-item` — a blanket jj sweep would false-positive on
`update-work-item`'s `jj restore`, which "What We're NOT Doing" deliberately
retains. The jj-side neutralisations are verified once by the Phase 5 grep
criteria, not durably; this asymmetry is intentional and noted so it is not read
as an oversight.

#### 2. SKILL↔CLI reference check

**File**: `tasks/lint/skill_cli_refs.py` (new)
**Changes**: Write the failing test first for **both** assertions (the prior
draft tested only the first); a fixture skill referencing `accelerator vcs bogus`
fails, and a `detect.rs` fixture (or the const) with an idiom anchor removed
fails. Then assert, failing with a precise message otherwise:

- **Subcommand resolution (a real two-way binding)**: every `accelerator vcs
  <sub>` token in a `SKILL.md` resolves to a real `accelerator-vcs` subcommand.
  Do **not** hardcode the subcommand set — pin it against the clap `Command` enum
  in `cli/vcs-cli/src/cli.rs` with a test, mirroring how
  `dispatch_coherence.py`'s `BUILTIN_SUBCOMMANDS` is pinned against the launcher
  enum, so the lint cannot go stale when a subcommand is added or renamed.
- **Idiom render-lock (a one-way existence guard, named as such)**: the reference
  consts in `detect.rs` still contain the diff-range and user-identity (with its
  fallback clause) idiom anchors. Assert the **minimal, most stable** anchors —
  the commands (`fork_point(trunk() | @)`, `<trunk>...HEAD`, `git config
  user.name`) plus one short descriptive phrase per idiom ("trunk divergence
  point", "user identity") — mode-specific, so removing the fallback clause from
  the jj block fails even though `git config user.name` still appears in the git
  block. Keeping the anchors minimal avoids needless duplication with Phase 2's
  Rust `.contains(...)` render tests: the render-lock guards *existence* of the
  reference content across the Python/Rust boundary, not its exact wording. This
  is a render-lock against deletion, **not** a dynamic skill→idiom binding (skills
  name idioms in free prose, which carries no machine-checkable token) — the
  Implementation Approach and Overview are worded to claim only that.

The subcommand half is what makes the Phase 3/5 → Phase 1/2 ordering a CI failure
rather than a discipline (see Implementation Approach).

### Success Criteria:

#### Automated Verification:

- [ ] The git-token lint fails on `log`/`rev-parse`/`config` fixtures and passes
      on the clean tree; the reference check fails on both a bogus-subcommand
      fixture and an idiom-anchor-removed `detect.rs` fixture, and passes on the
      real tree: run the two new lint tasks' tests.
- [ ] The subcommand set is pinned against the clap `Command` enum by a test
      (adding/removing a `vcs` subcommand without updating the lint fails that
      test).
- [ ] Both tasks resolve via `invoke lint.<name>.check` and are wired into
      `build-system:check` **and** `lint:check`; `test_mise.py`'s
      `_BUILD_SYSTEM_CHECK_GATES` lists both.
- [ ] The bare `default` task (`mise run`) runs both guards and exits 0 on the
      finished tree; the plugin-wide `SKILL.md`-scoped git-token sweep returns
      nothing.

#### Manual Verification:

- [ ] Temporarily reintroducing `git diff` into a SKILL.md, or renaming
      `vcs root`, or deleting an idiom anchor from `detect.rs`, each makes
      `mise run` fail — confirming the guards bite.

---

## Work Item Reconciliation

The plan corrects several things the work item (0286) still records incorrectly;
because the work item is the source of truth for the acceptance gate, edit it so
it does not disagree with the delivered design. Not gated by `mise run`; part of
closing the item.

- [ ] **Requirement + Acceptance Criterion 2**: reword to name the working-copy-
      root (`discover`) capability the `vcs root` command surfaces, not
      `repository_root`; state the secondary-workspace behaviour (returns the
      workspace, not the shared main repo). Preserve the intent ("the checkout's
      repository root").
- [ ] **Acceptance Criteria 3 and 5 (diff-range idioms)**: the plan deliberately
      changed the idioms the work item hard-codes. Reword AC3 and AC5 to the
      delivered forms — jj `jj diff --from 'fork_point(trunk() | @)' --to @` (a
      merge-base fix over the buggy single-revision `fork_point(trunk())`, not
      just wording) and git `git diff <trunk>...HEAD` where `<trunk>` is the
      resolved default branch (not literal `main...HEAD`) — and note the
      origin-trunk precondition (both backends). Without this, the acceptance gate
      still encodes the two defects the plan removed.
- [ ] **Acceptance Criterion 6 (author resolution)**: AC6 tests only jj-only and
      git-only identity. Add a colocated leg (jj identity unset, `git config
      user.name` set → the git value via the reference's model-driven fallback) —
      the plan calls this "the previously-regressing case", so the gate should
      require it. Verified at release time (the fallback is model-driven prose,
      not an adapter capability), not by CI.
- [ ] **Technical Note — dispatched-sub-binary checklist**: correct to "a plain
      second-level clap subcommand on the already-registered `vcs` token", not the
      thirteen-point per-token checklist.
- [ ] **Technical Note — allowed-tools**: correct to "`validate-plan` lists
      scoped entries, not blanket `Bash(accelerator ...)`, so the rewrite adds
      `Bash(accelerator vcs root)`".
- [ ] **Reference-as-steering contract**: the SessionStart VCS Command Reference
      is now extended a second time and depended on by four skills
      (`validate-plan`, `config/migrate`, `refine-work-item`, `research-issue`);
      no ADR covers session-VCS injection. ADR-0066 owns the status/log **output
      format** only — not the reference-as-steering/injection contract, which
      remains deliberately ADR-less. Decision for this item: **no new ADR** — the
      contract is recorded by this plan plus a Drafting Note in 0286 pointing at
      the `detect.rs` reference consts. Concrete promotion trigger: a **fifth**
      dependent skill, or any idiom removal — not the open-ended "if a further
      skill leans on it". (The Drafting Note, not a `detect.rs` comment, tracks
      the dependent count, so the source carries only the stable invariant.)

---

## Testing Strategy

### Unit Tests:

- `root.rs` handler over a stub `RepoRoot`: `Some(path)` prints the path, `None`
  errors naming the searched-from path. Cannot discriminate `discover` from
  `repository_root` (default identity impl) — the parity test does that.
- `detect.rs` render tests: both idioms per backend, plus the jj identity
  fallback clause and the load-bearing semantic phrases (trunk-agnostic hint,
  `root()` fallback) — `.contains(...)`.
- `vcs-adapters` adapter test: `InProcessProbe::user_name(root, Git)` resolves the
  git identity (bash-parity) — `user_name` is single-backend (its `Jj` arm has no
  git fallback), so this pins only git-kind resolution. The colocated jj→git
  fallback is **model-driven** (the reference bullet tells the model to try
  `git config user.name` when the jj identity is unset), not an adapter
  capability, so it is verified by the Phase 5 release-time colocated check, not by
  a CI adapter test.
- Phase 6 lint tests (synthetic `tmp_path` trees): the git-token sweep fails,
  parametrised over `log`/`rev-parse`/`config`; the SKILL↔CLI check fails on a
  bogus-subcommand fixture **and** on an idiom-anchor-removed `detect.rs` fixture;
  the subcommand set is pinned against the clap `Command` enum by a test.

### Integration Tests:

- `root_goldens.rs` (bash-parity, `#![cfg(feature = "bash-parity")]`) over real
  git-only, colocated, pure-jj, and jj-secondary checkouts, asserting the printed
  root equals the checkout's own canonical root. The secondary case is the
  discriminating assertion for the `discover`-not-`repository_root` decision and
  must assert `printed == secondary_canonical` (not merely non-empty) and not the
  main-repo path (mirror the `jj workspace add` setup and `/private`
  canonicalisation in `detect_goldens.rs:129-150,141-142`).
- `detect_goldens.rs` (bash-parity) against the three **hand-edited** fixtures
  (no `REGENERATE_GOLDENS` mechanism exists for these; review the fixture diff).

### Cross-skill Verification (CI-gated via Phase 6):

- The plugin-wide `SKILL.md`-scoped git-token sweep is a registered `tasks/lint`
  task wired into `check` (no longer a manual `grep`). Scoped to `SKILL.md`, so
  descriptive prose in eval fixtures (e.g. `create-work-item` evals narrating
  `git config user.name`) is not a false positive, satisfying AC1's reviewer
  clause.
- The SKILL↔CLI reference check binds every `accelerator vcs <sub>` in a skill to
  a real subcommand (pinned against the clap `Command` enum) and render-locks the
  idiom anchors (commands + descriptive phrases) in `detect.rs` against deletion.

### Release-time Verification (not CI-gated):

- `validate-plan`'s three-topology diff-equality checks (Phase 3), the source of
  the load-bearing AC5 guarantee, run against model-driven prose.
- `refine-work-item`'s eval suite (no `mise` eval task exists).
- `research-issue` regression (no code change): in a pure-jj fixture it gathers a
  non-empty log and diff with no `fatal: not a git repository` — coupled to
  Phase 2's reference edit, so re-run when the reference changes.

### Full Local CI:

- `mise run` (bare default task) exits 0 end-to-end — the acceptance gate.

## Performance Considerations

None. `vcs root` is one in-process `discover` call; the reference extension adds
four short lines to a string emitted once per session.

## Migration Notes

None. No data or state migration; `hooks/hooks.json` already passes
`--descriptive`, so the extended reference reaches sessions with no wiring
change.

## References

- Original work item: `meta/work/0286-eradicate-direct-git-calls-from-skills.md`
- Codebase research:
  `meta/research/codebase/2026-09-20-0286-eradicate-direct-git-calls-from-skills.md`
- Command surface to extend: `cli/vcs-cli/src/cli.rs:16`,
  `cli/vcs-cli/src/main.rs:98`
- Handler pattern to mirror: `cli/vcs-cli/src/detect.rs` (direct probe ports)
- Root capability: `cli/vcs-adapters/src/library.rs:511-519` (`discover`,
  `repository_root`); pinned by `cli/vcs-adapters/tests/library.rs:264-300`
- Reference consts: `cli/vcs-cli/src/detect.rs:11-73`
- Identity probe: `cli/vcs-adapters/src/library.rs:545`,
  `cli/vcs/src/lib.rs:111-132`
- Goldens: `cli/vcs-cli/tests/detect_goldens.rs`,
  `cli/vcs-test-support/fixtures/vcs-detect/`
- Session-VCS pattern to mirror:
  `skills/research/research-issue/SKILL.md:61-67`
- Injection precedent: `skills/vcs/commit/SKILL.md`
- Token registry (already carries `vcs`): `tasks/shared/paths.py:31`
- Lint pattern to mirror (Phase 6): `tasks/lint/bare_invocation.py` and its test
  `tests/unit/tasks/test_bare_invocation.py`; the shared SKILL.md parser
  `tasks/shared/skill_parsing.py`; the enum-pin precedent `BUILTIN_SUBCOMMANDS` in
  `tasks/shared/dispatch_coherence.py`
- Lint wiring surfaces (Phase 6): `tasks/lint/__init__.py` (registry),
  `tasks/__init__.py` (`add_collection`), `mise.toml` (`lint:<name>:check` into
  `build-system:check` and `lint:check`), `tests/unit/tasks/test_mise.py`
  (`_BUILD_SYSTEM_CHECK_GATES`)
