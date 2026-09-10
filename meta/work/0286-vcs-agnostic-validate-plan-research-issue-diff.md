---
type: "work-item"
id: "0286"
title: "Make validate-plan and research-issue VCS-agnostic via an accelerator vcs diff subcommand"
date: "2026-09-10T11:44:31+00:00"
author: "Toby Clemson"
producer: "conduct-spike"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0136"
blocked_by: ["work-item:0200"]
relates_to: ["work-item:0169", "work-item:0198", "work-item:0200"]
tags: ["vcs", "cli", "skills"]
last_updated: "2026-09-10T11:44:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0286: Make validate-plan and research-issue VCS-agnostic via an accelerator vcs diff subcommand

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

`skills/planning/validate-plan` and `skills/research/research-issue` gather
implementation evidence by running raw `git log` and `git diff` in a Bash
block. Both are denied by the `vcs guard` in pure-jj repos, so both skills are
broken there. Repoint them onto VCS-agnostic subcommands — reusing the
`vcs log` renderer 0198 built, and adding the `vcs diff` subcommand that does
not yet exist — so they work identically under git, colocated jj, and pure-jj.

## Context

Spawned by spike 0200, which decided to keep `log` and `diff` in the guard's
`BLOCKED_SUBCOMMANDS` and established that the blocklist is not what breaks
these skills: in a pure-jj repo (`--no-colocate`) there is no `.git`, so raw
`git log`/`git diff` fail with `fatal: not a git repository` regardless of the
guard. Dropping them from the blocklist would replace the guard's helpful
redirect with that cryptic failure — a worse outcome — so the fix is to stop
issuing raw git from the skills at all.

The precedent exists. `skills/vcs/commit/SKILL.md` already injects
`accelerator vcs status` and `accelerator vcs log` via the `!` preprocessor;
these dispatch to the library-backed adapter (`cli/vcs-cli`), render correctly
for git or jj, and never issue a raw `git` Bash call the guard would see. 0198
built `vcs status`/`vcs log` to a VCS-agnostic shape (a flat recent-commit list
for `log`; a backend-neutral change summary for `status`). No `vcs diff`
subcommand exists yet — that is the net-new piece.

## Requirements

- Add an `accelerator vcs diff` subcommand rendering a VCS-agnostic working-copy
  diff, mirroring how `vcs status`/`vcs log` shell the real backend under a
  scrubbed environment (per 0198). Decide the base of comparison explicitly —
  git's index-relative default and jj's parent-relative default diverge — and
  render one consistent meaning across backends.
- Repoint `skills/planning/validate-plan` off raw `git log`/`git diff` onto the
  VCS-agnostic path, and update its `allowed-tools` accordingly.
- Repoint `skills/research/research-issue` off raw `git log`/`git diff`
  likewise.
- Verify the skills work under all three repo modes (git-only, colocated jj,
  pure-jj), with the pure-jj case being the one that is broken today.

## Acceptance Criteria

- [ ] An `accelerator vcs diff` subcommand exists and renders a VCS-agnostic
      working-copy diff under git, colocated jj, and pure-jj, with a stated and
      consistent base of comparison.
- [ ] `validate-plan` and `research-issue` issue no raw `git log`/`git diff`
      Bash calls; they use the VCS-agnostic path and their `allowed-tools`
      reflect it.
- [ ] Both skills gather implementation evidence successfully in a pure-jj repo.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- Blocked by: work-item:0200 (the spike that decided keep-both and spawned this
  as the real remedy). Back-linked from 0200's Dependencies as a Blocks entry.
- Relates to: work-item:0198 (built the VCS-agnostic `vcs status`/`vcs log`
  renderer this extends), work-item:0169 (built the guard and the blocklist).
- Parent: epic 0136.

## References

- `meta/work/0200-decide-vcs-guard-log-diff-blocklist-membership.md` — the
  spike outcome that spawned this item
- `meta/work/0198-vcs-agnostic-status-log-renderer.md` — the VCS-agnostic
  renderer precedent
- `skills/planning/validate-plan/SKILL.md` — raw `git log`/`git diff` at
  lines 58-59
- `skills/research/research-issue/SKILL.md` — raw `git log`/`git diff` at
  lines 63, 66
- `skills/vcs/commit/SKILL.md` — the `!`-preprocessor `vcs status`/`vcs log`
  pattern to follow
- `cli/vcs/src/guard.rs`, `cli/vcs-cli/src/guard.rs` — the guard this routes
  around
