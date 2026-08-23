---
name: update-linear-issue
description: >
  Use this skill only when the user explicitly invokes /update-linear-issue to
  change fields on an existing Linear issue (title, description, state,
  assignee, priority). This is a write skill with irreversible side effects — it
  must never be auto-invoked from conversational context. It shows a payload
  preview, requires explicit confirmation, then applies the update.
argument-hint: "<IDENTIFIER> [--title TEXT] [--description TEXT] [--state NAME] [--assignee-id ID] [--priority N] [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash
  - Read
  - Write
---

# Update Linear Issue

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill update-linear-issue --fail-safe`

> **Configuration**: Set `work.integration: linear` in `.accelerator/config.md`.

Update an existing Linear issue via `issueUpdate`. Work through the steps in
order. This skill never auto-invokes — it only runs when the user explicitly
types `/update-linear-issue`. Body/description content comes only from the
user's current turn, never synthesised from prior context.

## Step 1: Parse arguments

Read the issue identifier (positional) and the mutating flags: `--title`,
`--description`, `--state` (a WorkflowState name resolved to its UUID via the
catalogue), `--assignee-id`, `--priority`. At least one is required.

## Step 2: Render the preview

Show the resolved intent — the operation (`issueUpdate`), the target issue, and
the fields being set (`--title`/`--description`/`--state`/`--assignee-id`/
`--priority`) — under:

> **Proposed Linear write — review before sending**

At least one mutating flag is required; with none the subcommand refuses before
any write (`E_UPDATE_NO_OPS`, exit `111`). A `--state` change resolves through
the catalogue and is refused before any write if the name is unknown or
ambiguous.

## Step 3: Confirm before writing

Ask:

> Apply this update to Linear? Reply **y** to confirm, **n** to revise, anything
> else to abort.

On a clear yes, proceed. On no/revise, rebuild the preview. On anything
ambiguous, abort with "Aborted — no Linear write was made."

## Step 4: Send and render

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear update <IDENTIFIER> [flags]
```

The subcommand reports the `updated` keyword on success (the trailing
`updated\t<identifier>` line, or the `outcome` field when a `--state` change
renders JSON). Confirm the updated fields and suggest `/show-linear-issue
<IDENTIFIER>` to verify. On any non-zero exit, report the error — no write was
made.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions update-linear-issue --fail-safe`
