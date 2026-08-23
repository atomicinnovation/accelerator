---
name: comment-linear-issue
description: >
  Use this skill only when the user explicitly invokes /comment-linear-issue to
  add a Markdown comment to an existing Linear issue. This is a write skill with
  irreversible side effects — it must never be auto-invoked from conversational
  context. It shows the comment preview, requires explicit confirmation, then
  posts the comment.
argument-hint: "<IDENTIFIER> --body TEXT | --body-file PATH [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash
  - Read
  - Write
---

# Comment on a Linear Issue

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill comment-linear-issue --fail-safe`

> **Configuration**: Set `work.integration: linear` in `.accelerator/config.md`.

Add a Markdown comment to a Linear issue via `commentCreate`. Work through the
steps in order. This skill never auto-invokes — it only runs when the user
explicitly types `/comment-linear-issue`.

The comment body comes ONLY from text the user typed this turn or a file path
the user named this turn — never synthesised from prior context.

## Step 1: Parse arguments

Read the issue identifier (positional) and the body source (`--body TEXT` or
`--body-file PATH`).

## Step 2: Render the preview

Resolve the body: if `--body-file PATH` was given, read the file; otherwise take
`--body TEXT` verbatim. If neither is present, STOP with `E_COMMENT_NO_BODY`.

Show the resolved intent — the operation (`commentCreate`), the target issue,
and the Markdown body — under:

> **Proposed Linear write — review before sending**

## Step 3: Confirm before writing

Ask:

> Post this comment to Linear? Reply **y** to confirm, **n** to revise, anything
> else to abort.

On a clear yes, proceed. On no/revise, rebuild the preview. On anything
ambiguous, abort with "Aborted — no Linear write was made."

## Step 4: Send and render

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear comment add <IDENTIFIER> --body "..."
```

The subcommand emits a JSON envelope with a top-level `outcome` keyword. On
`added`, confirm the comment was posted. On any non-zero exit, report the error
and stop.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions comment-linear-issue --fail-safe`
