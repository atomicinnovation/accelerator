---
id: "0093"
title: "Extend Templates With Typed-Linkage Slots"
date: "2026-05-31T12:13:53+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: story
priority: medium
parent: "work-item:0057"
tags: [templates, frontmatter, schema, linkage, emission]
last_updated: "2026-06-03T21:19:47+00:00"
last_updated_by: Toby Clemson
schema_version: 1
type: work-item
blocked_by: ["work-item:0061", "work-item:0034", "work-item:0065", "work-item:0066", "adr:ADR-0034"]
blocks: ["work-item:0070"]
relates_to: ["adr:ADR-0034", "adr:ADR-0033", "adr:ADR-0040", "work-item:0057", "work-item:0065", "work-item:0066", "work-item:0070"]
external_id: PP-115
---

# 0093: Extend Templates With Typed-Linkage Slots

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Extend every artifact template under `templates/` with empty optional slots for the typed-linkage keys defined in **ADR-0034** that the artifact type can legitimately carry, so newly-created artifacts can express cross-artifact links at draft time and downstream consumers (the corpus migration in 0070, and a future visualiser-graph epic — TBD) read the keys from a stable, documented template surface rather than inserting them ad hoc.

This story also adopts a corpus-wide **omit-when-empty** emission convention: producers emit an *optional non-base* frontmatter key only when it resolves to a non-empty value, and omit it entirely otherwise. This covers the new typed-linkage keys **and** the existing optional fields (`work_item_id`, `external_id`, `decision_makers`, and the lifecycle markers `reviewer`, `pr_url`, `merge_commit`). Templates keep every documented slot present-but-empty as the authoring surface; generated artifacts carry only the keys that have values. The convention is recorded in a new ADR (**ADR-0040**), supplementing ADR-0033, which mandates that base fields (incl. `tags`) remain present.

## Context

ADR-0034 (produced by 0061) defines the typed-linkage vocabulary: `parent`, `supersedes`/`superseded_by`, `blocks`/`blocked_by`, `target`, `derived_from`, `relates_to`, `source`, plus design-gap's type-specific `current_inventory`/`target_inventory`. The ADR also pins a type-pair semantic table enumerating which linkage edges each artifact type can legitimately participate in (e.g. `plan derived_from codebase-research`, `work-item source note`, `work-item blocks work-item`).

Story 0065 ("Update All Artifact Templates to Unified Schema") only carried forward linkage keys that were *already* present on legacy templates plus a small handful added incidentally during the rewrite:

- `parent` on work-item & plan (already present)
- `supersedes` on adr (reshape from single scalar to a list)
- `target` on validation (added new)
- `current_inventory` / `target_inventory` on design-gap (already present)

Its requirements §35 explicitly notes that the broader linkage vocabulary "is applied per that [ADR-0034] vocabulary, not this list" — but no story owns adding the remaining ADR-0034 keys to the templates. Story 0066 only adds linkage keys to the new review templates it creates (specifically `target`). Story 0070 (corpus migration) infers links from prose and writes them to existing artifacts but is not responsible for amending templates.

The 0057 epic's AC #114 ("Typed linkage keys (`parent`, `supersedes`/`superseded_by`, `blocks`/`blocked_by`, `target`, `derived_from`, `relates_to`, `source`) are documented and used where they apply") therefore hangs without a clear template-side owner. This story closes that gap.

## Decisions Made

The following decisions resolve open questions that the draft initially deferred. They are fixed inputs to the implementation:

- **Universal `parent`**: Per ADR-0034's "Corpus-wide — any artifact type may carry it" note on `parent`, every in-scope template carries `parent: ""`. Requirements §2 enumerates this on every template type.
- **Inverse keys (`superseded_by`, `blocked_by`)**: Templates expose both the canonical and inverse slots on **mutable** artifact types (work-item, plan), per ADR-0034's "both may be written on mutable artifacts" rule for `blocks`/`blocked_by`. ADRs are immutable once accepted, so the ADR template exposes only the canonical `supersedes` (the older ADR cannot be mutated to carry `superseded_by`); per ADR-0034 §"Single key sufficient for bidirectional pairs", consumers derive the inverse. The inline comment on inverse-key slots SHOULD note that producers prefer the canonical side.
- **Landing order vs 0066**: 0066 has shipped. All twelve in-scope templates (the nine frontmatter-bearing templates updated by 0065 plus the three review templates created by 0066) are within reach in a single sweep.
- **Omit-when-empty emission (widened scope)**: generated artifacts emit an optional non-base frontmatter key only when it has a non-empty value, and omit it otherwise — applied uniformly to the new typed-linkage keys, the foreign references (`work_item_id`, `external_id`), the optional extra `decision_makers`, and the lifecycle markers (`reviewer`, `pr_url`, `merge_commit`). Lifecycle markers are omitted-until-event (absence signals pending), not kept as empty placeholders. Base fields mandated by ADR-0033 (including `tags`) and always-valued per-type extras remain present.
- **New ADR (ADR-0040)**: the omit-when-empty convention is recorded as a new ADR supplementing ADR-0033, since it is a corpus-wide producer rule and neither ADR-0033 nor ADR-0034 states it today. (Supersedes the earlier "No new ADR" stance.)

## Requirements

- **Slot emission shape.** Add the typed-linkage slots listed in §2 below to each in-scope template. Slots are emitted present-but-empty:
  - Single-ref keys: `<key>: ""` with the comment `# typed-linkage ref: "<source-type>:NNNN" or ""`
  - List-cardinality keys: `<key>: []` with the comment `# typed-linkage list: ["<source-type>:NNNN", ...] or []`
  - `<source-type>` in the comment is illustrative of the expected target shape (e.g. `work-item`, `plan`, `adr`); the comment grammar above is normative and the template-shape test asserts this exact form.
  - Inverse-key slots (`superseded_by`, `blocked_by`) carry an additional trailing sentence in the comment: `Producers SHOULD prefer the canonical side ("supersedes" / "blocks").` *(Superseded by the implementation plan: the guidance moves to its own standalone comment line below the slot — so the gated list regex's `[]` end-anchor is preserved — and applies to `blocked_by` only; no template carries `superseded_by`, as ADRs are immutable. See the plan's "Emission model" supersedes note.)*
  - **This present-but-empty shape applies to templates only** — the documented authoring surface. Generated-artifact emission is governed by the omit-when-empty requirement below.
- **Omit-when-empty emission (generated artifacts; ADR-0040).** In generated artifacts, producers emit an optional non-base frontmatter key only when it resolves to a non-empty value, and omit it entirely otherwise. This covers the new typed-linkage keys **and** the existing optional fields `work_item_id`, `external_id`, `decision_makers`, `reviewer`, `pr_url`, `merge_commit`. Base fields mandated by ADR-0033 (`type`, `id`, `title`, `date`, `author`, `producer`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version`) and always-valued per-type extras remain present; `tags: []` stays. The convention is recorded in **ADR-0040** (new — see below).
- **Per-template expected slot set** (authoritative — this list is the closed set the template-shape test asserts against):
  - **work-item**: `parent: ""` (existing), `blocks: []`, `blocked_by: []`, `derived_from: []`, `relates_to: []`, `source: ""`
  - **plan**: `parent: ""` (existing), `blocks: []`, `blocked_by: []`, `derived_from: []`, `relates_to: []`
  - **adr**: `parent: ""`, `supersedes: []` (existing, list-shaped per 0065), `relates_to: []`. **No `superseded_by`** — ADRs are immutable once accepted; consumers derive the inverse from `supersedes` per ADR-0034.
  - **codebase-research**: `parent: ""`, `relates_to: []`
  - **rca** (`issue-research`): `parent: ""`, `relates_to: []`
  - **pr-description**: `parent: ""`, `relates_to: []`
  - **design-inventory**: `parent: ""`, `relates_to: []`
  - **design-gap**: `parent: ""`, `relates_to: []`. Retains its type-specific `current_inventory` / `target_inventory` keys verbatim per ADR-0034 §"Design-gap inventory keys".
  - **plan-validation**: `parent: ""`, `target: ""` (existing), `relates_to: []`
  - **plan-review**: `parent: ""`, `target: ""` (existing per 0066), `relates_to: []`
  - **work-item-review**: `parent: ""`, `target: ""` (existing per 0066), `relates_to: []`
  - **pr-review**: `parent: ""`, `target: ""` (existing per 0066), `relates_to: []`
- **Consuming SKILL.md updates.** Update the Populate-frontmatter step of every SKILL.md that produces an in-scope artifact, so the snippet names every omit-when-empty field — the new typed-linkage slots plus that artifact's optional foreign-ref / lifecycle fields — alongside existing fields, with a one-line guidance note containing the words "fill" or "omit". Omission is the default — producers fill a field only when it has a value at draft time, and omit it otherwise. The affected SKILL.md files (closed set, fifteen sites — confirmed to exist as of 2026-06-02) are:
  - `skills/work/create-work-item/SKILL.md` (work-item)
  - `skills/work/refine-work-item/SKILL.md` (work-item)
  - `skills/work/extract-work-items/SKILL.md` (work-item)
  - `skills/planning/create-plan/SKILL.md` (plan)
  - `skills/decisions/create-adr/SKILL.md` (adr)
  - `skills/decisions/extract-adrs/SKILL.md` (adr)
  - `skills/research/research-codebase/SKILL.md` (codebase-research)
  - `skills/research/research-issue/SKILL.md` (rca / issue-research)
  - `skills/github/describe-pr/SKILL.md` (pr-description)
  - `skills/design/inventory-design/SKILL.md` (design-inventory)
  - `skills/design/analyse-design-gaps/SKILL.md` (design-gap)
  - `skills/planning/validate-plan/SKILL.md` (plan-validation)
  - `skills/planning/review-plan/SKILL.md` (plan-review)
  - `skills/work/review-work-item/SKILL.md` (work-item-review)
  - `skills/github/review-pr/SKILL.md` (pr-review)

  The sweep reuses the canonical Populate-frontmatter snippet shape established in 0065's plan (Implementation Approach §Canonical persistence-step prose snippet); per-skill divergence from that shape is not permitted. The four reviewer skills (`validate-plan`, `review-plan`, `review-work-item`, `review-pr`), which today fold population into prose, gain a literal `Populate frontmatter` heading so the grep/section check has a section to assert against.
- **Schema mirror.** Extend `scripts/templates-schema.tsv` with a **seventh column** named `typed_linkage_keys` (space-separated list of expected linkage-key names per template) — the seventh-column form is mandatory; the extras-list alternative is rejected. Extend `scripts/skills-schema.tsv` with an `omit_when_empty` column (space-separated field names per skill) and assert fill/omit guidance via `scripts/test-skill-frontmatter-population.sh` (folding AC #3's check into the existing skill-population test rather than a standalone grep).
- **Template-shape test.** Extend `scripts/test-template-frontmatter.sh` to assert, for every in-scope template:
  - (a) every expected linkage slot is present;
  - (b) value shape matches the key's cardinality (single-ref keys default to `""`, list keys default to `[]`) — cardinality lookup comes from a small in-script map keyed by linkage-key name;
  - (c) the inline comment matches the normative grammar in §1 exactly (a regex per cardinality, with the inverse-key trailing sentence required for `superseded_by` / `blocked_by` slots);
  - (d) **no template carries a linkage key not listed in its TSV row** (closed-set assertion — guards against spurious slots).
- **No corpus migration.** Existing artifacts under `meta/` keep their current shape; populating the new slots on legacy artifacts is 0070's territory (it already infers links from prose). 0070's inferred-link writes follow the omit-when-empty convention.
- **New ADR (ADR-0040).** This story introduces **ADR-0040** recording the omit-when-empty emission convention (supplements ADR-0033). ADR-0034 remains the typed-linkage authority; this story implements its template-side projection plus the emission convention. ADR-0040 must be `accepted` before the producer-skill sweep lands.

## Acceptance Criteria

- [ ] Every in-scope template (the twelve listed in Requirements §2) carries exactly the typed-linkage slots that Requirements §2 enumerates for its type, present-but-empty. Requirements §2 is the closed set; no additions, no omissions.
- [ ] `scripts/test-template-frontmatter.sh` passes with new per-template assertions for: (a) each expected linkage slot is present, (b) value shape matches the key's cardinality, (c) the inline comment matches the normative grammar in Requirements §1 exactly, (d) no template carries a linkage key not listed in its TSV row.
- [ ] Each SKILL.md listed in Requirements §3 has its Populate-frontmatter step naming every omit-when-empty field, with a one-line guidance note containing "fill" or "omit". A check (each field name appears in the Populate-frontmatter section of every SKILL.md that produces an artifact carrying that field, accompanied by fill/omit guidance) passes for every (field, SKILL.md) pair — enforced by the new `omit_when_empty` column in `scripts/skills-schema.tsv` and `scripts/test-skill-frontmatter-population.sh`. The four reviewer skills carry a literal `Populate frontmatter` heading.
- [ ] Generated artifacts omit optional non-base keys (the typed-linkage keys, `work_item_id`, `external_id`, `decision_makers`, `reviewer`, `pr_url`, `merge_commit`) when empty; base fields (incl. `tags`) and always-valued per-type extras remain present. Asserted by the producer-skill fill/omit guidance and the `omit_when_empty` test column.
- [ ] **ADR-0040** (omit-when-empty frontmatter emission; supplements ADR-0033) exists and is `accepted`, and its scope table matches the emission classification used by the templates and producer skills.
- [ ] `templates/design-gap.md` retains its type-specific `current_inventory` / `target_inventory` keys verbatim; they are not folded into the generic vocabulary. (ADR-0034 records this carve-out under §"Design-gap inventory keys", which states: "`current_inventory` / `target_inventory` remain type-specific keys on design-gap, not folded into the generic vocabulary.")

## Dependencies

- Blocked by: 0061 (ADR-0034 typed-linkage vocabulary — done), 0065 (unified-schema templates — this story extends them), 0066 (creates the three review templates this story also extends — done as of 2026-06-02, so all twelve in-scope templates are reachable in a single sweep).
- External artefact: **ADR-0034** (`meta/decisions/ADR-0034-typed-linkage-vocabulary.md`) — the type-pair table and key-cardinality definitions are the binding contract. Amendments to ADR-0034 mid-flight trigger a re-audit of Requirements §2.
- External artefact: **ADR-0033** (`meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`) — mandates the base-field set is present on every artifact (`tags` possibly empty); this constrains the omit-when-empty convention to *optional non-base* fields only.
- Introduces: **ADR-0040** (`meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`) — authored by this story; must be `accepted` before the producer-skill sweep lands.
- Blocks: 0070 (corpus migration's link-inference can target stable template slots rather than inserting undocumented keys; its writes follow the omit-when-empty convention).
- Related: 0057 (parent epic — closes AC #114's template-side gap). A future visualiser-graph epic (TBD, not yet tracked) is a downstream consumer once it is drafted.

## Assumptions

- ADR-0034 is the authoritative input. New edges added to the ADR in future trigger a template-side update, but this story takes the table as currently written. Requirements §2 reflects the ADR as of work-item drafting; if the ADR is amended before implementation, §2 must be re-audited.
- Consumers (0070's link inference, future render layers, any future visualiser-graph work) read linkage keys from the unified base + linkage slots, not from per-type per-key heuristics. Keys absent from a template's frontmatter are treated as absent edges, not missing data.
- Producers populate slots only when the link is explicit at draft time; speculative or inferred links remain in body prose and are picked up by 0070's inference pass.

## Technical Notes

- The template-shape test (`scripts/test-template-frontmatter.sh`) already loads a per-template TSV (`scripts/templates-schema.tsv`) with extras assertions. This story adds a mandatory seventh column `typed_linkage_keys` (space-separated key names) and asserts each slot is present, value-shaped per cardinality, and comment-formed correctly. Cardinality lookup comes from a small in-script map keyed by linkage-key name. The closed-set check (Requirements §5(d)) walks the template's frontmatter and fails on any linkage-key name absent from the TSV row.
- The canonical Populate-frontmatter snippet documented in 0065's plan (Implementation Approach §Canonical persistence-step prose snippet) is the right place to slot the linkage-key guidance. Each per-skill phase extends its snippet instance with the new bullets, keeping the snippet's shape uniform. The SKILL.md sweep is mechanical (one canonical snippet × fifteen sites); per-skill divergence is rejected at review time.

## Drafting Notes

- Drafted as a follow-up after 0065's pass-3 review: the typed-linkage application surface was found to be split inconsistently across 0061, 0065, 0066, and 0070, with no story owning the empty-slot template extension. The 0057 epic's AC #114 is the ambiguous line ("documented and used *where they apply*") — this story interprets "used" as "templates expose empty slots so producers and migration can fill them".
- Priority is `medium` rather than `high` because newly-created artifacts can still link via body prose today and 0070's inference will pick those up; the cost of waiting is reader convenience and visualiser-graph readability, not a hard blocker.
- Sized as a story rather than a task because the per-template audit against ADR-0034's type-pair table and the test-extension work are non-trivial; a spike could precede if the type-pair coverage turns out to be larger than expected.

## References

- Parent epic: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` (closes AC #114's template-side gap)
- Authoritative linkage ADR: `meta/decisions/ADR-0034-typed-linkage-vocabulary.md`
- Predecessor: `meta/work/0065-update-artifact-templates-to-unified-schema.md` (added the unified base; this story adds the linkage slots)
- Related: `meta/work/0066-update-review-skills-inline-frontmatter.md` (creates the three review templates this story also extends — done)
- Related: `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` (corpus migration; consumes the new slots as targets for link inference)
