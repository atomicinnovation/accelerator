---
type: "adr"
id: "ADR-0073"
title: "Admit the Session Search Cap as an Optional Harness Variable"
date: "2026-10-16T00:06:28+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
parent: "work-item:0304"
relates_to: ["adr:ADR-0071"]
tags: ["cli", "harness", "coupling", "research", "web-profile"]
last_updated: "2026-10-16T15:54:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0073: Admit the Session Search Cap as an Optional Harness Variable

**Date**: 2026-10-16
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0071 lets the CLI read an optional harness variable with a stated
default, admitted to the coupling guard by name and confined to its reader
and that reader's test. Its first and only admission is
`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, behind an `ACCELERATOR_SUBAGENT_CAP`
that takes precedence.

Claude Code caps `WebSearch` calls per interactive terminal session at 200,
shared by the main conversation and every subagent. The cap is read from
`CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`. At the cap Claude Code raises no
error: each further search returns a notice telling the model to continue
with what it has. A `conduct` run that spends the cap leaves later web
researchers writing notes from `WebFetch` or memory. Sizing per-level search
budgets from the cap would prevent that, and needs `research topic
outstanding` to know the cap Claude Code enforces.

The same transport constraints apply as in ADR-0071. The skill cannot pass
the cap as a flag without a permission prompt, and cannot inject it with a
`!` read that fails when the variable is unset.

Claude Code 2.1.301 declares both caps with the same getter:
`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS:()=>ci` and
`CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION:()=>Ri`, where
`ci=M.int({min:1,digitsOnly:!0})` and `Ri=M.int({min:1,digitsOnly:!0})`.
Each is read as `a.VAR ?? default`, with defaults 20 and 200. So one parser
serves both.

## Decision Drivers

- Budgets must be sized against the cap Claude Code enforces, or a run
  either halts early or leaves searches unused.
- No `conduct` plan may raise a permission prompt.
- The coupling guard's admission set must grow only by reviewed decision.
- A budget sized against a value Claude Code silently ignored misleads the
  user about how far a run will go.

## Considered Options

1. **Follow ADR-0071 exactly** — admit the variable behind an
   `ACCELERATOR_*` counterpart that takes precedence, and fall back
   silently when the harness value is malformed.
2. **Harness variable alone, unhonoured value reported** — admit only
   `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`, and warn on `--start` when it
   holds a value Claude Code would not honour.
3. **Fixed cap** — size against 200 always, never reading the variable.

## Decision

We will take option 2. We will admit
`CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` as an optional
harness variable, read in `cli/research-cli/src/search_cap.rs` and tested in
`cli/research-cli/tests/topic_outstanding.rs`. It is read as Claude Code
reads it, with 200 when it holds no value Claude Code would honour. The
parse is shared with the subagent cap.

This extends ADR-0071 and does not supersede it. It departs from ADR-0071's
pattern in two ways:

- **No `ACCELERATOR_*` counterpart.** An override set above the harness's
  cap costs far more here than for the subagent cap. Spawns refused for
  capacity are retried in rounds, but searches budgeted beyond the quota
  exhaust it and halt the run until `/clear`. Claude Code also documents
  the limit only for interactive terminal sessions, so an opt-out is
  deferred until `conduct` runs off-terminal.
- **An unhonoured harness value is reported.** When the variable is set to a
  value Claude Code would not honour, `0` included, `outstanding --start`
  warns, naming the value and the default it sized against, but only when
  the run has web nodes left to research.

A fixed cap is rejected because a user who raises Claude Code's cap would
get budgets sized for 200, and a user who lowers it would halt early.

## Consequences

### Positive

- Budgets follow the cap Claude Code enforces, in the same shell, with no
  prompt.
- A malformed setting is visible before the run spends searches.
- Both harness caps share one parser, so a parity fix lands once.

### Negative

- The admission set grows to two names, each pinned by a test.
- Tests that run the binary must clear this variable too.
- The default of 200 and the parsing rules mirror Claude Code 2.1.301,
  which publishes neither as a contract. A harness upgrade needs both
  rechecked.

### Neutral

- Outside an interactive terminal Claude Code may enforce no cap, so budgets
  there are sized conservatively. That is accepted.
- An `ACCELERATOR_*` override can be added later under ADR-0071's
  precedence rule without changing this admission.

## References

- `meta/decisions/ADR-0071-optional-harness-variables-in-the-cli.md`
- `tasks/lint/claude_coupling.py` — the guard and its admission set
- `cli/research-cli/src/search_cap.rs` — the search cap reader
- `cli/research-cli/src/harness_setting.rs` — the shared parse
- `meta/work/0304-fail-web-nodes-on-websearch-quota-exhaustion.md`
- `meta/plans/2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion.md`
