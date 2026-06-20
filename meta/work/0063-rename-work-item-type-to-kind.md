---
id: "0063"
title: "Rename work-item `type:` Field to `kind:`"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: story
status: done
priority: high
parent: "work-item:0057"
tags: [refactor, work-item, schema, breaking-change]
type: work-item
schema_version: 1
last_updated: "2026-05-17T17:16:35+00:00"
last_updated_by: Toby Clemson
blocks: ["work-item:0065", "work-item:0070"]
relates_to: ["adr:ADR-0023", "work-item:0023", "work-item:0057", "work-item:0060"]
external_id: PP-85
---

# 0063: Rename work-item `type:` Field to `kind:`

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a plugin maintainer, I want the work-item frontmatter field `type:` (currently
holding the semantic kind — `story | epic | task | bug | spike`) renamed to `kind:`
across templates, helpers, work and review skills, the visualiser frontend, eval
fixtures and graders, and every existing work-item file, so that the unified-schema
work can repurpose `type:` as the artifact-type discriminator without indefinite
workarounds. Per epic 0057, this rename is coordinated in a single story because of
its disruption surface. The rename ships with its own migration, independent of the
broader unified-schema migration (0070).

## Context

The unified-schema work (epic 0057) needs every artifact type to carry a uniform
`type:` field naming its artifact-type discriminator (`work-item`, `plan`, `adr`,
etc.). Work-items currently overload `type:` with their semantic kind, blocking the
discriminator from being applied uniformly. Renaming to `kind:` resolves the
collision once and for all rather than working around it indefinitely.

The rename is the single most disruptive change in epic 0057 — it touches the
template, work-skill helpers, every work-skill SKILL.md, the review lenses that
branch on work-item kind, the visualiser kanban card, eval fixtures and graders,
and every existing work-item file. All of these are coordinated in this single
story.

## Requirements

- Update `templates/work-item.md` to use `kind:` in place of `type:`, with the same
  accepted vocabulary (`story | epic | task | bug | spike`). Update the body
  `**Type**:` label accordingly.
- Update work-skill helper scripts that reference `type` as a field name —
  at minimum `skills/work/scripts/work-item-template-field-hints.sh` (which has a
  hardcoded `case "$field" in type)` fallback returning the five kind values), plus
  any other helper that hardcodes the field name. The field-name-agnostic readers
  (`work-item-read-field.sh`, `work-item-resolve-id.sh`, etc.) require no code
  change but their callers must pass `kind` instead of `type`.
- Update every work-skill `skills/work/*/SKILL.md` that references the field:
  `create-work-item`, `refine-work-item`, `list-work-items`, `update-work-item`,
  `review-work-item`, `extract-work-items`, `stress-test-work-item`. The body
  `**Type**:` label sync rule in `update-work-item` also updates to `**Kind**:`.
- Update review-lens `skills/review/lenses/*/SKILL.md` files that branch on the
  work-item kind: `completeness-lens`, `scope-lens`, `dependency-lens`,
  `testability-lens`.
- Update the visualiser frontend:
  `skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx`
  reads `entry.frontmatter['type']` as a literal key and renders a `typeLabel` chip
  — switch to `frontmatter['kind']` and rename the variable accordingly. Update
  the component's tests in the same directory to match. The visualiser's Rust
  backend (under `skills/visualisation/visualise/`) reads frontmatter fields
  generically and needs no change.
- Update eval fixtures and graders that match on the work-item `type:` field —
  roughly 100 fixture work-item files across the work-skill and review-lens eval
  suites, plus several `evals.json` / `benchmark.json` graders that assert on the
  field name or its values.
- Author the rename migration (0005) at
  `skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh`,
  modelled on `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh:53-69`.
  Migration 0005 must:
  - Resolve the work-items directory via `bash "$PLUGIN_ROOT/scripts/config-read-path.sh" work`
    so userspace overrides (`.accelerator/config.md`, `.accelerator/config.local.md`,
    `paths.work`) are honoured.
  - Iterate `*.md` under that directory; for each file:
    - In the YAML frontmatter, rename `^type:` to `^kind:` using `atomic_write`
      from `scripts/atomic-common.sh`.
    - In the body, rewrite every line matching `^\*\*Type\*\*:` to
      `^\*\*Kind\*\*:` unconditionally — the rewrite is not filtered on the
      value, so any quoted or example `**Type**:` line is also rewritten. This
      matches the renamed template and avoids the migration needing to
      distinguish kind-bearing labels from prose mentions.
  - Be idempotent: skip files that already have `kind:` and no `type:`, and
    skip body rewrites where the label is already `**Kind**:`. If both
    `type:` and `kind:` are present (partial prior run), drop the stale
    `type:` line.
  - Follow framework conventions (`#!/usr/bin/env bash`, `# DESCRIPTION:` line,
    `set -euo pipefail`, source `scripts/config-common.sh` and
    `scripts/atomic-common.sh`, resolve `PROJECT_ROOT` via `config_project_root`).
    The driver appends to `.accelerator/state/migrations-applied` automatically —
    migration 0005 must not touch that file directly.
- Apply migration 0005 in this repo to rename the field and body label in all
  72 existing work-items under `meta/work/`.

### Sequencing

The producer rename, migration authoring, and corpus rewrite must land in a
single atomic delivery. The internal order within that delivery is:

1. Author migration 0005 (frontmatter + body-label rewrite, idempotent).
2. Apply migration 0005 in this repo so `meta/work/*.md` carries `kind:` in
   frontmatter and `**Kind**:` in body labels.
3. Land producer-code changes (template, helpers, work/review skills,
   visualiser frontend), eval-fixture rewrites, and grader updates in the same
   commit/PR as the migration so no consumer reads a `type:` frontmatter line
   from a producer that has already moved to `kind:`. Producer code MUST NOT
   land before step 2 completes, otherwise existing work-items become
   unreadable.

## Acceptance Criteria

- [ ] `templates/work-item.md` uses `kind:` for the semantic kind, with the same
  vocabulary. The body `**Type**:` label is updated to `**Kind**:`.
- [ ] No remaining references to the work-item semantic kind as `type:` in
  production code, prose, or eval fixtures. The following deterministic greps
  return zero hits — false positives are filtered mechanically, not by manual
  inspection:
  - Frontmatter / prose field-value combination:
    ```
    rg -n '(?:^|[^.\w])type:\s*(story|epic|task|bug|spike)\b' \
      | rg -v 'work-item-review|subagent_type'
    ```
  - Visualiser kanban literal-key access:
    ```
    rg -n "frontmatter\[['\"]type['\"]\]|frontmatter\.type" \
      skills/visualisation/visualise/frontend/src/routes/kanban/
    ```

  The first grep's `rg -v` step removes the two enumerable false-positive
  forms (`type: work-item-review` review-artifact frontmatter; `subagent_type`
  parameters). Other documented false-positive classes (`entry.type` /
  `params.type` doc-type infrastructure, TypeScript `type` keywords) are
  excluded by the field-value pattern itself, since they do not match the
  kind-vocabulary suffix.
- [ ] `work-item-template-field-hints.sh` accepts `kind` as the field name where
  the semantic kind is queried; the hardcoded `type)` case is renamed.
- [ ] All seven work-skill SKILL.md files and the four affected review-lens
  SKILL.md files describe `kind:` in their templates, examples, and instructions.
- [ ] `WorkItemCard.tsx` reads `frontmatter['kind']`; its component tests are
  updated to match.
- [ ] All eval fixture work-item files use `kind:`, and every grader asserting
  on the work-item field name or value is updated. Verified by:
  - `rg -n '^type:\s*(story|epic|task|bug|spike)\b' skills/work skills/review/lenses`
    returns zero hits across fixtures.
  - `rg -n '"type"\s*:\s*"(story|epic|task|bug|spike)"' -g '*evals.json' -g '*benchmark.json'`
    returns zero hits across grader files.
- [ ] Migration `skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh`
  exists, is idempotent, honours `paths.work` userspace overrides, and renames
  `type:` → `kind:` (frontmatter) and `**Type**:` → `**Kind**:` (body label)
  in every work-item file under the resolved work-items directory.
- [ ] Migration 0005 honours `paths.work` overrides: with
  `paths.work: <custom-path>` configured in `.accelerator/config.md` and a
  work-item containing `type: story` under that directory, running migration
  0005 renames the frontmatter and body label in that file. Default
  `meta/work` is left untouched if `paths.work` redirects elsewhere and no
  default-path work-items exist.
- [ ] Running `/accelerator:migrate` applies migration 0005 in this repo;
  re-running `/accelerator:migrate` exits 0 with the driver reporting
  migration 0005 as already applied and `jj status` showing no further
  changes under `meta/work/`.
- [ ] After migration 0005 runs in this repo, `meta/work/*.md` has zero
  `^type:` frontmatter lines, zero `^\*\*Type\*\*:` body-label lines (the body
  rewrite is unconditional on the regex), and every file has a `^kind:` line
  and (where one was present before) a `^\*\*Kind\*\*:` body label.
- [ ] Work-skill and review-lens eval suites pass on the renamed fixtures.
  Pass condition: the eval-suite pass rate is unchanged from the pre-rename
  baseline captured immediately before this story's producer-code changes
  are staged; any new failure attributable to the rename must be enumerated
  and explicitly attributed.

## Dependencies

- Depends on: ADR-0023 migration framework — the driver, state-file append,
  dirty-tree guard, preview banner, and migration discovery contract owned by
  `skills/config/migrate/scripts/run-migrations.sh` must remain stable for
  migration 0005 to execute correctly.
- Depends on: `paths.work` config-resolution layer
  (`scripts/config-read-path.sh`, `.accelerator/config.md` /
  `.accelerator/config.local.md` schema) — the migration relies on this to
  honour userspace overrides of the work-items directory.
- Blocks: 0065 (template updates).
- Blocks: 0070 — its migration must not include a `type:` → `kind:` rewrite
  step; the rewrite is owned by migration 0005 authored in this story. 0070
  still covers the other unified-schema changes (adding the artifact-type
  discriminator, etc.).
- Related: 0057 (parent epic), 0060 (base schema ADR — completed; decided the
  unified `type:` discriminator vocabulary this rename unblocks),
  0070 (broader unified-schema migration, beyond the field rename).

## Assumptions

- The semantic-kind vocabulary itself does not change — only the field name does.
- Userspace customisations of work-skills follow the same field-name conventions
  as the plugin defaults; the standard config scripts used by the migration
  reach userspace work-items via `paths.work` overrides.

## Technical Notes

- Migration framework (ADR-0023): migrations are single bash files at
  `skills/config/migrate/migrations/NNNN-<slug>.sh`, discovered by glob and
  ordered by numeric prefix. The canonical model for a frontmatter-key rename is
  `migrations/0001-rename-tickets-to-work.sh:53-69`.
- Required migration structure: `#!/usr/bin/env bash` shebang, `# DESCRIPTION:`
  line (parsed for the preview banner), `set -euo pipefail`, source
  `scripts/config-common.sh` and `scripts/atomic-common.sh`, resolve
  `PROJECT_ROOT` via `config_project_root` fallback.
- Config resolution: `bash "$PLUGIN_ROOT/scripts/config-read-path.sh" work`
  honours `paths.work` overrides from `.accelerator/config.md` and
  `.accelerator/config.local.md` (default `meta/work`).
- Atomicity: use `atomic_write` from `scripts/atomic-common.sh` for all rewrites.
- Framework concerns the migration must NOT implement: dirty-tree guard, state
  file append, preview mode (`DRY_RUN`), skip handling. These are owned by
  `skills/config/migrate/scripts/run-migrations.sh`.
- Affected surface (corrected against epic 0057's technical-notes):
  - `templates/work-item.md`
  - `skills/work/scripts/work-item-read-field.sh`,
    `skills/work/scripts/work-item-resolve-id.sh`,
    `skills/work/scripts/work-item-template-field-hints.sh`
  - All seven `skills/work/*/SKILL.md`
  - Four `skills/review/lenses/{completeness,scope,dependency,testability}-lens/SKILL.md`
  - `skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx`
    and its co-located tests
  - ~100 eval fixture work-item files across work-skill and review-lens evals
  - Several `evals.json` / `benchmark.json` graders that match on the field name
  - The 72 existing work-items under `meta/work/` (migrated automatically by
    the new migration script)
- **No agent prompts under `agents/` are affected**, despite the parent epic's
  technical-notes claim — every `type` reference in `agents/*.md` is unrelated
  (TypeScript types, HTML input types, `subagent_type` parameters).
- False-positive exclusions for verification greps: `type: work-item-review`
  (review-artifact's own frontmatter type), `entry.type` / `params.type` in
  visualiser doc-type infrastructure, `subagent_type` parameters, TypeScript
  `type` keywords.

## Drafting Notes

- Kept this as a single story per the parent epic's explicit "Coordinate this
  rename in a single story" instruction.
- Acceptance-criteria grep originally proposed in epic 0057
  (`grep -r "type:" skills/work`) was too coarse — replaced with field-value
  combination greps that exclude unrelated `type:` lines and explicitly enumerate
  the false-positive exclusions.
- Removed the "agent prompts under `agents/`" requirement after confirming no
  agent prompt references the work-item kind field. The parent epic 0057 may
  want its technical-notes amended to drop that speculative entry.
- Reassigned corpus migration from 0070 to this story per author direction —
  the rename can ship independently and benefits from owning its own migration.
  This story now blocks 0070, which must not include a `type:` → `kind:`
  rewrite step in its migration; the rewrite is performed once by migration
  0005.
- Migration 0005's corpus application (running `/accelerator:migrate` against
  this repo's `meta/work/`) is bundled with migration authoring because it is
  the only mechanism that keeps `meta/work/` consistent with the renamed
  producers; splitting authoring from application would leave the dogfood
  corpus in a broken intermediate state.
- 0060 (base schema ADR) is complete and decided the unified `type:`
  discriminator vocabulary that this rename unblocks; it is therefore a
  satisfied predecessor rather than an active blocker.
- Added the visualiser frontend `WorkItemCard.tsx` as an affected surface
  (server side is field-name-agnostic).
- Added the four review-lens SKILL.md files — they branch on work-item kind.
- Added `work-item-template-field-hints.sh` to the helper-script list — its
  hardcoded fallback returns the five kind values keyed by the field name.
- Added eval fixtures and graders explicitly — easy to forget, breaks CI if
  missed.
- Status kept at `draft`; no transition requested.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0057, 0060, 0065, 0070
- Migration framework: `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
- Migration model: `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`
