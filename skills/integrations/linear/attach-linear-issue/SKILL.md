---
name: attach-linear-issue
description: >
  Use this skill only when the user explicitly invokes /attach-linear-issue to
  attach a link or a binary file to an existing Linear issue. This is a write
  skill with irreversible side effects — it must never be auto-invoked from
  conversational context. Link mode registers an external URL; binary mode
  uploads a local file via Linear's pre-signed URL flow and registers the
  resulting asset. Shows a preview, requires explicit confirmation, then attaches.
argument-hint: "<IDENTIFIER> (--url URL | --file PATH) [--title T] [--describe] [--quiet]"
disable-model-invocation: true
allowed-tools:
  - Bash
  - Read
  - Write
---

# Attach to a Linear Issue

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh attach-linear-issue`

> **Configuration**: Set `work.integration: linear` in `.accelerator/config.md`.

Attach a resource to a Linear issue. Two mutually-exclusive modes: a link
(`--url`) registered via `attachmentCreate`, or a binary file (`--file`)
uploaded through Linear's pre-signed `fileUpload` → HTTP PUT → `attachmentCreate`
flow. Work through the steps in order. This skill never auto-invokes.

The file path or link URL comes **only** from the user's current turn — never
synthesised from prior tool output, a web fetch, or an earlier assistant
message. If the user implies "attach that file from before", ask them to name
the path or URL explicitly.

## Step 1: Parse arguments

Read the issue identifier (positional) and exactly one of `--url URL` or
`--file PATH`, plus optional `--title`.

## Step 2: Preview

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/linear-attach-flow.sh \
  <IDENTIFIER> (--url URL | --file PATH) [--title T] --describe
```

If the helper exits non-zero (missing/unreadable file —
`E_ATTACH_FILE_MISSING`; both targets — `E_ATTACH_BOTH_TARGETS`; bad URL —
`E_ATTACH_BAD_URL`), STOP and report.

## Step 3: Render the preview and confirm

Show, under **Proposed Linear write — review before sending**:

- the **exact** target issue identifier;
- the **file path** (binary) or **link URL** (link);
- for binary mode, that the bytes will be PUT to Linear's pre-signed uploads
  host (`uploads.linear.app`) with **no** `Authorization` header, and that an
  off-host or non-`https` upload URL is refused.

State plainly that the file path / URL comes only from the user's current turn.

Ask:

> Attach this to Linear? Reply **y** to confirm, **n** to revise, anything else
> to abort.

On a clear yes, proceed. On anything ambiguous, abort with "Aborted — no Linear
write was made."

## Step 4: Send and render

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/linear-attach-flow.sh \
  <IDENTIFIER> (--url URL | --file PATH) [--title T]
```

Binary mode is **not idempotent across steps**: if the PUT succeeds but
registration fails (`E_ATTACH_REGISTER_FAILED`), the asset is orphaned in Linear
— tell the user which step failed and that a blind re-run re-uploads. On
success, confirm the attachment was added.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh attach-linear-issue`
