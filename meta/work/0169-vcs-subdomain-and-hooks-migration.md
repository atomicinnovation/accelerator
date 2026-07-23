---
type: work-item
id: "0169"
title: "VCS Subdomain and Hooks Migration"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: ready
kind: story
priority: high
parent: "work-item:0136"
blocked_by: ["work-item:0164", "work-item:0166", "work-item:0167", "work-item:0179"]
blocks: ["work-item:0172", "work-item:0174"]
relates_to: ["work-item:0172", "work-item:0174"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
tags: [rust, vcs, hooks, migration]
last_updated: "2026-07-20T09:34:16+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-190"
---

# 0169: VCS Subdomain and Hooks Migration

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Build the `accelerator-vcs` subdomain (detection/status/log/guard) and migrate the
SessionStart and PreToolUse hook logic into the CLI (ADR-0048), registering the
bootstrap path (`${CLAUDE_PLUGIN_ROOT}/bin/accelerator`) in `hooks.json` to invoke
domain subcommands with a `--format=hook` envelope rather than a hook-specific
subcommand. The SessionStart migration covers VCS detection and config detection
only; the config side is **already shipped by 0167** as
`accelerator config summary --format=hook --fail-safe` (there is no
`config detect` subcommand — 0167 reuses `config summary`), so this story owns
the VCS half. The migration-discoverability reminder is deferred to 0172.

## Context

`scripts/vcs-common.sh` (the `.jj`-wins dispatch, the load-bearing
`classify_checkout` arm ordering) backs both the VCS skills and the hooks. ADR-0048
says hook logic moves into the CLI; consolidating it there removes the parallel bash
implementation the hooks and skills currently share and unblocks the remaining
epic-0136 shell retirement (0172, 0174) — the goal this story ultimately serves. The
hooks are currently four bare `.sh` paths in `hooks.json` honouring the Claude Code
hook I/O protocol. The invocation mechanism
(resolved during refinement of the parent epic 0136) is fixed: register the
universal wrapper invoking domain subcommands, with the hook I/O envelope produced
by a CLI `--format=hook` switch. The SessionStart hook
currently fans out to VCS detection, config detection, and a migration-discoverability
reminder; this story ports the first two and leaves the migration reminder to land
with the migration subdomain (0172).

## Requirements

- Implement `accelerator-vcs` as a hexagon: `vcs detect`, `vcs status`, `vcs log`,
  `vcs guard`, over the `vcs`/`vcs-adapters` crates delivered by 0179 — porting
  `vcs-common.sh` semantics (jj-outranks-git dispatch, the 6-line `classify_checkout`
  record with its first-match-wins arm order, worktree/submodule/bare/`GIT_DIR`
  handling) and extending those crates with the full `classify_checkout` taxonomy.
  "Full taxonomy" here means reproducing the shell's existing classification set
  (worktree, submodule, bare, `GIT_DIR`, plain) as typed variants — not adding
  classifications beyond what the shell produces. External VCS state (`jj`,
  `git`) is read through in-process Rust libraries — `gix` (gitoxide) for git
  and `jj-lib` for jujutsu — bound behind the outbound port, not by spawning
  `jj`/`git` subprocesses; a per-query shell fallback behind the same port is
  permitted only where no library equivalent exists.
- Migrate the VCS hook logic into the CLI: `hooks.json` registers the bootstrap
  path (which fetches `accelerator` on first use, then execs it) invoking domain
  subcommands — SessionStart → `accelerator vcs detect` (the config side,
  `accelerator config summary --format=hook --fail-safe`, is already registered by
  0167); PreToolUse(`Bash`) → `accelerator vcs guard`. No hook-specific subcommand.
  The SessionStart migration-discoverability reminder is out of scope here and
  remains its existing bash hook until 0172.
- Add a `--format=hook` switch producing the Claude Code hook I/O envelope, so one
  domain operation serves both its skill-injection caller (plain) and its hook
  caller (envelope).
- Serve the hot PreToolUse guard as the `accelerator-vcs` sub-binary invoked through
  the wrapper, relying on the 0164 fetch-verify-cache model so the sub-binary fetch
  is a first-use-only cost amortised by the warmed cache — no per-Bash-call fetch
  after warm-up. Keep the wrapper bash-3.2-safe.

## Acceptance Criteria

- [ ] `accelerator vcs detect` reproduces the shell detection behaviour, verified
      against the repointed `hooks/test-vcs-detect.sh` parity gate.
- [ ] `accelerator vcs status` and `accelerator vcs log` reproduce the shell
      behaviours, each verified against a golden-output test asserting the same
      output as the corresponding `vcs-common.sh` path across a fixture-repo set
      covering, at minimum: clean git, dirty git, git ahead/behind, detached-HEAD
      git, clean jj, and dirty jj.
- [ ] `accelerator vcs guard` reproduces the shell guard behaviour, verified
      against an allow/deny fixture set that covers each Bash-call class the shell
      guard distinguishes — the blocked git/jj mutation patterns and the
      allowed read-only/non-VCS patterns — not merely one allow and one deny.
- [ ] `classify_checkout` is verified by a fixture covering each arm (worktree,
      submodule, bare, `GIT_DIR`, plain) and asserting first-match-wins precedence
      for at least one ambiguous checkout, so the load-bearing arm order cannot
      regress silently.
- [ ] The `accelerator-vcs` adapters read VCS state through the `gix` and
      `jj-lib` libraries rather than by spawning `jj`/`git` subprocesses —
      verified by asserting zero `jj`/`git` process spawns across the
      detect/status/log/guard paths under an instrumented process launcher; any
      shell fallback is explicit, per-query, and covered by its own test.
- [x] The SessionStart config-detection behaviour is reproduced — **delivered by
      0167** as `accelerator config summary --format=hook --fail-safe`, registered
      via `hooks/config-detect.sh` (a thin exec-wrapper of the bootstrap path).
- [ ] `hooks.json` registers the bootstrap path invoking the VCS subcommands;
      SessionStart injects the VCS context and PreToolUse guards Bash calls, each
      emitting its hook I/O envelope via `--format=hook`, verified against a golden
      envelope fixture per hook type (SessionStart, PreToolUse). **The two envelopes
      are different shapes** — SessionStart is
      `{hookSpecificOutput:{hookEventName,additionalContext}}`; the PreToolUse guard
      is `{decision, reason}` / `{decision, hookSpecificOutput}` — so the fixture is
      per hook type, not one shared envelope.
- [ ] After first use, the PreToolUse guard resolves `accelerator vcs guard` from the
      warmed cache — verified by asserting zero sub-binary fetch invocations against
      a stubbed/instrumented fetcher across repeated guard calls after the cache is
      warm.
- [ ] The wrapper passes the bash-3.2 gate — `scripts/lint-bashisms.sh` (and the
      standard shfmt/ShellCheck checks) report no findings against it.
- [ ] The `vcs-detect.sh` and `vcs-guard.sh` hook scripts are removed and the hooks
      suite floor adjusted in the same change; `migrate-discoverability.sh` is left
      in place for 0172. `config-detect.sh` was re-homed by 0167 to a thin
      exec-wrapper of the bootstrap path — this story may fold it directly into
      `hooks.json` and delete it **only once the argument-splitting probe below is
      resolved** (whether `hooks.json`'s `command` field expands
      `${CLAUDE_PLUGIN_ROOT}` and splits argument tokens); until then the wrapper
      stays.

## Open Questions

- None outstanding — the config-detect / migrate-discoverability scope boundary was
  resolved during refinement (config detect in scope here; migrate discoverability
  deferred to 0172).

## Dependencies

- Blocked by: 0164 (the fetch-verify-cache model the warmed-cache guard relies on —
  AC "guard resolves from the warmed cache with no per-Bash-call fetch" is not
  validatable until 0164 lands), 0166 (shared crates — complete), 0167 (the wrapper
  + invocation contract, and the built-in `config` command this story extends with
  `config detect` — in progress, expected complete before this is picked up),
  0179 (the `vcs`/`vcs-adapters` crates this subdomain consumes and extends —
  complete on the feature branch).
- Blocks: 0172 (its final migrate-discoverability hook migration builds on this
  story's `hooks.json` rewrite), 0174 (retires the wrapper/shell tail left precisely
  because this story removes only three of the four hook scripts).
- Related: 0172, 0174 (see Blocks — both consume this story's `hooks.json` output).
- Parent: epic 0136.

## Assumptions

- The four target platforms (the epic 0136 target triples per
  `tasks/shared/targets.py`: `aarch64`/`x86_64-apple-darwin` and
  `aarch64`/`x86_64-unknown-linux-musl`) are all Unix.
- After the first-use fetch, the guard's warmed-cache per-Bash-call cost is
  comparable to today's `vcs-guard.sh` spawn. Only the absence of a per-call fetch
  is gated by an acceptance criterion; per-call latency parity is an assumption, not
  a verified bound.

## Technical Notes

- Source bash: `scripts/vcs-common.sh`, `hooks/vcs-detect.sh`, `hooks/vcs-guard.sh`,
  `hooks/config-detect.sh`, `hooks/hooks.json`, and the
  `hooks/test-fixtures/vcs-detect/` fixtures. (`hooks/migrate-discoverability.sh` is
  deliberately out of scope — it ports with the migration subdomain in 0172.)
- The `vcs`/`vcs-adapters` crates land in 0179; this story extends them with the full
  `classify_checkout` taxonomy rather than defining them from scratch.
- `classify_checkout` arm ordering is load-bearing — preserve first-match-wins
  semantics exactly.
- **Implementation approach — bind VCS libraries, don't shell out.** The adapters
  should drive git and jujutsu through in-process Rust libraries — `gix`
  (gitoxide) for git and `jj-lib` for jujutsu — bound behind the outbound VCS
  ports, rather than spawning `jj`/`git` child processes. This departs from the
  current `CommandProbe` adapter (`cli/vcs-adapters/src/lib.rs:48-196`), which
  runs `jj log`/`git rev-parse` as subprocesses; the port abstraction
  (`cli/vcs/src/lib.rs:48-55`) already allows swapping in a library-backed
  adapter without touching the domain.
  - Git queries `classify_checkout` needs map to `gix`: bare check
    (`git rev-parse --is-bare-repository`, `vcs-common.sh:207`), worktree
    detection (`--git-dir` vs `--git-common-dir`, `vcs-common.sh:217-219`), and
    superproject/submodule resolution (`--show-superproject-working-tree`,
    `vcs-common.sh:140-146`).
  - jj queries map to `jj-lib`: workspace-root resolution and the
    main-vs-secondary distinction (`.jj/repo` dir vs file,
    `vcs-common.sh:74-81`), plus revision reads.
  - Risk to validate early: `jj-lib`'s public API is explicitly unstable and
    pins to the jj release; confirm its workspace/repo-loading surface covers
    the secondary-workspace and colocated cases before committing. `gix` is the
    more mature of the two. Where a query has no library equivalent, a per-query
    shell fallback *behind the same port* is acceptable, but library-first is
    the default.
- Behavioural reference (`path:line`):
  - jj-outranks-git dispatch — a command-set selector, not a topology (git's
    index lags jj's working copy in colocated): `scripts/vcs-common.sh:27-36`.
  - `classify_checkout` contract + six-line `KEY=VALUE` record:
    `scripts/vcs-common.sh:157-176`; body `:177-280`.
  - Load-bearing arm cascade (first-match-wins) — `colocated` must precede the
    `nested-*` arms: `scripts/vcs-common.sh:240-272`.
  - Hook I/O envelopes to reproduce under `--format=hook`: SessionStart
    `{hookSpecificOutput:{hookEventName,additionalContext}}` + optional
    `systemMessage` (`hooks/vcs-detect.sh:177-181`); PreToolUse guard
    `{decision:block,reason}` (pure-jj) vs
    `{decision:allow,hookSpecificOutput}}` (colocated)
    (`hooks/vcs-guard.sh:97-108`).
  - Guard command-parsing (a separate behaviour from detection): compound-command
    split on `&& || ; |` then git-subcommand pattern match
    (`hooks/vcs-guard.sh:44-108`); `gh`/`rtk` unconditionally allowed.
  - Rust starting point: `vcs`/`vcs-adapters` today model corpus `RepoFacts`
    (root/name/kind/revision), *not* the hook taxonomy —
    `cli/vcs/src/lib.rs:32-78`; parity tests assert facts only
    (`cli/vcs-adapters/tests/detection.rs`). The launcher dispatches any new
    `accelerator-vcs` external subcommand with no launcher changes
    (`cli/launcher/src/launch/inbound/cli.rs:15-22`).

## Notes from 0167 (2026-07-22)

- **The SessionStart envelope contract is settled by 0167** and this story
  inherits it: `accelerator config summary --format=hook` emits
  `{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"…"}}`
  (compact `serde_json`, not `jq`'s pretty-print), emits **nothing** when the
  summary is empty, and — with `--fail-safe` — exits 0 with a stderr diagnostic on
  a read/IO failure. `accelerator vcs detect`'s SessionStart output must slot into
  the same `additionalContext` shape.
- **The registration names the bootstrap path**, `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`,
  not a "universal wrapper" — 0164/0165's fetch-verify-cache launcher. The word
  "wrapper" in this story's earlier prose refers to that bootstrap script.
- **PreToolUse's envelope is this story's own to define.** `vcs-guard.sh` emits
  `{decision, reason}` and `{decision, hookSpecificOutput}`, an unrelated shape to
  SessionStart's — there is no single envelope spanning all hooks, so the
  `--format=hook` switch renders a per-hook-type envelope, not one uniform one.
- **The hooks.json argument-splitting question is unresolved and shared.** 0167
  could not confirm headlessly whether `hooks.json`'s `command` field expands
  `${CLAUDE_PLUGIN_ROOT}` *and* splits argument tokens, so it left
  `config-detect.sh` as a thin exec-wrapper rather than inlining
  `bin/accelerator config summary --format=hook --fail-safe` into `hooks.json`.
  This story resolves the probe (it must, to register argument-bearing VCS
  subcommands) and can then fold the config registration in and delete the wrapper.

## Drafting Notes

- Treated as the Phase 6 story; bundles the VCS subdomain with the hooks migration
  because the hooks are the VCS subdomain's primary consumer.
- Scope boundary confirmed with the author: `config detect` is migrated here; the
  SessionStart migrate-discoverability reminder is deferred to 0172, so this story
  removes three hook scripts, not four.
- The built-in-vs-sub-binary question was closed: the guard is served as the
  `accelerator-vcs` sub-binary, and the fetch cost is a first-use-only hit amortised
  by the 0164 warmed cache — so it is not carried as an open question.
- Priority raised medium → high: this is a critical-path Phase 6 story that gates the
  hooks migration for epic 0136.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Related: 0166, 0167, 0172, 0174, 0179
- ADRs: ADR-0048, ADR-0053
