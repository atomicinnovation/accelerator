---
type: work-item
id: "0169"
title: "VCS Subdomain and Hooks Migration"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
tags: [rust, vcs, hooks, migration]
last_updated: "2026-06-28T17:01:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-190"
---

# 0169: VCS Subdomain and Hooks Migration

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Build the `accelerator-vcs` subdomain (detection/status/log/guard) and migrate the
SessionStart/PreToolUse hook logic into the CLI (ADR-0048), registering the
universal bash wrapper in `hooks.json` to invoke domain subcommands with a
`--format=hook` envelope rather than a hook-specific subcommand.

## Context

`scripts/vcs-common.sh` (the `.jj`-wins dispatch, the load-bearing
`classify_checkout` arm ordering) backs both the VCS skills and the hooks. ADR-0048
says hook logic moves into the CLI; the hooks are currently four bare `.sh` paths in
`hooks.json` honouring the Claude Code hook I/O protocol. Resolved Q4 fixes the
mechanism: register the universal wrapper invoking domain subcommands, with the hook
I/O envelope produced by a CLI `--format=hook` switch.

## Requirements

- Implement `accelerator-vcs` as a hexagon: `vcs detect`, `vcs status`, `vcs log`,
  `vcs guard`, porting `vcs-common.sh` semantics (jj-outranks-git dispatch, the
  6-line `classify_checkout` record with its first-match-wins arm order, worktree/
  submodule/bare/`GIT_DIR` handling). External tools (`jj`, `git`) behind an
  outbound port.
- Migrate hook logic into the CLI: `hooks.json` registers the universal bash
  wrapper (which fetches `accelerator` on first use, then execs it) invoking domain
  subcommands — SessionStart → `accelerator vcs detect` / `accelerator config detect`
  / `accelerator migrate discoverability`; PreToolUse(`Bash`) → `accelerator vcs
  guard`. No hook-specific subcommand.
- Add a `--format=hook` switch producing the Claude Code hook I/O envelope, so one
  domain operation serves both its skill-injection caller (plain) and its hook
  caller (envelope).
- Decide built-in vs `accelerator-vcs` sub-binary for the hot PreToolUse guard
  (lean built-in to avoid a per-Bash-call sub-binary fetch); keep the wrapper
  bash-3.2-safe.

## Acceptance Criteria

- [ ] `accelerator vcs detect|status|log|guard` reproduce the shell behaviours,
      verified against the repointed `hooks/test-vcs-detect.sh` parity gate.
- [ ] `hooks.json` registers the wrapper invoking domain subcommands; SessionStart
      injects the VCS/config/migrate context and PreToolUse guards Bash calls, both
      emitting the correct hook I/O envelope via `--format=hook`.
- [ ] The PreToolUse guard path does not trigger a sub-binary fetch on every Bash
      call (built-in, or warmed cache).
- [ ] The hook `.sh` files (`vcs-detect.sh`, `vcs-guard.sh`, `config-detect.sh`,
      `migrate-discoverability.sh`) are removed and the hooks suite floor adjusted
      in the same change.

## Open Questions

- Whether `config detect` and `migrate discoverability` are fully covered here or
  partly deferred to 0167 (config) / 0172 (migrate) — sequence at implementation.

## Dependencies

- Blocked by: 0166 (shared crates), 0167 (the wrapper + invocation contract).
- Parent: epic 0136.

## Assumptions

- The four target platforms are Unix; the guard's per-Bash-call cost is comparable
  to today's `vcs-guard.sh` spawn.

## Technical Notes

- Source bash: `scripts/vcs-common.sh`, `hooks/vcs-detect.sh`, `hooks/vcs-guard.sh`,
  `hooks/config-detect.sh`, `hooks/migrate-discoverability.sh`, `hooks/hooks.json`,
  and the `hooks/test-fixtures/vcs-detect/` fixtures.
- `classify_checkout` arm ordering is load-bearing — preserve first-match-wins
  semantics exactly.

## Drafting Notes

- Treated as the Phase 6 story; bundles the VCS subdomain with the hooks migration
  because the hooks are the VCS subdomain's primary consumer.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0048, ADR-0053
