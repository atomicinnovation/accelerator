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
calls. See [Issue Trackers](issue-trackers.mdx) for the integrations.

With `work.id_pattern: "{tracker}"` the tracker owns each item's `id`: an
item pushed at creation is named by its Jira or Linear key, and one no
tracker has confirmed is a **draft** in `meta/work/drafts/` under a
`draft-xxxxxx` ID. `sync-work-items` promotes drafts onto their keys by
default, and `accelerator work promote <draft-id>` promotes one. See
[Let the tracker own IDs](../guides/sync-work-items.mdx#let-the-tracker-own-ids).

```
existing docs (specs, PRDs, notes)
       │
       ├── extract-work-items ──┐
       │                     ↓
       create-work-item ──→  meta/work/  ←── update-work-item
                              │
                              ├── list-work-items ──┬──→  review-work-item → meta/reviews/work/
                              │                     └──→  create-plan → implement-plan
                              ├── sync-work-items ⇄ remote tracker (Jira/Linear)
                              └── rekey-work-items → IDs on work.id_pattern
```

## Capturing work

Two ways in:
[`create-work-item`](../reference/skills/work/create-work-item.md)
interactively drafts a single well-formed item (and can enrich an
existing one), while
[`extract-work-items`](../reference/skills/work/extract-work-items.md)
mines a batch from existing documents — specs, PRDs, research, plans,
meeting notes, design docs. Under `{tracker}` it creates the batch
through `accelerator work create-batch` after one push offer, linking each
child to its parent's key or draft.

Pass `--id` to `create-work-item` to choose an item's ID yourself, for
example when other branches have unmerged items:

```
/create-work-item --id PP-0400 Add a retry budget to sync
```

The ID must be in the exact shape `work.id_pattern` renders — `PP-0400`
under `{key}-{number:04d}`, `0400` under the default `{number:04d}` — with
a number of at least 1, no `draft-` prefix and no other item holding it.
The skill checks it with `accelerator work id check` before drafting: an
exact ID is used as given, and one that needs padding or names a key
other than `work.key` is proposed in full for you to confirm. `--id` is
refused under `{tracker}`, where the tracker owns every ID, and beside
`--project`. Run `accelerator work id check <ID>` yourself to see what an
ID would become without creating anything.

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
`--preview` first. After changing `work.id_pattern` or `work.key`,
[`rekey-work-items`](../reference/skills/work/rekey-work-items.md)
previews and then gives every existing item an ID in the new shape,
renaming files and rewriting references across `meta/`; the old ID joins
`aliases`. Under a `{key}` pattern every item keeps its number and its
key; the preview names items under keys other than `work.key` and asks
which keys' items to move onto it, as after `work.key` changes. Until
every item is in the pattern's shape, `work create`, `create-batch`,
`id next`, `id check`, `resolve` and `promote` refuse with
`E_REKEY_REQUIRED`, and a file whose IDs cannot be read, such as a
slugless `0400.md`, with `E_WORK_ITEM_MALFORMED`; each names its fix.
`work sync` and `work rekey` still run, so the way out stays open, and
`work list` lists the items with the refusal as a warning.

Work items share a template with YAML frontmatter (`work_item_id`,
`title`, `type`, `status`, `priority`, `parent`, `tags`) and structured
body sections; see
[The meta/ directory](../reference/meta-directory.md) for the status
lifecycle, and customise the template via
`/configure templates eject work-item`.
