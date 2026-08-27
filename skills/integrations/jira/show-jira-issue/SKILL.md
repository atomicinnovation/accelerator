---
name: show-jira-issue
description: >
  Use this skill when the user asks about a specific Jira issue by key
  (e.g. PROJ-123, ENG-456) — for viewing the description, status,
  comments, transitions, or any other field. Trigger when the user says
  'look up', 'check on', 'tell me about', 'what's on', or 'what is the
  status of' a key, or asks any direct question about an issue they
  reference. Do NOT trigger when an issue key appears incidentally inside
  other prose (commit messages, code review comments, release notes),
  where the user is talking about the issue rather than asking to fetch it.
argument-hint: "<ISSUE-KEY> [--fields a,b,c] [--expand a,b,c] [--comments N] [--render-adf|--no-render-adf]"
disable-model-invocation: false
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator jira *)
---

# Show Jira Issue

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill show-jira-issue --fail-safe`

Fetch and render a single Jira issue by key. Work through the steps below
in order.

## Step 1: Parse the issue key and flags

The first positional argument is the issue key (e.g. `ENG-42`, `PROJ-1234`).
Remaining arguments are passed through to the subcommand as-is.

`--render-adf` defaults to ON — single-issue reads are for humans, so rendered
Markdown is the natural output. Pass `--no-render-adf` verbatim if the user
explicitly asked for raw ADF or JSON.

## Step 2: Fetch the issue

Run the show subcommand with the key and any flags supplied:

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator jira show <ISSUE-KEY> [flags]
```

Run the bare launcher **directly** as an executable; never prefix it with
`bash`/`sh`/`env` and never pipe its output (a wrapper prefix or a pipe escapes
the skill's `allowed-tools` permission and forces an unnecessary prompt).

The subcommand emits a JSON document with a top-level `outcome` keyword:
`found` or `not-found`. On `not-found`, tell the user the issue was not found
and check the key. On a non-zero exit, show the error — a credential or site
failure names an `E_*` cause on stderr; suggest `/init-jira`.

## Step 3: Render the result

Parse the JSON response and present a human-readable summary from `.fields`:

- **Heading**: `## KEY — Summary text`
- **Fields block**: Status, Type, Priority, Assignee, Reporter (omit if absent).
- **Description**: render inline as Markdown (already rendered when
  `--render-adf` is on).
- **Comments** (when `--comments N` was passed): render each comment as a
  mini conversation block — author, timestamp, body.

If the `fields` object is sparse (e.g. `--fields summary,status` was used),
only render the fields that are present; do not invent missing ones.

## Examples

**Example 1 — look up an issue**
User: "look up PROJ-1234"
Skill invokes:
```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator jira show PROJ-1234
```
Then renders the issue with description as Markdown.

**Example 2 — show recent comments**
User: "what's the discussion on ENG-42 — show me the last few comments"
Skill invokes:
```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator jira show ENG-42 --comments 5
```
Then renders the summary + last 5 comments as an inline conversation.

**Example 3 — raw JSON escape hatch**
User: "give me the raw JSON for ENG-42 — I'm piping it to jq"
Skill invokes:
```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator jira show ENG-42 --no-render-adf
```
Then prints the response with ADF intact.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions show-jira-issue --fail-safe`
