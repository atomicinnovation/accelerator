---
date: "2026-05-30T01:11:44+01:00"
author: Toby Clemson
git_commit: 0985b070621b1f5a39548ad3ee964992c943313e
branch: HEAD
repository: accelerator
topic: "Update all artifact templates to the unified schema (work item 0065)"
tags: [research, codebase, templates, frontmatter, schema, work-item-0065]
status: complete
last_updated: 2026-05-30
last_updated_by: Toby Clemson
work_item_id: "0065"
---

# Research: Update All Artifact Templates to Unified Schema (Work Item 0065)

**Date**: 2026-05-30 01:11 BST
**Author**: Toby Clemson
**Git Commit**: 0985b070621b1f5a39548ad3ee964992c943313e
**Branch**: HEAD
**Repository**: accelerator

## Research Question

What is the concrete implementation surface for work item 0065 — updating every artifact template under `templates/` to emit the unified base frontmatter schema defined by ADR-0033 and the typed-linkage vocabulary from ADR-0034 — covering: which templates need which edits, which consuming skills need prose changes to populate the new fields, where inline frontmatter producers live (and which are owned by 0066 vs. this story), and what the gap is between today's emitted shapes and the target schema?

## Summary

The scope is **nine in-scope templates** under `templates/` (the work item lists them explicitly in its Schema Reference table). Eight already exist and need edits; one — `templates/validation.md` — exists as a body-only report and needs a frontmatter block added for the first time. The three review templates (`plan-review`, `work-item-review`, `pr-review`) **do not** belong to this story — `meta/work/0066-update-review-skills-inline-frontmatter.md` creates them as part of rewiring the four review/validation skills.

The producer-side reality is more demanding than "edit the templates and stop". The work item's last acceptance criterion requires that artifacts generated from updated templates carry non-empty, non-tokenised values for `producer`, `schema_version`, `last_updated`, `last_updated_by`, and (for code-state-anchored types) `revision`/`repository`. The investigation found:

- **Nine of the ten** template-consuming SKILL.md files have **no instruction** to populate `producer`, `schema_version`, `last_updated`, or `last_updated_by`. Only `research-codebase/SKILL.md` names `last_updated` and `last_updated_by` — and only in its follow-up flow, not in the initial write.
- The shared metadata helper `scripts/artifact-derive-metadata.sh` emits `git_commit`, `repository`, and a timestamp — but does **not** emit `producer`, `schema_version`, `last_updated`, `last_updated_by`, and uses a non-ISO timestamp format (`YYYY-MM-DD HH:MM:SS %Z`) that conflicts with the ISO `+00:00` format required by ADR-0033.
- `create-plan/SKILL.md` has **zero** metadata-capture instructions today (no helper call, no shell command), despite being a code-state-anchored producer.
- The provenance bundle (`revision` + `repository`) requires renaming `git_commit` → `revision` in the helper output AND threading the new name into prose; the helper already drops `branch` (good) but the rename is real work.

The discovery-pass requirement is bounded and easy to satisfy: the configured loader expression `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh <name>` is the canonical mechanism, and `rg -n "config-read-template\.sh|schema_version|last_updated|git_commit|verdict:" skills --glob '**/SKILL.md'` reproducibly enumerates every frontmatter-emitting skill. The set splits into: nine template-based consumers (this story's), four inline-only review/validation skills (0066's), and two hybrid emitters (`describe-pr`, `validate-plan`) that consume a template body but also bake their own inline frontmatter. The hybrids matter because their inline frontmatter overrides whatever the template emits — making `describe-pr` a candidate this story must change so the template's unified frontmatter actually reaches disk.

The own-identity rename is concentrated in two templates: `templates/work-item.md` (`work_item_id` → `id`) and `templates/adr.md` (`adr_id` → `id`). Foreign references (`work_item_id` on plans/research/work-item-reviews) stay as they are — they already follow the new convention per 0064.

## Detailed Findings

### Authoritative schema sources

ADR-0033 (`meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`) is the single source of truth. Its key contracts for this story:

- **Base fields** (every artifact): `type`, `id`, `title`, `date`, `author`, `producer`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version`. (`meta/decisions/ADR-0033-unified-base-frontmatter-schema.md:113-125`)
- **Provenance bundle** (code-state-anchored only): `revision` (replaces `git_commit`), `repository`. `branch` is removed. (`ADR-0033:127-139`)
- **Identity-value shape**: every `id` is a quoted YAML string. Own identity = `id`; foreign reference = `<snake_case_type>_id`. (`ADR-0033:141-167`)
- **`schema_version` is per-artifact-type**, initial value `1` for every type at acceptance. (`ADR-0033:169-180`)
- **`skill` → `producer` rename** in the mandatory base; `author` reserved for human identity. (`ADR-0033:182-190`)
- **Per-type extras** — `kind`/`priority`/`external_id` (work-item), `decision_makers` (ADR), `pr_url`/`pr_number`/`merge_commit` (pr-description), etc. (`ADR-0033:192-219`)

ADR-0034 (`meta/decisions/ADR-0034-typed-linkage-vocabulary.md`) decides linkage keys (`parent`, `supersedes`, `superseded_by`, `blocks`, `blocked_by`, `target`, `derived_from`, `relates_to`, `source`). Reference value shape is either `"doc-type:id"` (`"plan:0042"`, `"adr:ADR-0033"`) or a project-root-relative path — always a single quoted string. (`ADR-0034:44-82`) Relationship-named keys are NOT per-type extras: ADR-0033 reserves them to ADR-0034's vocabulary.

### Today's nine templates — gap to target

#### `templates/work-item.md` (`templates/work-item.md:1-11`)
**Current frontmatter:** `work_item_id`, `title`, `date`, `author`, `kind`, `status`, `priority`, `parent`, `tags`.
**Gap:**
- Add `type: work-item`, `producer`, `last_updated`, `last_updated_by`, `schema_version: 1`.
- Rename `work_item_id:` → `id:` (own-identity convention from ADR-0033 §Identity-value shape contract).
- `external_id` is a per-type extra per ADR-0033 — add as a present-but-empty key (or commented) for the cross-system pointer.
- `parent:` is a typed-linkage key (ADR-0034 §Linkage keys) — its value form must shift to `"work-item:NNNN"` or a quoted ID (the existing shape `parent: ""` / `parent: "0057"` is already a quoted string, so this is a content shape update rather than a structural one).
- Status-comment requirement: keep the existing enumeration (`draft | ready | in-progress | review | done | blocked | abandoned`) — vocabulary unification is out of scope per 0057.

#### `templates/plan.md` (`templates/plan.md:1-7`)
**Current frontmatter:** `date`, `type: plan`, `skill: create-plan`, `work_item_id`, `status`.
**Gap (largest delta):**
- Rename `skill:` → `producer:` (still values `create-plan`).
- Add `id` (own identity — plans don't currently have one; the work item's "id is always a base field on every artifact type" rule applies here). The natural value is the plan's filename stem or a slug-derived value per ADR-0033 §Base schema.
- Add `title`, `author`, `tags`, `last_updated`, `last_updated_by`, `schema_version: 1`.
- Add the provenance bundle: `revision`, `repository`.
- Add per-type extra `reviewer:` (present-but-empty until reviewed).
- Keep `work_item_id` (foreign reference, already in 0064-canonical shape).
- Corpus reality check: newer plans (e.g. `meta/plans/2026-05-26-0088-...md`) still emit `work-item:` (hyphenated) — 0064's plan-skill update apparently regressed for plans created via the skill since 0064 shipped. The template currently uses `work_item_id` so the regression is on the consuming-skill side, not the template. 0070's migration handles the corpus; 0065 only needs to confirm the template's `work_item_id` is what the skill writes.

#### `templates/validation.md` (`templates/validation.md:1-3`)
**Current:** body-only report — **no frontmatter block exists today**.
**Gap (greenfield frontmatter):**
- Add the unified base block: `type: plan-validation`, `id`, `title`, `date`, `author`, `producer: validate-plan`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version: 1`.
- Add per-type extra `result`.
- Linkage key `target` is per ADR-0034, not a per-type extra — note this in the template's status-comment block.
- The work item explicitly excludes this template from the final consuming-skill check (the rewiring of `validate-plan` to read the template is 0066's job). The story's responsibility is the frontmatter block only.
- The plan-validation `status` vocabulary per existing corpus is just `complete` (`meta/validations/2026-05-18-0042-templates-view-redesign-validation.md:7`); no other values observed.

#### `templates/pr-description.md` (`templates/pr-description.md:1-8`)
**Current:** `date`, `type: pr-description`, `skill: describe-pr`, `pr_number`, `pr_title`, `status: complete`.
**Gap:**
- Rename `skill:` → `producer:`.
- `pr_title` is not in ADR-0033's per-type extras list — keep as a producer-supplied secondary, or migrate the value into the base `title:` field (cleaner). The schema reference table in the work item lists only `pr_url`, `pr_number`, `merge_commit` as per-type extras.
- Add `id` (own identity — the PR's natural ID is the number, but `id` is the unified key; consider `id: "<pr_number>"` as a quoted string).
- Add `title`, `author`, `tags`, `last_updated`, `last_updated_by`, `schema_version: 1`.
- Add provenance bundle `revision`, `repository`.
- Add `pr_url`, `merge_commit` (present-but-empty until merged) per the per-type extras row.
- **Hybrid producer**: `describe-pr/SKILL.md:99-107` emits inline frontmatter that overrides the template's block. Section "Consuming-skill mechanics" below explains the implication — `describe-pr` needs prose changes so the inline emission stops fighting the template, OR the inline block must be brought into ADR-0033 shape.

#### `templates/adr.md` (`templates/adr.md:1-8`)
**Current:** `adr_id`, `date`, `author`, `status`, `supersedes`, `tags`.
**Gap:**
- Rename `adr_id:` → `id:` (own-identity convention, ADR-0033 §Identity-value shape contract). Value stays as `"ADR-NNNN"` per ADR-0033's example.
- Add `type: adr`, `title`, `producer: create-adr`, `last_updated`, `last_updated_by`, `schema_version: 1`.
- Add per-type extra `decision_makers`.
- `supersedes:` is a typed-linkage key per ADR-0034 — its value shape must be a list of quoted `"adr:ADR-NNNN"` strings (or remain absent). Today's shape is unquoted single value (`supersedes: ADR-NNNN`), so a real shape change.
- ADR-0033 itself is the schema source-of-truth, but ADR-0033's own frontmatter file uses `adr_id:` (unquoted) — corpus-frontmatter divergence per Technical Notes; 0070 will fix existing ADRs.

#### `templates/codebase-research.md` (`templates/codebase-research.md:1-12`)
**Current:** `date`, `author`, `git_commit`, `branch`, `repository`, `topic`, `tags`, `status`, `last_updated`, `last_updated_by`. (Closest of all templates to the target.)
**Gap:**
- Add `type: codebase-research`, `id`, `title`, `producer: research-codebase`, `schema_version: 1`.
- Rename `git_commit:` → `revision:`. Drop `branch:` (removed by ADR-0033).
- Today's `date` is an unquoted template placeholder; quote it per ADR-0033 (`date: "{ISO timestamp}"`).
- Today's `last_updated` value uses calendar-date form (`2026-05-26`) per corpus examples — ADR-0033 mandates quoted ISO UTC timestamp form. Template needs to switch the placeholder, and the helper must produce the right format.
- Keep `topic` (per-type extra).
- Foreign ref `work_item_id` already lands on issue-tied research per corpus — keep the template guidance.

#### `templates/rca.md` (`templates/rca.md:1-12`)
Same shape as codebase-research today; same gap. Add `type: issue-research`, rename `git_commit:` → `revision:`, drop `branch:`, add `id`, `title`, `producer: research-issue`, `schema_version: 1`, and quote-ISO the timestamps. Keep `topic`.

#### `templates/design-inventory.md` (`templates/design-inventory.md:1-17`)
**Current:** `date`, `type: design-inventory`, `source`, `source_kind`, `source_location`, `git_commit`, `branch`, `crawler`, `author`, `status`, `sequence`, `screenshots_incomplete`, `tags`, `last_updated`, `last_updated_by`.
**Gap:**
- Add `id`, `title`, `producer: inventory-design`, `schema_version: 1`.
- Rename `git_commit:` → `revision:`. Drop `branch:`.
- Keep all existing domain fields (`source`, `source_kind`, `source_location`, `crawler`, `sequence`, `screenshots_incomplete`) — work item explicitly says these are retained.

#### `templates/design-gap.md` (`templates/design-gap.md:1-9`)
**Current:** `date`, `type: design-gap`, `current_inventory`, `target_inventory`, `author`, `status`, `tags`.
**Gap:**
- Add `id`, `title`, `producer: analyse-design-gaps`, `last_updated`, `last_updated_by`, `schema_version: 1`.
- Keep type-specific keys `current_inventory`, `target_inventory` per ADR-0034 §Design-gap inventory keys (explicitly kept type-specific, not folded into the generic vocabulary).
- Not code-state-anchored per the schema reference, so no provenance bundle.

### Consuming-skill mechanics — what populates the new fields

The investigation walked all ten consuming skills (the nine for in-scope templates plus the hybrid `validate-plan`). Findings:

**Template loading is centralised**: every consumer routes through `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh <name>`. If template files are updated, the template content reaches the model with no further code change. The work happens in two places: (a) the templates themselves, (b) the SKILL.md prose that instructs the model what to substitute into placeholders.

**Field-by-field population status across the nine in-scope skills (verbatim from the analyser pass):**

| Skill | producer | schema_version | last_updated | last_updated_by | revision | repository |
|---|---|---|---|---|---|---|
| `create-work-item/SKILL.md` | not instructed | not instructed | not instructed | not instructed | N/A | N/A |
| `extract-work-items/SKILL.md` | not instructed | not instructed | not instructed | not instructed | N/A | N/A |
| `create-plan/SKILL.md` | not instructed | not instructed | not instructed | not instructed | NOT captured | NOT captured |
| `describe-pr/SKILL.md` (hybrid) | hardcoded literal `skill: describe-pr` (L103) | not instructed | not instructed | not instructed | NOT captured | NOT captured (despite `gh repo view` being invoked elsewhere) |
| `create-adr/SKILL.md` | not instructed | not instructed | not instructed | not instructed | N/A | N/A |
| `research-codebase/SKILL.md` | not instructed | not instructed | named in prose (L152, follow-up only) | named in prose (L152, follow-up only) | as `git_commit` via helper (L188) | via helper |
| `research-issue/SKILL.md` | not instructed | not instructed | not instructed | not instructed | via helper (L94-95) | via helper |
| `inventory-design/SKILL.md` | not instructed | not instructed | not instructed | not instructed | via `inventory-metadata.sh` (L215-218) | via helper |
| `analyse-design-gaps/SKILL.md` | not instructed | not instructed | not instructed | not instructed | via `gap-metadata.sh` (L138-143) | via helper |
| `validate-plan/SKILL.md` (hybrid; excluded from final criterion) | hardcoded literal `skill: validate-plan` (L138) | not instructed | not instructed | not instructed | N/A | N/A |

This says clearly: **no skill instructs `producer`/`schema_version` by name today**, and only `research-codebase` names `last_updated`/`last_updated_by` (and only in the follow-up branch). The final acceptance criterion is therefore a real piece of work that requires editing every SKILL.md in the table to add explicit population instructions.

### Shared metadata helper(s) — divergence points

Three helpers exist:

- `scripts/artifact-derive-metadata.sh` (consumed by `create-adr`, `research-codebase`, `research-issue`) — emits `DATETIME_TZ`, `FILENAME_TS`, `GIT_COMMIT`, `GIT_BRANCH`, `REPO_NAME`. Uses `date '+%Y-%m-%d %H:%M:%S %Z'` (line 5), **not** ISO `+00:00`.
- `skills/design/inventory-design/scripts/inventory-metadata.sh` — same key set with jj-aware branch detection.
- `skills/design/analyse-design-gaps/scripts/gap-metadata.sh` — same shape.

None of the three emit `producer`, `schema_version`, `last_updated`, or `last_updated_by`. ADR-0033 mandates quoted ISO UTC timestamps everywhere — the helpers' `YYYY-MM-DD HH:MM:SS %Z` form is incompatible.

The minimum-disruption response: 0065 can either (a) update the helpers' output to ISO `+00:00` and rename their commit key to `REVISION`, or (b) leave the helpers and instruct the SKILL.md prose to run `date -u +%Y-%m-%dT%H:%M:%S+00:00` directly. (a) is structurally cleaner because the work-item skills already use `-u +%Y-%m-%dT%H:%M:%S+00:00` in prose (e.g. `create-work-item/SKILL.md:576`), and a single helper convergence ends the format divergence.

### Discovery pass — reproducible producer enumeration

A grep that the work item's acceptance criterion can quote verbatim:

```
rg -n "config-read-template\.sh|schema_version|last_updated|git_commit" skills --glob '**/SKILL.md'
rg -n  "verdict:|review_pass:|review_target:|pr_number:" skills --glob '**/SKILL.md'
```

Result split:

- **Template-based emitters (this story's territory):** `create-work-item`, `extract-work-items`, `create-plan`, `create-adr`, `extract-adrs` (no direct `config-read-template.sh` call — references `create-adr`'s template via prose), `research-codebase`, `research-issue`, `inventory-design`, `analyse-design-gaps`.
- **Inline-only emitters (0066's territory, excluded from this story's discovery list):** `review-plan/SKILL.md` (L424-426), `review-work-item/SKILL.md` (L359-361), `review-pr/SKILL.md` (L457-460), `validate-plan/SKILL.md` (L136-141).
- **Hybrid emitters (template body + inline frontmatter):** `describe-pr/SKILL.md` (L99-107 inline ON TOP of `templates/pr-description.md`), `validate-plan/SKILL.md` (L117 template body, L136-141 inline frontmatter — overlaps with the four-skill list).
- **Non-emitter template consumers:** `refine-work-item`, `update-work-item`, `list-work-items` (read templates for schema reference; don't write new artifacts).

The crucial discovery: `describe-pr` is **not** in the four-skill exclusion list. Its inline frontmatter sits underneath the unified template's block and currently overrides it (because the SKILL.md's instruction at L99-107 enumerates the frontmatter fields the model should emit, distinct from whatever's in the loaded template). 0065 must change `describe-pr`'s prose so its emission aligns with the updated `templates/pr-description.md` — otherwise the new template's `producer`/`schema_version`/`last_updated_by`/`revision`/`repository` will never reach disk.

### Corpus reality — partial-rename evidence

The `meta/` corpus has not stayed in lockstep with the upstream renames. Notable divergences (which are 0070's migration problem, not 0065's, but inform the template work):

- Plans created after 0064 shipped still emit `work-item: "<id>"` (hyphenated) rather than `work_item_id: "<id>"` — see `meta/plans/2026-05-26-0088-markdown-body-width-harmonisation.md:5`. The template uses `work_item_id`; the producer skill's value substitution must be regressing. Worth a sanity-check on `create-plan/SKILL.md` while making other changes there.
- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md:5` still uses `researcher: Toby Clemson` rather than `author:`. The template (`templates/codebase-research.md:2`) says `author:`; the divergence again sits in the producer flow.
- Reviews and validations have field/body verdict disagreements (`verdict: APPROVE` in frontmatter, body text says REVISE) — unrelated to this story but documents the brittleness of inline-emission that 0066 will fix.

### Upstream-story handoff (per parallel analyser pass)

- **0063 (done)**: Already renamed `type:` → `kind:` in `templates/work-item.md`. 0065 must NOT touch that.
- **0064 (done)**: Already renamed plan `work-item:` → `work_item_id:` and research `researcher:` → `author:`. 0065 must NOT re-rename those.
- **0064 technical note (relevant)**: "migrating a work-item's **own** identity field `work_item_id` → `id` … lands via 0065 (templates) and the corpus migration (0070)." This confirms the own-identity rename is 0065's, even though 0064 renamed foreign references.
- **0066 (draft)**: Owns the four review/validation skills' inline-frontmatter rewiring AND **creates** the three new review templates (`plan-review`, `work-item-review`, `pr-review`). 0065 does NOT create those.
- **0066 prerequisite**: "0065 adds the unified frontmatter block to `templates/validation.md`" — this is the hard handoff. 0065 must complete the validation.md frontmatter block before 0066 can rewire `validate-plan`.
- **0070 (draft)**: Will migrate every existing `meta/` artifact to the unified schema. 0065 must leave `meta/` files untouched — they are 0070's exclusive territory.

### Migration framework constraint

`meta/decisions/ADR-0023-meta-directory-migration-framework.md` and successors (ADR-0037, ADR-0038) gate how the corpus migration runs. Templates change isn't a framework concern, but the template's `schema_version: 1` value is what 0070's migration will write into corpus files, so getting the value right (integer `1`, not quoted) is load-bearing.

## Code References

- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md:113-225` — base schema, provenance bundle, identity-value contract, schema versioning, per-type extras (authoritative)
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md:44-108` — linkage keys, reference value shape, design-gap inventory keys
- `templates/work-item.md:1-11` — current work-item frontmatter (uses `work_item_id` as own identity; needs → `id`)
- `templates/plan.md:1-7` — sparsest current frontmatter (needs largest delta: `producer`, `id`, `title`, `author`, `tags`, `last_updated`, `last_updated_by`, `schema_version`, provenance bundle, `reviewer`)
- `templates/validation.md:1-3` — body-only; needs full frontmatter block added
- `templates/pr-description.md:1-8` — needs `producer` rename, `id`, `title`, `author`, `tags`, `last_updated`/`last_updated_by`, `schema_version`, provenance bundle, `pr_url`, `merge_commit`
- `templates/adr.md:1-8` — own-identity rename (`adr_id` → `id`), add `type`, `producer`, `title`, `last_updated`/`last_updated_by`, `schema_version`, `decision_makers`; reshape `supersedes` to typed-linkage form
- `templates/codebase-research.md:1-12` — rename `git_commit` → `revision`, drop `branch`, add `type`, `id`, `title`, `producer`, `schema_version`, quote ISO timestamps
- `templates/rca.md:1-12` — same shape as codebase-research; same gap; `type: issue-research`
- `templates/design-inventory.md:1-17` — keep domain fields; add `id`, `title`, `producer`, `schema_version`; rename `git_commit` → `revision`, drop `branch`
- `templates/design-gap.md:1-9` — add `id`, `title`, `producer`, `last_updated`/`last_updated_by`, `schema_version`; keep `current_inventory`/`target_inventory`
- `scripts/artifact-derive-metadata.sh:5-12` — current shared metadata helper; format mismatch (non-ISO) and missing keys (producer, schema_version, last_updated_*)
- `skills/design/inventory-design/scripts/inventory-metadata.sh` — jj-aware variant; same gaps
- `skills/design/analyse-design-gaps/scripts/gap-metadata.sh` — jj-aware variant; same gaps
- `skills/work/create-work-item/SKILL.md:576` — `date -u +%Y-%m-%dT%H:%M:%S+00:00` instruction (only the work-item skills currently use ISO format)
- `skills/planning/create-plan/SKILL.md:224` — loads `plan` template; **no metadata-capture instructions anywhere** (largest single SKILL.md gap)
- `skills/github/describe-pr/SKILL.md:99-107` — inline frontmatter emission that conflicts with the template's block (hybrid producer — needs change in this story)
- `skills/research/research-codebase/SKILL.md:113-114, 152-154` — only skill that names `last_updated`/`last_updated_by` in prose (and only on the follow-up flow)
- `skills/planning/validate-plan/SKILL.md:117, 136-141` — hybrid (template body + inline frontmatter); inline side is 0066's, template-body frontmatter block is 0065's
- `skills/planning/review-plan/SKILL.md:424-426`, `skills/work/review-work-item/SKILL.md:359-361`, `skills/github/review-pr/SKILL.md:457-460` — three review skills with inline-only frontmatter (0066's; explicitly excluded from 0065's discovery list)

## Architecture Insights

- **Template-as-source-of-truth pattern**: All template-based producers route through `config-read-template.sh <name>`, which embeds the template inline at SKILL parse time. This makes template edits propagate to consumers without code changes — a real architectural lever for this story. The story can deliver most of its surface area through pure template-file edits; SKILL.md changes are needed only where the prose has to *instruct* the model to compute and substitute new dynamic values (timestamps, commit IDs, repo names).
- **Producer-emitted vs. template-emitted fields**: Some fields can never be template-static (`date`, `last_updated`, `revision`, `repository`, `author` from VCS) — they MUST be supplied at producer time. Other fields can be template-static (`type`, `producer`, `schema_version`). This bifurcation determines where the work lives:
  - **Template-static** (just edit the template): `type`, `producer`, `schema_version`.
  - **Producer-substituted via existing helper**: `date` (format change required), `revision` (rename from `git_commit`), `repository` (already emitted).
  - **Producer-substituted with no existing instruction**: `last_updated`, `last_updated_by`, `id` for types that don't have one today (plans, validations, pr-descriptions, research, inventories, gaps). These need new SKILL.md prose.
- **Hybrid producers are an anti-pattern in flight**: `describe-pr` and `validate-plan` both consume a template body AND emit inline frontmatter. The inline emission overrides the template's block. 0066 fixes `validate-plan`; 0065 must fix `describe-pr` (or accept that `pr-description` artifacts will keep diverging from the template until a future story addresses it).
- **Identity-value quoting contract**: Already enforced for work-item identity per `skills/config/configure/SKILL.md`; ADR-0033 generalises it. Practical consequence: every template that today emits an unquoted `id`-shaped value (ADR's `adr_id: ADR-NNNN`, plan placeholders) must switch to quoted form.

## Historical Context

- `meta/decisions/ADR-0028-common-frontmatter-schema-for-meta-artifacts.md` — Defined the prior minimal base (`date`, `type`, `skill`, `status`). ADR-0033 supplements it: keeps `date`/`type`/`status`, renames `skill` → `producer`, adds the wider base set.
- `meta/work/0021-artifact-persistence-lifecycle.md`, `meta/work/0022-artifact-metadata-and-lifecycle.md`, `meta/work/0023-adr-system-design.md` — Produced ADRs 0027/0028 and the 0029-0032 series. ADR-0033's "Decision" section explicitly preserves their validity ("Work items 0021, 0022, 0023 remain valid; this ADR is the supplementing decision.").
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` (in-progress) — Parent epic; specifies the own-identity-rename split between 0065 and 0070.
- `meta/work/0060-adr-unified-base-frontmatter-schema.md` (done) — Source task for ADR-0033.
- `meta/work/0061-adr-typed-linkage-vocabulary.md` (done) — Source task for ADR-0034.
- `meta/work/0063-rename-work-item-type-to-kind.md` (done) — Already renamed `type:` → `kind:` in `templates/work-item.md`; 0065 must not touch.
- `meta/work/0064-canonicalise-work-item-id-and-author-fields.md` (done) — Already renamed plan `work-item:` → `work_item_id:` and research `researcher:` → `author:`. Its technical note explicitly hands the own-identity rename (`work_item_id` → `id` on work-item; `adr_id` → `id` on ADR) to 0065.
- `meta/work/0066-update-review-skills-inline-frontmatter.md` (draft) — Owns the four review/validation skills' inline-frontmatter rewiring and the three new review template files.
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` (draft) — Owns the corpus rewrite; 0065 must leave `meta/` files alone.

## Related Research

- This is the first dedicated codebase research for work item 0065. Adjacent research bearing on the visualiser-graph consumer (the downstream beneficiary of the typed-linkage shape) is summarised in `meta/research/codebase/2026-02-22-pr-review-agents-design.md` and the visualiser-area `meta/research/codebase/2026-05-26-0088-markdown-body-width-harmonisation.md`.

## Open Questions

1. **`pr_title` field's fate.** ADR-0033's per-type extras for `pr-description` list only `pr_url`, `pr_number`, `merge_commit`. The current template has `pr_title`. Does it survive (as a producer-supplied convenience) or migrate into the base `title:` field? The work item's schema reference table doesn't say. Recommend: migrate `pr_title` → base `title:` (cleaner) since `title` is now a base field.
2. **Plan own-identity value shape.** Plans don't have a natural ID today; their filename encodes it (`2026-05-26-0088-markdown-body-width-harmonisation.md`). ADR-0033 says "the natural ID where it has one, or a slug/path-derived value otherwise". Options: (a) `id: "0088-markdown-body-width-harmonisation"` (slug-from-filename), (b) `id: "2026-05-26-0088-markdown-body-width-harmonisation"` (full filename stem). The work item doesn't pin this. Recommend resolving before implementation so the create-plan skill knows what to write.
3. **Helper convergence vs. SKILL prose timestamp instructions.** Two architectural choices for unifying the timestamp format:
   - (a) Update `scripts/artifact-derive-metadata.sh` and its two siblings to emit ISO `+00:00` and rename `GIT_COMMIT` → `REVISION`; consuming SKILL.md prose stays simple.
   - (b) Leave helpers alone; instruct each SKILL.md to run `date -u +%Y-%m-%dT%H:%M:%S+00:00` directly (matching the work-item skills' current pattern).
   The work item's acceptance criteria don't pin either. (a) is structurally cleaner but touches scripts; (b) keeps the change surface inside `templates/` + SKILL.md but duplicates the date instruction.
4. **Should `describe-pr/SKILL.md` be folded explicitly into 0065's scope?** The work item's Assumptions section says "Inline frontmatter generators in the four review/validation skills … are 0066's scope; any *other* inline producer discovered is this story's scope." `describe-pr` is a hybrid (template body + inline frontmatter) — by the assumption it IS this story's. Worth confirming in the implementation plan to avoid scope ambiguity.
5. **`extract-adrs` template-update propagation.** `extract-adrs/SKILL.md` doesn't call `config-read-template.sh` directly — it tells the model "Use the template exactly as defined in the `create-adr` skill". If 0065 updates `templates/adr.md`, `extract-adrs` picks the change up by indirection. Confirm this is intentional (vs. needing an explicit `config-read-template.sh adr` call added to `extract-adrs`).

## References

- Source work item: `meta/work/0065-update-artifact-templates-to-unified-schema.md`
- Authoritative schema ADR: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
- Authoritative linkage ADR: `meta/decisions/ADR-0034-typed-linkage-vocabulary.md`
- Parent epic: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Sibling dependencies: 0060 (done), 0061 (done), 0063 (done), 0064 (done), 0066 (draft, blocks/blocked-by 0065), 0067 (draft, owns templates/note.md), 0070 (draft, corpus migration)
