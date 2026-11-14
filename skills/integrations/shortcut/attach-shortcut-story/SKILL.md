---
name: attach-shortcut-story
description: >
  Use this skill only when the user explicitly invokes /attach-shortcut-story
  to attach an https link to a Shortcut story. This is a write skill, so it
  must never be invoked from conversational context. It shows the link, asks
  for confirmation, then adds it to the story's external links. Files cannot
  be uploaded yet.
argument-hint: "<sc-ID | URL> --url <https-URL> [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash(accelerator config *)
  - Bash(accelerator shortcut *)
---

# Attach a Link to a Shortcut Story

!`accelerator config context --skill attach-shortcut-story --fail-safe`

> **Configuration**: Set `work.integration: shortcut` in
> `.accelerator/config.md`, and run `/accelerator:init-shortcut` once per
> repository.

Add one https link to a story's external links. Shortcut stores a link as its
URL alone, so there is no title to give it. Epics take no links, so this skill
takes stories only.

## Step 1: Parse arguments

Read the story key (`sc-<id>` or a story URL) and `--url`. If the user wants
to attach a file, tell them upload is not supported yet, and that they can
upload it elsewhere and attach its https URL.

## Step 2: Render the preview

Under the heading

> **Proposed Shortcut write — review before sending**

show the story key and the URL.

## Step 3: Confirm before writing

Ask:

> Attach this link? Reply **y** to confirm, **n** to revise, anything else to
> abort.

On a clear yes, go on. On anything else, stop with "Aborted — no Shortcut
write was made."

## Step 4: Send the request

```
accelerator shortcut attach <KEY> --url <URL>
```

Run the bare launcher **directly** as an executable. Never prefix it with
`bash`, `sh` or `env`.

On the `attached` outcome, confirm the link was added. When stdout carries no
JSON, show the `E_*` line from stderr. `E_ATTACH_BAD_URL` means the URL is not
https, and `E_NOT_A_STORY` means the key names an epic; nothing is sent for
either.

!`accelerator config instructions attach-shortcut-story --fail-safe`
