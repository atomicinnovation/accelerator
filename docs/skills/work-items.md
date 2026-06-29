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

### create-work-item

**What it does** — Interactively create a well-formed work item. Can also enrich
an existing one when given its path or number.

**How to use it** — `/accelerator:create-work-item [topic or existing work item path/number]`

**Advice & guidelines** — Pair with `refine-work-item` to decompose and sharpen
the draft before planning begins.

### extract-work-items

**What it does** — Extract work items in batch from existing documents (specs,
PRDs, research, plans, meeting notes, design docs).

**How to use it** — `/accelerator:extract-work-items [document paths...]` (leave
empty to scan all)

### refine-work-item

**What it does** — Interactively refine a work item by decomposing it into
children, enriching it with codebase context, sharpening its acceptance
criteria, sizing it, or linking it to dependencies.

**How to use it** — `/accelerator:refine-work-item [work item number or path]`

**Advice & guidelines** — Use after a work item is drafted and before planning
begins.

### review-work-item

**What it does** — Review a work item through multiple quality lenses and
collaboratively iterate based on findings.

**How to use it** — `/accelerator:review-work-item [path to work item file]`

**Advice & guidelines** — Runs the multi-lens [Review System](review-system.md)
(completeness, testability, clarity); see that page for the lens catalogue.

### stress-test-work-item

**What it does** — Interactively stress-test a work item by grilling the user on
scope, assumptions, acceptance criteria, edge cases, and dependencies to surface
issues, gaps, and flawed assumptions before implementation is planned.

**How to use it** — `/accelerator:stress-test-work-item [work item number or path]`

**Advice & guidelines** — Complements `review-work-item`: review applies fixed
quality lenses, stress-test interrogates *your* assumptions interactively.

### update-work-item

**What it does** — Update fields (status, priority, tags, parent, etc.) of an
existing work item.

**How to use it** — `/accelerator:update-work-item [work-item-ref] [field-op...]`

**Advice & guidelines** — Shows a diff preview and asks for confirmation. There
is no transition enforcement — arbitrary field changes are allowed.

### list-work-items

**What it does** — List and filter work items from the configured work
directory.

**How to use it** — `/accelerator:list-work-items [filter description]`

**Advice & guidelines** — Filters by status, type, priority, tag, parent, or
title; shows a colour-coded Sync column when a remote integration is configured.

### sync-work-items

**What it does** — Reconcile local work items in meta/work/ with the active
remote tracker named by work.integration.

**How to use it** — `/accelerator:sync-work-items [--push-only|--pull-only] [--preview] [--all] [filter-flags…]`

**Advice & guidelines** — Run `--preview` first to see what a sync would change
before any write reaches the tracker.
| **review-work-item**   | `/accelerator:review-work-item [work-item-ref]`                                               | Review a work item through completeness, testability, and clarity lenses                                                                              |

Work items use a shared template with YAML frontmatter (`work_item_id`, `title`,
`type`, `status`, `priority`, `parent`, `tags`) and structured body sections
(Summary, Context, Requirements, Acceptance Criteria, Open Questions,
Dependencies, Assumptions, Technical Notes, Drafting Notes, References).
The template is customisable via
`/accelerator:configure templates eject work-item`.
