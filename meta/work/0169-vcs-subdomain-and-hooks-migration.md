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
universal bash wrapper in `hooks.json` to invoke domain subcommands with a
`--format=hook` envelope rather than a hook-specific subcommand. The SessionStart
migration covers VCS detection and config detection only (the latter via a
cross-subdomain `accelerator config detect` port); the migration-discoverability
reminder is deferred to 0172.

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
  classifications beyond what the shell produces. External tools (`jj`, `git`)
  behind an outbound port.
- Migrate the VCS and config hook logic into the CLI: `hooks.json` registers the
  universal bash wrapper (which fetches `accelerator` on first use, then execs it)
  invoking domain subcommands — SessionStart → `accelerator vcs detect` +
  `accelerator config detect`; PreToolUse(`Bash`) → `accelerator vcs guard`. No
  hook-specific subcommand. The SessionStart migration-discoverability reminder is
  out of scope here and remains its existing bash hook until 0172.
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
- [ ] `accelerator config detect` reproduces the SessionStart config-detection
      behaviour of `config-detect.sh`, verified against its hook test.
- [ ] `hooks.json` registers the wrapper invoking domain subcommands; SessionStart
      injects the VCS and config context and PreToolUse guards Bash calls, both
      emitting the hook I/O envelope via `--format=hook`, verified against a golden
      envelope fixture per hook type (SessionStart, PreToolUse) that pins the
      Claude Code hook I/O protocol fields.
- [ ] After first use, the PreToolUse guard resolves `accelerator vcs guard` from the
      warmed cache — verified by asserting zero sub-binary fetch invocations against
      a stubbed/instrumented fetcher across repeated guard calls after the cache is
      warm.
- [ ] The wrapper passes the bash-3.2 gate — `scripts/lint-bashisms.sh` (and the
      standard shfmt/ShellCheck checks) report no findings against it.
- [ ] The `vcs-detect.sh`, `vcs-guard.sh`, and `config-detect.sh` hook scripts are
      removed and the hooks suite floor adjusted in the same change;
      `migrate-discoverability.sh` is left in place for 0172.

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
