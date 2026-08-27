---
name: attach-jira-issue
description: >
  Use this skill only when the user explicitly invokes /attach-jira-issue to
  upload one or more local files as attachments to a Jira issue. This is a
  write skill with irreversible side effects — it must never be auto-invoked
  from conversational context. Previews what will be uploaded and requires
  explicit confirmation before POSTing.
argument-hint: "ISSUE-KEY FILE [FILE...]"
disable-model-invocation: true
allowed-tools:
  - Bash
  - Read
  - Write
---

# Attach files to a Jira Issue

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill attach-jira-issue --fail-safe`

Upload one or more local files as attachments to a Jira issue. Work through
the steps below in order. This skill never auto-invokes — it only runs when
the user explicitly types `/attach-jira-issue`.

## Step 1: Parse flags

Required:
- `KEY` — issue key (first positional), e.g. `ENG-42`
- `FILE [FILE...]` — one or more local file paths to upload

## Step 2: Preview the resolved intent

Show, under **Proposed Jira write — review before sending**:

- `POST` attachments to `<KEY>`;
- the file list: for each path, run `wc -c <path>` via Bash and show the
  basename with a humanised size (≥ 1 048 576 bytes → MB, ≥ 1 024 bytes → KB,
  otherwise bytes). A missing or unreadable path is surfaced here — say which
  file could not be read and stop, making no API call.
- ⚠️ "Attachments cannot be removed by this skill once uploaded."

## Step 3: Confirm before writing

Ask the user:

> Send this to Jira? Reply **y** to confirm, **n** to revise, anything else to
> abort.

A clear `y`/`yes` proceeds; a `n`/revise stays in review (typically different
paths or a different key); anything ambiguous aborts with "Aborted — no Jira
write was made."

## Step 4: Send the request

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator jira attach <KEY> <FILE> [FILE...]
```

Run the bare launcher **directly** as an executable; never prefix it with
`bash`/`sh`/`env` and never pipe its output.

## Step 5: Render the response

On success the subcommand emits the attachments as a JSON array (it carries no
outcome envelope — success is the clean, zero-status run). For each element show
the filename, id, and humanised size (same KB/MB logic as Step 2):

```
✓ Attached to <KEY>:
- <filename> (ID: <id>, <humanised-size>)
```

If the array is empty or unparseable, tell the user "Upload succeeded but the
response was empty or unreadable." On a non-zero exit, show the error — a
missing/unreadable file or a credential failure names its cause on stderr;
suggest `/init-jira` for a credential failure.

## Examples

**Example — single file**
User: `/attach-jira-issue ENG-42 ./screenshot.png`
Skill previews the upload (`screenshot.png`, e.g. 84.3 KB), waits for `y`,
uploads, and confirms with the returned id and size.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions attach-jira-issue --fail-safe`
