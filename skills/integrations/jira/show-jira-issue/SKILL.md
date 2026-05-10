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
argument-hint: "<ISSUE-KEY> [--fields a,b,c|--fields a]... [--expand a,b,c] [--comments N] [--render-adf|--no-render-adf]"
disable-model-invocation: false
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(jq)
  - Bash(curl)
---

# Show Jira Issue

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh show-jira-issue`

Fetch and render a single Jira issue by key. Work through the steps below
in order.

## Step 1: Parse the issue key and flags

The first positional argument is the issue key (e.g. `ENG-42`, `PROJ-1234`).
Remaining arguments are passed through to the helper as-is.

`--render-adf` defaults to ON at the helper layer — single-issue reads are
for humans, so rendered Markdown is the natural output. Pass `--no-render-adf`
verbatim if the user explicitly asked for raw ADF or JSON. The SKILL does
not invert the default; the default lives in the helper where it is testable
and consistent with running the helper directly.

## Step 2: Run the helper

Invoke `jira-show-flow.sh` with the key and any flags supplied:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-show-flow.sh \
  <ISSUE-KEY> [flags]
```

If the exit code is non-zero, show the error to the user:

- Exit 80 (`E_SHOW_NO_KEY`): no issue key was supplied — ask the user for one.
- Exit 81 (`E_SHOW_BAD_COMMENTS_LIMIT`): `--comments` was out of range [0, 100].
  Explain the constraint.
- Exit 82 (`E_SHOW_BAD_FLAG`): unrecognised flag. Show the usage banner from
  the helper's stderr.
- Any `jira-request.sh` exit (11–23): show the error and suggest checking
  credentials with `/init-jira`.

## Step 3: Render the result

Parse the JSON response and present a human-readable summary:

- **Heading**: `## KEY — Summary text`
- **Fields block**: Status, Type, Priority, Assignee, Reporter (omit if absent).
- **Description**: render inline as Markdown (already rendered by the helper
  when `--render-adf` is on).
- **Comments** (when `--comments N` was passed): render each comment as a
  mini conversation block — author, timestamp, body.

If the `fields` object is sparse (e.g. `--fields summary,status` was used),
only render the fields that are present; do not invent missing ones.

## Examples

**Example 1 — look up an issue**
User: "look up PROJ-1234"
Skill invokes:
```
jira-show-flow.sh PROJ-1234
```
(No `--render-adf` flag needed — it is the helper default.)
Then renders the issue with description as Markdown.

**Example 2 — show recent comments**
User: "what's the discussion on ENG-42 — show me the last few comments"
Skill invokes:
```
jira-show-flow.sh ENG-42 --comments 5
```
Then renders the summary + last 5 comments as an inline conversation.

**Example 3 — raw JSON escape hatch**
User: "give me the raw JSON for ENG-42 — I'm piping it to jq"
Skill invokes:
```
jira-show-flow.sh ENG-42 --no-render-adf
```
Then prints the response with ADF intact.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh show-jira-issue`
