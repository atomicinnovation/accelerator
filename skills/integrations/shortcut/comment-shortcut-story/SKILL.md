---
name: comment-shortcut-story
description: >
  Use this skill only when the user explicitly invokes /comment-shortcut-story
  to add a Markdown comment to a Shortcut story or epic. This is a write skill,
  so it must never be invoked from conversational context. It shows the
  comment, asks for confirmation, then posts it.
argument-hint: "<sc-ID | epic-ID | URL> (--body TEXT | --body-file PATH) [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash(accelerator config *)
  - Bash(accelerator shortcut *)
---

# Comment on a Shortcut Story

!`accelerator config context --skill comment-shortcut-story --fail-safe`

> **Configuration**: Set `work.integration: shortcut` in
> `.accelerator/config.md`, and run `/accelerator:init-shortcut` once per
> repository.

Add one comment to a story or an epic.

## Step 1: Parse arguments

Read the key (`sc-<id>`, `epic-<id>`, or a Shortcut URL) and the comment:
`--body TEXT`, or `--body-file PATH` for a longer one. If neither is given,
ask the user what to say.

## Step 2: Render the preview

Under the heading

> **Proposed Shortcut write — review before sending**

show the key and the comment text as it will be posted.

## Step 3: Confirm before writing

Ask:

> Post this comment? Reply **y** to confirm, **n** to revise, anything else to
> abort.

On a clear yes, go on. On anything else, stop with "Aborted — no Shortcut
write was made."

## Step 4: Send the request

```
accelerator shortcut comment add <KEY> --body "<TEXT>"
```

Run the bare launcher **directly** as an executable. Never prefix it with
`bash`, `sh` or `env`.

On the `added` outcome, confirm the comment was posted. When stdout carries no
JSON, show the `E_*` line from stderr. A failed comment is never sent twice by
the binary, so before retrying, check with `/show-shortcut-story <KEY>
--comments 3` that it did not land.

!`accelerator config instructions comment-shortcut-story --fail-safe`
