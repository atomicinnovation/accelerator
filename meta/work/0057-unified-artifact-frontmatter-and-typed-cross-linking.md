---
work_item_id: "0057"
title: "Unified Artifact Frontmatter and Typed Cross-Linking"
date: "2026-05-14T17:41:22+00:00"
author: Toby Clemson
type: epic
status: draft
priority: high
parent: ""
tags: [frontmatter, knowledge-graph, migration, schema, accelerator-plugin]
---

# 0057: Unified Artifact Frontmatter and Typed Cross-Linking

**Type**: Epic
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As a plugin author/maintainer, I want every artifact type the Accelerator plugin produces to share a unified, well-typed frontmatter schema with explicit cross-links between related artifacts, so that the `meta/` corpus forms a navigable knowledge graph rather than disconnected document silos. This unblocks future graph-based visualisation, enables automated traversal of the artifact corpus, and removes the inconsistencies (`work_item_id` vs `work-item`, `author` vs `researcher`, mixed date formats, etc.) that have accumulated as artifact types were added independently.

## Context

The Accelerator plugin currently produces twelve distinct artifact types across `meta/` (work items, plans, plan validations, plan reviews, work-item reviews, pr reviews, pr descriptions, ADRs, codebase research, issue research / RCA, design inventories, design gaps), plus an under-served notes category that has no creator skill and no frontmatter. The frontmatter schemas evolved per-skill without a unifying contract, producing:

- **Field-name inconsistencies** — `work_item_id` (work-item, work-item-review) vs `work-item` (plan) for the same concept; `author` vs `researcher` for the same role; `target` paths vs PR-number strings; `git_commit` exposing a specific VCS.
- **Shape inconsistencies** — `adr_id` unquoted while `work_item_id` is a quoted string; `date` quoted/unquoted variation; `last_updated` sometimes `YYYY-MM-DD`, sometimes full ISO timestamp.
- **Missing discriminators** — only some artifacts carry a `type:` field; work-item's `type:` is overloaded with its semantic kind (story/epic/task/bug/spike) rather than artifact-type discrimination.
- **Linkage gaps** — relationships between artifacts (a plan derived from research; a work-item that blocks another; an ADR's decision-makers) currently live in free-text body sections like `## Related Research` rather than structured frontmatter.
- **Absent fields** — no `schema_version` (every future migration must sniff field presence to detect old shapes); no uniform `last_updated` revision tracking; no `derived_from`; no `blocks`/`blocked_by`; no `reviewer` on review artifacts; no `pr_url` on PR artifacts.

The driving pain is that the visualiser tool currently models the artifact lifecycle as a *linear path* (work-item → research → plan → review → validation → decision), encoded in a closed Rust enum and hardcoded canonical rank. In reality the corpus is a graph — many plans derive from one research doc, work-items block each other, ADRs supersede each other, design gaps reference inventory pairs. Without typed structured linkages in the frontmatter, the visualiser cannot move to a graph rendering. **The visualiser changes themselves are deferred to a separate future epic; this epic produces the frontmatter foundation that future epic will consume.**

Three existing ADR-creation-task work items overlap with this epic:

- `0021-artifact-persistence-lifecycle.md` (status: ready) — establishes the principle that every producer writes machine-parseable frontmatter to `meta/`. **Supplemented** by this epic — the principle is preserved and broadened to notes.
- `0022-artifact-metadata-and-lifecycle.md` (status: ready) — decides a common base schema of `date`, `type`, `skill`, `status` plus per-type extensions. **Supplemented** — this epic extends the schema and overrides specific shape decisions where the corpus is inconsistent.
- `0023-adr-system-design.md` (status: ready) — decides ADR uses `adr_id`, `status`, `supersedes`. **Supplemented** — this epic adds typed linkage keys and normalises `adr_id`'s YAML shape.

These three remain valid ADR-creation tasks and will be referenced as `derived_from` from the new ADRs this epic produces.

## Requirements

### Schema standardisation

- Define and document a **unified base frontmatter schema** present on every artifact type:
  - `type` — kebab-case artifact discriminator (`work-item`, `plan`, `plan-review`, `plan-validation`, `work-item-review`, `pr-review`, `pr-description`, `adr`, `codebase-research`, `issue-research`, `design-inventory`, `design-gap`, `note`)
  - identity field — `<type>_id` where the artifact has a natural ID (`work_item_id`, `adr_id`); a slug or path-derived value otherwise. All identity values are **quoted YAML strings**.
  - `title` (kept in sync with body H1 where applicable)
  - `date` (ISO timestamp UTC, `YYYY-MM-DDTHH:MM:SS+00:00`, **always quoted**)
  - `author` (canonical identity field — replaces `researcher`)
  - `status` (artifact-specific vocabulary; vocabulary unification is out of scope)
  - `tags` (YAML array, possibly empty)
  - `last_updated` (quoted ISO timestamp UTC) and `last_updated_by` (string)
  - `schema_version` (**per-artifact-type** integer; enables future migrations to detect old-shape documents deterministically)

- Define a **provenance bundle** of `revision` + `repository` on code-state-anchored artifacts (plans, codebase-research, issue-research/RCA, design-inventory, pr-description). `revision` replaces git-specific `git_commit`; `branch` is dropped.

- Define a **typed linkage vocabulary** (specific keys where they add value; relationship type inferred from artifact-type pairs otherwise):
  - `parent` (single ref — hierarchical owner)
  - `supersedes` / `superseded_by` (list)
  - `blocks` / `blocked_by` (list)
  - `target` (single ref — what this artifact is *about*, used by reviews and validations)
  - `derived_from` (**list** — generative source; a plan can derive from multiple research docs)
  - `relates_to` (list — loose linkage)
  - `source` (single ref — external origin for extracted artifacts)

- **Resolve field-name conflicts**: `work_item_id` becomes the canonical work-item reference key (eliminate hyphenated `work-item`); `author` replaces `researcher`.

- **Resolve the work-item discriminator collision**: rename work-item's current semantic-kind `type:` field to `kind:` (`story | epic | task | bug | spike`) so `type:` can carry the artifact-type uniformly. Update templates, resolver scripts, agent prompts, and any helpers that read this field.

### Per-artifact extras

- **work-item**: `kind` (renamed from `type`), `priority`, `parent`, `blocks`, `blocked_by`, `derived_from`, `external_id` (cross-system pointer per epic 0045 conventions).
- **plan**: `target` (work-item ref) replacing hyphenated `work-item`, `derived_from` (research refs), provenance bundle, `reviewer` once reviewed.
- **plan-validation**: `target` (plan ref), `result`, baseline fields.
- **plan-review / work-item-review / pr-review**: `target`, `reviewer`, `verdict`, `lenses`, `review_number`, `review_pass` where applicable. Verdict enum alignment (`REVISE` vs `REQUEST_CHANGES`) is out of scope unless trivial.
- **pr-description**: `work_item_id` (promote from body to frontmatter), `pr_url`, `pr_number`, provenance bundle, `merge_commit` once merged.
- **adr**: `adr_id` (quoted YAML string), `status`, `supersedes` / `superseded_by`, `decision_makers`, `derived_from`.
- **codebase-research / issue-research**: `author` replacing `researcher`, provenance bundle, `topic`, `derived_from`.
- **design-inventory**: existing schema is already rich; align field-name conventions only.
- **design-gap**: keep `current_inventory` / `target_inventory` as specific keys (they add value over a generic `targets:` list).
- **note** (new artifact type): baseline + `topic` + provenance bundle.

### Notes creator skill

- Introduce a `create-note` skill at `skills/notes/create-note/SKILL.md` producing files under `meta/notes/` conforming to the unified schema. Notes are short-form observations or strategy snippets that don't fit the research/plan/ADR mould.

### Producer updates

- Update every template under `templates/` to the unified schema.
- Update every skill that emits frontmatter inline rather than via a template — `review-plan`, `review-work-item`, `review-pr`, `validate-plan`.
- Update helpers that read or write frontmatter fields (`work-item-read-field.sh`, `work-item-resolve-id.sh`, etc.) where field names change.

### Corpus migration

- Ship a meta-directory migration (numbered after the latest applied) that:
  - Rewrites existing frontmatter to the unified schema across every artifact in `meta/` — field rename, format normalisation, addition of new baseline fields with sensible defaults.
  - Parses free-form "Related documents" / "References" / "Related Research" body sections and populates structured linkage frontmatter where confident.
  - Records `schema_version` per artifact type so future migrations can deterministically detect old-shape documents.
  - Surfaces uncertain inferences for user confirmation. If a deterministic best-effort migration produces high-confidence results, interactivity is unnecessary; otherwise extend the migration framework with interactive validation hooks (the framework today is purely mechanical — this is a deliberate extension).
- Dogfood the migration against this repo's own `meta/` corpus and fix gaps surfaced.

## Acceptance Criteria

- [ ] A single source-of-truth document (ADR or schema file) defines the unified base frontmatter, the provenance bundle, the typed linkage vocabulary, and the per-artifact-type extras.
- [ ] `work_item_id` is the canonical work-item reference key across every template and producing skill (hyphenated `work-item` eliminated).
- [ ] `author` is the canonical identity field; `researcher` is eliminated from research and RCA templates.
- [ ] Every artifact template and inline frontmatter generator emits the unified base fields (`type`, identity, `title`, `date`, `author`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version`).
- [ ] Code-state-anchored artifacts emit the provenance bundle (`revision`, `repository`); `git_commit` and `branch` are removed.
- [ ] Work-item's semantic-kind field is renamed `type:` → `kind:`; resolver scripts, agent prompts, and helpers are updated.
- [ ] Typed linkage keys (`parent`, `supersedes`/`superseded_by`, `blocks`/`blocked_by`, `target`, `derived_from`, `relates_to`, `source`) are documented and used where they apply. `derived_from` accepts a list.
- [ ] A `create-note` skill exists and produces well-formed notes under `meta/notes/`.
- [ ] A migration is shipped that successfully upgrades this repo's own `meta/` corpus end-to-end; uncertain link inferences are surfaced (interactively if the framework was extended, or via a post-run report otherwise).
- [ ] `schema_version` values are recorded per artifact type so a subsequent breaking schema change can detect the old shape deterministically.
- [ ] Work items 0021, 0022, 0023 are referenced from the new ADRs and acknowledged as supplemented (not superseded) in those ADRs.

## Open Questions

- Should the new ADRs (unified base schema, typed cross-linking, migration strategy) be created and accepted before producer-skill updates begin, or alongside (each story carries its piece of the decision)?
- Where should the source-of-truth schema document live — as an ADR? As a dedicated schema file under `meta/` or `docs/`? As both (ADR for decision, schema file for machine-readable reference)?
- The migration's "Related documents" inference accuracy is unknown until prototyped; should there be an explicit spike story to evaluate inference confidence before committing to interactive-vs-non-interactive migration design?
- Should this epic's children include a verdict-enum unification pass (`REVISE` vs `REQUEST_CHANGES`) opportunistically, or strictly leave it for a follow-up?
- For `external_id` on work-items: is the field name and shape decided by epic 0045, or does this epic need to decide its own conventions?

## Dependencies

- Blocked by: none — this epic can begin independently.
- Blocks: future "Artifact knowledge graph visualisation" epic (visualiser graph backend + frontend) which will consume the typed linkages produced here.
- Related: 0021, 0022, 0023 (supplemented); 0045 (defines `work_item_id` shape for external trackers); 0056 (precedent for frontmatter-aware migration); 0040 (visualiser context).

## Assumptions

- The new ADRs produced under this epic **supplement** (not supersede) the ADRs that 0021/0022/0023 were intended to produce. The originals remain valid and are referenced by the new ADRs.
- `status:` vocabularies and review verdict enums remain artifact-specific; only field-name and shape unification is in scope.
- VCS revert remains the migration safety net; no inverse migration will be built.
- `last_updated` will only be refreshed by skills that touch the artifact — manual editor edits won't automatically update it. Acceptable per user direction.
- `specs/` and `global/` directories are deferred (specs aren't part of the plugin; global isn't heavily used).
- The visualiser graph rendering work is a separate future epic that consumes the typed linkage frontmatter this epic produces.

## Technical Notes

- The migration framework (`skills/config/migrate/`) today is purely mechanical (no prompts, no dry-run, VCS-as-rollback). Extending it with optional interactive validation hooks is a deliberate departure from the contract documented in ADR-0023. The extension is conditional — if best-effort deterministic inference produces high-confidence results across the dogfood corpus, the framework stays mechanical and a post-run report surfaces ambiguities.
- Identity-value shape contract: all identity values (`work_item_id`, `adr_id`, etc.) are quoted YAML strings for uniform consumer parsing — already the contract for `work_item_id` per `skills/config/configure/SKILL.md`; this epic extends the contract to `adr_id` and any new identity fields.
- The work-item `type:` → `kind:` rename is the single most disruptive change. Files affected include `templates/work-item.md`, every `skills/work/*/SKILL.md`, `skills/work/scripts/work-item-read-field.sh`, `skills/work/scripts/work-item-resolve-id.sh`, agent prompts that read the field, and every work-item in `meta/work/` (handled by the migration). Coordinate this rename in a single story.
- Inline frontmatter generators in review skills currently bake field shapes into SKILL.md prose. Extracting them into shared template files is not mandated by this epic but would simplify future schema changes — left as an option for the implementing story.
- `meta/notes/` files today are hand-written and frontmatter-free. The migration must either skip them or add baseline frontmatter with conservative defaults (e.g. `author: <unknown>` if not inferable from VCS history). Decision deferred to the migration-story scope.

## Drafting Notes

- Treated "everything" as the twelve artifact types the codebase research identified plus notes; specs and global excluded per user direction.
- Distinguished "field-name and shape unification" (in scope) from "vocabulary unification" (out of scope) per user decision.
- Chose epic over story because the work spans schema design / ADRs / template updates / multiple skill updates / a new skill / migration framework extension / corpus migration — at least seven independent threads.
- Priority "high" reflects breaking-change-with-migration cost and unblocking the future visualiser graph feature; could be lowered to medium if the visualiser-graph epic is itself deferred.
- Visualiser changes removed from epic scope per user instruction; deferred to a sibling future epic.
- Interpreted 0021/0022/0023 as remaining valid ADR-creation tasks that this epic supplements via narrower scope-overrides rather than wholesale supersession.
- `revision` chosen over `git_commit` (too git-specific) per user; `branch` dropped per user (`revision` alone identifies the snapshot).
- Per-artifact-type `schema_version` chosen over single global version per user.
- `derived_from` as list (not single ref) per user — accommodates plans derived from multiple research docs.
- Work-item `type:` → `kind:` rename committed per user — chose disruption-now over collision-forever.

## References

- Source research (in-conversation):
  - Frontmatter inventory across all artifact types
  - Migration framework architecture
  - Visualiser knowledge model (context for the future sibling epic)
- Related work items:
  - `meta/work/0021-artifact-persistence-lifecycle.md` — supplemented
  - `meta/work/0022-artifact-metadata-and-lifecycle.md` — supplemented
  - `meta/work/0023-adr-system-design.md` — supplemented
  - `meta/work/0045-work-management-integration.md` — defines `work_item_id` shape
  - `meta/work/0040-pipeline-visualisation-overhaul.md` — visualiser context
  - `meta/work/0056-restructure-meta-research-into-subject-subcategories.md` — migration precedent
