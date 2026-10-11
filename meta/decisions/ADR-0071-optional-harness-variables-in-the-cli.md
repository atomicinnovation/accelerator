---
type: "adr"
id: "ADR-0071"
title: "Optional Harness Variables in the CLI"
date: "2026-10-11T14:00:00+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "proposed"
parent: "work-item:0303"
tags: ["cli", "harness", "coupling", "research", "concurrency"]
last_updated: "2026-10-11T14:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0071: Optional Harness Variables in the CLI

**Date**: 2026-10-11
**Status**: Proposed
**Author**: Toby Clemson

## Context

The `lint:claude-coupling:check` guard fails on any `CLAUDE_*` text under
`cli/` or in `bin/accelerator`, with no allowlist. It was written after a
launcher-layer read of `CLAUDE_PLUGIN_ROOT` broke every CLI-invoking skill:
Claude Code substitutes that variable into skill content but never exports it
to the Bash tool, so a reader that required it failed wherever the CLI ran.
The guard's invariant is that no entry point may require a `CLAUDE_*`
variable.

`research topic outstanding` must clamp `conduct`'s concurrency to the number
of subagents Claude Code starts at once. Claude Code reads that cap from
`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, defaulting to 20. The variable reaches
the Bash tool when it is exported in the shell that launches Claude Code.

Passing the cap from the skill does not work. Claude Code 2.1.296 refuses to
auto-allow a Bash command that expands a shell variable, whatever the skill
grants, so `--subagent-cap "$CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"` prompts on
every plan. The skill permission lint bans shell metacharacters in a `!`
command, and a `!` read of an unset variable fails, so the cap cannot be
injected at invocation time either.

## Decision Drivers

- No `conduct` plan may raise a permission prompt.
- No CLI entry point may require a variable the harness does not guarantee.
- The guard must stay a textual rule that a reviewer can audit, with no file
  exemption that could hide the next `CLAUDE_PLUGIN_ROOT`.
- The CLI should not be tied to one agent harness.

## Considered Options

1. **Flag transport** — the skill expands the variable into a CLI flag.
2. **File exemption** — the guard skips the module that reads the cap.
3. **Named admission** — the guard admits one optional variable by name, in
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

Flag transport is rejected because the permission checker blocks it. A file
exemption is rejected because it admits every name in the file, which is the
failure the guard exists to prevent.

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

### Neutral

- Another agent's cap variable needs the same admission, by name and file,
  once that agent documents one.
- A settings-file `env` entry reaches the CLI only if Claude Code exports it
  to the Bash tool.

## References

- `tasks/lint/claude_coupling.py` — the guard and its admission set
- `cli/research-cli/src/subagent_cap.rs` — the cap reader
- `meta/work/0303-research-concurrency-exceeds-subagent-limit.md`
- `meta/plans/2026-10-10-0303-research-concurrency-subagent-limit.md`
