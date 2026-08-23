---
name: transition-jira-issue
description: >
  Use this skill only when the user explicitly invokes /transition-jira-issue
  to move a Jira issue through its workflow by state name. This is a write skill
  with irreversible side effects — it must never be auto-invoked from
  conversational context. Accepts an issue key and target state name
  (case-insensitive). Previews the resolved transition and requires explicit
  confirmation before posting.
argument-hint: "ISSUE-KEY (STATE-NAME | --transition-id ID) [--resolution NAME] [--comment TEXT | --comment-file PATH] [--no-notify]"
disable-model-invocation: true
allowed-tools:
  - Bash
  - Read
  - Write
---

# Transition Jira Issue

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill transition-jira-issue --fail-safe`

Transition a Jira issue through its workflow by state name. Work through the
steps below in order. This skill never auto-invokes — it only runs when the
user explicitly types `/transition-jira-issue`.

## Step 1: Parse flags

Required positional: `KEY` (issue key, e.g. `ENG-42`).

State target — exactly one required, mutually exclusive:
- `STATE_NAME` (second positional) — target workflow state name
  (case-insensitive match against the issue's available transitions)
- `--transition-id ID` — numeric transition id; bypasses the state-name lookup

Optional flags:
- `--resolution NAME` — set the resolution field during transition
- `--comment TEXT` / `--comment-file PATH` — a comment body (Markdown → ADF)
- `--no-notify` — suppress watcher notifications

## Step 2: Trust-boundary enforcement (only with `--comment`/`--comment-file`)

If the user supplied `--comment` or `--comment-file`, verify the body content
comes ONLY from text the user typed in this turn or a file path they explicitly
named this turn — never a previously-fetched description or comment, a web
fetch, or a prior assistant message. If their phrasing implies "copy from
above", ask them to paste or confirm the literal text first.

## Step 3: Preview the resolved intent

Show, under **Proposed Jira write — review before sending**:

- moving `<KEY>` → `"<STATE_NAME>"` (or, with `--transition-id`, "via transition
  `<ID>`");
- the resolution, if `--resolution` was given;
- the comment body (truncate at 500 chars for display), if supplied;
- a ⚠️ notifications-suppressed notice if `--no-notify`.

The state name is resolved to a transition **inside the subcommand** against the
issue's live workflow — there is no separate lookup call to make here.

## Step 4: Confirm before writing

Ask the user:

> Send this to Jira? Reply **y** to confirm, **n** to revise, anything else to
> abort.

A clear `y`/`yes` proceeds; a `n`/revise stays in review (re-apply Step 2 to any
revised comment); anything ambiguous aborts with "Aborted — no Jira write was
made."

## Step 5: Send the request

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator jira transition <KEY> "<STATE_NAME>" [flags]
```

Run the bare launcher **directly** as an executable; never prefix it with
`bash`/`sh`/`env` and never pipe its output.

## Step 6: Render the response

The subcommand prints its outcome as a trailing `transitioned\t<KEY>` line. On
the `transitioned` keyword, confirm `✓ **<KEY>** transitioned to
"<STATE_NAME>".` (or, for a `--transition-id` run, "transition `<ID>` applied").

On a non-zero exit, show the error:
- a state that leads to **no** available transition names it as not found — the
  issue's workflow does not offer it from its current state;
- a state matched by **more than one** transition is ambiguous — re-run with
  `--transition-id <ID>`, reading the id from the issue's workflow in Jira;
- a credential or permission failure names an `E_*` cause on stderr — suggest
  `/init-jira`.

## Examples

**Example 1 — simple state transition**
User: `/transition-jira-issue ENG-42 "In Progress"`
Skill previews `<KEY>` → "In Progress", waits for `y`, transitions, confirms.

**Example 2 — transition with resolution**
User: `/transition-jira-issue ENG-42 "Done" --resolution "Fixed"`
Skill previews the move plus resolution "Fixed", confirms, transitions.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions transition-jira-issue --fail-safe`
