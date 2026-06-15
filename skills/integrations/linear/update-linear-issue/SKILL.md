---
name: update-linear-issue
description: >
  Use this skill only when the user explicitly invokes /update-linear-issue to
  change fields on an existing Linear issue (title, description, state,
  assignee, priority). This is a write skill with irreversible side effects — it
  must never be auto-invoked from conversational context. It shows a payload
  preview, requires explicit confirmation, then applies the update.
argument-hint: "<IDENTIFIER> [--title TEXT] [--description TEXT] [--state NAME] [--assignee-id ID] [--priority N] [--print-payload] [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash
  - Read
  - Write
---

# Update Linear Issue

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh update-linear-issue`

> **Configuration**: Set `work.integration: linear` in `.accelerator/config.md`.

Update an existing Linear issue via `issueUpdate`. Work through the steps in
order. This skill never auto-invokes — it only runs when the user explicitly
types `/update-linear-issue`. Body/description content comes only from the
user's current turn, never synthesised from prior context.

## Step 1: Parse arguments

Read the issue identifier (positional) and the mutating flags: `--title`,
`--description`, `--state` (a WorkflowState name resolved to its UUID via the
catalogue), `--assignee-id`, `--priority`. At least one is required.

## Step 2: Generate the payload preview

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/linear-update-flow.sh \
  <IDENTIFIER> [flags] --print-payload
```

If the helper exits non-zero (e.g. unknown state — `E_UPDATE_BAD_STATE`, or no
mutating flags — `E_UPDATE_NO_OPS`), STOP and report the error; make no API
call.

## Step 3: Render the preview

Show the operation (`issueUpdate`), the target `id`, and the assembled `input`
under:

> **Proposed Linear write — review before sending**

## Step 4: Confirm before writing

Ask:

> Apply this update to Linear? Reply **y** to confirm, **n** to revise, anything
> else to abort.

On a clear yes, proceed. On no/revise, rebuild the preview. On anything
ambiguous, abort with "Aborted — no Linear write was made."

## Step 5: Send and render

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/linear-update-flow.sh \
  <IDENTIFIER> [flags]
```

Confirm the updated fields. Suggest `/show-linear-issue <IDENTIFIER>` to verify.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh update-linear-issue`
