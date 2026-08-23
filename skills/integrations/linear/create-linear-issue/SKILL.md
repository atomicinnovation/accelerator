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
argument-hint: "<work-item-file> [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash
  - Read
  - Write
---

# Create Linear Issue

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill create-linear-issue --fail-safe`

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

## Step 2: Render the preview

Read the work-item file. Show, under this heading:

> **Proposed Linear write — review before sending**

the operation (`issueCreate`), the configured team, the frontmatter `title`, and
the Markdown body (truncate to the first 500 characters for display if longer;
the full body is still sent). State plainly that on success this skill will
**set** the file's `external_id` to the new identifier.

A file that already carries a non-empty `external_id` (`E_CREATE_ALREADY_SYNCED`,
exit `102`) or has missing/unclosed frontmatter (`E_CREATE_BAD_FRONTMATTER`,
exit `101`) is refused at send time with no API call — if you can already see an
`external_id` in the file, stop now and tell the user it is already synced.

## Step 3: Confirm before writing

Ask:

> Create this issue in Linear and set the work item's `external_id`? Reply
> **y** to confirm, **n** to revise, anything else to abort.

On a clear yes, proceed. On no/revise, ask what to change and rebuild the
preview. On anything ambiguous, abort with "Aborted — no Linear write was made."

## Step 4: Send the request

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear create <work-item-file>
```

The subcommand prints its outcome as a trailing `<keyword>\t<identifier>` line.

## Step 5: Gate the writeback on the keyword

Branch on the keyword — the writeback is **fail-closed**, so a non-`created`
outcome never writes `external_id` and never invites a blind re-run:

- **`created`** — the issue exists remotely with `<identifier>`. Set
  `external_id: <identifier>` in the work-item file's frontmatter (insert the
  line if absent), then report: *"Issue created: **\<identifier\>** — the work
  item's `external_id` is now `\<identifier\>`."*
- **`writeback-failed`** (exit `107`) — the issue **was** created remotely but
  its identifier is unusable. Do **not** write `external_id`. Surface this
  loudly: give the user the identifier, tell them NOT to re-run (it would create
  a duplicate), and that they should reconcile `external_id` by hand.
- **any other non-zero exit** — no issue was created. Do **not** write
  `external_id`; report the error. The file is unchanged, so it is safe to fix
  and retry.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions create-linear-issue --fail-safe`
