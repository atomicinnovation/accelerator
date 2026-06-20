---
id: "0065"
title: "Update All Artifact Templates to Unified Schema"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: story
status: done
priority: high
parent: "work-item:0057"
tags: [templates, frontmatter, schema]
type: work-item
schema_version: 1
last_updated: "2026-05-17T17:16:35+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0033", "work-item:0061", "work-item:0060", "work-item:0033", "work-item:0063", "work-item:0064", "work-item:0057", "work-item:0066"]
blocks: ["work-item:0070", "work-item:0066"]
external_id: PP-87
---

# 0065: Update All Artifact Templates to Unified Schema

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Rewrite every artifact template under `templates/` so it emits the unified base frontmatter schema defined by **ADR-0033** (produced by work item 0060) and the typed linkage vocabulary from the sibling typed-linkage ADR (produced by 0061), so that every newly-produced artifact — and the downstream consumers that traverse the corpus, including the future visualiser graph epic — can rely on artifacts being born unified rather than waiting on the 0070 corpus migration. The scope also absorbs any *non-review* producer that bakes its frontmatter inline rather than reading a `templates/` file — only the four review/validation skills' inline generators are excluded (those are 0066's). This is the producer-side equivalent of the corpus migration: new artifacts are born unified.

ADR-0033 is the single source-of-truth for all field shapes, `schema_version` values, and per-artifact-type extras; the **Schema Reference** section below pins the concrete values this story must emit so the work is self-contained.

## Context

Per 0057, the Accelerator plugin produces twelve distinct artifact types (work-items, plans, plan-validations, plan-reviews, work-item-reviews, pr-reviews, pr-descriptions, ADRs, codebase-research, issue-research, design-inventories, design-gaps) plus the new `note` type. Each currently has its own template with idiosyncratic field names and shapes. With the schema and linkage ADRs decided, the templates must be brought into line. The epic separates template-based producers (this story) from the four review/validation skills that emit frontmatter inline (0066) — but any *other* inline producer that is neither a `templates/` file nor one of those four skills falls to this story. The new `note` type's template is **not** in scope here: it is created by the `create-note` story (0067) alongside the skill that consumes it.

## Requirements

- Update every template file under `templates/` to emit the unified base fields: `type`, `id` (the artifact's own identity), `title`, `date` (quoted ISO UTC), `author`, `producer` (the skill/automated agent that produced the artifact — renamed from `skill` per ADR-0033), `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version`. `id` is always a base field on every artifact type, never a per-type extra.
- Apply the identity-key naming convention: an artifact's **own** identity key is always `id`; a reference to a **foreign** artifact is keyed by the referenced artifact's snake_case type suffixed with `_id` (e.g. a pr-description referencing its work-item uses `work_item_id`; an ADR referencing a work-item uses `work_item_id`). This replaces the earlier `<type>_id`-for-own-identity shape (`adr_id`, `work_item_id` as the artifact's *own* key).
- Add the provenance bundle (`revision`, `repository`) to code-state-anchored templates: plan, codebase-research, issue-research, design-inventory, pr-description. Remove `git_commit` and `branch` where present.
- Apply per-artifact-type extras per **ADR-0033's "Per-artifact-type extras" section** (the authoritative, determinate list — it supersedes 0057's earlier hedged enumeration), reproduced in the Schema Reference table below: e.g. `kind` / `priority` / `external_id` on work-item, `decision_makers` on ADR, `pr_url` / `pr_number` / `merge_commit` on pr-description, `current_inventory` / `target_inventory` on design-gap. Note: ADR-0033 classifies relationship-named keys (`target`, `supersedes`, `superseded_by`, `derived_from`, `parent`, etc.) as **typed-linkage vocabulary** owned by 0061's sibling ADR — they are *not* per-type extras and are applied per that vocabulary, not this list.
- Ensure all identity values — own (`id`) and foreign (`<type>_id`) — are quoted YAML strings (e.g. `id: "0042"`, `work_item_id: "0031"`).
- Run a discovery pass over all frontmatter producers and bring any *non-review* inline producer (one that emits frontmatter outside `templates/` and is not `review-plan`, `review-work-item`, `review-pr`, or `validate-plan`) to the unified schema. If the pass finds none, record that it was run and found nothing.
- Each template carries a comment enumerating that artifact type's valid `status` values, taken from the per-type status vocabulary recorded in the Schema Reference table below. The vocabulary itself is unchanged (vocabulary unification is out of scope per 0057); the comment documents the existing per-type set in the place producers read.
- Confirm that each updated template's *consuming skill* supplies values for the new fields (`schema_version`, `producer`, `last_updated`, `last_updated_by`, and the provenance bundle where applicable) so artifacts created from the updated templates are born populated, not carrying placeholders. Where the consuming skill needs a code change to populate a field, that change is in scope for this story; where field-population was already handled by an upstream story (0063 `kind`, 0064 `work_item_id`/`author`), no further change is needed. **`validation.md` is excepted from this consuming-skill check**: its consuming skill (`validate-plan`) is rewired to read the template by 0066, so population from that skill is verified under 0066, not here. This story's responsibility for `validation.md` is the template's frontmatter block only.

## Schema Reference

Authoritative source: **ADR-0033** (base schema, provenance bundle, `schema_version` contract, per-artifact-type extras) and 0061's sibling ADR (typed-linkage vocabulary). The values below are pinned here so the acceptance criteria are verifiable from this document alone; on any discrepancy, ADR-0033 wins and this story should be re-synced.

**In-scope template files** (the nine frontmatter-bearing templates under `templates/`). Note on `validation.md`: it currently exists as a **body-only report template carrying no frontmatter** (the plan-validation frontmatter is emitted inline by `validate-plan`, which writes `date`, `type`, `skill`, `target`, `result`, `status`). This story **adds the unified frontmatter block to `validation.md`** so the template becomes the source of truth; the separate work of rewiring `validate-plan` to read frontmatter from the template instead of baking it inline is **0066's scope** (0066 is moving inline frontmatter into templates). The three review types (`plan-review`, `work-item-review`, `pr-review`) have no `templates/` file today — 0066 creates those when it moves their inline frontmatter into templates, so they are out of this story's scope.

| Template file | Artifact `type` | `schema_version` | Provenance bundle? | Per-type extras (beyond base) |
|---|---|---|---|---|
| `work-item.md` | `work-item` | 1 | no | `kind`, `priority`, `external_id` |
| `plan.md` | `plan` | 1 | yes | `reviewer` (present-but-empty until reviewed) |
| `validation.md` | `plan-validation` | 1 | no | `result` (`target` is a linkage key per 0061, not an extra). No "baseline fields" are emitted today — none are introduced here. |
| `pr-description.md` | `pr-description` | 1 | yes | `pr_url`, `pr_number`, `merge_commit` (present-but-empty until merged) |
| `adr.md` | `adr` | 1 | no | `decision_makers` |
| `codebase-research.md` | `codebase-research` | 1 | yes | `topic` |
| `rca.md` | `issue-research` | 1 | yes | `topic` |
| `design-inventory.md` | `design-inventory` | 1 | yes | retains its existing domain fields (`source`, `source_kind`, `source_location`, `crawler`, `sequence`, `screenshots_incomplete`); no new per-type extras beyond aligning these and the base fields to convention |
| `design-gap.md` | `design-gap` | 1 | no | `current_inventory`, `target_inventory` |

`schema_version` is `1` for every artifact type at ADR-0033's acceptance (per its §Schema versioning). The provenance bundle is `revision` + `repository` (replacing `git_commit`; `branch` dropped). For lifecycle-gated extras (`reviewer`, `merge_commit`, `pr_url`, `pr_number`), the template emits the key present-but-empty (or commented) and the consuming skill fills it at the relevant lifecycle event.

Reconciliation against the Context's twelve types: twelve artifact types plus `note` (thirteen), minus the three review types that have no template file (`plan-review`, `work-item-review`, `pr-review` — 0066 creates those), minus the `note` template (0067's) = nine frontmatter-bearing templates in scope, listed above.

**Base fields on every template**: `type`, `id`, `title`, `date`, `author`, `producer`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version`.

**Per-type `status` vocabularies** (for the status-comment requirement): the existing per-type sets are unchanged by this story. The implementer reads each type's current valid-status set from its existing template/skill and reproduces it verbatim in the comment — vocabulary unification is explicitly out of scope (0057).

## Acceptance Criteria

- [ ] All nine in-scope templates listed in the Schema Reference table (`work-item.md`, `plan.md`, `validation.md`, `pr-description.md`, `adr.md`, `codebase-research.md`, `rca.md`, `design-inventory.md`, `design-gap.md`) emit the unified base fields, including `producer`. (`validation.md`, previously body-only, gains a frontmatter block in this story; rewiring `validate-plan` to read it is 0066's scope.)
- [ ] The five code-state-anchored templates (`plan.md`, `codebase-research.md`, `rca.md`, `design-inventory.md`, `pr-description.md`) emit the provenance bundle (`revision`, `repository`) and no longer emit `git_commit` or `branch`.
- [ ] Each template's per-artifact extras match the Schema Reference table (sourced from ADR-0033's "Per-artifact-type extras"); relationship-named linkage keys are applied per 0061's vocabulary, not treated as extras.
- [ ] All identity-value fields in templates are quoted (e.g. `id: "NNNN"`, not bare integers).
- [ ] Each template's own-identity key is `id`; every foreign-artifact reference key is the referenced type's snake_case name suffixed with `_id` (e.g. `work_item_id`). No template uses `<type>_id` for its own identity.
- [ ] Every template includes `schema_version: 1` (the value pinned in the Schema Reference table per ADR-0033 §Schema versioning).
- [ ] A discovery pass has enumerated every frontmatter producer. The enumeration is **reproducible**: it records the exact grep command(s) used (e.g. `grep -rl "schema_version\|last_updated" skills/`) and the full list of matched skill files, such that re-running the recorded command reproduces the same producer set. The discovered set excludes exactly the four review/validation skills (`review-plan`, `review-work-item`, `review-pr`, `validate-plan`, 0066's) and every remaining non-template inline producer emits the unified base fields (or the recorded set, minus those four, is empty).
- [ ] Each template includes a comment listing that artifact type's valid `status` values; the listed set equals that template/skill's status set at the pre-edit revision (per VCS history) — no values added or removed, since vocabulary unification is out of scope.
- [ ] Generating one artifact from each updated template (except `validation.md`, whose skill is rewired under 0066) yields, for `producer`, `schema_version`, `last_updated`, `last_updated_by`, and (for the five code-state-anchored templates) `revision`/`repository`: a non-empty value containing no unsubstituted template token (`{{…}}` / `<…>`), with `schema_version` equal to the integer `1` and the two timestamps parsing as ISO-UTC — confirming the consuming skill supplies them.

## Open Questions

- None — all prior open questions were resolved during refinement (see Drafting Notes). The schema source-of-truth, ADR-0033, now documents the own-`id` / foreign-`<type>_id` convention, so this story can treat it as authoritative.

## Dependencies

- Blocked by: 0060 (base schema — produced **ADR-0033**, which is the authoritative source this story reads, not 0060's as-shipped wording), 0061 (linkage vocabulary — produced the sibling typed-linkage ADR), 0063 (work-item `kind:` rename — touches `templates/work-item.md`), 0064 (`work_item_id` / `author` canonicalisation — touches plan/research templates). Note: 0060 and 0064 shipped `status: done` before the own-`id`/foreign-`<type>_id` convention was finalised, so this story depends on the *corrected* convention as recorded in ADR-0033, not on those work items' original wording.
- Blocks: 0070 (corpus migration).
- Blocks: 0066 (for the `validation.md` handoff — see below).
- Related: 0057 (parent epic); 0066 (review/validation skills' frontmatter, now being *moved from inline into templates*). The cut line is **statically fixed**, not negotiated at runtime:
  - **This story (0065)** owns the template *files*: it adds the unified frontmatter block to the nine frontmatter-bearing templates, including `validation.md`.
  - **0066** owns the *skill-side* rewiring: it converts the four review/validation skills (`review-plan`, `review-work-item`, `review-pr`, `validate-plan`) from emitting frontmatter inline to reading it from templates, and creates the three missing review templates (`plan-review`, `work-item-review`, `pr-review`) as part of that move.
  - **Ordering**: 0066's rewiring of `validate-plan` depends on 0065 having added `validation.md`'s frontmatter first, so 0065 blocks 0066 for that piece. Any other inline producer discovered (non-review) is wholly this story's. Because the boundary is a fixed, named set, no producer can be claimed by both or neither.

## Assumptions

- The `templates/` directory is the source of truth for template-based producers. Inline frontmatter generators in the four review/validation skills (`review-plan`, `review-work-item`, `review-pr`, `validate-plan`) are 0066's scope; any *other* inline producer discovered is this story's scope.

## Technical Notes

- `last_updated` is only refreshed by skills that touch the artifact — manual editor edits don't automatically update it. Templates can document this contract in a comment.
- The identity-value shape contract (quoted strings) already applies to the work-item identity per `skills/config/configure/SKILL.md`; 0060 extends it to every artifact's `id` and to foreign `<type>_id` references.
- The own-`id` / foreign-`<type>_id` naming convention is documented in the schema source-of-truth, ADR-0033 (which work item 0060 produced): own identity was already keyed `id`, and the foreign-reference rule (`<snake_case_type>_id`) has been added to its identity-value shape contract. The 0057 epic and 0060 work item were corrected to match; 0064's renames were reviewed and remain valid (the plan's `work_item_id` is a foreign reference). ADR-0033's own frontmatter file still carries `adr_id` (unquoted) — this is *corpus-frontmatter divergence*: the field value sitting in an existing `meta/` file does not yet match the template output this story produces. Fixing existing files is the 0070 migration's job, not this template story's; ADR-0033's own frontmatter will be rewritten to `id` by that migration.
- The discovery pass needs to distinguish three producer shapes: (a) skills that read a `templates/` file, (b) the four review/validation skills that bake frontmatter inline (0066's), (c) any other skill that emits frontmatter inline (this story's). Grepping skill prose for frontmatter field literals (`schema_version`, `last_updated`) is a reasonable way to enumerate inline producers.

## Drafting Notes

- Open question on producers outside `templates/` resolved: this story absorbs any *non-review* inline frontmatter producer; only the four review/validation skills remain 0066's. This expands scope beyond pure template edits and adds a discovery-pass requirement.
- Open question on status comments resolved: each template carries a per-type valid-status comment. The status *vocabulary* stays unchanged per 0057's out-of-scope decision; only its documentation-in-place is added.
- Identity-key convention set per user: own identity = `id`; foreign-artifact references = `<snake_case_type>_id`. Threaded into the 0057 epic and 0060 work item, and into the source-of-truth ADR-0033 (own-`id` was already present; the foreign-`<type>_id` rule was added to its shape contract). 0064 was reviewed and needed no spec change. With ADR-0033 now authoritative, this story's prior open question (waiting on the schema source-of-truth) is resolved.
- `templates/note.md` was removed from this story's scope per user decision and moved to 0067 (create-note), so the note template ships alongside the skill that consumes it. This story therefore touches only templates that already exist under `templates/`.

## Discovery Pass Record

Commands executed (run from repo root, after Phases 3–10 have landed):

```
# Pass A — template-using and unified-schema-emitting producers
rg -n "config-read-template\.sh|^[[:space:]]*producer:|^[[:space:]]*schema_version:" skills --glob '**/SKILL.md'

# Pass B — legacy inline-frontmatter emitters that have NOT yet been moved
# to templates
rg -n "^[[:space:]]*verdict:|^[[:space:]]*review_pass:|^[[:space:]]*review_target:|^[[:space:]]*target:|^[[:space:]]*result:|^[[:space:]]*pr_number:" skills --glob '**/SKILL.md'
```

Pass A surfaces every skill that reads a template via the canonical
loader or directly names a unified base field. Pass B surfaces the
remaining inline emitters — specifically the four 0066-owned skills
(their `verdict:`, `review_pass:`, `target:`, `result:` literals)
plus describe-pr's `pr_number:` extra (for cross-reference).

Producer split:

- **Template-based emitters (updated by 0065)**: create-work-item,
  extract-work-items, create-plan, create-adr, extract-adrs,
  research-codebase, research-issue, inventory-design,
  analyse-design-gaps.
- **Hybrid emitter brought into compliance by 0065**: describe-pr.
- **Inline-only emitters owned by 0066 (excluded from this story)**:
  review-plan, review-work-item, review-pr, validate-plan.
- **Non-emitter template consumers (no action on frontmatter; read-path
  fallback handled in Phase 3 §4)**: refine-work-item, update-work-item,
  list-work-items.

Other non-review inline producers found: NONE.

### Consumer-side sweep

A second sweep checks read-path consumers of the renamed/removed keys
so the compatibility surface is recorded:

```
rg -n "work_item_id|adr_id|pr_title|^skill:|supersedes:|GIT_COMMIT|Current Git Commit Hash" skills scripts --glob '**/SKILL.md' --glob '**/*.sh'
```

Expected hits and resolution per hit:

- `work_item_id` in `work-item-common.sh` and `work-item-read-field.sh`
  → handled by Phase 3 §4 read-path fallback (both keys accepted on
  read during the 0065→0070 transition).
- `work_item_id` in the four work-item consuming SKILL.md files
  (`list-work-items`, `update-work-item`, `refine-work-item`, plus
  `create-work-item`'s enrich-existing self-check) → prose-updated
  by Phase 3 §4 to name both keys.
- `work_item_id` in `skills/config/configure/SKILL.md` →
  documentation-only; the legacy field-shape contract is preserved
  for the alias and rewritten by 0070.
- `work_item_id` in `skills/config/migrate/migrations/0001…0006` and
  in `skills/config/migrate/scripts/test-migrate.sh` → migration
  history and their fixtures; intentionally frozen at the legacy
  shape (the migrations target legacy files).
- `adr_id` in visualiser `indexer.rs` and `wiki-links.ts` → protected
  by filename-prefix fallback (`indexer.rs:1098`,
  `wiki-links.ts:103-115`); no change required.
- `adr_id` in `skills/decisions/scripts/test-adr-scripts.sh` and
  fixture ADRs under `skills/visualisation/visualise/server/tests/
  fixtures/` → fixture data exercising the read-path that already
  tolerates either shape.
- `pr_title` in `review-pr/SKILL.md` → owned by 0066; coordinated
  via the plan's Migration Notes.
- `Current Git Commit Hash` in `scripts/test-metadata-helpers.sh` →
  negative assertion ("no Current Git Commit Hash line"), expected.
- `GIT_COMMIT` matches in `skills/config/migrate/migrations/` →
  migration fixtures, frozen.

The Phase 11 SKILL-prose test's discovery assertion encodes the same
allowlists (`IN_SCOPE_PRODUCERS`, `OWNED_BY_0066`,
`NON_EMITTER_TEMPLATE_CONSUMERS`) and fails if any SKILL.md drifts
out of those three buckets.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Authoritative schema: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` (base schema, provenance, `schema_version`, per-type extras)
- `meta/work/0060-adr-unified-base-frontmatter-schema.md` — task that produced ADR-0033 (the base-schema decision)
- `meta/work/0061-adr-typed-linkage-vocabulary.md` — task that produced the sibling typed-linkage ADR
- Related: 0057, 0063, 0064, 0066, 0067, 0070
