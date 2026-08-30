---
type: "plan-validation"
id: "2026-06-02-0093-extend-templates-with-typed-linkage-slots-validation"
title: "Validation Report: Extend Templates With Typed-Linkage Slots"
date: "2026-06-04T20:56:25+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-06-02-0093-extend-templates-with-typed-linkage-slots"
tags: ["templates", "frontmatter", "schema", "linkage", "adr-0040", "emission"]
last_updated: "2026-06-04T20:56:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Extend Templates With Typed-Linkage Slots

### Implementation Status

- ✓ **Phase 0**: Author the emission-convention ADR (ADR-0040) — Fully implemented
- ✓ **Phase 1**: Extend template test infrastructure and update templates — Fully implemented
- ✓ **Phase 2**: Extend skill test infrastructure for omit-when-empty guidance — Fully implemented
- ✓ **Phase 3**: Writer-canon SKILL.md sweep (eight Group A skills) — Fully implemented
- ✓ **Phase 4**: Reviewer-canon heading lift (four Group B skills) — Fully implemented
- ✓ **Phase 5**: Design carve-out SKILL.md sweep (two Group C skills) — Fully implemented
- ✓ **Phase 6**: refine-work-item lift and bare-id producer normalisation — Fully implemented

Each phase maps to a discrete commit on the current line of work
(`mzoqqllt` → ADR-0040; `nosktryy` → templates/shape test; `onzxlpls` →
skill-prose test; `wrxtouon` → writer-canon skills; `olsonnno` → reviewer
skills; `svvzowvr` → design skills; `qvnxzonx` → refine-work-item
promotion), matching the plan's tests-first, content-after phase model.

### Automated Verification Results

All commands run from the `build-system` workspace.

- ✓ `bash scripts/test-template-frontmatter.sh` — 306 passed, 0 failed
- ✓ `bash scripts/test-skill-frontmatter-population.sh` — 162 passed, 0 failed
- ✓ `mise run test:unit:templates` — 36 passed, 0 failed (exit 0)
- ✓ `mise run test:unit:visualiser` — 393 passed, 0 failed (exit 0)
- ✓ `mise run test` (full suite) — exit 0; e2e visualiser 413 passed

**Count gates (the plan's "exact count, not absence of FAIL" discipline):**

- ✓ shape+comment PASS count = **36** (matches TSV-derived `expected` =
  36 from `awk` over column 7)
- ✓ closed-set PASS count = **12** (one per template; `design-inventory.md`
  passes — its foreign-source `source:` is exempted, not flagged)
- ✓ negative-fixture self-test = **6** PASS (all bad fixtures rejected)
- ✓ vocabulary-drift guard = **9** PASS (every `LINKAGE_VOCABULARY` entry
  has a cardinality)
- ✓ guidance-helper liveness self-test = **5** PASS
- ✓ reviewer literal-heading count = **4** PASS
- ✓ omit-when-empty TSV-derived total = **63** populated entries; every
  populated row reports PASS, no FAIL

**bash 3.2 compatibility:**

- ✓ `/bin/bash --version` → 3.2.57; `test-template-frontmatter.sh` runs
  clean with no `declare: -A: invalid option` (case-based
  `linkage_cardinality()` confirmed)

### Code Review Findings

#### Matches Plan

- **Phase 0 — ADR-0040.** File present
  (`meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`),
  `status: accepted`, dual title naming the supplemented ADR
  (`# ADR-0040: … — supplement to ADR-0033`). Full house section set
  present: Context, Decision Drivers, Considered Options, Decision,
  Consequences (split Positive/Negative/Neutral), References. The
  *Emission classification* scope table is reproduced inside Decision
  (Always emitted / Emitted only when non-empty). The reader-facing rule
  ("an absent optional key MUST be read as 'no value', never as an error
  or … missing/oversight data") appears in the Negative consequences. The
  recursive-supplement clause is present (§"Recursive supplement clause").
- **Phase 1 — templates + shape test.** `templates-schema.tsv` carries the
  seventh `typed_linkage_keys` column; `target` moved out of `extras` on
  the four reviewer rows. All 12 templates carry the standalone block-header
  comment (`# typed-linkage slots — omit-when-empty in artifacts …; see
  ADR-0040`) and the gated linkage slots in the normative grammar.
  `work-item.md` and `plan.md` each carry the standalone inverse-guidance
  line below `blocked_by`. `design-gap.md`'s `current_inventory` /
  `target_inventory` and `design-inventory.md`'s `source:` are preserved.
- **Phase 2 — skill-prose test.** `skills-schema.tsv` has the fourth
  `omit_when_empty` column (NF = 4); the new
  `in_populate_section_with_guidance` helper, its loop, and the 5-fixture
  liveness self-test are present and gated.
- **Phase 3 — writer skills.** All eight writer-canon rows populated;
  every omit-when-empty field reports PASS with fill/omit guidance.
- **Phase 4 — reviewer skills.** All four reviewer SKILL.md files carry a
  literal `### Populate frontmatter` heading; the count-gated (4)
  literal-heading assertion passes.
- **Phase 5 — design skills.** `inventory-design` and `analyse-design-gaps`
  populated; the `current_inventory` / `target_inventory` bullets are
  preserved.
- **Phase 6 — normalisation.** `refine-work-item` moved to
  `IN_SCOPE_PRODUCERS` (discovery pass green). `refine-work-item`'s
  `parent:` and `create-plan`'s `work_item_id:` both emit the typed
  `"work-item:NNNN"` form. The equivalence regression test
  `parent_typed_form_resolves_same_as_bare_id` is present in
  `cluster_key.rs` and passes within the green visualiser suite.

#### Deviations from Plan

- None material. The implementation tracks the plan's specified shapes,
  counts, and file targets exactly, including the count-gating discipline
  the plan emphasised.

#### Potential Issues

- **Guidance-level enforcement only (documented, accepted tradeoff).** As
  the plan's "What We're NOT Doing" and ADR-0040's Negative consequences
  both record, no test inspects a *generated artifact* to confirm an empty
  optional key was actually omitted — a producer emitting `external_id: ""`
  would violate the convention with no test signal. This is a deliberate
  scope boundary, explicitly flagged so 0070 does not assume structural
  enforcement. Not a defect; noted for the downstream consumer.

### Manual Testing Required

The plan's manual steps (live visualiser smoke of a parent/child cluster
created via `refine-work-item`; inspecting freshly generated frontmatter
to confirm empty optionals are omitted rather than present-but-empty) are
optional confidence checks. The runtime path they exercise is already
covered by the green `cross-refs.spec.ts` e2e suite and the new
`parent_typed_form_resolves_same_as_bare_id` equivalence test, so they are
not gating. Run them only if a live-eyes confirmation is desired:

1. - [ ] Start the visualiser; inspect a work-item cluster with a
     parent+child created via `refine-work-item` — confirm both render and
     the parent edge draws.
2. - [ ] Generate a child via `refine-work-item` decompose and a plan via
     `create-plan` with no linked work item — confirm `parent:` is written
     in `"work-item:NNNN"` form and empty optionals are omitted.

### Recommendations

- None blocking. The implementation is complete and the full suite is
  green. The follow-up to watch is 0070 (corpus migration), which owns
  inferring links into the new slots and must honour the omit-when-empty
  convention as ADR-0040 records.
