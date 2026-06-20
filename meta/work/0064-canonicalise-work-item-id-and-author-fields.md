---
id: "0064"
title: "Canonicalise `work_item_id` and `author` Field Names"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: story
status: done
priority: high
parent: "work-item:0057"
tags: [refactor, frontmatter, schema, migration]
type: work-item
schema_version: 1
last_updated: "2026-05-17T17:16:35+00:00"
last_updated_by: Toby Clemson
blocks: ["work-item:0065"]
relates_to: ["work-item:0057", "work-item:0060", "work-item:0063", "work-item:0070"]
external_id: PP-86
---

# 0064: Canonicalise `work_item_id` and `author` Field Names

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Replace the hyphenated `work-item:` field on plan frontmatter with the canonical `work_item_id:`, and replace `researcher:` on research and root-cause analysis (RCA) frontmatter with the canonical `author:`. The renames eliminate per-skill field-name inconsistencies and ship as a self-contained releasable change: templates, producing skills, consumers (the visualiser server, frontend types/components, and any helper scripts or agent prompts that read these fields), and a corpus migration land together so the repo and any userspace consumer stay coherent across the version bump. Primary beneficiaries are skill authors writing frontmatter and downstream consumers (the visualiser, helper scripts, userspace projects upgrading via `/accelerator:migrate`).

## Context

Per 0057, two field-name conflicts have accumulated:

- The same concept (a reference to a work-item) is spelled `work_item_id` in work-item and work-item-review frontmatter but `work-item` (hyphenated) in plan frontmatter.
- The same role (the person authoring an artifact) is spelled `author` on most artifacts but `researcher` on codebase-research and issue-research frontmatter.

Both are pure renames at the schema level. To stay releasable, this story bundles the producer updates, the consumer updates (including the visualiser server's hardcoded `work-item:` reads and any frontend types), and a corpus migration following the precedent of `0005-rename-work-item-type-to-kind.sh`. 0070 (broader frontmatter normalisation/extension) is a separate, larger workstream that may further reshape these fields — this story does not block on it.

## Requirements

- Rename plan frontmatter's `work-item:` → `work_item_id:` in `templates/plan.md` and in the plan-producing skill `skills/planning/create-plan/SKILL.md`.
- Rename research and RCA frontmatter's `researcher:` → `author:` in `templates/codebase-research.md`, `templates/rca.md`, and in the producing skills `skills/research/research-codebase/SKILL.md` and `skills/research/research-issue/SKILL.md`.
- Update visualiser consumers: `skills/visualisation/visualise/server/src/frontmatter.rs`, `skills/visualisation/visualise/server/src/indexer.rs`, `skills/visualisation/visualise/server/src/patcher.rs`, and `skills/visualisation/visualise/server/src/api/related.rs` (Rust reads of `work-item:`), plus the enumerated frontend files `skills/visualisation/visualise/frontend/src/api/types.ts`, `skills/visualisation/visualise/frontend/src/api/work-item.ts`, and `skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx`.
- Update helper scripts and agent prompts that read these fields. The non-template, non-visualiser consumer surface is `scripts/`, `skills/**/SKILL.md`, and `skills/**/scripts/`; any matches the grep in Acceptance Criterion 1 surfaces in those locations are in scope for this story.
- Add a numbered migration under `skills/config/migrate/migrations/` (next available number after 0063's migration) that:
  - rewrites `work-item:` → `work_item_id:` in plan frontmatter across the configured paths,
  - rewrites `researcher:` → `author:` in research and RCA frontmatter across the configured paths,
  - preserves the `work_item_id` quoted-string shape contract,
  - handles userspace template overrides at `.accelerator/templates/*.md` (precedent: 0005's `paths-override` fixture),
  - ships with test fixtures covering default-layout, partial-prior-run, fully-migrated (idempotence), and paths-override variants.

## Acceptance Criteria

- [ ] `rg -n 'work-item:' templates/ skills/ scripts/ skills/visualisation/visualise/server/src/ skills/visualisation/visualise/frontend/src/` returns no matches. (The search roots intentionally exclude `meta/` and `skills/config/migrate/migrations/`, where legacy references in historical artifacts and prior migration scripts are expected.)
- [ ] `rg -n 'researcher:' templates/ skills/ scripts/ skills/visualisation/visualise/server/src/ skills/visualisation/visualise/frontend/src/` returns no matches.
- [ ] A migration under `skills/config/migrate/migrations/` performs both renames across plans, research, and RCAs in the paths resolved by the migrate skill (including userspace overrides).
- [ ] The migration handles userspace template overrides (`.accelerator/templates/*.md`) per the 0005 precedent.
- [ ] Migration test fixtures cover default-layout, partial-prior-run, fully-migrated, and paths-override scenarios. Running the migration on the fully-migrated fixture produces no diff (idempotence).
- [ ] All `work_item_id:` values remain quoted YAML strings per the identity-value shape contract.
- [ ] After running the migration on this repo, the visualiser kanban view at `/kanban` lists all previously-visible work items with their `work_item_id` cross-references intact (no broken-link badges); the plan detail view for plan 0063 displays its parent work-item link to 0057; no plan detail page shows a broken-link badge in place of its parent work-item link; and the visualiser's research/RCA listings render with `author` populated from the renamed field.
- [ ] Upgrade-path check: starting from a named fixture repo at `skills/config/migrate/scripts/test-fixtures/<NNNN>-canonicalise-work-item-id-and-author/upgrade-path/` (containing at least one plan with `work-item:`, one research doc with `researcher:`, one RCA with `researcher:`, and one userspace template override under `.accelerator/templates/`) the following four-step sequence succeeds: (1) start at the pre-rename plugin version with the fixture's legacy frontmatter intact; (2) upgrade the plugin to the post-rename version; (3) run `/accelerator:migrate`; (4) verify all plan frontmatter has been rewritten to `work_item_id:`, all research/RCA frontmatter to `author:`, the userspace override has been rewritten in place, and the visualiser smoke-test in the previous criterion passes against the upgraded fixture.

## Dependencies

- Blocks: 0065 (template-wide updates).
- Related: 0057 (parent epic), 0060 (base schema ADR — its canonical-name decision is the source for the names used here; already settled), 0063 (sibling rename — work-item `type:` → `kind:` — already landed; this story sequences after it and picks the next available migration number), 0070 (broader frontmatter normalisation).

## Assumptions

- The two renames are logically independent (touching different templates, producing skills, and reviewer surfaces) but are intentionally bundled into a single migration and a single release for this story, for review-cost economy and to keep the migration single-shot.
- Outside the plugin and `meta/`, no external system (CI, dashboards, sync tooling) currently reads these field names. Userspace template overrides are handled by the migration; other userspace consumers are assumed to flow through the plugin's API and so are insulated.

## Technical Notes

- The shape contract for `work_item_id` (quoted YAML string) is already documented in `skills/config/configure/SKILL.md`. This story extends consistent use across the plan template and any inline frontmatter generators.
- Migration precedent: `skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh` is the direct template — same shape (frontmatter key rename across a directory of markdown files), with `paths-override` fixture for ejected userspace templates.
- Visualiser hardcoded reads to update: `frontmatter.rs:297-326`, `indexer.rs` (around the `work-item:` reverse cross-ref index, ~line 1008), plus TS in `frontend/src/api/types.ts`, `api/work-item.ts`, `routes/kanban/WorkItemCard.tsx`.
- Under the refined unified identity convention (own key `id`; foreign references `<snake_case_type>_id`), the `work-item:` → `work_item_id:` rename in plan frontmatter is correct as-is: the plan's reference to its work-item is a *foreign* reference, so `work_item_id` is the right key. The `researcher:` → `author:` rename is likewise unaffected. What this convention adds — out of scope for this story — is migrating a work-item's *own* identity field `work_item_id` → `id` in the work-item template and corpus; that lands via 0065 (templates) and the corpus migration (0070).

## Open Questions

None remain at the time of writing. Earlier ambiguities (visualiser scope, bundling vs splitting, 0060's blocker status, 0063's ordering) have been resolved in this draft.

## Drafting Notes

- Bundled the two renames into one story because they are pure mechanical renames with the same risk profile. If you'd rather split them — e.g. because `work_item_id` and `author` touch different reviewer surfaces — splitting is low-cost.
- Treated the migration of existing files as in-scope (corpus migration ships with the story) so the change is releasable on its own. 0070's broader normalisation remains a separate, larger workstream.
- Visualiser consumer updates are included in this story; previously the work item left this ambiguous via an open question.
- Reviewed against the refined own-`id` / foreign-`<type>_id` identity convention (set after this story was marked `done`): this story's renames remain valid (the plan's `work_item_id` is a foreign reference, and `researcher:` → `author:` is orthogonal), so no requirement or acceptance-criterion change was needed. Added a Technical Note clarifying the boundary with the separate work-item own-identity migration (0065 + 0070).

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0057, 0060, 0063, 0065, 0070
- Migration precedent: `skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh`
