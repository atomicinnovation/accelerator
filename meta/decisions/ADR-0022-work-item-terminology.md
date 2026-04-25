---
adr_id: ADR-0022
date: "2026-04-25T22:30:00+01:00"
author: Toby Clemson
status: accepted
tags: [terminology, work-items, naming]
---

# ADR-0022: Use `work`/`work-item` terminology

**Date**: 2026-04-25
**Status**: Accepted
**Author**: Toby Clemson

## Context

The plugin shipped a `tickets` skill category (v1.18.0 Unreleased) without a
documented alternatives review. The term "ticket" carries a service-desk
connotation (Jira, Zendesk, Help Scout) that does not match the plugin's actual
use case: managing product-development work items within a software project.
The research document `meta/research/2026-04-25-rename-tickets-to-work-items.md`
examined the vocabulary in depth and identified the conflict.

The surface is pre-release (the `tickets` category has never appeared in a
versioned CHANGELOG entry), so a clean rename without backwards-compatibility
shims is viable — following the same precedent as the v1.9.0 `initialise` →
`init` rename.

The choice of category name also has a consistency constraint: the config key
that sets the storage directory is `paths.<category>`, so the category name
directly determines the config key (`paths.work`) and the default storage
directory (`meta/work/`). A category name that collides with other vocabulary
in the template or the review system creates ambiguity.

## Decision Drivers

- Remove service-desk connotation from vocabulary used across slash commands,
  config keys, storage directories, and template frontmatter
- Align category name with config keys and storage directory name
  (`paths.work` → `meta/work/`)
- Avoid collision with existing template type enum values (`story`, `epic`,
  `task`, `bug`, `spike`) and review mode names (`pr`, `plan`)
- Prefer a term that reads naturally in both singular and plural forms across
  slash commands

## Considered Options

1. **`work` / `work-item`** — neutral product-development vocabulary; category
   name `work` aligns with `paths.work`; `work-item` hyphenated for identifiers,
   two-word "work item" for prose
2. **`task` / `task`** — rejected: `task` is an existing value in the template
   `type` enum (`story | epic | task | bug | spike`); collision would make
   `paths.task` ambiguous between "the task category" and "task-type items"
3. **`story` / `story`** — rejected: same collision problem; `story` is also a
   `type` enum value
4. **`backlog-item` / `backlog-item`** — rejected: verbose; implies a specific
   workflow (backlog grooming) that not all teams use
5. **`issue` / `issue`** — rejected: collides with GitHub Issues vocabulary and
   the common use of "issue" for bugs/defects specifically
6. **`item` / `item`** — rejected: too generic; likely to collide with other
   future vocabulary in the plugin or in user documentation
7. **`card` / `card`** — rejected: kanban-tool specific; excludes teams using
   non-kanban workflows
8. **Retain `ticket` / `ticket`** — rejected: service-desk framing; conflicts
   with the plugin's product-development context

## Decision

Use `work` as the skill category name and `work-item` as the item type name.

- Skill category directory: `skills/work/`
- Slash commands: `/accelerator:create-work-item`, `/accelerator:extract-work-items`,
  `/accelerator:list-work-items`, `/accelerator:refine-work-item`,
  `/accelerator:review-work-item`, `/accelerator:stress-test-work-item`,
  `/accelerator:update-work-item`
- Default storage directory: `meta/work/`
- Default review storage directory: `meta/reviews/work/`
- Config keys: `paths.work` (default `meta/work`),
  `paths.review_work` (default `meta/reviews/work`)
- Review mode literal: `work-item`
- Template: `templates/work-item.md` with frontmatter field `work_item_id:`
- Prose: "work item" (two words, with space); identifiers/paths/keys:
  "work-item" (hyphenated)

Pluralisation follows existing precedent: plural for bulk operations
(`extract-work-items`, `list-work-items`), singular for per-item operations
(`create-work-item`, `refine-work-item`, `update-work-item`,
`review-work-item`, `stress-test-work-item`).

## Consequences

### Positive

- Vocabulary is independent of specific issue-tracking tools (Jira, Linear,
  GitHub Issues, Shortcut) — the plugin works alongside any of them
- Category name `work` aligns directly with config key `paths.work` and
  storage directory `meta/work/`, making the relationship self-evident
- No collision with existing template `type` enum values or review mode names
- Users new to the plugin will not have pre-existing associations that conflict
  with the actual behaviour

### Negative

- `work-item` adds five characters and a hyphen to every slash command compared
  to `ticket`. The typing friction is judged acceptable because slash commands
  are auto-completed in Claude Code and are read more often than typed
- Existing repos using the pre-release `tickets` category must run
  `/accelerator:migrate` to apply migration `0001-rename-tickets-to-work`
  (automated; see ADR-0023)

### Neutral

- The hyphen/space distinction (identifiers vs. prose) requires a format-check
  guard to enforce consistently (added in Phase 3.10 of the implementation plan)

## References

- `meta/research/2026-04-25-rename-tickets-to-work-items.md` — alternatives
  review and naming rationale
- `meta/research/2026-04-08-ticket-management-skills.md` — original ticket
  category design (uses "ticket" without documented alternatives review)
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — migration
  framework that automates the rename for existing repos
- `CHANGELOG.md` — Unreleased section v1.9.0 precedent for unreleased-surface
  clean renames without shims
