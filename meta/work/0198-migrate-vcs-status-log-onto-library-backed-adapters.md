---
type: work-item
id: "0198"
title: "Migrate vcs status/log off subprocess onto library-backed adapters"
date: "2026-08-06T00:00:00+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: low
parent: "work-item:0136"
relates_to: ["work-item:0169", "work-item:0125"]
derived_from: ["plan:2026-08-05-0169-vcs-subdomain-and-hooks-migration"]
tags: [rust, vcs, cli, performance]
last_updated: "2026-08-06T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-728
---

# 0198: Migrate vcs status/log off subprocess onto library-backed adapters

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

`accelerator-vcs`'s `status` and `log` subcommands (`cli/vcs-adapters/src/subprocess.rs`'s
`status`/`log`, wired through `cli/vcs-cli/src/status.rs`/`log.rs`) are the two
remaining pieces of the `vcs` subdomain that shell out to the real `jj`/`git`
binaries rather than reading through `gix`/`jj-lib` in-process, the way
`detect` and `guard` do. 0169 chose subprocess deliberately, not by omission
— this item is where that choice gets revisited, not a bug report.

## Context

0169's own Key Discoveries recorded why: "`vcs status`/`vcs log` cannot be
produced from the six taxonomy queries — none of them render `jj
status`/`git diff --stat`-shaped text, and reimplementing that formatting
against `gix`/`jj-lib` would be a disproportionate undertaking with no
byte-parity guarantee." The existing `vcs_adapters::subprocess` module
already established shelling `jj`/`git` as a legitimate, first-class adapter
pattern in this codebase (`CommandProbe`, used as the oracle
`vcs_adapters::library`'s in-process queries are tested against), so `status`
and `log` followed the same shape rather than inventing a new one.

Today's implementation (`run_vcs_text` in `subprocess.rs`) runs exactly four
commands — `jj status`, `jj log --limit 5`, `git diff --cached --stat`, `git
log --oneline -5` — under a scrubbed environment and a 10-second cap, falling
back to a literal `(... unavailable)` string on any failure (never itself
fails). This is the same pattern `CommandProbe::revision` already used for
the working-copy revision before 0188's library-backed adapters replaced
that specific query.

**The two commands are not equally hard to replace.** `git diff --cached
--stat`/`git log --oneline -5` are well inside `gix`'s documented surface —
`gix::diff` and a revwalk over `gix::Repository` should reproduce them
directly, matching 0188's own precedent of driving `gix` in-process for
comparable git operations. `jj status`/`jj log`, by contrast, render through
jj's own CLI template engine, which `jj-lib` does not expose as a stable,
reusable library API in the same shape — the CLI's revset/log rendering is
substantially CLI-layer logic, not a thin wrapper over a `jj-lib` call. This
is 0169's "no byte-parity guarantee" concern made concrete: `jj log`
specifically may not be fully reproducible via `jj-lib` alone without
re-implementing template rendering, and that risk should be resolved by
investigation before committing to a full migration.

**Motivation for revisiting, now that the rest of the subdomain has
converged**: no runtime dependency on `jj`/`git` being installed and on
`PATH` for these two subcommands specifically (mirroring 0125's argument for
`detect`/`guard`); the same order-of-magnitude performance difference 0125
measured for other queries (~3.6-4.7 ms cold in-process against ~23.8 ms for
a single subprocess round-trip); and consistency — `status`/`log` would be
the last two `vcs` subcommands still spawning an external process, after
this story fully converged `detect`/`guard`.

## Requirements

- Investigate whether `git diff --cached --stat` and `git log --oneline -5`
  can be reproduced byte-for-byte (or acceptably close, with declared
  departures matching 0169's own convention) via `gix` in-process. If yes,
  implement it.
- Investigate whether `jj status` and `jj log --limit 5` can be reproduced
  via `jj-lib` in-process, given the template-rendering concern above. This
  is the item's real open question — resolve it with evidence (a working
  prototype against real fixtures, or a documented reason it cannot be done
  without unacceptable complexity) before committing to full migration of
  the jj side.
- If full byte-parity is not achievable for `jj status`/`jj log`, consider
  and record a decision on the partial option: migrate the git side to
  `gix`, keep the jj side on subprocess, and document why the two commands
  in one subcommand pair ended up on different implementation strategies.
- Preserve today's contract regardless of outcome: `status`/`log` never
  fail (fall back to an `(... unavailable)` text on any adapter failure,
  matching the shell's original `2>/dev/null || echo` behaviour), and stay
  diagnosable via `ACCELERATOR_LOG` on that fallback path.
- If migrated, drop the now-unneeded parts of `vcs_adapters::subprocess` for
  `status`/`log` specifically; `CommandProbe`'s `revision`/`kind` (the
  library-backed-adapter test oracle) are out of scope and stay as they are.

## Acceptance Criteria

- [ ] The `jj status`/`jj log` feasibility question is answered with
      evidence, not assumed, and the answer is recorded on this item even if
      the outcome is "not worth it."
- [ ] Whatever is migrated matches `cli/vcs-cli/tests/status_log_goldens.rs`'s
      existing fixture set (masked, from `hooks/test-fixtures/masks.toml`),
      or updates it with declared, tested departures in 0169's style.
- [ ] Whatever is not migrated stays on the existing subprocess path with no
      behavioural change, and the reason is recorded here rather than left
      implicit.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- Blocked by: work-item:0169 (done) — built `accelerator-vcs` and the
  subprocess `status`/`log` implementation this item migrates.
- Relates to: work-item:0125 — the performance/no-PATH-dependency argument
  this item borrows for `status`/`log` specifically.
- Parent: epic 0136.

## Assumptions

- A full rewrite of jj's template-rendering engine against `jj-lib` is out
  of scope regardless of outcome — if `jj log`'s exact output cannot be
  reached without that, the answer to the feasibility question is "no,"
  not "yes, with more work."

## References

- `cli/vcs-adapters/src/subprocess.rs` — `status`, `log`, `run_vcs_text`,
  `CommandProbe`
- `cli/vcs-cli/src/status.rs`, `cli/vcs-cli/src/log.rs`
- `cli/vcs-cli/tests/status_log_goldens.rs`
- `hooks/test-fixtures/vcs-status-log/`, `hooks/test-fixtures/masks.toml`
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — Key Discoveries,
  "`vcs status`/`vcs log` cannot be produced from the six taxonomy queries"
- `meta/work/0125-converge-vcs-detection-on-probe-layer.md` — the
  no-PATH-dependency and cold-call performance evidence this item echoes
- `meta/work/0188-library-backed-vcs-adapter.md` — the `gix`/`jj-lib`
  in-process precedent this item would extend
