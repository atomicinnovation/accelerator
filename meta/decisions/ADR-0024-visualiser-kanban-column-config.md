---
adr_id: ADR-0024
date: "2026-05-05T00:00:00+01:00"
author: Toby Clemson
status: proposed
tags: [visualiser, kanban, configuration]
---

# ADR-0024: Configurable Kanban Column Set

**Date**: 2026-05-05
**Status**: Proposed
**Author**: Toby Clemson

## Context

The visualiser's kanban view originally used a hardcoded set of status
columns derived from the seven values in `templates/work-item.md`. Different
projects use different status vocabularies — a project tracking stories in
Jira uses different states than one using the default accelerator templates.

Operators need to configure which columns the kanban renders without
changing code. The configuration must be validated at boot so a
misconfigured column set fails fast rather than producing a silently broken
kanban.

## Decision Drivers

- Projects use different status vocabularies; the accelerator's seven
  defaults are not universally appropriate.
- Drag-to-invalid-status must be caught server-side, not just frontend-side.
- The column set is read once per page-load (TanStack Query), so the server
  need not expose it on every request path.
- Boot-time validation is simpler and cheaper than runtime validation for a
  config value that never changes while the server is running.

## Considered Options

1. **Hardcoded seven defaults** — keep the existing fixed list.
2. **Configurable via `visualiser.kanban_columns`** — read from config at
   boot, validate, expose via API.
3. **Per-work-item `allowed_statuses` field** — each work item declares its
   own valid transitions.

## Decision

We will make the column set configurable via `visualiser.kanban_columns` in
`.claude/accelerator.md`, with the seven template-status defaults when the
key is absent, and a boot-time rejection when the list is empty or malformed.

### Config schema

`visualiser.kanban_columns` is an inline YAML array of column key strings.
The visualiser launcher reads the value via `config-read-value.sh`, splits
it with `config_parse_array`, and emits it to `config.json` as an ordered
list of strings. The server derives each column's display label by
title-casing the key (hyphens become spaces), so no separate label field is
required in this phase.

Default when absent: `[draft, ready, in-progress, review, done, blocked, abandoned]`.

### Boot-time validation rules

| Condition                      | Behaviour                           |
|--------------------------------|-------------------------------------|
| Key absent from config         | Fall back to seven defaults         |
| Key present, non-empty list    | Accept; each entry becomes a column |
| Key present, empty list (`[]`) | Reject at boot with a clear error   |
| Malformed inline-array syntax  | Reject at boot                      |

### API shape

A dedicated `GET /api/kanban/config` endpoint returns:

```json
{ "columns": [{ "key": "ready", "label": "Ready" }, ...] }
```

This is separate from `GET /api/types` because column layout is an
operator concern, not a doc-type metadata concern.

### PATCH validation

`PATCH /api/docs/{path}/frontmatter` validates the incoming `status` field
against the configured column keys. Any value not in the configured set —
including values currently in the "Other" swimlane (e.g. legacy `proposed`)
— returns 400:

```json
{ "error": "unknown_kanban_status", "acceptedKeys": ["ready", "in-progress", "done"] }
```

The `acceptedKeys` array is the live configured set, not the seven defaults.

### Other-swimlane write-blocked contract

Dragging a card out of the Other swimlane to a configured column succeeds.
Dragging a card back into Other (by PATCHing with a value not in the
configured set) is rejected with 400. Recovery from an accidental drag is a
direct file edit followed by VCS revert — consistent with the project's
"destructive op safety via VCS" convention.

### Boot-time immutability

The column set, like the work-item scan regex and `doc_paths.work`, is read
once at boot. Changing `visualiser.kanban_columns` requires a server restart.
Partial reload (reloading the browser without restarting the server) produces
no inconsistency for the column set because TanStack Query re-fetches
`/api/kanban/config` on every page-load — the frontend always reflects the
server's current config, which is stable across the server's lifetime.

## Consequences

### Positive

- Projects with non-standard status vocabularies can configure the kanban
  without forking the plugin.
- Boot-time rejection catches empty or malformed config before any HTTP
  requests are served.
- The seven-default fallback preserves zero-config behaviour for projects
  using the accelerator templates as-is.

### Negative

- Column set changes require a server restart; the operator must know this.
- The "Other" swimlane is permanent — values outside the configured set
  always fall there rather than producing an error at index time.

### Neutral

- Labels are title-cased from keys; custom label strings are out of scope.
- Per-column drag permissions and column reordering are out of scope.

## References

- `skills/visualisation/visualise/server/src/api/kanban_config.rs` — endpoint implementation
- `skills/visualisation/visualise/server/src/config.rs` — `KanbanColumn` struct and validation
- `skills/config/configure/SKILL.md` — `visualiser.kanban_columns` operator docs
- `templates/work-item.md:7` — canonical seven-status default list
