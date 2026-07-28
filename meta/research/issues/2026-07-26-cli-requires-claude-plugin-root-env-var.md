---
type: issue-research
id: "2026-07-26-cli-requires-claude-plugin-root-env-var"
title: "Investigation: bin/accelerator hard-requires CLAUDE_PLUGIN_ROOT in the environment, which Claude Code only exports to hooks"
date: "2026-07-26T18:27:02+00:00"
author: "Toby Clemson"
producer: research-issue
status: complete
work_item_id: "0182"
parent: "work-item:0182"
topic: "Every skill that calls the CLI fails with 'accelerator: CLAUDE_PLUGIN_ROOT is not set' because the bash bootstrap reads the plugin root from the environment rather than deriving it from its own location"
tags: [research, debugging, cli, launcher, bootstrap, plugin-root, allowed-tools]
revision: "e8822dd5eafc8ab68df66d7c9858f0eb702a9633"
repository: "accelerator"
last_updated: "2026-07-26T18:27:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Investigation: bin/accelerator hard-requires CLAUDE_PLUGIN_ROOT in the environment, which Claude Code only exports to hooks

**Date**: 2026-07-26 18:27 UTC
**Author**: Toby Clemson
**Git Commit**: e8822dd5eafc8ab68df66d7c9858f0eb702a9633
**Branch**: (jj working copy, no bookmark)
**Repository**: accelerator

## Issue Description

After the CLI was introduced and skill logic moved into the launcher, skills that
invoke the CLI fail. Two reported forms:

**1. Bash-tool invocation** — `/accelerator:configure templates list` ran a shell
command that failed; the model diagnosed "The launcher needs CLAUDE_PLUGIN_ROOT
set" and retried with an explicit env-var prefix, which then triggered a
permission prompt despite the skill's `allowed-tools` rule:

```
$ CLAUDE_PLUGIN_ROOT=/Users/…/accelerator/1.24.0-pre.16 \
    /Users/…/accelerator/1.24.0-pre.16/bin/accelerator config templates list
…
 This command requires approval
```

**2. `!` preprocessor invocation** — `/accelerator:commit` failed at skill load:

```
Error: Shell command failed for pattern
  "!`/Users/…/1.24.0-pre.16/bin/accelerator config instructions commit --fail-safe`":
  [stderr] accelerator: CLAUDE_PLUGIN_ROOT is not set
```

The reporter's hypothesis: the CLI requires `CLAUDE_PLUGIN_ROOT` in its
environment, and it would be preferable for the launch script and CLI to deduce
the plugin root from their own location. An alternative hypothesis was offered: a
Claude Code upgrade coinciding with the CLI release.

## Input Classification

Mixed — two concrete error strings with exact commands, plus a behavioural
description ("there are likely other errors in other skills") and two competing
suspected mechanisms.

## Affected Components

- `bin/accelerator:25` — `[[ -n "${CLAUDE_PLUGIN_ROOT:-}" ]] || fail
  "CLAUDE_PLUGIN_ROOT is not set"`. The abort site. Both reported errors
  originate here, before any Rust code runs.
- `bin/accelerator:26-27,44,70,73,101,117,122` — seven further uses of the
  variable (directory check, `plugin.json` version read, verify-shim path,
  release public key, cache dir, dev-launcher marker, `cli/target/`
  containment). All would need the same value.
- `bin/accelerator` (whole file) — contains **no** `BASH_SOURCE` reference; it is
  the only shell entry point in the repo that cannot locate itself.
- `cli/launcher/src/main.rs:175-177` — `plugin_root()` reads the variable into an
  `Option`; `None` when unset. **Degrades silently**, does not error.
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs:26-28,58-64` — reads
  the variable; hard `CacheRootUnavailable` error when unset with no
  `ACCELERATOR_CACHE_DIR`. Reached only for external subcommands (the
  visualiser), not for `config`.
- `cli/visualiser/server/src/main.rs:69-72` — hard-fails with exit 2 when unset.
  Pre-dates the CLI migration.
- `hooks/config-detect.sh:10-13` — self-locates `PLUGIN_ROOT` with a
  `${CLAUDE_PLUGIN_ROOT:-<self-located>}` fallback, then `exec`s
  `bin/accelerator`. Works today only because hooks *do* receive the export.
- 43 SKILL.md files invoke the launcher from a `!` preprocessor site (see Blast
  Radius).

## Timeline / Reproduction

1. `bin/accelerator` was added on **2026-07-04** in `9bd5f3be7` ("Add the
   bin/accelerator bootstrap and the verify shim crate"), part of the 0164
   launcher work. This introduced the first hard `CLAUDE_PLUGIN_ROOT`
   environment dependency in the shell tier.
2. Claude Code substitutes `${CLAUDE_PLUGIN_ROOT}` **textually** into skill
   content, so `${CLAUDE_PLUGIN_ROOT}/bin/accelerator …` resolves to a correct
   absolute path — visible in error 2, where the pattern shows the fully
   expanded path.
3. Claude Code does **not** export `CLAUDE_PLUGIN_ROOT` into the environment of
   the Bash tool or of `!` preprocessor shells.
4. The bootstrap therefore starts with a correct `argv[0]` and an empty
   environment, hits line 25, prints to stderr and exits 1.
5. At a `!` site, a non-zero exit aborts skill loading entirely — the skill never
   reaches the model.

### Controlled reproduction

Run with the variable stripped (equivalent to the real invocation environment):

```
$ env -u CLAUDE_PLUGIN_ROOT \
    /Users/…/1.24.0-pre.16/bin/accelerator config instructions commit --fail-safe
accelerator: CLAUDE_PLUGIN_ROOT is not set
```

Byte-identical to the reported error 2 stderr. With the variable set, the same
command succeeds silently.

### Note on the reporter's local environment

The reporter added `mise.local.toml` to the repo:

```toml
[env]
CLAUDE_PLUGIN_ROOT = "/Users/…/accelerator/1.24.0-pre.16/"
```

Because the Bash tool's shell is initialised from the user's profile with mise
active, this makes the variable appear set in every accelerator-repo session,
masking the bug locally. It also explains the trailing slash observed in that
environment (Claude Code's own substituted value has no trailing slash). **Any
local experiment run from this repo will show the variable set and must not be
treated as evidence.** All findings below were established either outside the
repo, with `env -u`, or from a separate `claude` process.

### Clean-environment probe

A nested Claude Code session started outside the repo with the variable stripped:

```
$ cd <scratch-dir> && env -u CLAUDE_PLUGIN_ROOT claude -p \
    'run: printf "PLUGIN_ROOT=[%s]\n" "${CLAUDE_PLUGIN_ROOT:-UNSET}"; env | grep -c CLAUDE'
PLUGIN_ROOT=[UNSET]
8
```

Eight `CLAUDE*` variables are present in the Bash tool environment and
`CLAUDE_PLUGIN_ROOT` is **not** among them. Claude Code v2.1.220.

## Hypotheses

### Hypothesis 1: `bin/accelerator` treats a substitution-only variable as an environment input
- **Evidence for**:
  - `bin/accelerator:25` aborts on the unset variable with no fallback, and the
    file contains no `BASH_SOURCE` self-location anywhere.
  - The documented Claude Code contract
    ([plugins reference § Environment variables](https://code.claude.com/docs/en/plugins-reference))
    scopes the export narrowly: *"All three are exported as environment variables
    to hook processes and to MCP and LSP server subprocesses. Which fields
    substitute them inline depends on the plugin component"*, with the
    accompanying table giving **"Skill and agent content → Anywhere the
    placeholder appears"**. Skill content gets *substitution*; only hooks and
    MCP/LSP subprocesses get an *export*. The Bash tool and `!` preprocessor
    shells appear in neither list.
  - The clean-environment probe above confirms the absence empirically.
  - Error 2's pattern string shows the path correctly expanded while stderr
    reports the variable unset — substitution working, export absent, in the
    same invocation.
  - `env -u` reproduces the exact reported stderr byte-for-byte.
  - The pre-CLI shell tier never depended on this. Every script self-locates via
    `BASH_SOURCE[0]`; all ten non-test reads of the variable use a
    `${CLAUDE_PLUGIN_ROOT:-<self-located fallback>}` form
    (`hooks/config-detect.sh:10`, `hooks/migrate-discoverability.sh:23`,
    `skills/config/migrate/scripts/run-migrations.sh:6`, and
    `skills/config/migrate/migrations/000{1..7}-*.sh:6-7`). The single hard `:?`
    read (`scripts/interactive-harness.sh:29`) is satisfied by the migration
    runner's own self-derived value (`run-migrations.sh:643`), never by Claude
    Code.
- **Evidence against**: None found.
- **Verdict**: **Confirmed** — this is the root cause.

### Hypothesis 2: A Claude Code upgrade removed or changed the export
- **Evidence for**: The reporter observes the failures coinciding with the CLI
  release, and a Claude Code upgrade fell in the same window.
- **Evidence against**:
  - No changelog or documentation entry records `CLAUDE_PLUGIN_ROOT` ever being
    exported to the Bash tool or to `!` preprocessor shells; the documented
    export scope (hooks, MCP, LSP) is unchanged. The one nearby change,
    v2.1.207, restricts `${user_config.*}` in shell-form commands — unrelated.
  - Decisively: **no Claude Code change is required to explain the failure.**
    Before 0164, nothing in the invocation path read the variable from the
    environment, so the plugin was insensitive to whether it was exported. After
    0164, it aborts without it. The dependency was never satisfiable under the
    documented contract, so the regression is fully accounted for by the new
    code alone.
- **Verdict**: **Eliminated** as root cause. Cannot be excluded as a coincident
  event, but it is not load-bearing for this failure.

### Hypothesis 3: The env-var-prefix workaround defeats the `allowed-tools` rule
- **Evidence for**:
  - Skills declare `allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator
    config *)` (e.g. `skills/vcs/commit/SKILL.md:8`,
    `skills/config/configure/SKILL.md:7`). The rule is a prefix/glob match
    against the literal command string.
  - Prefixing `CLAUDE_PLUGIN_ROOT=… ` makes the command begin with the
    assignment, not the authorised path, so the prefix no longer matches — which
    is exactly the approval prompt in error 1.
  - Same mechanism already documented in this repo for the `bash` wrapper:
    `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`
    (Claude Code strips only `timeout`, `time`, `nice`, `nohup`, `stdbuf` before
    matching; env assignments are not stripped).
- **Evidence against**: None — but this is a consequence of the workaround, not
  of the original defect.
- **Verdict**: **Confirmed as a secondary effect.** It matters because it rules
  out "tell the model to set the variable" as an acceptable fix: that fix trades
  a hard failure for a permission prompt on all 43 skills.

### Hypothesis 4: A fix confined to the bootstrap would silently corrupt output
- **Evidence for**: With the variable unset, the launcher's own `config` path
  does not error — `plugin_root()` (`main.rs:175`) returns `None` and
  `cli/config-adapters/src/store.rs:343-352` silently skips the plugin-default
  template tier. Measured against the cached launcher binary directly:

  ```
  $ env -u CLAUDE_PLUGIN_ROOT <launcher> config templates list
  | Template | Source | Path |
  |----------|--------|------|
  (empty, exit 0)

  $ CLAUDE_PLUGIN_ROOT=<root> <launcher> config templates list
  | `adr` | plugin default | `<plugin>/templates/adr.md` |
  … 10+ rows
  ```

  `config instructions commit --fail-safe` likewise exits 0 with no output.
- **Evidence against**: None.
- **Verdict**: **Confirmed.** If `bin/accelerator` derived the root only for its
  own internal use and did not export it, `config templates list` would return an
  empty table and exit 0 — turning a loud failure into a silent wrong answer.
  The derived value **must** be exported.

## Root Cause

`bin/accelerator:25` requires `CLAUDE_PLUGIN_ROOT` to be present in its process
environment. Claude Code provides `${CLAUDE_PLUGIN_ROOT}` to skill content by
**textual substitution only**; it exports it as a real environment variable
solely to hook processes and MCP/LSP server subprocesses. Every skill invocation
of the CLI — whether through a `!` preprocessor site or the Bash tool — therefore
arrives with a correct absolute path in `argv[0]` and no such variable in the
environment, and the bootstrap aborts before any Rust code runs.

The bootstrap is the only shell entry point in the repository that does not
derive its own root from `BASH_SOURCE[0]`, and it is the sole choke point for CLI
access: the ~20 scripts that reach the CLI all do so via
`"${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}"` using their own
self-derived `PLUGIN_ROOT`. A script that needs no environment variable to find
itself hands off to a bootstrap that does.

Self-location is provably sufficient. The bootstrap always lives at
`<plugin-root>/bin/accelerator`, so `cd "$(dirname "$0")/.." && pwd -P` yields
the root in both deployment shapes:

| Invocation path | Derived root | `.claude-plugin/plugin.json` | verify shim | public key |
|---|---|---|---|---|
| `…/cache/…/accelerator/1.24.0-pre.16/bin/accelerator` | `…/accelerator/1.24.0-pre.16` | found | found | found |
| `<repo>/bin/accelerator` | `<repo>` | found | — | — |

## Causal Chain

1. 0164 moves skill logic behind a CLI reached via `bin/accelerator`, a bash
   bootstrap that reads the plugin root from `$CLAUDE_PLUGIN_ROOT`
   (`9bd5f3be7`, 2026-07-04).
2. A skill is invoked. Claude Code substitutes `${CLAUDE_PLUGIN_ROOT}` textually
   into the skill body and `allowed-tools`, producing a correct absolute path.
3. The command is executed by a `!` preprocessor shell or the Bash tool — neither
   of which receives the variable as an export, per the documented contract.
4. `bin/accelerator:25` finds `${CLAUDE_PLUGIN_ROOT:-}` empty, prints
   `accelerator: CLAUDE_PLUGIN_ROOT is not set` to stderr, exits 1.
5. At a `!` site, the non-zero exit aborts skill loading — the skill is
   unusable. `--fail-safe` cannot help: it is a launcher flag, parsed by Rust
   that never runs.
6. At a Bash-tool site, the model observes the error and self-corrects by
   prefixing the assignment.
7. The prefixed command no longer matches
   `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)`, so the user is
   prompted for approval on a call that was meant to be pre-authorised.

## Blast Radius

**43 skills** invoke the launcher from a `!` preprocessor site and therefore
**fail at load**, not merely mid-run:

`config/paths`, `decisions/create-adr`, `decisions/extract-adrs`,
`decisions/review-adr`, `design/analyse-design-gaps`, `design/inventory-design`,
`github/describe-pr`, `github/respond-to-pr`, `github/review-pr`,
`integrations/jira/{attach,comment,create,init,search,show,transition,update}-*`
(8), `integrations/linear/{attach,comment,create,init,search,show,transition,update}-*`
(8), `notes/create-note`, `planning/{create-plan,implement-plan,review-plan,stress-test-plan,validate-plan}`,
`research/{conduct-spike,research-codebase,research-issue}`, `vcs/commit`,
`visualisation/visualise`,
`work/{create,extract,list,refine,review,stress-test,update}-work-item*` (7).

Additionally: 47 SKILL.md files reference `bin/accelerator` somewhere in their
body (410 `${CLAUDE_PLUGIN_ROOT}` occurrences across 48 files), and ~20 helper
scripts under `skills/*/scripts/` invoke the CLI through their self-derived
`PLUGIN_ROOT`.

**Not affected**: the two `hooks/` entry points. `hooks/config-detect.sh` and
`hooks/migrate-discoverability.sh` run as hook processes, which *do* receive the
export, and they additionally self-locate. This is why SessionStart config
detection keeps working while nearly every skill is broken — and it means the
reporter's suspicion that a hook was the cause of the `research-issue` failure is
not borne out: `research/research-issue` is itself one of the 43 skills with a
failing `!` site.

## Contributing Factors

- The bootstrap's `fail()` writes to stderr and exits 1 unconditionally,
  ignoring `--fail-safe`. The `--fail-safe` contract at every `!` site exists
  precisely so a config-tier problem degrades quietly, but it is implemented in
  Rust that never runs. A bootstrap-tier failure is therefore maximally loud.
- `bin/accelerator` is deliberately self-contained (it sources nothing, being the
  root of trust), which is also why it did not inherit the repo-wide
  `BASH_SOURCE` self-location idiom.
- No test covers the real invocation environment. `cli/launcher/tests/*` and the
  shell suites all set `CLAUDE_PLUGIN_ROOT` (or `ACCELERATOR_BIN`) themselves, so
  the one configuration that matters in production — path correct, environment
  empty — is never exercised.
- `mise.local.toml` in the working repo sets the variable, so the bug is
  invisible to any local test run from the repository.
- The launcher's `config` path degrades silently rather than erroring when the
  root is absent, so the bootstrap's hard failure is currently the only signal
  that anything is wrong.

## Fix Options

| Option | Description | Risk | Effort |
|--------|-------------|------|--------|
| A | `bin/accelerator` self-locates (`cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P`) as the sole source of truth, validates the result, and **exports** it as `CLAUDE_PLUGIN_ROOT` before `exec`ing the launcher so the launcher and every sub-binary inherit it. The existing negative test seam moves to `ACCELERATOR_PLUGIN_ROOT` (4 one-line edits) — see Recommended Fix item 2 for why the incoming env var should not win. | Low | Low |
| B | A, plus defence in depth in Rust: derive the root from `std::env::current_exe()` in `cache_root.rs` and `visualiser/server/src/main.rs` when the variable is absent. | Med — the cached launcher sits at `<root>/bin/`, so the walk-up holds only for the default cache dir and breaks under `ACCELERATOR_CACHE_DIR` | Med |
| C | Add `CLAUDE_PLUGIN_ROOT=…` to every skill's documented invocation and broaden `allowed-tools` to authorise the assignment-prefixed form. | High — 43 skills, and per Hypothesis 3 it converts hard failures into permission prompts | High |
| D | Ship `mise.local.toml`-style guidance telling users to export the variable themselves. | High — per-machine, version-pinned path, no fix for consumers | Low |
| E | A (with symlink-aware self-location), plus **eradicate `CLAUDE_PLUGIN_ROOT` from `cli/` entirely** in favour of an `ACCELERATOR_`-prefixed variable, so an installed `accelerator` is invocable from a terminal with no caller-set environment. Supersedes A. | Low | Med — 3 production sites, 9 test/tooling files, 2 error-message contracts |

## Recommended Fix

**Option E** — Option A as the mechanical fix, carried out as part of removing
`CLAUDE_PLUGIN_ROOT` from `cli/` altogether (see Scope Extension below, which
records the maintainer's standalone-runnability goal and the full removal set).
Option A alone is a sufficient and correct fix for the reported failure and can
ship on its own if the next prerelease is urgent; Option E addresses the boundary
violation that allowed the failure.

The Option A core resolves the root cause at the single choke point, restores the
repo-wide convention that a script locates itself, and needs no change to the 43
affected skills or to `allowed-tools`.

Three details are load-bearing:

1. **Export, don't just use.** Per Hypothesis 4, deriving the root for the
   bootstrap's internal use alone would leave `config templates list` returning an
   empty table at exit 0. The `exec` at `bin/accelerator:262` relies on the
   variable being in the environment — its own comment says so ("`CLAUDE_PLUGIN_ROOT`
   stays in the env so the launcher resolves the same cache root") — and that
   assumption is currently satisfied by nobody. Exporting the derived value also
   fixes `cache_root.rs` and the visualiser server without touching them.
2. **Precedence: self-location wins; the override becomes a test seam.** The
   obvious shape is `${CLAUDE_PLUGIN_ROOT:-<derived>}` (as at
   `hooks/config-detect.sh:10`), but for this file the override should *lose*,
   and move to a distinct `ACCELERATOR_PLUGIN_ROOT` seam alongside the existing
   `ACCELERATOR_UNAME_S/_M` and `ACCELERATOR_BOOTSTRAP_DOWNLOADER` block:

   - **An environment value can be stale, and a derived one cannot.** The
     plugins reference states that when a plugin updates mid-session, "hook
     commands, monitors, MCP servers, and LSP servers keep using the previous
     version's path" until `/reload-plugins`, and the previous version's
     directory survives ~2 weeks. A new-version bootstrap honouring that stale
     export would read the old `plugin.json` version, seek the shim in the old
     tree, and cache/exec the **old launcher** — a silent version mismatch.
     `mise.local.toml`'s version-pinned value is the same hazard and will
     mis-resolve on the next prerelease bump.
   - **Only one caller needs the override, and it is a negative seam.**
     `skills/work/scripts/test-work-item-scripts.sh:1053,1086,1096,1106` sets
     `CLAUDE_PLUGIN_ROOT="/nonexistent"` to force template lookup to fail and
     assert the hardcoded fallback in `work-item-template-field-hints.sh`; it
     reaches the bootstrap via that script's line 53. Self-location would find
     the real templates and break those four assertions, so the seam must
     survive — under its own variable name (four one-line edits).
   - **Nothing else constrains the bootstrap.**
     `cli/launcher/tests/config_read.rs:1169` (temp-dir `plugin/`) and `:1205`
     (committed `FIXTURES/plugin`, `plugin_root()` at `:1192-1194`) point at
     synthetic roots but invoke `CARGO_BIN_EXE_accelerator` directly, bypassing
     the bootstrap entirely. `hooks/config-detect.sh:10` already self-locates,
     and self-location returns the same value the export would.

   Production then has exactly one source of truth for the root.
3. **Chase the script's own symlink, not just the directory path.** The
   repo-wide idiom `cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P` is
   **insufficient** for this entry point. `pwd -P` resolves symlinks among the
   *directory* components but not the final symlink to the script file itself, so
   an `accelerator` symlinked onto `PATH` — the natural way to satisfy the
   terminal-invocation goal — derives the symlink's parent instead of the
   installation root. Measured:

   ```
   # invoked via its real path
   BASH_SOURCE : …/accelerator/1.24.0-pre.16/bin/probe.sh
   naive root  : …/accelerator/1.24.0-pre.16          (plugin.json: FOUND)

   # invoked via a PATH symlink at <scratch>/userbin/accelerator
   BASH_SOURCE : <scratch>/userbin/accelerator
   naive root  : <scratch>                            (plugin.json: MISSING)
   chased root : …/accelerator/1.24.0-pre.16          (plugin.json: FOUND)
   ```

   The failure is loud (`plugin.json not found: <scratch>/.claude-plugin/plugin.json`)
   but it defeats the goal. Resolve with a `while [ -L "$src" ]` chase honouring
   relative link targets, then `cd -P`. Stay within the bash 3.2 floor — no
   `readlink -f`. The existing sourced-library sites need no change: they are
   never symlinked onto `PATH`.

Option B is worth considering separately as hardening for direct-launcher
invocations, but it is not needed to close this bug and carries a real
`ACCELERATOR_CACHE_DIR` edge case.

## Scope Extension: Remove Claude Code Coupling from `cli/`

Option A closes the reported bug. The maintainer has set a broader goal that
subsumes it: **eradicate `CLAUDE_PLUGIN_ROOT` from the `cli/` codebase entirely**,
replacing it with an `ACCELERATOR_`-prefixed variable. The rationale is
independent of this bug — an installed `accelerator` should be invocable from a
terminal without the caller having to set `CLAUDE_PLUGIN_ROOT`, and any
dependence on a Claude-Code-owned variable prevents that.

**Scope of the goal**: this is about *invocation context*, not distribution
shape. The installation remains the Claude Code plugin cache — so
`.claude-plugin/plugin.json`, `templates/`, `keys/`, and a writable `<root>/bin`
are always present, and the only thing that changes is that the caller may be a
human shell rather than Claude Code's Bash tool or a `!` preprocessor site.
Standalone packaging is explicitly **not** in scope.

This reframes the defect. The root cause is not merely "the bootstrap reads the
wrong input"; it is that a Claude Code integration concept leaked across the
boundary into the CLI, which then inherited Claude Code's environment contract as
a hard runtime requirement. Restated as a boundary rule:

> Claude Code concepts belong to the adapter layer — `hooks/`, `SKILL.md`
> substitution, and the derivation step inside `bin/accelerator`. Nothing under
> `cli/` may name, read, or require a `CLAUDE_*` variable. The CLI receives an
> installation root; it does not know who computed it or that a plugin exists.

`hooks/config-detect.sh:10` and `hooks/migrate-discoverability.sh:23` may keep
reading `CLAUDE_PLUGIN_ROOT` — they *are* the adapter layer, are the one surface
Claude Code genuinely exports to, and already carry self-locating fallbacks.
After self-location lands, `bin/accelerator` needs no `CLAUDE_*` read either.

### Complete removal set in `cli/`

**Production reads (3 sites):**

| Site | Current behaviour | Notes |
|---|---|---|
| `launcher/src/main.rs:176` (doc `:173`) | `Option`, `None` when unset | Silently drops the plugin-default template tier |
| `launcher/src/launch/outbound/resolve/cache_root.rs:26` (docs `:3,:38`; error text `:60`) | Hard `CacheRootUnavailable` | External subcommands only |
| `visualiser/server/src/main.rs:69-70` | `eprintln!` + exit 2 | Pre-dates the CLI migration |

**Tests and tooling (9 files) — all must be updated in lockstep:**

| Site | Kind |
|---|---|
| `launcher/tests/config_read.rs:62` | `env_remove` |
| `launcher/tests/config_read.rs:1169,1205` | injects a synthetic root |
| `launcher/tests/version.rs:22` | `env_remove` |
| `launcher/tests/version.rs:179` | **asserts stderr contains the literal `"CLAUDE_PLUGIN_ROOT"`** — the message text is an observable contract and changes with the rename |
| `launcher/.../cache_root.rs:132,134` | unit-test asserts on the same message text |
| `visualiser/server/tests/api_smoke.rs:32`, `shutdown.rs:19`, `orchestration_lifecycle.rs:55,274` | injects / removes the root |
| `visualiser/frontend/e2e/start-server.mjs:98` | injects the root for the E2E server |
| `corpus-adapters/tests/parity.rs:85` | comment reference only |

### Propagation needs no new plumbing (verified)

`UnixExec::exec` (`cli/launcher/src/launch/outbound/exec.rs:16-17`) is
`Command::new(program).args(args).exec()` with no `.env()` calls, so the child
inherits the environment wholesale. One `export` in `bin/accelerator` therefore
reaches the launcher and, through `exec`, the visualiser server. The launcher
cannot self-locate reliably — under `ACCELERATOR_CACHE_DIR` it is cached outside
the root — so the exported variable, not `current_exe()`, is the correct channel
(and this is why Option B is the weaker option).

### Precedence, reconciled

Recommended Fix item 2 argues the *incoming* `CLAUDE_PLUGIN_ROOT` must lose to
self-location because it can be stale. That reasoning does **not** transfer to
the new variable, and the two positions are consistent: staleness is a hazard of
a variable owned by an external system with its own lifecycle (mid-session plugin
update, a version-pinned `mise.local.toml`). An `ACCELERATOR_`-prefixed variable
set by our own bootstrap during the same invocation cannot be stale. So:

- `bin/accelerator` self-locates unconditionally and **exports** the result.
- Everything under `cli/` reads that variable, env-wins, no `CLAUDE_*` fallback.
- The same variable serves as the test-injection seam, since tests and the
  bootstrap are the only writers.

This collapses the earlier separate `ACCELERATOR_PLUGIN_ROOT` test seam into the
single production channel — one variable, one writer per invocation.

### Naming

Because the installation is always a plugin installation, "plugin root" remains
an accurate description of the thing being named — the objection is to the
variable's *ownership*, not its noun. **`ACCELERATOR_PLUGIN_ROOT`** is therefore
the recommendation: it is accurate, it keeps the old name greppable during the
migration, and it makes the ownership transfer obvious at every call site.
`ACCELERATOR_HOME` and `ACCELERATOR_ROOT` remain available if the term "plugin"
is considered undesirable on principle. This is a decision for the work item, not
a finding.

### What the invocation-context goal does and does not require

Because the install shape is unchanged, most of what standalone packaging would
have demanded is unnecessary. Specifically **not** required:

- **No XDG cache fallback.** `cache_root.rs:3-5` refuses XDG because "an
  XDG-resident binary would break the plugin-root `allowed-tools` glob match".
  `<root>/bin` always exists and is the correct cache under this goal, so the
  rule stands unchanged — only the variable it reads is renamed.
- **No new version source.** `bin/accelerator:44-49` reads the version from
  `.claude-plugin/plugin.json`, which is always present. Artifact naming is
  unaffected.
- **No change to the visualiser server's fatal exit** (`main.rs:69-72`). Reached
  via the launcher's `exec`, it inherits the exported root; the assertion is
  still correct and only the variable name changes.

What **is** required, beyond the rename itself:

1. **Symlink-aware self-location** in `bin/accelerator` — see Recommended Fix
   item 3. Terminal invocation means a `PATH` symlink, and the repo's usual
   `dirname`+`pwd -P` idiom silently derives the wrong root through one. This is
   the one genuinely new engineering requirement the goal introduces.
2. **Keep the export.** Per Hypothesis 4 the derived root must be exported, not
   merely used, or `config templates list` returns an empty table at exit 0.
   Unchanged by this narrowing, and the reason the fix cannot stop at the
   bootstrap.
3. **Visible degradation, as defence in depth.** With self-location in place the
   empty-table path becomes unreachable in normal use, so this drops from
   usability defect to diagnostic hygiene — worth doing because it is what made a
   bootstrap-only fix dangerous, but no longer urgent.

## Prevention

- **Boundary rule, enforced by lint**: nothing under `cli/` may name, read, or
  require a `CLAUDE_*` variable. A `grep`-based guard in the existing Rust check
  chain would make the boundary non-negotiable rather than conventional, and
  would have prevented the leak that caused this bug.
- **Convention, stated once**: no plugin entry point may require
  `CLAUDE_PLUGIN_ROOT` from its environment. It is a substitution token, not a
  runtime input; the authoritative source is the entry point's own location. Only
  hooks and MCP/LSP subprocesses may rely on the export at all.
- **Test the real invocation environment.** Add a launcher test that runs
  `bin/accelerator` with `env -u CLAUDE_PLUGIN_ROOT` and an absolute path, and
  asserts success. This single assertion would have caught the bug at 0164. The
  existing suites all inject the variable, so they structurally cannot.
- **Assert no silent degradation.** Add a test that `config templates list` with
  no plugin root either errors or returns the plugin defaults — never an empty
  table at exit 0.
- **Make `mise.local.toml` suspect.** Any environment variable in
  `mise.local.toml` that production is expected to supply masks a bug class in
  every local run. Either drop it once Option A lands, or note it as a known
  masking factor.
- **Extend the `allowed-tools` conformance idea** from the 2026-06-10 RCA to
  cover env-assignment prefixes, not just `bash`/`sh` wrappers — both defeat the
  prefix match identically.

## Recent Changes

```
54d2b9796  Add a gated local-build launcher override and content-address the verify shim
39c0e33a0  Trim comments across the launcher, verify shim, bootstrap, and tasks
9bd5f3be7  Add the bin/accelerator bootstrap and the verify shim crate   (2026-07-04, first add)
```

`cli/launcher/src/launch/outbound/resolve/cache_root.rs`:

```
7d7c321ab  Isolate cli test fixtures with self-cleaning temp dirs
39c0e33a0  Trim comments across the launcher, verify shim, bootstrap, and tasks
f84a46c83  Implement the real fetch → verify → cache launcher resolver
```

No commit in either history relaxes or adds a fallback for the environment
dependency — it has been hard since the file was created.

## Open Questions

- Is `${CLAUDE_PLUGIN_ROOT}` (as opposed to `${CLAUDE_SKILL_DIR}` /
  `${CLAUDE_PROJECT_DIR}`) still substituted in `allowed-tools` Bash rules? The
  plugins reference implies yes via "Skill and agent content"; the skills
  reference names only `CLAUDE_SKILL_DIR` and `CLAUDE_PROJECT_DIR` as the two
  variables substituted in "the skill's markdown content, and Bash rules in the
  `allowed-tools` frontmatter". The 2026-06-10 RCA proved empirically that
  plugin-root rules did match at that time. Worth re-confirming on v2.1.220,
  since if it has stopped, all 43 skills would prompt even after Option A.
- Should the bootstrap gain a `--fail-safe`-aware exit path so a bootstrap-tier
  failure degrades quietly at `!` sites rather than aborting skill load? This is
  a separable robustness question, but it is what turned this bug from
  "one feature degraded" into "43 skills unusable".
- **Naming**: `ACCELERATOR_PLUGIN_ROOT` (recommended — accurate, greppable),
  `ACCELERATOR_HOME`, or `ACCELERATOR_ROOT`?
- **Is terminal invocation a supported, tested surface or merely permitted?** If
  supported, the work item should add a test that runs `bin/accelerator` through a
  `PATH` symlink, since that is the shape that breaks and nothing currently
  exercises it. If merely permitted, the symlink chase is still worth having but
  needs no coverage.
- Should the plugin ship a documented way to put `accelerator` on `PATH`? The
  install path contains the version (`…/accelerator/1.24.0-pre.16/bin`), so a
  user-created symlink goes stale on every upgrade — the same version-pinning
  hazard as `mise.local.toml`, just relocated from an env var to a link target.
