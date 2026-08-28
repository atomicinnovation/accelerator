---
name: create-jira-issue
description: >
  Use this skill only when the user explicitly invokes /create-jira-issue to
  create a new Jira issue. This is a write skill with irreversible side effects
  — it must never be auto-invoked from conversational context. Accepts a project
  key, issue type, summary, optional Markdown body, and optional fields
  (assignee, reporter, priority, labels, components, parent, custom fields).
  Previews the resolved intent, requires explicit confirmation, then POSTs to
  Jira and returns the new issue key.
argument-hint: "[--project KEY] --type NAME --summary TEXT [--body TEXT | --body-file PATH] [--assignee @me|ACCTID] [--reporter @me|ACCTID] [--priority NAME] [--label NAME]... [--component NAME]... [--parent KEY] [--custom SLUG=VALUE]... [--issuetype-id ID]"
disable-model-invocation: true
allowed-tools:
  - Bash
  - Read
  - Write
---

# Create Jira Issue

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill create-jira-issue --fail-safe`

> **Configuration**: Set `work.integration: jira` and
> `work.default_project_code: <KEY>` in `.accelerator/config.md` to
> enable auto-scoping. See the
> [`### work` section of `configure/SKILL.md`](../../config/configure/SKILL.md#work)
> for the full reference.

Create a new Jira issue via `POST /rest/api/3/issue`. Work through the steps
below in order. This skill never auto-invokes — it only runs when the user
explicitly types `/create-jira-issue`.

This skill never synthesises `--body` content from upstream context (issue
descriptions, web fetches, prior tool output) without explicit user approval —
body content always comes from the user's prompt or a path the user named.

## Two modes

This skill accepts **either** a work-item file or the explicit flag set:

- **Work-item-file mode** — the argument is a path to a `meta/work/` work item.
  The summary, body, type, and project are derived from the work item, and the
  created issue's key is written back into its `external_id`. Follow the
  **Work-item-file mode** section.
- **Flag-driven mode** — the argument is the `--project/--type/--summary/…` flag
  set (no work-item file). Follow **Steps 1–9**; it writes nothing back to any
  file.

Both modes share one create contract: read/preview/confirm/create, mirroring
`/create-linear-issue` in shape.

## Work-item-file mode

### WF-1: Resolve the issue type and project

Run the read-only resolver against the work-item file (pass `--project KEY` only
if the user explicitly overrode it):

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator jira resolve-fields --file <work-item-file>
```

Run the bare launcher **directly** as an executable; never prefix it with
`bash`/`sh`/`env` and never pipe its output. It prints one tab-separated line
`<issue_type>\t<issue_type_source>\t<project>\t<project_source>`. This is the
kind→issue-type map and project resolution read from the `config` crate — the
**same resolution path** `accelerator work create --push --dry-run` renders (it
emits the same fields prefixed by the tracker name), so the two entry points can
never disagree. Handle a non-zero exit **before** any preview or create:

- `E_RESOLVE_ALREADY_SYNCED` — the work item already carries a non-empty
  `external_id`. STOP; tell the user it is already synced and name the existing
  identifier. Make no API call.
- `E_RESOLVE_NO_PROJECT` — the project is unresolvable. STOP and tell the user
  to pass `--project KEY` or set `work.default_project_code`. This is a
  pre-create failure — do not proceed to the confirm gate.

On success, capture the four resolved values and continue.

### WF-2: Preview the resolved intent

Read the work item's `title` (the summary) and Markdown body (the description).
Show, under **Proposed Jira write — review before sending**:

- the resolved **issue type**, and — when `issue_type_source` is `default` —
  that the kind fell through, e.g. `kind "spike" → Task (default)`;
- the resolved **project** and which source it came from (`project_source`);
- the summary and the (≤500-char-truncated) description.

### WF-3: Confirm before writing

Ask the user to reply **y** to confirm, or anything else to abort:
`Create this Jira issue and set the work item's external_id?` Exactly `y`/`yes`
proceeds; anything else aborts with "Aborted — no Jira write was made."

### WF-4: Create and write back

On confirmation, create via the bare-key projection, writing the work item's
body to a file for `--body-file`:

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator jira create --project <project> --type <issue_type> --summary "<title>" --body-file <body-path> --emit key
```

`--emit key` prints **only** the bare issue key on success. Branch **fail-closed**:

- a bare key on a clean run — the issue exists remotely. Write it into the work
  item's `external_id` (insert the line if absent):

  ```
  ${CLAUDE_PLUGIN_ROOT}/bin/accelerator work link-external-id <work-item-file> <KEY>
  ```

  Then report `Issue created: <KEY> — the work item's external_id is now <KEY>.`
- `E_REQ_BAD_RESPONSE` on stderr — the issue **was** created remotely but its
  key is unusable. Do **not** write `external_id`; surface this loudly, give the
  user the key, and tell them NOT to re-run (it would create a duplicate) and to
  reconcile `external_id` by hand.
- any other non-zero exit — no issue was created. Do **not** write
  `external_id`; report the error. The file is unchanged, so it is safe to fix
  and retry.

The create-then-writeback sequence is **non-atomic**: if the create succeeds but
the `work link-external-id` writeback fails, surface it loudly — the
issue exists remotely as `<KEY>`, the user must NOT blindly re-run (it would
duplicate), and they should set `external_id: <KEY>` by hand.

## Step 1: Parse the flag set

Read the argument string and note each flag:

- `--project KEY` — Jira project key (e.g. `ENG`)
- `--type NAME` / `--issuetype-id ID` — issue type by name or numeric id (the id
  wins); one is required
- `--summary TEXT` — single-line summary (required)
- `--body TEXT` / `--body-file PATH` — Markdown description
- `--assignee @me|ACCTID` / `--reporter @me|ACCTID` — `@me` resolves via
  `site.json`; an email is refused
- `--priority NAME`, `--label NAME` (repeatable), `--component NAME`
  (repeatable), `--parent KEY`
- `--custom SLUG=VALUE` — repeatable; custom field by slug, `@json:<literal>`
  for arrays/objects

## Step 2: Resolve --project

If `--project` was not supplied, the subcommand reads `work.default_project_code`
from config. If neither is set the create refuses before any call; warn the user
to supply `--project KEY` or run `/init-jira` and set a default project.

## Step 3: Trust-boundary enforcement

Before assembling `--body`, verify that any body content comes ONLY from text the
user typed in this turn or a file path they explicitly named this turn — never a
previously-fetched description, a web fetch, or a prior assistant message. If
their phrasing implies "use the description from above", ask them to paste or
confirm the literal text first.

## Step 4: Preview the resolved intent

Show, under **Proposed Jira write — review before sending**: the project, issue
type, summary, the (≤500-char-truncated) description, and every resolved field
(assignee/reporter/priority/labels/components/parent/custom). `@me` and custom
slugs resolve inside the subcommand and are refused before any write if
unresolvable.

## Step 5: Confirm before writing

Ask the user:

> Send this to Jira? Reply **y** to confirm, **n** to revise, anything else
> to abort.

A clear `y`/`yes` proceeds; a `n`/revise stays in review (re-apply Step 3 to any
revised body); anything ambiguous aborts with "Aborted — no Jira write was
made."

## Step 6: Send the request

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator jira create [all flags from Steps 1-2]
```

## Step 7: Render the response

The subcommand emits a JSON envelope `{key}` with a top-level `outcome` keyword.
On the `created` keyword, present:

> Issue created: **<KEY>**

## Step 8: Error handling

On a non-zero exit, show the error, which names its cause on stderr:
- `E_CREATE_NO_SUMMARY` — `--summary` is required;
- `E_CREATE_BAD_FIELD` — a `--custom` value failed validation; run
  `/init-jira --refresh-fields` if a field id was rejected;
- `E_CREATE_NO_SITE_CACHE` — `@me` was used but `site.json` is missing; run
  `/init-jira`;
- `E_CREATE_BAD_ASSIGNEE` — `--assignee`/`--reporter` accepts `@me` or a raw
  accountId, not an email;
- an auth or transport failure — suggest checking credentials with `/init-jira`.

## Step 9: Examples

**Example — minimal task**
User: `/create-jira-issue --project ENG --type Task --summary "Fix login timeout"`
Skill previews the resolved intent, waits for `y`, then creates and renders
`Issue created: ENG-456`.

**Example — custom field with @json escape**
User: `/create-jira-issue --project ENG --type Task --summary "Sprint task" --custom sprint=@json:[42]`
Skill coerces the sprint field as a JSON array literal, previews, confirms.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions create-jira-issue --fail-safe`
