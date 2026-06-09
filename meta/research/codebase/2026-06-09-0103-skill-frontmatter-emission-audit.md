---
type: codebase-research
id: "2026-06-09-0103-skill-frontmatter-emission-audit"
title: "Research: Skill Frontmatter Emission Audit Against the Unified Schema (work item 0103)"
date: "2026-06-09T18:44:38+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0103"
parent: "work-item:0103"
topic: "How skills emit artifact frontmatter, what the unified-schema validator enforces, the known validate-plan divergence, and how to wire an automated producer-conformance guard"
tags: [research, codebase, frontmatter, schema, validator, skills, validate-plan, test-harness, mise]
revision: "dc9490fb6caadb129f9899e48d6e5e300ec2d663"
repository: "ticket-management"
last_updated: "2026-06-09T18:44:38+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Skill Frontmatter Emission Audit Against the Unified Schema (work item 0103)

**Date**: 2026-06-09T18:44:38+00:00
**Author**: Toby Clemson
**Git Commit**: dc9490fb6caadb129f9899e48d6e5e300ec2d663
**Branch**: (detached / jj workspace `ticket-management`)
**Repository**: ticket-management

## Research Question

For work item 0103 (*Audit Skill Frontmatter Emission Against the Unified Schema*): map the
implementation terrain the task touches — (1) the contract the corpus validator enforces, axis
by axis; (2) which skills emit frontmatter and how their emission instructions diverge; (3) the
exact mechanics of the known validate-plan bug; (4) the existing test harness and how to wire in
an automated producer-conformance guard; and (5) the historical decisions (ADR-0042, plan 0070,
epic 0057) that ground the task's framing.

## Summary

The task's premise is **accurate and corroborated**: the 0070 migration validated the *corpus*
and shipped the validator + emission-rules helper + schema TSV, but never audited the *producers*
(skills). All three contract surfaces exist on disk. The known divergence is real and isolated:
`validate-plan` writes the literal `complete` onto the validated **plan** (`SKILL.md:187`), outside
the plan's vocab `draft|ready|in-progress|done` — the fix is a one-word literal change to `done`.

Three findings materially shape how the task should be planned:

1. **The validator is a partial oracle.** The work item's Assumptions name
   `validate-corpus-frontmatter.sh` as the authoritative oracle, but it has real blind spots:
   it enforces the provenance "iff" only in the *present* direction (anchored ⇒ bundle present;
   it does **not** reject `revision`/`repository` on a non-anchored type), and it shape-checks
   only *quoted* typed-linkage tokens (a bare/unquoted ref yields no tokens and is silently
   un-checked). "Passes the validator" is therefore necessary but not sufficient for full
   conformance — the per-attribute audit (AC2) must cover axes the validator does not.

2. **Emission = skill prose + template, not prose alone.** Several skills (validate-plan among
   them) do not substitute every base field in their SKILL.md prose — `tags` and the provenance
   bundle often come from the loaded template / `artifact-derive-metadata.sh`, not an explicit
   instruction. The audit must evaluate the *composed* emission, or it will false-positive on
   "missing" fields that the template supplies.

3. **The new guard lands in `test:integration:config` by glob discovery**, but the closest
   precedent the work item cites (`test-skill-frontmatter-population.sh`) is a *static prose
   checker* that runs under `test:unit:templates` and never drives a skill or runs the validator.
   The better structural model is `test-validate-corpus-frontmatter.sh` (generates fixtures →
   runs the validator → asserts rc/stderr). Adding a suite requires bumping the
   `_EXPECTED_CONFIG_SUITES` floor from 15 → 16.

Net: the work item is well-grounded and implementable largely as written, with two refinements
worth folding into planning — (a) state that AC4's "passes the validator" is a floor, not the
whole conformance check, given the validator's blind spots; and (b) clarify that the audit checks
prose-plus-template composed emission.

## Detailed Findings

### 1. The contract — what `validate-corpus-frontmatter.sh` enforces (11 axes)

The contract is deliberately split three ways: per-type tabular facts in
`scripts/templates-schema.tsv`; cross-cutting rules (no per-type column) in
`scripts/frontmatter-emission-rules.sh`; and `scripts/validate-corpus-frontmatter.sh` as the
runtime gate combining both. The emission-rules helper is sourced by both the validator
(`validate-corpus-frontmatter.sh:26-27`) and the template-shape test
(`test-template-frontmatter.sh:20`) so those two surfaces cannot drift.

The TSV is parsed into seven parallel arrays (`:41-54`; bash 3.2 has no associative arrays);
per-file dispatch resolves a type's row via `schema_index()` (`:57-66`) and pulls the five
per-type facts into locals (`:261-265`).

**Pre-checks**: fence present (`has_fence()` `:115-120`, `NO-FENCE`); `type:` is a known schema
type (`:255-260`, `INVALID-TYPE`).

| # | Axis | Rule / diagnostic | Source | Ref |
|---|------|-------------------|--------|-----|
| 1 | Required base fields | each of `FM_BASE_FIELDS` present (`MISSING-BASE-FIELD`) | shared (`producer`,`status` deliberately **not** required) | `:271-275`; rules `:22-26` |
| 2 | Quoted `id` | value matches `^"[^"]*"(…)?$` (`UNQUOTED-ID`) | hard-coded local `re_id_val` | `:268,277-280` |
| 3 | `schema_version` bare `1` | value matches `^1(…)?$` (`BAD-SCHEMA-VERSION`) | hard-coded local `re_sv_val` | `:269,282-285` |
| 4 | `date`/`last_updated` ISO | full date-time `T`, secs, `Z`/`±HH:MM` (`BAD-TIMESTAMP`) | hard-coded `ISO_TS_RE` | `:214,287-295` |
| 5 | Per-type `status` vocab | inner value ∈ `status_vocab` split on `\|` (`BAD-STATUS`) | data-driven (TSV) | `:297-312` |
| 6 | Provenance iff anchored | `anchored=yes` ⇒ `revision`+`repository` present (`MISSING-PROVENANCE`) | anchored flag data-driven; field names shared | `:314-324` |
| 7 | Forbidden provenance | `git_commit`/`branch` never present, all types (`FORBIDDEN-PROVENANCE`) | shared | `:321-324` |
| 8 | Forbidden own-id key | per-type legacy id key(s) absent (`FORBIDDEN-OWN-ID`) | data-driven (TSV, `-` sentinel) | `:326-332` |
| 9 | Required extras | each per-type extra present unless in `FM_OPTIONAL_EXTRAS` (`MISSING-EXTRA`) | hybrid: extras data-driven, carve-out shared | `:334-338`; rules `:69` |
| 10 | Omit-when-empty | no value exactly `""` or `[]` (except `tags`) (`EMPTY-PLACEHOLDER`) | hard-coded blanket | `:340-351` |
| 11 | Typed-linkage shape | each *quoted* token matches `"doc-type:id"` (`BAD-LINKAGE-SHAPE`); + dangling-ref in corpus mode | per-type keys data-driven; regex shared | `:353-376`; rules `:83` |

**Two validator blind spots that matter for the audit** (the oracle is partial):

- **Provenance "iff" is one-directional** (`:314-324`): an anchored type missing the bundle fails,
  but a *non-anchored* type that wrongly emits `revision`/`repository` is **not** flagged. So a
  producer over-emitting provenance passes the validator.
- **Only quoted linkage tokens are shape-checked** (`:358-360`): the parser extracts
  `"…"` tokens; a bare `parent: 0042` or path-shaped value yields no tokens and escapes the
  `BAD-LINKAGE-SHAPE` check entirely (it would only be caught by omit-when-empty if empty, or by
  dangling-ref if it happened to parse).

**Referential integrity** (`DANGLING-REF`, `:366-374`) runs only in whole-corpus mode
(`referential=yes`); `pr:`-prefixed tokens are tolerated (`:368`).

**Intra-contract divergence**: the validator re-encodes `id`/`schema_version` as value-only
regexes (`re_id_val`/`re_sv_val`, `:268-269`) instead of reusing the shared full-line
`FM_ID_QUOTED_RE`/`FM_SCHEMA_VERSION_RE` (`frontmatter-emission-rules.sh:74,77`). Semantically
equivalent, but the only two axes not single-sourced between the validator and the template test.

### 2. The schema TSV — 13 types, the per-type fact table

`scripts/templates-schema.tsv:1` header (7 tab-separated columns): `template`, `type`,
`code_state_anchored`, `extras`, `status_vocab`, `forbidden_own_id_key`, `typed_linkage_keys`.
The 13 data rows (`:2-14`):

- **Anchored (`yes`)** — must carry `revision`+`repository`: `plan`, `pr-description`,
  `codebase-research`, `issue-research`, `design-inventory`, `note`.
- **`status_vocab`** — multi-value: `work-item` (`draft|ready|in-progress|review|done|blocked|abandoned`),
  `plan` (`draft|ready|in-progress|done`), `adr` (`proposed|accepted|superseded|deprecated`),
  `design-inventory` (`draft|superseded`), `design-gap` (`draft|accepted`). Single-token `complete`:
  `plan-validation`, `pr-description`, `codebase-research`, `issue-research`, `plan-review`,
  `work-item-review`, `pr-review`. `note` = `captured`.
- **`forbidden_own_id_key` (non-`-`)**: `work-item`→`work_item_id`, `pr-description`→`pr_title`,
  `adr`→`adr_id`, `pr-review`→`pr_title review_pass` (two-key list).
- **`target`** is a linkage key on the four review/validation types
  (`plan-validation`, `plan-review`, `work-item-review`, `pr-review`).
- **`source`** is a linkage key only on `work-item`; on `design-inventory` it is an *extra*
  (a documented exemption — see `test-template-frontmatter.sh:76-78,83`).

`scripts/frontmatter-emission-rules.sh` exports the cross-cutting sets a guard can reuse without
re-encoding: `FM_BASE_FIELDS` (`:26`), `FM_PROVENANCE_FIELDS` (`:29`),
`FM_FORBIDDEN_PROVENANCE_FIELDS` (`:30`), `FM_OPTIONAL_EXTRAS` (`:69`), `FM_TYPED_REF_RE`/
`FM_SOURCE_TYPE_RE` (`:83,36`), `FM_LINKAGE_VOCABULARY` (`:42`), and the functions
`fm_linkage_cardinality()` (`:47`) and `fm_is_linkage_key()` (`:89`, currently unused — a
ready-made helper for the new guard).

### 3. The producers — 17 confirmed frontmatter emitters

A tight grep (`schema_version` / `Populate frontmatter` / `Substitute…frontmatter` /
`frontmatter-emission` / `artifact-derive-metadata.sh`) yields **17 artifact-emitting SKILL.md
files** — this is the concrete "~18" enumeration AC1 depends on:

- **work/**: `create-work-item`, `refine-work-item`, `review-work-item`, `extract-work-items`
- **planning/**: `create-plan`, `review-plan`, `validate-plan`
- **decisions/**: `create-adr`, `extract-adrs`
- **research/**: `research-codebase`, `research-issue`
- **design/**: `inventory-design`, `analyse-design-gaps`
- **github/**: `describe-pr`, `review-pr`
- **notes/**: `create-note`
- **config/**: `migrate` (rewrites frontmatter during meta-corpus migration)

**Borderline (decide scope during the audit)** — these mutate/read frontmatter but may not emit a
fresh schema-governed block: `work/update-work-item` (edits arbitrary fields on an existing item),
`decisions/review-adr` (updates status), `work/list-work-items` (consumer), plus config/lens
skills that use non-artifact frontmatter. The "18th" file is most likely `update-work-item` or
`review-adr`. **Recommendation for AC1**: define the membership rule as "emits a fresh
schema-governed frontmatter block" and treat pure mutators (`update-work-item`, `review-adr`)
as a separately-scoped category, since they don't write a full block to validate.

**Emission-instruction shapes and where divergence concentrates** (from the pattern survey):

- **Three structural variants** of the "Populate frontmatter" block: canonical numbered
  (helper → Substitute list → write; `create-plan:225-275`, `create-work-item:436-487`);
  helper-in-earlier-step (`create-adr:144-187`, `research-issue:93-131`); flat bullet list
  (`research-codebase:123-161`, `create-note:92-125`).
- **Provenance handling has four styles** — always-emit (anchored), explicit-omit-with-prose
  (`create-adr:155`, `analyse-design-gaps:186-188`), conditional-omit
  (`inventory-design:247-250`), and **silent-omit** (the review skills request only the date and
  never mention the bundle). The silent-omit reviews are the easiest to mis-classify.
- **`verdict` enum diverges**: `review-pr` uses `APPROVE | REQUEST_CHANGES | COMMENT`
  (`review-pr:493`) while `review-work-item`/`review-plan` use `… | REVISE | …`.
- **`review_pass` presence diverges**: present in `review-work-item`/`review-plan`, deliberately
  absent in `review-pr` (`:501-506`, with prose explaining why) — matches the TSV, where
  `pr-review`'s `forbidden_own_id_key` is `pr_title review_pass`.
- **Scalar `work_item_id:` alias coexists with typed `target:`/`parent:`** in reviews
  (`review-work-item:383-385`), `describe-pr:141-142`, `research-codebase:160-161`,
  `research-issue:129-130` — a transitional duplication and an audit hotspot.
- **`create-note` writes `tags: []`** (`:107`) — the one documented case of writing an empty
  array rather than omitting; it is exempt because `tags` is the omit-when-empty carve-out
  (validator `:343`).
- **Omit-by-default preamble wording / ADR citation drifts**: design skills cite ADR-0040
  (`create-plan`-style blocks cite none).

### 4. The known divergence — `validate-plan` SKILL.md

`validate-plan` touches `status:` in exactly two places, one per artifact type:

- **Legitimate** — `skills/planning/validate-plan/SKILL.md:161`: `- \`status:\` ← \`complete\``
  for its own **plan-validation report**. Correct: `plan-validation` vocab is `complete`
  (`templates-schema.tsv:4`).
- **The bug** — `SKILL.md:186-188`: *"If the validation result is `pass`, update the plan's
  frontmatter `status` field to `complete` …"*. Wrong: the **plan** vocab is
  `draft|ready|in-progress|done` (`templates-schema.tsv:3`); `complete` is not a member. ADR-0042
  maps plan `complete → done`. **Fix = change the single literal `complete` at `:187` to `done`.**

The wrong value is a **hard-coded literal**, almost certainly copied from the (correct) report
status at `:161`. Nothing in validate-plan sources status facts from the TSV or emission-rules
helper — both status values are inline literals. So this is a "change a literal" fix, not a
"wire to the schema" fix.

**The bug is isolated to status.** The report's other fields conform to `templates-schema.tsv:4`:
`type: plan-validation`, extras `result`, typed-linkage `parent target relates_to`, quoted `id`,
bare-integer `schema_version`. The two base fields validate-plan does **not** explicitly
substitute in prose are `tags` and the provenance bundle — these rely on the loaded template
(`SKILL.md:33`) / `artifact-derive-metadata.sh` (`:150`). This is the concrete example of why the
audit must check composed (prose + template) emission, not prose alone.

### 5. The test harness and how to wire the new guard

**`test-skill-frontmatter-population.sh` is a static prose checker** — it does NOT drive a skill
or run the validator. It reads each SKILL.md and asserts (via `awk` matchers
`in_fenced_block`/`in_imperative_section`/`in_populate_section_with_guidance`, `:80-168`) that the
prose *instructs* populating required fields, driven by `scripts/skills-schema.tsv` (4 columns:
`skill_path`, `producer_name`, `fields`, `omit_when_empty`). It has count-gated aggregate
assertions and an inline liveness self-test (`:234-240,290-356`) so it can't go inert. It runs
under **`test:unit:templates`** (`tasks/test/unit.py:35-40`), not integration.

**`test-validate-corpus-frontmatter.sh` is the better structural model** for a producer-conformance
guard: it generates artifacts in a tmpdir (`emit_valid`, `:31-65`), uses a `trap … EXIT` teardown
(`:23`), and shells out to the validator capturing rc/stderr (`run_validator`, `:68-71`). It runs
under **`test:integration:config`**.

**Wiring (glob discovery, no registration):** `test:integration:config` (`mise.toml:132-134`) →
`invoke test.integration.config` (`tasks/test/integration.py:38-49`) → `run_shell_suites(context,
"scripts")` (`tasks/test/helpers.py:13-40`), which globs `scripts/**/test-*.sh`, filters to
regular files, name ≠ `test-helpers.sh`, **executable bit set** (`os.access(p, os.X_OK)`,
`helpers.py:34`), and runs each. A floor guard `_EXPECTED_CONFIG_SUITES = 15`
(`integration.py:14`) is an at-least check.

**Minimum for a new `scripts/test-<name>.sh` guard:**
1. Live at `scripts/test-*.sh`, be a regular file, **`chmod +x`** (non-executable = silently skipped).
2. **Bump `_EXPECTED_CONFIG_SUITES` 15 → 16** (`integration.py:14`) so the floor tracks the new count.
3. `#!/usr/bin/env bash` + `set -euo pipefail`; resolve `SCRIPT_DIR`; `source test-helpers.sh`
   (gives `PASS`/`FAIL` counters + `assert_*`); `export LC_ALL=C` (bash 3.2 / locale discipline).
4. Reuse `frontmatter-emission-rules.sh`, `templates-schema.tsv` (and `skills-schema.tsv` if
   asserting per-skill expectations) rather than re-encoding the contract.
5. Generate fixtures in `mktemp -d` with `trap 'rm -rf "$TMP"' EXIT`; invoke
   `validate-corpus-frontmatter.sh` as a subprocess; assert via shared helpers; end with
   `test_summary` so a non-zero return fails the task.
6. Add a count-gated aggregate assertion / liveness self-test so a zero-iteration loop can't go
   inert (the population test's precedent).

This confirms AC5/AC6 are achievable as written: a new integration suite is auto-discovered and a
negative test (mutate a fixture → assert the validator fails) is the natural shape. The one
mechanical gotcha is the floor bump.

## Code References

- `scripts/validate-corpus-frontmatter.sh:41-54` — TSV parsed into 7 parallel arrays (bash 3.2)
- `scripts/validate-corpus-frontmatter.sh:271-376` — the 11 validation axes
- `scripts/validate-corpus-frontmatter.sh:314-324` — provenance "iff" (one-directional)
- `scripts/validate-corpus-frontmatter.sh:358-362` — typed-linkage shape (quoted tokens only)
- `scripts/templates-schema.tsv:1-14` — header + 13 per-type fact rows
- `scripts/frontmatter-emission-rules.sh:26,29,30,69,83,89` — reusable cross-cutting sets/helpers
- `skills/planning/validate-plan/SKILL.md:161` — legitimate `status: complete` (plan-validation)
- `skills/planning/validate-plan/SKILL.md:186-188` — the bug: plan `status` → `complete` (should be `done`)
- `scripts/test-skill-frontmatter-population.sh:80-168,234-356` — static prose checker + self-tests
- `scripts/test-validate-corpus-frontmatter.sh:22-23,31-71` — fixture/teardown/validator-invocation model
- `tasks/test/integration.py:14,38-49` — `config` task + `_EXPECTED_CONFIG_SUITES` floor
- `tasks/test/helpers.py:13-40` — `run_shell_suites` glob discovery (executable-bit gate at `:34`)
- `tasks/test/unit.py:35-40` — `test:unit:templates` runs the population/template/metadata suites
- `mise.toml:113-169` — `test:integration:*` task definitions

## Architecture Insights

- **Three-file contract split is intentional and load-bearing**: per-type facts (TSV) vs
  cross-cutting rules (helper) vs runtime gate (validator). The audit and the new guard should
  source from the first two, never re-encode them — this is exactly what the work item's
  Technical Notes already say, and the codebase supports it (the helper exposes everything needed).
- **Emission is composed (prose + template + metadata helper)**, not prose-only. The population
  test checks *prose guidance*; the corpus validator checks *output*. The new producer-conformance
  guard the work item wants sits in the gap between them: it must validate *composed output*, which
  is why the fixture-driving model (`test-validate-corpus-frontmatter.sh`) is the right precedent.
- **The validator is a partial oracle** (provenance over-emission and bare-linkage shape are
  un-checked). Treating "passes the validator" as full conformance (a literal reading of AC4)
  would miss those axes. The per-attribute conformance table (AC2) is the part of the task that
  closes that gap — its completeness criterion (attribute set == validator-enforced set) should be
  read as "the axes the validator enforces," with the un-enforced axes (provenance over-emission,
  linkage quoting) called out as audit-by-inspection rather than validator-checkable.
- **bash 3.2 / `LC_ALL=C` discipline** pervades the suite (macOS CI floor). Any new guard must
  avoid associative arrays and bash-4 constructs, and parse the TSV with the established
  `tail -n +2 | IFS=$'\t' read` pattern.

## Historical Context

- `meta/decisions/ADR-0042-reconciling-pre-schema-status-values.md` — the status-reconciliation
  map (single-sourced in `scripts/status-legacy-map.tsv`). Plan: `accepted|complete|implemented|
  final|revised → done`; `approved|reviewed → ready`. Reconciles **`plan-review`** (vocab
  `complete`), and says **nothing** about `plan-validation` (a distinct type, treated as
  already-conforming). Chosen Option 1: collapse synonyms, widen a vocab only for genuinely
  distinct states (design-gap `+accepted`, design-inventory `+superseded`). Unmapped values are a
  migration error, not a pass-through.
- `meta/plans/2026-06-07-0070-meta-corpus-unified-schema-migration.md` (status `done`) — migrated
  and validated the **corpus**, shipped the validator (new in Phase 1), the shared emission-rules
  helper, and edited the TSV for the ADR-0042 widenings. **No phase audited the producing skills**
  — confirming 0103's premise. The producer-conformance gap is left open, not enumerated as a
  deferred item (the one explicit follow-on it does name is 0102, the visualiser fallback-arm
  contraction).
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` (epic, `in-progress`) —
  parent of 0103. Producer updates are *in scope as a requirement* but the epic never enumerated a
  dedicated producer-*audit* child; 0103 fills that. The "raise follow-ons as children under 0057"
  plan has solid precedent — **0102 is the structurally identical sibling** (also
  `parent: "work-item:0057"`).
- **No substantive contradictions** between these documents and the work item. The subtlety to
  hold: `plan-validation` "already conforms" refers to the migrated *corpus*; 0103's bug is at the
  *producer* (validate-plan's SKILL.md), never checked by 0070 — precisely the "conforming by
  inspection is how drift hides" seam.

## Related Research

- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` (parent epic)
- `meta/work/0070-*` / `meta/plans/2026-06-07-0070-meta-corpus-unified-schema-migration.md` (the migration that shipped the contract)
- `meta/work/0102-remove-visualiser-legacy-linkage-fallback-arms.md` (sibling follow-on precedent under 0057)
- `meta/decisions/ADR-0042-reconciling-pre-schema-status-values.md`, `ADR-0034` (typed linkage), `ADR-0033` (base schema)

## Open Questions

1. **AC1 set size**: the tight grep finds 17 clear producers. Is `update-work-item` (or
   `review-adr`) the intended "18th", or should pure mutators be a separate category? The audit's
   membership rule should resolve this explicitly.
2. **AC4 wording vs validator blind spots**: should AC4 acknowledge that "passes the validator" is
   a floor (provenance over-emission on non-anchored types and bare/unquoted linkage values are not
   validator-caught), with those axes verified by inspection in the AC2 table instead?
3. **Composed-emission methodology**: how should the audit attribute a field to "skill prose" vs
   "template" vs "metadata helper" when deciding whether a producer conforms — does a field the
   template supplies count as the skill emitting it correctly? (validate-plan's `tags`/provenance
   are the concrete case.)
4. **Guard mode per skill**: which producers can be fixture-driven (deterministic emission) vs
   which fall back to documented-emission assertion? The reviews and research skills interleave
   model-authored content with frontmatter, which may complicate fixture-driving.
5. **`migrate` skill scope**: `config/migrate` rewrites frontmatter during migration — is it in
   scope for the producer audit, or is it a corpus-transformer governed differently?
