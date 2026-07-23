---
name: transition-linear-issue
description: >
  Use this skill only when the user explicitly invokes /transition-linear-issue
  to move an existing Linear issue to a different workflow state. This is a write
  skill with irreversible side effects — it must never be auto-invoked from
  conversational context. The target state name is resolved to its UUID from the
  cached catalogue (no live lookup). Shows a preview, requires explicit
  confirmation, then applies the transition.
argument-hint: "<IDENTIFIER> <STATE-NAME> [--describe] [--quiet]"
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

## Step 2: Preview (resolve from the catalogue)

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/linear-transition-flow.sh \
  <IDENTIFIER> "<STATE-NAME>" --describe
```

State matching is case-insensitive and trimmed. If the name is not in the
catalogue (`E_TRANSITION_STATE_NOT_IN_CATALOGUE`) or is shared by two states
(`E_TRANSITION_STATE_AMBIGUOUS`), STOP and report — suggest `/init-linear` to
refresh, or ask the user to pick an unambiguous state.

## Step 3: Render the preview and confirm

Show the resolved `stateId` and target state under:

> **Proposed Linear write — review before sending**

Ask:

> Transition this issue? Reply **y** to confirm, **n** to revise, anything else
> to abort.

On a clear yes, proceed. On anything ambiguous, abort with "Aborted — no Linear
write was made."

## Step 4: Send and render

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/linear-transition-flow.sh \
  <IDENTIFIER> "<STATE-NAME>"
```

Confirm the new state. Suggest `/show-linear-issue <IDENTIFIER>` to verify.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions transition-linear-issue --fail-safe`
