---
name: show-linear-issue
description: >
  Use this skill when the user asks about a specific Linear issue by identifier
  (e.g. BLA-123, ENG-456) — for viewing the description, state, assignee, or
  comments. Trigger when the user says 'look up', 'check on', 'tell me about',
  'what's on', or 'what is the status of' an identifier, or asks any direct
  question about an issue they reference. Do NOT trigger when an identifier
  appears incidentally inside other prose (commit messages, code review
  comments, release notes), where the user is talking about the issue rather
  than asking to fetch it.
argument-hint: "<IDENTIFIER> [--comments N]"
disable-model-invocation: false
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(jq)
  - Bash(curl)
---

# Show Linear Issue

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh show-linear-issue`

> **Configuration**: Set `work.integration: linear` in `.accelerator/config.md`.
> See the
> [`### work` section of `configure/SKILL.md`](../../config/configure/SKILL.md#work)
> for the full reference.

Fetch and render a single Linear issue by identifier.

## Step 1: Resolve the identifier

Read the issue identifier from the argument string (e.g. `BLA-123`). If none was
supplied, ask the user which issue to show.

## Step 2: Fetch the issue

Run:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/linear-show-flow.sh <IDENTIFIER> [--comments N]
```

Run the bare path **directly** as an executable; never prefix it with
`bash`/`sh`/`env` (a wrapper prefix escapes the skill's `allowed-tools`
permission and forces an unnecessary prompt).

An unknown identifier exits non-zero (`E_SHOW_NOT_FOUND`) — tell the user the
issue was not found.

## Step 3: Render the issue

Render the issue's fields under `.data.issue`:

- **Identifier**: `.identifier`
- **Title**: `.title`
- **State**: `.state.name`
- **Assignee**: `.assignee.name` (or `unassigned`)
- **Description**: `.description` — already native Markdown, render it directly
  (no ADF conversion).
- **Comments**: each `.comments.nodes[].body` (Markdown), if any.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh show-linear-issue`
