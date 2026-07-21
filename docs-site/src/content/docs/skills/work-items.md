---
title: Work Items
---

Work item skills capture work — features, bugs, tasks, spikes, and
epics — as structured documents in `meta/work/` that feed into planning.
The filename prefix defaults to a 4-digit number
(`meta/work/0042-add-search.md`) but is configurable via
`work.id_pattern` and `work.default_project_code`. The
`work.integration` key selects an optional remote tracker (`jira` or
`linear` today); when unset, everything stays local with no external API
calls. See [Issue Trackers](issue-trackers.md) for the integrations.

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

## Capturing work

Two ways in:
[`create-work-item`](../reference/skills/work/create-work-item.md)
interactively drafts a single well-formed item (and can enrich an
existing one), while
[`extract-work-items`](../reference/skills/work/extract-work-items.md)
mines a batch from existing documents — specs, PRDs, research, plans,
meeting notes, design docs.

## Sharpening before planning

After drafting and before planning:

- [`refine-work-item`](../reference/skills/work/refine-work-item.md)
  decomposes an item into children, enriches it with codebase context,
  sharpens acceptance criteria, sizes it, or links dependencies.
- [`review-work-item`](../reference/skills/work/review-work-item.md)
  applies the [Review System](review-system.md) lenses (completeness,
  testability, clarity, scope, dependency).
- [`stress-test-work-item`](../reference/skills/work/stress-test-work-item.md)
  interrogates **your** assumptions interactively — scope, acceptance
  criteria, edge cases, dependencies. Complements review the same way
  `stress-test-plan` complements `review-plan`.

## Managing the backlog

[`list-work-items`](../reference/skills/work/list-work-items.md) filters
by status, kind, priority, tag, parent, or title;
[`update-work-item`](../reference/skills/work/update-work-item.md)
changes any frontmatter field with a diff preview (no transition
enforcement); and
[`sync-work-items`](../reference/skills/work/sync-work-items.md)
reconciles `meta/work/` with the configured remote tracker — run it with
`--preview` first.

Work items share a template with YAML frontmatter (`work_item_id`,
`title`, `type`, `status`, `priority`, `parent`, `tags`) and structured
body sections; see
[The meta/ directory](../reference/meta-directory.md) for the status
lifecycle, and customise the template via
`/configure templates eject work-item`.
