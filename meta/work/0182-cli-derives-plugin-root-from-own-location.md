---
type: work-item
id: "0182"
title: "bin/accelerator requires CLAUDE_PLUGIN_ROOT in the environment (never exported to skills)"
date: "2026-07-26T21:28:26+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: bug
priority: high
blocks: ["work-item:0186"]
relates_to: ["work-item:0164", "work-item:0167", "work-item:0136", "work-item:0183", "work-item:0184", "work-item:0186", "codebase-research:2026-07-27-0182-plugin-root-self-location-implementation-surface"]
source: "issue-research:2026-07-26-cli-requires-claude-plugin-root-env-var"
tags: [bug, cli, launcher, bootstrap, plugin-root, skills]
last_updated: "2026-07-29T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0182: bin/accelerator requires CLAUDE_PLUGIN_ROOT in the environment (never exported to skills)

**Kind**: Bug
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

[`bin/accelerator:25`](../../bin/accelerator#L25) aborts unless
`CLAUDE_PLUGIN_ROOT` is present in its process environment. Claude Code provides
`${CLAUDE_PLUGIN_ROOT}` to skill content by **textual substitution only** — it is
exported as a real environment variable solely to hook processes and MCP/LSP
subprocesses, never to the Bash tool or to `!` preprocessor shells. Every skill
invocation of the CLI therefore arrives with a correct absolute path and no such
variable in the environment, and the bootstrap fails before any Rust runs. **43
skills fail at load.** The fix is for the bootstrap to derive the root from its
own location and export it as `ACCELERATOR_PLUGIN_ROOT`, replacing
`CLAUDE_PLUGIN_ROOT` throughout the CLI layers; alongside that, terminal
invocation of an installed `accelerator` becomes a supported, documented and
tested surface, and a bootstrap-layer failure at a `!` site degrades quietly
under `--fail-safe` instead of discarding the prompt. The one question that could
have expanded this into a 47-file `allowed-tools` migration — whether
`${CLAUDE_PLUGIN_ROOT}` still substitutes into `allowed-tools` Bash rules — was
resolved positively on 2026-07-27 (R11), so the rules stay as they are.

## Context

Introduced 2026-07-04 by `9bd5f3be7` ("Add the bin/accelerator bootstrap and the
verify shim crate") as part of 0164. `bin/accelerator` is the only shell entry
point in the repository that does not self-locate; every other script derives its
root from `BASH_SOURCE[0]` and treats `CLAUDE_PLUGIN_ROOT` as an *override* at
most (`hooks/config-detect.sh:10`, `hooks/migrate-discoverability.sh:23`,
`skills/config/migrate/scripts/run-migrations.sh:6`, migrations `0001`–`0007`).
The pre-CLI shell layer was insensitive to whether the variable was exported; the
bootstrap made it a hard requirement. A script that needs no environment variable
to find itself hands off to a bootstrap that does.

The bootstrap is the sole choke point for CLI access — the ~20 helper scripts
under `skills/*/scripts/` all reach it via
`"${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}"` using their own self-derived
root — so a single fix there covers every caller.

**Not a Claude Code regression.** No changelog or documentation records the
variable ever being exported to the Bash tool, and no upstream change is required
to explain the failure: before 0164 nothing in the invocation path read it.
Confirmed on Claude Code v2.1.220.

**Why it was not caught.** Every existing test injects `CLAUDE_PLUGIN_ROOT` (or
`ACCELERATOR_BIN`), so the one configuration that matters in production — path
correct, environment empty — is never exercised. Worse, two tests in
`tests/integration/entrypoint/test_accelerator_entrypoint.py` *assert* the
faulty behaviour as intended. A `mise.local.toml` in the working repo also sets
the variable, masking the bug in every local run; this was observed live while
drafting this item, where both `!` sites of the drafting skill itself succeeded
solely because of it.

**Scope decision.** Rather than only repairing the bootstrap, remove
`CLAUDE_PLUGIN_ROOT` from the CLI layers altogether in favour of
`ACCELERATOR_PLUGIN_ROOT`, so an installed `accelerator` is invocable from a
terminal without the caller setting anything. Terminal invocation is a
**supported surface**: documented, given a fixed non-version-pinned path, and
covered by a test. This is about *invocation context*, not distribution shape:
the installation remains the Claude Code plugin cache, so
`.claude-plugin/plugin.json`, `templates/`, `keys/` and a writable `<root>/bin`
are always present. Standalone packaging is **out of scope**.

## Requirements

### Layer vocabulary

Used consistently below and in Technical Notes; "tier" is not used as a synonym.

| Layer          | What it is                                                                             | Root handling after this change      |
|----------------|----------------------------------------------------------------------------------------|--------------------------------------|
| **bootstrap**  | `bin/accelerator`, the shell entry point shipped inside the plugin                     | Self-locates; writes, never reads    |
| **launcher**   | the fetched, verified, cached Rust binary the bootstrap `exec`s                        | Reads `ACCELERATOR_PLUGIN_ROOT`      |
| **server**     | the visualiser server, reached via the launcher's `exec`                                | Reads `ACCELERATOR_PLUGIN_ROOT`      |
| **adapter**    | `hooks/`, `skills/config/migrate/**`, migrations `0001`–`0007`, `scripts/interactive-harness.sh`, `scripts/test-design.sh`, and the `${CLAUDE_PLUGIN_ROOT}` substitution sites in `skills/**/SKILL.md` | Keeps reading **and writing** `CLAUDE_PLUGIN_ROOT` |

The adapter layer is not purely a reader: `skills/config/migrate/scripts/run-migrations.sh:643`
and `interactive-lib.sh:433,744` **write** `CLAUDE_PLUGIN_ROOT` into migration
children, which is what satisfies `scripts/interactive-harness.sh:29`'s hard
`:?` read. That is a self-contained shell→shell producer/consumer pair with no
Claude Code involvement, so it is correctly exempt — but the exemption is
stronger than "keeps reading an externally-owned variable" implies, and the
purge criterion must not treat those writes as violations. `agents/**` is
**not** in the adapter set: it contains zero `CLAUDE_PLUGIN_ROOT` occurrences
(an earlier draft listed it; the 410 occurrences are all in `skills/**/SKILL.md`
across 48 files).

"Plugin-default template tier" and "config precedence tier" retain their own
established meanings and are unrelated to the above.

"Harness" is likewise qualified everywhere it appears, because four different
things carry the name and they sit on opposite sides of the rename boundary: the
**dev harness** (`tasks/dev.py`, `tasks/shared/dev/circus.py`,
`tests/integration/dev/dev_integration_driver.py` — must rename), the **shell
test harness** (`tasks/test/helpers.py` — must rename), the **work-item shell
suite** (`skills/work/scripts/test-work-item-scripts.sh` — R4), and the
**interactive/design harness scripts** (`scripts/interactive-harness.sh`,
`scripts/test-design.sh` — exempt, adapter layer). Bare "harness" is not used.

### Reproduction

```
$ env -u CLAUDE_PLUGIN_ROOT \
    ~/.claude/plugins/cache/.../1.24.0-pre.16/bin/accelerator \
    config instructions commit --fail-safe
accelerator: CLAUDE_PLUGIN_ROOT is not set
```

Byte-identical to the reported failure. In a live session, `/accelerator:commit`
fails at load with `Error: Shell command failed for pattern "!`…`"`, and
`/accelerator:configure templates list` fails then prompts for approval when the
model retries with an `CLAUDE_PLUGIN_ROOT=…` prefix (the assignment defeats the
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)` prefix match — same
mechanism as `2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission`).

### Expected vs actual

|                                                   | Expected                                                           | Actual                                                                 |
|---------------------------------------------------|--------------------------------------------------------------------|------------------------------------------------------------------------|
| Skill `!` preprocessor site                       | CLI runs; output injected into the prompt                          | Non-zero exit aborts skill loading; skill unusable                     |
| Skill Bash-tool call                              | CLI runs under the existing `allowed-tools` rule                   | Fails; retry with env prefix escapes the rule and prompts              |
| Terminal invocation of an installed `accelerator` | Runs — a documented, supported surface reachable via a fixed path  | Fails unless the caller sets `CLAUDE_PLUGIN_ROOT`                      |
| `--fail-safe` at a `!` site                       | Degrades quietly whichever layer fails                             | Ignored at the bootstrap layer — it is a launcher flag; Rust never runs |

### Required changes

- **R1** — `bin/accelerator` derives the installation root from its own location
  as the sole source of truth, with **symlink-aware** resolution (see Technical
  Notes). No `CLAUDE_*` read remains in the file, and the bootstrap is
  **write-only** for `ACCELERATOR_PLUGIN_ROOT` — it never reads it, so no
  ambient value can redirect it.
- **R2** — The bootstrap **exports** `ACCELERATOR_PLUGIN_ROOT` before `exec`ing
  the launcher. Deriving it for internal use only is insufficient (see Technical
  Notes, "Why the export is mandatory").
- **R3** — Rename every participant in the launcher and server layers from
  `CLAUDE_PLUGIN_ROOT` to `ACCELERATOR_PLUGIN_ROOT`, **readers and writers
  together, wherever they live** — the rename set is defined by participation,
  not by directory, so it extends beyond `cli/`. Renaming the `cli/` readers
  alone breaks the dev harness and the shell suites, so the two halves cannot be
  separated. The enumerated set — production readers, test and tooling readers,
  the four out-of-tree writers, and the explicitly exempt sites — is in Technical
  Notes; that table, not the phrase "wherever they live", is the authority on
  membership.
- **R4** — The negative test seam at
  `skills/work/scripts/test-work-item-scripts.sh:1053,1086,1096,1106`
  (`CLAUDE_PLUGIN_ROOT="/nonexistent"`, forcing the hardcoded-fallback path) must
  keep forcing that path after the rename. The script under test,
  `skills/work/scripts/work-item-template-field-hints.sh`, never reads
  `CLAUDE_PLUGIN_ROOT` at all — it self-locates (`:11-12`) and calls
  `"${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}" config template work-item`
  (`:53`), falling back to hardcoded values only when that call exits non-zero
  (`hardcoded_fallback`, `:22-49`). The four assertions expect exactly the values
  `templates/work-item.md:8-10` carries, so if the CLI call ever succeeds they
  pass **vacuously** — green, and testing nothing.

  **The seam breaks under R3, not R1** (corrected from an earlier draft, which
  had the mechanism wrong in two ways):

  - Under `mise run` the seam **never reaches the bootstrap**.
    `tasks/test/helpers.py:33-37` sets `ACCELERATOR_BIN=cli/target/debug/accelerator`
    for this subtree, so `bin/accelerator` is bypassed entirely. The seam works
    because the *launcher* resolves `/nonexistent/templates/work-item.md`, gets
    `Ok(None)` from `cli/config-adapters/src/store.rs:343-353`, and refuses. The
    bootstrap's non-directory gate is only the standalone-invocation mechanism.
  - A rootless launcher does **not** exit 0 for this command. Measured against a
    freshly built launcher: `config template work-item` with no root exits **1**
    with `Template 'work-item' not found` on stderr, because
    `resolve_template` maps `None` to `Failure::Refusal`
    (`cli/launcher/src/config_command/inbound/cli.rs:238-247`) and `finish`
    degrades only `Failure::Read` (`:452-470`) — so it is fail-closed even under
    `--fail-safe`. Renaming the seam variable to
    `ACCELERATOR_PLUGIN_ROOT="/nonexistent"` therefore **would** restore it.

  So R1 alone leaves all four assertions intact, standalone or under `mise run`.
  Vacuity appears the moment R3 renames the launcher's reader **and**
  `tasks/test/helpers.py`'s writer together: the injected old name goes inert and
  the overlay hands the launcher the real repo root.

  The fix is unchanged, and better justified than the mechanical argument
  suggested: state the seam's actual intent — *make the CLI call fail* — by
  pointing the script's own documented override at a nonexistent binary,
  `ACCELERATOR_BIN="/nonexistent/accelerator"`. That is both bootstrap- and
  launcher-independent, and it is the **established convention** in this repo
  rather than a workaround: 28 files reach the CLI through
  `"${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}"`, and the test overlay sets
  `ACCELERATOR_BIN` precisely so suites never traverse the fetch/verify chain.
  Four one-line edits after all, but not the ones originally scoped, and
  sequenced against R3 rather than R1.
- **R5** — A regression test runs `bin/accelerator config templates list`
  (deliberately **without** `--fail-safe`, so a bootstrap-layer abort cannot
  masquerade as success under R8) with the ambient environment otherwise
  inherited and only `CLAUDE_PLUGIN_ROOT` and `ACCELERATOR_PLUGIN_ROOT` removed
  via `env -u`, invoked by absolute path. It asserts exit 0 **and** a listed row
  whose Source column is `plugin default`. That output assertion, not the exit
  status, is what makes it falsifiable: it fails today (hard abort) and would
  fail again if the R2 export were ever lost (empty table, exit 0). Its inverse
  is `test_unset_plugin_root_is_a_named_error`, which R3 deletes.
- **R6** — Remove `mise.local.toml` from the repo. It is inert once R1 lands and
  otherwise keeps masking this bug class locally. Confirmed with the maintainer
  as serving no other purpose, so this is unconditional.
- **R7** — A lint guard rejects any `CLAUDE_*` reference in tracked source under
  `cli/`, added as an invoke task under `tasks/lint/` alongside
  `skill_permissions.py` (this repo's lint guards are Python invoke tasks per
  ADR-0048 "Four-toolchain split",
  `meta/decisions/ADR-0048-four-toolchain-split.md` — **not** the
  Rust check chain). The guard walks the tree honouring `.gitignore` — so
  `cli/target/` and `cli/visualiser/frontend/node_modules/` are never scanned —
  and carries a negative test proving it fails when a reference is reintroduced.
  It also states the wider convention it partially enforces: **no plugin entry
  point may require `CLAUDE_PLUGIN_ROOT` from its process environment**, which is
  the invariant this bug violated. Extending
  `tasks/lint/skill_permissions.py` to reject env-assignment prefixes as well as
  `bash`/`sh` wrappers is **out of scope** here (see Deliberately unchanged).

  Three implementation constraints, established from the existing guards:

  - **Model it on `tasks/lint/store_duplication.py`** — the closest precedent, a
    `cli/`-scoped regex guard with a named `ALLOWLIST: frozenset[str]` carrying
    per-entry reasons, a pure `violations(root: Path) -> list[str]` (root
    injected, which is what makes it testable), and a thin `@task check` raising
    `Exit(..., code=1)` whose message names the constant and file to edit. Report
    as `path:line:text` per `tasks/lint/call_site_migration.py:26-93`, since the
    reader needs to see *which* variable was found.
  - **Wire it into `cli:check`, not `lint:check`.** `mise run check` depends on
    the seven `<component>:check` roll-ups (`mise.toml:465-467`) and
    **no CI job runs `lint:check` or the bare `check`** —
    `.github/workflows/main.yml:257` runs `mise run cli:check`. So the leaf goes
    in `cli:check.depends` (`mise.toml:409`, alongside
    `lint:store-duplication:check`), and in `lint:check.depends` (`:451`) for
    completeness. Note both existing SKILL.md guards get their actual CI teeth
    from their paired unit test's `violations(REPO_ROOT) == []` assertion under
    `test:unit:tasks`, not from the lint task — so do **both**.
  - **Do not `rglob`.** Reuse `_ignore_spec()` from `tasks/shared/sources.py:40-48`;
    `shell_sources()` is hard-wired to `.sh`. `tests/unit/tasks/test_python_coverage.py:31,68-94`
    already reimplements the prune loop for `.py`, so promote it into a generic
    `sources(root, suffixes, subtree)` rather than writing it a third time. Fail
    closed on an empty match set (`tasks/lint/scripts.py:8-11`). Never write a
    sentinel violation into the live tree — `test:unit:tasks` runs concurrently
    with `cli:check`, so an in-tree sentinel makes the checks flake
    (`tests/unit/tasks/test_python_coverage.py:138-145`).
- **R8** — The bootstrap becomes `--fail-safe`-aware: when `--fail-safe` is
  present in argv, any bootstrap-layer abort exits **0** with empty stdout and a
  single diagnostic line on stderr, so a `!` site degrades to empty injected
  context instead of discarding the whole prompt. The flag is passed through to
  the launcher unchanged. One argv scan plus a conditional in `fail()`, which is
  the single abort path for all **16** gates (14 after R1 deletes the two root
  gates — an earlier draft said 14, counting post-R1).
- **R9** — Terminal invocation gets a fixed-path, upgrade-surviving entry via a
  **two-hop chain**. A new hook, `hooks/shim-refresh.sh`, registered as a fourth
  `SessionStart` entry in `hooks/hooks.json` alongside `vcs-detect.sh`,
  `config-detect.sh` and `migrate-discoverability.sh`, refreshes a symlink at
  `${CLAUDE_PLUGIN_DATA}/bin/accelerator` pointing at the current
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`, and prints the path it refreshed.
  Both inputs are taken from the environment with **no hardcoded home-relative
  paths**, which is what makes the hook testable out-of-band (R10, and the
  `SessionStart` acceptance criteria). When `${CLAUDE_PLUGIN_DATA}` is absent the
  hook is **inert** — it exits 0 having created nothing, rather than falling back
  to an absolute `/bin` path. The inertness guard must therefore test
  `[ -n "${CLAUDE_PLUGIN_DATA:-}" ]` **before** composing any path. The hook
  writes **only** inside `${CLAUDE_PLUGIN_DATA}`; it never writes to
  `~/.local/bin` or any other user-general directory (see Technical Notes for
  why).

  Three mechanics fixed by the codebase pass:

  - **The new `hooks.json` entry must be appended (index 3), never inserted at
    0.** `hooks/test-vcs-detect.sh:615-634` hard-codes `.hooks.SessionStart[0]`
    and asserts it is `vcs-detect.sh`. No `.claude-plugin/plugin.json` edit is
    needed — hooks are discovered by convention; `plugin.json` registers only
    `skills`.
  - **"Prints the path it refreshed" needs a channel decision.** Every stdout
    byte any `SessionStart` hook emits today is JSON — either the
    `{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":…}}`
    envelope (`hooks/vcs-detect.sh:177-181`) or a `systemMessage` sibling. Bare
    plain text on stdout has **no precedent**. The path is diagnostic, not
    context, so the match is `hooks/migrate-discoverability.sh:68-73` —
    plain text on **stderr** with an `[accelerator]` prefix and `exit 0`.
  - **This hook would be the plugin's first consumer of `${CLAUDE_PLUGIN_DATA}`,
    and it reverses a prior decision.**
    `meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md:125` records
    an explicit "**No `${CLAUDE_PLUGIN_DATA}` dependency**". Reversing it is
    fine on the maintainer's terminal-surface decision, but it should be
    acknowledged rather than silently overridden.
- **R9a** — Documentation for the terminal surface lands as a new
  **"Terminal invocation"** section in `docs/internals.md` (which already
  documents plugin-side mechanics such as VCS detection), linked from the
  Documentation list in `README.md`. It must state: the one-line `ln -s` from a
  `PATH` directory the user already owns (typically `~/.local/bin`) to
  `${CLAUDE_PLUGIN_DATA}/bin/accelerator`, run once and never again because its
  target never moves; that `~/.local/bin` is **not** on `PATH` by default on
  macOS; the per-channel shim paths, with a one-line note that a second channel
  can be linked under a different name (e.g. `accelerator-pre`) if the user wants
  both; the mid-session upgrade caveat from Technical Notes; and that the terminal
  surface assumes a cache already populated by a Claude Code session, since a
  first run against an empty cache needs network access to the artifact host and
  carries no `--fail-safe` degradation.
- **R10** — A test covers the `PATH`-symlink invocation shape end to end,
  **through both hops**, hermetically: `<tmp>/userbin/accelerator` →
  `<tmp>/plugin-data/bin/accelerator` → `<fixture-root>/bin/accelerator`
  resolves to the fixture root, exits 0, and produces output reflecting that
  root's `plugin.json` rather than either symlink's parent. No real
  `~/.local/bin` or real `${CLAUDE_PLUGIN_DATA}` path is touched. R9 makes this
  shape the documented default, so it is load-bearing rather than defensive, and
  it is what forces R1's chase to be a loop rather than a single dereference.
- **R11** — **DISCHARGED 2026-07-27.** `${CLAUDE_PLUGIN_ROOT}` still substitutes
  into `allowed-tools` Bash rules on **v2.1.220** (the version the plugin is
  developed against; the floor question is separate and confined to
  `${CLAUDE_PLUGIN_DATA}` in R12). The full basis is recorded in Open Questions:
  the maintainer's direct observation that covered invocations raise no prompt,
  the verified absence of any Bash allow rule under `defaultMode: "default"`
  (which excludes the broad-grant confound this item raised against itself), and
  the both-sides matching argument — a rule containing the variable cannot
  prefix-match an already-expanded absolute path unless Claude Code substituted it
  in the rule too.

  Nothing remains to probe, and no requirement is contingent on the answer. The
  `${CLAUDE_SKILL_DIR}` migration is **out of scope unconditionally** rather than
  out of scope in either branch: the rules stay as they are, and the conditional
  Dependencies edge is discharged. The manual pre-release criterion keeps its
  value as a re-confirmation against the release artifact; a prompt there would
  now signify a Claude Code regression between v2.1.220 and the tested version,
  not undiscovered scope in this item.

  Retained for the record, because it explains why the question was hard: **the
  manual pre-release check could not have answered it early.** Under the current
  plugin all 43 skills abort at their `!` site *before* the model issues any
  Bash-tool call, so "no prompt appeared" would have been fully explained by the
  skill never running. The question was settled instead by observing invocations
  that do reach the matcher, with the broad-grant confound ruled out directly.
- **R12** — Determine whether `${CLAUDE_PLUGIN_DATA}` exists at the plugin's
  declared Claude Code floor (v2.1.144). If it does, record the confirmation. If
  it does not, raise the declared floor as part of this change and note in
  Dependencies which installed versions lose support — which matters because
  those are among the consumers currently hitting this bug. R9's
  inert-when-unset behaviour caps the damage of being wrong either way, so this
  gates the floor declaration, not the design.

  **Correction: there is nowhere in `.claude-plugin/plugin.json` to raise it.**
  That file has no `claudeCodeVersion`, `engines` or `minimumClaudeCodeVersion`
  field; its only `requirements` entry is a free-text Node string. The v2.1.144
  floor is **prose-only**, at `CLAUDE.md:123` and
  `docs/releases-and-compatibility.md:36`. So R12 either updates those two sites
  or introduces the field — a decision this item must make rather than assume.

Everything under `hooks/` **keeps** reading `CLAUDE_PLUGIN_ROOT` — the three
existing `SessionStart` hooks (`vcs-detect.sh`, `config-detect.sh`,
`migrate-discoverability.sh`) and the `PreToolUse` hook (`vcs-guard.sh`). They
are the Claude Code adapter layer, are the one surface Claude Code genuinely
exports to, and already carry self-locating fallbacks. R9's `shim-refresh.sh`
joins them as the fourth `SessionStart` hook, in that same layer, for the same
reason.

## Acceptance Criteria

Every "exits 0" below is paired with an **output** assertion deliberately. The
`config` family carries `--fail-safe` by lint contract and R8 makes a
bootstrap-layer abort exit 0, so exit status alone cannot distinguish a working
CLI from a degraded one. The fail-safe signature is *empty stdout plus one
`accelerator:` diagnostic line on stderr*, so "exit 0 **and** non-empty stdout
**and** no `accelerator:` line on stderr" separates real success from
degradation. Stderr is asserted by that signature rather than by emptiness
because the gated dev-launcher override — which these tests use, see below —
legitimately warns on stderr every invocation (Technical Notes, R8).

**Met 2026-07-28** — the first five and the `--fail-safe` criterion below are
satisfied by `tests/integration/entrypoint/test_accelerator_entrypoint.py`, in
the restated forms recorded under *Deviations from the work item* in the plan:
`display_path` shortens plugin-root paths to a `<plugin>/` token, so a Path
column can never carry an absolute path, and the assertions use a per-root
`templates/adr.md` sentinel or the launcher's own dumped environment instead.
The launcher is supplied by serving the real compiled binary through the stub
release server — the genuine fetch → verify → cache → exec chain, no network and
no dev override, which answers the correction below rather than working around
it.

**Shared preconditions for the bootstrap-level criteria** (the first five, R5 and
R10): the bootstrap is invoked by absolute path from a fixture installation tree
containing `.claude-plugin/plugin.json`, the verify shim, the release public key,
a `templates/` directory and a writable cache dir — the same fixture shape the
existing entrypoint suite builds by copying `bin/accelerator` into it
(`tests/integration/entrypoint/test_accelerator_entrypoint.py:188-222`). No run
performs a network fetch and none of the 16 gates fails for reasons unrelated to
root resolution.

**Correction — the launcher cannot be supplied via the gated dev override.** An
earlier draft mandated "the **locally built** binary supplied via the gated dev
override". That is unworkable: the override's third gate requires the named
binary's canonical parent to sit inside `${CLAUDE_PLUGIN_ROOT}/cli/target/`
(`bin/accelerator:119-129`), which is only true when the plugin root *is* the
repo root — contradicting the fixture-tree precondition — and creating the
marker at the repo root is forbidden by the assertion at
`test_accelerator_entrypoint.py:823-829`. The existing suite instead writes a
**stub launcher** into `<fixture root>/cli/target/debug/accelerator`
(`_local_launcher`, `:499-507`) and creates the marker inside the fixture; that
is the shape to use where the bootstrap itself is under test. Where only the
*launcher's* behaviour matters, address it directly via `ACCELERATOR_BIN` (the
convention R4 now follows). Either way the build-order dependency in Dependencies
still holds for the suites that need a real compiled launcher.

- [x] Given both `CLAUDE_PLUGIN_ROOT` and `ACCELERATOR_PLUGIN_ROOT` are unset,
      when `bin/accelerator config template adr` is run by absolute path
      (deliberately **without** `--fail-safe`, and deliberately a *template*
      command — measured to be the only root-sensitive `config` family, see
      Technical Notes), then it exits 0 **and** stdout contains the fenced
      contents of the fixture's `templates/adr.md` **and** stderr carries no
      `accelerator:` diagnostic line. This is falsifiable in both directions: it
      fails today at the bootstrap gate, and with R1 but without R2's export it
      fails again as a fail-closed `Template 'adr' not found` at exit 1.
- [x] Given both variables are unset, when
      `bin/accelerator config instructions commit --fail-safe` is run by absolute
      path — the exact command from the reported failure — then it exits 0 **and**
      stderr carries no `accelerator:` diagnostic line. **Empty stdout is correct
      here and must not be asserted against**: `config instructions` renders a
      *user* per-skill override and the plugin ships none for `commit` (measured:
      0 bytes at exit 0, identically with and without a root). An earlier draft
      required "a known substring of the shipped commit instructions", which does
      not exist and made the criterion unsatisfiable. This criterion proves the
      bootstrap no longer aborts; the template criterion above is what proves the
      root reached the launcher.
- [x] Given both variables are unset, when `bin/accelerator config templates
      list` is run, then the output contains a row for `adr` whose Source column
      is `plugin default` and whose Path lies under the resolved installation root
      — **not** an empty table at exit 0.
- [x] Given `bin/accelerator` is invoked through a two-hop symlink chain
      (`<tmp>/userbin/accelerator` → `<tmp>/plugin-data/bin/accelerator` →
      `<fixture-root>/bin/accelerator`, no real `PATH` or `${CLAUDE_PLUGIN_DATA}`
      directory touched), when `config templates list` is run through it, then
      every plugin-default row's Path lies under `<fixture-root>/templates/` —
      i.e. the chain resolved to the fixture root, not to either symlink's parent.
- [x] Given a stale or wrong `CLAUDE_PLUGIN_ROOT` **or** `ACCELERATOR_PLUGIN_ROOT`
      is present in the environment — individually and together — when
      `bin/accelerator config templates list` is run, then the plugin-default rows
      still resolve under the real installation root and no row resolves under the
      injected path. (Consistent with R4, which no longer relies on the bootstrap
      honouring an ambient root at all.)
- [x] Given the R4 seam is re-pointed at `ACCELERATOR_BIN="/nonexistent/accelerator"`,
      when the four assertions in `test-work-item-scripts.sh` run, then they still
      exercise the hardcoded-fallback path in
      `work-item-template-field-hints.sh` — demonstrated by their failing if that
      `hardcoded_fallback` branch is temporarily removed. Without this the four
      assertions pass vacuously, since the shipping template carries the same
      values they expect.
- [x] Given a bootstrap-layer failure (e.g. a missing verify shim — the small
      pre-verified binary the bootstrap uses to check the launcher's signature),
      when `--fail-safe` is in argv, then the bootstrap exits 0 with empty stdout
      and one diagnostic line on stderr; without the flag it exits non-zero.
**Met 2026-07-29** — the hook criteria below are satisfied by
`tests/integration/hooks/test_launcher_link_refresh.py` (22 cases). Two naming
deviations from this list, both settled during implementation: the hook ships as
`hooks/launcher-link-refresh.sh`, not `shim-refresh.sh`; and "prints that
refreshed path" is narrowed to the **re-point** case — a first refresh is silent
on both channels, since the resolved Open Question above establishes that a
routine `SessionStart` stderr line reaches nobody, so only a *change* of target
is worth naming.

- [x] Given `hooks/shim-refresh.sh` is run with `CLAUDE_PLUGIN_DATA=<tmp>/data`
      and `CLAUDE_PLUGIN_ROOT=<tmp>/v1`, then `<tmp>/data/bin/accelerator` is a
      symlink to `<tmp>/v1/bin/accelerator` and the hook prints that refreshed
      path; when re-run with `CLAUDE_PLUGIN_ROOT=<tmp>/v2` it re-points there,
      and a pre-existing `<tmp>/userbin/accelerator → <tmp>/data/bin/accelerator`
      link — whose own target did not move — still executes.
      (`test_a_first_refresh_creates_the_link_and_says_nothing` and
      `test_a_second_root_repoints_the_link_and_names_both`, the latter executing
      the user's own hop after the re-point.)
- [x] Given `hooks/shim-refresh.sh` is run with `CLAUDE_PLUGIN_DATA` unset, then
      it exits 0 and creates nothing (in particular no `/bin` entry).
      (`test_an_unset_plugin_data_is_inert`, plus
      `test_a_relative_plugin_data_is_inert` for a value that would compose a
      cwd-relative path.)
- [x] Given `hooks/shim-refresh.sh` alone is run with `HOME` and
      `CLAUDE_PLUGIN_DATA` pointed into a temp tree, when that tree is snapshotted
      before and after and diffed, then the only paths created or modified lie
      inside `${CLAUDE_PLUGIN_DATA}`, and `<HOME>/.local/bin` does not exist.
      (Scoped to this hook alone — the other `SessionStart` hooks legitimately
      write config and migration state elsewhere.)
      (`test_nothing_outside_plugin_data_is_created_or_modified`.)
- [x] No tracked source file under `cli/` contains the string `CLAUDE_`, as
      enforced by the R7 lint task (honouring `.gitignore`, so `cli/target/` and
      `cli/visualiser/frontend/node_modules/` are excluded), and that guard has a
      negative test proving it fails when a reference is reintroduced.
      **Met 2026-07-29**: `mise run lint:claude-coupling:check` (28 packages,
      clean) with `tests/unit/tasks/test_claude_coupling.py` carrying the negative
      cases — a `var_os` read, a comment mention, any `CLAUDE_`-prefixed name, and
      reintroduction into both the bootstrap and an out-of-tree writer.
- [ ] `mise.local.toml` is absent from the repo, and `grep -rl
      'CLAUDE_PLUGIN_ROOT'` over the working tree (honouring `.gitignore`)
      returns **only**: `hooks/**`, `scripts/interactive-harness.sh`,
      `scripts/test-design.sh`, `skills/config/migrate/**`, migrations
      `0001`–`0007`, `tests/unit/tasks/test_call_site_migration.py`,
      `tests/unit/tasks/test_skill_permissions.py`,
      `tasks/lint/skill_permissions.py`, `.shellcheckrc`, `CLAUDE.md`, and
      `skills/**/SKILL.md` — the last because Claude Code substitutes
      `${CLAUDE_PLUGIN_ROOT}` textually into skill content and `allowed-tools`
      rules (410 occurrences across 48 files per Blast radius), which R11 keeps in
      place. "Reads" here means process-environment access, not a substituted
      `${…}` token in markdown or frontmatter.

      An earlier draft of this list **could not have passed**: it omitted
      `tests/unit/tasks/test_skill_permissions.py` (9 token fixtures),
      `tasks/lint/skill_permissions.py:43,55` (`_PLUGIN_PREFIX` and the
      `_BARE_LAUNCHER` probe — the matcher *model*, which is why R7 is a separate
      guard), `.shellcheckrc:51` and `CLAUDE.md:65` (comment and prose); and it
      listed `agents/**`, which has **zero** occurrences.
- [x] The renamed diagnostics name `ACCELERATOR_PLUGIN_ROOT`, asserted by the
      existing message-text tests at `launcher/tests/version.rs:179` and
      `cache_root.rs:132,134`. Both were **tightened further** on 2026-07-29: the
      config layer's new `PluginRootUnavailable` also names the variable, so a
      variable-name-only assertion no longer distinguishes the cache-root step
      from it. Each now additionally matches `no ACCELERATOR_CACHE_DIR override
      was given`, a clause only `CacheRootUnavailable` emits.
- [x] `hooks/hooks.json` registers `shim-refresh.sh` as a `SessionStart` entry
      (without it the hook never fires and R9's upgrade-survival property is
      absent, while every other hook criterion — which invokes the script
      directly — would still pass). **Met 2026-07-29**, as
      `launcher-link-refresh.sh` at index 3 — appended, so
      `test-vcs-detect.sh`'s hard-coded `SessionStart[0]` assertion still holds —
      and asserted by `test_the_hook_is_a_session_start_group` and
      `test_the_group_holds_exactly_one_command_hook`.
- [x] `docs/internals.md` contains a "Terminal invocation" section stating the
      one-line `ln -s` recipe, the macOS `~/.local/bin`-not-on-`PATH` caveat, the
      per-channel shim paths with the second-channel note, and the mid-session
      upgrade caveat; and the Documentation list in `README.md` links to it.
      **Met 2026-07-29** as "Terminal Invocation" (`docs/internals.md:88`),
      linked from `README.md:58`.
- [x] R11's result is recorded in this item's Open Questions, naming the Claude
      Code version tested (v2.1.220) and its basis. **Met 2026-07-27.**
- [x] R12's floor determination is recorded, and either the declared floor is
      unchanged with `${CLAUDE_PLUGIN_DATA}` confirmed present at it, or the floor
      is raised. The floor lives in **prose only** — `CLAUDE.md:123` and
      `docs/releases-and-compatibility.md:36` — since
      `.claude-plugin/plugin.json` has no version-floor field and the planning
      decision was not to introduce one, so those two sites are the target rather
      than `plugin.json`. **Met 2026-07-28**: the variable landed in v2.1.78, so
      the floor is unchanged and both prose sites are untouched.
- [x] **Automated** — every `bin/accelerator config *` command extracted from a
      `!` block in any `skills/**/SKILL.md` exits 0 **and** emits no
      `accelerator:` diagnostic line on stderr, with a per-family stdout
      assertion (below). The harness performs Claude Code's own textual
      substitution — rewriting the extracted
      `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` prefix to the resolved installation
      root — while running with both variables removed from the environment via
      `env -u`, which is precisely the production shape (correct path, empty
      environment). Extraction reuses `preprocessor_commands()` and
      `is_plugin_invocation()` from `tasks/lint/skill_permissions.py:105-112`.

      Measured shape of the corpus (**204** commands, ~125 distinct), which
      simplifies one half of this criterion and hardens the other:

      - **No command carries skill-time argument interpolation.** All 204 are
        static literals; the only `$` is the `${CLAUDE_PLUGIN_ROOT}` prefix. The
        "supplied fixture arguments or skipped with a logged reason" branch
        describes an empty set and can be dropped.
      - **~86 of the 204 (42%) legitimately emit empty stdout**, not the handful
        implied: `config instructions <skill>` (43) and `config context --skill
        <skill>` (43) render *user* per-skill overrides and are empty unless the
        project configures one. Worse, **which** ones are empty depends on the
        project's own `.accelerator/config.md`, so running against this repo makes
        the exception list drift with unrelated config changes. The harness must
        therefore run against a **fixture project** with a known configured set,
        and assert per family: non-empty for
        `agents`/`paths`/`path`/`template`/`work`/`review`, and exact-match
        against the fixture's configured set for `instructions`/`context`.
      - All 204 already carry `--fail-safe` (enforced by
        `skill_permissions.py` point 3). Note the 20 `config template <name>`
        sites are **fail-closed regardless** of that flag (R4), so they are the
        subset that stays loud rather than degrading.

      **Met 2026-07-29** by `tests/integration/skill-invocation/` (128 cases: 122
      distinct commands plus 6 corpus invariants), run as
      `mise run test:integration:skill-invocation`. Three counts above are off, as
      measured during implementation: **13** distinct `template <name>` commands
      (19 occurrences), not 20; there is **no** `templates list` command in the
      corpus at all; and a `config agent <name>` family is missing from the list.
      The totals — 204 commands, 122 distinct, 45 files, 42 injection skills each
      way — are exact. Better than predicted: **every** one of the 122 renders
      non-empty stdout against the fixture, so no family needs an empty-tolerant
      assertion.
- [ ] **Manual, pre-release** — on a clean install of the release artifact, with
      no `mise.local.toml` and neither variable exported into the shell, in
      permission mode `default` with no broad Bash allow rules: invoke
      `config/paths`, `integrations/linear/search-linear-issues`,
      `planning/create-plan` and `vcs/commit`, and confirm each loads **and**
      raises no permission prompt. Only skill load and prompt absence are under
      test — no Linear API call is required, so the validation environment needs
      no tracker credentials. The prompt half cannot be automated — no harness
      observes Claude Code's permission matcher. Record the outcome in a
      validation note under `meta/validations/` and reference it from this item.
      If a prompt appears, the `${CLAUDE_SKILL_DIR}` migration item is raised and
      the **prerelease** is gated on it (see Dependencies — this item still closes
      on its own criteria).
- [x] `mise run` (bare default task) exits 0 end-to-end. **Met 2026-07-29** with
      every phase landed.
- [x] The regression test from R5 fails against the current `bin/accelerator` and
      passes after the fix. **Met 2026-07-28**: confirmed at the intermediate
      commit with the bootstrap change stashed — 18 of the new entrypoint cases
      red, every one with `accelerator: CLAUDE_PLUGIN_ROOT is not set`. The three
      that pass pre-change are the ones predicted: the symlink-cycle case (which
      characterises the kernel's `ELOOP`) and the two scan-window cases (a gate
      fires either way).

## Open Questions

- **Does `${CLAUDE_PLUGIN_ROOT}` still substitute in `allowed-tools` Bash rules
  on v2.1.220?** **RESOLVED 2026-07-27 — yes, it still substitutes.** No
  `${CLAUDE_SKILL_DIR}` migration is required in any branch; the conditional
  Dependencies edge is discharged, and R11 needs no probe.

  Basis, in the order it removes doubt:

  - **Maintainer's direct observation** (v2.1.220): invocations covered by
    `${CLAUDE_PLUGIN_ROOT}`-prefixed `allowed-tools` rules do not raise a
    permission prompt.
  - **The stated confound is excluded.** This item's own caveat — that an absent
    prompt proves nothing in a session granting Bash broadly — does not apply.
    There is no project `.claude/settings.json` or `.claude/settings.local.json`,
    and `~/.claude/settings.json` carries `defaultMode: "default"` with **zero**
    Bash allow rules, so no ambient grant can explain the absence.
  - **The mechanism corroborates it.** A rule such as
    `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)` can only prefix-match
    a command already expanded to an absolute path if Claude Code substituted the
    variable on *both* sides. Substitution in content but not in the rule would
    leave the rule matching a literal `${CLAUDE_PLUGIN_ROOT}` prefix, and every
    covered call would prompt.

  The `mise.local.toml` contamination concern recorded here previously does not
  bear on this question: that file affects whether the *bootstrap* finds a root,
  not whether the *permission matcher* substitutes — the two mechanisms are
  independent. It is also, as established during planning, absent from `main`,
  `origin/main` and `@-`; it has never been pushed, so it masks nothing beyond the
  maintainer's local runs. **R6 is therefore resequenced last, not first**: the
  file is what keeps the installed unfixed plugin usable while this work proceeds,
  so deleting it early would disable the skills needed to carry it out.

  The manual pre-release criterion keeps its value as a re-confirmation against
  the release artifact, but is no longer a discovery gate.
- **What minimum Claude Code version provides `${CLAUDE_PLUGIN_DATA}`, and in
  which contexts is it available?** **RESOLVED 2026-07-28 — v2.1.78, comfortably
  below the declared v2.1.144 floor. The floor does not move.**

  Determined from the Claude Code changelog and the plugins reference, read on
  v2.1.220:

  - **First version: v2.1.78.** The changelog entry under that heading reads
    "Added `${CLAUDE_PLUGIN_DATA}` variable for plugin persistent state that
    survives plugin updates; `/plugin uninstall` prompts before deleting it".
    That is 66 patch releases below the declared floor, so R12 resolves as
    "unchanged floor, variable confirmed present at it" and neither
    `CLAUDE.md:123` nor `docs/releases-and-compatibility.md:36` is touched.
    `meta/decisions/ADR-0051-skills-as-the-product.md:116-118` likewise stands.
  - **Contexts.** The plugins reference states that `${CLAUDE_PLUGIN_ROOT}`,
    `${CLAUDE_PLUGIN_DATA}` and `${CLAUDE_PROJECT_DIR}` "are exported as
    environment variables to hook processes and to MCP and LSP server
    subprocesses", and separately substitute inline in skill and agent content,
    hook and monitor commands, and named MCP/LSP fields. So R9's hook receives
    it as a real environment variable, a `!` site would see only a textual
    substitution, and a plain user terminal has neither — which is why R9a's
    documentation uses the literal per-channel path rather than the token.
  - **Not version-scoped.** It "resolves to `~/.claude/plugins/data/{id}/`, where
    `{id}` is the plugin identifier with characters outside `a-z`, `A-Z`, `0-9`,
    `_`, and `-` replaced by `-`", is "created on first reference", and
    "outlives any single plugin version". `${CLAUDE_PLUGIN_ROOT}`, by contrast,
    "changes when the plugin updates". That asymmetry is exactly what makes the
    two-hop chain work and the user's `ln -s` a one-time action.

  The inert-when-unset guard in R9 stays regardless: it costs nothing and covers
  a Claude Code that stops exporting the variable to hooks, which the floor
  cannot.
- **Where does a `SessionStart` hook's output actually go?** **RESOLVED
  2026-07-28 from the Claude Code hooks reference**, fixing R9's channel choice:

  - **`systemMessage` is a universal top-level JSON output field**, documented
    for every event as "Warning message shown to the user" — not a
    `SessionStart`-specific field and not nested inside `hookSpecificOutput`. So
    `hooks/vcs-detect.sh:14`'s top-level `{"systemMessage": …}` is correct usage,
    and it is the documented way to put a line in front of the user.
  - **For `SessionStart`, plain stdout becomes Claude's context**, not user
    output: `UserPromptSubmit`, `UserPromptExpansion` and `SessionStart` are the
    three events where "stdout is added as context that Claude can see and act
    on". Bare stdout is therefore the one channel a diagnostic must avoid.
  - **stderr at exit 0 has no documented destination.** Exit 2 feeds stderr back
    (rendering as a `<hook name> hook error` notice for the non-blockable events,
    `SessionStart` among them) and other non-zero codes show its first line;
    exit 0 assigns stderr nothing. `bin/accelerator:114-116`'s "invisible at
    SessionStart" is the safe reading, and it is what R8's durable record exists
    for.

  Consequence for R9: routine and transient conditions go to stderr, accepting
  they may be unseen; the persistent states the hook cannot fix go to a
  top-level `systemMessage`, accumulated so at most one JSON object reaches
  stdout.

  Consequence beyond this item: `hooks/migrate-discoverability.sh:66-72`'s
  stderr advisory at exit 0 reaches nobody. Pre-existing and out of scope here —
  raised as **0183**.

**Resolved during review** — a user tracking both the prerelease and stable
marketplaces gets **two** fixed-path shims
(`~/.claude/plugins/data/accelerator-atomic-innovation-prerelease/bin/accelerator`
and `…/accelerator-atomic-innovation/bin/accelerator`). The two-hop design mostly
dissolves this: the user links whichever channel they want, explicitly rather
than racing. Decision: R9a documents the single `ln -s` for the channel the user
tracks, plus a one-line note that a second channel can be linked under a
different name (e.g. `accelerator-pre`). No mechanism, no per-channel naming
policy.

## Dependencies

- Blocked by: none. R11 is **discharged** (positive, 2026-07-27), so no successor
  item arises from it and no `${CLAUDE_SKILL_DIR}` migration is required. R12's
  floor determination remains, sequenced first but part of this item. **This item
  closes on its own criteria.** (An earlier draft made closure conditional on a
  successor shipping, which contradicted "Blocked by: none" and would have left
  the item open behind nominally downstream work.)
- **External — Claude Code (verified on v2.1.220).** Four behaviours this item
  relies on, none under our control:
  - `${CLAUDE_PLUGIN_ROOT}` substitutes into `allowed-tools` Bash rules —
    **confirmed 2026-07-27** (R11, Open Questions). Still an external dependency
    rather than a guarantee: no guard in this repo can detect it regressing, since
    `tasks/lint/skill_permissions.py` models the matcher rather than exercising it.
  - `${CLAUDE_PLUGIN_ROOT}` substitutes into skill content (the mechanism the 43
    `!` sites depend on).
  - `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}` are exported as real
    environment variables to hook processes — R9's hook depends on both.
  - Hook commands keep the *previous* version's `${CLAUDE_PLUGIN_ROOT}` until
    `/reload-plugins`, which is the mid-session upgrade caveat R9 accepts.
  The `${CLAUDE_PLUGIN_DATA}` version floor is an open question above.
- **Distribution — lockstep with the launcher artifact.** The bootstrap ships
  inside the plugin; the launcher and visualiser server are fetched, verified and
  cached keyed on the version in `.claude-plugin/plugin.json`. After R1–R3 the
  bootstrap exports only `ACCELERATOR_PLUGIN_ROOT`, so a pre-rename launcher
  would find no root and silently drop the plugin-default template tier (empty
  table, exit 0). The renamed launcher and server binaries must therefore ship in
  the **same version** as the renamed bootstrap; the version-keyed cache is what
  guarantees no pre-rename launcher stays reachable, so no extra cache
  invalidation is required — but a release that bumps only one side reproduces
  precisely the silent-wrong-answer mode this item exists to remove.
- **Build order — already solved; use the existing edge.** The automated
  `!`-extraction suite needs a compiled launcher before it runs. That edge exists:
  `build:cli:dev` (`mise.toml:107-109` → `tasks/build.py:248-260`) is already the
  declared prerequisite of **six** integration tasks, including
  `test:integration:work` (`mise.toml:220`) — so R4's re-pointed seam needs
  nothing new, contrary to an earlier draft. Two mechanics matter:

  - **mise resolves `depends` as a DAG and runs independent nodes in parallel.**
    There is no `wait_for` and no `depends_post` anywhere in `mise.toml`, and
    array position conveys no ordering. Adding `build:cli:dev` to
    `default.depends` would merely *race* the suites. The edge must go on the
    specific leaf task's own `depends` — which is what
    `tasks/build.py:252-255` and `tasks/test/helpers.py:27-30` both say in prose.
  - **CI needs no edit** for anything under `test-unit` or `test-integration`:
    both already carry the `RUSTUP_HOME` routing step and `workspaces: cli` cargo
    caching (`.github/workflows/main.yml:70-71,81-88`). CI would need two new
    steps only if `check-scripts` (`:147-163`) or `check-build-system`
    (`:165-181`) started requiring a cargo build — neither has rustup routing or a
    cargo cache today, and per `:244-247` each cargo job needs its own
    `cache_key_prefix`.

  Gap worth closing while here: **no test asserts that any `test:*` task depends
  on `build:cli:dev`** (`tests/unit/tasks/test_mise.py:17-35` covers only
  `_CHECK_GATES`), so a new task can silently ship without the prerequisite.
- Blocks: the next prerelease. The plugin is substantially unusable as shipped in
  1.24.0-pre.16 for any consumer who has not manually exported the variable. The
  closure sequence is: merge → build a signed candidate artifact → clean-install
  it → run the manual pre-release check → publish. That depends on the release
  pipeline being able to produce a verifiable candidate ahead of publication; if it
  cannot, the manual criterion is unperformable as written and needs restating
  against a locally staged artifact.
- ~~**Conditionally blocks**: an `allowed-tools` rule migration~~ —
  **discharged 2026-07-27.** R11 came back positive, so the migration to
  `${CLAUDE_SKILL_DIR}`-relative paths across the 47 SKILL.md files, and the
  accompanying `_PLUGIN_PREFIX` update in `tasks/lint/skill_permissions.py`, are
  not required. No successor item arises from this edge, and the prerelease is not
  gated on one.
- **Terminal surface, first run.** R9/R9a make terminal invocation a supported
  entry point, and unlike a `!` site it carries no `--fail-safe`. A first terminal
  run against an unpopulated cache needs the artifact host, a downloader binary
  and the release public key — so it can fail on a clean install, offline, or
  during a host outage. R9a states that the terminal surface assumes a cache
  already populated by a Claude Code session; anything stronger is a separate
  concern.
- **Consumer floor.** If R12 finds `${CLAUDE_PLUGIN_DATA}` absent at v2.1.144 and
  the floor is raised, this release also drops support for installed Claude Code
  versions between the old and new floor — among them consumers currently hitting
  this bug. That trade-off is the reason R12 is a determination rather than an
  assumption.
- **If split**: two cut lines are sanctioned, and the assignment is exhaustive so
  neither can silently drop a requirement.
  - *Urgency cut* — R1, R2, R4, R5, R6, R8 ship together. **R4 is not separable
    from R3** (corrected: an earlier draft said R1). R1 alone leaves R4's four
    assertions intact — under `mise run` the seam bypasses the bootstrap via
    `ACCELERATOR_BIN`, and a rootless launcher exits 1 rather than 0 for
    `config template work-item`. Vacuity arrives when R3 renames the launcher's
    reader alongside `tasks/test/helpers.py`'s writer. R5 remains inseparable from
    R2, being its regression guard. **R3 cannot be deferred wholesale either** —
    R2 exports `ACCELERATOR_PLUGIN_ROOT`, and it is R3 that teaches the launcher
    and server to read it, so an R1+R2-only release would export a name no
    shipped binary reads and reintroduce the empty-table mode. Either include the
    launcher/server readers in the cut, or have the bootstrap transitionally
    export **both** names until R3 lands. Only the wider purge (out-of-tree
    writers, R7's guard) is genuinely deferrable — and if R3 is deferred, R4 may
    be deferred with it.
  - *Scope cut* — R9, R9a, R10 move to a follow-up. R12 stays with whichever half
    ships first, being a determination rather than a change; R11 is already
    discharged and carries no work either way.
  Either split creates a follow-up work item carrying the deferred requirements,
  so the deferred half stays tracked rather than living only in Drafting Notes.
- Related: 0164 (introduced the bootstrap), 0167 (config-command migration, which
  routed the skills through it), 0136 (the parent Rust CLI migration epic).

## Assumptions

- **`${CLAUDE_PLUGIN_ROOT}` substitutes into `allowed-tools` Bash rules.** The
  documentation on v2.1.220 no longer confirms this. The skills reference states
  only that *"Claude Code substitutes `${CLAUDE_SKILL_DIR}` and
  `${CLAUDE_PROJECT_DIR}` in two places: the skill's markdown content, and Bash
  rules in the `allowed-tools` frontmatter"* — `${CLAUDE_PLUGIN_ROOT}` is absent.
  The plugins reference covers it only obliquely, as substituting in *"skill and
  agent content — anywhere the placeholder appears"*, leaving open whether
  frontmatter counts as content. The two pages are ambiguous rather than
  contradictory. Empirically it held in the 2026-06-10 root cause analysis (RCA)
  on an earlier version,
  and the reported failure here is consistent with it still holding (the plain
  absolute-path call ran and produced the error; only the env-prefixed retry
  prompted). It **cannot** be confirmed from a session that grants Bash broadly:
  an absent prompt then proves nothing, which is why the manual check specifies
  permission mode `default` and no broad allow rules.
- `${CLAUDE_PLUGIN_ROOT}` substitutes into skill content, and exports as a real
  environment variable only to hooks and MCP/LSP subprocesses. Verified against
  v2.1.220 docs and by direct probe.
- `${CLAUDE_PLUGIN_DATA}` is exported to hook processes, resolves to
  `~/.claude/plugins/data/{sanitised-plugin-id}/`, is created on first reference,
  and survives plugin updates — all documented on v2.1.220. R9 depends on all
  four. **Not** assumed: that it exists at the plugin's declared v2.1.144 floor —
  that is an open question, and R9's inert-when-unset behaviour is the guard
  against being wrong.
- The installation is always the Claude Code plugin cache, so `plugin.json`,
  `templates/`, `keys/` and a writable `<root>/bin` are always present.
- The bash 3.2 floor still applies (macOS): no `readlink -f`, no associative
  arrays.

## Technical Notes

### Why the export is mandatory (R2)

With no root, the launcher's `config` path does **not** error — `main.rs:175`
returns `None` and `cli/config-adapters/src/store.rs:343-352` silently skips the
plugin-default template tier. Measured against the cached launcher directly:

```
$ env -u CLAUDE_PLUGIN_ROOT <launcher> config templates list
| Template | Source | Path |
|----------|--------|------|
(empty, exit 0)
```

A fix that derived the root only for the bootstrap's own use would convert a loud
failure into a silent wrong answer.

**Scope correction — only the template family is root-sensitive.** Measured
against a freshly built launcher, each command run twice (`env -u` vs
`CLAUDE_PLUGIN_ROOT=<repo>`):

| Command (`--fail-safe`)         | Rootless                                    | Rooted        |
|---------------------------------|---------------------------------------------|---------------|
| `config agents`                 | rc 0, 739 B                                 | rc 0, 739 B   |
| `config paths`                  | rc 0, 473 B                                 | rc 0, 473 B   |
| `config path work`              | rc 0, 9 B                                   | rc 0, 9 B     |
| `config instructions commit`    | rc 0, **0 B**                               | rc 0, **0 B** |
| `config context --skill commit` | rc 0, **0 B**                               | rc 0, **0 B** |
| `config work integration`       | rc 0, 6 B                                   | rc 0, 6 B     |
| `config review plan`            | rc 0, 1320 B                                | rc 0, 1320 B  |
| **`config template adr`**       | **rc 1**, `Template 'adr' not found`        | rc 0, 1672 B  |
| `config templates list`         | rc 0, header-only empty table               | rc 0, 10+ rows |

Everything except the template family reads project config, not
`<root>/templates/`, and is byte-identical either way. So the silent-degradation
hazard is confined to `templates list`, and `config template <name>` is
**fail-closed even under `--fail-safe`** (`resolve_template` →
`Failure::Refusal`, `cli/launcher/src/config_command/inbound/cli.rs:238-247`;
`finish` degrades only `Failure::Read`, `:452-470`). Three consequences: R2's
justification narrows to `templates list` (still sufficient and still
mandatory); the mixed failure mode means a non-exporting fix would leave the 20
`!` sites calling `config template <name>` failing loudly rather than silently;
and the `accelerator:`-on-stderr discriminator detects only *bootstrap*-layer
aborts, since the launcher's own diagnostics carry no such prefix (that string is
`fail()`'s `printf 'accelerator: %s\n'`).

One measurement hazard: `cli/target/debug/accelerator` in a working tree can be
badly stale (found at v1.24.0-pre.13, built 2026-07-13, predating the `config`
builtin — it routed `config` through the external resolver and so reported
`CacheRootUnavailable`, which looks exactly like a root bug). Rebuild before
probing.

Propagation needs no new plumbing: `UnixExec::exec`
(`cli/launcher/src/launch/outbound/exec.rs:16-17`) is
`Command::new(program).args(args).exec()` with no `.env()` calls, so the child
inherits the environment wholesale. One `export` in the bootstrap reaches the
launcher and, through `exec`, the visualiser server. The launcher cannot
self-locate reliably — under `ACCELERATOR_CACHE_DIR` it is cached outside the
root — so the exported variable, not `current_exe()`, is the correct channel.

### Symlink-aware self-location (R1)

The repo-wide idiom `cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P` is
**insufficient here**. `pwd -P` resolves symlinks among the directory components
but not the final symlink to the script file itself, so an `accelerator`
symlinked onto `PATH` derives the symlink's parent. Measured:

```
# via its real path
BASH_SOURCE : …/accelerator/1.24.0-pre.16/bin/probe.sh
naive root  : …/accelerator/1.24.0-pre.16          (plugin.json: FOUND)

# via a PATH symlink
BASH_SOURCE : <scratch>/userbin/accelerator
naive root  : <scratch>                            (plugin.json: MISSING)
chased root : …/accelerator/1.24.0-pre.16          (plugin.json: FOUND)
```

Use a `while [ -L "$src" ]` chase honouring relative link targets, then `cd -P`.
The sourced libraries need no change — they are never symlinked onto `PATH`.
R9 makes the symlink the *documented* invocation shape, so this chase is
load-bearing, not defensive — and because R9's chain is **two hops deep**, the
loop form is required: a single dereference would land on the
`${CLAUDE_PLUGIN_DATA}` shim and derive that directory as the root.

**Do not design this from scratch — it has already been written and reviewed in
this repo.** There is no symlink chase in the tracked tree today (every tracked
`-L` use is a one-shot rejection; the only `readlink` uses are the GNU-only
best-effort `readlink -f … || true`, a documented BSD no-op). But the
specification survives at
`meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md:620-655`, the
as-shipped implementation at
`workspaces/visualisation-system/skills/visualisation/visualise/cli/accelerator-visualiser:17-32`
(a jj workspace checkout — reference only), and — most valuably — its review
already settled four things this change would otherwise re-litigate
(`meta/reviews/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding-review-1.md`):

- `:392` — `while [ -L`, `readlink` **without** `-f`, `case "$target" in /*)` to
  split absolute from relative targets.
- `:738` — why a `readlink -f` capability probe and a `perl` fallback were both
  rejected: pure bash plus `pwd -P` is identical on BSD and GNU.
- `:757` — **cycle detection is mandatory**, or `ln -sf a b; ln -sf b a` hangs
  the bootstrap.
- `:856` — 40 is Linux's `SYMLOOP_MAX` and Darwin's is 32: pick the lower bound
  or state the over-approximation.
- `:835` — **a cycle test cannot go through direct exec.** `execve()` returns
  `ELOOP` before bash starts, so the test must invoke `bash "$CYCLE_A"` for the
  in-script counter to fire. R10's suite needs this.

### Precedence and directionality

Self-location wins unconditionally; no environment variable overrides it in the
bootstrap. An externally-owned variable can be stale — the plugins reference
notes that after a mid-session plugin update, hooks keep the *previous* version's
path until `/reload-plugins`, and that directory survives ~2 weeks. A
new-version bootstrap honouring it would read the old `plugin.json`, seek the
shim in the old tree, and exec the **old launcher**. `mise.local.toml`'s pinned
value is the same hazard.

`ACCELERATOR_PLUGIN_ROOT` is exempt from that reasoning only because of strict
directionality, which R1 and R2 make explicit:

- the **bootstrap writes it and never reads it** — one writer per invocation, so
  it cannot be stale and cannot be redirected by an ambient value;
- the **Rust layers read it and never write it** — which is also what makes it
  usable as the test-injection seam, since every test that injects a synthetic
  root targets the launcher or server directly, not the bootstrap. Where a suite
  currently reaches the launcher *through* the bootstrap — the R4 seam in
  `skills/work/scripts/test-work-item-scripts.sh` is the one such case found —
  R4 routes it past the bootstrap via `ACCELERATOR_BIN` rather than weakening
  R1. That is the general rule for any future injection site: inject into the
  reader, address the reader.

Without that split the two roles would conflict: a bootstrap that read the
variable to support injection would reintroduce exactly the override hazard R1
removes.

### `--fail-safe` in the bootstrap (R8)

`fail()` (`bin/accelerator:20-23`) is the single abort path — there is no other
`exit 1` in the file — for all **16** gates: root unset (`:25`), root not a
directory (`:27`), unsupported architecture (`:35`), unsupported OS (`:40`),
missing `plugin.json` (`:45`), unreadable version (`:49`), no downloader (`:67`),
missing verify shim (`:72`) — the small pre-verified binary that checks the
launcher's minisign signature, introduced by 0164 — missing public key (`:74`),
unusable cache dir (`:107`), refused dev launcher (`:134`), unhashable shim
(`:163`), un-stageable shim (`:168`), un-chmod-able shim (`:170`), lock timeout
(`:208`), and fetch-and-verify failure (`:254`). R1 deletes the first two,
leaving 14 — which is where the earlier count of 14 came from.

So R8 is one argv scan setting a flag plus a conditional in `fail()`: exit 0,
print nothing on stdout, keep the diagnostic on stderr (invisible at `!` sites per
the comment at `:116`, useful in a terminal). Because `fail()` resolves its
variables at call time, the scan may sit anywhere in `:19-24` — after
`set -uo pipefail` (`:18`) and before the first gate — and consuming the flag
inside `fail()` covers **all** gates with no per-site edits. **argv is entirely
untouched before `exec`**: `"$@"` appears only at `:142` and `:262`, with no
`shift`, no `$#` test and no `case "$1"`, so nothing must run first. Under
`set -u` on the bash 3.2 floor the safe iteration form is
`for arg in ${1+"$@"}`. The existing local-build override already follows the
stderr pattern, warning to stderr and recording durably because stderr is
invisible at `SessionStart`.

### Fixed-path shim for terminal invocation (R9)

`${CLAUDE_PLUGIN_DATA}` resolves to `~/.claude/plugins/data/{id}/` where `{id}`
is the plugin identifier with non-`[A-Za-z0-9_-]` characters replaced by `-`.
Confirmed on this machine: `accelerator-atomic-innovation-prerelease`,
`accelerator-atomic-innovation` and `accelerator-inline` all already exist. The
hook creates `${CLAUDE_PLUGIN_DATA}/bin/` and refreshes the symlink each
`SessionStart`, so an upgrade is absorbed without the user touching anything.

The full chain has two hops:

```
~/.local/bin/accelerator                     (user, once, documented)
  -> ${CLAUDE_PLUGIN_DATA}/bin/accelerator   (hook, every SessionStart)
    -> <root>/bin/accelerator                (version-pinned)
```

The split is what makes it stale-proof: the hop the user creates points at a
target that **never moves**, so it never needs re-running; the hop that must
track the version is owned by the hook. Only the second hop is the plugin's
business.

**Why the hook does not write `~/.local/bin` itself.** It is the obvious
shortcut — Claude Code's own installer does exactly this
(`~/.local/bin/claude -> ~/.local/share/claude/versions/2.1.220`) — but that is
an installer the user ran deliberately, not a plugin hook firing unprompted at
every session start. Four differences decided it:

- **It would not remove the setup step on the primary platform.** `/etc/paths`
  on macOS is `/usr/local/bin`, `/System/Cryptexes/App/usr/bin`, `/usr/bin`,
  `/bin`, `/usr/sbin`, `/sbin` — `~/.local/bin` is not on `PATH` unless the user
  put it there. Debian/Ubuntu and Fedora add it when present; macOS does not.
- **It makes the channel collision non-deterministic.** Two installed channels
  would contend for one filename, so the winner becomes whichever session
  started most recently, changing silently. With per-channel `PLUGIN_DATA` dirs
  the user links the channel they want and `PATH` order settles the rest.
- **Clobber risk.** An existing `~/.local/bin/accelerator` — the user's own
  wrapper, or an unrelated tool — would need an "is this ours?" guard plus a
  refusal path, i.e. new logic and a new failure mode, to avoid destroying it.
- **Uninstall residue.** Removing the plugin would leave a broken `accelerator`
  command on the user's general `PATH`. Confined to `${CLAUDE_PLUGIN_DATA}`, the
  dangling link sits in plugin-owned space instead.

Remaining caveat — **mid-session upgrades.** Hook commands keep the previous
version's `${CLAUDE_PLUGIN_ROOT}` until `/reload-plugins`, so a shim refreshed
immediately after an upgrade may point at the old root for the rest of that
session. Not fatal — the old directory survives ~2 weeks — and the next session
corrects it.

### Rename set (R3)

Readers under `cli/` — production:

| Site                                                                                      | Current behaviour                                              |
|-------------------------------------------------------------------------------------------|----------------------------------------------------------------|
| `launcher/src/main.rs:176` (doc `:173`)                                                   | `Option`, `None` when unset — silently drops the template tier |
| `launcher/src/launch/outbound/resolve/cache_root.rs:26` (docs `:3,:38`; error text `:60`) | Hard `CacheRootUnavailable`; external subcommands only         |
| `visualiser/server/src/main.rs:69-70`                                                     | `eprintln!` + exit 2                                           |

Readers under `cli/` — tests and tooling:

| Site                                                                                             | Kind                                                              |
|--------------------------------------------------------------------------------------------------|-------------------------------------------------------------------|
| `launcher/tests/config_read.rs:62`                                                               | `env_remove`                                                      |
| `launcher/tests/config_read.rs:1169,1205`                                                        | injects a synthetic root                                          |
| `launcher/tests/version.rs:22`                                                                   | `env_remove`                                                      |
| `launcher/tests/version.rs:179`                                                                  | **asserts stderr contains the literal string** — message contract |
| `launcher/.../cache_root.rs:132,134`                                                             | unit-test asserts on the same message text                        |
| `visualiser/server/tests/api_smoke.rs:32`, `shutdown.rs:19`, `orchestration_lifecycle.rs:55,274` | injects / removes the root                                        |
| `visualiser/frontend/e2e/start-server.mjs:98`                                                    | injects the root for the E2E server                               |
| `corpus-adapters/tests/parity.rs:85`                                                             | comment reference only                                            |

Writers **outside** `cli/` that feed those readers — these must land in the same
change or `mise run` fails:

| Site                                                  | What it feeds                                                                                         |
|-------------------------------------------------------|-------------------------------------------------------------------------------------------------------|
| `tasks/dev.py:48`                                     | `{**os.environ, "CLAUDE_PLUGIN_ROOT": str(REPO_ROOT)}` → the dev visualiser server's hard-exit reader |
| `tasks/shared/dev/circus.py:43`                       | doc comment describing that propagation                                                               |
| `tasks/test/helpers.py:36`                            | `accelerator_env()` overlay used by the shell suites → template lookup at `launcher/src/main.rs:176`  |
| `tests/integration/dev/dev_integration_driver.py:138` | injects into the dev workspace                                                                        |

The bootstrap's own suite, `tests/integration/entrypoint/test_accelerator_entrypoint.py`:

- `:238` — the `_run_bootstrap` injection becomes **redundant**, not broken. The
  harness already `shutil.copy`s `bin/accelerator` into the fixture tree
  (`:210-212`) and invokes it by absolute path, so self-location resolves to the
  fixture root with no restructuring. **24 of the suite's 26 tests are unaffected**
  — path assertions all read the filesystem rather than comparing
  bootstrap-computed strings, and pytest's `tmp_path` is already canonical, so
  `/private/var` normalisation under `pwd -P` is harmless.
- `:260` `test_unset_plugin_root_is_a_named_error` — **delete**. It asserts
  exactly the behaviour being removed, and R5 is its inverse. Deletion is not
  merely tidiness: it is the only test besides the next one that runs the **repo**
  `bin/accelerator` rather than the fixture copy, and its env passes only `PATH`.
  With self-location it resolves the real repo root, satisfies the plugin.json,
  verify-shim, public-key and cache gates (all four `bin/accelerator-verify-*`
  triples are committed), then falls through to **real `curl` against the real
  GitHub release URL**, writing a cached launcher into the working tree's `bin/`.
  Leaving it in place is a network call and a working-tree write, not a red test.
- `:273` `test_non_directory_plugin_root_is_a_named_error` — **delete**; same
  shape, same hazard, and no env-var validation remains to test.
- `tests/unit/tasks/test_bootstrap_coverage.py` — **no change needed.** Despite
  its name it does not count gates or map tests to gates; it is four
  lint/discovery assertions (shfmt/shellcheck discovery, bashisms discovery, exec
  bit, shared-key coherence). Two *textual* couplings do constrain a rewrite: the
  literal `keys/accelerator-release.pub` must survive in `bin/accelerator`
  (asserted at `:39-43`), and the literal `.accelerator-dev-launcher` must
  survive it too (`test_accelerator_entrypoint.py:818`).

**No existing test invokes the bootstrap through a symlink** — the shape R1 and
R10 exist to support is entirely uncovered today.

**Exempt — do not blanket-rename.** `tests/unit/tasks/test_call_site_migration.py:26,35`
and `tests/unit/tasks/test_skill_permissions.py` (9 sites) are SKILL.md fixture
strings modelling the Claude Code surface, which keeps `${CLAUDE_PLUGIN_ROOT}`;
`tasks/lint/skill_permissions.py:43,55` models the matcher for the same reason.
Likewise the whole adapter layer: `hooks/`, `scripts/interactive-harness.sh`,
`scripts/test-design.sh`, `skills/config/migrate/**` and migrations `0001`–`0007`.

**The `cli/` set above was verified complete.** All 21 `CLAUDE_` occurrences under
`cli/` reconcile against these tables with no line drift, and
`CLAUDE_PLUGIN_ROOT` is the only `CLAUDE_*` variable that appears anywhere under
`cli/`. `ACCELERATOR_PLUGIN_ROOT` is unused across the whole repo, so the name is
free. Two incidental notes: `launcher/src/main.rs:176` has **no empty-string
filter**, unlike `cache_root.rs:27`, so an empty value becomes `Some("")` there;
and `ACCELERATOR_MIGRATION_MODE` is set by `cli/config-adapters/tests/config_reader.rs:34`
but read nowhere in `cli/` — dead, and cheap to remove while touching
neighbouring code.

### The existing guard cannot catch the substitution risk

`tasks/lint/skill_permissions.py` already enforces the `!`-site / `allowed-tools`
contract — including, at point 3 of its docstring, that every `config` command in
a `!` block carries `--fail-safe`. But `_PLUGIN_PREFIX = "${CLAUDE_PLUGIN_ROOT}/"`
(`:55`) *models* the matcher rather than exercising it. If Claude Code stopped
substituting the variable in `allowed-tools`, this guard would still pass while
every skill prompted. That is why the substitution question is resolvable only by
the manual pre-release check, and why R7 is a separate guard rather than an
extension of this one.

### Deliberately unchanged

- `cache_root.rs`'s refusal of an XDG fallback (`:3-5`). `<root>/bin` always
  exists under this goal; only the variable name changes.
- Version discovery from `.claude-plugin/plugin.json` (`bin/accelerator:44-49`).
- The visualiser server's fatal exit — reached via the launcher's `exec`, it
  inherits the exported root.
- Making the empty-table degradation visible. With self-location in place that
  path is unreachable in normal use, so it drops to diagnostic hygiene. Worth
  doing, not urgent; fold in only if cheap. **It is cheaper than assumed**: the
  root-sensitivity measurement above shows the affected surface is only the
  template family, and `plugin_root()`'s `Option` degrades silently at exactly
  four sites in `cli/config-adapters/src/store.rs` — `:282` `known_skill_names`
  → `Ok(vec![])` (suppressing skill-name validation), `:343` the plugin-default
  tier, `:357` `template_names` → `vec![]`, and `:413-417` `plugin_template_path`.
  Making the root non-optional at the config-command boundary would convert three
  of those into named errors. Reconsider on that basis rather than deferring by
  default — this is the design flaw the rename does not fix, and a future caller
  who forgets the export gets the same empty answers at exit 0.
- Extending `tasks/lint/skill_permissions.py` to reject **env-assignment
  prefixes** (`CLAUDE_PLUGIN_ROOT=… bin/accelerator …`) as well as `bash`/`sh`
  wrappers — a prevention recommendation from the source research. Once R1 lands
  no caller has a reason to set the variable, so the pattern this would catch
  stops being generated; it also belongs with the `allowed-tools` prefix-match
  work rather than here. Deferred, not rejected.
- The `${CLAUDE_SKILL_DIR}` migration of `allowed-tools` rules (bounded by the 47
  SKILL.md files that reference `bin/accelerator`). Out of scope
  **unconditionally** now that R11 is discharged positively — the rules keep their
  `${CLAUDE_PLUGIN_ROOT}` prefix and `_PLUGIN_PREFIX` in
  `tasks/lint/skill_permissions.py` stays as it is.

### Blast radius

43 skills invoke the CLI through the bootstrap from a `!` preprocessor site and
so fail at load before any Rust runs:
`config/paths`, all 3 `decisions/*`, 2 `design/*`, 3 `github/*`, 8
`integrations/jira/*`, 8 `integrations/linear/*`, `notes/create-note`, 5
`planning/*`, 3 `research/*`, `vcs/commit`, `visualisation/visualise`, 7
`work/*`. Also 47 SKILL.md files referencing `bin/accelerator` (410
`${CLAUDE_PLUGIN_ROOT}` occurrences across 48 files, all in `skills/**/SKILL.md`
— `agents/**` has none) and ~20 helper scripts. Those `!` sites carry **204**
`bin/accelerator config` commands between them, ~125 distinct (see the automated
acceptance criterion for the measured shape).
Unaffected: the two `hooks/` entry points that reach the CLI
(`config-detect.sh`, `migrate-discoverability.sh`), which run as hook processes and do
receive the export — which is why SessionStart config detection kept working
while nearly every skill was broken.

## Drafting Notes

- Scoped as a single bug rather than bug-plus-refactor: R3/R7 (the variable
  purge) are larger than the minimum repair, but the maintainer set removing the
  Claude Code coupling as the goal, and doing it in the same change avoids
  renaming the same call sites twice. R1+R2 alone would ship a correct fix if the
  next prerelease is urgent — split there if needed.
- R3 was originally scoped "under `cli/`". Widened after finding four writers
  outside `cli/` (`tasks/dev.py`, `tasks/shared/dev/circus.py`,
  `tasks/test/helpers.py`, `tests/integration/dev/dev_integration_driver.py`)
  that feed `cli/` readers. Renaming readers alone cannot ship green, so the
  requirement is stated by direction rather than by directory.
- R8 and R9/R10 were promoted from Open Questions on the maintainer's decision
  that terminal invocation is a supported surface and that the `--fail-safe`
  bootstrap belongs here. R8 remains the smaller change; R9 is the largest scope
  addition in this item, adding a fourth `SessionStart` hook and user-facing
  documentation. If scope needs cutting, R9 is the separable piece — R10 and the
  symlink chase stand on their own as insurance for user-made links.
- R9 was first drafted as a single hop, with the user adding the
  `${CLAUDE_PLUGIN_DATA}` directory to `PATH`. Revised to the two-hop chain after
  the maintainer asked whether the hook could write `~/.local/bin` directly.
  It could — that is how `claude` itself installs — but having the *hook* own a
  user-general `PATH` entry buys little on macOS (where `~/.local/bin` is not on
  `PATH` by default anyway) and costs unprompted writes, clobber risk, uninstall
  residue, and a non-deterministic winner across channels. Splitting the chain
  keeps the hook inside plugin-owned space while still giving the user a `PATH`
  entry that cannot go stale. Revisit if a deliberate `install-shim` subcommand
  is ever wanted — that was the third option considered and rejected only for
  scope.
- The `allowed-tools` substitution assumption was rewritten rather than closed.
  It was checked against the v2.1.220 docs during this pass and came back
  *weaker* than previously recorded — documented for `${CLAUDE_SKILL_DIR}` and
  `${CLAUDE_PROJECT_DIR}` but not for `${CLAUDE_PLUGIN_ROOT}`. Treating it as
  settled would risk shipping a fix that leaves all 43 skills prompting.
- Priority high, not critical: the scale argues for critical, but the available
  values are high/medium/low.
- `mise.local.toml` removal (R6) was confirmed with the maintainer as serving no
  other purpose, so the earlier hedge is gone and it now carries its own
  acceptance criterion.

### Review pass 1 (2026-07-26)

Five lenses (clarity, completeness, dependency, scope, testability) returned one
critical and nine major findings; see
`meta/reviews/work/0182-cli-derives-plugin-root-from-own-location-review-1.md`.
The substantive changes made in response:

- **Exit-0 assertions were tautological.** R8 makes a bootstrap-layer abort exit
  0, and the launcher already exits 0 with empty output when rootless, so the
  flagship automated criterion could not fail. Every success criterion now pairs
  exit status with an output assertion (non-empty stdout, empty stderr, or a
  named table row), and R5 was pinned to `config templates list` **without**
  `--fail-safe` for the same reason.
- **R4 conflicted with R1.** The seam injects a deliberately-wrong root but
  reaches the launcher through the bootstrap, which R1 makes ignore ambient
  roots. Resolved by routing the seam past the bootstrap via `ACCELERATOR_BIN`
  (maintainer's choice over adding a third variable), which keeps R1's
  unconditional self-location intact. The requirement is no longer "four one-line
  edits" — the suite must resolve a launcher path too.
- **The substitution question was a trapdoor.** It could have ballooned into a
  43-file migration discovered at the release gate, so it is now R11: probe
  before implementation, with the migration explicitly out of scope in either
  branch and a conditional Dependencies edge recording the successor item.
- **R9 was kept but pinned down.** The maintainer's decision that terminal
  invocation is a supported surface stands, so instead of splitting it out the
  gaps were closed: the hook is named (`hooks/shim-refresh.sh`, fourth
  `SessionStart` entry), its documentation target is named (R9a,
  `docs/internals.md`), its inputs are declared environment-overridable so the
  criteria are testable out-of-band, it is inert when `${CLAUDE_PLUGIN_DATA}` is
  unset, and the containment criterion became a temp-tree snapshot diff scoped to
  that hook alone rather than an unbounded whole-filesystem inspection.
- **Dependencies said "Blocked by: none"** while the item rested on four
  unverified Claude Code behaviours and a lockstep launcher-artifact release. Both
  are now recorded, along with the successor items either sanctioned split would
  create.
- **Vocabulary.** "Tier" carried four senses and "CLI tiers" silently defined the
  rename's scope; a layer table now fixes bootstrap / launcher / server / adapter,
  R3 defers to the enumerated rename table for membership, and "stable" is
  reserved for the release channel ("fixed-path" for the shim).
- Not adopted: splitting R9 out (maintainer kept it), and promoting the item off
  `kind: bug` — the scope lens flagged that ten requirements across five
  toolchains understate as a bug, but one item per investigation is the norm here.
- Variable name: `ACCELERATOR_PLUGIN_ROOT` was the source research's recommendation
  over `ACCELERATOR_HOME` and `ACCELERATOR_ROOT`, carried forward on its stated
  basis — the install is always a plugin install so the noun is accurate, the name
  is greppable during the migration, and the ownership transfer is obvious at every
  call site. Recorded here so it is not re-litigated at implementation time.

### Review pass 2 (2026-07-26)

Re-ran the same five lenses. No critical findings; the pass-1 critical (tautological
exit-0 criteria) and the R4/R1 contradiction were confirmed resolved. Three of the
new majors were defects introduced by the pass-1 edits, and all are fixed here:

- **"Empty stderr" was unsatisfiable.** The same criteria mandate the gated dev
  launcher, which warns on stderr every invocation, so the assertion is now "no
  `accelerator:` diagnostic line on stderr" — still discriminating against the
  fail-safe signature, but achievable.
- **The repo-wide purge criterion could never pass.** It omitted
  `skills/**/SKILL.md` and `agents/**`, whose 410 `${CLAUDE_PLUGIN_ROOT}`
  substitution tokens R11 deliberately keeps. It is now a mechanical
  `grep -rl` with the full exemption set, and "reads" is defined as
  process-environment access rather than a substituted token. (Still wrong after
  this pass, on both counts — the exemption set was *also* missing four paths, and
  `agents/**` has zero occurrences. Corrected in the codebase research pass
  below.)
- **Closure was stated in both directions.** "Blocked by: none" sat beside "does
  not close until the successor ships". Resolved in favour of closing on this
  item's own criteria, with the prerelease — not the item's status — gated on the
  conditional migration.

The substantive corrections beyond my own errors:

- **R4 was wrong about its own mechanism.** `work-item-template-field-hints.sh`
  never reads `CLAUDE_PLUGIN_ROOT` — it self-locates and shells out to the CLI, and
  the seam works only because the *bootstrap* rejects `/nonexistent` as a
  directory. Renaming the variable would not restore it, and neither would
  injecting into the launcher (a rootless launcher exits 0, so the fallback never
  fires). The seam now states its actual intent by pointing `ACCELERATOR_BIN` at a
  nonexistent binary, and a new criterion proves the fallback is still exercised by
  requiring the assertions to fail if that branch is removed.
- **R11's probe could not have answered its own question.** Running the manual
  check early proves nothing while all 43 skills abort at load before any Bash
  call. The probe now addresses the matcher directly, with both outcomes
  distinguishable under the broken bootstrap.
- **The urgency split would have shipped a dead export.** R2 exports the new name;
  R3 is what teaches the launcher to read it. The split lines are now exhaustive
  and record that R3's reader half cannot be deferred (or the bootstrap must export
  both names transitionally).
- Added R12 (the `${CLAUDE_PLUGIN_DATA}` floor determination, previously only an
  open question), criteria for the R9a documentation and the `hooks.json`
  registration, the launcher build-order dependency, and the candidate-artifact
  closure sequence the manual check implies.
- Still open by choice: the scope lens' recommendation to land R6 standalone before
  implementation (worth doing — the file masks the bug class in every local run
  while the work proceeds) and its objection that the urgent repair is gated behind
  R9. Both are maintainer scheduling calls, not defects.

### Codebase research pass (2026-07-27)

Verified every enumerated site against the tree and measured the launcher's actual
rootless behaviour; see
`meta/research/codebase/2026-07-27-0182-plugin-root-self-location-implementation-surface.md`.
The inventory held up — the `cli/` rename set is complete and exact, no line
number had drifted, `ACCELERATOR_PLUGIN_ROOT` is unused repo-wide, and every
pattern R7/R9/R10 needs already has a close precedent. Four mechanisms did not,
and are corrected above:

- **Acceptance criterion 1 was unsatisfiable.** It required `config instructions
  commit --fail-safe` to emit "a known substring of the shipped commit
  instructions". There are none — `config instructions` renders a *user* per-skill
  override. Measured 0 bytes at exit 0 both rootless and rooted. Split into a
  falsifiable `config template adr` criterion plus a reported-failure-command
  criterion that explicitly does not assert on stdout.
- **R4 had its mechanism wrong twice, and was sequenced against the wrong
  requirement.** Under `mise run` the seam bypasses the bootstrap entirely via
  `ACCELERATOR_BIN`, and a rootless launcher exits **1**, not 0, for `config
  template work-item` (a `Failure::Refusal`, fail-closed even under
  `--fail-safe`) — so renaming the variable *would* have restored the seam, and
  the vacuity is caused by R3, not R1. The prescribed `ACCELERATOR_BIN` fix stands
  and is now justified on the stronger ground that it is the repo's established
  convention (28 files), not a workaround.
- **Only the template family is root-sensitive.** Seven other `config` families
  produce byte-identical output with and without a root. This narrows R2's
  silent-degradation argument to `config templates list` (still sufficient),
  reveals that a non-exporting fix would fail *loudly* at the 20 `config template`
  `!` sites, and makes the deferred `plugin_root`-`Option` cleanup cheaper than
  assumed.
- **The purge criterion, the dev-override precondition, R12's target and the gate
  count were all wrong in detail.** The `grep -rl` exemption set omitted four
  paths and listed `agents/**` (zero occurrences); the dev override cannot supply
  a launcher to a fixture-rooted test (its containment gate requires the repo
  root, and the marker is forbidden there); `.claude-plugin/plugin.json` has no
  version-floor field to raise; and `fail()` has 16 gates, not 14.

Also folded in, as scope that was previously undiscovered rather than wrong: the
`!`-extraction suite is simpler than scoped (zero argument interpolation across
all 204 commands, so that branch is dead) but harder in one respect (~86 are
legitimately empty and *which* ones depends on project config, so it needs a
fixture project); the build-order dependency is already solved by the existing
`build:cli:dev` edge, with the caveat that mise `depends` is a set and not a
sequence; R7 must ride in `cli:check` because no CI job runs `lint:check`; the
symlink chase has a reviewed prior implementation that already settled cycle
detection and the `ELOOP` test constraint; the two entrypoint tests to delete are
actively hazardous rather than merely red; and R9 reverses a recorded
"no `${CLAUDE_PLUGIN_DATA}` dependency" decision.

Deliberately not changed: R11 remains unanswerable from the codebase, but the
note that a probe run from this repo is contaminated while `mise.local.toml`
exists is new, and strengthens the existing case for landing R6 first.

### Planning pass (2026-07-27)

Plan written at
`meta/plans/2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename.md` —
six phases plus a closing local-cleanup step. Four changes were made to this item
during that pass:

- **R11 discharged positively**, on the maintainer's direct observation plus
  verification that `~/.claude/settings.json` carries `defaultMode: "default"`
  with zero Bash allow rules — which excludes the broad-grant confound this item
  had raised against its own probe. The `${CLAUDE_SKILL_DIR}` migration is now
  out of scope unconditionally and the conditional Dependencies edge is gone.
- **R6 resequenced from first to last.** Both source documents recommended landing
  it first; neither noticed that `mise.local.toml` is what supplies
  `CLAUDE_PLUGIN_ROOT` to the *installed, still-unfixed* plugin in this
  repository, so deleting it early disables the very skills needed to carry out
  the work. Also established: it is absent from `main`, `origin/main` and `@-` —
  never pushed — so it masks nothing beyond local runs, and its removal is a
  working-copy cleanup rather than a mergeable change. The mirror hazard is now
  recorded in the plan: it must not ride along into a pushed commit, where CI
  would resolve a nonexistent `/Users/…` path.
- **R12's target fixed as the two prose sites**, not a new `plugin.json` field.
- **Three acceptance criteria restated** (1/3, 4, 5). `display_path`
  (`cli/config-adapters/src/store.rs:73-85`) deliberately renders plugin-root
  paths as the literal `<plugin>/…` token, so no criterion can assert a Path
  "lies under the resolved installation root" — measured. The plan substitutes
  sentinel strings in the fixture's `templates/adr.md` and asserts on template
  contents, which is absolute and falsifiable in both directions.

Also resolved rather than restated: the open question about the dev-override
precondition. The existing entrypoint harness's `_serve_launcher` signs whatever
file is placed in the stub release server, so serving the real compiled
`cli/target/debug/accelerator` exercises the genuine fetch → verify → cache → exec
chain from a fixture root with no network and no override — which is why the
containment-gate conflict never arises.

Deferred cleanup promoted into scope on the maintainer's decision: making
`plugin_root` non-optional at the config-command boundary is now the plan's final
phase, converting three of four silent degradations into named errors.

Incidental finding, out of scope here and worth its own item: `create-plan`'s
`allowed-tools` grants only
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)`, yet its body instructs
running `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`. That call has
no covering rule and so prompts. `tasks/lint/skill_permissions.py` cannot catch it
because the gap is in prose rather than a `!` block — a class of hole the guard is
structurally blind to.

## References

- Source: `meta/research/issues/2026-07-26-cli-requires-claude-plugin-root-env-var.md`
- Review: `meta/reviews/work/0182-cli-derives-plugin-root-from-own-location-review-1.md`
- Implementation-surface research:
  `meta/research/codebase/2026-07-27-0182-plugin-root-self-location-implementation-surface.md`
  (verified the rename set, measured root-sensitivity per `config` family, and
  corrected R4's mechanism, the gate count, and four acceptance criteria)
- Symlink-chase prior art: `meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md:620-655`
  and its review `meta/reviews/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding-review-1.md:392,738,757,835,856`
- Prior `${CLAUDE_PLUGIN_DATA}` decision R9 reverses:
  `meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md:125`
- ADR-0048 "Four-toolchain split": `meta/decisions/ADR-0048-four-toolchain-split.md`
  (why R7's guard is a Python invoke task, not a Rust check)
- Related: 0164, 0167, 0136
- Related research: `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`
  (the `allowed-tools` prefix-match mechanism behind the approval prompt)
- Claude Code docs: [plugins reference § Environment variables](https://code.claude.com/docs/en/plugins-reference)
  — "All three are exported as environment variables to hook processes and to MCP
  and LSP server subprocesses"; the substitution-by-component table; and
  § Persistent data directory for `${CLAUDE_PLUGIN_DATA}` semantics.
- Claude Code docs: [skills § available string substitutions](https://code.claude.com/docs/en/skills)
  — names only `${CLAUDE_SKILL_DIR}` and `${CLAUDE_PROJECT_DIR}` as substituted
  in `allowed-tools` Bash rules.
