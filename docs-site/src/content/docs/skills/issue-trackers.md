---
title: 'Issue Trackers (Jira & Linear)'
---

Accelerator integrates with two remote issue trackers — **Jira** and
**Linear** — through one shared pattern. Read skills (search, show)
auto-trigger on natural-language phrasing; write skills are slash-only
and display a payload preview that you must explicitly confirm before
any change reaches the tracker. Each integration keeps a team-shared
catalogue (committed) alongside gitignored per-developer credentials,
and treats the presence of an `external_id` on a work item as the signal
that it is synced to the remote. Building on that signal,
[`sync-work-items`](../reference/skills/work/sync-work-items.md)
reconciles `meta/work/` against the tracker in batch. Select the active
tracker with the `work.integration` key (`jira` or `linear`); when
unset, the work-management skills stay local with no external API calls.

Both integrations ship the same eight skill shapes:

| Shape      | Jira                                                                                  | Linear                                                                                        |
|------------|----------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------|
| Initialise | [`init-jira`](../reference/skills/integrations/jira/init-jira.md)                       | [`init-linear`](../reference/skills/integrations/linear/init-linear.md)                           |
| Search     | [`search-jira-issues`](../reference/skills/integrations/jira/search-jira-issues.md)     | [`search-linear-issues`](../reference/skills/integrations/linear/search-linear-issues.md)         |
| Show       | [`show-jira-issue`](../reference/skills/integrations/jira/show-jira-issue.md)           | [`show-linear-issue`](../reference/skills/integrations/linear/show-linear-issue.md)               |
| Create     | [`create-jira-issue`](../reference/skills/integrations/jira/create-jira-issue.md)       | [`create-linear-issue`](../reference/skills/integrations/linear/create-linear-issue.md)           |
| Update     | [`update-jira-issue`](../reference/skills/integrations/jira/update-jira-issue.md)       | [`update-linear-issue`](../reference/skills/integrations/linear/update-linear-issue.md)           |
| Comment    | [`comment-jira-issue`](../reference/skills/integrations/jira/comment-jira-issue.md)     | [`comment-linear-issue`](../reference/skills/integrations/linear/comment-linear-issue.md)         |
| Transition | [`transition-jira-issue`](../reference/skills/integrations/jira/transition-jira-issue.md) | [`transition-linear-issue`](../reference/skills/integrations/linear/transition-linear-issue.md) |
| Attach     | [`attach-jira-issue`](../reference/skills/integrations/jira/attach-jira-issue.md)       | [`attach-linear-issue`](../reference/skills/integrations/linear/attach-linear-issue.md)           |

Run the init skill once per project before the others: it verifies
credentials and persists the team-shared catalogue under
`.accelerator/state/integrations/`.

## Jira

The Jira skills target a Jira Cloud tenant. Add the shared site setting
to `.accelerator/config.md` and personal credentials to the gitignored
`.accelerator/config.local.md`:

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
`work.integration: jira` to enable auto-scoping. See `/configure help`
for the full credential resolution chain and `token_cmd` examples.

Jira Cloud stores rich text as Atlassian Document Format (ADF);
Accelerator converts bidirectionally — reading skills render ADF to
Markdown, writing skills convert your Markdown to ADF. The
[`init-jira`](../reference/skills/integrations/jira/init-jira.md) state
cache (field catalogue, project list, site metadata) is
version-controlled and team-shared, so teammates don't need to re-run
init.

## Linear

The Linear skills talk to the Linear GraphQL API directly
(Markdown-native — no ADF conversion). Auth is **token-only** — no site
or email. Put a Linear personal API key in the gitignored
`.accelerator/config.local.md`:

```yaml
# .accelerator/config.local.md — do not commit
---
linear:
  token_cmd: "op read op://Work/Linear/credential"  # or linear.token: lin_api_…
---
```

`.accelerator/config.local.md` must be mode `0600` or stricter.
[`init-linear`](../reference/skills/integrations/linear/init-linear.md)
fixes the workspace to a single team and caches that team with its
workflow states — `catalogue.json` is committed and team-shared,
`viewer.json` is gitignored and per-developer. Set
`work.integration: linear` to enable auto-scoping.

Per-skill flags, sub-actions, and behaviours are documented on each
generated skill page linked in the table above.
