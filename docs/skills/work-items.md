# Work Items

Work item skills capture work items — features, bugs, tasks, spikes, and epics —
as structured documents that feed into planning. The filename prefix
defaults to a 4-digit number (`meta/work/0042-add-search.md`) but is
configurable via `work.id_pattern` and `work.default_project_code` —
e.g. `{project}-{number:04d}` with `default_project_code: PROJ` produces
`meta/work/PROJ-0042-add-search.md`. The `work.integration` key (allowed
values `jira`, `linear`, `trello`, `github-issues`; empty by default)
selects the active remote tracker — only `jira` and `linear` ship skills today;
`trello` and `github-issues` are reserved values with no implementation yet.
When unset, all work-management skills operate purely against `meta/work/` with
no external API calls. See
[`skills/config/configure/SKILL.md`](../../skills/config/configure/SKILL.md#work)
for the full reference.

```
existing docs (specs, PRDs, notes)
       │
       ├── extract-work-items ──┐
       │                     ↓
       create-work-item ──→  meta/work/  ←── update-work-item
                              │
                              ├── list-work-items ──┬──→  review-work-item → meta/reviews/work/
                              │                     └──→  create-plan → implement-plan
                              └── sync-work-items ⇄ remote tracker (Jira/Linear)
```

| Skill                  | Usage                                                                                         | Description                                                                                                                                           |
|------------------------|-----------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------|
| **create-work-item**   | `/accelerator:create-work-item [topic-or-ref]`                                                | Interactively create a work item from a topic, or enrich an existing one by path or number                                                            |
| **extract-work-items** | `/accelerator:extract-work-items [doc paths...]`                                              | Batch-extract work items from existing specs, PRDs, research, plans, or notes                                                                         |
| **list-work-items**    | `/accelerator:list-work-items [filter]`                                                       | List and filter work items by status, type, priority, tag, parent, or title; shows a colour-coded Sync column when a remote integration is configured |
| **update-work-item**   | `/accelerator:update-work-item [work-item-ref] [field-op]`                                    | Update work item fields with diff preview and confirmation                                                                                            |
| **sync-work-items**    | `/accelerator:sync-work-items [--push-only\|--pull-only] [--preview] [--all] [filter-flags…]` | Reconcile local work items with the configured remote tracker (Jira or Linear), detecting per-item sync state and resolving conflicts                 |
| **review-work-item**   | `/accelerator:review-work-item [work-item-ref]`                                               | Review a work item through completeness, testability, and clarity lenses                                                                              |

Work items use a shared template with YAML frontmatter (`work_item_id`, `title`,
`type`, `status`, `priority`, `parent`, `tags`) and structured body sections
(Summary, Context, Requirements, Acceptance Criteria, Open Questions,
Dependencies, Assumptions, Technical Notes, Drafting Notes, References).
The template is customisable via
`/accelerator:configure templates eject work-item`.
