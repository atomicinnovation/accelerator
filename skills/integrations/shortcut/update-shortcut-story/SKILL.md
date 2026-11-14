---
name: update-shortcut-story
description: >
  Use this skill only when the user explicitly invokes /update-shortcut-story
  to change fields of a Shortcut story or epic: its title, description, state,
  owner or estimate. This is a write skill, so it must never be invoked from
  conversational context. It shows a preview of the fields it will change, asks
  for confirmation, then sends only those fields.
argument-hint: "<sc-ID | epic-ID | URL> [--title T] [--description D] [--state NAME] [--owner MENTION] [--estimate N] [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash(accelerator config *)
  - Bash(accelerator shortcut *)
---

# Update Shortcut Story

!`accelerator config context --skill update-shortcut-story --fail-safe`

> **Configuration**: Set `work.integration: shortcut` in
> `.accelerator/config.md`, and run `/accelerator:init-shortcut` once per
> repository.

Change named fields of one Shortcut story or epic. Fields the user does not
name are left as they are in Shortcut.

## Step 1: Parse arguments

Read the key (`sc-<id>`, `epic-<id>`, or a Shortcut URL) and the fields:

- `--title T` and `--description D` work on stories and epics.
- `--owner MENTION` takes a member's mention name, without the `@`. Shortcut
  calls the person on a story its owner, so there is no `--assignee`.
- `--state NAME` takes a state of the story's own workflow. Stories only.
- `--estimate N` takes the story's points. Shortcut keeps priority in a custom
  field, so there is no `--priority`. Stories only.

If no field is named, ask the user what to change.

## Step 2: Render the preview

Under the heading

> **Proposed Shortcut write — review before sending**

show the key and each field with its new value. Say that every other field
stays as it is.

## Step 3: Confirm before writing

Ask:

> Update this in Shortcut? Reply **y** to confirm, **n** to revise, anything
> else to abort.

On a clear yes, go on. On anything else, stop with "Aborted — no Shortcut
write was made."

## Step 4: Send the request

```
accelerator shortcut update <KEY> [fields]
```

Run the bare launcher **directly** as an executable. Never prefix it with
`bash`, `sh` or `env`.

On the `updated` outcome, confirm the change and suggest
`/show-shortcut-story <KEY>` to check it. When stdout carries no JSON, show
the `E_*` line from stderr:

- `E_STATE_UNKNOWN` and `E_STATE_AMBIGUOUS` list the workflow's states; ask the
  user to pick one.
- `E_STATE_NO_WORKFLOW` means the catalogue lacks the story's workflow; run
  `/accelerator:init-shortcut` to refresh it.
- `E_OWNER_UNKNOWN` lists the members' mention names.
- `E_NOT_A_STORY` means a state or estimate was given for an epic.

Each of these is refused before anything is sent.

!`accelerator config instructions update-shortcut-story --fail-safe`
