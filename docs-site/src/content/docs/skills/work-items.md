---
title: Work Items
---

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
[`skills/config/configure/SKILL.md`](https://github.com/atomicinnovation/accelerator/blob/main/skills/config/configure/SKILL.md#work)
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

<a id="create-work-item"></a>

### <img src="https://api.iconify.design/ph/file-plus-bold.svg?color=%230d9488" width="18" align="center" alt=""> `/create-work-item [topic or existing work item path/number]`

Interactively create a well-formed work item. Can also enrich an existing one
when given its path or number.

*Pair with `refine-work-item` to decompose and sharpen the draft before planning
begins.*

<a id="extract-work-items"></a>

### <img src="https://api.iconify.design/ph/export-bold.svg?color=%230d9488" width="18" align="center" alt=""> `/extract-work-items [document paths...]`

Extract work items in batch from existing documents (specs, PRDs, research,
plans, meeting notes, design docs).

*Leave the paths empty to scan all documents.*

<a id="refine-work-item"></a>

### <img src="https://api.iconify.design/ph/sliders-horizontal-bold.svg?color=%230d9488" width="18" align="center" alt=""> `/refine-work-item [work item number or path]`

Interactively refine a work item by decomposing it into children, enriching it
with codebase context, sharpening its acceptance criteria, sizing it, or linking
it to dependencies.

*Use after a work item is drafted and before planning begins.*

<a id="review-work-item"></a>

### <img src="https://api.iconify.design/ph/binoculars-bold.svg?color=%230d9488" width="18" align="center" alt=""> `/review-work-item [path to work item file]`

Review a work item through multiple quality lenses and collaboratively iterate
based on findings.

*Runs the multi-lens [Review System](review-system.md) (completeness,
testability, clarity); see that page for the lens catalogue.*

<a id="stress-test-work-item"></a>

### <img src="https://api.iconify.design/ph/barbell-bold.svg?color=%230d9488" width="18" align="center" alt=""> `/stress-test-work-item [work item number or path]`

Interactively stress-test a work item by grilling the user on scope, assumptions,
acceptance criteria, edge cases, and dependencies to surface issues, gaps, and
flawed assumptions before implementation is planned.

*Complements `review-work-item`: review applies fixed quality lenses, stress-test
interrogates **your** assumptions interactively.*

<a id="update-work-item"></a>

### <img src="https://api.iconify.design/ph/pencil-bold.svg?color=%230d9488" width="18" align="center" alt=""> `/update-work-item [work-item-ref] [field-op...]`

Update fields (status, priority, tags, parent, etc.) of an existing work item.

*Shows a diff preview and asks for confirmation. There is no transition
enforcement — arbitrary field changes are allowed.*

<a id="list-work-items"></a>

### <img src="https://api.iconify.design/ph/list-bold.svg?color=%230d9488" width="18" align="center" alt=""> `/list-work-items [filter description]`

List and filter work items from the configured work directory.

*Filters by status, type, priority, tag, parent, or title; shows a colour-coded
Sync column when a remote integration is configured.*

<a id="sync-work-items"></a>

### <img src="https://api.iconify.design/ph/arrows-clockwise-bold.svg?color=%230d9488" width="18" align="center" alt=""> `/sync-work-items [options]`

Reconcile local work items in meta/work/ with the active remote tracker named by
work.integration.

*Run with `--preview` first to see what a sync would change before any write
reaches the tracker. Flags: `--push-only`, `--pull-only`, `--preview`, `--all`,
plus filter flags.*

Work items use a shared template with YAML frontmatter (`work_item_id`, `title`,
`type`, `status`, `priority`, `parent`, `tags`) and structured body sections
(Summary, Context, Requirements, Acceptance Criteria, Open Questions,
Dependencies, Assumptions, Technical Notes, Drafting Notes, References).
The template is customisable via
`/configure templates eject work-item`.
