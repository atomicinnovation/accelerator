---
id: "0066"
title: "Move Review/Validation Skills' Frontmatter into Templates on Unified Schema"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: story
status: done
priority: medium
parent: "work-item:0057"
tags: [review-skills, frontmatter, schema]
type: work-item
schema_version: 1
last_updated: "2026-05-17T17:16:35+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0060", "work-item:0061", "work-item:0065"]
blocks: ["work-item:0070"]
relates_to: ["work-item:0057", "work-item:0064", "work-item:0065", "adr:ADR-0033", "adr:ADR-0034"]
---

# 0066: Move Review/Validation Skills' Frontmatter into Templates on Unified Schema

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Move the frontmatter that the review and validation skills (`review-plan`, `review-work-item`, `review-pr`, `validate-plan`) currently bake inline in their SKILL.md prose **into template files under `templates/`**, conforming to the unified base schema and typed linkage vocabulary, and rewire each skill to read its frontmatter from the template rather than emitting it inline. This brings these four producers onto the same template-based footing as every other producer (0065), so a single future schema change touches only template files.

## Context

Per 0057, four skills currently bake frontmatter field shapes directly into their SKILL.md prose rather than reading from a template under `templates/`: `review-plan`, `review-work-item`, `review-pr`, `validate-plan`. Their emitted frontmatter is therefore not updated by 0065 and needs a dedicated pass.

The epic's technical notes raised extracting these into shared template files as an *optional* simplification. **That option is now a decision**: this story moves the inline frontmatter into templates rather than merely rewriting the inline prose. Three of these artifact types have no template file today (`plan-review`, `work-item-review`, `pr-review`) and are created here; `plan-validation` already has a (body-only) `templates/validation.md`, to which 0065 adds the unified frontmatter block — this story rewires `validate-plan` to read it.

## Requirements

- Create template files under `templates/` for the three review artifact types that lack one — `plan-review`, `work-item-review`, `pr-review` — each emitting the unified base schema. (`plan-validation` reuses `templates/validation.md`, whose frontmatter block is added by 0065.)
- Each template must emit the unified base fields: `type`, `id` (own identity), `title`, `date`, `author`, `producer`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version` (value `1` per ADR-0033). Identity values are quoted YAML strings; foreign references use `<snake_case_type>_id`.
- Apply per-artifact extras per ADR-0033: every review template (`plan-review`, `work-item-review`, `pr-review`) carries `reviewer`, `verdict`, `lenses`, `review_number`, `review_pass`; `plan-validation` carries `result`. Per the typed-linkage vocabulary (ADR-0034), every review/validation also carries `target` as a single quoted YAML string in `"doc-type:id"` form pointing at its subject — `review-plan` and `validate-plan` emit `"plan:<id>"`, `review-work-item` emits `"work-item:<id>"`, and `review-pr` emits `"pr:<pr-number>"` referencing the external PR being reviewed (not the pr-description artifact — the review is about the PR itself).
- All extras are populated at artifact creation in a single write — the rewired skills compose the full frontmatter (base fields plus all extras above) and emit it once when the review/validation completes. No field is emitted present-but-empty.
- Rewire `review-plan`, `review-work-item`, `review-pr`, and `validate-plan` to read their frontmatter from the corresponding template via the canonical template-reading helper (`config-read-template.sh`) rather than baking field shapes into SKILL.md prose, and to populate the field values (including `producer`, `schema_version`, `last_updated`, `last_updated_by`, `target`, `reviewer`, `verdict`, `lenses`).
- For `validate-plan` specifically: read frontmatter from `templates/validation.md` (populated by 0065) instead of the inline block currently in its SKILL.md.

## Acceptance Criteria

- [ ] Template files exist under `templates/` for `plan-review`, `work-item-review`, and `pr-review`, each emitting the unified base fields (`type`, `id`, `title`, `date`, `author`, `producer`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version: 1`).
- [ ] `review-plan`, `review-work-item`, and `review-pr` read their frontmatter from the corresponding new template under `templates/` via the canonical template-reading helper (`config-read-template.sh`); verified by grep returning zero matches for `^type:`, `^schema_version:`, `^verdict:` in each SKILL.md outside fenced template-example blocks.
- [ ] `validate-plan` reads its frontmatter from `templates/validation.md` (whose frontmatter block is added by 0065) via the canonical template-reading helper rather than emitting it inline; verified by grep returning zero matches for `^type:`, `^schema_version:`, `^result:` in `validate-plan`'s SKILL.md outside fenced template-example blocks.
- [ ] SKILL.md for each of the four affected skills contains no inline YAML frontmatter block enumerating the unified base or extra fields as `key: value` pairs outside fenced template-example blocks; narrative references to individual field names when discussing population logic are permitted. Adding a new base field requires editing only the template(s), not SKILL.md prose.
- [ ] Each review template emits the per-ADR-0033 review extras: `reviewer`, `verdict`, `lenses`, `review_number` on all three review types (`plan-review`, `work-item-review`, `pr-review`); `review_pass` on `plan-review` and `work-item-review` only (`pr-review` omits `review_pass` — see plan §Design Decisions #1 — until a future story introduces a re-review lifecycle for the skill). Each review/validation template additionally emits `target` (per ADR-0034's typed-linkage vocabulary) as a single quoted YAML string in `"doc-type:id"` form — `"plan:<id>"` for `plan-review` and `plan-validation`, `"work-item:<id>"` for `work-item-review`, `"pr:<pr-number>"` for `pr-review` (regex: `^"pr:[0-9]+"$`).
- [ ] Plan-validation artifacts carry the `result` extra per ADR-0033.
- [ ] Identity values (`id` and any foreign references) in emitted frontmatter are quoted YAML strings.
- [ ] Generating one review/validation artifact via each of the four rewired skills yields non-empty values containing no unsubstituted template tokens for `producer`, `schema_version` (=1), `last_updated`, `last_updated_by`, `target`, `reviewer`, `verdict`, `lenses`, `review_number`, and `review_pass` (and `result` for `validate-plan`); both ISO-UTC timestamps parse.
- [ ] A reproducible discovery pass (recorded grep command and matched files) confirms the four named skills are the only inline producers of `plan-review`, `work-item-review`, `pr-review`, and `plan-validation` frontmatter, or any additional producer found is folded into scope. The exact grep recipe and producer split are captured in §"Discovery Pass Record" below.

## Open Questions

- None — the prior open question (extract into templates?) was resolved by user decision; see Drafting Notes.

## Dependencies

- Blocked by: 0060 (base schema), 0061 (linkage vocabulary), 0065 (adds the unified frontmatter block to `templates/validation.md`, which this story rewires `validate-plan` to read).
- Blocks: 0070 (corpus migration); future visualiser-graph epic (consumes the typed-linkage `target` shape this story finalises on review/validation artifacts).
- Related: 0057 (parent epic), 0064 (canonicalised the `work_item_id` foreign-reference shape this story emits on `work-item-review` artifacts), 0065 (template-based producer updates — owns the template *files*; this story owns the skill-side rewiring and the three new review templates).

## Assumptions

- The four named skills are the only inline producers of artifacts with `type:` values `plan-review`, `work-item-review`, `pr-review`, and `plan-validation`. If other surfaces emit the same frontmatter, this story's scope expands.
- Verdict-enum alignment stays explicitly out of scope per the epic.

## Technical Notes

- The epic left template extraction optional; this story now mandates it (per user decision). Moving the frontmatter into templates means a future schema change touches only `templates/` files, not skill prose — the same maintenance property every other producer already has after 0065.
- `templates/validation.md` is today a body-only report template that `validate-plan` reads for the report structure while emitting frontmatter inline. 0065 adds the frontmatter block to that file; this story changes `validate-plan` to read the frontmatter from it too. The two stories therefore touch the same artifact pipeline from different angles (0065 edits the template; 0066 edits the skill that reads it) — hence the 0065→0066 ordering.
- The rewired skills read their template frontmatter via the canonical template-reading helper (`config-read-template.sh`), the same helper template-based emitters already use after 0065. Review/validation artifacts are written once per review pass with every base field and extra populated (re-reviews bump `review_pass` and rewrite the artifact rather than mutating fields in place), so no lifecycle-gated present-but-empty fields are required from the helper. If the helper nonetheless needs a small extension to handle the unified review-extras shape, that extension is in-scope for this story rather than a separate follow-up.

## Drafting Notes

- Set priority to `medium` rather than `high` because these skills can keep functioning with their current inline frontmatter until the migration runs; the high-priority dependencies are the ADRs and the corpus migration.
- Scope changed per user decision from "rewrite inline frontmatter in prose" to "move frontmatter into template files and rewire skills to read them". This resolves the prior open question (extract into templates? — yes, under `templates/`) and adds creation of the three missing review templates plus a 0065 dependency for `validation.md`.
- If verdict-enum inconsistency (`REVISE` vs `REQUEST_CHANGES`) is observed across the four skills during rewiring, log it as a follow-up under 0057 rather than addressing it inline; enum alignment is explicitly excluded from this story's scope by the parent epic.
- `pr-review` emits `target: "pr:<pr-number>"`, which uses a `pr` doc-type prefix that is not in ADR-0034's published vocabulary today (the listed discriminators come from ADR-0033 and cover meta-artifact types only; external PRs are not meta). Using `pr:` keeps the `target` data model uniform across all four review/validation types — important for the future visualiser-graph epic — but should be formalised by a small ADR-0034 follow-up that adds `pr` (and any other external-entity prefixes the corpus needs) to the vocabulary. Log this as a follow-up under 0057 rather than blocking this story.
- Originally extracted from source documents without interactive enrichment; refined during 0065's review when the `templates/validation.md` boundary surfaced.

## Schema Reference

The three new template files created by this story emit the unified base
schema plus per-type extras per ADR-0033 and a `target` typed-linkage key
per ADR-0034. Authoritative source: ADR-0033 and ADR-0034. On any
discrepancy the ADRs win and this table should be re-synced.

| Template file         | Artifact `type`    | `schema_version` | Provenance bundle? | Per-type extras (beyond base)                                                                                                                                                                                                                                                                                                                                                                   |
|-----------------------|--------------------|------------------|--------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `plan-review.md`      | `plan-review`      | 1                | no                 | `reviewer`, `verdict`, `lenses`, `review_number`, `review_pass`, `target` (= `"plan:<id>"`)                                                                                                                                                                                                                                                                                                     |
| `work-item-review.md` | `work-item-review` | 1                | no                 | `reviewer`, `verdict`, `lenses`, `review_number`, `review_pass`, `target` (= `"work-item:<id>"`), `work_item_id` (transitional alias — see plan §Design Decisions #2; consumed by visualiser frontmatter.rs:330 until Phase 7)                                                                                                                                                                  |
| `pr-review.md`        | `pr-review`        | 1                | no                 | `reviewer`, `verdict`, `lenses`, `review_number`, `target` (= `"pr:<pr-number>"`; the `pr` prefix is queued for inclusion in ADR-0034's vocabulary via supplementary ADR — see follow-up under `meta/work/0057-...md`), `pr_number` (bare integer; foreign reference to the external PR per ADR-0033 §Identity-value shape contract). `review_pass` is omitted — see plan §Design Decisions #1. |

## Discovery Pass Record

Commands executed (run from the workspace root, after Phases 1-5 have landed):

```
# Pass A — template-using and unified-schema-emitting producers
rg -n "config-read-template\.sh|^[[:space:]]*producer:|^[[:space:]]*schema_version:" skills --glob '**/SKILL.md'

# Pass B — legacy inline-frontmatter emitters (now empty for 0066 scope)
rg -n "verdict:|review_pass:|review_target:|^[[:space:]]*target:|^[[:space:]]*result:|pr_number:" skills --glob '**/SKILL.md'
```

Pass A surfaces every skill that reads a template via the canonical
loader or directly emits a unified base field. Pass B surfaces every
SKILL.md that mentions a review/validation extra literal — post-0066,
those literals appear only inside fenced template-example blocks, but
the discovery patterns still match them; every Pass-B hit must resolve
to a SKILL.md in `IN_SCOPE_PRODUCERS`.

Producer split (post-0066):

- **Unified template-based emitters (10 from 0065 + 4 from 0066 = 14
  total)**: create-work-item, extract-work-items, create-plan,
  describe-pr, create-adr, extract-adrs, research-codebase,
  research-issue, inventory-design, analyse-design-gaps, **review-plan,
  review-work-item, review-pr, validate-plan**.
- **Inline-only emitters owned by 0066**: NONE (formerly:
  review-plan, review-work-item, review-pr, validate-plan).
- **Non-emitter template consumers**: refine-work-item, update-work-item,
  list-work-items.

Other inline producers found: NONE. The Phase-6 grep recipe and the
existing test driver's discovery assertion together form the
reproducible verification of work item AC #9.

For the `pr-review.target` regex shape pinned by Design Decision #3:

```
rg -n "^target:[[:space:]]+\"pr:[0-9]+\"" templates/pr-review.md
# Expected: zero matches (the template carries `target: ""` empty slot).

# Manual: against an artifact produced by `review-pr`, the same regex
# should match exactly once in the frontmatter block.
```

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0057, 0060, 0061, 0064, 0065, 0070
- Related: ADR-0033 (per-artifact extras, schema_version), ADR-0034 (typed linkage vocabulary, `target` reference shape)
