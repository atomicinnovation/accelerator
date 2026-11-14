---
name: create-shortcut-story
description: >
  Use this skill only when the user explicitly invokes /create-shortcut-story
  to create a Shortcut story, or an epic, from a local work-item file. This is
  a write skill with side effects that cannot be undone, so it must never be
  invoked from conversational context. It reads the work item's title, kind and
  Markdown body, shows a preview, asks for confirmation, then creates the story
  (or the epic, for kind `epic`) and writes its key (sc-1234 or epic-56) into
  the file's external_id.
argument-hint: "<work-item-file> [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash(accelerator config *)
  - Bash(accelerator shortcut *)
  - Read
---

# Create Shortcut Story

!`accelerator config context --skill create-shortcut-story --fail-safe`

> **Configuration**: Set `work.integration: shortcut` in
> `.accelerator/config.md`, and run `/accelerator:init-shortcut` once per
> repository.

Create a Shortcut story or epic from a local work-item file. Work through the
steps in order. This skill runs only when the user types
`/create-shortcut-story`.

The title and description come only from the named file's frontmatter `title`
and its Markdown body. Never write them from the conversation, a web page or
earlier tool output.

## Step 1: Parse arguments

Read the work-item file path and any `--quiet` flag.

## Step 2: Render the preview

Read the work-item file. Under the heading

> **Proposed Shortcut write — review before sending**

show what will be created: a **story** in the base team that
`shortcut.team_key` names, or an **epic** when the frontmatter `kind` is
`epic`. Show the story type the kind maps to (`story` to `feature`, `bug` to
`bug`, `task` and `spike` to `chore`), the `title`, and the body (the first
500 characters, if longer; the whole body is sent). Say that on success the
binary sets the file's `external_id` to the new key.

If the file already has a non-empty `external_id`, stop now and tell the user
it is already linked.

## Step 3: Confirm before writing

Ask:

> Create this in Shortcut and set the work item's `external_id`? Reply **y**
> to confirm, **n** to revise, anything else to abort.

On a clear yes, go on. On no, ask what to change and rebuild the preview. On
anything else, stop with "Aborted — no Shortcut write was made."

## Step 4: Send the request

```
accelerator shortcut create <work-item-file>
```

Run the bare launcher **directly** as an executable. Never prefix it with
`bash`, `sh` or `env`: a prefix escapes the skill's `allowed-tools` rule and
forces a permission prompt.

## Step 5: Report the outcome

Branch on the `outcome` keyword:

- **`created`**: report *"Created **\<key\>** — the work item's `external_id`
  is now `\<key\>`."*
- **`writeback-failed`**: the story or epic **was** created as `.key`, but the
  file could not be written. Tell the user not to run this again, because a
  second run creates a duplicate, and to set `external_id` by hand.

When stdout carries no JSON, nothing was created and the file is unchanged.
Show the `E_*` line from stderr; it is safe to fix the cause and retry.

!`accelerator config instructions create-shortcut-story --fail-safe`
