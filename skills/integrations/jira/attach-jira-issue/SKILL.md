---
name: attach-jira-issue
description: >
  Use this skill only when the user explicitly invokes /attach-jira-issue to
  upload one or more local files as attachments to a Jira issue. This is a
  write skill with irreversible side effects — it must never be auto-invoked
  from conversational context. Shows a preview of what will be uploaded and
  requires explicit confirmation before POSTing.
argument-hint: "ISSUE-KEY FILE [FILE...] [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash
  - Read
  - Write
---

# Attach files to a Jira Issue

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh attach-jira-issue`

Upload one or more local files as attachments to a Jira issue. Work through
the steps below in order. This skill never auto-invokes — it only runs when
the user explicitly types `/attach-jira-issue`.

## Step 1: Parse flags

Required:
- `KEY` — issue key (first positional), e.g. `ENG-42`
- `FILE [FILE...]` — one or more local file paths to upload

Optional flags:
- `--quiet` — suppress INFO stderr lines

## Step 2: Generate the describe preview

Invoke the flow script with `--describe`:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-attach-flow.sh \
  --describe <KEY> <FILE> [FILE...]
```

File validation runs before `--describe` returns, so missing or unreadable
files surface here as exit 132, before any confirmation is shown.

## Step 3: Handle preview failures

- **Exit 132 (`E_ATTACH_FILE_MISSING`)**: tell the user:
  > File not found or not readable: `<path>`. Check the path and try again.

  Stop — do not proceed to confirm.

- **Exit 133 (`E_ATTACH_BAD_FLAG`)**: tell the user:
  > Unrecognised flag. Usage: `/attach-jira-issue ISSUE-KEY FILE [FILE...] [--quiet]`

  Stop.

- **Other non-zero**: tell the user "Preview failed — no API call was made."
  Stop.

## Step 4: Render the preview

Show under this heading:

> **Proposed Jira write — review before sending**

Content:
- Endpoint: `POST /rest/api/3/issue/<KEY>/attachments`
- File list: for each file in the `files` array from the describe JSON, run
  `wc -c <path>` via Bash and show the basename with humanised size:
  - ≥ 1 048 576 bytes → display as MB (1 decimal place)
  - ≥ 1 024 bytes → display as KB (1 decimal place)
  - otherwise → display as bytes
- ⚠️ "Attachments cannot be removed by this skill once uploaded."

## Step 5: Confirm

Ask the user (exact canonical phrase):

> Send this to Jira? Reply **y** to confirm, **n** to revise, anything else to abort.

Interpret the reply:
- **Clear yes** (`y` / `yes`): proceed to Step 6.
- **Clear no** (`n` / `no` or an explicit revision request): ask "What would
  you like to change?" (typically the user will supply different file paths or
  a different issue key). Rebuild the preview from Step 2 with updated
  parameters. Allow up to 3 revision cycles; after the third rejection abort
  with "Aborted — no Jira write was made."
- **Ambiguous or off-topic**: abort immediately with "Aborted — no Jira write
  was made."

## Step 6: Send

Invoke without `--describe`:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-attach-flow.sh \
  <KEY> <FILE> [FILE...] [--quiet]
```

## Step 7: Render the response

Parse the JSON array returned by the API. For each element show filename, ID,
and humanised size (same KB/MB logic as Step 4):

```
✓ Attached to **<KEY>**:
- <filename> (ID: <id>, <humanised-size>)
```

If the JSON array is empty or unparseable, tell the user "Upload succeeded but
the response was empty or unreadable."

## Step 8: Exit-code handling

| Code | Name | User-facing message |
|------|------|---------------------|
| 130 | `E_ATTACH_NO_KEY` | No issue key supplied. Pass the key as the first positional argument. |
| 131 | `E_ATTACH_NO_FILES` | No file paths supplied. Provide at least one file path after the issue key. |
| 132 | `E_ATTACH_FILE_MISSING` | A named file does not exist or is not readable. Check the path and try again. |
| 133 | `E_ATTACH_BAD_FLAG` | Unrecognised flag. See usage above. |
| 11, 12 | auth errors | Check credentials with `/init-jira`. Note: exit 12 (HTTP 403) means you do not have the `CreateAttachments` permission on this project — check your Jira role with your project admin. |
| 13 | not found | Issue not found or you do not have permission to see it. |
| 19 | rate-limited | Wait briefly and retry. |
| 20 | server error | Jira returned a server error; check the Jira status page. |
| 21 | connection | Connection failed; check network and Jira site config. |
| 34 | `E_REQ_BAD_REQUEST` | HTTP 400 — Jira rejected the upload. See the error body above. |

## Examples

**Example 1 — single file**
User: `/attach-jira-issue ENG-42 ./screenshot.png`
Skill runs `--describe`, shows preview (POST to `/rest/api/3/issue/ENG-42/attachments`,
`screenshot.png` — e.g. 84.3 KB), waits for `y`, uploads, confirms:
`✓ Attached to **ENG-42**: - screenshot.png (ID: 10001, 84.3 KB)`

**Example 2 — multiple files**
User: `/attach-jira-issue ENG-42 ./logs.txt ./debug.json`
Skill shows preview listing both files with their sizes, waits for `y`, uploads
both in one request, confirms:
```
✓ Attached to **ENG-42**:
- logs.txt (ID: 10001, 12.4 KB)
- debug.json (ID: 10002, 3.1 KB)
```

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh attach-jira-issue`
