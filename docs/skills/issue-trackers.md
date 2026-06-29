# Issue Trackers (Jira & Linear)

Accelerator integrates with two remote issue trackers — **Jira** and
**Linear** — through one shared pattern. Read skills (search, show) auto-trigger
on natural-language phrasing; write skills are slash-only and display a payload
preview that you must explicitly confirm before any change reaches the tracker.
Each integration keeps a team-shared catalogue (committed) alongside gitignored
per-developer credentials, and treats the presence of an `external_id` on a work
item as the signal that it is synced to the remote. Building on that signal,
`/accelerator:sync-work-items` reconciles `meta/work/` against the tracker in
batch — pushing never-synced items, pulling untracked remote issues, and
resolving per-item divergence. Select the active tracker with the
`work.integration` key (`jira` or `linear`); when unset, the work-management
skills stay local with no external API calls.

<a id="jira-integration"></a>

## Jira

Accelerator includes a full set of skills for interacting with a Jira Cloud
tenant — searching for and reading issues, creating and updating them,
commenting, transitioning through workflows, and uploading attachments. Run
`/accelerator:init-jira` once to verify credentials and persist the
team-shared field and project catalogue before using the other skills.

### Jira Configuration

Add the shared site setting to `.accelerator/config.md` and personal
credentials to `.accelerator/config.local.md` (gitignored):

```yaml
# .accelerator/config.md — commit this
---
jira:
  site: your-subdomain   # e.g. "atomic-innovation" for atomic-innovation.atlassian.net
---
```

```yaml
# .accelerator/config.local.md — do not commit
---
jira:
  email: you@example.com
  token_cmd: "op read op://Work/Atlassian/credential"  # any password-manager command
---
```

The default project key reuses `work.default_project_code`; set
`work.integration: jira` to enable auto-scoping. See
`/accelerator:configure help` for the full credential resolution chain and
`token_cmd` examples (1Password, `pass`, macOS Keychain, AWS Secrets Manager).

### Jira Skills

| Skill                     | Usage                                              | Description                                                                                                                                       |
|---------------------------|----------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------|
| **init-jira**             | `/accelerator:init-jira`                           | Verify credentials, discover projects and custom fields, persist the team-shared catalogue to `.accelerator/state/integrations/jira/`             |
| **search-jira-issues**    | `/accelerator:search-jira-issues [flags]`          | Search via structured flags (assignee, status, label, type, component, reporter, parent, watching); composes safe JQL with a `--jql` escape hatch |
| **show-jira-issue**       | `/accelerator:show-jira-issue <KEY>`               | Fetch a single issue with optional comment slice and Markdown rendering of ADF descriptions                                                       |
| **create-jira-issue**     | `/accelerator:create-jira-issue [flags]`           | Create a new issue; body accepted inline, from a file, from stdin, or via `$EDITOR`                                                               |
| **update-jira-issue**     | `/accelerator:update-jira-issue <KEY> [flags]`     | Edit summary, description, assignee, priority, labels, components, parent, and custom fields                                                      |
| **comment-jira-issue**    | `/accelerator:comment-jira-issue <sub> <KEY>`      | Add, list, edit, or delete comments (`add`, `list`, `edit`, `delete` sub-actions)                                                                 |
| **transition-jira-issue** | `/accelerator:transition-jira-issue <KEY> <state>` | Move an issue through its workflow by state name (case-insensitive lookup), with optional resolution and comment                                  |
| **attach-jira-issue**     | `/accelerator:attach-jira-issue <KEY> <file...>`   | Upload one or more files as issue attachments                                                                                                     |

Read skills (`search-jira-issues`, `show-jira-issue`) trigger automatically on
natural-language phrasing. Write skills are slash-only — they display a
payload preview and require explicit confirmation before making any change to
the tenant.

### Jira ADF / Markdown

Jira Cloud v3 stores rich-text fields in Atlassian Document Format (ADF).
Accelerator converts bidirectionally using pure bash + awk + jq with no
additional dependencies:

- **Reading** — ADF is rendered to Markdown by default on `show-jira-issue`
  (pass `--no-render-adf` for raw JSON) and on `comment-jira-issue list`.
  `search-jira-issues` defaults render-off; pass `--render-adf` for bulk
  results with description text.
- **Writing** — `create-jira-issue`, `update-jira-issue`, and
  `comment-jira-issue add`/`edit` convert Markdown input to ADF before
  sending.

Supported Markdown: paragraphs, headings (`#`–`######`), fenced code blocks
with language, single-level bullet/ordered lists, GitHub-style checklists
(`- [ ]` / `- [x]`), inline bold/italic/code/links, and hard breaks.

### Jira state cache

`/init-jira` persists the field catalogue, project list, and site metadata to
`.accelerator/state/integrations/jira/`. This directory is version-controlled
and team-shared — instance-specific custom field IDs are not secrets and are
worth committing so teammates don't need to re-run init. Per-developer
credentials live in `.accelerator/config.local.md` and env vars only.

## Linear

Accelerator includes the same shape of skills for a Linear workspace — searching
and reading issues, creating and updating them, commenting, transitioning by
workflow state, and attaching links or files. It talks to the Linear GraphQL API
directly (Markdown-native — no ADF conversion). Run `/accelerator:init-linear`
once to verify the token and cache the team and workflow-state catalogue before
using the other skills.

### Linear Configuration

Linear uses **token-only** auth — no site or email. Put a Linear personal API
key in the gitignored `.accelerator/config.local.md`:

```yaml
# .accelerator/config.local.md — do not commit
---
linear:
  token_cmd: "op read op://Work/Linear/credential"  # or linear.token: lin_api_…
---
```

The token is resolved env → `config.local.md` → `config.md` (token only);
`.accelerator/config.local.md` must be mode `0600` or stricter. `init-linear`
fixes the workspace to a single team and caches that team along with its
workflow states (the statuses issues move through) under
`.accelerator/state/integrations/linear/` — `catalogue.json` is committed and
team-shared, while `viewer.json` is gitignored and per-developer. Set
`work.integration: linear` to enable auto-scoping.

### Linear Skills

| Skill                       | Usage                                                              | Description                                                              |
|-----------------------------|--------------------------------------------------------------------|--------------------------------------------------------------------------|
| **init-linear**             | `/accelerator:init-linear`                                         | Verify the token, cache the team and workflow-state catalogue            |
| **search-linear-issues**    | `/accelerator:search-linear-issues [flags]`                        | Search issues by state, assignee, label, or text (cursor-paginated)      |
| **show-linear-issue**       | `/accelerator:show-linear-issue <IDENTIFIER>`                      | Read a single issue, with an optional comment slice                      |
| **create-linear-issue**     | `/accelerator:create-linear-issue <work-item-file>`                | Create an issue from a work-item file (payload preview, then confirm)    |
| **update-linear-issue**     | `/accelerator:update-linear-issue <IDENTIFIER> [flags]`            | Edit title, description, state, assignee, or priority on an issue        |
| **comment-linear-issue**    | `/accelerator:comment-linear-issue <IDENTIFIER> --body …`          | Add a comment (`--body` text or `--body-file`)                           |
| **transition-linear-issue** | `/accelerator:transition-linear-issue <IDENTIFIER> <STATE-NAME>`   | Move an issue through its workflow by state name                         |
| **attach-linear-issue**     | `/accelerator:attach-linear-issue <IDENTIFIER> (--url \| --file)`  | Attach a URL or file to an issue                                         |

Read skills (`search-linear-issues`, `show-linear-issue`) trigger automatically
on natural-language phrasing. Write skills are slash-only — they display a
payload preview and require explicit confirmation before making any change to
the workspace.
