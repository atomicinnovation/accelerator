---
id: "0096"
title: "Templates View Auto-Discovers Available Templates"
date: "2026-06-02T12:11:27+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: story
priority: medium
tags: [visualiser, templates, frontend]
last_updated: "2026-06-11T13:17:54+00:00"
last_updated_by: Toby Clemson
schema_version: 1
type: work-item
relates_to: ["work-item:0042", "work-item:0089", "work-item:0029", "work-item:0037"]
---

# 0096: Templates View Auto-Discovers Available Templates

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a maintainer of the Accelerator template set, I want the templates view
(`/library/templates`) to discover available templates directly from the
`templates/` directory, so that
templates I add or remove appear in the view without separately editing a
hardcoded roster.

The view's list is currently driven by a hardcoded roster of eight template
names in the visualiser's config-generation step, while thirteen template files
exist on disk — so five templates never appear, and the list drifts further out
of sync each time a template is added. Discovery should instead be derived from
the contents of the `templates/` directory when the visualiser config is
generated.

## Context

The templates view (`/library/templates`) fetches its list at runtime from the
local visualiser server (`GET /api/templates`). The server does not scan the
`templates/` directory: its `TemplateResolver` enumerates the keys of a
`templates` map supplied in the generated server config. That map is produced by
the launcher script `write-visualiser-config.sh`, which lists template names as
a hardcoded roster of eight — `adr`, `plan`, `codebase-research`, `validation`,
`pr-description`, `work-item`, `design-gap`, `design-inventory`.

Thirteen `*.md` templates exist in `templates/`, so five — `plan-review`,
`pr-review`, `work-item-review`, `rca`, and `note` — can never appear in the
view. Adding a new template file has no effect until someone also edits the
hardcoded roster; that manual step is the drift this work item removes.

0042 designed the current view and is complete. 0096 does not change how the
view looks — it changes where the list of templates comes from, so the view
stays current as templates are added or removed over time.

## Requirements

- The visualiser config-generation step (`write-visualiser-config.sh`) derives
  the set of templates by scanning the plugin-default `templates/` directory for
  `*.md` files, rather than from a hardcoded roster. The `templates/` directory
  defines the full set of available templates.
- Each discovered template is wired into the generated config using the same
  three-tier path resolution as today (plugin-default, user-override,
  config-override), so the existing server → API → view chain renders it
  unchanged.
- The view continues to present the same per-template information it does today
  — name and tier-presence indicators — with no new metadata introduced.
- Discovery is performed at config-generation ("build") time: a newly added or
  removed template file is reflected the next time the visualiser config is
  generated (i.e. on next launch), not necessarily while the server is already
  running.

## Out of Scope

- Live/runtime hot-reload of templates while the server is running — discovery
  is build-time per the decision above.
- Config-override-only templates: a template present solely in the
  config-override directory (`.accelerator/templates/`) and not in `templates/`
  is not added to the set; `templates/` is the canonical roster.
- The separate hardcoded template list used by the config CLI tooling
  (`config-defaults.sh` `TEMPLATE_KEYS`) — that surface belongs to the
  template-management CLI (0029) and is a distinct drift.
- Any redesign of the view itself (covered by 0042) or any new per-template
  metadata.
- Extending the client-side glyph stem table (which maps a template's name stem
  — its base filename — to a glyph) — of the newly surfaced templates, only one
  (`rca`) lacks a glyph mapping and falls back to a blank glyph until mapped (see
  Technical Notes).

## Acceptance Criteria

- [ ] Given the visualiser config is generated, when `templates/` contains N
  `*.md` files, then the templates view lists all N templates — for the current
  on-disk set this means all thirteen appear, including `plan-review`,
  `pr-review`, `work-item-review`, `rca`, and `note`, which are absent today.
- [ ] Given a `templates/` directory containing exactly K `*.md` files (for K in
  {0, 1, 3, and the current on-disk count of 13}), when the config is generated,
  then the view lists exactly K templates — one row per file, with no dedup,
  reordering loss, or off-by-one — confirming the count mapping is one-to-one
  rather than a fixed roster.
- [ ] Given a new `*.md` file is added to `templates/`, when the visualiser
  config is regenerated (next launch), then the new template appears in the view
  without editing any hardcoded list.
- [ ] Given a template file is removed from `templates/`, when the config is
  regenerated, then it no longer appears in the view.
- [ ] Given a template present only in the config-override directory
  (`.accelerator/templates/`) and absent from `templates/`, when the config is
  generated, then it does not appear in the view (`templates/` is the canonical
  roster).
- [ ] Given a template present in plugin-default only and another present in
  plugin-default + user-override, when each is rendered in the view, then the
  tier-presence indicators show exactly those tiers lit for each (plugin-default
  / user-override / config-override) — confirming the indicators reflect actual
  tier membership, not merely that a template was discovered.
- [ ] No hardcoded template roster remains in `write-visualiser-config.sh` as
  the source of the view's list.

## Open Questions

- _(Resolved)_ Is every `*.md` file in `templates/` intended as a user-facing
  template, or is a name/frontmatter filter needed to exclude non-template files?
  Confirmed: `templates/` holds only user-facing templates and no non-template
  `.md` files will be placed there, so a bare `*.md` glob is sufficient — no
  name/frontmatter filtering is required.

## Dependencies

- Related: 0042 (templates view redesign — done; designed the view 0096 keeps
  current), 0089 (templates preview whitespace fix — same view surface), 0029
  (template management CLI — separate surface with its own hardcoded list).
- Coordination: 0089 touches the same view surface but changes presentation,
  whereas 0096 changes where the list comes from; the two are independent and
  need no ordering relative to each other.
- Follow-on: 0037 (Glyph component) — surfacing the hidden templates leaves
  `rca` with the blank-glyph fallback until a `STEM_TO_GLYPH` stem entry is
  added; tracked as a non-blocking nicety under 0037.
- Not blocked: 0042 is complete, so there is no blocking dependency.

## Assumptions

- "Build time" maps to the launcher's config-generation step
  (`write-visualiser-config.sh`). The visualiser has no separate compiled build
  for templates and the server reads template content at runtime, so
  config-generation is the natural discovery point; discovery therefore takes
  effect on config regeneration / next launch, not live.

## Technical Notes

**Size**: S — One script's config-assembly section (`write-visualiser-config.sh`)
restructured from eight fixed `template_tier` calls to a loop over the existing
`config_enumerate_templates` helper, plus splitting the `templates` object out of
the single `jq -n` so it can be built dynamically. Rust/frontend production code
is unchanged (both are already name-agnostic); only two test locations
(`config.rs:433`, `test-launch-server.sh:87-92`) need updating. The ready-made
glob helper and data-driven downstream chain keep this small; the jq restructure
is the only fiddly part.

- Source of truth today: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`,
  lines ~121-128 (the roster) and ~303-309 (the jq `templates` object). The
  `template_tier <name>` helper (~88-119) already resolves the three tier paths
  for a given name; discovery only needs to feed it the set of names found by
  scanning `templates/` instead of the hardcoded list.
- Downstream chain is unchanged: `write-visualiser-config.sh` → `config.json`
  (`Config.templates`, `server/src/config.rs:29`) → `TemplateResolver::build/list`
  (`server/src/templates.rs:115-232`) → `GET /api/templates`
  (`server/src/api/templates.rs`) → `fetchTemplates`
  (`frontend/src/api/fetch.ts:125`) → `TemplatesIndexList`
  (`frontend/src/routes/library/LibraryTemplatesIndex.tsx:76`). No frontend
  change is required.
- Per-template wire shape is `TemplateSummary { name, tiers[], activeTier }`
  (`frontend/src/api/types.ts:135-168`); no new metadata.
- Glyph caveat: the per-template glyph is derived client-side from the name stem
  via `STEM_TO_GLYPH` in `frontend/src/routes/library/template-tier.ts:31-79`; a
  discovered name with no stem entry renders the blank-glyph fallback
  (`LibraryTemplatesIndex.tsx:95-96`). Of the five currently-hidden templates,
  only `rca` lacks a `STEM_TO_GLYPH` entry (`template-tier.ts:31-59`) and renders
  the blank fallback; `plan-review`, `pr-review`, `work-item-review`, and `note`
  already resolve (to `plan-reviews`, `pr-reviews`, `work-item-reviews`,
  `notes`). Adding the one missing `rca` stem entry so it shows a glyph is a
  follow-on nicety, not a blocker (relates to 0037, Glyph component).
- Prior art for directory scanning: the server's docs indexer/watcher
  (`server/src/indexer.rs`, `docs.rs`, `watcher.rs`) already scans directories
  at runtime — relevant only if a runtime approach were ever chosen instead of
  build-time.
- A directory-scan helper already exists and is already sourced by the launcher
  (`scripts/config-common.sh:139-149`, pulled in at `write-visualiser-config.sh:6`):
  `config_enumerate_templates <plugin-root>` globs `*.md` under
  `<plugin-root>/templates` with the repo's bash 3.2-safe idiom (`for f in
  "$dir"/*.md; do [ -f "$f" ] || continue; basename "$f" .md; done` — no
  `nullglob`/`globstar`). It appends `/templates` itself, so it scans the same
  directory as `TEMPLATES_PLUGIN_ROOT` (~line 86). Sibling scripts
  `config-list-template.sh:21` and `config-eject-template.sh:121` already drive
  enumeration this way. Discovery is therefore "feed `template_tier` the names
  this helper returns" instead of the eight fixed calls — minimal new code.
- The one structurally non-trivial change is the jq assembly. The `templates`
  object literal (~lines 303-309) sits inside a single `jq -n` (~lines 256-315)
  that builds the entire config from statically-named `--argjson` args; a
  variable-length template set cannot be named statically. The templates object
  must be built separately (accumulate name→tier-JSON pairs into one object,
  capture into a var, splice in via a single `--argjson templates`).
- Tests pin the eight-name roster and must be updated alongside the change:
  `server/src/config.rs:433` asserts `templates.len() == 8`, and
  `test-launch-server.sh:87-92` asserts specific per-name `user_override` paths.
  Both will break for the discovered set (thirteen today). The launcher's own
  `test-write-visualiser-config.sh` does not currently assert on the roster.

## Drafting Notes

- Interpreted "build time" as the launcher config-generation step
  (`write-visualiser-config.sh`), since the visualiser has no separate compiled
  build for templates and the server reads them at runtime. Consequence:
  discovery takes effect on regeneration / next launch, not live.
- Scoped discovery to the plugin-default `templates/` directory only (per the
  author's decision); config-override-only templates in `.accelerator/templates/`
  are deliberately excluded so `templates/` remains the canonical set.
- Scoped 0096 to the templates view's source of truth only; deliberately
  excluded the config CLI's separate hardcoded list (`config-defaults.sh`),
  even though it exhibits the same drift, to keep the item to one concern (it
  belongs to the 0029 CLI surface).
- Treated the glyph stem-table extension as an out-of-scope follow-on; a newly
  discovered template still "appears" (satisfying the acceptance criteria) even
  with the blank-glyph fallback.

## References

- Related: 0042, 0089, 0029, 0037
- Key code: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`,
  `skills/visualisation/visualise/server/src/templates.rs`,
  `skills/visualisation/visualise/server/src/config.rs`,
  `skills/visualisation/visualise/server/src/api/templates.rs`,
  `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx`,
  `skills/visualisation/visualise/frontend/src/routes/library/template-tier.ts`
