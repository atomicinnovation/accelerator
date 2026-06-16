---
name: create-linear-issue
description: >
  Use this skill only when the user explicitly invokes /create-linear-issue to
  create a new Linear issue from a local work-item file. This is a write skill
  with irreversible side effects — it must never be auto-invoked from
  conversational context. It reads the work item's title and Markdown body,
  shows a payload preview, requires explicit confirmation, then creates the
  issue and writes the remote-allocated identifier (e.g. BLA-123) back into the
  file's external_id frontmatter field.
argument-hint: "<work-item-file> [--print-payload] [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash
  - Read
  - Write
---

# Create Linear Issue

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh create-linear-issue`

> **Configuration**: Set `work.integration: linear` in `.accelerator/config.md`.
> See the
> [`### work` section of `configure/SKILL.md`](../../config/configure/SKILL.md#work)
> for the full reference.

Create a Linear issue from a local work-item file via `issueCreate`, then write
the allocated identifier back into the file. Work through the steps below in
order. This skill never auto-invokes — it only runs when the user explicitly
types `/create-linear-issue`.

The issue title and description come ONLY from the named work-item file's
frontmatter `title` and its Markdown body. This skill never synthesises issue
content from upstream conversation, web fetches, or prior tool output.

## Step 1: Parse arguments

Read the work-item file path (positional) and any flags (`--quiet`).

## Step 2: Generate the payload preview

Invoke the helper with `--print-payload` to produce the preview without an API
call:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/linear-create-flow.sh \
  <work-item-file> --print-payload
```

If the helper exits non-zero (e.g. the file is already synced —
`E_CREATE_ALREADY_SYNCED` (it already carries a non-empty `external_id`), or
has missing/unclosed frontmatter — `E_CREATE_BAD_FRONTMATTER`), STOP and report
the error; make no API call.

## Step 3: Render the preview

Show the payload under this heading:

> **Proposed Linear write — review before sending**

Include the operation (`issueCreate`), the resolved `teamId`, the `title`, and
the `description` (truncate to the first 500 characters for display if longer;
the full body is still sent). State plainly that on success the file's
`external_id` will be **set** to the new identifier (the line is inserted if the
file does not already have one).

## Step 4: Confirm before writing

Ask:

> Create this issue in Linear and set the work item's `external_id`? Reply
> **y** to confirm, **n** to revise, anything else to abort.

On a clear yes, proceed. On no/revise, ask what to change and rebuild the
preview. On anything ambiguous, abort with "Aborted — no Linear write was made."

## Step 5: Send the request

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/linear-create-flow.sh \
  <work-item-file>
```

## Step 6: Render the response

On success the helper prints the new identifier. Report:

> Issue created: **\<IDENTIFIER\>** — the work item's `external_id` is now
> `\<IDENTIFIER\>`.

If the helper exits `E_CREATE_WRITEBACK_FAILED` (107), the issue **was** created
remotely but the local file was **not** updated. Surface this loudly: tell the
user the created identifier, that they must NOT blindly re-run (it would create
a duplicate), and that they should set `external_id: <IDENTIFIER>` by hand.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh create-linear-issue`
