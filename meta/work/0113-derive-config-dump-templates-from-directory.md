---
type: work-item
id: "0113"
title: "Derive the Config Dump's Template Rows From the Templates Directory"
date: "2026-06-17T11:39:08+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: low
relates_to: ["work-item:0096", "work-item:0029", "work-item:0067", "codebase-research:2026-06-17-readme-changelog-1.22.0-refresh"]
tags: [templates, configure, config-dump]
last_updated: "2026-06-17T11:39:08+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0113: Derive the Config Dump's Template Rows From the Templates Directory

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

`config-dump.sh` renders its template-key rows by iterating the hardcoded
`TEMPLATE_KEYS` array (6 entries) in `scripts/config-defaults.sh`. Every other
template surface — `/accelerator:configure templates list/show/eject/diff/reset`
and `config_resolve_template` — is driven by the **contents of the plugin's
`templates/` directory** instead. Make the config dump derive its rows the same
way, so it auto-syncs with the shipped templates and the separate registry stops
drifting.

## Context

There are 13 shipped templates (`templates/*.md`) but `TEMPLATE_KEYS` lists only
6 (`plan`, `codebase-research`, `adr`, `validation`, `pr-description`,
`work-item`). The other 7 — `note`, `rca`, `design-inventory`, `design-gap`,
`plan-review`, `pr-review`, `work-item-review` — are fully listable, ejectable,
diffable, resettable, and overridable (via a `templates.<key>` config key),
because those operations resolve by filename through
`config_enumerate_templates` / `config_resolve_template`
(`scripts/config-common.sh`). They are simply
**absent from the config dump**, which is the only runtime consumer of
`TEMPLATE_KEYS` (`scripts/config-dump.sh:185`).

This supersedes the original framing of this work item (register `note` in
`TEMPLATE_KEYS`). The better fix is to remove the second source of truth
altogether: derive the dump's rows from the directory, exactly as the
visualiser's Templates view already does (work item 0096) and as the
`configure templates` subcommands already do.

## Requirements

- Replace the `TEMPLATE_KEYS` loop in `config-dump.sh` with a loop over
  `config_enumerate_templates "$PLUGIN_ROOT"`, mapping each discovered template
  name to its `templates.<name>` config key for value and source attribution.
- The dump must list **every** template present in `templates/` (currently 13),
  each with its effective value and source (team / local / default), and
  `*(not set)*` when there is no override — matching the current per-row format.
- `TEMPLATE_KEYS` has no remaining consumer once the dump is converted; remove it
  from `scripts/config-defaults.sh` (and the tests that pin it), unless a
  consumer is found, in which case justify keeping it.

## Acceptance Criteria

- [ ] `config-dump.sh` derives its template rows from the templates directory
      (`config_enumerate_templates`), not a hardcoded list.
- [ ] All 13 shipped templates appear in the dump output, including `note`,
      `rca`, `design-inventory`, `design-gap`, `plan-review`, `pr-review`, and
      `work-item-review`.
- [ ] Per-row source attribution still works: a `templates.<key>` override in
      team/local config is reflected with the correct source; unset rows render
      `*(not set)*`.
- [ ] Adding or removing a `templates/*.md` file changes the dump output with no
      edit to any registry array (covered by a test).
- [ ] `TEMPLATE_KEYS` is removed from `scripts/config-defaults.sh` (no remaining
      consumer), and `scripts/test-config.sh`'s `TEMPLATE_KEYS` length/contents
      and sole-definition assertions are removed or replaced with a
      directory-derivation test.
- [ ] `mise run check` and the shell config suite pass.

## Open Questions

- Should the dump impose a stable display order (e.g. alphabetical) rather than
  raw glob order, for diff-stable output across machines and shells?
- Should the dump flag a `templates.<key>` override whose target file is missing,
  or is that out of scope here?

## Technical Notes

- Consumer to change: `scripts/config-dump.sh:185` (the `TEMPLATE_KEYS` loop).
- Directory enumeration already exists: `config_enumerate_templates`
  (`scripts/config-common.sh:349`), used by `config-list-template.sh` and the
  other `configure templates` scripts.
- Tests to update: `scripts/test-config.sh:2460-2465` (length/contents) and
  `:2519-2524` (the sole-definition guard, which lists `TEMPLATE_KEYS`).
- Precedent: work item 0096 made the visualiser's Templates view auto-discover
  from `templates/`; this applies the same principle to the config dump.

## Drafting Notes

- Classified as a **task** (a small, well-scoped refactor that removes a
  drift-prone second registry) rather than a bug.
- Replaces the earlier "register `note` in `TEMPLATE_KEYS`" framing, which was
  based on the mistaken premise that `TEMPLATE_KEYS` gates template
  customisation. It does not — it only feeds the config dump.
- Priority **Low**: no user-facing breakage today; the win is correctness and
  removing a maintenance footgun.

## References

- Related: 0096 (templates view auto-discovery — same derive-from-directory
  principle), 0029 (template management subcommands), 0067 (create-note skill,
  whose `note` template first exposed the gap)
- Research: `meta/research/codebase/2026-06-17-readme-changelog-1.22.0-refresh.md`
