---
type: "adr"
id: "ADR-0071"
title: "Optional Harness Variables in the CLI"
date: "2026-10-11T14:00:00+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
parent: "work-item:0303"
tags: ["cli", "harness", "coupling", "research", "concurrency"]
last_updated: "2026-10-13T09:40:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0071: Optional Harness Variables in the CLI

**Date**: 2026-10-11
**Status**: Accepted
**Author**: Toby Clemson

## Context

The `lint:claude-coupling:check` guard fails on any `CLAUDE_*` text under
`cli/`, in `bin/accelerator`, or in the four development scripts that feed
them, with no allowlist. It was written after a
launcher-layer read of `CLAUDE_PLUGIN_ROOT` broke every CLI-invoking skill:
Claude Code substitutes that variable into skill content but never exports it
to the Bash tool, so a reader that required it failed wherever the CLI ran.
The guard's invariant is that no entry point may require a `CLAUDE_*`
variable.

`research topic outstanding` must clamp `conduct`'s concurrency to the number
of subagents Claude Code starts at once. Claude Code reads that cap from
`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, defaulting to 20, and enforces it from
v2.1.217; earlier versions and ultracode sessions enforce no cap. The variable
reaches the Bash tool when it is exported in the shell that launches Claude
Code, or set under `env` in a Claude Code settings file.

Passing the cap from the skill does not work. Claude Code 2.1.296 refuses to
auto-allow a Bash command that expands a shell variable, whatever the skill
grants, so `--subagent-cap "$CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"` prompts on
every plan. A `!` read of the variable fails when it is unset, which
discards the whole prompt, so the cap cannot be injected at invocation time
either.

## Decision Drivers

- No `conduct` plan may raise a permission prompt.
- No CLI entry point may require a variable the harness does not guarantee.
- The guard must stay a textual rule that a reviewer can audit, with no file
  exemption that could hide the next `CLAUDE_PLUGIN_ROOT`.
- The CLI should not be tied to one agent harness.

## Considered Options

1. **Flag transport** — the skill expands the variable into a CLI flag.
2. **Accelerator-only variable** — the CLI reads only
   `ACCELERATOR_SUBAGENT_CAP`, defaulting to 20, and never names the
   harness's variable.
3. **File exemption** — the guard skips the module that reads the cap.
4. **Named admission** — the guard admits one optional variable by name, in
   the files that read and test it.

## Decision

We will let the CLI read an optional harness variable that has a stated
default, admitted to the coupling guard by name and confined to its reader
and that reader's test. Any other `CLAUDE_*` name in those files is still
flagged. An `ACCELERATOR_` variable for the same setting takes precedence, so
the agent-neutral setting always wins over a harness's own.

The first admission is `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, read in
`cli/research-cli/src/subagent_cap.rs` after `ACCELERATOR_SUBAGENT_CAP`, each
read as Claude Code reads its own setting, with 20 when neither holds a value
Claude Code would honour.

Flag transport is rejected because the permission checker blocks it. An
Accelerator-only variable is rejected because it drifts from the harness: a
user who lowers Claude Code's cap without also setting ours gets 20 spawns
offered against fewer slots, and the surplus is refused in every batch. A
file exemption is rejected because it admits every name in the file, which is
the failure the guard exists to prevent.

## Consequences

### Positive

- `conduct` plans carry no shell variable, so they run without a prompt.
- The CLI works with the variable absent, so the guard's invariant holds.
- A hand run of `outstanding` in the same shell sees the cap `conduct` sees.

### Negative

- The guard is no longer absolute: its admission set is a reviewed exception
  list, pinned by a test, that must not grow by convenience.
- Tests that run the binary must clear every admitted variable, or a cap set
  in the developer's environment leaks into them.
- The default of 20 and the parsing rules mirror Claude Code 2.1.296, which
  publishes neither as a contract. A harness upgrade needs both rechecked, or
  the CLI may offer more spawns than the harness admits.

### Neutral

- Another agent's cap variable needs the same admission, by name and file,
  once that agent documents one.
- A settings-file `env` entry reaches the CLI because Claude Code exports it
  to the Bash tool, which was verified on Claude Code 2.1.296 for the
  project, local and user settings files.

## References

- `tasks/lint/claude_coupling.py` — the guard and its admission set
- `cli/research-cli/src/subagent_cap.rs` — the cap reader
- `meta/work/0303-research-concurrency-exceeds-subagent-limit.md`
- `meta/work/0182-cli-derives-plugin-root-from-own-location.md` — the work
  that introduced the guard
- `meta/research/issues/2026-07-26-cli-requires-claude-plugin-root-env-var.md`
  — the breakage the guard prevents
- `meta/plans/2026-10-10-0303-research-concurrency-subagent-limit.md`
