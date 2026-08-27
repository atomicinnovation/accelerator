---
name: transition-linear-issue
description: >
  Use this skill only when the user explicitly invokes /transition-linear-issue
  to move an existing Linear issue to a different workflow state. This is a write
  skill with irreversible side effects — it must never be auto-invoked from
  conversational context. The target state name is resolved to its UUID from the
  cached catalogue (no live lookup). Shows a preview, requires explicit
  confirmation, then applies the transition.
argument-hint: "<IDENTIFIER> <STATE-NAME> [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash
  - Read
  - Write
---

# Transition a Linear Issue

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill transition-linear-issue --fail-safe`

> **Configuration**: Set `work.integration: linear` in `.accelerator/config.md`.

Move a Linear issue to a target WorkflowState via `issueUpdate`, resolving the
state name to its team-scoped UUID **from the cached catalogue** — there is no
live lookup. Work through the steps in order. This skill never auto-invokes.

## Step 1: Parse arguments

Read the issue identifier and the target state name (both positional, e.g.
`/transition-linear-issue BLA-123 "In Progress"`).

## Step 2: Render the preview

Show the resolved intent — the operation (`issueUpdate` state change), the target
issue, and the target state name — under:

> **Proposed Linear write — review before sending**

State matching is case-insensitive and trimmed. Resolution happens inside the
subcommand from the cached catalogue; a name that is not in the catalogue or is
shared by two states is refused at send time **before any write**
(`E_TRANSITION_STATE_NOT_IN_CATALOGUE`, `E_TRANSITION_STATE_AMBIGUOUS`).

## Step 3: Confirm before writing

Ask:

> Transition this issue? Reply **y** to confirm, **n** to revise, anything else
> to abort.

On a clear yes, proceed. On anything ambiguous, abort with "Aborted — no Linear
write was made."

## Step 4: Send and render

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear transition <IDENTIFIER> "<STATE-NAME>"
```

The subcommand emits a JSON envelope with a top-level `outcome` keyword. On
`transitioned`, confirm the new state and suggest `/show-linear-issue
<IDENTIFIER>` to verify. On a non-zero exit naming
`E_TRANSITION_STATE_NOT_IN_CATALOGUE` or `E_TRANSITION_STATE_AMBIGUOUS`, the
state could not be resolved and no write was made — suggest `/init-linear` to
refresh, or ask the user to pick an unambiguous state.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions transition-linear-issue --fail-safe`
