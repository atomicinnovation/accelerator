---
name: transition-shortcut-story
description: >
  Use this skill only when the user explicitly invokes
  /transition-shortcut-story to move a Shortcut story to another state of its
  workflow. This is a write skill, so it must never be invoked from
  conversational context. The state name resolves against the story's own
  workflow in the committed catalogue. It shows a preview, asks for
  confirmation, then moves the story.
argument-hint: "<sc-ID | URL> <STATE-NAME> [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash(accelerator config *)
  - Bash(accelerator shortcut *)
---

# Transition Shortcut Story

!`accelerator config context --skill transition-shortcut-story --fail-safe`

> **Configuration**: Set `work.integration: shortcut` in
> `.accelerator/config.md`, and run `/accelerator:init-shortcut` once per
> repository.

Move one story to a named state. Epics have no workflow state to move, so this
skill takes stories only.

## Step 1: Parse arguments

Read the story key (`sc-<id>` or a story URL) and the target state name, for
example `/transition-shortcut-story sc-123 "In Progress"`.

## Step 2: Render the preview

Under the heading

> **Proposed Shortcut write — review before sending**

show the story key and the target state. The binary reads the story first to
find its workflow, then matches the state name in that workflow, ignoring
case.

## Step 3: Confirm before writing

Ask:

> Move this story? Reply **y** to confirm, **n** to revise, anything else to
> abort.

On a clear yes, go on. On anything else, stop with "Aborted — no Shortcut
write was made."

## Step 4: Send the request

```
accelerator shortcut transition <KEY> "<STATE-NAME>"
```

Run the bare launcher **directly** as an executable. Never prefix it with
`bash`, `sh` or `env`.

On the `transitioned` outcome, confirm the new state and suggest
`/show-shortcut-story <KEY>`. When stdout carries no JSON, show the `E_*` line
from stderr. `E_STATE_UNKNOWN` and `E_STATE_AMBIGUOUS` list the workflow's
states; ask the user to pick one. `E_STATE_NO_WORKFLOW` means the catalogue
lacks the story's workflow; run `/accelerator:init-shortcut` to refresh it.
`E_NOT_A_STORY` means the key names an epic. Nothing is sent for any of these.

!`accelerator config instructions transition-shortcut-story --fail-safe`
