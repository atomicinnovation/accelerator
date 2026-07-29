---
type: plan
id: "2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename"
title: "Bootstrap Self-Location and the ACCELERATOR_PLUGIN_ROOT Rename Implementation Plan"
date: "2026-07-27T09:02:16+00:00"
author: "Toby Clemson"
producer: create-plan
status: in-progress
work_item_id: "work-item:0182"
parent: "work-item:0182"
derived_from:
  - "codebase-research:2026-07-27-0182-plugin-root-self-location-implementation-surface"
  - "issue-research:2026-07-26-cli-requires-claude-plugin-root-env-var"
tags: [plan, cli, launcher, bootstrap, plugin-root, hooks, lint-guards]
revision: "e56fb165ea4b7591de3586bc43e96cb8bf7ab6df"
repository: "accelerator"
last_updated: "2026-07-29T13:40:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Bootstrap Self-Location and the `ACCELERATOR_PLUGIN_ROOT` Rename Implementation Plan

## Overview

`bin/accelerator` aborts unless `CLAUDE_PLUGIN_ROOT` is in its process
environment, but Claude Code only ever *substitutes* that token into skill
content — it exports it to hooks and MCP/LSP subprocesses, never to the Bash tool
or `!` preprocessor shells. All 45 CLI-invoking skills therefore fail at load.

This plan makes the bootstrap derive the installation root from its own location
(symlink-aware) and degrade quietly under `--fail-safe`; renames every launcher-
and server-layer participant onto `ACCELERATOR_PLUGIN_ROOT`; adds a lint guard
making the must-not-name-`CLAUDE_*` boundary non-negotiable across the rename
set; makes terminal invocation a documented, tested surface via a
`SessionStart`-refreshed link; and closes the silent-empty-answer mode by making
the plugin root a named refusal at the three consumers that actually read it.

The core repair lands first and alone: Phase 1a self-locates and exports **both**
variable names, so it fixes all 45 skills against the *already-shipped* launcher
with no Rust change and is independently releasable. Phase 1b then performs the
rename and drops the transitional export.

## Current State Analysis

**The bootstrap** (`bin/accelerator`, 263 lines) reads `CLAUDE_PLUGIN_ROOT` at 13
code sites and never exports it — the launcher inherits it today only because it
was already in the bootstrap's environment. `fail()` (`:20-23`) is the single
abort funnel, with 16 call sites and no other `exit 1` in the file. The file
contains no `BASH_SOURCE` reference; it is the only shell entry point in the
repository that cannot locate itself.

**argv is completely untouched before `exec`.** `"$@"` appears only at `:142`
(dev-override exec) and `:262` (verified exec); there is no `shift`, no `$#`
test, no `case "$1"`. Every other `$1`/`$2` is a function parameter.

**`fail()` already has a durable-record precedent for stderr invisibility.** The
dev-launcher override (`:114-116`, `:136-141`) warns on stderr *and* appends to
`${cache_dir}/.accelerator-unverified.log`, with the comment stating why: stderr
"is invisible at SessionStart and behind the !-site `--fail-safe` contract". The
same mechanism carries the trust-chain gates under `--fail-safe` in Phase 1a.
Note the ordering constraint: `cache_dir` is not resolved until `:106`, after the
shim (`:72`) and public-key (`:74`) gates.

**The launcher** reads the variable into an `Option<PathBuf>`
(`cli/launcher/src/main.rs:176`, no empty-string filter) with a single call site
at `:161` behind a lazy closure (`:197-199`), so `version` and external
subcommands never touch it. `FileConfigStore` is boxed into six ports, so one
`Option` fans out to every `config` command and degrades silently at four sites
in `cli/config-adapters/src/store.rs`.

**Root-sensitivity is narrower than the bug report implies.** Measured against a
freshly built launcher, only the template family changes behaviour with and
without a root; `config agents`, `paths`, `path <k>`, `instructions <skill>`,
`context --skill`, `work <k>` and `review <k>` are byte-identical either way
because they read *project* config. `config template <name>` is fail-closed even
under `--fail-safe` (`Failure::Refusal`, which `finish` never degrades);
`config templates list` is the silent case — an empty table at exit 0.

That table covers `config` only. **External-subcommand dispatch hard-requires the
root**: it resolves `<plugin_root>/bin` as the sub-binary cache and fails closed
with `CacheRootUnavailable` when there is neither a root nor an
`ACCELERATOR_CACHE_DIR`, as `cli/launcher/tests/version.rs:166-183` asserts. The
one `!` site that exercises it is
`skills/visualisation/visualise/SKILL.md:30` — see Phase 4.

**Testing gaps.** Every existing test injects `CLAUDE_PLUGIN_ROOT` or
`ACCELERATOR_BIN`, so the one configuration that matters in production — correct
path, empty environment — is never exercised. Two tests actively assert the
faulty behaviour. No test invokes the bootstrap through a symlink.
`mise.local.toml` — on no pushed branch — sets the variable and masks the whole
bug class in every local run, while simultaneously being the only reason the
installed plugin's skills work in this repository at all. (It was in the jj
working-copy commit when this was written; it is now untracked, since
`.gitignore` gained the entry the closing step calls for, so only the on-disk
file remains.)

## Desired End State

An installed `accelerator` runs correctly when invoked by absolute path, through
a symlink chain, or from a terminal, with no caller-set environment. Nothing in
the rename set — `cli/`, `bin/accelerator`, and the four out-of-tree writers —
names a `CLAUDE_*` variable, and a lint guard enforces that. A bootstrap-layer
failure at a `!` site degrades to empty injected context rather than discarding
the prompt, and a trust-chain failure leaves a durable record rather than no
trace. A missing plugin root is a named refusal at the three consumers that read
plugin content, while every root-independent `config` family — including
`summary` — keeps working without one.

Verified by: `mise run` exits 0 end-to-end; the Phase 1a regression tests fail
against the pre-change bootstrap and pass after; the manual pre-release check
confirms all four sampled skills load without a permission prompt.

### Key Discoveries

- **The `templates list` Path column cannot carry an absolute path.**
  `display_path` (`cli/config-adapters/src/store.rs:74-84`) deliberately
  shortens plugin-root paths to `<plugin>/templates/adr.md`. Measured. Two
  acceptance criteria assert the Path "lies under the resolved installation
  root", which the renderer never emits. See *Deviations from the work item*.
- **The gated dev override is not needed to supply a launcher, and the existing
  harness already has the mechanism.** `_serve_launcher`
  (`tests/integration/entrypoint/test_accelerator_entrypoint.py:108-113`) signs
  *whatever file* is placed in the stub release server with the test release
  key, and `_run_bootstrap` (`:225-252`) stubs the downloader entirely. Serving
  the real compiled `cli/target/debug/accelerator` gives a fixture-rooted run
  through the genuine fetch → verify → cache → exec chain with no network and no
  override — dissolving the work item's open question about the unworkable
  containment gate. **This is the only workable launcher-supply mechanism for any
  suite that runs the bootstrap**, and Phase 4 must use it too: substituting the
  `!`-site prefix to the *repo* root instead would self-locate there, pass every
  gate, and `curl` the real GitHub release for a version that is not yet
  published — the same hazard that condemns the two deleted tests.
- **The launcher's `config` family works with no project in cwd.** Measured from
  a bare `mktemp -d`: `config template adr` and `config templates list` both
  succeed. No fixture project is needed for the Phase 1a bootstrap tests.
- **A reviewed symlink chase already exists in this repo's history** —
  `meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md:620-655`,
  whose review settled `while [ -L` over `readlink -f` (`:392`, `:738`),
  mandatory cycle detection (`:757`), and the hop bound (`:856` — Darwin's
  `SYMLOOP_MAX` is 32, Linux's 40).
- **That prior review's cycle-test conclusion is wrong, and the hop bound is
  unreachable for cycles.** It held that a cycle test must go through
  `bash "$CYCLE_A"` because `execve()` returns `ELOOP` first (`:835`). But
  `bash <path>` must `open(2)` that path and the kernel returns `ELOOP` there
  too, so bash aborts before reading a byte and the in-script counter never
  fires. More generally: any path bash *can* open has already had its whole
  chain resolved by the kernel within `SYMLOOP_MAX`, so a cyclic chain is never
  observable in-script, and a bound of 32 could only fire on Linux for a
  *non-cyclic* 33–40-hop chain — rejecting a chain the host kernel accepts.
  Consequence: the bound drops to **16**, well below both kernels, which makes
  it genuinely testable with a 17-link non-cyclic chain on both platforms. The
  documented chain is two hops, so 16 is generous.
- **A compose-time `ConfigError` cannot degrade under `--fail-safe`.** `dispatch`
  does `let stack = compose_config()?;` (`cli/launcher/src/launch/mod.rs:189`),
  converting straight to `kernel::Error`; the flag is translated into
  `OnFailure` by `to_action` and applied only by `finish`
  (`cli/launcher/src/config_command/inbound/cli.rs:452-471`), which runs once a
  `ConfigStack` exists. So a plugin-root requirement placed at `compose_stack`
  could not degrade — which is why Phase 5 places it at the consumers instead.
- **`run_in` is env hygiene, not a rootless assertion.**
  `cli/launcher/tests/config_read.rs:58-67` removes `ACCELERATOR_LOG`,
  `CLAUDE_PLUGIN_ROOT`, `ACCELERATOR_CACHE_DIR` and
  `ACCELERATOR_RELEASE_BASE_URL` for hermeticity; 47 call sites use it and none
  asserts anything about the root (the 36 template sites use the separate
  `run_with_plugin`, `:1196-1208`, which builds its own `Command`). A
  requirement at `compose_stack` would break all 47 as collateral.
- **`mise run check` does not run `lint:check`.** `check` depends on five
  `<component>:check` roll-ups plus the `deny:check` and `pup:check` entity
  tasks (`mise.toml:465-467`, `tasks/README.md:32-36`), and
  `.github/workflows/main.yml:257` runs `mise run cli:check`. A rename-set
  guard must ride in `cli:check.depends`, and its real CI teeth come from the
  paired unit test's `violations(REPO_ROOT) == []` assertion. Note
  `lint:store-duplication:check` and `lint:vendor-shims:check` are **not** in
  `lint:check.depends` (`:451`) — `cli/`-scoped guards ride in `cli:check` only,
  and the new guard follows that precedent rather than dual-wiring.
- **`_ignore_spec()` honours only the root `.gitignore`.** Documented at
  `tasks/shared/sources.py:13-16` and justified there for `.sh` files only. The
  built SPA at `cli/visualiser/frontend/dist/` is ignored *only* by the nested
  `cli/visualiser/frontend/.gitignore` (the root file's `/dist/` is
  root-anchored), and it exists under `mise run` because the frontend builds
  before `cli:check`. A guard scanning `.js`/`.json`/`.ts` would read the
  minified bundle. `.venv` is likewise absent from the root `.gitignore`, which
  is why `tests/unit/tasks/test_python_coverage.py:76-94` prunes it explicitly.
- **`ACCELERATOR_MIGRATION_MODE` is dead only inside `cli/`.**
  `skills/config/migrate/scripts/run-migrations.sh:643` and
  `interactive-lib.sh:434,745` export it into every migration child; migrations
  `0001`, `0002`, `0004`, `0005` and `0006` invoke the Rust CLI; and
  `scripts/config-common.sh:41,56` plus `scripts/doc-type-table.sh:43` read it.
  `cli/config-adapters/tests/config_reader.rs:34` feeds
  `a_legacy_layout_fails_closed_under_migration_mode` (`:67-77`), which pins
  0178's decision *not* to port the bash bypass. The assignment is a live
  cross-layer contract test, not a vestige.
- **A competing terminal recipe is already printed at skill load.**
  `skills/visualisation/visualise/SKILL.md:162` renders
  `ln -s "${CLAUDE_PLUGIN_ROOT}/bin/accelerator" "$HOME/.local/bin/accelerator"`
  — a **version-pinned one-hop** link, which is exactly what Phase 3's two-hop
  chain exists to avoid. After Phase 1a such a link silently runs whichever
  installation it was created against. `docs/visualiser.md:24,38` also still
  documents an `accelerator-visualiser` wrapper that has not existed since 0168.
- **`hooks/test-vcs-detect.sh:615-634` hard-codes `.hooks.SessionStart[0]`** —
  the only hard-coded SessionStart index in the repository. A new hook entry
  must therefore be appended, never inserted; but the *new* assertions select by
  command content rather than index, so registration order stops accumulating
  positional couplings.
- **mise `depends` is a set, not a sequence.** No `wait_for` or `depends_post`
  exists in `mise.toml`; array position conveys no ordering. A build edge must
  go on the specific leaf task's own `depends`.
- **`mise.local.toml` has never been pushed.** It is absent from `main`,
  `origin/main` and `@-`, existing only in the current jj working-copy commit.
  Its effect is confined to this machine's local runs — it reaches no CI job, no
  release and no other contributor. Both source documents describe it as
  "tracked, so R6 is a real deletion", which overstates its reach.
- **Discovery must not use `rglob`.** `tasks/shared/sources.py:1-17` records that
  `git ls-files` is blind inside a jj workspace and silently emptied the scan;
  `.gitignore:20-21,25` ignore `cli/target/`, `cli/.pup/` and `node_modules/`,
  so a bare `(root/"cli").rglob("*.rs")` descends into thousands of vendored
  files.

## What We're NOT Doing

- **Standalone packaging.** The installation remains the Claude Code plugin
  cache, so `.claude-plugin/plugin.json`, `templates/`, `keys/` and a writable
  `<root>/bin` are always present. Only the *caller* may now be a human shell.
- **The `${CLAUDE_SKILL_DIR}` migration of `allowed-tools` rules.** Out of scope
  unconditionally: `${CLAUDE_PLUGIN_ROOT}` still substitutes into `allowed-tools`
  Bash rules (Phase 0), so the rules stay as they are.
- **Extending `tasks/lint/skill_permissions.py` to reject env-assignment
  prefixes.** Once Phase 1a lands, no caller has a reason to set the variable, so
  the pattern stops being generated. Deferred, not rejected.
- **Removing `cache_root.rs`'s refusal of an XDG fallback** (`:3-5`).
  `<root>/bin` always exists; only the variable name changes.
- **A `~/.local/bin` writer.** The Phase 3 hook writes only inside
  `${CLAUDE_PLUGIN_DATA}`. Rationale is in the work item's Technical Notes.
- **Changing the visualiser server's fatal exit.** Reached via the launcher's
  `exec`, it inherits the exported root; only the variable name changes.
- **Pinning the release key into the bootstrap.** The derived root supplies the
  public key, the verifier shim *and* the cache the launcher is exec'd from, so
  relocating the entry point relocates the whole trust anchor — an asymmetry with
  the launcher, whose anchor is compile-time-fixed (`cli/launcher/build.rs` copies
  the key into `OUT_DIR`; `resolve/keys.rs` does `include_str!`). Not addressed
  here, for two reasons. It is **not a privilege gain**: planting a symlink named
  `accelerator` on the victim's `PATH`, or getting them to execute a path inside
  an attacker-controlled tree, already implies the ability to plant a binary
  directly. And a key-only pin would close nothing while `${shim}` is
  root-derived too — the verifier could simply ignore the key — so it would need
  the key digest *plus* four per-platform shim digests, and
  `vendor_shim_marker_digest()` is a rebuild marker, not a content pin. The
  existing narrower hardening still holds: the shim runs by absolute path so a
  `PATH`-planted decoy cannot stand in for the verifier (`bin/accelerator:156-158`).

  The reachable failure here is **accidental** — self-location deriving a
  directory that is not an installation root — and Phase 1a covers it directly
  with the new `plugin.json`-gate test.
- **Removing the `ACCELERATOR_MIGRATION_MODE` assignment.** An earlier draft of
  this plan folded it into the rename as dead-code cleanup. It is not dead: see *Key
  Discoveries*. Only the doc comment at `cli/config-adapters/src/store.rs:20` is
  touched, to say the variable is *received* from the shell migration runner and
  deliberately ignored.

## Implementation Approach

Eight phases, mergeable **in sequence**. Each is green on its own, but they are
not order-free: Phase 0 alone has no predecessor, and Phases 2–5 all require
Phase 1b's rename because their criteria are expressed against
`ACCELERATOR_PLUGIN_ROOT`. Claiming otherwise would be false — Phase 4's suite is
red before the bootstrap is fixed, and Phase 3 documents a recipe that cannot
work without the chase.

The core repair is split three ways so the urgent fix stops waiting on the
rename:

| Phase | Content | Green alone? | State |
|---|---|---|---|
| 0 | Determinations — no code | yes, no predecessor | **done** 2026-07-28 |
| **1a** | Bootstrap self-locates, `--fail-safe`-aware `fail()`, exports **both** names; new tests, 2 deletions | **yes — and independently releasable** | **done** 2026-07-28 |
| **1c** | Work-item seam re-point + the `build:cli:dev` edge | **yes, today** — needs neither 1a nor 1b | **done** 2026-07-29 |
| **1b** | The rename: 3 production + ~10 test `cli/` readers, 5 out-of-tree writers; drops the transitional export | yes, **given 1c** | **done** 2026-07-29 |
| 2 | The `CLAUDE_*` boundary guard | after 1b | **done** 2026-07-29 |
| 3 | Terminal invocation surface | after 1a | **done** 2026-07-29 |
| 4 | `!`-site conformance suite | after 1a | **done** 2026-07-29 |
| 5 | Named error for a missing plugin root | after 1b | **done** 2026-07-29 |

**Progress — 2026-07-29.** All eight phases — 0, 1a, 1c, 1b, 2, 3, 4 and 5 — are
implemented, verified and committed. Only the closing step remains, and it waits
on a precondition outside this plan (a prerelease carrying the Phase 1a fix
installed and in use), along with the manual criteria deferred to the release
candidate: Phase 1a's published-release check and Phase 3's five installed-hook
checks. Sixteen commits carry the work:

| Commit | Content |
|---|---|
| `Record the CLAUDE_PLUGIN_DATA and hook output channel determinations` | Phase 0 |
| `Make the entrypoint harness rootless and hermetic` | 1a commit 1 |
| `Cover rootless invocation, symlink chases and fail-safe aborts` | 1a commit 2 |
| `Derive the installation root from the bootstrap's own location` | 1a commit 3 |
| `Stop three suites failing on wall-clock noise under parallel load` | out of scope — see below |
| `Force the work-item template fallback through ACCELERATOR_BIN` | Phase 1c |
| `Rename the plugin root onto ACCELERATOR_PLUGIN_ROOT` | Phase 1b |
| `Promote the gitignore-honouring walk to a shared primitive` | 2 commit 1 |
| `Add a guard against CLAUDE_* coupling in the rename set` | 2 commit 2 |
| `Wire the CLAUDE_* boundary guard into cli:check` | 2 commit 3 |
| `Keep a terminal-reachable link to the current launcher` | 3 commit 1 |
| `Give every shell-suite subtree a discovery floor` | 3 commit 2 |
| `Document running the accelerator CLI from a terminal` | 3 commit 3 |
| `Extract the fixture-installation apparatus for reuse` | 4 commit 1 |
| `Run every skill's config commands in the production shape` | 4 commit 2 |
| `Refuse rather than answer empty when the plugin root is unknown` | Phase 5 |

Two things a later phase inherits:

- **Three pre-existing flakes were fixed to get a green run**, on their own
  commit outside this plan's scope. The `test:integration:config` one was a real
  daemon defect (`shutdown()` closed the browser before removing
  `server-info.json`/`server.pid`, so `run.sh`'s reuse check still passed and
  the next launcher dispatched onto a dead page); the `test:unit:frontend` and
  `test:integration:visualiser` ones were wall-clock budgets sitting on the
  noise floor. Details under Phase 1a's `mise run` criterion.
- **The entrypoint suite had been running on Homebrew bash 5.3, not the 3.2
  floor.** `_BASH` now pins `/bin/bash` with an assertion. Any later phase
  touching `bin/accelerator` is genuinely held to 3.2 by the suite.

**1c must precede 1b.** The seam re-point is green today and depends on neither,
but landing it *after* the rename opens a window in which it is actively harmful:
1b renames `accelerator_env()` to export `ACCELERATOR_PLUGIN_ROOT`, at which point
the seam's injected `CLAUDE_PLUGIN_ROOT="/nonexistent"` is ignored by both the
self-locating bootstrap and the renamed launcher. The CLI then succeeds and returns
the *shipping* `templates/work-item.md`, whose trailing comments are byte-identical
to `hardcoded_fallback`'s values — so the four assertions pass without entering the
fallback branch, and Test 8's three tripwire comparisons degenerate into comparing
the shipping template with itself. `mise run test:integration:work` would be green
with seven assertions testing nothing, and 1b's own criteria cite that task as
evidence of correctness.

Phases 3 and 4 need only 1a, not 1b: Phase 4 strips both variables and substitutes
the prefix to a fixture root, and the launcher it serves reads whichever name is
current — which 1a's transitional export satisfies. So the real graph has two
independent branches after 1a rather than one chain.

**Phase 1a is a complete fix on its own.** Exporting `CLAUDE_PLUGIN_ROOT` with the
*derived* value makes the already-shipped `1.24.0-pre.16` launcher work —
including the template family — so 1a can ship as a prerelease that unbreaks all
45 skills with zero Rust changes. The export is write-only in both names, so the
directionality argument is unaffected: the bootstrap still never *reads* either.

The transitional export is the one thing 1b must not forget to remove. Phase 2's
guard covers `bin/accelerator`, so once it lands a leftover export is a lint
failure rather than a silent carry-over.

**`mise.local.toml` is deleted last, not first.** It is what keeps the
*installed* — still unfixed — plugin working in this repository, so removing it
early would take out `create-plan`, `implement-plan`, `commit` and
`validate-plan` while they are still needed to do the work. Its precondition and
why keeping it costs nothing after Phase 1a are under *Closing step* below.

Test-first throughout: within each phase the failing assertion is written before
the change that satisfies it, which is what makes the Phase 1a regression tests
demonstrably falsifiable against the pre-change bootstrap.

Commit sequences, so the risky changes are reviewed on their own commits rather
than inside multi-file diffs. Note the harness must land **before** the tests that
use it — otherwise commit 2 fails with fixture and `TypeError` errors rather than
the assertion failures that make it falsifiable against the pre-change bootstrap:

- **1a**: harness changes + the `_run_bootstrap` preconditions → regression tests
  (red for the right reason) → `bin/accelerator`.
- **2**: `walk_files` promotion with both callers repointed → the guard + its
  tests → registration.
- **3**: hook + suite + `hooks.json` → the suite-discovery floors →
  documentation.
- **5**: the `ConfigError` variant + `is_refusal()` + their unit tests → the
  `template_names` port signature with its four mechanical call sites and `# Errors`
  doc (no behaviour change, green on its own) → the two accessors, the three
  refusals and the new `config_read.rs` cases. The middle commit cannot compile
  until all four call sites move, so it has to be one commit; splitting the
  behaviour change out of it is the point.

### Deviations from the work item

Three acceptance criteria are restated. Each keeps its falsifying power; the
mechanism changes because the measured behaviour differs from what was assumed.

| Criterion | As written | Restated as | Why |
|---|---|---|---|
| 3 (`templates list` row) | Path "lies under the resolved installation root" | the output has a line naming `` `adr` `` whose Source cell reads `plugin default` | `display_path` shortens plugin-root paths to the `<plugin>/` token; an absolute path is never emitted. Rootless yields no row at all, so the row's *existence* is the discriminator — asserted by locating the line and checking the field, not by pinning the renderer's column order, backtick quoting and padding. |
| 4 (two-hop symlink) | "every plugin-default row's Path lies under `<fixture-root>/templates/`" | `config template adr` through the chain emits the fixture's sentinel string | Same rendering limit, and the token is root-independent in text. Asserting on template *contents* is absolute and unambiguous. |
| 5 (stale/wrong root ignored) | "no row resolves under the injected path" | two fixture roots with distinct sentinels; the self-located root's sentinel appears and the injected root's does not | Same reason; the two-sentinel form is directly falsifiable in both directions. |

The work item's open question about the dev-override precondition is answered
rather than restated: the criteria are satisfiable as scoped by serving the real
compiled launcher through the stubbed release server, so no override is used.

**R11 is resolved rather than probed.** See Phase 0 — the answer is positive, so
the `${CLAUDE_SKILL_DIR}` migration is out of scope unconditionally and the
conditional successor-item dependency is discharged.

**R6 is resequenced from first to last.** Both source documents recommend landing
it first, on the grounds that it masks the bug class locally and contaminates the
substitution probe. Neither holds:

- There is **no probe left to contaminate** — R11 is answered. Even had one been
  needed, `mise.local.toml` is repo-scoped mise config that does not apply
  outside this repository, so location rather than deletion would have settled it.
- The file is **not on any pushed branch**, so it masks nothing for anyone but
  the maintainer locally, and nothing in CI or a release depends on its absence.

Against that, deleting it early has a concrete cost the source documents miss: it
is what supplies `CLAUDE_PLUGIN_ROOT` to the *installed* plugin's still-unfixed
bootstrap in this repository, so the skills used to carry out this plan stop
working. It therefore moves to a closing step.

**Two acceptance criteria are re-scoped rather than restated**, because as written
neither can pass:

- The repo-wide `grep -rl 'CLAUDE_PLUGIN_ROOT'` residue list omits `meta/**`,
  which carries hundreds of tracked matches across plans, research, reviews and
  work items — including this plan. It is re-scoped to **non-`meta/`** tracked
  files, since documentation of history is not a coupling.
- The same list omits `tests/integration/entrypoint/test_accelerator_entrypoint.py`,
  which Phase 1a deliberately keeps naming: `test_ambient_roots_never_redirect_the_resolved_root`
  parametrises over the old variable precisely to prove it is inert. Added to the
  permitted set with that reason.

**One work-item claim is corrected.** The item's automated criterion counts the
`!`-site corpus against the same population as
`tasks/lint/skill_permissions.py`'s `EXPECTED_INJECTION_SKILLS = 42`. They are
different populations — that constant counts *injection* skills. Measured: 45
SKILL.md files carry a `bin/accelerator config` `!` site, the corpus is 204
`config` commands (**122** distinct, not ~125), and there are 206 `!`-site
launcher invocations in total. The two non-`config` ones are both in
`skills/visualisation/visualise/SKILL.md` — see Phase 4.

---

## Phase 0: Determinations

> **Done 2026-07-28.**

### Overview

One of the two questions the work item sequences ahead of implementation is
already answered; the other remains. No code changes — the deliverable is two
recorded answers.

### Changes Required

#### 1. `allowed-tools` substitution — resolved, record it

**R11 is answered positively and needs no probe.** `${CLAUDE_PLUGIN_ROOT}` does
still substitute into `allowed-tools` Bash rules on v2.1.220. Basis:

- **Maintainer's direct observation**: invocations covered by
  `${CLAUDE_PLUGIN_ROOT}`-prefixed `allowed-tools` rules do not raise a
  permission prompt.
- **The confound is excluded.** The work item's caveat — that an absent prompt
  proves nothing in a session granting Bash broadly — does not apply here. There
  is no project `.claude/settings.json` or `.claude/settings.local.json`, and
  `~/.claude/settings.json` carries `defaultMode: "default"` with **zero** Bash
  allow rules. So no ambient grant can explain the absent prompt.
- **The mechanism corroborates it.** A rule such as
  `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)` can only prefix-match a
  command that has been expanded to an absolute path if Claude Code substituted
  the variable on *both* sides. Substitution in content but not in the rule
  would leave the rule matching a literal `${CLAUDE_PLUGIN_ROOT}` prefix, and
  every covered call would prompt.

Consequences for this plan: the `${CLAUDE_SKILL_DIR}` migration is **not**
required in any branch, the conditional successor-item dependency is discharged,
and the manual pre-release check becomes a re-confirmation against the release
artifact rather than a discovery gate.

#### 2. Determine `${CLAUDE_PLUGIN_DATA}`'s availability — **without** raising the floor

Two questions, not one. The work item asks only *from which version* the variable
exists; the codebase pass showed the load-bearing question is **where it is
visible**:

- **From which Claude Code version** does `${CLAUDE_PLUGIN_DATA}` exist? (Claude
  Code changelog and plugins reference.)
- **In which contexts is it available?** Specifically: exported to hook processes
  (which Phase 3's hook needs), substituted into skill content (which a `!`-site
  `printf` would need), and present in a plain user terminal (which the documented
  recipe would need). The plugins reference documents it as a *hook* environment
  variable; the founding premise of this work item is that plugin variables are
  **not** exported to `!` shells, and no Claude Code process sets anything in the
  user's own terminal. So the working assumption is hook-only, and Phase 3 is
  written against that — see its documentation section, which uses the literal
  per-channel path rather than the token.

**The plugin-wide floor does not rise, whatever the answer.** It stays v2.1.144,
and `CLAUDE.md:123`, `docs/releases-and-compatibility.md:36` and
`meta/decisions/ADR-0051-skills-as-the-product.md:116-118` are all left untouched.
Three reasons:

- The only thing that depends on the variable is Phase 3's terminal-convenience
  link, and the hook is already inert when it is unset. Nothing else in the plugin
  needs it.
- The floor is prose-only — `.claude-plugin/plugin.json` has no
  `claudeCodeVersion`, `engines` or `minimumClaudeCodeVersion` field, and its only
  `requirements` entry is a free-text Node string. Nothing enforces or detects a
  sub-floor install, so raising it is pure communication.
- The work item records that the versions a raise would drop "are among the
  consumers currently hitting this bug". Withdrawing declared support from the
  installed base this release exists to unbreak, in exchange for a feature that
  degrades cleanly, is the wrong trade.

Instead the requirement is stated **locally**, in Phase 3's new `docs/internals.md`
section: the plugin-owned link requires Claude Code ≥ *X*; below that, link the
version-pinned `<root>/bin/accelerator` directly and re-run after each upgrade.
That keeps the constraint next to the only feature it constrains.

(This also means no accepted ADR is edited. `ADR-0051` states the floor as a live
fact in its Negative consequences, and `ADR-0031` plus
`skills/decisions/review-adr/SKILL.md:85-100` make accepted ADRs append-only —
superseding one to adjust a parenthetical would be disproportionate, and is
unnecessary once the floor holds.)

The hook still emits one `[accelerator]` stderr line when it goes inert, and the
documentation still opens with a verify step: the user's `ln -s` is created by
hand, so without both, a Claude Code lacking the variable leaves a dangling
`accelerator` on their general `PATH` with no explanation from either side.

#### 3. Hook output channels — resolved, record it

**Determined 2026-07-27 from the Claude Code hooks reference.** Three facts, which
together fix Phase 3's channel design and correct a misreading this plan previously
carried:

- **`systemMessage` is a *universal* hook output field, valid at the top level for
  every event** — documented in the reference's "JSON output" section alongside
  `continue`, `stopReason`, `suppressOutput` and `terminalSequence`, as "Warning
  message shown to the user". It is **not** a `SessionStart`-specific field and does
  **not** need nesting inside `hookSpecificOutput`. So `hooks/vcs-detect.sh:14`'s
  top-level `{"systemMessage": …}` is correct usage, not a mistake, and it is the
  documented way for any hook to put a line in front of the user.
- **For `SessionStart`, plain stdout becomes *Claude's context*, not user output**:
  "The exceptions are `UserPromptSubmit`, `UserPromptExpansion`, and `SessionStart`,
  where stdout is added as context that Claude can see and act on." A diagnostic
  written there would be spliced into the prompt — so plain stdout is the one
  channel Phase 3 must **not** use.
- **stderr at exit 0 is not documented for `SessionStart`.** Exit 2 shows stderr in
  the transcript; exit 0 stderr is unspecified. So `bin/accelerator:114-116`'s
  "invisible at SessionStart" is the safe reading, and
  `hooks/migrate-discoverability.sh:68-72`'s stderr advisory may indeed reach nobody
  — pre-existing, out of scope here, worth its own item.

Note the apparent conflict in the reference — `SessionStart` is listed as having "no
blocking or decision control" — is about *decision control* (deny/allow/block) and
is orthogonal to output channels. `systemMessage` is neither.

Consequence: Phase 3's two-channel split is **correct and no longer contingent**.
Routine and transient conditions go to stderr (accepting they may be unseen, which
is what Phase 1a's durable record exists for); the two persistent states the hook
cannot fix go to a top-level `systemMessage`. The `NOTICE` accumulator stays, since
at most one JSON object may reach stdout.

#### 4. Record all answers

**File**: `meta/work/0182-cli-derives-plugin-root-from-own-location.md`
**Changes**: Replace both Open Questions with the recorded results, naming the
Claude Code version tested.

### Success Criteria

#### Automated Verification

- [x] `mise run check` exits 0 (unchanged — this phase touches no code)

#### Manual Verification

- [x] The work item's Open Questions carry both answers and the Claude Code
      version tested (v2.1.220 for the substitution answer)
- [x] The substitution answer records its basis: the maintainer's observation,
      the verified absence of any Bash allow rule with `defaultMode: "default"`,
      and the both-sides matching argument
- [x] The declared floor is **unchanged** at v2.1.144 — `CLAUDE.md:123`,
      `docs/releases-and-compatibility.md:36` and
      `meta/decisions/ADR-0051-skills-as-the-product.md:116-118` are untouched
- [x] The `${CLAUDE_PLUGIN_DATA}` answer records its first version, **which
      contexts export it** (hook / skill content / plain terminal — Phase 3's recipe
      depends on this), and that it is **not version-scoped** (confirmed
      2026-07-28, which is what makes the user's hop a one-time action)
- [x] The `SessionStart` stderr visibility answer is recorded, and if stderr is
      discarded, a follow-up item is raised for
      `hooks/migrate-discoverability.sh`'s advisory — it is discarded, and the
      follow-up is work item 0183
- [x] Phase 3's `docs/internals.md` section states the version requirement
      locally, with the version-pinned fallback for anyone below it. **Done
      2026-07-29**: the Terminal Invocation section names v2.1.78 and tells
      anyone below it to link `<plugin root>/bin/accelerator` directly and
      re-run after each upgrade.

---

## Phase 1a: Bootstrap Self-Location and `--fail-safe`

> **Done 2026-07-28.** Three commits, as sequenced below.

### Overview

The core repair, and nothing else. The bootstrap self-locates, becomes
`--fail-safe`-aware, and exports the derived root under **both** variable names.
No Rust changes, so the already-shipped launcher reads the old name and works —
which makes this phase a complete fix for all 45 skills and independently
releasable.

The dual export is explicitly transitional; Phase 1b removes it, and Phase 2's
guard makes a leftover a lint failure rather than a silent carry-over.

### Changes Required

#### 1. The regression tests (written first)

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`

A module-scoped `launcher_bin` fixture mirroring the existing `shim_bin`
(`:132-153`), building the real launcher in-fixture:

```python
@pytest.fixture(scope="module")
def launcher_bin() -> Path:
    _require("cargo")
    subprocess.run(
        [
            "cargo",
            "build",
            "--quiet",
            "--bin",
            "accelerator",
            "--manifest-path",
            str(_REPO_ROOT / "cli/Cargo.toml"),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    launcher = _REPO_ROOT / "cli/target/debug/accelerator"
    if not (launcher.exists() and os.access(launcher, os.X_OK)):
        pytest.fail(f"launcher not built: {launcher}")
    return launcher
```

Building in-fixture rather than via a `mise` edge is deliberate and follows
`shim_bin`: it keeps `uv run pytest tests/integration/entrypoint` working
standalone. The consequence is that `test:integration:entrypoint` must **not**
gain `build:cli:dev` — the two would contend on cargo's target lock and the
asserted edge would be inert. Phase 1c records the omission explicitly.

`make_harness` returns a small dataclass rather than a bare tuple, so each root
carries its own sentinel and no assertion does tag arithmetic against the
factory's call order:

```python
@dataclass(frozen=True)
class Harness:
    root: Path
    server: Path
    sentinel: str


_SENTINEL = "FIXTURE-ADR-SENTINEL-{label}"
```

`make_harness` gains `label: str` (e.g. `"self"`, `"injected"`) and
`real_launcher: bool = False`, and writes a `templates/adr.md` bearing
`_SENTINEL.format(label=label)`, so a resolved root is provable from stdout
rather than from a rendered path.

When `real_launcher` is set, `_serve_launcher` copies `launcher_bin` into the
stub release server instead of writing `_LAUNCHER_SRC`, and signs it with the
same test release key. The bootstrap then fetches it through the injected
downloader, verifies it with the real shim, caches it, and execs it — the genuine
chain, no network, no dev override. Reserved for the **three** tests that assert on
launcher-rendered stdout (`config template adr`, `templates list`, and the
`instructions` degradation case); every root-derivation test uses an extended
`_LAUNCHER_SRC` stub that dumps its environment to a file, which asserts the export
directly rather than inferring it from template output — cheaper, and it fails at
the layer that actually broke. Note the ambient-root test moves to the stub for
this reason: asserting `dumped_env["ACCELERATOR_PLUGIN_ROOT"]` is both more direct
than a two-sentinel stdout comparison and avoids signing a debug binary twice per
parametrisation.

`_run_bootstrap` drops `CLAUDE_PLUGIN_ROOT` from its explicit environment (the
injection becomes a stale seam once the bootstrap self-locates) and gains two
parameters: `cwd`, defaulting to a project-free temp directory, and
`entry: Path | None = None`, defaulting to `root / "bin/accelerator"` — required
by the symlink-chain and cycle tests, which must invoke a *different* path.

**A network and working-tree guard, enforced at the chokepoint.** An autouse
`conftest.py` fixture would **not** work here: `_run_bootstrap` (`:225-252`) builds
a complete explicit `env=` dict passing only `PATH` and `HOME` through, so nothing
a fixture sets on `os.environ` reaches the child — and the two hazardous tests
called `subprocess.run` directly, bypassing the funnel entirely. The preconditions
therefore go **inside `_run_bootstrap`**, which is the single funnel every
invocation must pass:

- assert the composed `env` contains `ACCELERATOR_BOOTSTRAP_DOWNLOADER`;
- assert `ACCELERATOR_RELEASE_BASE_URL` is present and ends in `.invalid`;
- assert the resolved entry path is not `_REPO_ROOT / "bin/accelerator"`.

**Implemented as a *hostname* check, not a suffix check.** The harness's URL is
`https://example.invalid/v{_VERSION}`, whose last component is the version — a
literal `endswith(".invalid")` would fail on the very value it is meant to
admit, and "fix" it by moving the unresolvable label off the host, where it
guarantees nothing. `urlparse(...).hostname.endswith(".invalid")` is what the
criterion means and is strictly stronger: it pins the reserved TLD on the part
that determines egress.

Plus a session-scoped assertion that the repo's `bin/` gained no cached-launcher,
staged-shim, lock or unverified-log entries during the run — a backstop for
anything that bypasses the funnel, accepting that it fires after egress rather
than preventing it. Replace the module-level `_BOOTSTRAP = _REPO_ROOT /
"bin/accelerator"` with a fixture that copies into `tmp_path`, so a
production-rooted invocation stops being expressible. Deleting the two hazardous
tests removes the known instances; this stops the trap being reachable again,
which self-location makes easier to fall into.

The same preconditions are needed by Phase 4, whose suite is outside this
directory — see *Phase 4* for the shared support module that carries them.

**File**: `.gitignore`
**Changes**: Add `bin/accelerator-launcher-*`, `bin/.accelerator-lock-*`,
`bin/.accelerator-unverified.log`, `bin/.tmp-launcher-*`, and — for the launcher's
*sub-binary* cache, which lands in the same directory under a different scheme
(`cache.rs:30,34`) — `bin/visualiser-*` and `bin/*.minisig`. None is ignored today,
so a stray fetched binary can currently be committed into the directory the plugin
ships from.

For the digest-suffixed staged shims, the pattern must be **digest-anchored**:
`bin/accelerator-verify-*-[0-9a-f][0-9a-f][0-9a-f][0-9a-f]*`. The obvious
`bin/accelerator-verify-*-*` also matches all four *committed* vendored shims
(`accelerator-verify-darwin-arm64` matches with `*`=`darwin`, `*`=`arm64`).
Tracked files are unaffected today, so this would be silent — but after
`mise run build:vendor-verify-shims` the refreshed shims could no longer be
`git add`ed without `-f`, while `lint:vendor-shims:check` compares the *marker*
digest and would stay green. A security-motivated `minisign-verify` bump could
then ship with the old verifier binaries. Add an assertion to
`tests/unit/tasks/test_bootstrap_coverage.py` that the `.gitignore` spec matches
none of the four `vendored_shim_path(platform)` values.

Also drop the stale `.gitignore:23` entry
(`skills/visualisation/visualise/bin/accelerator-visualiser-*`), a pre-0168 path
that no longer exists.

The new tests. **The comments below are for this plan and must not be
transcribed** — they name plan sections and deleted tests, which this repo's
comment rules forbid; the test names carry the intent:

```python
def test_rootless_template_render_resolves_the_fixture_root(...):
    result = _run_bootstrap(h.root, h.server, downloader,
                            args=("config", "template", "adr"))
    assert result.returncode == 0
    assert h.sentinel in result.stdout
    assert "accelerator:" not in result.stderr


def test_rootless_instructions_command_degrades_cleanly(...):
    result = _run_bootstrap(
        h.root, h.server, downloader,
        args=("config", "instructions", "commit", "--fail-safe"))
    assert result.returncode == 0
    assert "accelerator:" not in result.stderr


def test_rootless_templates_list_carries_the_plugin_default_row(...):
    result = _run_bootstrap(h.root, h.server, downloader,
                            args=("config", "templates", "list"))
    row = next(line for line in result.stdout.splitlines() if "adr" in line)
    assert "plugin default" in row


def test_two_hop_symlink_chain_resolves_to_the_fixture_root(...):
    result = _run_bootstrap(..., entry=userbin_link)
    assert result.returncode == 0
    assert dumped_env["ACCELERATOR_PLUGIN_ROOT"] == str(h.root)


def test_a_directory_symlink_in_the_entry_path_resolves_physically(...):
    # <tmp>/other/bin -> <fixture-root>/bin, invoked as <tmp>/other/bin/accelerator.
    # A logical `cd ..` yields <tmp>/other; only `cd -P` yields the fixture root.
    assert dumped_env["ACCELERATOR_PLUGIN_ROOT"] == str(h.root)


@pytest.mark.parametrize("inject", ["old", "new", "both"])
def test_ambient_roots_never_redirect_the_resolved_root(inject, ...):
    # CLAUDE_PLUGIN_ROOT alone, ACCELERATOR_PLUGIN_ROOT alone, and both —
    # the three combinations the work item's criterion requires.
    assert dumped_env["ACCELERATOR_PLUGIN_ROOT"] == str(self_h.root)


def test_relative_symlink_target_resolves(...):
    # cwd is a decoy directory carrying its own .claude-plugin/, so a
    # cwd-relative resolution would be observable rather than benign.
    assert dumped_env["ACCELERATOR_PLUGIN_ROOT"] == str(h.root)
    assert dumped_env["ACCELERATOR_PLUGIN_ROOT"] != decoy


def test_an_exported_cdpath_does_not_redirect_the_resolved_root(...):
    # extra_env={"CDPATH": decoy_parent} where decoy_parent contains a bin/.
    assert dumped_env["ACCELERATOR_PLUGIN_ROOT"] == str(h.root)
    assert "accelerator:" not in result.stderr


def test_a_derived_root_that_is_not_an_installation_aborts_by_name(...):
    # The message must name the *derived* path, which also pins that
    # self-location ran rather than something else failing.
    assert "plugin.json not found" in result.stderr
    assert str(bare_root) in result.stderr
    assert result.returncode != 0
    # and the --fail-safe variant exits 0 with empty stdout


def test_a_sixteen_link_chain_resolves(...):
    # The at-boundary partner: without it, `-gt` -> `-ge` reddens nothing.
    assert dumped_env["ACCELERATOR_PLUGIN_ROOT"] == str(h.root)


def test_a_seventeen_link_chain_exceeds_the_hop_bound(...):
    assert "exceeded 16 hops" in result.stderr


def test_a_symlink_cycle_terminates_rather_than_hanging(...):
    # Characterises kernel ELOOP-at-open, not the in-script counter: with a
    # true cycle bash never reads the script, so this passes even with the
    # chase deleted. timeout= is what makes the hang detectable.
    result = _run_bootstrap(..., timeout=30)   # pytest.fail on TimeoutExpired
    assert result.returncode != 0


def test_a_leading_dash_link_target_resolves(...):
    # readlink output is taken verbatim; `cd --` and dir_of are what keep a
    # target such as "-x/real" from being read as an option.
    assert dumped_env["ACCELERATOR_PLUGIN_ROOT"] == str(h.root)


@pytest.mark.parametrize("args", [
    ("config", "get", "k", "--", "--fail-safe"),   # after `--`: NOT honoured
    ("config", "get", "k"),                        # absent: NOT honoured
])
def test_fail_safe_is_not_honoured_outside_its_scan_window(args, ...):
    # Against a not-an-installation fixture, so a gate fires either way.
    assert result.returncode != 0


def test_trust_chain_failure_records_durably_under_fail_safe(...):
    # Missing verify shim. The cache dir must pre-exist, since the shim gate
    # fires before resolve_cache_dir.
    assert result.returncode == 0
    assert result.stdout == ""
    assert "verify shim missing" in log_lines[-1]


def test_a_record_is_always_one_line(...):
    # ACCELERATOR_CACHE_DIR carrying a newline reaches the :168 message, so
    # the sanitiser — not any input check — is what keeps the log parseable.
    result = _run_bootstrap(..., extra_env={"ACCELERATOR_CACHE_DIR": newline_dir})
    assert len(log_lines) == 1
```

Two tests are **deleted**, not re-asserted: `test_unset_plugin_root_is_a_named_error`
(`:260`) and `test_non_directory_plugin_root_is_a_named_error` (`:273`). They are
the only tests that run the *repo* bootstrap rather than the fixture copy, and
their environments pass only `PATH`. With self-location they resolve the real
repo root, satisfy the plugin.json / shim / key / cache gates (all four
`bin/accelerator-verify-*` triples are committed), and fall through to real
`curl` against the real GitHub release URL, writing a cached launcher into the
working tree's `bin/`. Leaving them is a network call and a working-tree write,
not merely a red test. `test_a_derived_root_that_is_not_an_installation_aborts_by_name`
covers what they were actually protecting.

#### 2. The bootstrap

**File**: `bin/accelerator`
**Changes**: Replace the two root gates (`:25-27`) with an argv scan, a
fail-safe-aware `fail()` carrying a durable record for trust-chain gates, and a
symlink-chasing self-location; rename all remaining reads onto a local variable;
export the derived root under both names.

```bash
set -uo pipefail
CDPATH=

abort_status=1
if [[ "$#" -gt 0 ]]; then
	for arg in "$@"; do
		if [[ "${arg}" == "--" ]]; then
			break
		fi
		if [[ "${arg}" == "--fail-safe" ]]; then
			abort_status=0
			break
		fi
	done
fi

max_hops=16
unverified_log=""

fail() {
	printf 'accelerator: %s\n' "$1" >&2
	exit "${abort_status}"
}

fail_integrity() {
	if [[ -n "${unverified_log}" ]]; then
		mkdir -p "$(dir_of "${unverified_log}")" 2>/dev/null
		printf '%s pid=%s %s\n' \
			"$(date -u '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || printf 'unknown')" \
			"$$" "${1//$'\n'/ }" \
			>>"${unverified_log}" 2>/dev/null ||
			printf 'accelerator: could not record to %s\n' "${unverified_log}" >&2
	fi
	fail "$1"
}

dir_of() {
	case "$1" in
	*/*)
		dir="${1%/*}"
		printf '%s\n' "${dir:-/}"
		;;
	*) printf '%s\n' "." ;;
	esac
}

source_path="${BASH_SOURCE[0]}"
self="${source_path}"
hops=0
while [[ -L "${self}" ]]; do
	hops=$((hops + 1))
	if [[ "${hops}" -gt "${max_hops}" ]]; then
		fail "symlink loop resolving ${source_path} (exceeded ${max_hops} hops)"
	fi
	link_target=$(readlink "${self}") ||
		fail "could not read the symlink target of ${self}"
	case "${link_target}" in
	/*) self="${link_target}" ;;
	*)
		link_dir=$(cd -P -- "$(dir_of "${self}")" && pwd -P) ||
			fail "could not resolve the directory of ${self}"
		self="${link_dir}/${link_target}"
		;;
	esac
done
plugin_root=$(cd -P -- "$(dir_of "${self}")/.." && pwd -P) ||
	fail "could not resolve the installation root from ${source_path}"
readonly plugin_root
unverified_log="${ACCELERATOR_CACHE_DIR:-${plugin_root}/bin}/.accelerator-unverified.log"

export ACCELERATOR_PLUGIN_ROOT="${plugin_root}"
export CLAUDE_PLUGIN_ROOT="${plugin_root}"
```

The second export is what makes 1a work against the shipped launcher, and 1b
deletes it. It carries **no** explanatory comment: a comment naming a plan phase
would outlive the plan, and Phase 2's guard — which covers `bin/accelerator` — turns
a forgotten line into a lint failure, which is a better tripwire than a note.

Notes on the shape, each load-bearing:

- The scan sits after `set -uo pipefail` and before `fail()`'s first possible
  call, so `abort_status` is always defined under `set -u`. Resolving the policy
  once into a named status, rather than branching on a flag inside `fail()`, keeps
  the contract visible as data at the top of the file instead of implicit at 14
  call sites.
- **The scan stops at the first `--`** and at the first match. Scanning all of
  argv would let a token appearing as an *option value* — or after a `--`
  separator, or in a future sub-binary's arguments — silently switch every gate to
  exit 0 for an invocation the launcher would itself reject as malformed.
- `if [[ "$#" -gt 0 ]]` then `for arg in "$@"` is the repo's precedented empty-argv
  guard (`scripts/lint-bashisms.sh:23`,
  `skills/integrations/jira/scripts/jira-jql.sh:55`). The terser `${1+"$@"}` form
  appears nowhere in the ~175-file shell corpus and would need a justified
  `# shellcheck disable=` under `enable=all`.
- **Trust-chain gates call `fail_integrity` rather than `fail`.** A named second
  entry point, not a positional tag on `fail()`: `fail "…" integrity` reads as a
  trailing noun at the call site, a reader must diff argument lists across fourteen
  calls to tell the six tagged ones apart, and a typo (`integity`) would silently
  disable the record with no other symptom. With a distinct verb the intent is in
  the name and a misspelling is `command not found`. The six gates are `:72`, `:74`,
  `:163`, `:168`, `:170` and `:254`.
- They still degrade under `--fail-safe` — a `!` site must never discard the prompt
  — but they leave a durable line, reusing the mechanism and the rationale already
  at `:114-116` and `:136-141`. Nothing unverified is ever exec'd in either case, so
  what this buys is detectability, not containment: without it a bad signature is
  byte-identical to the ~86 commands that legitimately emit nothing.
- **The record is best-effort, and says so when it fails.** `${unverified_log}` is
  initialised empty before the functions and set once the root is known;
  `resolve_cache_dir` does not run until `:106`, so the shim and key gates fire
  before the cache directory is known to exist — hence the `mkdir -p` and the
  explicit "could not record" diagnostic in place of a bare `|| true`, so
  suppression is visible rather than silent. Reassign
  `unverified_log="${cache_dir}/.accelerator-unverified.log"` immediately after
  `:106` and repoint the dev-override write at `:141` to `${unverified_log}`, so the
  filename literal and the cache-dir precedence rule each appear once rather than
  being restated at the top of the file.
- **The record needs a reader**, or it is an audit trail nobody sees: Phase 3's
  hook reports when the log exists and is non-empty. Without that, a signature
  failure is still invisible to both the user and the model.
- **The record is sanitised once, at the write site.** A log whose records are one
  line each can be forged by any newline that reaches a message, so `$1` is
  collapsed with `${1//$'\n'/ }` before the append. `$'\n'` is the right idiom here:
  it is bash-3.2-safe (measured on 3.2.57), already used in this file at
  `bin/accelerator:48` (`version=${version%%$'\n'*}`), and banned by neither
  `scripts/lint-bashisms.sh` nor `.shellcheckrc`. The `${//}` replacement is also
  3.2-safe with a space as the replacement — this repo has a recorded gotcha about
  `${//}` when the *replacement* contains slashes, which is not the case here.

  Sanitising at the write site rather than validating inputs is deliberate.
  An earlier draft instead rejected a newline-bearing `plugin_root`, which was
  wrong twice over. The idiom it used —
  `case "${plugin_root}" in *"$(printf '\n')"*)` — **cannot work**: command
  substitution strips trailing newlines, so `$(printf '\n')` is the empty string and
  the pattern degenerates to `**`, matching every root and aborting the bootstrap on
  every invocation (measured). And even written correctly it would have been
  incomplete: `plugin_root` is not the only value reaching a record. The gate at
  `:168` interpolates `${cache_dir}`, which derives from the caller-supplied and
  otherwise unvalidated `ACCELERATOR_CACHE_DIR`, so the forgery path would have
  stayed open. One sanitiser at the single write site covers every input, present and
  future; no separate `plugin_root` newline check is needed, and none is added.
- `dir_of` replaces `dirname`: pure parameter expansion, no subprocess, and — with
  `cd --` — immune to a target beginning with `-`. That matters because `dirname`
  treats such an argument as an option, errors, and prints nothing, whereupon
  `cd ""` **succeeds as a no-op in bash** (measured) and `pwd -P` yields the
  *current working directory* — which for a skill-invoked bootstrap is the user's
  project. The `${dir:-/}` arm is why `dir_of` is not a regression at the
  filesystem root: `${1%/*}` yields the empty string for `/accelerator`, where
  `dirname` returns `/`, and feeding that empty value to `cd` is the very hazard
  being closed.
- **`cd -P`, not bare `cd`, for the `..` step.** `cd` defaults to *logical* mode,
  which collapses `..` textually before `chdir`, so when the final component is a
  **directory** symlink the result is the link's parent rather than the target's.
  Measured: with `other/bin -> real/bin`, invoking `other/bin/accelerator` gives
  `cd -- "…/other/bin/.."` → `…/other` (no `plugin.json`), while
  `cd -P -- "…/other/bin/.."` → `…/real` (correct). A prepared tree carrying its own
  `plugin.json` and `keys/` in the symlink's parent would otherwise become the
  trust anchor with the genuine bootstrap running — the one shape the key-pinning
  decision's privilege-gain argument does not cover, since it needs no planted
  executable. `-P` is a no-op in the ordinary case, because the chase has already
  resolved the *file* symlinks by then.
- No absoluteness check on `plugin_root`: `pwd -P` always prints a non-empty
  absolute path and `CDPATH=` removes the only route to extra lines, so such a
  check would be unreachable. The reachable failures are inside the loop, and each
  now has its own `|| fail`.
- `CDPATH=` prevents `cd` searching `CDPATH` and, on a match, *printing* the
  resolved directory to stdout — inside the command substitution, which would make
  `plugin_root` two lines.
- Every `readlink` and `cd` failure is now a named abort. Under `set -uo pipefail`
  without `-e` an unchecked failure would leave an empty value and continue,
  resolving a root one or two levels off.
- `readlink` without `-f`, plus `cd -P`, is identical on BSD and GNU; a
  capability probe and a `perl` fallback were both litigated and rejected.
- **The hop bound is 16, not 32.** See *Key Discoveries*: a bound at or above
  either kernel's `SYMLOOP_MAX` is unreachable, so it can neither be tested nor
  detect a cycle. 16 is below both, generous against a documented two-hop chain,
  and testable with a 17-link non-cyclic chain on both platforms.
- The loop, not a single dereference, is required: the Phase 3 chain is two hops
  deep, and one dereference lands on the `${CLAUDE_PLUGIN_DATA}` link.

The remaining **8** `${CLAUDE_PLUGIN_ROOT}` reads become `${plugin_root}` —
`plugin_json` (`:44`), `shim_source` (`:70`), `public_key` (`:73`), the cache
primary and its error text (`:101`, `:107`), `dev_launcher_marker` (`:117`),
`dev_launcher_contained`'s target root (`:122`), and the refusal message
(`:135`). (There are 11 reads in the file; three are inside the two gates being
deleted.) The comments at `:93` and `:260-261` are updated to name the new
variable. The header's "Fail-closed throughout" (`:7`) becomes "Fail-closed
unless `--fail-safe` is present, in which case bootstrap-layer aborts exit 0 with
a stderr diagnostic; trust-chain aborts additionally record durably", and
`--fail-safe` is documented there as a reserved global token. The bootstrap
**never reads** either root variable — one writer per invocation, so no ambient
value can redirect it.

The literal `keys/accelerator-release.pub` must survive, being textually pinned by
`tests/unit/tasks/test_bootstrap_coverage.py:39-43`. (An earlier draft also
claimed `.accelerator-dev-launcher` was pinned there; it is not — that literal is
pinned only by `test_accelerator_entrypoint.py`.)

#### 3. Pin the export against the readers

**File**: `tests/unit/tasks/test_bootstrap_coverage.py`
**Changes**: The file already pins shared literals across the trust boundary
(`test_launcher_and_bootstrap_reference_the_same_committed_key`). Add a companion
asserting that the name the bootstrap `export`s is the name the launcher and
`cache_root` read via `var_os`. A one-sided rename then fails in seconds as a unit
test rather than surfacing as a missing sentinel deep in an integration suite —
which is the guard the lockstep hazard otherwise lacks. During 1a it asserts both
names; 1b narrows it to one.

### Success Criteria

#### Automated Verification

- [x] Entrypoint suite passes: `uv run pytest tests/integration/entrypoint -v`
      *(46 passed)*
- [x] The two deleted tests are gone:
      `! grep -q 'test_unset_plugin_root_is_a_named_error' tests/integration/entrypoint/test_accelerator_entrypoint.py`
- [x] Build-system unit tests pass: `mise run test:unit:tasks` *(341 passed)*
- [x] Bash 3.2 floor holds: `mise run lint:scripts:check` — noting this proves
      only the enumerated bash-4 denylist, since `scripts/lint-bashisms.sh` is
      self-documented KNOWN-INCOMPLETE and bans none of the constructs introduced
      here
- [x] The bootstrap under test runs on the floor interpreter: the entrypoint
      harness invokes `/bin/bash` explicitly (or asserts `BASH_VERSINFO` is 3 on
      Darwin), so a contributor with Homebrew bash 5 cannot run the suite
      off-floor and green. **Both**: `_BASH` pins `/bin/bash`, and
      `test_the_suite_runs_the_bootstrap_on_the_bash_floor` asserts it is major
      version 3 on Darwin. The suite had been running on Homebrew bash 5.3.
- [x] The suite writes nothing into `bin/`: the session-scoped guard reports no
      new cached-launcher, staged-shim, lock or unverified-log entries
- [x] The committed vendored shims are not gitignored:
      `uv run pytest tests/unit/tasks/test_bootstrap_coverage.py -v`
- [x] `mise run check` exits 0
- [x] `mise run` (bare default) exits 0 end-to-end.

      Reached only after fixing three pre-existing flakes this phase surfaced
      but did not cause. Three earlier runs each failed exactly one *different*
      wall-clock assertion — `test:unit:frontend` (`Test timed out in 5000ms`,
      4–10 tests, varying set), `test:integration:config` (Playwright daemon:
      `Target page, context or browser has been closed`) and
      `test:integration:visualiser` (`scan took 5.390830542s, expected < 5 s`)
      — while each passed in the other two runs and in isolation.

      The Playwright one was a genuine daemon defect: `shutdown()` closed the
      browser before removing `server-info.json` and `server.pid`, leaving a
      window in which `run.sh`'s reuse check still passed and the next launcher
      dispatched onto a dead page. The other two were budgets sitting too close
      to the noise floor. Fixed on their own commit, outside this plan's scope.

#### Manual Verification

- [ ] Rootless invocation by absolute path succeeds:
      `env -u CLAUDE_PLUGIN_ROOT -u ACCELERATOR_PLUGIN_ROOT ./bin/accelerator config templates list`
      exits 0 and lists an `adr` row with Source `plugin default`. **Manual, with a
      precondition**: it fetches the launcher for the version in
      `.claude-plugin/plugin.json`, so it needs that version published — during 1a's
      development it 404s. It passes once released *because* of the transitional
      export, since the pre-rename launcher reads the old name. The hermetic
      equivalent, and the actual CI gate, is the entrypoint suite.
      **Deferred to the release candidate**: `1.24.0-pre.16` is the version in
      `plugin.json` and its assets are not published, so running this now would
      404 against the real GitHub release *and* write into the shipped `bin/`.

- [x] The Phase 1a regression tests fail when run against the pre-change
      `bin/accelerator` (stash the bootstrap change, confirm red, restore).
      **Confirmed at the intermediate commit**: 18 of the new cases red, every
      one with `accelerator: CLAUDE_PLUGIN_ROOT is not set`. The three that pass
      pre-change are the ones the plan predicts — the cycle case (characterises
      the kernel's ELOOP) and the two scan-window cases (a gate fires either way).
The remaining four were promoted to automated cases rather than performed by
hand — each is hermetic and cheap, so leaving it manual would have meant a
one-off check that never runs again:

- [x] A 17-link non-cyclic chain aborts with `exceeded 16 hops`; `ln -sf a b;
      ln -sf b a` terminates non-zero without hanging —
      `test_a_seventeen_link_chain_exceeds_the_hop_bound`,
      `test_a_symlink_cycle_terminates_rather_than_hanging`, plus
      `test_a_sixteen_link_chain_resolves` as the at-boundary partner
- [x] Invoking the bootstrap through a hand-made `PATH` symlink in a scratch
      directory resolves the real installation root —
      `test_two_hop_symlink_chain_resolves_to_the_fixture_root`
- [x] With `CDPATH` exported to a directory containing a `bin`, invocation still
      resolves the real installation root —
      `test_an_exported_cdpath_does_not_redirect_the_resolved_root`, which
      invokes relatively because `cd` consults `CDPATH` only for a relative path
- [x] A missing verify shim under `--fail-safe` exits 0, emits nothing on stdout,
      and appends one line to `.accelerator-unverified.log` —
      `test_trust_chain_failure_records_durably_under_fail_safe`

---

## Phase 1b: The Rename

> **Done 2026-07-29**, after Phase 1c as sequenced. 1c is green today and
> independent, but the rename makes its seam vacuous if it goes first — see the
> mergeability table in *Implementation Approach* and Phase 1c's Overview.

### Overview

Move the launcher and server layers onto `ACCELERATOR_PLUGIN_ROOT`, then drop
Phase 1a's transitional `CLAUDE_PLUGIN_ROOT` export. A pure refactor: no
behaviour changes, which is what makes it reviewable independently of the shell
work.

Readers and writers move together — renaming the `cli/` readers alone breaks the
dev harness and the shell suites, so `mise run` would go red between them.

### Changes Required

#### 1. The launcher and server readers

**Files**:
- `cli/launcher/src/main.rs:173,176` — the doc comment and `var_os`. An empty
  value must not become `Some("")`, as the filter at `cache_root.rs:27` already
  ensures for its own read. *Landed one layer down instead:
  `FileConfigStore::with_plugin_root` (`cli/config-adapters/src/store.rs:65-69`)
  drops an empty value, so the launcher and the visualiser server inherit the
  rule from one place rather than each carrying a filter — `main.rs`'s `var_os`
  is deliberately left unfiltered and its doc comment says so. Recorded in Phase
  5's implementation notes, which also covers the server half.*
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs:3,26,38,60` — module
  doc, `var_os`, error doc, and the `CacheRootUnavailable` detail text.
- `cli/visualiser/server/src/main.rs:69,70` — the let-else and its `eprintln!`.

#### 2. The `cli/` test and tooling readers

**Files**: `cli/launcher/tests/config_read.rs:62,1169,1205`;
`cli/launcher/tests/version.rs:22,179`;
`cli/launcher/src/launch/outbound/resolve/cache_root.rs:132,134`;
`cli/visualiser/server/tests/api_smoke.rs:32`, `shutdown.rs:19`,
`orchestration_lifecycle.rs:55,274`;
`cli/visualiser/frontend/e2e/start-server.mjs:98`;
`cli/corpus-adapters/tests/parity.rs:85` (comment).

`version.rs:179` and `cache_root.rs:132,134` assert on the message text, so they
are the observable-contract sites that must move with the rename.

A new case covers the empty-string branch the filter above adds:
`ACCELERATOR_PLUGIN_ROOT=""` must behave identically to unset, parametrised
alongside the existing `env_remove` case in `config_read.rs`. Without it the
behaviour change ships unasserted, and after Phase 5 the difference is a named
error versus a `PathBuf::from("")` resolving templates relative to the cwd.

**Doc comment only**: `cli/config-adapters/src/store.rs:20` is reworded to state
that `ACCELERATOR_MIGRATION_MODE` is *received* from the shell migration runner
(`skills/config/migrate/scripts/run-migrations.sh:643`) and deliberately ignored.
The assignment at `cli/config-adapters/tests/config_reader.rs:34` **stays** — see
*What We're NOT Doing*.

#### 3. The out-of-tree writers

**Files**:
- `tasks/dev.py:48` (and the comment at `:45`) — the dev server's environment.
  This feeds the hard-exit reader; without it `mise run dev:*` dies with exit 2.
- `tasks/shared/dev/circus.py:43` — doc comment only; the mechanism is
  `copy_env`.
- `tasks/test/helpers.py:36` and its docstring (`:18,:25,:29`) — the shell-suite
  overlay.
- `tests/integration/dev/dev_integration_driver.py:138` — a production-parity
  mirror of `tasks/dev.py:48`. Its value reaches the fake server via `copy_env`
  but `_SERVER_TEMPLATE` (`:45-63`) discovers its root from `os.getcwd()` and
  never reads it, so this is renamed for fidelity, not function.

#### 4. Drop the transitional export

**File**: `bin/accelerator`
**Changes**: Remove the `export CLAUDE_PLUGIN_ROOT` line added in 1a, and narrow
the `test_bootstrap_coverage.py` companion assertion to the single name.

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**: `test_ambient_roots_never_redirect_the_resolved_root` keeps naming
`CLAUDE_PLUGIN_ROOT` — it must, to prove the old name is inert. This is why that
file is on Phase 2's permitted-residue list.

### Success Criteria

#### Automated Verification

- [x] `cli/` workspace tests pass: `mise run test:unit:cli`
- [x] The empty-string case is asserted:
      `cargo test -p accelerator --test config_read empty_plugin_root` — note
      the package is `accelerator`, not `accelerator-launcher`. The case seeds a
      `templates/decoy.md` in the cwd, so an unfiltered empty value (which
      becomes `PathBuf::from("")`) is *observable* rather than merely equal:
      confirmed red without the filter, listing `decoy` as a plugin default.
- [x] Entrypoint suite still passes: `uv run pytest tests/integration/entrypoint -v`
      *(46 passed)*
- [x] Work-item shell suites pass: `mise run test:integration:work`
- [x] Dev-task integration tests pass: `mise run test:integration:dev` *(17 passed)*
- [x] Visualiser integration tests pass: `mise run test:integration:visualiser`
- [x] Build-system unit tests pass: `mise run test:unit:tasks` *(368 passed)*
- [x] No `CLAUDE_` string remains in `bin/accelerator`:
      `! grep -q 'CLAUDE_' bin/accelerator`
- [x] `mise run check` exits 0
- [x] `mise run` (bare default) exits 0 end-to-end

#### Manual Verification

- [x] `mise run dev:*` still starts the visualiser server (the renamed
      `tasks/dev.py` writer feeds its hard-exit reader). **Confirmed**:
      `mise run dev` then `dev:status` reports both server and frontend active.
      `mise.local.toml` sets only `CLAUDE_PLUGIN_ROOT`, so the server's root can
      only have come from the renamed writer — had the rename been one-sided the
      let-else would have exited 2.
- [x] A freshly built launcher, invoked directly with only
      `ACCELERATOR_PLUGIN_ROOT` set, renders the plugin-default template tier.
      **Confirmed** under `env -i`: 13 plugin-default rows with the variable set,
      a header-only table without it.

---

## Phase 1c: The Work-Item Seam and the Build Edge

> **Done 2026-07-29.**

### Overview

Two changes that are green **today**, independent of 1a and 1b: the seam re-point
uses an override the script under test already honours, and the build-edge
assertion documents an existing prerequisite.

**This must land before Phase 1b.** It is green in isolation at any time, but 1b
renames `accelerator_env()` (`tasks/test/helpers.py:36`) to export
`ACCELERATOR_PLUGIN_ROOT`, after which the seam's injected
`CLAUDE_PLUGIN_ROOT="/nonexistent"` is ignored by both the self-locating bootstrap
and the renamed launcher. The CLI then succeeds and returns the shipping
`templates/work-item.md`, whose trailing comments are byte-identical to
`hardcoded_fallback`'s values — so the four assertions pass without entering the
fallback, and Test 8's three tripwire comparisons compare the shipping template
with itself. Seven assertions would be green and testing nothing, in a window whose
exit criterion (`mise run test:integration:work`) is one of 1b's own.

### Changes Required

#### 1. The work-item seam

**File**: `skills/work/scripts/test-work-item-scripts.sh`
**Changes**: Re-point the four seam sites onto the script's own documented
override. Note they are not uniform: `:1053` injects
`CLAUDE_PLUGIN_ROOT="/nonexistent/plugin"` and runs from `$REPO`, while `:1086`,
`:1096` and `:1106` inject `CLAUDE_PLUGIN_ROOT="/nonexistent"` and run from `/tmp`.
All four become:

```bash
ACCELERATOR_BIN="/nonexistent/accelerator"
```

This states the seam's actual intent — make the CLI call fail — and is both
bootstrap- and launcher-independent. It is the established convention: 28 files
reach the CLI through `"${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}"`
(`work-item-template-field-hints.sh:53` among them), and the test overlay sets
`ACCELERATOR_BIN` precisely so suites never traverse the fetch/verify chain.
Update Test 6's comment, which describes the old mechanism.

The four assertions expect exactly the values `templates/work-item.md:8-10`
carries, so they pass **vacuously** if the CLI ever succeeds. A new test makes
the fallback branch's exercise provable without a mutation:

```bash
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator/templates"
cat >"$REPO/.accelerator/templates/work-item.md" <<'FIXTURE'
---
status: draft  # alpha | beta | gamma
---
FIXTURE

OUTPUT=$(cd "$REPO" && ACCELERATOR_BIN="/nonexistent/accelerator" \
  bash "$FIELD_HINTS" status 2>/dev/null) || true
EXPECTED=$(printf "draft\nready\nin-progress\nreview\ndone\nblocked\nabandoned")
assert_eq "seam forces the hardcoded fallback" "$EXPECTED" "$OUTPUT"

OUTPUT=$(cd "$REPO" && bash "$FIELD_HINTS" status)
EXPECTED=$(printf "alpha\nbeta\ngamma")
assert_eq "a working CLI reads the override" "$EXPECTED" "$OUTPUT"
```

The pair is falsifiable in both directions: if the seam stopped forcing the
fallback, the first assertion would see `alpha beta gamma`. Note the positive half
duplicates existing Test 5 (`:1043-1045`); its value here is the explicit pairing
with the negative, not new coverage.

#### 2. The build edge

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: No test asserts that any `test:*` task depends on `build:cli:dev`,
so a new task can silently ship without the prerequisite. Add a companion to
`_CHECK_GATES`, pinned **exhaustively** so adding an integration task forces a
decision rather than defaulting to silence:

```python
_LAUNCHER_DEPENDENTS = [
    "test:integration:config",
    "test:integration:decisions",
    "test:integration:migrate",
    "test:integration:work",
    "test:integration:integrations",
    "test:integration:visualiser",
]

# Integration tasks that deliberately need no prebuilt launcher, each with a
# reason. Every test:integration:* task must appear in exactly one of the two.
_NO_LAUNCHER_NEEDED = {
    "test:integration:entrypoint": "builds it in-fixture, mirroring shim_bin",
    ...
}
```

`test:integration:entrypoint` is deliberately in the second set: Phase 1a builds
the launcher in-fixture so the suite runs standalone, and adding the edge as well
would contend on cargo's target lock while making the assertion inert.

CI needs no edit: `test-unit` and `test-integration` already carry the
`RUSTUP_HOME` routing step and `workspaces: cli` cargo caching
(`.github/workflows/main.yml:70-71,81-88`).

### Success Criteria

#### Automated Verification

- [x] Work-item shell suites pass: `mise run test:integration:work` *(242 passed
      in the work-item suite; 0 failed across the subtree)*
- [x] The exhaustive edge assertion passes:
      `uv run pytest tests/unit/tasks/test_mise.py -v` *(18 passed)*
- [x] `mise run check` exits 0

#### Manual Verification

- [x] Temporarily removing `hardcoded_fallback`'s `status)` arm turns the
      re-pointed seam assertions red. (It also reddens Test 7 at `:1071` and both
      tripwire comparisons, so the mutation is a sanity check on the pair rather
      than an isolating one.) **Confirmed**: 4 failures, including both
      `returns hardcoded status values` and `seam forces the hardcoded fallback`,
      while `a working CLI reads the override` stayed green — the pair is
      falsifiable in both directions as designed.
- [x] Adding a `test:integration:*` task to neither `_LAUNCHER_DEPENDENTS` nor
      `_NO_LAUNCHER_NEEDED` turns `test_mise.py` red. **Confirmed**: a temporary
      `test:integration:probe` leaf fails
      `test_every_integration_task_declares_its_launcher_need` with
      `Extra items in the left set: 'test:integration:probe'`.

---

## Phase 2: The `CLAUDE_*` Boundary Guard

> **Done 2026-07-29.** Three commits, as sequenced.

### Overview

Make the boundary rule non-negotiable rather than conventional: **nothing in the
rename set** may name, read, or require a `CLAUDE_*` variable. Must follow Phase
1b or it fails on landing.

The rename set — not `cli/` — is the right scope. The entry point that violated
the invariant is `bin/accelerator`: a shell script outside `cli/` with no
extension, which a `cli/`-scoped guard cannot see. It is also where Phase 1a's
transitional export lives, so including it makes a forgotten export a lint
failure. The four out-of-tree writers are in scope for the same reason: they feed
the `cli/` readers, and a reintroduced write is as much a coupling as a read.

| In scope | Why |
|---|---|
| `cli/**` | the launcher and server layers |
| `bin/accelerator` | the entry point the bug was in; carries 1a's transitional export |
| `tasks/dev.py`, `tasks/shared/dev/circus.py`, `tasks/test/helpers.py`, `tests/integration/dev/dev_integration_driver.py` | the writers that feed those readers |

Everything else is out of scope by construction rather than by exemption — the
adapter layer (`hooks/`, `skills/config/migrate/**`,
`scripts/interactive-harness.sh`, `scripts/test-design.sh`), the matcher model in
`tasks/lint/skill_permissions.py`, and the SKILL.md substitution tokens all
legitimately keep the variable, and none is in the rename set. That is why the guard ships with **no allowlist**: an empty
`frozenset()` with "per-entry reasons" is YAGNI, and a failure message advertising
a whole-path override invites exactly the wrong escape hatch — the first exemption
added under pressure could be `cli/launcher/src/main.rs`, blinding the guard to
the site that caused this bug. The message says the reference must be *removed*.

### Changes Required

#### 1. Promote the gitignore-honouring walk

**File**: `tasks/shared/sources.py`
**Changes**: Promote the **walk**, not the policy. `_ignore_spec()` (`:40-48`) is
already the reusable primitive but the prune loop has been reimplemented once for
`.py` (`tests/unit/tasks/test_python_coverage.py:68-94`); this would be the third
copy. Suffix filtering, subtree scoping and per-caller keep rules stay in the
callers, so one function does not become the single point of change for three
different discovery policies.

```python
from collections.abc import Iterator

_BUILD_OUTPUT = ("dist", ".venv", "node_modules", "playwright-report", "coverage")


def walk_files(
    repo: Path,
    subtree: str | None = None,
    prune: tuple[str, ...] = _BUILD_OUTPUT,
) -> Iterator[str]:
    """Repo-relative paths, gitignore-honouring, pruning ignored dirs in place.

    The ignore spec is always read from ``repo`` and matched repo-relative,
    whatever ``subtree`` scopes the walk to — the root ``.gitignore``'s entries
    are root-anchored (``cli/target/``), so reading a spec at a subtree would
    silently match nothing.

    ``prune`` defaults to build output that the root ``.gitignore`` does not
    cover: ``dist`` and ``playwright-report`` are ignored only by
    ``cli/visualiser/frontend/.gitignore``, and ``.venv`` is ignored nowhere.
    """
```

Three details are load-bearing:

- **The spec is read from the repo root, never from the subtree.** `_ignore_spec`
  (`sources.py:40-48`) reads `<root>/.gitignore` and matches paths relative to it,
  and the entries that matter are root-anchored (`.gitignore:20-21` are
  `cli/target/` and `cli/.pup/`). A call like `walk_files(repo / "cli")` would test
  `target/` against `cli/target/`, match nothing, and descend into the whole Rust
  build tree — exactly the failure the *Performance Considerations* section warns
  about. Hence `repo` plus an explicit `subtree`, rather than one conflated `root`.
- **`prune` defaults to `_BUILD_OUTPUT` in the signature**, not to `()` with a
  hidden additive union. *Implemented with replace semantics, which is what the
  signature default means — the plan's later test-case bullet said "adds to
  rather than replaces", contradicting this paragraph's own rationale. A caller
  wanting both writes `_BUILD_OUTPUT + (...)`, and the test asserts that.* A caller can then see what it gets and opt out. Without
  `dist`/`playwright-report` the guard reads the minified SPA bundle and Playwright
  traces — both present under bare `mise run`, which builds the frontend and runs
  E2E — so the result would depend on what had been run locally, and a vendored
  string in a bundle would be an unfixable false positive.
- **`.venv` pruning is a behaviour change for `shell_sources()`**, which today
  prunes only gitignored directories. Capture `shell_sources()`'s current output as
  an equality assertion *before* the refactor so the change is a visible test diff
  rather than an inference.

`shell_sources()` is rewritten to call it, keeping `_keep`'s `workspaces/`
exclusion, the `.sh` filter and the `_EXTRA_SHELL_SOURCES` append — and note the
extras are gated on `not spec.match_file(s)` (`sources.py:96`), so either
`walk_files` exposes the spec or `shell_sources` builds its own for that check;
state which. `test_python_coverage.py`'s `_py_files` is repointed at it, keeping its
`set[str]` return. Both callers' return shapes are preserved; only the traversal is
shared.

#### 2. The guard

**File**: `tasks/lint/claude_coupling.py` (new)
**Changes**: Modelled on `tasks/lint/store_duplication.py` — a pure
`violations(root: Path) -> list[str]` (root injected, which is what makes it
testable) and a thin `@task check` raising `Exit(..., code=1)` whose message names
the guard file and says the reference must be removed.

```python
_SHAPE = re.compile(r"CLAUDE_[A-Z0-9_]*")

# Speed only; the correctness rule is the decode below.
_SKIP_SUFFIXES = (".png", ".ico", ".woff2", ".lock", ".snap", ".bin")

_SUBTREES = ("cli",)
_FILES = (
    "bin/accelerator",
    "tasks/dev.py",
    "tasks/shared/dev/circus.py",
    "tasks/test/helpers.py",
    "tests/integration/dev/dev_integration_driver.py",
)


def violations(root: Path) -> list[str]:
    """Repo-relative ``path:line:text`` for every CLAUDE_* reference in scope."""
```

Reported as `path:line:text` per `tasks/lint/call_site_migration.py:26-93` — the
reader needs to see *which* variable was found.

**Scan by default; skip on a decode failure, not on a suffix.** The rule is "in
scope unless it is unreadable as text", so that must be the *mechanism*: read with
`errors="strict"` and skip on `UnicodeDecodeError`. `_SKIP_SUFFIXES` is then a
speed optimisation rather than the correctness boundary — otherwise a `.ttf`,
`.wasm` or `.jpg` landing anywhere under `cli/` turns `mise run cli:check` into a
traceback, and the fix under time pressure is to append suffixes, which is the
drift the no-allowlist decision exists to prevent. An allowlist of *included*
suffixes is worse still: it silently exempts whatever nobody thought of, and `cli/`
already holds `.md`, `.html` and ~45 `.module.css` files.

**Fail closed on empty *discovery*, not on empty violations.** The convention at
`tasks/lint/scripts.py:8-11` (`_EMPTY_SCOPE = "no shell sources matched — scope
discovery is broken"`) is about the file set, not the finding set; read the other
way the guard would invert and could never pass on a clean tree, which is its
whole purpose. So: raise when the scanned-file set is empty, and additionally
assert a floor on its size, because a silently-emptied scan otherwise reads as
cleanliness in both the lint task *and* `violations(REPO_ROOT) == []`.

**Apply the same fail-closed rule to `_FILES`.** Each entry is a literal path with
no discovery behind it, so a rename, move or split — `tasks/test/helpers.py` is
exactly the kind of module that gets split — would silently drop it from scope while
`violations(REPO_ROOT) == []` still passed and the file-count floor stayed
satisfied (it is dominated by thousands of `cli/**` files). `violations()` therefore
raises when any `_FILES` entry does not resolve to a file under `root`. That matters
most for `bin/accelerator`: it is both the file the bug was in and the one carrying
1a's transitional export, so it is the entry whose silent loss would cost the most.

The module docstring states the invariant: **no plugin entry point may require
`CLAUDE_PLUGIN_ROOT` from its process environment** — what this bug violated — and
names the adapter layer as out-of-scope-by-construction so a reader does not take
the rule as unqualified. It is a separate guard from `skill_permissions.py`
because that module deliberately *models* the Claude Code matcher
(`_PLUGIN_PREFIX` at `:55`, `_BARE_LAUNCHER` at `:43`) and must keep naming the
variable.

#### 3. Registration

**File**: `tasks/lint/__init__.py`
**Changes**: Add `claude_coupling` to the import tuple and `__all__`, both of
which are kept alphabetical — it sorts between `call_site_migration` and `cli`.

**File**: `tasks/__init__.py`
**Changes**: `ns_lint.add_collection(Collection.from_module(lint.claude_coupling))`
— underscores become hyphens automatically. Every neighbouring
`add_collection` call (`:84-113`) carries a trailing comment naming the resulting
task path; match it (`# lint.claude-coupling.check`).

**File**: `mise.toml`
**Changes**: A leaf modelled on `lint:store-duplication:check` (`:392-395`) with
the mandatory `depends = ["deps:install:python"]`, added to `cli:check.depends`
(`:409`) **only**. Not `lint:check.depends` (`:451`): neither
`lint:store-duplication:check` nor `lint:vendor-shims:check` is there, because
`cli/`-scoped guards ride in `cli:check` — which is what CI actually runs
(`.github/workflows/main.yml:257`). Dual-wiring would make this the sole
`cli/`-scoped guard reachable from the bare `default` task, establishing a third
pattern. (Closing that default-task blind spot for all three guards together is
worth doing, but as its own change.)

*Superseded 2026-07-29, at validation.* Validation measured what "as its own
change" cost in the meantime: the bare `default` task depends on `lint:check` and
never on `check`, so **none** of the three guards ran in a full local `mise run`
— a `CLAUDE_*` reintroduction in the rename set was green locally and caught only
by CI's separate `cli:check` step. All three are now in `lint:check.depends`
*and* `cli:check.depends`, together rather than one at a time, so no third
pattern is established. `test_mise.py`'s `_CLI_CHECK_GATES` pins both placements.

**File**: `tasks/README.md`
**Changes**: The per-component table describes `cli:check` as "format + lint
(rustfmt, workspace-wide clippy)", already omitting the two Python guards wired
into it. One sentence naming all three.

#### 4. The paired test

**File**: `tests/unit/tasks/test_claude_coupling.py` (new)
**Changes**: Follows `tests/unit/tasks/test_store_duplication.py` —
`REPO_ROOT = Path(__file__).resolve().parents[3]`, per-branch positive and
negative tests over `tmp_path`, and `violations(REPO_ROOT) == []` as the durable
enforcement. That last assertion, not the lint task, is what both existing
SKILL.md guards get their actual CI teeth from.

Every probe stays in `tmp_path`. A sentinel written into the live tree would make
the checks flake, because `test:unit:tasks` runs concurrently with `cli:check`
under `mise run` (`tests/unit/tasks/test_python_coverage.py:138-145`).

Cases: flags a `var_os` read in `cli/`; flags a comment mention; flags a `.mjs`
injection; **flags a read reintroduced into `bin/accelerator`** and into one
out-of-tree writer; flags a `.md` and a `.sh` under `cli/` (the scan-by-default
property); skips a file of undecodable bytes under an unlisted suffix (`cli/a.dat`)
without raising; does not flag `cli/target/`, `node_modules/`, `dist/` or
`playwright-report/`; does not flag `tasks/lint/skill_permissions.py` or `hooks/`
(outside the rename set); fails closed on an empty tree; fails closed when a
`_FILES` entry is missing; the scanned-file count on the real tree is above a floor;
the real tree is clean.

Note the pruning cases need a `.gitignore` written into `tmp_path` for the
gitignored half to mean anything — `_ignore_spec` reads only that file — while
`dist/` and `playwright-report/` are pruned unconditionally and so hold without one.
State which mechanism each case is exercising, or a case can pass for the wrong
reason.

**File**: `tests/unit/tasks/shared/test_sources.py`
**Changes**: The module-mirroring convention puts `walk_files`'s own cases here,
alongside the eleven existing `shell_sources`/`_keep` cases (which import `_keep`
privately, so a signature change must be reflected). New cases: a gitignored tree
is pruned; `workspaces/` handling is unchanged; each `_BUILD_OUTPUT` entry is
pruned — in particular no path under `cli/visualiser/frontend/dist/` is ever
yielded; `prune` adds to rather than replaces the defaults.

**File**: `tests/unit/tasks/test_python_coverage.py`
**Changes**: Add `assert not any(p.startswith(".venv/") for p in py)` to
`TestInScopeSet` **before** the refactor. No existing assertion would fail if the
`.venv` prune vanished — the suite checks specific inclusions and the absence of
`workspaces/` only.

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: `_CHECK_GATES` asserts only that `cli:check`/`deny:check`/`pup:check`
appear in `check.depends`, so nothing pins the new leaf's placement — the plan's
previous "the gate is reachable from `cli:check`" criterion was vacuous. Add
`_CLI_CHECK_GATES`, mirroring `_CHECK_GATES`, asserting
`lint:claude-coupling:check` is in `cli:check.depends`.

### Success Criteria

#### Automated Verification

- [x] The guard reports nothing on the real tree:
      `mise run lint:claude-coupling:check` *(724 files scanned, 0 violations)*
- [x] The guard's own tests pass:
      `uv run pytest tests/unit/tasks/test_claude_coupling.py -v` *(16 passed)*
- [x] The promoted walk did not regress its callers:
      `uv run pytest tests/unit/tasks/shared/test_sources.py tests/unit/tasks/test_python_coverage.py tests/unit/tasks/test_exec_bits.py -v`
      *(42 passed)*. `shell_sources()`'s real-tree output is byte-identical
      before and after (192 entries either way) — the `.venv`/`dist` prune is
      latent on this tree, so it is pinned by a `tmp_path` case instead.
- [x] The leaf is pinned into `cli:check.depends`:
      `uv run pytest tests/unit/tasks/test_mise.py -v` *(21 passed)*
- [x] `mise run cli:check` exits 0
- [x] The purge holds over **non-`meta/`** tracked files —
      `grep -rl 'CLAUDE_PLUGIN_ROOT'` honouring `.gitignore` and excluding
      `meta/` returns only `hooks/**`, `scripts/interactive-harness.sh`,
      `scripts/test-design.sh`, `skills/config/migrate/**`, migrations
      `0001`–`0007`, `tests/unit/tasks/test_call_site_migration.py`,
      `tests/unit/tasks/test_skill_permissions.py`,
      `tests/integration/entrypoint/test_accelerator_entrypoint.py` (the
      ambient-root test must name the old variable to prove it is inert),
      `tasks/lint/skill_permissions.py`, `.shellcheckrc`, `CLAUDE.md`,
      `skills/**/SKILL.md`, `skills/work/create-work-item/evals/benchmark.json`
      (a recorded eval transcript quoting a command, not a coupling), plus the
      three files *this* work adds that must name what they forbid or strip:
      `tasks/lint/claude_coupling.py`, `tests/unit/tasks/test_claude_coupling.py`
      and `tests/integration/skill-invocation/test_skill_invocation_conformance.py`.
      `meta/` is excluded because it records history — including this plan —
      rather than coupling. Derive the list by running the grep rather than by
      enumeration, so it cannot drift from what the tree actually contains.

      **Measured at Phase 2: 86 files. Re-measured after Phases 3 and 4: 87,
      every one in the permitted set.** Three corrections to the list as
      written. The `hooks/**` entry covers four more files than it implies
      (`hooks.json`, the two bash harnesses and
      `hooks/test-fixtures/vcs-detect/regenerate.sh`). Phase 3 adds one file
      outside that prefix —
      `tests/integration/hooks/test_launcher_link_refresh.py`, which must name
      the variable in order to assert the hook does *not* — and that is the only
      addition between the two measurements. And of the "three files this work
      adds that must name what they forbid or strip", only **two** do:
      `tests/integration/skill-invocation/test_skill_invocation_conformance.py`
      never names the variable, because Phase 4 promoted `PLUGIN_PREFIX` to
      public and imports it rather than carrying a third literal copy.
- [x] `mise run check` exits 0

#### Manual Verification

- [x] Reintroducing a `CLAUDE_PLUGIN_ROOT` read into `cli/launcher/src/main.rs`
      turns `mise run cli:check` red with a `path:line:text` report naming the
      variable, and the failure message names the guard file. **Confirmed**:
      `cli/launcher/src/main.rs:177:std::env::var_os("CLAUDE_PLUGIN_ROOT")`.
- [x] Reintroducing Phase 1a's transitional `export CLAUDE_PLUGIN_ROOT` into
      `bin/accelerator` turns it red too — the property that makes the guard the
      backstop for 1b's cleanup. **Confirmed** in the same run:
      `bin/accelerator:353:export CLAUDE_PLUGIN_ROOT="${plugin_root}"`.
- [x] The guard's scan does not descend into `cli/target/`,
      `node_modules/` or `dist/` (observable as a sub-second run rather than a
      multi-second one, with or without a built frontend). **Confirmed**: 275ms
      against a built tree, 724 files scanned.

---

## Phase 3: Terminal Invocation Surface

> **Done 2026-07-29.** Three commits, as sequenced.

### Overview

A fixed-path, upgrade-surviving entry for terminal use, via a two-hop chain, plus
the documentation that makes it usable.

```
~/.local/bin/accelerator                     (user, once, documented)
  -> ${CLAUDE_PLUGIN_DATA}/bin/accelerator   (hook, every SessionStart)
    -> <root>/bin/accelerator                (version-pinned)
```

The split is what makes it stale-proof: the hop the user creates points at a
target that never moves, so it never needs re-running; the hop that must track
the version is owned by the hook.

**"Shim" is not used for this.** In `bin/accelerator` the word already denotes one
specific thing — the vendored per-triple minisign verifier (`shim_source`,
`shim_digest`, `bin/accelerator-verify-<platform>`, `tasks/lint/vendor_shims.py`,
`lint:vendor-shims:check`, the entrypoint suite's `shim_bin` fixture) — and Phase
1a uses it that way. Reusing it for a convenience symlink would mean a
"shim refresh" failure at `SessionStart` reads as the trust root breaking. The
hook is `hooks/launcher-link-refresh.sh`, matching the existing
`<subject>-<action>.sh` naming (`vcs-detect.sh`, `config-detect.sh`,
`migrate-discoverability.sh`), and the documentation says "link".

This reverses an earlier explicit decision of "no `${CLAUDE_PLUGIN_DATA}`
dependency"
(`meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md:125`),
acknowledged here rather than silently overridden. Worth noting the prior decision
was narrower than its wording suggests — it chose `~/.cache/accelerator/` for a
Playwright *binary cache*, on the grounds that availability across Claude Code
versions was uncertain, which is exactly what Phase 0 §2 now determines.

**Why this hook is shell rather than CLI logic.** ADR-0048 says hook logic belongs
in the CLI, fronted by a thin shell shim only where the hook entry point demands a
shell command — and this phase adds ~50 lines of non-trivial bash plus its own suite
and a CI floor, so the divergence needs recording rather than leaving as apparent
drift. The justification is circularity: the link exists to make the *launcher*
reachable, so maintaining it cannot depend on the launcher being fetched, verified
and cached. A CLI implementation would mean that on a cold cache the terminal link
is only repaired *after* a successful network fetch — precisely when the user is
most likely to be reaching for a terminal command that does not work — and it would
put a fetch on the `SessionStart` path. `hooks/config-detect.sh` shows the
alternative shape is viable in general (it does exec the bootstrap), which is why
this needs saying: the reason is specific to what this hook maintains, not a general
preference. If the launcher ever gains a guaranteed-present, no-network mode, this
should move.

### Changes Required

#### 1. The hook test suite (written first)

> **Superseded on the language, not the cases.** The suite shipped as
> `tests/integration/hooks/test_launcher_link_refresh.py`, not a
> `hooks/test-*.sh` harness: ADR-0048 makes Python the test language for the
> non-Rust surfaces, shell wrappers included, and the two bash harnesses still
> under `hooks/` predate that decision. Every *case* below is implemented; the
> bash-specific mechanics are not. See *Implementation notes*.

**File**: `hooks/test-launcher-link-refresh.sh` (new, `chmod 0755`, mode committed)
**Changes**: Copies the structure of `hooks/test-migrate-discoverability.sh` —
`set -euo pipefail`, `SCRIPT_DIR`/`PLUGIN_ROOT`/`HOOK`, sourcing
`scripts/test-helpers.sh`, `TMPDIR_BASE=$(mktemp -d)` with
`trap 'rm -rf "$TMPDIR_BASE"' EXIT`, a `run_hook()` wrapper applying the env
overlay per invocation, and a closing bare `test_summary`.

There is no symlink assertion helper, so link targets are asserted with
`assert_eq` over `readlink` output.

**Tool constraints**, stated because this is where GNU-only behaviour creeps into
a suite that runs on both CI legs: snapshots use
`find "$tree" -print | LC_ALL=C sort` compared with `diff -u`. No `find -printf`
and no `stat -c` (neither exists on macOS; BSD `stat` uses `-f`), no `readlink -f`
(a documented BSD no-op this repo already avoids), and `LC_ALL=C` because an unset
locale orders tree listings differently under UTF-8 and would surface as a
spurious one-platform diff. Capture `readlink` on stdout only (`2>/dev/null`):
GNU prints a diagnostic for a non-symlink argument where BSD is silent. Neither the
hook nor the suite may canonicalise the paths it is handed — on macOS `mktemp -d`
returns `/var/folders/…` while `cd … && pwd -P` returns `/private/var/folders/…`,
so an `assert_eq` over `readlink` output would fail on that leg alone.

**Mode assertions use `ls -ld`, not `stat`.** The tool constraints ban `stat`
(GNU `-c` versus BSD `-f`), and the permission-test operators (`[ -w ]`) answer a
caller-relative question that is all-true for uid 0 — so a mode assertion needs
`ls -ld "$dir" | cut -c1-10` compared against `drwx------`. Truncating at column 10
is deliberate: macOS appends `@` or `+` for xattrs and ACLs at column 11.

**`run_hook()` must scrub, capture separately, and return the status.**

- **The hook is copied into each fixture root, not pointed at one.** Because it
  self-locates unconditionally, `CLAUDE_PLUGIN_ROOT` is no longer a test seam — the
  root under test is wherever the *hook file* sits. So each case that needs a
  particular root does `mkdir -p <tmp>/v1/hooks <tmp>/v1/bin`,
  `cp "$HOOK" <tmp>/v1/hooks/`, `touch <tmp>/v1/bin/accelerator`, `chmod +x` it, and
  invokes the copy. That is more setup than exporting a variable, and it is better
  fidelity: it exercises the resolution production actually performs. The
  re-point case therefore uses two roots, `<tmp>/v1` and `<tmp>/v2`, each with its
  own copy of the hook.
- **Scrub, not merely omit**: `tasks/test/integration.py` runs the `hooks` subtree
  with no env overlay, so the suite inherits the ambient environment — and
  `mise.local.toml` exports `CLAUDE_PLUGIN_ROOT` into every local mise task until
  the closing step deletes it. Scrubbing matters even though the hook no longer
  reads that variable: a leaked `ACCELERATOR_CACHE_DIR` would redirect the
  unverified-log path, and a leaked `CLAUDE_PLUGIN_DATA` would defeat the inertness
  cases. So invoke via
  `env -u CLAUDE_PLUGIN_ROOT -u CLAUDE_PLUGIN_DATA -u CLAUDE_CONFIG_DIR -u ACCELERATOR_CACHE_DIR -u ACCELERATOR_BIN -u HOME`
  and set only what the case wants.
- **Capture separately**: `hooks/test-migrate-discoverability.sh:18` does
  `bash "$HOOK" 2>&1 || true`, which merges the streams and discards the status.
  This suite needs all three, because the channel a message arrives on is itself
  under test: `RC=0; env -u … bash "$HOOK" >"$out" 2>"$err" || RC=$?`.
  **Every** case asserts `RC == 0`. Every case also asserts the state of *both*
  streams — routine cases expect `stdout` empty with the `[accelerator]` line on
  `$err`; the two actionable cases expect a single JSON object on `stdout` (parsed
  with `jq -r '.systemMessage'`) and no `[accelerator]` line on `$err`. Asserting
  only the stream a case cares about would let a message silently move channels.

Cases. Each names the guard it exercises, because several plausible fixtures reach
a *different* guard and pass while asserting nothing:

- Hook copied into `<tmp>/v1`, `CLAUDE_PLUGIN_DATA=<tmp>/data` →
  `<tmp>/data/bin/accelerator` is a symlink to `<tmp>/v1/bin/accelerator`. The hook
  is **silent on a first successful refresh** — no previous link, so no re-point
  notice — so this case asserts the link plus empty stdout *and* empty stderr.
- Re-run from a second copy in `<tmp>/v2` → the link re-points; the **re-point is
  reported** naming both roots; and a pre-existing
  `<tmp>/userbin/accelerator → <tmp>/data/bin/accelerator` link — whose own target
  did not move — still executes. That last assertion is the one that pins the
  two-hop design's central claim.
- **The destination pre-exists as a regular file** → refused, exits 0, the file's
  **content is unchanged** (`assert_file_content_eq`, not mere existence), and the
  refusal arrives as a **`systemMessage` on stdout** rather than on stderr — this is
  one of the two actionable cases, so the channel is part of the assertion. Parse it
  with `jq -r '.systemMessage'` as `test-vcs-detect.sh:682` does.
- **The destination pre-exists as a symlink to a directory** → the final
  `readlink` is exactly `<root>/bin/accelerator`, **and** the pointed-to directory
  is empty. This is the trap `mv -f` reintroduces (measured), so it is the case that
  fails if the clear-before-rename step is dropped; the second assertion is the
  "what was *not* written" half.
- **`${CLAUDE_PLUGIN_DATA}/bin` pre-exists as a symlink to a directory** → refused
  by the `[ -L ]` guard, and the symlink's target is **not** created or modified.
  The flavour matters: a *dangling* symlink or a regular file reaches the `mkdir`
  guard instead and produces a different diagnostic, so state which is under test.
- **`${CLAUDE_PLUGIN_DATA}/bin` pre-exists as a regular file** → refused by the
  second clause of the same guard. Without this case that clause has no coverage.
- **`${CLAUDE_PLUGIN_DATA}/bin` pre-exists at mode `0700`** → mode untouched,
  asserted via `ls -ld | cut -c1-10`.
- **`${CLAUDE_PLUGIN_DATA}/bin` is unwritable** (`chmod 0555` on it, not on the
  parent) → `mkdir -p` succeeds on the existing directory, the guards pass, and
  `ln` fails: the hook reports, exits 0, and leaves no `accelerator.new.*` entry.
  This replaces an earlier `${dest}.new.<pid>`-pre-exists-as-a-directory case, which
  was unbuildable (`$$` is the hook child's pid, unknowable in advance) *and* wrong
  (measured: `ln -sfn` onto a real directory **succeeds**, linking inside it). Skip
  under uid 0 with `skip_test` — mode bits are advisory for root, so the case
  inverts in a root container — and restore the mode before the suite's cleanup trap
  runs, or `rm -rf "$TMPDIR_BASE"` cannot remove the tree.
- **The staging path pre-exists** → seed `${LINK}.new.<known-pid>` by invoking the
  hook through a wrapper that `exec`s it, so the child inherits a pid the fixture
  chose; assert the "stale staging path" refusal. If that proves awkward, record the
  pre-check as covered by inspection rather than by test rather than writing a case
  that passes for the wrong reason.
- **The launcher is missing or not executable** (hook copied into a root whose
  `bin/accelerator` was removed, or left non-`+x`) → refused by `-x`, no link
  created, on **stderr**. This is the state after an upgrade removes the old
  directory, and it is the only launcher-validation case that survives
  self-location: there is no relative-root or non-absolute-root case, because the
  hook never reads a root from the environment and `pwd -P` cannot yield a relative
  path.
- **`CLAUDE_PLUGIN_DATA` unset** → exits 0, creates nothing, one `[accelerator]`
  stderr line. The assertion is on **the hook's decision** (that line), not on the
  absence of a `/bin` entry: `/` is unwritable on both CI legs and SIP-protected on
  macOS, so an absence-of-`/bin` assertion passes whether or not the guard exists,
  and the one environment where it matters — Claude Code as root in a container — is
  unreachable from the test.
- **`CLAUDE_PLUGIN_DATA` is relative** (e.g. `./data`) → inert, and the cwd is
  untouched. Composing against a relative value would put the link inside the user's
  project directory, which the absolute-path snapshot case cannot catch.
- **The unverified log is non-empty** → a `systemMessage` on stdout naming the log
  path (the second actionable case); **and separately absent or zero-length** → no
  such message, and stdout empty. Seeded at the path the hook derives
  (`${ACCELERATOR_CACHE_DIR:-<root>/bin}/…`), inside the fixture root so a stray log
  from a local bootstrap run in the real repo cannot leak in. These two cases are
  what would have caught the earlier path mismatch.
- **The log is non-empty *and* the destination is a regular file** → **exactly one**
  JSON object on stdout carrying both notices, not two. This is the case that pins
  the `NOTICE` accumulator; two `systemMessage` objects would be invalid output.
- **`jq` is absent** (shadow it on `PATH`) → the notices degrade to stderr and the
  hook still exits 0, mirroring `vcs-detect.sh`'s own jq guard.
- With `HOME` and `CLAUDE_PLUGIN_DATA` pointed into a temp tree, snapshot before and
  after and diff. The snapshot must carry **one line per entry plus its link
  target** — `find "$tree" -print | LC_ALL=C sort` then, per path,
  `printf '%s|%s\n' "$p" "$(readlink "$p" 2>/dev/null)"` — because a name-only
  listing cannot see a re-pointed symlink or a rewritten file, which is exactly what
  "created **or modified**" claims to cover. Root the snapshot at the tree
  containing both `HOME` and `CLAUDE_PLUGIN_DATA`, and include the case's cwd, so
  `<HOME>/.local/bin` absence and cwd-untouched are asserted by the same mechanism.
- **Registration is asserted here, not only in the checklist.** Select the
  `SessionStart` group by command content, then assert the **full literal**
  `${CLAUDE_PLUGIN_ROOT}/hooks/launcher-link-refresh.sh` (with the
  `# shellcheck disable=SC2016` the `vcs-detect` block already carries),
  `matcher == ""`, `hooks|length == 1` and `type == "command"` — matching
  `hooks/test-vcs-detect.sh:615-634`. A bare `endswith` match passes for a relative
  or wrongly-prefixed command, and omitting `matcher` would let a
  `"matcher": "startup"` group silently stop refreshing on resume and clear
  sessions. This is the one assertion distinguishing "the hook exists" from "the
  hook runs", so it belongs in CI rather than in a one-off `jq` line.

#### 2. The hook

**File**: `hooks/launcher-link-refresh.sh` (new, `chmod 0755`, mode committed)

The hook is pinned in full below, because the two things previous drafts left to
prose — the `set` line and how `LAUNCHER` is derived — turned out to be the two
that decide its behaviour. Values follow the hooks' uppercase convention
(`SCRIPT_DIR`, `PLUGIN_ROOT`, `ACCELERATOR` in the siblings), and are named for
what they *are* rather than for their role in `ln`'s argument order: `LAUNCHER` is
the version-pinned binary, `LINK` the plugin-owned fixed path, `STAGED` the
pre-rename link.

```bash
#!/usr/bin/env bash
set -uo pipefail
CDPATH=

# Accumulated rather than emitted inline: at most one JSON object may reach
# stdout, and two conditions can hold in one run.
NOTICE=""

finish() {
	[ -n "$1" ] && printf '[accelerator] %s\n' "$1" >&2
	if [ -n "${NOTICE}" ]; then
		jq -n --arg m "[accelerator] ${NOTICE}" '{systemMessage: $m}' 2>/dev/null ||
			printf '[accelerator] %s\n' "${NOTICE}" >&2
	fi
	exit 0
}

SCRIPT_DIR=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
PLUGIN_ROOT=$(cd -P -- "${SCRIPT_DIR}/.." && pwd -P)
LAUNCHER="${PLUGIN_ROOT}/bin/accelerator"

UNVERIFIED_LOG="${ACCELERATOR_CACHE_DIR:-${PLUGIN_ROOT}/bin}/.accelerator-unverified.log"
if [ -s "${UNVERIFIED_LOG}" ]; then
	NOTICE="an unverified launcher was recorded in ${UNVERIFIED_LOG}"
fi

case "${CLAUDE_PLUGIN_DATA:-}" in
/*) ;;
*) finish "CLAUDE_PLUGIN_DATA unavailable; no terminal link refreshed. See docs/internals.md#terminal-invocation" ;;
esac

[ -x "${LAUNCHER}" ] ||
	finish "launcher not executable: ${LAUNCHER}; leaving the terminal link alone"

DATA_BIN="${CLAUDE_PLUGIN_DATA}/bin"
LINK="${DATA_BIN}/accelerator"
STAGED="${LINK}.new.$$"

if [ -L "${DATA_BIN}" ] || { [ -e "${DATA_BIN}" ] && [ ! -d "${DATA_BIN}" ]; }; then
	finish "${DATA_BIN} is not a plain directory; remove it to let Accelerator manage the terminal link"
fi
mkdir -p -m 0700 "${DATA_BIN}" ||
	finish "could not create ${DATA_BIN}; a terminal 'accelerator' command may be stale"

if [ -L "${LINK}" ] && [ -d "${LINK}" ]; then
	rm -f "${LINK}" ||
		finish "could not clear ${LINK}; a terminal 'accelerator' command may be stale"
fi
if [ -e "${LINK}" ] && [ ! -L "${LINK}" ]; then
	NOTICE="${NOTICE}${NOTICE:+; }${LINK} exists and is not a symlink, so the terminal 'accelerator' command will not be updated. Remove it to let Accelerator manage the link."
	finish ""
fi

if [ -e "${STAGED}" ] || [ -L "${STAGED}" ]; then
	finish "stale staging path ${STAGED} is in the way; remove it and start a new session"
fi
trap 'rm -rf "${STAGED}" 2>/dev/null || true' EXIT

PREVIOUS=$(readlink "${LINK}" 2>/dev/null || true)
ln -sfn "${LAUNCHER}" "${STAGED}" ||
	finish "could not stage ${STAGED}; a terminal 'accelerator' command may be stale"
mv -f "${STAGED}" "${LINK}" ||
	finish "could not install ${LINK}; a terminal 'accelerator' command may be stale"
[ -L "${LINK}" ] ||
	finish "${LINK} is not a symlink after refresh; remove it and start a new session"

if [ -n "${PREVIOUS}" ] && [ "${PREVIOUS}" != "${LAUNCHER}" ]; then
	finish "terminal link re-pointed: ${PREVIOUS} -> ${LAUNCHER}"
fi
finish ""
```

- **`set -uo pipefail`, no `-e`** — matching `bin/accelerator:18` rather than
  `config-detect.sh`'s `set -euo pipefail`. Every mechanic carries its own `||`
  handler, so `-e` buys nothing and costs two hazards: an unguarded step becomes a
  bare non-zero exit with no diagnostic, and a failing command inside the `EXIT`
  trap can carry the shell out non-zero. `CDPATH=` is the same guard Phase 1a
  adds, for the same reason (`cd` printing a resolved directory into a command
  substitution).
- **`finish()` is the single exit point**, so the `[accelerator]` prefix and the
  exit-0 policy each appear once rather than at eight call sites, and every guard
  reads as one line naming its intention. Phase 1a takes the same shape for
  `fail`/`fail_integrity`; this is the hook-side equivalent. It also keeps every
  diagnostic inside 80 columns.
- Each message states the condition **and** what it means or what to do, following
  the only existing `[accelerator]` diagnostic
  (`hooks/migrate-discoverability.sh:68-70`, which names `/accelerator:migrate` as
  the remedy). A user whose terminal command breaks needs the next step, not just
  the symptom.
- **The root is self-located unconditionally; the hook does not read
  `CLAUDE_PLUGIN_ROOT`.** This matches Phase 1a's invariant — no ambient value may
  redirect a resolved root — and it matters more here than in the siblings, because
  this hook *publishes* the value into persistent, channel-global state rather than
  using it transiently and then `exec`ing. A stale export poisons the terminal
  command until someone notices, where in `config-detect.sh` it fails one session.
  The variable also adds nothing: `hooks.json` invokes this file as
  `${CLAUDE_PLUGIN_ROOT}/hooks/launcher-link-refresh.sh`, so its own location *is*
  the value the variable would have supplied, and reading it only introduces an
  input that can disagree with reality.

  On house consistency: **two of the four existing hooks are env-first**
  (`config-detect.sh:10`, `migrate-discoverability.sh:23`) and two self-locate only
  (`vcs-detect.sh:19`, `vcs-guard.sh`). The split is not arbitrary — the env-first
  pair are exactly the two that must locate *plugin assets* (the launcher binary,
  the migrations directory), while the self-locating pair only need their own
  sibling libraries. This hook locates a plugin asset, so it resembles the
  env-first pair in purpose; it self-locates anyway, for the publishing reason
  above. Recorded because the divergence is deliberate.

  Consequences, both of which simplify the hook: an absolute-path guard on
  `LAUNCHER` would be **unreachable** (`pwd -P` always yields an absolute path), so
  there is none — the same reasoning that removed Phase 1a's absoluteness check
  rather than shipping dead code. And there is no relative-`CLAUDE_PLUGIN_ROOT`
  case to test, because the hook never reads it. `-x` remains reachable and useful:
  it catches a root whose launcher has been removed, which is the state after an
  upgrade deletes the old directory.
- **Two channels, split by audience.** Routine and transient conditions go to
  stderr; the two that indicate a persistent state the hook cannot fix go to a
  `systemMessage` JSON object on stdout, following `hooks/vcs-detect.sh:14` (used
  for exactly this when `jq` is missing) and `vcs-guard.sh:105-106`, pinned by
  `test-vcs-detect.sh:668-683`. The two actionable cases are: `LINK` exists and is
  not a symlink (the hook will never repair it), and an unverified launcher was
  recorded. Everything else — `CLAUDE_PLUGIN_DATA` unavailable, a missing launcher,
  a failed `mkdir`/`ln`/`mv`, a stale staging path — is transient or environmental
  and would become per-session noise as a `systemMessage`.

  `NOTICE` accumulates rather than emitting inline because **at most one JSON
  object may reach stdout** and both conditions can hold in one run; `finish` is
  therefore the only exit point. The `jq -n … || printf … >&2` fallback mirrors
  `vcs-detect.sh`'s own jq guard, so a host without `jq` degrades to stderr rather
  than emitting malformed JSON.

  **Settled by Phase 0 §3, not contingent.** `systemMessage` is a universal
  top-level hook field documented as "Warning message shown to the user", so it is
  the right channel and `vcs-detect.sh`'s use of it is precedent rather than a bug.
  Critically, the split must **not** be collapsed into plain stdout: for
  `SessionStart`, stdout is added as *Claude's context*, so a diagnostic written
  there would be spliced into the prompt instead of shown to the user. stderr at
  exit 0 is undocumented, which is precisely why the routine half accepts possible
  invisibility and the trust-chain half has a durable record behind it.
- **The re-point is reported.** `PREVIOUS` is captured before the rename and
  compared after, so a backwards move — the consequence the declined downgrade
  guard leaves open — is observable rather than silent. One `readlink`, no policy,
  and it names both roots.
- **The directory-resolving-symlink clear is not optional.** Measured on Darwin:
  with `LINK -> somedir`, `mv -f STAGED LINK` **exits 0 having written
  `somedir/STAGED`** — both BSD and GNU `mv` `stat()` the destination and treat a
  directory as a target directory, and GNU's `-T` opt-out does not exist on macOS.
  So the temp-plus-`mv` dance, added for atomicity, reintroduces exactly the trap
  `-n` was chosen to avoid, writing outside `${CLAUDE_PLUGIN_DATA}` while reporting
  success. `ln -sfn` alone handles that case correctly (also measured), so the
  alternative is to drop the temp dance and accept non-atomicity. Clearing the
  symlink first keeps both properties: atomic `rename(2)` in the ordinary case,
  correct in the hostile one. Verified across all four destination states —
  symlink-to-directory, symlink-to-file, absent, and regular file.
- **`STAGED` is guarded before use, and the trap is `rm -rf`-tolerant.** Measured
  on Darwin: `ln -sfn X dir` where `dir` is a **real** directory *succeeds*,
  creating `dir/X` — `-n`/`-h` suppresses following a *symlink* to a directory, not
  a real one. So a stale `accelerator.new.<pid>` directory (left by a SIGKILLed
  session whose trap never ran, on a recycled pid) would otherwise make `ln`
  succeed and `mv` rename a **directory** onto the documented fixed path, silently
  and permanently. Also measured: `rm -f` on a directory exits 1 and leaves it, so
  the trap could not clean up after itself. Hence the pre-check, the `rm -rf`, and
  the post-`mv` `[ -L "${LINK}" ]` assertion — a belt-and-braces check that the
  published path is a symlink and not something else.
- `-n` on the `ln` is now belt-and-braces rather than load-bearing: `STAGED` is
  guaranteed absent by the pre-check, so the flag has no state left to protect
  against. Kept because it costs nothing and documents intent; **not** counted as
  covering the symlink-to-directory trap, which the clear step owns.
- The temp-plus-`mv -f` makes the ordinary re-point **atomic**: bare `ln -sf` is
  unlink-then-symlink, so two sessions starting at once — or a terminal invocation
  resolving the user's hop at that instant — can observe `ENOENT`. That property is
  **design-pinned, not test-pinned**: no automated case distinguishes it from a
  bare `ln -sfn`, and a concurrency assertion would be flaky. Recorded so a future
  refactor does not read the green suite as licence to simplify it away.
- **The `[ -L ]`/`[ ! -d ]` guard runs *before* `mkdir -p`.** Reversed, `mkdir -p`
  fails first for a regular file or dangling symlink at `DATA_BIN`, so the guard's
  second clause is unreachable and the user is told the directory could not be
  created rather than that it is not a plain directory. Worse, for a symlink to a
  non-existent path `mkdir -p` *follows it* and creates directories outside the
  tree before the refusal fires.
- **`mkdir -p -m 0700`, not `-m 0755` and not `mkdir -p` then `chmod`.** An
  unconditional symlink-following `chmod` would relax a directory the user
  restricted, or apply to a symlink's target; setting the mode at creation leaves
  an existing directory alone. `0700` rather than `0755` because only the owner's
  own shell ever resolves this link, and `-m` deliberately bypasses the umask — so
  `0755` would *widen* access for a user running `umask 077`. Note `-m` applies to
  the final component only, so if `${CLAUDE_PLUGIN_DATA}` itself must be created it
  gets a umask-derived mode; that is Claude Code's directory to own, not ours.
- **The launcher is validated, not just `${CLAUDE_PLUGIN_DATA}`.** `-x` catches a
  root that no longer exists — the state after an upgrade removes the old
  directory — and refuses rather than publishing a dangling link.
- **The trust-chain record is read before the terminal-link guards**, and from the
  path Phase 1a actually writes (`${ACCELERATOR_CACHE_DIR:-${PLUGIN_ROOT}/bin}/…`,
  the same precedence the bootstrap uses). Ordering matters: behind the
  `${CLAUDE_PLUGIN_DATA}` guard, an unverified-launcher report would be suppressed
  on exactly the Claude Code versions that lack the variable — coupling a
  trust-chain signal to the availability of a convenience feature. An earlier draft
  read `${CLAUDE_PLUGIN_DATA}/.accelerator-unverified.log`, which nothing writes, so
  the check was dead and Phase 1a's durable record had no reader at all.

  **Why it lives in this hook, and what the alternatives were.** It is a second,
  unrelated responsibility, and that is a real cost — the hook's name covers one job.
  Two alternatives were considered and rejected:

  - `config-detect.sh` is a poor host despite owning session-start config
    advisories: it is thirteen lines ending in
    `exec "$ACCELERATOR" config summary --format=hook --fail-safe`, so its stdout
    *is* the launcher's JSON envelope. Emitting before the `exec` risks corrupting
    it, and after the `exec` there is no "after".
  - `migrate-discoverability.sh` looks like the natural host — informational-only,
    already emits `[accelerator]` advisories, already computes `PLUGIN_ROOT` — but
    it early-exits when there is no `PROJECT_ROOT` or the directory does not look
    like an Accelerator project (`:16-21`). An unverified launcher is a fact about
    the *plugin*, not the project, so that precondition is subtly wrong for this
    signal: the warning would vanish in any other repository.

  A fifth, single-purpose `SessionStart` hook is the clean answer and remains the
  escape hatch if trust-chain reporting grows. Not taken now because the ordering
  fix removes the actual defect and the residue is cohesion, which does not yet
  justify a fifth hook and a fifth suite.
- A regular file at `LINK` is refused rather than clobbered — the same reasoning
  the work item gives for not writing `~/.local/bin` applies inside
  `${CLAUDE_PLUGIN_DATA}`, which the plugin does not exclusively own.
- **These guards are non-destructiveness guards, not an adversarial boundary.**
  `${CLAUDE_PLUGIN_DATA}` is under the user's own `~/.claude`, so the only actor who
  can win the remaining test-then-act races is the user or code already running as
  them. They exist to avoid destroying or writing outside user-owned state under
  concurrent or unusual filesystem shapes. Worth stating so a later reader neither
  over-credits them nor spends effort closing races that shell cannot close.

Exits 0 on every path, like every other hook.

**Three decisions taken, recorded so they are not re-litigated.** Each was raised by
multiple review lenses; each is a judgement call rather than a defect, and the
reasoning is above at the relevant note.

1. **Root precedence — self-locate unconditionally.** Chosen over env-first because
   this hook publishes a persistent, channel-global pointer rather than using the
   value transiently, and because the file's own location is by construction the
   value `${CLAUDE_PLUGIN_ROOT}` would have supplied. Two of four existing hooks are
   env-first, and both are asset-locating like this one — so the divergence is
   deliberate rather than accidental. Consequence: the absolute-path guard on
   `LAUNCHER` is unreachable and omitted, and there is no relative-root case to test.
2. **Output channel — split by audience.** Routine and transient conditions on
   stderr; the two persistent states the hook cannot fix as a top-level
   `systemMessage`, which the hooks reference documents as a universal field
   "shown to the user". Settled by Phase 0 §3 rather than contingent, and it
   confirms `vcs-detect.sh:14` as correct precedent. Plain stdout is excluded: for
   `SessionStart` it becomes Claude's context.
3. **The log reader stays in this hook**, moved above the `${CLAUDE_PLUGIN_DATA}`
   guard. `config-detect.sh` cannot host it (its stdout is the launcher's envelope
   and it ends in `exec`); `migrate-discoverability.sh`'s project-shaped precondition
   is wrong for a plugin-level fact. A fifth single-purpose hook is the escape hatch
   if trust-chain reporting grows.

**Deliberately not implemented: a backwards-re-point guard.** The link is
channel-global with last-session-wins semantics, so a session started *before* an
upgrade re-points it to the older root for every terminal user and every
concurrent session — a wider blast radius than the "for the rest of that session"
caveat implies. Refusing to move backwards would need the hook to read and compare
the version in each root's `plugin.json`, and semver-with-prerelease-tag
comparison on the bash 3.2 floor is more machinery than a `SessionStart` hook
should carry. An mtime heuristic (`[ "${LAUNCHER}" -nt "$(readlink "${LINK}")" ]`)
would be cheap and bash-3.2-safe, but it refuses legitimate re-points too — a
same-version reinstall, or a rollback the user actually wants — and adds a branch to
a hook whose contract is to always exit 0.

What the link selects is not merely a CLI version: it is the bootstrap, the
vendored verifier, the committed release key and the version-keyed launcher cache.
So the rollback consequence is documented explicitly rather than guarded — a stale
session can put the terminal command back on an installation up to ~2 weeks old,
including past a security fix — with `/reload-plugins` named as the immediate
remedy and the next session as the automatic one. If that trade proves wrong in
use, the mtime heuristic is the cheapest thing to add.

**But the re-point is reported.** Declining to *guard* it is a reasonable trade;
leaving it *unobservable* is not, since the link is opaque and `accelerator
version` is the only other tell. Before installing, compare `readlink "${LINK}"`
with `${LAUNCHER}` and, when they differ, emit one line naming both roots. That is
one syscall, refuses nothing, keeps the always-exit-0 contract, and turns a silent
backwards move into a visible one — which is the property the rejected guards were
reaching for. (Subject to the output-channel decision above: a backwards re-point
is arguably one of the actionable cases.) The `~2 weeks` figure is an observation
about Claude Code's cache retention, not a guarantee this plan controls — state it
as such in the documentation, and give the reader a command that answers "which
installation does my terminal command use?" rather than relying on the retention
window.

#### 3. Registration

**File**: `hooks/hooks.json`
**Changes**: Append a fourth `SessionStart` matcher-group, following house style
of one hook per group and the literal un-expanded
`"${CLAUDE_PLUGIN_ROOT}/hooks/launcher-link-refresh.sh"` command string.

It must be appended, never inserted: `hooks/test-vcs-detect.sh:615-634`
hard-codes `.hooks.SessionStart[0]` and asserts it is `vcs-detect.sh` with
`matcher == ""` and `hooks|length == 1`. That is the only hard-coded SessionStart
index in the repository, and the new assertions select by content precisely so
this phase does not add a second one.

No `.claude-plugin/plugin.json` edit is needed — `hooks` is not a declared field
there (it registers only `skills`), so hooks are discovered by convention.

#### 4. Suite discovery floor

**File**: `tasks/test/integration.py`
**Changes**: `run_shell_suites` globs `<subtree>/**/test-*.sh` filtered on the
exec bit, so no registry entry is needed. But `hooks` has no floor count, so a
dropped exec bit would silently remove a suite from CI.

A **count** floor alone would not have closed that for a *shell* suite: `hooks/`
holds two suites, three with a bash link-refresh harness, and the paired guard
builds its at-baseline case from the constant itself, so a floor left at `2` would
pass every test while failing to detect the loss of the very suite it was added for.
The plan therefore called for a `_REQUIRED_HOOKS_SUITES` by-name entry.

**That requirement lapses with the Python port.** `_EXPECTED_HOOKS_SUITES` stays at
`2` — the two bash harnesses — and the by-name tuple is empty, because the exec-bit
hazard the floor exists for does not apply to a pytest file: losing one is a
collection error, not a silently smaller run. The floor still guards the two
harnesses that *are* exec-bit-discovered.

Rather than a fifth copy of the seven-line `if len(suites) < _EXPECTED_*: raise
Exit(...)` block (and a fifth near-identical guard class), extract one
`_require_suite_floor(suites, floor, required, subject)` helper carrying the message
template, so each task reads as a single call. That also settles the deferral
question below by making it nearly free.

(`hooks` is one of **three** subtrees lacking a floor, not the only one:
`decisions` has one suite and `github` three, both also unguarded. With the helper
in place they are one line each — worth doing here. Their paired guard classes
collapse into one parametrised test over `(task, constant)` rather than a class per
subtree.)

**File**: `tests/unit/tasks/test_integration.py`
**Changes**: Every existing floor constant has a paired guard class here —
`TestConfigSuiteGuard`, `TestMigrateSuiteGuard`, `TestWorkSuiteGuard`,
`TestIntegrationsSuiteGuard` — each asserting `pytest.raises(Exit)` below baseline
and (for most) passing at baseline built from the constant itself. Every guarded
task gains a case, so no `Exit` branch ships without unit coverage and an
off-by-one or inverted comparison cannot pass silently.

The hook itself is an entrypoint and must **not** be added to `SHELL_LIBRARIES`
(`tasks/lint/scripts.py:18-49`) — that would trip the `chmod -x` branch
(`:128-129`) and break the pinned-membership test
`tests/unit/tasks/test_exec_bits.py:288-292`. Only one new `.sh` file lands now
that the suite is Python.

#### 5. Documentation

**File**: `docs/internals.md`
**Changes**: A new `## Terminal Invocation` section inserted at line 88, before
the `---` and the footer nav, making it the fourth `##`. Title Case, matching the
file's other three (`## The meta/ Directory`, `## Agents`, `## VCS Detection`) —
the file is internally consistent even though `docs/` as a whole is not. Same
class as VCS Detection (host-environment mechanics) and least likely to be what a
reader opens the file for. The footer nav chain (`visualiser.md` →
`internals.md` → `configuration.md`) stays intact.

House style: British spelling; third-person declarative about the product, second
person for user actions; prose → table or code fence → rationale paragraph;
`bash` fences use aligned trailing `#` comments; 80-column prose wrap is
hand-maintained (no formatter runs over `docs/`).

Content:

- **Discovery first, not a hardcoded literal.** `${CLAUDE_PLUGIN_DATA}` is a Claude
  Code *hook* environment variable, so a recipe written against the token expands to
  `/bin/accelerator` in the user's terminal and `ln` reports success on a dangling
  link. But the resolved literal is not safe to hardcode either: `~/.claude` is
  relocatable via `CLAUDE_CONFIG_DIR`, and the `data/<plugin>-<marketplace>` layout
  is an undocumented internal — the *cache* on this machine nests the other way
  round (`plugins/cache/<marketplace>/<plugin>/<version>/`) and
  `~/.claude/plugins/data/` does not exist here at all, so the shape is a guess no
  test can keep honest. So: lead with discovery —
  `ls -d "${CLAUDE_CONFIG_DIR:-$HOME/.claude}"/plugins/data/*accelerator*/bin/accelerator`
  — and show the per-channel literals as *typical* examples rather than as the
  answer. The hook prints the path it refreshed, so copying that is the most
  reliable route of all.
- **`${CLAUDE_PLUGIN_DATA}` is not version-scoped** — confirmed with the
  maintainer. The two-hop design's central premise therefore holds: the user's hop
  points at a target that genuinely never moves, so it is created once and never
  re-run. Worth stating explicitly in the documentation, because it is the property
  that makes the recipe a one-time action rather than an upgrade chore.
- A verify step **first** — `ls -l` on the discovered path — so a user on a Claude
  Code without the hook variable does not link a target that was never created.
  State the required Claude Code version inline (from Phase 0 §2), with the fallback
  for anyone below it: link the version-pinned `<root>/bin/accelerator` directly and
  re-run after each upgrade.
- The recipe as `mkdir -p ~/.local/bin && ln -sfn <discovered path>
  ~/.local/bin/accelerator`, run once and never again because its target never
  moves. The `mkdir -p` is not optional: the directory frequently does not exist,
  and the bare `ln -s` then fails with `No such file or directory`.
- That `~/.local/bin` is **not** on `PATH` by default on macOS, and that some Linux
  distributions add it only in a **login** shell — so a user who creates the
  directory to follow this recipe may still have no `accelerator` in the current
  session. Settle it for the reader with the **shell-agnostic** check first — open a
  new shell and run `command -v accelerator`, which works in every shell including
  fish — and offer
  `case ":$PATH:" in *":$HOME/.local/bin:"*) echo on ;; *) echo missing ;; esac`
  as the POSIX-shell diagnostic, labelled as such (it is a syntax error in fish,
  where the fix is `fish_add_path ~/.local/bin` rather than a profile edit).
  (Avoid pinning `/etc/paths`' contents or per-distro behaviour: the macOS list
  includes a Ventura-only entry, Fedora adds the directory unconditionally rather
  than "when present", and several distributions never add it — none of which any
  test can keep honest.)
- A command that answers **"which installation does my terminal command use?"** —
  `accelerator version`, or `readlink` through both hops — so the documented
  backwards-re-point consequence is checkable rather than merely stated.
- One line recommending a user-owned, non-group-writable `PATH` directory, since
  that directory is part of the trust chain for a command that fetches and execs
  signed binaries (Homebrew-managed `/usr/local/bin` is frequently group-writable).
- Supported platforms: **macOS or Linux (including WSL) on x86-64 or arm64**. The
  bootstrap gates on `uname -s` for `Darwin`/`Linux` (so Git Bash and Cygwin are
  refused) *and* on `uname -m` for the four published triples, so armv7, riscv64 and
  ppc64le hit a different refusal — worth naming, since a reader on one of those
  follows the whole recipe before finding out. The Linux artifacts are
  `*-unknown-linux-musl` and therefore statically linked, so there is no glibc floor
  to document — a useful fact for the Alpine and minimal-container readers this
  section is aimed at. One filesystem caveat: the plugin data directory must live on
  a symlink-capable filesystem, so under WSL it must be on the Linux filesystem
  rather than a `/mnt/<drive>` DrvFs path.
- The per-channel link paths, with a one-line note that a second channel can be
  linked under a different name (e.g. `accelerator-pre`) if the user tracks both.
- The mid-session upgrade caveat, stated at its real scope: the link is
  channel-global with last-session-wins semantics, so a session started before an
  upgrade re-points it backwards for every consumer of the fixed path, not just
  itself — and what it selects is the whole installation (bootstrap, verifier,
  release key, launcher cache), so this can put the terminal command back behind a
  security fix for up to the ~2 weeks the old directory survives.
  `/reload-plugins` corrects it immediately; the next session corrects it anyway.
- One line recommending a user-owned, non-group-writable directory for
  `ACCELERATOR_CACHE_DIR` if it is set at all, and a host you trust not to serve an
  older signed release for `ACCELERATOR_RELEASE_BASE_URL`. Both are trust-root
  inputs, not ordinary conveniences: the cache directory is where the bootstrap
  writes and *executes* a probe file and stages the fetched launcher, and because
  the cache key carries no content hash, a mirror can serve an older
  validly-signed launcher for the current version.
- That the terminal surface assumes a cache already populated by a Claude Code
  session: a first run against an empty cache needs network access to the
  artifact host and carries no `--fail-safe` degradation. Name
  `ACCELERATOR_RELEASE_BASE_URL` and `ACCELERATOR_CACHE_DIR` as the supported
  overrides for mirrored, offline or read-only installs — they exist in
  `bin/accelerator` today but are documented only in `meta/`.
- One line that `ACCELERATOR_PLUGIN_ROOT` is *exported by* the bootstrap and never
  read by it, so a packager does not mistake it for an input.
- A "removing it" line: delete your own `ln -s` when uninstalling the plugin. The
  hook that repairs the second hop stops running, so the link otherwise persists
  as a broken `accelerator` on `PATH`.

**File**: `skills/visualisation/visualise/SKILL.md`
**Changes**: `:159-162` prints, live at every skill load, a **version-pinned
one-hop** `ln -s` to `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` — exactly what the
two-hop chain exists to avoid, and after Phase 1a such a link silently runs
whichever installation it was created against. **Replace the printed command with a
reference to the new `docs/internals.md` section** rather than re-pointing it at
`${CLAUDE_PLUGIN_DATA}`: `:162` is a `!` preprocessor site, and plugin variables are
not exported into `!` shells — that is this work item's founding premise — so a
re-point would render `ln -s "/bin/accelerator" …` and hand the user a dangling link.

**File**: `docs/visualiser.md`
**Changes**: `:24,38` document an `accelerator-visualiser` "CLI wrapper —
optionally symlink onto `$PATH`". No such binary has existed since the 0168 fold.
Correct to `accelerator visualiser`, and rewrite `:24`'s trailing comment to point
at `internals.md#terminal-invocation` — otherwise the third instance of ad-hoc
symlink advice survives the rename untouched, and the `accelerator-visualiser`
grep criterion cannot detect it.

**File**: `docs/releases-and-compatibility.md`
**Changes**: The channel-switch procedure (`:21-29`) becomes incomplete once the
link exists — it is per-plugin-id, so the user's first hop must be re-pointed at
the other channel's link. Add that step, and the same to any uninstall guidance.

**File**: `README.md`
**Changes**: The `Internals` entry (`:57-58`) enumerates the file's three
sections — "the `meta/` directory deep-dive, the agent roster, and VCS detection"
— so that gloss goes stale when a fourth section lands. Update it to name
terminal invocation.

**File**: `CHANGELOG.md`
**Changes**: An `Added` bullet under `## [Unreleased]` for the terminal-invocation
surface and the `SessionStart` link-refresh hook. The repo keeps a
keepachangelog-format changelog with a live `[Unreleased]` section and generates
release notes from it (`tasks/release.py:120` calls `changelog.release`), so this is
the mechanism by which any of this reaches users.

### Success Criteria

#### Automated Verification

- [x] The hook suite passes: `uv run pytest tests/integration/hooks`
      *(22 tests, 0.5s — ported from the 62-assertion bash draft)*
- [x] Hooks integration tests pass, including the new floor:
      `mise run test:integration:hooks` *(all three suites discovered and green)*
- [x] The floor's own guard passes:
      `uv run pytest tests/unit/tasks/test_integration.py -v` *(17 passed)*
- [x] The existing hook suites still pass — in particular the
      `SessionStart[0]` index assertion: `bash hooks/test-vcs-detect.sh`
      *(131 passed; the new group was appended at index 3)*
- [x] `hooks.json` is valid and registers the hook, selected by content:
      `jq -e '[.hooks.SessionStart[].hooks[].command] | any(endswith("launcher-link-refresh.sh"))' hooks/hooks.json`
- [x] The exec-bit invariant holds: `mise run lint:scripts:check` and
      `uv run pytest tests/unit/tasks/test_exec_bits.py -v` *(17 passed)*
- [x] `docs/internals.md` carries the section:
      `grep -q '^## Terminal Invocation' docs/internals.md`
- [x] No stale wrapper name remains:
      `! grep -q 'accelerator-visualiser' docs/visualiser.md`
- [x] The competing recipe is gone:
      `! grep -q 'CLAUDE_PLUGIN_ROOT}/bin/accelerator" "\$HOME/.local/bin' skills/visualisation/visualise/SKILL.md`
- [x] No ad-hoc symlink advice survives in the visualiser docs:
      `! grep -q 'symlink onto' docs/visualiser.md`
- [x] The documented recipe carries no unexpanded token:
      `! grep -q 'CLAUDE_PLUGIN_DATA}/bin/accelerator' docs/internals.md`
- [x] `README.md` links to it and the gloss names the new section
- [x] The hook emits **at most one** JSON object on stdout, asserted by the
      both-notices-at-once case
- [x] With `jq` shadowed off `PATH`, the notices degrade to stderr and the hook
      still exits 0. **Shadowed, not stripped**: macOS 15 ships `/usr/bin/jq`,
      so dropping every directory holding a `jq` also removes `dirname`,
      `mkdir` and `ln`, and the hook fails for an unrelated reason well before
      the jq fallback. The case prepends a `jq` stub that exits 127 instead.
- [x] The hook reads no root from the environment:
      `! grep -q 'CLAUDE_PLUGIN_ROOT' hooks/launcher-link-refresh.sh` — which
      also constrains the *header comment*: the divergence note is worded
      without naming the variable so the bare grep stays a usable guard.
- [x] A re-point is reported naming both roots, and a first refresh is silent
- [x] `mise run check` exits 0

#### Manual Verification

All five need the hook running from an **installed** plugin, so they are
**deferred to the release candidate** — the installed plugin is `1.24.0-pre.16`,
which predates this hook, and Claude Code invokes `hooks.json` from the
installation rather than the working tree. The hermetic equivalents are the 62
suite assertions; what these add is the real `SessionStart` path and a real
upgrade.

- [ ] In a real session, the hook fires at `SessionStart` and
      `${CLAUDE_PLUGIN_DATA}/bin/accelerator` resolves to the current
      installation
- [ ] Following the documented recipe verbatim, on a machine where
      `~/.local/bin` does not yet exist, produces a working `accelerator` command
      on `PATH` in a new login shell
- [ ] After a plugin upgrade, a new session's link points at the new root without
      the user touching anything
- [ ] Two sessions started concurrently never leave the link absent
- [ ] Removing the plugin leaves the dangling *plugin-owned* link inside
      `${CLAUDE_PLUGIN_DATA}`; the user's own first hop is theirs to remove, as
      the documentation now says

### Implementation notes

**The suite is Python, not bash.** ADR-0048 makes Python the test language for
the non-Rust surfaces, shell wrappers included; the plan drafted a
`hooks/test-*.sh` harness by analogy with `test-migrate-discoverability.sh` and
`test-vcs-detect.sh`, which predate that decision. Ported to
`tests/integration/hooks/test_launcher_link_refresh.py`: 22 tests in 0.5s,
against 62 assertions in the bash draft. Consequences:

- **Most of the "tool constraints" paragraph evaporates.** `find -printf` versus
  BSD `find`, `stat -c` versus `stat -f`, `ls -ld | cut -c1-10` to dodge macOS's
  xattr column, `LC_ALL=C sort` for stable tree listings, `readlink 2>/dev/null`
  for GNU's non-symlink diagnostic — every one of those exists because bash has
  no portable primitive. `os.lstat`, `Path.readlink` and `sorted()` behave the
  same on both legs. What survives is the constraint that *matters*: the fixture
  root is resolved physically because the hook uses `pwd -P`, while
  `CLAUDE_PLUGIN_DATA` is never canonicalised because the hook composes it
  verbatim.
- **Scrubbing becomes construction.** `run_hook`'s `env -u …` list existed
  because a bash suite inherits the ambient environment. `subprocess.run` takes
  an explicit `env` dict, so the leak is impossible by default rather than
  guarded against.
- **The channel split is asserted more precisely.** `jq -r '.systemMessage'`
  becomes `json.loads`, and the exactly-one-JSON-object case counts objects with
  `raw_decode` rather than trusting `jq -s 'length'`.
- **`hooks/` now has two test languages, so its mise task runs both** — the
  pytest directory and the two remaining bash harnesses.
- **The floor's by-name requirement lapses**, and
  `tests/unit/tasks/test_integration.py` has to stub `Context.run`: the `hooks`
  task shells out to pytest before its floor check, and invoke's `@task` rejects
  a stand-in Context, so without the stub the floor unit tests would run the
  whole hooks suite twice.
- **The stale-staging case is unchanged in substance**, and worth re-noting: it
  still goes through a wrapper that `exec`s the hook, because the staging path
  carries the hook child's pid and no language makes that knowable in advance.

Four further things the plan did not anticipate, each settled by measurement:

- **Fixture roots must be returned as *physical* paths.** The plan's tool
  constraints say neither the hook nor the suite may canonicalise "the paths it
  is handed", which is right for `CLAUDE_PLUGIN_DATA` — the hook composes that
  verbatim — but wrong for the fixture *root*: the hook resolves its own root
  with `pwd -P`, so on macOS it reports `/private/var/folders/…` against a
  `mktemp -d` of `/var/folders/…` and every `readlink` comparison fails on that
  leg. `make_root` therefore returns `cd -P … && pwd -P`, deriving the expected
  value the same way the hook derives the actual one. Three cases were red
  before this.
- **The jq-absent case shadows rather than strips.** See the criterion above.
- **The hook's header comment cannot name `CLAUDE_PLUGIN_ROOT`.** The plan asks
  the hook to record *why* it self-locates rather than reading the variable, and
  separately asserts `! grep -q 'CLAUDE_PLUGIN_ROOT'` over the file. Both are
  satisfiable only if the note is worded without the literal, which it now is.
- **Two shellcheck findings are the plan's own deliberate choices**, so both
  carry justified disables: `SC2174` (`-m` with `-p` applies only to the deepest
  directory — exactly the intent, since `${CLAUDE_PLUGIN_DATA}` itself is Claude
  Code's to own) and `SC2012` (`ls -ld | cut` is the only portable mode read once
  `stat` is banned for the GNU/BSD flag split and BSD `find` has no `-printf`).

The plan's optional deferral is taken: `decisions` and `github` gained floors
alongside `hooks`, since the extracted helper made each one line.

---

## Phase 4: `!`-Site Conformance Suite

> **Done 2026-07-29.** Two commits: the harness extraction, then the suite.

### Overview

Prove that every `bin/accelerator config *` command the skills actually invoke
works in the production shape — correct path, empty environment.

The measured corpus is **204 `config` commands, 122 distinct**, across **45**
SKILL.md files. Its shape simplifies one half of the problem and hardens the
other. No `config` command carries skill-time argument interpolation: all 204 are
static literals whose only `$` is the `${CLAUDE_PLUGIN_ROOT}` prefix, so no
fixture-argument or skip-with-reason branch is needed. But ~86 of them (42%)
legitimately emit empty stdout — `config instructions <skill>` and
`config context --skill <skill>` render *user* per-skill overrides — and *which*
ones are empty depends on the project's own `.accelerator/config.md`. Running
against this repo would make the exception list drift with unrelated config
changes.

The answer is a committed fixture project with a known configured set, and
assertions classified by family rather than enumerated per command.

**Scope note.** There are 206 `!`-site launcher invocations in total; this suite
covers the 204 `config` ones. Both remainders are in
`skills/visualisation/visualise/SKILL.md`: `:162` is a `printf` that *renders* a
recipe rather than invoking the launcher, and `:30` is
`accelerator visualiser --owner-pid $PPID ${ARGUMENTS:-start}` — the only `!` site
with genuine argument interpolation and the only one carrying no `--fail-safe`. It
is excluded because it starts a daemon, and it stays excluded: an
`accelerator visualiser status` substitute cannot be made to do useful work here.
`LazyProductionResolver::resolve` (`main.rs:57-69`) checks `override_path` first, so
pointing `ACCELERATOR_VISUALISER_BIN` at a local build short-circuits before
`cache_root::resolve` is ever called; and without the override the resolver fetches
`manifest.json` and verifies it against the *real* release key that `build.rs`
embeds via `include_str!` (`resolve/keys.rs:11-12`), which a locally signed fixture
manifest cannot satisfy — and which would violate this suite's own hermeticity
criterion. So external-subcommand dispatch is out of scope for this suite, and the
renamed `cache_root.rs` read is covered where it already is: `version.rs:166-183`
and `cache_root.rs:130-136`. Recorded as a known coverage gap rather than papered
over with an assertion that exercises neither path.

### Changes Required

#### 1. A shared fixture-installation builder, and the fixture project

**File**: `tests/integration/support/installation.py` (new)
**Changes**: Extract the entrypoint suite's fixture-installation apparatus into a
shared helper as a **Phase 1a deliverable**, so this phase can import it. Today
`shim_bin`, the release/attacker keypairs, `_sign`, `_serve_launcher`,
`_DOWNLOADER_SRC` and `make_harness` are module-private in
`test_accelerator_entrypoint.py`, and `tests/integration/` has no shared support
module — every suite there is self-contained. Without the extraction this phase
either duplicates ~150 lines of trust-chain harness, giving two independent
encodings of the release asset-naming and signing contract that will drift when
`tasks/shared/targets.py` or the shim naming changes, or it cannot be built at all.
The helper also carries the `_run_bootstrap` preconditions from Phase 1a, so this
suite inherits the network guard rather than needing its own.

**The installation root is constructed per session, not committed.** It must be:
`.claude-plugin/plugin.json`, a verify shim named for the *running* host
(`accelerator-verify-<darwin|linux>-<arm64|x64>`, derived at runtime as the
entrypoint suite already does), the *generated* test release public key, a writable
cache dir, and a symlink to the repo's `templates/` tree so the 20
`config template <name>` non-empty assertions resolve. None of that can be
committed: the shim is platform-specific and would skip or fail one CI leg, and the
keypair is generated per run with `*.key` gitignored, so a committed public key
could never match a signature this suite produces. Committing a shim would also put
a second, unguarded copy of a trust-root binary under `tests/`.

**This is what supplies the launcher.** Substituting the extracted prefix to the
*repo* root instead would self-locate there, pass every gate, and `curl` the real
GitHub release for a version that is not yet published — the hazard that condemns
the two tests Phase 1a deletes. The dev override cannot substitute either: its
containment gate requires the plugin root to *be* the repo root, and creating
`.accelerator-dev-launcher` there is forbidden by
`test_accelerator_entrypoint.py:826` and would flake that suite under parallel
`mise run`. So the harness reuses Phase 1a's mechanism — the freshly built launcher
served through the stub release server with `ACCELERATOR_RELEASE_BASE_URL` pointed
at it — and substitutes the prefix to the fixture root.

**Files**: `tests/integration/skill-invocation/fixtures/conformance-project/` (new,
committed)
**Changes**: The only committed fixture — a minimal project tree with
`.accelerator/config.md` configuring a per-skill instructions **and** context
override for **every** skill name appearing in the extracted corpus, plus the
document-path sections the `paths`/`path` families read. Co-located with its suite
per the repo's convention (`tests/unit/tasks/fixtures/`,
`tests/integration/deny/fixtures/{banned,clean,…}/` — there is no top-level
`tests/fixtures/`, and introducing one would be a third pattern). Generate the
config file and the expectations from one fixture data structure the test reads, so
the oracle stays single-sourced as skills are renamed.

Configuring every skill rather than a token few is the point. With only one or two
configured, ~84 of the 86 expectations reduce to `stdout == ""` — which is
byte-identical to a `Failure::Read` degrading under the `--fail-safe` every one of
these commands carries. Forty-one per cent of the suite would be unable to
distinguish success from silent degradation while the corpus size made it look
comprehensive. With every skill configured, all 86 become non-empty content
matches.

#### 2. The harness

**File**: `tests/integration/skill-invocation/test_skill_invocation_conformance.py`
(new)
**Changes**: Extraction reuses the public machinery already in
`tasks/lint/skill_permissions.py` — `preprocessor_commands(text)` (`:94-96`)
and `is_plugin_invocation(command)` (`:99-101`) — rather than a second regex. Note
`is_plugin_invocation` is `command.startswith("${CLAUDE_PLUGIN_ROOT}/")`, so it
matches every plugin *script* invocation too, not just launcher calls; the filter
must be composed —

```python
is_plugin_invocation(cmd) and "/bin/accelerator config " in cmd
```

— or the harness would execute arbitrary plugin shell scripts, some of which write
into `meta/` or call remote trackers. The prefix used for substitution is
`skill_permissions.py`'s own `_PLUGIN_PREFIX` (`:55`), promoted to a public
`PLUGIN_PREFIX`, rather than a third literal copy that could desynchronise from
the guard.

For each extracted command the harness performs Claude Code's own textual
substitution, rewriting the `PLUGIN_PREFIX` prefix to the fixture installation
root, and runs it with **both** variables removed from the environment, cwd set to
the fixture project. Execution is `shlex.split` with **`shell=False`**: `!`-block
content is arbitrary shell text, so shelling out to markdown-derived strings would
make any SKILL.md an execution vector inside the test process. The measured corpus
is entirely static literals, so nothing is lost; any command containing shell
metacharacters (`|`, `;`, `&`, backticks, `$(`, redirection) is reported through
the decline channel rather than executed.

Assertions, per command: exit 0, and no `accelerator:` diagnostic line on stderr.
That prefix is `fail()`'s `printf 'accelerator: %s\n'`, so it detects
bootstrap-layer aborts specifically — it is not a general "the CLI fully
succeeded" signal, since the launcher's own diagnostics carry no such prefix.

Per-family stdout assertions:

| Family | Count | Assertion |
|---|---|---|
| `agents`, `paths`, `path <k>`, `work <k>`, `review <k>` | ~75 | non-empty |
| `template <name>` | 20 | non-empty; a launcher-layer `Failure::Refusal`, so fail-closed regardless of `--fail-safe`. Note this holds for the *launcher* only — a bootstrap-layer abort still degrades these to empty at exit 0. |
| `templates list` | few | non-empty, with plugin-default rows |
| `instructions <skill>`, `context --skill <skill>` | 86 | exact content match against the fixture's configured override |

**Corpus integrity is asserted structurally, not by a magic number.** A
`>= 200` floor fires on routine skill consolidation while staying quiet on the loss
it exists to detect — dropping one skill's two commands still clears 200 if skills
are added elsewhere. Instead:

- every `skills/**/SKILL.md` containing a `bin/accelerator config` `!` site
  contributes at least one command to the corpus;
- the number of skills contributing a `config instructions <name>` command equals
  `skill_permissions.EXPECTED_INJECTION_SKILLS`, and likewise for
  `config context --skill` — cross-checking against a constant that is already
  single-sourced and already an exact equality, rather than merely asserting the
  family is non-empty (which one command would satisfy while 85 vanished);
- the declined set is **asserted empty**, not logged: `skill_permissions.py`'s rule
  4 already guarantees no `!` command contains a shell metacharacter, so a non-empty
  decline list means the corpus changed shape and should fail rather than inform.

The corpus size and the distinct count are `log`-reported alongside, so a shrinking
scan is visible as well as fatal.

#### 3. Task registration and the build edge

**File**: `tasks/test/integration.py`
**Changes**: A new `skill_invocation` integration task. Two naming constraints.
`tests/integration/tasks/` is not the home for this — it holds integration tests of
invoke task *modules* (`test_github.py` → `tasks/github.py`, `test_release.py` →
`tasks/release.py`), and `test:integration:tasks`' own description is "Run pytest
integration tests for invoke tasks" (`mise.toml:161`). Nor is `skills`: every
existing `test:integration:<name>` leaf whose name is not a component denotes a
*subtree of `skills/`* run through `run_shell_suites(context, "skills/<name>")`
(`decisions`, `github`, `migrate`, `work`, `integrations`), so a `skills` leaf would
read as their roll-up and sit in the file beside `def work` doing something
categorically different. `skill-invocation` names what is under test and matches the
hyphenated multiword style of `lint:store-duplication:check`.

**File**: `mise.toml`
**Changes**: A `test:integration:skill-invocation` leaf with
`depends = ["deps:install:python", "build:cli:dev"]`, modelled on the six existing
launcher-dependent integration tasks, with a description naming what it does
(extracts `!`-block commands from `skills/**/SKILL.md` and runs them in the
production shape). **Added to `test:integration.depends` (`:228-243`)** — that list
is what CI runs (`.github/workflows/main.yml:91`) and what the bare `default` task
reaches; it is curated rather than derived (`test:integration:pup` is deliberately
absent), so membership is an explicit decision. Without it the 204-command suite
ships green and never executes.

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: Added to `_LAUNCHER_DEPENDENTS`, and — because nothing today pins
roll-up membership — a second exhaustive assertion that every `test:integration:*`
leaf appears in `test:integration.depends` except an explicitly-reasoned exclusion
set containing `pup`. Same fail-loudly shape as the launcher-edge split, closing the
same class of silent omission one level up.

CI needs no edit: `test-integration` already carries the `RUSTUP_HOME` routing
step and `workspaces: cli` cargo caching.

### Success Criteria

#### Automated Verification

- [x] The suite passes:
      `uv run pytest tests/integration/skill-invocation -v` *(128 passed in 80s;
      122 command cases + 6 corpus invariants)*
- [x] The structural corpus invariants hold — every SKILL.md with a `config` `!`
      site contributes a command, the `instructions`/`context` counts equal
      `EXPECTED_INJECTION_SKILLS`, and the declined set is empty. A fourth
      invariant was added: every skill *named* by an `instructions`/`context`
      command must exist. The fixture derives its overrides from the corpus, so
      without it a `!` site naming a nonexistent skill would have an override
      helpfully created for it and pass — and neither the permissions census nor
      the count checks would notice, since both count skills carrying an
      injection rather than whether the name resolves.
- [x] The suite is hermetic — it performs no live fetch, serving the launcher from
      the stub release server only (enforced by `run_bootstrap`'s preconditions,
      plus the session-scoped `bin/` guard)
- [x] `mise run test:integration:skill-invocation` exits 0
- [x] The leaf is reachable from the roll-up CI runs, and the build edge is
      asserted: `uv run pytest tests/unit/tasks/test_mise.py -v` *(23 passed)*
- [x] `mise run test:integration` runs it (not just the leaf directly)
- [x] `mise run check` exits 0

#### Manual Verification

- [x] Adding a new `!` site with a broken `config` command to any SKILL.md turns
      the suite red. **Confirmed**: a `config template no-such-template
      --fail-safe` site appended to `create-note` failed as its own case.
- [x] Adding a `!` site to a *new* SKILL.md that the scan fails to pick up turns
      the suite red via the per-file invariant. **Confirmed** with a real new
      `skills/zzz-probe/SKILL.md` and the extractor's walk narrowed to skip it:
      only `test_every_skill_with_a_config_site_contributes_a_command` fired.
- [x] Changing this repository's own `.accelerator/config.md` does **not** change
      the suite's result — the fixture project is the only oracle. **Confirmed**:
      with `paths.work`, `paths.plans` and `agents.reviewer` all rewritten in the
      repo config, all 128 still pass.
- [x] Deleting the fixture's instructions override for one skill turns exactly one
      assertion red, confirming the 86 are content-discriminating rather than
      empty-tolerant. **Confirmed, with a count correction**: dropping the
      `commit` override reddens *two* cases, not one — `instructions commit` and
      `context --skill commit`. Each configured skill contributes one command to
      each family, so two is the correct blast radius.

### Implementation notes

Seven departures from the section above, each measured.

- **The support extraction was not a Phase 1a deliverable after all.** Phase 1a
  shipped with the apparatus still module-private in the entrypoint suite, so
  this phase does the extraction itself — commit 1, with the entrypoint suite
  refactored onto it and still 46 green.
- **The fixture project is generated per run, not committed.** The section asks
  for both ("the only committed fixture" *and* "generate the config file and the
  expectations from one fixture data structure"). Generation wins: a committed
  tree of ~84 override files would need hand-maintenance on every skill rename,
  which is precisely the drift the single-sourcing exists to prevent. The cost
  is that the fixture is derived from the thing under test, which is why
  `test_every_named_skill_exists` was added — see the criterion above.
- **The corpus module lives in `tests/integration/support/`.** A module inside
  `tests/integration/skill-invocation/` is not importable: the directory name is
  hyphenated. The suite directory keeps the hyphen (matching the task name); only
  the importable module moves.
- **The leaf gets no `build:cli:dev` edge, and goes in `_NO_LAUNCHER_NEEDED`.**
  The section says to add both, but the suite builds the launcher in-fixture
  through the shared `build_launcher` — which exists so a suite runs standalone —
  and Phase 1a's own note says a build edge would then contend on cargo's target
  lock while making the assertion inert. The two halves of the plan contradict
  each other here; the Phase 1a reasoning is the one with a mechanism behind it.
- **The family table's counts are off in three places.** Measured: 13 distinct
  `template <name>` commands (19 occurrences), not 20; there is **no**
  `templates list` command in the corpus at all; and there is a
  `config agent <name>` family the table omits. The totals the section states —
  204 commands, 122 distinct, 45 files, 42 injection skills each way — are all
  exact.
- **Every one of the 122 distinct commands renders non-empty stdout** against the
  fixture, which is better than the section predicts. So no family needs an
  empty-tolerant assertion: the per-skill 86 are exact content matches and
  everything else is asserted non-empty.
- **The suite does not name `CLAUDE_PLUGIN_ROOT`.** Phase 2's residue criterion
  lists it among the files that "must name what they forbid or strip". It does
  not need to: promoting `skill_permissions._PLUGIN_PREFIX` to a public
  `PLUGIN_PREFIX` — which the section calls for anyway, to avoid a third literal
  copy — means the substitution reads the constant instead. Phase 2's criterion
  is corrected accordingly.
- **One bootstrap invocation per command, not two.** The section implies separate
  exit-code and stdout tests; merging them halves 244 subprocess runs to 122
  (~80s for the suite). Cases are parametrised over *distinct* commands — the 204
  occurrences carry 122 distinct texts, and a duplicate exercises nothing new.

---

## Phase 5: A Missing Plugin Root Becomes a Named Error

> **Done 2026-07-29.** One commit. All implementation phases are now complete;
> what remains in this plan is the closing `mise.local.toml` step and the manual
> criteria deferred to the release candidate (Phase 1a's published-release check
> and Phase 3's five installed-hook checks).

### Overview

Remove the design flaw the rename does not fix. `plugin_root()` returns
`Option<PathBuf>` and the store degrades to `Ok(None)` / `vec![]` at the sites that
read plugin content — so a caller who reaches the launcher without the export gets
an empty template list and an empty template-name set at exit 0.

Three moving parts: a new `ConfigError` variant classified as a **refusal** (so it
never degrades and `config template <name>` keeps its documented fail-closed
behaviour), a small port change making `template_names` fallible (without which the
empty-table case is unreachable), and refusals at the three plugin-content consumers
behind one private accessor. `known_skill_names` is deliberately left alone — see §3.

**The requirement goes at the consumers, not at the composition boundary.** An
earlier draft made `plugin_root()` return `Result` and had `compose_stack`
propagate, on the stated grounds that the failure would classify as
`Failure::Read` and so degrade under `--fail-safe`. That mechanism does not exist:
`dispatch` does `let stack = compose_config()?;`
(`cli/launcher/src/launch/mod.rs:189`), converting straight to `kernel::Error`,
while `--fail-safe` is applied only by `finish`
(`cli/launcher/src/config_command/inbound/cli.rs:452-471`) once a `ConfigStack`
exists. Placing it there would therefore have three costs, all avoidable:

- A missing root would hard-fail **every** `config` command, reinstating at the
  launcher layer the prompt-discarding mode Phase 1a removes at the bootstrap
  layer. Routing it through the fail-safe policy instead would need a second
  degradation funnel at `dispatch`, duplicating a decision `finish` currently owns
  alone.
- It would break the **root-independent families** — `agents`, `paths`,
  `path <k>`, `instructions <skill>`, `context --skill`, `work <k>`, `review <k>`,
  and `summary` (which the work item's measured table omits, but which reaches
  `known_skill_names` via `skill_customisations`) — widening the empty-answer
  surface from one family to nine, the opposite of the phase's goal. At a `!` site
  most would emit an "unavailable" notice on stdout (`finish`'s `Degrade::Notice`
  arm) in place of an answer they can still compute correctly.
- It would break **47 collateral tests**. `config_read.rs:62` is not one case: it
  is inside `run_in` (`:58-67`), an env-hygiene helper used by 47 call sites that
  also strips `ACCELERATOR_LOG`, `ACCELERATOR_CACHE_DIR` and
  `ACCELERATOR_RELEASE_BASE_URL`. None of them asserts anything about the root;
  the 36 template cases use the separate `run_with_plugin` (`:1196-1208`), which
  builds its own `Command` and injects one.

Requiring the root where it is actually read changes no constructor signature and
breaks no existing test. It does add one small port change — `template_names` must
become fallible or the empty-table case stays unreachable — which §2 scopes.

### Changes Required

#### 1. A named error for the missing root, classified as a refusal

**File**: `cli/config/src/error.rs`
**Changes**: Add a **unit** variant `PluginRootUnavailable` to the
`#[non_exhaustive]` `ConfigError` enum, following `LegacyLayout` — which is the
in-file precedent for a payload-free variant carrying its whole message, including
its remedy, in the `Display` arm. A `detail: String` would have exactly one producer
and one constant value, duplicating into `store.rs` a literal the `Display` arm
already owns; `#[non_exhaustive]` leaves room to add a payload later if a
caller-varying one ever appears. No existing variant fits: `Io { path, detail }`
would render "I/O error on 'ACCELERATOR_PLUGIN_ROOT'", which it is not, and
`Invalid { detail }` carries unrelated validation semantics.

The message text, stated here rather than left to the implementer:

> `the plugin installation root is unknown: set ACCELERATOR_PLUGIN_ROOT, or invoke
> accelerator through bin/accelerator, which derives it`

Add the matching `Display`-arm unit test to the module's existing per-variant test
block (`not_found_names_the_key`, `io_names_the_path_and_detail`,
`legacy_layout_names_the_migrate_directive`, …) asserting the rendered string
contains `ACCELERATOR_PLUGIN_ROOT`. Without it, the property the whole phase turns
on is pinned only by a launcher subprocess test.

**File**: `cli/config/src/error.rs` (classification)
**Changes**: The classification must be **opt-in, not opt-out**. Today
`From<ConfigError> for Failure` (`cli/launcher/src/config_command/inbound/cli.rs:422-429`)
ends in `_ => Self::Read(error)`, and because `ConfigError` is `#[non_exhaustive]`
*across a crate boundary* the compiler can never force the launcher to classify a
new variant — so the default for anything added later is "degradable", which at a
`!` site means a silent empty or notice-only answer. That is the exact failure class
this phase exists to close, and the phase's own named follow-up would land in it.

So put the classification where exhaustiveness is enforced — in the defining crate:

```rust
impl ConfigError {
    /// Whether `--fail-safe` must not absorb this failure.
    pub(crate) const fn is_refusal(&self) -> bool {
        match self {
            Self::Invalid { .. } | Self::PluginRootUnavailable => true,
            Self::NotFound { .. } | Self::Io { .. } | Self::LegacyLayout => false,
        }
    }
}
```

An exhaustive `match` inside the defining crate means a new variant does not compile
until it is classified. `From<ConfigError> for Failure` then becomes a call to it,
and the wildcard disappears.

**File**: `cli/launcher/src/config_command/inbound/cli.rs`
**Changes**: `From<ConfigError> for Failure` delegates to `is_refusal()`. Also
**broaden two doc comments** that now misdescribe what they cover: `Failure::Refusal`
reads "a validation refusal that stays fail-closed regardless of `--fail-safe`", but
a missing installation root is an environment precondition, not validation — so the
arm's name and doc describe a *cause* while what `finish` consumes is a *policy*.
Reword to "a failure `--fail-safe` must never absorb", and drop the variant name from
`finish_scalar`'s doc comment (`:439-440`), which becomes an incomplete enumeration.
Both edits shorten existing comments rather than adding new ones.

`Refusal`, not `Read`, and this is the decision the phase turns on. Three
consequences, all of them the ones we want:

- **`config template <name>` keeps its documented fail-closed behaviour.** Today a
  rootless render is `Ok(None)` → `Failure::Refusal(not_found(...))`
  (`resolve_template`, `:238-246`), which `finish` never degrades — a property this
  plan states in *Current State Analysis* and relies on in Phase 4's table.
  Classifying the new variant as `Read` would silently reclassify it to a
  degrading failure; classifying it as `Refusal` leaves the exit status unchanged
  and merely improves the message from "not found" to one naming the variable.
- **There is no `--fail-safe` degradation case to specify.** A `Refusal` never
  reaches `finish`'s degrade arm, so stdout stays empty and the exit is non-zero
  whether or not the flag is present. An earlier draft asserted "exits 0 with empty
  stdout" under `--fail-safe`, which was doubly wrong: `Read` would have degraded,
  and the degrade arm for every template action is
  `Degrade::Notice(template_render::render_unavailable)`, whose `render::emit` does
  `print!` — so stdout would have carried `## Template Unavailable`, exactly as
  `templates_list_with_fail_safe_renders_the_unavailable_notice`
  (`config_read.rs:1504-1514`) already pins.
- **`!` sites are unaffected, because they cannot reach it.** After Phase 1a the
  bootstrap always exports a derived, absolute, non-empty root, so a launcher
  reached from a `!` site always has one. The loud path fires only for the
  non-bootstrap caller this phase exists to protect — which is the whole point.

#### 2. Make `template_names` fallible

**Files**: `cli/config/src/service.rs`, plus four call sites
**Changes**: `ReadTemplate::template_names(&self) -> Result<Vec<String>, ConfigError>`
(`:328`). This is in scope, and it is small — the earlier draft deferred it on the
belief that it rippled to "every implementor", but there is exactly **one**
(`store.rs:310`):

| Site | Today | Change |
|---|---|---|
| `core/template.rs:43` (`list`) | already returns `Result<_, ConfigError>` | add `?` |
| `inbound/cli.rs:541` (`eject_all`) | returns `Result<(), kernel::Error>` | add `?` |
| `visualiser/server/src/compose.rs:157` | `ComposeError` has `#[from] ConfigError` | add `?` |
| `core/template.rs:62` (`available`) | `-> String`, builds an error hint | `.unwrap_or_default()` |

The last is the only judgement call and it lands cleanly: `available` exists to
build the "here are the available names" hint *inside* a not-found diagnostic, and
`available_or_none` (`:68`) already renders an empty result as `(none found)`. So a
failed enumeration degrades the hint while the error it decorates carries the real
signal — no signature change, and no possibility of constructing an error becoming
itself fallible. Put that rationale in `available`'s **existing** doc comment (it
already says "for the not-found diagnostic") rather than an inline comment: a bare
`.unwrap_or_default()` on a `Result<_, ConfigError>` is otherwise the classic
swallowed error a reviewer flags, and this repo's comment rules rule out explaining
it at the call site.

The signature change also needs an **`# Errors` doc section** on the trait method.
Every other fallible method on these ports has one, and the `cli/` workspace sets
`warnings = "deny"` with `clippy::pedantic`, so `missing_errors_doc` makes its
absence a `cli:check` failure mid-phase — not a style nit.

Without this change the phase's own headline case is unreachable: `template_view::list`
derives every row from `template_names()`, so a rootless call yields zero rows,
`resolve_template` is never invoked, and `templates list` renders a header-only
table at exit 0 regardless of what the other consumers do.

#### 3. Refuse at the three plugin-content consumers

**File**: `cli/config-adapters/src/store.rs`
**Changes**: `plugin_root` stays `Option<PathBuf>` on the struct — so
`FileConfigStore::at()` (`:42-49`), `with_plugin_root` (`:58`),
`config_adapters::compose()` (`compose.rs:41`), the visualiser server's
`compose.rs:44`, the 17 `at(&root)` calls in this file's own tests and the one in
`tests/parity.rs:53` are all untouched.

Route **every** reader through one of **two** named accessors, so `self.plugin_root`
has zero direct readers left in the impl:

```rust
/// The installation root, required: this read cannot be synthesised without it.
fn require_plugin_root(&self) -> Result<&Path, ConfigError> {
    self.plugin_root.as_deref().ok_or(ConfigError::PluginRootUnavailable)
}

/// The installation root when known. Callers here degrade a hint or a warning
/// rather than an answer, so an absent root is tolerated.
fn plugin_root_if_known(&self) -> Option<&Path> {
    self.plugin_root.as_deref()
}
```

An earlier draft had a single `plugin()` accessor and justified it as making "a
fourth plugin-content reader correct by default" — but it left two raw readers in the
same file (`known_skill_names` at `:281`, `display_path` at `:78`), so the next
reader had two live precedents for the pattern the accessor exists to retire, and the
tolerant one was the nearer neighbour. Two named accessors put the choice in the name
instead: a `grep` for `self.plugin_root` returns exactly the two accessor bodies, and
a new reader must pick one. `require_plugin_root` is also guard-shaped rather than
getter-shaped, so `let plugin = self.require_plugin_root()?;` reads as the refusal it
is.

Required (`require_plugin_root`):

- `:356` `template_names` — no longer returns `vec![]`, so a **missing root** can no
  longer render a header-only empty table.
- `:343` the plugin-default template tier. **The check replaces the
  `if let Some(plugin)` *in place*, at the third tier** — it must not be hoisted to
  the top of `resolve_template` as a guard clause, natural though that reads. The
  configured-path and user-override tiers return earlier (`:335-341`), so hoisting
  would make every rootless template read refuse and break project-local overrides —
  a regression in the very "root-independent families keep working" property this
  phase preserves. Pinned by a new criterion below.
- `:413-417` `plugin_template_path` — signature becomes
  `fn plugin_template_path(&self, name: &str) -> Result<PathBuf, ConfigError>`.
  Both consumers currently use the `Option` combinator
  `let Some(p) = self.plugin_template_path(name).filter(|p| p.is_file()) else { … }`
  (`plugin_default` at `:382-386`, `TemplateOverride::eject` at `:429-437`), and
  `Result` has no `filter` — so both become `let path = self.plugin_template_path(name)?;`
  followed by a separate `if !path.is_file()` retaining the existing early return.
  Note the consequence: `eject` can no longer report `EjectOutcome::NoDefault` for a
  *missing root* (it still does for a present root with no default), and
  `plugin_default`'s port doc (`service.rs:336-339`, "`None` when the plugin ships no
  default") gains a third outcome and needs its `# Errors` updated.

Tolerated (`plugin_root_if_known`): `display_path` (`:74-84`, unchanged behaviour,
keeps its `<plugin>/` shortening) and `known_skill_names` (`:281`), which becomes
`let Some(plugin) = self.plugin_root_if_known() else { return Ok(Vec::new()) };` —
the same tolerance as today, now expressed through the accessor so the choice is
visible in the code rather than only in this plan. Record the contract at the port
too: `ReadLensCatalogue::known_skill_names`'s doc (`service.rs:188-191`) already says
"Empty when the plugin root is unknown"; add the symmetric `# Errors` sentence to
`ReadTemplate::template_names` naming `PluginRootUnavailable`, so the asymmetry reads
as a decision at both sites.

**`known_skill_names` (`:281`) is deliberately excluded.** An earlier draft called it
"the widest behavioural change in the phase", on the grounds that
`config instructions <bogus-skill>` would start erroring. Traced: it has exactly one
caller, `skill_customisations` (`core/summary.rs:141`), and `config instructions`
never touches it — that path validates via `config::validate_identifier`
(`core/context.rs:54`). So the claimed benefit does not exist. The cost does:
`skill_customisations` calls `known_skill_names()?` *before* the
`if !known.is_empty()` gate, so the `?` propagates, and `assemble` reaches it
whenever team or personal config is present (`:64-68`). That makes **`config summary`**
root-requiring — an eighth family the root-sensitivity table never measured, and the
`SessionStart` hook's own command (`hooks/config-detect.sh:13` runs
`config summary --format=hook --fail-safe`). It would also redden two existing
rootless tests, `summary_matches_the_committed_golden` (`config_read.rs:1072`) and
`summary_hook_wraps_the_plain_output_as_additional_context` (`:1081`), both of which
run `run_in` against the `summary` fixture — which carries a `config.md` and a
`skills/create-plan/`, so it takes the Configured path. Pure cost, no benefit:
leave it returning `Ok(Vec::new())`.

**What this does and does not buy.** It converts a *missing root* into a named
refusal at the three plugin-content sites — note the narrower phrasing: the earlier
bullet claiming `templates list` "can never render a header-only empty table"
contradicted this paragraph and is corrected above.

What survives is broader than "a missing `templates/` directory". `template_names`
also has `let Ok(entries) = fs::read_dir(plugin.join("templates")) else { return
Ok(Vec::new()) }` (note: `Ok(...)` after §2's signature change), and that arm swallows
**any root that is not an installation** — including a root that is *set but wrong*,
which is the accident Phase 1a's self-location makes newly plausible and which the
directly-invoked launcher never passes a `plugin.json` gate to catch. So a
`Result`-returning enumerator still succeeds-with-nothing on an I/O failure, which is
a mixed signal worth naming rather than leaving for the next reader to discover.

Two consequences for this phase:

- Add a **characterisation test** so the surviving behaviour is visible rather than
  implicit: a plugin root fixture with no `templates/` directory yields a
  header-only table at exit 0, named so it reads as deliberate (e.g.
  `a_root_without_a_templates_directory_still_renders_an_empty_table`).
- Raise a **follow-up work item** for converting that arm's `NotFound` case into
  `PluginRootUnavailable` (keeping genuine I/O failures as `Io`), and reference its
  id here — a prose note in a completed plan is not a work queue. **Raised as
  0184** (`meta/work/0184-template-enumeration-swallows-a-wrong-plugin-root.md`).

#### 4. Update the tests

**File**: `cli/launcher/tests/config_read.rs`
**Changes**: `run_in` is untouched, so all 47 rootless hygiene cases keep passing —
and because `known_skill_names` is excluded, the two rootless `summary` cases
(`:1072`, `:1081`) are unaffected too, which is the regression the exclusion exists
to avoid.

The **"with and without `--fail-safe`"** pairing applies only to the commands that
carry the flag. `templates list`, `template <name>` and `templates show` do; the
`TemplatesAction::Eject`/`Diff`/`Reset` clap variants (`launch/inbound/cli.rs:285-311`)
**do not**, so passing it there is a clap usage error whose stderr is usage text, not
the variable — and a test loosened to "exit non-zero" would pass on argument parsing
rather than on the refusal. So:

- **Flag-bearing commands** (`templates list`, `template <name>`, `templates show`):
  non-zero exit, empty stdout, and a stderr diagnostic naming
  `ACCELERATOR_PLUGIN_ROOT` — **identical with and without** `--fail-safe`, since a
  `Refusal` does not degrade. That identity is what distinguishes the `Refusal`
  classification from a `Read` one: a `Read` would degrade to
  `## Template Unavailable` on stdout at exit 0, which
  `templates_list_with_fail_safe_renders_the_unavailable_notice` (`:1504-1514`)
  already pins.
- **`templates eject <name>`, `eject --all`, `diff`, `reset`**: unflagged only, each
  asserting non-zero exit and the variable-naming diagnostic. `eject --all` matters
  most and was previously unnamed: rootless today it enumerates `vec![]`, loops zero
  times and **exits 0 having ejected nothing** — the silent-empty-answer mode this
  phase exists to close — so the transition needs an assertion, plus "no file written
  under `.accelerator/templates/`". These commands never construct a `Failure` at all
  (`run` routes them past `run_read`), so their `ConfigError` reaches
  `From<ConfigError> for kernel::Error` directly; their exit code is 1 either way and
  the *message* is the only observable, which is why the diagnostic must be asserted
  rather than the status.
- **A rootless user override still resolves**: with `.accelerator/templates/demo.md`
  present and no root, `config template demo` exits 0 and emits the override's
  content. This is what pins the "in place, not hoisted" requirement from §3 — the
  existing override test (`:1299`) injects a root via `run_with_plugin`, so it cannot
  catch the hoisting mutation.
- **The empty-string case.** Phase 1b adds the filter at `main.rs:176` and keeps a
  case asserting `ACCELERATOR_PLUGIN_ROOT=""` behaves byte-identically to unset —
  falsifiable there as soon as the filter exists. Phase 5 **strengthens** that same
  case to the named refusal rather than "moving" it: an earlier draft said move, which
  left Phase 1b with a named criterion (`cargo test … empty_plugin_root`) for a test
  that no longer existed. One case, two phases, each asserting what is observable at
  its point.

  Note the filter belongs at **one choke point**, not per-composer:
  `cli/visualiser/server/src/main.rs:69` has its own bare `var_os` with no filter, so
  an empty root there yields `Some("")`, `plugin.join("templates")` becomes the
  *relative* path `templates`, and the server resolves plugin-default templates
  against its process cwd. Apply the emptiness rule in `with_plugin_root`
  (`store.rs:58`) — dropping an empty `PathBuf` — so the launcher, the server and any
  future composer inherit it from one place.

**File**: `cli/launcher/tests/version.rs`
**Changes**: `version` with no root still exits 0 — the lazy closure guarantees
that. But do **not** add "external-subcommand dispatch still succeeds with no
root": it does not, and `:166-183`
(`an_unresolvable_subcommand_exits_non_zero_with_a_named_step`) already asserts the
opposite in this very file. An external subcommand still calls
`cache_root::resolve`, which fails closed with neither a root nor an
`ACCELERATOR_CACHE_DIR`.

The invariant worth pinning is that it fails at the *cache-root* step, not the
plugin-root step — and the renamed message assertions do **not** currently pin it:
`:166-183` and `cache_root.rs:132-134` assert only that stderr contains the variable
name, which the new `PluginRootUnavailable` message also does, so the two named
errors are interchangeable to the test. Tighten both to a step-distinguishing
substring only `CacheRootUnavailable` emits — e.g. the "no `ACCELERATOR_CACHE_DIR`
override was given" clause.

### Success Criteria

#### Automated Verification

Hand-run criteria assume the repository root as cwd and a host-native debug build
(`mise run build:cli:dev`); write the binary as
`"$(git rev-parse --show-toplevel)"/cli/target/debug/accelerator` if
`CARGO_TARGET_DIR` may be set. The hermetic equivalents are the `config_read.rs`
cases, which pin cwd via `run_in` and are what CI actually runs.

- [x] `cli/` workspace tests pass: `mise run test:unit:cli` — including the 47
      untouched `run_in` cases and both rootless `summary` cases *(586 passed)*
- [x] The new variant's `Display` names the variable, asserted as a unit test in
      `cli/config/src/error.rs`'s existing per-variant block —
      `plugin_root_unavailable_names_the_variable_and_the_bootstrap`
- [x] Every `ConfigError` variant is classified: adding one without extending
      `is_refusal()` fails to compile. **Confirmed** by adding a throwaway variant
      mid-phase: `error[E0004]: non-exhaustive patterns` at the `is_refusal`
      match. Classification of the four variants the plan's snippet omitted
      (`PathConflict`, `MalformedFrontmatter`, `InvalidKey`, `UnsafePath`) follows
      the wildcard it replaced — degradable — so no existing behaviour moved.
- [x] A rootless `config templates list` is a named error rather than an empty
      table, exiting non-zero with a diagnostic naming the variable —
      `templates_list_with_no_plugin_root_is_a_named_refusal_not_an_empty_table`
- [x] The same command **with** `--fail-safe` behaves identically — non-zero, empty
      stdout, diagnostic on stderr — because the failure is a `Refusal`
- [x] `config template adr` with no root still exits non-zero with and without
      `--fail-safe`, as it does today, with a better message. Both this and the
      criterion above are one parametrised case,
      `a_missing_plugin_root_refuses_identically_with_and_without_fail_safe`,
      covering `templates list`, `template <name>` and `templates show` and
      asserting **byte-identical stderr** across the flag — which is the property
      that separates the `Refusal` classification from a `Read` one.
- [x] A rootless `config templates eject --all` is a named error and writes no
      file, where today it exits 0 having ejected nothing —
      `templates_eject_all_with_no_plugin_root_writes_nothing`, plus
      `the_unflagged_template_commands_name_the_variable_with_no_plugin_root` for
      `eject <name>`, `diff` and `reset`
- [x] **A rootless `config template <name>` still resolves a user override** — the
      criterion that pins the tier check being in place rather than hoisted:
      `a_user_override_still_resolves_with_no_plugin_root`
- [x] A root that exists but has no `templates/` still renders a header-only table
      at exit 0 (characterisation — the deferred residue, visible not implicit) —
      `a_root_without_a_templates_directory_still_renders_an_empty_table`
- [x] The root-independent families still succeed rootless, `config summary` among
      them: `config paths` and `config summary` both exit 0 with non-empty stdout,
      run against a stated fixture project — the `summary` fixture, in
      `the_root_independent_families_still_succeed_with_no_plugin_root`
- [x] `version` with no root still exits 0 — already covered: `version.rs`'s `run`
      helper strips `ACCELERATOR_PLUGIN_ROOT` from every case in the file
- [x] An external subcommand with no root fails at the **cache-root** step, pinned
      by a substring only that error emits — `no ACCELERATOR_CACHE_DIR override
      was given`, tightened at `version.rs`'s
      `an_unresolvable_subcommand_exits_non_zero_with_a_named_step` and at
      `cache_root.rs`'s `unset_plugin_root_with_no_override_is_a_named_error`
- [x] An empty-string root behaves identically to an absent one, for the launcher
      **and** the visualiser server (one filter in `with_plugin_root`) —
      `an_empty_plugin_root_behaves_as_an_unset_plugin_root` (strengthened to the
      named refusal) and the server's `an_empty_plugin_root_refuses_to_compose`
- [x] Architecture rules hold: `mise run pup:check`
- [x] The Phase 4 conformance suite still passes:
      `mise run test:integration:skill-invocation` *(128 passed)*
- [x] `mise run check` exits 0 — including `missing_errors_doc` on the newly
      fallible port
- [x] `mise run` (bare default) exits 0 end-to-end

#### Manual Verification

- [x] A rootless launcher reached via `ACCELERATOR_BIN` prints a diagnostic naming
      `ACCELERATOR_PLUGIN_ROOT` for `templates list` instead of an empty table.
      **Confirmed**: exit 1 with `the plugin installation root is unknown: set
      ACCELERATOR_PLUGIN_ROOT, or invoke accelerator through bin/accelerator,
      which derives it`, run from a bare `mktemp -d` with both variables stripped.
- [x] Nothing reaches stdout on that path, so no resolved path could be spliced
      into a prompt. **Confirmed**: 0 bytes, with and without `--fail-safe`.
- [x] A `!` site still cannot reach this failure, because the bootstrap always
      exports a root — confirm by checking one skill loads normally after the
      change. **Discharged by automation instead**: Phase 4's conformance suite
      runs all 122 distinct `config` `!`-site commands through the real bootstrap
      with both variables removed from the environment, and all 128 cases stay
      green. That covers every skill rather than one, so a hand check adds
      nothing.

### Implementation notes

Four departures from the section above.

- **`is_refusal` is `pub`, not `pub(crate)`.** The section's snippet says
  `pub(crate)`, but its sole consumer — `From<ConfigError> for Failure` — lives in
  the `accelerator` launcher crate, so `pub(crate)` on a method of `config` makes
  it unreachable and the delegation cannot compile. The property the section
  actually wants is *exhaustiveness inside the defining crate*, which the
  `match` provides regardless of the visibility of the method wrapping it.
- **Nine variants to classify, not five.** The snippet omits `PathConflict`,
  `MalformedFrontmatter`, `InvalidKey` and `UnsafePath`. All four take the
  `false` arm, which is what the wildcard they replace already gave them, so no
  existing classification moved and no existing test changed. `UnsafePath` is
  the one worth naming: it *reads* like a refusal, and
  `paths_doc_types_stays_fail_closed_on_escape_with_fail_safe` proves that path
  is already fail-closed — but via `ConfigError::Invalid` raised in
  `config::paths`, not via `UnsafePath`, so classifying it `true` would have
  been a behaviour change dressed as a tidy-up.
- **`plugin_default`'s and `eject`'s rewrites take the `if !path.is_file()`
  shape the section predicts, but `plugin_template_path` keeps no `Option`
  seam at all** — it is `Result<PathBuf, ConfigError>` outright, so the two
  consumers each read `let path = self.plugin_template_path(name)?;`. The
  `EjectOutcome::NoDefault` consequence the section flags is real and now
  observable: with no root, `eject` refuses before it can report it.
- **The server's half of the empty-root criterion is a `compose_contract.rs`
  case, not a subprocess test.** `an_empty_plugin_root_refuses_to_compose`
  calls `compose::load` with `PathBuf::new()` and asserts the error names the
  variable. The server's own `var_os` at `main.rs:69` is left unfiltered as the
  section intends — `store.template_names()?` refuses first, so `load` fails
  before `plugin_root.join("templates")` can become a cwd-relative path. Absent
  and empty are therefore not byte-identical at the server: absent exits 2 at
  `main.rs` naming the variable, empty exits 2 via `failed to compose config`
  also naming it. Both refuse to start; only the launcher achieves literal
  identity.

---

## Closing step: remove `mise.local.toml`

### Overview

Not an implementation phase, and **the plan reaches "all phases complete" before
this step.** It is a local-working-copy cleanup with a precondition outside the
plan's control: nothing about it is mergeable, because the file is on no pushed
branch, so deleting it changes no CI job, no release and no other contributor's
checkout. Treat it as a line in the closure sequence (Migration Notes) and in the
`meta/validations/` note, not as work that gates the phases.

**Precondition: a prerelease carrying the Phase 1a fix is installed and in use.**
Until then this file is what supplies `CLAUDE_PLUGIN_ROOT` to the installed
plugin's still-unfixed bootstrap in this repository, and every CLI-invoking skill
— including the ones used to carry out this plan — depends on it.

### Why keeping it this long costs nothing

The file's only hazard is masking the bug class in local test runs, and **Phase 1b
removes that hazard without deleting it.** `accelerator_env()`
(`tasks/test/helpers.py:36`) currently reads
`os.environ.get("CLAUDE_PLUGIN_ROOT", str(repo))`, which the file overrides toward
the installed cache. Phase 1b renames that read to `ACCELERATOR_PLUGIN_ROOT`,
which the file does not set, so the overlay falls back to the repo root from 1b
onward. The working-tree bootstrap self-locates and ignores ambient values by
construction from 1a. After 1b the file therefore influences exactly one thing:
the installed plugin's bootstrap. Deletion becomes hygiene rather than a fix.

Note the ordering subtlety: during 1a the overlay still reads the old name, so the
file still redirects the shell suites toward the installed cache. That is harmless
— 1a's own tests are the fixture-rooted entrypoint suite, which the overlay does
not touch — but it means the local masking ends at 1b, not 1a.

### Changes Required

**File**: `mise.local.toml`
**Changes**: Delete, and confirm it is absent from the working-copy commit.

The per-push hazard is already handled by mechanism rather than ritual:
`.gitignore:26` lists `mise.local.toml`. That entry was **new in the working copy**
when this was written, not pre-existing, so it is part of this work — and it has
since landed. With it in place the file cannot be accidentally `git add`ed, and
jj no longer snapshots it: it is now untracked rather than sitting in the
working-copy commit, so the only thing deletion still removes is the on-disk file
feeding the installed plugin's bootstrap.

### Success Criteria

#### Automated Verification

- [ ] The file is gone: `test ! -e mise.local.toml`
- [x] The ignore entry landed: `grep -qx 'mise.local.toml' .gitignore` —
      `.gitignore:26`, and the file is untracked as a result
- [x] It is on no branch that can reach CI:
      `! git cat-file -e main:mise.local.toml 2>/dev/null`
- [ ] The shell suites still pass with no ambient root:
      `mise run test:integration:work test:integration:config`
- [ ] `mise run` (bare default) exits 0 end-to-end

#### Manual Verification

- [ ] With the file deleted, `/accelerator:commit` and one other CLI-invoking
      skill load without error against the **installed** plugin — the direct
      confirmation that the fix reached the artifact
- [ ] No permission prompt appears for either skill

---

## Testing Strategy

### Unit Tests

- The export-versus-reader coherence assertion in `test_bootstrap_coverage.py`,
  so a one-sided rename fails in seconds rather than as a missing sentinel deep in
  an integration suite (Phases 1a, 1b).
- The lint guard's every branch over `tmp_path` trees, including a
  `bin/accelerator` reintroduction and an out-of-tree writer, the fail-closed
  empty-discovery branch, the scanned-file floor, and
  `violations(REPO_ROOT) == []` (Phase 2).
- `walk_files` in its own `tests/unit/tasks/shared/test_sources.py`: gitignored
  trees pruned, `workspaces/` unchanged, every `_BUILD_OUTPUT` entry pruned — in
  particular nothing under `cli/visualiser/frontend/dist/` — and `prune` adding
  rather than replacing. Plus the `.venv` assertion added to
  `test_python_coverage.py` *before* the refactor (Phase 2).
- `test_mise.py`'s `_CHECK_GATES`, the new `_CLI_CHECK_GATES`, and the
  exhaustively-pinned `_LAUNCHER_DEPENDENTS` / `_NO_LAUNCHER_NEEDED` split
  (Phases 1c, 2, 4).
- `TestHooksSuiteGuard` in `test_integration.py`, the paired guard the new floor
  would otherwise be the only one to lack (Phase 3).
- One test per changed store consumer, each asserting a non-zero exit, empty
  stdout and a variable-naming diagnostic **identically with and without**
  `--fail-safe` (the failure is a `Refusal`), plus the relocated empty-string case
  (Phase 5).

### Integration Tests

- The entrypoint suite end to end against a fixture installation tree, with an
  env-dumping launcher stub for the root-derivation cases and the real compiled
  launcher served through the stubbed downloader for the two rendering cases:
  rootless template render, the reported `instructions` command, the
  `templates list` row, the two-hop symlink chain, ambient-root rejection,
  relative link targets, a dash-leading link target, a derived root that is not an
  installation, a 17-link over-bound chain, cycle termination, and trust-chain
  degradation with its durable record (Phase 1a).
- The work-item shell suite's re-pointed seam, with its distinguishable-values
  pair (Phase 1c).
- The hook suite: refresh, re-point, a regular file at the destination, a
  symlink-to-directory at the destination, `CLAUDE_PLUGIN_ROOT` unset, a relative
  `${CLAUDE_PLUGIN_DATA}`, inertness asserted on the hook's decision, the temp-tree
  snapshot diff, and content-based `hooks.json` registration (Phase 3).
- All 204 extracted `config` commands, plus `visualiser status`, against the
  fixture installation root and fixture project with both variables stripped and
  `shell=False` (Phase 4).

### Key Edge Cases

- A relative symlink target, resolved against the link's own directory rather
  than the cwd.
- A link target beginning with `-`, where `dirname` would error and print nothing,
  `cd ""` would succeed as a no-op, and `pwd -P` would yield the cwd — making the
  session's project directory the resolved root. Same failure shape for a path whose
  only slash is the leading one, which is why `dir_of` has its `${dir:-/}` arm.
- **A directory symlink in the entry path**, where a logical `cd ..` yields the
  link's parent rather than the target's — measured: `other/bin -> real/bin` gives
  `other` logically and `real` with `-P`. This is the one shape in which the genuine
  bootstrap adopts a foreign root without an attacker planting an executable.
- An existing `${CLAUDE_PLUGIN_DATA}/bin/accelerator` that is a symlink **to a
  directory**, where `mv -f` moves the replacement *inside* it (measured on Darwin,
  and the same on GNU, with no portable `-T` opt-out) while exiting 0.
- `${CLAUDE_PLUGIN_DATA}/bin` unwritable, where `mkdir -p` succeeds on the existing
  directory and `ln` is what fails — the path where the hook must diagnose rather
  than fail silently or leave an orphan.
- A **real directory** at the staging path, where `ln -sfn` *succeeds* and links
  inside it (measured: `-n`/`-h` covers only symlinks-to-directories) and `rm -f`
  cannot clean it up — hence the pre-check and the `rm -rf` trap.
- `CDPATH` exported, where `cd` prints the resolved directory into the command
  substitution and `plugin_root` becomes two lines.
- A 17-link non-cyclic chain, which the kernel resolves and the in-script bound of
  16 rejects. A **true cycle is not testable** — the kernel returns `ELOOP` when
  bash opens the path, so only termination and a non-zero status are observable;
  the prior review's `bash "$CYCLE_A"` conclusion does not hold.
- A `readlink` that fails between the `-L` test and the read — a real window, since
  the Phase 3 hook rewrites the middle hop at every `SessionStart`.
- A newline reaching a durable-record message — most reachably via
  `ACCELERATOR_CACHE_DIR`, which the `:168` message interpolates and which nothing
  validates. Handled by sanitising at the write site rather than by validating each
  input, so a future interpolated value cannot reopen it.
- An empty-string `ACCELERATOR_PLUGIN_ROOT`, which today becomes `Some("")` at
  `main.rs:176` for want of the filter `cache_root.rs:27` has. Asserted at the
  launcher level in Phase 1b, since the bootstrap always exports a non-empty
  derived root and cannot reach the branch.
- `${CLAUDE_PLUGIN_DATA}` unset **or relative**, where composing a path before the
  guard yields an absolute `/bin` target or a path inside the user's project.
- An existing symlink-to-a-directory at the link destination, where `ln -sf`
  without `-n` writes *inside* it while still reporting success.
- A `noexec` cache directory, already covered by `probe_dir` and unchanged.

### Manual Testing Steps

Steps 1–2 gate the **Phase 1a** prerelease and are the pre-condition for deleting
`mise.local.toml`. Steps 3–4 belong to Phase 3 and can be performed against a
later artifact.

1. Build a signed candidate artifact carrying Phase 1a and clean-install it, with
   no `mise.local.toml` and neither variable exported into the shell.
2. In permission mode `default` with no broad Bash allow rules, invoke
   `config/paths`, `integrations/linear/search-linear-issues`,
   `planning/create-plan` and `vcs/commit`. Confirm each loads **and** raises no
   permission prompt. Only skill load and prompt absence are under test — no
   Linear API call is needed, so no tracker credentials are required. This
   re-confirms the Phase 0 substitution answer against the release artifact; it is
   no longer a discovery gate, so a prompt here would indicate a regression in
   Claude Code rather than unbounded new scope.
3. Verify `ls -l "${CLAUDE_PLUGIN_DATA}/bin/accelerator"` resolves, then follow the
   documented recipe verbatim — including the `mkdir -p` — on a machine where
   `~/.local/bin` does not already exist, and run `accelerator config paths` from a
   plain terminal in a **new login shell** (the Debian/Ubuntu `PATH` entry only
   appears there).
4. Upgrade the plugin, start a new session, and confirm the link re-points. Then
   start two sessions concurrently and confirm the link is never absent.
5. Record the outcome in a validation note under `meta/validations/` and
   reference it from the work item.

A prompt at step 2 would contradict the Phase 0 answer and mean Claude Code's
matcher changed between v2.1.220 and the tested version. That would raise a
`${CLAUDE_SKILL_DIR}` migration item and gate the **prerelease** on it; 0182 still
closes on its own criteria either way.

## Performance Considerations

The symlink chase adds at most 16 `readlink` calls plus one `cd -P` per
invocation, and in the overwhelmingly common case (no symlink) zero. `dir_of`
replaces a `dirname` subprocess per hop with parameter expansion. Negligible
against the existing shim hash and signature verification.

The Phase 2 guard must prune gitignored directories in place rather than
`rglob`ing, or it descends into `cli/target/` and
`cli/visualiser/frontend/node_modules/` — thousands of vendored files on every
`cli:check`. The unconditional `_BUILD_OUTPUT` prune matters for the same reason
and is not covered by the root `.gitignore`: `dist/` holds the minified SPA bundle
whenever the frontend has been built, which under `mise run` is always.

Phase 4 runs 204 subprocesses. The distinct set is 122, so deduplicating
identical command strings before execution keeps it comfortably inside a normal
integration-test budget. Phase 1a's fixture work is the more significant cost: the
real compiled launcher is copied into the stub server, signed, fetched and
re-staged per test, so it is reserved for the two tests that need the launcher's
rendering. The other cases use an env-dumping stub, which is both faster and a more
direct assertion of the export.

## Migration Notes

**Phase 1a can ship alone; Phase 1b must ship in lockstep with the launcher.**
1a's transitional dual export is precisely what removes the lockstep hazard from
the urgent fix: the already-published launcher reads `CLAUDE_PLUGIN_ROOT`, so 1a is
releasable on its own. After 1b the bootstrap exports only
`ACCELERATOR_PLUGIN_ROOT`, and a pre-rename launcher would find no root and
silently drop the plugin-default template tier — the silent-wrong-answer mode this
work removes.

**The version-keyed cache does not fully guarantee that.** The cache key is the
version in `.claude-plugin/plugin.json`, which covers a *released, version-bumped*
artifact but leaves three reachable paths, all silent:

- The dev override execs whatever `ACCELERATOR_LAUNCHER_BIN` names inside
  `cli/target/`, so a pre-rename build left by a branch switch is used as-is. Run
  `mise run build:cli:dev` before any dev-override validation.
- During development `plugin.json` does not change, so a warm
  `<root>/bin/accelerator-launcher-<same-version>-<platform>` is a cache **hit**
  (`bin/accelerator:246` re-verifies the signature and reuses it) and nothing
  invalidates it on a rebuild.
- A same-version re-publish leaves the old binary cached, because the key carries
  no content hash.
- **The documented visualiser-binary override sits outside the cache entirely.**
  `ACCELERATOR_VISUALISER_BIN` and the `visualiser.binary` config key
  (`docs/visualiser.md:61`, `skills/config/configure/SKILL.md:580`) pin an arbitrary
  server binary, and `cli/visualiser/server/src/main.rs:69-72` hard-exits with code
  2 on a missing root — so after 1b any pinned or locally built *pre-rename* server
  dies at launch with `CLAUDE_PLUGIN_ROOT is not set`. The `ACCELERATOR_BIN`
  convention is the mirror case: it bypasses the bootstrap, so nothing exports a
  root at all, which is exactly why `tasks/test/helpers.py:36` must be renamed
  rather than dropped. Remedy for both: rebuild or re-fetch the pinned binary; for
  `ACCELERATOR_BIN` consumers, *rename* their exported root rather than removing it.

Phase 1a's `test_bootstrap_coverage.py` assertion is the cheap guard against the
in-tree half of this: it pins that the name the bootstrap exports is the name the
launcher reads, so a one-sided rename fails as a fast unit test. The cross-release
half is handled by 1a being independently shippable and 1b riding with the
artifact.

The closure sequence is: merge 1a → build a signed candidate artifact →
clean-install it → run the manual pre-release check → publish → delete
`mise.local.toml` → merge 1b and the remaining phases.

**`mise.local.toml` must not ride along into a pushed commit.** It exists only in
the jj working-copy commit and is absent from `main`, so any branch cut from the
current working copy would carry it unless excluded. If it reached CI it would
point `CLAUDE_PLUGIN_ROOT` at `/Users/tobyclemson/.claude/plugins/cache/…`, a path
no runner has. The guard is mechanical rather than a per-push ritual:
`.gitignore:26` now lists it (a working-copy addition that lands with this work),
so git cannot accidentally add it and jj will not snapshot it unless force-tracked.

**Consumers who exported `CLAUDE_PLUGIN_ROOT` as a workaround should remove it.**
The earlier claim that they are "unaffected" is wrong, though the blast radius is
narrower than a first pass suggested. The CLI ignores the variable after 1b, but the
migration runner reads it as a **higher-precedence override**, not a fallback:
`skills/config/migrate/scripts/run-migrations.sh:6` uses
`"${CLAUDE_PLUGIN_ROOT:-$(self-locate)}"` and `:643` exports it into every migration
child, which is what satisfies `scripts/interactive-harness.sh:29`'s hard `:?` read.
`run-migrations.sh` runs under the Bash tool, where Claude Code supplies no value of
its own — so after upgrading, a stale version-pinned export silently makes it source
`scripts/config-common.sh`, `atomic-common.sh` and the entire `migrations/` set from
a *previous* plugin version, with no signal, because the CLI now works regardless.

The two `hooks/` readers (`config-detect.sh:10`, `migrate-discoverability.sh:23`)
are **not** at risk despite the same `:-` shape: hook processes are the one surface
Claude Code does export the variable to, so its correct value wins over anything
inherited from the user's shell. Phase 1a's transitional export does not change
either picture — it lands only in the exec'd launcher's process image and never
propagates back to the invoking shell.

Carry the removal advice into the release notes for the version shipping 1b.

## References

- Original work item: `meta/work/0182-cli-derives-plugin-root-from-own-location.md`
- Work item review: `meta/reviews/work/0182-cli-derives-plugin-root-from-own-location-review-1.md`
- Plan review (eight lenses; drove the 1a/1b/1c split, Phase 5's re-siting, the
  hop-bound change, the guard's scope, and the launcher-supply mechanism):
  `meta/reviews/plans/2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename-review-1.md`
- Implementation-surface research:
  `meta/research/codebase/2026-07-27-0182-plugin-root-self-location-implementation-surface.md`
- Root cause analysis:
  `meta/research/issues/2026-07-26-cli-requires-claude-plugin-root-env-var.md`
- Symlink-chase prior art and its review:
  `meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md:620-655`,
  `meta/reviews/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding-review-1.md:392,738,757,835,856`
- Prior `${CLAUDE_PLUGIN_DATA}` decision Phase 3 reverses:
  `meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md:125`
- `meta/decisions/ADR-0048-four-toolchain-split.md` — why the Phase 2 guard is a
  Python invoke task
- `meta/decisions/ADR-0049-bash-3.2-compatibility-floor.md` — the constraint on
  the Phase 1a chase
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md` — the boundary
  Phase 2 enforces
- Lint-guard template: `tasks/lint/store_duplication.py`,
  `tests/unit/tasks/test_store_duplication.py`
- Hook template: `hooks/migrate-discoverability.sh`,
  `hooks/test-migrate-discoverability.sh`
- Related: 0164 (introduced the bootstrap), 0167 (routed the skills through it),
  0136 (the parent Rust CLI migration epic)
