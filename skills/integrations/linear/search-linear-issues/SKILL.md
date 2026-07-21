---
name: search-linear-issues
description: >
  Use this skill whenever the user wants to search, list, or filter Linear
  issues — by state, assignee, label, or free text — even if they say 'find',
  'show me', 'what's open', 'list my issues', or similar phrasing rather than
  'search Linear'. Composes a Linear IssueFilter from structured flags, executes
  a cursor-paginated search scoped to the configured team, and renders a summary
  table of the results. Prefer this skill over raw GraphQL whenever the user's
  intent maps to a structured flag.
argument-hint: "[--state NAME] [--assignee NAME] [--label NAME] [--text STR] [--limit 1..250] [--quiet]"
disable-model-invocation: false
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)
  - Bash(jq)
  - Bash(curl)
---

# Search Linear Issues

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill search-linear-issues --fail-safe`

> **Configuration**: Set `work.integration: linear` in `.accelerator/config.md`.
> The team is fixed at `/init-linear` time (single-team scoping) — there is no
> per-search `--team` flag. See the
> [`### work` section of `configure/SKILL.md`](../../config/configure/SKILL.md#work)
> for the full reference.

Search the configured Linear team using structured flags that compose into a
Linear `IssueFilter`. Work through the steps below in order.

## Step 1: Parse the flag set

Read the argument string and note each flag:

- `--state NAME` — WorkflowState name. Resolved (case-insensitively) to its
  team-scoped UUID via the cached catalogue; an unknown state is an error.
- `--assignee NAME` — assignee display name.
- `--label NAME` — label name.
- `--text STR` — free-text match on the issue title.
- `--limit N` — page size (1..250, default 50). Pagination follows every page
  regardless; `--limit` only sets the per-request page size.

## Step 2: Run the search

Run the search flow, passing the flags through verbatim:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/linear-search-flow.sh [flags]
```

Run the bare path **directly** as an executable; never prefix it with
`bash`/`sh`/`env` (a wrapper prefix escapes the skill's `allowed-tools`
permission and forces an unnecessary prompt).

The flow echoes the composed `IssueFilter` to stderr (`INFO:`) for auditability
and emits a single merged JSON result with all pages under `.data.issues.nodes`.
If `.data.issues.truncated` is `true`, more pages remained than were fetched —
say so.

## Step 3: Render the results

Render a Markdown table with one row per issue:

| Identifier | Title | State | Assignee |
|------------|-------|-------|----------|

Read each row from `.data.issues.nodes[]`: `.identifier`, `.title`,
`.state.name`, `.assignee.name` (show `—` for an unassigned issue). Report the
total count and note if the result was truncated.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions search-linear-issues --fail-safe`
